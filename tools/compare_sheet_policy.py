"""Compare sheet_policy_audit runs; emit every previously small A2+ case as CSV and Markdown.

python3 -I tools/compare_sheet_policy.py BEFORE/audit.json AFTER/audit.json OUT
Explicit paper/count/scale intents are recorded alongside automatic overflow failures.
"""
import csv
import json
import re
from pathlib import Path
import sys


def main():
    before, after = (json.loads(Path(p).read_text()) for p in sys.argv[1:3])
    out = Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    if before.keys() != after.keys():
        raise ValueError('audit case sets differ')
    for case, new in after.items():
        old = before[case]
        if 'size' in old and 'size' not in new:
            raise ValueError(f'{case}: newly refused')
        if old.get('small') is False:
            for field in ('size', 'scale', 'sheets'):
                if old[field] != new.get(field):
                    raise ValueError(f'{case}: large part changed {field}')
        if (new.get('small') and new.get('size') in ('A2', 'A1', 'A0')
                and not new['spec'].get('sheet', {}).get('size')
                and new['spec'].get('sheets', {}).get('max') != 1
                and not all(f'{paper} multi-sheet:' in new['basis'] for paper in ('A4', 'A3'))):
            raise ValueError(f'{case}: large paper without small-sheet attempts')
    rows = []
    for case, old in before.items():
        if not old.get('small') or old.get('size') not in ('A2', 'A1', 'A0'):
            continue
        new = after[case]
        if 'size' not in new:
            raise ValueError(f'{case}: newly refused')
        spec = old['spec']
        reason = ''
        if new['size'] in ('A2', 'A1', 'A0'):
            if spec.get('sheet', {}).get('size'):
                reason = f"explicit sheet.size = {spec['sheet']['size']}"
            elif spec.get('sheets', {}).get('max') == 1:
                reason = 'explicit sheets.max = 1'
            else:
                reason = '; '.join(re.findall(r'A[43] multi-sheet:[^;]+', new['basis']))
                if 'multi-sheet:' not in reason:
                    raise ValueError(f'{case}: large paper without small multi-sheet attempt')
        rows.append({'case': case,
                     'before': f"{old['size']} {old['scale']} / {old['sheets']} sheet(s)",
                     'after': f"{new['size']} {new['scale']} / {new['sheets']} sheet(s)",
                     'reason': reason or 'valid complete small-sheet allocation'})
    with (out / 'small-large-cases.csv').open('w', newline='') as target:
        writer = csv.DictWriter(target, fieldnames=['case', 'before', 'after', 'reason'], lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)
    lines = [f"Audited {len(before)} default/spec cases. {len(rows)} were small on A2+ before.",
             '', '| Case | Before | After | Reason |', '|---|---|---|---|']
    lines += ['| ' + ' | '.join(v.replace('\n', ' ').replace('|', '\\|') for v in row.values()) + ' |'
              for row in rows]
    (out / 'small-large-cases.md').write_text('\n'.join(lines) + '\n')
    print(lines[0])
    for row in rows:
        print(row['case'], row['before'], '->', row['after'],
              row['reason'][:180])


if __name__ == '__main__':
    main()
