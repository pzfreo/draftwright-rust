# draftwright-rust

A pure-Rust port of [draftwright](https://github.com/pzfreo/draftwright): automated,
fully-annotated engineering drawings from STEP B-rep. No OpenCascade: geometry comes from the
[haecceity](https://github.com/pzfreo/quiddity-rust/tree/prototype/rust-port/crates/haecceity)
kernel, shared with the Rust port of quiddity.

**Status: starting.** As with quiddity, the hard kernel pieces are proved first, each against
what the Python implementation produces:

1. **Hidden-line projection** — `project_to_viewport` (OpenCascade `HLRBRep`): visible and
   hidden sharp edges, smooth (G1) edges and silhouettes per view. In haecceity.
2. **Section views** — the solid cut by a plane, its section loops and hatch regions. In
   haecceity.
3. **Text geometry** — glyph outlines and extents for layout (build123d `Text`).
4. **Output** — PDF, SVG and DXF (reportlab and ezdxf in Python).

Feature recognition is bridged from Python quiddity's JSON results until the Rust quiddity
covers the families and the aggregate draftwright needs.

Licensed AGPL-3.0, as draftwright is; haecceity and quiddity-rust are Apache-2.0.
