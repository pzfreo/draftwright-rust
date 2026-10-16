"""Render-cost bound for the corpus's slow tail (#43 item 3): cgb243 and threaded_connector_109.

DRAFTWRIGHT_CORPUS=... python3 -I tools/check_render_tail.py --out target/render-tail.json

Both parts spent nearly all their time in the hidden-line projection of one view (seen along
their axis, where a thread's turns, or a comb's teeth, stack up): cgb243 took 425 s and
threaded_connector_109 135 s (23.5 trillion instructions) to render where the median part takes
well under a second. Each part is rendered once (SVG only, the sheet the projection serves).

Where the kernel counts retired instructions (Darwin), a part fails when it takes more than
MARGIN above the figure tools/render-tail-budget.json records for it on this platform; the
count does not depend on load, core type or a governor pausing the process. Elsewhere (GitHub's
Linux runners) each part must stay within the processor seconds the budget's
`cpu_seconds_ceiling` allows it, a coarse bound below the old cost as measured on the
development machine (cgb243 1,092 s, threaded_connector_109 1,577 s; after the fix 195 s and
254 s), which a hosted runner, slower per thread, would exceed by more. Use --record
deliberately after a performance change, on the platform whose figures you are updating.
"""
import argparse
import gzip
import importlib.util
import json
import os
from pathlib import Path
import resource
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
FORMAT = 'draftwright.render-tail-budget/1'
MARGIN = 0.15
PARTS = {
    'cgb243': 'cadgenbench_inputs/cgb243.step.gz',
    'threaded_connector_109': 'cadgenbench/threaded_connector_109.step',
}
spec = importlib.util.spec_from_file_location('plan_time', ROOT / 'tools/check_plan_time.py')
plan_time = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan_time)


def measured(binary, step, out):
    """Render *step*; return (instructions or None, processor seconds, wall seconds)."""
    out.parent.mkdir(parents=True, exist_ok=True)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    instructions, _ = plan_time.counted([str(binary), 'render', str(step), '-o', str(out),
                                         '--formats', 'svg'])
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    if not list(out.parent.glob(f'{out.name}*.svg')):
        raise ValueError(f'{step}: no sheet rendered')
    return instructions, cpu, wall


def verdict(parts, budget, key):
    """Gate each part on its recorded instructions, or on its processor-time ceiling."""
    if budget.get('format') != FORMAT:
        raise ValueError('incompatible render-tail budget')
    if set(parts) != set(PARTS):
        raise ValueError('incomplete render-tail report')
    figures = budget.get('instructions', {}).get(key, {})
    ceilings = budget['cpu_seconds_ceiling']
    failures = []
    gated = all(row['instructions'] is not None and name in figures for name, row in parts.items())
    for name, row in sorted(parts.items()):
        if gated:
            limit = figures[name] * (1 + MARGIN)
            row.update(limit=limit, ratio_to_budget=row['instructions'] / figures[name])
            if row['instructions'] > limit:
                failures.append(f'{name}: {row["instructions"]} instructions > {limit:.0f}')
        elif row['cpu_seconds'] > ceilings[name]:
            failures.append(f'{name}: {row["cpu_seconds"]:.1f} processor seconds > {ceilings[name]}')
    return {'gated_on': 'instructions' if gated else 'cpu_seconds', 'passed': not failures,
            'failures': failures}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/draftwright')
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--budget', type=Path, default=ROOT / 'tools/render-tail-budget.json')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--record', action='store_true')
    args = parser.parse_args()
    if args.corpus is None:
        parser.error('corpus required')
    key = plan_time.platform_key()
    parts = {}
    with tempfile.TemporaryDirectory(prefix='draftwright-render-tail-') as tmp:
        for name, relative in PARTS.items():
            step = args.corpus / relative
            if step.suffix == '.gz':
                plain = Path(tmp) / step.name[:-3]
                plain.write_bytes(gzip.decompress(step.read_bytes()))
                step = plain
            instructions, cpu, wall = measured(args.binary, step, Path(tmp) / 'sheets' / name)
            parts[name] = {'instructions': instructions, 'cpu_seconds': cpu, 'seconds': wall}
            print(f'{name}: {instructions} instructions, {cpu:.1f} processor s, {wall:.1f} s',
                  flush=True)
    budget = json.loads(args.budget.read_text())
    if args.record:
        if any(row['instructions'] is None for row in parts.values()):
            parser.error(f'no instruction counter on {key}; nothing to record')
        budget.setdefault('instructions', {})[key] = {
            name: row['instructions'] for name, row in sorted(parts.items())}
        args.budget.write_text(json.dumps(budget, indent=2, sort_keys=True) + '\n')
    report = {'platform': key, 'parts': parts, 'budget': budget, **verdict(parts, budget, key)}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    for failure in report['failures']:
        print(f'render-tail: {failure}', file=sys.stderr)
    print(f'gated on {report["gated_on"]}: ' + ('PASS' if report['passed'] else 'FAIL'))
    return int(not report['passed'])


if __name__ == '__main__':
    raise SystemExit(main())
