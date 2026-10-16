"""Compare all unique corpus/fixture STEP contents against an archived executable.

Exactly 227 drawable parts, plus the intentionally unresolved fixture, at this revision.
The contents manifest, complete Plans, errors and PMI requirement states are retained.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
import shutil
from pathlib import Path
import subprocess


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--corpus', type=Path, required=True)
    ap.add_argument('--binary', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
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
    (args.out / 'manifest.json').write_text(json.dumps(inputs, indent=2) + '\n')
    def run(item):
        key, source = item
        reply = subprocess.run([str(binary.resolve()), 'plan', source['path']], capture_output=True, timeout=3600)
        (args.out / (key + '.plan.json')).write_bytes(reply.stdout)
        if reply.returncode:
            row = {**source, 'refused': reply.stderr.decode() + reply.stdout.decode()}
        else:
            p = json.loads(reply.stdout)
            row = {**source, 'sheet': p['sheet']['size'], 'scale': p['sheet']['scale'],
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
