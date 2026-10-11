"""Fail if any thin-slice quality part is worse than the latest status table.

Usage: python3 -I tools/quality_gate.py <status.md> <quality/baseline.json>
"""
import json, pathlib, re, sys
status = pathlib.Path(sys.argv[1]).read_text()
table = status[status.rfind('part                            off_sheet'):].split('```')[0]
keys = ['off_sheet','repeated_label','repeated_measure','views_overlap','view_on_title_block','ink']
expected = {}
for row in table.splitlines()[1:]:
    m = re.fullmatch(r'(\S+)\s+((?:\d+\s+){5}\d+)\s*', row)
    if m and m[1] != 'TOTAL': expected[m[1]] = dict(zip(keys, map(int, m[2].split())))
actual = json.loads(pathlib.Path(sys.argv[2]).read_text())
assert len(expected) == len(actual) == 30, (len(expected), len(actual))
for name, metrics in actual.items():
    assert 'error' not in metrics, (name, metrics)
    matches = [prefix for prefix in expected if name == prefix or name[:28] == prefix]
    assert len(matches) == 1, (name, matches)
    before = expected[matches[0]]
    for key in keys: assert metrics[key] <= before[key], (name, key, before[key], metrics[key])
print('PASS: all 30 parts are no worse on all six metrics than the latest status table.')
print(json.dumps({key: sum(m[key] for m in actual.values()) for key in keys},sort_keys=True))
