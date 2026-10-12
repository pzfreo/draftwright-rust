# draftwright-rust

A pure-Rust port of [draftwright](https://github.com/pzfreo/draftwright): automated,
fully-annotated engineering drawings from STEP B-rep. No OpenCascade: geometry comes from the
[haecceity](https://github.com/pzfreo/quiddity-rust/tree/prototype/rust-port/crates/haecceity)
kernel, and features from the Rust port of quiddity, both in
[quiddity-rust](https://github.com/pzfreo/quiddity-rust).

**Status: alpha 1** (`0.1.0-alpha.1`, not yet tagged: the version and tag are set when the
release is cut). Its gates are in `docs/alpha1.md`; what changed is in `CHANGELOG.md`. It is an
alpha: every drawing reports what it does not state, but drawings are not yet as clean as Python
draftwright's on dense parts (see "Known limitations"). The port is checked against
what Python draftwright produces, and every difference has a verdict (rust-correct, rust-wrong,
equivalent, undetermined or not-applicable) in a known-differences list under
`tests/fixtures/` (`known_*.json`).

## Building

The toolchain is pinned in `rust-toolchain.toml`. haecceity and Rust quiddity come by git from
the public `pzfreo/quiddity-rust`, pinned in `Cargo.lock`, so no secret is needed.

```
cargo build --release      # target/release/draftwright and target/release/draftwright-mcp
```

## Using it

A drawing is three JSON documents: the Analysis, the Spec and the Plan. Five stateless
operations work on them (`docs/model.md`, ADR R1). Every command prints JSON.

On the command line, `analyse`, `plan`, `render` and `review` take the part's features from
Rust quiddity's recognition, run on the part (`src/bridge/native.rs`); the Analysis names it
(`recognition.recogniser`, `quiddity-rust <version> (quiddity-rust/recognition/2)`).
`--features` passes Python quiddity's recognition JSON instead, written once with
`tools/recognise.py`; `--no-features` draws with none (no feature dimensions, sections or
lint); `--analysis` reuses an Analysis that `analyse` printed for the same file:

```
draftwright analyse part.step > part.analysis.json     # the Analysis, features from Rust quiddity
draftwright plan part.step --spec part.drawing.json    # the Plan, every choice with its basis
draftwright plan --analysis part.analysis.json         # the Plan, without analysing again (about
                                                       # 16 ms on NIST FTC 06); naming part.step
                                                       # too checks the Analysis was made from it
draftwright edit part.drawing.json ops.json            # apply edits to a Spec
draftwright render part.step --spec part.drawing.json -o out
                                                       # out.svg, out.pdf
draftwright render part.step -o out --formats svg,pdf,png --dpi 150
                                                       # choose the formats; PNG at --dpi
draftwright review part.step --spec part.drawing.json  # the drawing as SVG text, with problems and lint

~/repos/quiddity/.venv/bin/python tools/recognise.py part.step -o part.features.json
draftwright render part.step --features part.features.json -o out
                                                       # features from Python quiddity instead
draftwright spec out.pdf                               # the Spec a drawing (SVG, DXF, PDF, PNG) was made from
draftwright serve                                      # the same operations as a JSON-lines service
draftwright-mcp                                        # the same operations as an MCP server (stdio)
```

File exports stage each format in a unique sibling file and atomically replace its destination.
Concurrent exports to the same prefix use last-writer-wins per file: the last successful rename
wins. A multi-format export is not transactional; formats can come from different callers,
and a later failure leaves already published formats in place. Cleanup removes only that
invocation's staging file. Output names and rendered bytes are unchanged.

`--spec` is optional: without it the defaults are drawn. The service, the MCP server and the
library (`api::analyse(step, None)` and the rest) are not switched to native recognition:
there, no `features` still means no features, and Python's JSON is passed as `features`.
`api::analyse_from` and its siblings take `Features::Native` for the command line's choice.

To use the MCP server from an MCP client, register the `draftwright-mcp` binary as a stdio
server (for Claude Code: `claude mcp add draftwright -- /path/to/draftwright-mcp`). It has not
yet been tried in a real client.

## What alpha 1 does

- **Recognition.** Features come from Rust quiddity's recognition document
  (`quiddity-rust/recognition/2`), read fail-closed: an unknown schema, family or record type is
  refused with the reason, never guessed. The families read are listed in
  `src/bridge/native.rs` (`READ`). Families the document has but the reader does not take are
  named in the Analysis (`unrecognised`) and reported by the plan (`features_not_recognised`).
- **Views and layout:** front, plan, side and isometric views from haecceity's hidden-line
  projection, with draftwright's sheet, scale and view arithmetic (`src/layout.rs`); room is
  reserved for each view's annotation bands, so that views do not overlap each other or the
  title block. A plan with a hard layout failure (an annotation off the sheet or over another,
  views that do not fit) or a dropped hole item is re-planned at other standard scales and
  sheets, and the first valid, complete one is taken (page escalation, `model::recover`). The
  title block, general notes and the spec's corner-block tables are drawn.
- **Section A–A** by draftwright's rule, done without a boolean, hatched, with its
  cutting-plane line and caption.
- **Dimensions,** one planner per family (`src/dims/`): the overall envelope, turned steps,
  holes and hole patterns (callouts with counterbore, spotface and countersink, locations and
  pitches), slots and slot patterns, bosses, fillet and chamfer callouts, grooves, flats,
  steps and the height ladder, section recesses (from `--features`; see below) and hexagonal
  stock. Each is a Plan item with an ID, label and basis. Tolerances and ISO 286 fits come from
  the spec or the part's PMI, never guessed from the geometry.
- **Hole tables** by draftwright's drop-and-escalate: a hole callout or location that cannot be
  placed on the sheet is dropped and reported, and a dense plan view escalates to a hole table
  with balloons (`src/hole_table.rs`; limits under "Known limitations").
- **Lint** (`src/lint/`, independent of the Plan and Spec): every recognised feature's
  requirements must be stated on the drawing; `review` reports what is not, with structural
  checks and a coverage backstop.
- **PMI from AP242:** the part's semantic PMI is read through haecceity (`src/pmi.rs`) and drawn:
  feature control frames and datum feature symbols (`src/pmi_lowering.rs`, `src/dims/gdt.rs`),
  and toleranced and basic dimensions (`src/dims/pmi_dimensions.rs`), given to the model
  dimension that states the same quantity or drawn beside it (basic ones boxed), angles between
  two planar faces in the view that shows them true; surface and common labels with leaders;
  thread and knurl requirements on the callout of the hole or turned step that owns them, or
  `SEE MFG n` with a manufacturing requirements table; the general tolerance in the title block,
  the default surface finish beside it, edge conditions as general notes, and the title block's
  material from the file's one usable material. The layout reserves band space for frames,
  datum symbols and labels. The **PMI coverage lint** (`src/lint/pmi_coverage.rs`) checks that
  every PMI item is on the drawing or reported. What is not drawn is reported, never dropped
  silently. Checked record by record against Python draftwright's drawings of 22 AP242 files
  (NIST CTC/FTC/STC models and files written by specify-core and by haecceity's PMI writer):
  every difference has a verdict, and none is rust-wrong.
- **Output:** SVG, PDF and PNG (`src/output.rs`, `src/png.rs`); DXF is drawn but its export is
  disabled until it is fixed. The PDF embeds subset
  fonts and carries every label as searchable text, callout symbols included; the SVG carries
  the text too. Every format embeds the Spec, which `draftwright spec` reads back.
- **Deterministic:** the same input gives the same bytes, run to run and in every time zone
  (`tests/timezone.rs`, `tests/png.rs`).

Problems are reported, not hidden: anything the plan could not place, state or draw is a
Problem with a code on the item's ID, in `plan`, `render` and `review`.

## What alpha 1 does not do

- **PMI, not drawn yet:** lengths oblique to the views' axes, a cone's angle (an angle no view
  shows true), diameters or radii not along x, y or z, a size several faces state together that
  draftwright gives its own step, and bolt circles whose holes carry different requirements
  (`pmi_dimensions_not_ported`). A thread or knurl on a boss, face-finish requirements, datum
  scheme and model representation notes are read but not drawn; each is reported
  (`pmi_not_lowered`). A record haecceity could read only in part (NIST CTC-02's
  composite_shape_aspect groups, for example) is reported, not drawn, where draftwright draws
  OpenCascade's reading.
- **Families not read natively:** draftwright's section recesses (pockets, channels, blind
  slots, passages, read as one section-recess feature) are not in Rust quiddity's recognition
  document: it carries the pocket, channel, blind slot and passage records they are made from,
  which the reader does not take. Some other families it has (oriented chamfers, …) are not read
  either. Both are named in the Analysis and reported as `features_not_recognised`;
  `--features` gives section recesses from Python quiddity.
- **Families read but not dimensioned:** a recognised feature that no planner claims (blends,
  for example) gets a `dimensions_not_ported` info problem. An angled step cannot be dimensioned
  (draftwright has no grammar for it) and gets an `angled_step_requirement_unsupported` warning.
- **Native recognition gaps** (`tests/fixtures/known_native.json`): a hole's faces lack the face
  that closes it (a drill point's cone, a blind floor), and a boss its free-end face, as the
  document carries only defining faces (rust-wrong).
- **Revisions:** a spec made for another revision of the part keeps its sheet, view and
  envelope choices, but its feature entries stay inactive until revision rebinding (ADR R3) is
  implemented.
- **Callouts inside a view:** draftwright puts some callouts over blank face inside the view;
  the port places every annotation outside the views (it has no view line-work to test against,
  ADR R1), so some leader crossings draftwright avoids that way remain (below).
- **The rules catalogue** (`docs/rules/`) tracks what Python draftwright learnt; most of its
  rules are still pending.
- DXF table head handles (ezdxf adds them on audit) are not written.

## Known limitations

Measured for alpha 1 at 873506e with `tools/quality.py` on the 30 parts of
`tools/quality_parts.txt`; the per-part figures, the comparison with the 86ffcbc baseline and
the evidence are in `docs/quality-2026-10-09.md`. On those parts no annotation is off the sheet,
no view overlaps another or the title block, no annotation lies over a view and no annotation
line crosses one. What remains (282 ink problems, from 506 at the baseline) is reported on the
plan, never hidden:

- **Leader crossings, 109** (`annotation_leader_crossing`, warning; ctc04 29, ftc08 24,
  ctc02 21, others 35). Rust-wrong, mostly inventory-limited: in a sample of 73 measured earlier,
  71 were crossings every candidate leader of that callout has, so no ordering or cost change
  removes them. draftwright avoids some by putting callouts inside the view (above) and the
  plan's Y locations above the side view (an open maintainer question).
- **Ink past a view's area, 155** (`annotation_outside_area`, info). Rust-wrong as to the
  reservation, harmless on the page: lanes above and left of the plan and below the front run
  past the bands draftwright's estimate gives them, onto open paper, crossing nothing; reserving
  those bands moves sheets and scales on other parts (an open question).
- **Ink overlaps, 18** (`annotation_ink_overlap`, warning: a line through a label; ctc02 7,
  ftc08 7, ftc10 3, ftc09 1). Rust-wrong; how a text contact weighs against a leader crossing
  is an open question (draftwright gates text first).
- **Worse than the 86ffcbc baseline, accepted:** ctc03's ink 21 to 24 (a valid A2 sheet where
  the baseline's A3 had lines through labels), leader crossings on ftc08 (22 to 24) and ctc04
  (27 to 29), traded for locations brought onto the sheet and labels kept clear, and cgb245's
  leader crossings 0 to 2: one trades an off-sheet location for a crossing, the other follows
  from merged items sharing a lane and is accepted as an overturnable judgement. Each is in
  `docs/quality-2026-10-09.md`, "Accepted regressions".
- **Repeated dimension labels, 18** (16 groups on 9 parts), each with a verdict against
  draftwright's sheet in `tests/fixtures/known_repeated_labels.json` (kept by
  `tests/repeated_labels.rs`): 10 groups are identical features each dimensioned, 5 are distinct
  quantities that happen to share a value, and 1 (ftc10's front `12.5`, a hole location and a
  step-height rung on one span) is a repeat draftwright also makes, pending a maintainer
  question. The repeats on the AP242 fixtures (below) have no such verdicts yet.
- **The AP242 fixtures** (22 files with PMI, not in the 30): no frame, datum symbol, label or
  dimension is off the sheet or over a view, but dense parts carry much more ink: 1736 problems
  in all (1525 leader crossings, 132 past a view's area, 79 ink overlaps), most on NIST STC-09
  (521), FTC-10 (449) and STC-10 (288). STC-09 is drawn on A1 at 1:5 where draftwright keeps
  1:1 by dropping 15 items. Undetermined PMI differences (a part placed several times,
  dimensions on supplemental geometry, an ISO 286 class stated without limits, records
  haecceity reads only in part) wait on maintainer questions; see the `known_pmi*.json` fixtures.
- **Hole tables** are drawn only by drop-and-escalate: a hole callout or location whose ink runs
  off the sheet is dropped and reported (`callout_dropped`, `location_ref_dropped`,
  `off_axis_location_dropped`), and a plan-view drop escalates to a hole table with balloons
  only when the plan has at least 16 holes outside any pattern. An item that collides, rather
  than leaving the sheet, is placed and reported, not dropped; balloons are placed more simply
  than draftwright's, and a table whose balloons do not all land is withdrawn whole. On the 30
  parts no table is drawn: ftc08 drops 3 plan-view locations and ftc09 a callout and 13
  off-axis locations. Hole tables are not an alpha 1 gate.

## Background: the four hard pieces

As with quiddity, the hard pieces were proved first, each against what the Python implementation
produces:

1. **Hidden-line projection** (OpenCascade's `HLRBRep`): visible and hidden sharp edges, smooth
   edges and silhouettes per view, in haecceity (`hlr::project`), checked against OpenCascade's
   drawings of the 100-part corpus.
2. **Section views** without a boolean, in haecceity (`hlr::project_section`, `hlr::section`).
3. **Text geometry:** glyph outlines and extents as build123d's `Text` gives them
   (`src/text.rs`, `tests/text.rs`, recorded by `tools/capture_text.py`).
4. **Output:** SVG, DXF and PDF as draftwright writes them (`src/output.rs`, `tests/output.rs`,
   `tools/capture_output.py`, `tools/compare_pdf.py`): the same lines and styles per layer, and
   PDFs that render to the same ink with the same searchable text.

The plan (`docs/plan.md`), the ADRs (`docs/adr/`), the rules catalogue (`docs/rules/`), the
recorded facts (`docs/facts.md`), the handoff (`docs/next-session.md`) and the latest review
and status files are in `docs/`.

## Checking a change

`DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test
--release`, `cargo fmt --check` and `cargo clippy --all-targets --release -- -D warnings` (as CI
does, `.github/workflows/ci.yml`). With `DRAFTWRIGHT_CORPUS_REQUIRED=1` a missing corpus fails
every corpus-backed test; without it they are skipped.

## Taking a new haecceity

haecceity and Rust quiddity come by git from `pzfreo/quiddity-rust`. Alpha 1 pins them to the
tag `v0.1.0-alpha.1` (`Cargo.toml`), one commit in `Cargo.lock`. To follow the development
branch again, set `branch = "prototype/rust-port"` in place of the tag in `Cargo.toml`; then, to
move the pin to what is pushed on that branch:

1. `cargo update -p haecceity -p quiddity`. Both must name the same commit (the `source` lines
   in `Cargo.lock`); `tests/native.rs` (`quiddity_and_haecceity_are_pinned_together`) fails if
   they differ. Note the old and new commits.
2. Fix what no longer builds in the same change. For example, 879dcf7 took haecceity's
   projections returning `Result` and answered a refused projection as a failed call.
3. Run the full check above. No fixture is recaptured for a pin move: everything under
   `tests/fixtures/` (sheets, layout, recognition, output, text, lint ledgers) was written by
   Python draftwright or Python quiddity, or is a corpus STEP file (`parts/`), and does not
   depend on the pin. What can change is the
   port's side of each comparison, so the known-difference lists are re-checked by their tests:
   - `known_native.json` (`tests/native.rs`, Rust quiddity against Python's recognition). A new
     recognition schema version or family is refused until `bridge::native` knows it (30cfb98).
   - The bridge's face-area binding: a part it now reads or refuses changes `known_views`,
     `known_bands`, `known_dimensions`, `known_lint` and `known_placement`.
   - The projections themselves: `tests/output.rs`, `tests/png.rs`, `tests/sections.rs`.

   A listed difference that goes away, or a new one, fails its test. `NATIVE_REPORT=1`,
   `DIMS_REPORT=1`, `LINT_REPORT=1` and `PLACEMENT_REPORT=1` print the differences. Each change
   gets a verdict with evidence in its list.
4. Commit as "Pin haecceity to quiddity-rust <commit>", saying what changed, or "No API change"
   when nothing did (df90ace), and add a line to the current `docs/status-<date>.md`, as
   4462375 was recorded: "4462375: haecceity and quiddity pinned to quiddity-rust 7f4b1a9. No
   API change." CI needs nothing: the dependency is public.

Licensed AGPL-3.0, as draftwright is; so are haecceity and quiddity-rust (AGPL-3.0-only since
quiddity-rust 8023ea7). The fonts in `fonts/` are IBM Plex, vendored from draftwright under the
SIL Open Font License 1.1 (the `LICENSE-*-OFL.txt` files beside them).
