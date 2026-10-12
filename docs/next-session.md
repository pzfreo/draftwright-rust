# Next session: handoff (2026-10-08)

This is written for a fresh session continuing the port with parallel workflows ("ultracode").
Read it first, then the documents it points to: `docs/review-2026-10-08.md` (the latest review,
its improvement plan and the maintainer questions) and `docs/plan.md`'s "Next steps
(2026-10-08)".

## Where things stand

Updated 2026-10-09 at the end of the resumed autonomous run. The final summary at the top of
`docs/status-2026-10-08.md` lists what each stream changed, what is verified and the backlog in
order.

**quiddity-rust** (`~/repos/quiddity-rust`, branch `prototype/rust-port`, at 5798c59, pushed):

- `crates/haecceity` is the kernel. It covers:
  - STEP import, rays, classification, volume probes and coverage;
  - surface recovery;
  - hidden lines with exact projected curves, and sections without booleans (coplanar cut faces
    traced since 10ce207);
  - prism probes and face sweeps, and face mass;
  - AP242 (merged from `ap242`): lossless Part 21, EXPRESS validation, semantic PMI read and
    write, removal; `quiddity parts` and `quiddity pmi read|write`.
- 45 recogniser entry points are ported (the README's table), each checked call by call against
  Python's captured calls, over the 100-part corpus and under rigid motion. Recognition runs in
  the part's own frame by default and reports in file coordinates (`recognise_placed`,
  document `quiddity-rust/recognition/2` with a part `frame` and per-record `local` fields).
- Ported but not yet wired into `recognise()` or the document: the section-recess types,
  geometry and discovery; the aggregate's reconciliation decisions (261/261 match Python);
  locally degraded holes and pockets (the retry waits on the panic-versus-refusal question).
- Verify with `QUIDDITY_CORPUS_REQUIRED=1 cargo test --workspace --release` (401 passed,
  2 ignored at 5798c59), fmt and clippy. The corpus is at `~/repos/quiddity/tests/corpus`. Set
  `QUIDDITY_CORPUS` when working from a worktree.
- CI (nextest shards, Linux and macOS): Linux about 10 min, macOS about 20 min. **Linux failed
  at 0c76531** (the uv-bounds merge): `tests/recognition.rs::the_default_document_moves_with_the_part`,
  cgb203 under translation, `circular_face_patterns` fit_error 0.278052 vs 0.386336; macOS
  passes. Still failing at 5798c59 (run 37880089947). Fixing it is the first backlog item.

**draftwright-rust** (`~/repos/draftwright-rust`, `main`, at 5ca1130 plus the final docs commit,
pushed):

- **The document model** (ADR R1, `docs/model.md`):
  - Analysis, Spec and Plan;
  - `analyse`, `plan`, `edit`, `render`, `review` and `spec`, through the CLI, the JSON-lines
    service and the MCP server (`src/api.rs`, `src/bin/`);
  - the spec embedded in SVG, PDF, DXF and PNG; general notes (`n:<k>`) and spec tables
    (`t:<k>`) drawn and in the PDF text;
  - output deterministic to the byte, in every time zone (`tests/timezone.rs`).
- **Recognition:** the CLI takes features from Rust quiddity's recognition by default
  (`src/bridge/native.rs`, recognition/2, 26 families read; a record with `local` fields is
  refused); `--features` reads Python quiddity's JSON through `src/bridge.rs`. serve, MCP and the
  library's `None` still use the bridge (maintainer question). Differences are listed in
  `known_native.json`. Not read natively: section recesses (not in quiddity-rust's document),
  prismatic pockets (Python publishes none). Pads are read (d-native-pads).
- **haecceity** is pinned at quiddity-rust 10884b8 (`Cargo.lock`; AGPL-3.0-only). The bridge
  reads nist_ftc_07, nist_ftc_10 and nist_ctc_04 (sphere areas held to 1e-2,
  `known_bridge_faces.json`). haecceity refuses a part whose drawn faces or edges did not resolve
  or whose visibility no ray decides; `analyse` and `render` pass that on.
- **Layout and placement:** draftwright's sheet, scale and view arithmetic (`src/layout.rs`),
  exact against 285 recorded calls (`known_views.json`, 71 entries). Placement keeps annotations
  on the sheet with bounded lazy slides and places callout leaders beside the view with a
  horizontal shoulder; remaining collisions are rust-wrong in `known_placement.json` and
  `known_leaders.json`. Page escalation (`model::recover`) is applied to every plan the library
  gives (`model::plan_recovered`, called by `api`; `model::plan` is one try), under the
  maintainer's directive of 2026-10-09.
- **Dimensions** (`src/dims/`, one planner per family): envelope, turned steps (step diameters
  and lengths), holes and patterns with locations, slots and bosses, grooves and flats, steps and
  the height ladder, section recesses, hexagonal stock; coincident spans merged; spotface symbol;
  spec tolerances. Compared with the captured facts (`known_dimensions.json`).
- **Lint** (`src/lint/`, never reads the Plan or Spec): hole coverage with group credit, family
  coverage, slot patterns, eleven structural codes, the coverage backstop. Compared with Python's
  lint (`known_lint.json`, `known_structural.json`).
- **PMI stage 1:** `src/pmi.rs` reads a part's AP242 semantic PMI into draftwright's records
  (`tests/pmi.rs`, `known_pmi.json`: 330 differences, 55 rust-wrong). Not lowered or drawn yet.
- **PMI stage 2b (dimensions):** `src/dims/pmi_dimensions.rs` gives each AP242 dimension to the model dimension that states it (hole callouts split by requirement, turned and boss diameters) or draws it beside them (basic ones boxed), else reports why (`tests/pmi_dimensions.rs`, `known_pmi_dimensions.json`); left: angles, oblique lengths, bolt-circle splits, a radius on several faces.
- **PMI on the drawing vs draftwright's (alpha 1 gate):** `tests/pmi_drawings.rs` (`tools/capture_pmi_drawings.py`, `tests/fixtures/pmi_drawings/`) compares, per AP242 record of the 14 fixtures, what each drawing states (view, label or frame, datums, modifiers, basic box, the model dimension it rides on) through `plan_recovered` and `render`: 334 differences, all with verdicts (252 rust-correct, 38 rust-wrong, 40 undetermined, 4 equivalent; after d-pmi-labels-sheet-reqs 299: 253 rust-correct, 2 rust-wrong, 40 undetermined, 4 equivalent); every frame both draw states the same compartments, and where both draw a dimension they differ only by the port's grid suffix or its giving the requirement to the callout that states the size (verdicted). Rust-wrong left: only the two angular dimensions (ctc_01 `60 ±0.5`).
- **Angular PMI dimensions (d-pmi-angular-dims):** `src/dims/angular.rs` derives an angle's vertex and sides from its two planar faces when the part is analysed (draftwright's `_angular_reference_from_planes`; the Analysis's PMI `geometry.angle`) and draws it in the view along the normal of their plane (arc broken around the label, arrowheads, extension lines past the witnesses, draftwright's `minimum_radius`, then 1 mm steps to the first clear place; `Placed.lines` carries the arc's chords, plan kind `angular`); an angle no view shows true is `pmi_not_lowered` with draftwright's reason (`tests/pmi_angular.rs`); the coverage lint (`pmi_coverage::dimension`) lowers an angle exactly where the plan draws it, so a drawable angle missing from a drawing is `pmi_not_rendered`. The Analysis gains an optional field (`pmi::Geometry::angle`, set by `pmi_lowering::dimension_geometry`); an older Analysis's angle is refused with "analyse the part again". ctc_01's `60 ±0.5` is drawn in the plan as draftwright draws it: the last 2 rust-wrong entries of `known_pmi_drawings.json` (now 297: 253 rust-correct, 0 rust-wrong) and the angular ones of `known_pmi_dimensions.json` and `known_pmi_coverage.json` are gone; the 30 quality parts' and the other 12 AP242 fixtures' plans are byte-identical. Not drawn yet: a cone's included angle (draftwright's `_angular_reference_from_shapes`; refused as not planar). A basic angle is drawn boxed, where draftwright refuses it for want of boxed angular ink (`_basic_unboxed_producer`; no fixture has one). **Question for the maintainer:** draftwright prints an angle's label without `°` (`implicit_degrees`), and so does the port; ISO 129-1 writes the unit: add it?
  - PMI labels and sheet requirements (d-pmi-labels-sheet-reqs): surface and common labels are drawn as leadered `label` items (`pmi_lowering::label_geometry`, `dims::gdt`); the general tolerance fills the title block's cell (a spec tolerance wins; a different PMI one is `step_general_tolerance_disagreement`), the `Ra` default finish is an ISO 1302 symbol beside the title block (draftwright's glyph has a vertical second leg; rust-correct), edge conditions are general notes `pmi:<source>`; the plan's new `requirements` lists them; a turned part keeps a view a PMI item needs (`annotation_view_required`). The 30 quality parts' plans are byte-identical; on the AP242 fixtures label leaders add Policy B crossings (ftc_10 +88, stc_09 +46, ftc_07 +21), and assembly_plate_pin, now with its side view as draftwright, recovers to A0 2:1 (draftwright A1 1:1, incomplete). **Questions for the maintainer:** the PMI's general notes follow the spec's (a spec note keeps its number; draftwright numbers by its model's order): keep? Should label leaders get their own placement pass (draftwright's corridor solver drops what it cannot place; the port places and reports)? **Question for the maintainer:** should a basic PMI dimension whose span a model dimension already states box that dimension instead of being drawn again beside it (bracket_positions, assembly_plate_pin; draftwright never gives a basic dimension to a model dimension)?
  - Landed and re-verdicted (d-land-pmi-labels, on main after c3f2e33/dd5d915): the coverage lint
    now counts a drawing-wide requirement `pmi_lowering::sheet_requirements` lowers as lowered (so
    one lowered and not drawn is `pmi_not_rendered`, a refused one `pmi_not_lowered` with
    `sheet_requirements`' reason), and a common label not lowered gets `lower`'s reason (it said
    "the port draws no labels"); no finding on the fixtures changes. `known_pmi_coverage.json`:
    ftc_10 `4X R0.15 MAX` (dimension:#20824) rust-wrong -> equivalent (its references are edges:
    draftwright refuses it at extraction, the port at lowering, `label_geometry`); the four
    thread/knurl entries' evidence no longer lists labels and notes. `tests/pmi_labels.rs` wrote
    one scratch file per fixture from parallel tests (a STEP parse error when two raced); each
    call has its own now. **assembly_plate_pin:** the port recovers to A0 2:1 (A3 1:1 hard ->
    A3 1:2, A3 2:1, A2 1:1, A1 2:1 each with a hard finding -> A0 2:1); draftwright takes A1
    1:1, incomplete (`pmi_dropped` '45', `scale_decision`). Pinned to A1 1:1 by a spec the port's
    plan has no hard layout finding and drops nothing: rust-wrong, but the cause is the first
    sheet choice's bands, not the recovery (corrected by d-plate-pin-recovery, below).
    Quality (`tools/quality.py`, `tools/quality_parts.txt`, release): 475eefc (before the labels),
    origin/main ba0bd72 and this head give byte-identical `baseline.json`; no part changes in any
    column:

    | part | off_sheet | repeated_label | repeated_measure | views_overlap | view_on_title_block | ink |
    |---|---|---|---|---|---|---|
    | TOTAL (30 parts, each column the same before and after) | 3 | 18 | 0 | 0 | 0 | 284 |
    | parts affected | 2 | 9 | 0 | 0 | 0 | 21 |
  - assembly_plate_pin's sheet, diagnosed (d-plate-pin-recovery, no code change): the
    recovery is not the cause and the "try both" claim above was wrong. draftwright's first build
    is already A1 1:1 (its log: every build before the ladder is `page A1`); its tries
    (`scale_decision.attempts`) are scale and view trials on A1 (2:1, 1:1 less the isometric,
    2:1, 5:1) and one A0 2:1 without the isometric, all rejected, so it keeps A1 1:1 incomplete
    (`pmi_dropped` '45'). The port's first plan is A3 1:1, and from there its ladder is
    draftwright's (`try_validity_scales_on_selected_page`, then `try_larger_standard_pages` with
    the scale automatic). First divergence: the bands `choose_scale` is given. draftwright's
    `compose._reserve_authored_dimensions` reserves a tier on each side its route rule gives an
    AP242 dimension; plate_pin has seven X and seven Y linear dimensions hinted to the plan (the
    X ones take above and below, the Y ones left and right), so `_append_anno_corridor_bands`
    makes `right`, `left`, `pv_authored_top` and `pv_bottom` 10 + 7 x 16.5 = 125.5 mm. The
    port's `demand::strips` reserves nothing for PMI dimensions
    (right 36.5, left 20, `pv_authored_top` and `pv_bottom` 0; `Strips::reserve` widens no plan
    band from `dims::lanes`, which counts plan_above 14 and plan_left 9). Measured with the port's
    `layout::choose`, those four bands widened to 10 + n x 16.5: n = 0, 3, 5, 7 give A3, A2, A2,
    A1 at 1:1; the port's plan lanes as bands (top 105, left 70) give A2 1:1. Pinned A1 1:1 the
    port's plan has no hard finding, nothing over a view and nothing dropped, so the over-view gate
    is not involved and the same first choice gives the same sheet. Verdict rust-wrong, owner the
    first choice's bands, not `model::recover`. The change, to follow d-gdt-band-space (it edits
    `demand::size` and `layout::Strips`): port the authored-dimension corridor count
    (`authored_dimension_target_view`, the sides rule of `_reserve_authored_dimensions`, and
    `_append_anno_corridor_bands`: left as the deepest one-view count, front right on top of the
    height ladder, plan right, side right and the six above/below bands at 10 + n x 16.5) as a
    `Demand` field from the lowered PMI dimensions, widened in `demand::strips`, then re-verdict
    the 14 AP242 fixtures' sheets and rerun `tools/quality.py`. **Question for the maintainer:**
    the plan bands this widens are the ones d-quality-lanes' question (1) leaves unreserved for the
    port's own lanes; reserve them for PMI dimensions by draftwright's count (parity, A1 1:1 here),
    or by the port's lanes once that question is answered?
- **PMI on haecceity-written files (d-pmi-haecceity-written, alpha 1 PMI gate):** the 8 files haecceity's PMI writer wrote (quiddity-rust 694dc17 `tests/fixtures/ap242/write/`, as `tests/fixtures/pmi/step/hw_*.step.gz`) are in the reading, lowering, dimensions, drawings and coverage comparisons, captured from draftwright 4dfc63f (no capture errors; draftwright reads `thread_replaced` and `thread_removed` as no PMI at all). 175 new differences, all verdicted: 111 rust-correct, 45 undetermined, 16 equivalent, 3 rust-wrong (`hw_thumbwheel_notes_add`'s thread and knurl requirements #689, #722, #749 not drawn: the same entries as thumbwheel_thread_knurl's, owner the thread/knurl callouts work). Integrated on main after d-pmi-thread-knurl-owner-callouts: the port draws those three on their owners' callouts (`⌀1.6 ↧ 3.8 SEE MFG 1`, `⌀6 SEE MFG 2`, `⌀5 SEE MFG 3`, with the table), so their three `pmi_not_lowered` entries left `known_pmi_coverage.json` (now 1405: 1239 rust-correct, 75 equivalent, 91 undetermined, 0 rust-wrong) and `hw_thumbwheel_notes_add`'s drawings capture was re-run with the extended `tools/capture_pmi_drawings.py` from draftwright 4dfc63f (it now records the owner callouts and the manufacturing table, matching the port's; the other 7 `hw_*` captures came out byte-identical). No haecceity reader defect found: on every record both readers read the same faces and values (OpenCascade's known misreads aside). What the written files show: (1) haecceity writes each intent's fits as the ISO 286 class alone (`LIMITS_AND_FITS`, no tolerance values; specify-core's Python files stated the limits), so the port draws none of the 9 fits in the four intents and draftwright draws their plain sizes: all undetermined under the open ISO 286 question, which now blocks every fit specify-core-rust writes. (2) haecceity's typed thread and knurl features (AP242 `THREAD`, turned knurl, with a 'thread manufacturing' attribute set) are read by haecceity but not projected by `src/pmi.rs` (it projects the requirement-property form), so neither side draws them; the port reports each `pmi_not_extracted` error, draftwright never sees them. The thread/knurl callouts work draws requirement records, so it moves only the 3 rust-wrong entries; projecting haecceity's thread and knurl features is a further gap with no owner yet. (3) Surface textures (`every_kind`: Ra 3.2 on the part, Ra 0.8 / Rz 4 on two faces) and part notes in 'semantic text' attributes (coating, heat treatment, edges) are read but not projected or drawn; draftwright never sees them. **Question for the maintainer:** are such part notes drawing requirements the port should draw as general notes (and report as errors until drawn, not info)? (4) The other differences repeat the Python-written fixtures' causes (the plate and pin's repeated datum letters, the pin placed twice, a datum or size on a conical face in `every_kind`, which puts items on faces by index). Re-checked after merging origin/main `50be9ac` (stc_09's hole-occurrence re-verdict): the five PMI comparisons pass with these entries unchanged, the 8 `hw_*` files are still byte-identical to quiddity-rust's written fixtures (none changed there since 694dc17), and re-running `tools/capture_pmi_drawings.py` on `hw_every_kind` and `hw_thumbwheel_notes_add` from draftwright 4dfc63f reproduces their captures exactly. Gate status for this item: every difference verdicted, none rust-wrong after the thread/knurl merge; it leaves open only the undetermined entries' maintainer questions (ISO 286 class without limits, part placed twice, part notes).
- **Thread and knurl PMI (PMI stage 2c, streams d-pmi-thread-knurl-callouts, d-pmi-thread-knurl-owner-callouts):** drawn as draftwright draws them, on the owner's callout, never as a leader note. `pmi_lowering::manufacturing` ports `lower_ap242_manufacturing_requirements` and `manufacturing_schedule`: each `internal_thread`, `external_thread` or `knurl` record is read from its words (draftwright's three grammars) and its structured statement, reconciled value by value; its one cylinder (`requirement_geometry`, read with the PMI) must be the size its text states; its owner is the one hole (a blind one of the drill depth, or a through one) or turned step (or boss) the cylinder proves, with the external thread length checked against the cylinder or the owner's span and its tip chamfers. The owner's callout prints the suffix (string_post_tapped `⌀1.6 ↧ 4 M2 x 0.4-6H RH; 2.5 MIN FULL THREAD`; assembly_plate_pin's through tap `⌀4.2 M5 x 0.8-6H RH; THRU`, the bore's THRU dropped), or, with two or more owned and one suffix over 52 characters, `SEE MFG n` and a `manufacturing_requirements` plan table (`tables::manufacturing`: `REF | MANUFACTURING REQUIREMENT`, wrapped at 44, column 2 left-aligned, reserved at sheet selection and placed before the notes; thumbwheel_thread_knurl `⌀1.6 ↧ 3.8 SEE MFG 1`, `ø6 SEE MFG 2`, `ø5 SEE MFG 3`, rows identical to draftwright's). Each owner keeps a callout of its own; the callout claims `pmi:<source>` (parameter `thread` or `knurl`), so the coverage lint credits it. What no one feature owns is a `pmi_not_lowered` error with draftwright's reason (unmatched, ambiguous, text against cylinder, words against fields), tested in `tests/pmi_thread_knurl.rs`. `tools/capture_pmi_drawings.py` now records the owner callout that prints each requirement and the placed table's rows (`manufacturing_table`); `tests/pmi_drawings.rs`: 298 differences at haecceity db9d1be (269 rust-correct, 0 rust-wrong, 25 undetermined, 4 equivalent, with d-pmi-angular-dims' and the stc_09 hole re-verdict), the 3 new ones rust-correct (assembly_plate_pin #1526, which draftwright cannot extract; bolt_thread_knurl #444 and #475, which draftwright lowers and plans a table for but whose step diameters its renderer never draws). `known_pmi_coverage.json`: 1312 differences at haecceity db9d1be (1185 rust-correct, 59 equivalent, 68 undetermined, 0 rust-wrong, with d-title-block-material's, d-pmi-angular-dims' and the stc_09 hole re-verdict); the four thread/knurl rust-wrong entries are gone (both drawings state them and neither lint reports them). The 30 quality parts' plans are byte-identical. Not ported: a requirement a boss owns is refused (`dims/bosses.rs` prints none yet); the `pmi_source_text_mismatch` and `manufacturing_reference_unresolved` checks. **Questions for the maintainer:** (1) when the table has no place, draftwright prints the full suffixes on the callouts instead; the port, whose callouts are placed before the table, reports `table_not_placed` (error) and keeps the `SEE MFG n` references: port the fallback (a second dimension pass), or keep the report? (2) bolt_thread_knurl's thread is on the ø10 band the overall diameter states, so the port prints `ø10 SEE MFG 1` on the overall diameter, where draftwright prints nothing and its lint reports the thread not drawn: keep (verdicted rust-correct)?
- **PMI stage 3 (streams d-pmi-coverage-lint, d-pmi-coverage-complete):** the coverage lint
  (`src/lint/pmi_coverage.rs`, `tests/pmi_coverage.rs`, `known_pmi_coverage.json`: 1312
  differences at haecceity db9d1be, 1185 rust-correct, 59 equivalent, 68 undetermined, 0
  rust-wrong; the 16 hole-occurrence entries went with haecceity 694dc17's basic_round_hole and
  the four thread/knurl ones with d-pmi-thread-knurl-owner-callouts).
  nist_stc_10 is compared now it renders.
  Questions for the maintainer: (1) draftwright's lint skips a source its build already explained
  (`*_dropped`, `_EXPLAINED_OMISSION_CODES`); the port's lint cannot read the plan's problems (R1),
  so an omitted frame or a refused dimension is reported by the plan and by lint
  (`pmi_not_rendered`, `pmi_not_lowered`): record the explanation on the drawing, accept the
  double report, or drop the plan's? (2) `step_material_mismatch`/`_disagreement` and
  `step_general_tolerance_*` turn on whether the caller authored the title field, which lint cannot
  read (the Spec): record a field's origin on the drawing, or leave them unported?
- **Title block material (d-title-block-material):** the plan's MATERIAL cell takes the file's one
  usable material when the spec states none (`pmi::Report::title_material`, draftwright's
  `_resolve_title_document_defaults`; a spec value, even blank, wins; several materials, or one of
  several parts, fill nothing), and render names the cell's run `pmi:material:<part>:0`, so the
  coverage lint no longer reports it (`tests/title_block_material.rs`; the text equals
  draftwright's on the 14 AP242 fixtures and cgb203). The 6 `material:0:0` rust-wrong entries are
  gone (1356 differences). The mismatch/disagreement codes still wait on question (2) above.
- **GD&T band space (d-gdt-band-space):** the layout reserves room for the lowered PMI
  (`layout::Strips::reserve_gdt`, `dims::gdt::demand`: each strip's dimensions' room plus its
  frames', datum symbols' and labels' tiers packed first fit at the chosen scale,
  `demand::size` re-choosing until they hold; a new `sv_left` band), and `dims::gdt` stands an
  item beyond the dimensions across from it and prefers a clear place inside its view's area.
  The dimensions' room is the lesser of the band before GD&T and the lane count's band (the
  count overstates the bands facing open paper, which `Strips::reserve` keeps at draftwright's
  estimate); a plan that still leaves an item off the sheet or over another view is made again
  with the tiers beyond the full lane count (`model::plan_escalated`; assembly_plate_pin,
  bracket_positions, stc_09). draftwright reserves none (it drops what its strips cannot hold).
  On the 14 AP242 fixtures against origin/main 50be9ac: GD&T off the sheet 6 → 0, over a view
  15 → 0, past their areas 150 → 41; `tools/quality.py` off the sheet 7 → 0, ink 1692 → 1627,
  every fixture lower but stc_09 (403 → 524, at 1:5 where draftwright keeps 1:1)
  (`tests/gdt_band.rs`; `docs/quality-2026-10-09.md`, "GD&T band space" and "Finished
  (alpha 1)"). The 30 quality parts are byte-identical.
  **Question for the maintainer:** should GD&T room cost scale (draftwright's ladder, reducing on
  the same sheet first, as now) or paper (keep the scale chosen without GD&T and take a larger
  sheet)? stc_09 pinned to A0 at 1:1 is clean of off-sheet and over-view items too, at no less
  ink.
- **Facts:** Python draftwright's sheets recorded for 13 parts (`tools/capture_sheet_facts.py`,
  `tests/fixtures/sheets/`, `docs/facts.md`).
- **Rules:** ADRs R1–R4 accepted. `docs/rules/` has 515 rules learnt from Python; 89 ported
  (adr 25, issues 22, lint 42), 44 translated, 8 retired, 382 pending. **Update a rule's status
  when you port it;** a partial port stays pending with `rust_test` and `rust_note`.
- `docs/plan.md` is the overall plan.
- **Render never panics (d-render-panic-stc10):** a closed NURBS stretch unwrapped past its
  knots (nist_stc_10) is wrapped into the domain (`output::stretch_pieces`,
  `tests/render_panics.rs`); the 30 quality parts and 14 AP242 fixtures all render, no panic.
- **Verify** with
  `DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`
  (45 binaries, 434 passed at 5ca1130), `cargo fmt --check` and
  `cargo clippy --all-targets --release -- -D warnings`. Without
  `DRAFTWRIGHT_CORPUS_REQUIRED=1`, corpus-backed tests skip (print SKIPPED and pass) when the
  corpus is missing; with it, they fail.

**Open maintainer questions** are recorded with the stream entries below and in quiddity-rust's
`docs/review-2026-10-08.md`. Two raised at the end of the run are not written elsewhere:

- Hole tables: draftwright adds one (with balloons) only when its hole placers drop a callout or
  location and escalate (`orchestrator.py` `_prepare_hole_table_run`). The port never drops: it
  places everything and records collisions as rust-wrong. Adopt drop-and-escalate, make hole
  tables spec-driven (a `t:<k>` of kind holes), or leave them out?
- GD&T room (d-gdt-band-space, judgement call, overturnable): the first plan starts the GD&T's
  tiers within draftwright's band estimate where the port's lane count is larger, and only a
  plan that leaves a frame, datum symbol or label off the sheet or over a view is made again
  with them beyond the full count. Taking whichever of the two has less ink instead would lower
  the fixtures' ink 1627 → about 1582 (ftc_10 450 → 413) at twice the planning time for every
  part with PMI; not done.
- Accepted regressions (a1/baseline-regressions, `docs/quality-2026-10-09.md`, "Accepted
  regressions"): of the seven cells worse than the 86ffcbc baseline, ctc02's ink overlaps are back
  at the baseline (a leader through another leader's label now weighs two crossings), and ctc03's
  three cells, ftc08's crossings and ctc04's crossings are judged better drafting (labels and
  locations traded for crossings, or a valid sheet). One is accepted as a judgement, and is
  overturnable: cgb245's crossing between `d:f:slot:2`'s 2× R3 leader and
  `d:f:slot:3/slot_width.length@plan`'s lines (7.0 left of the plan, where draftwright puts it
  right). It has no clear route (tested), and it follows from merged-away lanes (question 9 of the
  alpha 1 readiness record, `docs/alpha1.md` section 6, branch r/d-alpha1-readiness), whose revert
  drops three hole items; draftwright's right-first slot widths keep two crossings on cgb245 and
  make ctc04 worse. Overturning it means the gate is not met on that cell until a rule moves the
  7.0 alone.
- ISSUE-1247-a: draftwright reports an angled step as an explicit unsupported completeness
  outcome; the port has no completeness report. Carry an `unsupported` Problem for each angled
  step (removing it from `dims::UNDRAFTED`), or wait for a completeness report?
- Repeated labels (a1/repeated-labels, judgement call, overturnable): every repeated dimension
  label on the 30 quality parts (16 groups, 18 repeats) has a verdict against draftwright's sheet
  in `tests/fixtures/known_repeated_labels.json`, held by `tests/repeated_labels.rs`. None is a
  quantity draftwright states once or identical features it counts `n×`, so no plan changed.
  ftc10's front `12.5` (hole_pattern 7's z location and the h12.5 rung, one span) is the one
  same-quantity repeat; it is kept as `draftwright-repeats` (draftwright states the span twice,
  front rung and side location, and its rungs take no dedup against hole locations) rather than
  merged, pending alpha 1 question 11 (merge, and which owns it). Answering "merge" removes that
  entry.

- Stable identity (ts/s1-identity, the editor thin slice's S1; judgement calls, overturnable;
  `docs/model.md`, "IDs"):
  1. A PMI record's identity uses the drawing's face and edge indices, not their
     `ADVANCED_FACE` instances as the editor's `gdt::editor_key` did: no Part 21 number enters
     an ID, and a rewrite that keeps the geometry keeps the indices.
  2. A record's label enters its identity only to tell apart records that otherwise share it
     (composite frames on NIST FTC-07, FTC-10, STC-09), so a changed magnitude keeps an ID.
     Records alike in their labels too (STC-09's two `0.03 ±0.01 INCH`, FTC-10's three `ø3`, all
     already undrawn for other reasons) get no ID and are refused.
  3. A migrated `draftwright.spec/1` key does not keep its `@<view>` as the entry's `view`: the
     view was where the plan routed it, not a decision, so a migrated spec plans as before and
     an omitted view never refuses it.
  4. `dimensions/<id>/view` naming an omitted view leaves the dimension in its default view with
     a warning (`dimension_view_not_shown`), where the editor's fork refused the plan; a view
     that cannot carry it is refused. S3's `place.view` may absorb the field.
  5. Coincident GD&T leaders still take their tiers in source order (Part 21 text), as before,
     so no drawing changed; a rewrite that renumbers may reorder two frames whose leaders stand
     together (their IDs stay). The manufacturing table's tags keep draftwright's source order.
  6. Problems about PMI records still name the record's source (`p:pmi_not_lowered:<source>`):
     the record is not an item. Typed subjects are S2's.
  7. The layout's lane reservation (`dims::lanes`) does not see a spec's `view`.
  8. `edit` has no analysis, so it leaves a version 1 spec's version and keys as they are; `plan`
     migrates (and its embedded spec is version 2).

- Typed state and problems (ts/s2-typed-state, the editor thin slice's S2; judgement calls,
  overturnable; `docs/model.md`, "Plan"):
  1. Built on S1 as the integrator merged it (`ts-int/s1`, 0c72672): `origin/main` did not yet
     have S1 when S2 started.
  2. A `fix` is a list of `edit` operations (`{op, path, value?}`), applied together, not the
     plan's `{op: "place", …}` example: `place` is S3's, and a fix must be one `edit` accepts
     today. Fixes are given only where the edit resolves the problem (each checked by applying it
     and planning again); none for placement collisions until `place` exists, none where the
     remedies are a matter of intent (`step_general_tolerance_disagreement`: the spec's
     tolerance or the PMI's), and none for a turned part's left-out plan view (the message's
     `set views/plan to {}` is not checked).
  3. Where a message offers two remedies the fix takes the one that keeps the author's intent:
     an omitted view restored rather than the dimension's chosen view removed
     (`dimension_view_not_shown`, the message's first); an omitted dimension brought back
     rather than its tolerance removed (`tolerance_not_shown`; a tolerance on a covered or
     unauthored dimension is removed, there being no edit that shows it); a fit no callout
     states is removed (the callout that would state it is not tracked there).
  4. `subjects` hold only live item IDs: the plan's items, the sheet (`sheet`, its ID by
     convention: `PlannedSheet` has no `id` field) and the Analysis's features and hole
     patterns. Everything else moved to a new `refs`: PMI record sources (S1's decision 6),
     sources not read, recogniser families, the undrawn section `v:section_aa`, the height
     ladder `step_level`, title-block cells (`title_block/<field>`, whose problems now name
     `sheet`), spec entries not applied, and a schedule's threads. A problem about the whole part
     (`pmi_not_read`, `pmi_presentation_not_read`) has no subjects and names the part in `refs`
     (`["part"]`; restored after review, as `spec_from_other_revision`'s own subjects stay empty). Problem IDs are unchanged.
  5. `severity` stays a string (`error`, `warning`, `info`): its JSON is the same as an enum's
     would be, and some 70 sites and many tests compare it as text.
  6. A view omitted by the spec is not added to the plan as an item (`views` lists drawn views,
     which renderers draw); its absence is the state. The isometric that projects no edges is
     `shown` (it is planned and drawn, empty) with its `iso_projection_empty` warning.
  7. An item dropped from a spec entry for another revision (`inactive:` basis) is
     `not_authored` in an authored set, as that is what happened to it; the editor's prose review
     called it unplaced.
  8. The label of an omitted dimension is its label when it was left out: a spec-omitted one's
     default label (without the spec's tolerance, which `tolerance_not_shown` reports), a
     covered one's as planned. A frame's label joins its compartments with ` | `, as the editor
     shows it.

**Housekeeping:** clean leftover worktrees in quiddity-rust can be removed
(`.claude/worktrees/agent-a461c0d2e1bd3494a`, `wf_7fc79fa8-4f8-162`, `wf_7ff99f80-ece-23`,
`wf_7ff99f80-ece-24`, and the scratchpad `wt-ap242`); their branches are merged or superseded.

**Python references (read-only):**

- draftwright: references were taken at origin/main 586628b; origin/main is now 4dfc63f (one
  commit later, 2026-10-07). Read it with
  `git -C ~/repos/draftwright show origin/main:<path>` (the working tree is on a stale branch).
- Run it with `~/repos/draftwright/.venv/bin/python` and `PYTHONPATH` pointing at a source
  export (`git -C ~/repos/draftwright archive origin/main src | tar -x -C <dir>`).
- quiddity is at `~/repos/quiddity`, with its venv in `.venv`. At 6d544e0, the ref CI checks
  out, its corpus files match the sha256 bindings of every sheet-facts and corpus recognition
  fixture.

## Taking a new haecceity

See README.md, "Taking a new haecceity".

## CI

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests: the toolchain from
`rust-toolchain.toml`, `cargo fmt --all --check`, clippy with `-D warnings`, and
`cargo test --release` with `DRAFTWRIGHT_CORPUS` at a sparse checkout of the public
`pzfreo/quiddity`'s `tests/corpus` (pinned to 6d544e060cd0eeb4442d2196e5df757b5a15ea55) and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Since d-ci-fast (d3b92f8) the build compiles every test binary
once into a nextest archive and the tests run in parallel shards (`.config/nextest.toml`); about
11 min per push, every test still run.

haecceity comes by git from the public `pzfreo/quiddity-rust` (Cargo.toml, pinned in
`Cargo.lock`), so CI needs no secret.

When fixtures are re-captured against a changed corpus file, move the workflow's `ref` to a
quiddity commit that has it.

## How we work

- **Accuracy over parity.** Matching Python is a stepping stone. Every difference gets a verdict
  in a known-differences list: rust-correct, rust-wrong, equivalent, undetermined or
  not-applicable, with evidence.
- **One worktree and branch per stream.** In quiddity-rust, `isolation: 'worktree'` works when
  the session starts there. For draftwright-rust, create worktrees under the scratchpad.
- **Each stream gets a skeptical review and one fix round.** An integrator then merges locally
  and runs the full checks.
- **The main session re-verifies before pushing.** Never push unverified, and never publish to
  anyone without the maintainer's explicit permission.
- **Commit attribution:** messages end with the Co-Authored-By and Claude-Session lines the
  session gives.
- **Maintainer decisions to respect:**
  - lint is a module with a guard test that it never reads the Plan;
  - both dimension modes stay: defaults, and an opt-in complete set;
  - there are no declared builds;
  - placement offsets are paper millimetres and a request;
  - the safe repairs run automatically and are recorded;
  - replies carry everything an LLM needs to fix a drawing, as in R4's decision 5.
- **Maintainer decisions, 2026-10-08:**
  - **Keep porting.** The maintainer has said "continue the port". The autonomous run keeps
    dispatching implementation streams from the backlog until it is exhausted. A status question
    from the maintainer ("how is it going") never pauses dispatch.
  - **Hole locations:** a hole's or hole pattern's in-plane position is dimensioned in the view
    that sees it end-on (the port's choice, ADR1-6), not in the side view as Python does. The 20
    pending side-view routing entries in `tests/fixtures/known_dimensions.json` become
    rust-correct with that reason.
  - **Spotface:** the dedicated ASME Y14.5-2018 spotface symbol (the counterbore symbol with SF
    inside), in both the dimension text and lint's fix text; the glyph must be in the font (and
    searchable in PDF).
  - **Slots and bosses:** a slot is also located across its width, and bosses are located (both
    beyond Python, rust-correct).
  - **Off-axis parts (quiddity-rust):** recognition canonicalises the frame first. It recognises
    in the part's own principal frame (the ported `frames.py` path) and reports results in the
    file's coordinates, so results don't depend on how the part was placed. This is the default
    for `recognise`, the CLI and the recognition document.
  - **AP242** decisions are in quiddity-rust `docs/step-ap242.md` (ap242 branch).
- **Priority for the next dispatch (main session, 2026-10-08)** (done: q-seam-fix merged at
  e6234b6; Linux CI green from 384992f until the uv-bounds merge, see "Where things stand"):
  quiddity-rust's GitHub CI (Linux)
  has been red since the first run on one platform-dependent count (cgb202 torus
  `uv_bounds … another turn`, 13 listed, 12 found). The fix is ready on quiddity-rust's local
  branch `q-seam-fix` (commit 1966d3c, "uv: place a loop centred on the seam deterministically",
  with its known-divergence updates; full checks passed on macOS against 1bf3db2). Dispatch a
  stream `q-seam-fix` whose brief is: rebase that commit onto the current branch head, re-run
  the full checks, adjust the known-divergence counts the newer merges change, keep the 1e-9
  seam band and its reasons; then the integrator merges it like any other stream.
- **Priorities from a CLI trial (main session, 2026-10-08, afternoon)** (all five done: views
  memory by d-coincident-locations, CLI native default by d-cli-native, annotations on the sheet by
  d-placement-fit, turned steps by d-dims-turned-steps, ctc04 by d-bridge-area-tolerance): the
  maintainer is trying
  `draftwright render` on corpus parts (at 98deec8). Dispatch these ahead of other backlog work:
  1. **Memory blow-up in `tests/views.rs`.** Since the restart, a guard has killed the `views` test
     binary 15 times at 16–26 GB, always in stream worktrees under the session scratchpad; the
     same test on main uses ~150 MB in 9 s, including in a fresh worktree of origin/main. Find the
     cause (a stream change, or something about how streams run it) and fix it; streams lose
     whole runs to it.
  2. **The CLI uses Rust quiddity's recognition.** `render`/`plan`/`analyse`/`review` without
     `--features` should take features from quiddity-rust's recognition document
     (`src/bridge/native.rs`), keeping `--features` (Python JSON) as the override; README updated.
  3. **Annotations stay on the sheet.** On cgb245 `render` reports 10 `annotation_off_sheet`
     errors (left-hand location ladders cut off by the border) and leaders run across the sheet
     to the isometric view. Port the placement/fit rules that keep annotations inside the
     drawing area (pending in docs/rules), or pick a larger sheet as draftwright does.
  4. **Turned parts' step diameters.** On GRM-05 only ⌀10 and the overall 32 are dimensioned;
     the smaller step diameters and their lengths are missing.
  5. **The bridge refuses on tiny area differences.** `render` refuses nist_ctc_04 entirely
     ("face 367 has area 13.080630 here and 13.100238 in the recognition JSON", 0.15%). Decide a
     principled tolerance (with evidence that the faces are the same) instead of exact agreement.
- **Top priority from the maintainer (2026-10-08 evening): make quiddity-rust's CI much
  faster** (done in part by q-ci-fast, ea4eda7: Linux about 10 min, macOS about 20 min; under 10 on
  macOS needs a decision on runners or kernel speed; draftwright-rust's CI is about 11 min)
  ("I'd like to speed it up a lot"). Then 37–57 min per push (run 37791515187 onward):
  almost all of it is `cargo test --workspace --release`, which runs its 27 test binaries one after
  another on a 4-core runner (Linux: 52 min of tests after a 4 min compile; recognition.rs 13.1 min,
  invariance.rs 11.5, correspondence.rs 6.7, section_recess_helpers.rs 4.0, determinism.rs 2.7,
  passages.rs 2.2, drawings.rs 2.1, corpus.rs 1.9, wiring.rs 1.9; macOS similar). Nothing is cached.
  Dispatch a quiddity-rust stream `q-ci-fast` (owns .github/workflows/ci.yml and may split the slow
  test files) with the target **under 10 minutes wall clock on both platforms, with no test dropped
  or weakened**:
  1. cache the build (Swatinem/rust-cache, pinned by SHA like the existing actions);
  2. build the test binaries once per platform (`cargo nextest archive`) and run them sharded
     across parallel jobs (`cargo nextest run --archive-file … --partition hash:k/N`), N chosen
     from measured times; keep fmt and clippy as a separate fast job;
  3. split the slowest tests so they shard: where one test iterates the whole corpus (recognition,
     invariance, correspondence, section_recess_helpers), make it one test per family or per
     corpus slice, keeping every assertion and its failure messages;
  4. keep macOS running the full suite (maintainer decision H1: float pins must hold on both),
     sharded the same way;
  5. measure before and after on a pushed branch or a workflow_dispatch run, and record the
     numbers in the status file. Do the same for draftwright-rust's CI afterwards if it is cheap
     (now ~20 min, one job).
- **Priority from the maintainer (2026-10-08 evening): draw PMI** (stage 1, reading, done by
  d-pmi-read; lowering and coverage lint are the next draftwright backlog items). When this was
  written draftwright-rust read no PMI (only Spec tolerances on dimensions); Python draftwright
  reads AP242 semantic PMI
  (pmi.py, _pmi_part21.py, _pmi_schema.py, _pmi_topology.py, _pmi_datum_geometry.py,
  _pmi_linear_geometry.py, _pmi_support_blockers.py), lowers it to drawing items
  (model/pmi_lowering.py: feature control frames and datum symbols, annotations/_gdt.py;
  toleranced dimensions, annotations/_pmi_dimensions.py) and lints coverage
  (linting/pmi_coverage.py). As soon as `q-merge-ap242` has merged and been pinned here (haecceity's
  `pmi` model and reader: semantic datums, tolerances and dimensions anchored to faces, checked on
  NIST CTC/FTC), dispatch draftwright-rust streams to: (1) read the part's PMI through haecceity
  into the Analysis (fail closed: what the model reports as a finding is a problem, not guessed);
  (2) port the lowering to Plan items and their rendering: feature control frames, datum feature
  symbols, toleranced and basic dimensions, in the views draftwright chooses; (3) port the PMI
  coverage lint; (4) test on NIST CTC/FTC files and specify-core-written files (quiddity-rust
  tests/fixtures/ap242), each compared with Python draftwright's drawing, every difference given
  a verdict; update the PMI rules in docs/rules. Split so streams own disjoint files.
- **Drawing PMI, stage 1 (stream d-pmi-read): the part's PMI is read.** `draftwright::pmi::read`
  (`src/pmi.rs`) reads a STEP file's semantic PMI through haecceity's reader into draftwright's
  records (`extract_pmi_report`): dimensions, geometric tolerances and datums, each with its
  drawing faces (haecceity's indices, as the bridge uses), their box in the file's frame, the
  instances it was read from and its blockers; and a census in which every haecceity finding,
  every item this stage does not project and every record whose face does not resolve is a source
  not read, with its reason. Not yet attached to the Analysis (lowering's job). `tests/pmi.rs`
  compares record by record with draftwright on the 14 AP242 files copied from quiddity-rust
  (`tests/fixtures/pmi/`, `tools/capture_pmi.py`) and the corpus's 10 NIST parts (no PMI on
  either side): of draftwright's 592 records, 358 agree in every compared field (dimensions
  153/175, tolerances 147/204, datums 58/158); 330 differences listed in
  `tests/fixtures/known_pmi.json`: 272 rust-correct (OpenCascade misreads draftwright inherits:
  magnitudes in complex instances read as 0, g6/f7 fits garbled, a radius's nominal taken as its
  lower limit, datum letters repeated per occurrence, valueless dimensions typed `location`;
  XCAF occurrences draftwright cannot coalesce; patterns XCAF names one member of; datum faces
  draftwright cannot find; tolerances XCAF drops silently: all-over profiles, unit-basis
  flatness and straightness, positions on supplemental geometry), 55 rust-wrong (draftwright's
  manufacturing requirements, surface labels and common labels, not projected yet: reported as
  sources not read), 3 undetermined (below). Left for later stages: manufacturing requirements,
  surface and common labels, material facts; lowering's geometry (`ref_pts`, cylinder and angular
  supports, datum normals: derived from the faces then); datum targets, composite relations,
  projected zones and datum modifiers (record blockers). Proposed catalogue changes (issues.yaml
  and adr.yaml belong to d-issue-drafting-4; statuses stay pending):
  - LINT-code-pmi_not_extracted: rust_test `tests/pmi.rs::every_item_and_finding_is_accounted_for_and_the_output_is_deterministic`;
    rust_note "Partly: `pmi::read` inventories every dimension, tolerance and datum and reports
    every haecceity finding, unprojected item and unresolved face as a source not read with its
    reason; the lint code on a drawing is the coverage lint's (not ported)".
  - ISSUE-1116-a, ISSUE-1116-b, ISSUE-1209-a, ISSUE-1296-b, ISSUE-1298-b, ISSUE-1298-c,
    ISSUE-1325-a, ISSUE-1357-d, ISSUE-1563-a, ARCH0016-7: rust_note "AP242 PMI is read
    (`src/pmi.rs`, `tests/pmi.rs`: records, faces, census) but not lowered: …" in place of "the
    port reads no AP242 PMI".
  - Questions for the maintainer:
    - A part placed several times (quiddity-rust's assembly fixture: the pin twice): a record
      lists the drawing's faces of every placement, as the drawing draws them all. Keep, or
      should a drawing of an assembly choose one placement (or refuse)?
    - Semantic dimensions on supplemental geometry whose only presentation is invisible
      (NIST FTC-10's three Ø3, `INVISIBILITY`): read as records with a blocker; draftwright never
      sees them. Should lowering draw them, report them, or skip them as not meant to be shown?
    - An ISO 286 class stated without limits (FTC-10's three Ø3.50 G6): the record keeps the
      class and a blocker; haecceity has no ISO 286 table. Compute the limits (a table to add),
      or draw the class alone?
  - Stage 1b (d-pmi-manufacturing): requirements, surface and common labels are records (0 rust-wrong), each `pmi_not_lowered` warning/info until drawn; question: draftwright's lint makes that an error for a kind it draws, keep warning/info?
- **Maintainer decision (2026-10-08 evening): merge AP242 into quiddity-rust now** (done:
  q-merge-ap242, 48dbe43). The `ap242`
  branch (pushed, at a16d7af: lossless Part 21, schema validation, semantic PMI reader and
  writer, removal plan, `quiddity parts` / `quiddity pmi read|write`; four stages each integrated
  green on the branch) is to join `prototype/rust-port`. A new repo, specify-core-rust, depends on
  it. Dispatch at once a quiddity-rust stream `q-merge-ap242`: from origin/prototype/rust-port,
  merge `ap242` (resolve conflicts keeping both sides' intent: the branch forked at 7f4b1a9, and
  both sides changed crates/haecceity/src/lib.rs, step.rs, src/bin/quiddity.rs, README.md and
  fixtures), run the full checks including the AP242 suites, update README's status for the
  merged capabilities; then the integrator merges it like any other stream.
- **Top priority from the maintainer (2026-10-08 evening): callout leaders** (done by
  d-callout-leaders, 9a4af8f; interior label candidates and outline-based checks still open).
  "Something is
  wrong with the leaders, especially diagonals." Callouts are stacked in lanes beyond every linear
  dimension with a fixed 45° leader (`src/dims/placement.rs` `place_callout`, `turned`), so on a
  part with location ladders the leaders run across the view, the ladders and the sheet. Even
  with d-placement-fit (98190ba) on cgb245, three callouts (2× R3, 2× ⌀6, 4× ⌀10) still sit at
  the sheet's top with diagonals across the plan view, and those moved to the right fan across
  the view and its ladder. Python draftwright on the same part (the reference; render it with
  `draftwright cgb245.step`) puts each callout just outside the view, level with or near its
  hole, with a short leader and a horizontal shoulder that crosses no geometry or dimension.
  Dispatch one stream (after d-placement-fit merges, same file) to port draftwright's leader
  placement: annotations/_leader_candidates.py, leader_policy.py, _placement_occupancy.py and
  the pending leader rules in docs/rules (crossing, length, angle, side). Accept: on cgb245,
  ctc04 (native) and the 13 captured sheets, no leader crosses its own or another view's
  outline or another annotation, leader lengths are comparable to Python's (report the
  distribution), and the rendered PDFs are compared visually with Python's.
- **Main-session decisions, 2026-10-08** (judgement calls; the maintainer can overturn them):
  - fingerprints belong to quiddity (ADR R3). Once quiddity-rust publishes its recognition
    document, `bridge.rs`'s own fingerprint is retired in favour of quiddity's;
  - fingerprints and correspondence use only verified occurrences;
  - `plan`, `render` and `review` accept an Analysis document, as `docs/model.md` says, so a
    caller need not re-analyse;
  - quiddity-rust's CI also runs on macOS, beside Linux.

- **The run ends with alpha 1 (maintainer, 2026-10-09):** once the gates in `docs/alpha1.md`
  are met, dispatch nothing new (return no streams); let in-flight streams finish and integrate.
- **Milestone: alpha 1 (maintainer, 2026-10-09). Read `docs/alpha1.md`; it governs dispatch.**
  Until its gates are met, dispatch only alpha 1 work, in this order: the quality streams and PR #1
  (already queued), PMI stage 2b (dimensions from PMI) and stage 3 (coverage lint) and the
  `known_pmi.json` rust-wrong entries, the plan speed-up, then the quiddity-rust items (haecceity
  writer decisions, section recesses, specify-core-rust's upstream needs, CI). Re-run
  `tools/quality.py` after each draftwright-rust integration and record the table in the status
  file. Report each gate's state in the status file. The releases are cut by the main session.
  - Release hygiene (d-alpha1-readme-changelog): README.md (what alpha 1 does and does not do)
    and CHANGELOG.md (`0.1.0-alpha.1 (unreleased)`) were written at origin/main 15629f1 (PMI
    stage 1b included); the main session must refresh both before cutting the release for what
    merged after: d-quality-merged-lanes, d-pmi-drawings-check and the PMI coverage lint (stage 3).
- **Merge the maintainer's PR #1 first (2026-10-09):** "Render: keep each view's projection
  across renders; box straight pieces by their end points" (branch `perf/render-cache`, by the
  maintainer; read its description with `gh pr view 1`). Dispatch at once a stream `d-pr1-render-cache`
  that branches from origin/main, **merges** `origin/perf/render-cache` into it (git merge, not
  rebase or cherry-pick, so the PR's commits land on main unchanged and GitHub marks the PR merged),
  resolves any conflict with what main gained since (keeping both intents), runs the full checks,
  and confirms the speed-up (render time on NIST FTC 06 and CTC 02 before and after) and that the
  SVGs are byte-identical before and after on the 30 quality parts. Do not comment on, close or
  edit the PR. The integrator merges it like any other stream.
  - Merged on branch `r/d-pr1-render-cache` (clean merge, no conflicts). CLI `render --analysis`,
    release, median of 5 on a loaded box: FTC 06 9.80 s → 1.39 s, CTC 02 19.76 s → 12.00 s (one
    fresh render, so the gain is the box fix; the kept `Projections` helps later renders). SVG
    byte-identical on all 30 quality parts. Plan on FTC 06 0.37 s either way.
- **Performance is part of quality: `plan` became much slower** (the PR's description): on NIST
  FTC 06 from 2 ms to about 490 ms natively (about 1 s in WASM; CTC 01 from 1.5 ms to 34 ms),
  between c1933ad and 2eee576, now about 70% of an edit in the maintainer's browser editor.
  Suspects: callout-leader placement (7eae60f) and the placement fall-through (98190ba, 4061d81).
  Bisect, find the cause and make placement fast again with the drawings unchanged (byte-identical
  plans on the 30 quality parts), reporting plan times before and after; coordinate with the
  in-flight quality streams that own src/dims/placement.rs (dispatch after them if they conflict).
  - Done (stream d-plan-speedup, `src/dims/leaders.rs`, `placement::crosses`). **Cause:** the
    callout leaders' joint assignment (7eae60f). On FTC 06 one group of 10 callouts with 40
    candidates each runs to the search's 200 000-node bound on every plan, and each node re-sorted
    the next job's 40 candidates after summing their crossings through a BTreeMap lookup per
    earlier job: about 95% of the plan. The search now keeps conflicts dense (a word per
    candidate) and takes each job's candidates lazily in the same order (same nodes, same order,
    same bound; a new test checks the node trail against the sorted search). The rest of the time
    was the candidates' measurement (about 12 000 candidates per callout, each scanning all the
    placed ink): a grid finds the ink near a candidate, labels clear of its ink box are skipped,
    each leader's direction is worked out once, `crosses` rejects a segment wholly beside the box
    before clipping, and the measured keys are sorted instead of the candidates. Plans (JSON) and
    SVG renders are **byte-identical** with the base (ca792c8) on all 30 quality parts, from the
    same saved Analyses.
  - Release build, saved Analysis, median of 5, on a machine with load average about 55 (18 cores),
    so absolute times run high; "plan" is one `model::plan`, "recovered" is `model::plan_recovered`
    (what the command line and editor get); Minstr is instructions per call, which load does not
    change:

    | part | plan before | plan after | recovered before | recovered after | Minstr recovered before → after |
    |---|---|---|---|---|---|
    | NIST FTC 06 | 285 ms | 12.9 ms | 250 ms | **12.9 ms** | 4841 → 207 |
    | NIST CTC 01 | 21.7 ms | 11.6 ms | 21.8 ms | 11.7 ms | 354 → 170 |
    | NIST CTC 02 | 680 ms | 106 ms | 2866 ms | 630 ms | 38821 → 5668 |
    | NIST CTC 04 | 300 ms | 68 ms | 3259 ms | 506 ms | 32732 → 5355 |
    | NIST FTC 08 | 657 ms | 122 ms | 3149 ms | 830 ms | 20782 → 3056 |
    | NIST FTC 09 | 388 ms | 91 ms | 2874 ms | 561 ms | 23544 → 8133 |
    | NIST FTC 10 | 125 ms | 17 ms | 260 ms | 70 ms | 5713 → 1387 |
    | NIST CTC 03 | 91 ms | 24 ms | 510 ms | 117 ms | 4814 → 840 |
    | cgb245 | 35 ms | 16 ms | 286 ms | 113 ms | 6084 → 2231 |

    The gate (FTC 06 under 50 ms natively) is met. Of the other parts on the list, CTC 05 and
    FTC 07 take about 100 ms recovered (5 plans each, about 10 ms a plan), cgb207 about 22 ms,
    and the rest under 5 ms.
  - **What remains, for the slow parts:** (1) layout recovery multiplies the plan: CTC 02 and
    CTC 04 make 4 full plans per `plan_recovered`, FTC 08, FTC 09, CTC 05 and cgb245 make 5 (the
    first and each try in `model::recover`; the hole-table escalation adds no re-plan). That is
    model.rs, not this stream's: proposal for the maintainer, not decided: let the tries share
    what does not depend on scale and sheet, or evaluate the bounded list of tries in parallel on
    native builds and take the same first acceptable one (no WASM gain). (2) Within one plan, the
    leaders' candidate measurement is still about 70% (CTC 02: 22 callouts, about 90 000
    candidates per plan), and `layout::fit_box`'s sorting about 8%. `tools/plan_timing.py` times
    `plan --analysis` per part.
  - Stream d-plan-recover-speed (measured on 4ff1b94, no code change kept): the tries share almost
    nothing worth sharing. What does not depend on scale and sheet (`demand::read`, `dims::lanes`,
    `dims::wants`) is 0.1-0.3 ms of a 17-130 ms plan, under 0.2% of `plan_recovered` (CTC 02 5798,
    CTC 04 5388, FTC 08 3125, FTC 09 7735, CTC 03 888, cgb245 2282, FTC 06 255 Minstr); in each try
    `dims::leaders::place` is about 97% of `dims::plan`, and a try that escalates to a hole table
    (CTC 02 at A1 1:2) spends about 190 ms more in `hole_table::place_table` → `tables::place` →
    `layout::fit_auxiliary_box` → `fit_box` (one block count after another, up to four), whose
    candidate grid grows as the square of the obstacles (every dimension's ink). Faster recovery
    needs those (leaders.rs, layout.rs), or fewer tries (a maintainer decision).
    `tools/plan_timing.py` now gives CPU time and, on macOS, instructions retired.
- **Maintainer directive, 2026-10-09: quality before features in draftwright-rust.** "Rather
  than add new features I'd rather that we fixed the obvious problems with the current code: out
  of bounds, repeated dimensions, overlapping views, overlapping ink." Until these are fixed, give
  every draftwright-rust slot to them and dispatch no new draftwright-rust features (PMI drawing,
  angular dimensions, note leaders and the like wait; the in-flight PMI lowering stream finishes
  and merges as it is). Baseline and method: `docs/quality-2026-10-09.md`, tool `tools/quality.py`
  on `tools/quality_parts.txt` (30 parts): 34 annotations off the sheet in 6 parts, 50 repeated
  labels in 15, 34 overlapping view pairs in 16 plus one view on the title block, 506 ink problems
  in 24. Streams by problem, owning disjoint files: (1) out of bounds: nothing off the sheet
  (escalate scale or sheet, or the hole table now decided, as draftwright does; never clip);
  (2) repeated dimensions: state each quantity once and group identical features with n× as
  draftwright does, judged per case against Python draftwright's drawing of the same part;
  (3) overlapping views: layout reserves each view's bands so no view, band or title block
  overlaps another; (4) overlapping ink: leaders that cross, text on lines, annotations over
  views, by placement candidates that clear existing ink. Each stream reports the tool's table
  before and after on all 30 parts (no part may get worse) and compares its worst parts' PDFs
  visually with Python's. Re-run the tool and update the baseline doc at each integration. The
  hole-table and angled-step decisions below are part of this work where they remove problems.
  - Quality 3 (r/d-quality-view-overlap): the layout reserves every view's bands (isometric clear
    of the full blocks, section gap, exact abutment, row lifted off the title block or not fitting):
    overlapping view pairs 33 to 0, views on the title block 1 to 0, off the sheet 33 to 27
    (`docs/quality-2026-10-09.md`, "After Quality 3"). Question for the maintainer: cgb245 trades
    its off-sheet location for one leader crossing (ink 27 to 28, rust-wrong in placement:
    draftwright carries the plan's Y locations in the side view); accept the trade until the ink
    stream moves that ladder, or hold Quality 3 until it does?
  - Hole tables (d-hole-tables, `src/hole_table.rs`): hole items placed off the sheet are dropped,
    reported and escalated as draftwright does; off the sheet 33 → 3 (the three slot locations),
    no table kept on the corpus (as draftwright), `docs/quality-2026-10-09.md` "After hole tables".
    Question for the maintainer: ftc09's 18 front-view hole locations and one pattern callout are
    now dropped and reported (`off_axis_location_dropped` / `callout_dropped`, as draftwright drops
    them), not drawn off the sheet. Keep draftwright's rule that off-axis drops never tabulate, or
    give the front view its own table?
  - Quality 4 (d-quality-ink): ink 486 to 458; three questions (ctc03 21 to 24 via the recovery taking A2; draftwright's text gate; a post-assignment leader repair) in `docs/quality-2026-10-09.md`, "After Quality 4".
  - Integration r-draftwright-rust-10 (all four merged): off the sheet 33 to 3, view overlaps 30 to
    0, title block 5 to 0, ink 448 to 382; only ctc03 worse (ink 21 to 24). Question for the
    maintainer: ftc07 now goes to A0 at 1:1 through the recovery (draftwright A2 at 1:2); weigh
    sheet size or ink in the recovery, or accept? (`docs/quality-2026-10-09.md`,
    "Integration r-draftwright-rust-10").
  - Lanes (d-quality-lanes): on the integration's combined baseline (off 3, ink 382, re-measured identical at f4b2e8e) the layout reserving the band right of the front and the plan for the front's lanes: ink 382 to 279, labels over a view 15 to 0, lines across a view 36 to 0, no part worse, ftc07 to draftwright's A2 1:2 directly (the integration's A0 1:1 case is gone; whether the recovery should weigh sheet size stays open for ctc03); off the sheet stays 3 (`docs/quality-2026-10-09.md`, "After the three quality merges and lane reservation"). Questions for the maintainer: (1) reserve the bands facing open paper too (below the front, above and left of the plan)? With all of them off the sheet is 0 and ink 170, but 10020 gets worse (A4 2:1 to 1:1, ink 1 to 2) and 11 parts change sheet or scale, cgb245, ftc08 to 1:5 and ctc01 to 1:10 against draftwright's 1:2, 1:2 and 1:5, because the lanes count callouts, every rung and merged-away items (the open questions on those decide it); (2) draftwright puts the plan's Y locations above the side view: adopt it (needs `dims::placement::strip` to stack horizontal side locations above the side, d-plan-speedup's file)? (3) the remaining off-sheet items are slot locations, which draftwright drops and reports (`slot_dim_dropped`): extend drop-and-escalate to them? Open item: the port's over-view gate in `model::recover` (a try with more items over a view than the first is not taken) has lost its only corpus case (ftc07) and is untested; it needs a stand-in test in tests/page_escalation.rs or a decision to drop the gate.
  - Merged-away lanes (d-quality-merged-lanes): items merged away share their survivor's lane (draftwright's corridor dedup), so 3 hole items are no longer dropped and outer lanes close up, but ink 279 to 284 (crossings 98 to 108 on ctc04, ctc02, cgb245: items stay in their own strip, away from their features); the 98 base crossings are 71 of 73 inventory-limited (no ordering or cost fix); the over-view gate has a stand-in test. Question: keep, or let strips prefer the feature's side first? (`docs/quality-2026-10-09.md`, "After merged-away lanes").
- **Maintainer decisions, 2026-10-09** (answers to recorded questions; dispatch these ahead of
  other backlog work):
  - **Hole tables:** adopt draftwright's drop-and-escalate. When the hole placers cannot place a
    callout or location ref, drop it and escalate to a hole table with balloons, as
    orchestrator.py `_prepare_hole_table_run` does.
  - **ISSUE-1247-a:** report an `unsupported` Problem for each angled step now (remove it from
    `dims::UNDRAFTED`), until it is dimensioned or a completeness report exists. Done (stream
    d-angled-step-unsupported): `p:angled_step_requirement_unsupported:<id>`, a warning.
  - **quiddity-rust macOS CI:** keep the full suite on macOS (decision H1 stands), sharded across
    more parallel jobs to reach about 10 minutes, like Linux.
  - **haecceity's PMI writer (quiddity-rust),** from specify-core-rust's questions Q16, Q20 and Q3
    (`~/repos/specify-core-rust/docs/questions.md`, upstream needs U3 and U15):
    1. **Do not write the `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` relating datum feature
       symbols to the part.** OpenCascade 7.9's STEP reader crashes (SIGSEGV) on every file that
       has it; without it all seven test files load and read correctly in OpenCascade. Record it
       as a deliberate departure from the full form, to revisit when OpenCascade is fixed.
    2. **A size or tolerance on a feature of several faces: one `geometric_item_specific_usage`
       per face** (PMI practice §5.1), not one `item_identified_representation_usage` of a
       `set_representation_item` (§6.5.1), which OpenCascade drops along with the datums only
       it cites.
    3. **Notes and surface finish in standard forms:** surface texture as AP242's
       `surface_texture_parameter` (with the practice's representation), and other part notes
       (coating, heat treatment, edge breaks) as standard text notes, so specify-core-rust can
       write what is now listed as `not_written`. Verify each by read-back and an OpenCascade
       cross-check, with verdicts.
    Then specify-core-rust is moved to the new haecceity by its own run.

## Open leftovers from earlier reviews

The 2026-10-08 review (`docs/review-2026-10-08.md`) supersedes the old spike list: its
improvement plan and questions are the current work. Still open from before it:

- The MCP server hasn't been tried in a real client.
- PDF fonts are subset (stream d-font-subset, `src/output.rs`, `tests/output.rs`): each embedded
  font program holds only the glyphs the sheet's text shows, `.notdef` and the parts of composite
  glyphs (the `subsetter` crate, MIT/Apache-2.0, without its variable-font feature), tagged
  `ABCDEF+name` with the tag drawn from the glyphs kept. The CIDs, `/W` widths, ToUnicode map
  and content stream are unchanged; a `CIDToGIDMap` maps each CID shown to its glyph's new ID.
  Measured: ctc01's PDF 244,241 → 146,131 bytes, m10127's 174,017 → 75,836, string_post's
  165,159 → 66,529; the plate's (counterbored) 113,025 → 58,883. PDFium renders the same
  pixels, and PDFium and Poppler extract the same text and character boxes, before and after;
  `tools/compare_pdf.py` reports the same against draftwright's captures. Differences from
  draftwright, with verdicts:
  - draftwright subsets too (ReportLab's `TTFont`), as simple TrueType fonts of up to 256
    glyphs each, tagged by subset number (`AAAAAA+` for the first, so the same tag for every
    sheet's first subset). Parity on subsetting; the tag from the glyph set is rust-correct, as
    PDF asks different subsets of a font for different tags;
  - one CID font per face, not simple fonts, so the callout symbols keep their own codes
    (equivalent; already so before subsetting).
- PNG output (stream d-png, `src/png.rs`, `tests/png.rs`): `render` writes `png` when it is named
  in `formats`, at `dpi` (150 by default, `--dpi` on the command line; `dpi <= 0` refused with
  draftwright's message). It rasterises the layers the PDF draws (tiny-skia, BSD-3-Clause,
  without SIMD; PNG through the `png` crate, MIT/Apache-2.0) and carries the spec in an iTXt
  chunk `DraftwrightSpec`, which `spec` reads back. Differences from draftwright, with verdicts:
  - the pixels are tiny-skia's, not PDFium's (equivalent: the same page at the same size,
    `ceil(page × dpi)`; no pixel parity is claimed);
  - the PNG carries the spec, attribution text and a `pHYs` resolution, where Pillow's has none
    (rust-correct: model.md asks every format to carry the spec, and the size in mm is kept);
  - 8-bit RGB, as PDFium renders to Pillow (equivalent);
  - a raster over 2^28 pixels is refused (`invalid`, "choose a lower dpi") where draftwright
    would try to allocate it (rust-correct: one bad request must not exhaust a long-lived
    service's memory);
  - `formats` absent still means svg, dxf and pdf, and the CLI's default too, where draftwright's
    CLI defaults to pdf alone (not-applicable: the port's default predates PNG; adding PNG to it
    would be a new default for every caller).
  Left: ADR5-1's `rust_note` in `docs/rules/adr.yaml` (owned by another stream during wave 3)
  should add that PNG bytes are also pinned run to run and across time zones (`tests/png.rs`).
  Cross-platform bytes are not checked: points go to the rasteriser on a 1/256-pixel grid, which
  absorbs last-bit differences in the platform's sine and cosine except at a grid boundary, and
  tiny-skia's clipping of a cubic that crosses the page edge calls `acos`/`cos`.
- quiddity-rust: the volume module's general `Probe::Prism` has no direct unit test for the
  convex case gussets use. Python's `local_degradation` path is not ported in any family.
- From wave 1 (review H1–H5, M2 merged):
  - Answered by haecceity (quiddity-rust d6a4703): HLR no longer emits cgb242's zero-span NURBS
    stretch (breakpoints within 1e-9 of the part's size are passed over), so cgb242 draws with
    no `edges_not_drawn`; the port's check stays as a guard, unit-tested in `output`.
- Closed in wave 2 (stream w2/d-model-honesty): `Drawing::problems` reach `review` and `render`;
  the unresolved-geometry refusal is tested (`tests/fixtures/parts/unresolved.step`); the sheet's
  basis names a computed or overflow fallback; an empty iso projection is reported
  (`iso_projection_empty`); review M4, M5, M6 and L2. Left from them: a section's outcome is
  reported only when it is missing (`section_not_shown`), not recorded for every plan as
  ISSUE-1190-b's closed vocabulary asks; feature-keyed spec entries stay inactive across a
  revision until R3's rebinding lands.
  - Question for the maintainer: draftwright refuses the drawing when section A–A is needed but
    the plan view, or both front and side, are missing (`sections.py:389-404`); the port draws
    the rest and reports `section_not_shown`, as R4 decision 2 does for a dimension no view can
    carry. Keep the problem, or refuse as Python does?
- From wave 2 (stream d-dims-split): `src/dims.rs` is split into `src/dims/` (`mod.rs` the
  driver and public API, `placement.rs`, and one planner per family: `envelope.rs`, `turned.rs`,
  `holes.rs`); a new family is a module and an entry in `dims::PLANNERS`. Every recognised
  feature of a kind no planner claims gets an info problem `dimensions_not_ported` (review M8).
  demand now groups holes by `bridge::holes::established`, as dims and lint do, and a hole with
  both a counterbore and a spotface states both. Left:
  - Questions for the maintainer:
    - Record types draftwright itself leaves undimensioned (`detect.py`'s
      `_UNCONSUMED_RECORDS`: freeform surfaces, thin-wall and sheet-metal bodies, oriented
      chamfers, interior voids, …; lint.yaml's LINT-quality-recognised_inventory_registers calls
      most of them "undecided", surfaced at runtime) get no problem from dims. Should the plan
      report them (for example an info problem citing draftwright's disposition), or is that
      lint's job?
    - A hole with both recesses prints the counterbore first (draftwright's segment order),
      although quiddity's spotface is usually the outer, wider step nearer the opening. Keep
      the segment order, or state them from the opening inward? Tied to the ⌴/SF question.
- From wave 3 (stream d-dims-slots): `dims::slots` (through slots: width, length, end radius,
  location along the length; equal widths counted) and `dims::bosses` (diameter, height,
  location; groove floors left to the groove). Left: round-bottom and rectangular blind slots
  (they arrive as `section_recess`, shared with pockets, so slot depth has nothing to apply to
  yet); slot patterns and polygonal bosses (ctc01's hex boss height is rust-wrong in
  `known_dimensions.json`); ISSUE-1154-a, ISSUE-1511-a and ARCH0009-6 stay pending; a boss with
  no positive height gets no height dimension and no problem (should raise
  `feature_not_dimensioned`); the new items add 3 `annotation_off_sheet` (cgb245, ctc01) and 15
  ink overlaps for the placement work.
  - Questions for the maintainer:
    - Answered 2026-10-08 (applied by stream d-location-decisions): a slot is located across its
      width too, and boss locations stay (both beyond draftwright, rust-correct).
- From wave 3 (stream d-slots-rest): polygonal bosses (`HEX 100 A/F`, `4-SIDED 25.4 A/F`, with
  their heights and locations; ctc01's hex boss height now agrees with draftwright's sheet), a
  `feature_not_dimensioned` warning for a boss with no positive height, ISSUE-1154-a's
  consolidation of a boss height onto an overall extent between the same two faces (to the
  plan's `omitted`, basis `conveyed by <extent ID>`; ISSUE-1154-a and ISSUE-1511-a stay pending
  for the tolerance clause and other families' lengths), and rectangular and round-bottom blind
  slots, decoded from `section_recess` records as draftwright's `section_recess_fields` does (the
  other section recesses stay `dimensions_not_ported`). The blind-slot callouts match
  draftwright's on all ten it prints across eight corpus parts (on a ninth, mfcadpp_holdout 363,
  draftwright fails to build the sheet, so its open slot has nothing to compare with); three of
  those parts and
  nist_ftc_07 have recognition fixtures in `tests/fixtures/recognition/dims/`. Left:
  - **Slot patterns are blocked on the bridge.** quiddity publishes slot arrays only in its
    derived `slot_patterns` (`document.py`), which `src/bridge.rs` does not read (it keeps the
    derived hole patterns only) and whose member slots lose their `pattern_sibling` links
    (`dependents` is dropped). So no `slot_pattern` reaches the Analysis and an array's members
    are dimensioned as lone slots, where draftwright prints one `N× SLOT W × L` callout, an
    obround's `2× R`, and the pitches. The bridge needs to keep `derived.slot_patterns` beside
    the hole patterns, naming member slots by feature ID; then `dims::slots` can group them. No
    corpus part has a slot array (checked with Python quiddity over all 94 files), so nothing on
    the captured sheets differs today.
  - ARCH0009-6 stays pending (it touches `src/dims/holes.rs`): ctc01's hex boss stands at the
    centre of its 2×2 hole grid, so its location (400, 225) repeats the grid's, listed rust-wrong
    in `known_dimensions.json`.
  - The new items add placement problems for the placement work: on ctc01, 2
    `annotation_off_sheet` (the HEX callout and the boss's y location) and 5 ink overlaps; on
    nist_ftc_07, 4 overlaps (the boss heights with a pattern location in the side view); on
    mfcadpp 11512, the open-slot callout over the iso view and through a hole callout.
  - Questions for the maintainer:
    - The port counts identical polygonal bosses once (`4× 4-SIDED 25.4 A/F` on nist_ftc_07),
      as it and draftwright do cylindrical ones; draftwright calls each polygonal boss out on
      its own. Keep the count?
    - Interpreted 2026-10-08 (stream d-location-decisions): the decision says "a slot is also
      located across its width"; the port reads that as covering blind slots too, so a blind
      slot is located across its width, as a through slot is. Overturn if not meant.
    - An authored set that lists both a boss height and the overall extent between the same
      faces keeps both in the port (an explicit listing is kept); draftwright consolidates
      them. Which?
    - ISSUE-1154-a's consolidation compares the boss's base and top with the extent's faces
      within 0.005 mm (half quiddity's 0.01 mm rounding of a boss height, from which
      draftwright builds the span), where draftwright's `_same_support_planes` uses 1e-6 and
      says nothing looser is admissible. No corpus sheet differs. Keep the port's tolerance, or
      take the height unrounded from the faces and compare at 1e-6?
- From wave 3 (stream d-dims-edges): `dims::edges` (fillet `R`, chamfer `C` or `a × θ` callouts,
  equal ones counted, where the edge is seen end-on or in a turned profile). Oriented chamfers
  (always oblique in quiddity) stay in `UNDRAFTED`, undimensioned and unreported, as draftwright
  #1365. Left: ISSUE-1276-a only partly ported (on a part with several or partial shafts the
  leader stays at the treatment's physical point, not turned about its own shaft); ISSUE-1374-a/b
  not ported (no loss of credit for a leader moved off the bevel or into the wrong view); blends
  still get `dimensions_not_ported`; older `known_lint.json` reason texts still say the port draws
  only envelope, turned and hole dimensions.
  - Questions for the maintainer:
    - One n× callout counts equal members along other axes or seen in other views, as
      draftwright does, and its leader points only at the largest set in one view. Keep that, or
      split callouts per view?
    - Oriented chamfers stay undimensioned and unreported, as in draftwright #1365. Do they need
      a callout, and in which view?
- From wave 3 (stream d-trust-rules): the trust rules of `adr.yaml` (61) now read 16 ported, 4
  translated, 41 pending, every pending one with a `rust_note` saying what is missing; the new
  guards are in `tests/trust_rules.rs`. `plan` now refuses an Analysis or Spec of a schema it
  does not read (a `draftwright.spec/2` was read as spec/1 and rewritten as one), and
  `api::write` removes its staged copy when a write fails. Left:
  - Edit locality (ARCH0003-2): closed by stream d-placement (below).
  - Questions for the maintainer:
    - ADR5-16: the Plan records the analysis's binding and the spec, but not the recognition
      it was planned from, so two plans of one file and spec made with different `--features`
      JSON agree on everything they record and still differ. Should the Plan name its
      recognition (for example the recogniser and a hash of the recognition JSON), and where?
    - ADR5-13: should the `bounded-clear` / `needs-attention` summary live in the Plan (and so
      in `plan` and `review` replies) or wait for a report document?
- From wave 3 (stream d-native-holes): `bridge::native` reads holes and hole patterns from Rust
  quiddity's recognition document (`quiddity-rust/recognition/1`, quiddity pinned with
  haecceity at 1bf3db2), fail-closed on schema version, family, record type, coordinate space
  and face count, with draftwright's IDs and fingerprints. `tests/native.rs` compares it with the
  bridge on 29 parts (the 5 fixtures, and the corpus and sheets recognitions; sheets' ctc04 is
  refused by the bridge): 115 differences in `known_native.json`, 101 equivalent (a through
  hole drilled from its other opening, and what follows from it; frame-relative rectangle angles
  and member order; Python's `named_dimensions` restating sizes) and 14 rust-wrong. **The live
  pipeline is not switched:** the rust-wrong ones are 7 holes per corpus set whose native faces
  lack the face that closes them (a drill point's cone or a blind floor). Rust quiddity finds
  that face (the hole occurrence's context) but `recognition::document` carries only defining
  faces, while Python's bridge gives constituent faces. Also owed before a switch: the Analysis
  has one recognition frame, Python's working frame, so native holes (in the file's frame) must
  be mapped into it or the frame made per feature; and the drawings compared before and after.
  - Questions for the maintainer:
    - Should quiddity-rust's recognition document carry each record's constituent faces
      (defining plus context, as Python's `constituent_faces`), under a new schema version?
      That unblocks the switch.
    - Who owns fingerprints, draftwright or quiddity? Until that is decided the native reader
      keeps draftwright's (`draftwright-bridge/2`) and does not adopt quiddity's.
- Native families (d-native-families): `bridge::native` also reads fillets, chamfers,
  countersinks, bosses and turned steps, each record checked against a `FIELDS` table of
  required fields and shapes (missing or mis-shaped refuses the read). Over the 29 parts
  `known_native.json` grows by 182 entries (297 in all). Fillets 148 and chamfers 64 pairs
  agree in full. Countersinks: only ctc04 has them and the bridge refuses it; its 12 native
  countersinks match Python 0.3.9's raw records. Bosses: 55 rust-wrong (40 native bosses
  Python rejects because a turned step has the same faces, quiddity-rust does not port
  `reconcile_boss_turned_step_candidates`; 10 connector109 ID shifts from those; 15 missing
  free-end cap faces, the constituent-faces gap; 8 on ftc06 where both ends are open and the
  free end follows the frame), 29 equivalent. Turned steps: 72 equivalent, 8 undetermined
  (string_post body key ~1e-5 between OpenCascade and haecceity). Still not switched.
  sheet_metal_bodies (quiddity-rust dff320c) is known, not read. Review minors, not fixed:
  `FIELDS` requires `Fillet.side` and `Chamfer.corner` that draftwright does not read, and
  does not check `TurnedStep.profile.axis_origin`/`body_key` that `dims/turned.rs` and
  `demand.rs` do; `faces.is_sorted()` accepts duplicates (as for holes).
  - Questions for the maintainer:
    - Should quiddity-rust port Python's boss-versus-turned-step reconciliation, or record the
      disposition in its document? Without it a switched pipeline dimensions those bands twice.
      **Settled (2026-10-09):** quiddity-rust applies the reconciliation in `recognise()`
      (a02b758, pinned at 9f1e8ed); the boss entries it caused left `known_native.json`.
    - quiddity's boss free-end choice depends on the frame when both ends are open (Python and
      Rust alike); a quiddity-rust fix is needed. The constituent-faces question above now also
      covers boss cap faces.
- Lint families (d-lint-families): grooves, flats, angled, paired-ramp and circular blind
  steps run in `lint::check`; envelope and turned steps are ledgers only
  (`lint::envelope::outcomes`, `lint::turned::outcomes`), because Python's lint reports no
  finding for them (they feed completeness in `quality.py` and `requirements.py`). Compared
  with Python's own ledgers (`tests/fixtures/lint_ledgers.json`): 73 agree, 2 in
  `known_lint.json`. Not reached: suppressed, dropped and satisfied-by-note states, and the
  assembly info severity; envelope's alternative evidence (whole-extent measurements,
  profile-grouped chains). Review minors: flats' requirement order (first flat in Analysis
  order, Python sorts by `_FlatRequirementKey`) and envelope's chain deduplication are not
  documented; circular blind steps skip Python's centreline/section consistency check; the
  capture script's and the ledger test's comments disagree on the turned-step span frame.
  - Questions for the maintainer:
    - Should the port's lint report an unstated overall extent or turned band as a finding,
      when Python uses them only for completeness?
    - Should the ledgers feed a completeness report, so that they have a consumer?
- From wave 3 (stream d-issue-trust): the trust rules of `issues.yaml` (83 pending) now read 5
  newly ported, 2 translated and 76 pending (2 of them owned by quiddity), every pending one with a
  `rust_note` saying what is checked and what is missing; the new guards are in
  `tests/issue_trust.rs`. Left, where the port does not uphold a rule (no `src/` change was in
  this stream's scope):
  - ISSUE-1217-a: lint credits a claimed size when *any* number in the callout rounds from it,
    so a repeat count confirms a diameter of the same value (`6× ⌀8 THRU` or `(6×2)` claiming a
    ⌀6 bore is credited on plate.step). The fix is in `lint::holes::states`.
  - ISSUE-1143-b: lint credits a hole group's requirements when any claim names any one member
    (`lint::holes::coverage`), so one hole's callout certifies the diameter, THRU and count of
    every hole in its group; draftwright leaves the count missing. The fix is to require claims
    on every member.
  - ISSUE-1332-a: one pair of labels can raise both `annotation_overlap` and
    `annotation_ink_overlap` (m10127: the side view's depth with `d:f:hole:2/bore.diameter@front`).
    Owed in placement.
  - ISSUE-1247-a (corrected at integration): the stream branched before d-lint-families, which
    added lint's `angled_step_requirement_unsupported` warning, so an angled step is now reported
    as in draftwright. Still pending: the completeness outcome (no completeness report). No
    sampled part has one.
  - Questions for the maintainer:
    - draftwright's lint warns for a recognised feature it cannot dimension (passages, prismatic
      pockets, pads, angled steps: ISSUE-1245-a, 1246-a, 1247-a, 1392-a); the port reports an
      unported kind as an info `dimensions_not_ported` problem (an angled step gets lint's warning).
      Should these be warnings, and should they come from lint (as in draftwright) or the plan?
    - `edit` does not check view names (it is given no analysis), so `views/top` is accepted
      and refused only by `plan` (ISSUE-1260-b); model.md says each edit is validated. Should
      `edit` refuse what it can check without the analysis (view names, sheet sizes, scales)?
    - ISSUE-676-c validates declared polygonal bosses; with no declared builds (R4 decision 3)
      it has nothing to govern. Retire it (and any like it), or keep it pending in case
      declarations return?
- From wave 3 (stream d-bridge-slot-patterns): slot patterns are unblocked end to end. The bridge
  reads quiddity's derived `slot_patterns` as `slot_pattern` features (`f:slot_pattern:<n>`, ranked
  by faces, on their members' faces, the members named by feature ID in the record's `slots`),
  the representation `lint::slots` and `dims::slots` already took; a member must be one slot
  record and the members' `pattern_sibling` links must agree, or the read is refused; members and
  pattern carry count and pitch in their fingerprints. `dims::slots` calls a pattern out as
  draftwright does: `3× SLOT 6 × 20`, `2× R3`, `2× 25` (or the grid's column and row pitches, as
  quiddity names them), and the centre located on both axes; a pattern whose slots do not stand
  as quiddity says is reported and its slots dimensioned singly. No corpus part has a slot array,
  so two fixture plates (`tests/fixtures/recognition/slot_patterns/`: a 1×3 array and a 3×2 grid
  of obround slots, `make.py`, recognised by quiddity 0.3.9, draftwright's sheet facts captured by
  `capture.py` there) join `tests/dimensions.rs`: 12 of draftwright's items agree, 9 differences
  listed (4 equivalent: the overall width's view; 4 undetermined: the pattern's y location in the
  plan, not the side, pending the maintainer's decision on hole locations; 1 rust-correct: the
  grid's row pitch `2× 40`, which draftwright drops and its own lint reports). The captured
  corpus sheets' dimensions are unchanged. ADR3-4 on the bridge path: quiddity's document is
  refused unless it is `quiddity-recognition` version 3, written by quiddity 0.3.9 or 0.4.0.dev0
  (and the release the wrapper names), in local coordinates, with known families, family/record
  pairs and derived collections (`tests/bridge.rs::a_document_the_bridge_does_not_know_is_refused`);
  ADR3-4 stays pending (below). Left:
  - Placement problems from the new items: on both fixtures the `N× SLOT` and `2× R` callouts
    lie over the isometric (`annotation_over_view`); on the grid, the y location runs off the
    sheet and through the column pitch's label.
  - LINT-slot_coverage-slot_pattern (lint.yaml, not edited here) still reports each pattern
    unverifiable; its requirements can now be checked against the fixtures' sheets.
  - Questions for the maintainer:
    - ADR3-4 pins the provider's exact version. 0.4.0.dev0, which wrote most fixtures, names no
      exact quiddity commit, and the document carries none. Regenerate those fixtures with
      0.3.9 and accept only it, or pin 0.4.0.dev0 some other way?
    - The bridge knows but does not read quiddity's `oriented_slot_patterns` and
      `section_recess_patterns` (their members are read singly). Refuse a document that has
      them, as unknown records are refused, or keep reading the members?
- From wave 4 (stream d-spec-tolerances): the spec takes `dimensions/<id>/tolerance` (`±T` or
  `+U -L`) and `features/<id>/fit` (ISO 286 classes from draftwright's `fits.py` table, on holes
  and cylindrical bosses), both optional, so `draftwright.spec/1` reads older specs unchanged.
  `src/dims/tolerance.rs` puts them on the label before placement, as draftwright prints them
  (authored digits kept, one decimal at least), lists them in the item's `from_spec`, and
  withholds with a `tolerance_withheld` warning a band of nothing (ARCH0019-2) or a fit the
  members of an `n×` callout do not share (ARCH0019-3); an entry on what the plan leaves out is
  a `tolerance_not_shown` warning. `tests/tolerances.rs` compares the labels with draftwright's
  for the same declarations on ftc06 and ctc01 (`tools/probe_tolerances.py`): 12 equal, 3
  different (1 rust-correct, 2 rust-wrong) and 1 not-applicable. No per-family planner was
  touched. Left:
  - ftc06's mixed boss fits are rust-wrong: draftwright splits the group (`3× ø44.5` and
    `ø44.5 h6`), the port withholds the fit; splitting needs `dims::bosses` and `dims::holes` to
    group by fit.
  - Longer labels, for the placement work: on ftc06 with the test's spec, `127 ±0.1` (the
    grid's row pitch) now overlaps the boss location `139.7` in the front view
    (`annotation_ink_overlap`); ctc01's spec adds no placement problem.
  - A callout's other terms (counterbore diameter and depth, bore depth) have no ID, so they
    cannot be toleranced; draftwright's bare hole `.tolerance()` also lands on the counterbore
    diameter. GD&T, finish and a fit shown as deviations (`show="deviation"`) are not ported.
  - Questions for the maintainer:
    - A collapsed callout whose members have different fits: split it into one callout per fit,
      as draftwright does for bosses (and for holes with PMI tolerances, ISSUE-1116-b), or keep
      one callout and withhold the fit with a warning, as ARCH0019-3 says?
    - ARCH0019-2 is read as draftwright now formats a band (the drawn precision is a minimum,
      authored digits kept), so only `±0` is withheld; draftwright prints it `±0.0`. Should a
      band of nothing be refused by `edit` instead of withheld?
    - A fit on a feature that does not exist is refused (`invalid`, nearest IDs), as a spec
      naming an unknown dimension already is; one on a feature whose callout the plan leaves out
      is a `tolerance_not_shown` warning. Is that the line wanted?
- From wave 4 (stream d-issue-drafting, 2026-10-08): a first batch of 20 drafting rules of
  `issues.yaml` examined as scenario tests in `tests/issue_drafting.rs` (19 tests over the five
  fixtures and the twelve sampled parts): 2 newly ported (ISSUE-1142-a, a hole callout claims
  every number it prints; ISSUE-1146-c, every view on the page and no principal views
  overlapping), 16 pending with `rust_test` and a `rust_note` saying what is and is not checked,
  and ISSUE-1515-a pending as not-applicable (third angle only). Drafting/pending now 87 of 93.
  Left, where the port breaks a rule (ignored tests; no `src/` change in this stream):
  - ISSUE-1585-a: `sheet.title` takes any field, but `src/model.rs::render` draws only `title`
    and `number`, so a supplied `date` (or `material`) is accepted and silently not drawn.
  - ISSUE-1536-a: no title-block value is checked against the block, so an overlong title runs
    out of it with no problem or finding (`src/model.rs`, or lint).
  - Observed, verdict undetermined (layout, `src/layout.rs`): ftc06's section A–A area overlaps
    the side view's by 10 mm; ctc01's isometric reaches into the plan's right band; string_post
    is A4 at 2:1 automatically but A4 pinned gives 1:1 (row fit versus packed fit, as the port
    documents draftwright's `choose_scale`; not compared with a Python run).
  - Questions for the maintainer:
    - Two holes on one ordinate share one location dimension (`d:f:hole:5/location.length.y@plan`
      states hole:5 and hole:6), so neither member's coordinate can be omitted alone, where
      draftwright addresses each member's axis (ISSUE-883-a). Should the spec address a member's
      coordinate inside a shared dimension?
    - Should `edit`/`plan` refuse a `sheet.title` field the port does not draw, until it does?
- From wave 4 (stream d-placement, review M3 and ARCH0003-2):
  - The side view's bands come from the lanes its dimensions take (`dims::lanes`,
    `layout::Strips::reserve`, `Demand::lanes`), and the front's lower band is at least the
    side's (they share a row). `tests/bands.rs` still compares draftwright's estimate
    (`demand::read` leaves the lanes empty), so it is unchanged.
  - Every item the part would have is placed before the spec chooses what is drawn, so omitting
    or adding one moves no other (ARCH0003-2 now translated; `tests/placement.rs`).
  - Labels settle off other items' ink (`placement::settle`): a linear label slides along its
    line or moves to the other end of a short span; a callout's leader may turn left. Ink
    overlaps on the 12 captured parts went from 39 to 7, `annotation_overlap` from 1 to 0. A
    turned leader's tip moves to the rim's left 45° point, so the leader stays radial (at review
    it had kept its tip and lay tangent to the rim; cgb245 keeps one more overlap for it).
  - Rebased onto main after d-slots-rest: ctc01's polygonal boss adds 2 off-sheet items and
    pushes a bore callout off too (7 to 10 off-sheet), and 4 items past their area (18 to 22);
    verdicted rust-wrong in `known_placement.json` with the front/plan band question below. A
    boss height the overall extent conveys is consolidated after placement, so its lane stays
    reserved (a gap), as for any omission.
  - New info problem `annotation_outside_area`: placed ink past its view's area. Placement
    problems are a ratchet (`tests/fixtures/known_placement.json`, `tests/placement.rs`), whose
    failure prints the entries to paste; d-slots-rest's new items will need entries there.
  - Left: 21 `annotation_off_sheet` (cgb245 9, ctc01 10, ftc06 1, connector109 1), all in the
    front's or the plan's lanes, past bands that are still draftwright's estimates; 7 ink
    overlaps, all involving a callout's leader or the front callout of m10127 running into the
    side's band; ftc06's 2 plan callouts over the isometric.
  - Question for the maintainer: reserve the port's lanes in the front's and the plan's bands
    too? On the captured parts that removes every off-sheet item (cgb245 stays A2 at 1:2, ctc01
    moves from A3 to A2 at 1:5, m10127 moves to draftwright's A3 at 2:1), but grm05 drops from
    2:1 to 1:1 and connector109 from 1:2 to 1:5 on draftwright's ladder, because the port's lanes
    stack outward 7 mm apart where draftwright's solver runs on past its bands to the sheet's
    margin. The alternative is to let lanes run past the band while the sheet has room, as
    draftwright's strips do.
- From wave 4 (stream d-native-more-families): `bridge::native` also reads grooves, flats,
  angled, paired-ramp, circular blind, through and oblique through steps, and blends: every
  family quiddity-rust dff320c's document has that draftwright draws (the families 6065d26 added,
  slots, pockets and the rest, are still known and left to the bridge). Face levels and
  risers are not in that document (quiddity-rust's `features::levels` finds them; its document
  does not carry them), so they still come only through the bridge. Review minors fixed:
  `FIELDS` no longer requires `Fillet.side` and `Chamfer.corner` (nothing reads them), checks
  a turned step's `profile.axis`, `axis_origin` and `body_key` when it has a profile (fields
  `a.b`), and faces must be strictly ascending (duplicates refused, holes included).
  `tests/native.rs` now also compares the `dims` parts and a new `steps` set (4 mfcadpp parts
  with paired-ramp, circular blind and oblique through steps, `recognition/steps/`, written by
  `tools/recognise.py` with quiddity 0.3.9): 37 parts. `known_native.json` grows by 519
  entries (816 in all). Pairs and new entries by family: grooves 6 pairs, 12 equivalent
  (connector109's profile box and key in a rotated frame); flats 6, 6 equivalent (axis sense,
  axis line in a rotated frame); angled steps 3, 3 rust-wrong (the closing triangular flat, the
  constituent-faces gap); paired-ramp steps 3, 1 equivalent (`half_widths` low side follows the
  frame); circular blind steps 3, 1 equivalent (section endpoint order); through steps 2, 2
  equivalent (section endpoint order); oblique through steps 1, 2 equivalent (run sense and
  the outline measured from its other end); blends 292 pairs (47 agree in full), 136 native
  blends Python rejects (rust-wrong: `reconcile_blend_candidates`, every one's faces are native
  fillets'), 236 ID shifts from them (rust-wrong), 73 equivalent (path direction or normal
  sense), 4 undetermined (ftc07's two oblique blends, unpaired: the path point differs 1.3e-3
  along the line, OpenCascade's and haecceity's area centres). Also 3 native chamfers on an angled step's
  slant and 3 native fillets on a circular blind step's wall that Python rejects
  (`reconcile_bevel_candidates`, `reconcile_circular_step_fillets`; rust-wrong, with 2 fillet ID
  shifts), and the dims and steps parts' holes, patterns and boss (12 + 2 + 1 rust-wrong, the
  constituent-faces gap; 20 equivalent). In a one-off check (no test) the reader read every
  corpus part (94 files, none refused). Still not switched (the constituent-faces and single-frame questions above).
  - Questions for the maintainer:
    - The boss-versus-turned-step question above now covers three more reconciliations Python's
      aggregate makes and quiddity-rust does not (blend superseded by fillet, chamfer by angled
      step, fillet by circular blind step): port them in quiddity-rust, or have its document
      record dispositions? Switched as is, draftwright would ask for those radii and bevels
      twice. **Settled (2026-10-09):** ported in quiddity-rust (a02b758, pinned at 9f1e8ed); 432
      rust-wrong `known_native.json` entries (372 blend, 50 boss, 5 fillet, 3 chamfer, 2 slot)
      no longer occur and were removed. The #1247 fixture's slant chamfer (f:chamfer:2) is still
      produced natively beside its angled step (not in the native comparison; see ISSUE-1247-a).
    - Face levels and risers (`step_levels`, `risers`) are evidence the bridge carries: should
      quiddity-rust's document carry them (a schema change) so the native path can replace it?
- From wave 4 (stream d-location-decisions, 2026-10-08): the maintainer's location decisions
  applied. (a) The 23 pending side-view routing entries in `known_dimensions.json` (19 hole and
  hole-pattern, 4 slot-pattern; cgb245, ctc01, ftc06, slot_array, slot_grid, both sides; the
  decision said 20) are rust-correct citing the decision and ADR1-6, and ctc01's combined
  `225` entry says so for its hole-grid half (it stays rust-wrong for the boss's half). (b)
  `dims::slots` locates each lone through slot and each blind slot across its width, from the
  part's box to its centre line (`location_slot_across.length`), in the view that sees it
  end-on, one dimension per distinct distance in a view; a slot pattern was already located on
  both axes by its centre. One that a hole's, boss's or slot pattern's location already states
  (same view, axis and distance) moves to `omitted`, `conveyed by <ID>` (ARCH0009-6's hole-over-
  slot clause, `slots::consolidate`, called beside `bosses::consolidate`); omitting the winner
  brings it back (`tests/trust_rules.rs` now expects that). On the captured sheets this adds one
  item, cgb245's `38.9` (rust-correct); cgb245's other two slots (50.4) and ctc01's (225) are
  conveyed by hole locations. (c) Boss locations were already rust-correct; their questions are
  marked answered. ADR1-6 and ADR5-1 (PNG bytes pinned) rust_notes updated. Left:
  - Placement, for the placement work: cgb245's `38.9` runs off the sheet
    (`annotation_off_sheet`); on m10047 the side view's `11.0` runs through the slot location
    `5.1`, and on m10245 the `OPEN SLOT` callout's leader runs through the `33.9` label
    (`annotation_ink_overlap`).
  - ARCH0009-6 stays pending: coincident hole and boss locations (ctc01's 400, 225) are still
    not merged.
- `plan`, `render` and `review` accept an Analysis document (stream d-api-analysis, per the
  main-session decision of 2026-10-08; `src/api.rs`, both binaries, `tests/api_analysis.rs`):
  `analysis` inline in `api::call`, the service and the MCP tools, `--analysis a.json` on the
  command line, and `api::{plan,render,review}_analysed`. `plan` then needs no STEP file;
  `render` and `review` read the STEP without recognition and refuse an Analysis made from
  another file (binding SHA-256 or face count), of another version, or given beside `features`.
  Output is byte-identical to the re-analysing path (plate, prismatic, corpus grm03). See
  `docs/model.md`, "Adapters". `src/model.rs` is unchanged.
- From wave 4 (stream d-title-block): the title block is draftwright's ISO 7200 grid
  (`src/title_block.rs`, `draftwright_title_block_layout`): four 8 mm rows (legal owner and
  document type; title and drawing number; material, general tolerance, units and format; drawn
  by, approved by, date, revision and sheet), at the bottom of the 40 mm box the layout reserves,
  as draftwright draws its 32 mm block in the same `_TB_H` reservation (so no height difference).
  Each `sheet.title` key (`title number material tolerance drawn_by approved_by date revision
  sheet company document_type`, draftwright's `Sheet` options) fills its own cell; units (`mm`)
  and format (the sheet's size) are stated, as draftwright does. The plan records the cells
  (`sheet.title_cells`: field, label, box, value); `render` draws every cell's label and each
  value centred in IBM Plex Sans Condensed at 3 mm (now bundled in the PDF by `api`), as an
  invisible text run too, and the scale note above the block's right end (`_add_scale_note`).
  A value wider or taller than its cell is a `title_field_overflow` warning (one per cell, with
  the full value, drawn whole); one past the drawing area is also `annotation_off_sheet`. Text is
  measured within 0.001 mm of draftwright's `_text_size`, multi-line values included (each line
  centred on its advance, as OpenCascade lays it). ISSUE-1585-a and ISSUE-1536-a are ported
  (`tests/title_block.rs`, `tests/issue_drafting.rs` un-ignored and tightened, the PDF text layer
  in `tests/output.rs`); ARCH0006-1, ISSUE-1157-a and LINT-code-title_field_overflow notes
  updated. No corpus part's default title overflows; no captured fixture or placement result
  changed (the grid lies inside the reservation placement already keeps out). Differences, with
  verdicts:
  - a `sheet.title` key with no cell (`units`, `scale`, a typo) is refused by `plan` and `edit`
    with the keys there are (equivalent: build123d_drafting's `TitleBlock` raises for a value its
    layout cannot place, and `Sheet` takes no other option). This answers the d-issue-drafting
    question above in the refusing direction; a spec that carried such a key is now refused;
  - whitespace-only values are no value, for every field (rust-correct: draftwright strips six
    fields and draws spaces for the others);
  - the grid's outer line is on the part layer and its dividers on the dims layer, where
    draftwright draws all of it as 0.15 mm faces (undetermined: line weight only);
  - the scale note's text run is in the condensed face it is drawn in (draftwright's text layer
    names Plex Mono for it; equivalent).
  - Questions for the maintainer: draftwright fills three cells nobody supplied: revision `A`,
    general tolerance `UNSPECIFIED` (ISSUE-1157-a, still pending) and drawn by `draftwright` (or
    `<author> / draftwright`, with a link over the cell). The port leaves them blank and draws a
    supplied `drawn_by` as given. Fill them as draftwright does? (`A` is an invented revision;
    `UNSPECIFIED` and the attribution are not.)
- From wave 5 (stream d-coincident-locations): ARCH0009-6 is ported. `dims::holes::consolidate`
  (replacing `slots::consolidate`) runs once over every kept location from the part's box, after
  the spec's tolerances: equal view, axis, raw ends (within 1e-6 mm) and label make one span,
  drawn once. Rank: hole, hole-pattern and slot-pattern locations (draftwright's
  `_location_candidate`, `precedence=2`), then bosses (port-only), then slot positions
  (`precedence=1`); then listed over default (pinned), then the smaller ID. Losers are omitted,
  `conveyed by <id>: ...`; omitting the survivor promotes the top loser. A spec-listed loser now
  yields too (it was kept before), as draftwright's pinned slot position does. ctc01's hex boss
  location is conveyed by its hole grid's (two rust-wrong `known_dimensions.json` entries gone,
  `known_placement.json` counts 10 → 9 and 22 → 20); boss_circle's boss location by its bolt
  circle's. The edit-locality tests (`tests/placement.rs`, `tests/issue_drafting.rs`) now expect
  the conveyed item back when its survivor is omitted. Separately, `placement::slides` now makes
  its candidates lazily: `tests/views.rs::a_fallback_scale_or_sheet_says_so_in_its_basis` (a
  200 km part at 1:1) built one per 0.5 mm of a 200 km line and was killed at 16 GB on main
  448031a; the same candidates are tried in the same order.
- From wave 5 (stream d-issue-drafting-2, 2026-10-08): the merged streams' issues.yaml proposals
  checked against main, and a second batch of drafting rules examined as scenario tests in
  `tests/issue_drafting.rs` (11 new tests). issues.yaml before → after: ported 15 → 16, pending
  173 → 172 (drafting: ported 8 → 9, pending 85 → 84, pending with a `rust_test` 22 → 33).
  - Proposals applied after checking: ISSUE-1154-a's tolerance clause is ported (a toleranced
    boss height is not a default, so it is never consolidated; a tolerance on the extent alone
    still takes it; both toleranced keep both, as draftwright's asymmetric rule), the rule stays
    pending for other features' lengths. d-spec-tolerances' ISSUE-1215-a, 1215-c and 1215-d stay
    pending with tests and notes naming the clauses not ported (below). Stale notes refreshed:
    ISSUE-1332-a (no captured part raises both overlap codes for one pair since d-placement, the
    mechanism remains) and ISSUE-1432-b (slot patterns are dimensioned and malformed ones refused
    since d-bridge-slot-patterns; oriented slots are not). Nothing from unmerged branches.
  - Second batch: ISSUE-1215-b ported (a toleranced label's box is the ink of what is drawn and
    grows along its line). Pending with a test and a note: 1215-a, 1215-c, 1215-d, 1215-e, 1216-a,
    1524-a, 1334-a, 1240-a, 367-a, 1166-b and 563-b (the last two cite existing tests). Pending,
    examined and not ported (a note only): 563-a and 1466-b (the spec has no placement intent or
    side hint), 1333-a (no repair stage or pins), 1349-a (no display precision), 1612-a (no sheet
    margin options) and 1116-a (not-applicable until AP242 PMI is read).
  - Left, where the port falls short (no `src/` change in this stream):
    - ISSUE-367-a: placement's collision checks (`src/dims/placement.rs`, `settle` and
      `collisions`, through `crosses`) test zero-width centrelines against label boxes and leave
      out the drawn arrowhead (2.7 by 0.9 mm) and the stroke width, so a contact by an
      arrowhead's flare alone would go unreported. Not observed: on the fixtures and the twelve
      sampled parts every arrowhead in another label has its shaft through it too (m10127). The
      fix is to test the arrowhead triangle and the line's half-width there.
    - ISSUE-1215-d, a verdict: the port does not collapse a pattern whose gaps differ by more than
      1e-4 mm, so it never prints a pitch (or its band) over unequal gaps; draftwright collapses
      within its recogniser's 2 % and withholds only the band (rust-correct).
  - Questions for the maintainer:
    - draftwright's bare `envelope().tolerance(t)` tolerances all three extents at once, and a fit
      class can be set on an extent (`30 h6`); the port's spec takes a tolerance per dimension ID
      and a fit only on a hole or a cylindrical boss (ISSUE-1215-a). Add an envelope-wide entry
      and fits on extents, or keep the per-ID surface?

- From wave 3 (stream d-dims-grooves-flats): `dims::grooves` (`10 WIDE × ø122`, one callout per
  groove, stating `groove.length` and `groove.diameter`, in the view that shows its profile, the
  leader to its floor) and `dims::flats` (`52.6 A/F`, one callout per across-flats definition,
  counted `n×` over independent stock, every face's `flat.length` stated, in the view down the
  stock; draftwright's mixed-size decimals). On the captured sheets all five of draftwright's
  groove and flat callouts (connector109, string_post) agree in label, view, parameters and
  values (`tests/dimensions.rs` now compares the two families; no `known_dimensions.json` entry
  was needed); `tests/dims_grooves_flats.rs` has the scenarios. Differences from draftwright that
  no captured sheet shows, with verdicts:
  - a groove whose profile view is not shown falls back to the other profile view; draftwright
    skips it without trace (rust-correct: nothing is dropped silently);
  - a record that cannot be stated (no width, floor diameter, size, position, or a direction off
    its named axis) is a `feature_not_dimensioned` warning; draftwright skips such a groove
    without trace, and fails conversion on a flat whose direction is off its axis (rust-correct);
  - flat decimals are chosen over every recognised flat, draftwright's over the sizes the spec
    keeps, so omitting one flat of two sizes can leave the other with a decimal draftwright drops
    (rust-wrong, minor: the planners run before the spec is applied; fixing it touches
    `dims::plan`);
  - callouts are ordered, and take their IDs, by the Analysis's first member, not by
    draftwright's sorted keys (equivalent);
  - a flat record without `axis_line` or `stock_span` is a definition of its own, as
    `lint::flats` keys it; draftwright defaults both to `(0, 0)` and so merges such flats as if
    on one stock (rust-correct: quiddity always emits both, and its `Flat` docs name merging
    independent stock as the defect those fields fix; no corpus record lacks them);
  - an authored `display_decimals` on a flat (draftwright's `_group_value_text`) is not honoured
    (not-applicable here: the port's `Spec` carries no display decimals for any family; that
    belongs with the spec work, d-spec-tolerances).
  Left:
  - Rebased onto main 7b155db (stream d-rebase-grooves-steps): `known_lint.json` lost the three
    `own_drawings` entries the new callouts resolve (`string_post flat_requirement_missing` 1;
    `connector109 flat_requirement_missing` 1 and `groove_requirement_missing` 6, checked with
    `LINT_OWN_ALL=1`). With d-placement's settling, connector109's grooves add no ink overlap
    but 3 `annotation_off_sheet` (`d:f:groove:2`, `d:f:groove:3`, and the chamfer callout pushed
    out past them; 1 → 4) and 3 `annotation_outside_area` (5 → 8), all below the front view,
    recorded rust-wrong in `known_placement.json` with the front/plan band question.
  - lint keys a flat by its record's raw `stock_span` (as draftwright's lint does), the planner by
    the span along the canonical direction (as draftwright's `FlatFeature` does), so a reversed
    face whose span runs along its own direction is one definition to the planner and two
    requirements to lint, in both programs. No corpus part has one.
  - ADR3-12 (a groove that cannot name one turned-profile owner is refused) is not checked.
  - `src/dims/mod.rs`'s module docs name steps and levels but not grooves and flats (only the
    planner list was touched for them, as other streams edit that file).
  - Applied by the integrator after d-issue-drafting-2 merged (integration run 2): this hunk,
    verbatim from the original commit a991e49, is now in `docs/rules/issues.yaml`:

    ```diff
    diff --git a/docs/rules/issues.yaml b/docs/rules/issues.yaml
    index 7c37cb4..f416476 100644
    --- a/docs/rules/issues.yaml
    +++ b/docs/rules/issues.yaml
    @@ -1484,7 +1484,7 @@
         corpus_parts: ["GRM-03", "grm03_thumbwheel_drive_screw.step"]
       status: pending
       rust_test: src/dims/edges.rs::tests::a_turned_site_turns_onto_the_profile
    -  rust_note: "Partly ported for fillets and chamfers: on a part turned whole, a turned treatment's leader site is set at its radius off the turning axis, at its own axial station, so it lands on the profile in whichever profile view carries it (src/dims/edges.rs::site; tests/dims_edges.rs::turned_callouts_in_the_fallback_profile_end_on_the_profile). Not ported: rotation about the feature's own shaft on a part with several or partial shafts (draftwright ranks external cylinder patches, _turned_profile_site), so there the physical site is kept; knurl and groove notes are not planned."
    +  rust_note: "Partly ported for fillets and chamfers: on a part turned whole, a turned treatment's leader site is set at its radius off the turning axis, at its own axial station, so it lands on the profile in whichever profile view carries it (src/dims/edges.rs::site; tests/dims_edges.rs::turned_callouts_in_the_fallback_profile_end_on_the_profile). A groove's callout leads to its floor: sites at the floor radius off its own axis, at its own axial station, on both radial axes, so the tip lands on the floor in whichever profile view carries it (src/dims/grooves.rs). Not ported: rotation about the feature's own shaft on a part with several or partial shafts (draftwright ranks external cylinder patches, _turned_profile_site), so there the physical site is kept; knurl notes are not planned."
       port_note: "Keep: plan leader targets reference analysis edges/faces; scenario on GRM-03."
     
     - id: ISSUE-1281-a
    @@ -1969,7 +1969,8 @@
           - "tests/test_issue_1371_flat_completeness_evidence.py::test_deleting_provider_flats_cannot_shrink_the_independent_denominator"
         corpus_parts: ["flat-coaxial.step", "flat-double-d.step", "flat-topology-a.step"]
       status: pending
    -  rust_note: "Not ported: flats are not dimensioned and lint has no flat coverage (LINT-flat_coverage-across_flats)."
    +  rust_test: "tests/dims_grooves_flats.rs::equal_flats_on_other_stock_are_counted_and_a_double_d_is_one"
    +  rust_note: "Partly ported: flats are dimensioned by definition (faces reflected through their stock's axis are one, distinct stock is not merged; src/dims/flats.rs, tests/dims_grooves_flats.rs), and lint groups flat requirements by stock and loses credit for a misprinted value (src/lint/flats.rs::tests::the_two_faces_of_a_double_d_share_one_across_flats_requirement). Not checked: a severed provenance, and a denominator that cannot shrink when provider flats are deleted."
       port_note: "Keep."
     
     - id: ISSUE-1371-b
    ```
- From wave 4 (stream d-dims-steps-levels): `dims::steps` (a through step's two legs in the view
  along its run; circular blind steps `R4 × 25 DEEP` and paired ramps `2× 51.3° × 25 RUN` as one
  callout each) and `dims::levels`, draftwright's height ladder: each face level's height from the
  base in the front view (witnesses on the level's support; three or more equal rises one
  `n× rise`) and the shoulders' positions along x (plan) and y (side). The levels come less what
  draftwright's ownership stage gives other owners (a multi-axis part's base-plate top, a
  side-facing pad's edges, an edge-open pocket's floor along z, a lowered through step's corner),
  and a through step along x or y whose legs the ladder, a plate and the overall extents already
  state is left to them, as in draftwright (reported, info). The shoulders need the part's risers
  in its own frame, which quiddity's document cannot give (its working frame's z is the part's x
  on cgb245), so the Analysis is now `draftwright.analysis/4`, recording `level_supports` and
  `risers` read with Rust quiddity's levels module (`dims::levels::evidence`); a version 3
  analysis gets an info problem. Fixtures: quiddity's golden through step, circular blind step,
  paired ramp and plates/pads/levels parts, and a block stepped along y, with their recognition
  and draftwright's sheet facts (`tests/fixtures/recognition/steps/`, `make.py`). Compared with
  draftwright (`tests/dimensions.rs`): of draftwright's 25 step and ladder items on the 12 sheets
  and 5 fixtures, 24 agree (all but hanger's 65, in its detail view); 32 differences listed: 21
  undetermined (20 rungs draftwright withholds or drops at the sheet's scale, its legibility
  gate, and hanger's 65 in a detail view vs the front, counted on both sides), 1 rust-correct
  (m10046's 13.5, which draftwright's solver drops with an error), 10 equivalent (the fixtures'
  overall width in the front, not the plan). AngledStep and ObliqueThroughStep stay in `UNDRAFTED`. Left:
  - Rebased onto main 7b155db after the grooves and flats work (stream d-rebase-grooves-steps):
    the port's own drawing of m10127 states its through step's legs, so `known_lint.json`'s
    `own_drawings` entry `m10127 through_step_requirement_missing` (2) is gone. No other
    `own_drawings` entry changed (`tests/lint.rs` passes with `LINT_OWN_ALL=1`). The location
    consolidation of d-coincident-locations (`dims::holes::consolidate`) does not take the
    ladder's rungs and shoulders or a through step's legs, as draftwright's corridor dedup does
    not: its shoulders, rungs and through-step legs register their `CorridorCandidate` with no
    dedup key (`dedup=None`, "keep both": `_axial_render.py`, "No cross-dedup against hole
    locations"; `_height_ladder.py`; `from_model.py::render_through_steps`).
    `known_dimensions.json` needed no change.
  - **Placement problems the new items add** (for d-placement; `src/layout.rs` and
    `src/dims/placement.rs` not edited), on main after d-placement's merge (no new ink overlap or
    off-sheet problem; re-checked after the rebase, the same items): cgb245
    `annotation_outside_area` 22 → 30 (7 rungs and the overall
    height past the front's area) and 1 new `annotation_over_view` (the overall height's label
    over the side view, beyond the 11 rungs); ftc06 `annotation_outside_area` 21 → 23 (the
    three shoulders) and `annotation_over_view` 2 → 3 (chamfer:2's label over the isometric);
    m1000 `annotation_outside_area` 1 → 2 and 1 new `annotation_over_view` (the overall height,
    beyond its rungs). Recorded with verdicts (rust-wrong) in `tests/fixtures/known_placement.json`
    so `tests/placement.rs` stays green; the cause is the legibility gate below: the layout reserves
    the front's right band for the rungs legible at the scale while the plan draws every rung.
  - Not ported: draftwright's legibility gate (`_classify_steps`: withhold a rung shorter than
    its arrows, drop one too close to the last and escalate to a detail view); a correlated set
    addressed by one ID (ARCH0015-2 stays pending: each rung and shoulder is its own item, named
    by its own value to the thousandth, `step_height.length.h16`, `step_position.length.x136_525`,
    never by its rank, so a level a revision adds renames no other rung (ADR4-6); a spec can
    still omit one rung);
    draftwright's channel absorption into the ladder (`_step_level_owns_channel`); lint's
    alternative grammars that credit a ladder-owned through step's legs (`lint::steps`). The
    plate a through step shares is found by shared constituent faces (draftwright uses defining
    faces); Rust quiddity's risers carry their solid's levels within 0.5 mm of its ends where
    draftwright's aggregate uses 0.6 (no sampled part differs). Equal shoulders are stated once
    (draftwright draws each riser's: ctc04's two 75s and two 225s; ctc04 is not among the
    compared sheets, so this has no verdict entry: see the question below). A level with no
    recorded support is reported (`step_level_support_missing`), its witnesses on the box.
  - Catalogue changes (applied by the integrator, run 2): issues.yaml ISSUE-915-a stays pending,
    rust_note "the ladder's witnesses stand on each level's own support (src/dims/levels.rs); no
    detail views"; ISSUE-924-a stays pending, rust_note "a shoulder on the box is reported
    (step_position_coincident_with_datum), not drawn; quiddity never gives one, and the plan's
    omitted list does not record it". lint.yaml unchanged.
  - Questions for the maintainer:
    - Port draftwright's legibility gate? It is a placement decision at the sheet's scale, which
      the planners do not see (`dims::Context`); without it cgb245 states 11 heights where
      draftwright states 1 and withholds 10 (info), and the port's sheet collides.
    - Address a ladder as one correlated set (ARCH0015-2), or keep per-rung IDs a spec can omit
      one at a time? One set ID needs the plan driver (`src/dims/mod.rs`) to take a set's
      omission for all its members; until then each rung's ID is its height.
    - Two shoulders at the same position (ctc04's two risers at 75, two at 225): one dimension
      (the port, as one value stated twice is redundant, ISO 129-1) or one per riser
      (draftwright)?
  - Applied by the integrator after d-general-notes merged (integration run 3): this hunk,
    verbatim from the original commit 065dd08 (the analysis/4 schema line and the "Level
    supports and risers" paragraph), is now in `docs/model.md`. The ISSUE-915-a and ISSUE-924-a
    `rust_note`s above are already in `docs/rules/issues.yaml`.

    ```diff
    diff --git a/docs/model.md b/docs/model.md
    index 1f440ff..ae3809b 100644
    --- a/docs/model.md
    +++ b/docs/model.md
    @@ -33,7 +33,7 @@ edited.
     
     ```json
     {
    -  "schema": "draftwright.analysis/3",
    +  "schema": "draftwright.analysis/4",
       "binding": { "source_sha256": "…", "part": "10127", "face_count": 27, "loader": "haecceity 0.1" },
       "frame": { "origin": [0, 0, 0], "axes": [[1, 0, 0], [0, 1, 0], [0, 0, 1]] },
       "envelope": { "min": [0, 0, 0], "max": [28.8, 32.3, 44.7] },
    @@ -49,6 +49,14 @@ edited.
     - **Levels** (`draftwright.analysis/3`) are the heights of the part's horizontal face levels,
       read from the geometry as quiddity's `step_level_records` reads them but in the part's own
       frame: what a prismatic part's height ladder dimensions. Older analyses read with none.
    +- **Level supports and risers** (`draftwright.analysis/4`): each level's support (its height and
    +  the x and y extents of its faces, per solid) and the step risers between the levels (quiddity's
    +  `FaceLevel` and `RiserEvidence`, read with Rust quiddity in the part's own frame, as
    +  draftwright's own recognition run reads them). The height ladder's witnesses stand on a level's
    +  support, and its shoulders' positions come from the risers (`src/dims/levels.rs`). quiddity's
    +  recognition document cannot give them: it reads them in its working frame, whose z need not be
    +  the part's. Older analyses read with none; a version 3 analysis with levels gets an info
    +  problem saying the step positions could not be found.
     
     - **Feature IDs** are stable within one file. They are anchored on face indices and derived
       from the feature's kind and faces, never from the order it was found in.
    ```
- General notes (stream d-general-notes; `src/notes.rs`, `tests/notes.rs`, `docs/model.md`'s
  "General notes"): the spec's `notes` (`[{id: "n:<k>", text}]`) are read. `plan` and `edit`
  refuse an ID not of the form `n:<k>`, a duplicate ID and empty text, naming the next free ID;
  `edit` gains `add` (`{"op": "add", "path": "notes", "value": {"text": …}}`, which takes the next
  free ID when none is given; the edit tool's choices are now `set, remove, add`). Each note is a
  Plan item (`notes`: ID, text, number, rows, `default: false`, basis, placed), drawn in one
  ruled `GENERAL NOTES` block (`notes_block`) as draftwright's `render_document_notes` draws its
  document notes: rows numbered by position and wrapped at 48 characters as `textwrap` wraps
  them, IBM Plex Mono at 3 mm, 7 mm rows; the block reserved top right when the sheet is chosen
  (`Request::required_tables`) and placed by `fit_box` after the dimensions, clear of the views'
  areas, the isometric, every dimension's ink and the title block; each row an invisible text
  run (`tests/output.rs::general_notes_reach_the_pdf_text_layer`). A block that has no place is
  a `notes_not_placed` error on every note, which stays in the plan undrawn. Compared with
  draftwright on m10127 and ctc01 (`tools/capture_notes.py`, `tests/fixtures/notes.json`): rows,
  numbering, wrapping (seven cases), height, sheet and scale (ctc01; m10127's sheet already
  differs without notes) and the top-right corner agree. Differences, with verdicts:
  - a continuation row keeps its three-space indent, so the block is wider by the indent
    (rust-correct: draftwright writes the indent into the row and its left alignment on each
    row's ink drops it; the captured table's continuation ink starts at the cell padding, as the
    number's does);
  - the block keeps clear of the views' bands as well as the drawn ink (equivalent on the parts
    compared: both land in the corner the sheet reserved);
  - the text layer keeps `⌀` where draftwright's has `ø` (rust-correct, as for callouts);
  - the block's top and right rules lie on the port's border line at the drawing area's corner
    (draftwright draws no border by default; equivalent).
  ARCH0006-1, ISSUE-1352-a, ISSUE-1351-a, ISSUE-1613-b and LINT-code-title_field_overflow
  rust_notes updated (statuses unchanged: each is still partial). Left: the per-feature `note`
  (`features/<id>/note`, e.g. `REAM` on a hole callout, which needs `src/dims/holes.rs`), its
  structured `satisfies` claims (ISSUE-1351-a), hole and callout tables, and balloons; notes are
  not lint annotations (they claim nothing); `title_field_overflow`'s message does not yet point
  to the notes. Questions for the maintainer:
  - A block that fits no sheet at a standard scale is not reserved, so it never shrinks the views
    (the plan is laid out as without notes, and the block is placed if it can be, else
    reported); draftwright reserves it regardless and would take a computed scale for it. Keep
    the port's rule?
  - Numbering: draftwright numbers notes by their place in the list (kept), so removing note 1
    renumbers the others on the sheet while their IDs stay. Number by the ID's `k` instead?
  - Heading: draftwright's document notes print `GENERAL NOTES` (kept); its author-declared
    `Sheet.notes()` block prints `NOTES` and does not wrap. The spec's notes are authored, so
    either could be argued; which heading?

- From stream d-dims-polygonal-stock (2026-10-08): ARCH0015-4 ported (`src/dims/stock.rs`):
  whole-part hexagonal stock states `HEX <A/F> A/F` down its axis and its length in the profile
  view, and no overall extent, matching draftwright's sheets of five fixture bars
  (`tests/fixtures/recognition/polygonal_stock/`, no corpus part is stock). ISSUE-1082-a ported;
  ISSUE-1082-b and 1371-b notes refreshed (still pending).
  - Question for the maintainer: on a stock part draftwright's #1560 safeguard
    (`compiled.py`, `overall_dim_withheld`, an error in its lint) reports the x and y extents as
    withheld whenever no spanned measurement covers them: the across-corners width of a plain
    hexagon bar (40 for A/F 34.6), both extents of a bar turned about its axis (38.3 × 39.0),
    and even the depth the A/F itself states. ADR 0015 says the stock dimensions replace these
    orientation-dependent extents, and the port follows it: it records nothing for them beyond
    the A/F's basis. Should a stock part's across-axis extents be reported as withheld (the
    #1560 reading), or are they replaced and not reported (the ADR 0015 reading)?
  - Plate thickness (scoping, not built): dimensioning it depends on draftwright's own plate
    conversion, not only on quiddity's `Plate` records. `_thin_profiles.py:101`
    (`register_plate_thickness`) dimensions only `plan.of_kind("plate")`, the `PlateFeature`s
    `detect.py:2019-2033` makes: only on a part with no turned profile or rotational feature,
    only when the plates lie on two axes or more (`multi_plate`, `detect_ownership.py:470`), and
    leaving out each plate whose interval equals a lowered through step's leg
    (`through_leg_spans`, `detect_ownership.py:392`, from draftwright's through-step ownership
    stage over levels, shoulders, plates and `envelope_is_emittable`). That last filter has no
    counterpart in the Analysis, so it waits on LINT-plate_coverage-thickness's open question
    (should the Analysis carry draftwright's plate conversion and step ladder?).

- From stream d-issue-drafting-3 (issue regressions, third batch; only tests/issue_drafting.rs,
  docs/rules/issues.yaml and this entry): scenario tests for ISSUE-1276-a (a groove callout's tip
  on its floor), ISSUE-915-a (each rung's witnesses on its level's support) and ISSUE-924-a (a
  shoulder on the box reported, not drawn, and only it leaves the chain); all stay pending, with
  rust_test and what is missing. ISSUE-1298-b gets a not-applicable rust_note (no PMI). The
  batch-2 review minors: ISSUE-1240-a's test now also runs on plate pinned to A3 at 2:1 and
  requires a label over the isometric there; ISSUE-367-a's guard still rests on m10127's one
  collision (no other case on any part or pinned sheet tried), now recorded in its rust_note and
  the test. No pending issues.yaml rule states the coincident-location rule (ARCH0009-6, already
  ported in adr.yaml), so none moved for it. Noted, not pursued: a riser 0.001 mm off the box is
  drawn as a `0.0` shoulder, as draftwright's exact-zero `_is_zero_step_position` would draw it.
  Question for the maintainer:
  - A shoulder on the box (ISSUE-924-a) is an info problem only; draftwright also lists it among
    its suppressions (value 0, not authored), and the rule's port_note asks for a plan omission
    with basis "coincident with datum". The plan's `omitted` holds only items a spec can address.
    Add such shoulders to `omitted` (with an ID like `d:step_level/step_position.length.x0@plan`),
    or keep the problem alone and settle the rule on that?

- Tables (stream d-spec-tables; `src/tables.rs`, `tests/tables.rs`, `docs/model.md`'s
  "Tables"): the spec's `tables` (`[{id: "t:<k>", rows: [[cell, …], …], prefer?}]`) are read.
  `plan` and `edit` refuse a malformed or duplicate ID (naming the next free ID), no rows, a
  string row, unequal or empty rows (draftwright's `Sheet.table` refusals), a row that is not a
  list, a cell that is not text or a number, and an unknown `prefer`; `edit`'s `add` on `tables`
  takes the next free ID. Each table is a Plan item (`tables`: ID, cells, `default: false`,
  basis, placed: each cell's ink centre, the column edges, the box), reserved towards its corner
  when the sheet is chosen (ahead of the notes, in draftwright's order), placed after the
  dimensions and the notes block by `fit_box` towards its corner clear of the views, the
  dimensions, the title block, the notes and earlier tables, drawn ruled as `_build_table` rules
  it with each cell one invisible text run; a table with no place is a `table_not_placed` error
  naming it, which stays in the plan undrawn. The notes block now takes its text metrics, ink,
  obstacles, placement and rules from `src/tables.rs` (`tests/notes.rs` unchanged). Compared with
  draftwright on m10127 and ctc01 (`tools/capture_tables.py`, `tests/fixtures/tables.json`):
  rows, height, column widths and cell ink centres agree to 1e-4 mm; ctc01's corners agree;
  m10127's sheet already differs. Differences and verdicts in `docs/model.md`. Not ported:
  `block_cols`, `left_align_cols`, `Sheet.schedule`'s measured tables, hole tables. Rule
  proposals (files owned by other streams): ISSUE-1146-c's rust_note ("Authored tables do not
  exist in the port") is stale: authored tables now take part in page and scale selection
  (`tests/tables.rs::a_long_table_moves_the_sheet_on_rather_than_be_dropped`); ARCH0004-4 could
  cite the same test (stays pending: a caller-forced page is not yet checked to place every
  requirement). Questions for the maintainer:
  - `table_not_placed` is an error (draftwright's `table_dropped` is a warning, though its scale
    policy treats a declared table's drop as a lost requirement). Keep error?
  - A number cell is printed as JSON writes it (`0.5`, `1e+20` as `1e20`); Python's `str()`
    would differ for some floats. Keep accepting numbers, or require text?

- From stream d-rebase-lint-native-recesses (2026-10-08), for the integrator (applied at merge
  in integration run r-draftwright-rust-10: the three entries below, and ISSUE-827-a's and
  ISSUE-885-b's rust_notes refreshed; all stay pending):
  r/d-lint-holes-slot-patterns (1406131, merged here) proposes these docs/rules/issues.yaml
  changes, not applied because issues.yaml belongs to the issue streams. Copied verbatim from
  that stream's report; apply them to the three entries (only status, rust_test and rust_note
  change). Its author kept ISSUE-1142-b pending because centre-line overlap by text box is placement's, not lint's; if
  the maintainer treats that part as out of scope, the rule could be marked ported. Also stale
  since dims::recesses merged (not edited here, same reason): ISSUE-827-a's and ISSUE-885-b's
  rust_notes say pockets are not dimensioned (`dimensions_not_ported`); pockets, hexagonal
  pockets, circular seats and open channels now are (src/dims/recesses.rs; pads still are not),
  and ISSUE-885-b's "lint has no pad or pocket coverage" was already out of date
  (src/lint/pads.rs, src/lint/pockets.rs).

  ```yaml
  - id: ISSUE-1143-b
    status: pending
    rust_test: "tests/issue_trust.rs::a_callout_certifies_only_the_holes_it_claims"
    rust_note: "Partly ported. Checked: a callout claiming every member of one group does not certify another group of the same size; text with no claim certifies no hole; a callout claiming one member of a group certifies its common sizes (one truthful statement, as draftwright's _state) but not its quantity, whatever it prints: grouping.count is reported missing, naming the members no callout counts (draftwright's test_one_singleton_cannot_claim_an_entire_multi_owner_group). src/lint/holes.rs::uncounted: a member is counted by a diameter callout whose leading n× (or one) equals the holes it claims, all of them the group's. Holes drilled from opposite faces are separate groups (holes_drilled_from_opposite_faces_are_separate_groups). The port has no declarations or IR to join, so there is no tool-centre fallback. Not ported: hole locations (LINT-hole_coverage-location). Difference: draftwright credits a callout whose label was rewritten while its IDs survive; lint also needs the text to state the value (rust-correct, ISSUE-1217-a)."
  - id: ISSUE-1217-a
    status: pending
    rust_test: "tests/issue_trust.rs::a_claim_is_credited_only_when_its_text_states_the_value"
    rust_note: "Partly ported. Hole coverage reads a claiming callout term by term (src/lint/holes.rs::Callout): each size is credited only when the term that states it (the number after ⌀, after ↧, in its recess or countersink segment) rounds from the part's value at its printed precision; a repeat count, a grid's (r×c), a bolt circle's diameter, a tolerance or another size of the same value confirms nothing ('6× ⌀8 THRU' and '4× ⌀8 THRU (6×2)' no longer confirm ⌀6 on plate.step; draftwright's test_a_repeat_count_cannot_confirm_a_diameter). Stricter than draftwright's evidence.py, which strips only the leading count (rust-correct: the rule asks for every number of a compound callout). Not ported: the families sharing lint::parameters still accept any number in the label (holes::states); locations and pitches are not verified against spans; there are no hole tables."
  - id: ISSUE-1142-b
    status: pending
    rust_test: "tests/issue_trust.rs::a_claim_is_credited_only_when_its_text_states_the_value"
    rust_note: "Partly ported: lint credits a requirement only through a structured claim on the feature by ID, so a callout's words cannot claim a feature; a count is read from the callout's leading n× against the holes it claims (no count shortfall is waived, ISSUE-1143-b), and a bolt circle's diameter (after EQ/ON/BC) or any other term cannot confirm a hole's size. Not ported: centre-line overlap by text box (placement's)."
  ```

- From wave 5 (stream d-native-recess-families): `bridge::native` also reads through slots and
  slot patterns (members named by slot ID, as the bridge does), polygonal bosses and stock,
  double-D bores, plates, and gusset ribs and their patterns, each against a `FIELDS` table
  (member records checked too). `tests/native.rs` adds two fixture sets that need no corpus: the
  slot-pattern plates and quiddity-rust's golden double-D bore, gusset rib and hexagonal stock
  parts (`recognition/golden/`, Python quiddity 0.4.0.dev0): 43 parts. `known_native.json` grows
  by 71 entries (887 in all): 53 equivalent (named dimensions; a polygon's ring starting at
  another flat; a slot array's direction sense and a grid's angle and member order in the other
  frame), 11 rust-wrong (6 polygonal-boss and 1 boss free-end cap faces, the constituent-faces
  gap; m1000's slot, which Python's aggregate rejects as superseded by passages, twice; grm03's
  plate, which Python's orchestration does not look for on a turned solid, twice), 6
  undetermined (cgb245's slot body keys, OpenCascade against haecceity) and 1 rust-correct (the
  gusset mirror pair, which Python's document publishes only as its ribs' links, so the bridge
  drops it). The recess families (pockets, channels, pocket patterns, blind slots, section
  passages, edge-open pockets and recesses) stay known and not read: draftwright reads them only
  as `section_recess` features, which the document does not carry.
  - Questions for the maintainer:
    - Should quiddity-rust's document publish section recesses (Python's `SectionRecess`: frame,
      section profile, run interval, ends, classification) in place of, or beside, its internal
      pocket, channel, blind-slot, passage and edge-open records? Without them no native reader
      can give draftwright its pockets, channels, blind slots and passages.
    - The cross-family reconciliation question above now also covers slots superseded by passages
      (`reconcile_recess_candidates`) and plates on turned solids (`plates._discover`).
    - A native polygonal boss's fingerprint has no height (the document has no
      `named_dimensions`), and `demand.rs` counts z boss heights from fingerprint sizes: take it
      from `top - base`, or have the document carry named dimensions?

- From wave 5 (stream d-dims-recesses): `dims::recesses` dimensions the section recesses
  draftwright reads as pockets, hexagonal pockets, circular seats and open channels, through lint's
  port of `section_recess_fields` (`lint::section_recesses::grammar`, `lint::pockets`,
  `lint::channels`): a pocket's `W × L × D DEEP` (`MAX DEEP` for a cylindrical mouth) in the view
  that looks into it, located from the box to its centre (or its near end on the box) in both
  directions across it, in that view; `HEX AF A/F × D DEEP`; `øD × L LONG × A° ARC` with the seat
  axis's x, y and z locations; an open channel's width (only with plates along two axes, as
  draftwright). Identical hex pockets and coaxial seats are counted once, as draftwright batches
  them. Every recess item on the captured sheets agrees with draftwright's (16 more items agree in
  `tests/dimensions.rs`, which now compares a `recess` family), and so do m10047's and m10245's
  pockets in `tests/dims_slots.rs`. 4 new `known_dimensions.json` entries, all rust-correct: m1000's
  pocket y location in the plan, not the side (2 entries, ADR1-6 decision), and m10046's two pocket
  locations that draftwright drops (its own lint: `pocket_dim_dropped`). Depth values differ from
  draftwright's by 0.001 mm (m10127 9.960 against 9.959, m1000 3.446 against 3.445): the fixture's
  records are rounded in quiddity's frame, draftwright's in the part's; labels agree. Left:
  - Done on merge (stream d-rebase-lint-native-recesses): these `own_drawings` entries of
    `known_lint.json` came off, the lint test showing exactly these six stale (each requirement is
    now stated): ftc06 `channel_requirement_missing` (7), m1000
    `circular_channel_requirement_missing` (6), m1000 `hex_pocket_requirement_missing` (2), m1000
    `pocket_requirement_missing` (5), m10046 `pocket_requirement_missing` (5), m10127
    `hex_pocket_requirement_missing` (2). The views memory kill was main's (`dims::placement`
    `slides`, fixed since); the merged suite passes with nothing killed.
  - Placement, for the placement work (`known_placement.json`, rust-wrong): the recess callouts'
    long labels run past the estimated bands (m1000, m10046, m10127, ftc06's channel widths); m1000's
    seat callout runs into the title block; m10127's hex callout overlaps the side's depth label.
    On top of the steps and levels planners (merged since) the recess items also push ftc06's plan
    and front lanes 7 to 14 mm further out (28 items past the areas, the label over the isometric
    now the front width's instead of three plan callouts'), and on m1000 the seat's z location
    joins the front's lanes beyond the height ladder, so the front height's label lies over the
    side view and two side recess items' lines cross it (a new `annotation_ink_overlap`, 2).
  - Not done: quiddity's `section_recess_patterns` (draftwright's `N×` pocket patterns): the bridge
    does not read them, so pattern members would be dimensioned singly; no captured recognition has
    one. draftwright consolidates a seat's run onto the overall extent when the seat spans the box;
    the port always states the run. A spec tolerance on a recess callout follows its first value
    only (as for blind slots); draftwright suffixes each value. A recess the grammar reads but
    whose axes lie along no part axis gets `feature_not_dimensioned` and also
    `dimensions_not_ported` (the not-ported filter in `dims::wants` reads only stated features).
    Pads are quiddity's `RaisedPad` records, not section recesses: out of this stream's scope.
  - Questions for the maintainer:
    - draftwright locates no hexagonal pocket (only pockets and seats are locatable), and the port
      follows it; a hex pocket's position is then unstated. Locate it from the box like a boss
      (beyond draftwright), or keep draftwright's choice?
    - draftwright locates a seat along its own axis to the axis's midpoint, and sends x and z to
      the front and y to the side whatever the seat's axis. The port states the same three values
      but puts the two across the seat in the view along it and the one along it in its first
      profile view (the same views for a y seat, the only one sampled). Keep the along-axis
      location (half the run, for a seat open at both ends), and is the view rule right?
    - Section recesses draftwright has no grammar for (passages, triangular and 12- or 20-sided
      pockets) are still `dimensions_not_ported` info problems, whose message says "no dimensions
      for section_recess are ported yet", now only partly true. This joins the earlier question on
      undimensioned records (warnings, and from lint or the plan?).

- The CLI recognises with Rust quiddity (stream d-cli-native, maintainer priority 2):
  `draftwright analyse|plan|render|review` without `--features` or `--analysis` take features
  from `model::analyse_native` (`bridge::native::read` on quiddity-rust's document, the part's
  frame); `--features` still reads Python's JSON, and the new `--no-features` draws with none
  (the only way left to draw a part whose document the reader refuses). `api::Features` and
  `analyse_from`/`plan_from`/`render_from`/`review_from` carry the choice; `api::call`, the
  service, the MCP server and `api::analyse(step, None)` are unchanged. `draftwright.analysis/5`
  adds `unrecognised` (families the document has that are not read, with counts, and section
  recesses and pads, which it lacks); `plan` reports them as a `features_not_recognised`
  warning. By hand (default, native): cgb245 renders (39 dimensions against Python's 51: its 3
  slots are not read; 124 blends `dimensions_not_ported` against 82, the unreconciled blends;
  6 `annotation_off_sheet` against 10); GRM-05 renders with its bosses' step diameters and
  lengths (⌀6.2, ⌀6, 12.5, 5, 12) that Python's aggregate drops for its turned steps, and ⌀10
  twice (envelope and boss, rust-wrong); nist_ctc_04 renders (no area binding natively;
  pockets, slots and oriented chamfers named as not recognised). `tests/cli_native.rs`.
  - Question for the maintainer: should `draftwright serve`, the MCP server and the library's
    `None` follow the CLI to native recognition (with Python's JSON as the override), or stay
    as they are until the constituent-faces and reconciliation questions above are settled?

- Structural sheet lint (stream d-lint-structural): `lint::structural::check(&Drawing)` ports
  five of `linting/structural.py`'s codes (scale_not_stated, view_overlap, view_out_of_bounds,
  annotation_out_of_bounds, derived_view_identifier_reused); annotation_overlap is partly ported
  and label_vs_measured not ported (both pending in docs/rules/lint.yaml). They read a new record
  of drawn geometry, `Drawing::sheet` (drawable area, each view's visible-edge box, each
  annotation's and furniture's ink and label box), measured by `model::render`; spec tables are
  recorded too (`Kind::Table`, added on merge). Findings go to a separate `api::Review::structural`
  (MCP: `"structural"`), not `Review::lint`. `tests/lint_structural.rs`,
  `tests/fixtures/known_structural.json` (27 annotation_out_of_bounds findings on 4 parts, all
  rust-wrong: placement's known off-sheet items; 2 on 2 parts since stream d-placement-fit
  merged, cgb245's and ctc01's plan y locations).
  - Questions for the maintainer:
    - Should structural findings join `Review::lint`, as draftwright orders them? That changes
      tests/lint.rs's plate expectation (its y location sits 0.1 mm past the left border), and
      probably the MCP and serve expectations of an empty lint.
    - Is `Drawing::sheet` the intended home for lint's sheet facts, and should it grow to carry
      each linear dimension's measured path and scale so label_vs_measured and nominal_rounded
      can be ported?
- From stream d-bridge-area-tolerance: the bridge reads nist_ctc_04 (`render` draws it,
  146 features from quiddity main's recognition). Face 367 is the same face: the JSON's
  13.100238 is OpenCascade's default-precision area, off its own converged area (13.080648 at
  eps 1e-10, and by Green's theorem on the exact sphere) by 1.5e-3; haecceity's 13.080630 is
  the region its edges bound (the file has no pcurves). Every face past 1e-4 over all 4,031
  recognised faces is listed with evidence in `tests/fixtures/known_bridge_faces.json` (20:
  spheres on ctc04 and ftc07, rust-correct; one B-spline sliver, equivalent); spheres are now
  held to 1e-2 like B-splines, the other analytic types still to 1e-3. ctc04 is still left
  out of `tests/dimensions.rs`, `tests/native.rs`, `tests/lint.rs` (its two `known_lint.json`
  `analysis` entries now say "not compared yet", undetermined), `tests/trust_rules.rs`,
  `tests/issue_trust.rs` and `tests/issue_drafting.rs`: its differences are owed by the streams
  that own those tables. Comparing it already shows lint's `python_subject` finds no group for
  Python's oblique "4× ø12 blind depth 20" finding. The "still refused" note in this file's
  summary above is out of date.
  Question for the maintainer: area cannot tell apart same-sized faces (a pattern's instances);
  should `tools/recognise.py` also write each face's area centre, so the bridge proves order by
  place?

- From stream d-dims-turned-steps (CLI-trial priority 4; `src/dims/turned.rs`,
  `tests/dims_turned.rs`): turned steps are dimensioned. Each band no groove owns (lint's
  `turned::bands`) gets `step.diameter` (a `ø` callout in the front view, led to its silhouette for
  x and z, to its circle for y) and `step.length` (one chain item per band in its first profile
  view, per turned profile); the overall diameter also claims the band it is (one band, or two
  separate flanges as `2× ø130`, draftwright's `_share_unique_outer_diameter`); a z band printing as
  a coaxial bore reads `ø70 OD`; two equal z bands share `2× ø16`; and a turned part's overall
  length is withheld when its chain (bands and grooves) runs end to end. `turned_step` is claimed;
  an unreadable band, or every band when groove ownership cannot be decided, is
  `feature_not_dimensioned`. GRM-05 now reads ø10, ø6.2, ø6, 14, 5, 13 (and C1, C1.5), as
  draftwright's sheet; before, ø10 and 32. On the five captured turned sheets the step items equal
  draftwright's; `known_dimensions.json` loses 5 entries (the overall lengths, spool132's `2× ø130`)
  and gains grm03's ø10 band (equivalent: the port's overall ø10 states it). Not ported (placement,
  the planners do not see the scale): draftwright's smallest-diameter drop, chain stagger and dense
  drop, crowded-head details, and moving a parallel profile to the other view; a baseline ladder
  comes only from authored PMI, which the port does not read. Left:
  - Placement, for the placement work: the step diameter callouts stack in the lanes below the
    front with 45° leaders (draftwright: a column left of the front for z, a row below for x), so
    `known_placement.json` grows on connector109, grm03, grm05, spool132 and string_post
    (rust-wrong). The ratchet holds one entry per part and code, so those five parts' existing
    entries had their counts raised and a sentence added; two new `annotation_ink_overlap` entries.
  - The native reader (`src/bridge/native.rs`) reads every field the planner uses (axis, lo, hi,
    diameter, `profile.axis_origin`; grooves' axis, at, width, profile), and `known_native.json`
    shows those agreeing with Python on every pair. A native step whose `profile` is null (the
    schema allows it) is reported, not dimensioned: draftwright then puts the band on the box's
    centre line. Profile grouping and groove ownership compare whole `profile` objects, as lint
    does.
  - Questions for the maintainer:
    - draftwright collapses three or more chain lengths within 10 % of their mean into `N× L`
      whatever they print, so `3× 23.3` can stand for 23, 23.3 and 23.7. The port collapses only
      when each prints as `L` (no captured sheet has a collapse). Keep the stricter rule?
    - A z band whose diameter equals a coaxial hole's counterbore or spotface (not only its bore)
      reads `OD`, standing in for draftwright's rotational bores (spool132's ø70, which the port
      states as a spotface). Is `OD` wanted beyond z (draftwright limits it to z, where its bore
      leaders share the front view)?

- From stream d-placement-fit (CLI-trial priority 3, 2026-10-08): annotations stay on the sheet
  where they can (`src/dims/placement.rs`, `tests/placement.rs`).
  - **Fall-through** (ISSUE-1215-e, now ported): a strip holds its linear dimensions up to the
    first lane whose item does not fit the page (ink on the sheet, label off the views and the
    title block, no line through the title block or another view); that item and those outside it
    take the next free lanes of the strip across the view. One that fits on neither side keeps its
    lane and is reported, never dropped. Edit-local (the page and every item's lane come from the
    full set).
  - **Callouts beside their view**: a callout whose lane does not fit the page (off the sheet, or
    its leader across another view) turns left or, failing that, goes out beside its view as
    draftwright's plan callouts do (`holes._leader_anchors`): the elbow past the strip's lanes on
    a row within the view's height, the leader radial from the nearest site's rim, rows
    `pad_around_text` apart where they can be.
  - **New warning `annotation_line_over_view`**: a line (leader, extension or dimension line)
    across a view not its own; such lines ran unreported before.
  - **`slides` bounded by the sheet**: its places keep the label on the drawing area along the
    line, so their count is bounded by the sheet, not the line (a unit test on a 200 km line).
  - cgb245 before → after: `annotation_off_sheet` 10 → 1 (d:f:hole:1's y location, whose next
    lane right of the plan would cross the isometric), `annotation_ink_overlap` 2 → 0,
    `annotation_outside_area` 30 → 24, plan callout leaders across the isometric: none now (the
    2×2 pattern's, 2× ⌀5's, 4× ⌀7's, the fillet's and one end radius go beside the plan; the
    three outermost keep their lanes above the plan, their leaders long but clear of the
    isometric), 6 `annotation_line_over_view` (the front's overall height and 5 ladder rungs
    through the side view; unreported before). All 12 compared parts: `annotation_off_sheet` 24 →
    2 (cgb245 1, ctc01 1); ink overlaps 9 → 4; `known_placement.json` updated with verdicts.
  - **Escalation is not what cgb245 needs**: draftwright's captured sheet
    (`tests/fixtures/sheets/cgb245.json.gz`) is A2 at 1:2, the port's sheet too; draftwright's
    scale attempts there (1:5, 1:1, A1, A0) were all rejected and it kept A2 1:2. The difference
    is where the y locations go (draftwright: the side view, staggered-side arrangement; the port:
    the plan, by the end-on rule ADR1-6) and the bands: the plan's left band is draftwright's
    20 mm estimate. ISSUE-1299-a (escalation, in `model::plan`) is unchanged.
  - Left, with blockers: the front's height ladder still runs into the side view on cgb245 and
    m1000 (the front's right band is draftwright's estimate and draftwright withholds those rungs
    by its legibility gate: both open maintainer questions above); the strip right of the plan,
    where items now fall, is reserved by no band (`layout::Strips::reserve`, in `src/layout.rs`,
    outside this stream), so those items are `annotation_outside_area` (info); a leader crossing
    other dimensions' lines is not checked; ftc06's front width and a boss location still cross
    the isometric (no room above the front either).
  - Outside the stream's files, minimal: `src/dims/mod.rs` wires `fall_through` and `leaders`;
    `tests/issue_drafting.rs::a_label_over_the_isometric_is_reported` moves its crowded guard
    from plate on A3 at 2:1 (where every label now fits) to A4 at 5:1, still asserting labels
    over the isometric and off the sheet; `tests/tolerances.rs` expects the one info problem the
    longer `7× R50 ±0.5` label adds beside ctc01's plan; `docs/model.md` names the new code.
  - After review (stream d-placement-fit): an overall extent now has first claim on the opposite
    strip's free lanes (`fall_through`; draftwright falls through only the extent), placed outside
    the other moved items where that still fits. On cgb245 and m1000 the front's overall height
    takes the one lane left of the front (before, cgb245's rung h9_876 and m1000's seat z location
    took it and the height crossed the side view); on ftc06 the front width takes the one lane
    above the front, off the isometric. Those lower-priority items keep their lanes and stay
    reported (`known_placement.json`: ftc06 `annotation_over_view` and m1000 `annotation_ink_overlap`
    off; cgb245's three rungs h21–h25 now have labels over the side view). Falling through every
    linear item, beyond draftwright, has a verdict (rust-correct, with the split-ladder cost) in
    ISSUE-1215-e's note, whose rust_test is now
    `tests/placement.rs::a_starved_overall_extent_has_first_claim_on_the_opposite_strip`.

- Structural sheet lint, second part (stream d-lint-structural-2): `lint::structural::check` now
  also ports title_field_overflow, leader_line_through_text, annotation_ink_overlap (and the
  line-work remedy annotation_overlap lacked, so annotation_overlap is ported),
  label_centerline_overlap, view_annotation_overlap and view_annotation_inside_extents; in
  docs/rules/lint.yaml 7 more entries are ported (pending 131 → 124, ported 34 → 41).
  `output::Sheet` grew what they read, all measured in `model::render` as the ink is drawn
  (plan untouched): each thing's straight line-work (`Drawn::lines`: extension, dimension and
  leader lines, the title block's rules) and a callout's elbow, each view's visible edges
  (`DrawnView::edges`), each title-block value's ink and cell (`Sheet::fields`), and a
  section's furniture as draftwright places it (hatch, chain line as a `Kind::Centreline`,
  each arrowhead and wing stub), where it was one `SectionInk` box. New fixture
  `tests/fixtures/sheet_lint/` (13 sheets, 410 KB), written by `tools/capture_sheet_lint.py`
  from draftwright 4dfc63f: what its one `lint_drawing` call read (each item's boxes,
  line-work, elbow and riders, each view's edges, the title fields) and found; the port's
  checks on that record give exactly draftwright's 18 findings in these codes (ftc06's ten
  crossings, cgb245's and ctc04's one, ctc04's overlap, five labels over blank face). On the
  port's own drawings: 9 annotation_ink_overlap and 4 view_annotation_overlap, all rust-wrong
  and each one of known_placement.json's annotation_ink_overlap / annotation_over_view items
  (known_structural.json; the test checks a finding between dimensions is among the plan's).
  Merged after stream d-placement-fit, those became 3 annotation_ink_overlap (ftc06 1, m10127 2),
  1 view_annotation_overlap (m1000) and 3 view_annotation_inside_extents (cgb245's rungs
  h21–h25 over the side view's blank face), all rust-wrong.
  Still pending: label_vs_measured and nominal_rounded (the measured-path question above),
  interior_label_on_narrow_material and leader_crosses_silhouette (they read the view's filled
  material, which the drawing does not record), missing_principal_dimension (below).
  - Questions for the maintainer:
    - draftwright's sheet lint never runs missing_principal_dimension: its one `lint_drawing`
      call passes no `part_bbox`, so `_lint_principal_extents` returns at once on every
      finished drawing, and no captured sheet reports it. Port it anyway (the drawing would
      need the part's box and scale), leave it pending, or drop it in favour of
      `lint::envelope`'s ledger?
    - title_field_overflow is now reported twice by `api::review`: as a plan problem (asking
      for a concise value) and as a structural finding (draftwright's message, pointing to a
      notes block). Keep both, or drop one?

- From stream d-native-pockets-slots: `bridge::native` reads oriented slots (`oriented_slot`
  features, `FIELDS` as Python draftwright's converter reads them: width, length, centre, width and
  long directions, the source passage's frame, run interval, section and ends) and checks
  oriented slot patterns (fields, each member exactly one oriented slot, none in two patterns)
  without making them features, as the bridge reads none of Python's (`DERIVED_NOT_READ`, #1430);
  `READ` is 25 families. Over the 100 corpus parts only mfcadpp_holdout/467 has an oriented slot
  in quiddity-rust's reading (and none has an oriented slot pattern), so `tests/native.rs` adds it
  (set `oriented`, Python quiddity 0.4.0.dev0) and quiddity-rust's golden oriented slots (`golden`,
  3 slots in one array): 4 oriented slots and 1 pattern compared field by field, 45 parts in all.
  21 new `known_native.json` entries, all equivalent: the passage frame's run sense and in-plane
  basis, the width and long directions' senses, and so the section's vertices and run interval,
  chosen canonically in each recogniser's frame (the prism checked identical, to the 0.001 each
  side rounds to; quiddity-rust's known_framed_document.json shows the same mechanism
  between its own default frame and caller space for 467), and the
  array's direction sense, member order and named dimensions. Faces, widths, lengths, centres,
  ends and body keys agree, and so do all 4 fingerprints. Downstream: `plan` on 467 and the golden
  part now reports each oriented slot as `dimensions_not_ported`, exactly as with `--features`,
  and no longer names oriented_slots in `features_not_recognised`; their SVGs are byte-identical
  before and after. Nothing to propose for the catalogue (the port draws and lints no oriented
  slot).
  - Not done, and why: prismatic pockets stay known and not read. Python's published document has
    no `prismatic_pockets` (they are in `_LegacyRecognitionResult` only); Python projects them into
    `section_recesses` (`result.py` `_prismatic_pocket_recess`), which is what draftwright lints
    and dimensions, so there is no Python record to compare a native one with, and reading them as
    `prismatic_pocket` features would give consumers a kind the `--features` path never has (the
    hexagonal pocket callouts come from section recesses). Quiddity-rust's document has prismatic
    pockets on 45 corpus parts.
  - Question for the maintainer: this is the earlier section-recess question again, now for
    prismatic pockets: should quiddity-rust's document publish Python's `SectionRecess` (into which
    prismatic pockets, pockets, channels, blind slots and passages are projected), so the native
    path can give draftwright its pockets?
- From stream d-callout-leaders (maintainer top priority, "something is wrong with the leaders,
  especially diagonals"): callout leaders are placed as draftwright places them, in place of the
  fixed 45° lanes (new `src/dims/leaders.rs`; `src/dims/placement.rs` loses `place_callout`, the
  turned and sideways leaders; on top of r/d-placement-fit and r/d-dims-turned-steps, merged here).
  - **What it does.** The linear dimensions are placed and settled first; then every callout
    (holes, recesses, turned diameters, fillets, chamfers, grooves, flats, bosses, steps) gets a
    bounded inventory of leaders: columns beside its view (draftwright's plan/side hole callouts and
    z-turned diameter column), rows above and below (its front hole callouts and x-turned row), and
    16-direction escapes from each site (its corner and radial candidates), each elbow just past
    the view's edge or past the dimensions of that strip, plus two text rows further out; the tip on
    the site's rim, radial to the elbow; a horizontal shoulder and the label beyond it on the side
    the leader runs to. Each is measured analytically: hard blockers (off the drawing area, in the
    title block, label or shoulder over any view, leader across another view or back into its own,
    leader under 6 mm or over 75 mm), then crossings (a line through a dimension's or leader's line,
    including running along it within 1 mm, or through a label, or a label on a label), then length
    (µm). The callouts are assigned jointly (groups that could conflict, exact branch and bound
    within 200 000 nodes from a greedy floor). Nothing is dropped; what a leader still crosses is
    reported, with a new plan problem `annotation_leader_crossing` (warning) for a leader across or
    along another item's line. What the collision checks cannot see is reported by the leader
    placement itself: a leader taken back across its own view (`annotation_line_over_view` on its
    own view), one over the 75 mm limit (`annotation_leader_long`, warning), and a group whose
    joint search stopped at its node budget (`annotation_leader_search_capped`, info).
    `tests/leaders.rs` asserts the first two are reported wherever they occur (neither occurs on
    the parts checked). The third fires on ftc06, whose 10 plan callouts form one group: listed
    in known_placement.json as undetermined (a 100× budget gives the same leaders). The search
    now starts from the greedy assignment improved by descent (single jobs, then conflicting
    pairs, at most 50 passes); before it, ftc06's bounded search kept an assignment 28 mm longer
    in total, which the review's question about the budget brought out. The limits (6 mm, 75 mm, 3 mm clearance, 7 mm rows) are calibrated from draftwright's
    sheets, not its code: draftwright has no hard length cap or angle limit, and its
    `leader_policy.py` is the region policy only; its side rule (`text_side`) is ported, and the
    port has no angle limit. Callouts take no lanes now (`fall_through` ignores them).
  - **Acceptance** (`tests/leaders.rs`): on the 12 captured parts with corpus recognition, and on
    cgb245 and nist_ctc_04 natively, no callout's leader, shoulder or label crosses a view, the
    title block, another annotation's line or label, except the 39 listed in
    `tests/fixtures/known_leaders.json`, all rust-wrong with evidence: 8 on connector109 (the
    turned bosses' ø118/ø122/ø130.9 are linear dimensions below the front whose extension lines run
    up the silhouette, where groove, fillet and chamfer leaders leave it; draftwright states those
    diameters as callouts) and 31 on ctc04 native (the plan's location ladders' extension lines run
    from interior holes through the view; the ⌀6.6 holes in the centre pockets cannot leave without
    crossing some; draftwright's own ctc04 sheet has 24 leader-line crossings by the same test).
    cgb245, both ways, has none (before: three plan callouts' 45° diagonals from the lanes above
    the plan, and leaders fanning across the plan to its right).
  - **Leader lengths** (mm, min/median/mean/max; port's from `LEADERS_REPORT=1`, draftwright's
    measured on its own drawings built from origin/main with the same predicates; draftwright draws
    more callouts on most parts, its blends and some fillets not being ported):
    cgb245 6.0/9.6/16.5/49.6 vs 7.0/15.0/19.8/52.9; ctc01 6.0/16.0/17.5/39.3 vs 7.0/7.0/16.2/52.0;
    ctc04 (native) 6.1/11.5/20.2/57.6 vs 5.6/14.9/19.1/47.0; ftc06 6.1/9.3/13.8/34.1 vs
    6.4/15.0/18.8/47.0; connector109 6.0/6.7/19.1/73.7 vs 7.0/10.3/18.8/73.5; spool132
    6.1/10.9/17.6/37.0 vs 7.0/37.7/31.3/62.2; m10127 6.0/8.7/11.5/19.9 vs 7.0/39.6/32.3/60.0;
    m1000 6.4/8.5/11.4/21.9 vs 7.0/16.7/14.4/17.2; m10046 30.4 vs 18.7 (one callout);
    grm03 6.1/7.8/8.2/11.4 vs 7.0/11.8/15.3/28.5; grm05 6.0/6.6/6.5/7.0 vs 10.8/13.0/12.9/15.0;
    string_post 6.0/7.6/7.5/8.9 vs 8.5/11.1/15.1/30.4; hanger 6.0/6.2/6.2/6.4 vs 7.0/7.0/9.0/15.0.
    On most parts the port's are shorter (both minimise length; the port's floor is its 6 mm
    minimum, draftwright's its 7 mm reach/lane spacing; and draftwright's longest run to labels
    in its side columns or inside the views). Four parts have longer ones, each with a verdict
    (callout by callout, from `LEADERS_REPORT=1` and a candidate dump of the port's inventory):
    - ctc01 (median 16.0 vs 7.0): the populations differ. Six of draftwright's 15 callouts are
      blends at 7.0 mm each (R0.8, R4.9, 6× R5, R6.3, 18× R10, 4× R25), which the port does not
      dimension (not-applicable: blends not ported). Callout by callout the port is shorter on
      seven of nine (⌀20 16.0 vs 23.0, ⌀35 9.5 vs 22.7, 2× R20 19.4 vs 52.0, R1.6 16.8 vs 44.6,
      C50 8.0 vs 15.0, 2× C5 6.1 vs 15.0, 7× R50 6.0 vs 15.0) and longer on two: 4× ⌀25 36.5 vs
      7.0 and HEX 100 A/F 39.3 vs 7.0, where draftwright's labels stand inside the plan over its
      face (not-applicable: interior labels, below).
    - m10046 (30.4 vs 18.7, its one callout): equivalent as leader placement, the difference is
      the layout. Draftwright lays the part out on A3 at 2:1 with nothing right of the front, and
      its label sits there level with the pocket. The port's A4 at 1:1 puts the y location ladder
      (13.5, 41.0, 7.7, 1.8) right of the front and the 23.0/27.3 ladder below it. Every shorter
      candidate in the port's inventory crosses one of their lines (right, down) or puts the
      40 mm label off the drawing area (left, 21 mm to the sheet's edge), so the 30.4 mm leader
      up to a row above the view is the shortest that crosses nothing.
    - ctc04 native (max 57.6 vs 47.0, mean 20.2 vs 19.1): three of the port's four ⌀6.6 callouts
      run 47.1–47.5 mm against draftwright's 47.0 (equivalent: both leave the centre pockets
      horizontally for a column beside the plan). Draftwright draws two `3× ⌀6.6 ↧ 25` callouts
      and the port four (the patterns' grouping, upstream of leaders), and the fourth takes the
      next row out, 57.6 mm (not-applicable to leader placement). The port's 16× R4 runs 45.1 mm
      against draftwright's 7.0: draftwright's label stands inside the front over its face
      (not-applicable: interior labels). No candidate of the port's is free of crossings: the
      chosen one sets its label against the 60.6 step height's label (listed in
      known_leaders.json), and the shorter ones, up or down out of the front, cross two or more
      of the dimension lines in the strips above and below it.
    - connector109 (mean 19.1 vs 18.8): the chamfer's 73.7 mm against draftwright's 7.0 (rust-wrong,
      below) accounts for it; the groove leader up the profile, 71.2 mm against draftwright's
      73.5 for the same groove, is equivalent.
    Shoulder: 3 mm (the port's `SHOULDER`)
    against draftwright's 2 mm (`pad_around_text`), equivalent. Median angle from horizontal:
    10–90° against draftwright's 0–72°, no pattern (both take vertical and horizontal leaders).
    Where they differ: draftwright puts some labels inside a view over blank face (its interior
    candidates; most of its leaders over 30 mm, on m10127, spool132 and ctc01, end at such labels
    or at rows of its side columns) — the port cannot prove blank face without the projected line-work, so it keeps every label outside
    (not-applicable to the plan as R1 defines it; draftwright's `--leader-region exterior`).
    connector109's chamfer leader runs 73.7 mm down the profile's edge (rust-wrong: the column left
    of the front holds the diameters, and shorter routes cross the bosses' extension lines; a
    leader laid on the part's own silhouette is not seen, the plan having no line-work).
  - **Visual comparison** (PNGs of both, kept in the scratchpad): cgb245's plan callouts now sit
    just outside the plan next to their holes with short leaders and shoulders (the one above it,
    2× ⌀6, just over its edge under the ladder), none crossing the ladder or another view;
    connector109's step diameters form a column left of the front as draftwright's do, its groove
    callouts right of it; spool132's and ctc01's plan callouts stand just outside the plan (draftwright's
    stand partly inside it); ctc04's ring of 14 mm holes is called out from both sides of the plan,
    the 6.6 mm holes by long horizontal leaders out of the centre pockets.
  - **Outside the stream's files, minimal**: `src/dims/mod.rs` (`mod leaders`, the call after
    `settle`, its doc); `tests/tolerances.rs`'s ctc01 test (the fillet's longer label no longer runs
    past the area: the spec adds no problem now); `tests/issue_drafting.rs::an_arrowhead_in_another_label_is_reported`
    (its guard needed m10127's arrowhead-in-label collision, now gone; the planted case is the unit
    test `dims::placement::tests::an_arrowhead_and_a_leader_through_other_ink_are_reported`);
    `tests/lint_structural.rs::the_review_reports_structural_findings_beside_coverage` and
    `tests/fixtures/known_structural.json` (for the integrator reconciling with
    d-lint-structural-2): r/d-placement-fit forked before lint::structural landed, so merging it
    here moves the plate's y location right of the plan (no structural finding or off-sheet
    problem now), cgb245's and ctc01's annotation_out_of_bounds go 10 → 1 each (the one y location
    each that fits on neither side), ftc06's 1 → 0 (both from placement-fit's fall-through), and
    connector109's 6 → 0 (this stream: its callouts no longer stack in lanes off the sheet).
  - **Catalogue** (`docs/rules`): ISSUE-740-a, ISSUE-740-b, ISSUE-798-a, ARCH0014-3 and ADR2-9
    ported; ISSUE-1188-a translated; ISSUE-1166-a/b, ISSUE-367-a, ADR2-D6, ADR2-5, ADR2-7, ADR2-8,
    ARCH0009-7 pending with exact notes. Proposed for files this stream does not own: `docs/model.md`
    should list `annotation_leader_crossing`, `annotation_leader_long` (a leader over 75 mm, taken
    only when every shorter place was blocked) and `annotation_leader_search_capped` (info: a
    group of callouts whose joint search stopped at its node budget) beside the other annotation
    codes, and its "Not checked
    yet" note should say leaders' line crossings are now checked (two linear dimensions' still are
    not); `docs/rules/lint.yaml` LINT-code-leader_crosses_silhouette stays pending (no projected
    material in the plan).
  - Questions for the maintainer:
    - Interior labels: draftwright places a callout's label inside its view over blank face when it
      can prove the face blank (its default `auto` region); the port keeps every label outside, the
      plan having no projected line-work. Should the plan see the views' line-work (a planned-view
      projection at plan time), or is exterior-only the port's rule?
    - The layout's band estimate still counts lanes for callouts (`dims::lanes`), though callouts no
      longer take lanes; dropping that would change view positions on most sheets (`known_views`).
      Keep it as a margin for the callouts beside the views, or reserve their actual footprint?
    - Turned bosses' diameters as linear dimensions below the front (connector109) make every
      leader leaving that side cross their extension lines; draftwright calls them out in its
      left column. Should `dims::bosses` give a turned boss's diameter as a callout?

- Coverage backstop (stream d-lint-coverage; `src/lint/coverage.rs`, `tests/lint_coverage.rs`,
  `tools/capture_lint_coverage.py`, `tests/fixtures/lint_coverage/`): draftwright's
  `lint_feature_coverage` and `lint_location_coverage`. `lint::check` now runs
  `feature_not_dimensioned` (before boss heights) and `feature_not_located` (after them), as
  draftwright orders them; `feature_count_mismatch` is ported but not run (draftwright's drawing
  lint passes `check_hole_counts=False`), and `feature_no_centermark` is ported but not run (the
  port draws no centre marks, so it would report every hole). On draftwright's recorded input (18
  drawings: the 13 sheet parts, ctc03, ctc05 and ftc09, which a hunt over the corpus found firing,
  and two drawings with callouts and centre marks taken off) the port's checks give exactly its
  findings in all four codes. On the port's own drawings neither wired code fires; the
  differences (centre marks on 9 parts, rust-wrong; cgb245's two holes the port locates in y,
  rust-correct) are in `tests/fixtures/lint_coverage/known.json`. Outside the stream's files:
  `tests/issue_trust.rs::a_claim_is_credited_only_when_its_text_states_the_value` now filters to
  hole coverage's codes (a misprinted diameter is also `feature_not_dimensioned`, as in
  draftwright). LINT-code-feature_count_mismatch is ported; the other three stay pending.
  - Questions for the maintainer:
    - Should the port draw centre marks (draftwright marks every hole in its end-on view)? Until
      it does, `feature_no_centermark` stays out of `lint::check`.
    - draftwright drops a boss diameter that a blend of the same face owns
      (`boss_blend_owner_pairs`, #1450), comparing quiddity's defining faces. The bridge keeps
      only constituent faces (`src/bridge.rs`), which differ (grm04_drive_plate: boss [0, 3],
      blend [0]). Should `bridge::Feature` also carry `defining_faces` (it changes the Analysis
      schema), so the ownership can be ported in dims and lint alike?
    - `feature_not_located` reads only claimed locations on the port's drawings; draftwright also
      accepts any dimension's witness point on the hole's projected coordinate. Should
      `Drawing::sheet` record each dimension's witness points and each view's projection (the
      same record label_vs_measured would need), or is the claim the right evidence for the
      port's own drawings?

- From wave 3 (stream d-spotface): a spotface is stated with ASME Y14.5-2018's spotface symbol
  (maintainer decision 2026-10-08), in the hole callout (`src/dims/holes.rs`, the term
  `dims::holes::SPOTFACE`) and in lint's fix text (`src/lint/holes.rs`): `⌀62 THRU ⌴SF ⌀70 ↧ 10`.
  - **Representation, and why:** Unicode has no code point for the symbol, and the bundled fonts
    (IBM Plex Mono and Sans Condensed) have none of the callout symbols `⌀ ⌴ ↧ ⌵` (checked with
    fontTools), which the drawing has always drawn as the font's `ø` and as strokes. So the term
    is the text `⌴SF`: `dims::label_ink` draws it as one symbol, the counterbore's open box
    widened to 1.1 text heights (`SF_CELL`) around the drawing font's own `S` and `F` outlines (set
    at 0.6 of the text height, inside the box); placement measures the same ink, so collision and
    band boxes include the wider symbol. The PDF's and SVG's invisible text runs carry `⌴SF`, so
    both `⌴SF` and `SF` are searched and copied, and the plan, replies and lint read the same
    text; no private-use code point reaches the JSON. Rejected: a private-use character with a
    glyph added to the font (the plan and replies would carry an unreadable character, and adding
    a glyph means modifying IBM Plex, whose OFL reserves the name "Plex", or synthesising glyphs in
    the PDF subset, which the `subsetter` crate does not do).
  - **Verdicts:** spool132's bore callout (`known_dimensions.json`) now prints `⌴SF` where
    draftwright's grammar prints `⌴` for a spotface: rust-correct by the maintainer's decision (the
    entry stays undetermined for its other difference, the bore's callout convention). Its
    `annotation_outside_area` overrun grows 25.5 → 26.7 mm with the wider symbol (count unchanged,
    reason updated in `known_placement.json`). The two-recess label in
    `tests/dimension_rules.rs` is now `⌴ ⌀11 ↧ 6.4 ⌴SF ⌀20 ↧ 1`, and lint still finds nothing
    missing on it.
  - **Lint's reader** takes `⌴SF` (and `⌴ SF`) as one spotface segment, and still takes a bare
    `SF`, draftwright's `⌴` for a lone spotface, the earlier two-`⌴` form and the captured sheets'
    symbol-less runs (unit tests in `lint::holes`). `tests/lint.rs` and `known_lint.json` needed no
    change; LINT-hole_coverage-spotface_diameter_depth was already ported.
  - **Tests:** `tests/dims_spotface.rs`: the symbol's ink (box strokes, the font's `SF` contours
    wholly inside, the label measured wider), and spool132 rendered: the callout's run reads back
    from the PDF's and SVG's text layers, and its `S` and `F` are real glyphs of the embedded subset
    font (not `.notdef`). The PDF/SVG text-layer readers moved from `tests/output.rs` to
    `tests/common/pdf.rs` so both files share them. Outside the stream's files:
    `tests/trust_rules.rs::no_document_infers_manufacturing_intent` lists `SF` among the printed
    words (the symbol's letters, not an inferred finish or fit).
  - Left: the box (like every callout symbol) is ink, not a font glyph; in the PDF's invisible text
    `⌴` is shown as `.notdef` and found through the ToUnicode map, as `⌀ ↧ ⌵` are. The symbol's
    proportions (box 1.1 × 0.7 text heights, letters at 0.6) are the port's, not measured from the
    standard's figure.
  - Questions for the maintainer:
    - Should the callout symbols (`⌀ ⌴ ↧ ⌵` and the spotface's box) be real glyphs in the embedded
      font, rather than ink with searchable text? That needs a modified, renamed font (OFL Reserved
      Font Name "Plex") or glyphs synthesised into the PDF's subset font.

- From stream d-issue-drafting-4 (catalogue proposals checked; issue regressions, fourth batch;
  `docs/rules/issues.yaml`, `tests/issue_regressions.rs` (new), `tests/issue_drafting.rs`,
  `tests/issue_trust.rs`, fixtures in `tests/fixtures/recognition/issues/`, and this entry).
  issues.yaml before → after: ported 21 → 22, pending 166 → 165, translated 3 → 3; adr.yaml
  unchanged.
  - **The recorded proposals, checked against main (9a4af8f).** All were already applied by
    integrators, and each cited test exists (a script over every `rust_test` of issues.yaml and
    adr.yaml: none missing) and passes in the full run: d-lint-holes-slot-patterns' ISSUE-1143-b,
    1217-a and 1142-b, and the ISSUE-827-a and ISSUE-885-b notes (e2bf434's two), in ddfa081;
    d-spec-tables' ISSUE-1146-c and ARCH0004-4 in 3927b65; d-dims-polygonal-stock's ISSUE-1082-a,
    1082-b and 1371-b with its own merge; d-dims-recesses' ARCH0015-1 note at its merge (db3a62c);
    d-placement-fit's ISSUE-1215-e (4061d81); d-dims-turned-steps' notes (a9ade84);
    d-callout-leaders' catalogue changes (7eae60f, 22f5159). ISSUE-885-b's "pads are not
    dimensioned" still holds (the whistle frame's three pads are `dimensions_not_ported`).
    d-cli-native, d-lint-structural, d-native-pockets-slots and d-bridge-area-tolerance proposed
    nothing for issues.yaml or adr.yaml. Changed here: ISSUE-1332-a, whose note said the port does
    not uphold it, now cites `tests/lint_structural.rs::overlapping_labels_name_the_line_work_that_also_crosses`
    (the sheet lint gives one code per pair with a consistent fix since d-lint-structural-2) and
    stays pending for placement's problems, which still raise both codes. d-bridge-area-tolerance's
    owed item is done: `tests/issue_drafting.rs` and `tests/issue_trust.rs` now read ctc04 with
    the other twelve sampled parts, and every test passes on it (the one count, 12 → 13, in
    `corpus_fixtures_are_bound_to_their_files`; four notes say thirteen). The stale "nist_ctc_04
    is still refused" line in this file's summary is corrected. Not applied, not this stream's
    file: d-callout-leaders' `docs/model.md` proposal (list `annotation_leader_crossing`,
    `annotation_leader_long`, `annotation_leader_search_capped`; the "Not checked yet" note at
    model.md:258) is still open. No lint.yaml proposal.
  - **Main was red at 9a4af8f** on `tests/lint_structural.rs::structural_findings_on_the_ports_own_drawings_are_known`
    (checked with this stream's changes stashed): the d-callout-leaders merge kept main's
    `tests/fixtures/known_structural.json`, whose m10127 `annotation_ink_overlap` (2) the new
    leaders removed, as `known_placement.json` already records. That one stale entry is removed
    here; nothing else in the file changed. (Main's a5174c4 removed the same entry independently,
    so at the merge this was already done.)
  - **Fourth batch.** Ported on the parts draftwright's tests build (`make.py` there; recognised by
    Python quiddity 0.3.9; draftwright's own runs of the same parts compared by hand), and its
    whistle frame (gzipped STEP, unpacked by the test): ISSUE-916-a **ported** (a pocket's leader
    tip is a rim side's midpoint on the matching axis, in four orientations and with a curved
    mouth; nothing crosses). Pending with a test and a note saying what is missing: ISSUE-1485-a
    (MAX DEEP and five broken records refused; the planar-substitution clause is lint coverage,
    left to d-lint-coverage), ISSUE-917-a (one channel width; wall thickness not dimensioned),
    ISSUE-1593-a (no dimension in the title block on draftwright's four cases; one with nowhere
    to go is kept and reported, not withheld and re-planned), ISSUE-443-a (GRM-03's four step
    lengths and the isometric on A4; draftwright reaches 5:1 by its legibility gate), ISSUE-1598-a
    (six hex pockets, one counted callout with twelve distinct claims, irregular sections
    refused), ISSUE-1613-a (three seats, six requirements each, coaxial locations shared), and
    ISSUE-1374-c (the plan side is now checked too; its test's doc said turned steps were not
    dimensioned).
  - **Found, not fixed (input, not `src/`):** the recognition document rounds records to 0.001 mm
    in quiddity's working frame, whose origin is off that grid (pocket_square's at -0.5634), so
    values read back in the part's frame are up to 0.0005 mm off and some labels print a decimal
    draftwright does not: pocket locations `35.0` (draftwright `35`), a curved pocket's depth
    `12.0` (`12`: the run interval is rounded, the mouth's axis given to the micrometre), the
    whistle seats' x location `15.6` (`15.7`; the exact value is 15.65). Recorded rust-wrong in
    ISSUE-916-a, 1485-a and 1613-a. Also observed: on the curved-mouth parts the plan conveys the
    cylinder's length by the overall extent (ISSUE-1154-a) while lint's boss coverage reports
    `boss_height_missing` on the port's own drawing (for d-lint-coverage); on the whistle frame
    the document has 11 flat records where draftwright's own run has 14, so the port prints
    `3× 6.50 A/F` against draftwright's `6× 6.50 A/F` (undetermined, not pursued); the stepped box
    of ISSUE-1593-a fits A3 at 1:1 in the port and A4 at 1:1 in draftwright; a sheet pinned with
    both size and scale is always `page_fit_uncertain`, even when it is the sheet the port would
    choose itself.
  - Questions for the maintainer:
    - Should quiddity's recognition document (and `tools/recognise.py`) carry coordinates
      unrounded, or rounded in the caller's frame, so a value on a whole millimetre reads back as
      one? Today the port prints `35.0` and `12.0` where the part is 35 and 12.
    - ISSUE-1613-a's and 1598-a's declaration clauses (a declared seat with a wrong radius or arc
      midpoint refused, declared hex pockets) are not applicable without declared builds; this
      joins ISSUE-676-c's question (retire such clauses, or keep them pending).

- From stream d-page-escalation (ISSUE-1299-a, page escalation recovery):
  - **Measured first** (the 13 captured sheets, `tests/fixtures/sheets/*.json.gz`, their
    `decisions.scale.attempts`, against the port's `model::plan` and its plans pinned at the
    candidates): draftwright re-planned on two parts, kept its first choice after rejected
    attempts on five (cgb245, ctc04, ftc06, m10046, hanger), tried nothing on six.
    - grm03: A4 2:1 → 5:1, `measured_upscale` against the recovery detail its estimate reserved.
      The port plans no detail views and its 2:1 plan is valid and complete: it keeps 2:1
      (not-applicable).
    - m10127: A4 1:1 → A3 2:1, `page_escalation_after_hard_layout` on its own
      `annotation_ink_overlap`. The port's A4 1:1 plan is valid and complete: it keeps A4 1:1
      (undetermined until placement agrees).
    - cgb245: draftwright's 1:2 is `hard_layout_invalid` (an `annotation_ink_overlap`, '297.9'
      through '15.0'); it tries 1:5 (rejected: 2 `location_ref_dropped` blockers, hole locations
      closer than 4.7 mm on the page), 1:1 (`view_out_of_bounds`), A1 and A0 at 1:1 (a required
      drop each), and keeps the invalid 1:2. The port's 1:2 has one `annotation_off_sheet`
      (d:f:hole:1's y location); its ladder tries the same four, rejects each (1:5: 4 hole
      locations too close to read; A2 1:1: `page_fit_uncertain` and labels off the sheet; A1:
      labels off the sheet; A0: an ink overlap; each with 2 locations too close to read besides)
      and keeps 1:2, saying so (`layout_recovery_exhausted`): equivalent outcome. (The first
      version of this stream took 1:5 here, valid by the port's placement checks but with
      location witness lines as close as 0.6 mm: its gate read no completeness at all. The
      reviewer's evidence; fixed below.)
    - ctc01: draftwright tries nothing (its A3 1:5 has no hard layout finding). The port's A3 1:5
      has one `annotation_off_sheet` (the hole pattern's y location, a placement residual it
      reports in `known_placement.json`); the ladder takes A3 1:10, valid and complete. The
      ladder and gate are draftwright's (equivalent); the sheet differs because the input does.
    - No other part's first plan has a hard layout finding or a location too close to read.
  - **Ported**: `layout::recover` (draftwright's `recover_hard_layout` and
    `recover_required_no_iso` order: a hard failure tries one scale each way on the sheet, then
    each larger standard sheet; a required loss tries the next two larger scales, then each larger
    sheet; a valid, complete plan tries nothing; a named sheet is never left; at most six plans)
    and `model::recover` (hard layout = `annotation_off_sheet`, `annotation_overlap`,
    `annotation_ink_overlap`, `title_field_overflow`, `page_fit_uncertain`, `views_do_not_fit`;
    `annotation_over_view` excluded, as the port judges it by the view's box and draftwright's
    `view_annotation_overlap` by line-work; never re-plans a named scale; says `layout_replanned`
    or `layout_recovery_exhausted`, info, and the sheet's basis names the recovery; the spec is
    kept). The completeness tier reads `layout::crowded_locations`: draftwright's
    `_legible_locations`/`_discard_short_refs` measure (4.7 mm between successive located
    ordinates of one view axis, 1 mm least span, a pattern's centre ordinate apart), which
    draftwright drops as `location_ref_dropped` (a blocker) and the port keeps
    (LINT-hole_coverage-location), so for the port it counts witness lines too close to read.
    Nothing else is dropped for want of room in the port. Each try is a plan (milliseconds, no
    projection). `tests/page_escalation.rs` ports draftwright's six tests (the stand-in ones on
    `layout::recover`; GRM-03, the named sheet and scale, and cgb245/ctc01/m10127/m1000 on corpus
    parts). ISSUE-1299-a stays pending, with notes.
  - **Differences from draftwright's retries, not ported** (stated in `model::recover`):
    draftwright rebuilds each candidate with the first drawing's settled arrangement and
    principal views; the port plans each afresh at the named sheet and scale, so the arrangement
    and view set are chosen again. draftwright's other candidate gates (a retained recovery
    detail, a recognised-requirement regression, the structural replay) have no input in the
    port; a principal view too large for the page shows as the try's `views_do_not_fit`.
  - **Not applied in `model::plan`: a conflict, not wired.** Wired (measured on this branch,
    rebased on local main b318337), only ctc01 changes sheet (A3 1:5 → 1:10); cgb245 keeps 1:2
    and gains the info `layout_recovery_exhausted`, which no fixture reads. Six tests go red, all
    on ctc01, in files other streams own: `known_placement.json` (ctc01 `annotation_off_sheet` 1
    gone), `known_structural.json` (ctc01 `annotation_out_of_bounds` 1 gone), `known_views.json`
    (ctc01 scale and every view's scale and centre), `notes.rs::the_block_matches_draftwrights_on_corpus_parts`
    and `tables.rs::tables_match_draftwrights_on_corpus_parts` (`sheets_compared > 0`: no part
    left at draftwright's sheet to compare), and
    `tolerances.rs::ctc01_prints_the_authored_bands_as_draftwright_does`. Wiring is `let first =
    plan_once(analysis, spec)?; recover(analysis, spec, first)`, with today's body as a private
    `plan_once` that `recover` re-plans through. Full suite wired: 9:58 wall here, nothing killed.
  - **Question for the maintainer**: should the recovery act on the port's own placement residuals
    now? Wired, it halves ctc01 to A3 1:10 to clear one off-sheet label, moving it off
    draftwright's sheet (draftwright's ladder and gate, applied to the port's findings), while
    cgb245 stays at 1:2 as in draftwright. The alternative is to wire it once placement no
    longer leaves ctc01's label off the sheet (d-placement-fit's note above: escalation is not
    what cgb245 needs, and now does not act there).
    Answered by the maintainer's directive of 2026-10-09 (quality before features): the recovery
    is applied to every plan the library gives (`model::plan_recovered`, d-quality-off-sheet).
    With the port's gate below, ctc01 goes to A2 1:5, not A3 1:10.
  - **Question for the maintainer** (d-quality-off-sheet): `model::recover` has one gate
    draftwright does not: a try with more `annotation_over_view` + `annotation_line_over_view`
    than the first plan is not taken. Without it native ctc01 goes to A3 1:10, clearing one
    off-sheet label but laying three items over the side view (ink 14 to 21); with it, A2 1:5,
    no worse in any column. Keep the gate (accuracy over parity), or follow draftwright's ladder
    exactly? Also disclosed, not gated: native GRM-03 goes A4 2:1 to A4 1:1 (draftwright 5:1;
    cause a leader label on an extension line in `leaders.rs`), and ftc07 A1 1:1 (draftwright
    A2 1:2; its A2 1:2 try fails on three ink-placement findings).

## Thin slice S3: placement as intent (2026-10-10)

Initial implementation committed by the driver as `908f178`: dimension and semantic PMI `place` (`view`,
`side`, zero-based `lane`, fractional/named `along`, `pinned`), and view `place`
(`relative_to`, `side`, paper `gap`, `align`, `pinned`). The model reference documents the
schema and anchor resolution. Fixed annotations participate in linear, leader, PMI and view
packing; pinned hole items cannot be dropped or replaced by a hole table. Unavailable anchors,
covered pins, collisions and sheet overruns are reported while their Spec entries are retained.
S4 sessions and S5 editor migration/fork retirement are outside this change.

Overturnable judgement calls made unattended:

- Retain the S1 `dimensions/<id>/view` spelling for compatibility; `place.view` wins. The
  editor's old `keep` spelling is not introduced: `omit: false` still follows consolidation.
- A view's relative anchor is another view name or `sheet` (the drawing area's lower left).
  Gaps use paper mm (default 10); alignment is start/middle/end. Lane and gap inputs are bounded
  at 1000 to keep layout searches finite. A pin without explicit strip/label fields uses the canonical automatic geometry.
- A pin fixes its carrier and the carrier's view-anchor chain for automatic packing. The
  canonical frames and automatic sheet choice come from the part's default recovered plan,
  with explicit sheet and scale choices applied. Recovery does not change sheet or scale under
  pins in response to unrelated edits. Editing an anchor's own intent is a related edit and
  intentionally re-resolves its dependants; sheet and scale edits also re-resolve them. These
  rules require no hidden session or page coordinates and replay from the Spec alone.
- Fixed ink bounds reserve conservative rectangles. Free views use deterministic edge-aligned
  candidates around these obstacles and the title block. An unpinned isometric may shrink;
  principal views keep their sheet scale. An infeasible placement is retained and reported.
- Existing angular PMI keeps its measured sector; lane adjusts its radius and along its arc.
  A side request is reported with a fix removing that field, rather than changing the sector.
- A placement fix on a pin resets the annotation's `place` to `{}` or removes the view's
  `place`, returning it to automatic placement. Free-item fixes suggest another strip/label
  position; callers must re-plan to assess them. If tabulation would replace a pinned hole
  item, keep that pin and report the original drops instead of duplicating its measurements.

Six S3 regression tests in `tests/placement_intent.rs` were run before implementation: all six
failed on the unchanged model's refusal of `place`/`annotations`. They were then strengthened
with seeded sequences (two seeds, 16 edits per seed, random N=1–4) on the editor plate, corpus
10127, GRM-03 and CTC-01, plus semantic AP242 CTC-01. They cover omission, tolerance and relative
placement edits of other annotations and views, sheet/scale re-resolution, collision and
missing-anchor reports, executable placement fixes, and SVG/DXF/PDF/PNG Spec extraction and exact
re-rendering. Existing test expectations were not weakened; two `ViewSpec` literals only gained
the new field's default.

Seeded pin distribution (before the other-item edits; all pins retain byte-identical geometry
after 16 edits per configuration). The corpus CTC-01 is presentation PMI; the AP242 fixture
adds semantic frames and datum symbols. This is a report, not a zero-collision gate:

| Part | Seed | Pins | Off sheet | View pin overlap | Label / line over view | Label / ink overlap | Leader crossing | Outside area | Fit uncertain | Dropped free items |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plate | 1379 | 3 | 0 | 1 | 2 / 4 | 0 / 0 | 0 | 7 | 1 | 1 |
| plate | 3227993 | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 1 | 4 | 1 | 0 |
| 10127 | 1379 | 3 | 0 | 0 | 0 / 0 | 0 / 0 | 0 | 3 | 0 | 0 |
| 10127 | 3227993 | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 0 | 1 | 0 | 0 |
| GRM-03 | 1379 | 3 | 0 | 1 | 0 / 0 | 0 / 0 | 0 | 3 | 0 | 0 |
| GRM-03 | 3227993 | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 0 | 4 | 0 | 0 |
| CTC-01 | 1379 | 3 | 0 | 0 | 0 / 0 | 0 / 2 | 3 | 9 | 0 | 0 |
| CTC-01 | 3227993 | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 3 | 8 | 0 | 0 |
| CTC-01 AP242 | 1379 | 3 | 0 | 0 | 0 / 0 | 1 / 1 | 19 | 11 | 1 | 0 |
| CTC-01 AP242 | 3227993 | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 23 | 10 | 1 | 0 |

The supplied, deliberately conflicting pins can leave collisions. They remain drawn and their
IDs are reported. Source/recognition/PMI coverage findings are excluded from this layout table.


Final validation on the completed S3 code (2026-10-10, `CARGO_BUILD_JOBS=4`,
`RUST_TEST_THREADS=4`; one Cargo command at a time):

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`:
  571 passed, 0 failed, 0 ignored, across 63 result groups (including doc tests).
  The six S3 tests also passed separately on the final code: 6 passed, 0 failed.
- `cargo build --release`: passed. No process was killed by the memory guard.
- The requested `sed`/`tr`/`xargs` invocation of `python3 -I tools/quality.py` ran on all 30
  `tools/quality_parts.txt` parts, with no pins. Compared every part and every quality column,
  including the five individual ink codes, against the latest values in
  `docs/quality-2026-10-09.md` (including its final CTC-02 correction): no regressions.
  FTC-10 measured four leader crossings instead of the table's five; all other cells matched.
  Results are in `target/quality-s3/baseline.json` and the rendered SVGs in that directory.

```text
                              off_sheet repeated_lab repeated_mea views_overla view_on_titl          ink
TOTAL                                   0           18            0            0            0          282
parts affected                          0            9            0            0            0           21 of 30
```

`git diff --check` passed. The initial agent made no commits or pushes; the driver then committed the work as `908f178`.

## S3 review fix round (2026-10-11)

Changes are uncommitted in this worktree; the driver owns commits. Fixed the major review
finding: `{place: {pinned: true}}` on a dimension or PMI annotation now keeps its canonical
automatic ink, rather than rebuilding it in lane zero. The ink is translated only when its
carrier anchor is deliberately changed. Explicit linear strip intent without a lane uses the
automatic lane. Pins still participate as obstacles before free items are packed.

Three added regressions were each observed failing before their corresponding fix:
`pinning_without_offsets_keeps_automatic_geometry` (plate hole-pattern callout moved),
`missing_pmi_placement_on_a_revision_is_inactive` (missing annotation refused the plan), and
`an_off_axis_pin_does_not_suppress_a_plan_view_table_attempt` (table attempt suppressed).
The first checks every placed dimension and semantic PMI item on the slice parts, byte for
byte, both on pinning and after an unrelated omission. The existing sheet/scale test now
checks off-sheet label and leader ink for its specific `annotation_off_sheet` problem; a
front-view pin fits A0, overruns A4 without moving, and must have `pinned_view_off_sheet`
with a fix. Re-resolution across sheets permits only floating-point rounding of its box
(1e-9 mm); the lower-left anchor is checked exactly.

Removed the redundant final dimension pin-copy loop; the placement stages themselves preserve
fixed items. Empty annotation maps skip the extra PMI lowering used for key validation.
A different source binding now makes PMI dimensions and annotations inactive alongside
feature entries, retaining them and their original binding with removal fixes; unknown PMI
IDs on the same source are still refused. GD&T's beyond-lanes retry runs with pins. A hole
table is prevented only when its replacement set includes a pin; an off-axis hole pin no
longer suppresses an unrelated plan-view table attempt. Corrected render defaults and restored
the original model design fields as documented, unimplemented design; corrected the initial
S3 report's stale commit wording.

Overturnable judgement call retained: automatic sheet/scale recovery stays disabled with
pins. Its retries change the carrier scale or page frame, contradicting the required
byte-identical pin position under unrelated edits. Explicit sheet/scale edits still re-resolve
pins and report failures. Recovering free items on another sheet while keeping pinned carrier
frames would require a separate layout contract; this round keeps the existing invariant.

Seeded comparison now reports the unpinned baseline alongside each pinned configuration (the
same deliberately conflicting explicit pins as the initial report). No new off-sheet ink in
any configuration; ink totals below use quality.py's five annotation codes. This is measured
layout degradation, not a claim that arbitrary explicit pins meet the later go/no-go gate.

| Part | Seed | Unpinned ink | Pinned ink | Off sheet before → after |
| --- | --- | --- | --- | --- |
| plate | 1379 | 3 | 13 | 0 → 0 |
| plate | 3227993 | 3 | 5 | 0 → 0 |
| 10127 | 1379 | 1 | 3 | 0 → 0 |
| 10127 | 3227993 | 1 | 1 | 0 → 0 |
| GRM-03 | 1379 | 2 | 3 | 0 → 0 |
| GRM-03 | 3227993 | 2 | 4 | 0 → 0 |
| CTC-01 | 1379 | 11 | 14 | 0 → 0 |
| CTC-01 | 3227993 | 11 | 11 | 0 → 0 |
| CTC-01 AP242 | 1379 | 26 | 31 | 0 → 0 |
| CTC-01 AP242 | 3227993 | 26 | 33 | 0 → 0 |

Final review-round validation (`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`, one Cargo
command at a time; no process killed by the memory guard):

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- Required full-corpus `cargo test --release`: **574 passed, 0 failed, 0 ignored**, 63 result
  groups including doc tests. Logs: `target/s3-fix-full-tests.log`.
- Targeted placement tests: **8 passed**; targeted hole-table tests: **11 passed**.
  Seeded baseline/pin comparison: `target/s3-fix-regressions.log`.
- `cargo build --release`: passed.
- Required `sed`/`tr`/`xargs` invocation of `python3 -I tools/quality.py`: all **30 parts**
  succeeded. Compared every documented column and all five ink codes with the latest complete
  table in `docs/quality-2026-10-09.md`, applying its final CTC-02 correction: **no part worse**.
  FTC-10 has 4 leader crossings / 22 ink rather than the table's 5 / 23. Results exactly match
  the initial S3 artifact, including dimensions and repeated drawn spans.
  Artifacts: `target/quality-s3-fix/baseline.json`, its rendered SVGs, and
  `target/s3-fix-quality.log`.

```text
                              off_sheet repeated_lab repeated_mea views_overla view_on_titl          ink
TOTAL                                   0           18            0            0            0          282
parts affected                          0            9            0            0            0           21 of 30
```

`git diff --check` passed. No commits or pushes in this fix round.

## S3 review fix round 2 (2026-10-11, Claude)

The two major review items are fixed; the two minor items are left as recorded below.

- **A bare pin's own label edits:** a bare pin (`place: {pinned:true}`) kept the canonical
  plan's ink, including the label box sized for the default text, so a tolerance added to the
  pinned item itself was drawn outside its box. `dims::placement::relabel` now re-sets the label
  from the item's current terms when they differ from the held terms (unchanged terms keep the
  held ink byte for byte). The lines and arrows stay; a linear or angular label keeps its centre
  along its direction, a callout's label keeps the edge at its shoulder and a shoulder that ran
  under the label follows the new width. On the editor's plate.step,
  `d:envelope/width.length = {tolerance:'±0.25', place:{pinned:true}}` now gives the box
  [72.305, 55.964, 87.695, 58.13], the same as unpinned. Regression test
  `a_bare_pin_relays_its_own_label_when_its_text_changes` (46 linear and 22 callout items on
  the slice parts; it fails without the fix). **Judgement call (overturnable):** a re-laid
  linear label is not moved outside its arrows if the longer text no longer fits between them;
  the held dimension line is kept, and any collision is reported as usual.
- **No sheet anchor for views:** `views/<name>/place.relative_to` accepts only another view
  (`front`, `plan`, `side`, `iso`, `section_aa`); `sheet` was a page coordinate in disguise
  (ADR R1 invariant 1, maintainer decision 2) and is now refused. The tests that pinned views to
  the sheet now pin them beside another view; the sheet-change test checks that the relation to
  the anchor holds on A0 and on A4 and that A4 reports `pinned_view_off_sheet`. In the seeded
  property test, the anchor chain of a pinned view is excluded from the "other items" edits,
  as a pinned dimension's carrying view already was (editing the anchor is editing the pin's
  own intent). docs/model.md updated.
- **Left (minor):** tools/quality.py has no random-pin mode (the distribution is still the
  cargo test's eprintln on the slice parts), and recovery stays off for the whole drawing while
  any pin exists, with the canonical plan recomputed on each `plan_with` (S4 latency).

Validation (`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`, one at a time): `cargo fmt --check`
passed; `cargo clippy --all-targets --release -- -D warnings` passed; required full-corpus
`cargo test --release`: **575 passed, 0 failed, 0 ignored** (63 result groups; log
`target/s3-fix2-full-tests.log`); `tools/quality.py` on the 30 parts: TOTAL off_sheet 0,
repeated_label 18, repeated_measure 0, views_overlap 0, view_on_title_block 0, ink 282, and
`baseline.json` byte-identical to the previous round's, so no part is worse than the latest
table in `docs/quality-2026-10-09.md` (per part, against the gate audit's "after" column, ftc10
ink 22 against 23 after the accepted leader-label weighting).

## S4 part A: render split and caches (2026-10-10)

Implemented the render split, placed geometry/view-bounds cache and single-report PMI reader.
See `docs/model.md`, "Rendering caches", and the measurements/checks in
`docs/status-2026-10-10.md`. No Session, operations, deltas or placement changes are included.

Overturnable judgement calls:

- Keep the latest placement/scale per projection, rather than every page origin visited during
  a drag. Include centre and origin in that key: translating previously measured bounds could
  change floating-point rounding relative to a fresh render. Moving a view rebuilds its placed
  curves/bounds while keeping HLR; unchanged views rebuild neither. Part B may change the cache
  policy if it can preserve byte equality and bounded memory.
- Expose independent fragments for the slice's dimensions/callouts and PMI frames/datums/labels.
  Whole-sheet furniture remains in the annotation assembly path; independent note/table edits
  are outside the slice's item vocabulary and would need their own fragment entry points.
- Keep a `pmi::Reader` per immutable geometry, retaining only the latest report (including empty
  and refused reports). Its owner replaces it on geometry replacement. Byte changes trigger one
  semantic read, even if the resulting requirements are identical. The kernel's separate STEP
  geometry/part inventory parser is unchanged; "one parse" means one semantic PMI parser feeding
  all three consumers, not eliminating the geometry loader.
- Timings are native warm-render samples, excluding planning and export. They show the cache's
  measured effect, not the complete S4 p90 or WASM latency gate; those belong to the session and
  editor streams.

## Thin slice S4 part B: Session (2026-10-11)

Overturnable judgement calls made unattended:

- S0's `bench/edits/*.json` are present on draftwright-editor `origin/thin-slice` and
  `origin/ts/s0-baseline` (and scratchpad `wt-ts-int-s0/bench/edits`). The initial report
  checked only main and incorrectly treated the files as unavailable. The shared sequence in
  `tests/common/session_edits.rs` now covers all nine edit kinds, including width's carrier
  move to `plan` and `views/side/omit`; GRM-03 has eight edits, without a carrier move.
  Keep the existing seven relative Spec operations as conservative equivalents of S0's old
  page-offset/Spec-1 commands; the new commands map directly to Spec-2 fields. This judgement
  is overturnable if S5 needs exact offset-to-anchor migration. No editor files were changed.
- Always use the bundled `tests/fixtures/parts/mounting_plate.step` for the plate benchmark
  and latency example invocation; results no longer depend on a host's editor checkout.
- Retain the existing regular-test and example p90 assertions against 150 ms, consistent with
  the maintainer's acceptance gate and this fix round's prohibition on weakened tests. A future
  dedicated performance job could isolate the gate from loaded CI; no such change is made here.
- S5/S7 payload follow-up: `open` serialises the full State including projected view geometry;
  moving/rescaling views resends their geometry (CTC-01 deltas around 5.2 MB). This remains
  acceptable in process but costly over JSON-lines; consider geometry handles/caching or compact
  transport in those streams, preserving lossless replay. This stream adds no transport policy.
- Count complete recovered-planner invocations, the same unit as the batch API/old editor's
  multiple plans. One per apply/undo/redo, zero for queries. The existing global planner has no
  safe partial-plan entry point, so use its full-plan fallback. Internal recovery candidates
  and S3's canonical pin plan remain; the counter explicitly does not count those trials.
  Removing those trials would alter fresh-plan equivalence and drafting behaviour. A future
  planner-locality API can replace this fallback under the same replay oracle.
- Keep one live part per JSON-lines connection, opened explicitly by `open`; a successful open
  replaces its history. Continue answering all existing stateless requests alongside it.
  No handles, persistence, paging or MCP transports are added (S7 owns that surface).
- Send typed curve/ink fragments, including searchable text and lint provenance, rather than
  add a second SVG-fragment formatter. Clients can replay a State and assemble/export with the
  library renderer. Sheet furniture is still assembled from the patched Plan, as in S4 part A.
- Options conservatively offer placements on the current carrier and compatible view anchors;
  they are not an exhaustive enumeration of alternative dimensional/PMI projections. Collision
  checks happen in preview/apply, without running extra plans from a query.
- Latency gate covers native apply, complete sheet assembly, delta construction and JSON
  encoding after open, ten repetitions of nine edits (eight for GRM-03). Undo restores the
  starting state between repetitions and is not timed. Opening (including HLR) is recorded separately.
  WASM/UI display-rate drag checks remain in S5; no editor or MCP files were touched.

## Thin slice S7: stateless MCP (2026-10-11)

Overturnable judgement calls made unattended:

- Use std's sequential loopback HTTP, OS file locks and adjacent staged/renamed files rather
  than add an HTTP framework or an MCP SDK. The local adapter uses Streamable HTTP's JSON
  response mode (GET 405, no sticky session), with synchronous cold computation. Leased cold
  jobs, distributed head CAS, authenticated handles and cache hydration belong to the
  draftwright-io integration design in `docs/mcp.md`; no production code was changed.
- Keep the existing `draftwright-mcp` executable as a compatibility adapter, so its existing
  document-passing tests and callers remain valid. New stateless tools live in `draftwright
  mcp-stdio` and `draftwright mcp-http`, and are also available as `mcp_<tool>` in `serve`.
  The legacy tools can be retired with the production rollout, rather than during this slice.
- The engine key combines package version, an explicit cache-format/behaviour revision and
  Cargo.lock. Bump `stateless::ENGINE_VERSION` whenever engine behaviour changes before a
  release; unversioned source changes are not automatically reflected in that revision.
- Retain the first source basename for a STEP content key, since Analysis uses it for the
  title/binding. Registering an alias reuses that analysis. Refuse different recognition JSON
  for an already-registered STEP+engine key: native and S0 bridge evaluation caches have
  separate roots. The key must stay STEP SHA plus engine version, so recognition mode is
  not silently added as a third key component. A future artifact registration API may make
  the canonical name and recognition policy explicit.
- Persist Analysis, raw kernel HLR/sections and the latest placed curves/visible bounds per
  direction, following S4's cache policy. STEP B-rep still parses in a fresh process; no
  recognition/semantic PMI or HLR/bounds work is repeated for warm, unchanged views. Enable
  serde_json's float_roundtrip feature to preserve exact doubles; no new Rust crate is added.
  Python jsonschema 4.25.1 is only an independent acceptance-check dependency.
- Default list/delta JSON budgets are 4096 bytes, with offsets against immutable inputs and
  ID/label/state rows. Explicit detail records that exceed the record budget become artifact
  references. Lossless Plan/Delta documents, images and complete Specs are explicit reads
  and may be larger. Further committed delta pages use resources/read rather than repeating
  apply_ops. The production adapter should map those content refs to its authenticated artifact
  handles. Initial Spec hashes identify content; applied version hashes include parent and ops,
  preserving a new immutable version even for a no-op.
- Restore S0 fixtures from editor origin/thin-slice, not its main checkout. Preserve tasks.json,
  all four feature files and the exact scripted baseline. Adapt only check.mjs's import/path;
  all success conditions, including info-level problems, remain unchanged. Token estimates
  are ceil(bytes/4), with judge calls excluded and fixed context reported separately as in S0.
- The scripted pin task reveals an upstream layout limit: right-side pinned plate callouts
  extend beyond the plan view's reserved right band (`annotation_outside_area`), and leaders
  can cross existing location dimensions. Placement ops keep the requested right-side pins
  and all other dimensions; MCP must report these problems, not filter them or weaken the
  task's `no_problems` condition. S7 does not change the planner's reservation policy. The
  scripted runner exits nonzero for an unsatisfied task, so this remains a measured go/no-go
  limitation for Stage D. The slot task uses ordinary above/lane-0/middle placement for both
  affected dimensions. A planner fix must preserve S3's pin invariance and unpinned quality.
- The S7 parity oracle covers native Session, live serve and stateless HTTP using all nine
  S4B S0 operations (eight on GRM-03). The editor's WASM session joins in S5. Production
  wiring and a real-model evaluation remain explicitly deferred; neither was attempted.

- Review correction: count the complete MCP JSON-RPC text/structured response in the default
  4 KB budget, reserving transport overhead in paging. Add explicit `type: object` roots to
  published output schemas as required by MCP's Tool definition; an independent schema
  check and the transport tests guard it. ID overlay text uses the bundled mono font at
  1.8 mm above each resolved item's ink bounds; it is review-only, not Spec intent.
- Tighten Session's linear placement options to the two sides parallel to the measured span.
  This is shared by the editor/core as well as MCP; it changes suggestions, not planning,
  geometry or accepted Spec operations. Callouts retain all four strip choices and collisions
  are still reported by preview/apply.
- Stage C integration: S5's geometry-backed `options()` (from `dims::placement_options`)
  replaced S7's side filter in the merge. It keeps S7's intent (linear dimensions offer only
  the two sides parallel to the span) and also covers omitted dimensions and GD&T.

## S5 editor integration ports (2026-10-11)

The library now exposes geometry-backed dimension/PMI placement choices through Session
options (batched once for browser reads), transactional PMI/spec replacement against the retained part/cache, typed-source PMI
coverage, and SVG fragments partitioned into views, annotations and sheet furniture.
Authored PMI carriers are validated geometrically before placement; a datum
view change chooses its outward side. Curved SVG edges explicitly disable fill while glyph faces retain it. Semantic internal
metric threads with a uniquely paired `thread manufacturing` attribute use native
manufacturing records/lowering, not a post-plan replacement. Symbolic ISO bore fits are
printed verbatim on uniquely matched bore owners (uppercase bore classes only); unstated numeric limits are not computed.

Overturnable judgements: an explicit manual title/fit/tolerance keeps the core's documented
precedence over imported defaults. Semantic thread nominal size is retained from its entity and checked against the designation;
missing fit classes remain blocked. The fork's strict outward-side rejection is dropped: S3
permits explicit pinned strip choices while preserving the attachment, as protected by its
existing seeded placement and export tests. Options/automatic sides retain outward normals. Unsupported, contradictory or ambiguous manufacturing
fields remain refused by native lowering. Semantic thread source IDs use their Part 21
instance as other record sources do, while their **item IDs** use S1 semantic identity;
the existing rewrite-stability test checks both. The editor carries its joint intent/drawing
history and sends Spec operations for restoration; Session PMI replacement clears its local
history because prior Specs alone cannot undo a different PMI reading. Browser packets omit
raw cached curves and carry changed SVG ink; full Session deltas remain available unchanged.

Added `tests/editor_session.rs` and its captured editor PMI fixture. The existing quintic
SVG test retains every geometry assertion and reads the d attribute independently of fill.
Validation and quality comparison are appended after the final checks.

Exact PMI ledgers are updated, with their exhaustive/no-stale-entry tests intact: four
semantic thread records are added to known_pmi.json. known_pmi_coverage.json removes
16 stale findings (consumed thread/attribute sources and bore fits now drawn symbolically)
and adds four exact not-lowered findings for historical `full thread depth` attributes.
That field is not reinterpreted as `minimum full thread`; the old editor fallback also
required the latter. These requirements remain read, supported and visibly reported.

The fit reading now treats a sole "class without limits" notice as understood only for a
valid uppercase symbolic class on geometrically proven internal principal cylinders.
It infers no numeric limits, and keeps every other reading or geometry blocker. Lowering
still requires one recognised owner. This fixes the unchanged no-blocked-records-drawn
invariant test; NIST's malformed G6/nonconformance case stays refused. Four obsolete
known_pmi_dimensions outcome entries are replaced by four exact differences that retain
the literal class, with evidence. The comparison and exact-one-outcome tests are unchanged.

## Changed files in this worktree

```text
docs/next-session.md
src/dims/gdt.rs
src/dims/mod.rs
src/dims/pmi_dimensions.rs
src/dims/tolerance.rs
src/model.rs
src/output.rs
src/pmi.rs
src/session.rs
tests/editor_session.rs
tests/fixtures/known_pmi.json
tests/fixtures/known_pmi_coverage.json
tests/fixtures/known_pmi_dimensions.json
tests/fixtures/parts/editor_pmi.step
tests/fixtures/pmi_drawings/known_pmi_drawings.json
tools/quality_gate.py
```

The exhaustive drawing ledger also replaces four stale fit refusals with four exact
class-bearing drawing differences and adds four historical thread record refusals.
Its comparison test is unchanged (332 exact differences after these ports).

## S5 verification and quality

`cargo fmt --check`, `cargo clippy --all-targets --release -- -D warnings`, and
`cargo build --release` pass. All Cargo commands were serialized, with
`CARGO_BUILD_JOBS=8` and `RUST_TEST_THREADS=8`; no memory-guard kill occurred.
The editor checks and isolated WASM latency measurements are recorded in its
`docs/next-session.md` and `results/session-wasm.csv`.

The required release quality pipeline passed:

```sh
C=/Users/paul/repos/quiddity/tests/corpus
sed "s#^#$C/#" tools/quality_parts.txt | tr '\n' '\0' | xargs -0 python3 -I tools/quality.py target/release/draftwright target/quality-s5
python3 -I tools/quality_gate.py docs/status-2026-10-10.md target/quality-s5/baseline.json
```

All 30 parts are identical to the latest status table on every metric. Totals:
off_sheet 0, repeated_label 18, repeated_measure 0, views_overlap 0,
view_on_title_block 0, ink 282. The output is `target/quality-s5/baseline.json`.
A deliberately increased 1000.step ink count fails the new gate.

Before-change regression evidence: the nine-test editor Session suite has 11
missing-API compile errors against the original S4. Six focused probes using only
that original API all fail before and all pass after the ports: geometry choices,
semantic-thread native ownership, literal bore fits, unfilled curves/filled faces,
historical thread fields/support/provenance, and refusal of a face-on datum A.
The nine tracked tests additionally exercise transactional PMI+Ops, fragment
partition/caching, refusal rollback, batched options, stable IDs and contradictory
thread nominal sizes. Existing S3 pin tests and exact PMI comparison invariants are
unchanged. The new suite does not add an adapter for S7. No commit or push was made.

The final required corpus test command passed:
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`.
Result: **605 passed, 0 failed, 0 ignored across 66 result groups**, including all
nine new editor Session tests. The final exhaustive reading, coverage, dimensions,
drawings and labels comparisons all pass with their original invariant assertions.
Both worktrees also pass `git diff --check`, with empty staged diffs.

## D1: go/no-go instruments and MCP gaps (2026-10-11)

Overturnable judgement calls made unattended:

- Random-pin sampling ranks drawable dimension, PMI annotation and non-root view IDs by
  SHA-256 of `seed:part:item`. N selects a prefix, without replacement, so samples are
  independent of Plan ordering and nested across N. A bare annotation `{pinned:true}`
  retains canonical automatic geometry. View intent requires another view, a side and a
  named alignment: retain `front` as the unpinned root (or the first ID if absent), use
  the dominant side from footprint centres, a legal existing gap and the nearest named
  alignment to that root, and exclude the root from sampling
  to avoid cycles. This tests pinning existing placements, rather than random manual
  relocation; it is an overturnable interpretation of section 6. IDs and complete Specs
  are kept with every run. A missing part or insufficient candidates is an instrument
  error, never a smaller silent sample. Zero baseline ink requires zero pinned ink.
- `tools/quality.py` reports a measured no-go with exit 0 so all nine measurements can
  complete; `--require-go` enforces failure on any part, including a zero-ink baseline
  that acquires ink. Totals do not hide individual failures. Off-sheet subjects are
  retained as well as counts, to detect replacement with a different off-sheet item.
- The engine fingerprint includes `src/`, Cargo.lock, Cargo.toml, build scripts, schemas
  and fonts, sorted by relative path with length-delimited contents. Directory watches
  catch new/deleted files. Paths and mtimes do not affect identity. It conservatively
  invalidates caches even for a source comment or schema-only change; this supersedes
  S7's hand-bumped suffix judgement above. It describes the source and locked dependencies,
  not external compiler/environment reproducibility. No production cache was migrated.
- The real-model client targets the existing loopback JSON-response HTTP mode. It logs
  exact UTF-8 JSON-RPC body bytes and S0-compatible argument/text bytes per call, with
  `ceil(bytes/4)` estimates. Actual model tokens must be supplied by the evaluator.
  Judge calls are excluded. The seven task conditions remain unchanged. CLI PNG calls
  print the structured text metadata; `--image-out` saves MCP image content for a vision
  model's image reader. D1 supplies instruments, not a claimed Claude run.

### Diagnosis: plate-callouts-right-pinned remains a no-go

Reproduced with the pre-D1 release binary, the unchanged scripted ops and S0 feature
bridge. The A4 1:1 plan reserves plan-view area `[10,77,150,175]`. Its three right-pin
shoulders end at x=175.793, 163.719 and 174.319 mm (lanes 0,1,2, along start/middle/end).
They exceed the right band by **25.793, 13.719 and 24.319 mm**. The location-Y dimension
`d:f:hole:4/location.length.y` has its dimension line at x=140, y=95..125 and witnesses
at y=95 and y=125. The pattern pin's leader ends at `[140,95]`; hole-4's leader/shoulder
runs along y=125 from x=84 to 163.719. Both intersect the location dimension's ink,
reported as two `annotation_leader_crossing` problems. The pin geometry is retained,
not dropped; all other task conditions pass.

Evidence is reproducible with:

```sh
node tools/mcp-eval/run-scripted.mjs --out target/d1-mcp-eval.json --documents-dir target/d1-eval-documents
tools/mcp-eval/check plate-callouts-right-pinned --spec target/d1-eval-documents/plate-callouts-right-pinned/spec.json --plan target/d1-eval-documents/plate-callouts-right-pinned/plan.json --baseline target/d1-eval-documents/plate-callouts-right-pinned/baseline.json
```

The first instrument and the task checker exit 1. Inspect the emitted Plan's placed
lines and problems; the old-binary evidence is also in `target/d1-diagnosis-documents/`.

Cause: `model::plan_with` computes demand/reservations before resolving explicit pin
ink and restores canonical anchor areas for pins; `dims::placement::intent` uses the
ordinary lane grid plus a full horizontal label shoulder regardless of available band
width. `avoid_fixed` checks label/label and line/label collisions only, and searches
larger lanes on the same side; it has no line/line constraints or alternate-side solve.
The leader solver skips already placed pins. Moving the free location dimension to
another lane on the right does not address every crossing; the witnesses still traverse
that strip. A joint choice of its side and the callouts' reserved width is needed.
Expanding the reported area after placement alone would hide the reservation finding,
change areas used as pin anchors, and leave crossings. Therefore no layout patch was
made: the small-change route cannot establish the unchanged pin-geometry and quality
invariants. Carry forward section 6's constraint-based lane redesign. No task conditions,
collision reporting, pins or quality bars were weakened to obtain a pass.

Final measurements and verification are in `docs/status-2026-10-10.md`'s D1 entry.

The 30 native-recognition quality parts produced no eligible PMI annotation IDs:
across the nine runs, 895 selected pins are dimensions and 185 are views (1,080
selected items in 270 experiments). The instrument supports PMI and tests that
Spec path with a drawable PMI fixture shape; the existing S3 AP242 property tests
also cover real PMI pins. The section 6 corpus measurement therefore establishes
no new PMI-layout distribution. Adding an AP242 quality corpus is a future decision,
not an excuse to replace the prescribed thirty parts here.

One Node live-client test invocation reported `fetch failed` on its first registration
request while the full corpus suite was running. An immediate unchanged-code rerun
passed all three tests, including online checking and PNG export. The underlying
connection failure was not identified; no retries, skips or weaker assertions were
added. Keep this as a possible harness/host-load flake if it recurs.

An initial source-mutation probe reused the working target directory. It was corrected
to use a disposable isolated target. The working build script was forced to rerun,
and the engine-version test verified the compiled identifier against the working
source before the complete 615-test run. Final artifacts and measurements use the
verified working engine; there was no layout code difference in the initial probe.


## D1 review fix round (2026-10-11)

The reported off-sheet plan views were false positives: a view at the 10 mm
margin can resolve to `9.999999999999996`. The reporting comparison now allows
**1e-9 mm** on each drawing-area boundary. This is an overturnable conservative numerical
tolerance: it covers anchor arithmetic rounding while still reporting real
overruns. Packing, resolved geometry, pin intent and collision checks are unchanged.
A unit regression covers exact boundaries, 1e-12 mm rounding and 1e-4 mm real
overruns on all four edges, retaining the geometry and checking problem subjects.
A corpus regression verifies byte-identical unpinned/pinned plan-view geometry
for 1000, 10000 and 1004 and no off-sheet finding. Both failed before this fix.

The fingerprint now includes only Rust `*.rs` source, MCP `*.json` schemas,
`*.ttf`/`*.otf` fonts and the four explicit build/manifest/lock files. Extension
filtering instead of invoking git is an overturnable choice: release source
archives must work without repository metadata. Directory watches remain, so
new/deleted relevant inputs are detected. A regression fails before the fix
when `.DS_Store`, swaps and backup files change the input set; it also checks
that adding/removing Rust source changes/restores the fingerprint.

The existing random selection and bare annotation pins are retained for an
exact comparison with the reviewed grid. This measures random subsets of
existing placements; it does not measure random relocation to option-selected
placements. That narrower interpretation remains overturnable, and the new
report must not be used to claim relocation quality. A second sampling policy
and full measured grid would be a separate instrument change, not a cheap
reporting fix. The observed NO-GO already requires the lane redesign.

The earlier implementation is driver-committed as `9b7ee2b`. The fix-round
agent did not commit, push or write the index; the driver recorded the round
as `e605d3a`. No editor or draftwright-io files
are changed. The existing geometry-backed diagnosis of the plate task stands;
no scripted conditions or quality thresholds are weakened.


The fix-round MCP runner initially failed before reaching any task with
`EHOSTUNREACH 0.0.0.127:80`. Its stderr parser accepted a partial startup URL
(`http://127`) before the rest of the host, port and `/mcp` arrived. The same
pattern was present in the live-client test. Both now use a shared parser that
requires the complete newline-terminated `/mcp` startup line. A Node regression
checks every split point plus CRLF and subsequent logs; it failed against the
old pattern (`http://1` returned from a prefix) and passes after the fix. No
retries or relaxed assertions were added. This explains the reproduced startup
failure; the earlier implementation's unspecified `fetch failed` may have had
the same cause, but that historical attribution is not proven.


The fresh grid disproves the reviewer's provisional GRM-03 diagnosis: its
isometric right edge is exactly 287 mm, but the pin uses `align:start` to the
front view, raising its area from `[137,71,287,200]` to `[137,84,287,213]`. On
the 297×210 mm page, the drawing-area top is 200 mm, so this is a **real 13 mm
overrun**, not rounding at the right edge. The bounds report correctly remains.
Reproduce with `draftwright plan` on the corpus GRM-03 file and the saved
`target/d1-fix-pins-1-3/pinned/GRM-03_thumbwheel_drive_screw.spec.json`; the
comparison plans are `target/d1-fix-grm-{default,pinned}-plan.json`. Named view
alignment is intentionally quantized by the existing sampler. Do not remove a
real finding or alter that sampling policy to improve the measured bar.


Final fix-round measurements: **270/270 completed**, **7/9 combinations fail**,
**8/270 experiments fail**. Twelve plan-view false positives are removed; three
real isometric overruns remain. Six experiments exceed the ink bar, with one
also off-sheet. Selected IDs, complete Specs and every non-off-sheet metric
match the original grid. The corrected full tables and review-count arithmetic
are in `docs/status-2026-10-10.md`; raw comparison is
`target/d1-fix-grid-comparison.json`. Unpinned quality is identical in all 180
cells and in complete baseline JSON. `--require-go` accepts the corrected 1004
case and rejects the real GRM-03 overrun.

Fmt, release Clippy, release build and required-corpus tests pass: **618 tests**,
**68 result groups**, zero failures/ignored. Python tests pass **5/5** and Node
tests pass **4/4**. S7 still scores **6/7** (exit 1), with the documented plate
failure; that all-tasks gate is not passed. No conditions or tests were weakened.
Logs are in `target/d1-fix-logs/`.
## Fit limits (maintainer's decision, 2026-10-11)

- An ISO 286 fit prints its class and its limits from draftwright's table (`⌀10 H7 (+0.015/0)`),
  for fits authored in the Spec and for PMI hole classes stated without limits. This settles the
  open question on PMI classes without limits for bores.
- `sheet/fit_limits: false` prints the class alone (draftwright's default form); the parity
  tests in `tests/tolerances.rs` use it.
- Still refused: PMI shaft classes without limits on bosses (`g6` on the spool and pin), which
  are not lowered for other reasons (`symbolic_bore` is bores only). Their limits are now
  computable from the same table if they are lowered later.

## One span, one dimension for PMI (maintainer's request, 2026-10-11)

- `share_pmi` (src/dims/mod.rs): PMI linear dimensions across the same span with the same label
  are drawn once (a model that states each grid hole's basic location states a row's or a
  column's span once per hole), and a default dimension with nothing from the spec gives way to
  a PMI dimension of the same span and value. Each one that gives way is omitted, covered by
  the survivor, which claims its measurement. 15 records per PMI oracle are verdicted
  rust-correct (draftwright draws them all). Judgement call: a default dimension the spec has
  toleranced keeps its own line beside the PMI one.


## E1: layout around pins (2026-10-11)

Reproduced the Stage D failures from freshly generated plans; the named saved report
folders were absent in the main checkout. Before evidence is under
`target/e1-before-pins/`, `target/e1-before-documents/` and
`target/e1-logs/regressions-before.log`. No commit, push or git-index write was made.

- GRM-03 and cgb245: restoring only the pinned labels while starting free linear
  labels at their unsolved centres reintroduced collisions already removed by the
  canonical slides. `avoid_fixed` then moved short lengths/rungs to outer lanes.
  Start free linear items from their settled canonical candidate, relabel current
  text, and adjust around obstacles only where needed. Recompute held translations
  when a free view moves in the second packing pass.
- GRM-03: the isometric's 13 mm overrun was real. Resolve its relative region, clip
  it to the drawable sheet and fit its ink before fixing the resulting geometry.
  sm-hanger had a different manifestation of the same missing fitting step: its
  requested region covered the side view and pinned ink. Choose a free subrectangle
  inside the region. Use canonical principal footprints even if an unrelated edit
  omits a view, so that edit cannot change the pin's resolved position.
- Plate: side callout lanes used the 7 mm linear pitch for labels up to 32.793 mm
  wide. Use canonical measured width plus shoulder and clearance for separate
  columns; explicit pin ink reserves real carrier bands before packing free views.
  Free linear dimensions also avoid intersections with pinned callout shafts and
  shoulders, trying slides, lanes on the same side, then the opposite side. The
  original location-Y dimension could not clear the leaders by moving outward:
  its witnesses still crossed them; an opposite-side candidate resolves it.
  Attach the pinned leader radially to the nearest proven site, using the existing
  automatic leader geometry. This also prevents the extra rim/arrow crossing
  caused by attaching on the cardinal right edge while the shoulder is above it.

Overturnable judgements made unattended:

- A pinned isometric fixes a fitted orientation view in the requested relative
  region. Gap/alignment describe that initial region; clipping or excluding another
  footprint may trim the final reserved rectangle. Principal view pins still resolve
  literal gap/alignment and retain/report impossible placements. This avoids moving
  or shrinking a pin during subsequent automatic packing. It is a deliberate
  extension of the existing automatic isometric policy, documented in `docs/model.md`
  and `tests/fixtures/known_pin_layout.json`, with Python source evidence.
- Explicit side callout columns use the widest canonical callout on their carrier,
  independent of unrelated current text/omission edits. A pin's own longer text can
  still require more room and is visibly reported when it cannot fit. Identical
  along positions get disjoint label columns; conflicting requested leader paths
  remain reported. The scripted task's requested start/middle/end paths are clear.
- Keep the prescribed SHA-256 selection, named view alignment and bare annotation
  sampling unchanged. The grid establishes pinning-in-place quality on the same
  thirty native parts, not arbitrary manual relocation or a PMI pin distribution.
  The existing canonical S3 PMI/property tests remain required.

No constraint-based lane redesign, corpus/recognition change, problem filtering,
threshold change or weakening of existing tests was used. Three new regressions
failed on the original source, covering settled linear candidates (GRM-03/cgb245),
relative isometric fitting (GRM-03/sm-hanger), and pinned callout columns plus the
MCP task. Final measured tables and verification are appended to the status report.


Additional manual-station diagnosis: `target/e1-plate-same-station.{spec,plan}.json`
sets all three callouts to along=middle. Their labels now occupy disjoint columns
`[143,175.793]`, `[185.793,199.512]`, `[228.586,245.905]` (x bounds in mm),
so the reported real-model label-overlap cause is fixed. Their forced horizontal
shoulders and straight shafts share y=125, however, producing six Policy B
`annotation_leader_crossing` findings (both directions for each pair). These are
kept visible; that manually conflicting Spec does not pass the task's no-problems
condition. Avoiding such paths while keeping the three fixed label stations needs
joint bent-leader routing, beyond the bounded linear lane/side fallback in this
stream. No such routing redesign was attempted. The unchanged scripted task uses
start/middle/end and passes; a new real-model evaluation is not claimed here.


Final E1 verification: fmt, release Clippy and release build passed. The required-corpus
full release suite passed **623 tests / 68 result groups**, zero failed/ignored,
including the existing S3 pin invariance test. Unpinned quality: all 30 parts and
180 cells identical to the prior status table; complete baseline JSON identical
(off-sheet 0, repeated labels 18, repeated measures 0, view overlaps/title-block
overlaps 0, ink 282). The complete unchanged random-pin grid with `--require-go`
passes **270/270 experiments / 9/9 combinations**, zero off-sheet items. Pinned
ink totals by seed/N: 1/1=273, 1/3=265, 1/8=265, 2/1=280, 2/3=236, 2/8=241, 3/1=282, 3/3=271, 3/8=246.
Scripted MCP: **7/7**, 16 calls, 4,292 estimated tool tokens, no refusals; Python
instrument tests 5/5 and Node client/checker tests 4/4. No real-model rerun is claimed.
Reports are `target/e1-quality/`, `target/e1-pins-<seed>-<N>/`,
`target/e1-mcp-eval.json`, `target/e1-eval-documents/` and `target/e1-logs/`.
Full tables are in `docs/status-2026-10-10.md`. The agent did not commit or push.

## E2 MCP call efficiency (2026-10-11)

Overturnable judgements made unattended:

- Keep twelve tools and the stateless Spec/version contract. Extend `inspect_part` with an
  optional `problem_filter`, returning the same bounded `problem_page` as `list_problems`.
  Registration plus a feature's corrective answer is one question, so it can share a call.
  Default problem rows include fixes; `detail` still adds the human explanation.
- Feature problem filtering uses the Session's typed item/feature associations and direct
  feature subjects, rather than a string prefix or a guessed spatial radius. It combines with
  existing code/severity/subject filters. Paging and lossless artifacts remain available.
- Summary `key_items` prioritises envelope dimensions, then diameters, then a representative
  dimension per feature, with stable ID ordering and a 700-byte cap. It is a useful index,
  not an exhaustive list; `list_items` remains the complete, paged query. This policy can be
  overturned if model evaluations demonstrate another general ordering is more useful.
- `Options.settable` is shared by Session and MCP: escaped paths, choices, ranges, tolerance
  forms, sheet title keys, bare view names, and fits from the existing validation table at the
  recognised nominal diameter. A grouped fit descriptor's `paths` shares its choices across
  every listed feature path (a consolidated mark still needs a fit on each member). Place
  fields are candidates, with view/side compatibility in `placements`; preview/apply still
  resolves collisions. A view relation needs `relative_to` and `side` together.
- Refusals retain the invalid request and append an explicit `Corrected op: {JSON}` for
  unambiguous translations: a supported fit misfiled on its diameter dimension, a bare view
  key, a title cell's canonical Spec key, or an explicit one-field `{symmetric: magnitude}`
  tolerance and the existing ASCII tolerance spellings. The caller must review/retry it.
  Never infer a tolerance magnitude, choose a member of a pattern, or alter stale-head choices.
- Fix round supersedes the GRM batching judgement: preserve its two sequential commits,
  so a later sheet/scale change must retain the earlier tolerance/omission. H7 is a task
  input, just as in the original scripted agent; the script no longer adds a fit-options
  discovery call when measuring against that baseline. Rust tests still cover valid fits.
  `--separate-problems` is a paired ablation: the same script/binary/fixes make one extra
  slot query, isolating the saving from `inspect_part.problem_filter`. This is evidence of
  the tool's call saving, not a prediction of a real model's choices.
- No placement/layout algorithms or drafting outputs were changed. The existing plate pin
  task's failure cannot be fixed within E2: E1 owns placement. Retain its strict failure and
  report the full scripted go/no-go separately from E2's fewer-calls/preserved-success gate.
- Schema factoring uses short local definition names and omits empty `required` lists, with
  identical JSON Schema semantics, to keep the existing 39,500-byte session limit. No schema
  budget, test condition, or task success condition was relaxed.

The read-only Python `origin/main:src/draftwright/fits.py` confirms holes H/G/F, shafts
h/g/f/js/k/n/p, grades 5–11 and sizes over zero through 250 mm. Options enumerate the Rust
validator's same table; there is no new drafting difference requiring a known-differences
ledger entry. Full parity and quality verification are recorded in the E2 status entry.

E2 verification note: copying a before checkout into the current Cargo target initially
reused newer library artifacts because of restored source timestamps. That probe is invalid
and is superseded by the package-clean rebuild. In the clean original build all seven new
tests and the description guard fail for the intended missing capabilities, while its engine
fingerprint test passes. Final checks rebuild the actual worktree. No baseline claims rely on
reused artifacts.

When inspection requests problems, that answer takes priority over the summary's optional
key-item index: shorten `key_items` first to fit the complete MCP envelope, then page problems
if necessary. The original paging regression correctly caught truncating a two-fix answer
while keeping all index hints; its assertions remain unchanged. Broad queries still follow
`problem_page.next_cursor` through `list_problems` with the same filter.

E2 final verification: required-corpus release suite **627/627**, 69 result groups, no
failures/ignored; fmt, release Clippy and build pass. Node tests **5/5**, Python instrument
tests **5/5**, all twelve tool schemas/refusals and 24 served/file documents pass independent
validation. Required quality: **30/30 parts**, all **180 cells identical**, no part worse.
Scripted calls **16 → 15**, all six previous successes retained; its strict all-task runner
still exits 1 for the unchanged plate layout failure. E1 must resolve that condition before
claiming the global seven-task gate. No real-model evaluation, commits or pushes were made.
Per-task counts, byte/token measurements and artifact paths are in the E2 status section.

## E2 review fix round (2026-10-11)

Overturnable judgements made unattended:

- Offer one correction for the reported batch failure, naming its zero-based op index.
  Match the isolated Session refusal to the actual error and preview the proposed replacement
  through Session before offering it. Do not attach a correction for a different mistake.
  Multiple bad ops may still need multiple retries; repairing unrelated ops or inventing missing
  engineering intent would overstate what this reply validates. Supported translations remain
  the five documented unambiguous patterns; other refusals retain their valid choices.
- The plate instrument now requires all three task callouts explicitly. A shortened index fails
  the instrument instead of silently changing its input. No item-index policy or layout code
  was tuned for this task.
- Nullable type-only schemas use JSON Schema's equivalent `type: [T, "null"]` form. The local
  validator checks these unions strictly, including negative cases. Regeneration saves 1,190 B,
  giving 1,206 B of headroom inside the existing 39,500 B budget. Keep the short local definition
  names; no consumer depends on them and `$defs/op` remains stable.
- The strict seven-task evaluation must keep failing for the existing plate layout problem until
  E1 fixes it. E2 cannot satisfy that global gate within the explicit prohibition on placement
  changes. Do not label the full required check suite passed on the strength of six successes.

Fix-round verification: **629/629** required-corpus release tests, fmt, strict release
Clippy and release build pass; final MCP regressions **9/9**, Node **7/7**, Python **5/5**,
and independent schemas **24/24** pass. Quality **30/30**, all **180 cells identical**.
Scripted calls **16→15**, with GRM-03 **3→3** and H7 **2→2**; only CTC-01 saves a call.
The same-binary separate-query ablation reproduces **16→15** with **21/21 identical judge
JSON documents**. The strict seven-task gate remains **6/7, exit 1** for the untouched
plate layout findings, so `tests_passed` remains false. The corrected measurements and
artifact paths are in the final E2 fix-round entry of `docs/status-2026-10-10.md`.

## E3: repeated spans across the corpus (2026-10-11)

The survey adds `repeated_span` and `repeated_span_groups` to `tools/quality.py`.
It measures placed linear arrow-tip stations along their axis, divided by sheet
scale, ignoring the dimension lane and label. Endpoint and nominal-value identity
use 1e-6 mm; unlike the old `repeated_drawn`, no rounding bucket or witness-line
position decides identity. Equal values on the same PMI face pair in the same
part and view, through different stations, are reported as `feature_pair` near
misses. This is conservative evidence: equal labels alone prove no shared
features. Callouts, angles, unplaced dimensions and different axes/views are
excluded. The eight instrument regressions all fail against the original tool.

The 42 corpus parts in `tools/repeated_span_parts.txt` include all thirty quality
parts, literal test inputs and captured sheet bindings. The survey script checks
that those test inputs remain covered and unpacks all 22 AP242 fixtures. Run:

```sh
DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus python3 -I tools/survey_repeated_spans.py target/release/draftwright "$PWD/target/e3-survey"
```

The before/after tables and verification are in `docs/status-2026-10-10.md`.
Each of the eight before patterns has a rust-correct verdict with numerical and
Python evidence in `tests/fixtures/known_repeated_spans.json`. Optional
`--before <baseline.json>` checks that this historical ledger exactly describes
the before groups. The script fails on any remaining repeated span or failed
part; `quality.py --require-no-repeated-span` provides the same zero gate for
an arbitrary measured set. Plans and SVGs are saved beside the baseline.

Overturnable judgement calls made unattended:

- A plain hole location and a face-level datum distance on identical stations
  are one statement. The listed item wins over a default, then the smaller ID.
  It claims both measurements; omission promotes the surviving independent
  statement. This settles the earlier FTC-10 repeat question within this stream's
  authorized scope. Different bands, basic status or values stay separate.
- A counted boss height on the face level it states conveys the rung too.
  Only boss-height claims enter the count: FTC-06 stays `4× 38.1`, with five
  claims including the level. A level is not a fifth boss. Other families'
  coincident sizes still keep their own statements under the existing tests.
- A turned boss whose linear diameter equals the overall diameter across
  identical radial stations gives way to that extent. Separate axial lengths,
  smaller diameters and different tolerance bands remain meaningful statements.
- PMI/default value equality uses the same 1e-6 mm tolerance as their endpoints.
  AP242 304.79999999999995 mm and geometry 304.8 mm are one nominal size;
  98.816351 versus 98.81635164398 mm is also within that identity tolerance.
  The PMI requirement, including its basic box or inch units/band, wins over
  an untouched default. No Spec-authored tolerance is withdrawn.

- Near-miss feature identity requires a proven two-face PMI pair. Ordinary
  measurement claims identify the quantity's owner, not both witness features;
  treating two equal claims or labels as an endpoint pair would flag distinct
  chain stations without evidence. The metric records exact station repeats for
  those dimensions and uses the AP242 pair for shifted stations. If the Plan later
  exposes witness-feature identities, extend this check to those identities.

New cross-family merges retain their original lane reservations, so the consolidation
removes ink without moving unrelated items. This preserves edit locality and the
quality baseline; the old same-family sharing still determines lane ownership.

No pin/lane-placement or MCP implementation file is changed. No commit or push
was made; the driver owns version control.

Final checks: fmt, release clippy, required-corpus release tests (**626 passed,
68 groups, 0 failed/ignored**) and release build passed. Python instrument tests
13/13; the focused Rust run 32/32. The post-build thirty-part quality run passes
all 180 cells against the prior status table: ink 282 unchanged, labels 18→17,
repeated spans 2→0. All sixty-four survey rows have no worsened metric or ink
problem code; repeated spans 8→0, ink 2007→1978, labels 47→41. Both before
rejection and after acceptance are checked by the CLI zero gate.

Scripted MCP remains **6/7** (exit 1), 16 calls / 4,298 estimated tool tokens,
plus 10,905 fixed-context estimated tokens. `plate-callouts-right-pinned` has
the same three band overruns (25.8/13.7/24.3 mm) and two leader crossings recorded
in Stage D. The all-tasks gate is unmet; fixing the existing pin/lane placement
is E1's scope, excluded from E3. No task/checker condition was weakened.
Logs, plans and SVGs are under `target/e3-*`; the full table and verdict evidence
are recorded in the status document and known-differences ledgers.


## Stage F / S1: leaders through features (2026-10-11)

Overturnable judgements made unattended:

- Analysis-only planning remains the ADR R1 boundary. End-on hole footprints are exact
  circles (the largest bore/recess diameter); side-on holes/bores and slots use projected
  bounds. Double-D bores use their major diameter; supported section-recess pockets use
  their published bounds. Principal-face inner wires supply unrecognised openings. Arcs
  are sampled every pi/64 radians, with a 0.05 mm paper tolerance for a crossing or tip
  attachment. This conservative approximation can over-report grazing a rounded slot or
  bore corner; replace it with exact footprints if better Analysis evidence becomes available.
- Principal profiles retain inner wires and outer arcs, but no complete outer wire. Solid
  bounds measure an exit and detect re-entry. An unrecognised opening uses the carrying
  view ID as the crossed outline subject, rather than inventing an Analysis feature ID.
  This does not establish complete concave-silhouette crossing coverage. The existing
  geometry contract was not changed, and section/render line-work belongs to S2.
- A feature owner's first contiguous passage near the tip is exempt, not every intersection
  with that feature. Grouped member measurements and explicit hole/slot pattern membership identify their own openings; another
  passage farther along the shaft is still reported. A crossed feature is counted once per
  leader; extension lines are counted separately, with distinct problem IDs per line.
  A dimension witnesses its complete feature/pattern, so all its owned members are exempt
  for extensions; leaders exempt only the initial passage of the selected attachment.
  The quality metric counts each leader/item-feature pair and each extension line once,
  even if that line crosses several features; individual warnings still name every crossed
  feature. The problem ID retains the extension index so the instrument can deduplicate.
  Extensions report other recognised features, not outer-outline re-entry.
- Explicit callout strip intent keeps side and lane. If its prescribed station crosses a
  footprint, resolve to the nearest station with fewer crossings, using Analysis alone;
  equal shifts choose the lower station deterministically. Shoulder and label translate
  together. This deliberately lets a pin's station move during initial resolution, as the
  stream authorizes; subsequent edits to unrelated annotations do not move the pin. Bare
  pins still keep canonical automatic ink. The plate's middle station moves 5 mm down.
- Joint inventory scoring counts feature crossings and reserved-area overruns with other
  crossings, then text and length (including initial travel inside solid bounds). Retain
  the original inventory assignment if the same collision predicates find more existing
  annotation defects. Also preview the dependent PMI placement and retain the original
  assignment if any PMI annotation problem kind grows. This preserves the existing GD&T
  bands without changing their solver. This general floor prevents the bounded candidate inventory from
  trading improved feature clearance for worse text/ink; it can retain a feature crossing
  that a routing redesign might avoid. There are no part/feature-ID branches or thresholds.
- Extension placement is unchanged. Warnings offer the opposite strip at lane zero as a
  reviewable Spec edit, never apply it automatically. Re-plan the suggestion: it is a
  candidate, not a promise of globally clear placement. The scripted slot task applies it
  and retains every dimension. The residual 10020 notch route crosses its hole-location
  extension, reports it, and has the same one existing ink defect as before.

Evidence and the deliberate difference from Python's actual projected-material predicate
are in `tests/fixtures/known_line_through_feature.json`. Problem codes are strings in MCP
schemas, so no schema regeneration is necessary. The original thirty-part base table and
an instrumented, geometry-identical before table are in the S1 status entry; an old table's
missing metric is not presented as measured zero. No commits or pushes are made.

Final S1 verification: fmt and release clippy with warnings denied pass; required-corpus
release tests **648/648 in 70 groups**, 0 failed/ignored, and release build pass. Python
instrument tests **15/15**. Thirty-part quality: no part worse across 240 cells; ink
282→278, leader/feature pairs 111→66, extension lines 203→203, combined column
314→269. The reporting-only before build has identical base dimension geometry.
Random-pin grid with `--require-go`: **270/270, 9/9 GO**. Scripted MCP: **7/7**,
15 calls. All thirty final sheets were rasterised at 2400 pixels; all eleven changed
sheets and the pinned plate were inspected. CGB203’s unrecognised blade crossings and
dense extension ladders remain; the metric is not complete concave-material coverage.
Full before/after tables, red-test evidence and artifact paths are in the S1 status entry.
No commit or push was made.

## Stage F / S1 review fix (2026-10-11)

This supersedes the first S1 entry's fallback and extension-fix decisions. Footprints
are now cached per resolved frame within `model::plan_with`, shared by dimensions,
leader placement, GD&T collision reporting and finish collision reporting. Repacking
changes the key. Candidate geometry is measured once; two bounded rankings share it,
and their annotation-cost floor uses candidate costs and pair contacts without
building dimensions or running collision/PMI planning as probes. Initial exits from
*any* containing feature are exempt; later entry into that footprint still counts.
Extension warnings are report-only and have no fix ops. The scripted slot editor reads
the offending dimension and deliberately moves it to the opposite strip; the planner
never applies or offers that edit. The seven task conditions are unchanged.

Overturnable judgement calls and remaining limitations:

- Semantic PMI's strips depend on the dimensions. Keep the original leader objective
  on parts with drawable semantic PMI rather than probing two complete PMI placements.
  The failed experiment without this safeguard made STC-09 e4 report an additional
  frame outside its band (3 instead of 2) and a frame leader across another view;
  `tests/gdt_band.rs` caught both. The conservative safeguard passes those same tests.
  Footprint warnings are still reported. A future joint dimension/PMI solve may remove
  this restriction. Presentation-only PMI does not disable feature-aware routing.
- Retain the existing aware-mode area/exit objective for this fix round. Reverting its
  two score changes together restores 10020's bad notch route (14 actual boundary
  crossings instead of 5) and worsens ink on six quality parts (see
  `target/f1-review-trial/` and `/tmp/f1-review-focused.log`). The new floor is cheap and
  explicit; it still favours fewer annotation defects before accepting a feature gain.
  This objective remains a deliberate behavioural difference, rather than claiming
  that the review's observation of broader scoring was wrong.
- `along` on an explicit callout is the preferred station, subordinate to fewer feature
  crossings; side and lane remain prescribed. The plate middle station may move 5 mm.
  Candidate alternatives now check the other prescribed pins' labels and lines; free
  annotations subsequently yield in the existing solver. It is not a joint pin solve.
- The metric remains limited to Analysis feature footprints and solid-box re-entry.
  It does not measure complete concave material. CGB241's arm and 10000's through-body
  extensions can still score zero. Adding complete outer wires needs an Analysis change,
  outside this stream's geometry restrictions. FTC-09 hole 18 and CTC-05 hole 17 / flat 2
  still have reported crossings; clearing them without extra annotation defects requires
  a larger routing change. These minor review findings are accepted as limitations.
- The metric correction lowers warning totals without necessarily moving any ink. Do
  not interpret the whole before/after reduction as drafting improvement. Only four
  quality sheets change geometry in this round; extension placement is unchanged.

The new containing-loop unit regression and FTC-08 drawing/report-only regression both
fail against an archive of the actual pre-review HEAD, not an ablation. Red logs:
`/tmp/f1-review-red-unit.log`, `/tmp/f1-review-red-drawing.log`. The cache test checks
reuse and invalidation after repacking. `tools/check_leader_planning_time.py` provides
an explicit serial before/after release-time gate (aggregate median ratio <= 0.8),
so performance is no longer inferred from quality/MCP success alone.

Final review-fix verification: fmt, release clippy with warnings denied, required-corpus
release tests **651/651 in 70 groups**, and release build pass. Quality **30 parts /
240 cells no worse**, ink **278**, line-through-feature **269→212**; extensions are
unchanged geometrically. Random pins **270/270, 9/9 GO**, zero off-sheet. Scripted MCP
**7/7**, **16 calls** (one explicit item read for the report-only slot extension).
Python tests **15/15**, Node tests **8/8**. Inspected all four changed sheets and the
pinned plate at 2400 pixels, plus the notch and HEX release sheets.

Measured release planning medians on the three reviewed STEP files: CTC-04
**10.74→3.06 s**, FTC-08 **3.45→1.76 s**, CTC-02 **9.49→6.30 s**; aggregate **53.0%
less time**. The timing gate passes at ratio **0.470** and fails at **1.230** when the
reviewed binary is compared with itself. This establishes recovery from the reviewed
slow build, not parity with the pre-feature base. A further profiling target is the
bounded candidate rankings/assignments; no full collision/PMI fallback remains.

Final table, timing and pin rows, commands and artifact locations are in the S1 review
fix entry of `docs/status-2026-10-10.md`. No commit or push was made.

## Stage F S2: line precedence and section linework (2026-10-11)

Overturnable judgement calls made unattended:

- Interpret “within 0.15 mm of visible ink” as distance to the visible stroke,
  including its half-width; with the existing 0.5 mm part stroke the centreline
  threshold is 0.4 mm. Subtraction adds 0.002 mm for the two 0.001 mm chord errors.
  The SVG instrument uses 0.002 mm chord error and integrates capsule intervals;
  it measures underlying hidden paths, not dash pixels, and includes isometrics.
- Deduplicate hidden supports within 0.004 mm on the page, enough for two chord
  errors. This is smaller than any printed stroke. Merge touching collinear
  fragments to retain one dash run; preserve exact curved fragments and stable
  source order for equal lengths. Reconsider this tolerance if very fine detail
  views supply a reason to retain distinguishable supports within it.
- Hatch only material on both sides of the plane. Probe 0.01 model mm on each
  side (retaining the previous kept-side offset), split every candidate at the
  solid-face intersections, and classify each resulting span. This conservative
  treatment excludes untouched coplanar faces. A much thinner than 0.02 mm wall
  merits revisiting the probe offset; no part-specific hatch rule was used.
- Correct the review catalogue's string_post bore orientation: the ø1.6 depth-4
  bore is on the bottom, z=0..4; the upper opening is the transverse ø1.7 bore at
  z=11.05. STEP recognition and point-in-solid evidence support this; the base
  hatch already avoids the axial bore. The regression also requires its walls
  visible and its section free of hidden items.

Open question: should isometric hidden lines be omitted entirely? This stream
leaves their original pieces and dash policy intact, as requested. The quality
column includes their residual proximity to visible ink; the status entry gives
both totals and per-view evidence so this separate decision is visible.

No new model/editor problem kind is introduced: line precedence, hidden section
suppression and correct material hatching are renderer responsibilities. The
cleaned geometry lives in Projections' placed cache; warm renders still perform
zero HLR and bounds passes. The original render_caches expectations stay intact.


S2 follow-up judgement: normalize non-isometric projected conics with minor radius
≤1e-6 page mm to one straight support spanning all major-axis extrema. At that
size SVG's endpoint precision can invent return arcs; the actual deviation is
≤2e-6 mm, far below stroke width. Do this before cached subtraction, keeping
ordinary arcs exact and isometric geometry untouched. The CTC-02 all-view gate
caught three such numerical artifacts and now rejects them.

S2 draftsman/model verdicts: FTC-10's right obround/rectangular islands are cut
material; their coplanar surroundings are not. Samples at x=35/45, y=23±0.01,
z=22/-22 classify In on both sides within the islands, while x=25,z=22 and
x=45,z=15 are one-sided. FTC-06's four circles are external cylindrical boss
cross-sections (x=±76.2,z=-158.75 and x=±114.3,z=-107.95), with material at
all four centres on y=50.8±0.01. Their smaller coaxial bores lie farther back.
Do not “fix” those hatch regions by visual analogy with pockets/bores: the review
catalogue marked these as unproven, and the model disproves that interpretation.
See the S2 status entry and `target/s2-material-verdict.txt` for evidence.


S2 final checks pass: fmt, strict release clippy, required-corpus release tests
**648/648 (70 groups, no ignored tests)**, release build, Python instruments
**18/18**, unchanged render caches **12/12**, all thirty quality parts / 270 cells
no worse, nine `--require-go` pin runs **270/270 GO**, scripted MCP **7/7**.
The first full run exposed a 273 ms NIST session edit; computing hidden-curve
lengths once before sorting fixes it (isolated maximum 85.251 ms; unchanged
150 ms gate passes in the final full run). Every final-build SVG is byte-identical
to the thirty reviewed rasters' SVGs. Before/after tables, model verdicts, failure
proof and final artifact locations are in the S2 status entry; deliberate Python
reference differences have `rust-correct` verdicts in the S2 status entry
(the unused known_linework.json ledger was removed in the fix round).
No commit or push was made.

## Stage F S2 fix round: dash continuity (2026-10-11)

Overturnable unattended judgement: join raw hidden curves at endpoints within
1e-7 page mm, before visible subtraction and duplicate trimming. Normalize
collinear linear Bézier pieces (deviation <1e-8 mm) to their exact union
between extrema, including backtracking and closed edge-on polylines. Straight
joins collapse to a single line only within 1e-6 mm of the same support.
Within the established 0.004 mm duplicate tolerance, retain both source lines
and an explicit connector in one Bézier path rather than flattening their small
offset (the SVG gate found two CTC-02 joins differing by 2–4e-6 mm). Curved
joins require matching
endpoint tangent direction within 0.001 radians, and conics additionally require
identical ellipse support. Preserve spline poles, reversing their order where
needed. Endpoint indexing and source-order candidates make joins deterministic.
The final straight merge accepts at most 2×0.004 mm. A final endpoint pass
allows 2×(0.004+0.001) = 0.010 mm, recovering capsule trims and chord
error on both sides; spline runs insert an explicit straight connector across
that gap so their original poles and stretches stay exact. Conics join only
on identical ellipse support. Visible precedence cuts are much wider, so remain
separated. The thirty-part SVG gate exposed backtracking straight polylines
and trim gaps beyond exactly 2×0.004 mm; neither is silently waived.

The reviewer correctly identified an unresolved solid-grey sub-defect. The
line-only merge also missed straight Bézier fragments emitted by HLR. The new
unit chains and 10063 corpus regression fail on the prior implementation;
`tools/check_hidden_runs.py` independently rejects its rendered SVGs. This check
is separate from the original two quality columns, which cannot detect dash
phase resets. Isometrics remain untouched. The unused known_linework.json
ledger is removed: no capture differs, and the ISO/material verdicts now live
in the S2 status entry. Keep the existing header-driven quality checker version
when merging S1; this fix round does not modify that shared file or its columns.
Closed curves are excluded from endpoint joining, and a run stops when it closes;
this prevents coincident circles from joining into a twice-traced loop. The
curved-chain unit also checks duplicate full ellipses.

Final fix-round checks pass on the finished source: fmt, strict release clippy,
required-corpus release tests **651/651 (70 groups, none ignored)**, release
build, **30 parts / 270 quality cells no worse**, both rendered linework gates
(**zero touching short-path pairs**), random-pin grid **270/270 GO**, scripted
MCP **7/7**, Python **18/18**, Node **7/7**. Render caches remain **12/12** and
sessions **8/8**, including the 150 ms NIST gate. The final repeated quality SVGs
are byte-identical to all thirty reviewed raster inputs; pin/MCP runs used the
exact final executable. Evidence and SHA are in the S2 fix-round status entry
and `target/s2-fix-*`. No commit or push was made.

## Stage F integration judgement calls (2026-10-11, overturnable)

- `tools/check_quality_table.py` keeps S1's header-driven checker; a quality table
  header is now any `part` row containing `ink` or `off_sheet` (S1 keyed on `ink`,
  S2's test table has no `ink`). Columns absent from the baseline are skipped.
- S1's mocked crossing test in `tools/test_quality.py` writes an empty `part.svg`
  so S2's SVG metric can read it; the assertion is unchanged.

## Rules catalogue brought up to date from the rules assessment (2026-10-11)

`docs/rules/{adr,issues,lint}.yaml` now record the assessment's verdicts on the 353 rules it rated:
82 pending rules became ported (each with a `rust_test` checked to exist), 52 retired and 3
translated; 15 of the rated rules the assessment called done or obsolete stay pending, each with a
`rust_note` saying why. `docs/rules/README.md` has the new status counts.

Overturnable judgement calls made unattended:
- **Retired rules carry the reason in `why:`**, the field the catalogue README defines for a
  retired rule, not in `port_note`; the extractor's `port_note` is kept as it was. Translated
  rules carry `rust_rule:` (and a `rust_test:`) in the same way.
- **Done in substance but no test to name: left pending.** ADR2-U1 (fixed text height), ADR5-U1
  (a documented limitation), ARCH0009-5 (lane order: sizes nearest, overall outermost),
  ISSUE-1155-a (GRM-04, no recognition fixture here), ISSUE-1215-f (an omitted callout has no
  hole-table row), LINT-hole_coverage-thread_and_profile and LINT-code-gear_semantics_missing.
  Each needs only a small test to close.
- **Done in substance but a real remainder: left pending.** LINT-code-feature_not_dimensioned
  (boss-blend exclusion, grm04_drive_plate; goes with ISSUE-1450-a), LINT-code-boss_height_missing
  (consolidated height reported missing on GRM-04; goes with ISSUE-1154-a),
  LINT-oriented_slot_coverage-width_length (pattern membership, #1430) and
  LINT-code-pmi_not_rendered (the recorded double-reporting decision).
- **The 16 "features nobody has asked for"** in the assessment's drop list are retired like the
  machinery ones, each with its reason; they can be reopened as new rules if a defect calls for
  them.
- **The 20 `*_requirement_state` lint rules** are ported with a note that `_suppressed` is the
  Plan's authored omission, which lint by design does not read (R1).
- The 20 pending rules the assessment did not rate were left as they were.


## Stage G1: honest completeness and muted lint ledgers (2026-10-11)

Overturnable unattended judgement calls:

- Keep the requested `clear` / `needs_attention` JSON vocabulary. Clear is bounded to
  assessed Plan/render problems; independent finished-drawing lint remains in review.
  Info-only multi-solid coverage does not change that status. This is not manufacturing
  readiness and does not certify recognition recall.
- Face claims are the union of Analysis feature faces, not all source recogniser
  candidates or evidence records. This makes the bound reproducible even for old Analysis
  documents. Stock/background faces are counted and may be unclaimed: warn for review,
  never call each face a missed feature. Unknown solid count in an older Analysis warns;
  new analyses record the actual solid count without a schema bump (optional field).
- Unplanned features are whole recognised features without drawn measurement evidence,
  including fully authored omissions. A partly dimensioned feature is not counted again
  per missing parameter; independent lint assesses the parameters. Preserve the existing
  undrafted-record exclusions and proven pattern/groove absorption. This count is not an
  audited physical-requirement score. Four quality components/scoring and an independent
  expected-feature denominator remain outside G1's authorised scope.
- Extend Python's envelope/turned completeness ledgers into lint findings as explicitly
  requested. Use `missing_principal_dimension` for overall axes, `axial_length_missing`
  for missing band lengths, and typed missing/unverifiable codes for band diameters.
  Multi-solid findings are info; single-solid missing/unverifiable requirements warn.
- Give patterns their own `hole_pattern_requirement_*` code family: subjects are pattern
  owners, requirements name exact parameters, related IDs list member holes. They are not
  machining-size groups. A fully proven axis-aligned grid uses its physical centre;
  other arrays need a physical member anchor. Zero datum distances need no ordinate.
  Printed text must state the value; a claim alone is insufficient. Off-axis patterns are not assessed by this additional ledger; a principal-axis
  declared grid whose physical lattice cannot be proved has unverifiable spacing.
- Keep negative rounded zero as `0.0`, preserving the existing noninteger precision and
  matching the reference's explicit-decimal suppression of a minus sign, while exact zero still prints `0`.

No planners gain dimensions, no rule YAML changes, no task conditions change, no commit
or push. Findings and table completeness expose work owed to G2/G3 rather than masking it.


G1 assessment correction: GRM-03's default sheet proves its 23.5 mm axial
extent with a contiguous 0.5 + 2 + 3 + 18 mm band chain. It does not owe a second
label under the existing ledger. The controlled omission script breaks the chain
and then correctly reports `missing_principal_dimension`; this judgement is
recorded in `tests/fixtures/known_completeness.json` with visual evidence.

Historical tests keep their original geometry oracles. The migration plate gains
a separate literal reporting fixture; annotation-clearing lint tests additionally
expect the envelope and pattern statements they explicitly removed. Default
feature-free stock now expects its exact unclaimed warning rather than silence.
Slot-array measurement ownership credits its physical member slots, preventing
false whole-feature gaps. The old fixture geometry is unchanged.

Spool132 (captured corpus recognition) newly reports six numerical centre-location
requirements for its three bolt circles. By eye it looks concentric with the
turned stock and its PCDs are stated. No certified pattern-centreline carrier is
available in the mark inventory; the typed missing-location findings warn, while
the draftsman verdict is **undetermined**, not six independent machining defects.
This is recorded in known_lint.json / known_completeness.json `not_done`. A proved
concentricity/centreline carrier is follow-up work for G2/G3; do not silence real
pattern omissions to remove this uncertainty.

Final-suite transport transient: one required full run failed the unchanged
`streamable_http_and_stdio_publish_the_same_tools_and_obey_transport_rules` on
`read_to_end` with OS 54 / ConnectionReset; its other seven stateless tests
passed. The isolated transport recheck and an unchanged full stateless group
then passed (1/1 and 8/8). The existing server can reject a header before draining
its declared request body, a plausible timing cause; this is an inference, not
a proved diagnosis. G1 changes no transport code or assertions. Preserve this
case for follow-up; the required full suite is repeated on the same source.


G1 final checks complete on the unchanged source: fmt, strict release clippy,
required corpus release tests **673/673 (72 groups, none ignored)**, release
build, **30 parts / 300 Stage F and 390 extended quality cells no worse**,
random-pin grid **270/270 GO** with `--require-go`, unchanged scripted MCP
**7/7 (16 calls)**, Python **23/23**, Node **8/8**, **12 MCP tools / 24 schemas**.
All nine new Rust and three new Python regressions failed on the original
implementation. Final default/omission SVGs and 2400-pixel PNGs are byte-identical
to the inspected inputs. Final binary SHA-256 `77438e8f7c98521423a6979c982bbbae8d6d70424e4545e45ce4f966daf2f743`.
The earlier transport ConnectionReset passed isolated and complete-group
rechecks and the repeated required full suite; its cause remains an inference.
No test was weakened, no planner gained dimensions, no catalogue/task JSON
changed, no commit or push. Final counts and limitations are in the G1 status
entry and known_completeness.json `not_done`; G2/G3 still own the dimensions owed.

## Stage G1 fix round (2026-10-11)

Overturnable unattended judgements:

- Retain the brief's warning for every single-solid face outside the Analysis
  feature inventory, including stock faces. Consequently all thirty quality parts
  and a fully dimensioned feature-free box need attention. This is a conservative
  recognition-review floor, not a discriminator of drawing completeness. Crediting
  envelope dimensions to arbitrary faces would invent occurrence ownership; a future
  stock-face disposition ledger should supply explicit evidence before changing this.
- An established linear array or grid may be located by truthful
  ordinates for all members, or one fully located physical member plus all printed
  pitches (the anchor route requires a proved lattice for a grid). Fully located
  members also prove spacing without redundant pitch labels, including rotated grids
  in principal planes.
  Claims must name the exact member and axis and print its distance from the datum.
  Bolt circles keep their centre/PCD requirements; numerical member positions alone
  do not certify that circular arrangement in this fix.
- Keep GRM-03's existing chain-credit verdict: 0.5+2+3+18 proves 23.5 geometrically.
  A draftsman would normally still want an overall length to control tolerance
  stack-up; maintainer confirmation remains owed. This stream adds no dimensions.
- Preserve the 150 ms session performance test and transport assertions unchanged.
  Reviewer failures under load (~60) passed serial rechecks, suggesting pre-existing
  load/timing sensitivity; this is evidence of flakiness, not a proved diagnosis.

Fix-round timing evidence: an initial full run passed 677/677 before the final
rotated-grid extension. On the final source, the required eight-thread suite hit
`session::nist_benchmark_deltas` at **158.08625 ms** against its unchanged 150 ms
threshold (other seven session tests passed). The unchanged complete session group
then passed **8/8** with `--test-threads=1`. Session does not invoke the changed
independent pattern lint. Preserve `target/g1-fix-checks/test-session-timing-failure.log`
and `session-serial.log`; the repeated required eight-thread full run passes
**678/678 in 72 groups, none ignored**, including NIST timing and HTTP/stdio
transport, on the same final source. No assertion, threshold, transport or session code was changed.

## G2 plate and wall thickness (2026-10-11)

Ported ISSUE-1373-a, LINT-plate_coverage-thickness,
LINT-plate_coverage-exclusions and LINT-code-plate_requirement_state. Catalogue
YAML is deliberately left to G4. `src/dims/plates.rs` creates stable
`d:f:plate:<n>/thickness.length` items: z left of front, y above side, x below
front, with ordinary omission, tolerance, view and placement edits. Equal
thicknesses of one provider body share a counted statement, claiming every slab.
`src/lint/plates.rs` reads the Analysis and rendered annotation provenance only.
A count or tolerance number cannot confirm the nominal thickness. Authored plate
omissions reach the consumer as zero-ink `suppressed:<dimension id>` annotations;
they never count as measured facts. The drawing's metadata distinguishes
missing, suppressed and unverifiable without letting lint read the Spec.

Overturnable judgements made during this unattended run:

- Require native single-axis slabs too when no printed alternative states their
  interval. Python's multi-axis conversion gate alone proves no thickness; 1000,
  10033, 10047, 10049 and 10060 expose the gap. Retain native records, rather than
  importing Python's IR conversion into the Analysis. Python comparisons record
  the m1000 difference in `tests/fixtures/known_dimensions.json`.
- Equal raw thicknesses (within 1e-6 mm) on exactly the same provider body key
  share `N× size`, even across axes; different authored bands do not merge.
  ftc06's two Python `31.8` dimensions become `2× 31.8` with both claims, with an
  equivalent verdict in the known-dimensions comparison. In multi-axis parts
  explicit thickness takes priority over subtracting two ladder heights.
  Single-axis slabs retain Python's proven ladder alternatives: e.g. 10047's
  y interval [23.144,31.800] is stated by its located shoulder and overall depth.
  That avoids adding a closed dimension chain; it is derived thickness evidence,
  rather than a separate 8.7 label. This is an overturnable interpretation of
  'stated once' and is reported explicitly in the final sheet audit.
- Interval matches use 1e-3 mm because Plate lo/hi are quantised to three
  decimals while the envelope and ladder retain kernel precision. Alternative
  credit still requires the actual owners' claimed and printed measurements.
- Supporting polygonal slabs require the witness inside the support polygon and
  one solid (Analysis principal profiles), or matching body keys. Slot-pattern
  webs require identical body scope and a cut crossing the witness; disconnected
  lookalikes receive no exclusion. If old bridged analyses lack that proof, keep
  the thickness requirement. Upper U-channel walls require a unique lower slab,
  matching witnesses, a full-span opening and all three printed dependencies.
- Plate corner reservations cover short-span outside arrows and text. They do
  not change the general policy for other families' open-paper bands. A family
  whose own annotation carries a native plate interval (a turned band or through
  leg) may cover it only when the retained measurement states the interval.

Evidence and final checks are recorded in the G2 entry of
`docs/status-2026-10-10.md`; working artifacts use `target/g2-*`.

The Y-wall default falls back from side to plan when its projected width is
less than two arrowheads; the plan uses the same proven interval in its left
band. A Spec view request wins. This is a deliberate, overturnable placement
difference from Python's fixed Y-to-side map (FTC-07's 5.08 mm back wall). The
front's extra band counts actual linear lanes, excluding leader callouts that
share the lower strip but do not stack in linear lanes. Corner reach uses the
actual counted label width and the existing arrow/text gap, avoiding fixed
part-specific clearances.

Counted walls promoted after an omission retain a strip parallel to their own
projected interval; copying an orthogonal owner's strip had collapsed the
promoted FTC-06 Y thickness to zero. The strengthened omission test checks the
native face-to-face span through the rendered arrow tips and still requires all
unrelated ink to stay fixed. Multi-measurement alternatives list all owners in
`conveyed by` evidence; the merge tests now exercise each dependency ID separately.
The exact FTC-06 comparison ledgers retain its remaining leader crossing and
fourteen band overruns; the disappeared reciprocal crossing and bounded-search
finding are removed. Captured m1000's seat callout newly overruns its front band
by 2.1 mm (four exact overruns rather than three); this remains `rust-wrong` in
known_placement.json. Native quality and raster checks are separate evidence,
so that residual is not presented as fixed. GRM-03 gains a 2.5 head thickness;
its separate 0.5 and 2 turned bands do not alone state that aggregate interval.

A Covered item's leading `conveyed by <ID>:` phrase names the single stable
representative in its typed state. Multiple printed dependencies follow after
`; dependencies:`; they are not a fabricated comma-separated item ID. The old
prose-versus-typed review comparison stays unchanged and checks this convention.
Aggregate ladders and derived opposite walls now require Analysis evidence of
one solid: those aggregate records cannot establish body-local ownership in an
assembly, or in an older Analysis without principal-profile evidence. Turned
bands read their provider body key from the profile, and contradictory known
body keys reject polygonal support even if other metadata says one solid. The
six coverage tests include negative disconnected/unknown-body cases; absent
ownership evidence conservatively keeps an explicit thickness requirement.
Null body keys do not join equal thicknesses into a counted statement. The
public planning regression also checks matching, distinct and unknown bodies.

Final G2 verification: 673/673 Rust tests, 73 result groups,
zero failed/ignored; fmt and strict release all-target clippy pass. Final release
quality passes 30 parts / 300 cells against the prior Stage F table, with three
improvements and 297 unchanged cells. Ink 278 → 273; through-feature lines
212 → 211; every other total unchanged. Final --require-go pin grid is 270/270 GO,
zero off-sheet; scripted MCP is 7/7. Python quality tests 20/20, Node client 8/8.
All thirty final SVGs match the reviewed raster-input hashes; the eleven relevant
sheets were also inspected individually. The final binary SHA is in the G2 status
entry. No commit or push was made.

## G2 review fix round (2026-10-11, unattended)

The fixed axis order put 1000's 4.2 wall on a hidden front edge and 10038's
4.4 flange on absent front geometry. Principal-profile Analysis evidence now
retains optional straight outer boundary edges. The plate planner prefers a
facing boundary profile containing both thickness edges with a shared span,
then attaches witnesses to the end nearest its carrying strip. No part names
enter planning. Spec view requests win; old Analyses retain the earlier routing
(axis map plus the existing narrow-Y readability preference).
The isolated-solid restriction avoids claiming visibility across an assembly.
Recognition-frame body bounds are transformed before matching the evidence.

Removed the multi-axis exception to interval consolidation. 10007's 3.9 is
covered by 28.0 and 31.9; its 11.7 is covered by the through-step's 2.3 baseline
leg and the 14.0 rung. A through-step leg can replace a ladder rung only with
body-local interval proof and both printed dependencies. Omitting either owner
restores the plate dimension. This applies the existing 10047 policy consistently.

Overturnable judgement calls: exposed boundary profiles take priority over
Python origin/main `_thin_profiles.register_plate_thickness`'s fixed map;
absent evidence retains that map. A thickness derived from two retained,
proven stations is covered, avoiding a redundant closed chain; this replaces
the earlier multi-axis judgement. Short Y walls still prefer the plan when
its profile is available. Outside thickness text reserves its adjacent corner;
the aligned front/side row shares an upper corner reservation, like its lower
band. A new plate lane above plan reserves the actual linear lanes sharing it.
Front-left lanes now feed `plate_front_left`, separately from `plan_left`.

Regression evidence and final verification: see the appended G2 review entry in
`docs/status-2026-10-10.md`; artifacts are under `target/g2-fix-*`.

The captured m1000 seat-diameter overrun is fixed (4 → 3 exact band findings).
FTC-06's captured reciprocal leader crossing returns (1 → 2) when the counted
thickness moves from front to its exposed side profile; both are explicitly
`rust-wrong` in the unchanged exact comparison predicates. Its fourteen band
findings remain unchanged after the corner reservation. No source severity,
quality predicate, blend logic or catalogue YAML changed.

The minor FTC-07 A3 1:5 recovery observation remains 16 more items over another
view at the rejected A3 1:1 alternative (14 before thickness planning). The
three thickness statements are required manufacturing information; that try
remains rejected and the final A2 1:2 drawing has unchanged native quality.
Do not hide required thicknesses or relax the recovery gate to improve a rejected
alternative's count. Fixing that alternative needs broader lane packing, which
is deferred as an overturnable scope judgement; its exact test remains intact.

The visual audit also exposed 10038's 7.2 + 4.4 = 11.6 chain. For a retained
top slab, its thickness and the printed overall height now cover its underside
ladder station; 7.2 is withdrawn while 4.4 remains on the exposed side profile.
This is the inverse of the existing base-slab ownership of its top station.
A station borrowed by another plate stays retained, and omitting the thickness
restores its underside station. This characteristic-size preference is an
explicit, overturnable judgement; the regression checks both the chain removal
and the restored station. Endpoint coverage keeps every valid rung/through-step
alternative, so an unprinted recognised replacement cannot displace a printed
ladder owner.

The same top-slab rule withdraws FTC-06's 215.9 underside rung: the exposed
side's counted 31.8 and front overall 247.7 convey it. The Python difference
has a `rust-correct` verdict with the interval evidence in known_dimensions.
The old sampled-step regression now checks both printed owners, all other
Python heights unchanged, and restoration of 215.9 when either owner is omitted.
The placement omission test checks the restored datum-to-underside span in
front for this inverse ownership, rather than requiring a noncoincident rung
to reuse the omitted side slab's lane; unrelated view and ink identities stay
asserted. Native FTC-06 quality is unchanged. Its captured reciprocal crossing
is the pre-G2 crossing restored when the front thickness obstacle is removed;
this residual remains `rust-wrong`, rather than being accepted as good drafting.

The editability/trust regression now recognises every exact ID in the recorded
coverage dependencies, in addition to the first `conveyed by` owner. Its former
prefix-only lookup missed restoration after omitting the overall-height owner.
Exact surviving item sets, restoration of all promoted items, omission counts
and individually authored dimensions remain asserted. The focused twelve-test
trust group passes with both owners treated consistently.

Final fix-round verification (completed 2026-10-12): 678/678 Rust tests in
73 groups, zero failed/ignored; fmt and strict release clippy pass. Release
quality: 30 parts / 300 cells identical to preceding G2 table, no part worse;
ink 273, through-feature 211. Pin grid 270/270 GO, zero off-sheet; scripted MCP
7/7; Python 20/20, Node 8/8; plate audit passes. Twelve sheets inspected;
all changed native SVGs match the reviewed hashes. Grid/MCP binary SHA and
baseline match final output; see the appended G2 fix-round status entry and
`target/g2-fix-evidence/final-verification.json`. No commit or push.

## G2 review round 3: thickness witnesses leave through the silhouette (2026-10-12, unattended)

Review found 10033's 8.3 (plan) and 10049's 4.5 (front) with extension lines
starting mid-view and running across the part to a strip fixed by Python's axis
map. 10038's 10.1 had the same defect above the side view. `prefer_readable` now
anchors the witnesses at an end of the two solid edges' overlap that lies on the
view's silhouette (`placement::silhouette_gap`, within 0.05 mm), preferring the
default strip's end. If no view has such a pair, it takes the pair whose end lies
nearest the silhouette, provided that is within 3 mm on the page
(`NEAR_SILHOUETTE`); otherwise the fixed-map fallback applies as before.
`placement::lanes` puts a profile-anchored thickness in the strip on the
silhouette side nearest its anchor, so a thickness can now take the front's upper
band, the plan's right or lower band, or the side's left band. Those bands are
reserved in `Lanes` and `Strips::reserve`. Results:
10049's 4.5 sits above the front. 10038's 10.1 sits below the side, inside the
29.7. 10033's 8.3 sits above the side, because the plan foot's lower edge ends at
a chamfer, so the plan cannot carry it cleanly. FTC-06's counted 2× 31.8 stays
right of the side (second tier). Its flange end faces stop 2.5 mm short of the
outline, and the witnesses run over the middle section's top and underside
object lines to the silhouette. With the strict rule alone it fell back to the
fixed map's front-left, where the lower witness lands on a hidden line. A
horizontal side thickness's outside arrow now reserves the side's left gap below
the view as well as above it. Without that, 10038 showed a new 4.7 mm band overrun.

Overturnable judgement calls:
- Silhouette means the view's projected envelope (frame outline, 0.05 mm). A wall
  whose faces end at a local notch rather than the outer box is not anchored there.
- The 3 mm second tier (`NEAR_SILHOUETTE`) is a tuned allowance: about one
  arrowhead of witness over the part. Today it applies only to FTC-06.
- 10033's 8.3 goes to side rather than plan-right as the reviewer suggested,
  because plan's lower edge stops at the C3.4 chamfer. Residual: the R6.1 callout
  in side now sits between the 8.3 witnesses with its shoulder ending on the right
  witness (a touch, not a counted crossing). Counting a line within LABEL_PAD of a
  label as text contact would move it, but changes leader placement corpus-wide
  (4 fixture tests); left for the leader stream.
- The minor coverage policy point stands as before and is NOT counted as
  acceptance: on 10007 (3.9 = 31.9 - 28.0, 11.7 = 14.0 - 2.3), 10047 (31.8 - 23.1)
  and FTC-07 plates 2 and 5 the thickness is only derivable from two retained
  stations, matching Python's lint (`_step_level_dependencies`). That contradicts
  the brief's lesson as written and the inverse top-slab rule (10038, FTC-06);
  the brief owner should decide which way it goes.
- FTC-07's rejected A3 1:1 alternative still has 16 items over a view (14 before
  G2); unchanged by this round and recorded, not fixed.

Also fixed: the HTTP MCP server closed connections with an unread request body
after early refusals (405/403/400), so the kernel could reset the connection
before the client read the response (tests/stateless.rs:103 ConnectionReset).
It now half-closes and drains (bounded, 2 s) before closing. Before the fix the
test failed in both of this round's runs (full suite and a six-binary subset);
after it, 5/5 isolated runs and the final full run pass.
