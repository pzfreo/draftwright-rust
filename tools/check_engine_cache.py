"""Check Cargo's actual source-change invalidation (two serial release builds).

Run from the repo root: python3 -I tools/check_engine_cache.py
Copies engine inputs to a disposable checkout; never edits the working source.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='d1-cache-build-') as temp:
    copied = Path(temp) / 'engine'
    copied.mkdir()
    for path in ('src', 'fonts', 'schemas', 'Cargo.toml', 'Cargo.lock', 'build.rs', 'build_support.rs'):
        source = root / path
        if source.is_dir():
            shutil.copytree(source, copied / path)
        else:
            shutil.copy2(source, copied / path)
    # A separate target prevents copied-manifest artifacts from replacing working binaries.
    env = {**os.environ, 'CARGO_TARGET_DIR': str(Path(temp) / 'target'),
           'CARGO_BUILD_JOBS': '8', 'RUST_TEST_THREADS': '8',
           'DRAFTWRIGHT_CACHE': str(Path(temp) / 'cache')}
    binary = Path(temp) / 'target/release/draftwright'
    step = str(root / 'tests/fixtures/parts/mounting_plate.step')
    def build(manifest):
        subprocess.run(['cargo', 'build', '--release', '--bin', 'draftwright', '--manifest-path', str(manifest)], env=env, check=True)
    def rpc(tool, args):
        request = {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call', 'params': {'name': tool, 'arguments': args}}
        process = subprocess.run([binary, 'mcp-stdio'], input=json.dumps(request)+'\n', text=True, capture_output=True, env=env, check=True)
        return json.loads(process.stdout)['result']
    build(copied / 'Cargo.toml')
    first = rpc('inspect_part', {'step': step})
    assert not first['isError'], first
    old_ref = first['structuredContent']['part_ref']
    with (copied / 'src/stateless.rs').open('a') as source:
        source.write('\n// D1 source-mutation cache probe.\n')
    build(copied / 'Cargo.toml')
    second = rpc('inspect_part', {'step': step})
    assert not second['isError'], second
    new_ref = second['structuredContent']['part_ref']
    assert old_ref.split('-')[0] == new_ref.split('-')[0]
    assert old_ref != new_ref, 'source edit reused stale cache namespace'
    stale = rpc('inspect_part', {'part_ref': old_ref})
    assert stale['isError'], 'old engine cache was served'
    assert 'engine version' in stale['structuredContent']['error']['message']
    print(json.dumps({'old_part_ref': old_ref, 'new_part_ref': new_ref, 'stale_ref_refused': True}))
