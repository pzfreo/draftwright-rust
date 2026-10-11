"""Regenerate the standalone MCP input/output JSON Schemas (draft 2020-12)."""
import json
from pathlib import Path
S = {'type': 'string'}
B = {'type': 'boolean'}
N = {'type': 'number'}
I = {'type': 'integer', 'minimum': 0}
def arr(item, **kw): return {'type': 'array', 'items': item, **kw}
def obj(props, required=(), extra=False): return {'type':'object', 'properties':props, 'required':list(required), 'additionalProperties':extra}
def ref(name): return {'$ref':'#/$defs/'+name}
def nullable(s): return {'anyOf':[s, {'type':'null'}]}
state={'anyOf':[obj({'kind':{'enum':['shown','omitted_by_user','not_authored']}},['kind']), obj({'kind':{'const':'covered'},'by':S},['kind','by']),obj({'kind':{'const':'not_drawable'},'reason_code':S},['kind','reason_code'])]}
op={'anyOf':[obj({'op':{'enum':['set','add']},'path':S,'value':{}},['op','path','value']),obj({'op':{'const':'remove'},'path':S},['op','path'])]}
place=obj({'view':{'enum':['front','plan','side']},'side':{'enum':['above','below','left','right']},'lane':{'type':'integer','minimum':0,'maximum':1000},'along':{'anyOf':[{'enum':['start','middle','end','outside']},{'type':'number','minimum':0,'maximum':1}]},'pinned':B})
compact=obj({'id':S,'kind':S,'label':nullable(S),'view':nullable(S),'state':state},['id','kind','label','view','state'])
item=obj({'item':{'type':'object'},'value':nullable(N),'dimension_kind':nullable(S),'placement':nullable({'type':'object'}),'anchors':arr(S),'features':arr(S),'basis':S,'operations':arr(S)},['item','value','dimension_kind','placement','anchors','features','basis','operations'])
problem=obj({'id':S,'code':S,'severity':S,'message':S,'subjects':arr(S),'refs':arr(S),'fix':arr(op)},['id','code','severity','message','subjects'])
rectangle=arr(N,minItems=4,maxItems=4)
filter_=obj({'view':nullable(S),'kind':nullable(S),'state':nullable(state),'feature':nullable(S),'region':nullable(rectangle),'near':nullable(S),'radius':nullable({'type':'number','minimum':0})})
problem_filter=obj({'code':nullable(S),'severity':nullable({'enum':['info','warning','error']}),'subject':nullable(S)})
spec=obj({'schema':{'enum':['draftwright.spec/1','draftwright.spec/2']},'binding':nullable({'type':'object'}),'sheet':obj({'size':nullable(S),'scale':nullable(S),'title':{'type':'object','additionalProperties':S}}),'views':{'type':'object','additionalProperties':{'type':'object'}},'dimensions':{'type':'object','additionalProperties':{'type':'object'}},'annotations':{'type':'object','additionalProperties':{'type':'object'}},'features':{'type':'object','additionalProperties':{'type':'object'}},'dimension_source':{'enum':['auto','authored']},'notes':arr({'type':'object'}),'tables':arr({'type':'object'})})
defs={'spec':spec,'op':op,'item':item,'compact':compact,'problem':problem,'error':obj({'code':{'enum':['invalid','failed']},'message':S,'choices':arr(S)},['code','message'])}
stub=obj({'id':nullable(S),'detail_ref':S},['id','detail_ref'])
problem_small=obj({'id':S,'code':S,'severity':S,'subjects':arr(S)},['id','code','severity','subjects'])
row_item={'anyOf':[ref('compact'),ref('item'),stub]}
row_problem={'anyOf':[problem,problem_small,stub]}
def paged(row):return obj({'items':arr(row),'total':I,'next_cursor':nullable(I)},['items','total','next_cursor'])
page=paged({'anyOf':[obj({'change':{'enum':['changed','added']},'item':row_item},['change','item']),obj({'change':{'const':'removed'},'id':S},['change','id']),obj({'change':{'enum':['problem_added','problem_resolved']},'problem':row_problem},['change','problem']),stub]})
delta=obj({'events':page,'delta_ref':S,'plans':I},['events','delta_ref','plans'])
summary=obj({'part_ref':S,'part':S,'face_count':I,'envelope':arr(arr(N,minItems=3,maxItems=3),minItems=2,maxItems=2),'sheet':obj({'size':S,'scale':S,'page':arr(N,minItems=2,maxItems=2)},['size','scale','page']),'views':arr(S),'items':I,'problems':I,'spec_version':S,'plan_ref':S},['part_ref','part','face_count','envelope','sheet','views','items','problems','spec_version'])
common={'part_ref':S,'spec':ref('spec'),'spec_version':S,'drawing_ref':S}
paging={'cursor':I,'limit':{'type':'integer','minimum':1,'maximum':100},'detail':B}
artifact=obj({'artifact_ref':S,'mime_type':{'enum':['image/png','application/pdf']},'bytes':I,'spec_version':S},['artifact_ref','mime_type','bytes','spec_version'])
tools={
'inspect_part':(obj({**common,'step':S,'features':S,'detail':B}),summary),
'list_items':(obj({**common,**paging,'filter':filter_},['part_ref']),paged(row_item)),
'get_item':(obj({**common,'id':S,'detail':B},['part_ref','id']),obj({'item':{'anyOf':[ref('item'), obj({**compact['properties'],'basis':S,'value':nullable(N),'operations':arr(S)},[*compact['required'],'basis','value','operations'])]}},['item'])),
'list_problems':(obj({**common,**paging,'filter':problem_filter},['part_ref']),paged(row_problem)),
'item_options':(obj({**common,'id':S},['part_ref','id']),obj({'id':S,'operations':arr(S),'placements':arr(place),'view_anchors':arr(S),'scales':arr(S)},['id','operations','placements','view_anchors','scales'])),
'preview_ops':(obj({**common,**paging,'ops':arr(ref('op'),maxItems=100)},['part_ref','ops']),delta),
'apply_ops':(obj({**common,**paging,'ops':arr(ref('op'),maxItems=100),'parent':S},['part_ref','ops','parent']),obj({'spec_version':S,'delta':delta},['spec_version','delta'])),
'view_sheet':(obj({**common,'region':rectangle,'overlay_ids':B,'dpi':{'type':'number','exclusiveMinimum':0,'maximum':400}},['part_ref']),artifact),
'render_pdf':(obj(common,['part_ref']),artifact),
'get_spec':(obj(common,['part_ref']),obj({'spec_version':S,'spec':ref('spec')},['spec_version','spec'])),
'analyse':(obj({'step':S,'features':S},['step']),obj({'schema':{'const':'draftwright.analysis/7'},'binding':obj({'source_sha256':S,'part':S,'face_count':I,'loader':S},['source_sha256','part','face_count','loader']),'envelope':arr(arr(N,minItems=3,maxItems=3),minItems=2,maxItems=2),'features':arr({'type':'object'}),'iso_bounds':rectangle},['schema','binding','envelope','iso_bounds'],True)),
'render':(obj({**common,'step':S,'features':S,'format':{'enum':['png','pdf']},'region':rectangle,'overlay_ids':B,'dpi':{'type':'number','exclusiveMinimum':0,'maximum':400}}),artifact),
}
for tool,(inp,out) in tools.items():
    if tool in ['inspect_part','render']:
        inp['anyOf']=[{'required':['step']},{'required':['part_ref']}]

    # Refusals are also structured results described by each tool's output schema.
    for kind,schema in [('input',inp),('output',{'type':'object','anyOf':[out,obj({'error':ref('error')},['error'])]})]:
        used={}
        def collect(s):
            if isinstance(s,dict):
                if '$ref' in s:
                    name=s['$ref'].split('/')[-1]
                    if name not in used:
                        used[name]=defs[name]; collect(defs[name])
                for value in s.values(): collect(value)
            elif isinstance(s,list):
                for value in s: collect(value)
        collect(schema)
        schema={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':f'https://draftwright.io/schemas/mcp/{tool}.{kind}.json',**schema,'$defs':used}
        Path(f'schemas/mcp/{tool}.{kind}.json').write_text(json.dumps(schema,indent=2)+'\n')
