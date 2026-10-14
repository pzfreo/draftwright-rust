"""Count the complete 64-part CLI survey on macOS, comparing two release binaries.

Darwin's proc_pid_rusage reads retired instructions for the unreaped child. This
uses the same kernel counter as time -l without its sandbox-blocked clock sysctl.
No wall-clock conversion or load correction is applied. --check gates the 55% total
target and a per-part limit of PART_TOLERANCE (1%: run-to-run counts differ by about
0.5%); otherwise writes evidence.
"""
import argparse
import importlib.util
import hashlib
import json
import os
import statistics
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PART_TOLERANCE = 0.01
spec = importlib.util.spec_from_file_location('plan_time', ROOT / 'tools/check_plan_time.py')
plan_time = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan_time)


def counted(command):
    instructions, reply = plan_time.counted(command)
    if instructions is None:
        raise ValueError('retired-instruction measurement requires Darwin with a working counter')
    return instructions, reply


def assess(parts):
    base = sum(row['base'] for row in parts.values())
    head = sum(row['head'] for row in parts.values())
    if not parts or any(type(row[k]) is not int or row[k] <= 0
                        for row in parts.values() for k in ('base', 'head')):
        raise ValueError('positive integer instruction counts required')
    worse = {name: row['head'] / row['base'] for name, row in parts.items()
             if row['head'] > row['base'] * (1 + PART_TOLERANCE)}
    return {'base_total': base, 'head_total': head, 'ratio': head / base,
            'worse_parts': worse, 'passed': head <= base * .55 and not worse}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-binary', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/draftwright')
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--runs', type=int, default=1, help='Median per-part counts over paired runs')
    parser.add_argument('--initial-results', type=Path, help='Reuse one complete paired run as the first sample')
    parser.add_argument('--resume', type=Path, help='Continue an interrupted report without repeating completed parts')
    parser.add_argument('--operation', choices=('plan', 'analyse'), default='plan',
                        help='analyse measures the unchanged input-preparation floor separately')
    args = parser.parse_args()
    if args.runs < 1 or args.runs % 2 == 0:
        parser.error('--runs must be positive and odd')
    if args.resume and args.initial_results:
        parser.error('--resume and --initial-results are mutually exclusive')
    initial = json.loads(args.initial_results.read_text()) if args.initial_results else None
    resumed = json.loads(args.resume.read_text()) if args.resume else None
    if resumed and (resumed.get('complete') or resumed['operation'] != args.operation
                    or resumed.get('runs', 1) != args.runs):
        parser.error('--resume requires an interrupted report with matching operation and runs')
    if initial and (not initial.get('complete') or len(initial.get('parts', {})) != 64
                    or initial.get('runs', 1) != 1 or initial['operation'] != args.operation or args.runs < 2):
        parser.error('--initial-results requires a complete matching report and --runs >= 2')
    if args.corpus is None:
        parser.error('corpus required')
    provenance = {'runs': args.runs,
                  'base_binary_sha256': hashlib.sha256(args.base_binary.read_bytes()).hexdigest(),
                  'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                  'initial_report_sha256': hashlib.sha256(args.initial_results.read_bytes()).hexdigest()
                  if args.initial_results else None,
                  'unfingerprinted_resumed_parts': len(resumed['parts'])
                  if resumed and 'base_binary_sha256' not in resumed else 0,
                  'resumed_report_sha256': hashlib.sha256(args.resume.read_bytes()).hexdigest()
                  if args.resume else None}
    if initial or resumed:
        for key in ('base_binary_sha256', 'binary_sha256'):
            previous = initial or resumed
            if key in previous and previous[key] != provenance[key]:
                parser.error(f'previous report: {key} differs')
    parts = dict(resumed['parts']) if resumed else {}
    if parts:
        assess(parts)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='draftwright-instructions-') as directory:
        inputs = plan_time.inputs(args.corpus, Path(directory))
        if not set(parts).issubset({p.stem for p in inputs}):
            parser.error('--resume contains unknown parts')
        for part in inputs:
            if part.stem in parts:
                if ('input_sha256' in parts[part.stem]
                        and parts[part.stem]['input_sha256'] != hashlib.sha256(part.read_bytes()).hexdigest()):
                    parser.error(f'{part.stem}: resumed input changed')
                print(f'{part.stem}: retained completed measurement', flush=True)
                continue
            base_samples, head_samples = [], []
            if initial:
                base_samples.append(initial['parts'][part.stem]['base'])
                head_samples.append(initial['parts'][part.stem]['head'])
            for _ in range(len(base_samples), args.runs):
                base, before = counted([str(args.base_binary), args.operation, str(part)])
                if args.base_binary.resolve() == args.binary.resolve():
                    head, after = base, before
                else:
                    head, after = counted([str(args.binary), args.operation, str(part)])
                if before != after:
                    raise ValueError(f'{part.stem}: output bytes changed')
                base_samples.append(base)
                head_samples.append(head)
            base, head = int(statistics.median(base_samples)), int(statistics.median(head_samples))
            parts[part.stem] = {'base': base, 'head': head,
                                'input_sha256': hashlib.sha256(part.read_bytes()).hexdigest(),
                                'base_samples': base_samples, 'head_samples': head_samples}
            report = {'format': 'draftwright.plan-instructions/1', 'operation': args.operation,
                      **provenance, 'parts': parts, 'complete': len(parts) == 64, **assess(parts)}
            report['passed'] = report['complete'] and report['passed']
            args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
            print(f'{part.stem}: {base} -> {head} ({head / base:.4f})', flush=True)
    print(json.dumps({k: v for k, v in report.items() if k != 'parts'}, indent=2))
    return int(args.check and not report['passed'])


if __name__ == '__main__':
    raise SystemExit(main())
