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
    pmi = {r['id'] for field in ('gdt', 'dimensions', 'requirements')
           for r in p.get(field, []) if r.get('placed') is not None
           and (field == 'gdt' or r['id'].startswith(('pmi:', 'd:pmi:'))
                or 'AP242' in r.get('basis', ''))}
    return p, pmi


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('before', type=Path)
    ap.add_argument('after', type=Path)
    ap.add_argument('out', type=Path)
    args = ap.parse_args()
    old = json.loads((args.before / 'summary.json').read_text())
    new = json.loads((args.after / 'summary.json').read_text())
    assert old.keys() == new.keys(), 'content manifests differ'
    report = {'parts': 0, 'changes': [], 'details': [], 'new_errors': [], 'lost_pmi': [],
              'small_to_large': [], 'refusals': []}
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
        pair = lambda plan: f"{plan['sheet']['size']} {plan['sheet']['scale']} / {1+len(plan.get('sheets', []))} sheet(s)"
        if pair(p) != pair(q):
            report['changes'].append({'part': name, 'before': pair(p), 'after': pair(q)})
        if row['details']:
            report['details'].append({'part': name, 'sheet': pair(q), 'details': row['details']})
        errors = lambda plan: Counter(error_key(r) for r in plan['problems'] if r['severity'] == 'error')
        added = errors(q) - errors(p)
        if added:
            report['new_errors'].append({'part': name, 'errors': [list(k) for k in added.elements()]})
        if pm - qm:
            report['lost_pmi'].append({'part': name, 'ids': sorted(pm-qm)})
        if p['sheet']['size'] in ('A4', 'A3') and q['sheet']['size'] in ('A2', 'A1', 'A0'):
            report['small_to_large'].append({'part': name, 'before': pair(p), 'after': pair(q)})
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / 'comparison.json').write_text(json.dumps(report, indent=2) + '\n')
    with (args.out / 'sheet-changes.csv').open('w') as f:
        w = csv.DictWriter(f, ['part', 'before', 'after']);w.writeheader();w.writerows(report['changes'])
    print(json.dumps({k: len(v) if isinstance(v, list) else v for k, v in report.items()}, indent=2))
    assert not report['new_errors'], report['new_errors']
    assert not report['lost_pmi'], report['lost_pmi']
    assert not report['small_to_large'], report['small_to_large']
    assert all(r['before'] is not None and r['after'] is not None for r in report['refusals'])


if __name__ == '__main__':
    main()
