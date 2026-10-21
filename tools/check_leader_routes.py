"""Rendered leader acceptance: length, hidden/section/cut crossings and 0.3 mm runs.

A moved callout's label may not gain an extension line within its 1 mm pad (text on an
extension line is worse than a leader across one), its arrowhead may not gain an extension
line within 0.2 mm, and its elbow or padded label may not come within 1 mm of the outline.

Record origin/main with --record, then compare any quality.py output directory.
The fixture records the original routes on all thirty quality parts.
An explicit-paper mechanism test can retain its published budgets with
--record-paper, --record-part and --baseline. The recorder checks the current
measurement before storing both budgets and measurements separately.
Compare common IDs; inventory and information coverage are checked by the Rust tests.
Unchanged pre-existing runs are reported; moving a callout may not retain one.
Dashes are judged by their underlying stroke; curves use 0.002 mm chord error.
"""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('quality', ROOT / 'tools/quality.py')
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)

def flatten(at, end, f, depth=0):
    middle = f(0.5)
    flat = all(math.dist(f(t), [at[k]+t*(end[k]-at[k]) for k in (0,1)]) <= 0.002
               for t in (.25,.5,.75))
    if depth >= 20 or flat:
        return [at, end]
    return (flatten(at, middle, lambda t: f(t / 2), depth + 1)[:-1]
            + flatten(middle, end, lambda t: f((t + 1) / 2), depth + 1))

def points(element):
    tag = element.tag.rsplit('}', 1)[-1]
    a = element.attrib
    if tag == 'line':
        return [(float(a['x1']), float(a['y1'])), (float(a['x2']), float(a['y2']))]
    if tag in ('circle', 'ellipse'):
        rx, ry = float(a.get('rx', a.get('r', 0))), float(a.get('ry', a.get('r', 0)))
        cx, cy = float(a['cx']), float(a['cy'])
        f = lambda t: [cx + rx * math.cos(t * 2 * math.pi), cy + ry * math.sin(t * 2 * math.pi)]
        return sum((flatten(f(i/4), f((i+1)/4), lambda t, i=i: f((i+t)/4))[:-1]
                    for i in range(4)), []) + [f(1)]
    if tag != 'path':
        return []
    tokens = re.findall(r'[A-Za-z]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?', a['d'])
    out, at, i = [], [0, 0], 0
    while i < len(tokens):
        command = tokens[i]; i += 1
        n = {'M': 2, 'L': 2, 'Q': 4, 'C': 6, 'A': 7, 'Z': 0}[command]
        values = list(map(float, tokens[i:i+n])); i += n
        if command == 'Z':
            out.append(out[0]); continue
        end = values[-2:]
        if command in ('M', 'L'):
            out.append(end)
        elif command in ('Q', 'C'):
            poles = [at] + [values[k:k+2] for k in range(0, n, 2)]
            def bezier(t):
                q = poles
                while len(q) > 1:
                    q = [[(1-t)*a[k]+t*b[k] for k in (0,1)] for a,b in zip(q,q[1:])]
                return q[0]
            out.extend(flatten(at, end, bezier)[1:])
        else:
            rx, ry, rotation, large, sweep = values[:5]
            phi = math.radians(rotation); c, s = math.cos(phi), math.sin(phi)
            dx, dy = (at[0]-end[0])/2, (at[1]-end[1])/2
            x, y = c*dx+s*dy, -s*dx+c*dy
            factor = max(1, math.sqrt((x/rx)**2+(y/ry)**2))
            rx *= factor; ry *= factor
            k = math.sqrt(max(0, (rx*ry)**2-(rx*y)**2-(ry*x)**2)
                          / max(1e-30, (rx*y)**2+(ry*x)**2))
            if bool(large) == bool(sweep): k = -k
            ux, uy = k*rx*y/ry, -k*ry*x/rx
            cx, cy = (at[0]+end[0])/2+c*ux-s*uy, (at[1]+end[1])/2+s*ux+c*uy
            start = math.atan2((y-uy)/ry, (x-ux)/rx)
            stop = math.atan2((-y-uy)/ry, (-x-ux)/rx)
            angle = (stop-start) % (2*math.pi)
            if not sweep: angle -= 2*math.pi
            f = lambda t: [cx+c*rx*math.cos(start+angle*t)-s*ry*math.sin(start+angle*t),
                           cy+s*rx*math.cos(start+angle*t)+c*ry*math.sin(start+angle*t)]
            steps = max(1, math.ceil(abs(angle)/(math.pi/2)))
            for j in range(steps):
                out.extend(flatten(f(j/steps), f((j+1)/steps),
                                   lambda t, j=j: f((j+t)/steps))[1:])
        at = end
    return out


def crosses(a, b):
    def side(p, q, r):
        return (q[0]-p[0])*(r[1]-p[1])-(q[1]-p[1])*(r[0]-p[0])
    return (side(*a, b[0])*side(*a, b[1]) < -1e-12
            and side(*b, a[0])*side(*b, a[1]) < -1e-12)


def along_interval(a, b):
    """Fraction of a inside the finite 0.3 mm strip beside b, including near-parallel travel."""
    if any(max(a[0][k],a[1][k]) < min(b[0][k],b[1][k])-.3
           or min(a[0][k],a[1][k]) > max(b[0][k],b[1][k])+.3 for k in (0,1)):
        return None
    d = [a[1][k]-a[0][k] for k in (0,1)]
    w = [b[1][k]-b[0][k] for k in (0,1)]
    n = sum(x*x for x in w)
    if n < 1e-18:
        return None
    # Transverse strips shorter than 2 mm cannot join into collinear travel.
    if abs(d[0]*w[1]-d[1]*w[0]) > (2*.3/2)*math.dist(*a)*math.sqrt(n):
        return None
    q = [a[0][k]-b[0][k] for k in (0,1)]
    band = .3*math.sqrt(n)
    lo, hi = 0.0, 1.0
    for at,slope,lower,upper in ((sum(x*y for x,y in zip(q,w)),sum(x*y for x,y in zip(d,w)),0,n),
                                (q[0]*w[1]-q[1]*w[0],d[0]*w[1]-d[1]*w[0],-band,band)):
        if abs(slope) < 1e-15:
            if not lower <= at <= upper:
                return None
        else:
            x,y = (lower-at)/slope,(upper-at)/slope
            lo,hi = max(lo,min(x,y)),min(hi,max(x,y))
    return (lo,hi) if hi > lo else None


def longest_run(a, ink):
    intervals = sorted(q for b in ink if (q := along_interval(a,b)) is not None)
    start, stop, longest = 0.0, 0.0, 0.0
    for lo,hi in intervals:
        if lo > stop + 1e-9:
            start = lo
        stop = max(stop,hi)
        longest = max(longest,stop-start)
    return longest * math.dist(*a)


PAD = 1.0
ARROW_HALF = 2.7 / 6.0


def grow(b, d):
    return [b[0]-d, b[1]-d, b[2]+d, b[3]+d]


def box_of(s):
    return [min(s[0][0],s[1][0]), min(s[0][1],s[1][1]), max(s[0][0],s[1][0]), max(s[0][1],s[1][1])]


def touches(s, b):
    """quality.segment_box: the segment meets the closed box."""
    return quality.segment_box(s, b)


def head(arrow):
    tip, tail = arrow
    n = [tail[1]-tip[1], tip[0]-tail[0]]
    k = ARROW_HALF / max(math.hypot(*n), 1e-12)
    a = [tail[0]+k*n[0], tail[1]+k*n[1]]
    b = [tail[0]-k*n[0], tail[1]-k*n[1]]
    return [[tip,a],[a,b],[b,tip]]


def measures(plan, svg, analysis):
    if plan.get('sheets'):
        out = {}
        for sheet in [plan['sheet'], *plan['sheets']]:
            local = quality.local_sheet_plan(plan,sheet)
            measured = measures(local,quality.sheet_svg(str(svg),sheet.get('number',1)),analysis)
            if set(out) & set(measured):
                raise ValueError('a leader has more than one carrying sheet')
            out.update(measured)
        return out

    root = ET.parse(svg).getroot()
    layers = {g.get('id'): [points(e) for e in g]
              for g in root.iter() if g.get('id') in ('part', 'hidden', 'hatch', 'centres')}
    segments = {k: [s for c in cs for s in zip(c,c[1:])] for k,cs in layers.items()}
    extensions = [line for d in plan['dimensions'] if d.get('placed')
                  and len(d['placed'].get('arrows', [])) == 2
                  for line in d['placed']['lines'][:2]]
    # Use the resolved strokes the renderer draws, regardless of the cut's model axis.
    cuts = [curve['Line'] for v in plan['views'] for w in v.get('line_work', [])
            if w['kind'] == 'cutting_plane' for curve in w['curves'] if 'Line' in curve]
    section_views = [v for v in plan['views'] if v.get('cut')]
    def in_section(c):
        if not c:
            return False
        q = c[len(c)//2]
        return any(v['area'][0] <= q[0] <= v['area'][2]
                   and v['area'][1] <= q[1] <= v['area'][3] for v in section_views)
    section = [s for c in layers.get('part', []) if in_section(c) for s in zip(c,c[1:])]
    # Section hatching shares the dims layer. Subtract the Plan's annotation strokes,
    # then include its remaining straight ink in each section's reserved area.
    annotation = [s for d in plan['dimensions'] if d.get('placed')
                  for s in d['placed']['lines'] + d['placed'].get('arrows', [])]
    for g in root.iter():
        if g.get('id') != 'dims':
            continue
        for e in g:
            if e.tag.rsplit('}',1)[-1] != 'line':
                continue
            c = points(e)
            if in_section(c) and not any(max(math.dist(a,b) for a,b in zip(c,s)) < 1e-5
                                         for s in annotation):
                section.append(c)
    out = {}
    for d in plan['dimensions']:
        if d.get('kind') != 'callout' or not d.get('placed'):
            continue
        p = d['placed']; lines = p['lines']
        view = next(v for v in plan['views'] if v['id'] == d['view'])
        route = [[q[k]-view['at'][k] for k in (0,1)] for l in lines for q in l]
        label = [p['label_box'][k]-view['at'][k%2] for k in range(4)]
        model = segments.get('part', []) + segments.get('hidden', []) + extensions
        counts = {k: sum(any(crosses(l,s) for l in lines) for s in ink)
                  for k,ink in [('hidden', segments.get('hidden', [])),
                                ('section', section), ('cutting_plane', cuts)]}
        owners = [(o['id'], e) for o in plan['dimensions'] if o.get('placed')
                  and len(o['placed'].get('arrows', [])) == 2
                  and len(o['placed'].get('lines', [])) >= 3 for e in o['placed']['lines'][:2]]
        pad = grow(p['label_box'], PAD)
        text = len({o for o, e in owners if touches(e, pad)})
        flare = [s for a in p.get('arrows', [])[:1] for s in head(a)]
        heads = sum(any(touches(s, grow(box_of(e), .2)) for s in flare) for _, e in owners)
        elbow = lines[0][1]
        outline = segments.get('part', [])
        near = grow([elbow[0], elbow[1], elbow[0], elbow[1]], PAD)
        out[d['id']] = dict(text_extension=text, head_extension=heads,
                            elbow_outline=any(touches(s, near) for s in outline),
                            label_outline=any(touches(s, pad) for s in outline),
                            length=sum(math.dist(*l) for l in lines),
                            run=max((longest_run(l,model) for l in lines), default=0),
                            route=route, label=label, **counts)
    return out


def check(before, after):
    failures = []
    if set(before) != set(after):
        failures.append('incomplete thirty-part inventory')
    for part, old in before.items():
        now = after.get(part, {})
        for id_, a in now.items():
            if id_ not in old and a['run'] > 2.0 + 1e-6:
                failures.append(f"{part}: {id_}: new route runs beside ink {a['run']:.3f} mm")
        for id_, b in old.items():
            if id_ not in now:
                continue
            a = now[id_]
            if a['length'] > b['length'] + 2.0 + 1e-6:
                failures.append(f"{part}: {id_}: length {b['length']:.3f} -> {a['length']:.3f} mm")
            for key in ('hidden', 'section', 'cutting_plane'):
                if a[key] > b[key]:
                    failures.append(f'{part}: {id_}: {key} {b[key]} -> {a[key]}')
            moved = (len(a['route']) != len(b['route'])
                     or any(math.dist(x,y) > 1e-6 for x,y in zip(a['route'],b['route']))
                     or any(abs(x-y) > 1e-6 for x,y in zip(a['label'],b['label'])))
            if moved:
                for key in ('text_extension', 'head_extension', 'elbow_outline', 'label_outline'):
                    if key in b and a[key] > b[key]:
                        failures.append(f'{part}: {id_}: moved {key} {b[key]} -> {a[key]}')
            if a['run'] > 2.0 + 1e-6 and (moved or a['run'] > b['run'] + 1e-6):
                failures.append(f"{part}: {id_}: runs beside model/hidden/extension {a['run']:.3f} mm")
    return failures


def collect(directory):
    out = {}
    for file in (ROOT / 'tools/quality_parts.txt').read_text().splitlines():
        name = Path(file).stem
        plan = json.loads((directory / (name+'.plan.json')).read_text())
        analysis = json.loads((directory / (name+'.analysis.json')).read_text())
        out[name] = measures(plan, directory / (name+'.svg'), analysis)
    return out


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('drawings', type=Path)
    p.add_argument('--baseline', type=Path, default=ROOT / 'tests/fixtures/leader_routes.json')
    p.add_argument('--record', type=Path)
    p.add_argument('--record-part', action='append', default=[],
                   help='Update only these reviewed parts in the existing record')
    p.add_argument('--record-paper', choices=['A4', 'A3', 'A2', 'A1', 'A0'],
                   help='Retain and validate --baseline budgets as an explicit-paper reference')
    p.add_argument('--base-ref', default='HEAD', help='Source revision when recording')
    p.add_argument('--require-improvement', action='store_true')
    p.add_argument('--report', type=Path)
    args = p.parse_args()
    if args.record_paper and (not args.record or not args.record_part):
        p.error('--record-paper requires --record and --record-part')
    result = collect(args.drawings)
    contacts = {name: quality.leader_on_extension(json.loads(
        (args.drawings / (name+'.plan.json')).read_text())) for name in result}
    if args.record:
        if args.record_paper:
            record = json.loads(args.record.read_text())
            source = json.loads(args.baseline.read_text())
            reference = record.setdefault('paper_references', {}).setdefault(
                args.record_paper, dict(parts={}, contacts={}, sheet_choices={}))
            reference.setdefault('measurements', {})
            reference['budget_base'] = source.get('base')
            reference['base'] = subprocess.check_output(
                ['git', 'rev-parse', args.base_ref], cwd=ROOT, text=True).strip()
            for part in args.record_part:
                if part not in result or part not in record['parts']:
                    p.error(f'unknown recorded part: {part}')
                plan = json.loads((args.drawings / (part+'.plan.json')).read_text())
                if plan['sheet']['size'] != args.record_paper:
                    p.error(f'{part}: drawing is not on {args.record_paper}')
                if set(source['parts'][part]) != set(result[part]):
                    p.error(f'{part}: explicit-paper reference must retain the same callouts')
                failures = check({part: source['parts'][part]}, {part: result[part]})
                if failures:
                    p.error('; '.join(failures))
                reference['parts'][part] = source['parts'][part]
                reference['measurements'][part] = result[part]
                reference['contacts'][part] = contacts[part]
                reference['sheet_choices'][part] = [plan['sheet']['size'],
                    plan['sheet']['scale'], 1 + len(plan.get('sheets', []))]
        elif args.record_part:
            record = json.loads(args.record.read_text())
            revision = subprocess.check_output(['git', 'rev-parse', args.base_ref], cwd=ROOT, text=True).strip()
            for part in args.record_part:
                if part not in result or part not in record['parts']:
                    p.error(f'unknown recorded part: {part}')
                record['parts'][part] = result[part]
                record['contacts'][part] = contacts[part]
                record.setdefault('updated_parts', {})[part] = revision
        else:
            revision = subprocess.check_output(
                ['git', 'rev-parse', args.base_ref], cwd=ROOT, text=True).strip()
            record = dict(base=revision, parts=result, contacts=contacts)
        args.record.write_text(json.dumps(record, indent=2, sort_keys=True)+'\n')
        return 0
    baseline = json.loads(args.baseline.read_text())
    failures = check(baseline['parts'], result)
    for name, value in contacts.items():
        if value > baseline['contacts'][name]:
            failures.append(f'{name}: extension contacts {baseline["contacts"][name]} -> {value}')
    if args.require_improvement and sum(contacts.values()) >= sum(baseline['contacts'].values()):
        failures.append(f'extension contacts did not fall: {sum(contacts.values())}')
    residual = {name: {id_: row['run'] for id_,row in rows.items() if row['run'] > 2.0 + 1e-6}
                for name,rows in result.items()}
    residual = {name:rows for name,rows in residual.items() if rows}
    if args.report:
        args.report.write_text(json.dumps(dict(parts=result, contacts=contacts, failures=failures,
            residual_runs=residual), indent=2, sort_keys=True)+'\n')
    print('\n'.join(failures) if failures else
          f'PASS: {sum(map(len,result.values()))} callouts on {len(result)} parts; '
          'length <= base + 2 mm, no extra hidden/section/cut crossings or new runs over 2 mm; '
          f'{sum(map(len,residual.values()))} unchanged pre-existing runs remain')
    return bool(failures)


if __name__ == '__main__':
    sys.exit(main())
