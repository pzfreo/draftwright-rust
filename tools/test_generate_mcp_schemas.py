"""Equivalence checks for schema compaction; use the check_mcp_schemas.py Python environment."""
import ast
import subprocess
import sys
import tempfile
import re
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator

source = ast.parse(Path(__file__).with_name('generate_mcp_schemas.py').read_text())
namespace = {'re': re}
for name in ('hoist_objects', 'name_choices'):
    function = next(node for node in source.body if isinstance(node, ast.FunctionDef)
                    and node.name == name)
    exec(compile(ast.Module(body=[function], type_ignores=[]), '<compaction>', 'exec'), namespace)
hoist = namespace['hoist_objects']


class Hoisting(unittest.TestCase):
    def test_object_union_preserves_types_required_and_closed_properties(self):
        schema = {'anyOf': [
            {'type': 'object', 'properties': {'kind': {'const': 'shown'}},
             'required': ['kind'], 'additionalProperties': False},
            {'type': 'object', 'properties': {'kind': {'const': 'covered'}, 'by': {'type': 'string'}},
             'required': ['kind', 'by'], 'additionalProperties': False},
        ]}
        compact = hoist(schema)
        self.assertEqual(compact['type'], 'object')
        self.assertTrue(all('type' not in branch for branch in compact['anyOf']))
        old, new = map(Draft202012Validator, (schema, compact))
        for value in [None, True, 3, 'shown', [], {}, {'kind': 'shown'},
                      {'kind': 'shown', 'extra': True}, {'kind': 'covered'},
                      {'kind': 'covered', 'by': 'd:1'}, {'kind': 'covered', 'by': 1}]:
            self.assertEqual(old.is_valid(value), new.is_valid(value), value)

    def test_nullable_and_mixed_unions_keep_their_types(self):
        for other in ['null', 'array', 'string']:
            schema = {'anyOf': [{'type': 'object'}, {'type': other}]}
            self.assertEqual(hoist(schema), schema)

    def test_nested_unions_do_not_mutate_shared_input(self):
        inner = {'anyOf': [{'type': 'object', 'required': ['a']},
                           {'type': 'object', 'required': ['b']}]}
        schema = {'type': 'array', 'items': inner}
        compact = hoist(schema)
        self.assertNotIn('type', inner)
        for value in [None, [], [{}], [{'a': 1}], [{'b': 1}], ['a']]:
            self.assertEqual(Draft202012Validator(schema).is_valid(value),
                             Draft202012Validator(compact).is_valid(value), value)


class PropertyNames(unittest.TestCase):
    def test_prefix_and_suffix_groups_match_only_the_original_names(self):
        names = ['hidden', 'centre_marks', 'centre_lines', 'thread_depiction', 'knurl_depiction']
        pattern = '^(' + namespace['name_choices'](names) + ')$(?!\\n)'
        old = Draft202012Validator({'type': 'object', 'properties': {n: {'type': 'boolean'} for n in names}, 'additionalProperties': False})
        new = Draft202012Validator({'type': 'object', 'patternProperties': {pattern: {'type': 'boolean'}}, 'additionalProperties': False})
        self.assertLess(len(pattern), len('^(' + '|'.join(names) + ')$(?!\\n)'))
        for name in names + ['centre_depiction', 'thread_marks', 'knurl_lines', 'depiction', '', 'thread_depiction_extra'] + [n + '\n' for n in names]:
            for value in [True, False, None, 1, 'false', {}, []]:
                self.assertEqual(old.is_valid({name: value}), new.is_valid({name: value}), (name, value))


class SpecContextBudget(unittest.TestCase):
    def test_new_nested_spec_field_does_not_change_any_advertised_schema(self):
        generator = Path(__file__).with_name('generate_mcp_schemas.py').resolve()
        original = generator.read_text()
        changed = original.replace("full_spec = document()", """full_spec = document()
full_spec['$defs']['sheet']['properties']['future_setting'] = {'type': 'string'}
spec['properties']['sheet']['properties']['future_setting'] = {'type': 'string'}""")
        outputs = []
        full = []
        private = []
        for source in (original, changed):
            with tempfile.TemporaryDirectory(prefix='mcp-outline-') as directory:
                root = Path(directory)
                (root / 'schemas/mcp').mkdir(parents=True)
                # __file__ keeps the source root for MAX_OPS; only outputs use cwd.
                subprocess.run([sys.executable, '-c',
                    "import sys; sys.path.insert(0, sys.argv[1]); "
                    "exec(compile(sys.stdin.read(), sys.argv[2], 'exec'), {'__file__': sys.argv[2]})",
                    str(generator.parent), str(generator)], input=source, text=True,
                    cwd=root, check=True, capture_output=True)
                outputs.append({p.name: p.read_bytes() for p in (root / 'schemas/mcp').glob('*.json')})
                full.append((root / 'schemas/spec.json').read_bytes())
                private.append((root / 'schemas/mcp-validation.json').read_bytes())
        self.assertEqual(len(outputs[0]), 28)
        repository = generator.parents[1]
        self.assertEqual(outputs[0], {p.name: p.read_bytes()
            for p in (repository / 'schemas/mcp').glob('*.json')})
        self.assertEqual(full[0], (repository / 'schemas/spec.json').read_bytes())
        self.assertEqual(private[0], (repository / 'schemas/mcp-validation.json').read_bytes())
        self.assertEqual(outputs[0], outputs[1])
        self.assertNotEqual(full[0], full[1])
        self.assertNotEqual(private[0], private[1])

    def test_full_schema_covers_every_current_nested_spec_field(self):
        from mcp_spec import document
        root = Path(__file__).resolve().parents[1] / 'src'
        definitions = document()['$defs']
        types = {
            'model': {'Binding': 'binding', 'SheetSpec': 'sheet', 'ViewSpec': 'view',
                      'DimensionSpec': 'dimension', 'FeatureSpec': 'feature'},
            'sheets': {'SheetsSpec': 'sheets'},
            'placement_intent': {'Place': 'place', 'ViewPlace': 'view_place', 'AnnotationSpec': 'annotation'},
            'label_text': {'Text': 'text'},
            'spec_gdt': {'ControlSpec': 'control', 'FaceSpec': 'face'},
            'spec_notes': {'FinishSpec': 'finish', 'ThreadSpec': 'thread', 'KnurlSpec': 'knurl'},
            'recognition_corrections': {'Anchor': 'anchor', 'Declaration': 'declaration', 'PatternSpec': 'pattern'},
            'dimension_scheme': {'Scheme': 'scheme'}, 'notes': {'NoteSpec': 'note'},
            'tables': {'TableSpec': 'table'}, 'gears': {'GearSpec': 'gear'},
        }
        for module, structs in types.items():
            source = (root / (module + '.rs')).read_text()
            for struct, definition in structs.items():
                fields = source.split('pub struct ' + struct + ' {', 1)[1].split('\n}', 1)[0]
                self.assertEqual(set(re.findall(r'pub (\w+):', fields)),
                                 set(definitions[definition]['properties']), struct)

    def test_outline_covers_all_spec_keys_with_closed_top_level(self):
        from mcp_spec import document, outline
        full, shallow = document(), outline()
        self.assertEqual(set(full['properties']), set(shallow['properties']))
        self.assertIs(shallow['additionalProperties'], False)
        self.assertEqual(full['additionalProperties'], shallow['additionalProperties'])
        self.assertIn('spec_schema', shallow['description'])
        for key, shape in shallow['properties'].items():
            self.assertTrue(set(shape) <= {'type', 'enum'}, key)
        # Also guard drift from the actual Rust document model.
        model = Path(__file__).resolve().parents[1] / 'src/model.rs'
        spec = model.read_text().split('pub struct Spec {', 1)[1].split('\n}', 1)[0]
        self.assertEqual(set(re.findall(r'pub (\w+):', spec)), set(shallow['properties']))


if __name__ == '__main__':
    unittest.main()
