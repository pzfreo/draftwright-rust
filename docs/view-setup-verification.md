# View setup verification (#76, item 3)

Implementation: `6aa5247c`, `7e0d4eed`, `6344921d`, `2c92efd` on
`cx/steer-view-setup`. No paper-ranking rules or acceptance budgets changed.

## Interface choices

The complete contract and examples are in [model.md](model.md#front-frame-explicit-principals-auxiliary-and-partial-views-76-item-3).
Front setup belongs to `views/front`:

```json
{"views":{"front":{"orientation":{"from":"z","rotation":90},"principal_views":["front","plan","side"]}}}
```

`orientation.from` also accepts signed principal axes, stable planar face
fingerprints, or feature IDs. Rotation is counterclockwise in degrees. Auxiliary
and partial views reuse the existing eight lettered slots and section-letter
namespace, rather than adding another identity scheme:

```json
{"views":{
  "detail_b":{"auxiliary":{"face":"face:sha256:<fingerprint>","from":"side","scale_factor":1}},
  "detail_c":{"partial":{"from":"detail_b","around":"f:slot:1","scale_factor":2}}
}}
```

The explicit principal set contains the three existing principal names,
`front`, `plan`, and `side`. A partial can use a principal or an authored
auxiliary as its source. Auxiliary views retain full-part context; partials crop
to model-derived feature or face regions. The shared slots preserve caption,
scale, placement, line-work, section and sheet-planning conventions.

References must resolve uniquely. An auxiliary needs a planar face and an
available principal where that face is edge-on; a partial needs a nondegenerate
projected region and an available source. Conflicting forms, included-and-omitted
principals, excluded placement targets, nonpositive scales and invalid rotations
are refused with the offending path and a corrective edit. Saved feature
references participate in the existing face-anchor rebinding transaction.

With setup absent the automatic cameras and principal selection remain active.
An explicit principal set overrides reduction, and authored geometry still runs
through the ordinary sheet allocator, dimensional ownership and independent lint.
Plan basis and `from_spec` record each authored field.

## Native and schema checks

Rust commands use `CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8`, after sourcing
`$HOME/.cargo/env`. Corpus checks use
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | Passed |
| Full `cargo test --release` | 1,274 passed, zero failed/ignored, 121 targets; `target/view-setup-tests-final.log` |
| Focused view setup/detail/schema/line-work tests | 51 passed; `target/view-setup-integration-tests.log` |
| `tools/generate_mcp_schemas.py` | 14 tools; 34,477 compact advertised schema bytes, unchanged budget |
| Schema outline unittest discovery | Three passed; `target/view-setup-mcp-outline-tests.log` |
| Full Spec / private validation schema | 30,910 / 94,535 bytes; new fields exposed by `tools/mcp_spec.py` |
| 30-part `tools/quality.py` and `ci_acceptance.py --check-results quality` | Passed; all 300 baseline metric cells identical; `target/view-setup-quality/baseline.json` |
| `python3 -I tools/check_leader_routes.py target/view-setup-quality` | Passed: 119 callouts on 30 parts, no new route/crossing regressions |
| `python3 -I tools/check_plan_time.py --out target/view-setup-plan-time.json` | Passed: 2,755,642,763,843 instructions, limit 3,165,755,533,285; 232.3 seconds wall time |

The 17 new `tests/view_setup.rs` tests exercise every front axis, rotated
measurement carriers, face/feature anchors, explicit view restoration and
exclusion, auxiliary true shape and source-edge angle, partial crops and scales,
invalid-reference remedies, saved Spec replay/rebinding, session preview/history,
lint on damaged output and missing coverage, sheet allocation, remote markers,
rotated section markers, explicit dimension ownership and pin-frame sheet policy.
The pin regression failed before its fix because sheet 2 was incorrectly checked
against an automatic one-sheet allowance.

An earlier native run exceeded the unchanged MCP multi-sheet latency gate
(p90 173.250 ms against 150 ms; native p90 29.808 ms). Subsequent complete native
runs passed that gate, including the final-source run above. The threshold and
test were not changed.

## WASM and CI acceptance

The final WASM build, API review, native reference build and six parity tests pass.
Optimized raw WASM is 14,595,883 bytes (gzip 4,859,548; Brotli 3,247,260),
with 1,104,117 bytes remaining under the unchanged 15,700,000-byte ceiling.
The new view setup fixture exercises 15 session states, 43 byte comparisons and
two preview document comparisons.

The initial build was blocked by the sandbox trying to write to the default
wasm-pack cache. Its unoptimized intermediate binary exceeded the size ceiling;
this was not accepted as a size result. Rebuilding with
`WASM_PACK_CACHE=$PWD/target/wasm-pack-cache` and the copied existing cache ran
the normal optimizer successfully. Initial parity also exposed negative-zero
camera components; `2c92efd` canonicalizes authored vectors, and unchanged parity
assertions now pass. No optimizer, size or parity guard was bypassed.

The commands match `.github/workflows/ci.yml`: WASM safeguards, locked `wasm-pack` web build
with `wasm-release`, API comparison against `main`, locked native reference build,
Node parity and size checks. Acceptance passed all nine quality-pin cells
(seeds 1/2/3 by 1/3/8 pins), all four repeated-span shards, MCP, plan-time and
render-tail. Raw logs and exact arguments are retained under
`target/view-setup-acceptance/` and `target/view-setup-gate-results.json`.

| CI acceptance job | Result |
| --- | --- |
| Quality-pins seeds 1/2/3 × 1/3/8 pins | All nine passed, each with a fresh 30-part baseline, leader checks, lane locality and deliberate-regression rejection |
| Repeated-span shards 1/4, 2/4, 3/4, 4/4 | All passed; 17 + 16 + 16 + 16 = 65 parts, zero repeated spans |
| MCP | Passed: all seven tasks succeeded; Python and Node client checks passed |
| Plan-time | Passed: 2,755,868,605,516 instructions against 3,165,755,533,285; wall 235.1 seconds |
| Render-tail | Passed: cgb243 2,830,303,884,842 instructions; threaded_connector_109 4,098,645,207,232 |

The first attempted `quality-pins 1-1` invocation refused a directory left by
the interrupted pre-parity run, before executing a check. The wrapper therefore
returned nonzero. Its exact job was rerun successfully with
`--out target/view-setup-acceptance-fresh-1-1` (225.69 seconds). The initial refusal
log is preserved; no result or baseline was edited to make it pass.
`target/view-setup-acceptance-final.json` verifies all 16 successful summaries
against the SHA-256 of the same final native binary. The other 15 matrix jobs
passed on their first completed run.

## Drafting review

Reproduce with `cargo run --release --example steer_views -- target/view-setup-review`.
The example writes the before/after PNG, SVG, saved Spec and review JSON for each
case. All six PNGs were regenerated on the final source and visually inspected:

- Front frame: the broad plate face becomes the front and rotates 90°; the true-size
  dimensions and projected principal alignment follow it. The existing automatic
  allocator selects A3 at 2:1 for this authored frame, versus A4 at 1:1 before; its
  ranking was not changed in this stream.
- Auxiliary: the inclined rectangular face is shown in true shape, with its edge
  angle preserved, parent viewing arrow B and a clear `AUXILIARY B (1:1)` caption.
  Both before and after fit A4 at 1:1.
- Partial: the representative hole region is enlarged to 2:1 with a legible
  `PARTIAL B` caption, parent region marker and single-owner pattern callout. The
  principal geometry and smaller isometric remain readable on A4 at 1:1.

Independent structural lint reports no findings for all six drawings. The
fixture plans still report their existing unrecognised faces and missing general
tolerance; auxiliary before/after both retain the same annotation-area finding.
No authored view suppresses those findings.

| Case | Before PNG | After PNG |
| --- | --- | --- |
| Front direction and rotation | `target/view-setup-review/front-frame-before.png` | `target/view-setup-review/front-frame-after.png` |
| Inclined-face auxiliary | `target/view-setup-review/auxiliary-before.png` | `target/view-setup-review/auxiliary-after.png` |
| Feature-region partial | `target/view-setup-review/partial-before.png` | `target/view-setup-review/partial-after.png` |
