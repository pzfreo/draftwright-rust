"""Go/no-go instrument contracts; run with python3 -I tools/test_quality.py."""
import importlib.util
import pathlib
import unittest
from unittest.mock import patch
import json
import subprocess
import tempfile

spec = importlib.util.spec_from_file_location('quality', pathlib.Path(__file__).with_name('quality.py'))
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)


class RandomPins(unittest.TestCase):
    def plan(self):
        return {'dimensions': [{'id': 'd:a/length', 'placed': {}, 'state': {'kind': 'shown'}},
                               {'id': 'd:covered', 'placed': None}],
                'gdt': [{'id': 'pmi:a', 'placed': {}, 'state': {'kind': 'shown'}}],
                'views': [{'id': 'v:front', 'area': [0, 0, 10, 10]},
                          {'id': 'v:plan', 'area': [0, 20, 10, 30]}]}

    def test_selection_is_repeatable_and_pins_all_supported_kinds(self):
        plan = self.plan()
        spec, ids = quality.random_pins(plan, 'part', 1, 3)
        self.assertEqual((spec, ids), quality.random_pins(plan, 'part', 1, 3))
        self.assertEqual(set(ids), {'d:a/length', 'pmi:a', 'v:plan'})
        self.assertTrue(spec['dimensions']['d:a/length']['place']['pinned'])
        self.assertTrue(spec['annotations']['pmi:a']['place']['pinned'])
        view = spec['views']['plan']['place']
        self.assertEqual(view, {'relative_to': 'front', 'side': 'above', 'gap': 10,
                                'align': 'middle', 'pinned': True})
        self.assertNotIn('at', view)

    def test_not_enough_items_is_an_error(self):
        with self.assertRaises(ValueError):
            quality.random_pins(self.plan(), 'part', 1, 8)

    def test_selection_is_independent_of_plan_order_and_nested_for_n(self):
        plan = self.plan()
        one = quality.random_pins(plan, 'part', 2, 1)[1]
        plan['dimensions'].reverse()
        three = quality.random_pins(plan, 'part', 2, 3)[1]
        self.assertEqual(one, three[:1])

    def test_off_sheet_measurement_includes_pinned_views(self):
        plan = self.plan()
        plan['sheet'] = {}
        plan['dimensions'] = []
        render = {'problems': [
            {'code': 'annotation_off_sheet', 'subjects': ['d:a/length']},
            {'code': 'pinned_view_off_sheet', 'subjects': ['v:plan']}]}
        replies = [subprocess.CompletedProcess([], 0, json.dumps(render), ''),
                   subprocess.CompletedProcess([], 0, json.dumps(plan), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies):
            _, result = quality.one('part.step', 'dw', out)
        self.assertEqual(result['off_sheet'], 2)
        self.assertEqual(result['off_sheet_items'], ['d:a/length', 'v:plan'])

    def test_bar_is_per_part_and_zero_ink_means_zero(self):
        self.assertTrue(quality.pin_bar({'ink': 2, 'off_sheet': 0}, {'ink': 3, 'off_sheet': 0}))
        self.assertFalse(quality.pin_bar({'ink': 0, 'off_sheet': 0}, {'ink': 1, 'off_sheet': 0}))
        self.assertFalse(quality.pin_bar({'ink': 10, 'off_sheet': 0}, {'ink': 0, 'off_sheet': 1}))
        self.assertFalse(quality.pin_bar({'ink': 2, 'off_sheet': 0}, {'ink': 4, 'off_sheet': 0}))
        self.assertFalse(quality.pin_bar({'ink': 2, 'off_sheet': 1, 'off_sheet_items': ['a']},
                                        {'ink': 2, 'off_sheet': 1, 'off_sheet_items': ['b']}))


if __name__ == '__main__':
    unittest.main()
