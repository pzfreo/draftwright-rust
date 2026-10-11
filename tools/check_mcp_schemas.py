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
    print(f'{len(tools)} tool inputs, outputs and refusals validated; {len(validators)} published and served schemas agree')
