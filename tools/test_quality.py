"""Go/no-go instrument contracts; run with python3 -I tools/test_quality.py."""
import importlib.util
import pathlib
import unittest
from unittest.mock import patch
import json
import subprocess
import tempfile
import contextlib
import io
import sys

spec = importlib.util.spec_from_file_location('quality', pathlib.Path(__file__).with_name('quality.py'))
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)


class RepeatedSpans(unittest.TestCase):
    def dimension(self, id_, ends=(10, 30), lane=0, rotation=0, value=20, feature=None):
        tips = [[x, lane] if rotation == 0 else [lane, x] for x in ends]
        return {'id': id_, 'kind': 'linear', 'view': 'v:plan', 'value': value,
                'label': str(value), 'measurements': [{'feature': feature or id_,
                                                     'parameter': 'linear'}],
                'placed': {'rotation': rotation,
                           'arrows': [[tip, tip] for tip in tips]}}

    def groups(self, dims, scale='1:1'):
        return quality.repeated_spans({'sheet': {'scale': scale}, 'dimensions': dims})

    def test_span_ignores_lane_label_and_witness_order(self):
        a = self.dimension('a')
        b = self.dimension('b', ends=(30, 10), lane=70)
        b['label'] = '20 ±0.1'
        self.assertEqual(self.groups([b, a]), [{'kind': 'span', 'items': ['a', 'b']}])

    def test_tolerance_is_in_model_mm_without_rounding_buckets(self):
        for scale, factor in [('1:5', 0.2), ('5:1', 5)]:
            a = self.dimension('a', ends=(10 * factor, 30 * factor))
            b = self.dimension('b', ends=((10 + 0.9e-6) * factor, 30 * factor))
            self.assertEqual(len(self.groups([a, b], scale)), 1)
            b = self.dimension('b', ends=((10 + 1.1e-6) * factor, 30 * factor))
            self.assertEqual(self.groups([a, b], scale), [])

    def test_distinct_axes_values_views_and_unplaced_items_do_not_repeat(self):
        a = self.dimension('a')
        for b in [self.dimension('b', rotation=90), self.dimension('b', value=21),
                  dict(self.dimension('b'), view='v:front'),
                  dict(self.dimension('b'), placed=None),
                  dict(self.dimension('b'), kind='callout')]:
            self.assertEqual(self.groups([a, b]), [])

    def test_same_face_pair_at_different_stations_is_a_near_miss(self):
        a = self.dimension('a', feature='pmi:dimension.linear:f2,8')
        b = self.dimension('b', ends=(40, 60), feature='pmi:dimension.linear.basic:f2,8')
        self.assertEqual(self.groups([a, b]), [{'kind': 'feature_pair', 'items': ['a', 'b']}])
        b['measurements'][0]['feature'] = 'pmi:dimension.linear:f3,8'
        self.assertEqual(self.groups([a, b]), [])

    def test_three_dimensions_count_as_two_extras_in_one_group(self):
        dims = [self.dimension(id_, lane=i * 7) for i, id_ in enumerate('cba')]
        self.assertEqual(self.groups(dims), [{'kind': 'span', 'items': ['a', 'b', 'c']}])

    def test_geometric_noise_in_values_is_not_a_different_requirement(self):
        a = self.dimension('a')
        b = self.dimension('b', value=20 + 1e-9)
        self.assertEqual(len(self.groups([a, b])), 1)
        b['value'] = 20 + 2e-6
        self.assertEqual(self.groups([a, b]), [])

    def test_face_indices_in_different_parts_are_not_the_same_pair(self):
        a = self.dimension('a', feature='pmi:dimension.linear:p0:f2,8')
        b = self.dimension('b', ends=(40, 60), feature='pmi:dimension.linear:p1:f2,8')
        self.assertEqual(self.groups([a, b]), [])

    def test_cli_can_enforce_zero_repeated_spans(self):
        metrics = {k: 0 for k in ('off_sheet', 'repeated_label', 'repeated_measure',
                                 'views_overlap', 'view_on_title_block', 'ink', 'repeated_span')}
        with tempfile.TemporaryDirectory() as out:
            baseline = pathlib.Path(out) / 'source.json'
            for repeated in (0, 1):
                metrics['repeated_span'] = repeated
                baseline.write_text(json.dumps({'part': metrics}))
                argv = ['quality.py', '--require-no-repeated-span', '--baseline', str(baseline),
                        'dw', out, 'part.step']
                with patch.object(sys, 'argv', argv), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(quality.main(), repeated)


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
        plan['sheet'] = {'scale': '1:1'}
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
