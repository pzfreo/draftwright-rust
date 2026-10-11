"""Fail if a quality.py baseline worsens any cell of the latest complete status table.

Usage: python3 -I tools/check_quality_table.py <status.md> <baseline.json>
"""
import json
import sys
from pathlib import Path

keys = ["off_sheet", "repeated_label", "repeated_measure", "views_overlap",
        "view_on_title_block", "ink", "repeated_span", "line_through_feature",
        "hidden_on_visible", "section_hidden"]
tables = []
rows = None
columns = []
for line in Path(sys.argv[1]).read_text().splitlines():
    fields = line.split()
    if fields and fields[0] == "part" and ({"ink", "off_sheet"} & set(fields[1:])):
        columns = [next((key for key in keys if key == name or key[:12] == name), name)
                   for name in fields[1:]]
        rows = {}
    elif rows is not None:
        if fields and fields[0] == "TOTAL":
            tables.append((columns, rows))
            rows = None
        elif len(fields) == len(columns) + 1 and all(n.isdecimal() for n in fields[1:]):
            rows[fields[0]] = dict(zip(columns, map(int, fields[1:])))

assert tables and tables[-1][1], "no complete quality table in status document"
columns, reference = tables[-1]
actual = json.loads(Path(sys.argv[2]).read_text())
assert len(actual) == len(reference), "part count differs from status table"
keys = [key for key in columns if all(key in m for m in actual.values())]
seen = set()
worse = []
totals = [0] * len(keys)
for part, measurements in sorted(actual.items()):
    assert "error" not in measurements, f"{part}: {measurements}"
    name = part[:28]
    assert name in reference and name not in seen, f"missing/ambiguous part: {part}"
    seen.add(name)
    for i, key in enumerate(keys):
        value = measurements[key]
        totals[i] += value
        if value > reference[name][key]:
            worse.append(f"{part}: {key} {reference[name][key]} -> {value}")
assert not worse, "quality regressed:\n" + "\n".join(worse)
print(f"{len(actual)} parts, {len(actual) * len(keys)} cells: no part worse")
print("Totals:", dict(zip(keys, totals)))
