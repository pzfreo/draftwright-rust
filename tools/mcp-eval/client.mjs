import { appendFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

export const tokens = bytes => Math.ceil(bytes / 4);
export const size = value => Buffer.byteLength(JSON.stringify(value));

export async function rpc(url, method, params, { log, task } = {}) {
  const request = JSON.stringify({ jsonrpc: '2.0', id: 1, method, params });
  let response = '', reply, failure;
  try {
    const fetched = await fetch(url, {
      method: 'POST', headers: { 'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream', 'MCP-Protocol-Version': '2025-06-18' },
      body: request, signal: AbortSignal.timeout(600000)
    });
    response = await fetched.text();
    if (!fetched.ok) throw new Error(`HTTP ${fetched.status}: ${response}`);
    reply = JSON.parse(response);
    if (reply.error) throw new Error(JSON.stringify(reply.error));
  } catch (error) { failure = error; }
  const result = reply?.result;
  const content = (result?.content ?? []).filter(c => c.type === 'text').map(c => c.text).join('\n');
  const entry = { task, method, tool: params.name, isError: !!failure || !!result?.isError,
    wire: { request: Buffer.byteLength(request), response: Buffer.byteLength(response) },
    content: { request: size(params.arguments ?? params), response: Buffer.byteLength(content || JSON.stringify(result ?? null)) } };
  entry.tokens = { request: tokens(entry.content.request), response: tokens(entry.content.response) };
  if (failure) entry.failure = failure.message;
  if (log) {
    mkdirSync(dirname(log), { recursive: true });
    appendFileSync(log, JSON.stringify(entry) + '\n');
  }
  if (failure) throw failure;
  return { result, content, entry };
}

export async function tool(url, name, args) {
  const { result } = await rpc(url, 'tools/call', { name, arguments: args });
  if (result.isError) throw new Error(result.content[0].text);
  return result.structuredContent;
}

export async function artifact(url, ref) {
  const { result } = await rpc(url, 'resources/read', { uri: `draftwright:${ref}` });
  return JSON.parse(result.contents[0].text);
}
