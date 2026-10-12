"""Check H1 drawing evidence after quality.py; fails on the reviewed four-column schedules.

Usage: python3 -I tools/check_hole_schedules.py <quality output directory>
"""
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
metrics = json.loads((out / 'baseline.json').read_text())
required = {'nist_ctc_02_asme1_rc', 'nist_ctc_04_asme1_rd', 'nist_ftc_06_asme1_rd'}
changed = []
for name in sorted(metrics):
    plan = json.loads((out / (name + '.plan.json')).read_text())
    tables = plan.get('hole_tables', [])
    assert tables or name not in required, f'{name}: dense acceptance part has no schedule'
    if not tables:
        continue
    changed.append(name)
    assert metrics[name].get('schedule_ink') == 0, f'{name}: new schedule ink collides'
    for table in tables:
        for row in table['rows']:
            assert len(row['cells']) >= 5 and '/' in row['cells'][4], f'{name}: missing entry distinction'
            assert all(m['value'] >= -1e-6 for m in row['measurements']
                       if m['parameter'].startswith('location.length.')), f'{name}: negative ordinate'
            markers = [b for b in plan['balloons'] if row['feature'] in b['features'] and b['certifying']]
            assert len(markers) == 1 and row['tag'] in markers[0]['tag'].split('/'), f'{name}: missing/ambiguous tag'
            assert markers[0].get('label_box'), f'{name}: unmeasured tag ink'
            for other in table['rows']:
                if other['feature'] != row['feature'] and other['cells'][2:4] == row['cells'][2:4]:
                    assert row['cells'][4] != other['cells'][4], f'{name}: indistinguishable stacked openings'
    if name == 'nist_ctc_05_asme1_rd':
        table = next(t for t in tables if t['view'] == 'v:plan')
        circle = table['pcd']
        assert abs(circle['diameter'] - 444.5) < 1e-4 and len(circle['members']) == 10
        angles = sorted(float(r['cells'][5].removesuffix('°')) for r in table['rows']
                        if r['feature'] in circle['members'])
        assert angles == [30, 60, 90, 120, 150, 210, 240, 270, 300, 330], 'false angular positions'
        assert 'PCD' in table['blocks'][0]['rows'][0][1]
print(f'{len(changed)} scheduled parts: positive coordinates, entry distinctions, measured clear tags; CTC-05 PCD/angles')
