"""Survey datum-bearing STEP files without changing fixtures or byte guards.

Collect both revisions with the same corpus and fixtures, then compare saved planner and
rendered-lint evidence. Identical STEP bytes are drawn once; inventory.json lists aliases.
The regression bar allows no increase in either crossings or overlaps on any part.
Crossings include leaders through recognized features; annotation-only counts are retained
separately so changes between collision types remain visible.

python3 -I tools/survey_datum_placement.py BASE_BINARY target/datum/base --corpus CORPUS
python3 -I tools/survey_datum_placement.py HEAD_BINARY target/datum/head --corpus CORPUS \
    --compare target/datum/base

--reuse reuses already saved plans/reviews, useful when extending a report after collection.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def write(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def inventory(corpus):
    files = sorted(p for directory in (corpus, ROOT / 'tests/fixtures')
                   for p in directory.rglob('*') if p.is_file()
                   and re.search(r'\.(step|stp)(\.gz)?$', p.name, re.I))
    unique, names, scanned = {}, set(), []
    for path in files:
        data = path.read_bytes()
        if data[:2] == b'\x1f\x8b':
            data = gzip.decompress(data)
        scanned.append(str(path))
        if not re.search(rb'\bDATUM_FEATURE\s*\(', data, re.I):
            continue
        digest = hashlib.sha256(data).hexdigest()
        if digest in unique:
            unique[digest]['aliases'].append(str(path))
            continue
        name = path.name.removesuffix('.gz').rsplit('.', 1)[0]
        if name in names:
            name = path.parent.name + '-' + name
        if name in names:
            raise ValueError(f'duplicate output name: {name}')
        names.add(name)
        unique[digest] = dict(name=name, path=str(path), aliases=[str(path)], sha256=digest)
    return dict(scanned=scanned, parts=list(unique.values()))


FEATURE_CROSSINGS = {'leader_through_feature', 'extension_through_feature',
                     'leader_crosses_silhouette'}
VIEW_CROSSINGS = {'annotation_line_over_view'}
INK_OVERLAPS = {'annotation_over_view', 'annotation_over_title_block',
                'leader_line_through_text'}


def crossing(code):
    return 'crossing' in code or code in FEATURE_CROSSINGS | VIEW_CROSSINGS


def overlapping(code):
    return 'overlap' in code or code in INK_OVERLAPS


def measure(plan, review):
    items = plan.get('gdt', [])
    ids = {g['id'] for g in items}
    datums = {g['id'] for g in items if g['kind'] == 'datum_feature'}
    findings = {(p['code'], tuple(sorted(p['subjects'])))
                for p in plan.get('problems', []) + review.get('structural', [])
                if (crossing(p['code']) or overlapping(p['code']))
                and ids.intersection(p.get('subjects', []))}
    return dict(datums=len(datums), gdt=len(ids),
                crossings=sum(crossing(c) for c, _ in findings),
                overlaps=sum(overlapping(c) for c, _ in findings),
                annotation_crossings=sum('crossing' in c for c, _ in findings),
                feature_crossings=sum(c in FEATURE_CROSSINGS for c, _ in findings),
                view_crossings=sum(c in VIEW_CROSSINGS for c, _ in findings),
                datum_crossings=sum(crossing(c) and bool(datums.intersection(s))
                                    for c, s in findings),
                datum_overlaps=sum(overlapping(c) and bool(datums.intersection(s))
                                   for c, s in findings),
                findings=sorted(findings), sheet=plan['sheet']['size'],
                scale=plan['sheet']['scale'])


def collect(row, args):
    name = row['name']
    try:
        documents = []
        for command, suffix in [('plan', 'plan'), ('review', 'review')]:
            path = args.out / f'{name}.{suffix}.json'
            if args.reuse and path.exists():
                value = json.loads(path.read_text())
            else:
                result = subprocess.run([str(args.binary.resolve()), command, row['path']],
                                        capture_output=True, text=True, timeout=args.timeout)
                if result.returncode:
                    raise ValueError(result.stderr or result.stdout)
                value = json.loads(result.stdout)
                write(path, value)
            documents.append(value)
        result = measure(*documents)
    except (ValueError, KeyError, subprocess.TimeoutExpired) as error:
        result = dict(error=str(error))
    print(f'{name}: {result.get("crossings", "ERROR")} crossings, '
          f'{result.get("overlaps", "ERROR")} overlaps', flush=True)
    return name, result


def compare(before, after):
    failures = []
    if set(before) != set(after):
        failures.append('part inventories differ')
    lines = ['| Part | Datums | Crossings base → head | Overlaps base → head |',
             '| --- | ---: | ---: | ---: |']
    total = [0, 0, 0, 0]
    for name in sorted(before.keys() | after.keys()):
        b, a = before.get(name, {}), after.get(name, {})
        if not a or not b or 'error' in a or 'error' in b:
            failures.append(f'{name}: {b.get("error", "")} {a.get("error", "")}')
            continue
        if a['datums'] != b['datums'] or a['gdt'] != b['gdt']:
            failures.append(f'{name}: annotation inventory changed')
        values = [b['crossings'], a['crossings'], b['overlaps'], a['overlaps']]
        total = [t + v for t, v in zip(total, values)]
        if a['crossings'] > b['crossings'] or a['overlaps'] > b['overlaps']:
            failures.append(f'{name}: crossings/overlaps regressed')
        lines.append(f'| {name} | {a["datums"]} | {values[0]} → {values[1]} | '
                     f'{values[2]} → {values[3]} |')
    lines.append(f'| **Total** | | **{total[0]} → {total[1]}** | '
                 f'**{total[2]} → {total[3]}** |')
    if total[1] + total[3] >= total[0] + total[2]:
        failures.append('no overall improvement')
    return '\n'.join(lines) + '\n', failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('out', type=Path)
    parser.add_argument('--corpus', type=Path, required=True)
    parser.add_argument('--compare', type=Path)
    parser.add_argument('--reuse', action='store_true')
    parser.add_argument('--timeout', type=float, default=1200)
    parser.add_argument('--jobs', type=int, default=4)
    args = parser.parse_args()
    if not args.corpus.is_dir() or args.timeout <= 0 or args.jobs < 1:
        parser.error('a corpus directory and positive timeout/jobs are required')
    args.out.mkdir(parents=True, exist_ok=True)
    manifest = inventory(args.corpus)
    write(args.out / 'inventory.json', manifest)
    with ThreadPoolExecutor(args.jobs) as pool:
        results = dict(pool.map(lambda row: collect(row, args), manifest['parts']))
    write(args.out / 'survey.json', results)
    failures = [f'{name}: {r["error"]}' for name, r in results.items() if 'error' in r]
    if args.compare:
        table, failures = compare(json.loads((args.compare / 'survey.json').read_text()), results)
        (args.out / 'comparison.md').write_text(table)
        print(table)
    write(args.out / 'failures.json', failures)
    print('\n'.join(failures) if failures else 'PASS: complete survey, no regressions')
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
