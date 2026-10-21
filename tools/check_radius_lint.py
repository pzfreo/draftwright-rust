"""Record only added radius-direction findings from the historical drawing test itself.

--test-binary is the release audits executable Cargo builds. --record updates only
these new codes in known_lint.json; every pre-existing code/count must still match.
Using the exact test producer matters: CLI review uses layout recovery, while this
historical fixture deliberately exercises model::plan without recovery.
"""
import argparse
import collections
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CODES = {'radius_leader_not_radial', 'radius_leader_target_unverifiable'}
RETIRED = {'radius_leader_direction_unverifiable'}
TEST = 'lint::lint_on_the_ports_own_drawings_is_known'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test-binary', type=Path, required=True)
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--record', type=Path)
    args = parser.parse_args()
    if args.corpus is None:
        parser.error('--corpus or DRAFTWRIGHT_CORPUS is required')
    args.out.mkdir(parents=True, exist_ok=True)
    names = {p.name.removesuffix('.json.gz') for p in
             (ROOT / 'tests/fixtures/recognition/corpus').glob('*.json.gz')}
    names.discard('connector109')  # Same explicit slow-part exclusion as the test.
    env = dict(os.environ, LINT_REPORT='1', RUST_TEST_THREADS='6',
               DRAFTWRIGHT_CORPUS=str(args.corpus.resolve()), DRAFTWRIGHT_CORPUS_REQUIRED='1')
    reply = subprocess.run([str(args.test_binary.resolve()), TEST, '--exact', '--nocapture'],
                           env=env, capture_output=True, text=True, timeout=3600)
    output = reply.stdout + reply.stderr
    (args.out / 'test.log').write_text(output)
    actual = collections.defaultdict(collections.Counter)
    for line in output.splitlines():
        part, separator, tail = line.partition(': ')
        if separator and part in names:
            code = tail.partition(' ')[0]
            if re.fullmatch('[a-z][a-z0-9_]*', code):
                actual[part][code] += 1
    reported = re.search(r'(?m)^(\d+) findings on the port\'s own drawings,', output)
    if not reported or sum(sum(row.values()) for row in actual.values()) != int(reported[1]):
        raise ValueError('incomplete test findings; inspect --out/test.log')
    # Include parts with no findings, so a stale baseline entry cannot escape the audit.
    for name in names:
        actual[name]
    baseline = args.record or ROOT / 'tests/fixtures/known_lint.json'
    record = json.loads(baseline.read_text())
    expected = {(r['part'], r['code']): r['count'] for r in record['own_drawings']
                if r['part'] in names and r['code'] not in CODES | RETIRED}
    measured = {(part, code): count for part, counts in actual.items()
                for code, count in counts.items() if code not in CODES}
    if measured != expected:
        raise ValueError('existing lint families changed; review them independently: '
                         + repr({k: (expected.get(k, 0), measured.get(k, 0))
                                 for k in expected.keys() | measured.keys()
                                 if expected.get(k, 0) != measured.get(k, 0)}))
    added = [dict(part=part, code=code, count=count, verdict='rust-correct',
                  reason=('Issue #145: the drawn radius shaft violates the middle-arc, '
                          'radial, non-material-side approach rule.' if code == 'radius_leader_not_radial'
                          else 'Issue #145: the recorded geometry cannot prove the drawn '
                               'radius section in this view; informational, not a wrong shaft.'))
             for part, counts in sorted(actual.items()) for code, count in sorted(counts.items())
             if code in CODES]
    (args.out / 'radius-direction.json').write_text(json.dumps(added, indent=2) + '\n')
    if args.record:
        record['own_drawings'] = [r for r in record['own_drawings'] if r['code'] not in CODES | RETIRED] + added
        args.record.write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n')
    else:
        listed = [r for r in record['own_drawings'] if r['code'] in CODES]
        if {(r['part'], r['code']): r['count'] for r in listed} != {
                (r['part'], r['code']): r['count'] for r in added}:
            raise ValueError('new radius-direction findings differ; inspect --out/radius-direction.json')
    print(f'PASS: existing lint counts unchanged; {sum(r["count"] for r in added)} '
          f'radius direction findings from {TEST}')


if __name__ == '__main__':
    main()
