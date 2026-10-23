"""Print release_only=true when a PR changes nothing but release metadata.

A release PR bumps the workspace version and edits notes. Such a PR runs lint (fmt and clippy,
which compiles every target) and skips the build, test, acceptance and WASM jobs. main's push
run after the merge does the same when CI passed on the commit it was merged onto (ci.yml checks
that); otherwise it runs everything.

Allowed: Markdown files and docs/, and in Cargo.toml / crates/*/Cargo.toml / Cargo.lock only
lines that change a version string from the old workspace version to the new one.

Usage: python3 -I tools/release_only.py <base-sha> <head-sha>   (appends to $GITHUB_OUTPUT if set)
"""
import os
import re
import subprocess
import sys

VERSION = re.compile(r'^[+-]version = "([^"]+)"$')


def git(*args):
    return subprocess.run(['git', *args], check=True, capture_output=True, text=True).stdout


def release_only(base, head):
    files = [f for f in git('diff', '--name-only', f'{base}...{head}').splitlines() if f]
    if not files:
        return False, 'no changes'
    versions = set()
    for f in files:
        if f.endswith('.md') or f.startswith('docs/'):
            continue
        if f not in ('Cargo.toml', 'Cargo.lock') and not re.fullmatch(r'crates/[^/]+/Cargo\.toml', f):
            return False, f'{f} is not release metadata'
        for line in git('diff', '-U0', f'{base}...{head}', '--', f).splitlines():
            if line.startswith(('+++', '---', '@@', 'diff ', 'index ')):
                continue
            m = VERSION.match(line)
            if not m:
                return False, f'{f}: {line!r} is not a version line'
            versions.add((line[0], m.group(1)))
    old = {v for s, v in versions if s == '-'}
    new = {v for s, v in versions if s == '+'}
    if len(old) > 1 or len(new) > 1:
        return False, f'more than one version changes: {sorted(old)} -> {sorted(new)}'
    return True, f'release metadata only ({", ".join(files)})'


if __name__ == '__main__':
    ok, why = release_only(sys.argv[1], sys.argv[2])
    print(why)
    if os.environ.get('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a') as out:
            out.write(f'release_only={"true" if ok else "false"}\n')
