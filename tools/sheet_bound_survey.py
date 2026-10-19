"""Audit every skipped sheet candidate against one unrestricted explicit-paper plan.

Use the same Analysis files and input list as tools/plan_timing.py. BEFORE and AFTER
are timing work directories with plans/ and analysis/; AFTER holds the bounded policy.
The opt-in Rust test uses the policy's own failure comparison, including baseline exceptions.
Results and the full test log are written outside the source tree, under OUT.

python3 tools/sheet_bound_survey.py BEFORE AFTER OUT INPUTS.txt
"""
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    before, after, out, inputs = map(Path, sys.argv[1:])
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    for source in inputs.read_text().splitlines():
        name = Path(source).name.split('.')[0]
        rows.append({'name': name,
                     'analysis': str((before / 'analysis' / (name + '.json')).resolve()),
                     'baseline': str((before / 'plans' / (name + '.json')).resolve()),
                     'selected': str((after / 'plans' / (name + '.json')).resolve())})
    manifest = (out / 'requests.json').resolve()
    manifest.write_text(json.dumps(rows, indent=2) + '\n')
    env = {**os.environ, 'DRAFTWRIGHT_SHEET_BOUND_SURVEY': str(manifest)}
    with (out / 'audit.log').open('w') as log:
        result = subprocess.run(['cargo', 'test', '--release', '--lib',
                                 'sheet_policy::tests::corpus_bound_survey', '--',
                                 '--ignored', '--exact', '--nocapture'], env=env,
                                stdout=log, stderr=subprocess.STDOUT)
    results_path = Path(str(manifest) + '.results.json')
    if results_path.exists():
        results = json.loads(results_path.read_text())
        counts = {}
        for row in results:
            key = f"{row['size']}/max={row['max']}"
            counts[key] = counts.get(key, 0) + 1
        print(json.dumps({'parts': len(rows), 'skipped': len(results),
                          'by_request': counts,
                          'false_refusals': [r for r in results if not r['codes']]}, indent=2))
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
