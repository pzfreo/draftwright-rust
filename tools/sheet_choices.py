"""Before/after sheet, scale and footprint fill evidence for H6 (no geometry mutation).

Read plans emitted by quality.py; remeasure old plans with their own Analysis and
exact same per-item bounds used by sheet_fit. The before executable stays archived.
"""
import argparse
import json
import math
from pathlib import Path
import re
import subprocess


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
    boxes.extend(furniture)
    w,h = p['sheet']['page']; tb=list(p['sheet']['title_block']); tb[3]=tb[1]+32
    clipped = [[max(10,b[0]-1),max(10,b[1]-1),min(w-10,b[2]+1),min(h-10,b[3]+1)] for b in boxes]
    clipped = [b for b in clipped if b[0]<b[2] and b[1]<b[3]]
    in_title = [[max(b[0],tb[0]),max(b[1],tb[1]),min(b[2],tb[2]),min(b[3],tb[3])] for b in clipped]
    in_title = [b for b in in_title if b[0]<b[2] and b[1]<b[3]]
    return (union_area(clipped)-union_area(in_title))/((w-20)*(h-20)-(tb[2]-tb[0])*(tb[3]-tb[1]))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', required=True, type=Path)
    parser.add_argument('--after', required=True, type=Path)
    parser.add_argument('--before-binary', required=True)
    parser.add_argument('--measure', required=True, help='release examples/sheet_fill executable')
    parser.add_argument('--corpus', required=True, type=Path)
    parser.add_argument('--fixtures', required=True, type=Path)
    parser.add_argument('--plate', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args=parser.parse_args()
    keys=['off_sheet','repeated_label','repeated_measure','views_overlap',
          'view_on_title_block','ink','repeated_span','line_through_feature',
          'hidden_on_visible','section_hidden','claimed_faces','faces','unplanned_features']
    old_metrics=json.loads((args.before/'baseline.json').read_text())
    new_metrics=json.loads((args.after/'baseline.json').read_text())
    assert old_metrics.keys()==new_metrics.keys(),'metric part sets differ'
    for name,old in old_metrics.items():
        new=new_metrics[name]
        for key in keys:
            good=(new[key]>=old[key] if key=='claimed_faces' else
                  new[key]==old[key] if key=='faces' else new[key]<=old[key])
            assert good,(name,key,old[key],new[key])
    # The manifest disambiguates duplicate basenames in different corpus collections.
    files=[args.corpus/name for name in Path(__file__).with_name('quality_parts.txt').read_text().splitlines()]
    files += sorted(p for p in args.fixtures.glob('*') if p.suffix in ('.step','.stp'))
    sources={p.stem:p for p in [*files,args.plate]}
    rows=[]
    for f in sorted(args.before.glob('*.plan.json')):
        stem=f.name.removesuffix('.plan.json')
        before=json.loads(f.read_text())
        after=json.loads((args.after/f.name).read_text())
        a=json.loads(subprocess.check_output([args.before_binary,'analyse',str(sources[stem])],text=True))
        analysis_file=args.after/(stem+'.analysis.json')
        analysis_file.write_text(json.dumps(a))
        def measure(path):
            return json.loads(subprocess.check_output([args.measure,str(analysis_file),str(path)],text=True))
        old_measure=measure(f); new_measure=measure(args.after/f.name)
        bf=footprint_fill(a,before,old_measure['furniture']); af=footprint_fill(a,after,new_measure['furniture'])
        assert abs(bf-old_measure['fill']) < 1e-9, (stem,bf,old_measure['fill'])
        assert abs(af-after['sheet']['fill']) < 1e-9, (stem,af,after['sheet']['fill'])
        choice=after['sheet']; old=before['sheet']
        index=lambda n: 4-int(n[1])
        factor=lambda n: float(n.split(':')[0])/float(n.split(':')[1])
        match=re.search(r'smallest fit (A[0-4]) at ([0-9:]+);',choice['basis'])
        if match:
            smallest,scale=match.groups()
            if 'which is kept' not in choice['basis']:
                assert index(choice['size']) <= index(smallest)+1, stem
                if index(choice['size']) > index(smallest): assert factor(choice['scale'])>factor(scale),stem
            records=re.findall(r'(A[0-4]) ([0-9:]+) fill ([0-9.]+)% '
                               r'\(valid and complete, ([0-9]+) additional view conflicts, '
                               r'([0-9]+) additional material crossings, ([0-9]+) annotation conflicts',choice['basis'])
            changed=next((q for q in after['problems'] if q['code']=='layout_replanned'),None)
            initial=re.search(r'from (A[0-4]) at ([0-9:]+)',changed['message']) if changed else None
            initial_pair=initial.groups() if initial else (choice['size'],choice['scale'])
            seed_clean=f'first estimate {initial_pair[0]} at {initial_pair[1]} (valid and complete)' in choice['basis']
            # Validity (no hard finding or loss, no extra ink over another view) alone defines
            # the smallest fit; against a valid seed, allowed pairs adding material crossings
            # are set aside, and if all do the seed is kept.
            valid=[(n,s,f,int(cross)) for n,s,f,over,cross,ink in records if int(over)==0]
            assert valid,stem
            assert index(smallest)==min(index(n) for n,_,_,_ in valid),stem
            assert factor(scale)==max(factor(s) for n,s,_,_ in valid if n==smallest),stem
            valid=[v[:3] for v in valid if ((v[0]==smallest and v[1]==scale)
                   or (index(v[0])==index(smallest)+1 and factor(v[1])>factor(scale)))
                   and (not seed_clean or v[3]==0)]
            if not valid:
                assert seed_clean and 'which is kept' in choice['basis'],stem
                assert (choice['size'],choice['scale'])==initial_pair,stem
                rows.append((stem,old['size'],old['scale'],round(100*bf,2),choice['size'],choice['scale'],round(100*af,2)))
                continue
            assert (choice['size'],choice['scale']) in [(n,s) for n,s,_ in valid],stem
            # Candidate basis rounds fill to 0.1%; avoid guessing which side of a band
            # boundary an exactly rounded endpoint occupies. The independent measure
            # above verifies the selected pair at full precision.
            ambiguous=any(abs(float(f)-edge)<=.05 for _,_,f in valid for edge in (60,85))
            band=[v for v in valid if 60.05<float(v[2])<84.95]
            if not ambiguous and band:
                best=max(band,key=lambda v:(factor(v[1]),-index(v[0])))
                assert (choice['size'],choice['scale'])==best[:2],stem
            elif not band:
                assert 100*af>=max(float(f) for _,_,f in valid)-.05,stem
        else:
            assert 'no valid complete smallest fit' in choice['basis'],stem
            # Title-block lifting can improve the seed estimate even when no complete
            # candidate exists. Such a fallback may not spend more paper or reduce scale.
            assert index(choice['size'])<=index(old['size']),stem
            assert factor(choice['scale'])>=factor(old['scale']),stem
        if stem=='nist_ctc_03_asme1_rc':
            assert not any(q['code'] in {'annotation_ink_overlap','annotation_off_sheet',
                                        'annotation_overlap','layout_recovery_exhausted'}
                           for q in after['problems']), 'CTC-03 must be valid, not an exhausted fallback'
        if stem=='manufacturing-pmi':
            # A4 has no valid candidate; the rule decides between A3 1:1 and A2 2:1 by fill.
            assert match and match.groups()==('A3','1:1'),stem
            assert (choice['size'],choice['scale'])==('A3','1:1'),stem
        if stem=='GRM-03_thumbwheel_drive_screw':
            assert factor(choice['scale'])>factor(old['scale']) and .60<=af<=.85,stem
        rows.append((stem,old['size'],old['scale'],round(100*bf,2),choice['size'],choice['scale'],round(100*af,2)))
    lines=['| Part | Before sheet | Scale | Fill % | After sheet | Scale | Fill % |',
           '|---|---|---|---:|---|---|---:|']
    lines += ['| '+' | '.join(map(str,r))+' |' for r in rows]
    args.out.write_text('\n'.join(lines)+'\n')
    print(f'{len(rows)} sheet choices and {len(rows)*len(keys)} quality cells checked; independent footprint measurements agree')

if __name__=='__main__': main()
