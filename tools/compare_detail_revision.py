"""Compare detail_revision_survey outputs without changing any acceptance baseline."""
import argparse
from collections import Counter
import csv
import json
from pathlib import Path


def error_key(row):
    return row['code'], tuple(row.get('subjects', [])), tuple(row.get('refs', []))


def snapshot(directory, key):
    p = json.loads((directory / (key + '.plan.json')).read_text())
    if 'sheet' not in p:
        return None
    pmi = {r['id']: {k: r[k] for k in ('label', 'cells', 'value', 'measurements') if k in r}
           for field in ('gdt', 'dimensions', 'requirements')
           for r in p.get(field, []) if r.get('placed') is not None
           and (field == 'gdt' or r['id'].startswith(('pmi:', 'd:pmi:'))
                or 'AP242' in r.get('basis', ''))}
    return p, pmi


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('before', type=Path)
    ap.add_argument('after', type=Path)
    ap.add_argument('out', type=Path)
    ap.add_argument('--dimension-labels', action='store_true',
                    help='Compare printed controlling labels and account for removed references')
    args = ap.parse_args()
    old = json.loads((args.before / 'summary.json').read_text())
    new = json.loads((args.after / 'summary.json').read_text())
    assert old.keys() == new.keys(), 'content manifests differ'
    label_report = {'parts': 0, 'changed': [], 'lost_controlling': [], 'added_controlling': [], 'remaining_references': [], 'unaccounted_references': []}
    report = {'parts': 0, 'changes': [], 'details': [], 'new_errors': [], 'lost_pmi': [],
              'small_to_large': [], 'refusals': [], 'changed_pmi': []}
    for key, row in new.items():
        name = row['name']
        a, b = snapshot(args.before, key), snapshot(args.after, key)
        if a is None or b is None:
            report['refusals'].append({'part': name, 'before': old[key].get('refused'),
                                       'after': row.get('refused')})
            continue
        report['parts'] += 1
        p, pm = a
        q, qm = b
        if args.dimension_labels:
            label_report['parts'] += 1
            def labels(plan, reference):
                return Counter((d['id'], d['label']) for d in plan['dimensions']
                               if d.get('placed') is not None
                               and bool(d.get('reference')) == reference)
            old_control, new_control = labels(p, False), labels(q, False)
            lost, added = old_control-new_control, new_control-old_control
            for field, delta in [('lost_controlling', lost), ('added_controlling', added)]:
                if delta:
                    label_report[field].append({'part': name, 'labels': list(delta.elements())})
            removed = labels(p, True)-labels(q, True)
            if labels(q, True):
                label_report['remaining_references'].append({'part': name, 'labels': list(labels(q, True).elements())})
            omitted = {d['id']: d for d in q.get('omitted', [])}
            drawn = {d['id'] for d in q['dimensions'] if d.get('placed') is not None}
            for d in p['dimensions']:
                if (d['id'], d['label']) not in removed:
                    continue
                o = omitted.get(d['id'], {})
                state = o.get('state', {})
                if (state.get('kind') != 'covered' or state.get('by') not in drawn
                        or any(by not in o.get('basis', '') for by in d.get('determined_by', []))):
                    label_report['unaccounted_references'].append({'part': name, 'id': d['id'], 'omission': o})
            extra = labels(q, True)-labels(p, True)
            if removed or extra or lost or added:
                label_report['changed'].append({'part': name, 'references_removed': sum(removed.values()),
                    'removed': list(removed.elements()), 'added_references': list(extra.elements())})
        pair = lambda plan: f"{plan['sheet']['size']} {plan['sheet']['scale']} / {1+len(plan.get('sheets', []))} sheet(s)"
        if pair(p) != pair(q):
            report['changes'].append({'part': name, 'before': pair(p), 'after': pair(q)})
        if row['details']:
            report['details'].append({'part': name, 'sheet': pair(q), 'details': row['details']})
        errors = lambda plan: Counter(error_key(r) for r in plan['problems'] if r['severity'] == 'error')
        added = errors(q) - errors(p)
        if added:
            report['new_errors'].append({'part': name, 'errors': [list(k) for k in added.elements()]})
        if pm.keys() - qm.keys():
            report['lost_pmi'].append({'part': name, 'ids': sorted(pm.keys()-qm.keys())})
        changed = [k for k in pm.keys() & qm.keys() if pm[k] != qm[k]]
        if changed:
            report['changed_pmi'].append({'part': name, 'ids': sorted(changed)})
        if p['sheet']['size'] in ('A4', 'A3') and q['sheet']['size'] in ('A2', 'A1', 'A0'):
            report['small_to_large'].append({'part': name, 'before': pair(p), 'after': pair(q)})
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / 'comparison.json').write_text(json.dumps(report, indent=2) + '\n')
    with (args.out / 'sheet-changes.csv').open('w') as f:
        w = csv.DictWriter(f, ['part', 'before', 'after']);w.writeheader();w.writerows(report['changes'])
    print(json.dumps({k: len(v) if isinstance(v, list) else v for k, v in report.items()}, indent=2))
    if args.dimension_labels:
        (args.out / 'dimension-labels.json').write_text(json.dumps(label_report, indent=2) + '\n')
        print(json.dumps({k: len(v) if isinstance(v, list) else v
                          for k, v in label_report.items()}, indent=2))
        for field in ('lost_controlling', 'added_controlling', 'remaining_references', 'unaccounted_references'):
            assert not label_report[field], label_report[field]
        assert all(r['before'] is not None and r['after'] is not None for r in report['refusals'])
        return

    assert not report['new_errors'], report['new_errors']
    assert not report['lost_pmi'], report['lost_pmi']
    assert not report['changed_pmi'], report['changed_pmi']
    assert not report['small_to_large'], report['small_to_large']
    assert all(r['before'] is not None and r['after'] is not None for r in report['refusals'])


if __name__ == '__main__':
    main()
