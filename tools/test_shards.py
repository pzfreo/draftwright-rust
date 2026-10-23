#!/usr/bin/env python3
"""Duration-balanced nextest shards for CI.

nextest's hash partition ignores durations, and a few corpus tests take minutes, so hash shards
of the same suite differed by more than two to one (CI run 39813975347: 34 to 76 test-minutes).
This assigns every test the `ci` profile runs to one of N shards, longest first to the
least-loaded shard, using the durations recorded in tools/test-durations.json; a test with no
record weighs the median. The assignment is a deterministic function of the archive's test list,
so the N shards partition it; stale durations only cost balance, never coverage.

  cargo nextest list --archive-file A --profile ci --message-format json > list.json
  python3 tools/test_shards.py --list list.json --shard 3/8   # prints the shard's filterset

--priorities PATH also writes a nextest tool config (pass it as --tool-config-file) that
starts the shard's tests longest first, and unrecorded tests before them: a 270 s test that
starts 460 s into its shard sets the shard's wall time. The repository's own `ci` overrides (the exclusive latency gates)
take precedence over it; it takes precedence over the `default` profile's hand-kept order.

Refresh the record from the JUnit reports of a CI run's test shards (artifacts test-junit-*):
  python3 tools/test_shards.py --record junit-dir
"""
import argparse
import json
from pathlib import Path
import statistics
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
DURATIONS = ROOT / 'tools/test-durations.json'


def shard_arg(text):
    k, n = map(int, text.split('/'))
    if not 1 <= k <= n:
        raise argparse.ArgumentTypeError('shard must be K/N with 1 <= K <= N')
    return k, n


def listed(listing):
    """(binary-id, test name) of every test the listing's filters run."""
    return sorted((suite['binary-id'], name)
                  for suite in listing['rust-suites'].values()
                  for name, case in suite['testcases'].items()
                  if case['filter-match']['status'] == 'matches')


def assign(tests, durations, n):
    known = [durations[f'{b} {t}'] for b, t in tests if f'{b} {t}' in durations]
    default = statistics.median(known) if known else 1.0
    weight = {test: durations.get(f'{test[0]} {test[1]}', default) for test in tests}
    load, shards = [0.0] * n, [[] for _ in range(n)]
    for test in sorted(tests, key=lambda t: (-weight[t], t)):
        i = min(range(n), key=lambda j: (load[j], j))
        load[i] += weight[test]
        shards[i].append(test)
    return shards, load


def filterset(tests):
    by_binary = {}
    for binary, name in sorted(tests):
        by_binary.setdefault(binary, []).append(name)
    return ' | '.join(f'(binary_id(={b}) & (' + ' | '.join(f'test(={t})' for t in names) + '))'
                      for b, names in by_binary.items())


def priorities(tests, durations):
    """A nextest tool config ranking the tests of 10 s or more by recorded duration. A test with
    no record (one added since the record, or from the base branch a pull request merges with)
    starts first: it may be long, and a long test started last sets the shard's wall time."""
    buckets = {}
    for test in tests:
        seconds = durations.get(f'{test[0]} {test[1]}')
        if seconds is None or seconds >= 10:
            buckets.setdefault(99 if seconds is None else min(99, round(seconds / 4)), []).append(test)
    return ''.join(f"[[profile.ci.overrides]]\nfilter = '{filterset(group)}'\npriority = {p}\n\n"
                   for p, group in sorted(buckets.items(), reverse=True))


def record(directory):
    durations = {}
    for path in sorted(Path(directory).glob('**/*.xml')):
        for case in ET.parse(path).getroot().iter('testcase'):
            durations[f'{case.get("classname")} {case.get("name")}'] = round(float(case.get('time')), 1)
    if not durations:
        raise SystemExit(f'no JUnit test cases under {directory}')
    DURATIONS.write_text(json.dumps(durations, indent=0, sort_keys=True) + '\n')
    print(f'recorded {len(durations)} durations')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--list', type=Path, help='nextest list --message-format json output')
    parser.add_argument('--shard', type=shard_arg, metavar='K/N')
    parser.add_argument('--priorities', type=Path, metavar='TOML',
                        help='Also write the shard\'s longest-first nextest tool config here')
    parser.add_argument('--record', type=Path, metavar='JUNIT_DIR')
    args = parser.parse_args()
    if args.record:
        return record(args.record)
    if not (args.list and args.shard):
        parser.error('give --list and --shard, or --record')
    tests = listed(json.loads(args.list.read_text()))
    k, n = args.shard
    durations = json.loads(DURATIONS.read_text())
    shards, load = assign(tests, durations, n)
    if not shards[k - 1]:
        raise SystemExit(f'shard {k}/{n} has no tests')
    print(f'shard {k}/{n}: {len(shards[k - 1])} of {len(tests)} tests, '
          f'{load[k - 1] / 60:.1f} of {sum(load) / 60:.1f} recorded test-minutes', file=sys.stderr)
    if args.priorities:
        args.priorities.write_text(priorities(shards[k - 1], durations))
    print(filterset(shards[k - 1]))


if __name__ == '__main__':
    raise SystemExit(main())
