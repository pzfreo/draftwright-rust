// Run from the repository root after a web wasm-pack build and:
// cargo build --release --example wasm_reference
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import init, { Engine, start } from '../crates/draftwright-wasm/pkg/draftwright_wasm.js';

await init({ module_or_path: readFileSync(new URL('../crates/draftwright-wasm/pkg/draftwright_wasm_bg.wasm', import.meta.url)) });
start(); // Also called automatically by wasm-bindgen's start hook; idempotent.
const decoder = new TextDecoder();
const root = fileURLToPath(new URL('../', import.meta.url));
function documents(engine, edit) {
  const svg = engine.renderSvg();
  assert.ok(svg instanceof Uint8Array);
  return { edit, spec: engine.spec(), plan: engine.plan(), svg: decoder.decode(svg) };
}
function equalBytes(actual, expected, label) {
  const a = Buffer.from(actual), b = Buffer.from(expected);
  if (a.equals(b)) return;
  let offset = 0;
  while (offset < Math.min(a.length, b.length) && a[offset] === b[offset]) offset++;
  assert.fail(`${label} byte ${offset}: WASM ${a.subarray(offset, offset + 100)}; native ${b.subarray(offset, offset + 100)}`);
}

function invalid(action) {
  assert.throws(action, error => {
    assert.equal(typeof error, 'string');
    const parsed = JSON.parse(error);
    assert.equal(parsed.code, 'invalid');
    assert.equal(typeof parsed.message, 'string');
    return true;
  });
}

function replayPlan(previous, delta) {
  const plan = structuredClone(previous);
  const id = item => item.id ?? `sheet:${item.number}`;
  for (const [name, value] of Object.entries(delta.fields)) {
    if (value === null) delete plan[name];
    else plan[name] = value;
  }
  for (const [name, patch] of Object.entries(delta.collections)) {
    if (!patch.present) { delete plan[name]; continue; }
    const records = new Map((plan[name] ?? []).map(item => [id(item), item]));
    const order = patch.order ?? (plan[name] ?? []).map(id);
    for (const key of patch.removed) records.delete(key);
    for (const [key, value] of Object.entries(patch.set)) records.set(key, value);
    plan[name] = order.map(key => {
      assert.ok(records.has(key), `delta is missing ${key}`);
      const value = records.get(key);
      records.delete(key);
      return value;
    });
    assert.equal(records.size, 0, 'delta records are missing from order');
  }
  return plan;
}

for (const part of ['prismatic', 'polygonal']) {
  test(`${part}: WASM and native plan, spec and SVG bytes match throughout a session`, () => {
    const replay = JSON.parse(readFileSync(new URL(`../tests/fixtures/wasm/${part}.json`, import.meta.url)));
    const plain = readFileSync(new URL(`../${replay.part}`, import.meta.url));
    const bytes = replay.gzip ? gzipSync(plain) : plain;
    const engine = new Engine(replay.name, bytes);
    try {
      const snapshot = JSON.parse(engine.snapshot());
      assert.deepEqual(snapshot.plan, JSON.parse(engine.plan()));
      assert.deepEqual(snapshot.spec, JSON.parse(engine.spec()));
      assert.deepEqual(snapshot.placementOptions, JSON.parse(engine.placementOptions()));
      assert.deepEqual(snapshot.pmi, JSON.parse(engine.pmi()));
      assert.equal(snapshot.svg, decoder.decode(engine.renderSvg()));
      assert.ok(engine.faceCount() > 0);
      assert.ok(engine.featureCount() > 0);
      assert.ok(JSON.parse(engine.fragments()));
      invalid(() => engine.undo());
      invalid(() => engine.redo());
      // Exercise recognition correction, bound face anchors, delta replay and history on
      // the existing planar fixtures; exports and comparison tolerances stay unchanged.
      const corrected = snapshot.analysis.features.find(feature => feature.faces.length > 0);
      assert.ok(corrected);
      replay.steps.push(
        { edit: 'suppress recognition', action: 'fragments', ops: [{ op: 'set', path: `features/${corrected.id}/suppress`, value: true }] },
        { edit: 'undo recognition', action: 'undo' },
        { edit: 'redo recognition', action: 'redo' },
      );
      const actual = [documents(engine, 'open')];
      for (const step of replay.steps) {
        const ops = JSON.stringify(step.ops ?? []);
        const before = engine.planCount();
        const previousPlan = JSON.parse(engine.plan());
        if (step.action === 'undo') JSON.parse(engine.undo());
        else if (step.action === 'redo') JSON.parse(engine.redo());
        else if (step.action === 'preview') {
          const saved = documents(engine, 'saved');
          const preview = JSON.parse(engine.preview(ops));
          assert.deepEqual(documents(engine, 'saved'), saved);
          JSON.parse(engine.apply(ops, false));
          assert.deepEqual(documents(engine, 'saved'), saved);
          assert.equal(engine.planCount(), before);
          actual.push({ edit: step.edit, spec: JSON.stringify(preview.spec), plan: JSON.stringify(preview.plan), svg: preview.svg });
          continue;
        } else if (step.action === 'spec') {
          const spec = JSON.parse(engine.spec());
          // Fixture's whole-spec edit deliberately exercises the wrapper's diff/migration.
          spec.sheet.title.material = step.ops[0].value;
          engine.setSpec(JSON.stringify(spec));
        } else if (step.action === 'fragments') {
          const packet = JSON.parse(engine.applyFragments(ops));
          assert.ok(packet.delta && packet.fragments && packet.pmi);
          assert.deepEqual(replayPlan(previousPlan, packet.delta.plan), JSON.parse(engine.plan()));
          const complete = JSON.parse(engine.fragments());
          for (const [id, svg] of Object.entries(packet.fragments)) {
            assert.equal(svg, complete[id], `changed fragment ${id}`);
          }
          if (step.ops.some(op => op.path.startsWith('views'))) {
            assert.deepEqual(packet.placementOptions, JSON.parse(engine.placementOptions()));
          }
          for (const [name, patch] of Object.entries(packet.delta.plan.collections)) {
            assert.equal(patch.present, Object.hasOwn(JSON.parse(engine.plan()), name));
          }
        } else JSON.parse(engine.apply(ops, true));
        assert.equal(engine.planCount() - before, 1, step.edit);
        actual.push(documents(engine, step.edit));
      }
      const native = spawnSync(process.env.DRAFTWRIGHT_WASM_REFERENCE ?? 'target/release/examples/wasm_reference', [], {
        cwd: root, input: JSON.stringify(replay), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
      });
      assert.equal(native.status, 0, native.error?.message ?? native.stderr);
      const expected = native.stdout.trim().split('\n').map(line => JSON.parse(line));
      assert.equal(actual.length, expected.length);
      for (let i = 0; i < actual.length; i++) {
        assert.equal(actual[i].edit, expected[i].edit);
        for (const document of ['spec', 'plan', 'svg']) {
          if (i > 0 && replay.steps[i - 1].action === 'preview' && document !== 'svg') {
            // Preview embeds JSON documents in a snapshot; JS reserialization changes
            // object order and float spelling. Compare those documents structurally.
            assert.deepEqual(JSON.parse(actual[i][document]), JSON.parse(expected[i][document]));
          } else equalBytes(actual[i][document], expected[i][document], `${part}: ${actual[i].edit} ${document}`);
        }
      }
      invalid(() => engine.apply('{', true));
      invalid(() => engine.applyFragments('{'));
      invalid(() => engine.preview('{'));
      invalid(() => engine.setSpec('{'));
      invalid(() => engine.setPmi('another source', bytes));
      invalid(() => engine.setPmiOps('another source', bytes, '[]'));
      engine.setPmi(snapshot.provenance.source_sha256, bytes);
      engine.setPmiOps(snapshot.provenance.source_sha256, bytes, '[]');
      const pdf = engine.renderPdf();
      assert.ok(pdf instanceof Uint8Array);
      assert.equal(decoder.decode(pdf.subarray(0, 5)), '%PDF-');
      console.log(`${part}: ${actual.length} states, ${actual.length * 3 - 2} byte comparisons, 2 preview document comparisons`);
    } finally { engine.free(); }
  });
}
test('bad STEP is thrown as a JSON api::Error string', () => {
  invalid(() => new Engine('bad.step', new Uint8Array([0, 1, 2])));
});

test('geometry-authored dimensions replay with identical native/WASM references and ink', () => {
  const replay = JSON.parse(readFileSync(new URL('../tests/fixtures/wasm/prismatic.json', import.meta.url)));
  const engine = new Engine(replay.name, readFileSync(new URL(`../${replay.part}`, import.meta.url)));
  try {
    const refs = Object.entries(JSON.parse(engine.snapshot()).analysis.edge_ink.references)
      .filter(([, records]) => records.length === 1 && records[0].kind === 'vertex');
    const pair = refs.flatMap(p => refs.map(q => [p, q])).find(([[, [p]], [, [q]]]) =>
      Math.abs(p.point[1] - q.point[1]) < 1e-7 && Math.abs(p.point[0] - q.point[0]) > 1e-3 && Math.abs(p.point[2] - q.point[2]) > 1e-3);
    assert.ok(pair, 'principal-plane vertex diagonal');
    const ops = [{ op: 'set', path: 'dimensions/d:authored:diagonal', value: {
      between: pair.map(([key]) => key), along: 'true', view: 'front', tolerance: '±0.1',
    } }];
    replay.steps = [{ edit: 'author diagonal', action: 'apply', ops },
      { edit: 'undo authored', action: 'undo' }, { edit: 'redo authored', action: 'redo' }];
    const actual = [documents(engine, 'open')];
    for (const step of replay.steps) {
      if (step.action === 'undo') engine.undo();
      else if (step.action === 'redo') engine.redo();
      else engine.apply(JSON.stringify(step.ops), true);
      actual.push(documents(engine, step.edit));
    }
    const native = spawnSync(process.env.DRAFTWRIGHT_WASM_REFERENCE ?? 'target/release/examples/wasm_reference', [], {
      cwd: root, input: JSON.stringify(replay), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
    });
    assert.equal(native.status, 0, native.error?.message ?? native.stderr);
    const expected = native.stdout.trim().split('\n').map(line => JSON.parse(line));
    assert.equal(actual.length, expected.length);
    for (let i = 0; i < actual.length; i++) {
      for (const document of ['spec', 'plan', 'svg']) equalBytes(actual[i][document], expected[i][document], `authored ${i} ${document}`);
    }
  } finally { engine.free(); }
});
