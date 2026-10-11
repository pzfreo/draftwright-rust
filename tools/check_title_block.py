"""Check projection furniture and blank-tolerance reporting in quality.py exports.

Usage: python3 -I tools/check_title_block.py <quality directory>
"""
import json
import math
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

root = Path(sys.argv[1])
plans = sorted(root.glob('*.plan.json'))
assert plans, 'no rendered plans'
blank = 0
for path in plans:
    plan = json.loads(path.read_text())
    sheet = plan['sheet']
    symbol = sheet.get('projection_symbol')
    assert symbol and symbol['method'] == 'third-angle', f'{path.stem}: no third-angle symbol'
    views = {v['id']: v for v in plan['views']}
    if 'v:front' in views:
        for name, axis in [('v:plan', 1), ('v:side', 0)]:
            if name in views:
                assert views[name]['at'][axis] > views['v:front']['at'][axis], path.stem
    svg = ET.parse(path.with_name(path.name.replace('.plan.json', '.svg'))).getroot()
    group = next((g for g in svg.iter() if g.get('id') == 'projection_symbol'), None)
    assert group is not None, f'{path.stem}: planned symbol not rendered'
    assert float(group.get('stroke-width')) == 0.15
    lines = [e for e in group if e.tag.endswith('line')]
    circles = [e for e in group if e.tag.endswith('circle')]
    assert len(lines) == 5 and len(circles) == 2, path.stem
    ends = sorted((e for e in lines if e.get('x1') == e.get('x2')), key=lambda e: float(e.get('x1')))
    assert len(ends) == 2
    heights = [abs(float(e.get('y2')) - float(e.get('y1'))) for e in ends]
    assert math.isclose(heights[1] / heights[0], 0.6, abs_tol=1e-6), path.stem
    radii = [float(e.get('r')) for e in circles]
    assert math.isclose(radii[1] / radii[0], 0.6, abs_tol=1e-6)
    assert circles[0].get('cx') == circles[1].get('cx') and circles[0].get('cy') == circles[1].get('cy')
    block = sheet['title_block']
    band = [block[2] - 19, block[1] + 32, block[2] - 3, block[3]]
    half = 0.075
    for e in lines:
        for n in ['1', '2']:
            x, y = float(e.get('x' + n)), float(e.get('y' + n))
            assert band[0] <= x - half and x + half <= band[2] + 1e-6
            assert band[1] <= y - half and y + half <= band[3]
    for e, r in zip(circles, radii):
        x, y = float(e.get('cx')), float(e.get('cy'))
        assert band[0] <= x - r - half and x + r + half <= band[2] + 1e-6
        assert band[1] <= y - r - half and y + r + half <= band[3]
    tolerance = next(c for c in sheet['title_cells'] if c['field'] == 'general_tolerance')
    missing = [p for p in plan['problems'] if p['code'] == 'general_tolerance_missing']
    if not tolerance.get('value'):
        blank += 1
        assert len(missing) == 1 and missing[0]['severity'] == 'warning', path.stem
        assert missing[0]['subjects'] == ['sheet']
        assert missing[0]['fix'] == [{'op': 'set', 'path': 'sheet/title/tolerance', 'value': 'UNSPECIFIED'}]
    else:
        assert not missing, path.stem
print(f'{len(plans)} projection symbols: third angle, correct proportions, ink inside the band')
print(f'{blank} blank general-tolerance cells: {blank} warnings with explicit Spec fix ops')
