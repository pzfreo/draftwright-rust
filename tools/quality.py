"""Drawing quality for draftwright-rust: per part, items off the sheet, repeated dimensions,
overlapping views, and ink overlaps, from `draftwright plan` and `render`. Writes baseline.json
in <out dir> and prints a table with totals. Random pins: --seed S --pins N;
--require-go enforces the per-part 1.5x ink / no new off-sheet items bar.

The maintainer's priority (2026-10-09) is to drive these to zero before adding features; see
docs/quality-2026-10-09.md for the baseline and the parts it is measured on.

Repeated spans use placed linear arrow tips, model-mm tolerance 1e-6 for ends and values,
and ignore lane and label. PMI face-pair near misses are included. Use
--require-no-repeated-span to enforce zero; baseline.json retains the groups' item IDs.

Usage: python3 -I tools/quality.py <draftwright binary> <out dir> <step files...>
(the parts: tools/quality_parts.txt, paths relative to $DRAFTWRIGHT_CORPUS)"""
import argparse, collections, hashlib, json, subprocess, sys, os, re
from concurrent.futures import ThreadPoolExecutor

INK = {"annotation_ink_overlap", "annotation_over_view", "annotation_line_over_view",
       "annotation_leader_crossing", "annotation_outside_area"}


def line_through_features(plan):
    """Leader/feature pairs, plus extension lines (once even if several features are crossed)."""
    leaders, extensions = set(), set()
    for p in plan.get("problems", []):
        subjects = p.get("subjects", [])
        if p["code"] == "leader_through_feature":
            leaders.add(tuple(subjects))
        elif p["code"] == "extension_through_feature":
            extensions.add((subjects[0], p.get("id", "").rsplit(":line:", 1)[-1]))
    return len(leaders) + len(extensions)


def overlap(a, b):
    return a[0] < b[2] and a[2] > b[0] and a[1] < b[3] and a[3] > b[1]


def repeated_spans(plan):
    """Placed linear statements, independent of lane and label, in model millimetres.

    Arrow tips retain the witness stations even with outside arrows or a broken line.
    PMI identities also prove a face pair: equal values on that pair at different
    stations are reported separately for review, never inferred from equal labels.
    """
    numerator, denominator = map(float, plan['sheet']['scale'].split(':'))
    scale = numerator / denominator
    spans = []
    for d in sorted(plan['dimensions'], key=lambda d: d['id']):
        placed = d.get('placed')
        if d.get('kind') != 'linear' or not placed or len(placed.get('arrows', [])) != 2:
            continue
        axis = 0 if placed['rotation'] % 180 == 0 else 1
        ends = sorted(a[0][axis] / scale for a in placed['arrows'])
        faces = set()
        for m in d.get('measurements', []):
            if m['feature'].startswith('pmi:'):
                match = re.search(r':f([0-9,]+)(?=:|$)', m['feature'])
                if match:
                    pair = tuple(sorted(map(int, match[1].split(','))))
                    if len(pair) == 2:
                        part = re.search(r':p([0-9]+)(?=:|$)', m['feature'])
                        faces.add((part[1] if part else None, pair))
        spans.append((d, axis, ends, faces))
    groups = []
    for d, axis, ends, faces in spans:
        matches = []
        for i, group in enumerate(groups):
            for other, k, stations, pair in group['members']:
                if d['view'] != other['view'] or axis != k or abs(d['value'] - other['value']) > 1e-6:
                    continue
                same = all(abs(a - b) <= 1e-6 for a, b in zip(ends, stations))
                if same or faces & pair:
                    matches.append((i, 'span' if same else 'feature_pair'))
                    break
        if matches:
            first, kind = matches[0]
            groups[first]['members'].append((d, axis, ends, faces))
            if kind == 'feature_pair':
                groups[first]['kind'] = kind
            for i, kind in reversed(matches[1:]):
                groups[first]['members'].extend(groups[i]['members'])
                if kind == 'feature_pair' or groups[i]['kind'] == 'feature_pair':
                    groups[first]['kind'] = 'feature_pair'
                del groups[i]
        else:
            groups.append({'kind': 'span', 'members': [(d, axis, ends, faces)]})
    return sorted(({'kind': g['kind'], 'items': sorted(d['id'] for d, *_ in g['members'])}
                   for g in groups if len(g['members']) > 1), key=lambda g: g['items'])


def one(f, dw, out, spec=None):
    name = os.path.basename(f).split(".")[0]
    os.makedirs(out, exist_ok=True)
    extra = ["--spec", spec] if spec else []
    r = subprocess.run([dw, "render", f, "-o", f"{out}/{name}", "--formats", "svg", *extra], capture_output=True, text=True, timeout=600)
    p = subprocess.run([dw, "plan", f, *extra], capture_output=True, text=True, timeout=600)
    try:
        rend, plan = json.loads(r.stdout), json.loads(p.stdout)
    except Exception:
        return name, {"error": (r.stdout + r.stderr)[:200]}
    if r.returncode or p.returncode or "dimensions" not in plan or "error" in rend:
        return name, {"error": json.dumps(plan.get("error", plan))[:200]}
    with open(f"{out}/{name}.plan.json", "w") as output:
        json.dump(plan, output, indent=1, sort_keys=True)
    codes = collections.Counter(x["code"] for x in rend.get("problems", []))
    # Repeated dimensions: the same measured quantity (feature, parameter) stated more than once,
    # and, separately, the same label on the same span in the same view (a drawn duplicate).
    meas = collections.Counter()
    spans = collections.Counter()
    for d in plan["dimensions"]:
        for m in d.get("measurements", []):
            meas[(m.get("feature"), m.get("parameter"))] += 1
        pl = d.get("placed") or {}
        lines = pl.get("lines") or []
        view = d["view"]
        if lines:
            ends = sorted((round(x, 1), round(y, 1)) for l in lines[:2] for (x, y) in l)
            spans[(view, d.get("label"), tuple(ends))] += 1
    labels = collections.Counter(
        (d["view"], d.get("label"), (d.get("placed") or {}).get("rotation", 0))
        for d in plan["dimensions"] if d.get("kind") == "linear")
    rep_label = sum(n - 1 for n in labels.values() if n > 1)
    rep_meas = sum(n - 1 for n in meas.values() if n > 1)
    rep_span = sum(n - 1 for n in spans.values() if n > 1)
    repeated = repeated_spans(plan)
    views = [v["area"] for v in plan["views"] if v.get("area")]
    tb = plan["sheet"].get("title_block")
    view_over = sum(overlap(views[i], views[j]) for i in range(len(views)) for j in range(i + 1, len(views)))
    view_tb = sum(overlap(v, tb) for v in views) if tb else 0
    return name, {
        "off_sheet": codes.get("annotation_off_sheet", 0) + codes.get("pinned_view_off_sheet", 0),
        "off_sheet_items": sorted({subject for problem in rend.get("problems", [])
                                   if problem["code"] in {"annotation_off_sheet", "pinned_view_off_sheet"}
                                   for subject in problem.get("subjects", [])}),
        "repeated_measure": rep_meas, "repeated_drawn": rep_span, "repeated_label": rep_label,
        "repeated_span": sum(len(g['items']) - 1 for g in repeated),
        "repeated_span_groups": repeated,
        "views_overlap": view_over, "view_on_title_block": view_tb,
        "ink": sum(codes[c] for c in INK), "ink_by_code": {c: codes[c] for c in INK if codes[c]},
        "line_through_feature": line_through_features(plan),
        "dimensions": len(plan["dimensions"]),
    }


def table(res):
    keys = ["off_sheet", "repeated_label", "repeated_measure", "views_overlap", "view_on_title_block", "ink", "repeated_span", "line_through_feature"]
    print(f"{'part':28}" + "".join(f"{k[:12]:>13}" for k in keys))
    tot = collections.Counter(); bad = collections.Counter()
    for n, v in sorted(res.items()):
        if "error" in v:
            print(f"{n:28} ERROR {v['error'][:80]}"); continue
        print(f"{n[:28]:28}" + "".join(f"{v[k]:13}" for k in keys))
        for k in keys:
            tot[k] += v[k]; bad[k] += v[k] > 0
    print(f"{'TOTAL':28}" + "".join(f"{tot[k]:13}" for k in keys))
    print(f"{'parts affected':28}" + "".join(f"{bad[k]:13}" for k in keys), f"of {sum('error' not in v for v in res.values())}")


def random_pins(plan, name, seed, count):
    """Bare annotation pins; view pins use the nearest legal relation to a fixed root."""
    candidates = [(d["id"], "dimensions", {"pinned": True})
                  for d in plan["dimensions"] if d.get("placed") is not None]
    candidates += [(g["id"], "annotations", {"pinned": True})
                   for g in plan.get("gdt", []) if g.get("placed") is not None]
    views = sorted(plan["views"], key=lambda v: (v["id"] != "v:front", v["id"]))
    if len(views) > 1:
        anchor = views[0]
        b = anchor["area"]
        for v in views[1:]:
            a = v["area"]
            dx, dy = (a[0]+a[2]-b[0]-b[2])/2, (a[1]+a[3]-b[1]-b[3])/2
            if abs(dx) > abs(dy):
                side, gap, k = ("right", a[0]-b[2], 1) if dx > 0 else ("left", b[0]-a[2], 1)
            else:
                side, gap, k = ("above", a[1]-b[3], 0) if dy > 0 else ("below", b[1]-a[3], 0)
            offsets = {"middle": (b[k]+b[k+2]-a[k]-a[k+2])/2, "start": b[k]-a[k],
                       "end": b[k+2]-a[k+2]}
            align = min(offsets, key=lambda slot: abs(offsets[slot]))
            candidates.append((v["id"], "views", {"relative_to": anchor["id"][2:], "side": side,
                               "gap": max(0, min(1000, gap)), "align": align, "pinned": True}))
    candidates.sort(key=lambda c: (hashlib.sha256(f"{seed}:{name}:{c[0]}".encode()).digest(), c[0]))
    if count > len(candidates):
        raise ValueError(f"{name}: requested {count} pins, only {len(candidates)} drawable candidates")
    spec = {"schema": "draftwright.spec/2"}
    for id_, kind, place in candidates[:count]:
        spec.setdefault(kind, {})[id_[2:] if kind == "views" else id_] = {"place": place}
    return spec, [c[0] for c in candidates[:count]]


def pin_bar(before, after):
    no_new = after["off_sheet"] <= before["off_sheet"]
    if "off_sheet_items" in before and "off_sheet_items" in after:
        no_new = not (set(after["off_sheet_items"]) - set(before["off_sheet_items"]))
    return after["ink"] <= 1.5 * before["ink"] and no_new


def pinned_one(f, args, baseline):
    name = os.path.basename(f).split(".")[0]
    try:
        if "error" in baseline[name]:
            raise ValueError(baseline[name]["error"])
        p = subprocess.run([args.dw, "plan", f], capture_output=True, text=True, timeout=600)
        if p.returncode:
            raise ValueError(p.stderr or p.stdout)
        spec, ids = random_pins(json.loads(p.stdout), name, args.seed, args.pins)
        path = f"{args.out}/pinned/{name}.spec.json"
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as output:
            json.dump(spec, output, indent=1, sort_keys=True)
        _, after = one(f, args.dw, f"{args.out}/pinned", path)
        if "error" in after:
            raise ValueError(after["error"])
        before = baseline[name]
        return name, {"ids": ids, "unpinned": before, "pinned": after,
                      "ink_delta": after["ink"]-before["ink"],
                      "off_sheet_delta": after["off_sheet"]-before["off_sheet"],
                      "go": pin_bar(before, after)}
    except (ValueError, KeyError, subprocess.TimeoutExpired) as error:
        return name, {"error": str(error), "go": False}


def pin_report(parts, seed, count):
    total = {variant: {key: sum(p[variant][key] for p in parts.values() if "error" not in p)
                       for key in ("ink", "off_sheet")} for variant in ("unpinned", "pinned")}
    go = all(p["go"] for p in parts.values())
    print(f"seed={seed} N={count}: part                         ink before/after off-sheet before/after go")
    for name, p in sorted(parts.items()):
        if "error" in p:
            print(f"{name:48} ERROR {p['error']}")
        else:
            b, a = p["unpinned"], p["pinned"]
            print(f"{name:48} {b['ink']:5}/{a['ink']:<5} {b['off_sheet']:5}/{a['off_sheet']:<5} {'GO' if p['go'] else 'NO-GO'}")
    b, a = total["unpinned"], total["pinned"]
    print(f"TOTAL {b['ink']}/{a['ink']} ink; {b['off_sheet']}/{a['off_sheet']} off-sheet; {'GO' if go else 'NO-GO'} ({sum(not p['go'] for p in parts.values())} failed parts)")
    return {"seed": seed, "pins": count, "parts": parts, "total": total, "go": go}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--pins", type=int)
    parser.add_argument("--baseline", help="Reuse this unpinned baseline JSON (same binary and parts)")
    parser.add_argument("--require-go", action="store_true", help="Exit 1 if any part fails the pin bar")
    parser.add_argument("--require-no-repeated-span", action="store_true", help="Exit 1 if any placed linear span repeats")
    parser.add_argument("dw")
    parser.add_argument("out")
    parser.add_argument("files", nargs="+")
    args = parser.parse_args()
    if (args.seed is None) != (args.pins is None) or (args.pins is not None and args.pins < 1):
        parser.error("provide both --seed and --pins (N >= 1)")
    os.makedirs(args.out, exist_ok=True)
    if args.baseline:
        with open(args.baseline) as source:
            res = json.load(source)
        if set(res) != {os.path.basename(f).split(".")[0] for f in args.files}:
            parser.error("baseline parts must equal the requested parts")
    else:
        with ThreadPoolExecutor(4) as ex:
            res = dict(ex.map(lambda f: one(f, args.dw, args.out), args.files))
    with open(f"{args.out}/baseline.json", "w") as output:
        json.dump(res, output, indent=1, sort_keys=True)
    if args.pins is not None:
        with ThreadPoolExecutor(4) as ex:
            parts = dict(ex.map(lambda f: pinned_one(f, args, res), args.files))
        report = pin_report(parts, args.seed, args.pins)
        with open(f"{args.out}/random-pins.json", "w") as output:
            json.dump(report, output, indent=1, sort_keys=True)
        return int(any("error" in p for p in parts.values()) or (args.require_go and not report["go"]))
    table(res)
    return int(any("error" in v for v in res.values())
               or (args.require_no_repeated_span and any(v['repeated_span'] for v in res.values())))


if __name__ == "__main__":
    sys.exit(main())
