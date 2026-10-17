"""Equivalence checks for schema compaction; use the check_mcp_schemas.py Python environment."""
import ast
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator

source = ast.parse(Path(__file__).with_name('generate_mcp_schemas.py').read_text())
namespace = {}
function = next(node for node in source.body if isinstance(node, ast.FunctionDef)
                and node.name == 'hoist_objects')
exec(compile(ast.Module(body=[function], type_ignores=[]), '<hoist_objects>', 'exec'), namespace)
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


if __name__ == '__main__':
    unittest.main()
