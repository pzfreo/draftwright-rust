"""Drawing quality for draftwright-rust: per part, annotations off the sheet, repeated dimensions,
overlapping views, and ink overlaps, from `draftwright plan` and `render`. Writes baseline.json
in <out dir> and prints a table with totals.

The maintainer's priority (2026-10-09) is to drive these to zero before adding features; see
docs/quality-2026-10-09.md for the baseline and the parts it is measured on.

Usage: python3 -I tools/quality.py <draftwright binary> <out dir> <step files...>
(the parts: tools/quality_parts.txt, paths relative to $DRAFTWRIGHT_CORPUS)"""
import collections, json, subprocess, sys, os
from concurrent.futures import ThreadPoolExecutor

dw, out, files = sys.argv[1], sys.argv[2], sys.argv[3:]
INK = {"annotation_ink_overlap", "annotation_over_view", "annotation_line_over_view",
       "annotation_leader_crossing", "annotation_outside_area"}


def overlap(a, b):
    return a[0] < b[2] and a[2] > b[0] and a[1] < b[3] and a[3] > b[1]


def one(f):
    name = os.path.basename(f).split(".")[0]
    r = subprocess.run([dw, "render", f, "-o", f"{out}/{name}", "--formats", "svg"], capture_output=True, text=True, timeout=600)
    p = subprocess.run([dw, "plan", f], capture_output=True, text=True, timeout=600)
    try:
        rend, plan = json.loads(r.stdout), json.loads(p.stdout)
    except Exception:
        return name, {"error": (r.stdout + r.stderr)[:200]}
    if "dimensions" not in plan:
        return name, {"error": json.dumps(plan.get("error", plan))[:200]}
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
    views = [v["area"] for v in plan["views"] if v.get("area")]
    tb = plan["sheet"].get("title_block")
    view_over = sum(overlap(views[i], views[j]) for i in range(len(views)) for j in range(i + 1, len(views)))
    view_tb = sum(overlap(v, tb) for v in views) if tb else 0
    return name, {
        "off_sheet": codes.get("annotation_off_sheet", 0),
        "repeated_measure": rep_meas, "repeated_drawn": rep_span, "repeated_label": rep_label,
        "views_overlap": view_over, "view_on_title_block": view_tb,
        "ink": sum(codes[c] for c in INK), "ink_by_code": {c: codes[c] for c in INK if codes[c]},
        "dimensions": len(plan["dimensions"]),
    }


with ThreadPoolExecutor(4) as ex:
    res = dict(ex.map(one, files))
json.dump(res, open(f"{out}/baseline.json", "w"), indent=1, sort_keys=True)
keys = ["off_sheet", "repeated_label", "repeated_measure", "views_overlap", "view_on_title_block", "ink"]
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
