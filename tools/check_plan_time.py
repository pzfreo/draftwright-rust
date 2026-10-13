"""Serial CLI plan budget, in units of a fixed same-run calibration plan.

DRAFTWRIGHT_CORPUS=... python3 -I tools/check_plan_time.py --out target/plan-time.json
Use --record deliberately after validating a performance change; CI only compares.
The survey is the same 42 corpus parts + 22 AP242 fixtures as repeated-spans.
"""
import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
CALIBRATION = 'tools/plan-time-calibration.json'
CALIBRATION_ITERATIONS = 32768


def calibration_fingerprint():
    source = (ROOT / CALIBRATION).read_bytes()
    return hashlib.sha256(CALIBRATION_ITERATIONS.to_bytes(8, 'big') + source).hexdigest()


def calibration_time(binary):
    request = json.dumps(json.loads((ROOT / CALIBRATION).read_text()), separators=(',', ':')).encode() + b'\n'
    start = time.perf_counter()
    reply = subprocess.run([str(binary), 'serve'], input=request * CALIBRATION_ITERATIONS,
                           capture_output=True, check=True, timeout=3600)
    seconds = (time.perf_counter() - start) / CALIBRATION_ITERATIONS
    lines = reply.stdout.splitlines()
    if (len(lines) != CALIBRATION_ITERATIONS or any(line != lines[0] for line in lines)
            or not json.loads(lines[0]).get('ok')
            or 'dimensions' not in json.loads(lines[0]).get('result', {})):
        raise ValueError('calibration did not replay the fixed plan')
    return seconds


def inputs(corpus, directory):
    files = [corpus / p for p in (ROOT / 'tools/repeated_span_parts.txt').read_text().splitlines()]
    fixtures = sorted((ROOT / 'tests/fixtures/pmi/step').glob('*.gz'))
    if len(files) != 42 or len(fixtures) != 22:
        raise ValueError('incomplete survey (42 corpus + 22 AP242 required)')
    for fixture in fixtures:
        path = directory / fixture.name[:-3]
        path.write_bytes(gzip.decompress(fixture.read_bytes()))
        files.append(path)
    if len({p.stem for p in files}) != 64 or not all(p.is_file() for p in files):
        raise ValueError('missing/ambiguous survey input')
    return sorted(files, key=lambda p: p.stem)


def timed(binary, part):
    start = time.perf_counter()
    command = [str(binary), 'plan', str(part)]
    reply = subprocess.run(command, capture_output=True,
                           check=True, timeout=3600)
    seconds = time.perf_counter() - start
    if 'dimensions' not in json.loads(reply.stdout):
        raise ValueError(f'{part}: not a plan')
    return seconds


def within_budget(total, calibration, budget):
    return total / calibration <= budget['normalised_total'] * 1.15


def finish(measured, args):
    expected = {Path(p).stem for p in (ROOT / 'tools/repeated_span_parts.txt').read_text().splitlines()}
    expected |= {Path(p.name[:-3]).stem for p in (ROOT / 'tests/fixtures/pmi/step').glob('*.gz')}
    if len(expected) != 64 or set(measured['parts']) != expected:
        raise ValueError('incomplete 64-part timing report')
    def median(values):
        if not values or not all(type(v) in (int, float) and math.isfinite(v) and v > 0 for v in values):
            raise ValueError('invalid timing samples')
        return statistics.median(values)
    if len(measured['calibration_samples']) != 6:
        raise ValueError('six calibration samples required')
    cal = median(measured['calibration_samples'])
    seconds = {p: median(row['samples']) for p, row in measured['parts'].items()}
    total = sum(seconds[p] for p in sorted(seconds))
    if cal != measured['calibration_seconds'] or total != measured['total_seconds'] or any(
            seconds[p] != row['seconds'] for p, row in measured['parts'].items()):
        raise ValueError('timing aggregates differ from samples')
    budget = json.loads(args.budget.read_text())
    if (budget['format'] != measured['format'] or measured['format'] != 'draftwright.plan-time/2'
            or budget['calibration'] != measured['calibration'] or measured['calibration'] != CALIBRATION
            or budget['calibration_sha256'] != measured['calibration_sha256']
            or budget['calibration_iterations'] != CALIBRATION_ITERATIONS
            or measured['calibration_iterations'] != CALIBRATION_ITERATIONS
            or measured['calibration_sha256'] != calibration_fingerprint()
            or not math.isfinite(budget['normalised_total']) or budget['normalised_total'] <= 0):
        raise ValueError('incompatible calibration budget')
    measured.update(normalised_total=total / cal, budget=budget,
                    passed=within_budget(total, cal, budget))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(measured, indent=2, sort_keys=True) + '\n')
    print(f'{total:.3f}s / {cal:.6f}s = {total / cal:.3f} calibration plans; '
          f'limit {budget["normalised_total"] * 1.15:.3f}; load <= {measured["max_load"]:.2f}')
    return int(not measured['passed'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/draftwright')
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--budget', type=Path, default=ROOT / 'tools/plan-time-budget.json')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--record', action='store_true')
    parser.add_argument('--runs', type=int, default=1)
    parser.add_argument('--max-load', type=float, help='Reject measurements above this 1-minute load')
    parser.add_argument('--results', type=Path, help='Recheck a complete measured report (budget controls)')
    args = parser.parse_args()
    if args.results:
        if args.record or args.max_load is not None or args.runs != 1:
            parser.error('--results cannot record or change measurement options')
        return finish(json.loads(args.results.read_text()), args)
    if args.corpus is None or args.runs < 1:
        parser.error('corpus and a positive run count are required')
    loads = []
    def load():
        loads.append(os.getloadavg()[0])
        if args.max_load is not None and loads[-1] >= args.max_load:
            raise ValueError(f'load {loads[-1]:.2f} >= {args.max_load}; timing unmeasured')
    load()
    stop = threading.Event()
    def monitor():
        while not stop.wait(1):
            loads.append(os.getloadavg()[0])
    thread = threading.Thread(target=monitor)
    thread.start()
    try:
        samples = [calibration_time(args.binary) for _ in range(3)]
        parts = {}
        with tempfile.TemporaryDirectory(prefix='draftwright-plan-time-') as tmp:
            for part in inputs(args.corpus, Path(tmp)):
                load()
                values = [timed(args.binary, part) for _ in range(args.runs)]
                parts[part.stem] = {'seconds': statistics.median(values), 'samples': values}
                print(f'{part.stem}: {parts[part.stem]["seconds"]:.3f}s', flush=True)
        samples += [calibration_time(args.binary) for _ in range(3)]
        load()
    finally:
        stop.set()
        thread.join()
    if args.max_load is not None and max(loads) >= args.max_load:
        raise ValueError(f'load reached {max(loads):.2f}; timing unmeasured')
    cal = statistics.median(samples)
    total = sum(p['seconds'] for p in parts.values())
    measured = {'format': 'draftwright.plan-time/2', 'calibration': CALIBRATION,
                'calibration_sha256': calibration_fingerprint(),
                'calibration_iterations': CALIBRATION_ITERATIONS,
                'calibration_seconds': cal, 'calibration_samples': samples,
                'total_seconds': total, 'normalised_total': total / cal,
                'max_load': max(loads), 'parts': parts}
    if args.record:
        args.budget.write_text(json.dumps({k: measured[k] for k in
                              ('format', 'calibration', 'calibration_sha256', 'calibration_iterations', 'normalised_total')}, indent=2) + '\n')
    return finish(measured, args)


if __name__ == '__main__':
    raise SystemExit(main())
