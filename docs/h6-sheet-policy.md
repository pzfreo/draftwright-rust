# H6 sheet and scale choice, 2026-10-12

This report describes the code as committed after the second fix round (Claude, taking over
from two Codex attempts). Every figure below was measured on that build; earlier drafts of
this report are superseded and live only in git history.

## The rule as implemented

`model::recover` plans every standard sheet and scale at or above the legibility floor as a
finished drawing and judges it. `sheet_fit::choose` then applies the maintainer's rule.

1. **Valid and complete.** A candidate is valid and complete when it has no hard layout
   finding, every drawn item and view lies inside the border and off the title block, no two
   view areas overlap, it drops or loses no measurement the first estimate carried, every
   GD&T item is drawn, and it lays no more annotations over another view than the first
   estimate.
2. **Smallest fit.** The smallest fit is the smallest sheet with a valid complete candidate,
   at its largest such scale.
3. **One step up, only with enlargement.** Only the smallest fit and the next larger sheet at
   a larger scale compete.
4. **Material ratchet.** If the first estimate is valid, an allowed pair that adds leaders or
   witnesses through material is set aside. This never changes which pair is the smallest
   fit. If every allowed pair adds them, the valid first estimate is kept, and the basis says
   so. An invalid first estimate vetoes nothing.
5. **Fill.** In `sheet_fit::FILL_BAND` (60–85%, one named constant) the larger scale wins,
   then the smaller sheet. If no allowed pair reaches the band, the fullest wins.
6. **Spec overrides.** A Spec-named sheet keeps its sheet and chooses its scale by the same
   ranking. A named scale or pinned placement is never re-planned.

On the larger neighbour the orientation view keeps the smallest fit's paper scale. The extra
paper therefore goes to the dimensioned views and cannot be bought with a bigger isometric.
This removed the plate's A2 2:1 candidate's advantage (its isometric had grown to 2.05) and
keeps hidden-line counts at their earlier levels on CTC-03, cgb207 and STC-06.

If no candidate is valid and complete, the first drawing and its findings are kept on their
own sheet, with `layout_recovery_exhausted`. The finding is a warning when that drawing is
below the legibility floor, otherwise info. Only a first drawing below 1:10 is redrawn, at
1:10 on its own sheet.

**Fill** (`sheet.fill`, and the `fill` column of `tools/quality.py`) is the union of each
drawn item's bounding footprint with 1 mm clearance, inside the border and excluding the drawn
title grid. The items are view envelopes, dimensions with their witnesses, leaders and arrows,
GD&T frames, datum symbols, notes, tables, balloons, and the scale, projection and section
furniture. Reserved bands never count. The basis lists every measured candidate with its fill
and findings, and states whether the first estimate was valid.

**Legibility floor** (`sheet_fit::min_legible_scale`): the smallest hole or slot opening
must be at least 0.8 mm on paper and the longest extent at least 20 mm, and the floor never
goes below 1:10.

## Changes in this fix round

- **Removed the "least annotation conflicts" filter.** It was not part of the rule and it made
  10060, 10063, 10000 and string_post_tapped spend A3 for less fill. All four are back on A4.
- **The material veto no longer moves the smallest fit.** assembly_plate_pin's smallest fit is
  now A3 1:2 (valid, with two extra crossings), so A1 is out of reach. It stays on A2 1:1.
  spool_fits shows why a fallback is needed: its smallest fit is A4 1:5, and both allowed
  pairs (A4 1:5, A3 1:2) add crossings. Its valid first estimate, A2 1:1, is kept.
- **The exhausted fallback keeps the first drawing.** STC-09 stays on A1 1:5 instead of moving
  to an invalid A0 1:2. Its warning says the scale is below the 1:2 floor that its smallest hole
  needs. The fill is no longer printed twice in the basis, and "as the spec asks" no longer
  appears when the Spec is the default.
- **Orientation view.** On the larger neighbour it keeps the smallest fit's scale, shrunk
  about its own centre. Before, it kept the first estimate's scale, which made STC-06's
  isometric a thumbnail.
- **View areas may not overlap** in a valid candidate. Without this, CTC-01 chose A4 1:10 with
  overlapping view areas.

## Tests

`tests/common/sheet_rule.rs` is an independent oracle. It re-derives the rule from the
candidates that each basis records, without calling the selection code. It is applied to:

- 10060, 10063 and 10000, native: below the band the fullest valid pair wins
  (`below_the_band_the_fullest_valid_pair_wins_on_real_candidates`);
- GRM-03 and m10127 (`a_valid_plan_is_reconsidered_for_fill`, which no longer pins output);
- FTC-07, m1000, the 1004 view-gate stand-in, the supplied plate and GRM-03 in `sheet_fill`;
- every sampled part in `issue_drafting::sheet_selection_is_bounded_and_keeps_every_outcome`.

Run against the previous commit 4751b9f, with only the basis-format requirement relaxed, the
oracle fails on 10060 (A3 2:1 at 14.9% chosen, 17.0% available), m10127 (A3 2:1 at 25.8%,
26.9% available) and the plate (A3 1:1 at 38.0% chosen only through the extra filter, 44.0%
available). It passes now. On 4751b9f the STC-09 regression also fails (A0 1:2 instead of
A1 1:5). The unit tests in `sheet_fit` cover the band, the one-step limit, a named sheet,
the fullest pair below the band, and a material veto that never moves the smallest fit
(assembly_plate_pin and spool_fits as measured).

Tests restored from the first round:

- STC-09's band test again runs `plan_recovered`.
- The m1000 view-box test again injects a realistic over-view finding.
- The 1004 gate test checks that A4 1:1 is valid with exactly 2 extra over-view conflicts and
  is not taken.
- The title-block test also requires that no view area is over the title block.

The identity test keeps the plate in `REPLANNED`: H6 legitimately moves that fixture from A4
1:1 to A4 2:1 (the smallest fit's largest scale). Its title-furniture merge stays, guarded for
parts that are not replanned. The isometric-label test keeps its enlarged-isometric case. No
natural case is left: the five fixtures on A4 and A3 at 10:1 to 1:1, and on A3 and A2 with the
isometric pinned at zero gap beside each view, place no label over it.

## Sheet, scale and fill, before and after

"Before" is the archived pre-H6 release (`target/h6-before/draftwright`). Fill is measured
by `tools/quality.py` from each plan's own geometry. `tools/sheet_choices.py` independently
checks every row against the rule and against the Rust fill measure (to 1e-9).

Every sheet that is larger than before follows from the rule: the oracle confirms it is the
fullest allowed pair, or the larger-scale pair in the band.

### 30 native quality parts

| Part | Before | Fill % | After | Fill % | Basis |
|---|---|---:|---|---:|---|
| 1000 | A4 1:1 | 19.20 | A4 2:1 | 34.74 | smallest fit A4 at 2:1 |
| 10000 | A4 1:1 | 23.97 | A4 1:1 | 23.97 | smallest fit A4 at 1:1 |
| 10007 | A4 1:1 | 21.83 | A3 2:1 | 22.91 | smallest fit A4 at 1:1 |
| 10020 | A4 2:1 | 29.53 | A4 2:1 | 29.53 | smallest fit A4 at 2:1 |
| 10033 | A4 1:1 | 25.71 | A3 2:1 | 39.90 | smallest fit A4 at 1:1 |
| 10038 | A4 1:1 | 20.05 | A4 2:1 | 42.30 | smallest fit A4 at 2:1 |
| 1004 | A4 1:1 | 20.54 | A4 1:1 | 20.54 | smallest fit A4 at 1:1 |
| 10046 | A4 1:1 | 20.03 | A3 2:1 | 20.69 | smallest fit A4 at 1:1 |
| 10047 | A4 1:1 | 15.18 | A4 1:1 | 15.18 | smallest fit A4 at 1:1 |
| 10049 | A4 1:1 | 17.14 | A4 1:1 | 17.14 | smallest fit A4 at 1:1 |
| 10060 | A4 1:1 | 17.04 | A4 1:1 | 17.04 | smallest fit A4 at 1:1 |
| 10063 | A4 1:1 | 22.62 | A4 1:1 | 22.62 | smallest fit A4 at 1:1 |
| GRM-03_thumbwheel_drive_screw | A4 2:1 | 13.94 | A3 10:1 | 60.06 | smallest fit A4 at 5:1 |
| GRM-05_depth_lock_bolt | A4 2:1 | 12.54 | A3 5:1 | 16.64 | smallest fit A4 at 2:1 |
| cgb203 | A3 1:5 | 19.89 | A4 1:5 | 43.22 | smallest fit A4 at 1:5 |
| cgb207 | A2 1:1 | 29.51 | A1 2:1 | 35.83 | smallest fit A2 at 1:1 |
| cgb241 | A3 1:5 | 47.38 | A3 1:5 | 47.38 | smallest fit A3 at 1:5 |
| cgb245 | A2 1:2 | 40.27 | A2 1:2 | 40.27 | no valid complete candidate; first drawing kept |
| nist_ctc_01_asme1_rd | A3 1:5 | 57.56 | A3 1:5 | 57.56 | smallest fit A3 at 1:5 |
| nist_ctc_02_asme1_rc | A1 1:5 | 38.13 | A1 1:5 | 38.13 | no valid complete candidate; first drawing kept |
| nist_ctc_03_asme1_rc | A2 1:5 | 17.61 | A1 1:2 | 24.36 | smallest fit A2 at 1:5 |
| nist_ctc_04_asme1_rd | A1 1:5 | 24.32 | A1 1:5 | 24.32 | no valid complete candidate; first drawing kept |
| nist_ctc_05_asme1_rd | A2 1:5 | 38.89 | A2 1:5 | 38.89 | no valid complete candidate; first drawing kept |
| nist_ftc_06_asme1_rd | A2 1:5 | 14.43 | A1 1:2 | 19.51 | smallest fit A2 at 1:5 |
| nist_ftc_07_asme1_rd | A2 1:2 | 44.85 | A1 1:1 | 50.33 | smallest fit A2 at 1:2 |
| nist_ftc_08_asme1_rc | A2 1:2 | 42.19 | A2 1:2 | 42.19 | no valid complete candidate; first drawing kept |
| nist_ftc_09_asme1_rd | A2 1:1 | 69.09 | A2 1:1 | 69.09 | no valid complete candidate; first drawing kept |
| nist_ftc_10_asme1_rb | A1 1:1 | 19.72 | A1 1:1 | 19.72 | no valid complete candidate; first drawing kept |
| sm-hanger | A2 1:1 | 47.39 | A4 1:2 | 53.83 | smallest fit A4 at 1:2 |
| string_post | A4 2:1 | 8.99 | A3 5:1 | 11.60 | smallest fit A4 at 2:1 |

### 14 AP242 fixtures and the supplied plate

| Part | Before | Fill % | After | Fill % | Basis |
|---|---|---:|---|---:|---|
| assembly_plate_pin | A2 1:1 | 24.54 | A2 1:1 | 19.19 | smallest fit A3 at 1:2 |
| bolt_thread_knurl | A4 1:1 | 15.10 | A4 2:1 | 22.87 | smallest fit A4 at 2:1 |
| bracket_positions | A2 1:1 | 33.34 | A2 1:1 | 33.34 | smallest fit A2 at 1:1 |
| manufacturing-pmi | A2 1:1 | 17.99 | A3 1:1 | 38.01 | smallest fit A3 at 1:1 |
| nist_ctc_01_asme1_ap242-e1 | A3 1:5 | 60.72 | A3 1:5 | 60.72 | smallest fit A3 at 1:5 |
| nist_ctc_01_merge | A3 1:5 | 61.92 | A3 1:5 | 61.92 | smallest fit A3 at 1:5 |
| nist_ctc_02_asme1_ap242-e2 | A1 1:5 | 39.12 | A1 1:5 | 39.12 | no valid complete candidate; first drawing kept |
| nist_ftc_07_asme1_ap242-e2 | A2 1:2 | 47.80 | A2 1:2 | 47.80 | no valid complete candidate; first drawing kept |
| nist_ftc_10_asme1_ap242-e2 | A1 1:1 | 23.86 | A1 1:1 | 23.86 | no valid complete candidate; first drawing kept |
| nist_stc_06_asme1_ap242-e3 | A0 1:1 | 42.46 | A0 1:1 | 42.46 | smallest fit A0 at 1:1 |
| nist_stc_09_asme1_ap242-e4 | A1 1:5 | 12.07 | A1 1:5 | 12.07 | no valid complete candidate; first drawing kept |
| nist_stc_10_asme1_ap242-e2 | A1 1:1 | 23.61 | A1 1:1 | 23.61 | no valid complete candidate; first drawing kept |
| spool_fits | A2 1:1 | 47.53 | A2 1:1 | 47.53 | valid first estimate kept (every allowed pair adds material crossings) |
| string_post_tapped | A4 2:1 | 14.89 | A4 2:1 | 14.89 | smallest fit A4 at 2:1 |
| thumbwheel_thread_knurl | A4 2:1 | 25.99 | A3 10:1 | 48.45 | smallest fit A4 at 5:1 |

## Quality

No part is worse on any column. The native 30 were compared with the latest pre-H6 table in
`status-2026-10-10.md` (390 cells). The AP242 14 and the plate were compared with the
archived before binary under the same `quality.py` (195 cells).

| Metric | Native before | Native after | AP242 + plate before | AP242 + plate after |
|---|---:|---:|---:|---:|
| ink | 273 | 262 | 1612 | 1593 |
| line_through_feature | 211 | 207 | 447 | 446 |
| hidden_on_visible | 2263 | 2257 | 1425 | 1424 |
| repeated_label | 17 | 17 | 16 | 16 |
| off_sheet, views_overlap, view_on_title_block, repeated_span, section_hidden | 0 | 0 | 0 | 0 |
| claimed_faces / faces / unplanned | 1390 / 4268 / 277 | same | 988 / 2492 / 149 | same |

Per-part changes, all improvements:

- Native: 1000 ink 1→0; 10038 ink 1→0; cgb207 ink 7→6; CTC-03 ink 24→21 and material 8→4;
  FTC-06 ink 11→9; FTC-07 ink 9→7; sm-hanger hidden 68→62; string_post ink 2→1.
- AP242: bolt_thread_knurl ink 4→3; manufacturing-pmi ink 18→8; thumbwheel_thread_knurl ink
  12→4, material 1→0 and hidden 14→13.

## Visual review

Sheets were rendered with the release CLI, rasterised with resvg at 1600 px and inspected:

- **Plate (A3 1:1):** all six GD&T items are clear and no view is over the title block.
- **GRM-03 (A3 10:1):** clean, but the isometric is large. See the open item below.
- **assembly_plate_pin (A2 1:1):** smaller isometric, otherwise unchanged.
- **STC-06 (A0 1:1):** its isometric is back to its earlier size.
- **10060 (A4 1:1):** unchanged.
- **CTC-03 (A1 1:2), cgb207 (A1 2:1), FTC-06 (A1 1:2), 10046 (A3 2:1):** valid and clear, but
  with a lot of empty paper. That is what the rule requires below the band.

## Verification

All checks ran on this commit's source, one at a time:

- `cargo fmt --check`: passes.
- `cargo clippy --all-targets --release -- -D warnings`: passes.
- `DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`: exit 0, 735 tests passed, 0 failed.
- `tools/quality.py` with the release build, on the 30 parts of `tools/quality_parts.txt`: no
  part worse than the latest pre-H6 table (390 cells). The same run on the 14 AP242 fixtures
  and the plate: no part worse than the archived before binary (195 cells).
- `tools/sheet_choices.py`: all 45 choices pass the rule check, and the independent footprint
  measurements agree with the Rust fill.
- Random-pin grid with `--require-go`, seeds 1–3 × N = 1/3/8, against the same-binary
  baseline: 9/9 GO, 0 new off-sheet items. Pinned ink: 255, 247, 246, 260, 212, 216, 259, 248,
  229 (unpinned 262).
- Scripted MCP evaluation (`tools/mcp-eval/run-scripted.mjs`): 7/7 tasks succeed with 16
  calls; `tasks.json` is unchanged.
- `tools/test_quality.py`: 24/24. Node client tests: 8/8.

## Open items and judgement calls

All of these are recorded in `next-session.md`.

- **Material ratchet.** It is part of the selection, not of the smallest fit, and a valid first
  estimate is kept when every allowed pair adds crossings (spool_fits). Without it,
  spool_fits' material count rises 10→12. Overturnable.
- **Paper economy.** Several A3/A1 choices gain only about one point of fill (10007 +1.1,
  10046 +0.7, assembly_plate_pin's alternative +0.6). The rule says "fullest". A
  minimum-gain margin is the maintainer's call.
- **GRM-03's band.** GRM-03 reaches 60.06% only because the measured correction
  (`sheet_fit::correct_estimate`) grows its free-row isometric up to 1.3× the drawing scale.
  Without that growth it is about 45%. This is unchanged from the previous round and is left
  for the maintainer to reconcile with a subordinate orientation view.
- **Legibility floor.** The 0.8 mm opening and 20 mm extent are geometric proxies. STC-09 is
  below its floor and kept there with a warning, because no legible candidate is valid.
- **Footprints.** Each item's footprint is its axis-aligned bounding box, so a long diagonal
  leader counts its empty box. This is unchanged.
