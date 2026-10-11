// S0's unchanged task conditions, driven only through the stateless HTTP MCP tools.
// node tools/mcp-eval/run-scripted.mjs [--binary target/release/draftwright] [--url URL] [--out FILE]
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { check, loadTasks } from './check.mjs';
import { localParts, featureNames } from './parts.mjs';
import { serverUrl } from './client.mjs';
import { pinCallouts, keepIntent } from './scripted-ops.mjs';
const args=process.argv.slice(2);
const option=(key,fallback)=>args.includes(key)?args[args.indexOf(key)+1]:fallback;
const tokens=bytes=>Math.ceil(bytes/4);
const size=v=>Buffer.byteLength(JSON.stringify(v));
const segment=id=>id.replaceAll('~','~0').replaceAll('/','~1');
const set=(path,value)=>({op:'set',path,value});
let child, cache, url=option('--url',null), nextId=0;
try {
  if(!url){
    cache=mkdtempSync(resolve(tmpdir(),'draftwright-mcp-eval-'));
    child=spawn(resolve(option('--binary','target/release/draftwright')),['mcp-http','127.0.0.1:0'],{env:{...process.env,DRAFTWRIGHT_CACHE:cache},stdio:['ignore','ignore','pipe']});
    url=await new Promise((done,fail)=>{let text='';child.stderr.on('data',chunk=>{text+=chunk;const found=serverUrl(text);if(found)done(found);});child.on('error',fail);child.on('exit',code=>fail(new Error(`server exited ${code}: ${text}`)));});
  }
  const rpc=async(method,params)=>{
    const response=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json',Accept:'application/json, text/event-stream','MCP-Protocol-Version':'2025-06-18'},body:JSON.stringify({jsonrpc:'2.0',id:++nextId,method,params})});
    if(!response.ok)throw new Error(`HTTP ${response.status}`);const result=await response.json();if(result.error)throw new Error(JSON.stringify(result.error));return result.result;
  };
  const invoke=async(name,arguments_)=>rpc('tools/call',{name,arguments:arguments_});
  const tool=async(name,arguments_)=>{const result=await invoke(name,arguments_);if(result.isError)throw new Error(result.content[0].text);return result.structuredContent;};
  const artifact=async(ref)=>{const r=await rpc('resources/read',{uri:`draftwright:${ref}`});return JSON.parse(r.contents[0].text);};
  const info=await rpc('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'s7-scripted',version:'1'}});
  const tools=await rpc('tools/list',{});
  const data=loadTasks(), results=[];
  for(const task of data.tasks){
    const calls=[];
    const call=async(name,args)=>{const result=await invoke(name,args);calls.push({tool:name,isError:result.isError,request_bytes:size(args),response_bytes:Buffer.byteLength(result.content[0].text)});if(result.isError)throw new Error(result.content[0].text);return result.structuredContent;};
    let initial, current, part_ref, failure;
    try {
      initial=await call('inspect_part',{step:localParts[task.part],features:resolve('tools/mcp-eval/features',featureNames[task.part]+'.features.json'),drawing_ref:task.id,...(task.id==='ctc01-fix-problems-near-slot'&&!args.includes('--separate-problems')?{problem_filter:{feature:'f:slot:1'}}:{})});
      part_ref=initial.part_ref;current=initial.spec_version;
      const apply=async ops=>{const r=await call('apply_ops',{part_ref,drawing_ref:task.id,parent:current,ops});current=r.spec_version;return r;};
      const dimension=(q,field,v)=>set(`dimensions/${segment('d:'+q)}/${field}`,v);
      const keyItem = predicate => { const found=initial.key_items.find(predicate); if(!found)throw new Error('summary has no matching key item'); return found; };
      const width = () => keyItem(i=>i.id==='d:envelope/width.length').id.slice(2);
      switch(task.id){
        case 'plate-callouts-right-pinned':
          await apply(pinCallouts(task,initial.key_items));break;
        case 'm10127-hide-width':await apply([dimension(width(),'omit',true)]);break;
        case 'ctc01-fix-problems-near-slot':{
          // Opening and the feature's complete corrective answer share one bounded call.
          let problems=initial.problem_page??await call('list_problems',{part_ref,spec_version:current,filter:{feature:'f:slot:1'}});
          const fixes=problems.items.flatMap(p=>p.fix??[]);
          while(problems.next_cursor!==null){
            problems=await call('list_problems',{part_ref,spec_version:current,filter:{feature:'f:slot:1'},cursor:problems.next_cursor});
            fixes.push(...problems.items.flatMap(p=>p.fix??[]));
          }
          if(fixes.length)await apply(fixes);
          break;
        }
        case 'grm03-sheet-scale-keep-intent':
          await keepIntent(apply,keyItem(i=>i.label?.includes('5')&&i.id.endsWith('/step.diameter')).id);break;
        case 'm10127-fit-through-hole':{
          const hole=keyItem(i=>i.id.endsWith('/bore.diameter')&&i.label?.includes('8.1'));
          // H7 is requested by the task, as in the pre-E2 instrument.
          await apply([set(`features/${segment(hole.id.slice(2).split('/')[0])}/fit`,'H7')]);break;
        }
        case 'plate-a4-title-no-iso':await apply([set('views/iso/omit',true),set('sheet/size','A4'),set('sheet/scale','1:1'),set('sheet/title/title','MOUNTING PLATE'),set('sheet/title/number','DW-0001')]);break;
        case 'plate-width-in-plan':await apply([dimension(width(),'place',{view:'plan',side:'above'})]);break;
        default:throw new Error(`no script for ${task.id}`);
      }
    } catch(error){failure=error.message;}
    let verdict={success:false,conditions:[]};
    if(part_ref){
      // S0 also excludes the judge's full plans from the agent's call and token counts.
      const baselineRef=await tool('inspect_part',{part_ref,spec_version:initial.spec_version,detail:true});
      const finalRef=await tool('inspect_part',{part_ref,spec_version:current,detail:true});
      const baseline=await artifact(baselineRef.plan_ref),plan=await artifact(finalRef.plan_ref);
      const spec=(await tool('get_spec',{part_ref,spec_version:current})).spec;
      verdict=check(task,{spec,plan,baseline});
      const documents=option('--documents-dir',null);
      if(documents){
        const dir=resolve(documents,task.id);mkdirSync(dir,{recursive:true});
        for(const [name,document] of Object.entries({spec,plan,baseline}))writeFileSync(resolve(dir,name+'.json'),JSON.stringify(document,null,2)+'\n');
      }
    }
    const request_bytes=calls.reduce((s,c)=>s+c.request_bytes,0),response_bytes=calls.reduce((s,c)=>s+c.response_bytes,0);
    const result={task:task.id,part:task.part,success:verdict.success&&!failure,tool_calls:calls.length,refused_calls:calls.filter(c=>c.isError).length,request_bytes,response_bytes,tokens_estimate:tokens(request_bytes)+tokens(response_bytes),failure,failed_conditions:verdict.conditions.filter(c=>!c.pass),calls};results.push(result);
    console.error(`${task.id}: ${result.success?'success':'FAIL'} (${calls.length} calls) ${failure??JSON.stringify(result.failed_conditions)}`);
  }
  const report={format:'draftwright.mcp-eval-result/1',agent:'scripted, stateless HTTP; S0 task conditions unchanged',tokens_estimate:'ceil(bytes / 4) of tool arguments and result text; no tokenizer; judge calls excluded as in S0',server:info.serverInfo,fixed_context:{instructions_bytes:Buffer.byteLength(info.instructions),tools_bytes:size(tools.tools),tokens_estimate:tokens(Buffer.byteLength(info.instructions))+tokens(size(tools.tools))},summary:{tasks:results.length,succeeded:results.filter(r=>r.success).length,tool_calls:results.reduce((s,r)=>s+r.tool_calls,0),tokens_estimate:results.reduce((s,r)=>s+r.tokens_estimate,0)},results};
  const out=resolve(option('--out','target/s7-mcp-eval.json'));mkdirSync(dirname(out),{recursive:true});writeFileSync(out,JSON.stringify(report,null,2)+'\n');
  if(report.summary.succeeded!==data.tasks.length)process.exitCode=1;
} finally {
  if(child && child.exitCode===null && child.signalCode===null){const ended=new Promise(resolve=>child.once('exit',resolve));child.kill();await ended;}
  if(cache)rmSync(cache,{recursive:true,force:true});
}
