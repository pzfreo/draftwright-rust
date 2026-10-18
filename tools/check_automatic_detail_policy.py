#!/usr/bin/env python3
"""Independently solve each surveyed automatic-detail drawing with the switch off."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('survey', type=Path)
args = parser.parse_args()
r = args.survey
manifest = json.loads((r / 'after/manifest.json').read_text())
out = r / 'ordinary'
out.mkdir(exist_ok=True)
spec = out / 'no-automatic-details.spec.json'
spec.write_text(json.dumps({'sheet': {'automatic_details': False}}) + '\n')
binary = (r / 'after/draftwright').resolve()
papers = ['A4', 'A3', 'A2', 'A1', 'A0']
receipts = []
for key, source in manifest.items():
    plan = json.loads((r / 'after' / (key + '.plan.json')).read_text())
    if 'sheet' not in plan:
        continue
    automatic = [v for v in plan['views'] if v.get('detail') and not v['detail']['authored']]
    if not automatic:
        continue
    reply = subprocess.run([str(binary), 'plan', source['path'], '--spec', str(spec)], capture_output=True, check=True, timeout=3600)
    (out / (key + '.plan.json')).write_bytes(reply.stdout)
    ordinary = json.loads(reply.stdout)
    assert len(automatic) <= 2, source['name']
    assert len(plan.get('sheets', [])) <= len(ordinary.get('sheets', [])), source['name']
    assert papers.index(plan['sheet']['size']) <= papers.index(ordinary['sheet']['size']), source['name']
    assert plan['sheet']['scale_factor'] >= ordinary['sheet']['scale_factor'], source['name']
    for view in automatic:
        detail = view['detail']
        assert detail['bounds'] == detail['marked_bounds'], source['name']
        parent = next(v for v in plan['views'] if v['id'] == detail['source_view'])
        assert view.get('sheet', 1) == parent.get('sheet', 1), source['name']
        assert any(d['view'] == view['id'] and d.get('placed') and not d.get('reference') and any(
            old['id'] == d['id'] and old['view'] == detail['source_view'] and old.get('placed') and not old.get('reference')
            for old in ordinary['dimensions']) for d in plan['dimensions']), source['name']
    receipts.append({'part': source['name'], 'details': len(automatic), 'ordinary': [ordinary['sheet']['size'], ordinary['sheet']['scale']], 'after': [plan['sheet']['size'], plan['sheet']['scale']]})
    print(source['name'], 'PASS', flush=True)
(r / 'automatic-policy-audit.json').write_text(json.dumps({'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'drawings': receipts}, indent=2) + '\n')
