"""H3 acceptance: unchanged GD&T/content, less ink and less detached dimension text.

Usage: python3 -I tools/check_label_lanes.py before-dir after-dir
Both directories contain quality.py baseline.json and stc09.plan.json.
"""
import itertools
import json
from pathlib import Path
import sys

def shared_dimension_lines(plan):
    linear = [d for d in plan['dimensions'] if d['kind'] == 'linear'
              and d.get('placed') and len(d['placed']['arrows']) == 2]
    pairs = set()
    for a, b in itertools.combinations(linear, 2):
        p, q = a['placed'], b['placed']
        k = int(p['rotation'] != 0)
        if a['view'] != b['view'] or p['rotation'] != q['rotation']:
            continue
        if abs(p['arrows'][0][0][1-k] - q['arrows'][0][0][1-k]) >= 1e-6:
            continue
        lo, hi = sorted(x[0][k] for x in p['arrows'])
        low, high = sorted(x[0][k] for x in q['arrows'])
        if min(hi, high) - max(lo, low) > 1e-6:
            pairs.add(tuple(sorted([a['id'], b['id']])))
    return pairs


before, after = map(Path, sys.argv[1:])
a, b = (json.loads((p / 'baseline.json').read_text()) for p in (before, after))
assert len(a) == len(b) == 30 and a.keys() == b.keys(), 'incomplete quality set'
keys = ['off_sheet', 'repeated_label', 'repeated_measure', 'views_overlap',
        'view_on_title_block', 'ink', 'repeated_span', 'line_through_feature',
        'hidden_on_visible', 'section_hidden', 'unplanned_features', 'text_off_span']
for part in a:
    assert 'error' not in a[part] and 'error' not in b[part], part
    for key in keys:
        assert b[part][key] <= a[part][key], (part, key, a[part][key], b[part][key])
    assert b[part]['faces'] == a[part]['faces'], part
    assert b[part]['claimed_faces'] >= a[part]['claimed_faces'], part
    plans = [json.loads((p / f'{part}.plan.json').read_text()) for p in (before, after)]
    # Measurements survive lane changes, including a different recovered sheet choice.
    measurements = [sorted((m['feature'], m['parameter'], m['value'])
                           for d in plan['dimensions'] for m in d['measurements'])
                    for plan in plans]
    assert measurements[0] == measurements[1], (part, 'measurements changed')
    assert shared_dimension_lines(plans[1]) <= shared_dimension_lines(plans[0]), (part, 'new shared dimension line')
for key in ['ink', 'text_off_span']:
    old, new = (sum(m[key] for m in x.values()) for x in (a, b))
    assert new < old, (key, old, new, 'no improvement')
    print(f'{key}: {old} -> {new}')
plans = [json.loads((p / 'stc09.plan.json').read_text()) for p in (before, after)]
assert plans[0]['gdt'] and plans[0]['gdt'] == plans[1]['gdt'], 'stc09 GD&T changed'
assert plans[0]['sheet'] == plans[1]['sheet'], 'stc09 sheet changed'
assert plans[0]['views'] == plans[1]['views'], 'stc09 views changed'
print('PASS: 30 parts no worse, measurements preserved, stc09 GD&T exactly unchanged')
