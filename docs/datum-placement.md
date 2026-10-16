# Datum placement validation

Base: `3836b296` (the branch's starting main). Evidence is under
`target/datum-placement/`; the base and verified executables were copied before collection
so an in-progress build cannot change the binary halfway through a survey.

Automatic datums try compact tiers before outer dimension tiers. Cylinder alternatives are
points proved on the trimmed face's opposite silhouette; planar alternatives carry an actual
face extension, with a 1 mm outline gap and an overrun beyond the triangle. Shared obstacle
grading counts crossings and collinear travel for datum leaders, frames, labels and finishes.
The original complete arrangement remains a candidate. A rearrangement must introduce no new
annotation contact (code and unordered subject pair); datum-only improvements are tried when
the full rearrangement cannot meet that condition. Explicit placement intents and pins retain
priority. No sheet policy or single-sheet byte fixture was changed.

## Spindle

The CNC-review spindle stays on A4 at 1:1. B stands inside the right-hand dimension lanes.
A stands on a short leftward extension of the bottom face. Both have zero crossings and no
planner or rendered structural-lint findings. Its other, pre-existing GD&T findings remain
visible. Visual review confirms the short leaders, clear datum letters, and unbroken dimension
stack; no scale reduction or larger sheet was used to obtain the improvement.

## Survey method

`tools/survey_datum_placement.py` scans STEP inputs in the corpus and fixtures, including gzip
and case-insensitive extensions. It identifies datum-bearing files, deduplicates identical STEP
bytes, and records aliases and scanned paths in `inventory.json`. It saves both the full Plan
and rendered `review` lint. Crossings and overlaps involving any drawn GD&T item are counted as
unique `(code, unordered subjects)` findings, merging duplicate reports from planner and lint.
Crossings include dimension/extension/leader intersections, leaders through recognized
features (`leader_through_feature`), extension-through-feature and silhouette/view crossings.
Overlaps include ink/text, view, title-block and furniture overlap codes. Neither category may
increase on any part. Annotation-only, feature-only and datum-specific counts remain in JSON. The corpus's ten older NIST
CTC/FTC RD/RC/RB files have presentation-only PMI and no semantic datum features; their native
quality drawings are checked by the 30-part quality gate. AP242 semantic datums are surveyed
from the CTC/FTC/STC fixtures.

Reproduce with the two saved executables, or binaries built from the corresponding revisions:

```sh
python3 -I tools/survey_datum_placement.py target/datum-placement/base-draftwright \
  target/datum-placement/base --corpus /Users/paul/repos/quiddity/tests/corpus
python3 -I tools/survey_datum_placement.py target/datum-placement/head-draftwright \
  target/datum-placement/verified --corpus /Users/paul/repos/quiddity/tests/corpus \
  --compare target/datum-placement/base
```

| Part | Datums | Crossings base → head | Overlaps base → head |
| --- | ---: | ---: | ---: |
| assembly_pin_pmi | 0 | 0 → 0 | 0 → 0 |
| assembly_plate_pin | 3 | 7 → 7 | 0 → 0 |
| bolt_thread_knurl | 2 | 3 → 1 | 0 → 0 |
| bracket_positions | 3 | 10 → 9 | 0 → 0 |
| cnc-review-spindle | 2 | 5 → 2 | 0 → 0 |
| editor_pmi | 3 | 15 → 15 | 0 → 0 |
| hw_assembly_plate_pin_intent | 3 | 6 → 6 | 0 → 0 |
| hw_every_kind | 3 | 29 → 24 | 1 → 0 |
| hw_spool_fits_intent | 2 | 19 → 16 | 0 → 0 |
| hw_string_post_tapped_intent | 1 | 3 → 3 | 0 → 0 |
| hw_thumbwheel_notes_add | 2 | 7 → 4 | 0 → 0 |
| hw_thumbwheel_thread_knurl_intent | 2 | 4 → 1 | 0 → 0 |
| manufacturing_pmi | 3 | 15 → 15 | 0 → 0 |
| nist_ctc_01_asme1_ap242-e1 | 3 | 14 → 13 | 0 → 0 |
| nist_ctc_01_merge | 3 | 14 → 13 | 0 → 0 |
| nist_ctc_02_asme1_ap242-e2 | 5 | 39 → 38 | 0 → 0 |
| nist_ftc_07_asme1_ap242-e2 | 5 | 62 → 62 | 5 → 5 |
| nist_ftc_10_asme1_ap242-e2 | 8 | 415 → 415 | 15 → 15 |
| nist_stc_06_asme1_ap242-e3 | 4 | 80 → 80 | 0 → 0 |
| nist_stc_09_asme1_ap242-e4 | 5 | 475 → 475 | 9 → 9 |
| nist_stc_10_asme1_ap242-e2 | 8 | 288 → 288 | 11 → 11 |
| plate_face_ra0_8 | 3 | 16 → 16 | 0 → 0 |
| plate_intent | 3 | 5 → 5 | 0 → 0 |
| plate_intent_rewritten | 3 | 5 → 5 | 0 → 0 |
| plate_part_ra1_6 | 3 | 15 → 15 | 0 → 0 |
| spindle_diamond_knurl | 2 | 3 → 0 | 0 → 0 |
| spindle_external_thread | 2 | 3 → 0 | 0 → 0 |
| spindle_turned_knurl | 2 | 3 → 0 | 0 → 0 |
| spool_fits | 2 | 19 → 14 | 0 → 0 |
| string_post_tapped | 1 | 4 → 4 | 0 → 0 |
| thumbwheel_thread_knurl | 2 | 4 → 1 | 0 → 0 |
| **Total** | | **1587 → 1547** | **41 → 40** |

Combined findings fall **1628 → 1587**, a reduction of 41 across 15 improved parts
(16 unchanged). Crossings fall **1587 → 1547** and overlaps **41 → 40**; neither category
increases on any part. Datum-involved crossings fall **133 → 97**, and datum-involved overlaps
**20 → 19**. Annotation-only crossings are **1304 → 1271**; recognized-feature crossings are
**283 → 276**. The extra bounds audit also improves: out-of-area findings **56 → 55**, with
no added off-sheet, title-block or view contacts.

`hw_every_kind` loses one overlap and six recognized-feature crossings while gaining one
annotation-line crossing, reducing its full crossing/overlap total from 30 to 24. Its sheet
falls from A1 to A2 at the same 1:1 scale. This keeps the better drawing: legible frames and
less paper at the same measured scale. CTC-02 crosses one fewer recognized feature while its
annotation-only count stays unchanged. STC-06 and STC-09 retain their original arrangement.
The survey includes 31 unique inputs from 183 scanned STEP files; 30 inputs carry drawn
datums. `assembly_pin_pmi` has no lowerable datum attachment: its datum faces occur in two
assembly placements, and datum B's reference faces are not coaxial. The reader's existing
refusal is unchanged; no attachment is invented for an ambiguous assembly reference.

## Renders

All are PNG at 150 dpi; both revisions were rendered and visually inspected.

| Part | Before | After |
| --- | --- | --- |
| CNC-review spindle | [PNG](../target/datum-placement/base/cnc-review-spindle.png) | [PNG](../target/datum-placement/verified/cnc-review-spindle.png) |
| NIST CTC-02 AP242 e2 | [PNG](../target/datum-placement/base/nist_ctc_02_asme1_ap242-e2.png) | [PNG](../target/datum-placement/verified/nist_ctc_02_asme1_ap242-e2.png) |
| NIST STC-06 AP242 e3 | [PNG](../target/datum-placement/base/nist_stc_06_asme1_ap242-e3.png) | [PNG](../target/datum-placement/verified/nist_stc_06_asme1_ap242-e3.png) |
| NIST STC-09 AP242 e4 | [PNG](../target/datum-placement/base/nist_stc_09_asme1_ap242-e4.png) | [PNG](../target/datum-placement/verified/nist_stc_09_asme1_ap242-e4.png) |

## Checks

The four regressions in `tests/datum_placement.rs` all failed against the base implementation
(`target/datum-placement/regressions-base.log`), then passed with the change: the spindle's
clear datums, inside-tier preference, the opposite cylinder silhouette, and a planar face
extension. The spindle assertion also requires A4 at 1:1 and checks rendered structural lint;
the cylinder test requires the selected opposite site to be reader-proven. The three synthetic
regressions also check that the editor's empty `place: {}` reset restores automatic placement. Existing tests
were not weakened. The opt-in single-sheet byte guard was not enabled or re-recorded.

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`: passed, 997 tests across 102 test/doc-test binaries, zero failures or ignored tests.
  The required checks ran sequentially with `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6`;
  evidence: `target/datum-placement/tests-final.log` and `clippy.log`.
- `python3 -I tools/check_leader_routes.py target/datum-placement/verified-quality`: passed,
  119 callouts on 30 parts; 14 unchanged pre-existing long runs remain.
- `tools/quality.py` followed by `tools/ci_acceptance.py --check-results quality`: passed all
  300 cells on 30 parts, against both the old and re-recorded quality baseline. Its sole
  improvement is `string_post.hidden_on_visible` **26 → 25** (aggregate **2232 → 2231**),
  already present at base `3836b296`. All other quality metrics are unchanged.
- All eight requested PNGs were rendered and visually inspected.
- Sheet audit: every selected A2-or-larger sheet is needed at 1:1. The conservative envelope
  budget alone identifies all but `bracket_positions`; a measured A3-at-1:1 trial for that
  part has six hard findings (two off-sheet annotations, two ink overlaps, a label overlap,
  and an unplaced note), confirming its content needs the larger sheet. Evidence:
  `target/datum-placement/sheet-rule.json` and `bracket-a3.plan.json`.

The crossing-repair tests (`tests/fix_relayout.rs`) retain all their existing assertions.
Their fixture now explicitly authors A below the face, B beyond the dimensions, and the two
initial dimension lanes, preserving the deliberately crossed starting state that automatic placement has fixed.
This continues to test both proposed fixes and the hard error when an editor pins both
measurements to a shared line.


The unchanged multi-sheet latency test exposed filesystem stalls in this environment:
base MCP p90 was 447.4 ms and head reached 912.7 ms, despite comparable instruction counts.
The fixture has no GD&T. Moving only Cargo's temporary test cache out of the watched worktree
(`target/tmp` now links to `/private/tmp/draftwright-datum-test-cache-4f6aac0c-d5a0-4c82-bc67-2d2552947882`)
made the same unmodified isolated test pass at 134.7 ms p90. The 150 ms gate remains enabled;
no latency threshold, test body, or instruction budget was changed.

Additional Python checks pass: `tools/test_quality.py` runs 40 tests;
`tools/test_ci_acceptance.py` runs 24 tests with its one existing platform skip. The latter's
Darwin pause test now captures the actual child PID from `Popen`, instead of searching the
process list through unavailable `sysmond`/`pgrep`. It retains all three real stop/continue
cycles and the original instruction-count assertion, and adds a child-start precondition.
No test was skipped or weakened to obtain this pass.
