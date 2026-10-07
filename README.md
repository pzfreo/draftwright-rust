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
   - Not yet done: PNG, font subsetting (the PDFs embed whole fonts) and DXF table head
     handles (ezdxf adds them on audit).

## Using it

A drawing is three JSON documents: the Analysis, the Spec and the Plan. Five stateless
operations work on them (`docs/model.md`, ADR R1):

```
draftwright analyse part.step                          # the Analysis
draftwright plan part.step --spec part.drawing.json    # the Plan, every choice with its basis
draftwright edit part.drawing.json ops.json            # apply edits to a Spec
draftwright render part.step --spec part.drawing.json -o out   # out.svg, out.dxf, out.pdf
draftwright spec out.pdf                               # the Spec a drawing was made from
```

It covers the views, sheet, scale and title block so far; dimensions and annotations come next.
The plan, the ADRs (`docs/adr/`) and the rules catalogue that tracks what the Python version
learnt (`docs/rules/`) are in `docs/`.

Feature recognition is bridged from Python quiddity's JSON results until the Rust quiddity
covers the families and the aggregate draftwright needs.

Licensed AGPL-3.0, as draftwright is; haecceity and quiddity-rust are Apache-2.0. The fonts in
`fonts/` are IBM Plex, vendored from draftwright under the SIL Open Font License 1.1 (the
`LICENSE-*-OFL.txt` files beside them).
