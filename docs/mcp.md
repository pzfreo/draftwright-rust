# Stateless MCP

`draftwright mcp-http [127.0.0.1:8765]` serves local Streamable HTTP at `/mcp`.
`draftwright mcp-stdio` serves the same JSON-RPC tools over newline-delimited stdio.
Set `DRAFTWRIGHT_CACHE` to a persistent directory (default `.cache/draftwright`).
These are development/evaluation adapters, bound to loopback without authentication.
The earlier `draftwright-mcp` executable retains its document-passing compatibility API;
new integrations use the subcommands. No production service was changed.

The HTTP implementation uses the JSON-response mode of [MCP Streamable HTTP
2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports).
POST accepts `application/json` and requires `Accept: application/json, text/event-stream`.
Accepted notifications/responses return 202 without a body. GET returns 405: there is no
unsolicited SSE stream. Initialization negotiates 2025-11-25, 2025-06-18 or 2025-03-26;
subsequent requests may send `MCP-Protocol-Version`, with unknown versions refused.
No `Mcp-Session-Id` or live drawing session is retained. Origins must match the local endpoint.
The local adapter handles requests sequentially, caps bodies at 8 MiB and the complete
request head at 16 KiB (request line, headers, line endings and final blank line, counted
as wire bytes), and refuses chunked bodies. Head reads copy only the remaining budget
into a fixed-capacity buffer; exhausting it before the final blank line returns 431,
including input without a newline. A complete head of exactly 16 KiB is accepted.
One absolute 10-second deadline covers reading the head and body after the connection
is accepted for handling; incremental delivery does not reset it. Expiry returns 408
and releases the adapter for the next connection. This read deadline excludes tool
computation. Cold computation runs synchronously here; production jobs are below.

## Tools and documents

Every query carries `part_ref` and either `spec` or `spec_version`. An absent Spec means
the deterministic default Spec, rather than an implicit remembered edit. `get_spec` without
either loads the explicitly named drawing's head; `drawing_ref` defaults to `default`.
Use different drawing references for independent branches/tasks.

| Tool | Input beyond the part/Spec | Output |
| --- | --- | --- |
| `inspect_part` | `step` and optional `features` to register, or `part_ref`; `detail`, `problem_filter` | Compact sheet summary, bounded key item IDs/labels, counts, default/current input Spec hash; optional `problem_filter` adds a paged corrective answer; detail adds a lossless Plan artifact |
| `list_items` | `filter`, `cursor`, `limit`, `detail` | Paged IDs, labels, kind, view, typed state; detail adds Session item answers |
| `get_item` | `id`, `detail` | Label, state, value, basis and allowed operations; detail adds resolved placement/anchors/features |
| `list_problems` | `filter`, `cursor`, `limit`, `detail` | Paged ID, code, severity and subjects; default includes corrective ops; detail adds message |
| `item_options` | `id` | Operations, relative placements, compatible view anchors and scales, plus every settable escaped Spec path and its choices/forms/ranges |
| `preview_ops` | `ops`, `cursor`, `limit`, `detail` | Paged delta events, planner count and lossless delta artifact; no Spec version/head is kept |
| `apply_ops` | `parent`, `ops`, `drawing_ref`, `limit`, `detail` | Immutable new Spec hash and paged delta; stale parent refuses with current head in message and choices |
| `view_sheet` | `region`, `overlay_ids`, `dpi` | PNG artifact metadata plus MCP image content for vision clients |
| `render_pdf` | — | PDF artifact metadata |
| `get_spec` | Optional version/Spec, or drawing head | Spec and its immutable version hash |
| `analyse` | `step`, optional `features` | One-shot full Analysis document |
| `render` | `step`/`part_ref`, optional `features`, `format` (`png`/`pdf`), PNG region/overlay/dpi | One-shot render artifact metadata |

Tool input and output schemas are standalone draft-2020-12 files under `schemas/mcp/`.
Both adapters serve exactly those documents in `tools/list`, with `outputSchema` and
`structuredContent`. Regenerate with `python3 tools/generate_mcp_schemas.py`.
The shared operation shape is each apply/preview schema's `$defs/op`: `set`, `remove`, `add`,
using the existing RFC 6901 escaped Spec paths (a slash within an ID is `~1`). S5 can consume
that definition for its typed operation wrapper. The model performs semantic validation.
Every tool's output also describes structured refusals (`error.code`, `message`, `choices`).
Linear placement options offer only the two sides compatible with their measured span.
Unknown IDs offer nearest valid IDs; invalid choices/paths come from the same validation as
Session. A malformed input names the accepted schema shape or missing field.

Default lists and delta envelopes are below **4096 UTF-8 JSON bytes**, including the MCP
JSON-RPC response with both its serialized text and structured-content copy. Paging reserves
room for that transport overhead. Paging stops at the
byte limit or the requested record limit (default 20, maximum 100). `next_cursor: null` ends
the collection; cursors are offsets against the same immutable input Spec. Geometry is absent
by default. Oversized detail records become `{id, detail_ref}`; the full record stays available.
A committed delta's subsequent pages are read using
`resources/read {uri: "draftwright:<delta_ref>", events: true, cursor: <next_cursor>}`;
this does not commit the ops again. Preview pages can also be recomputed with the same input.
Lossless JSON artifacts use `resources/read {uri: "draftwright:<artifact_ref>"}` (text);
PNG/PDF artifacts return base64 blobs. These explicit detail reads can exceed the default budget.
The PNG image content is also intentionally outside the text budget. `get_spec` returns the
complete author document, which grows with authored intent rather than projected geometry.

## Library, cache and handle model

`stateless::Cache` exposes `apply(part_ref, spec, ops) -> Applied {spec, delta}`, `preview`,
`items`, `item`, `problems`, `options`, `summary`, and `render(region, format, overlay, dpi)`.
The cache object contains only a filesystem root. Each call reconstructs the same `Session`
from immutable documents, then discards it. No undo stack, live part or planner survives a call.
The cache affects cost, never answers. `serve` additionally exposes `stateless_apply` and
`stateless_preview` for lossless subprocess access, and all tools as `mcp_<tool_name>`.
Tool subprocess requests may put fields inside `arguments` (e.g.
`{"op":"mcp_get_item","arguments":{"part_ref":"...","id":"d:..."}}`) so the transport's
request ID does not conflict with the item's ID. Existing live Session operations are preserved.

`part_ref` is `<STEP SHA-256>-<engine SHA-256>`. `build.rs` derives `ENGINE_VERSION` from
sorted, length-delimited relative paths and contents of `src/**/*.rs`, Cargo.lock, Cargo.toml,
the build scripts, MCP `*.json` schemas and bundled `*.ttf`/`*.otf` fonts. Stray files
(such as `.DS_Store`, swap files and backups) are excluded. Cargo watches files and
directories, so edits, additions and removals of relevant inputs invalidate the key
automatically; no manual suffix bump is needed.
Absolute paths and timestamps are excluded, so identical checkouts share keys. Unknown keys, including keys
from another engine, explain registration with `inspect_part {step: ...}`. Registration copies
STEP bytes, their original basename, and optional recognition JSON. Cold keys compute Analysis,
HLR and visible bounds on demand. Warm calls read Analysis and projections/bounds from disk;
STEP B-rep still parses on reconstruction, but recognition, semantic PMI parsing, HLR and bounds
need not rerun. Source SHA/schema checks refuse damaged bindings.

The local layout is:

- `parts/<part_ref>/`: immutable source and registration, Analysis, HLR/section/placed-geometry
  cache (visible/hidden curves and visible bounds), drawing heads and version-edge references.
- `specs/<hash>.json`: immutable, canonical serialized bound Specs; an applied version hash includes parent and ops as well as Spec content.
- `artifacts/<hash>.<json|png|pdf>`: immutable plans, deltas, history edges and renders.

Sorted numeric cache keys and round-trip float parsing preserve exact projection bytes.
Per-part OS file locks protect cold computation, projection replacement and head comparison
across processes. Writes stage adjacent files and rename; a refused edit never advances its head.
`apply_version` records parent, ops, new Spec and lossless delta before updating the head.
Initial/standalone Spec hashes identify content. Applied version hashes include parent and ops, so a no-op also creates a distinct immutable version and history edge.
Registration retains the first basename for a digest. Different recognition for the same key is
refused: evaluation bridges use a separate cache root from native recognition. There is no
cache eviction in this slice. Operators may remove regenerable artifacts; persisted versions
and history are author state and should be retained.

PNG uses the existing deterministic renderer, with a direct paper-region viewport (millimetres,
y up: `[left,bottom,right,top]`). ID overlays use the bundled font over resolved item bounds;
PDF uses the existing delivery renderer. All exports retain the bound Spec, including crops.

No new Rust crate was added: std supplies sockets, OS locks and atomic files; existing serde,
serde_json and sha2 supply serialization and hashes. `serde_json/float_roundtrip` is enabled
because persisted doubles must read back bit-identically. The independent schema check uses
Python `jsonschema==4.25.1` as a development dependency, not an engine dependency.

## Production integration design: draftwright-io

This section is a design only. The existing authenticated `/mcp-stateless/` endpoint keeps
Supabase bearer-token validation, `X-Maquetto-Project` scope, upload/artifact/job tools, and
leased jobs that any Fly machine can execute. It adds the ten drawing tools above and the
`analyse`/`render` conveniences, publishing the same schemas. It replaces immutable script
versions with immutable Spec versions; no sticky Session is introduced.

1. Upload gives a project-scoped STEP artifact. The server computes its SHA and combines it
   with the deployed engine revision to obtain `part_ref`. Project access is checked before
   resolving that reference or accepting any client-supplied Spec/artifact handle.
2. Registration/analysis jobs stage STEP and recognition inputs in a worker's persistent
   cache. The authoritative Analysis, HLR and bounds artifacts live in shared object storage
   keyed by content and engine version. A worker hydrates its local cache; a cold miss creates
   a leased job handle, while a warm query answers synchronously. Local locks coordinate one
   host only; storage/database leases coordinate Fly machines. A future Python binding may
   replace the subprocess without changing the functions or schemas.
3. The server calls `draftwright serve` with `mcp_inspect_part`, `mcp_list_items`, etc., or the
   lossless `stateless_apply`/`stateless_preview` interface. Requests carry the staged `part_ref`
   and complete Spec or a resolved version. Large artifacts go to object storage; local paths
   are never advertised as public capabilities. The server maps raw content refs to existing
   authenticated artifact handles/URLs and image content.
4. `apply_ops` names a project drawing and immutable parent version. The server resolves its
   Spec, runs the pure apply operation, then transactionally compare-and-swaps the database
   head and persists the new Spec, parent, ops and delta. A race refuses with the current
   project version. The local `apply_version` lock/head implementation is for development;
   it is not a distributed lock. History/undo uses the version chain, not a worker's memory.
5. Rendering is a leased job, resulting in project-scoped PNG/PDF artifact handles. Crops and
   ID overlays are render parameters. Job status/artifact tools remain the existing ones.
   Error choices map engine IDs/versions to project handles; no foreign-project paths leak.

Wiring, rollout flags, auth changes, cloud persistence and a real-model evaluation are deferred.
The editor's WASM Session joins the parity oracle in S5; the native Session/serve/HTTP oracle
here covers the complete shared S4B S0 op sequence on all four slice parts.

## Reproducible acceptance

- `python3 tools/check_http_limits.py target/release/draftwright --report target/http-limits.json`
  checks oversized request lines/headers, aggregate overflow, newline-free input, head/body
  trickles and a queued valid ping after each refusal. It also checks the exact head-budget
  boundary with multibyte UTF-8, body cap, Origin rules, loopback binding and valid JSON-RPC.
- `DRAFTWRIGHT_CORPUS=... DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release --test stateless -- --nocapture`
  checks schema contracts, 4 KB budgets, fresh-process warm-cache equality, Session/serve/HTTP
  op-sequence parity, refusal/concurrency behaviour, transports and render embedding.
- Install `jsonschema==4.25.1` into a Python environment, then run
  `python tools/check_mcp_schemas.py target/release/draftwright` for independent schema validation.
- `node tools/mcp-eval/run-scripted.mjs --out target/s7-mcp-eval.json` starts a local HTTP adapter,
  runs all seven unchanged S0 tasks, and fails if any task fails. `--url` connects to an existing
  adapter; `--binary` selects a binary. The task set/features and exact S0 baseline are copied
  from draftwright-editor `origin/thin-slice`. Parts resolve from the bundled mounting plate
  and `DRAFTWRIGHT_CORPUS`. The judge's success-condition logic is unchanged; only its import
  and task-file location were adapted. Per-task bytes/tokens exclude judge calls as S0 did;
  tokens are `ceil(bytes/4)`, not actual model tokenizer counts. Fixed context is reported
  separately. Task IDs/quantities are known scripted inputs, not inferred model outputs.

D1 adds the real-model call client and finished-task checker; see
[`tools/mcp-eval/README.md`](../tools/mcp-eval/README.md) for starting HTTP, listing tools,
calling, logging bytes/tokens and scoring. Served descriptions explain each tool without
requiring source access. Schemas factor repeated shapes through local `$defs` while
remaining standalone. `cargo test --release --test engine_version` checks the source hash,
cache namespace, tool descriptions and a 39,500-byte per-session schema budget.

`inspect_part.key_items` is a deterministic, 700-byte index: envelope dimensions and
representative feature sizes/dimensions. Use `list_items` for the complete list.
`inspect_part.problem_filter` batches opening and a problem query; its `problem_page`
has the same paging contract as `list_problems`. A problem filter's `feature` matches
all associated item subjects and the feature itself; `subject` remains exact and all
filter fields combine. Default problem rows now carry `fix` ops.

`item_options.settable` maps escaped Spec paths to accepted `choices`, numeric `range`
(inclusive, with `integer` where needed), tolerance `forms`, or title `fields`. A
fit descriptor with `paths` applies to all listed feature paths. Fits list only classes
valid for the feature kind and recognised size in the engine's ISO 286 table. Tolerances
are strings (`±T` or `+U -L`); fits use `features/<id>/fit`. View keys are bare (`iso`)
and the drawing number is `sheet/title/number`. Compatible view/side pairs remain in
`placements`; candidates require preview/apply, and a view relation needs both
`relative_to` and `side`. Set several subfields by setting their complete `place` object.

Common unambiguous vocabulary errors include valid choices and an explicit
`Corrected op: {JSON}` in the refusal message. They keep the invalid edit unapplied;
The message identifies the replaced zero-based `ops[index]`; the correction is
previewed through Session and corresponds to the reported refusal. A batch with multiple
invalid operations can still require another retry. Review and resubmit the correction. Ambiguous feature ownership and missing engineering
magnitudes are not guessed. Batch all edits for one request in `apply_ops.ops`.
