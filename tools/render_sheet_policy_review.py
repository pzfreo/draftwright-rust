#!/usr/bin/env python3
"""Native rendering is deliberate: binding a saved Analysis rereads the STEP
without native recognition and can change the planning scope's geometry inventory.
Use each stage's immutable executable and the exact surveyed STEP basename.
"""
import concurrent.futures,json,pathlib,subprocess
import argparse
parser=argparse.ArgumentParser(description="Render native paired sheet-policy proofs, including every A3-to-A4 scale reduction.")
parser.add_argument('survey',type=pathlib.Path)
parser.add_argument('--stage', choices=['before','after','both'], default='both')
args=parser.parse_args()
r=args.survey
stages=['before','after'] if args.stage=='both' else [args.stage]
m=json.load(open(r/'before/manifest.json'))
requested=['mfcadpp/10000.step','mfcadpp_holdout/520.step','mfcadpp/10167.step','mfcadpp/10049.step','mfcadpp/10119.step','mfcadpp_holdout/163.step','mfcadpp_holdout/274.step','mfcadpp/10212.step','gramel/GRM-03_thumbwheel_drive_screw.step','ttt_inputs/sm-hanger.step','nist/nist_ctc_03_asme1_rc.stp','mfcadpp/10063.step','mfcadpp_regressions/11512.step','cadgenbench/threaded_connector_109.step','parts/boss_circle.step','mfcadpp/1004.step','mfcadpp/10007.step','mfcadpp/1017.step','mfcadpp/10098.step','mfcadpp/1013.step']
requested.append('nist/nist_ftc_07_asme1_rd.stp')
selected={};lower=[]
for k,v in m.items():
 old=json.load(open(r/'before'/(k+'.plan.json')));new=json.load(open(r/'after'/(k+'.plan.json')))
 if 'sheet' not in old:continue
 reduction=old['sheet']['size']=='A3' and new['sheet']['size']=='A4' and new['sheet']['scale_factor']<old['sheet']['scale_factor']
 wanted=any(v['name'].endswith('/'+name) for name in requested)
 automatic=any(w.get('detail') and not w['detail'].get('authored',False) for w in new['views'])
 gained=sum(bool(w.get('detail')) for w in new['views'])>sum(bool(w.get('detail')) for w in old['views'])
 if reduction:lower.append(v['name'])
 if wanted or reduction or automatic or gained:
  stem=pathlib.Path(v['path']).stem
  selected[k]={'part':v['name'],'name':stem,'file':k[:12]+'-'+stem,'reduction':reduction,'requested':wanted,'automatic':automatic,'before':old['sheet']['size']+' '+old['sheet']['scale'],'after':new['sheet']['size']+' '+new['sheet']['scale']}
assert sum(v['requested'] for v in selected.values())==len(requested),[n for n in requested if not any(v['name'].endswith('/'+n) for v in m.values())]
(r/'render-index.json').write_text(json.dumps(selected,indent=2)+'\n')
for stage in stages:(r/stage/'renders').mkdir(exist_ok=True)
def run(item):
 k,v=item
 for stage in stages:
  out=r/stage/'renders'/v['file'];binary=r/stage/'draftwright'
  p=subprocess.run([str(binary.resolve()),'render',m[k]['path'],'-o',str(out),'--formats','svg,png','--dpi','150'],capture_output=True,timeout=3600)
  out.with_suffix('.render.json').write_bytes(p.stdout)
  if p.returncode:raise RuntimeError((stage,v['part'],p.stderr.decode()))
 print(v['part'],v['before'],'->',v['after'],flush=True)
with concurrent.futures.ThreadPoolExecutor(4) as pool:list(pool.map(run,selected.items()))
print('Rendered',len(selected),'pairs;',len(lower),'A3 -> A4 reductions',flush=True)
