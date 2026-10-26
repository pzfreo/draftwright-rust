# Changelog

All notable changes to draftwright-rust are recorded here, in the style of
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### Changed

- On quiddity-rust **v0.2.0**. Its face accounting (blended box corners, plate edges, stock
  walls, drill-point tips, slot end walls) raises claimed faces on 65 corpus parts: CTC-01 AP242
  goes from 27 unclaimed faces to 3, and the claim losses listed for 0.1.0 are recovered. Newly
  recognised steps change the layout of 10033 (now A3 2:1, with one leader crossing a dimension
  line), 10127 and holdout 119. FTC-10 rb gains 3 and FTC-07 AP242 2 extension lines through
  features, and the FTC-10 AP242 drawing one leader through text. The three new families
  (annular grooves, V channels, semicircular blind slots) are read but not yet drawn; they are
  reported as `features_not_recognised`.

### Fixed

- **Coincident copies of a body no longer multiply every count (#212).** A STEP holding the same
  solid several times on top of itself (a FreeCAD export of a body and its `Pad`) was drawn
  whole, each feature once per copy: dw-examples' gantry plate printed "24× ⌀5.1 THRU" for 12
  holes and went to A1 5:1. When every body of an assembly coincides with the first (the same
  faces, each of the same surface type and box, as placed), that part is drawn alone, as
  `--part K` draws it: the gantry plate is A3 2:1 with its 12 holes, and the V-slot profile's
  one hole is no longer "3× ⌀4.2 THRU". The copies are reported as `coincident_bodies` (info);
  where only some bodies coincide the file is drawn whole as before, and `coincident_bodies`
  warns that their features are counted again.
- **PMI from inch-authored files prints as millimetres an mm sheet can state (#214).** A length is
  converted from its stated digits less the writer's float noise (at most six decimals in its own
  unit, more for a larger unit such as the metre, the product to three more), so NIST STC-08's
  frames print `1.524` and `0.381`, not `1.524000000006`; a PMI dimension authored in inches
  prints its exact conversion (`6.35 ±0.254`, not `0.25 ±0.01 INCH`, STC-09); a zero deviation is
  `0`, unsigned (`⌀6.35 +0.0762 0`, ISO 129-1, STC-07 and STC-09); and a common label that states
  a radius without its value (`2X R`, `4X R`: ASME's full radius) is not drawn but reported as
  `pmi_not_lowered` (a warning).

## 0.1.0 (2026-10-25)

The first release without an alpha tag. It is still on quiddity-rust **v0.2.0-alpha.2**, as in
alpha.14. Since alpha.14: dimension lanes are closed up, views keep a 3 mm gutter inside the
frame, more feature callouts sit inside the views, radius leaders prefer the middle of their arc
and a radial approach, only datums that something uses are drawn, and face claims follow quiddity's face
accounting. Drawings still report what they do not state; dense parts are not yet as clean as
Python draftwright's (see **Known regressions** and the README's "Known limitations").

**Speed.** In instructions retired, as measured in the pull requests and not re-measured for this
release: the interior-callout pass (#208) makes each edit cost about 10% more; the frame gutter
(#205), against the build before it, is 0.89× on CTC-01, 1.002× on string_post, 1.012× on
FTC-08 and 1.048× on cgb245.

### Known regressions

Measured on this release; the alpha.12 figures are the reference from before the move to
quiddity-rust's framed records.

- STC-06 AP242: A2 1:5 (alpha.12: A0 1:1).
- FTC-06: A2 1:5 (alpha.12: A1 1:2).
- FTC-10 rb: two A2 sheets (alpha.12: one A1).
- 10119: A4 1:1 (alpha.12: 2:1). Closing up the lanes restored 2:1; the frame gutter took it back.
- CTC-01 merge: 23 warnings (alpha.12: 21; alpha.14: 30).
- FTC-07 AP242: its 4 polygonal bosses are not located (12 unlocated features), extension lines
  through features are 26 (alpha.12: 14), and it is now on A2 1:2 (alpha.14: A1 1:1) with
  40 leader crossings (alpha.14: 33).
- cgb241: `2× R23` is still led from the fillet: 71 mm with three hidden-line crossings, where
  alpha.12's leader from the blend was 37 mm with none.
- cgb217: quiddity's frame vote still puts it at 45° (#81), leaving 15 undimensioned features
  (alpha.13: 7).
- string_post: one random-pin cell (seed 1, 8 pins) puts the hole centreline on the title
  block's scale note (#206), listed in `tools/known-pin-failures.json`.
- 58 of the 94 corpus parts claim 1–6 fewer faces (CTC-01, FTC-06 and string_post among them):
  the envelope planes quiddity marks `unexplained` are no longer claimed by draftwright's own
  stock rule, until quiddity-rust#100 adds body evidence.

### Changed

- **Dimension lanes are closed up after layout.** Dimensions that do not overlap now share a lane
  whatever their kind, instead of each taking its own. After layout each dimension line, inner
  first, moves into the innermost lane where nothing already there comes within a clearance of
  it and none of its own findings grows; a line stays outside any dimension it overlaps that
  lies inside it, so sizes stay nearest and a baseline keeps its order. Authored, pinned and
  interior dimensions stay put, omitting one dimension moves no other, and an item promoted
  into an omitted item's lane follows it. NIST CTC-01's plan band goes from 8 lanes to 5. On the
  100-part corpus no finding changes.
- **A 3 mm gutter inside the frame.** Views and their annotation keep at least 3 mm clear of the
  frame (maintainer, 2026-10-25); `sheet/frame_gutter` sets another width, 0–20 mm. Tables,
  notes, the title block and other sheet furniture still stand against the frame. When there is
  spare paper, each sheet's composition moves away from the frame as one block, up to 10 mm
  more, after the gaps between views have had their share. A strip of dimensions that would
  overflow is first packed, nesting lines that overlap, which keeps NIST CTC-01's baseline whole
  on A3 1:5. On the 100-part corpus, 91 parts are
  unchanged; seven synthetic mfcadpp parts change paper or scale (four one step smaller on A4,
  three from A4 1:1 to A3 2:1), and NIST CTC-04 loses its two leader crossings and five
  extension lines through features. Off the corpus, FTC-07 AP242 moves from A1 1:1 to A2 1:2
  (its leaders through features fall from 10 to 4, but extension lines through features rise
  from 23 to 26 and leader crossings from 33 to 40), and the CTC-01 merge fixture falls from 30
  warnings to 23.
- **Only used datums are drawn by default (revising #141).** A declared datum's symbol is drawn
  when a control frame cites it or dimensions measure from it (the planar datum selected as an
  axis's origin, as for Specify ± mode origins). Any other declared datum is not drawn; it stays
  in the plan as `not_drawable` and is reported as `datum_not_drawn` (info) (maintainer,
  2026-10-25). `sheet/uncited_datums: true` draws every declared datum; `false` hides every
  datum no frame cites, origins included. The layout's room for datum symbols now follows the
  Spec's datum visibility, not only edited wording. Among the PMI fixtures only GRM-03 changes:
  its cylindrical datum A, which nothing cites, is reported instead of drawn.
- **More feature callouts inside the views, and radius leaders that point at the arc's middle.**
  An automatic feature callout drawn outside its view moves inside when its whole label sits on
  blank face or in an opening the part surrounds (a slot, the gap between lugs), its leader is
  no longer and none of its findings grows (maintainer, 2026-10-25). A leader leaving its tip may
  cross outlines parallel to the tip's own edge, as around a chamfered hex's flats, which any
  route must cross; NIST CTC-01's HEX 100 A/F now sits on the plate beside the hex. Radius
  leaders, slot ends included, prefer the middle quarter of the arc and within 7.5° of radial,
  taking a longer clear route to get there: CTC-01's 4× R25 now meets its arc radially (its
  radius_leader_not_radial finding goes) and 7× R50 points at the middle of a lug corner. The
  pass runs on the selected drawing only; when the Spec omits items, the moves come from the same
  request without them, so an omission moves nothing else. A radius route may not run into more
  extension lines than the one drawn.

### Fixed

- **Unclaimed faces follow quiddity's face accounting (#207).** A face quiddity accounts to any
  record, evidence records such as `step_levels` included, is claimed, and its `body` faces are
  credited once the overall sizes are stated; the envelope-plane stock rule now applies only to
  analyses without that accounting. On NIST CTC-01 (AP242) the end blocks' inner walls, the notch
  side walls and slot 1's end walls are no longer reported, while the envelope planes quiddity
  leaves unexplained, the hex boss's top among them, now are (26 unclaimed faces become 27)
  until quiddity-rust#100 supplies body evidence. Across the 94-part corpus, 27 parts claim more
  faces (cgb245 +274, CTC-02 rc +51, sm-hanger +44), 58 claim 1–6 fewer and 9 are unchanged.
- **Sheet choice.** A part whose views fit A3 at 1:1 is no longer offered A2 or larger by the
  paper search either, as the automatic planner already ruled (maintainer, 2026-10-15). A refused
  automatic drawing gets one smaller readable scale on its own paper before larger paper, and a
  trial that dropped items for want of room counts as cramped, so it too tries the smaller scale
  (cadgenbench cgb245 stays on A3 1:5).

## 0.1.0-alpha.14 (2026-10-25)

Now on quiddity-rust **v0.2.0-alpha.2** (previously `0.1.1`), which publishes recognition schema 6.
It picks up quiddity-rust's fixes for #81, #83, #84, #85 and #89 and its decision on #82, and
recovers most of the regressions accepted in alpha.13. Those still open are listed under **Known regressions** below.

### quiddity-rust v0.2.0-alpha.2

- **Recognition schema 6** (#92). Records quiddity measures in a part's own frame now carry an
  `origin` and measuring axes (a file-axis letter or a unit vector in file coordinates), with
  distances measured along those axes. draftwright reads them back into file coordinates.
  Oriented slots, chamfers and slot patterns now arrive in `slots`, `chamfers` and
  `slot_patterns`; draftwright still checks and draws them as oriented features, and their
  draftwright IDs (`f:oriented_slot:1`, …) are unchanged. A record still in the part's frame is left out with a reason, never
  drawn in the wrong place. Schema 5 and earlier are refused.
- **Recovered from alpha.13's known regressions.**
  - CTC-02 AP242: the axis-aligned frame is back (#81). It has no undimensioned features
    (previously 15), and `10× R25` is drawn again.
  - FTC-07 AP242: A1 1:1 again (previously A2 1:2), with 4 undimensioned features
    (previously 25). quiddity now also finds its 4 polygonal bosses, which are not yet located
    (see Known regressions).
  - The countersink-two-face fixture's hole links to its seat again (#83).
  - 1000.step's plate top is on the grid (#84), so its 4.219 has no rounding advisory.
  - whistle_frame_reference's plate is 7.0 thick again (#85), and it is back on alpha.12's
    single A2 2:1 sheet.
  - cgb202: the false fillets are gone (#89). 91 warnings and 1 error fall to 50 warnings and no
    errors, with no undimensioned features (previously 42).
  - cgb207: `2× R5` and "ALL UNSPECIFIED RADII" are drawn again, and R5 is no longer
    undimensioned.
- **Settled as correct.** CTC-02 rc's 49.5 mm array stays unpublished: quiddity found two
  mirror-image rows with equal claim and publishes neither (#82). Every hole is still drawn and
  located.
- **Other drawing changes** (#89): a few mfcadpp parts lose one radius callout, as quiddity no
  longer reads a concave round as a fillet. mfcadpp 10033 moves from A3 2:1 to A4 1:1, and 10163
  from A4 1:2 to A4 1:1.
- **Speed.** Unchanged: planning CTC-02 rc costs 1.0003× main's instructions, FTC-06 0.993×.

### Known regressions

- STC-06 AP242: A2 1:5 (alpha.12: A0 1:1).
- FTC-06: A2 1:5 (alpha.12: A1 1:2).
- FTC-10 rb: two A2 sheets (alpha.12: one A1).
- 10119: A4 1:1 (alpha.12: 2:1).
- CTC-01 merge: 30 warnings (alpha.12: 21).
- FTC-07 AP242: its 4 newly recognised polygonal bosses are not located (12 unlocated features,
  previously 4), and extension lines through features rise from 14 to 23.
- cgb241: quiddity now calls one of its two mirrored R23 rounds a fillet. The sheet stays A3 1:5,
  but `2× R23` is led from the fillet: 71 mm with three hidden-line crossings, where main's
  leader from the blend was 37 mm with none.
- cgb217: quiddity's frame vote moves it from 13.1° to 45° (#81), leaving 15 undimensioned
  features (alpha.13: 7). Its ø60 band is still read as a boss, not a turned step.

### Added

- **Outer-profile corners are defined, angled only where needed** (LINT-angular-profile_angle_requirement,
  now ported). A non-right convex outer-profile corner is satisfied by its angle or by linear
  dimensions that fix both its edges (ISO 129-1, ASME Y14.5); lint reads those dimensions from the
  drawn ink, so a corner fixed linearly is no longer reported. The planner adds no angle on a
  linearly fixed corner, gives any other corner its angle in the view that shows it true
  (`d:envelope/profile_angle.angle.f<face>_e<i>_e<j>`), gives a closed polygon of equal corners
  one counted angle (`8× 135°`) and two mirror-equal corners one `2× θ°`; lint proves both counts
  from the drawn edges. An angle is placed only local to its corner (arc and label within twelve
  label heights of it, label on its arc) and clear of holes and every other item; otherwise it
  is dropped (`angular_dimension_dropped`, info) and its corner stays reported. Placement never
  moves another item or changes the sheet, and the 30-part quality metrics are unchanged. NIST
  FTC-08 (`8× 135°`), CTC-03 (`2× 135°`) and cadgenbench cgb207 now have no
  `angular_requirement_*` finding; CTC-04 and the mfcadpp parts keep theirs.
- `faces/<face>/profile_corners` (`auto`, `angle`, `linear`) chooses how a face's outer-profile
  corners are defined (`linear` locates the missing stations,
  `d:envelope/profile_location_<axis>.length.f<face>_<n>`); `dimensions/<id>` `{}` shows and `omit` hides one corner's angle.
- **Opening a saved Spec across kernel upgrades** (#191). `Session::open_spec(json)` applies what
  the part still has and sets aside each entry naming a feature, face, dimension, annotation, view
  or measurement it no longer has, with the entry and a reason code (`no_feature`, `no_face`,
  `no_dimension`, `no_measurement`, `no_annotation`, `no_view`). A saved face anchor the part no
  longer has is set aside too. A gone member of an authored group is taken out of the group alone
  (`features/<group>/pattern/members/<i>`) while the rest is still a group, else the whole group
  is set aside. Reason codes depend on what the part has, not on the order of refusals. It reads
  `sheet/preferred_printer` as `preferred_max_size`. Any other refusal still refuses the whole
  Spec. All the stale entries one check finds are set aside together, so many cost about one
  plan.
- `api::Error` and `model::Invalid` gain `path`: the escaped Spec entry
  (`dimensions/d:f:hole:1~1bore.diameter`) of every "there is no …" refusal. Messages and choices
  are unchanged.
- `api::spec_ops(before, after)`: the `set`/`remove` ops that turn one Spec into another, as the
  MCP's `spec_diff` makes them (a new empty object is set). An explicit `null` in *after* removes
  the key, where the editor's `specOperations` sets `null`. `Session::set_spec` replaces the Spec
  as the WASM `setSpec` did.

### Changed

- **One owner for through-step legs and blind depths** (ADR1-8, still pending). A through step's
  leg now has support planes and yields to the plate thickness, face-level rung or overall extent
  that states it (lint already accepts those as the leg's statement); the plate, rung and overall
  stay controlling. A circular blind step's, hexagonal pocket's, rectangular blind slot's and
  plain flat-bottomed blind hole's depth now counts as the interval its callout states: the
  callout never yields, so a rung it states or closes is conveyed by it or shown as reference,
  and a boss height that a hole's depth states (a boss on the hole's floor) is conveyed by the
  hole callout. A tolerance keeps any of these controlling; `sheet/references` shows a yielded one
  bracketed. On the corpus, 4 legs (3 parts), 10 rungs (8 parts) and 32 boss heights (23 parts)
  leave mfcadpp drawings; NIST, gramel and cadgenbench drawings are unchanged and there are no
  new lint findings. Lint no longer reports two holes' callouts or schedule rows of equal depth
  as one measurement stated twice, and a pinned item that its canonical plan conveys by another
  is pinned in its own view (it was given its owner's, which could refuse the spec).
  Hole point ordinates and rebased ladder datums remain.
- **Leader and witness targets.** Lint checks flat, recess and slot callout tips against the
  faces their records publish, never a pocket's open mouth or a seat's chord
  (`flat_`, `recess_` and `slot_leader_target_*`), and a turned length's extension lines against
  its band's shoulders (`step_length_witness_target_*`). A chamfer, turned diameter or turned
  length whose leader or witness misses its feature no longer earns coverage credit.
- **Obround pockets.** Their leaders no longer aim at the box corners beyond the end arcs; 20
  MFCAD pocket callouts move onto the straight walls.
- **Thread depth lint.** A thread recognised from modelled geometry no longer has to state a
  depth: its trimmed span includes partial end turns and runouts, so it does not prove one, and
  its extent is shown by the ISO 6410 depiction. A depth from a Spec thread length or an AP242
  minimum full thread is still required. cgb243's three `thread.depth` warnings go away.
- **Rules catalogue.** Four Python clauses are retired rather than ported, with the maintainer's
  approval: ADR4-U1's angular sets (Rust draws no angular pattern locations, so no two automatic
  angles share a vertex), ISSUE-1505-b's off-page Z shoulder recovery (Rust places those
  shoulders on the main view) and solid Boolean cut caps (the detail clips exact projected
  intervals), and the modelled-thread half of LINT-hole_coverage-thread_and_profile. The three
  rules are marked ported, which leaves three pending.

### Fixed

- **Pin replay on multi-sheet plans** (#170). Pins replay in the unpinned drawing's frames, and
  later sheets keep its gutters and incumbent callout routes. FTC-10's five and CTC-03's one
  failing random-pin cells, accepted in alpha.13, now pass; `tools/known-pin-failures.json` is
  empty.

- **A fillet callout joined by equal blends is placed at its final size.** When quiddity calls
  one of two equal rounds a fillet and the other a blend, the blend is still joined to the
  fillet's callout after layout (`2× R23`), but the fillet callout now carries that count from the
  start, so its leader is placed with the label it ends with. A label that grew after placement
  could run past its view's area and push the sheet up: cgb241 went to A0 1:2 during the move to
  quiddity-rust 0.2.0-alpha.2 and is A3 1:5 again. Overridden callouts are joined as before.

- **Slot location lint agrees with the drawn end.** Lint judged a slot's `location_slot.length` at
  quiddity's recorded `lo`, while the planner draws the low end at the exact centre of the slot's
  faces less half its length. Each is rounded to 0.01 mm separately, so on quiddity-rust 0.1.1 they
  fall either side of a 0.005 mm tie: cgb245 drew 285.965 and 79.075 against a recorded 285.96
  and 79.07, and both slots' locations were reported unverifiable. Lint now reads the end the
  planner draws. cgb245 has no slot findings again, and CTC-03's first slot location (112.547) is
  no longer reported unverifiable.
- **Identical slots share one width** (CTC-03's repeated label, accepted in alpha.13). Lone slots
  in one view with the same width, length and end radius, run the same way, share one counted
  width wherever they lie (`2× 19.05`); widths spanning one stretch still share as before.
  CTC-03's two slots, which alpha.12 read as 19.04/19.05 wide, now read the same 19.05 and were
  stated twice: repeated_label 1 → 0. Their length was already shared across its one span
  (`2× 107.95`), and each slot keeps its own `2× R9.52`. CTC-04 now reads `2× 75` and
  `2× 140` (repeated_label 2 → 0, line_through_feature 22 → 19; the `2× 75` label sits beside
  its 15 mm span, text_off_span 0 → 1, accepted) and FTC-09 `2× 6.35` (repeated_label 1 → 0,
  line_through_feature 2 → 0). Sharing lengths and end radii of
  identical slots on different spans too was tried and left out: it moved CTC-03's radius
  leader onto an extension line and put FTC-08 on A1 at 1:1 with three extensions through a
  slot.
- **CTC-01's radius-centre table: correct as it is** (listed as a known regression in alpha.13).
  alpha.12 tabulated the 4× R25 corners only because the 350 row pitch was witnessed from the
  grid's left-hand column and its extension lines crossed the 4× R25 leader (two
  `annotation_leader_crossing` findings). The pitch is witnessed from the grid's first member,
  which quiddity-rust lists in the right-hand column, so its extension lines are short and
  cross nothing, and the leader is not obstructed. The 4× R25 callout stays; it meets its arc off
  radial, as the counted leaders on CTC-02 and FTC-09 do, which are kept by design. The pitch's
  station still follows quiddity's member order.
- **Manufacturing table with no room.** When the thread and knurl requirements table has no place
  on the sheet (only reachable with an explicit `sheet/size` and `sheet/scale`), the callouts now
  state each requirement in full instead of pointing with `SEE MFG n` to a table that is not drawn,
  as draftwright does. The `table_not_placed` error goes away with the table, and the plan keeps
  the requested spec; an info finding `manufacturing_table_fallback` names the requirements printed
  in full. Drawings with room for the table are unchanged (ISSUE-1298-a).

### WASM API

- `Engine.openSpec(json)` returns `{spec, set_aside: [{path, entry, reason_code}]}` (#191).
- `specOps(before, after)`, a module function, returns the ops JSON (#191).
- Refusals carry an additive `path` for an entry the part does not have.

## 0.1.0-alpha.13 (2026-10-23)

Now on quiddity-rust **v0.1.1** (previously `0.1.0-alpha.8`). This release picks up quiddity-rust's fixes
for #69, #70, #71, #75 and #76, and its improvement to #72. Some drawings get worse, and those regressions are
accepted for this alpha (listed under **Known regressions** below). Fixes are planned for alpha.14,
on quiddity 0.1.2 and in draftwright.

### quiddity-rust v0.1.1

- **Grid and frames.** Framed positions are on the 0.001 mm grid (#69) and near-axis frames are
  treated as axis-aligned (#70), so draftwright's origin-relative re-rounding and near-axis remap
  are removed. A sub-floor snap stays for values quiddity still publishes with frame round-off.
- **Plates, CTC-04 and CTC-05.**
  - GRM-03's plate is 2.5 thick again, not 2.25 (#71).
  - CTC-04's 12 false countersinks, which were pocket-rim chamfer corners, are gone (#76).
  - CTC-05's shaft outline is drawn visible (#75).
- **Framed records restored to file axes.** Records on frames that are only partly aligned have
  their turned-step ends, turned-profile axes and flat axis lines mapped back to file axes.
  threaded_connector_109 keeps its 7 turned steps, 3 grooves and flat, and stays A3 1:5.
- **Speed.** Planning CTC-02 rc costs 1.13× main's instructions; FTC-06 is unchanged.
- **WASM size.** The raw WASM grows by 622 KB, to 17.34 MB. The size budget is re-baselined on
  that build, with a ceiling of 19.1 MB.

### Known regressions (fix in alpha.14)

- **Frame inference rotates axis-aligned parts** (quiddity-rust #81).
  - CTC-02 AP242: +15 undimensioned features, and "10× R25" is lost.
  - FTC-07 AP242: A1 1:1 → A2 1:2, +21 undimensioned features.
- **Layouts that change with the move to framed records.**
  - FTC-06: A1 1:2 → A2 1:5.
  - FTC-10: 1×A1 → 2×A2.
  - STC-06 AP242: A0 1:1 → A2 1:5.
  - 10119: A4 2:1 → 1:1.
  - CTC-01 rd: the radius-centre table is no longer planned, and 4× R25 is a placed callout.
  - CTC-01 merge: 21 → 30 warnings.
- **Other changes in quiddity's output.**
  - CTC-02 rc: the 49.5 hole array is not found (#82).
  - A countersunk hole loses its seat on the countersink-two-face fixture (#83).
  - 1000.step: the plate's top is off the grid (#84).
  - whistle_frame_reference: the plate is 7.3 thick, not 7.0 (#85).
  - cgb217: its ø60 band is read as a boss, not a turned step.
- **Draftwright's handling of framed records.**
  - cgb202: 16 fillets are read as blends (51 → 86 warnings, 1 error).
  - cgb207: "ALL UNSPECIFIED RADII" is not planned, and R5 is undimensioned.
  - cgb245: slot ends round the other way on a 0.005 tie.
- **Random-pin acceptance.**
  - FTC-10 fails five pin cells, because pin replay on multi-sheet plans re-packs the views
    (draftwright #170).
  - CTC-03 fails one pin cell.
  - Both are listed in `tools/known-pin-failures.json`; every other part and cell must pass.

### Added

- **ISO 5457 sheet furniture on every sheet.** Grid zones (numbers along the top and bottom,
  letters down the sides, skipping I and O) are now drawn on single-sheet drawings too, not
  only on multi-sheet ones. Each sheet also has centring marks: 0.7 mm lines at the midpoint of
  each side, from the trimmed edge to 5 mm inside the frame. A mark stops at the frame where
  it would meet the title block, a table or other drawn ink.
  - Everything is in the margin outside the frame (apart from the short mark ends), so sheet
    size, scale, view placement and annotations do not change.
  - `sheet/zones: false` omits the zones and centring marks. Zones that section
    cross-references cite (`SHEET n, ZONE …`) stay, with one `sheet_zones_kept` info finding.
- **Thread depth lint.** A modelled internal thread shorter than the hole it is cut in must
  state its depth: lint reports `hole_requirement_missing` for `thread.depth` when the callout
  claiming the thread gives no `↧` depth. cgb243's M14×1.5 and two M62×1.5 threads are reported.
  The callout does not print the depth yet: on cgb243 that makes the leader search draw two
  bolt-circle labels on top of each other.

### Changed

- **The title block sits flush with the frame.** Following ISO 7200, its right and bottom edges
  are now the frame line on every sheet, instead of 1 mm inside it, which drew a doubled line
  with a thin gap. Those two sides are drawn once, as the frame's line. Its size and contents
  are unchanged; the views reserve exactly its area, and the scale note, projection symbol and
  default Ra symbol move with it.
- The Python-rules catalogue (`docs/rules`) applies the 2026-10-22 evaluation of the 30 pending
  rules: 2 met in full and 14 ported for what Rust implements, each naming the clauses retired
  and why; `leader_shaft_unverified` retired; ARCH0020-1 left pending on
  quiddity (quiddity-rust #69–#72); stale notes corrected. 11 rules remain pending (ISSUE-1298-b was since ported with document PMI on the part's solid, and ISSUE-958-a with the two-channels fixture). Two new
  tests check that a fillet radius leader moved off its arc is reported, and that omitting a
  threaded hole's callout reports its thread rather than losing it.
- **Profile-angle lint credits determined corners.** Two kinds of outer-profile corner no
  longer report `angular_requirement_missing`, because the drawing already fixes them:
  - the chamfered cap of a regular polygonal boss, whose sides stay parallel to the boss's
    flats, so the `HEX … A/F` callout states its angles (NIST CTC-01's two 120° corners);
  - a corner between a principal line and the common tangent of two arcs centred on located
    hole or boss axes (GRM-04's two 96° taper corners between R3 and R4).
  Real gaps still report: the corners on cgb202, cgb207, CTC-03, CTC-04 and FTC-08 have no
  stated size or angle on the drawing. Drawings are unchanged.
- **ISSUE-958-a ported.** The forward-order two-channels fixture held only one occurrence of
  the channelled block, so it reported one channel (reported upstream as quiddity-rust #73, which
  was the fixture, not recognition). It is replaced with quiddity-rust's
  `two_channels_ordered.step`, and both orders now report two channels.
- **Ruled-out sheet requests say why.** A paper and sheet-count request that cannot fit at any
  readable scale was already skipped without planning. Its candidate now carries
  `kind: "ruled_out"` and an outcome giving the first condition that fails, in paper mm against
  the drawable area (for example FTC-09: `A4 up to 1: ruled out: view v:front with its annotation
  bands needs at least 305 × 217 mm at 1:2; A4 drawable area is 277 × 190 mm; …`). The same
  requests are skipped as before.

### Fixed

- **Holes and bosses on inclined axes get a callout** instead of none (defect E2). The callout
  goes in the view that sees the feature most nearly end-on, where it shows as an ellipse. The
  leader lands on an end of the ellipse's major axis, on the rim nearer the viewer.
  - Every view that sees it as an ellipse can carry it, so `dimensions/<id>/view` can move it,
    and a part whose best view is not drawn uses the next one. On an exact 45° tie, x is taken
    before y before z.
  - Inclined holes and bosses are still not located, and a boss's height is still not stated:
    those need a view square to the axis. Each reports this as `feature_not_dimensioned`.
    Inclined bosses of one diameter are counted once only along one line.
  - A plan-view hole table leaves inclined holes as callouts. Its rows give a direction along
    z and a plan location, and an inclined hole has neither.
  - nist_ctc_03, nist_ctc_04, nist_ftc_07 and cgb217: no hole_requirement_missing or diameter
    feature_not_dimensioned findings remain (22 → 15, 43 → 34, 16 → 12 and 19 → 9 lint
    findings), on the same paper as before.
- **One owner per measurement** in four more cases (ADR1-8, ADR4-U1, ISSUE-1505-b).
  - A recess location on a stated face level is stated once, by the level's rung, even when the
    two print their value to different precisions (ftc09: `128.100` and `128.1` → `128.100`).
  - Equal sizes over one span are counted together when recognition reads their ends a few
    micrometres apart (cgb202: `255` and `3× 255` → `4× 255`).
  - A polygonal boss's height on a stated level is conveyed by that level and the overall, as a
    round boss's is (ctc01: `50` → covered by the `100` rung and `150` overall).
  - A closed chain leaves its longest member to the total, never a short shoulder, and lint
    accepts a length the stated chain determines (cgb217: the 1 mm shoulder is drawn and the
    `45` is covered; `axial_length_missing` 1 → 0).
- **Document PMI on the part's own solid.** A general tolerance or default finish presented on
  the part's solid, not on a face, is a requirement of the whole part. GRM-03's ISO 2768-m now
  fills the GEN. TOL. cell and its Ra 3.2 is drawn beside the title block. Before, both were
  `pmi_not_extracted` errors and the sheet reported `general_tolerance_missing`.
- **Threads and knurls on bosses.** A Spec or AP242 thread or knurl on a boss prints on the
  boss's diameter callout, as on a turned step: an external thread's designation replaces the
  diameter (`M31.75 x 1.5`), and the boss keeps a callout of its own. Before, it was a
  `pmi_not_lowered` error worded with internal names.
- **Leader tips on flat-cut bands, chamfers and fillets** land on the feature's own drawn
  face.
  - A turned diameter on a band that flats cut is read first in the profile view where the
    band's outline is its cylinder. Elsewhere it leads to the edge of the trimmed cylinder on the
    flat, not to the full circle in air (string_post's ø3.4, ctc05's ø558.8).
  - A turned chamfer or fillet turns about the coaxial band that holds it, not only about the
    axis of a part turned whole. ctc05's 4× C6.35 no longer tips at the shaft's axis.
  - A fillet callout whose tip would miss every arc its view sees end-on now leads to one of
    its members' arcs, never a straight edge (ftc10's 41× R1). Its shaft may still approach
    more than 15° off radial, so `radius_leader_not_radial` remains on ftc10 (E4 stays open).
  - Lint checks turned-diameter and chamfer leader tips against those faces
    (`step_diameter_leader_target_mismatch`, `chamfer_leader_target_mismatch`). Each has an
    `…_unverifiable` info when the view cannot show the target, or when the tip misses the
    members it can measure while others in the callout cannot be measured.
- **Oblique radius exceptions no longer withdraw the general radius note.** A blend on an
  oblique axis has no arc in any principal view, so it gets no leader of its own. When a fillet
  leader already states the same radius, its count now includes the blend, so the radius is
  stated and `ALL UNSPECIFIED RADII` is placed instead of a `notes_not_placed` error (NIST
  CTC-03, FTC-07; cgb207 loses this on quiddity v0.1.1, see Known regressions). With no equal leader the note is still withdrawn, as an error.
- **Hexagonal stock states its overall extents.** The envelope lint now credits a stock's
  length with the extent along the bar, and its `HEX … A/F` callout with the extents across
  it that the regular section reaches, whatever its roll. The three false
  `missing_principal_dimension` warnings on the stock bars (roll17 among them) are gone.

### Performance

- **Late callout repair.** Routes the repair must refuse on geometry or on their first
  measurement are skipped before they are measured, and repeated measurements are computed
  once. Planning the quality set costs about 25% fewer instructions, and every plan and sheet
  SVG is byte-identical.

### Development

- CI: quality-pins measures each part once per cell and gates in one job; the acceptance jobs
  start from a separate binary build; test shards are balanced by recorded duration. A main run
  takes about 22 minutes, down from 32. A push to main that changes only release metadata runs
  lint alone when CI passed on its parent.

## 0.1.0-alpha.12 (2026-10-22)

Still on quiddity-rust `0.1.0-alpha.8`. The move to quiddity-rust 0.1.0 is waiting on
quiddity-rust #69–#72.

### Added

- **Compact dimension spacing.** By default, the first dimension line stays 10 mm from the
  outline and later lines stay 7 mm apart. When a one-sheet layout runs out of room, the same
  paper and scale are tried with compact spacing before a smaller scale, larger paper or more
  sheets.
  - Compact spacing puts the first line 7 mm from the outline, then 5.5 mm between lines. That
    leaves 1.5 mm clear above each label and clears the 2 mm extension overshoot.
  - The retry runs only when compact lanes could make the room: a dimension dropped for space,
    a page-fit verdict, dimension ink off the sheet or over the title block, or clipping that
    compact lanes could cover (no view outline larger than the frame). It does not run for
    overlaps or unrelated furniture.
  - The usable test (overlaps, leaders, legibility, lost information) still decides whether a
    compact layout is accepted. Splits onto more sheets keep normal spacing.
  - `sheet/dimension_spacing: normal | compact` states the intent: `normal` forbids the
    fallback, and `compact` uses compact spacing for every candidate, including an explicit
    paper and scale.
  - The plan records `sheet.dimension_spacing: "compact"` when compact spacing is used (absent
    means normal), and the sheet choice text adds "with compact dimension spacing". The Spec
    is not changed.
  - A local fix offered on a compact drawing keeps `sheet/dimension_spacing: compact` along
    with its paper, scale and count. An explicit `sheet/dimension_spacing` edit releases an
    inherited `sheet/keep_layout` hold, as paper, scale and sheet-count edits do.
  - Drawings that change: mfcadpp 1013 goes from A3 2:1 to A4 1:1, with the same problem
    counts. The prismatic test fixture without its isometric stays at A4 2:1 instead of
    dropping to 1:1. No other part in the 63-part comparison (quality set, NIST CTC/FTC, NIST
    STC 06–10, 40 mfcadpp parts) changes (#163).

### WASM API

- **Spec:** `sheet/dimension_spacing` (`"normal"` or `"compact"`) is added.
- **Plan:** `sheet.dimension_spacing` is added; it appears only as `"compact"`.
- **MCP:** `sheet.dimension_spacing` (`normal | compact`) is added to the MCP Spec schemas.
- **Options:** the sheet item's settable fields list `sheet/dimension_spacing` with choices
  `normal` and `compact`.
- Method signatures are unchanged.

### Performance

Ratio of retired CPU instructions for `draftwright plan`, alpha.12 ÷ alpha.11, from
interleaved runs on PR #167 (the only change since alpha.11):

| Part | Ratio |
|---|---|
| NIST CTC-01 | 1.00 |
| NIST CTC-02 | 1.00 |
| NIST CTC-04 | 1.00 |
| NIST FTC-06 | 1.00 |
| NIST FTC-09 | 1.01 |
| cgb243 | 1.00 |
| cgb245 | 1.06 |
| string_post | 1.00 |
| MFCAD++ part 1000 | 1.00 |
| MFCAD++ part 10163 | 1.00 |
| MFCAD++ part 1013 (changed drawing) | 1.35 |

- cgb245 makes one compact retry on A4, which fails.
- 1013 pays for the trials that end on A4 compact.

## 0.1.0-alpha.11 (2026-10-21)

### Added

- **Preferred sheet size and count.** `sheet/preferred_max_size` (A4–A0, default A3) and
  `sheet/preferred_max_sheets` (integer ≥ 1, default 2) steer automatic paper choice:
  - The search tries one sheet from A4 up to the preferred size, then more sheets at that
    size, then one paper size up at a time.
  - A candidate is never chosen if it clips a view or prints below a readable scale.
  - Otherwise it is refused only for unreadable ink, errors or lost information that the
    automatic first plan does not already have. So a crowded but readable small sheet beats
    a blow-up.
  - No sheet carries only the isometric unless the author put it there. A split onto more
    sheets is also refused if it adds leader runs or hidden-edge crossings.
  - Going past a preference reports one `sheet_preference_exceeded` info notice, and a fresh
    automatic plan no longer carries a `layout_replanned` notice.
  - An explicit size, scale or count, `sheet/keep_layout` and `sheets/max` still win.
  - Drawings that change: CTC-02 goes from A0 to A1, 1013 and string_post_tapped from two A4
    sheets to one A3, cgb243 from one A1 to two A2 sheets, and bracket_positions from A1 2:1
    to A2 1:1.
- **Finding impact and groups.** Every plan problem, lint finding and structural finding
  carries an additive `impact` (`incomplete`, `decision`, `readability` or `advisory`) and a
  stable repeat-cause `group`. These appear in serve, WASM and compact MCP results.
  - Review adds per-impact and per-group counts, also returned by the MCP review tool.
  - Severity, finding order and drawings are unchanged, and gaps are never promoted to errors.
  - On NIST CTC-01 (`nist_ctc_01_asme1_rd`), review shows 8 incomplete findings in 3 groups,
    1 decision, 1 readability, and 13 advisory findings in 4 groups.
  - See `docs/finding-impact.md` (#152).
- **Interior callouts.** Feature callouts, leader-attached GD&T frames and AP242 surface
  labels that cross other ink are retried over blank face inside their own view, where the
  geometry confirms there is room. This includes inward diameter and thread leaders over hole
  floors.
  - Text height stays 3 mm, and equal costs favour exterior routes.
  - GD&T frames now route around neighbouring views.
  - On NIST CTC-01 AP242, leader-crossing warnings fall from 25 to 10 (53 → 38 problems in
    total), with no other problem code changing.
  - Across 68 parts there were no regressions in other problem codes, sheet size, scale or
    retained information (#147).
- **Frameless datums are drawn.** Datums declared in STEP AP242 or the Spec are drawn even
  when no control frame cites them.
  - `sheet/uncited_datums: false` hides uncited symbols, and `annotations/<id>/omit` hides one.
    Omitted symbols keep their provenance in the plan.
  - Datum leaders that end on the frame border are no longer reported as
    `leader_line_through_text` errors; a shaft through the datum letter still is. This removes
    95 false errors on cited datums across 31 test parts (#141).
- **Leader crossing kinds.** Retained crossings are classified, in additive `crossing_kinds`
  metadata, by what they cross: a leader, dimension line, extension line or arrow.
  - Leader-through-text is marked `ink_is_leader`.
  - Crossed linear dimension lines are reported as `dimension_line_crossing` warnings.
  - Drawings are unchanged (#144).
- **Local fixes keep the layout.** A fix with local scope keeps its sheet size, scale and
  count, and `sheet/keep_layout: true` states the same intent for any edit. No page
  coordinates or item lists enter the Spec.
  - An offer discloses any new findings that applying it would introduce.
  - Conflicts are refused, and moved views are named.
  - An edit that does change paper, scale or count reports `layout_replanned`.
  - A later explicit paper, scale or sheet-count edit lifts the hold.
  - See `docs/local-fixes.md` (#126).

### Changed

- `sheet/preferred_printer` is removed. A Spec that still carries it is refused with guidance:
  an A4 printer becomes `sheet/preferred_max_size: "A4"`, an A3 printer `"A3"`.
- **Radius leaders are radial.** The arrow meets the middle half of the arc within 15° of
  radial, from whichever side gives the clearer route (ISO 129), so a small fillet on a cutout
  can be called out from the surrounding face.
  - A blocked case keeps its callout and reports one `radius_leader_not_radial` warning (new).
  - Sections the engine cannot verify stay informational (`radius_leader_target_unverifiable`)
    (#145, #146).
- **Datum leaders leave square to the face.** Datum frames are placed on the face normal
  through the triangle. Failing that, they take one bend after a square stub, and an oblique
  leader is only a reported last resort.
  - A drawn datum leader, automatic or pinned, that does not leave square is reported as the
    lint warning `datum_leader_not_normal` (new).
  - Datum survey (33 parts): crossings 1,063 → 1,040, overlaps 27 → 23 (#146).
- **Fix descriptions in drawing language.** Each fix says in one sentence what applying it
  changes, for example "Move the M14×1.5 callout above the front view, nearest the view."
  Sheet-wide fixes add what happens to placements already moved by hand. Fix operations are
  unchanged (#138).
- **Pinned group callouts** point at the member nearest their elbow, keeping the automatic
  member on ties. Existing pins keep their label positions; only the member the arrow lands on
  can change (#149).
- **Messages.**
  - PMI that Specify wrote is credited to Specify rather than the STEP file, and a compatible
    Spec thread designation refines it.
  - Override and lint messages no longer show internal entity IDs or Spec paths.
  - `unclaimed_geometry` is informational (#102, #25).
- **Verbose MCP problems** keep their code and subjects inline, plus the preferred fix, its
  scope and the full trade description when they fit. `detail_ref` keeps every full
  alternative, and no offer hides a trade (#151).
- The advertised MCP schema ceiling is 40,000 compact bytes (was 38,000), to make room for the
  sheet-size preference keys.

### Fixed

- **Per-face surface finishes** place beside their face, clear of the dimensions, and no
  longer force a larger sheet.
  - A finish leader stays a text height away from lines it runs alongside.
  - A cylinder trimmed by flats gets a dot-ended leader on its face (ISO 128-22).
  - `annotations/<id>/place/side` picks either side.
  - GD&T placed only left of the front/plan column no longer pins the section row.
  - string_post with two Spec finishes: A3 → A4, crossings 3 → 0, no undrawn finishes (#98).
- An explicit view placement outside the drawing area is a plan error, with a fix that removes
  the placement (#95).
- Spec surface finishes no longer trigger the dimensional location audit and invent
  missing-location warnings (string_post with Spec finishes: 18 → 15 problems).
- Malformed browser requests no longer abort the WASM module, which keeps answering
  afterwards (#134).
  - Edit paths over 64 components and view gutters over 1189 mm are refused before work
    starts.
  - Conflicting dimension ownership returns an error.
  - A malformed projected spline returns an `edges_not_drawn` problem.

### WASM API

- `calloutOffset` and `Session::callout_offset` return `members: [{centre: [x,y], radius,
  span?: {start, sweep}}]` (#149). Sites are paper mm, y up; arc spans are counterclockwise
  radians. Existing fields are unchanged.
- **Spec:** `sheet/preferred_printer` is removed (refused with guidance), and
  `sheet/preferred_max_size`, `sheet/preferred_max_sheets`, `sheet/keep_layout` and
  `sheet/uncited_datums` are added.
- **Findings:**
  - Every finding gains additive `impact` and `group`.
  - Leader crossings gain `crossing_kinds`, and `annotation_ink_overlap` gains `ink_is_leader`.
  - Review gains per-impact and per-group counts.
  - New codes: `dimension_line_crossing`, `radius_leader_not_radial`,
    `datum_leader_not_normal`, `sheet_preference_exceeded`.
- Method signatures are unchanged.

### Performance

Ratio of retired CPU instructions for `draftwright plan`, alpha.11 ÷ alpha.10, from
interleaved runs; below 1 is faster:

| Part | Ratio |
|---|---|
| NIST CTC-01 AP242 | 0.89 |
| string_post | 1.00 |
| MFCAD++ part 1000 | 1.01 |
| cgb243 | 1.03 |
| NIST CTC-01 (native STEP) | 1.08 |
| NIST CTC-02 | 1.11 |
| NIST FTC-09 | 1.89 |

- FTC-09's cost is the preferred-size search: its A2 candidates fit by footprint, so they are
  planned and judged before A1 is kept.
- On native CTC-01, #146's leader search measured +9% on its own PR.

## 0.1.0-alpha.10 (2026-10-20)

### Added

- Modelled screw threads are drawn by ISO 6410 convention (#137). The helix faces of a
  recognised thread are replaced, for drawing only, by a smooth surface at the crest diameter:
  thick crests, thin roots, termination lines, a thin three-quarter root arc in end views and
  section hatching to the crest. The part, face IDs and recognition records are unchanged. The
  info problem `modelled_thread_not_depicted` is now reported only when depiction is turned off
  for the thread or the sheet, or is unavailable.
- Thread callouts state the designation only, inferred from a unique ISO 261 diameter/pitch
  match (e.g. `M75×1.5`; pitch is always stated, coarse or fine; multiple-start and left-hand
  threads in ISO 965 form, e.g. `M20×Ph2P1-LH`). With no unique match the measured values are
  stated, e.g. `⌀110 × P6.35`, with info `thread_designation_measured`. No length is stated: the
  depiction shows the thread extent, and the measured span stays in the plan as evidence. A
  semantic PMI thread on the same feature owns the callout in `pmi: "first"` and `"strict"`,
  ahead of any Spec `designation`; `"ignore"` uses recognition alone.
- Per-thread Spec override `features/<id>/modelled_thread`: `depiction: false` draws the
  modelled edges again, and `designation` replaces the callout text verbatim. Schemas and MCP
  item options include both. A `modelled_thread` override on any other feature is refused
  ("features/<id>/modelled_thread requires a recognised modelled thread."), and designation text
  is validated like a note. See `docs/modelled-threads.md`.
- On cgb243 the callouts are `M14×1.5`, `M62×1.5` (twice) and `M75×1.5`; on
  threaded_connector_109, `⌀110 × P6.35`. The other 98 corpus plans and SVGs are byte-identical.
  With depiction on, the helix edges are no longer projected. Retired instructions: cgb243 plan
  2,779 → 578 billion and render 3,010 → 696 billion; threaded_connector_109 render 4,127 → 299
  billion (plan unchanged, 147 → 146 billion). cgb243 has 21 leader crossings against 17 before;
  no labels overlap.

### Changed

- The two tests that pinned SHA-256 hashes of output (plans, SVGs and PDFs for notes identity;
  plans for legacy view alignment) are replaced by assertions of what they protected: a part
  without part PMI draws only its Spec notes, and explicit view alignments keep their edge and
  gap (#137).

## 0.1.0-alpha.9 (2026-10-20)

### Changed

- Pinned to quiddity-rust `v0.1.0-alpha.8` (#135). Recognition documents v3 and v4 are both
  read (v3 only without thread records). Modelled threads appear as `modelled_thread` features
  with their measurements, claimed faces and refusals; each is reported as info
  `modelled_thread_not_depicted`, in place of `feature_not_dimensioned`, until conventional
  depiction lands, and the modelled edges are drawn as before. On cgb243 (4 threads) and threaded_connector_109 (1), every dimension, the
  sheet and the SVG bytes are unchanged. Drawing fields are unchanged across all 100 corpus
  plans. Planning costs 1.7% more retired instructions over the corpus (mfcadpp/10038, the
  worst, +36%, 1.03 → 1.41 billion).
- Graphic PMI accounting from haecceity: geometry-only STEP files no longer report PMI
  presentation and no longer report `pmi_presentation_not_read` (the ten NIST geometry-only files
  1,595 → 0 in total; nist_ctc_01 AP242 keeps 71, nist_ftc_10 AP242 279). The plan, SVG and PDF carry the
  PMI loader binding `haecceity-pmi-read/8` (#135, for #125).

## 0.1.0-alpha.8 (2026-10-20)

### Added

- PMI modes: `pmi: "strict" | "first" | "ignore"`, `"first"` by default (#122).
  - `first`: semantic PMI takes precedence. A feature that PMI already sizes, or locates through
    a chain to an origin, gets no second engine dimension on that axis. Covered engine IDs stay
    in the omitted ledger with the IDs of their PMI owners.
  - `strict`: draws the part's semantic dimensions, callouts, GD&T and cited datum symbols only.
    Missing sizes and locations are reported as `feature_not_sized` and `feature_not_located`;
    no hole schedule is drawn. `dimension_source: "authored"` can
    add listed engine IDs.
  - `ignore`: draws the ordinary generated drawing and reports how many PMI items it withheld
    (`pmi_ignored`).
  - With no PMI, all three modes give the ordinary drawing; the 213 plain corpus drawings are
    byte-identical to alpha.7.
- Location origins (#122). Each axis's origin is the first datum, in letter order, that is a plane
  normal to it, or else the box minimum. `origin` overrides it per part axis with `"min"`, `"max"`
  or a face/feature reference (`{"x": "face:5", "y": "min", "z": "max"}`). An unusable datum gives
  info `datum_origin_unusable`,
  and dimension basis text names the selected origin.
- Located-once checks on parts drawn with PMI (`first` or `strict`) or with a Spec `origin` (#122). `feature_not_located`
  means no drawn dimension chain reaches an origin; `feature_located_twice` names independent
  chains locating the same feature axis. Plain parts keep their existing location checks.

### Changed

- Datum symbols are drawn only when a geometric tolerance cites them. Using a datum only as a
  dimension origin no longer draws its symbol (#122).
- Identical PMI spans with identical text share one drawn dimension, which claims every source
  record. Distinct authored constraints are kept even when redundant (#122).
- Hole schedules state every size and coordinate, grouped by size, including coordinates already
  given by retained PMI; a schedule measured from a selected origin labels it ORIGIN. Automatic
  schedules that would retain every source callout stay inline; `hole_table: "always"` still
  requests one, except in `strict` mode (#122).
- `schemas/spec.json` and the MCP tool input schemas gain `pmi` and `origin`; `pmi` is left out of
  saved Specs when it is `"first"`. The WASM API is unchanged (#122).
- PMI planning costs at most 1.12× alpha.7 in retired instructions; plain parts are unchanged
  within 1% (#122).

## 0.1.0-alpha.7 (2026-10-20)

### WASM API

- `calloutOffset(address)` and `labelOffset(address)` return a callout's or linear dimension's
  automatic anchor, current label position and offset (paper mm), so an editor can pin a label
  dropped anywhere, inside a view included, with `place.offset`. `placementOptions` marks items
  that accept an offset (`"offset": "callout" | "linear"`) (#121).
- Problems in snapshots and deltas carry ordered fix alternatives (`fixes`), each with a
  `scope` (below); `placementOptions` gives each item's `operation_scopes` (#128).

### Added

- Prefer A4 for the automatic sheet. The automatic plan is the baseline; when it is not
  already one A4 sheet, forced candidates are tried and the first with no layout or
  readability problem the baseline lacks (including new leader-on-extension contacts) is
  taken, else the baseline stays. `sheet/preferred_printer` (`"A4"` default, or `"A3"`) sets
  the order: A4 printer 1×A4 → up to 2×A4 → 1×A3; A3 printer 1×A4 → 1×A3 → A3 overflow. The
  sheet basis records the choice. 16 of the 100 corpus parts move to smaller paper and none to
  larger (e.g. mfcadpp/10190 A1 10:1 → A4 1:1, NIST FTC-07 A2 1:2 → A3 1:5). The search adds
  about 5% planning cost in retired instructions; mfcadpp/10278, which tries every candidate
  before falling back, costs 1.82× (#130).
- Draw the part's semantic-text notes from PMI in the general notes block, before the Spec's
  notes, ordered by Specify's `note index` and then file order: Specify's free-text `general
  note`s, coating (`surface treatment` or `coating`), heat treatment and edge conditions. Values that state no
  requirement (`None`, `N/A`, `-`, empty) are info only; a value that does not name its process
  is prefixed (`Coating: Black`). `annotations/<PMI id>/omit` leaves one off the drawing. A Spec
  note repeating a PMI note is drawn once, with info `note_duplicated_in_spec` and a fix
  removing the Spec copy (#127).
- Every offered fix states its `scope`, `local`, `view` or `sheet`, from the Spec paths it
  writes (unknown paths count as `sheet`), and alternatives are listed local first; a sheet-wide
  fix says it re-plans the sheet. MCP (`list_problems`, `get_item`, `item_options`, preview and
  apply rows), WASM and lint output carry it. NIST CTC-01's leader and extension crossings at A3
  1:5 now offer local placement edits before the hole-table remedy (#128).
- `place.offset` pins a linear dimension's label as well as a callout's (#121).

### Changed

- Omit redundant reference dimensions by default instead of drawing them in brackets; each is
  a `covered` omission naming the dimensions that determine it. `sheet/references: true`
  restores them; pinned references and per-dimension `style: "reference"` still draw. The demo
  bracket's ladder becomes 8, 12, 20, 32, 48; 21 references go across 8 corpus parts (#120).

### Fixed

- Draftwright's own records in a STEP file (Specify's stored answers, Python Specify's earlier
  `schema 1` property, the editor's project export) are no longer reported as unread PMI;
  foreign attribute sets still are (#129).
- Isometric captions stay clear of reserved projection bands (#120).

### Internal

- The WASM size budget is re-baselined on this build's main at +10% (ceiling 17,300,000 raw
  bytes), after feature growth since alpha.6 (#128).

## 0.1.0-alpha.6 (2026-10-19)

### WASM API

- `sheetFragments(sheet)` and `sheetSvg(sheet)` give any sheet's fragments and SVG, as
  `fragments()` and `renderSvg()` give sheet 1's; `renderPdf()` has a page per sheet and
  throws an API error instead of returning bytes unconditionally.

### Added

- Pin a projected view's gap with `views/<name>/place/align: "projection"` while keeping
  geometry on its projection line despite asymmetric dimension bands. Valid front/plan
  and front/side pairs offer it first in `item_options`; other pairs refuse it with choices.
- Leave blank gutters between views by default: spare paper is shared between the gaps, up
  to 25 mm, and shrinks toward 0 only when needed to fit. Gutters never change the sheet size
  or scale; rows and columns move together, and pinned views keep their positions. Override
  with `sheet/view_gutter` (`0` disables).
- Choose thread and knurl callout form with `sheet/callouts/style` (`auto`, `compact`, `full`
  or `note`), overridden per feature by `features/<id>/callouts/style`. `auto` moves a
  callout that would still need more than two lines to `<size> SEE MFG n`.

### Changed

- Omit hidden edges from the isometric by default, as pictorial views conventionally do;
  `views/iso/hidden: true` or `sheet/hidden: true` still draws them (#117).
- Shorten thread and knurl callouts to the ISO 965 / ISO 6410 short forms: `M5-6g` without
  the coarse pitch or RH, the designation alone on a threaded diameter, two-line blind tapped
  holes (`M2-6H ↧ 2.5` over `⌀1.6 ↧ 4`), and horizontal knurl callouts with process wording in
  the numbered manufacturing note. Callouts wrap at 48 mm with real font metrics. 8 of the 11
  thread and knurl parts now fit on A4, up from 2 (#94).

### Fixed

- Print the modelled value: labels use the shortest form, up to three decimals, that equals
  it (`4.75`, not `4.8`); per-measurement Spec precision still wins (#92). Two audited
  exceptions keep a compact label where the longer one cannot be routed: NIST FTC-08's
  `8× R1.5` (modelled 1.524) and mfcadpp/1004's `⌀7.4 ↧ 5.9` (modelled 7.447, 5.86).
- Dimension a single flat by its distance (`7.4`) without `A/F`; `A/F` needs proven opposed
  flats (#93).
- Read the structured `full thread depth` property, so a tapped hole whose depth is given only
  by that property keeps its thread callout.
- Draw `⌀ ↧ ⌴ ⌵` in tables and numbered notes as in callout labels (SVG, PNG and PDF), keeping
  the text searchable. GRM-03's table printed `⌀1.6 □ 8`.
- Draw AP242 dimensions and tolerances on resolved derived axes and planes, preserving
  authored values, tolerances and MAX/MIN/AVG qualifiers. GRM-03 now carries its ten
  semantic sizes and lengths instead of geometric fallback dimensions.
- Keep datum symbols apart during placement so no two overlap or touch. NIST FTC-07 moves
  from A2 at 1:5 to A1 at 1:1; FTC-10, STC-09 and STC-10 also change.
- Default drawing numbers to a leading part-number token in the STEP product name, then
  the file stem (including numeric names such as `10000`), before `DWG-001`. Explicit Spec
  numbers, including blanks, win; readable titles retain the part number (#103).
- Print GRM-03's CAx-IF material (`CZ121 / CW614N / CuZn39Pb3`) in the title block without a
  false undrawn warning (#101).
- Keep radius leaders on the drawn part of trimmed arcs (quarter-cylinder edges, obround
  and slot-pattern ends) instead of the full construction circle.
- Prefer perpendicular location witnesses on either side and finite-gap witnesses before
  any oblique fallback; a fallback uses parallel 30° witnesses clear of material and other
  dimensions. On the 30 quality parts oblique witnesses fall from 16 dimensions to none.
- Lint radius-leader targets on circular steps and slot ends, and oblique witness ink.
- Stop reporting a STEP material as undrawn when the title block states it; retain the
  warning when a blank or conflicting Spec material replaces it.
- Describe missed feature recognition and unsupported STEP annotations in drawing language,
  with Spec remedies; retain tool/version and reader diagnostics in structured references.
- Prefer meaningful STEP product names for default titles, otherwise print readable file
  stems with spaces. Explicit Spec titles, including blanks, keep precedence.
- Explain that hole tables require X/Y locations from a datum corner and recommend the
  retained individual dimensions when those locations are absent.
- Clarify that view placement gaps measure between annotation bands, not part outlines,
  in the Spec schema, MCP item options and documentation. Tools/list stays at 34,477
  compact schema bytes.

## 0.1.0-alpha.5 (2026-10-18)

### Added

- Consume oriented chamfers from native recognition, claim their faces, and include equal
  oblique and principal bevels in one counted true-section C/size-and-angle callout.
- Dimension symmetric V-notches from ordered profile evidence with true included angles,
  theoretical sharp depths and datum locations. Their ordinary IDs support dimension edits,
  authored ownership and location schemes; lint verifies live values against source geometry
  and printed scheme links. Reserve acute-notch arcs before layout; bevel run midpoints do
  not introduce artificial automatic datum stations. Existing PMI angles on the same
  physical flank faces own the notch angle across normal-section heights, retaining their
  tolerance and implicit degree notation without a duplicate automatic arc. An existing
  bevel first leg owns a matching physical notch depth, keeping its callout without an
  extra depth line.
- Read native recognition/3 unified section recesses, source refusals, face levels
  and risers. The default path now uses the existing pocket/channel/blind-slot/
  passage drafting grammars and no longer warns that section recesses are absent
  from the document. Native and Python 0.4 readers retain unified pocket patterns: grouped count
  and size/depth callouts, pitch and absolute locations, independent coverage
  outcomes, and four retained radius requirements per rounded pocket. Five pending
  recess-reader rules and four formerly bridge-only rules have native regressions.
  The kernel is pinned to quiddity-rust release `v0.1.0-alpha.6`.
  See docs/native-recesses.md for reader and drafting details.

- Authored detail `from` selects an existing principal or section parent; `around` accepts
  a feature/face ID or a nonempty list whose crops are united. Missing members and
  degenerate projections in an explicit parent are refused. Mixed natural sources choose
  the view with most visible members and explain the choice in the basis. Saved strings
  remain compatible.

- Enlarged orthographic detail views for crowded turned chains and heads, thin terminal
  flanges, prismatic height ladders and authored feature crops. Details participate in sheet
  planning, carry stable view IDs and their own scale, and draw parent circles, letters,
  captions and relocated dimensions. A fitting iso keeps sheet scale beside a detail.
  Verification and Python differences: [detail views](docs/detail-verification.md).
- MCP `spec_schema {}` returns the full draft-2020-12 author Spec schema and its
  named `get_spec` read shape on demand. Tool schemas retain explicit typed top-level
  outlines. Against alpha.4 the per-session schema context falls from 42,464 to 34,477 bytes
  (−18.8%), including this release's new Spec fields.
  Nested Spec additions no longer grow tools/list; server validation and corrective
  guidance are unchanged.

- Spec dimensioning schemes by feature pattern, station set or principal view axis: chain,
  chosen-datum baseline and signed running ordinate, with stable references, session edits,
  one-owner networks and independent printed-link lint (#76 item 4).

- Spec label wording under `dimensions/<id>/text` and prose `annotations/<id>/text`:
  prefix/suffix or one `{value}` template retain every generated measured term. Explicit
  dimension `style: reference | basic` draws parentheses or a box. Full ink drives placement;
  plans expose the overrides and lint still reads live measured text (#76, item 5).
- Session/editor text and style options use existing Spec operations, preview and history;
  saved Specs round-trip and stale/merged label IDs are refused with a correction.
  Large MCP option replies retain full structured data and a lossless text-channel resource.
- Recognition corrections in the Spec (#76 item 2): reclassify supported cylindrical and
  rectangular features, group holes as a linear/grid/bolt-circle pattern, split recognised
  patterns, suppress spurious recognitions, and declare missed features from face fingerprints.
  Geometry supplies every size; saved face anchors and authored IDs survive re-analysis.
  Corrections share ordinary dimensions, schedules and independent lint, with `from_spec`
  provenance, transactional editor operations and MCP schemas inside the existing byte budget.
- Geometry-authored dimensions (`dimensions/d:authored:<name>`): principal or true distances,
  cylindrical/spherical diameters and radii, and planar-face/straight-edge angles, with stable
  fingerprint references, ordinary tolerance/placement, editor/session operations and independent
  geometry lint (#76 item 1). Duplicate drawn quantities are refused with their existing owner.
- Spec line-work switches per view and drawing-wide sheet defaults: hidden lines,
  centre marks, centre lines, thread depiction and knurl depiction (#76 item 6).
  Session options/edits support them; effective paths are reported in view `from_spec`.

### Changed

- Planning is faster on PMI-heavy AP242 files (#90): linear-dimension trials reuse their
  stroke geometry and footprint witness counts are memoised; with quiddity-rust alpha.6's
  bounded emptiness probe, NIST FTC-10's AP242 plan falls from 203 s to about 5 s. Every
  plan JSON and SVG on the 65-part survey is byte-identical to the output before this change.
- The plan-time budget is re-recorded at 2,352,592,623,176 instructions on the 65-part
  survey (from 3,166,420,524,176 after native recesses; margin still 15%).
- Development: integration tests are grouped into eight binaries, halving CI's build step,
  with a guard against unreachable test files (#91); long test loops are split into
  individual cases (#89); release-only PRs skip the build in CI (#86).

### Fixed

- Preserve archived Python 0.3.9 pocket callouts and individual datum locations.
- Repair displaced location witnesses through foreign features and overlapping
  callout text with bounded route moves that retain the existing physical guards.

- Avoid repeated projection and collision work during complete sheet searches with native
  recesses, retaining the original native and WASM release settings.


- Automatic sheet candidates require dimensioned turned-step intervals to span at least
  the printed annotation capital height in their owning view, including baseline replacements.
  Explicit scales retain their geometry and warn below this floor. A complete, legible
  A3 drawing keeps practical paper instead of growing to A2 solely for fill.
- Keep geometry-authored angle arcs local to their support intersection and place aligned
  dimensions at a clear parallel offset, moving unpinned linear dimensions out of their way
  before escalating the sheet.

### WASM API

- Additive Spec `dimension_schemes` map and scheme measurement claims; native and browser
  sessions share the same JSON vocabulary. Scheme fragment edits refresh placement options.
  Engine method signatures are unchanged.
- Additive recognition-correction Spec fields and Plan provenance use the shared native
  session model; binding methods and TypeScript signatures are unchanged.

- Establish the reviewed TypeScript API snapshot and changelog gate, a measured WASM size
  ceiling, and warning-only cross-package quiddity pin checks in CI and release notes.
- Additive Spec line-work switches and optional Plan view `line_style`/`from_spec`
  fields, shared with native sessions; no binding method changes.
- Spec JSON accepts authored feature crops (`detail`, `around`, `from`, `scale_factor`)
  and optional Plan view `detail` records; engine method signatures are unchanged.
- Additive Spec text/style fields and annotation/note `from_spec` JSON provenance
  through the existing session methods; no new JavaScript methods or signatures.
- Spec JSON accepts authored dimension fields; Analysis/7 exposes optional exact
  dimension-reference evidence. Engine method signatures are unchanged.

## 0.1.0-alpha.4 (2026-10-16)

### Added

- Browser `draftwright-wasm` workspace crate, moving the editor's live-session wrapper here
  (#69, #66). JSON-string operations and errors, SVG/PDF bytes, bundled fonts and panic hook;
  byte-based analysis and native-only adapter gates keep the browser independent of files.
- Dedicated `wasm-release` profile, Node/native session parity CI, and a published-release
  workflow attaching the web package and SHA-256 checksum. See `docs/wasm.md` for builds,
  package sizes and switching the editor to a pinned release asset.
- Spec-authored external spur gears, with an ISO 1328 gear data table, validated anchors
  and gear coverage and reconciliation lint; gears are declared, not recognised (#67).

### Changed

- Small parts try a measured single A4/A3 sheet, then a second A4/A3 sheet, before larger
  paper; second-sheet views are centred and section references name the right zones (#63).
- ISO 286 fit callouts show the class alone by default; `sheet/fit_deviations: true` draws
  stacked upper and lower deviations after the class (#72).

### Fixed

- Datum placement skips searches that cannot improve the result, restoring planning cost;
  planar datum extensions require the face to end within 1 mm of the view outline (#61).
- Pinned labels obey the shared-line guard, omitted authored-set callouts report a restoring
  fix, dimension edits cannot add extension crossings through the part, and thread inference
  requires a positive designation (#62).
- Schedule cells verify tolerance, precision and THRU evidence; manufacturing references
  require complete source-matched ink, and a retained leader crossing rolls back the whole
  table. Damaged cells lose evidence and descriptive tables cannot replace measurements (#64).
- Opposed coaxial taps retain their authored callout sides and GRM-04 retains its small-hole
  callout; plan provenance binds the full recognition input, ownerless occurrences state
  closed policy reasons, and assembly severity reaches every feature coverage family (#65).
- Prismatic bore labels keep their view row and turned bore labels clear the axis; lint
  checks overlaps and manufacturing source text, and finish values respect declared precision
  and multi-word parameters (#68).
- Middle turned bodies use their own corridors; prismatic shoulders need truthful evidence,
  omitted separating views report a restoring fix, double-D callouts preserve their roles,
  countersink seats stay independent, and hole-pattern counts and balloon allocation are
  checked (#70).
- Finish leaders stay on their source faces, oblique PMI uses its exact projected span,
  envelope bands and extent fits reach the ink, and requested precision reaches dimension
  chains and locations. Invalid or conflicting precision is refused; structured notes cover
  only their placed owner's explicit roles (#71).
- Full fit-label ink drives reservation and placement; dimension lines and arrowheads are
  obstacles, final settlement includes callouts, and lint counts linear arrow flares (#72).

### Fixed

- Optional detail requests no longer veto valid ordinary drawings or cap principal scale.
  Cropped edges meet thin zigzag break lines; captions follow real projected ink. Prismatic
  details keep the feature silhouette and use preferred enlargement scales. Extent queries
  map only the envelope, once per projected view, and large-paper omissions precede planning.
- Multi-step turned profiles use one inspection end by default, preserving PMI datum
  precedence and saved chain IDs. Cross-hole axial locations use the same end; baseline
  lanes run shortest nearest the part. Valid complete larger principal views take precedence
  over optional details under the practical-paper sheet rules.
- Visible edges take precedence over hidden edges in the isometric as well as principal
  views, including when a detail changes the isometric scale.

## 0.1.0-alpha.3 (2026-10-16)

### Added

- Gzipped STEP input (maintainer decision 2026-10-15): every command (`analyse`, `plan`,
  `render`, `review`, `parts`), serve and MCP request, cached registration and library entry
  point that reads a STEP file or its bytes (`api`, `model::analyse_step`, `model::parts`,
  `pmi::read`, `pmi::Reader::attach`, `Session::open`, `stateless::Cache::register`) accepts
  gzip, told by its magic bytes (`1f 8b`), not the file name, and decompressed in memory
  (`model::step_bytes`, `model::read_step_file`). The binding's `source_sha256` and the cache
  key are of the decompressed bytes, so a `.step` and its `.step.gz` bind, cache and draw
  identically (a `.step.gz` is titled by its stem without `.gz`). A gzip that does not
  decompress is refused as invalid input, saying so.

- Draw one chosen assembly part through the CLI (`parts`, `--part`), API, sessions and MCP,
  with its own geometry, PMI and material; drawing an assembly whole reports it (#28).
- AP242 external threads replace turned-diameter callouts with their designations; profile
  views draw thread minor-diameter lines and knurl bands (#35).
- AP242 surface textures draw as ISO 1302 face symbols or a part-wide default finish (#38).
- Pinned callouts accept a paper-space `place.offset`, including positions inside their view;
  sessions report the current offset and pinning it leaves the callout in place (#39).
- First- and third-angle projection via `sheet/projection` (#37); off-scale isometric views
  carry `ISO VIEW (NTS)` (#49).
- ISO shaft fits on turned diameters print their ISO 286 limits; `sheet/fit_limits: false`
  keeps the class alone (#42).
- Spec-authored feature and face datums and GD&T controls, sharing the AP242 drawing path;
  spec entries override STEP PMI and report undeclared datum letters (#47).
- Spec-authored surface finish, threads, knurls and leadered notes on features and faces,
  plus `sheet/default_finish`, through the same paths as AP242 PMI (#52).
- Structural lint checks labels against measured ink, angles against their rays and visible
  edges, datum-symbol distance, and nominal rounding (#50).
- Per-body length chains for parallel turned bodies, raised-pad sizes, heights and locations,
  and counted free-direction slot callouts; lint credits full extents and exact adjoining
  groups, and ambiguous groove ownership is refused (#55).

### Changed

- The Python-rules catalogue (`docs/rules`) applies the maintainer-approved triage of every
  pending rule: 12 retired, 91 translated with the Rust mechanism that meets each, 7 already
  met marked ported, and the 80 left to port tagged with value, size and stream; stale notes
  describe current Rust. `tests/rules_catalogue.rs` checks that every rule says what backs its
  status, every cited test exists and the README tables match the files.

- Pinned haecceity and quiddity to quiddity-rust `v0.1.0-alpha.4` (commit `f5ddb726`), from rev `7cb26d85`.
- Plan problems use drawing language and identify the affected faces and callouts;
  reference dimensions carry `reference` and `determined_by` (#29).
- Turned profiles measured from a datum end use baseline shoulder positions; existing band
  dimension IDs remain covered by those positions (#48).
- Faster hidden-line rendering through quiddity-rust, with render-tail acceptance budgets
  for cgb243 and threaded_connector_109 (#51).
- The single-sheet byte guard is opt-in (`DRAFTWRIGHT_SINGLE_SHEET=1`); its fixtures are kept
  without maintenance and CI no longer records them. Quality, leader-route, lint and test
  gates remain (#53).
- One owner per measurement: authored sets preserve covered items, ladder and shoulder sets
  are approved or omitted whole unless an explicit member entry conflicts, and closed boss
  heights become references. Lint reports redundant pattern locations and toleranced
  references (#57).

### Fixed

- Pinned sheet size and scale are judged on the finished drawing, with a verified fix for
  real misfits (#37).
- The third-angle projection symbol follows ISO 5456-2; basic cross-hole locations work
  across principal bore axes and replace the plain default with a boxed dimension (#40).
- Applied leader fixes re-space overlapping dimension lanes and keep outside text beside
  its line; plan problems and lint report shared lines and stray text (#46).
- Leader routing and lint share the material re-entry check; pinned callouts report re-entry
  with an unpin fix, diameter and radius leaders are checked against their targets, and
  callout arrowhead flares count as collision ink (#54).

- Datum feature symbols try the space inside dimension lanes before outer tiers, can use
  either proven cylinder silhouette, and can stand on drawn planar face extensions. Shared
  GD&T placement counts leader crossings with dimension and extension lines for frames,
  labels and surface finishes too. Complete arrangements cannot introduce a new annotation
  contact to clear an earlier leader; placement intents and pins retain priority.
  Empty placement resets restore automatic tiers. The CNC-review spindle's A and B clear
  the dimension stack.

- NTS captions retry with reduced clearance and nearby vertical positions before retaining
  a clash; structural lint reports shallow caption overlaps with text and line ink. Pinned
  placement refits use the same strict 5% sheet-scale snap as measured refits. An authored
  iso scale only skips infeasible-iso candidates during recovery; other errors propagate.

- `--part 0` (`"part": 0`) of a file of one part is the whole file: drawn byte for byte as without
  a part chosen, titled by the file's stem rather than the STEP product's name (`plate.step`
  gave `COMPOUND`). A part of a real assembly is still titled by its product's name.

## 0.1.0-alpha.2 (2026-10-14)

### Changed

- Pinned to quiddity-rust `v0.1.0-alpha.3` (from `v0.1.0-alpha.1`). Its recognition document's
  new `section_recesses` family (`SectionRecess`, `SectionRecessRefusal`) is admitted as known
  and not yet read, as the other recess families are; without that every part with a section
  recess was refused. No drawing changes: across the single-sheet guard's 1876 cases the new pin
  leaves SVG, PDF and PNG bytes unchanged and each plan differs only in its
  `features_not_recognised` warning (the recogniser's version, and section recesses counted from
  the document); the version bump then changes only the loader version in each plan's binding
  and each file's embedded Spec. The guard is re-baselined for this (`quiddity-alpha3`).
- Multiple sheets: a plan allocates views and tables across sheets (SHEET n/m), every sheet is
  rendered, sessions and MCP work on multi-sheet plans, and sheets carry ISO 5457 zones and
  section cross-references (`lint_sheets`).
- Leader routes keep callout text, arrowheads and elbows off extension lines and the outline;
  pinned callout labels keep their place and side.
- Fits print their ISO 286 limits (`sheet/fit_limits: false` prints the class alone); repeated
  spans are drawn once; the title block credits draftwright in DRAWN BY.
- DXF export is disabled until it is fixed.
- Stable identity (`draftwright.spec/2`): a dimension's ID is `d:<feature>/<parameter>`, never
  its view, so moving it to another view, omitting a view or changing the sheet or scale keeps
  it; the view is the item's `view` and may be chosen in the spec (`dimensions/<id>/view`). A
  PMI item's ID is its record's semantic identity (`pmi:<category>.<kind>:f…:ref=…`), never its
  Part 21 instance, so the same intent written again keeps it; records that cannot be told apart
  are refused. A `draftwright.spec/1` spec is migrated when planned; ambiguous keys are refused
  with their candidates (`docs/model.md`, "IDs").

### Added

- Typed state and problems: every plan item carries a `state` (`shown`, `omitted_by_user`,
  `not_authored`, `covered` by another item, or `not_drawable` with the code of the problem that
  says why), beside its basis sentence; every problem's `subjects` are live item IDs, with what
  else it names in `refs`, and a machine-readable `fix` (edit operations) where one resolves it;
  `items::items` and `items::label` list every item with the text it draws, an omitted
  dimension's and a GD&T item's label included (`docs/model.md`, "Plan").
- Placement as intent, a session core (apply, preview, undo and redo, queries) and a stateless
  MCP core with its cache and schemas.
- AP242 semantic `turned_knurl` PMI is drawn.

## 0.1.0-alpha.1 (2026-10-10)

The first alpha. Its gates and their state are in `docs/alpha1.md`.

### Added

- The document model (ADR R1): Analysis, Spec and Plan, with `analyse`, `plan`, `edit`,
  `render`, `review` and `spec` on the command line, as a JSON-lines service (`draftwright
  serve`) and as an MCP server (`draftwright-mcp`).
- Native feature recognition through quiddity-rust (`quiddity-rust/recognition/2`), read
  fail-closed; Python quiddity's JSON through `--features`.
- Layout with draftwright's sheet, scale and view arithmetic, reserved annotation bands, and
  page escalation and recovery; section A–A without a boolean.
- Dimensions per family (envelope, turned steps, holes and patterns, slots, bosses, fillets and
  chamfers, grooves, flats, steps and the height ladder, section recesses, hexagonal stock),
  with spec tolerances and ISO 286 fits; hole tables by drop-and-escalate.
- Lint of the drawing against the recognised features, independent of the Plan.
- AP242 semantic PMI: read through haecceity; feature control frames, datum feature symbols,
  and toleranced and basic dimensions drawn; surface and common labels drawn with leaders, and
  the general tolerance, default surface finish and edge-condition notes on the sheet; thread
  and knurl requirements on their owning hole's or turned step's callout, or `SEE MFG n` with a
  manufacturing requirements table.
- The title block's material from the STEP file's one usable material property when the spec
  states none, as draftwright does.
- Angular PMI dimensions between two planar faces, drawn about their vertex in the view that
  shows the angle true (nist_ctc_01's `60 ±0.5` in the plan); one no view shows true is refused.
- The AP242 files haecceity's PMI writer writes (8, from quiddity-rust's written fixtures) in the
  PMI reading, drawing and coverage comparisons against draftwright, every difference verdicted.
- Band space for GD&T: the layout reserves room for feature control frames, datum feature
  symbols and PMI labels, and places them inside their views' bands (draftwright reserves none).
- PMI on hole occurrences (AP242 edition 4 `basic_round_hole`), now that haecceity reads them:
  nist_stc_09's hole callouts and their position frames are drawn, counted over the pattern.
- For callers with no file system, such as a browser build: `model::analyse_step` and
  `analyse_native_step` analyse a STEP file from its bytes, and the bundled font is public
  (`api::MONO`); haecceity runs hidden lines on one thread where none can be spawned.
- SVG, DXF, PDF (subset fonts, searchable text) and PNG output, each carrying its Spec;
  deterministic to the byte in every time zone.
- The PMI coverage lint: every PMI item is on the drawing or reported.
- `plan` from a saved Analysis is fast again (the maintainer's render cache and straight-piece
  boxes, PR #1): about 16 ms on NIST FTC 06, from about 490 ms; drawings unchanged.
- Drawing quality on the 30 measured parts (`tools/quality_parts.txt`), against the 86ffcbc
  baseline: annotations off the sheet 34 to 0, overlapping views 34 to 0, views on the title
  block 1 to 0, repeated labels 50 to 18 (each remaining one verdicted), ink problems 506 to 282,
  with none over a view.

### Known limitations

- Oblique PMI lengths, a cone's angle, a thread or knurl on a boss and face-finish
  requirements are read and reported, not drawn.
- Section recesses and some other families are not read from quiddity-rust's document
  (`features_not_recognised`); an angled step is reported unsupported.
- Revision rebinding (ADR R3) is not implemented.
- 282 ink problems remain on the 30 measured parts (109 leader crossings, 155 cases of ink
  past a view's area, 18 lines through labels), four parts have accepted cells worse than the
  baseline, dense AP242 parts carry much more ink, and hole tables are drawn only by
  drop-and-escalate; the README's "Known limitations" lists each with its counts and verdict.
