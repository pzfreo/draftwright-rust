# Stateless HTTP model evaluation

Run from the draftwright-rust repository root with Node 22+ and the release binary.
The committed `tasks.json` and `check.mjs` retain S0's seven task conditions. A real
model receives the task prompt and the served tool descriptions/schemas, then uses
these commands; it does not need the source. The scripted agent knows the answers
and is a separate instrument, not evidence of real-model performance.

Start the server in a separate terminal, with a fresh persistent evaluation cache
(separate from native-recognition caches because these tasks use S0 feature files):

```sh
export DRAFTWRIGHT_CACHE="$PWD/.cache/mcp-eval/http"
target/release/draftwright mcp-http 127.0.0.1:8765
```

List tools and select a task:

```sh
node tools/mcp-eval/call.mjs tools --out /tmp/tools.json
node tools/mcp-eval/call.mjs tasks plate-callouts-right-pinned
export MCP_EVAL_TASK=plate-callouts-right-pinned
export MCP_EVAL_LOG="$PWD/.cache/mcp-eval/model-calls.jsonl"
node tools/mcp-eval/call.mjs inspect_part '{"step":"tests/fixtures/parts/mounting_plate.step","features":"tools/mcp-eval/features/plate.features.json","drawing_ref":"plate-callouts-right-pinned"}'
```

`inspect_part` returns `part_ref` and the initial `spec_version`. Register the other
parts using `$DRAFTWRIGHT_CORPUS/mfcadpp/10127.step`,
`$DRAFTWRIGHT_CORPUS/gramel/GRM-03_thumbwheel_drive_screw.step` and
`$DRAFTWRIGHT_CORPUS/nist/nist_ctc_01_asme1_rd.stp`, with the matching files in
`tools/mcp-eval/features/`. Shell variables inside single-quoted JSON are literal;
put expanded paths into an argument file instead.

Make one tool call per command. Arguments may be JSON or `@file.json`. Substitute
returned handles in these examples:

```sh
node tools/mcp-eval/call.mjs list_items '{"part_ref":"PART","spec_version":"VERSION"}'
node tools/mcp-eval/call.mjs get_item '{"part_ref":"PART","spec_version":"VERSION","id":"d:envelope/width.length","detail":true}'
node tools/mcp-eval/call.mjs item_options '{"part_ref":"PART","spec_version":"VERSION","id":"d:envelope/width.length"}'
node tools/mcp-eval/call.mjs preview_ops @/tmp/preview.json
node tools/mcp-eval/call.mjs apply_ops @/tmp/apply.json --out /tmp/applied.json
```

An operation is `{"op":"set","path":"dimensions/d:envelope~1width.length/place",
"value":{"view":"plan","side":"above","lane":0,"pinned":true}}`.
Use `~1` for a slash and `~0` for a tilde within a Spec key. Preview arguments are
`{part_ref,spec_version,ops}`; apply arguments are
`{part_ref,drawing_ref,parent,ops}`. Use the current version as `parent` and keep the
new returned version. An absent query version uses the default Spec. `get_spec`
without a version reads the named drawing head. Follow `next_cursor` for lists.
To page a committed delta, read its artifact through MCP `resources/read`, rather
than repeating `apply_ops`. `view_sheet` returns PNG image content plus an artifact
handle. Save the image for a vision model's image reader while printing the text result:

```sh
node tools/mcp-eval/call.mjs view_sheet '{"part_ref":"PART","spec_version":"VERSION","overlay_ids":true}' --image-out /tmp/sheet.png
```

Score the finished task through the same HTTP adapter:

```sh
tools/mcp-eval/check plate-callouts-right-pinned --part-ref PART --spec-version FINAL_VERSION
```

The online checker verifies the handle names the task's STEP file, fetches the final
Spec/Plan and the default baseline, then evaluates
all unchanged conditions. Judge calls are excluded from the model log, as in S0.
Exit codes: checker 0 success, 1 failed conditions, 2 invalid invocation/transport;
call client 0 success, 2 tool refusal, 1 invocation/transport error. An offline
check requires all three documents:

```sh
tools/mcp-eval/check TASK --spec /tmp/spec.json --plan /tmp/plan.json --baseline /tmp/default-plan.json
```

`--url` or `MCP_EVAL_URL` selects an endpoint (default
`http://127.0.0.1:8765/mcp`). Each client call appends one JSONL record, including
refusals and transport errors, tagged with `--task` or `MCP_EVAL_TASK`. `wire`
counts exact UTF-8 JSON-RPC body bytes sent/received (excluding HTTP headers);
`content` counts tool arguments and text result, matching S0's accounting.
`tokens` estimates each content count as `ceil(bytes/4)`, not tokenizer usage.
`tools/list` records fixed context separately (`method` differs from `tools/call`).
Keep actual provider token usage and model/version with the real-model report.

Run the independent scripted evaluation and instrument tests:

```sh
node tools/mcp-eval/run-scripted.mjs --out target/d1-mcp-eval.json
node --test tools/mcp-eval/test-client.mjs
python3 -I tools/check_engine_cache.py
python3 -I tools/test_quality.py
```

Random-pin go/no-go measurement (all 30 parts, seed 1, N=3):

```sh
C=/Users/paul/repos/quiddity/tests/corpus
sed "s#^#$C/#" tools/quality_parts.txt | tr '\n' '\0' | xargs -0 python3 -I tools/quality.py --seed 1 --pins 3 target/release/draftwright target/pins-1-3
```

Run the complete grid using a baseline from the same binary:

```sh
for seed in 1 2 3; do
  for n in 1 3 8; do
    sed "s#^#$C/#" tools/quality_parts.txt | tr '\n' '\0' | xargs -0 python3 -I tools/quality.py --seed "$seed" --pins "$n" --baseline target/quality/baseline.json target/release/draftwright "target/pins-$seed-$n"
  done
done
```

Generate `target/quality/baseline.json` with the ordinary quality invocation first.
`random-pins.json` reports selected IDs, before
and after ink/off-sheet counts, and the bar per part and overall. The generated
`pinned/*.spec.json` and SVGs preserve the experiment. To reuse a baseline from
the same binary and exact part set, pass `--baseline PATH/baseline.json`.
Measurements exit 0 even for a measured no-go; add `--require-go` to enforce the
bar (nonzero for any failing part). Missing parts, failed commands and insufficient
pin candidates always fail. For zero unpinned ink, the 1.5× bar requires zero
pinned ink. The aggregate passes only if every part passes.
