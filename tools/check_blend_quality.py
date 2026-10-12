"""G3 acceptance: no quality cell grows and unlocated radii never receive false credit.

Usage: python3 -I tools/check_blend_quality.py BINARY BEFORE_DIR AFTER_DIR CORPUS
Reads quality plans/baselines; native reviews independently check rendered coverage.
Radius-centre tables locate every occurrence; shallow relief grants no radius credit.
"""
import json
from pathlib import Path
import subprocess
import sys

binary, before_dir, after_dir, corpus = sys.argv[1:]
before = json.loads((Path(before_dir) / "baseline.json").read_text())
after = json.loads((Path(after_dir) / "baseline.json").read_text())
keys = ("off_sheet", "repeated_label", "repeated_measure", "views_overlap",
        "view_on_title_block", "ink", "repeated_span", "line_through_feature",
        "hidden_on_visible", "section_hidden")
assert before.keys() == after.keys()
for name in before:
    for key in keys:
        assert after[name][key] <= before[name][key], (name, key, before[name][key], after[name][key])

rows = []
for relative in Path("tools/quality_parts.txt").read_text().splitlines():
    name = Path(relative).stem
    old = json.loads((Path(before_dir) / f"{name}.plan.json").read_text())
    features = {m["feature"] for d in old.get("dimensions", []) for m in d.get("measurements", [])
                if m["parameter"] == "blend.radius"}
    features.update(m["feature"] for n in old.get("notes", []) for m in n.get("measurements", [])
                    if m["parameter"] == "blend.radius")
    features.update(s for p in old["problems"] for s in p.get("subjects", []) if s.startswith("f:blend:"))
    if not features:
        continue
    plan = json.loads((Path(after_dir) / f"{name}.plan.json").read_text())
    review = json.loads(subprocess.check_output([binary, "review", str(Path(corpus) / relative)]))
    missing = {s for p in review["lint"] if p["code"] == "blend_requirement_missing"
               for s in p["subjects"]}
    notes = [n for n in plan.get("notes", []) if n.get("measurements")]
    assert all("REF SITE" not in n["text"] and "UNSTATED" not in n["text"] for n in notes), (name, "unlocated note")
    leaders = [d for d in plan["dimensions"] if d.get("placed")
               and any(m["parameter"] in {"blend.radius", "fillet.radius"} for m in d.get("measurements", []))]
    for note in notes:
        if not note.get("placed"):
            continue
        assert note["text"].startswith("ALL UNSPECIFIED RADII R"), (name, "not a true default")
        value = note["measurements"][0]["value"]
        assert all(abs(d["value"] - value) >= 0.05 for d in leaders), (name, "default duplicated by leader")
        assert all(m["feature"] not in missing for m in note["measurements"]), (name, "invalid default credited")
        cells = note["placed"]["box"]
        assert cells[2] <= plan["sheet"]["page"][0] - 16 and cells[3] <= plan["sheet"]["page"][1] - 16, (name, "frame clearance")
    if name in {"cgb245", "string_post"}:
        assert not missing, (name, "required radius coverage", sorted(missing))
    if name == "nist_ctc_01_asme1_rd":
        assert not missing, (name, "real corners and default must be stated")
        assert any(n.get("placed") and n["text"] == "ALL UNSPECIFIED RADII R10" for n in notes), (name, "R10 default")
        table = plan.get("radius_table", {})
        assert table.get("table", {}).get("placed"), (name, "located radius table")
        assert sum(r["radius"] == 25 for r in table["rows"]) == 4, (name, "four real R25 corners")
    analysis = json.loads(subprocess.check_output([binary, "analyse", str(Path(corpus) / relative)]))
    relief = [f for f in analysis["features"] if f["kind"] in {"blend", "fillet"} and f["faces"]
              and any(set(f["faces"]) <= set(r["faces"]) for r in analysis.get("radius_reliefs", []))]
    claimed = {m["feature"] for d in plan["dimensions"] for m in d.get("measurements", [])}
    claimed.update(m["feature"] for n in plan.get("notes", []) for m in n.get("measurements", []))
    assert all(f["id"] not in claimed for f in relief), (name, "relief must not receive radius claims")
    if name == "nist_ftc_06_asme1_rd":
        assert len(relief) >= 4, (name, "lettering relief")
    table = plan.get("radius_table")
    if table:
        assert table["table"].get("placed"), (name, "radius table drawn")
        covered_dimensions = {d["id"] for d in plan["omitted"]
                              if d.get("state") == {"kind": "covered", "by": "t:radius-centres"}}
        covered = {r["feature"] for r in table["rows"] if r["dimension"] in covered_dimensions}
        located = [r["feature"] for r in table["rows"]]
        assert len(located) == len(set(located)) and set(located) == covered, (name, "one locator per occurrence")
        assert not covered & missing, (name, "radius table not credited")
    unverified = {s for problem in plan["problems"] if problem["code"] == "feature_not_dimensioned"
                  for s in problem.get("subjects", [])}
    unverified.update(m["feature"] for d in plan["dimensions"]
                      if d.get("state", {}).get("kind") == "not_drawable"
                      for m in d.get("measurements", []) if m["parameter"] == "blend.radius")
    assert missing <= unverified, (name, "missing radii without an explicit refusal", sorted(missing - unverified))
    rows.append({"part": name, "before": len(features), "after": len(missing),
                 "general_notes": sum(bool(n.get("placed")) for n in notes),
                 "relief_excluded": sum(f["kind"] == "blend" for f in relief),
                 "radius_table_rows": len(table["rows"]) if table else 0, "refused": sorted(unverified)})

print("part | blend_requirement_missing before | after | general radius notes")
for r in rows:
    print(f"{r['part']} | {r['before']} | {r['after']} | {r['general_notes']}")
print(f"TOTAL | {sum(r['before'] for r in rows)} | {sum(r['after'] for r in rows)}")
Path(after_dir, "blend-coverage.json").write_text(json.dumps(rows, indent=2) + "\n")
print(f"{len(after)} parts / {len(after) * len(keys)} quality cells no worse; blend evidence checked")
