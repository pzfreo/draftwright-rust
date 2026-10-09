# draftwright-rust

A pure-Rust port of [draftwright](https://github.com/pzfreo/draftwright): automated,
fully-annotated engineering drawings from STEP B-rep. No OpenCascade: geometry comes from the
[haecceity](https://github.com/pzfreo/quiddity-rust/tree/prototype/rust-port/crates/haecceity)
kernel, shared with the Rust port of quiddity.

**Status: the four hard pieces are proved.** As with quiddity, the hard pieces are proved first,
each against what the Python implementation produces:

1. **Hidden-line projection**: `project_to_viewport` (OpenCascade `HLRBRep`), giving visible
   and hidden sharp edges, smooth (G1) edges and silhouettes per view. Proved, in haecceity
   (`hlr::project`). It is checked against OpenCascade's drawings of the 100-part corpus. Most
   remaining differences are OpenCascade's own approximations of projected curves.
2. **Section views**: draftwright's section A–A, done without a boolean. Proved, in haecceity
   (`hlr::project_section`, `hlr::section`). The cut outlines agree on 95 of 99 parts, and the
   other four are OpenCascade boolean failures.
3. **Text geometry**: glyph outlines and extents for layout, as build123d's `Text` gives them.
   Proved, in `src/text.rs`. Every sample label in both of draftwright's fonts has
   build123d's outline within 0.1 µm, its bounding box within 0.05 µm and its area
   (`tests/text.rs`, recorded by `tools/capture_text.py`).
4. **Output**: SVG, DXF and PDF as draftwright writes them (build123d's exporters, ezdxf,
   svglib and reportlab). Proved, in `src/output.rs`, using the `dxf` and `pdf-writer` crates.
   - Three real draftwright drawings were written from the same content
     (`tools/capture_output.py`).
   - Every SVG and DXF layer matches draftwright's: the same lines within 1 µm both ways
     and the same styles (`tests/output.rs`).
   - The PDFs render to the same ink within a pixel at 200 dpi, and their searchable text is
     identical (`tools/compare_pdf.py`).
   - Each label is laid as invisible text over its ink, in the PDF and the SVG, and reads back
     as the label, callout symbols included (`⌀6 ↧ 5`): each character has its own CID in the
     PDF's ToUnicode map, also those the bundled font lacks (`tests/output.rs`). draftwright's
     PDF text has `ø` and spaces there, and its SVG has no text.
   - Not yet done: PNG, font subsetting (the PDFs embed whole fonts) and DXF table head
     handles (ezdxf adds them on audit).

## Using it

A drawing is three JSON documents: the Analysis, the Spec and the Plan. Five stateless
operations work on them (`docs/model.md`, ADR R1).

On the command line, `analyse`, `plan`, `render` and `review` take the part's features from
Rust quiddity's recognition, run on the part (`src/bridge/native.rs`); the Analysis names it
(`recognition.recogniser`, `quiddity-rust <version> (quiddity-rust/recognition/1)`).
`--features` passes Python quiddity's recognition JSON instead, written once with
`tools/recognise.py`; `--no-features` draws with none (no feature dimensions, sections or
lint):

```
draftwright analyse part.step                          # the Analysis, features from Rust quiddity
draftwright plan part.step --spec part.drawing.json    # the Plan, every choice with its basis
draftwright edit part.drawing.json ops.json            # apply edits to a Spec
draftwright render part.step --spec part.drawing.json -o out
                                                       # out.svg, out.dxf, out.pdf
draftwright review part.step --spec part.drawing.json  # the drawing as SVG text, with problems and lint

~/repos/quiddity/.venv/bin/python tools/recognise.py part.step -o part.features.json
draftwright render part.step --features part.features.json -o out
                                                       # features from Python quiddity instead
draftwright spec out.dxf                               # the Spec a drawing (SVG, DXF, PDF) was made from
draftwright serve                                      # the same operations as a JSON-lines service
draftwright-mcp                                        # the same operations as an MCP server (stdio)
```

The service, the MCP server and the library (`api::analyse(step, None)` and the rest) are not
switched: there, no `features` still means no features, and Python's JSON is passed as
`features`. `api::analyse_from` and its siblings take `Features::Native` for the command line's
choice.

Native recognition, current limitations (details and verdicts in `docs/next-session.md`, the
d-native-holes, d-native-families and d-native-more-families entries):

- **Not every family.** Rust quiddity's document does not carry section recesses (pockets,
  channels, blind slots, circular recesses), and the reader does not take every family the
  document has (oriented chamfers, …). The Analysis lists what it lacks (`unrecognised`, with
  each family's record count on the part) and the plan reports it as a `features_not_recognised`
  warning: features of those families are not dimensioned, sectioned or linted. Use `--features`
  for them.
- **A hole lacks the face that closes it** (a drill point's cone, a blind floor), and a boss
  its free-end face, as the document carries only defining faces (rust-wrong).
- **No cross-family reconciliation.** Python's aggregate drops a boss whose faces are a turned
  step's, a blend whose faces are fillets', a chamfer on an angled step's slant and a fillet on
  a circular blind step's wall; Rust quiddity keeps both, so those radii and diameters can be
  asked for twice (rust-wrong; on GRM-05 ⌀10 is stated twice, as the envelope's and as a
  boss's).
- **A refusal is not drawn around.** When the reader refuses Rust quiddity's document (an
  unknown schema, family or record type) the command fails with the reason; `--no-features` or
  `--features` draws the part anyway.

To use the MCP server from an MCP client, register the `draftwright-mcp` binary as a stdio
server (for Claude Code: `claude mcp add draftwright -- /path/to/draftwright-mcp`).

What it draws so far (at bbd3dd7):

- **Views, sheet, scale and title block,** with draftwright's layout arithmetic; a turned part
  leaves out the view that repeats its profile, and room is reserved for the annotation bands
  its features need.
- **Section A–A** by draftwright's rule (a z-axis hole that is counterbored, spotfaced or
  blind), hatched at 45°, with its cutting-plane line and caption.
- **First dimensions:** the overall envelope, turned steps, hole callouts with their
  locations and pattern pitches, through slots, bosses, and fillet and chamfer callouts, each a
  Plan item with an ID, label and basis.
- **Lint for holes:** every recognised hole's sizes, THRU or depth, recesses and count must be
  stated on the drawing; `review` reports what is not, reading the port's own callouts.

Other feature kinds (blends, slot patterns, blind slots, …) and hole tables are not drawn yet;
the spec's notes and corner-block tables are.
The plan (`docs/plan.md`), the ADRs (`docs/adr/`), the rules catalogue that tracks what the
Python version learnt (`docs/rules/`) and the latest review (`docs/review-2026-10-08.md`) are in
`docs/`.

Feature recognition on the command line is Rust quiddity's by default, with Python quiddity's
JSON (`--features`) for what it does not cover yet (above).

To check a change: `DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus
DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`, `cargo fmt --check` and `cargo clippy
--all-targets --release -- -D warnings` (as CI does, `.github/workflows/ci.yml`). With
`DRAFTWRIGHT_CORPUS_REQUIRED=1` a missing corpus fails every corpus-backed test.

## Taking a new haecceity

haecceity and Rust quiddity come by git from `pzfreo/quiddity-rust` (`Cargo.toml`, branch
`prototype/rust-port`) and are pinned to one commit in `Cargo.lock`. To move the pin to what is
pushed on that branch:

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
     `known_bands`, `known_dimensions`, `known_lint` and `known_placement` (ctc04 is refused
     today on one face's area; 879dcf7 updated its message).
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
