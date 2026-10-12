"""Acceptance gates must reject bad/incomplete evidence; no renderer mocks or extra packages."""
import copy
import json
import os
import importlib.util
import re
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / 'tools/ci_acceptance.py'
KEYS = ['off_sheet', 'repeated_label', 'repeated_measure', 'views_overlap',
        'view_on_title_block', 'ink', 'repeated_span', 'line_through_feature',
        'hidden_on_visible', 'section_hidden']
NAMES = [Path(p).stem for p in (ROOT / 'tools/quality_parts.txt').read_text().splitlines()]


class Acceptance(unittest.TestCase):
    def check(self, kind, result, baseline=None):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'result.json'
            path.write_text(json.dumps(result))
            args = [sys.executable, '-I', str(RUNNER), '--check-results', kind, '--results', str(path)]
            if baseline is not None:
                ref = Path(tmp) / 'baseline.json'
                ref.write_text(json.dumps({'metrics': KEYS, 'parts': baseline}))
                args += ['--baseline', str(ref)]
            return subprocess.run(args, cwd=ROOT, capture_output=True, text=True)

    def quality(self):
        return {name: dict.fromkeys(KEYS, 0) for name in NAMES}

    def test_quality_accepts_equal_and_improved_cells(self):
        before, after = self.quality(), self.quality()
        before[NAMES[0]]['ink'] = 1
        for actual in (before, after):
            result = self.check('quality', actual, before)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_quality_rejects_every_regressed_or_missing_column(self):
        before = self.quality()
        for key in KEYS:
            for missing in (False, True):
                after = copy.deepcopy(before)
                if missing:
                    del after[NAMES[0]][key]
                else:
                    after[NAMES[0]][key] = 1
                result = self.check('quality', after, before)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn(key, result.stderr)

    def test_quality_rejects_missing_parts_and_errors(self):
        before = self.quality()
        for error in (False, True):
            after = copy.deepcopy(before)
            if error:
                after[NAMES[0]] = {'error': 'render failed'}
            else:
                del after[NAMES[0]]
            result = self.check('quality', after, before)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('incomplete' if not error else 'render failed', result.stderr)

    def test_survey_requires_all_64_parts_and_zero_repeated_spans(self):
        names = [Path(p).stem for p in (ROOT / 'tools/repeated_span_parts.txt').read_text().splitlines()]
        names += [Path(p.name[:-3]).stem for p in (ROOT / 'tests/fixtures/pmi/step').glob('*.gz')]
        result = {name: {'repeated_span': 0} for name in names}
        self.assertEqual(len(result), 64)
        self.assertEqual(self.check('survey', result).returncode, 0)
        for bad in (dict(list(result.items())[1:]), {**result, names[0]: {'repeated_span': 1}},
                    {**result, names[0]: {'error': 'render failed'}}):
            checked = self.check('survey', bad)
            self.assertEqual(checked.returncode, 1, checked.stderr)
            self.assertIn('acceptance:', checked.stderr)

    def pins(self):
        parts = {name: {'ids': ['d:a'], 'unpinned': {'ink': 2, 'off_sheet': 0, 'off_sheet_items': []},
                       'pinned': {'ink': 3, 'off_sheet': 0, 'off_sheet_items': []}, 'go': True}
                 for name in NAMES}
        return {'seed': 1, 'pins': 1, 'go': True, 'parts': parts}

    def test_pin_bar_is_enforced_even_with_forged_go(self):
        good = self.pins()
        self.assertEqual(self.check('pins', good, self.quality()).returncode, 1,
                         'unpinned metrics must match the supplied same-run baseline')
        before = {n: p['unpinned'] for n, p in good['parts'].items()}
        self.assertEqual(self.check('pins', good, before).returncode, 0)
        for change in ({'ink': 4}, {'off_sheet': 1, 'off_sheet_items': ['d:a']}):
            bad = copy.deepcopy(good)
            bad['parts'][NAMES[0]]['pinned'].update(change)
            result = self.check('pins', bad, before)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('pin bar', result.stderr)
        bad = copy.deepcopy(good)
        bad['parts'][NAMES[0]]['unpinned']['ink'] = 0
        bad_before = {n: p['unpinned'] for n, p in bad['parts'].items()}
        self.assertIn('pin bar', self.check('pins', bad, bad_before).stderr)

    def test_pin_reports_require_complete_parts_and_requested_ids(self):
        good = self.pins()
        before = {n: p['unpinned'] for n, p in good['parts'].items()}
        for missing_part in (False, True):
            bad = copy.deepcopy(good)
            if missing_part:
                del bad['parts'][NAMES[0]]
            else:
                bad['parts'][NAMES[0]]['ids'] = []
            result = self.check('pins', bad, before)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('incomplete', result.stderr)

    def test_mcp_requires_seven_unique_successful_tasks(self):
        tasks = json.loads((ROOT / 'tools/mcp-eval/tasks.json').read_text())['tasks']
        rows = [{'task': t['id'], 'success': True, 'failed_conditions': []} for t in tasks]
        report = {'results': rows, 'summary': {'tasks': 7, 'succeeded': 7}}
        self.assertEqual(self.check('mcp', report).returncode, 0)
        for bad_rows in (rows[:-1], rows[:-1] + [rows[0]],
                         [{**rows[0], 'success': False}] + rows[1:]):
            result = self.check('mcp', {**report, 'results': bad_rows})
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('acceptance:', result.stderr)

    def test_ci_calls_local_runner_and_always_uploads_evidence(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        for required in ('quality-pins', 'repeated-spans', 'mcp', 'tools/ci_acceptance.py',
                         'if: always()', 'acceptance-inputs.tar.gz', 'timeout-minutes: ${{ matrix.timeout }}'):
            self.assertIn(required, workflow)
        self.assertNotIn('continue-on-error', workflow)

    def runner(self):
        spec = importlib.util.spec_from_file_location('ci_acceptance', RUNNER)
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)
        return runner

    def test_hosted_pin_matrix_covers_every_cell_with_bounded_jobs(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        cells = re.findall(r'job: quality-pins, label: quality-pins-(\d-\d), '
                           r'pin_args: "--pin-cell (\d-\d)", timeout: (\d+)', workflow)
        expected = {(f'{seed}-{pins}', f'{seed}-{pins}', '30')
                    for seed in (1, 2, 3) for pins in (1, 3, 8)}
        self.assertEqual(len(cells), 9)
        self.assertEqual(set(cells), expected)
        self.assertIn('${{ matrix.pin_args }}', workflow)
        self.assertIn('name: acceptance-${{ matrix.label }}', workflow)
        runner = self.runner()
        with tempfile.TemporaryDirectory() as tmp:
            for cell in runner.PIN_CELLS:
                args = SimpleNamespace(binary=Path('binary'), corpus=Path('corpus'),
                                       baseline=Path('baseline'), pin_cell=cell)
                run = runner.Run(args, Path(tmp))
                with patch.object(run, 'step') as step:
                    run.execute('quality-pins')
                calls = step.call_args_list
                self.assertEqual([c.args[0] for c in calls], ['quality', 'pins-' + cell])
                command = calls[1].args[1]
                self.assertIn('--require-go', command)
                self.assertEqual(command[command.index('--baseline') + 1],
                                 Path(tmp) / 'quality/baseline.json')
                self.assertEqual(command[command.index('--seed') + 1], int(cell[0]))
                self.assertEqual(command[command.index('--pins') + 1], int(cell[2]))

    def test_survey_budgets_allow_observed_hosted_slowdown(self):
        runner = self.runner()
        run = runner.Run(SimpleNamespace(binary=Path('binary')), Path('out'))
        with patch.object(run, 'step') as step:
            run.execute('repeated-spans')
        command = step.call_args.args[1]
        self.assertEqual(command[command.index('--timeout') + 1], 3600)
        self.assertEqual(step.call_args.kwargs['timeout'], 6600)
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertRegex(workflow, r'job: repeated-spans,.*timeout: 120')

    def test_part_timeout_retains_error_evidence(self):
        quality = self.runner().quality
        with tempfile.TemporaryDirectory() as tmp:
            for operation in ('render', 'plan'):
                expired = subprocess.TimeoutExpired(['binary', operation], 3600)
                outputs = [expired] if operation == 'render' else [SimpleNamespace(), expired]
                with patch.object(quality.subprocess, 'run', side_effect=outputs) as child:
                    name, result = quality.one('slow.step', 'binary', tmp, timeout=3600)
                self.assertEqual(name, 'slow')
                self.assertIn('3600', result['error'])
                self.assertIn(operation, result['error'])
                self.assertTrue(all(c.kwargs['timeout'] == 3600 for c in child.call_args_list))

    def test_survey_writes_all_part_errors_after_timeouts(self):
        spec = importlib.util.spec_from_file_location('survey', ROOT / 'tools/survey_repeated_spans.py')
        survey = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(survey)
        corpus = os.environ.get('DRAFTWRIGHT_CORPUS')
        if not corpus:
            self.skipTest('DRAFTWRIGHT_CORPUS required for full survey error evidence')
        with tempfile.TemporaryDirectory() as tmp:
            expired = subprocess.TimeoutExpired(['binary', 'render'], 1)
            with patch.object(sys, 'argv', ['survey', '--timeout', '1', 'binary', tmp]), \
                    patch.object(survey.quality.subprocess, 'run', side_effect=expired), \
                    patch.object(survey.quality, 'table'):
                self.assertEqual(survey.main(), 1)
            results = json.loads((Path(tmp) / 'baseline.json').read_text())
            self.assertEqual(len(results), 64)
            self.assertTrue(all('timed out' in row['error'] for row in results.values()))

    def test_failed_subprocess_reports_exit_before_missing_results(self):
        runner = self.runner()
        with tempfile.TemporaryDirectory() as tmp:
            run = runner.Run(None, Path(tmp))
            self.assertFalse(run.step('survey', [sys.executable, '-c', 'raise SystemExit(1)'],
                                      'survey', Path(tmp) / 'missing.json'))
            self.assertEqual(run.steps[0]['exit_code'], 1)
            self.assertEqual(run.steps[0]['error'], 'survey exited 1; see survey.log')

    def test_mcp_requires_readable_judge_documents_and_rendered_artifacts(self):
        tasks = json.loads((ROOT / 'tools/mcp-eval/tasks.json').read_text())['tasks']
        report = {'results': [{'task': t['id'], 'success': True, 'failed_conditions': []} for t in tasks],
                  'summary': {'tasks': 7, 'succeeded': 7}}
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            result = path / 'mcp.json'
            result.write_text(json.dumps(report))
            for task in tasks:
                directory = path / task['id']
                directory.mkdir()
                for name in ('spec', 'plan', 'baseline'):
                    (directory / (name + '.json')).write_text('{}')
                (directory / 'sheet.png').write_bytes(b'\x89PNG\r\n\x1a\nartifact')
            command = [sys.executable, '-I', str(RUNNER), '--check-results', 'mcp', '--results',
                       str(result), '--documents-dir', str(path)]
            checked = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(checked.returncode, 0, checked.stderr)
            (path / tasks[0]['id'] / 'sheet.png').unlink()
            checked = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(checked.returncode, 1, checked.stderr)
            self.assertIn('sheet.png', checked.stderr)

    def test_missing_fixtures_fail_before_starting_mcp(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run([sys.executable, '-I', str(RUNNER), '--job', 'mcp',
                                     '--binary', sys.executable, '--corpus', tmp,
                                     '--out', str(Path(tmp) / 'out')],
                                    cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn('missing fixture', result.stderr)
            evidence = json.loads((Path(tmp) / 'out/summary.json').read_text())
            self.assertFalse(evidence['passed'])


if __name__ == '__main__':
    unittest.main()
