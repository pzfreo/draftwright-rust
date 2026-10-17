"""Measure compact served JSON; attribute Spec cost by removing its value schemas.

Replace every properties.spec value with {}, then prune local reference targets no
longer reachable from JSON Schema keywords. Shared targets still used elsewhere
are charged to 'other'. The difference is the marginal Spec/spec_read byte cost,
including exclusively owned reference targets and their keys, excluding the spec
property name. Descriptions, tool names and transport envelopes are not counted.
"""
import argparse
from copy import deepcopy
import json
from pathlib import Path


def size(value):
    return len(json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode())


def without_spec(schema):
    schema = deepcopy(schema)
    targets = set()
    def refs(value):
        if isinstance(value, dict):
            if '$ref' in value:
                targets.add(value['$ref'].split('/')[1])
            for child in value.values(): refs(child)
        elif isinstance(value, list):
            for child in value: refs(child)
    refs(schema)
    def strip(value):
        if isinstance(value, dict):
            if 'spec' in value.get('properties', {}):
                value['properties']['spec'] = {}
            for child in value.values(): strip(child)
        elif isinstance(value, list):
            for child in value: strip(child)
    strip(schema)
    definitions = {key: schema.pop(key) for key in targets if key in schema}
    targets.clear()
    refs(schema)
    pending = list(targets)
    seen = set()
    while pending:
        key = pending.pop()
        if key in seen: continue
        seen.add(key)
        if key in definitions:
            schema[key] = definitions[key]
            refs(definitions[key])
            pending.extend(targets - seen)
    return schema


def measure(directory):
    rows = []
    for path in sorted(directory.glob('*.input.json')):
        name = path.name.removesuffix('.input.json')
        schemas = [json.loads(p.read_text()) for p in (path, directory / f'{name}.output.json')]
        total = sum(map(size, schemas))
        other = sum(size(without_spec(s)) for s in schemas)
        rows.append((name, *(size(s) for s in schemas), total - other, other, total))
    return rows


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, nargs='?', default=Path('schemas/mcp'))
    args = parser.parse_args()
    print('| Tool | Input | Output | Spec + spec_read | Other | Total |')
    print('| --- | ---: | ---: | ---: | ---: | ---: |')
    rows = measure(args.directory)
    for row in rows + [('TOTAL', *(sum(r[i] for r in rows) for i in range(1, 6)))]:
        print('| ' + ' | '.join(map(str, row)) + ' |')
