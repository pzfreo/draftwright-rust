import { createHash } from 'node:crypto';
import { localParts } from './parts.mjs';
import { readFileSync } from 'node:fs';
import { check, loadTasks } from './check.mjs';
import { tool, artifact } from './client.mjs';
const args = process.argv.slice(2);
const option = name => {
  const at = args.indexOf(name);
  return at < 0 ? undefined : args.splice(at, 2)[1];
};
const read = path => JSON.parse(readFileSync(path, 'utf8'));
try {
  const specFile = option('--spec'), planFile = option('--plan'), baselineFile = option('--baseline');
  const part_ref = option('--part-ref'), spec_version = option('--spec-version');
  const url = option('--url') ?? process.env.MCP_EVAL_URL ?? 'http://127.0.0.1:8765/mcp';
  const task = loadTasks().tasks.find(t => t.id === args[0]);
  if (!task) throw new Error(`choose a task from tasks.json; got ${args[0]}`);
  let spec = specFile ? read(specFile) : undefined, plan, baseline;
  if (planFile && baselineFile && spec) {
    plan = read(planFile); baseline = read(baselineFile);
  } else {
    if (!part_ref || (!spec && !spec_version)) throw new Error('check needs --part-ref and --spec-version (or --spec); offline: --spec --plan --baseline');
    const expected = createHash('sha256').update(readFileSync(localParts[task.part])).digest('hex');
    if (part_ref.split('-')[0] !== expected) throw new Error(`part_ref must name ${task.part}, the part assigned to ${task.id}`);
    const current = await tool(url, 'inspect_part', { part_ref, ...(spec ? { spec } : { spec_version }), detail: true });
    const initial = await tool(url, 'inspect_part', { part_ref, detail: true });
    plan = await artifact(url, current.plan_ref);
    baseline = await artifact(url, initial.plan_ref);
    if (!spec) spec = (await tool(url, 'get_spec', { part_ref, spec_version })).spec;
  }
  const verdict = check(task, { spec, plan, baseline });
  console.log(JSON.stringify({ task: task.id, ...verdict }, null, 1));
  process.exitCode = verdict.success ? 0 : 1;
} catch (error) { console.error(error.message); process.exitCode = 2; }
