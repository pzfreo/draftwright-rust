"""Local/CI acceptance using one existing release binary; run from the repository root.

DRAFTWRIGHT_CORPUS=... python3 -I tools/ci_acceptance.py --out target/acceptance
No build, fixture fallback, or baseline update is performed. Each job keeps its evidence.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
KEYS = ['off_sheet', 'repeated_label', 'repeated_measure', 'views_overlap',
        'view_on_title_block', 'ink', 'repeated_span', 'line_through_feature',
        'hidden_on_visible', 'section_hidden']
JOBS = ('quality-pins', 'repeated-spans', 'mcp')
CORPUS_REV = '6d544e060cd0eeb4442d2196e5df757b5a15ea55'
spec = importlib.util.spec_from_file_location('quality', ROOT / 'tools/quality.py')
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def manifest(name):
    files = (ROOT / 'tools' / name).read_text().splitlines()
    require(len(files) == len(set(files)), f'duplicate fixture in {name}')
    return files


def names(files):
    return {Path(p).stem for p in files}


def survey_names():
    corpus = manifest('repeated_span_parts.txt')
    fixtures = sorted((ROOT / 'tests/fixtures/pmi/step').glob('*.gz'))
    require(len(corpus) == 42 and len(fixtures) == 22, 'incomplete survey fixture manifest (42 + 22 required)')
    result = names(corpus) | names(p.name[:-3] for p in fixtures)
    require(len(result) == 64, 'incomplete/ambiguous 64-part survey')
    return result


def complete(actual, expected):
    require(set(actual) == expected, f'incomplete part set: missing {sorted(expected - set(actual))}, '
            f'unexpected {sorted(set(actual) - expected)}')
    for name, row in actual.items():
        require('error' not in row, f'{name}: {row.get("error")}')


def count(row, key, part):
    require(key in row and type(row[key]) is int and row[key] >= 0,
            f'{part}: missing/invalid {key}')
    return row[key]


def validate(kind, actual, baseline=None):
    expected = names(manifest('quality_parts.txt'))
    require(len(expected) == 30, 'incomplete quality manifest (30 required)')
    if kind == 'quality':
        require(baseline['metrics'] == KEYS, 'incomplete baseline metric list')
        reference = baseline['parts']
        complete(reference, expected)
        complete(actual, expected)
        for part in sorted(expected):
            for key in KEYS:
                before, after = count(reference[part], key, part), count(actual[part], key, part)
                require(after <= before, f'{part}: {key} regressed {before} -> {after}')
        return {'parts': 30, 'cells': 300,
                'before': {k: sum(p[k] for p in reference.values()) for k in KEYS},
                'after': {k: sum(p[k] for p in actual.values()) for k in KEYS}}
    if kind == 'survey':
        complete(actual, survey_names())
        for part, row in actual.items():
            require(count(row, 'repeated_span', part) == 0, f'{part}: repeated_span must be zero')
        return {'parts': 64, 'repeated_span': 0}
    if kind == 'pins':
        require(actual['seed'] in (1, 2, 3) and actual['pins'] in (1, 3, 8), 'unexpected pin grid cell')
        complete(actual['parts'], expected)
        reference = baseline.get('parts', baseline)
        complete(reference, expected)
        for part, row in actual['parts'].items():
            ids = row['ids']
            require(len(ids) == actual['pins'] and len(set(ids)) == len(ids), f'{part}: incomplete selected pins')
            require(row['unpinned'] == reference[part], f'{part}: unpinned baseline differs from same-run evidence')
            for variant in ('unpinned', 'pinned'):
                for key in ('ink', 'off_sheet'):
                    count(row[variant], key, part)
            require(quality.pin_bar(row['unpinned'], row['pinned']), f'{part}: pin bar failed')
            require(row['go'] is True, f'{part}: NO-GO')
        require(actual['go'] is True, 'pin grid NO-GO')
        return {'parts': 30, 'seed': actual['seed'], 'pins': actual['pins'],
                'ink': sum(p['pinned']['ink'] for p in actual['parts'].values()),
                'off_sheet': sum(p['pinned']['off_sheet'] for p in actual['parts'].values())}
    tasks = read(ROOT / 'tools/mcp-eval/tasks.json')['tasks']
    expected_tasks = {t['id'] for t in tasks}
    rows = actual['results']
    require(len(expected_tasks) == len(rows) == 7 and {r['task'] for r in rows} == expected_tasks,
            'incomplete/duplicate seven-task MCP run')
    require(all(r['success'] is True and not r.get('failure') and not r['failed_conditions'] for r in rows),
            'MCP task failed')
    require(actual['summary']['tasks'] == actual['summary']['succeeded'] == 7, 'incomplete MCP summary')
    return actual['summary']


def mcp_documents(directory):
    for task in read(ROOT / 'tools/mcp-eval/tasks.json')['tasks']:
        path = directory / task['id']
        for name in ('spec', 'plan', 'baseline'):
            read(path / (name + '.json'))
        image = (path / 'sheet.png').read_bytes()
        require(len(image) > 8 and image[:8] == b'\x89PNG\r\n\x1a\n',
                f'{task["id"]}: missing/invalid rendered PNG artifact')


class Run:
    def __init__(self, args, out):
        self.args, self.out, self.steps = args, out, []
        self.metrics = {}

    def step(self, label, argv, kind=None, result=None, baseline=None):
        start = time.monotonic()
        record = {'name': label, 'command': [str(a) for a in argv]}
        try:
            with (self.out / (label + '.log')).open('w') as log:
                child = subprocess.run(record['command'], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                       timeout=1500)
            record['exit_code'] = child.returncode
            if kind:
                record['metrics'] = validate(kind, read(result), read(baseline) if baseline else None)
                if kind == 'mcp':
                    mcp_documents(self.out / 'mcp-documents')
                self.metrics[label] = record['metrics']
            require(child.returncode == 0, f'{label} exited {child.returncode}; see {label}.log')
            record['passed'] = True
        except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
            record.update(passed=False, error=str(error))
        record['wall_seconds'] = round(time.monotonic() - start, 3)
        self.steps.append(record)
        print(f'{label}: {"PASS" if record["passed"] else "FAIL"} ({record["wall_seconds"]}s)', flush=True)
        return record['passed']

    def execute(self, job):
        args, out = self.args, self.out
        binary = args.binary
        py = [sys.executable, '-I']
        if job == 'quality-pins':
            files = [args.corpus / p for p in manifest('quality_parts.txt')]
            quality_out = out / 'quality'
            fresh = quality_out / 'baseline.json'
            self.step('quality', [*py, ROOT / 'tools/quality.py', binary, quality_out, *files],
                      'quality', fresh, args.baseline)
            # The deliberate failure uses a real measured report, without changing the accepted baseline.
            if fresh.is_file():
                bad = read(fresh)
                reference = read(args.baseline)['parts']
                part = sorted(reference)[0]
                bad[part]['ink'] = reference[part]['ink'] + 1
                write(out / 'deliberate-regression.json', bad)
                command = [*py, Path(__file__), '--check-results', 'quality', '--results',
                           out / 'deliberate-regression.json', '--baseline', args.baseline]
                with (out / 'deliberate-rejection.log').open('w') as log:
                    rejected = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
                self.steps.append({'name': 'reject-deliberate-regression', 'passed': rejected.returncode == 1,
                                   'observed_exit_code': rejected.returncode, 'expected_exit_code': 1})
            for seed in (1, 2, 3):
                for pins in (1, 3, 8):
                    pin_out = out / f'pins-{seed}-{pins}'
                    self.step(f'pins-{seed}-{pins}', [*py, ROOT / 'tools/quality.py', '--seed', seed,
                              '--pins', pins, '--baseline', fresh, '--require-go', binary, pin_out, *files],
                              'pins', pin_out / 'random-pins.json', fresh)
        elif job == 'repeated-spans':
            survey_out = out / 'survey'
            self.step('survey', [*py, ROOT / 'tools/survey_repeated_spans.py', binary, survey_out],
                      'survey', survey_out / 'baseline.json')
        else:
            self.step('python-quality-tests', [*py, ROOT / 'tools/test_quality.py'])
            self.step('acceptance-tests', [*py, ROOT / 'tools/test_ci_acceptance.py'])
            self.step('node-client-tests', ['node', '--test', 'tools/mcp-eval/test-client.mjs'])
            self.step('mcp', ['node', 'tools/mcp-eval/run-scripted.mjs', '--binary', binary,
                      '--out', out / 'mcp.json', '--documents-dir', out / 'mcp-documents'],
                      'mcp', out / 'mcp.json')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--job', choices=(*JOBS, 'all'), default='all')
    parser.add_argument('--binary', type=Path, default=Path('target/release/draftwright'))
    parser.add_argument('--corpus', type=Path, default=os.environ.get('DRAFTWRIGHT_CORPUS'))
    parser.add_argument('--out', type=Path, default=Path('target/acceptance'))
    parser.add_argument('--baseline', type=Path, default=ROOT / 'tools/ci-quality-baseline.json')
    parser.add_argument('--check-results', choices=('quality', 'survey', 'pins', 'mcp'))
    parser.add_argument('--results', type=Path)
    parser.add_argument('--documents-dir', type=Path, help='Check saved MCP judge documents and PNGs')
    args = parser.parse_args()
    if args.check_results:
        try:
            print(json.dumps(validate(args.check_results, read(args.results), read(args.baseline)), sort_keys=True))
            if args.documents_dir:
                mcp_documents(args.documents_dir)
            return 0
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f'acceptance: {error}', file=sys.stderr)
            return 1
    args.out = args.out.resolve()
    require(not args.out.exists() or not any(args.out.iterdir()), 'use a fresh/empty output directory')
    args.out.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    summary = {'format': 'draftwright.ci-acceptance/1', 'passed': False, 'jobs': {},
               'corpus_revision': CORPUS_REV, 'corpus_revision_note': 'CI checkout pin; local files hashed below'}
    try:
        args.binary = args.binary.resolve()
        args.baseline = args.baseline.resolve()
        require(args.binary.is_file() and os.access(args.binary, os.X_OK), 'release binary missing or not executable')
        require(args.corpus is not None, 'DRAFTWRIGHT_CORPUS or --corpus is required')
        args.corpus = args.corpus.resolve()
        os.environ['DRAFTWRIGHT_CORPUS'] = str(args.corpus)
        required = [args.corpus / p for p in manifest('repeated_span_parts.txt')]
        required += [ROOT / 'tests/fixtures/parts/mounting_plate.step']
        required += sorted((ROOT / 'tests/fixtures/pmi/step').glob('*.gz'))
        required += [ROOT / 'tools/mcp-eval/features' / (name + '.features.json')
                     for name in ('plate', '10127', 'GRM-03_thumbwheel_drive_screw', 'nist_ctc_01_asme1_rd')]
        survey_names()
        for path in required:
            require(path.is_file(), f'missing fixture: {path}')
        digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
        summary['binary_sha256'] = digest(args.binary)
        summary['fixture_sha256'] = {str(p.relative_to(args.corpus) if p.is_relative_to(args.corpus)
                                         else p.relative_to(ROOT)): digest(p) for p in required}
        for job in JOBS if args.job == 'all' else (args.job,):
            out = args.out / job
            out.mkdir()
            run = Run(args, out)
            began = time.monotonic()
            run.execute(job)
            result = {'passed': all(s['passed'] for s in run.steps), 'steps': run.steps,
                      'wall_seconds': round(time.monotonic() - began, 3)}
            write(out / 'job.json', result)
            summary['jobs'][job] = result
        require(digest(args.binary) == summary['binary_sha256'], 'binary changed during acceptance')
        summary['passed'] = all(j['passed'] for j in summary['jobs'].values())
    except (OSError, ValueError, KeyError, TypeError) as error:
        summary['error'] = str(error)
        print(f'acceptance: {error}', file=sys.stderr)
    summary['wall_seconds'] = round(time.monotonic() - start, 3)
    write(args.out / 'summary.json', summary)
    print(f'acceptance: {"PASS" if summary["passed"] else "FAIL"} ({summary["wall_seconds"]}s)', flush=True)
    return 0 if summary['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
