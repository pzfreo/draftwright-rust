"""Acceptance instrument regressions, including pre-H7 quality records."""
import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from check_interior import check


class InteriorAcceptance(unittest.TestCase):
    def runs(self, root):
        before, after = root / 'before', root / 'after'
        before.mkdir()
        after.mkdir()
        rows = {}
        for n in range(30):
            name = f'part{n}'
            rows[name] = dict.fromkeys((
                'off_sheet', 'repeated_label', 'repeated_measure', 'views_overlap',
                'view_on_title_block', 'ink', 'repeated_span', 'line_through_feature',
                'hidden_on_visible', 'section_hidden', 'unplanned_features'), 0)
            rows[name].update(faces=10, claimed_faces=10)
            dims = [{'id': 'd:a', 'kind': 'callout', 'placed': {}},
                    {'id': 'd:b', 'kind': 'linear', 'placed': {}}]
            plan = {'dimensions': dims, 'problems': [], 'sheet': {'size': 'A3'}}
            if n == 0:
                plan['problems'] = [{'code': 'layout_replanned',
                    'message': 'The sheet was re-planned: A4 at 1:1 to A3 at 1:1.'}]
            (before / f'{name}.plan.json').write_text(json.dumps(plan))
            for d in dims:
                d['region'] = 'interior'
            plan['problems'] = []
            (after / f'{name}.plan.json').write_text(json.dumps(plan))
        for directory in (before, after):
            (directory / 'baseline.json').write_text(json.dumps(rows))
        return before, after

    def test_baselines_without_new_columns_are_recomputed_from_plans(self):
        with tempfile.TemporaryDirectory() as temp:
            before, after = self.runs(pathlib.Path(temp))
            with contextlib.redirect_stdout(io.StringIO()) as output:
                check(before, after)
            totals = json.loads(output.getvalue())['totals']
            self.assertEqual(totals, {'interior_labels': [0, 60],
                'outer_labels': [60, 0], 'sheet_escalations': [1, 0]})

    def test_a_regression_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            before, after = self.runs(pathlib.Path(temp))
            file = after / 'baseline.json'
            rows = json.loads(file.read_text())
            rows['part0']['ink'] = 1
            file.write_text(json.dumps(rows))
            with self.assertRaisesRegex(AssertionError, 'part0: ink'):
                check(before, after)


if __name__ == '__main__':
    unittest.main()
