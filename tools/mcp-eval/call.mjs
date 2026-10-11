#!/usr/bin/env node
// A single stateless HTTP request. Judge requests use client.mjs without a log.
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { rpc } from './client.mjs';
import { loadTasks } from './check.mjs';

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const at = args.indexOf(name);
  if (at < 0) return fallback;
  if (!args[at + 1]) throw new Error(`${name} needs a value`);
  return args.splice(at, 2)[1];
};

try {
  const url = option('--url', process.env.MCP_EVAL_URL ?? 'http://127.0.0.1:8765/mcp');
  const log = resolve(option('--log', process.env.MCP_EVAL_LOG ?? '.cache/mcp-eval/calls.jsonl'));
  const task = option('--task', process.env.MCP_EVAL_TASK);
  const out = option('--out');
  const imageOut = option('--image-out');
  const [name, given = '{}'] = args;
  if (!name) throw new Error('usage: call.mjs tools | tasks [id] | <tool> <json|@file> [--url URL] [--task id] [--log file] [--out file]');
  if (name === 'tasks') {
    const data = loadTasks();
    const tasks = args[1] ? data.tasks.filter(t => t.id === args[1]) : data.tasks;
    if (!tasks.length) throw new Error(`unknown task ${args[1]}`);
    console.log(JSON.stringify(tasks, null, 1));
  } else {
    const arguments_ = name === 'tools' ? {} : JSON.parse(given.startsWith('@') ? readFileSync(given.slice(1), 'utf8') : given);
    const { result, content, entry } = await rpc(url, name === 'tools' ? 'tools/list' : 'tools/call',
      name === 'tools' ? {} : { name, arguments: arguments_ }, { log, task });
    if (imageOut && !result.isError) {
      const image = result.content?.find(c => c.type === 'image');
      if (!image) throw new Error('tool returned no image content for --image-out');
      writeFileSync(imageOut, Buffer.from(image.data, 'base64'));
    }
    const output = name === 'tools' ? JSON.stringify(result, null, 1) : content || JSON.stringify(result);
    if (out) writeFileSync(out, output + '\n');
    else process.stdout.write(output + '\n');
    console.error(`[${name}] ${entry.isError ? 'error' : 'ok'}: wire ${entry.wire.request}/${entry.wire.response} B; content ${entry.content.request}/${entry.content.response} B (~${entry.tokens.request + entry.tokens.response} tokens)`);
    process.exitCode = entry.isError ? 2 : 0;
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
