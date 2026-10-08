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

## Open leftovers from earlier reviews

The 2026-10-08 review (`docs/review-2026-10-08.md`) supersedes the old spike list: its
improvement plan and questions are the current work. Still open from before it:

- Facts: `capture_sheet_facts.py` gives no usage message. `docs/facts.md` doesn't explain the
  identity-collision nulls.
- The MCP server hasn't been tried in a real client.
- PDF fonts are embedded whole, not subset.
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
    - draftwright does not dimension a slot's position across its width, so the port doesn't.
      Should it?
    - Boss locations go beyond draftwright, which locates no boss (ftc06's 266.7, 38.1, 139.7
      are marked rust-correct). Keep them?
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
    - A blind slot is not located, as in draftwright (it opens at the part's edge, so its
      position along the run is fixed, but not across it). Should the port locate it across?
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
  - Edit locality (ARCH0003-2) does not hold: lanes are packed in order, so omitting one item
    moves others (on plate.step, omitting the column pitch moves the x location and the
    callout). Owed in placement.
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
