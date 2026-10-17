# Authored dimensions: issue #76 item 1

The Spec contract and reference fingerprint contract are documented in
[model.md](model.md#dimensions-authored-from-geometry-76-item-1). The implementation
adds quantities before ordinary view routing, tolerance formatting, lane reservation
and placement. It uses the existing transactional session operations and browser
bindings. No tool or WASM method signature was added.

The ownership choice is **refuse duplicate active owners**, with the existing
dimension ID and a fix in the refusal. In `dimension_source: "authored"`, unlisted
automatic candidates are withheld. A geometry-authored replacement receives only
the numeric completeness roles it actually states; a diameter does not also prove
THRU, depth or quantity.

Linear references are planar supports, straight edges, vertices or cylinder axes.
Principal distances require a unique station in that direction; true distances
are shortest support separations. True distances and angles need a principal view
showing the quantity true. Face/edge angles use the physical minor included sector
(including obtuse angles). Diameter/radius references accept analytic cylindrical
and spherical faces. Unsupported, stale, missing, ambiguous, zero-distance and
duplicate references are refused rather than guessed. Saved v1 fingerprint keys
have a checked-in round-trip fixture and topology-order/reversal regression coverage.

## Reproduce the visual review

```sh
source "$HOME/.cargo/env"
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
cargo run --release --example authored_dimensions -- target/authored-dimensions
```

All six PNGs were inspected. Each has matching SVG, saved Spec, Plan and combined
geometry/structural lint JSON alongside it:

| Example | Before | After | Review |
| --- | --- | --- | --- |
| Plate diagonal | `target/authored-dimensions/diagonal-before.png` | `target/authored-dimensions/diagonal-after.png` | True 60.2 ±0.1 above the front view, parallel to its measured span with short perpendicular witnesses. No crossing with other dimension strokes or arrowheads. One expected nominal-rounding info; A4 at the original 2:1. |
| Face angle | `target/authored-dimensions/angle-before.png` | `target/authored-dimensions/angle-after.png` | 90° ±0.5 on the two extended supports at the front view's corner, with a 15.6 mm page radius and its arc outside the outline. No geometry/structural findings. A4 at the original 2:1. |
| Unrecognized cylindrical face | `target/authored-dimensions/cylinder-before.png` | `target/authored-dimensions/cylinder-after.png` | ⌀6.6 ±0.05 with a rim leader, from B-rep without a recognized feature record. No geometry/structural findings. A3 at 2:1. |

The examples do not require A2+, extra details or changed datum/fit policies.
The angle example uses automatic sheet selection and keeps the original scale. Its earlier
manual scale pin selected A3 even without an angle, so that unnecessary pin and the manual
width-label adjustment have been removed. The diagonal retains its unpinned below/lane-1
preference: the ordinary placement code repairs its obstructed side.

The placement search measures arrow flares as well as line strokes, fits the angle's actual
text gap and arrows, and can use the equal included sector on both extended supports. It
settles authored oblique quantities after merged dimensions receive their ink. Unpinned
linear dimensions can exchange lanes, and spare projection-row space is tried against the
finished drawing before changing paper or scale. Measured sheet fitting treats a corner arc
as its own obstacle. The existing drawing-bounds and overlap checks remain the acceptance
criteria. Structural lint accepts reversal of both physical supports and still rejects a
single reversed support, including supplementary obtuse sectors.

## Validation

Follow-up results after rebasing onto `origin/main` (`861a94e7`). Generated reports and logs
are in `target/authored-validation/placement-*`; render artifacts remain untracked.
No existing test, threshold, schema limit or size ceiling was relaxed.

- `cargo fmt --check` and `cargo clippy --workspace --all-targets --release -- -D warnings` pass.
- The affected release run covers the library and 18 integration suites, including authored
  dimensions, PMI dimensions, placement, layout, lint fidelity/structure, leader rules,
  session/editor operations and MCP/stateless APIs. Its final result is recorded in
  `placement-affected-final-complete.log`: **377 passed**, with one contended session-latency
  failure (**407.064 ms** p90 against **150 ms**). The same test passed in isolation at
  **43.097 ms** p90 (`placement-session-latency-isolated.log`), validating all **378** tests.
- New regressions compare the plate's sheet and scale before/after the authored angle, limit
  its ink to a local corner, require its arc outside the outline, and independently check the
  authored aligned span against every dimension stroke and arrowhead edge. Both automatic
  placement and the obstructed unpinned below/lane-1 hint are exercised. The new structural
  lint test accepts 60°/120° on both extended supports; existing single-reversal failures stay.
- MCP schemas remain **42,452 bytes / 42,500**. The budget test passes; no MCP schema or budget
  file changed in the placement follow-up.
- All **300** quality cells on **30** corpus parts match the CI baseline exactly.
- Leader routes pass for **119** callouts: no new hidden/section/cut crossings or long runs;
  the same **14** pre-existing runs remain.
- The CI-style `wasm-pack build crates/draftwright-wasm --target web --profile wasm-release
  --locked` passes, including wasm-opt, using the existing tool cache under `/private/tmp`.
- `python3 tools/check_wasm_api.py --base-ref origin/main` passes: the generated API matches
  the reviewed snapshot. No snapshot update is required. The additive JSON contract is
  already documented under Unreleased / WASM API.
- All **four** native/WASM parity tests pass, including geometry-authored references,
  editing and ink replay. WASM size passes the unchanged **15,700,000-byte** ceiling:
  **13,727,945** raw, **4,534,944** gzip, **3,027,698** Brotli bytes.
- All six regenerated PNGs were inspected. Angle and diagonal pairs remain **A4 at 2:1**;
  cylinder remains **A3 at 2:1**. The angle and cylinder renders have no geometry/structural
  findings. The diagonal has only its expected nominal-rounding info. Final PNG hashes
  match the inspected images (`placement-png-review.json`).
