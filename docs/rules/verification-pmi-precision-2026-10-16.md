# PMI source and precision batch verification — 2026-10-16

The six maintainer-approved PORT entries are implemented. Python source and the cited Python
regressions were compared with the Rust Analysis/Spec/Plan and final drawing paths. No rule
was translated, retired, or left pending. `.rules-batch.json` was removed and is not committed.

| Rule | Outcome | Principal regression |
| --- | --- | --- |
| LINT-code-pmi_source_site_mismatch | ported | `tests/surface_texture.rs::a_finish_leader_must_stay_on_its_source_face` |
| ISSUE-1209-a | ported | `tests/pmi_dimensions.rs::oblique_pmi_draws_its_exact_projected_station_span` |
| ISSUE-1215-a | ported | `tests/issue_drafting.rs::envelope_wide_bands_and_extent_fits_reach_the_ink` |
| ISSUE-1349-a | ported | `tests/precision.rs::requested_precision_reaches_extents_chains_ladders_and_locations` |
| ISSUE-1349-b | ported | `tests/precision.rs::invalid_and_conflicting_precision_fail_closed_at_both_inputs` |
| ISSUE-1351-a | ported | `tests/spec_notes.rs::structured_feature_notes_cover_only_explicit_roles_of_their_placed_owner` |

Additional regressions pin compound terms, independent tolerance digits, close diameters,
zero-place fidelity, conflicting coincident policies, symbolic thread/reference text, and
hole-schedule coordinates and counterbore depths. Schedule precision was checked failing
before the fix (`68` instead of `68.000`) and passing after it. Exact aligned PMI is checked
in front, plan and side projections with both default and three-place labels, including
arrow span, shaft collinearity, centered text, border clearance and the reserved area.

| Catalogue | Pending | Ported | Translated | Retired | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| adr.yaml | 46 | 56 | 33 | 25 | 160 |
| lint.yaml | 42 | 110 | 0 | 13 | 165 |
| issues.yaml | 96 | 67 | 6 | 21 | 190 |
| All | 184 | 233 | 39 | 59 | 515 |

## Faithful choices and scope

- Exact proven stations determine the aligned span. As in Python, only a preserving principal
  projection is accepted; incomplete, wrong-length and nonprincipal relationships remain
  refusals. Text remains centered between the stations with either arrow arrangement, as in
  the Python venv's `build123d_drafting.helpers.Dimension`. Witnesses are clipped around text;
  ordinary axis-only label sliders cannot distort oblique geometry.
- Precision is decimal places 0–15, keyed by stable semantic measurement IDs. It changes
  nominal display and its reservation, retaining raw measurement values and tolerance digits.
  Conflicting text policies on a shared term are refused using the existing owners. Symbolic
  thread designations and manufacturing references retain their authored text.
- Rust's annotation identity resolves directly to a typed source record. There is no separate
  Python Part21/source_id pair to lose. Unknown source claims and census-only finish claims
  cannot acquire coverage merely by naming a source. Finish lint checks the final leader tip
  against the exact source face in the recorded view.
- Structured note authority uses explicit canonical roles on its owner
  (`features/<id>/note_satisfies`). It is separate from dimension claims and requires a
  source-resolved, placed leadered note with surviving text. It grants neither sibling
  coverage nor implicit THRU/count authority. Duplicate claims are set membership.
- Sheet selection, dimension ownership, datum/GD&T placement and detail views were retained.
  The opt-in single-sheet byte guard was not re-recorded.

## Visual evidence

All paths below are relative to the repository root, in the ignored local evidence directory
`target/rule-evidence`. PNG and SVG files, specs and JSON reports are retained. The before
binary was built from unchanged base `53aa9f3f`; it is `before-draftwright` in that directory.
The native precision/fit comparisons use base defaults before and saved authored specs after.
Forced schedule mode is `always` in both phases; only after adds precision. The base refuses
the newly introduced spec fields (`authored-before-refusal.json`). Default AP242 comparisons
use identical inputs.

| Comparison | Before paths | After paths | Review |
| --- | --- | --- | --- |
| Authored precision, bands and fits on three native parts | `before/{plate,turned,prismatic}.{png,svg}` | `after/{plate,turned,prismatic}.{png,svg}` | Two-place nominals and independent ±0.05 bands; width/height h7 fits where authored. Text remains readable and on the sheet. Sheets stay A4/A3/A4. |
| Three forced schedules | `schedule-before/{scattered_holes,counterbored_block,single_hole}.{png,svg}` | `schedule-after/{scattered_holes,counterbored_block,single_hole}.{png,svg}` | Three-place size/location cells and counterbore depth; columns expand around the longer text. No sheet or scale growth. |
| Exact aligned witnesses in three preserving projections | `aligned-before/{front,plan,side}.{png,svg}` | `aligned-after/{front,plan,side}.{png,svg}` and `aligned-after/plan-precision.{png,svg}` | Synthetic proven-station reading on STC-10, matching the regression; this does not infer extra requirements from the real file. Exact straight span and centered text remain inside the drawing border. |
| Default AP242 output on three parts | `pmi-before/{bracket_positions,string_post_tapped,nist_stc_10_asme1_ap242-e2}.{png,svg}` | `pmi-after/{bracket_positions,string_post_tapped,nist_stc_10_asme1_ap242-e2}.{png,svg}` | Pixel-identical to unchanged base on all three parts. Final authored native renders also match the previously reviewed images; see `final-pixel-comparison.json`. |

The initial aligned plan-view border failure and its fail-before regression are retained as
`aligned-border-before-fix.png` and `aligned-border-before-fix.log`. The final six-case
regression is recorded in `aligned-complete.log`. Schedule fail-before/pass-after evidence is
in `schedule-precision-before.log` and `schedule-precision-after.log` (21 existing schedule
and seven precision tests passed).

## Required checks

Final engine source commit: `a12205aed09624c92ba2d24be316b842e21ee51e`.
Final CLI SHA-256: `9f843ea65f77cd52f9863c37354a420d33572db5057cd773f312f1aa8d0fb3df`.

Commands were run serially, after sourcing `$HOME/.cargo/env`, with
`CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`. The corpus was required, and no latency gate was
disabled. `--no-fail-fast` only collects all harness results; it does not waive failures.

| Check | Result | Local evidence |
| --- | --- | --- |
| `cargo fmt --check` | PASS | `fmt-final-3.log` |
| `cargo clippy --all-targets --release -- -D warnings` | PASS | `clippy-final-3.log` |
| Corpus-required `cargo test --release --no-fail-fast` | FAIL: 1,012 passed, two latency failures, zero ignored, 103 harnesses | `full-test-final-3.log` |
| `tools/quality.py` on all 30 manifest parts | PASS: all 30 parts completed | `quality-final/`, `quality-final.log` |
| `tools/ci_acceptance.py --check-results quality` | PASS: 300 cells, no regressions; also passes after baseline refresh | `quality-acceptance-final.log`, `quality-acceptance-final-refreshed.log` |
| `python3 -I tools/check_leader_routes.py target/rule-evidence/quality-final` | PASS: 119 callouts on 30 parts; 14 unchanged pre-existing runs | `leader-routes-final.log` |

The quality baseline refresh records the already-landed K5 improvement documented in
`docs/status-2026-10-10.md`: string_post `hidden_on_visible` 26 → 25, total 2232 → 2231.
This batch does not claim that improvement. The original baseline is retained as
`quality-baseline-before-refresh.json`; all 300 part/metric cells are compared without
weakening any metric. Other metric totals are unchanged: off_sheet 0, repeated_label 7, repeated_measure 0, views_overlap 0, view_on_title_block 0, ink 52, repeated_span 0, line_through_feature 73, section_hidden 0.

## Remaining verification blocker

All-green verification was **not achieved**. The full run failed only
`multi_sheet_latency::r5_multi_sheet_latency` and `session::nist_benchmark_deltas`:

| Fixed 150 ms p90 gate | Full suite | Strict isolated harness retry |
| --- | ---: | ---: |
| Multi-sheet native | 276.214 ms | 138.394 ms (pass) |
| Multi-sheet MCP | 349.072 ms | 262.014 ms (fail) |
| NIST session | 202.799 ms | 150.748 ms (fail) |

The retries retain four test threads, the required corpus and every bound. Their logs are
`multi-sheet-strict-final-retry.log` and `session-strict-final-retry.log`. The full NIST
failure had eight samples below 114 ms and one at 202.799 ms. All non-latency tests and
all six batch regressions passed.

An unchanged-main archive at `53aa9f3f` also fails the same multi-sheet MCP gate:
native p90 57.681 ms, MCP p90 921.774 ms (`latency-original-main.log`). Its native/MCP
median instruction counts were 1,132,285,582 / 2,031,880,099, versus the final full run's
1,134,291,332 / 2,036,727,552. The final run's host load rose from about 10 to over 50;
`host-load-during-final.log` records 52.07, and 56.92 was also observed. These are evidence
of a pre-existing gate failure and substantial timing variability, not proof of a sole cause.
No latency bound, opt-in guard, quality metric or instruction budget was changed. Meeting
those fixed wall-time gates remains the concrete verification blocker; no rule implementation
is left pending. Commits are local and have not been pushed.
