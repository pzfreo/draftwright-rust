"""H7 acceptance: compare complete thirty-part quality runs and interior ink proofs.

Usage: python3 -I tools/check_interior.py BEFORE_QUALITY_DIR AFTER_QUALITY_DIR
"""
import json
import pathlib
import sys


def check(before_dir, after_dir):
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
        for key in totals:
            totals[key][0] += b[key]
            totals[key][1] += a[key]
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
    assert 'callout' in kinds and 'linear' in kinds, kinds
    assert totals['outer_labels'][1] < totals['outer_labels'][0], totals
    assert totals['sheet_escalations'][1] < totals['sheet_escalations'][0], totals
    print(json.dumps({'parts': len(after), 'totals': totals, 'kinds': sorted(kinds)}, indent=2))


if __name__ == '__main__':
    check(pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]))
