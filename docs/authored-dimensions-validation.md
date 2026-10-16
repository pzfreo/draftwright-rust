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
| Plate diagonal | `target/authored-dimensions/diagonal-before.png` | `target/authored-dimensions/diagonal-after.png` | True 60.2 ±0.1 diagonal in front view, placed below in lane 1 to separate it from the overall width; one expected nominal-rounding info, no error. A4 at 2:1. |
| Face angle | `target/authored-dimensions/angle-before.png` | `target/authored-dimensions/angle-after.png` | 90° ±0.5 arc below the front view; width label moved along its ordinary lane to clear the witnesses. No geometry/structural findings. A3 at 2:1. |
| Unrecognized cylindrical face | `target/authored-dimensions/cylinder-before.png` | `target/authored-dimensions/cylinder-after.png` | ⌀6.6 ±0.05 with a rim leader, from B-rep without a recognized feature record. No geometry/structural findings. A3 at 2:1. |

The examples do not require A2+, extra details or changed datum/fit policies.

## Validation

Results recorded on the supplied Apple Silicon workspace. Limits, baselines and
existing tests were not relaxed. Machine-readable reports and logs are in
`target/authored-validation/` (generated artifacts, not committed).

- Formatting and `cargo clippy --workspace --all-targets --release -- -D warnings` pass.
- The corpus-enabled full release run completed 113 binaries: 1,124 passed, two
  failed. Its original schema-budget failure was subsequently fixed and rechecked
  without changing the 42,500-byte limit. The other failure was the wall-clock
  multi-sheet MCP p90 (533.813 ms against 150 ms).
- The final affected-suite run passed 269 tests, including all 212 library and 12
  authored integration tests, editor/session operations, MCP schemas and stateless
  APIs. Two wall-clock gates failed under load: multi-sheet MCP p90 659.435 ms and
  session NIST p90 649.677 ms, each against 150 ms. These gates remain unchanged.
- Authored tests cover applied/default behavior, invalid and ambiguous references,
  precise ownership, saved Spec/Analysis round-trip, pinned precision, class-only
  fits, independent lint detecting tampered nominal ink, cylinders/spheres/axes,
  true vertex distances, face/edge angles including 120°, and atomic preview,
  apply, undo and redo.
- MCP: 26 compact standalone schemas total **42,452 bytes**, below the unchanged
  42,500-byte gate (main's supplied schemas: 42,464). Served tools/list: 13 tools,
  **49,172 bytes**. Three schema-compaction equivalence tests pass; independent
  JSON Schema validation agrees on all published/served schemas, inputs, outputs
  and refusals. Input maps retain their existing sparse form; runtime Spec and
  geometry validators enforce the new fields.
- The 30-part quality survey exactly matches all 300 baseline cells. Leader-route
  checking passes for 119 callouts: no new hidden/section/cut crossings or runs;
  14 unchanged pre-existing runs remain.
- Planning instruction budget passes on all 65 parts: 2,768,766,950,276 retired
  instructions against a 3,165,755,533,285 limit (1.006 times the recorded baseline;
  wall time 334.1 s, informational). No override resolver runs when the Spec has
  no authored entries.
- WASM release build (including wasm-opt) passes. The default cache location was
  outside the sandbox's writable roots; retrying with `WASM_PACK_CACHE` under
  `/private/tmp` and copies of the existing tool cache completed normally.
  `node --test tools/wasm-parity.test.mjs` passes all four tests, including existing
  prismatic/polygonal byte comparisons and geometry-authored apply/undo/redo parity.
  The optimized module is 13,689,027 bytes raw; gzip -9 is 4,518,350 bytes and
  Brotli quality 11 is 2,992,142 bytes.
- `python3 tools/check_wasm_api.py` was attempted and exited 2 because the requested
  file is absent from this checkout. WASM method signatures are unchanged; the
  additive JSON API is called out under Unreleased “WASM API”.
