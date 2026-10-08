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
  fingerprints that include the AAG neighbourhood. haecceity is pinned at quiddity-rust a46b91c
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

haecceity and quiddity come by git from `pzfreo/quiddity-rust` (`Cargo.toml`: branch
`prototype/rust-port`), pinned to one commit in `Cargo.lock`, the same for both
(`tests/native.rs` checks it). To move the pin:

1. `cargo update -p haecceity -p quiddity`, and note the old and new commits (the `source`
   lines in `Cargo.lock`). A new recognition schema version or family fails
   `tests/native.rs` until `bridge::native` is taught it.
2. Re-run the full verify line above.
3. Record every change it causes: a known-differences entry that went away or appeared, a part
   the bridge now reads or refuses, a changed count. Each gets a verdict with evidence in its
   list, and the commit message names the new pin and what changed (971e17e is an example).

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

- Facts: `capture_sheet_facts.py` gives no usage message. `docs/facts.md` doesn't explain the
  identity-collision nulls.
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
    - Answered 2026-10-08 (applied by stream d-location-decisions): a blind slot is located
      across its width, as a through slot is.
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
