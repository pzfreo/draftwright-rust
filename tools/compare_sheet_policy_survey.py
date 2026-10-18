"""Audit the paired, native 227-part sheet-policy survey and write its report.

Pass two detail_revision_survey directories plus their sheet_readability JSON files.
The input manifest is hash bound; no baseline or measurement threshold is rewritten.
"""
import argparse
import collections
import csv
import json
import re
from pathlib import Path

PAPERS = ('A4', 'A3', 'A2', 'A1', 'A0')
DIAGNOSTICS = {'layout_replanned', 'layout_recovery_exhausted', 'legibility_floor_breached',
               'detail_unplaceable'}


def read(path):
    return json.loads(Path(path).read_text())


def pair(plan):
    return f"{plan['sheet']['size']} {plan['sheet']['scale']} / {1 + len(plan.get('sheets', []))} / " + str(sum(bool(v.get('detail')) for v in plan['views']))


def claims(plan):
    result = {(m['feature'], m['parameter'], m['value']) for d in plan['dimensions']
              if d.get('placed') for m in d.get('measurements', [])}
    for note in plan.get('notes', []):
        if note.get('placed'):
            result.update((m['feature'], m['parameter'], m['value']) for m in note.get('measurements', []))
    for table in ([plan['hole_table']] if plan.get('hole_table') else []) + plan.get('hole_tables', []):
        assert table.get('blocks') and all(b.get('placed') for b in table['blocks']), 'undrawn schedule cannot carry audited facts'
        result.update((m['feature'], m['parameter'], m['value'])
                      for row in table['rows'] for m in row.get('measurements', []))
        result.update((m['feature'], m['parameter'], m['value']) for m in table.get('covered', []))
    return result


def axis_coordinates(plan, analysis):
    """Coordinates determined by a drawn common axis and measured round stock.

    A central bore need not repeat the two half-diameter coordinates formerly
    tabulated. Require the actual end-view cross, common axial line and OD ink,
    and independently match the bore's model-space centre to the stock.
    """
    direct = claims(plan)
    od = next((value for feature, role, value in direct
               if feature == 'envelope' and role == 'od.diameter'), None)
    if od is None:
        return set()
    lo, hi = analysis['envelope']
    if any(abs(hi[k] - lo[k] - od) > 1e-6 for k in (0, 1)):
        return set()
    result = set()
    for feature in analysis.get('features', []):
        p = feature.get('parameters', {})
        axis, location = p.get('axis'), p.get('location')
        if feature['kind'] != 'hole' or not isinstance(axis, list) or not location:
            continue
        if abs(abs(axis[2]) - 1) > 1e-6 or any(abs(location[k] - (lo[k] + hi[k]) / 2) > 1e-6 for k in (0, 1)):
            continue
        identity = feature['id']
        if not any(f == identity and role == 'bore.diameter' for f, role, _ in direct):
            continue
        common = any(w.get('global_axis') and w.get('kind') == 'axis' and w.get('curves')
                     and w['id'].endswith(':' + identity)
                     for v in plan['views'] for w in v.get('line_work', []))
        cross = any(abs(v['direction'][2]) > .999 and any(
            w['id'].endswith(':' + identity) and w.get('kind') == 'cross'
            and w.get('centre') == v['at'] and len(w.get('curves', [])) >= 2
            for w in v.get('line_work', [])) for v in plan['views'])
        if common and cross:
            result.update((identity, 'location.length.' + role, location[k] - lo[k])
                          for k, role in enumerate(('x', 'y')))
    return result


def annotation_claims(plan):
    result = set()
    for d in plan.get('dimensions', []):
        if d.get('placed') and set(d.get('from_spec', [])) & {'fit', 'tolerance', 'text', 'style', 'precision'}:
            result.add(('dimension', d['id'], json.dumps(d['terms'])))
    for g in plan.get('gdt', []):
        if g.get('placed'):
            result.add(('gdt', g['id'], json.dumps({k: g.get(k) for k in ('kind', 'label', 'cells', 'faces', 'source')}, sort_keys=True)))
    for r in plan.get('requirements', []):
        if r.get('placed') or r.get('state', {}).get('kind') == 'shown':
            result.add(('requirement', r['id'], json.dumps({k: r.get(k) for k in ('kind', 'text', 'source')}, sort_keys=True)))
    return result


def findings(plan):
    return collections.Counter((q['code'], q.get('severity'), tuple(q.get('subjects', [])),
                                tuple(q.get('refs', []))) for q in plan['problems']
                               if q['severity'] != 'info' and q['code'] not in DIAGNOSTICS)


def reasons(plan):
    basis = plan['sheet']['basis']
    if basis.startswith('screen-first fitted-reference retention'):
        # Later basis text describes refused trials, not the published drawing.
        retained = f"Established readable whole-part drawing retained on {plan['sheet']['size']} at {plan['sheet']['scale']}. "
        refused = re.search(r'smaller ordinary fit [^;]+', basis)
        return retained + (refused.group(0) if refused else 'Alternative search refused against the fitted drawing.')
    prefix = ''
    if 'independently recovered ordinary drawing without automatic details' in basis:
        prefix = 'Ordinary readable main retained: a smaller reduction makes openings or depictions unreadable. No automatic detail purchases paper. '
    rejected = re.search(r'no new crossing pairs are allowed; rejected drawing (A[0-4]) at ', basis)
    if rejected and PAPERS.index(rejected.group(1)) < PAPERS.index(plan['sheet']['size']):
        prefix = 'Smaller-paper candidate adds crossing pairs against the retained fitted drawing. '
    selected = re.search(r'whole-part paper (A[0-4]) at ', basis)
    if '; whole-part trials: ' in basis and selected:
        ordinary = basis.split('; whole-part trials: ', 1)[1].split('; local-detail trials: ', 1)[0]
        selected = selected.group(1)
        if selected == 'A4' and plan['sheet']['size'] == 'A3':
            prefix = 'Ordinary A3 recovery: A4 misses a whole-part floor/fit or needs multiple sheets; no automatic detail purchases paper. '
        elif PAPERS.index(plan['sheet']['size']) < PAPERS.index(selected) and any(
                v.get('detail') for v in plan['views']):
            prefix = f"Whole-part annotation fit needs {selected}; local details allow {plan['sheet']['size']}. "
    else:
        ordinary = basis
    if plan['sheet']['size'] == 'A4':
        return prefix + ('same-paper readability refusal' if 'local-detail trials refused' in basis else 'whole part fits A4')
    reasons = []
    for paper in PAPERS[:PAPERS.index(plan['sheet']['size'])]:
        trials = re.findall(r'\b' + paper + r' [0-9]+:[0-9]+ fill [^;]+', ordinary)
        # Retain the settled trial at the smallest fitted scale, along with its
        # actual ink/route/lane refusal, rather than a local floor as justification.
        if trials:
            eligible = [t for t in trials if 'principal' not in t]
            if eligible:
                trial = eligible[-1]
                trial = re.sub(r', [0-9]+ (?:unreadable step spans|openings below 2 mm|depiction gaps below 0.7 mm)', '', trial)
                reasons.append(trial)
                continue
        preflight = re.findall(r'\b' + paper + r' [0-9]+:[0-9]+ \(principal [^)]*\)', ordinary)
        if preflight:
            reasons.append(preflight[-1])
    return prefix + '; '.join(reasons or [ordinary.split('; candidates:', 1)[0]])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('before_readability', type=Path)
    parser.add_argument('after_readability', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    old_manifest, new_manifest = read(args.before / 'manifest.json'), read(args.after / 'manifest.json')
    assert old_manifest.keys() == new_manifest.keys(), 'survey contents differ'
    before_read, after_read = read(args.before_readability), read(args.after_readability)
    stats = {stage: collections.Counter() for stage in ('before', 'after')}
    rows, changes, up, lost, added, derived, lost_annotations, large, gained = [], [], [], [], [], [], [], [], []
    sheet_changes = []
    for key, source in old_manifest.items():
        old = read(args.before / (key + '.plan.json'))
        new = read(args.after / (key + '.plan.json'))
        if 'sheet' not in old:
            assert 'sheet' not in new, 'unresolved input changed status'
            continue
        assert 'sheet' in new, source['name']
        views = {v['id']: v for v in new['views']}
        all_automatic = [v for v in new['views'] if v.get('detail') and not v['detail'].get('authored')]
        assert len(all_automatic) <= 2, source['name']
        for v in all_automatic:
            assert v['detail']['bounds'] == v['detail']['marked_bounds'], source['name']
            assert any(d['view'] == v['id'] and d.get('placed') and not d.get('reference') for d in new['dimensions']), source['name']
        automatic = [v for v in new['views'] if v.get('detail', {}).get('kind', '').startswith('readability') and not v.get('detail', {}).get('authored')]
        assert all(sum(v.get('sheet', 1) == sheet for v in automatic) <= 2 for sheet in {v.get('sheet', 1) for v in automatic}), source['name']
        if automatic:
            ordinary = re.search(r'whole-part paper (A[0-4]) at ([0-9]+):([0-9]+)', new['sheet']['basis'])
            assert ordinary, (source['name'], 'missing independent whole-part choice')
            assert PAPERS.index(new['sheet']['size']) <= PAPERS.index(ordinary[1]), source['name']
            scale = int(ordinary[2]) / int(ordinary[3])
            assert new['sheet']['scale_factor'] + 1e-6 >= scale, source['name']
            assert all(v['scale'] + 1e-6 >= scale for v in new['views'] if v['id'] in ('v:front', 'v:plan', 'v:side')), source['name']
        for view in new['views']:
            detail = view.get('detail')
            if detail and detail['kind'].startswith('readability') and not detail['authored']:
                parent = views[detail['source_view']]
                assert view.get('sheet', 1) == parent.get('sheet', 1), source['name']
                assert detail['bounds'] == detail['marked_bounds'], source['name']
                axes = {'v:plan': (0, 1), 'v:side': (1, 2)}.get(detail['source_view'], (0, 2))
                spans = [detail['bounds'][1][axis] - detail['bounds'][0][axis] for axis in axes]
                assert min(spans) > 0 and max(spans) <= 3 * min(spans) + 1e-6, source['name']
                assert any(d['view'] == view['id'] and d.get('placed') and not d.get('reference') for d in new['dimensions']), source['name']
        assert key in before_read and key in after_read, source['name']
        assert not after_read[key]['failures'] or any(
            q['code'] == 'layout_recovery_exhausted' for q in new['problems']), source['name']
        for stage, plan in (('before', old), ('after', new)):
            stats[stage][plan['sheet']['size']] += 1
            stats[stage]['sheets'] += 1 + len(plan.get('sheets', []))
            stats[stage]['details'] += sum(bool(v.get('detail')) for v in plan['views'])
        row = dict(part=source['name'], before=pair(old), after=pair(new),
                   reason=reasons(new), readability_after=after_read[key]['reason'],
                   recovery_exhausted=any(q['code'] == 'layout_recovery_exhausted' for q in new['problems']))
        rows.append(row)
        if sum(bool(v.get('detail')) for v in new['views']) > sum(bool(v.get('detail')) for v in old['views']):
            gained.append(row)
        if row['before'] != row['after']:
            changes.append(row)
        if len(new.get('sheets', [])) != len(old.get('sheets', [])):
            sheet_changes.append(row)
        if PAPERS.index(new['sheet']['size']) > PAPERS.index(old['sheet']['size']):
            up.append(row)
        if new['sheet']['size'] in ('A2', 'A1', 'A0'):
            large.append(row)
        raw_missing = claims(old) - claims(new)
        inferred = raw_missing & axis_coordinates(new, read(args.after / (key + '.analysis.json')))
        if inferred:
            derived.append({'part': source['name'], 'measurements': sorted(inferred),
                            'proof': 'measured round-stock OD, centred end-view cross and drawn common axial line'})
        missing = raw_missing - inferred
        if missing:
            lost.append({'part': source['name'], 'measurements': sorted(missing)})
        missing_annotations = annotation_claims(old) - annotation_claims(new)
        if missing_annotations:
            lost_annotations.append({'part': source['name'], 'claims': sorted(missing_annotations)})
        new_findings = findings(new) - findings(old)
        if new_findings:
            added.append({'part': source['name'], 'findings': list(new_findings.elements())})
    assert len(rows) == 227, len(rows)
    assert len(before_read) == len(after_read) == len(rows), 'incomplete readability audit'
    rows.sort(key=lambda row: row['part'])
    up.sort(key=lambda row: row['part'])
    with (args.out / 'sheet-policy-results.csv').open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)
    readable = {stage: sum(v['failures'] == 0 for v in data.values())
                for stage, data in (('before', before_read), ('after', after_read))}
    report = dict(parts=len(rows), changed=len(changes), stats=stats, fully_readable=readable, up=up,
                  large_paper=large, gained_details=gained, sheet_count_changes=sheet_changes, lost_measurements=lost, axis_derived_measurements=derived, lost_annotation_claims=lost_annotations, new_drawing_findings=added)
    (args.out / 'audit.json').write_text(json.dumps(report, indent=2) + '\n')
    lines = ['# Paired sheet-policy results', '',
             '227 drawable inputs; the same unresolved input is refused on both sides. Counts classify the primary paper.', '',
             '| Paper | Before | After |', '| --- | ---: | ---: |']
    lines += [f"| {paper} | {stats['before'][paper]} | {stats['after'][paper]} |" for paper in PAPERS]
    lines += [f"| {key.title()} | {stats['before'][key]} | {stats['after'][key]} |" for key in ('sheets', 'details')]
    lines += ['', 'Every paper up-move (the paired Plans retain full candidate evidence):', '',
              '| Part | Before | After | Reason |', '| --- | --- | --- | --- |']
    for row in up:
        lines.append('| ' + ' | '.join(str(row[k]).replace('|', '/') for k in ('part', 'before', 'after', 'reason')) + ' |')
    lines += ['', f'{len(gained)} parts gain details. The complete before/after inventory is in the CSV.', '']
    lines += ['', f"{len(changes)} drawings change paper, scale, sheet count or detail count. "
              f"Fully readable drawings: {readable['before']} → {readable['after']}. "
              'Every remaining readability failure has an explicit refusal.', '',
              'No lost measurement facts, lost PMI/GDT claims or new drawing-finding identities. '
              'Direct dimensions and independently verified common-axis geometry retain the measurement inventory. '
              'See [verification and PNG index](sheet-policy-verification.md).']
    (args.out / 'summary.md').write_text('\n'.join(lines) + '\n')
    changed_frames = [row for row in changes
                      if row['before'].rsplit(' / ', 1)[0] != row['after'].rsplit(' / ', 1)[0]]
    lines = ['# Paper, scale or sheet-count changes', '',
             'Before and after use paper, main scale / physical sheets / details.', '',
             '| Part | Before | After |', '| --- | --- | --- |']
    lines += ['| ' + ' | '.join(row[k] for k in ('part', 'before', 'after')) + ' |'
              for row in sorted(changed_frames, key=lambda row: row['part'])]
    lines += ['', f'{len(sheet_changes)} parts change sheet count; no part may gain sheets.', '']
    (args.out / 'changes.md').write_text('\n'.join(lines))
    print(json.dumps({k: v if k in ('parts', 'changed', 'stats', 'fully_readable') else len(v) for k, v in report.items()}, indent=2))
    assert not lost and not lost_annotations and not added, 'information or finding regression; see audit.json'
    assert not any(int(row['after'].split(' / ')[1]) > int(row['before'].split(' / ')[1])
                   for row in sheet_changes), 'a part gained sheets; see audit.json'


if __name__ == '__main__':
    main()
