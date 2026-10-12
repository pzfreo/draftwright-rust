"""Survey quality parts, corpus parts used by tests, and every AP242 fixture.

Run from the repository root: python3 -I tools/survey_repeated_spans.py <binary> <absolute out>
Set DRAFTWRIGHT_CORPUS. The manifest includes literal test inputs and sheet-facts bindings;
the thirty quality parts are a subset. The survey fails on a repeated span or a failed part.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import importlib.util
import json
import os
from pathlib import Path

module = importlib.util.spec_from_file_location('quality', Path(__file__).with_name('quality.py'))
quality = importlib.util.module_from_spec(module)
module.loader.exec_module(quality)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary')
    parser.add_argument('out', type=Path)
    parser.add_argument('--timeout', type=int, default=600, help='Per render/plan timeout in seconds')
    parser.add_argument('--before', type=Path, help='Verify the recorded pre-E3 patterns against this baseline')
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error('--timeout must be positive')
    verdicts = json.loads(Path('tests/fixtures/known_repeated_spans.json').read_text())
    assert all(v['verdict'] == 'rust-correct' and v['evidence'].strip() for v in verdicts)
    if args.before:
        before = json.loads(args.before.read_text())
        found = [{'part': part, **g} for part, p in before.items() for g in p['repeated_span_groups']]
        recorded = [{k: v[k] for k in ('part', 'kind', 'items')} for v in verdicts]
        assert sorted(found, key=str) == sorted(recorded, key=str), 'pre-E3 patterns differ from verdicts'
    corpus = Path(os.environ['DRAFTWRIGHT_CORPUS'])
    assert corpus.is_dir(), corpus
    assert args.out.is_absolute(), 'out must be absolute'
    inputs = args.out / 'inputs'
    inputs.mkdir(parents=True, exist_ok=True)
    files = [corpus / f for f in Path('tools/repeated_span_parts.txt').read_text().splitlines()]
    assert set(Path('tools/quality_parts.txt').read_text().splitlines()) <= {
        str(p.relative_to(corpus)) for p in files}
    sources = '\n'.join(p.read_text() for p in Path('tests').rglob('*.rs'))
    used = {p for p in corpus.rglob('*') if p.suffix.lower() in {'.step', '.stp'}
            and (p.name in sources or str(p.relative_to(corpus)) in sources)}
    used.update(corpus / json.loads(gzip.decompress(p.read_bytes()))['binding']['file']
                for p in Path('tests/fixtures/sheets').glob('*.gz'))
    assert used <= set(files), f'add test inputs to the manifest: {sorted(used - set(files))}'
    for fixture in sorted(Path('tests/fixtures/pmi/step').glob('*.gz')):
        path = inputs / fixture.name[:-3]
        path.write_bytes(gzip.decompress(fixture.read_bytes()))
        files.append(path)
    assert all(p.is_file() for p in files), 'a survey input is missing'
    assert len({p.stem for p in files}) == len(files), 'ambiguous output basenames'
    (args.out / 'inputs.txt').write_text(''.join(f'{p}\n' for p in files))
    with ThreadPoolExecutor(4) as executor:
        results = dict(executor.map(lambda p: quality.one(str(p), args.binary, str(args.out), timeout=args.timeout), files))
    (args.out / 'baseline.json').write_text(json.dumps(results, indent=1, sort_keys=True) + '\n')
    quality.table(results)
    return int(any('error' in p or p['repeated_span'] for p in results.values()))


if __name__ == '__main__':
    raise SystemExit(main())
