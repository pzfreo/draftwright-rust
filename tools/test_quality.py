"""Go/no-go instrument contracts; run with python3 -I tools/test_quality.py."""
import importlib.util
import pathlib
from pathlib import Path
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
                                 'views_overlap', 'view_on_title_block', 'ink', 'repeated_span',
                                 'line_through_feature', 'hidden_on_visible', 'section_hidden',
                                 'claimed_faces', 'faces', 'unplanned_features')}
        with tempfile.TemporaryDirectory() as out:
            baseline = pathlib.Path(out) / 'source.json'
            for repeated in (0, 1):
                metrics['repeated_span'] = repeated
                baseline.write_text(json.dumps({'part': metrics}))
                argv = ['quality.py', '--require-no-repeated-span', '--baseline', str(baseline),
                        'dw', out, 'part.step']
                with patch.object(sys, 'argv', argv), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(quality.main(), repeated)


class CentreLineInk(unittest.TestCase):
    def test_a_real_label_centreline_contact_counts_as_ink(self):
        plan = {'sheet': {'scale': '1:1'}, 'dimensions': [], 'problems': [],
                'views': [{'line_work': [{'kind': 'cross'}]}]}
        replies = [subprocess.CompletedProcess([], 0, '{}', ''),
            subprocess.CompletedProcess([], 0, json.dumps(plan), ''),
            subprocess.CompletedProcess([], 0, json.dumps({'structural': [
                {'code': 'label_centerline_overlap', 'annotation': 'd:a'}], 'lint': []}), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies):
            (Path(out) / 'part.svg').write_text('<svg/>')
            _, result = quality.one('part.step', 'dw', out)
        self.assertEqual(result['ink'], 1)
        self.assertEqual(result['ink_by_code'], {'label_centerline_overlap': 1})


class FeatureCrossings(unittest.TestCase):
    def test_counts_plan_feature_pairs_and_extension_lines(self):
        plan = {'sheet': {'scale': '1:1'}, 'dimensions': [], 'views': [],
                'completeness': {'claimed_faces': 0, 'faces': 0, 'unplanned_features': 0},
                'problems': [{'code': 'leader_through_feature', 'subjects': ['d:a', 'f:b']},
                             {'code': 'leader_through_feature', 'subjects': ['d:a', 'f:c']},
                             {'code': 'extension_through_feature', 'id': 'p:d:e|f:b:line:0', 'subjects': ['d:e', 'f:b']},
                             {'code': 'extension_through_feature', 'id': 'p:d:e|f:c:line:0', 'subjects': ['d:e', 'f:c']},
                             {'code': 'extension_through_feature', 'id': 'p:d:e|f:c:line:1', 'subjects': ['d:e', 'f:c']},
                             {'code': 'leader_through_feature', 'subjects': ['d:a', 'f:b']}]}
        replies = [subprocess.CompletedProcess([], 0, '{}', ''),
                   subprocess.CompletedProcess([], 0, json.dumps(plan), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies):
            (Path(out) / 'part.svg').write_text('<svg/>')
            _, result = quality.one('part.step', 'dw', out)
        self.assertEqual(result['line_through_feature'], 4)

    def test_status_headers_select_shared_columns_and_reject_a_regression(self):
        checker = Path(__file__).with_name('check_quality_table.py')
        old = 'part off_sheet repeated_lab repeated_mea views_overla view_on_titl ink\np 0 0 0 0 0 1\nTOTAL 0 0 0 0 0 1\n'
        new = 'part ink line_through off_sheet\np 1 2 0\nTOTAL 1 2 0\n'
        with tempfile.TemporaryDirectory() as out:
            status, baseline = Path(out) / 'status.md', Path(out) / 'baseline.json'
            baseline.write_text(json.dumps({'p': {'off_sheet': 0, 'repeated_label': 0,
                'repeated_measure': 0, 'views_overlap': 0, 'view_on_title_block': 0,
                'ink': 1, 'line_through_feature': 3}}))
            for table, expected in [(old, 0), (new.replace("p 1 2 0", "p 1 3 0"), 0), (new, 1)]:
                status.write_text(table)
                result = subprocess.run([sys.executable, '-I', str(checker), str(status), str(baseline)], capture_output=True)
                self.assertEqual(result.returncode, expected, result.stderr.decode())


class Linework(unittest.TestCase):
    def measure(self, visible, hidden, views=None, width=0):
        views = views or [{'id': 'v:front', 'area': [-20, -20, 20, 20]}]
        with tempfile.TemporaryDirectory() as out:
            path = pathlib.Path(out) / 'sheet.svg'
            path.write_text(f'<svg><g id="part" stroke-width="{width}">{visible}</g><g id="hidden">{hidden}</g></svg>')
            return quality.svg_linework(path, {'views': views})

    def test_page_length_and_distance_thresholds(self):
        visible = '<line x1="0" y1="0" x2="10" y2="0"/>'
        for length, distance, count in [(2, .14, 1), (2, .16, 0), (.9, 0, 0)]:
            hidden = f'<path d="M 0 {distance} L {length} {distance}"/>'
            self.assertEqual(self.measure(visible, hidden)['hidden_on_visible'], count)

    def test_visible_stroke_width_is_ink(self):
        visible = '<line x1="0" y1="0" x2="10" y2="0"/>'
        for offset, count in ((.39,1),(.41,0)):
            hidden = f'<line x1="0" y1="{offset}" x2="10" y2="{offset}"/>'
            self.assertEqual(self.measure(visible, hidden, width=.5)['hidden_on_visible'], count)

    def test_arc_coincidence_and_partial_overlap(self):
        self.assertEqual(self.measure('<circle cx="0" cy="0" r="5"/>',
                         '<path d="M 5 0 A 5 5 0 0 1 0 5"/>')['hidden_on_visible'], 1)
        self.assertEqual(self.measure('<line x1="0" y1="0" x2="3" y2="0"/>',
                         '<line x1="2" y1="0" x2="7" y2="0"/>')['hidden_on_visible'], 1)

    def test_section_counts_elements_and_separates_views(self):
        views = [{'id': 'v:front', 'area': [0,0,10,10]},
                 {'id': 'v:section_aa', 'area': [20,0,30,10], 'cut': {'normal': [0,1,0]}}]
        hidden = '<line x1="21" y1="1" x2="29" y2="1"/>'
        visible = '<line x1="1" y1="1" x2="9" y2="1"/>'
        self.assertEqual(self.measure(visible, hidden, views),
                         {'hidden_on_visible': 0, 'section_hidden': 1})

    def test_checker_reads_header_and_rejects_each_linework_regression(self):
        checker = pathlib.Path(__file__).with_name('check_quality_table.py')
        with tempfile.TemporaryDirectory() as out:
            status, baseline = [pathlib.Path(out) / x for x in ('status.md', 'baseline.json')]
            status.write_text('part off_sheet repeated_lab section_hidd hidden_on_vi\npart1 0 0 0 0\nTOTAL 0 0 0 0\n')
            for key in ('section_hidden', 'hidden_on_visible', 'off_sheet'):
                baseline.write_text(json.dumps({'part1': {'section_hidden': 0, 'hidden_on_visible': 0,
                                                          'off_sheet': 0, 'repeated_label': 0, key: 1}}))
                self.assertNotEqual(subprocess.run([sys.executable, '-I', str(checker), str(status),
                                                   str(baseline)], capture_output=True).returncode, 0)
            baseline.write_text(json.dumps({'part1': {'section_hidden': 0, 'hidden_on_visible': 0, 'off_sheet': 0, 'repeated_label': 0}}))
            self.assertEqual(subprocess.run([sys.executable, '-I', str(checker), str(status),
                                            str(baseline)], capture_output=True).returncode, 0)


class Completeness(unittest.TestCase):
    def test_quality_records_face_counts_and_unplanned_features(self):
        plan = {'sheet': {'scale': '1:1'}, 'views': [], 'dimensions': [],
                'completeness': {'claimed_faces': 3, 'faces': 52, 'unplanned_features': 7}}
        replies = [subprocess.CompletedProcess([], 0, '{}', ''),
                   subprocess.CompletedProcess([], 0, json.dumps(plan), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies), patch.object(quality, 'svg_linework', return_value={}):
            _, result = quality.one('part.step', 'dw', out)
        self.assertEqual(result['claimed_faces'], 3)
        self.assertEqual(result['faces'], 52)
        self.assertEqual(result['unplanned_features'], 7)

    def test_missing_completeness_is_unavailable_instead_of_perfect(self):
        plan = {'sheet': {'scale': '1:1'}, 'views': [], 'dimensions': []}
        replies = [subprocess.CompletedProcess([], 0, '{}', ''),
                   subprocess.CompletedProcess([], 0, json.dumps(plan), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies), patch.object(quality, 'svg_linework', return_value={}):
            _, result = quality.one('part.step', 'dw', out)
        self.assertIn('error', result)

    def test_checker_rejects_less_coverage_more_unplanned_or_changed_denominator(self):
        checker = Path(__file__).with_name('check_quality_table.py')
        with tempfile.TemporaryDirectory() as out:
            status, baseline = Path(out) / 'status.md', Path(out) / 'baseline.json'
            status.write_text('part ink claimed_face faces unplanned_fe\np 0 3 52 7\nTOTAL 0 3 52 7\n')
            for claimed, faces, unplanned, code in [(3,52,7,0),(4,52,6,0),(2,52,7,1),(3,52,8,1),(3,53,7,1)]:
                baseline.write_text(json.dumps({'p': {'ink': 0, 'claimed_faces': claimed,
                                                      'faces': faces, 'unplanned_features': unplanned}}))
                result = subprocess.run([sys.executable, '-I', str(checker), str(status), str(baseline)], capture_output=True)
                self.assertEqual(result.returncode, code, result.stderr.decode())


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
        plan['completeness'] = {'claimed_faces': 0, 'faces': 0, 'unplanned_features': 0}
        plan['dimensions'] = []
        render = {'problems': [
            {'code': 'annotation_off_sheet', 'subjects': ['d:a/length']},
            {'code': 'pinned_view_off_sheet', 'subjects': ['v:plan']}]}
        replies = [subprocess.CompletedProcess([], 0, json.dumps(render), ''),
                   subprocess.CompletedProcess([], 0, json.dumps(plan), '')]
        with tempfile.TemporaryDirectory() as out, patch.object(quality.subprocess, 'run', side_effect=replies), patch.object(quality, 'svg_linework', return_value={'hidden_on_visible': 0, 'section_hidden': 0}):
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
