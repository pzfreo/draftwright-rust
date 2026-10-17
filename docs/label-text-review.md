# Label text steering review — issue #76, item 5

2026-10-16–17. Implementation commits: `1e1397a3`, `0e8ed9f`, `0ff067e` and `525b72b`.

## Authoring choices

```json
{
  "dimensions": {
    "d:envelope/width.length": {
      "text": {"prefix": "2×", "suffix": "TYP"},
      "style": "reference"
    },
    "d:f:hole:3/bore.diameter": {
      "text": {"template": "{value} AFTER PLATING"},
      "style": "basic"
    }
  },
  "annotations": {
    "<current-prose-callout-id>": {"text": {"template": "{value} REF"}}
  }
}
```

The deliberate conservative choice is that `{value}` retains the **complete generated
label**, including other measured terms, counts, fits, tolerances and precision. It is not
an authored copy of today's nominal. A saved template re-expands when the measurement
changes. Templates require exactly one placeholder and exclude prefix/suffix. Literal
wording cannot supply another number or measurement/tolerance symbol; one positive leading
`N×` quantity is allowed, stays visible to quantity lint, and cannot double an existing count.
Each literal is a single line of at most 240 UTF-8 bytes. Unknown fields/placeholders,
control characters, missing values and stale IDs are refused with corrective guidance.
Merged dimensions name their printed owner rather than silently accepting an unused edit.

The two explicit styles are mutually exclusive. Removing the style restores the automatic
choice. Reference parentheses enclose all wording; basic boxes enclose the full ink.
Source PMI basic dimensions cannot be made reference through wording. Datums, control
frames and finish symbols retain their structured edits. Existing `set`/`remove` operations,
preview, undo/redo and replay carry the new fields; no new operation kind is needed.

Ownership and consolidation run before wording. Wording is resolved before reservation
and placement; explicitly edited labels survive later schedule/radius-note substitution.
Dimensions, prose annotations and automatic notes report their text/style provenance.
Measured values and claims remain geometry-owned. Lint strips only the compiler-bound
wording wrapper from live text, then judges the actual printed measured body. A damaged
wrapper or numeric body cannot borrow its previous claims. Unmentioned decisions keep the
automatic policy, including paper, scales, views, datums and class-only fits.

## Render review

Reproduce from the repository root:

```sh
source "$HOME/.cargo/env"
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
cargo run --release --example label_text -- target/label-text-renders
```

The example writes full 150-DPI PNGs, SVGs, Specs, Plans, lint reports and 250-DPI label crops.
Paths below are relative to the repository root.

| Part | Before PNG | After PNG | Reviewed change |
| --- | --- | --- | --- |
| Plate | `target/label-text-renders/plate-before.png` | `target/label-text-renders/plate-after.png` | `60` → `(2× 60 TYP)`; A4 at 2:1 retained. |
| Turned | `target/label-text-renders/turned-before.png` | `target/label-text-renders/turned-after.png` | `ø16` → `ø16 AFTER PLATING`; recovery selects A4 at 1:1 instead of A3 at 2:1. |
| Prismatic | `target/label-text-renders/prismatic-before.png` | `target/label-text-renders/prismatic-after.png` | `40` → boxed `40 (NOM)`; A4 at 2:1 retained. |

All six sheets and their label crops were visually inspected. The long diameter callout has
room for its entire wording and an attached clear leader; the reference enclosure includes
the prefix and suffix, and the basic frame includes the suffix with padding. Witnesses and
values remain readable. All six render lint reports are empty. No larger-paper, second-sheet,
detail-view, datum-placement or fit-display rule was changed.
The six PNGs were regenerated from the final source and visually reviewed again after all
required gates; their six lint reports remain empty.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --release -- -D warnings`: passed.
- The existing lint-independence guard passes unchanged. Read-side `LabelWording` records
  live in `output`; lint reaches neither the Plan nor the Spec.
- Independent `jsonschema==4.25.1` verification: all 13 tool inputs, outputs and refusals
  validated; all 26 served schemas match the generated files.
- MCP schemas: **42,342 bytes** in the existing UTF-8 session measurement, below the unchanged
  **42,500-byte** cap (main: 42,464). Only the canonical saved-Spec output needs the detailed
  text shape; other tools retain the existing generic per-item input convention. Equivalent
  minimum-property counts compact the two closed edit-result schemas without relaxing them.
- Full corpus-required release suite: **1,130 passed, one failed**, across 113 test groups.
  All seventeen label-text integration tests and all eight precision tests passed. Coverage
  includes preserved defaults, invalid references and wording, saved-Spec migration and
  round-trip, geometry changes, compound measured terms, session edits and independent lint
  of damaged printed values.
- The sole suite failure was the unchanged wall-clock gate
  `multi_sheet_latency::r5_multi_sheet_latency`: MCP p90 **553.228 ms** exceeded **150 ms**.
  Native p90 was 64.739 ms; MCP p50 was 130.356 ms. The full test command exited 101.
  No threshold or assertion was changed. See `release-tests.log` and `release-summary.json`.
- Thirty-part quality and `ci_acceptance.py --check-results quality`: passed; all **300
  cells exactly match** `tools/ci-quality-baseline.json`. No baseline was updated.
- `check_leader_routes.py`: passed on **119 callouts / 30 parts**. No extra hidden,
  section or cut crossings, no new runs over 2 mm, and lengths within baseline + 2 mm;
  fourteen unchanged pre-existing runs remain.
- `check_plan_time.py --out /tmp/steer-label-text-plan-time.json`: passed on all **65
  parts**; **2,766,353,642,191 instructions**, 1.004912× the recorded budget figure,
  below the unchanged **3,165,755,533,285-instruction** limit. Wall time was 348.5 seconds.
  The report is also copied to `target/label-text-checks/plan-time.json`.
- WASM `wasm-release` web build with `--locked`: passed, including the normal `wasm-opt`
  optimization. The first attempt compiled successfully but was denied a write to the
  shared wasm-pack tool cache. Copying its existing tool versions to
  `/tmp/steer-label-text-wasm-pack-cache` and setting `WASM_PACK_CACHE` resolved this without
  changing repository configuration or skipping optimization. Both build logs are retained.
- Native WASM reference build: passed. `node --test tools/wasm-parity.test.mjs`: **3/3
  passed**, with **47 byte comparisons across 17 states** and four preview-document
  comparisons. The replay includes text, reference/basic and removal of both overrides.
- `python3 tools/check_wasm_api.py`: passed for wording/style, measured values, saved Spec,
  atomic refusal and history. The Unreleased **WASM API** line documents the additive JSON
  fields; JavaScript method signatures are unchanged.

Large `item_options` replies preserve the complete descriptor map in `structuredContent`.
Their text channel uses the existing lossless `detail_ref` resource convention when duplicating
the JSON would exceed the unchanged 4,096-byte response cap. This keeps every native/session
edit path available. The wire-budget integration test checks the cap and identical resource
retrieval. Additional regressions cover quarantined saved text on reused IDs and style-only
counted references, bringing the label-text suite to seventeen integration tests. All nine
original MCP efficiency tests pass; forty item-option responses peak at 3,926 bytes under
the unchanged 4,096-byte cap.

Logs and machine-readable reports are in `target/label-text-checks/`; the thirty quality
drawings and their plans are in `target/label-text-quality/`. The independent MCP schema
validator was rerun against the final native binary and passed all thirteen tools.

The release command used the required corpus and four build/test workers:

```sh
source "$HOME/.cargo/env"
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus \
  DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release --no-fail-fast
```

`--no-fail-fast` runs every test target even if a latency gate fails. The source stayed
unchanged through the final verification sequence. Render and check artifacts are ignored
build outputs; the review and reproducible example are committed.

## Main integration — 2026-10-17

Merge `ae544a80` incorporates `origin/main` at `923bab69`, including #77's Spec
line-work switches. The generator retains main's anchored name alternatives and both
features' fields; the branch's separate `minProperties` compaction is removed. All MCP
schemas were regenerated, without merging generated JSON by hand. Explicit required
fields and additional-property checks retain the label-text strictness regressions.
The session schema total is **42,490 bytes**, below main's unchanged **42,500-byte** cap.

Both sides' Unreleased notes are retained, with API entries under `### WASM API`.
Main's `tools/check_wasm_api.py` snapshot gate is unchanged; the earlier browser behavior
check is preserved as `tools/check_wasm_label_text.py`. The prismatic parity fixture
replays line-work edits, preview and history as well as label wording and both styles.

Integration logs and command results are retained in `target/merge-main-checks/`.

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --release -- -D warnings`: passed.
- `wasm-pack build crates/draftwright-wasm --target web --profile wasm-release --locked`:
  passed with wasm-opt and the existing writable `/tmp` tool cache.
- `python3 tools/check_wasm_api.py --base-ref origin/main`: passed; declarations match
  the reviewed snapshot. The sixteen WASM safeguard tests also passed.
- `cargo build --locked --release --bins --example wasm_reference`: passed.
- Independent MCP validation: all thirteen tools' inputs, outputs and refusals passed;
  all twenty-six published and served schemas agree. The exact requested uv command
  passed with `UV_CACHE_DIR=/tmp/cx-steer-text-uv-cache` after its default cache was denied
  by the sandbox. No dependency or repository configuration was changed.
- Label-text tests: **17/17 passed**. Line-work tests: **6/6 passed**. The native engine
  schema-budget test passed and reported **42,490 bytes**.
- `python3 tools/check_wasm_label_text.py`: passed. Node/native WASM parity: **3/3 passed**.
- WASM size gate: passed with **13,674,074 raw bytes**, **4,512,643 gzip bytes** and
  **3,011,100 Brotli bytes**; main's **15,700,000-byte** raw ceiling is unchanged.

The unchanged `multi_sheet_latency::r5_multi_sheet_latency` wall-clock gate failed:
MCP p90 was **224.994 ms** against **150 ms** (p50 **125.138 ms**, maximum **236.370 ms**).
Native p90 was **72.891 ms**. No latency threshold, assertion or baseline was changed.
The unchanged `session::nist_benchmark_deltas` wall-clock gate also failed: native p90
was **601.115 ms** against **150 ms** for `nist/nist_ctc_01_asme1_rd.stp`.

The full corpus-required workspace release suite completed: **1,137 passed,
2 failed, 0 ignored**, across 116 test groups, with exit status **101**.
The two failures above were the only failures. Every target ran with `--no-fail-fast`:

```sh
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 \
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus \
DRAFTWRIGHT_CORPUS_REQUIRED=1 \
  cargo test --workspace --release --no-fail-fast
```

No source changed during these checks. The follow-up corrects the documented browser
check command and records these results; generated binaries and logs remain ignored.

## Main integration (#78–#80) — 2026-10-17

Merged `origin/main` at `00ebbb93`, after reviewing the authored-dimension,
views/layout and recognition-correction commits. `DimensionSpec` retains `kind`,
`between` and `along` alongside `text` and `style`. The saved-Spec schema describes
all five fields together. Both editor operation sets and geometry reference anchors
are retained. Complete-view canonical Specs clear text/style with tolerance/fit while
retaining authored geometry, so wording edits use the settled view frames.

The new authored-dimension regression covers reference/basic styles and prefix/suffix
wording around a toleranced, geometry-measured distance. It checks saved-Spec round-trip,
all five fields' schema validation, measured claims and provenance, rendered ink and
independent lint after damage to the live numeric body.

Schemas were regenerated exclusively with `tools/generate_mcp_schemas.py`. Main's
`hoist_objects`, iterative compaction, strict field-name anchors and root-local
references are retained. The new finite-name suffix factoring shares expressions such
as `(thread|knurl)_depiction`; an independent equivalence test includes invalid types,
unlisted combinations and trailing newlines. The native finite-name matcher also handles
trailing literal suffixes, with native strictness regressions for both grouped forms.
The annotation entry's object type is
already required by the common map constraint, as for main's dimension entries.

The exact compact UTF-8 session schema accounting is:

| Measurement | Bytes |
| --- | ---: |
| Main at `00ebbb93` | 42,405 |
| Main vocabulary with the new equivalent name compaction | 42,207 |
| Dimension and annotation text, including the shared closed prefix/suffix/template shape | +219 |
| Dimension reference/basic style enum | +39 |
| Final session schemas | **42,465** |
| Unchanged budget (strictly less than) | **42,500** |

Text fields preserve generated measured terms while exposing editable wording on both
dimensions and prose annotations. Style fields retain the explicit reference/basic
choice. Neither shape is discarded or weakened to save bytes. Label fields add **258
bytes**, equivalent compaction saves **198 bytes**, and the net increase from main is
**60 bytes**. Only `get_spec` describes the detailed saved fields; inputs retain the
existing generic per-item convention. No validation or latency threshold was changed.

Verification logs and machine-readable results are in
`target/integration-78-80-checks/`; the thirty-part quality comparison is in
`target/integration-78-80-quality/`.

Final verification:

- `cargo fmt --all --check` and workspace/all-targets release Clippy with `-D warnings`:
  passed, including a fresh run after the native finite-name matcher update.
- Locked release build of bins and `wasm_reference`: passed. The exact requested
  `uv run --with jsonschema python tools/check_mcp_schemas.py` command passed using
  the writable `/tmp/cx-steer-text-uv-cache`: all thirteen tool inputs, outputs and
  refusals validate, and all twenty-six served schemas match their generated files.
- All four Python schema-compaction equivalence tests and all sixteen WASM safeguard
  tests passed.
- Targeted release tests: **15 authored dimensions, 21 recognition corrections,
  12 views/layout rules, 10 MCP efficiency and 3 engine-version/schema-budget tests
  passed**. All **17 label-text tests passed**; the three label/MCP strictness and
  wire-budget tests also passed again after the native matcher fix.
- Thirty-part quality generation and `ci_acceptance.py --check-results quality`:
  passed. Every one of the **300 cells equals main's baseline**; no baseline was
  changed beyond the incoming main merge.

The first schema-compaction run exposed the native matcher's former prefix-only
assumption. It rejected valid suffix-grouped names such as `part_ref`, while the
independent JSON Schema validator accepted them. The matcher now strips both exact
literal ends before matching the finite alternatives; the strict native regressions
and independent all-tool validation pass on the corrected source. Initial logs are
retained alongside the successful `*-final.log` rechecks.

- Final locked `wasm-pack build crates/draftwright-wasm --target web --profile
  wasm-release --locked`, including wasm-opt: passed with the existing writable
  `/tmp/steer-label-text-wasm-pack-cache`.
- `check_wasm_api.py --base-ref origin/main`: passed against the reviewed snapshot.
  All **four Node/native parity tests passed**, including geometry-authored dimensions;
  the label text/style browser API smoke test passed too.
- WASM size gate: **14,097,243 raw**, **4,670,078 gzip**, **3,116,505 Brotli bytes**,
  under the unchanged **15,700,000-byte** raw ceiling.

The unchanged `multi_sheet_latency::r5_multi_sheet_latency` gate failed in the full
suite: MCP p90 **511.794 ms > 150 ms** (p50 **124.993 ms**, maximum **551.614 ms**).
Native p90 was **64.937 ms**. No latency threshold, assertion or environment
override was changed.

The complete required-corpus workspace release suite finished with **1,192 passed,
1 failed, 0 ignored**, across **119 test/doc-test harnesses**, exit status **101**.
The only failure was the multi-sheet MCP wall-clock gate above. All session latency
gates, the HTTP/stateless tests, all authored/recognition/layout/label regressions,
and both documentation-test harnesses passed. Every target ran; no gate was disabled.

```sh
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus \
DRAFTWRIGHT_CORPUS_REQUIRED=1 \
  cargo test --workspace --release --no-fail-fast
```

Source and generated schema contents remained unchanged through the final full-suite
run. The detailed full-suite summary is in `full-suite-summary.json`, command statuses
in `results.json` and `rechecks.json`, and exact schema attribution in
`schema-accounting.json`, under `target/integration-78-80-checks/`.
Nothing was pushed.
