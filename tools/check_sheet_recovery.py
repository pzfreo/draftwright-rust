"""R5: allowing extra sheets preserves today's complete sheet, before any overflow.
Run against the release CLI and the required corpus. Older single-sheet builds
refuse the additive Spec; the initial I1 build fails the equality assertion.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/draftwright')
    parser.add_argument('--corpus', default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    args = parser.parse_args()
    assert args.corpus, 'DRAFTWRIGHT_CORPUS is required'
    parts = ['mfcadpp/1000.step', 'gramel/GRM-03_thumbwheel_drive_screw.step',
             'gramel/string_post.step', 'ttt_inputs/sm-hanger.step',
             'cadgenbench_inputs/cgb203.step', 'mfcadpp/10020.step', 'mfcadpp/10063.step']
    with tempfile.TemporaryDirectory() as work:
        spec = Path(work) / 'max2.json'
        spec.write_text('{"sheets":{"max":2}}')
        for part in parts:
            command = [args.binary, 'plan', str(Path(args.corpus) / part)]
            plans = [json.loads(subprocess.check_output(command + extra, timeout=3600))
                     for extra in [[], ['--spec', str(spec)]]]
            assert not plans[1].get('sheets'), part
            for plan in plans:
                plan.pop('spec')
                plan['sheet'].pop('basis')
            assert plans[0] == plans[1], part
            print(f'{part}: unchanged {plans[0]["sheet"]["size"]} {plans[0]["sheet"]["scale"]}', flush=True)


if __name__ == '__main__':
    main()
