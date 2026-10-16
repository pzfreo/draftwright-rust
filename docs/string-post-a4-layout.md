# String post on A4 (2026-10-16)

The case is `corpus/gramel/string_post.step:spec:1c29f39d…`, with the fourteen
independent tolerances preserved in `tests/fixtures/layout/string_post_tolerances.spec.json`.
The baseline is this branch merged with `origin/main` de60956a; single-sheet
expectations were taken from main before recording.

## Diagnosis

Coordinates below are paper millimetres, origin at the lower left. A4's drawable
area is `[10, 10, 287, 200]`.

| Pinned pair | Hard findings before the fix | Cause |
| --- | --- | --- |
| A4 5:1 | `page_fit_uncertain`; `annotation_off_sheet` on `ink:v:plan:f:hole:1`; `annotation_off_sheet` on `ink:v:plan:f:hole:2`; `annotation_ink_overlap` between `d:f:hole:1/location.length.z` and `d:f:turned_step:1/step.length` | The section's bottom band hit the title block, so `layout::geometry_with` lifted every view by 38.75 mm. The end view's area ended at y=229.5 and its outline at y=209.5; the two centre strokes ended at y=207.254 and y=212.5. Separately, the hole's witness at y=133.25 crossed the middle of the `5.5 ±0.0008` label, whose box was `[88.37,123.767,90.536,142.733]`. Sliding within the 27.5 mm span could not clear that witness. |
| A4 2:1 | `annotation_ink_overlap` between `d:f:turned_step:4/step.length` and `d:f:hole:1/bore.diameter` | The 7.2 mm length has outside arrows and a long label below its short span. Its dimension line at x=76, y=73.3–97.1 crosses the hole callout's label box `[45.5,73.83,79.445,76.08]`. The callout settles in the lower corridor among the long tolerance labels; the later bounded label pass cannot clear the remaining contact within the occupied right-hand tower. |
| A3 5:1 | The same hole-location witness/step-length label overlap as A4 5:1 | More paper does not change the lane assignment or the relative witness/label positions. The witness is at y=138 and the label spans y=128.517–147.483. |

The search reports **five** hard findings for A4 5:1: the four reported problems
above plus its failed independent measured-fit check. This is not a fifth
annotation problem. The automatic baseline selected A3 2:1, 6.6% fill.

## General changes

* When the aligned section/title-block lift would overflow, try lifting only the
  section group, keeping the established row when GD&T corridors are reserved. Require every resulting block to stay on the page, clear the title
  block and clear the principal blocks. Preserve front/plan projection alignment.
* After linear label sliding, try up to eight lanes on the opposite side for an
  unresolved collision in edited tolerance/fit text, before routing callouts. Require a candidate to
  clear every peer label and line, avoid shared dimension lines, and fit the page.
  Canonical layouts stay stable; drawings with authored placements keep their existing
  avoidance pass. Fixed annotation ink remains an obstacle. The full inventory retains
  omission locality.
* Keep the hard checks, 60–85% fill target and A2 policy unchanged.

The automatic result is **one A4 sheet, 5:1, 35.1% fill, zero hard findings**, with
all fourteen toleranced statements drawn. The `5.5 ±0.0008` statement uses the
left corridor. The default string post is also A4 5:1 (33.6% fill), previously
A3 5:1 (12.7%). This deliberately accepts a vertically offset section rather than
wasting the usable height of the principal projection column. The target remains
unmet: the valid 5:1 drawing is chosen by the existing fullest-valid policy;
10:1 remains invalid. No second sheet is needed.

## Visual review

Artifacts are under `target/layout-review/`, relative to this worktree. Each stem
has PNG (120 dpi), SVG, PDF, plan JSON and render JSON.

| Case | Before PNG | After PNG | Visual assessment |
| --- | --- | --- | --- |
| string post default | `before/string_post.png` | `after/string_post.png` | Same dimensioned scale on A4; clearer use of paper. |
| fourteen tolerances | `before/string_post_spec.png` | `after/string_post_spec.png` | 2:1 to 5:1; readable separate labels; no clipping. |
| GRM-03 | `before/grm03.png` | `after/grm03.png` | Unchanged A3 10:1. |
| GRM-05 | `before/grm05.png` | `after/grm05.png` | Unchanged A3 5:1. |
| mfcadpp/1000 | `before/1000.png` | `after/1000.png` | Unchanged A4 2:1. |

The pinned diagnostic renders are `before/A4-5-1.png`, `before/A4-2-1.png`
and `before/A3-5-1.png`. All ten comparison PNGs and all three pinned PNGs
were inspected. No comparison part moves to A2 or larger.

## Historical panic resolved

The unchanged rebaseline guard first stopped on cgb245 spec `721695f2…`: main
recorded `the prior route meets its own floor`, whereas the new layout produces
exact drawing bytes. The archived merged-baseline executable reproduces that panic.
The new PNG was inspected (`target/layout-review/cgb245-after.png`); this is the
380 mm sheet-metal part on A2, outside the small-part case. Its prior panic
expectation was replaced by the exact hashes captured by the guard, then folded
into the rebaseline. This large-part case still reports unresolved layout findings;
it now produces a drawing instead of panicking. The test's refusal/panic equality check was retained.

## Fixture record

Re-recorded all **1,876 macOS cases** with
`DRAFTWRIGHT_REBASELINE_SINGLE_SHEET=f1-ext-threads-knurl`. Against main's
committed effective expectations (base plus main pins and approved replacements),
**171 cases change: 127 plan-only and 44 rendered**. The case set is unchanged.
Linux overrides and PNG hashes are stale after this rebaseline and await CI.
The complete changed-case list is `target/layout-review/fixture-changes.json`.

The plan-only changes record sheet candidates, recovery diagnostics and the
large-sheet rule, including 102 GRM-03 cases. Of the rendered cases, 33 retain
this PR's existing thread/knurl depiction and turned-leader changes: GRM-03 ×13,
CTC-05 ×4, turned ×2, assembly plate pin ×5 (original and haecceity-written), and
thread/knurl fixtures ×9. The other eleven reflect this layout work: string post
×7, one edited case each of CTC-01, CTC-04 and FTC-06, and the cgb245 panic case.
The seven string-post variants move to A4 5:1; none loses a statement. The two
large NIST cases with edited tolerance text retain their scales; the large CTC-04
case can enlarge from A1 1:5 to A0 1:2 under the unchanged sheet policy.

The extra GD&T comparison, `hw_string_post_tapped_intent`, initially exposed a
scale regression during development. Retaining its reserved projection row
restores A3 5:1 and byte-identical PNG output. Both
`target/layout-review/hw-tapped-before.png` and `hw-tapped-after.png` were inspected;
a second regression test protects its readable scale.

## Overflow test coverage

R5's original overflow fixture relied on the very section-placement defect fixed
here. The bare string post now correctly needs one A4 sheet. The overflow tests
therefore add an inspection schedule to the same native part, retaining A4, the
5:1 floor, two sheets, both cumulative prefixes, the planner-work bounds and all
existing assertions. New assertions also require the table to be drawn at 5:1.
The unchanged 150 ms gates passed with session p90 52.5 ms and MCP p90 119.1 ms;
all 27 allocation tests passed. The schedule is test input only.

## Final checks

* `cargo fmt --check` and release Clippy for all targets with `-D warnings` pass.
* The required corpus release suite passes every binary before `stateless`,
  including the normal 1,876-case single-sheet replay and both latency gates.
  Seven HTTP tests fail at server startup: this sandbox rejects the localhost
  socket bind with `Operation not permitted (os error 1)`. Its other six tests pass.
  The nine integration binaries after `stateless` were run separately and pass;
  doc tests also pass (none defined). No tests were skipped or disabled in source.
* Quality is no worse in all 300 metric cells across 30 parts. Aggregate
  hidden-on-visible contacts improve from 2,232 to 2,231; other metrics are unchanged.
* `python3 -I tools/check_leader_routes.py` passes for 119 callouts on 30 parts.
  Its fourteen pre-existing runs are unchanged.

Logs and machine-readable comparisons are in `target/layout-review/`: use
`full-tests-complete.log`, `remaining-tests.log`, `doc-tests.log`,
`quality-validation-final-source.json` and `leader-routes-final-source.log`.
