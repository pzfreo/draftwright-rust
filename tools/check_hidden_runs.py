"""Reject touching short straight hidden paths that restart the SVG dash phase.

Usage: python3 -I tools/check_hidden_runs.py <quality output directory>
Only principal views are checked; isometric policy is unchanged. Counts paths,
including straight M/L polylines, rather than the number of commands in a path.
"""
import collections
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

out = Path(sys.argv[1])
failures = []
short = total = views = 0
for name in sorted(json.loads((out / 'baseline.json').read_text())):
    plan = json.loads((out / f'{name}.plan.json').read_text())
    views += sum(v['id'] != 'v:iso' for v in plan['views'])
    root = ET.parse(out / f'{name}.svg').getroot()
    hidden = next(g for g in root.iter() if g.get('id') == 'hidden')
    dash = float(re.split(r'[ ,]+', hidden.get('stroke-dasharray'))[0])
    runs = []
    for element in hidden:
        tag = element.tag.rsplit('}', 1)[-1]
        if tag == 'line':
            points = [[float(element.get(f'{k}{i}')) for k in ('x', 'y')] for i in (1, 2)]
        elif tag == 'path':
            path = element.get('d')
            if re.search(r'[A-KN-Za-kn-z]', path.replace('e', '').replace('E', '')):
                continue
            values = list(map(float, re.findall(r'[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?', path)))
            points = [values[i:i+2] for i in range(0, len(values), 2)]
        else:
            continue
        a, b = points[0], points[-1]
        length = math.dist(a, b)
        if length < 1e-9:
            continue
        v = [(b[k] - a[k]) / length for k in (0, 1)]
        if any(abs((p[0]-a[0])*v[1] - (p[1]-a[1])*v[0]) > 1e-5 for p in points):
            continue
        middle = [(a[k]+b[k])/2 for k in (0, 1)]
        view = next((v for v in plan['views'] if v.get('area') and
                     v['area'][0] <= middle[0] <= v['area'][2] and
                     v['area'][1] <= middle[1] <= v['area'][3]), None)
        if not view or view['id'] == 'v:iso':
            continue
        runs.append((a, b, v, length, view['id']))
        total += 1
        short += length < dash
    grid = collections.defaultdict(list)
    for i, (a, b, v, length, view) in enumerate(runs):
        for p in (a, b):
            cell = tuple(math.floor(k / .02) for k in p)
            for x in range(cell[0]-1, cell[0]+2):
                for y in range(cell[1]-1, cell[1]+2):
                    for j, q in grid[(view, x, y)]:
                        c, d, w, other_length, _ = runs[j]
                        if j == i or min(length, other_length) >= dash or math.dist(p, q) > .0081:
                            continue
                        if abs(v[0]*w[1]-v[1]*w[0]) > 1e-6:
                            continue
                        if abs((c[0]-a[0])*v[1]-(c[1]-a[1])*v[0]) > 1e-5:
                            continue
                        failures.append(f'{name} {view}: touching short paths {j}/{i}, gap {math.dist(p,q):.6f} mm')
            grid[(view, *cell)].append((i, p))
print(f'{views} principal views; {total} straight paths, {short} shorter than one dash; {len(failures)} touching short-path pairs', flush=True)
assert not failures, '\n'.join(failures[:50]) + f'\n{len(failures)} touching short hidden paths'
