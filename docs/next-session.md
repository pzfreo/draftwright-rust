# Next session: handoff (2026-10-08)

This is written for a fresh session continuing the port with parallel workflows ("ultracode").
Read it first, then the documents it points to: `docs/review-2026-10-08.md` (the latest review,
its improvement plan and the maintainer questions) and `docs/plan.md`'s "Next steps
(2026-10-08)".

## Where things stand

**quiddity-rust** (`~/repos/quiddity-rust`, branch `prototype/rust-port`, at c9ce1cd):

- `crates/haecceity` is the kernel. It covers:
  - STEP import, rays, classification, volume probes and coverage;
  - surface recovery;
  - hidden lines with exact projected curves, and sections without booleans;
  - prism probes and face sweeps, and face mass.
- 28 recogniser entry points are ported (the README's table). Each family is checked:
  - call by call against Python's captured test calls;
  - over the 100-part corpus, including defining faces;
  - under rigid motion.
- Correspondence (`docs/correspondence.md`) is implemented in the library and the CLI
  (`quiddity part.step` writes records and fingerprints; `quiddity correspond old new`), not yet
  in a JSON-lines or MCP surface.
- Its own review (`docs/review-2026-10-08.md` there) found CI red at 2fa7207 on a pinned face
  count that depends on float round-off.
- Verify with
  `QUIDDITY_CORPUS_REQUIRED=1 cargo test --workspace --release`, fmt and clippy. The corpus is at
  `~/repos/quiddity/tests/corpus`. Set `QUIDDITY_CORPUS` when working from a worktree.

**draftwright-rust** (`~/repos/draftwright-rust`, `main`; the state below is at bbd3dd7, with
the corpus check, CI and these docs from stream d-ci-docs on top):

- **The document model** (ADR R1, `docs/model.md`):
  - Analysis, Spec and Plan;
  - `analyse`, `plan`, `edit`, `render`, `review` and `spec`, through the CLI, the JSON-lines
    service and the MCP server (`src/api.rs`, `src/bin/`);
  - the spec embedded in SVG, PDF and DXF;
  - output deterministic to the byte, in every time zone (`tests/timezone.rs`).
- **Layout:** draftwright's sheet, scale and view layout arithmetic (`src/layout.rs`), exact
  against 285 recorded Python calls. Differences from Python's final sheets are listed with
  verdicts in `tests/fixtures/known_views.json` (71 entries).
- **Bridge:** quiddity's features come in through `src/bridge.rs`, with stable IDs and
  fingerprints that include the AAG neighbourhood. haecceity is pinned at quiddity-rust 4258e35
  (`Cargo.lock`; AGPL-3.0-only since 8023ea7); the bridge reads nist_ftc_07 and nist_ftc_10.
  nist_ctc_04 is still refused (one face's area differs from OpenCascade's by 0.15 %). Since
  05c5d99 haecceity refuses to draw a part whose drawn faces or edges did not resolve on
  reading, and since 8023ea7 one with a stretch whose visibility no ray can decide (no corpus
  part has one); `analyse` and `render` pass that refusal on (`render` as a `failed` error).
  Since 8023ea7 it makes no stretch of no span, so cgb242 no longer reports `edges_not_drawn`
  and no corpus part has a drawing problem (`output::projected` stays as a guard).
- **Merged streams** (2026-10-07):
  - **View gaps** (d707219): a turned part drops the view that repeats its profile (ISSUE-1262-a,
    pending: the veto is narrower than Python's); annotation bands reserved from the features
    (`src/demand.rs`, `known_bands.json`); section A–A planned and hatched at 45°.
  - **First dimensions** (5b7be0d, now `src/dims/`, one planner module per family): the envelope, turned steps and hole callouts
    with locations and pattern pitches, as Plan items with IDs, labels, defaults and bases;
    compared with the captured facts (`tests/dimensions.rs`, `known_dimensions.json`).
  - **Lint** (54ce4cc, `src/lint/`): a guarded module that never reads the Plan or Spec,
    `LintIssue`, and hole coverage, compared with Python's lint on the captured sheets
    (`tests/lint.rs`, `known_lint.json`).
- **Facts:** Python draftwright's sheets are recorded as facts (`tools/capture_sheet_facts.py`,
  `tests/fixtures/sheets/`, `docs/facts.md`) for 13 parts.
- **ADRs R1–R4** are accepted. `docs/rules/` has 515 rules learnt from Python, classed drafting,
  trust, spec or machinery, with statuses (23 ported after wave 2). **Update a
  rule's status when you port it;** a partial port stays pending with `rust_test` and
  `rust_note`.
- `docs/plan.md` is the overall plan.
- **Verify** with
  `DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`,
  `cargo fmt --check` and `cargo clippy --all-targets --release -- -D warnings`. Without
  `DRAFTWRIGHT_CORPUS_REQUIRED=1`, corpus-backed tests skip (print SKIPPED and pass) when the
  corpus is missing; with it, they fail.

**Python references (read-only):**

- draftwright at origin/main 586628b. Read it with
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
`DRAFTWRIGHT_CORPUS_REQUIRED=1`.

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
- **Priority for the next dispatch (main session, 2026-10-08):** quiddity-rust's GitHub CI (Linux)
  has been red since the first run on one platform-dependent count (cgb202 torus
  `uv_bounds … another turn`, 13 listed, 12 found). The fix is ready on quiddity-rust's local
  branch `q-seam-fix` (commit 1966d3c, "uv: place a loop centred on the seam deterministically",
  with its known-divergence updates; full checks passed on macOS against 1bf3db2). Dispatch a
  stream `q-seam-fix` whose brief is: rebase that commit onto the current branch head, re-run
  the full checks, adjust the known-divergence counts the newer merges change, keep the 1e-9
  seam band and its reasons; then the integrator merges it like any other stream.
- **Priorities from a CLI trial (main session, 2026-10-08, afternoon):** the maintainer is trying
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
- **Main-session decisions, 2026-10-08** (judgement calls; the maintainer can overturn them):
  - fingerprints belong to quiddity (ADR R3). Once quiddity-rust publishes its recognition
    document, `bridge.rs`'s own fingerprint is retired in favour of quiddity's;
  - fingerprints and correspondence use only verified occurrences;
  - `plan`, `render` and `review` accept an Analysis document, as `docs/model.md` says, so a
    caller need not re-analyse;
  - quiddity-rust's CI also runs on macOS, beside Linux.

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
      twice.
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

- From stream d-rebase-lint-native-recesses (2026-10-08), for the integrator:
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
