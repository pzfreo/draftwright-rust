"""H7 acceptance: compare complete thirty-part quality runs and interior ink proofs.

Usage: python3 -I tools/check_interior.py [--strict] BEFORE_QUALITY_DIR AFTER_QUALITY_DIR

Always enforced: no part worse on any quality column, every interior item drawn and free of
ink findings, and more interior labels than before. The remaining acceptance goals (linear
dimensions inside views, fewer outer-band labels, fewer sheet escalations) are reported under
"goals"; --strict enforces them too.
"""
import json
import pathlib
import sys

# Recompute new columns from saved plans, including baselines predating H7.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from quality import interior_labels, sheet_escalations


def placement_counts(directory, name):
    plan = json.loads((directory / f'{name}.plan.json').read_text())
    inside = interior_labels(plan)
    return {'interior_labels': inside,
            'outer_labels': sum(d.get('placed') is not None for d in plan['dimensions']) - inside,
            'sheet_escalations': sheet_escalations(plan)}


def check(before_dir, after_dir, strict=False):
    before = json.loads((before_dir / 'baseline.json').read_text())
    after = json.loads((after_dir / 'baseline.json').read_text())
    assert len(before) == 30 and set(before) == set(after), 'require all thirty parts'
    metrics = ('off_sheet', 'repeated_label', 'repeated_measure', 'views_overlap',
               'view_on_title_block', 'ink', 'repeated_span', 'line_through_feature',
               'hidden_on_visible', 'section_hidden', 'unplanned_features')
    totals = {'interior_labels': [0, 0], 'outer_labels': [0, 0], 'sheet_escalations': [0, 0]}
    kinds = set()
    for name, b in before.items():
        a = after[name]
        assert 'error' not in a and 'error' not in b, name
        for key in metrics:
            assert a[key] <= b[key], f'{name}: {key} {b[key]} -> {a[key]}'
        assert a['faces'] == b['faces'] and a['claimed_faces'] >= b['claimed_faces'], name
        bc = placement_counts(before_dir, name)
        ac = placement_counts(after_dir, name)
        for key in totals:
            totals[key][0] += bc[key]
            totals[key][1] += ac[key]
        plan = json.loads((after_dir / f'{name}.plan.json').read_text())
        for d in plan['dimensions']:
            if d.get('region') != 'interior':
                continue
            assert d.get('placed') is not None, f'{name}: undrawn interior {d["id"]}'
            kinds.add(d['kind'])
            assert not any(d['id'] in p.get('subjects', []) and p['code'] in {
                'annotation_overlap', 'annotation_ink_overlap', 'annotation_over_view',
                'annotation_line_over_view', 'annotation_over_title_block', 'annotation_off_sheet',
                'leader_through_feature', 'extension_through_feature', 'annotation_leader_crossing',
            } for p in plan['problems']), f'{name}: interior ink conflict {d["id"]}'
    assert totals['interior_labels'][1] > totals['interior_labels'][0], totals
    goals = {
        'callout_and_linear_inside': 'callout' in kinds and 'linear' in kinds,
        'fewer_outer_labels': totals['outer_labels'][1] < totals['outer_labels'][0],
        'fewer_sheet_escalations':
            totals['sheet_escalations'][1] < totals['sheet_escalations'][0],
    }
    print(json.dumps({'parts': len(after), 'totals': totals, 'kinds': sorted(kinds),
                      'goals': goals}, indent=2))
    if strict:
        assert all(goals.values()), goals
    return goals

if __name__ == '__main__':
    args = sys.argv[1:]
    strict = '--strict' in args
    args = [a for a in args if a != '--strict']
    check(pathlib.Path(args[0]), pathlib.Path(args[1]), strict)
