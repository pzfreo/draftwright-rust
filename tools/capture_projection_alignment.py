"""Capture complete Plan hashes for the legacy projection-alignment fixture.

Use --reference-binary to prove that only fix metadata changed before --write.
The normalization matches tests/projection_alignment.rs: round floating values
at 0.01 mm, and omit analysis_sha256 and loader version.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from check_plan_identity import without_fix_metadata

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/fixtures/projection_alignment/legacy.sha256'


def rounded(value):
    if isinstance(value, float):
        return format(value, '.2f')
    if isinstance(value, list):
        return [rounded(v) for v in value]
    if isinstance(value, dict):
        return {k: rounded(v) for k, v in value.items() if k != 'loader'}
    return value


def digest(data):
    plan = json.loads(data)
    plan.pop('analysis_sha256', None)
    encoded = json.dumps(rounded(plan), sort_keys=True, ensure_ascii=False,
                         separators=(',', ':')).encode()
    return hashlib.sha256(encoded).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/draftwright')
    parser.add_argument('--reference-binary', type=Path)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    previous = {(a, b): h for a, b, h in
                (line.split() for line in FIXTURE.read_text().splitlines()
                 if line and not line.startswith('#'))}
    step = ROOT / 'tests/fixtures/samples/editor-plate.step'
    rows = []
    with tempfile.TemporaryDirectory(prefix='projection-alignment-') as tmp:
        path = Path(tmp) / 'spec.json'
        for sheets in (False, True):
            for align in ('none', 'start', 'middle', 'end', 'outside'):
                spec = {'sheet': {'size': 'A2', 'scale': '2:1'}}
                if sheets:
                    spec['sheets'] = {'max': 1}
                if align != 'none':
                    spec['views'] = {'plan': {'place': {'relative_to': 'front',
                        'side': 'above', 'gap': 15, 'align': align, 'pinned': True}}}
                path.write_text(json.dumps(spec))
                command = ['plan', str(step), '--spec', str(path)]
                current = subprocess.check_output([str(args.binary.resolve()), *command])
                key = (str(sheets).lower(), align)
                if args.reference_binary:
                    reference = subprocess.check_output([str(args.reference_binary.resolve()), *command])
                    assert digest(reference) == previous[key], f'{key}: baseline fixture mismatch'
                    assert without_fix_metadata(current) == without_fix_metadata(reference), \
                        f'{key}: non-fix Plan fields changed'
                rows.append(f'{key[0]} {align} {digest(current)}')
    text = '\n'.join(line for line in FIXTURE.read_text().splitlines() if line.startswith('#'))
    text += '\n' + '\n'.join(rows) + '\n'
    if args.write:
        FIXTURE.write_text(text)
        print('Updated 10 alignment hashes; all non-fix fields match the baseline.'
              if args.reference_binary else 'Updated 10 alignment hashes.')
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
