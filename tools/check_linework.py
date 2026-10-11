"""Require zero hidden-over-visible ink in principal views and zero section hidden ink.

Usage: python3 -I tools/check_linework.py <quality output directory>
Isometrics keep the existing rendering policy and are reported separately.
"""
import importlib.util
import json
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location('quality', Path(__file__).with_name('quality.py'))
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)
out = Path(sys.argv[1])
parts = json.loads((out / 'baseline.json').read_text())
failures = []
isometric = 0
for name in sorted(parts):
    plan = json.loads((out / f'{name}.plan.json').read_text())
    principal = dict(plan, views=[v for v in plan['views'] if v['id'] != 'v:iso'])
    metrics = quality.svg_linework(out / f'{name}.svg', principal)
    for key, value in metrics.items():
        if value:
            failures.append(f'{name}: {key} = {value}')
    iso = dict(plan, views=[v for v in plan['views'] if v['id'] == 'v:iso'])
    isometric += quality.svg_linework(out / f'{name}.svg', iso)['hidden_on_visible']
assert not failures, 'linework defects:\n' + '\n'.join(failures)
print(f'{len(parts)} parts: principal hidden_on_visible = 0, section_hidden = 0')
print(f'Unchanged isometric hidden_on_visible = {isometric}')
