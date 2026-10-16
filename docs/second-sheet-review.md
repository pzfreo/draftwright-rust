# Single-sheet review, 2026-10-16

## Why the original candidates failed

The 43602494 audit selected two A3 sheets for both 10224 and 10255. The exact
single-sheet A3 candidate findings in that audit were:

| Part | Scale | Hard findings | Crowded hole locations | Hole items dropped |
|---|---|---:|---:|---:|
| 10224 | 10:1 | Principal ink exceeds usable paper (preflight) | — | — |
| 10224 | 5:1 | 14 | 10 | 9 |
| 10224 | 2:1 | 2 | 0 | 0 |
| 10224 | 1:1 | 0 | 1 | 0 |
| 10224 | 1:2 | 0 | 1 | 0 |
| 10255 | 10:1 | Principal ink exceeds usable paper (preflight) | — | — |
| 10255 | 5:1 | 10 | 0 | 0 |
| 10255 | 2:1 | 2 | 0 | 0 |
| 10255 | 1:1 | 0 | 1 | 0 |
| 10255 | 1:2 | 0 | 1 | 0 |

At 5:1 the 10224 count is 12 `annotation_off_sheet` findings, one
`page_fit_uncertain`, and the failed finished-bounds gate. For 10255 it is eight
`annotation_off_sheet` findings plus those same two fit failures.

At 2:1 both hard counts consist of `page_fit_uncertain` and the additional failed
`drawn_items_fit` gate. The section reservation ended at x=416.391 mm (10224) or
419.488 mm (10255), outside A3's x=410 mm drawable edge. The actual section ink
was inside the page. Measured correction explicitly refused every plan containing
a section. These failures were conservative layout limitations, not a physical
need for another sheet. Existing annotation/material-crossing warnings were not
what rejected the candidates.

The failures at smaller scales are real under the unchanged legibility rule:
10224's side-view ordinates 39.025654 and 42.361174 mm are only 3.335521 mm apart
at 1:1; 10255's 11.573071 and 14.581762 mm ordinates are 3.008691 mm apart.
Both fall below the 4.7 mm minimum witness separation. At 2:1 the separations
are 6.671041 and 6.017383 mm. Larger scales have real space/annotation failures;
there is no reason to use them once a complete 2:1 single A3 drawing is available.

## Correction and verification

Recovery measures complete view blocks, including dimension arrows, centrelines,
section captions and cutting-plane letters. It preserves principal-view alignment,
can place a free section in another gap, and refits the iso around the settled
blocks. A correction is kept only if the existing whole-sheet bounds and disjoint
view checks pass; it never changes an already fitting section layout. Both reviewed
parts now use one A3 sheet at 2:1, without dropping a required measurement.

Extra sheets first try evenly spaced cells above the title block, with 10 mm of
additional frame clearance; they retain dense wrapping packing when cells cannot
hold the blocks. A reduced iso gets the normal `ISO VIEW (NTS)` caption.

The old forward zone was calculated from the section caption, so it could point
to B4 while the section itself occupied A3–A4. Structural lint checked that same
caption, perpetuating the mistake. The target is now the section view; the return
target remains the actual cutting-plane letter. Each reference uses the target
sheet's grid. Tests render both 10224 and 10255 with explicit auxiliary assignments,
compare both printed references to the drawn target bounds, move a caption across
a zone boundary without moving the view, and verify that moving either actual
target without changing its reference is detected.

## Render evidence

PNG and SVG files are in `target/second-sheet/revision/renders/`.

| Case | Sheets inspected | PNG paths within that directory |
|---|---:|---|
| 10224, automatic | 1 | `10224.png` |
| 10255, automatic | 1 | `10255.png` |
| 10224, explicit auxiliary assignment | 2 | `10224-assigned.png`, `10224-assigned.sheet2.png` |
| 10255, explicit auxiliary assignment | 2 | `10255-assigned.png`, `10255-assigned.sheet2.png` |
| Other changed default selections | 1 each | `14052.png`, `205.png`, `1008.png` |

The automatic 10224/10255 drawings retain aligned principal projections, place the
section inside the right frame and fit an NTS iso above the principal row. The
explicit-assignment examples exercise the extra-page code: iso and section occupy
separate, horizontally spaced cells near the page's vertical centre, clear of the
frame and title block. These examples are deliberately assigned; a second page
would be unnecessary for either default drawing. Their forward address is C6 and
the return address is C1, matching the section and cutting-plane letter drawn.

## Audit totals

The rerun covers the same 190 inputs and 2,150 cases: 1,947 planned, 203 refused,
zero panics or newly refused cases. Compared with 43602494, **2 of the reviewed
68 selections change**, both from two A3 2:1 sheets to one. Relative to the original
53aa9f3f baseline, three of those 68 improve (including the existing GRM-03 fixed
10:1 improvement). All 68 now use one sheet; 44 explicitly requested large sheets
remain as authored. The 742 cases classified as requiring large paper at full
size retain their paper, scale and count.

Across the complete audit, **156 small default parts use one sheet and zero use
two**. Counting their captured specs as separate cases, all 1,205 small cases use
one sheet. No automatic multi-sheet selection remains anywhere in the audit, so
there are no further automatic two-sheet drawings to render.

Eight paper/scale/count selections change from 43602494:

| Case (default intent) | Previously | Now |
|---|---|---|
| 10224 | A3 2:1, two sheets | A3 2:1, one sheet |
| 10255 | A3 2:1, two sheets | A3 2:1, one sheet |
| 14052 | A3 2:1 | A4 2:1 |
| 1008 | A4 1:2 | A3 2:1 |
| holdout/205 | A4 1:1 | A3 2:1 |
| counterbored_block fixture | A4 1:1 | A3 2:1 |
| u_channel fixture | A3 1:1 | A4 1:1 |
| u_channel_asymmetric fixture | A3 1:1 | A4 1:1 |

Every listed drawing was rendered and inspected. The last three fixtures are also
in the render directory as `counterbored_block.png`, `u_channel.png` and
`u_channel_asymmetric.png` (with matching SVGs). Complete machine-readable evidence
is in `target/second-sheet/revision/audit/audit.json` and `audit-summary.json`.


## Required checks

Ran with `CARGO_BUILD_JOBS=4` and `RUST_TEST_THREADS=4`, sourcing Cargo's environment.
The required corpus path and `DRAFTWRIGHT_CORPUS_REQUIRED=1` were set for tests.

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --release -- -D warnings` | Passed |
| Release tests, required corpus | 1,007 passed; two wall-clock timing failures; zero ignored |
| Fresh `tools/quality.py`, all 30 parts | Completed |
| `tools/ci_acceptance.py --check-results quality`, frozen baseline | Passed all 300 cells; nothing worse |
| `python3 -I tools/check_leader_routes.py` on those drawings | Passed: 119 callouts / 30 parts; no regressions |
| `tools/check_plan_time.py`, unchanged 63-part budget | Passed: 2,739,438,041,483 instructions against a 3,027,016,716,719 limit (1.041 × recorded figure; 1.15 × allowed) |

The exact requested `cargo test --release` stopped at `r5_multi_sheet_latency`:
MCP p90 was 518.002 ms against 150 ms; direct edit p90 passed at 62.209 ms. The
remaining 46 targets then ran with `--no-fail-fast`, followed by doc tests.
`nist_benchmark_deltas` also failed its unchanged wall-clock bound: 468.962 ms
p90 against 150 ms. Every integration target is accounted for (98), with 102
completed suites overall. All other tests, including the new zone and single-sheet
regressions, passed. No timing threshold, quality baseline, instruction budget or
opt-in byte guard was changed or re-recorded.

Logs, comparison results and the complete check summary are in
`target/second-sheet/revision/verification.json` and its sibling log files.
