"""Independent draft-2020-12 validation of every MCP tool and its served schemas.

Run with jsonschema==4.25.1 installed: python tools/check_mcp_schemas.py target/release/draftwright
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from jsonschema import Draft202012Validator

root = Path(__file__).resolve().parent.parent
binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else root / 'target/release/draftwright').resolve())
with tempfile.TemporaryDirectory(prefix='s7-schema-') as cache:
    env = {**os.environ, 'DRAFTWRIGHT_CACHE': cache}
    def rpc(method, params):
        request = {'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params}
        proc = subprocess.run([binary, 'mcp-stdio'], input=json.dumps(request)+'\n', text=True, capture_output=True, env=env, check=True)
        response = json.loads(proc.stdout)
        assert 'error' not in response, response
        return response['result']
    tools = rpc('tools/list', {})['tools']
    validators = {}
    for tool in tools:
        for direction, key in [('input', 'inputSchema'), ('output', 'outputSchema')]:
            published = json.loads((root / f'schemas/mcp/{tool["name"]}.{direction}.json').read_text())
            assert published == tool[key]
            assert published.get('type') == 'object', 'MCP schema roots are objects'
            Draft202012Validator.check_schema(published)
            validators[tool['name'], direction] = Draft202012Validator(published)
    nullable = validators['inspect_part', 'input']
    assert not nullable.is_valid({'part_ref': 'part', 'part_ref\n': 'bad'}), 'literal property names stay strict at a final newline'
    for spec in ({'dimensions\n': {}}, {'sheet': {'centre_marks\n': True}},
                 {'sheet': {'fit_limits\n': True}}):
        assert not nullable.is_valid({'part_ref': 'part', 'spec': spec}), spec
    for sheet in [{'size': None, 'fit_limits': None}, {'size': 'A4', 'fit_limits': True}]:
        nullable.validate({'part_ref': 'part', 'spec': {'sheet': sheet}})
    for sheet in [{'size': 42}, {'fit_limits': 'yes'}]:
        assert not nullable.is_valid({'part_ref': 'part', 'spec': {'sheet': sheet}})
    for field in ('hidden', 'centre_marks', 'centre_lines', 'thread_depiction', 'knurl_depiction'):
        for scope in ('sheet', 'views'):
            def authored(value):
                settings = {field: value}
                return {'part_ref': 'part', 'spec': {scope: settings if scope == 'sheet' else {'side': settings}}}
            for value in (True, False, None):
                nullable.validate(authored(value))
            for value in ('false', 0, [], {}):
                assert not nullable.is_valid(authored(value)), (scope, field, value)
    declaration = {'id': 'f:declared:x', 'kind': 'hole', 'faces': []}
    nullable.validate({'part_ref': 'part', 'spec': {'feature_declarations': [declaration]}})
    for bad in ({'kind': 'hole', 'faces': [], 'replacement': 'x'},
                {**declaration, 'extra': 'x'}, {'id': 'f:declared:x', 'faces': []},
                {**declaration, 'faces': 'not-a-list'}, None, [], 'bad'):
        assert not nullable.is_valid({'part_ref': 'part', 'spec': {'feature_declarations': [bad]}}), bad
    # Named view constraints and the common object-map shape must both apply.
    assert not nullable.is_valid({'part_ref': 'part', 'spec': {'views': {'side': []}}})
    assert not nullable.is_valid({'part_ref': 'part', 'spec': {'feature_declarations': 'bad'}})
    for bad in (None, [], 'bad', {'locations': []}, {'locations': None}):
        assert not nullable.is_valid({'part_ref': 'part', 'spec': {'dimension_schemes': bad}}), bad
    assert not nullable.is_valid({'part_ref': 'part', 'spec': {'dimension_schemes\n': {}}})
    read_spec = validators['get_spec', 'output']
    def dimension_result(dimension):
        return {'spec_version': 'v', 'spec': {'dimensions': {'d:authored:test': dimension},
                'feature_declarations': [declaration]}}
    for dimension in ({}, {'kind': 'linear', 'between': ['a', 'b'], 'along': 'true'},
                      {'kind': None, 'between': None, 'along': None},
                      {'kind': 'linear', 'between': ['a', 'b'], 'along': 'x',
                       'text': {'prefix': 'TYP', 'suffix': 'AFTER PLATING'}, 'style': 'basic'}):
        read_spec.validate(dimension_result(dimension))
    for dimension in ([], None, {'kind': 'bad'}, {'between': []},
                      {'between': ['a', 'b', 'c']}, {'between': [1]}, {'along': 'bad'},
                      {'text': {'prefix': 1}}, {'text': {'unknown': 'bad'}},
                      {'text': {'prefix\n': 'bad'}}, {'style': 'bad'}):
        assert not read_spec.is_valid(dimension_result(dimension)), dimension
    def call(name, args):
        validators[name, 'input'].validate(args)
        result = rpc('tools/call', {'name': name, 'arguments': args})
        validators[name, 'output'].validate(result['structuredContent'])
        assert not result['isError'], result
        return result['structuredContent']
    step = str(root / 'tests/fixtures/parts/mounting_plate.step')
    inspected = call('inspect_part', {'step': step, 'detail': True})
    part = inspected['part_ref']
    spec = call('get_spec', {'part_ref': part})
    item = 'd:envelope/width.length'
    ops = [{'op': 'set', 'path': 'sheet/title/title', 'value': 'SCHEMA CHECK'}]
    cases = {
        'analyse': {'step': step},
        'list_items': {'part_ref': part, 'detail': True},
        'get_item': {'part_ref': part, 'id': item, 'detail': True},
        'list_problems': {'part_ref': part, 'detail': True},
        'item_options': {'part_ref': part, 'id': item},
        'preview_ops': {'part_ref': part, 'ops': ops, 'detail': True},
        'apply_ops': {'part_ref': part, 'parent': spec['spec_version'], 'ops': ops},
        'view_sheet': {'part_ref': part, 'region': [10, 10, 100, 100], 'dpi': 30, 'overlay_ids': True},
        'render_pdf': {'part_ref': part},
        'render': {'step': step, 'format': 'pdf'},
    }
    for name, args in cases.items():
        call(name, args)
    for item_id in ('face:0', 'f:hole:1'):
        call('item_options', {'part_ref': part, 'id': item_id})
    call('inspect_part', {'part_ref': part, 'problem_filter': {'feature': 'f:hole:4'}})
    call('list_problems', {'part_ref': part, 'filter': {'feature': 'f:hole:4'}})
    corrected = rpc('tools/call', {'name': 'preview_ops', 'arguments': {
        'part_ref': part, 'ops': [{'op': 'set', 'path': 'views/v:iso/omit', 'value': True}]}})
    assert corrected['isError'] and 'Corrected op:' in corrected['structuredContent']['error']['message']
    validators['preview_ops', 'output'].validate(corrected['structuredContent'])
    for tool in tools:
        result = rpc('tools/call', {'name': tool['name'], 'arguments': {'unknown_field': True}})
        assert result['isError']
        validators[tool['name'], 'output'].validate(result['structuredContent'])
    sheets = {'sheets': {'max': 2}, 'views': {'iso': {'sheet': 2}}}
    summary = call('inspect_part', {'part_ref': part, 'spec': sheets})
    assert summary['sheets'] == len(summary['sheet_details']) == 2
    local = call('list_items', {'part_ref': part, 'spec': sheets, 'filter': {'sheet': 2}})
    assert local['items'] and all(i['sheet'] == 2 for i in local['items'])
    for tool in ('get_item', 'item_options'):
        call(tool, {'part_ref': part, 'spec': sheets, 'id': 'sheet:2/v:iso'})
    call('render', {'part_ref': part, 'spec': sheets, 'sheet': 2,
                    'dpi': 30, 'overlay_ids': ['sheet:2/v:iso']})
    for sheet in (0, 10):
        assert not validators['render', 'input'].is_valid({'part_ref': part, 'sheet': sheet})
    wrong = rpc('tools/call', {'name': 'get_item', 'arguments': {
        'part_ref': part, 'spec': sheets, 'id': 'sheet:1/v:iso'}})
    assert wrong['isError'] and 'sheet:2/v:iso' in wrong['structuredContent']['error']['choices']
    validators['get_item', 'output'].validate(wrong['structuredContent'])
    schemes = {'dimension_schemes': {'locations': {
        'scheme': 'chain', 'axis': 'x', 'view': 'plan'}}}
    call('inspect_part', {'part_ref': part, 'spec': schemes})
    saved = call('get_spec', {'part_ref': part, 'spec': schemes})
    assert saved['spec']['dimension_schemes'] == schemes['dimension_schemes']
    assert not validators['inspect_part', 'input'].is_valid({
        'part_ref': part, 'spec': {'dimension_schemes': {'locations': False}}})
    bad_scheme = rpc('tools/call', {'name': 'inspect_part', 'arguments': {
        'part_ref': part, 'spec': {'dimension_schemes': {'locations': {
            'scheme': 'baseline', 'axis': 'x', 'view': 'plan', 'from': 'face:missing'}}}}})
    assert bad_scheme['isError']
    refusal = bad_scheme['structuredContent']
    validators['inspect_part', 'output'].validate(refusal)
    assert refusal['error']['code'] == 'invalid'
    assert 'dimension_schemes/locations' in refusal['error']['message']
    assert 'remove' in refusal['error']['message']
    print(f'{len(tools)} tool inputs, outputs and refusals validated; {len(validators)} published and served schemas agree')
