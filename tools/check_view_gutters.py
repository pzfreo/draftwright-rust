#!/usr/bin/env python3
"""Compare quality.py plans: sheet/view scales must match; report blank band gaps.

Usage: python3 tools/check_view_gutters.py before-dir after-dir output.json
The iso's empty fitting region is normalized to its drawn outline and caption in
both runs. Other views use their reserved annotation bands. Gaps are the signed
width of the widest separating horizontal/vertical strip, in paper mm.
"""
import itertools
import json
from pathlib import Path
import sys

PREFERRED = 25.0


def bands(plan, analysis):
    result = {}
    for view in plan['views']:
        box = list(view['area'])
        if view['id'] == 'v:iso':
            box = [view['at'][k % 2] + view['scale'] * x
                   for k, x in enumerate(analysis['iso_bounds'])]
            for caption in plan.get('captions', []):
                if caption['view'] == view['id']:
                    ink = caption['ink']
                    box = [min(box[0], ink[0]), min(box[1], ink[1]),
                           max(box[2], ink[2]), max(box[3], ink[3])]
        result[view['id']] = (view.get('sheet', 1), box)
    return result


def minimum(plan, analysis):
    pairs = []
    for (a, (sa, x)), (b, (sb, y)) in itertools.combinations(bands(plan, analysis).items(), 2):
        if sa == sb:
            gap = max(x[0] - y[2], y[0] - x[2], x[1] - y[3], y[1] - x[3])
            pairs.append((gap, a, b))
    return min(pairs) if pairs else (None, None, None)


def compare(before, after):
    rows = []
    root = Path(__file__).resolve().parents[1]
    for part in (root / 'tools/quality_parts.txt').read_text().splitlines():
        name = Path(part).stem
        a, b = [json.loads((d / f'{name}.plan.json').read_text()) for d in (before, after)]
        analysis = json.loads((before / f'{name}.analysis.json').read_text())
        geometry = lambda p: [(s.get('number', 1), s['size'], s['scale']) for s in [p['sheet'], *p.get('sheets', [])]]
        assert geometry(a) == geometry(b), f'{part}: sheet/scale changed'
        assert [(v['id'], v['scale']) for v in a['views']] == [(v['id'], v['scale']) for v in b['views']], f'{part}: view scale/inventory changed'
        old, new = minimum(a, analysis), minimum(b, analysis)
        rows.append(dict(part=part, sheet=b['sheet']['size'], scale=b['sheet']['scale'],
                         before=old[0], after=new[0], limiting_pair=new[1:],
                         shrank=new[0] is not None and new[0] < PREFERRED - 1e-6))
    return rows


def main():
    before, after, output = map(Path, sys.argv[1:])
    rows = compare(before, after)
    output.write_text(json.dumps(rows, indent=2) + '\n')
    print('| Part | Sheet | Scale | Gap before → after (mm) |')
    print('| --- | --- | --- | ---: |')
    for row in rows:
        print(f'| {row["part"]} | {row["sheet"]} | {row["scale"]} | {row["before"]:.2f} → {row["after"]:.2f} |')
    print('\nAll 30 sheet/scale choices and all view scales/inventories unchanged.')
    for row in rows:
        if row['shrank']:
            print(f'{row["part"]}: {row["after"]:.3f} mm between {", ".join(row["limiting_pair"])}')


if __name__ == '__main__':
    main()
