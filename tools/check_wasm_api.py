"""Smoke-check label wording through the built browser API (after wasm-pack build)."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = r'''
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import init, {Engine} from './crates/draftwright-wasm/pkg/draftwright_wasm.js';
await init({module_or_path:readFileSync('./crates/draftwright-wasm/pkg/draftwright_wasm_bg.wasm')});
const engine = new Engine('prismatic.step', readFileSync('./tests/fixtures/parts/prismatic.step'));
try {
  const id = 'd:envelope/width.length';
  const path = 'dimensions/d:envelope~1width.length';
  const original = JSON.parse(engine.plan()).dimensions.find(d => d.id === id);
  const ops = [{op:'set',path:path+'/text',value:{template:'{value} AFTER PLATING'}},
               {op:'set',path:path+'/style',value:'basic'}];
  engine.apply(JSON.stringify(ops),true);
  const saved = engine.spec();
  const d = JSON.parse(engine.plan()).dimensions.find(d => d.id === id);
  assert.equal(d.label, original.label+' AFTER PLATING');
  assert.equal(d.value,original.value);
  assert.deepEqual(d.measurements,original.measurements);
  assert.ok(d.from_spec.includes('text') && d.from_spec.includes('style.basic'));
  const svg = new TextDecoder().decode(engine.renderSvg());
  assert.ok(svg.includes('AFTER PLATING'));
  const good = engine.plan();
  assert.throws(() => engine.apply(JSON.stringify([{op:'set',path:path+'/text',value:{template:'99 AFTER PLATING'}}]),true), error => {
    assert.equal(JSON.parse(error).code,'invalid'); return true;
  });
  assert.equal(engine.plan(),good);
  engine.undo();
  assert.equal(JSON.parse(engine.plan()).dimensions.find(d => d.id === id).label,original.label);
  engine.setSpec(saved);
  assert.equal(engine.spec(),saved);
  assert.equal(JSON.parse(engine.plan()).dimensions.find(d => d.id === id).label,d.label);
  engine.apply(JSON.stringify([{op:'set',path:path+'/style',value:'reference'}]),true);
  const ref = JSON.parse(engine.plan()).dimensions.find(d => d.id === id);
  assert.equal(ref.reference,true);
  assert.equal(ref.label,'('+d.label+')');
  console.log('WASM API label text/style, values, saved Spec, refusal atomicity and history passed');
} finally { engine.free(); }
'''
subprocess.run(['node', '--input-type=module', '--eval', SCRIPT], cwd=ROOT, check=True)
