"""Render and check G1 completeness evidence; fails on the pre-G1 binary.

Usage: python3 -I tools/check_completeness.py BINARY CORPUS QUALITY_DIR OUT_DIR
Uses default quality plans to choose physical owners, then omits their statements in
Spec variants. No measured values or planner geometry are invented by this instrument.
"""
import json
from pathlib import Path
import subprocess
import sys

binary, corpus, quality, out = map(Path, sys.argv[1:])
binary = binary.resolve()
corpus, quality, out = corpus.resolve(), quality.resolve(), out.resolve()
out.mkdir(parents=True, exist_ok=True)
parts = {Path(p).stem: corpus / p for p in Path(__file__).with_name('quality_parts.txt').read_text().splitlines()}
report = {}


def plan(name):
    return json.loads((quality / f'{name}.plan.json').read_text())


def review(name, case, omitted=()):
    prefix = out / f'{name}-{case}'
    spec = {'dimensions': {id_: {'omit': True} for id_ in omitted}}
    spec_path = prefix.with_suffix('.spec.json')
    spec_path.write_text(json.dumps(spec, indent=2) + '\n')
    result = subprocess.run([str(binary), 'review', str(parts[name]), '--spec', str(spec_path)],
                            capture_output=True, text=True, check=True)
    result = json.loads(result.stdout)
    prefix.with_suffix('.review.json').write_text(json.dumps(result, indent=2) + '\n')
    prefix.with_suffix('.svg').write_text(result['svg'])
    subprocess.run(['resvg', '--background', 'white', '-w', '2400',
                    str(prefix.with_suffix('.svg')), str(prefix.with_suffix('.png'))],
                   capture_output=True, check=True)
    return result


for name in ['1000', '10046', '10049']:
    p = plan(name)
    assert p['status'] == 'needs_attention', name
    c = p['completeness']
    assert c['claimed_faces'] < c['faces'] == p['binding']['face_count'], name
    assert any(i['code'] == 'unclaimed_geometry' and i['severity'] == 'warning'
               for i in p['problems']), name
    report[f'{name}-unclaimed'] = c

for name in ['1000', '10038', '10049']:
    p = plan(name)
    assert p['completeness']['unplanned_features'] > 0, name
    assert any(i['code'] == 'dimensions_not_ported' and i['severity'] == 'warning'
               for i in p['problems']), name

for name in ['GRM-03_thumbwheel_drive_screw', '10049', '1000']:
    omitted = [d['id'] for d in plan(name)['dimensions'] if any(
        m['feature'] == 'envelope' and m['parameter'].endswith('.length')
        for m in d.get('measurements', []))]
    if not omitted:
        # A complete turned-length chain can prove the overall axis without a separate extent.
        omitted = [d['id'] for d in plan(name)['dimensions'] if any(
            m['parameter'] == 'step.length' for m in d.get('measurements', []))]
    r = review(name, 'missing-extents', omitted)
    found = [i for i in r['lint'] if i['code'] == 'missing_principal_dimension']
    assert found, name
    report[f'{name}-missing-extents'] = found

for name in ['GRM-05_depth_lock_bolt', 'string_post', 'GRM-03_thumbwheel_drive_screw']:
    dimensions = plan(name)['dimensions']
    bands = sorted({m['feature'] for d in dimensions for m in d.get('measurements', [])
                    if m['feature'].startswith('f:turned_step:')})
    assert bands, name
    owner = next(b for b in bands if {'step.length', 'step.diameter'} <= {
        m['parameter'] for d in dimensions for m in d.get('measurements', []) if m['feature'] == b})
    omitted = [d['id'] for d in dimensions if any(m['feature'] == owner for m in d.get('measurements', []))]
    r = review(name, 'missing-band', omitted)
    found = [i for i in r['lint'] if i['subjects'] == [owner]
             and i['code'] in {'axial_length_missing', 'turned_step_requirement_missing'}]
    assert {i['requirements'][0]['parameter'] for i in found} == {'step.length', 'step.diameter'}, name
    report[f'{name}-missing-band'] = found

for name in ['nist_ftc_08_asme1_rc', 'nist_ftc_09_asme1_rd', 'nist_ctc_01_asme1_rd']:
    dimensions = plan(name)['dimensions']
    owners = sorted({m['feature'] for d in dimensions for m in d.get('measurements', [])
                     if m['feature'].startswith('f:hole_pattern:')})
    assert owners, name
    owner = owners[0]
    omitted = [d['id'] for d in dimensions if any(m['feature'] == owner and
        (m['parameter'].startswith(('location.', 'grid_pitch.', 'pitch.'))
         or m['parameter'] == 'bolt_circle.diameter') for m in d.get('measurements', []))]
    r = review(name, 'missing-pattern', omitted)
    found = [i for i in r['lint'] if i['subjects'] == [owner]
             and i['code'].startswith('hole_pattern_requirement_')]
    parameters = {i['requirements'][0]['parameter'] for i in found}
    assert any(p.startswith('location.') for p in parameters), (name, parameters)
    assert any(p.startswith(('grid_pitch.', 'pitch.', 'bolt_circle.')) for p in parameters), (name, parameters)
    report[f'{name}-missing-pattern'] = found

(out / 'findings.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
print('Completeness reporting and envelope/band/pattern findings: three parts per class pass')
