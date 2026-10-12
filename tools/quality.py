"""Drawing quality for draftwright-rust: per part, items off the sheet, repeated dimensions,
overlapping views, and ink overlaps, from `draftwright plan` and `render`. Writes baseline.json
in <out dir> and prints a table with totals. Random pins: --seed S --pins N;
--require-go enforces the per-part 1.5x ink / no new off-sheet items bar.

The maintainer's priority (2026-10-09) is to drive these to zero before adding features; see
docs/quality-2026-10-09.md for the baseline and the parts it is measured on.

Repeated spans use placed linear arrow tips, model-mm tolerance 1e-6 for ends and values,
and ignore lane and label. PMI face-pair near misses are included. Use
--require-no-repeated-span to enforce zero; baseline.json retains the groups' item IDs.

Usage: python3 -I tools/quality.py <draftwright binary> <out dir> <step files...>
(the parts: tools/quality_parts.txt, paths relative to $DRAFTWRIGHT_CORPUS)"""
import argparse, collections, hashlib, json, subprocess, sys, os, re, math
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor

INK = {"annotation_ink_overlap", "annotation_over_view", "annotation_line_over_view",
       "annotation_leader_crossing", "annotation_outside_area"}


def line_through_features(plan):
    """Leader/feature pairs, plus extension lines (once even if several features are crossed)."""
    leaders, extensions = set(), set()
    for p in plan.get("problems", []):
        subjects = p.get("subjects", [])
        if p["code"] == "leader_through_feature":
            leaders.add(tuple(subjects))
        elif p["code"] == "extension_through_feature":
            extensions.add((subjects[0], p.get("id", "").rsplit(":line:", 1)[-1]))
    return len(leaders) + len(extensions)


def overlap(a, b):
    return a[0] < b[2] and a[2] > b[0] and a[1] < b[3] and a[3] > b[1]


def repeated_spans(plan):
    """Placed linear statements, independent of lane and label, in model millimetres.

    Arrow tips retain the witness stations even with outside arrows or a broken line.
    PMI identities also prove a face pair: equal values on that pair at different
    stations are reported separately for review, never inferred from equal labels.
    """
    numerator, denominator = map(float, plan['sheet']['scale'].split(':'))
    scale = numerator / denominator
    spans = []
    for d in sorted(plan['dimensions'], key=lambda d: d['id']):
        placed = d.get('placed')
        if d.get('kind') != 'linear' or not placed or len(placed.get('arrows', [])) != 2:
            continue
        axis = 0 if placed['rotation'] % 180 == 0 else 1
        ends = sorted(a[0][axis] / scale for a in placed['arrows'])
        faces = set()
        for m in d.get('measurements', []):
            if m['feature'].startswith('pmi:'):
                match = re.search(r':f([0-9,]+)(?=:|$)', m['feature'])
                if match:
                    pair = tuple(sorted(map(int, match[1].split(','))))
                    if len(pair) == 2:
                        part = re.search(r':p([0-9]+)(?=:|$)', m['feature'])
                        faces.add((part[1] if part else None, pair))
        spans.append((d, axis, ends, faces))
    groups = []
    for d, axis, ends, faces in spans:
        matches = []
        for i, group in enumerate(groups):
            for other, k, stations, pair in group['members']:
                if d['view'] != other['view'] or axis != k or abs(d['value'] - other['value']) > 1e-6:
                    continue
                same = all(abs(a - b) <= 1e-6 for a, b in zip(ends, stations))
                if same or faces & pair:
                    matches.append((i, 'span' if same else 'feature_pair'))
                    break
        if matches:
            first, kind = matches[0]
            groups[first]['members'].append((d, axis, ends, faces))
            if kind == 'feature_pair':
                groups[first]['kind'] = kind
            for i, kind in reversed(matches[1:]):
                groups[first]['members'].extend(groups[i]['members'])
                if kind == 'feature_pair' or groups[i]['kind'] == 'feature_pair':
                    groups[first]['kind'] = 'feature_pair'
                del groups[i]
        else:
            groups.append({'kind': 'span', 'members': [(d, axis, ends, faces)]})
    return sorted(({'kind': g['kind'], 'items': sorted(d['id'] for d, *_ in g['members'])}
                   for g in groups if len(g['members']) > 1), key=lambda g: g['items'])


def svg_linework(path, plan):
    """Count SVG edge elements (line/path/ellipse), in page mm, per placed view.

    Distance is to visible ink (half the SVG stroke width), plus 0.15 mm; dashes are
    measured as their underlying path. Curves are flattened to 0.002 mm chord
    error. Isometric edges are included, although their rendering is unchanged.
    """
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

    root = ET.parse(path).getroot()
    layers = {g.attrib.get('id'): [points(e) for e in g]
              for g in root.iter() if g.attrib.get('id') in ('part', 'hidden')}
    visible = layers.get('part', [])
    tolerance = .15 + next((float(g.attrib.get('stroke-width', 0))/2
                            for g in root.iter() if g.attrib.get('id') == 'part'), 0)
    counts = {'hidden_on_visible': 0, 'section_hidden': 0}
    def view_of(p):
        return next((v for v in plan['views'] if v.get('area') and
                     v['area'][0] <= p[0] <= v['area'][2] and
                     v['area'][1] <= p[1] <= v['area'][3]), None)
    by_view = collections.defaultdict(list)
    for curve in visible:
        if len(curve) >= 2:
            view = view_of(curve[len(curve)//2])
            if view:
                by_view[view['id']].extend(zip(curve, curve[1:]))
    grids = {}
    def cells(a, b, pad=0):
        return ((x,y) for x in range(math.floor((min(a[0],b[0])-pad)/2), math.floor((max(a[0],b[0])+pad)/2)+1)
                for y in range(math.floor((min(a[1],b[1])-pad)/2), math.floor((max(a[1],b[1])+pad)/2)+1))
    for view, segments in by_view.items():
        grid = collections.defaultdict(set)
        for i, (a,b) in enumerate(segments):
            for cell in cells(a,b,tolerance): grid[cell].add(i)
        grids[view] = grid
    for curve in layers.get('hidden', []):
        if len(curve) < 2: continue
        view = view_of(curve[len(curve)//2])
        if not view: continue
        counts['section_hidden'] += bool(view.get('cut'))
        if sum(math.dist(a,b) for a,b in zip(curve,curve[1:])) < 1-1e-9:
            continue
        segments = by_view[view['id']]; grid = grids.get(view['id'], {})
        length = 0
        for a,b in zip(curve,curve[1:]):
            candidates = set().union(*(grid.get(cell,set()) for cell in cells(a,b)))
            intervals = []
            vx, vy = b[0]-a[0], b[1]-a[1]
            vv = vx*vx+vy*vy
            if not vv: continue
            for i in candidates:
                c,d = segments[i]
                if any(max(a[k],b[k]) < min(c[k],d[k])-tolerance or
                       min(a[k],b[k]) > max(c[k],d[k])+tolerance for k in (0,1)):
                    continue
                wx, wy = d[0]-c[0], d[1]-c[1]
                ww = wx*wx+wy*wy
                def clip(value, slope, lower, upper, lo, hi):
                    if abs(slope) < 1e-15:
                        return (lo,hi) if lower <= value <= upper else (0,0)
                    u,v = (lower-value)/slope, (upper-value)/slope
                    return max(lo,min(u,v)), min(hi,max(u,v))
                if ww:
                    ax, ay = a[0]-c[0], a[1]-c[1]
                    lo,hi = clip(ax*wx+ay*wy,vx*wx+vy*wy,0,ww,0,1)
                    band = tolerance*math.sqrt(ww)
                    lo,hi = clip(ax*wy-ay*wx,vx*wy-vy*wx,-band,band,lo,hi)
                    if hi>lo: intervals.append((lo,hi))
                for end in (c,d):
                    ax,ay = a[0]-end[0],a[1]-end[1]
                    dot = ax*vx+ay*vy
                    disc = tolerance**2-ax*ax-ay*ay+dot*dot/vv
                    if disc>=0:
                        half = math.sqrt(disc/vv)
                        lo,hi = max(0,-dot/vv-half),min(1,-dot/vv+half)
                        if hi>lo: intervals.append((lo,hi))
            stop = 0
            for lo,hi in sorted(intervals):
                if hi>stop:
                    length += max(0,hi-max(lo,stop))*math.sqrt(vv)
                    stop = hi
            if length >= 1-1e-9: break
        counts['hidden_on_visible'] += length >= 1-1e-9
    return counts


def text_size(text, font, size):
    """Measure bundled TrueType glyph boxes and advances without a font dependency."""
    import struct
    from pathlib import Path
    data = (Path(__file__).resolve().parent.parent / 'fonts' / font).read_bytes()
    u16 = lambda offset: struct.unpack_from('>H', data, offset)[0]
    i16 = lambda offset: struct.unpack_from('>h', data, offset)[0]
    u32 = lambda offset: struct.unpack_from('>I', data, offset)[0]
    tables = {data[at:at+4].decode(): u32(at+8)
              for at in range(12, 12 + 16*u16(4), 16)}
    cmap = tables['cmap']
    offsets = [cmap + u32(at+4) for at in range(cmap+4, cmap+4+8*u16(cmap+2), 8)]
    sub = next(at for at in offsets if u16(at) == 4)
    count = u16(sub+6)//2
    ends = sub+14; starts = ends+2*count+2
    deltas = starts+2*count; ranges = deltas+2*count
    def glyph(char):
        code = ord(char)
        for k in range(count):
            if u16(starts+2*k) <= code <= u16(ends+2*k):
                delta = i16(deltas+2*k); offset = u16(ranges+2*k)
                value = u16(ranges+2*k+offset+2*(code-u16(starts+2*k))) if offset else code
                return (value+delta)&65535 if value else 0
        return 0
    long_loca = i16(tables['head']+50)
    def location(g):
        return u32(tables['loca']+4*g) if long_loca else 2*u16(tables['loca']+2*g)
    pen = 0; boxes = []
    metrics = u16(tables['hhea']+34)
    for char in text:
        g = glyph(char); start = location(g)
        if start != location(g+1):
            at = tables['glyf']+start
            boxes.append([pen+i16(at+2), i16(at+4), pen+i16(at+6), i16(at+8)])
        pen += u16(tables['hmtx']+4*min(g,metrics-1))
    scale = size/u16(tables['head']+18)
    return [(max(b[k+2] for b in boxes)-min(b[k] for b in boxes))*scale
            for k in range(2)] if boxes else [0,0]


def furniture_boxes(a, p):
    def text_box(text, font, size, at):
        w,h = text_size(text,font,size)
        return [at[0]-w/2,at[1]-h/2,at[0]+w/2,at[1]+h/2]
    tb = p['sheet']['title_block']; scale = 'SCALE '+p['sheet']['scale']
    w,h = text_size(scale,'IBMPlexSansCondensed-Regular.ttf',3)
    boxes = [[tb[2]-19-w,tb[1]+35,tb[2]-19,tb[1]+35+h]]
    for v in p['views']:
        cut = v.get('cut')
        if not cut: continue
        extent = [hi-lo for lo,hi in zip(*a['envelope'])]
        half = sum(abs(x)*length for x,length in zip(v['up'],extent))*v['scale']/2
        boxes.append(text_box('SECTION '+cut['label']+'–'+cut['label'],
                              'IBMPlexMono-Regular.ttf',3,[v['at'][0],v['at'][1]-half-7]))
        m = next((m for m in p['views'] if m['id']==cut['marked_in']), None)
        if m:
            # Cutting-plane ink is already inside its marked view's footprint;
            # captions extend 7 mm beyond the envelope at each end.
            width = sum(abs(x)*length for x,length in zip(
                [m['up'][1]*m['direction'][2]-m['up'][2]*m['direction'][1],
                 m['up'][2]*m['direction'][0]-m['up'][0]*m['direction'][2],
                 m['up'][0]*m['direction'][1]-m['up'][1]*m['direction'][0]],extent))*m['scale']
            centre = [(lo+hi)/2 for lo,hi in zip(*a['envelope'])]
            y = m['at'][1] + m['scale']*sum(u*(o-c) for u,o,c in zip(m['up'],cut['origin'],centre))
            for x in [m['at'][0]-width/2-7,m['at'][0]+width/2+7]:
                boxes.append(text_box(cut['label'],'IBMPlexMono-Regular.ttf',3,[x,y+4.2]))
            boxes.append([m['at'][0]-width/2-4-0.9, y-6.75,
                          m['at'][0]+width/2+4+0.9,y])
    return boxes


def union_area(boxes):
    xs = sorted({x for b in boxes for x in (b[0], b[2])})
    area = 0
    for left, right in zip(xs, xs[1:]):
        ys = sorted((b[1], b[3]) for b in boxes if b[0] < right and b[2] > left)
        end, height = -math.inf, 0
        for bottom, top in ys:
            height += max(0, top - max(bottom, end))
            end = max(end, top)
        area += (right - left) * height
    return area


def footprint_fill(a, p, furniture=()):
    boxes = []
    def add(box, points=()):
        b = list(box)
        for x, y in points:
            b = [min(b[0], x), min(b[1], y), max(b[2], x), max(b[3], y)]
        boxes.append(b)
    size = [hi-lo for lo, hi in zip(*a['envelope'])]
    for v in p['views']:
        x, y = v['at']; s = v['scale']
        if v['id'] == 'v:iso':
            b = a['iso_bounds']; add([x+s*b[0], y+s*b[1], x+s*b[2], y+s*b[3]])
        else:
            # Principal and section directions have orthogonal unit basis vectors.
            up, toward = v['up'], v['direction']
            right = [up[1]*toward[2]-up[2]*toward[1], up[2]*toward[0]-up[0]*toward[2],
                     up[0]*toward[1]-up[1]*toward[0]]
            w, h = (sum(abs(d[k])*size[k] for k in range(3))*s for d in (right, up))
            add([x-w/2, y-h/2, x+w/2, y+h/2])
    def arrow_points(arrows):
        result=[]
        for tip,tail in arrows:
            normal=[tail[1]-tip[1], tip[0]-tail[0]]
            k=.45/max(1e-12,math.hypot(*normal))
            result.extend([tip,[tail[0]+k*normal[0],tail[1]+k*normal[1]],
                           [tail[0]-k*normal[0],tail[1]-k*normal[1]]])
        return result
    for d in p['dimensions']:
        q = d.get('placed')
        if not q: continue
        points = [r for line in q['lines'] + q['arrows'] for r in line]
        points.extend(arrow_points(q['arrows']))
        add(q['label_box'], points)
    for g in p.get('gdt', []):
        q = g.get('placed')
        if q:
            add(q['label_box'], [r for line in q['lines']+q['strokes']+q['fills'] for r in line]+arrow_points(q['arrows']))
    if p.get('notes_block'): add(p['notes_block']['box'])
    for t in p.get('tables', []) + (p.get('hole_table') or {}).get('blocks', []):
        if t.get('placed'): add(t['placed']['box'])
    for b in p.get('balloons', []):
        x,y = b['at']; r = b['radius']; add([x-r,y-r,x+r,y+r], b.get('leader') or [])
    for r in p.get('requirements', []):
        q = r.get('placed')
        if q:
            b=q['box']; add(q['label_box'], [[b[0],b[1]], [b[2],b[3]]])
    # Projection furniture is resolved geometric ink, independent of sheet.fill.
    symbol = p['sheet'].get('projection_symbol')
    if symbol:
        points = []
        for curve in symbol['curves']:
            if 'Line' in curve:
                points.extend(curve['Line'])
            elif 'Conic' in curve:
                c = curve['Conic']; x,y = c['centre']; rx,ry = c['radii']
                points.extend([[x-rx,y-ry], [x+rx,y+ry]])
        if points:
            xs,ys = zip(*points); add([min(xs),min(ys),max(xs),max(ys)])
    boxes.extend(furniture)
    w,h = p['sheet']['page']; tb=list(p['sheet']['title_block']); tb[3]=tb[1]+32
    clipped = [[max(10,b[0]-1),max(10,b[1]-1),min(w-10,b[2]+1),min(h-10,b[3]+1)] for b in boxes]
    clipped = [b for b in clipped if b[0]<b[2] and b[1]<b[3]]
    in_title = [[max(b[0],tb[0]),max(b[1],tb[1]),min(b[2],tb[2]),min(b[3],tb[3])] for b in clipped]
    in_title = [b for b in in_title if b[0]<b[2] and b[1]<b[3]]
    return (union_area(clipped)-union_area(in_title))/((w-20)*(h-20)-(tb[2]-tb[0])*(tb[3]-tb[1]))


def one(f, dw, out, spec=None, timeout=600):
    name = os.path.basename(f).split(".")[0]
    os.makedirs(out, exist_ok=True)
    extra = ["--spec", spec] if spec else []
    try:
        r = subprocess.run([dw, "render", f, "-o", f"{out}/{name}", "--formats", "svg", *extra], capture_output=True, text=True, timeout=timeout)
        p = subprocess.run([dw, "plan", f, *extra], capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        return name, {"error": str(error)}
    try:
        rend, plan = json.loads(r.stdout), json.loads(p.stdout)
    except Exception:
        return name, {"error": (r.stdout + r.stderr)[:200]}
    if r.returncode or p.returncode or "dimensions" not in plan or "error" in rend:
        return name, {"error": json.dumps(plan.get("error", plan))[:200]}
    if "completeness" not in plan:
        return name, {"error": "Plan has no completeness counts; rebuild draftwright before measuring quality"}
    with open(f"{out}/{name}.plan.json", "w") as output:
        json.dump(plan, output, indent=1, sort_keys=True)
    if plan['sheet'].get('page'):
        analysis = subprocess.run([dw, "analyse", f], capture_output=True, text=True, timeout=timeout)
        if analysis.returncode:
            return name, {"error": analysis.stderr[:200]}
        a = json.loads(analysis.stdout)
        with open(f"{out}/{name}.analysis.json", "w") as output:
            json.dump(a, output, indent=1, sort_keys=True)
        fill = footprint_fill(a, plan, furniture_boxes(a, plan))
    else:
        fill = 0.0
    codes = collections.Counter(x["code"] for x in rend.get("problems", []))
    # Repeated dimensions: the same measured quantity (feature, parameter) stated more than once,
    # and, separately, the same label on the same span in the same view (a drawn duplicate).
    meas = collections.Counter()
    spans = collections.Counter()
    for d in plan["dimensions"]:
        for m in d.get("measurements", []):
            meas[(m.get("feature"), m.get("parameter"))] += 1
        pl = d.get("placed") or {}
        lines = pl.get("lines") or []
        view = d["view"]
        if lines:
            ends = sorted((round(x, 1), round(y, 1)) for l in lines[:2] for (x, y) in l)
            spans[(view, d.get("label"), tuple(ends))] += 1
    labels = collections.Counter(
        (d["view"], d.get("label"), (d.get("placed") or {}).get("rotation", 0))
        for d in plan["dimensions"] if d.get("kind") == "linear")
    rep_label = sum(n - 1 for n in labels.values() if n > 1)
    rep_meas = sum(n - 1 for n in meas.values() if n > 1)
    rep_span = sum(n - 1 for n in spans.values() if n > 1)
    repeated = repeated_spans(plan)
    views = [v["area"] for v in plan["views"] if v.get("area")]
    tb = plan["sheet"].get("title_block")
    view_over = sum(overlap(views[i], views[j]) for i in range(len(views)) for j in range(i + 1, len(views)))
    view_tb = sum(overlap(v, tb) for v in views) if tb else 0
    return name, {
        "off_sheet": codes.get("annotation_off_sheet", 0) + codes.get("pinned_view_off_sheet", 0),
        "off_sheet_items": sorted({subject for problem in rend.get("problems", [])
                                   if problem["code"] in {"annotation_off_sheet", "pinned_view_off_sheet"}
                                   for subject in problem.get("subjects", [])}),
        "repeated_measure": rep_meas, "repeated_drawn": rep_span, "repeated_label": rep_label,
        "repeated_span": sum(len(g['items']) - 1 for g in repeated),
        "repeated_span_groups": repeated,
        "views_overlap": view_over, "view_on_title_block": view_tb,
        "ink": sum(codes[c] for c in INK), "ink_by_code": {c: codes[c] for c in INK if codes[c]},
        "line_through_feature": line_through_features(plan),
        "dimensions": len(plan["dimensions"]),
        "sheet": plan["sheet"].get("size"), "scale": plan["sheet"].get("scale"),
        "fill": round(100 * fill, 2),
        **{key: plan["completeness"][key]
           for key in ("claimed_faces", "faces", "unplanned_features")},
        **svg_linework(f"{out}/{name}.svg", plan),
    }


def table(res):
    keys = ["off_sheet", "repeated_label", "repeated_measure", "views_overlap", "view_on_title_block", "ink", "repeated_span", "line_through_feature", "hidden_on_visible", "section_hidden", "claimed_faces", "faces", "unplanned_features", "fill"]
    print(f"{'part':28}" + "".join(f"{k[:12]:>13}" for k in keys))
    tot = collections.Counter(); bad = collections.Counter()
    for n, v in sorted(res.items()):
        if "error" in v:
            print(f"{n:28} ERROR {v['error'][:80]}"); continue
        print(f"{n[:28]:28}" + "".join(f"{v.get(k, 0):13}" for k in keys))
        for k in keys:
            tot[k] += v.get(k, 0); bad[k] += v.get(k, 0) > 0
    tot['fill'] = round(tot['fill'] / max(1, sum('error' not in v for v in res.values())), 2)
    print(f"{'TOTAL':28}" + "".join(f"{tot[k]:13}" for k in keys))
    print(f"{'parts affected':28}" + "".join(f"{bad[k]:13}" for k in keys), f"of {sum('error' not in v for v in res.values())}")


def random_pins(plan, name, seed, count):
    """Bare annotation pins; view pins use the nearest legal relation to a fixed root."""
    candidates = [(d["id"], "dimensions", {"pinned": True})
                  for d in plan["dimensions"] if d.get("placed") is not None]
    candidates += [(g["id"], "annotations", {"pinned": True})
                   for g in plan.get("gdt", []) if g.get("placed") is not None]
    views = sorted(plan["views"], key=lambda v: (v["id"] != "v:front", v["id"]))
    if len(views) > 1:
        anchor = views[0]
        b = anchor["area"]
        for v in views[1:]:
            a = v["area"]
            dx, dy = (a[0]+a[2]-b[0]-b[2])/2, (a[1]+a[3]-b[1]-b[3])/2
            if abs(dx) > abs(dy):
                side, gap, k = ("right", a[0]-b[2], 1) if dx > 0 else ("left", b[0]-a[2], 1)
            else:
                side, gap, k = ("above", a[1]-b[3], 0) if dy > 0 else ("below", b[1]-a[3], 0)
            offsets = {"middle": (b[k]+b[k+2]-a[k]-a[k+2])/2, "start": b[k]-a[k],
                       "end": b[k+2]-a[k+2]}
            align = min(offsets, key=lambda slot: abs(offsets[slot]))
            candidates.append((v["id"], "views", {"relative_to": anchor["id"][2:], "side": side,
                               "gap": max(0, min(1000, gap)), "align": align, "pinned": True}))
    candidates.sort(key=lambda c: (hashlib.sha256(f"{seed}:{name}:{c[0]}".encode()).digest(), c[0]))
    if count > len(candidates):
        raise ValueError(f"{name}: requested {count} pins, only {len(candidates)} drawable candidates")
    spec = {"schema": "draftwright.spec/2"}
    for id_, kind, place in candidates[:count]:
        spec.setdefault(kind, {})[id_[2:] if kind == "views" else id_] = {"place": place}
    return spec, [c[0] for c in candidates[:count]]


def pin_bar(before, after):
    no_new = after["off_sheet"] <= before["off_sheet"]
    if "off_sheet_items" in before and "off_sheet_items" in after:
        no_new = not (set(after["off_sheet_items"]) - set(before["off_sheet_items"]))
    return after["ink"] <= 1.5 * before["ink"] and no_new


def pinned_one(f, args, baseline):
    name = os.path.basename(f).split(".")[0]
    try:
        if "error" in baseline[name]:
            raise ValueError(baseline[name]["error"])
        p = subprocess.run([args.dw, "plan", f], capture_output=True, text=True, timeout=600)
        if p.returncode:
            raise ValueError(p.stderr or p.stdout)
        spec, ids = random_pins(json.loads(p.stdout), name, args.seed, args.pins)
        path = f"{args.out}/pinned/{name}.spec.json"
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as output:
            json.dump(spec, output, indent=1, sort_keys=True)
        _, after = one(f, args.dw, f"{args.out}/pinned", path)
        if "error" in after:
            raise ValueError(after["error"])
        before = baseline[name]
        return name, {"ids": ids, "unpinned": before, "pinned": after,
                      "ink_delta": after["ink"]-before["ink"],
                      "off_sheet_delta": after["off_sheet"]-before["off_sheet"],
                      "go": pin_bar(before, after)}
    except (ValueError, KeyError, subprocess.TimeoutExpired) as error:
        return name, {"error": str(error), "go": False}


def pin_report(parts, seed, count):
    total = {variant: {key: sum(p[variant][key] for p in parts.values() if "error" not in p)
                       for key in ("ink", "off_sheet")} for variant in ("unpinned", "pinned")}
    go = all(p["go"] for p in parts.values())
    print(f"seed={seed} N={count}: part                         ink before/after off-sheet before/after go")
    for name, p in sorted(parts.items()):
        if "error" in p:
            print(f"{name:48} ERROR {p['error']}")
        else:
            b, a = p["unpinned"], p["pinned"]
            print(f"{name:48} {b['ink']:5}/{a['ink']:<5} {b['off_sheet']:5}/{a['off_sheet']:<5} {'GO' if p['go'] else 'NO-GO'}")
    b, a = total["unpinned"], total["pinned"]
    print(f"TOTAL {b['ink']}/{a['ink']} ink; {b['off_sheet']}/{a['off_sheet']} off-sheet; {'GO' if go else 'NO-GO'} ({sum(not p['go'] for p in parts.values())} failed parts)")
    return {"seed": seed, "pins": count, "parts": parts, "total": total, "go": go}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--pins", type=int)
    parser.add_argument("--baseline", help="Reuse this unpinned baseline JSON (same binary and parts)")
    parser.add_argument("--require-go", action="store_true", help="Exit 1 if any part fails the pin bar")
    parser.add_argument("--require-no-repeated-span", action="store_true", help="Exit 1 if any placed linear span repeats")
    parser.add_argument("dw")
    parser.add_argument("out")
    parser.add_argument("files", nargs="+")
    args = parser.parse_args()
    if (args.seed is None) != (args.pins is None) or (args.pins is not None and args.pins < 1):
        parser.error("provide both --seed and --pins (N >= 1)")
    os.makedirs(args.out, exist_ok=True)
    if args.baseline:
        with open(args.baseline) as source:
            res = json.load(source)
        if set(res) != {os.path.basename(f).split(".")[0] for f in args.files}:
            parser.error("baseline parts must equal the requested parts")
    else:
        with ThreadPoolExecutor(4) as ex:
            res = dict(ex.map(lambda f: one(f, args.dw, args.out), args.files))
    with open(f"{args.out}/baseline.json", "w") as output:
        json.dump(res, output, indent=1, sort_keys=True)
    if args.pins is not None:
        with ThreadPoolExecutor(4) as ex:
            parts = dict(ex.map(lambda f: pinned_one(f, args, res), args.files))
        report = pin_report(parts, args.seed, args.pins)
        with open(f"{args.out}/random-pins.json", "w") as output:
            json.dump(report, output, indent=1, sort_keys=True)
        return int(any("error" in p for p in parts.values()) or (args.require_go and not report["go"]))
    table(res)
    return int(any("error" in v for v in res.values())
               or (args.require_no_repeated_span and any(v['repeated_span'] for v in res.values())))


if __name__ == "__main__":
    sys.exit(main())
