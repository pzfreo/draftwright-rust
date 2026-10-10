# Alpha 1: the plan and its gates

The maintainer's plan (2026-10-09): bring draftwright-rust to an alpha 1 with PMI and the quality
fixes; then get the quiddity-rust (haecceity) and specify-core-rust changes in; then cut an alpha 1
release of each; then carry on with draftwright-rust improvements.

Until every gate below is met, the rolling run dispatches **only alpha 1 work** in all three repos
(and fixes to it). Everything else in the backlog waits until the releases are cut.

## 1. draftwright-rust alpha 1

- **PMI, drawn from the part's AP242 semantic PMI** (haecceity's reader):
  - reading: done (stage 1); every `rust-wrong` entry in `known_pmi.json` (55 at the time of
    writing) fixed, or re-examined and given a settled verdict with evidence;
  - feature control frames and datum feature symbols: done (stage 2a);
  - **toleranced and basic dimensions from PMI** (stage 2b; Python's
    annotations/_pmi_dimensions.py);
  - **PMI coverage lint** (stage 3; linting/pmi_coverage.py): every PMI item is on the drawing
    or reported;
  - checked on NIST CTC/FTC and on specify-core-written files against Python draftwright's
    drawings, every difference with a verdict.
- **Quality**, measured with `tools/quality.py` on `tools/quality_parts.txt` (baseline in
  `docs/quality-2026-10-09.md`):
  - annotations off the sheet: **0**;
  - overlapping view pairs and views on the title block: **0**;
  - repeated dimension labels: each one remaining justified with a verdict (identical features
    or quantities draftwright also states separately), none unexplained;
  - ink problems (gate relaxed by the maintainer on 2026-10-09, from "at least 80% fewer"):
    **no annotation over a view and no annotation line over a view** (0), **no part worse than
    the baseline** in any column, and every remaining ink problem (leader crossings and ink
    overlaps, about 284 at the time, from 506) listed as a known limitation in the README with
    its verdict. Reducing them further continues after alpha 1.
- **Hole tables are not an alpha 1 gate** (maintainer, 2026-10-09): drop-and-escalate stays as
  merged; any remaining hole-table issue is listed as a known limitation in the README, not
  worked on before the release.
- **Performance:** the maintainer's PR #1 merged (render cache and straight-piece boxes); `plan`
  back to fast (NIST FTC 06 under 50 ms natively, from about 490 ms), drawings unchanged.
- **Release hygiene:** CI green; README states what alpha 1 does and does not do; a short
  CHANGELOG entry.

## 2. quiddity-rust (with haecceity) alpha 1

- haecceity's PMI writer, from the maintainer's decisions of 2026-10-09: no
  `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` for datum feature symbols (OpenCascade crashes on
  it); one `geometric_item_specific_usage` per face for multi-face sizes and tolerances; notes
  and surface texture in standard forms. Each verified by read-back and an OpenCascade
  cross-check.
- All 46 recognisers ported: section recesses wired into recognition.
- specify-core-rust's upstream needs (its docs/upstream-needs.md) done or explicitly deferred
  with a reason.
- CI green on Linux and macOS, each around 10 minutes.

## 3. specify-core-rust alpha 1

(Its own run, driven from the main session.)

- The maintainer's decisions merged: the move to `prototype/rust-port`, fingerprint schema 2,
  ISO 286 to 3150 mm, faceless items written around, knurl parameters.
- The new haecceity writer adopted: OpenCascade (and so specify-core's Python) opens every
  Rust-written file; multi-face tolerances cross; notes and surface finish written.
- CI green, including the Linux-only cross-engine test.

## 4. Cutting the releases

Done by the main session, in dependency order, once 1-3 are met:

1. quiddity-rust: version `0.1.0-alpha.1`, tag `v0.1.0-alpha.1`.
2. draftwright-rust and specify-core-rust: pin quiddity-rust to that tag, CI green, then
   version `0.1.0-alpha.1` and tag `v0.1.0-alpha.1`.
3. GitHub releases with notes, drafted for the maintainer to approve before they are published.

## 5. After alpha 1: the run stops

The maintainer's decision (2026-10-09): the rolling run ends with alpha 1. Once the gates in 1-3
are met, the dispatcher starts no new streams of any kind; streams already in flight finish and
are integrated as usual, and the run then writes its final report and stops. The releases are
cut by the main session, which then stops the workflows. draftwright-rust improvements from the
backlog resume in a later run, when the maintainer starts one.

## 6. Readiness record: draftwright-rust (section 1's gates)

Stream d-alpha1-readiness, 2026-10-10. **Checked at** `2f11694` (local main: origin/main
`a11808d` plus `5bcc741`, the plate_pin diagnosis, and `2f11694`, haecceity pinned to
quiddity-rust `694dc17`), arm64 macOS, release build. Run there:

- `cargo fmt --check`: clean. `cargo clippy --all-targets --release -- -D warnings`: clean.
- `DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test
  --release --no-fail-fast`: **57 test results, 534 passed, 4 failed, 0 ignored** (none
  skipped; peak resident 2.6 GB). The four failures are the four PMI comparisons
  (`pmi_lowering_matches_draftwright_or_is_known`, `pmi_dimensions_match_draftwright_or_are_known`,
  `pmi_coverage_matches_draftwright_or_is_known`,
  `pmi_on_the_drawing_matches_draftwright_or_is_known`),
  all on one file, `nist_stc_09_asme1_ap242-e4`: quiddity-rust `694dc17` includes `2af483f`
  (haecceity's reader models edition 4 hole occurrence definitions, `basic_round_hole`), so
  stc_09's hole occurrences are read now and the four fixtures' stc_09 entries are stale (41
  coverage, 19 lowering, 16 dimensions and 28 drawings lines of new or vanished differences, no
  other file). Re-verdicting them is stream **d-pmi-hole-occurrences-reverdict** (branched at
  `2f11694`). At origin/main `a11808d` (pin `c8a3114`) the suite is green: run 41's 538 passed,
  and CI below.
- `tools/quality.py` on `tools/quality_parts.txt` (release binary of this head): the table is
  identical, part by part and ink code by ink code, to the "after" column of
  `docs/quality-2026-10-09.md`'s "Alpha 1 gate audit" (`2425a2a`): off the sheet 0, repeated
  labels 18, repeated measurements 0, view overlaps 0, views on the title block 0, ink 283
  (`annotation_outside_area` 155, `annotation_leader_crossing` 108, `annotation_ink_overlap` 20,
  over a view 0, line over a view 0).
- `tools/plan_timing.py --runs 9` on NIST FTC 06.
- CI on main (read-only, `gh run list`/`gh run view`): the latest run, 38023949849 at `a11808d`
  (origin/main; `2f11694` is not pushed, so has no run), **green**: lint, build and the five test
  shards plus the harness job; 20.1 min wall (build 6.6, slowest shard 13.4). The seven runs
  before it on main are green too (16.1 to 24.8 min).

Fixture verdicts at `2f11694` (counted from the files; stc_09's entries stale as above):

| fixture | entries | rust-correct | equivalent | not-applicable | undetermined | rust-wrong |
|---|---|---|---|---|---|---|
| `known_pmi.json` | 286 | 283 | 0 | 0 | 3 | 0 |
| `known_pmi_lowering.json` | 244 | 208 | 17 | 1 | 18 | 0 |
| `known_pmi_dimensions.json` | 59 | 34 | 3 | 0 | 22 | 0 |
| `known_pmi_coverage.json` | 1354 | 1187 | 65 | 0 | 82 | 20 |
| `pmi_drawings/known_pmi_drawings.json` | 297 | 253 | 4 | 0 | 40 | 0 |

| Gate (section 1) | Status at `2f11694` |
|---|---|
| PMI reading, the 55 `known_pmi.json` rust-wrong settled | **met** |
| Feature control frames and datum feature symbols | **in progress**: done and verdicted; stc_09 re-verdict (d-pmi-hole-occurrences-reverdict) |
| Toleranced and basic dimensions from PMI | **in progress**: 0 rust-wrong; stc_09 re-verdict; 22 undetermined on questions 1-3, 5 |
| PMI coverage lint | **in progress**: lint landed; 20 rust-wrong (16 stc_09 hole occurrences, d-pmi-hole-occurrences-reverdict; 4 thread/knurl, d-pmi-thread-knurl-owner-callouts) |
| Checked on NIST and specify-core-written files | **in progress**: 14 fixtures, every difference verdicted at `a11808d`; stc_09 re-verdict; haecceity-written files (d-pmi-haecceity-written) |
| Annotations off the sheet 0 | **met** (30 parts) |
| Overlapping views 0, views on the title block 0 | **met** |
| Repeated labels each justified | **not met**: 18 remain; no per-repeat verdict list in the repository (below) |
| No annotation (or annotation line) over a view | **met** (30 parts) |
| No part worse than the baseline | **not met**: 5 parts, 7 cells; questions 6-9 |
| Every remaining ink problem in the README | **met** (108, 155, 20 match this head) |
| Hole tables | not a gate; listed in the README |
| Performance | **met** |
| CI green | **met** at `a11808d`; not run at `2f11694` (unpushed, and its suite is red, above) |
| README says what alpha 1 does and does not do | **not met**: present, but four statements are now false (below); refreshed at the cut |
| CHANGELOG entry | **present**; one statement now false (below); refreshed at the cut |

### PMI reading

**Met.** `known_pmi.json` had 330 differences, 55 rust-wrong, at `0ffa3fd` (stage 1); stage 1b
(`35c400a`, `519fdee`, merge `2007669`, d-pmi-manufacturing) reads manufacturing requirements,
surface and common labels as records, and the file has 286 differences since: 283 rust-correct,
3 undetermined (FTC-10's three Ø3 on supplemental geometry, question 2), 0 rust-wrong.
`tests/pmi.rs` passed at this head (8 of 8, `pmi_records_match_draftwright_or_are_known`,
`haecceity_findings_are_sources_or_record_blockers`,
`every_item_and_finding_is_accounted_for_and_the_output_is_deterministic` among them),
including stc_09 under the new pin.

### Feature control frames and datum feature symbols

**In progress.** Landed by d-pmi-lowering (merge `e39b3e4`); `known_pmi_lowering.json` 244
differences, 0 rust-wrong, 18 undetermined: 7 stc_09 hole occurrences (being re-verdicted), 8
composite_shape_aspect findings and 1 datum feature without a face on NIST CTC-02 (question 5),
2 for plate_pin's pin placed twice (question 1). At this head
`pmi_lowering_matches_draftwright_or_is_known` fails on stc_09 only (12 new, 7 vanished
differences); the other 9 tests of `tests/pmi_lowering.rs` passed, and so did
`tests/pmi_labels.rs` (6) and `tests/pmi_angular.rs` (3).

On the AP242 fixtures (outside the quality gate's 30 parts), `plan` and `render` at this head
report frames and datum symbols off the sheet or over a view: NIST CTC-02 e2 datum symbols E and
J (`pmi:datum:#75`, `#78`) off the sheet; stc_09 e4 four frames and one hole callout off the
sheet and 13 frame or dimension lines over a view (plan and side); spool_fits one datum symbol
over the side view; thumbwheel_thread_knurl one datum line over the side view. Owner: stream
**d-gdt-band-space** (GD&T band space in the layout; branch `633a370`).

### Toleranced and basic dimensions from PMI

**In progress.** Stage 2b (`src/dims/pmi_dimensions.rs`) and angular dimensions between two
planar faces (d-pmi-angular-dims, merge `adfd909`). `known_pmi_dimensions.json` 59 differences,
0 rust-wrong, 22 undetermined: 8 stc_09 hole occurrences (being re-verdicted), 7 CTC-02
composite_shape_aspect and 1 stc_06 ControlledRadius modifier (question 5), 3 FTC-10 Ø3 on
supplemental geometry (question 2), 3 FTC-10 G6 without limits (question 3).
`pmi_dimensions_match_draftwright_or_are_known` fails on stc_09 only (8 new, 8 vanished: the
hole callouts are drawn now); the other 9 tests passed. Not drawn and reported (README): oblique
lengths, a cone's angle, bolt circles whose holes differ, a radius on several faces.

### PMI coverage lint

**In progress.** Landed by d-pmi-coverage-lint and d-pmi-coverage-complete (merges `d7a2aec`,
`0bfbba3`), re-verdicted after labels (`47f2aec`), material (`d319afc`) and angles (`adfd909`):
every PMI source is on the drawing or reported. `known_pmi_coverage.json` 1354 differences,
20 rust-wrong, all the drawing's or the reader's, not the lint's:

- 16 on `nist_stc_09_asme1_ap242-e4` (`pmi_not_extracted` and `pmi_not_lowered` on 8
  dimensions, #9866, #9889, #10063, #10191, #10215, #10239, #10287, #10311): haecceity did not
  model the hole occurrence; at this head it does, and all 16 no longer differ (with 23 other
  stc_09 entries; 2 new differences on datums #8807 and #8839). Owner
  **d-pmi-hole-occurrences-reverdict**.
- 4 thread and knurl requirements not drawn (`string_post_tapped` #3157,
  `thumbwheel_thread_knurl` #689, #722, #749): draftwright prints them on the owning hole or
  turned-diameter callout, or `SEE MFG n` with a table. Owner
  **d-pmi-thread-knurl-owner-callouts** (branch `6616966`).

82 undetermined: 30 composite_shape_aspect and 2 datum-feature findings (question 5), 12
supplemental geometry (question 2), 9 ISO 286 class without limits (question 3), 4 plate_pin
placed twice (question 1), 11 omissions draftwright's build already explained, reported again by
the port's lint (question 4), 14 stc_09 hole occurrences (being re-verdicted). The other 2 tests
of `tests/pmi_coverage.rs` and the 3 of `tests/title_block_material.rs` passed.

### Checked on NIST CTC/FTC and specify-core-written files

**In progress.** `tests/pmi_drawings.rs` compares, record by record, what the port's drawing and
draftwright's state on the 14 AP242 fixtures: 7 NIST MBE models (CTC-01, CTC-02, FTC-07,
FTC-10, STC-06, STC-09, STC-10) and 7 files specify-core wrote (`tests/fixtures/pmi/README.md`).
At `a11808d` every difference has a verdict (297: 253 rust-correct, 4 equivalent, 40
undetermined, 0 rust-wrong; run 41). At this head
`pmi_on_the_drawing_matches_draftwright_or_is_known` fails on stc_09 only (13 new, 15 vanished).
Undetermined: 15 stc_09 hole occurrences (being re-verdicted), 15 composite_shape_aspect, 1
datum feature and 1 ControlledRadius (question 5), 3 supplemental geometry (question 2), 3 ISO
286 class (question 3), 2 plate_pin (question 1). Thread and knurl requirements are not in this
comparison (`tools/capture_pmi_drawings.py` records none); they are in the coverage lint's.
Files written by the new haecceity writer: stream **d-pmi-haecceity-written**. plate_pin's sheet
(A0 2:1 against draftwright's A1 1:1) is rust-wrong, owned by the first choice's bands, to follow
d-gdt-band-space (`5bcc741`), with question 10.

### Quality

Measured at this head (above); baseline `86ffcbc`.

- **Off the sheet: met**, 0 (34 at the baseline; 3 → 0 by `7b4ada1`).
- **Overlapping view pairs and views on the title block: met**, 0 and 0.
- **Repeated labels: not met.** 18 remain, the same 18 since d-quality-repeated-dims (`d744899`,
  merge `0a09c12`), whose status line says each has a verdict "in the stream's report and
  known_dimensions.json"; the report is not in the repository, and `d744899`'s
  `known_dimensions.json` entries cover cgb245 and ftc06 only. The 18 at this head (view, label,
  items):
  - counted pitches or sizes of distinct features, each stated once per feature group: ctc02
    plan `2× 75` (hole_pattern 1, 4); ctc04 plan `2× 75` (hole_pattern 1, 6), `2× 45`
    (hole_pattern 8, 10), `75` (slot 3, 5 widths), `140` ×3 (hole 18's x location, slot 4 and 6
    widths); cgb245 plan `15.0` (slot 1, 2 lengths); ftc08 plan `31.8` (slot 2, 4 lengths); ftc09
    front `2× 19.1` (hole_pattern 2, 3), `6.3` (slot 1, 2 widths), `38.1` (hole 5's x location,
    slot 3's length); ftc10 front `2× 11` ×3 (hole_pattern 5, 6, 7);
  - a feature dimension and a ladder rung or other measure on one value: ctc01 front `50`
    (polygonal_boss 1 height, step height h50); ftc10 front `12.5` (hole_pattern 7's z location,
    step height h12.5); 10060 side `19.3` (hole 1's y location, step position y19.256); ftc06
    front `266.7` (hole_pattern 1's column pitch, boss 6's x location), `127` (hole_pattern 1's
    row pitch and z location). The step-rung ones wait on question 11.

  Met when each has a written verdict against draftwright's sheet; no stream is in flight for it.
- **No annotation over a view and no annotation line over a view: met**, 0 and 0 on the 30
  parts (24 and 46 at the baseline). The AP242 fixtures' cases are under frames above.
- **No part worse than the baseline: not met.** Seven cells on five parts, unchanged since the
  audit: ctc03 ink 21 → 24 (`annotation_outside_area` 12 → 16, `annotation_leader_crossing`
  7 → 8; question 6), ftc08 leader crossings 22 → 24 (question 7), ctc02 ink overlaps 7 → 9
  (question 7), cgb245 leader crossings 0 → 2 (questions 8 and 9), ctc04 leader crossings
  27 → 29 (questions 8 and 9). Stream **d-placement-share-held-taken** (the review's point that
  unplaced stranded items are not registered in `share_held`'s `taken`) is in flight in
  placement; it is not traced to any of these cells.
- **Every remaining ink problem listed in the README with its verdict: met.** README "Known
  limitations" gives leader crossings 108, ink past a view's area 155 and ink overlaps 20, each
  with its verdict; the three counts match this head.

### Hole tables

Not a gate (maintainer, 2026-10-09). Drop-and-escalate is merged (`50be334`); README "Known
limitations" lists ftc08's and ftc09's drops (not re-measured by this stream).

### Performance

**Met.** The maintainer's PR #1 merged 2026-10-09 (`572a9b2`, integrated as `2f683a8`, SVGs
byte-identical on the 30 parts per that stream, status run 10). `tools/plan_timing.py --runs 9`
at this head, NIST FTC 06, `plan --analysis` with layout recovery: median **24.2 ms** wall on a
loaded machine (eight streams building), 20.0 ms CPU, **250 M instructions** retired; the audit
measured the same 250 M as 14.8 ms. Under 50 ms either way, from about 490 ms.

### Release hygiene

- **CI green: met** at origin/main `a11808d` (run 38023949849, 20.1 min). `2f11694` is
  unpushed; pushed as it is, its test shards would fail as the local suite does, until
  d-pmi-hole-occurrences-reverdict lands.
- **README: not met as it stands** (the main session refreshes it at the cut). False at this
  head:
  - "What alpha 1 does not do", PMI: "angular dimensions" not drawn: an angle between two planar
    faces is drawn since `adfd909` (only a cone's angle is refused).
  - Same bullet: "The PMI coverage lint (every PMI item on the drawing or reported) is in
    progress": it landed (`0bfbba3`, `47f2aec`).
  - "Quality gaps remain": "a few annotations off the sheet on the larger NIST parts": 0 on the
    30 parts since `7b4ada1`.
  - "Known limitations", "Off the sheet on the AP242 fixtures: two datum feature symbols on NIST
    CTC 02": also five on stc_09 e4 at this head (above), and frame lines over a view on stc_09,
    spool_fits and thumbwheel_thread_knurl.
  - Will become false at the next pin: "section recesses … are not in Rust quiddity's
    recognition document yet" is true at `694dc17`, but quiddity-rust `8a78314` wires them in.
- **CHANGELOG: present** (`0.1.0-alpha.1 (unreleased)`); its "Known limitations" still says "the
  PMI coverage lint is in progress".

### Questions

Open maintainer questions that hold a gate. None is answered here.

1. **A part placed several times** (assembly_plate_pin's pin): annotate one placement, or
   refuse? First recorded in `docs/next-session.md`, PMI stage 1 (d-pmi-read), "Questions for
   the maintainer". Holds 2 lowering, 4 coverage and 2 drawings entries.
2. **Semantic dimensions on supplemental geometry with only an invisible presentation**
   (FTC-10's three Ø3): draw, report or skip? Same place. Holds 3 reading, 3 dimensions, 12
   coverage and 3 drawings entries.
3. **An ISO 286 class stated without limits** (FTC-10's three Ø3.50 G6): compute the limits (a
   table to add) or draw the class alone? Same place. Holds 3 dimensions, 9 coverage and 3
   drawings entries.
4. **Lint and omissions the build explained**: record the explanation on the drawing, accept the
   double report, or drop the plan's? `docs/next-session.md`, PMI stage 3, question (1). Holds 11
   coverage entries.
5. **Records carrying a haecceity finding the reader recovered from** (CTC-02's
   composite_shape_aspect groups and datum feature #79 without a face, stc_06's ControlledRadius
   in a compound item): the port never draws a record with a reading blocker (the stage-1/2a
   policy) where draftwright draws XCAF's reading. Keep refusing until haecceity reads the
   construct, or draw the recovered reading? Not recorded as a maintainer question before: only
   in the fixtures' evidence ("whether that frame is complete needs the construct read";
   `known_pmi_lowering.json`, `known_pmi_dimensions.json`). Holds 9 lowering, 8 dimensions, 32
   coverage and 17 drawings entries.
6. **Should page recovery weigh ink or sheet size** (ctc03 taking A2 1:5, ink 21 → 24)?
   `docs/quality-2026-10-09.md`, "After Quality 4", and `docs/next-session.md` (Quality 4,
   integration r-draftwright-rust-10).
7. **How a text contact weighs against a leader crossing** (ftc08, ctc02; draftwright gates text
   first)? `docs/quality-2026-10-09.md`, "After Quality 4".
8. **The plan's Y locations above the side view**, as draftwright puts them (cgb245, ctc04)?
   `docs/next-session.md`, Lanes (d-quality-lanes), question (2).
9. **Keep merged-away lanes, or revert `29575ec` and `b81a577`** (restores ink 279; cgb245,
   ctc04)? `docs/next-session.md`, Merged-away lanes; status run 23.
10. **Reserve the plan bands for PMI dimensions by draftwright's count or by the port's lanes**
    (plate_pin A1 1:1)? `docs/next-session.md`, plate_pin diagnosis (d-plate-pin-recovery).
11. **A step-height rung and a feature dimension on the same span: merge, and which owns it**
    (ftc06 h38.1, ftc10 h12.5; here also ctc01 `50`, 10060 `19.3`)? `docs/status-2026-10-09.md`,
    integration r-draftwright-rust-7.
