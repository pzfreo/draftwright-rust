#!/usr/bin/env python3
"""Check integration-module reachability and nextest's unfiltered test inventory.

The source walk understands ordinary Rust modules, #[path], #[test], and the
part_cases!/pmi_cases! generators in tests/common/support.rs. It compares names,
not just totals, so a lost test cannot be hidden by an unrelated added test.
Run without arguments in lint; pass --nextest-list after the release build.
"""
import argparse
import json
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
# Keep strings atomic (including raw strings), and discard comments before looking
# for modules or attributes. Braces in fixture JSON must not affect the walk.
TOKEN = re.compile(
    r'//[^\n]*|/\*|r(?P<hashes>\#*)"[\s\S]*?"(?P=hashes)'
    r'|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\''
    r'|[A-Za-z_][A-Za-z_0-9]*|[^\s]', re.DOTALL
)


def tokens(text):
    out = []
    pos = 0
    while match := TOKEN.search(text, pos):
        token = match.group()
        pos = match.end()
        if token == '/*':
            depth = 1
            while depth:
                comment = re.search(r'/\*|\*/', text[pos:])
                if comment is None:
                    raise ValueError('unterminated Rust comment')
                depth += 1 if comment.group() == '/*' else -1
                pos += comment.end()
        elif not token.startswith('//'):
            out.append(token)
    return out


def closing(ts, start):
    pairs = {'(': ')', '[': ']', '{': '}'}
    stack = [pairs[ts[start]]]
    for i in range(start + 1, len(ts)):
        if ts[i] in pairs:
            stack.append(pairs[ts[i]])
        elif ts[i] == stack[-1]:
            stack.pop()
            if not stack:
                return i
    raise ValueError(f'unclosed Rust delimiter near {ts[max(0, start - 8):start + 12]}')


def inventory(root=ROOT):
    root = Path(root).resolve()
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    if manifest['package'].get('autotests', True):
        raise ValueError('set package.autotests = false and declare every [[test]] target')
    reached = set()
    expected = {}
    pmi = tokens((root / 'tests/common/support.rs').read_text())
    start = next(i for i in range(len(pmi)) if pmi[i:i + 3] == ['macro_rules', '!', 'pmi_files']) + 3
    pmi = pmi[start:closing(pmi, start) + 1]
    pmi_names = [pmi[i - 1] for i in range(1, len(pmi) - 2)
                 if pmi[i:i + 2] == ['=', '>'] and pmi[i + 2].startswith('"')]

    def walk_file(path, prefix, names, ancestors=()):
        path = path.resolve()
        if path in ancestors:
            raise ValueError(f'cyclic module: {path}')
        reached.add(path)
        ts = tokens(path.read_text())
        module_dir = path.parent if path.name in ('main.rs', 'mod.rs') else path.with_suffix('')
        try:
            walk(ts, path.parent, module_dir, prefix, names, (*ancestors, path))
        except ValueError as error:
            raise ValueError(f'{path.relative_to(root)}: {error}') from error

    def walk(ts, path_dir, module_dir, prefix, names, ancestors):
        i = 0
        attrs = []
        while i < len(ts):
            t = ts[i]
            if t == '#' and i + 1 < len(ts) and ts[i + 1] in ('[', '!'):
                start = i + 1 + (ts[i + 1] == '!')
                end = closing(ts, start)
                attrs.append(ts[start + 1:end])
                i = end + 1
                continue
            if t == 'mod' and i + 2 < len(ts):
                name, kind = ts[i + 1:i + 3]
                child_prefix = prefix + name + '::'
                if kind == ';':
                    explicit = [a[2] for a in attrs if a[:2] == ['path', '=']]
                    if explicit:
                        path = path_dir / json.loads(explicit[-1])
                    else:
                        candidates = [module_dir / (name + '.rs'), module_dir / name / 'mod.rs']
                        found = [p for p in candidates if p.is_file()]
                        if len(found) != 1:
                            raise ValueError(f'{name}: expected one module file, found {found}')
                        path = found[0]
                    walk_file(path, child_prefix, names, ancestors)
                    i += 3
                elif kind == '{':
                    end = closing(ts, i + 2)
                    walk(ts[i + 3:end], module_dir / name, module_dir / name,
                         child_prefix, names, ancestors)
                    i = end + 1
                else:
                    raise ValueError(f'unsupported module syntax: {ts[i:i + 4]}')
                attrs = []
                continue
            if t == 'fn' and ['test'] in attrs:
                name = prefix + ts[i + 1]
                if name in names:
                    raise ValueError(f'duplicate test: {name}')
                names.add(name)
            if t in ('part_cases', 'pmi_cases') and ts[i + 1:i + 3] == ['!', '(']:
                end = closing(ts, i + 2)
                args = ts[i + 3:end]
                cases = pmi_names if t == 'pmi_cases' else [
                    args[j - 1] for j in range(1, len(args) - 1) if args[j:j + 2] == ['=', '>']]
                if not cases:
                    raise ValueError(f'empty {t} invocation')
                names.update(prefix + args[0] + '::' + case for case in cases)
                i = end + 1
                attrs = []
                continue
            if t in ('{', '(', '['):
                i = closing(ts, i) + 1
                attrs = []
                continue
            if t == ';':
                attrs = []
            i += 1

    for target in manifest.get('test', []):
        if target['name'] in expected:
            raise ValueError(f'duplicate target: {target["name"]}')
        names = set()
        walk_file(root / target['path'], '', names)
        expected[target['name']] = names
    if not expected:
        raise ValueError('no explicit [[test]] targets')
    orphaned = set((root / 'tests').rglob('*.rs')) - reached
    if orphaned:
        raise ValueError('orphan Rust files: ' + ', '.join(str(p.relative_to(root)) for p in sorted(orphaned)))
    return expected, reached


def check_list(expected, listing):
    actual = {}
    for suite in listing['rust-suites'].values():
        binary = suite
        if binary['kind'] == 'test':
            actual[binary['binary-name']] = set(suite['testcases'])
    problems = []
    for binary in expected.keys() | actual.keys():
        missing = expected.get(binary, set()) - actual.get(binary, set())
        extra = actual.get(binary, set()) - expected.get(binary, set())
        if missing or extra or binary not in actual or binary not in expected:
            problems.append(f'{binary}: missing={sorted(missing)}, unexpected={sorted(extra)}')
    if problems:
        raise ValueError('nextest differs from source modules (use --ignore-default-filter):\n' + '\n'.join(problems))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--nextest-list', type=Path, help='cargo nextest list --release --ignore-default-filter --message-format json output')
    args = parser.parse_args()
    try:
        expected, reached = inventory()
        if args.nextest_list:
            check_list(expected, json.loads(args.nextest_list.read_text()))
    except (ValueError, OSError, KeyError) as error:
        print(error, file=sys.stderr)
        return 1
    print(f'{len(expected)} integration binaries; {len(reached)} reachable Rust files; '
          f'{sum(map(len, expected.values()))} integration tests'
          + (' match nextest' if args.nextest_list else ' expected from source'))
    return 0


if __name__ == '__main__':
    sys.exit(main())
