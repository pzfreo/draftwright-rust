"""Regenerate the standalone MCP input/output JSON Schemas (draft 2020-12)."""
import json
import re
from pathlib import Path
MAX_OPS = int(re.search(r'pub const MAX_OPS: usize = (\d+);', (Path(__file__).resolve().parent.parent / 'src/stateless.rs').read_text())[1])
S = {'type': 'string'}
B = {'type': 'boolean'}
N = {'type': 'number'}
I = {'type': 'integer', 'minimum': 0}
SHEET = {'enum':list(range(1, 10))}
def arr(item, **kw): return {'type': 'array', 'items': item, **kw}
def obj(props, required=(), extra=False): return {'type':'object', 'properties':props, **({'required':list(required)} if required else {}), **({'additionalProperties':extra} if extra is not True else {})}
def ref(name): return {'$ref':'#/$defs/'+name}
def nullable(s):
    if set(s) == {'enum'}: return {'enum':[None, *s['enum']]}
    # Type-specific constraints do not apply to null; avoid a redundant anyOf wrapper.
    if isinstance(s.get('type'), str) and not any(k in s for k in ('enum', 'const')):
        return {**s, 'type':[s['type'], 'null']}
    return {'anyOf':[s, {'type':'null'}]}
state={'anyOf':[obj({'kind':{'enum':['shown','omitted_by_user','not_authored']}},['kind']), obj({'kind':{'const':'covered'},'by':S},['kind','by']),obj({'kind':{'const':'not_drawable'},'reason_code':S},['kind','reason_code'])]}
op={'anyOf':[obj({'op':{'enum':['set','add']},'path':S,'value':{}},['op','path','value']),obj({'op':{'const':'remove'},'path':S},['op','path'])]}
place=obj({'view':{'enum':['front','plan','side']},'side':{'enum':['above','below','left','right']},'lane':{'type':'integer','minimum':0,'maximum':1000},'along':{'anyOf':[{'enum':['start','middle','end','outside']},{'type':'number','minimum':0,'maximum':1}]},'offset':arr(N,minItems=2,maxItems=2),'pinned':B})
compact=obj({'sheet':SHEET,'id':S,'kind':S,'label':nullable(S),'view':nullable(S),'state':ref('state')},['id','kind','label','view','state'])
item=obj({'item':{'type':'object'},'value':nullable(N),'dimension_kind':nullable(S),'placement':nullable({'type':'object'}),'anchors':arr(S),'features':arr(S),'basis':S,'operations':arr(S)},['item','value','dimension_kind','placement','anchors','features','basis','operations'])
problem=obj({'id':S,'code':S,'severity':S,'message':S,'subjects':arr(S),'refs':arr(S),'fix':arr(ref('op'))},['id','code','severity','message','subjects'])
rectangle=arr(N,minItems=4,maxItems=4)
filter_=obj({'sheet':nullable(SHEET),'view':nullable(S),'kind':nullable(S),'state':nullable(ref('state')),'feature':nullable(S),'region':nullable(rectangle),'near':nullable(S),'radius':nullable({'type':'number','minimum':0})})
problem_filter=obj({'code':nullable(S),'severity':nullable({'enum':['info','warning','error']}),'subject':nullable(S),'feature':nullable(S)})
# Inputs retain the established sparse per-ID map; get_spec describes authored forms once.
dimension=obj({'kind':nullable({'enum':['linear','diameter','radius','angle']}),'between':nullable(arr(S,minItems=1,maxItems=2)),'along':nullable({'enum':['x','y','z','true']})},extra=True)
spec=obj({'schema':{'enum':['draftwright.spec/1','draftwright.spec/2']},'binding':nullable({'type':'object'}),'sheets':nullable(obj({'max':nullable(SHEET),'smallest_scale':nullable(S),'keep_together':arr(arr(S)),'hole_table':nullable(SHEET),'gdt_on_sheet_1':B})),'sheet':obj({'size':nullable(S),'scale':nullable(S),'title':{'type':'object','additionalProperties':S},'fit_limits':nullable({'type':'boolean'}),'fit_deviations':nullable({'type':'boolean'}),'projection':nullable({'enum':['first','third']}),'default_finish':nullable({'type':'object'})}),'views':{'type':'object','additionalProperties':{'type':'object'}},'dimensions':{'type':'object','additionalProperties':{'type':'object'}},'annotations':{'type':'object','additionalProperties':{'type':'object'}},'features':{'type':'object','additionalProperties':{'type':'object'}},'faces':{'type':'object','additionalProperties':{'type':'object'}},'dimension_source':{'enum':['auto','authored']},'hole_table':{'enum':['auto','always','never']},'notes':arr({'type':'object'}),'tables':arr({'type':'object'}),'gears':arr({'type':'object'})})
# Line-work intent is typed in both scopes; other view fields retain their existing schema.
line_fields = {name: nullable(B) for name in
               ('hidden', 'centre_marks', 'centre_lines', 'thread_depiction', 'knurl_depiction')}
spec['properties']['sheet']['properties'].update(line_fields)
spec['properties']['views']['additionalProperties'] = obj(line_fields, extra=True)
spec_read={**spec,'properties':{**spec['properties'],'dimensions':{'type':'object','additionalProperties':dimension}}}
defs={'state':state,'spec':spec,'spec_read':spec_read,'op':op,'item':item,'compact':compact,'problem':problem,'error':obj({'code':{'enum':['invalid','failed']},'message':S,'choices':arr(S)},['code','message'])}
stub=obj({'id':nullable(S),'detail_ref':S},['id','detail_ref'])
problem_small=obj({'id':S,'code':S,'severity':S,'subjects':arr(S),'fix':arr(ref('op'))},['id','code','severity','subjects'])
defs.update({'stub':stub, 'problem_small':problem_small})
row_item={'anyOf':[ref('compact'),ref('item'),ref('stub')]}
row_problem={'anyOf':[ref('problem'),ref('problem_small'),ref('stub')]}
def paged(row):return obj({'items':arr(row),'total':I,'next_cursor':nullable(I)},['items','total','next_cursor'])
page=paged({'anyOf':[obj({'change':{'enum':['changed','added']},'item':row_item},['change','item']),obj({'change':{'const':'removed'},'id':S},['change','id']),obj({'change':{'enum':['problem_added','problem_resolved']},'problem':row_problem},['change','problem']),ref('stub')]})
delta=obj({'events':page,'delta_ref':S,'plans':I},['events','delta_ref','plans'])
summary=obj({'part_ref':S,'part':S,'face_count':I,'status':{'enum':['clear','needs_attention']},'completeness':obj({'claimed_faces':I,'faces':I,'unplanned_features':I},['claimed_faces','faces','unplanned_features']),'envelope':arr(arr(N,minItems=3,maxItems=3),minItems=2,maxItems=2),'sheet':obj({'size':S,'scale':S,'page':arr(N,minItems=2,maxItems=2)},['size','scale','page']),'sheets':SHEET,'sheet_details':arr(obj({'number':SHEET,'size':S,'scale':S,'page':arr(N,minItems=2,maxItems=2)},['number','size','scale','page'])),'views':arr(S),'items':I,'problems':I,'spec_version':S,'plan_ref':S,'key_items':arr(obj({'id':S,'label':nullable(S)},['id','label']))},['part_ref','part','face_count','status','completeness','envelope','sheet','views','items','problems','spec_version'])
common={'part_ref':S,'spec':ref('spec'),'spec_version':S,'drawing_ref':S}
paging={'cursor':I,'limit':{'type':'integer','minimum':1,'maximum':100},'detail':B}
artifact=obj({'artifact_ref':S,'mime_type':{'enum':['image/png','application/pdf']},'bytes':I,'spec_version':S},['artifact_ref','mime_type','bytes','spec_version'])
tools={
'inspect_part':(obj({**common,'step':S,'features':S,'part':I,'detail':B,'problem_filter':problem_filter}),obj({**summary['properties'],'problem_page':paged({'anyOf':[ref('problem_small'),ref('stub')]})},summary['required'])),
'list_items':(obj({**common,**paging,'filter':filter_},['part_ref']),paged(row_item)),
'get_item':(obj({**common,'id':S,'detail':B},['part_ref','id']),obj({'item':{'anyOf':[ref('item'), obj({**compact['properties'],'basis':S,'value':nullable(N),'operations':arr(S)},[*compact['required'],'basis','value','operations'])]}},['item'])),
'list_problems':(obj({**common,**paging,'filter':problem_filter},['part_ref']),paged(row_problem)),
'item_options':(obj({**common,'id':S},['part_ref','id']),obj({'id':S,'operations':arr(S),'placements':arr(place),'view_anchors':arr(S),'scales':arr(S),'settable':{'type':'object','additionalProperties':obj({'choices':arr({}),'range':arr(N,minItems=2,maxItems=2),'integer':B,'forms':arr(S),'fields':arr(S),'paths':arr(S),'consequences':arr(S)})}},['id','operations','placements','view_anchors','scales','settable'])),
'preview_ops':(obj({**common,**paging,'ops':arr(ref('op'),maxItems=MAX_OPS)},['part_ref','ops']),delta),
'apply_ops':(obj({**common,**paging,'ops':arr(ref('op'),maxItems=MAX_OPS),'parent':S},['part_ref','ops','parent']),obj({'spec_version':S,'delta':delta},['spec_version','delta'])),
'view_sheet':(obj({**common,'region':rectangle,'overlay_ids':{'type':['boolean','array'],'items':S},'sheet':SHEET,'dpi':{'type':'number','exclusiveMinimum':0,'maximum':400}},['part_ref']),artifact),
'render_pdf':(obj({**common,'sheet':SHEET},['part_ref']),artifact),
'get_spec':(obj(common,['part_ref']),obj({'spec_version':S,'spec':ref('spec_read')},['spec_version','spec'])),
'analyse':(obj({'step':S,'features':S,'part':I},['step']),obj({'schema':{'const':'draftwright.analysis/7'},'binding':obj({'source_sha256':S,'part':S,'face_count':I,'loader':S,'part_index':I},['source_sha256','part','face_count','loader']),'envelope':arr(arr(N,minItems=3,maxItems=3),minItems=2,maxItems=2),'features':arr({'type':'object'}),'iso_bounds':rectangle},['schema','binding','envelope','iso_bounds'],True)),
'render':(obj({**common,'step':S,'features':S,'part':I,'format':{'enum':['png','pdf']},'region':rectangle,'overlay_ids':{'type':['boolean','array'],'items':S},'sheet':SHEET,'dpi':{'type':'number','exclusiveMinimum':0,'maximum':400}}),artifact),
'parts':(obj({'step':S},['step']),obj({'parts':arr(obj({'part':I,'name':S,'product_id':S,'faces':I,'instances':I},['part','name','product_id','faces','instances']))},['parts'])),
}
def inline_single_use(schema, keep=()):
    """A one-use local definition costs a reference and key without sharing any shape."""
    from collections import Counter
    from copy import deepcopy
    schema = deepcopy(schema)
    while True:
        counts = Counter()
        def count(value):
            if isinstance(value, dict):
                if '$ref' in value: counts[value['$ref']] += 1
                for child in value.values(): count(child)
            elif isinstance(value, list):
                for child in value: count(child)
        count(schema)
        names = sorted(name for name in schema['$defs']
                       if name not in keep and counts['#/$defs/' + name] == 1)
        if not names: return schema
        definitions = {name: schema['$defs'].pop(name) for name in names}
        def replace(value):
            if isinstance(value, dict):
                name = value.get('$ref', '').removeprefix('#/$defs/')
                if name in definitions: return replace(definitions[name])
                return {k: replace(v) for k, v in value.items()}
            if isinstance(value, list): return [replace(v) for v in value]
            return value
        schema = replace(schema)

def share_repeated(schema):
    """Factor repeated shapes only when the complete standalone schema gets smaller."""
    from collections import Counter
    from copy import deepcopy
    def encoded(value): return json.dumps(value, sort_keys=True, separators=(',', ':'))
    for index in range(20):
        counts = Counter()
        def count(value):
            if isinstance(value, dict):
                counts[encoded(value)] += 1
                for child in value.values(): count(child)
            elif isinstance(value, list):
                for child in value: count(child)
        count(schema)
        best = None
        for shape, occurrences in counts.items():
            if occurrences < 2: continue
            name = chr(ord('A') + index)
            replacement = ref(name)
            if len(shape) <= len(encoded(replacement)): continue
            def replace(value):
                if isinstance(value, dict):
                    if encoded(value) == shape: return replacement
                    return {k: replace(v) for k, v in value.items()}
                if isinstance(value, list): return [replace(v) for v in value]
                return value
            trial = replace(deepcopy(schema))
            trial['$defs'][name] = json.loads(shape)
            saving = len(encoded(schema)) - len(encoded(trial))
            if saving > 0 and (best is None or saving > best[0]): best = (saving, trial)
        if best is None: break
        schema = best[1]
    return schema

def hoist_objects(schema):
    """Move a type shared by every anyOf branch to their parent, preserving accepted values."""
    from copy import deepcopy
    schema = deepcopy(schema)
    def visit(value):
        if isinstance(value, list):
            for child in value: visit(child)
        elif isinstance(value, dict):
            for child in list(value.values()): visit(child)
            branches = value.get('anyOf', [])
            if branches and all(branch.get('type') == 'object' for branch in branches):
                if 'type' not in value and len(branches) > 1:
                    value['type'] = 'object'
                if value.get('type') == 'object':
                    for branch in branches: branch.pop('type')
    visit(schema)
    return schema

def group_properties(schema, top=True):
    """Use anchored name alternatives for equal property shapes, only when smaller.

    This preserves object types, required fields and additionalProperties checks.
    """
    from collections import defaultdict
    if isinstance(schema, list): return [group_properties(v, False) for v in schema]
    if not isinstance(schema, dict): return schema
    schema = {k: group_properties(v, False) for k, v in schema.items()}
    props = schema.get('properties', {})
    groups = defaultdict(list)
    for name, value in props.items():
        if (not top or name not in schema.get('required', [])) and re.fullmatch(r'[A-Za-z0-9_]+', name):
            groups[json.dumps(value, sort_keys=True, separators=(',', ':'))].append(name)
    for shape, names in groups.items():
        if len(names) < 2: continue
        pattern = '^(' + '|'.join(names) + ')$'
        before = len(json.dumps(schema, separators=(',', ':')))
        trial = {**schema, 'properties': {k:v for k,v in props.items() if k not in names},
                 'patternProperties': {**schema.get('patternProperties', {}), pattern: json.loads(shape)}}
        if not trial['properties']: del trial['properties']
        if len(json.dumps(trial, separators=(',', ':'))) < before:
            schema = trial
            props = schema.get('properties', {})
    return schema

def short_names(schema):
    names = {'state':'t','spec':'s','spec_read':'d','op':'op','item':'i','compact':'c',
             'problem':'p','error':'e','stub':'u','problem_small':'r'}
    if isinstance(schema, dict):
        result = {k: short_names(v) for k, v in schema.items()}
        if '$ref' in result:
            name = result['$ref'].split('/')[-1]
            result['$ref'] = '#/$defs/' + names.get(name, name)
        if '$defs' in result:
            result['$defs'] = {names.get(k, k): v for k, v in result['$defs'].items()}
        return result
    if isinstance(schema, list): return [short_names(v) for v in schema]
    return schema

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
        # MCP defaults to 2020-12; local refs need no repeated canonical $id URL.
        schema={**schema,'$defs':used}
        keep = ('op',) if kind == 'input' and tool in ('apply_ops', 'preview_ops') else ()
        # An anyOf branch inherits its parent's type. Keep the existing session byte
        # budget as sheet vocabulary grows, without changing any accepted value.
        for branch in schema.get('anyOf', []):
            if branch.get('type') == schema.get('type'):
                branch.pop('type', None)
        schema = short_names(share_repeated(group_properties(inline_single_use(hoist_objects(schema), keep))))
        if not schema['$defs']: del schema['$defs']
        Path(f'schemas/mcp/{tool}.{kind}.json').write_text(json.dumps(schema,indent=2)+'\n')
