"""G3 acceptance: no quality cell grows, and every recognised blend has radius evidence.

Usage: python3 -I tools/check_blend_quality.py BINARY BEFORE_DIR AFTER_DIR CORPUS
Reads quality plans/baselines; native reviews independently check rendered coverage.
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
    features = {s for p in old["problems"] if p["code"] == "dimensions_not_ported"
                for s in p.get("subjects", []) if s.startswith("f:blend:")}
    if not features:
        continue
    plan = json.loads((Path(after_dir) / f"{name}.plan.json").read_text())
    review = json.loads(subprocess.check_output([binary, "review", str(Path(corpus) / relative)]))
    missing = {s for p in review["lint"] if p["code"] == "blend_requirement_missing"
               for s in p["subjects"]}
    unverified = {s for p in plan["problems"] if p["code"] == "feature_not_dimensioned"
                  for s in p.get("subjects", []) if s.startswith("f:blend:")}
    assert missing <= unverified, (name, "unstated blends without an explicit refusal", sorted(missing - unverified))
    if name in {"cgb245", "nist_ctc_01_asme1_rd", "string_post"}:
        assert not missing, (name, "required radius coverage", sorted(missing))
    notes = [n for n in plan.get("notes", []) if n["text"].startswith("ALL UNSPECIFIED BLEND RADII")
             or "OTHER BLEND RADII UNSTATED" in n["text"]]
    if missing:
        assert not any(n["text"].startswith("ALL UNSPECIFIED BLEND RADII") for n in notes), (name, "unresolved exceptions misread as the default radius")
    if len(features) >= 20:
        assert notes and any(n.get("placed") for n in notes), (name, "missing general radius note")
    rows.append({"part": name, "before": len(features), "after": len(missing),
                 "general_notes": len(notes), "refused": sorted(unverified)})

print("part | blend_requirement_missing before | after | general radius notes")
for r in rows:
    print(f"{r['part']} | {r['before']} | {r['after']} | {r['general_notes']}")
print(f"TOTAL | {sum(r['before'] for r in rows)} | {sum(r['after'] for r in rows)}")
Path(after_dir, "blend-coverage.json").write_text(json.dumps(rows, indent=2) + "\n")
print(f"{len(after)} parts / {len(after) * len(keys)} quality cells no worse; blend evidence checked")
