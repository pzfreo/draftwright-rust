"""Audit the added radius-direction codes on the existing recognised-sheet fixtures.

Use --record tests/fixtures/known_lint.json to record ONLY the two new code families.
Existing lint-code counts must still agree: this cannot bless a historical regression.
All reviews, recognition inputs and SVGs stay in --out, outside the fixture directory.
"""
import argparse
import collections
from concurrent.futures import ThreadPoolExecutor
import gzip
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CODES = {'radius_leader_not_radial', 'radius_leader_direction_unverifiable'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--record', type=Path)
    args = parser.parse_args()
    if args.corpus is None:
        parser.error('--corpus or DRAFTWRIGHT_CORPUS is required')
    args.out.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve()
    inputs = []
    for sheet in sorted((ROOT / 'tests/fixtures/sheets').glob('*.json.gz')):
        name = sheet.name.removesuffix('.json.gz')
        recognition = ROOT / 'tests/fixtures/recognition/corpus' / (name + '.json.gz')
        if name == 'connector109' or not recognition.is_file():
            continue  # Same slow-part exclusion as lint::lint_on_the_ports_own_drawings_is_known.
        facts = json.loads(gzip.decompress(sheet.read_bytes()))
        inputs.append((name, args.corpus / facts['binding']['file'], recognition))

    def review(row):
        name, step, recognition = row
        features = args.out / (name + '.features.json')
        features.write_bytes(gzip.decompress(recognition.read_bytes()))
        result = subprocess.run([str(binary), 'review', str(step), '--features', str(features)],
                                capture_output=True, text=True, check=True, timeout=3600)
        result = json.loads(result.stdout)
        (args.out / (name + '.svg')).write_text(result['svg'])
        (args.out / (name + '.review.json')).write_text(json.dumps(
            {k: result[k] for k in ('lint', 'structural', 'problems')}, indent=2) + '\n')
        return name, collections.Counter(i['code'] for i in result['lint'])

    with ThreadPoolExecutor(4) as executor:
        actual = dict(executor.map(review, inputs))
    baseline = args.record or ROOT / 'tests/fixtures/known_lint.json'
    record = json.loads(baseline.read_text())
    expected = {(r['part'], r['code']): r['count'] for r in record['own_drawings']
                if r['part'] in actual and r['code'] not in CODES}
    measured = {(part, code): count for part, counts in actual.items()
                for code, count in counts.items() if code not in CODES}
    if measured != expected:
        raise ValueError('existing lint families changed; review them independently: '
                         + repr({k: (expected.get(k, 0), measured.get(k, 0))
                                 for k in expected.keys() | measured.keys()
                                 if expected.get(k, 0) != measured.get(k, 0)}))
    added = [dict(part=part, code=code, count=count, verdict='rust-correct',
                  reason=('Issue #145: the drawn radius shaft violates the middle-arc, '
                          'radial, open-centre approach rule.' if code == 'radius_leader_not_radial'
                          else 'Issue #145: the recorded fillet/toroidal geometry cannot prove '
                               'the drawn shaft radial from the open centre side.'))
             for part, counts in sorted(actual.items()) for code, count in sorted(counts.items())
             if code in CODES]
    (args.out / 'radius-direction.json').write_text(json.dumps(added, indent=2) + '\n')
    if args.record:
        record['own_drawings'] = [r for r in record['own_drawings'] if r['code'] not in CODES] + added
        args.record.write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n')
    else:
        listed = [r for r in record['own_drawings'] if r['code'] in CODES]
        if {(r['part'], r['code']): r['count'] for r in listed} != {
                (r['part'], r['code']): r['count'] for r in added}:
            raise ValueError('new radius-direction findings differ; inspect --out/radius-direction.json')
    print(f'PASS: {len(actual)} recognised drawings; existing lint counts unchanged; '
          f'{sum(r["count"] for r in added)} radius direction findings')


if __name__ == '__main__':
    main()
