"""Render changed native sheet selections plus named review parts, using archived builds.

python3 tools/render_sheet_selection.py BEFORE AFTER OUT [--part PATH ...]
BEFORE/AFTER are detail_revision_survey.py directories. Writes both stages' PNGs,
SVGs, render findings and an index with every changed paper/count/scale choice.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess


def read(path):
    return json.loads(path.read_text())


def choice(plan):
    return [plan['sheet']['size'], plan['sheet']['scale'], 1 + len(plan.get('sheets', []))]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('out', type=Path)
    parser.add_argument('--part', action='append', type=Path, default=[])
    parser.add_argument('--corpus-only', action='store_true', help='Review changed corpus inputs only')
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    sources = read(args.before / 'manifest.json')
    selected = {}
    for key, source in sources.items():
        if args.corpus_only and not source['name'].startswith('corpus/'):
            continue
        plans = [read(stage / (key + '.plan.json')) for stage in (args.before, args.after)]
        if all('sheet' in p for p in plans) and choice(plans[0]) != choice(plans[1]):
            selected[key] = {**source, 'before': choice(plans[0]), 'after': choice(plans[1])}
    for part in args.part:
        key = hashlib.sha256(part.read_bytes()).hexdigest()
        if key in selected:
            continue
        plans = [json.loads(subprocess.check_output(
            [str((stage / 'draftwright').resolve()), 'plan', str(part)], timeout=3600))
            for stage in (args.before, args.after)]
        selected[key] = {'name': str(part), 'path': str(part),
                         'before': choice(plans[0]), 'after': choice(plans[1])}
    for label in ('before', 'after'):
        (args.out / label).mkdir(exist_ok=True)

    def render(item):
        key, source = item
        stem = key[:12] + '-' + Path(source['path']).stem
        for label, stage in (('before', args.before), ('after', args.after)):
            output = args.out / label / stem
            result = subprocess.run([str((stage / 'draftwright').resolve()), 'render',
                                     source['path'], '-o', str(output), '--formats',
                                     'svg,png', '--dpi', '150'], capture_output=True,
                                    check=True, timeout=3600)
            output.with_suffix('.render.json').write_bytes(result.stdout)
        source['render_stem'] = stem
        print(source['name'], source['before'], '->', source['after'], flush=True)

    with ThreadPoolExecutor(2) as pool:
        list(pool.map(render, selected.items()))
    (args.out / 'index.json').write_text(json.dumps(selected, indent=2) + '\n')


if __name__ == '__main__':
    main()
