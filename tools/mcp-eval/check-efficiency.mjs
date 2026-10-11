// Compare scripted reports, retaining every previously successful task.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
export function checkEfficiency(before, after) {
  const failures = [];
  for (const [name, report] of [['before', before], ['after', after]]) {
    if (!Array.isArray(report.results) || report.results.some(result =>
      typeof result.task !== 'string' || typeof result.success !== 'boolean' ||
      !Number.isInteger(result.tool_calls) || result.tool_calls < 0 ||
      !Array.isArray(result.calls) || result.calls.length !== result.tool_calls)) {
      return { pass: false, failures: [`${name}: invalid task/call accounting`] };
    }
    if (new Set(report.results.map(result => result.task)).size !== report.results.length) {
      failures.push(`${name}: duplicate task`);
    }
  }
  const tasks = new Map(after.results.map(result => [result.task, result]));
  if (after.results.length !== before.results.length || tasks.size !== before.results.length || before.results.some(result => !tasks.has(result.task))) {
    failures.push('task set changed');
  }
  for (const result of before.results) {
    if (result.success && !tasks.get(result.task)?.success) failures.push(`${result.task}: lost success`);
  }
  const calls = report => report.results.reduce((sum, result) => sum + result.tool_calls, 0);
  if (calls(after) >= calls(before)) failures.push(`calls did not drop: ${calls(before)} → ${calls(after)}`);
  return { pass: failures.length === 0, before: calls(before), after: calls(after), failures };
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  if (process.argv.length !== 4) throw new Error('usage: node check-efficiency.mjs BEFORE.json AFTER.json');
  const result = checkEfficiency(...process.argv.slice(2).map(file => JSON.parse(readFileSync(file, 'utf8'))));
  console.log(JSON.stringify(result));
  if (!result.pass) process.exitCode = 1;
}
