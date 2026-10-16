# Native section recess verification

Implementation source: `aab5f8a8113283c75e8e9f3028eef4468d2e52a7`. Quiddity source:
`1b171f4d678966945193047f871988295a2667c1`, document
`quiddity-rust/recognition/3`. The dependency pins remain `v0.1.0-alpha.4`;
verification used the uncommitted local Cargo path override. The final quiddity
HEAD is `2709a331a7c0e3d80734768834ff12e36caf92f5`; it only formats the K3 test
and contains the same recognition implementation. The merge pin in
[native-recesses.md](native-recesses.md) includes that formatting commit.

This branch is not merge-ready while its layout and quality regressions remain.
The five requested pending rules have native implementations and regression tests;
the four previously bridge-only rules also have native regressions. The twelve
native recess tests and six catalogue tests pass.

## Required checks

| Check | Result |
| --- | --- |
| fmt | PASS |
| clippy | PASS |
| tests | FAIL (exit 101) |
| quality measurement | Completed |
| quality baseline comparison | FAIL: 14/30 parts worse |
| routes-before | FAIL (exit 1) |
| routes-fixture | FAIL (exit 1) |

The full suite reported 1051 passed, 8 failed and 0 ignored across 107 test executables/doc-test groups.

Failures:

- `frames_datum_symbols_and_labels_are_on_the_sheet_and_off_the_other_views`
- `ctc_02_frames_and_datum_symbols_are_inside_their_bands_and_on_the_sheet`
- `stc_09_datum_symbols_are_inside_their_bands_and_frames_past_them_had_no_clear_strip`
- `stc09_without_a_complete_fit_keeps_its_first_drawing_and_reports_the_floor`
- `stc09_slot_length_stays_beside_its_peer_and_gdt_corridors`
- `r5_multi_sheet_latency`
- `a_location_that_fits_on_neither_side_shares_a_held_lane`
- `every_repeated_label_on_the_quality_parts_has_a_verdict`

The R5 MCP p90 was 275.248 ms in the full suite, above its unchanged 150 ms
limit. A standalone rerun passed at 124.432 ms (`/tmp/recess-final-latency-isolated.log`).
The full-suite failure remains recorded above.

No test was disabled and no acceptance threshold or latency limit was relaxed. The
quality runner can exit successfully while the baseline comparison fails; its
exit code above records successful measurement, not acceptance of the drawing.

## Quality comparison

Against the unchanged baseline (`699acae7`, released dependency), 14 of 30 parts worsen in monitored metrics.

| Metric | Before | After |
| --- | ---: | ---: |
| off_sheet | 0 | 0 |
| repeated_label | 7 | 10 |
| repeated_measure | 0 | 0 |
| views_overlap | 0 | 0 |
| view_on_title_block | 0 | 0 |
| ink | 52 | 76 |
| repeated_span | 0 | 0 |
| line_through_feature | 73 | 97 |
| hidden_on_visible | 2231 | 2139 |
| section_hidden | 0 | 0 |
| leader_on_extension | 53 | 71 |
| text_off_span | 5 | 9 |

No threshold, route fixture or CI baseline was relaxed. New unsupported recesses
and provider refusals remain explicit rather than being hidden to improve counts.

## Artifacts and reproduction

The artifacts are under `/tmp/section-recess-review/`. `README.md` links the
three before/after drawings and records the drafting review. `final-checks.json`
contains exact commands, exit codes, log paths and timings. `final-comparison.json`
contains every per-part regression; `final-leader-routes-before.json` and
`final-leader-routes-fixture.json` contain the route checks. Final images and the
30-part analysis/plan/SVG/review records are in `after-final/`.

```sh
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --workspace --release --no-fail-fast
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo clippy --workspace --all-targets --release -- -D warnings
cargo fmt --all -- --check
```

The sole oracle adjustment records the physically shared 48.4 mm recess/envelope
span on m1000. Its measurement claims and editable covered row remain independently
tested, including promotion when the envelope carrier is omitted. Hole and slot
location identities remain separate, preserving their table cells.

Non-Z patterns retain Python's missing-location-owner refusal. Rotated lattices
outside the part axes are explicitly unverifiable under the current compiler.

See [native-recesses.md](native-recesses.md) for the upstream release requirement,
the exact two dependency-pin changes and the remaining unread families. Merge
quiddity first, then update both pins and Cargo.lock and resolve the drawing gates
before merging draftwright. Neither repository has been pushed.
