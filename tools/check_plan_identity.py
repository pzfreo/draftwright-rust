"""Capture/compare exact CLI plan JSON and every sheet SVG over the plan-time survey.

The 30 quality parts must be a subset. Run --record with an origin/main release
binary into a fresh directory, then --reference that directory with the new binary.
No JSON normalisation or SVG filtering: raw bytes and sheet counts must agree.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('plan_time', ROOT / 'tools/check_plan_time.py')
plan_time = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan_time)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/draftwright')
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--out', type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--record', action='store_true')
    mode.add_argument('--reference', type=Path)
    parser.add_argument('--check-only', action='store_true', help='Compare already captured outputs')
    args = parser.parse_args()
    if args.check_only and (args.record or not args.out.is_dir()):
        parser.error('--check-only requires --reference and an existing capture')
    if args.corpus is None or (args.out.exists() and not args.check_only):
        parser.error('corpus and a fresh output directory are required')
    inputs = args.out / 'inputs'
    inputs.mkdir(parents=True, exist_ok=args.check_only)
    files = plan_time.inputs(args.corpus, inputs)
    if not args.check_only:
        provenance = {
            'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
            'inputs_sha256': {p.stem: hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
        }
        (args.out / 'provenance.json').write_text(
            json.dumps(provenance, indent=2, sort_keys=True) + '\n')
    if args.reference:
        before = json.loads((args.reference / 'provenance.json').read_text())
        after = json.loads((args.out / 'provenance.json').read_text())
        if before['inputs_sha256'] != after['inputs_sha256']:
            raise ValueError('reference input bytes changed')
    quality = {(args.corpus / p).stem for p in (ROOT / 'tools/quality_parts.txt').read_text().splitlines()}
    if len(quality) != 30 or not quality <= {p.stem for p in files}:
        raise ValueError('quality manifest is not a 30-part subset of the survey')
    reference = json.loads((args.reference / 'manifest.json').read_text()) if args.reference else None
    manifest = {}
    for part in files:
        directory = args.out / part.stem
        if not args.check_only:
            directory.mkdir()
            reply = subprocess.run([str(args.binary), 'plan', str(part)], capture_output=True,
                                   check=True, timeout=3600)
            (directory / 'plan.json').write_bytes(reply.stdout)
            subprocess.run([str(args.binary), 'render', str(part), '-o', str(directory / part.stem),
                            '--formats', 'svg'], capture_output=True, check=True, timeout=3600)
        if 'dimensions' not in json.loads((directory / 'plan.json').read_bytes()):
            raise ValueError(f'{part}: not a plan')
        outputs = [directory / 'plan.json', *sorted(directory.glob('*.svg'))]
        if len(outputs) < 2:
            raise ValueError(f'{part}: no sheet SVG')
        manifest[part.stem] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in outputs}
        if reference is not None:
            if manifest[part.stem] != reference[part.stem]:
                raise ValueError(f'{part.stem}: plan bytes or sheet SVG bytes/count changed')
            for p in outputs:
                if p.read_bytes() != (args.reference / part.stem / p.name).read_bytes():
                    raise ValueError(f'{part.stem}/{p.name}: bytes changed')
        print(f'{part.stem}: {len(outputs) - 1} sheets OK', flush=True)
    if reference is not None and manifest.keys() != reference.keys():
        raise ValueError('reference part set changed')
    (args.out / 'manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    print(f'{len(files)} survey parts / {len(quality)} quality parts: ' +
          ('reference capture complete' if args.record else 'exact plan JSON and SVG PASS'))


if __name__ == '__main__':
    main()
