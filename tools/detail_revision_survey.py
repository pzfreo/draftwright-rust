"""Compare all unique corpus/fixture STEP contents against an archived executable.

The review manifest has 227 drawable parts plus the intentionally unresolved fixture.
Use --manifest to replay that exact set after additional fixtures are merged.
The contents manifest, complete Plans, errors and PMI requirement states are retained.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
import math
import shutil
from pathlib import Path
import subprocess


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--corpus', type=Path, required=True)
    ap.add_argument('--binary', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--manifest', type=Path, help='Replay an existing unique-content survey exactly')
    ap.add_argument('--audit-legibility', action='store_true', help='Retain Analysis and audit dimensioned step spans at owning-view scale')
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    # Every child uses one immutable executable, even if another verification builds the CLI.
    binary = args.out / 'draftwright'
    shutil.copy2(args.binary, binary)
    (args.out / 'binary.json').write_text(json.dumps({
        'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'source': str(args.binary.resolve())}, indent=2) + '\n')
    inputs = {}
    for root, prefix in [(args.corpus, 'corpus'), (Path('tests/fixtures'), 'fixtures')]:
        for p in sorted(root.rglob('*')):
            if not p.is_file() or not any(p.name.lower().endswith(x) for x in ('.step', '.stp', '.step.gz', '.stp.gz')):
                continue
            data = gzip.decompress(p.read_bytes()) if p.suffix == '.gz' else p.read_bytes()
            key = hashlib.sha256(data).hexdigest()
            name = f'{prefix}/{p.relative_to(root)}'
            if key in inputs:
                inputs[key]['aliases'].append(name)
            else:
                target = args.out / 'inputs' / key / (p.name[:-3] if p.suffix == '.gz' else p.name)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                inputs[key] = {'name': name, 'aliases': [], 'path': str(target)}
    if args.manifest:
        inputs = json.loads(args.manifest.read_text())
        for key, source in inputs.items():
            path = Path(source['path'])
            if not path.is_file():
                raise FileNotFoundError(path)
            if hashlib.sha256(path.read_bytes()).hexdigest() != key:
                raise ValueError(f'survey input changed: {path}')
    (args.out / 'manifest.json').write_text(json.dumps(inputs, indent=2) + '\n')
    def run(item):
        key, source = item
        reply = subprocess.run([str(binary.resolve()), 'plan', source['path']], capture_output=True, timeout=3600)
        (args.out / (key + '.plan.json')).write_bytes(reply.stdout)
        if reply.returncode:
            row = {**source, 'refused': reply.stderr.decode() + reply.stdout.decode()}
        else:
            p = json.loads(reply.stdout)
            audit = {}
            if args.audit_legibility:
                analysed = subprocess.run([str(binary.resolve()), 'analyse', source['path']],
                                          capture_output=True, timeout=3600, check=True)
                (args.out / (key + '.analysis.json')).write_bytes(analysed.stdout)
                a = json.loads(analysed.stdout)
                features = {f['id']: f for f in a['features']}
                views = {v['id']: v for v in p['views']}
                short = {}
                for d in p['dimensions']:
                    if not d.get('placed'): continue
                    for m in d.get('measurements', []):
                        f = features.get(m['feature'], {})
                        if m['parameter'] not in ('step.length', 'step_position.length') or f.get('kind') != 'turned_step': continue
                        size = f['parameters']['hi'] - f['parameters']['lo']
                        drawn = size * views[d['view']]['scale']
                        if size > 0 and drawn + 1e-6 < 2.1:  # dims::CAP = 0.7 * TEXT
                            short[m['feature']] = {'dimension': d['id'], 'view': d['view'], 'drawn_mm': drawn}
                openings = [f['parameters'].get('diameter' if f['kind'] == 'hole' else 'width')
                            for f in a['features'] if f['kind'] in ('hole', 'slot')]
                opening = min((v for v in openings if isinstance(v, (int, float))
                               and math.isfinite(v) and v > 0), default=float('inf'))
                longest = max(hi-lo for lo,hi in zip(*a['envelope']))
                floor = max(0.8/opening, 20/max(longest, 1e-12), 0.1)
                audit = {'unreadable_steps': short, 'geometric_floor': floor,
                         'below_geometric_floor': p['sheet']['scale_factor'] + 1e-9 < floor,
                         'legibility_warnings': [q for q in p['problems'] if q['code'] in
                             ('legibility_floor_breached', 'layout_recovery_exhausted')]}
            row = {**source, **audit, 'sheet': p['sheet']['size'], 'scale': p['sheet']['scale'],
                   'sheets': 1 + len(p.get('sheets', [])),
                   'details': [{k: v['detail'][k] for k in ('label', 'kind', 'scale', 'dimensions')} for v in p['views'] if v.get('detail')],
                   'errors': [q for q in p['problems'] if q['severity'] == 'error'],
                   'pmi': sorted(r['id'] for field in ('gdt', 'dimensions', 'requirements')
                                 for r in p.get(field, []) if r.get('placed') is not None
                                 and (field == 'gdt' or r['id'].startswith(('pmi:', 'd:pmi:'))
                                      or 'AP242' in r.get('basis', ''))),
                   'claims': sorted({(m['feature'], m['parameter']) for d in p['dimensions']
                                     if d.get('placed') for m in d.get('measurements', [])}),
                   'manufacturing_tables': [t['id'] for t in p.get('tables', [])
                                            if t.get('placed') and 'manufacturing' in t['id']]}
        print(source['name'], row.get('sheet', 'REFUSED'), row.get('scale', ''), flush=True)
        return key, row
    with ThreadPoolExecutor(max_workers=4) as pool:
        rows = dict(pool.map(run, inputs.items()))
    (args.out / 'summary.json').write_text(json.dumps(rows, indent=2) + '\n')


if __name__ == '__main__':
    main()
