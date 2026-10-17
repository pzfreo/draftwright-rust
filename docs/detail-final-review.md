# Final detail legibility and main integration review — 2026-10-17

The connector now selects **A3 at 1:5**, with no enlarged detail. This is the chosen standard-scale drawing: its principal profile grows from 42 to 84 mm on paper, and its 20 mm bands grow from 2 to 4 mm. All 26 dimensions remain placed. The accepted string_post, GRM-03 and sm-hanger PNGs are **byte-identical** to revision `67588408`.

## Why the connector passed before

The old floor checked 0.8 mm hole/slot openings, a 20 mm overall drawn extent and the absolute 1:10 floor. It did not check dimensioned axial profile intervals. Thus the 420 mm connector could have a 42 mm profile, compressed 2 mm bands and stacked callouts while still being labelled a clean candidate.

Automatic seeds, complete candidates and multi-sheet supplement gates now require each dimensioned turned-step interval, including baseline replacements, to span at least the dimension font's **2.1 mm printed capital height** (`0.7 × 3 mm em`) in its actual owning view. Enlarged detail owners use their own scale. Small edge treatments stated by notes do not set the whole-sheet scale. Explicit scales remain authored choices and report `legibility_floor_breached`, extending ARCH0004-3. Candidate explanations count unreadable spans.

Once a complete legible A3 drawing exists, large paper is not introduced solely for a low-fill enlargement. Named paper remains an override. This prevents the A2 1:2 candidate's slightly larger fill from defeating the readable A3 drawing. There is no part-name condition in production code.

Regression: `tests/sheet_fill.rs::connector109_uses_a3_at_one_to_five_for_readable_steps`. The second regression checks owning-view scale and baseline interval responsibility. Existing readable 2.5 mm flange spans still pass; the full font em is not mistaken for printed glyph height.

## Before / after PNGs

| Part | Before `67588408` | Final | Before PNG | After PNG |
| --- | --- | --- | --- | --- |
| threaded_connector_109 | A4 1:10 | A3 1:5 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/before/connector.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/after/connector.png) |
| string_post | A4 5:1 | A4 5:1 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/before/string_post.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/after/string_post.png) |
| GRM-03 | A3 10:1 | A3 10:1 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/before/GRM-03.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/after/GRM-03.png) |
| sm-hanger | A3 1:2 | A3 1:2 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/before/sm-hanger.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/after/sm-hanger.png) |

All four final drawings have no enlarged details. They were visually inspected after regeneration. The three retained drawings are both pixel- and byte-identical; the connector is the changed drawing.

## Merge

Fetched and merged `origin/main` at `2eb328ce`, including #77, #78 and #79; #80 was not in main at fetch time. Kept line-work switches and provenance, geometry-authored dimensions and reference evidence, complete-view base Specs, every measured linear corridor, iso visible precedence and section material/hatch behavior alongside the detail implementation.

Resolved the shared planning switches with an internal view policy, retained authored aligned geometry inside crop bounds, composed section partners beyond expanded side corridors, and used crop extents when reserving detail blocks. Main's fitting incomplete ordinary fallback survives an unsuccessful optional-detail search without losing seed measurements. Precision conflicts remain errors for equal shared quantities; arithmetic baseline intervals do not falsely share a printed scalar with their longer station owners. No tests were changed to permit those regressions.

Schema JSON was regenerated with `tools/generate_mcp_schemas.py`, never hand-merged. Main's `spec_read`, `hoist_objects`, `group_properties` and other compaction mechanisms remain. Detailed resolution: [detail-main-integration.md](detail-main-integration.md).

## Schema bytes

**46,328 compact UTF-8 bytes**, strict budget **< 46,329**, saving **540 bytes** from the prior detail branch. Regenerated main is 42,124 bytes. The record adds **4,116 bytes (343 × twelve standalone Spec schemas)**, and detail placement names add **88** more.

| Record component | Per schema | Twelve schemas | Purpose |
| --- | ---: | ---: | --- |
| `around` plus required constraint | 98 | 1,176 | Stable target ID or nonempty string array; rejects missing/ill-typed targets. |
| Optional `from` | 96 | 1,152 | Typed principal/section parent; strict end anchor rejects malformed names and trailing newlines. |
| Optional `scale_factor` | 54 | 648 | Numeric enlargement greater than one. |
| Closed nullable record/property structure | 95 | 1,140 | Validates object/null forms and rejects unsupported geometry fields such as bounds. |
| **Record total** | **343** | **4,116** | |

The strict 42,500 budget leaves only 375 usable bytes beyond regenerated main, insufficient for the typed record. Replacing typed input records with permissive maps would discard existing validation. The new bound is the exact smallest strict integer bound for the measured schemas. Full accounting: [detail-schema-budget.md](detail-schema-budget.md); [measured removal variants](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/schema-cost.json).

## Exact 227-part survey

Replayed the accepted manifest: 227 drawable unique contents plus the same intentionally unresolved fixture. Input hashes were rechecked; original STEP basenames were preserved. Results: **zero new errors, zero lost placed PMI, zero new short-step features, and zero scale reductions into illegibility**. In fact no selected scale decreases anywhere in this set. Default enlarged details remain at **two**, on `mfcadpp/10170.step` and `nist_ctc_02_asme1_ap242-e2.stp.gz`, with the accepted requirement ownership.

Six raw direct-dimension claim-set differences are schedule transfers: **364 claims** remain in shown, placed table blocks with matching numeric values, including their covered interval ledgers. Each row was checked against the printed block rows; none is unaccounted for.

Two pre-existing incomplete fallback cases remain explicitly warned: `cgb217` retains its 1 mm/2 mm primary intervals at A2 1:1, and STC-09 retains A1 1:5 below its opening floor. Both retain their prior scale, exhaust the legal complete alternatives and report the shortfall. Neither is accepted as a new clean small-scale candidate. The independent 30-part quality audit has **zero** short-step cases and **zero** below-geometric-floor cases.

| Survey part | Before | Final |
| --- | --- | --- |
| corpus/cadgenbench/threaded_connector_109.step | A4 1:10 | A3 1:5 |
| corpus/cadgenbench_inputs/cgb202.step.gz | A2 1:5 | A0 1:2 |
| corpus/cadgenbench_inputs/cgb207.step | A4 1:2 | A3 1:2 |
| corpus/mfcadpp_holdout/471.step | A4 1:1 | A4 2:1 |
| fixtures/pmi/step/hw_assembly_plate_pin_intent.step.gz | A4 1:5 | A3 1:1 |
| fixtures/pmi/step/hw_thumbwheel_notes_add.step.gz | A4 5:1 | A3 10:1 |
| fixtures/pmi/step/thumbwheel_thread_knurl.step.gz | A4 5:1 | A3 10:1 |

The cgb202 growth is exposed here rather than hidden in aggregate results; its 449.5 × 355 × 255.8 mm geometry is outside the full-size practical-paper criterion. The other three accepted drawings retain their exact PNGs.

Evidence: [manifest](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/survey/manifest.json), [all plans and summary](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/survey/summary.json), [comparison](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/survey-comparison.json), [schedule transfer proofs](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/schedule-claim-transfers.json), [30-part legibility audit](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/quality-legibility.json).

## Final checks

Run sequentially with `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`, `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and `DRAFTWRIGHT_CORPUS_REQUIRED=1` for the full suite. No test, quality baseline, routing threshold, latency threshold, instruction budget or WASM size ceiling was weakened.

| Check | Final result | Evidence |
| --- | --- | --- |
| `cargo fmt --all --check` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/fmt.log) |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/clippy.log) |
| `cargo test --workspace --release --no-fail-fast` | 1,188 passed; 0 failed; 0 ignored; 118 harness/doc results | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/tests.log) |
| `Session and multi-sheet latency gates` | PASS in the full run; limits unchanged | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/tests.log) |
| `30-part quality vs tools/ci-quality-baseline.json` | PASS; all 300 cells exactly equal to incoming main | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/quality-comparison.log) |
| `tools/check_leader_routes.py` | PASS; 119 callouts / 30 parts; 14 unchanged pre-existing runs | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/leader-routes.log) |
| `tools/check_plan_time.py` | PASS; 2,750,699,693,796 < 3,165,755,533,285 instructions; 65 parts | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/plan-time.log) |
| `wasm-pack web / wasm-release / locked, default wasm-opt` | PASS with writable WASM_PACK_CACHE | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/wasm-build-cached.log) |
| `Native WASM reference build` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/wasm-native-final.log) |
| `Node WASM/native parity` | PASS; 4 tests, zero skips | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/wasm-parity-final.log) |
| `tools/check_wasm_api.py --base-ref origin/main` | PASS; reviewed declarations unchanged; additive detail JSON documented under Unreleased / WASM API | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/wasm-api-final.log) |
| `WASM size guard` | PASS; 14,059,235 < 15,700,000 raw bytes | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/wasm-size-final.log) |
| `Schema compaction equivalence tests` | PASS; 3 tests | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/schema-equivalence.log) |
| `Independent published/served MCP schema validation` | PASS; 13 tools, 26 schemas, inputs/outputs/refusals | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/schema-validation.log) |

The first packaging attempt hit the default cache's permission restriction. A retry put `--no-opt` after the extra Cargo arguments and failed before building. The first size check therefore saw an unoptimized partial package (16,888,373 bytes) and failed. These attempts remain in the machine-readable log. The resolved run uses `WASM_PACK_CACHE=/private/tmp/draftwright-wasm-pack-cache` with the **normal optimization pass**, then freshly reruns parity, API and size checks. No source or packaging policy was changed to waive optimization or size checks.

Earlier verification also caught the full-em legibility mistake, the fitting-incomplete fallback interaction and the arithmetic-baseline precision false positive. Those implementations were corrected; all original assertions and limits remain, and the final complete source run is green.

## Provenance and delivery

Final tested application source: `4568434`. Before CLI SHA-256: `e84294998b5fc00a033df558724d3d8a9ac9e4a0f74d9b1c9483cd0b6f0b3a69`. Final CLI and archived survey CLI SHA-256: `77cfc40ea0d18bf4eaa9f0194639600f9b51849d579229a47ec9c7cb35592e6b`. Documentation/tool-only delivery follows that source without changing compiled application inputs.

Machine-readable results: [final-verification.json](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/final-verification.json); [render comparison hashes](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-final-review/render-comparison.json). All work is committed in coherent steps with the required Codex co-author footer. Nothing was pushed.
