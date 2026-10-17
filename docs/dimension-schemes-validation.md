# Dimensioning schemes: validation and drawing review

Issue #76 item 4. Reproduce the six PNG/SVG examples with:

```sh
source "$HOME/.cargo/env"
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
cargo run --release --example dimension_schemes -- target/dimension-schemes
```

The public Spec vocabulary and edit paths are documented in [model.md](model.md#dimensioning-schemes-76-item-4).

## Decisions

- `from` defaults to `min`, along the named part-frame axis. The empty map preserves the
  automatic decision, including the inspection-end baseline on turned parts.
- Ordinate means a signed running coordinate, with an origin ring and one endpoint arrow.
  The ring is recognised from its actual closed ink by geometric fidelity lint; the
  independent scheme check also verifies the signed nominal against the named references.
- Named networks are checked together. They may meet at a datum, but cannot close a chain.
  Automatic controls survive only while adding an independent interval.
- A pattern expands to every physical member, including every interval of counted pitches.
  Grouped turned lengths likewise expand every shoulder. Partial counted-group replacements
  are refused with a remedy instead of losing the unselected intervals.
  Coincident stations share one witness, chosen nearest the ordinary exterior strip.
- Existing dimension placement/tolerance/precision edits apply to generated owner IDs.
  Conflicting intent on a retired ID is refused. #78 dimensions and semantic PMI cannot
  close the same network. Authored-only mode still requires listing generated owners.
- Explicit schemes take precedence over coordinate schedules in their carrying view;
  full-part table rescue cannot reintroduce duplicate coordinate owners. Other views
  retain normal per-view scheduling.
- Ambiguous multi-solid networks are refused: select one part before authoring its scheme.
  Circular patterns retain PCD; individual member targets can request Cartesian locations.
  Feature IDs and edits on generated feature-based owners from a different bound revision
  are refused until replaced with fingerprint
  references or explicitly rebound. Fingerprint-only entries resolve after reload.
- Endpoint attachment provenance uses `scheme_witness.<name>.<hash>` roles to retain the
  existing feature witness routing. Those roles state no manufacturing size or location;
  coverage comes only from independently measured printed links and their connectivity.

## Focused tests

The 12 tests in `tests/dimension_schemes.rs` cover all three schemes; high-end and fingerprint datums;
unchanged defaults after removal; complete hole/slot pattern expansion; turned shoulders;
composition at shared datums, partial counted-pitch refusal, repeated turned shoulders;
unknown references, incompatible axes/views, overlapping schemes, duplicate targets across
views, stale generated edits and conflicting edits;
saved Spec/Analysis and embedded SVG round trips; precision, tolerance and placement;
authored-only selection; session preview/refusal/undo/redo; and wrong, missing or duplicated
printed evidence. The WASM replay fixture exercises chain, high-end baseline preview,
negative ordinates, refreshed fragment placement options, undo/redo and removal against
the same native replay. The unchanged lint independence guard also checks the new rule module.

## MCP budget

The 26 generated schemas total **42,340 bytes** in compact served JSON, compared with
42,124 on the starting main: **216 additional bytes**, **160 bytes remaining** under the
unchanged 42,500-byte ceiling. The new map shares the existing sparse per-ID object shape
and the generator's equivalent `patternProperties` grouping. Complete entries are strictly
validated by Rust, as other sparse Spec maps are. No budget or test threshold was raised. The existing 4,096-byte MCP response tests
also pass: global scheme discovery is offered once on sheet options, matching authored
dimensions, rather than repeated on every dimension answer. All 13 tools and 26 published/served
schemas pass the independent draft-2020-12 validator, including scheme round-trip and refusal cases.

## Render review

All paths below are relative to the repository root. Each PNG has matching `.svg`,
`.plan.json`, `.spec.json` and `.lint.json` evidence beside it.

| Example | Before PNG | After PNG |
| --- | --- | --- |
| Mounting plate chain | `target/dimension-schemes/holes-chain-before.png` | `target/dimension-schemes/holes-chain-after.png` |
| Turned high-end baseline | `target/dimension-schemes/turned-baseline-before.png` | `target/dimension-schemes/turned-baseline-after.png` |
| Pattern ordinate | `target/dimension-schemes/pattern-ordinate-before.png` | `target/dimension-schemes/pattern-ordinate-after.png` |

The turned example replaces the controlling chain/reference member with the 30 and 70
baseline from the high end. It retains A3 at 2:1. The ordinate example shows the three
member coordinates in place of the counted pitch, with the unselected pattern retaining its
automatic presentation, on A4 at 1:1. The chain connects adjacent stations, rather than
repeating distances from the end: 20, 30, 30 and 15, including both intervals of the counted
grid. Its extra strips cause the ordinary fill/layout policy to select A3 at 2:1 instead of
A4 at 1:1; no large paper or second sheet is needed. The mounting plate has the same one
`leader_centerline_overlap` warning on `d:f:hole:4/bore.diameter` before and after; it is
still reported. The turned and ordinate pairs have zero findings before and after.
All six PNGs were opened and visually reviewed after the final production build. Labels,
arrow directions, origin rings, physical member witnesses and surviving automatic owners
are legible and consistent with the intended measurements.

## Required checks

Raw local logs are `/tmp/scheme-*.log`; generated check evidence is under
`target/dimension-scheme-*`. Implementation commit: `e8fec5d`.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --release -- -D warnings`: pass.
- Corpus-required `cargo test --release --no-fail-fast`, with four build jobs and four test
  threads: **1,164 passed, 1 failed, 0 ignored**, across 116 targets. The sole final failure is
  `r5_multi_sheet_latency`: MCP p90 **269.196 ms** versus **150 ms**. Its native p90 is
  61.978 ms and both instruction gates pass (native median 1,148,733,714; MCP median
  1,804,789,093). This is a wall-clock failure; the gate is unchanged. In the previous
  complete run, NIST session edit p90 was 397.386 ms versus 150 ms; that target passes in
  the final run. No test, budget or known-defect entry was weakened.
- `tools/quality.py` on all 30 manifest parts: pass. The `ci_acceptance.py --check-results
  quality` comparison against `tools/ci-quality-baseline.json` passes with **all 300 cells
  identical**, including zero off-sheet items, repeated measurements/spans and view overlaps.
  Existing ink contacts (17), repeated labels (7) and line-through-feature contacts (73) are
  unchanged. Evidence: `target/dimension-scheme-quality/baseline.json`.
- `python3 -I tools/check_leader_routes.py target/dimension-scheme-quality`: pass, 119
  callouts on 30 parts; no new crossing or excessive run; 14 pre-existing long runs unchanged.
- `DRAFTWRIGHT_CORPUS=… python3 -I tools/check_plan_time.py --out
  target/dimension-scheme-plan-time.json`: **pass**, all 65 parts counted;
  **2,620,808,288,959 instructions**, below the unchanged **3,165,755,533,285** limit
  (0.952 of the recorded baseline, wall 191.9 seconds). No budget increase.
- `wasm-pack build crates/draftwright-wasm --target web --profile wasm-release --locked`:
  pass. The initial helper-cache installation hit the filesystem sandbox; retry used an
  unchanged copy of the existing helpers under `WASM_PACK_CACHE=/tmp/dimension-scheme-wasm-pack-cache`.
  No manifest, compiler profile or optimisation setting changed.
- `python3 tools/check_wasm_api.py`: pass; reviewed binding signatures are unchanged.
  The additive JSON vocabulary and fragment-option refresh have an Unreleased WASM API entry.
- `cargo build --locked --release --example wasm_reference` and
  `node --test tools/wasm-parity.test.mjs`: **5/5 pass**. Scheme replay has 7 states,
  19 byte comparisons and 2 preview document comparisons, plus explicit two-station,
  provenance, negative-ordinate/arrow and restored-automatic assertions. All existing parity
  assertions are retained.
- The new replay uses two analytic feature stations on the existing prismatic parity part.
  The initial scattered-hole replay exposed platform-dependent last-bit differences in
  pre-existing sampled principal-profile curves: only `analysis_sha256` differed in the
  default Plan, before any scheme edit. No geometry, plan-hash or default algorithm was
  changed to mask that difference, and no comparison was relaxed. Native scheme tests retain
  scattered holes, whole view axes, hole/slot patterns and turned profiles.
- `python3 tools/wasm_size.py crates/draftwright-wasm/pkg/draftwright_wasm_bg.wasm`:
  pass, raw **14,028,083 bytes** under the unchanged 15,700,000 ceiling (1,671,917 bytes
  headroom); gzip 4,644,803, Brotli 3,103,287.


