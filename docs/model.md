# The draftwright model: three documents and five operations

This note describes how a drawing is described, edited and produced in draftwright-rust. It
replaces the Python implementation's authoring DSL (the fluent `Sheet` facade, the script
emitter and the deferred-intent compiler) with a small, stateless API. A command line, a
JSON-lines service and an MCP server can all sit thinly on top of it. The decision and the
reasons for it are recorded in [ADR R1](adr/R1-document-model.md); this note is the reference.

## Why

- **For an LLM, an API works as well as a program, at far less cost.** In the Python version,
  changing one callout means editing a generated script, running it and resubmitting it whole.
  pzfreo/specify-core shows the alternative: small stateless calls over plain JSON, with
  explicit IDs, defaults that explain themselves, and errors that say how to fix them.
- **The cost of the old approach was real.** In the Python version:
  - about 12.5k lines are authoring surface and about 35k compile and orchestrate the result;
  - a script emitter has to round-trip the whole DSL;
  - finalising snapshots and restores about ten collections in a hand-rolled transaction;
  - an auto path and a replay path are kept in step by a shared sequence of thirty passes.
- **The part worth keeping is the narrow waist.** Features become a plan of dimensions and
  annotations, and renderers draw only from that plan. That boundary fixed a real class of bugs
  (#921), and an API needs it just as much: the plan is what a caller inspects and edits by ID.

## The three documents

All three are JSON, versioned by a `schema` field, and deterministic. The same inputs give the
same bytes.

### Analysis: what the part is

`analyse(step)` produces it. It can always be regenerated from the STEP file, and it is never
edited.

```json
{
  "schema": "draftwright.analysis/7",
  "binding": { "source_sha256": "…", "part": "10127", "face_count": 27, "loader": "haecceity 0.1" },
  "frame": { "origin": [0, 0, 0], "axes": [[1, 0, 0], [0, 1, 0], [0, 0, 1]] },
  "envelope": { "min": [0, 0, 0], "max": [28.8, 32.3, 44.7] },
  "features": [
    { "id": "f:hole:3", "kind": "hole", "faces": [12, 13],
      "parameters": { "diameter": 8.1, "depth": "through" },
      "fingerprint": { "…": "quiddity's revision-stable signature, see Rebinding" } }
  ],
  "levels": [28.288, 32.368]
}
```

- **Levels** (`draftwright.analysis/3`) are the heights of the part's horizontal face levels,
  read from the geometry as quiddity's `step_level_records` reads them but in the part's own
  frame: what a prismatic part's height ladder dimensions. Older analyses read with none.
- **Level supports and risers** (`draftwright.analysis/4`): each level's support (its height and
  the x and y extents of its faces, per solid) and the step risers between the levels (quiddity's
  `FaceLevel` and `RiserEvidence`, read with Rust quiddity in the part's own frame, as
  draftwright's own recognition run reads them). The height ladder's witnesses stand on a level's
  support, and its shoulders' positions come from the risers (`src/dims/levels.rs`). quiddity's
  recognition document cannot give them: it reads them in its working frame, whose z need not be
  the part's. Older analyses read with none; a version 3 analysis with levels gets an info
  problem saying the step positions could not be found.
- **Unrecognised families** (`draftwright.analysis/5`, `unrecognised`): the quiddity families
  the recogniser did not give, each with how many records of it its document has on the part
  (absent when the document does not carry the family at all). Empty, and left out, for Python
  quiddity's JSON, which the bridge reads in full; for Rust quiddity's recognition
  (`model::analyse_native`, the command line's default) the families its document has that
  `bridge::native` does not read (oriented chamfers among them), and section recesses, which it
  does not carry. `plan` reports them as one `features_not_recognised` warning. `plan` reads older
  analyses with none; `render` and `review` draw only from this version.
- **Principal profiles** (`draftwright.analysis/6`, `principal_profiles`): for each solid of a
  part of several (else the whole part), its box and each principal boundary face (a plane flat
  across one axis on that box), with its inner loops' edges and its outer loop's arcs: the B-rep
  scan draftwright's lint makes for undimensionable openings and toothed outlines, made here when
  the part is analysed because lint never sees the part (`src/lint/profiles.rs`). `plan` reads
  older analyses with none; `review` judges only this version.
- **PMI** (`draftwright.analysis/7`, `pmi`): the part's AP242 semantic PMI as `src/pmi.rs`
  reads it (`read`, the report: records with their faces, and every source not read with its
  reason), each geometric tolerance and datum with where it attaches (`geometry`: a datum's site
  on each face, its plane normal or cylinder axis and its outward normal; a tolerance's common
  cylinder axis), derived from the faces as draftwright's `_datum_geometry_from_shapes` derives
  them; or `refused` with the reason when a file naming semantic PMI cannot be read. Absent when
  the file has none (presentation alone is counted). `plan` lowers it (below); older analyses
  read with none. Manufacturing requirements (threads and knurls with their structured fields
  and faces, general tolerances, surface texture and edge notes), surface labels and common
  labels are records too; `plan` reports each requirement and surface label as read but not
  drawn yet (`pmi_not_lowered`), and a common label, read as a PMI dimension, as not drawn yet
  (`pmi_dimensions_not_ported`).

- **Feature IDs** are stable within one file. They are anchored on face indices and derived
  from the feature's kind and faces, never from the order it was found in.
- **Features** come from Rust quiddity. Until a family is ported, Python quiddity's JSON fills in
  through a bridge (`src/bridge.rs`): `tools/recognise.py part.step -o r.json` writes it, and
  `analyse`, `plan`, `render` and `review` take it as `--features r.json` (an optional
  `features` path in the service and MCP requests). The bridge refuses JSON from
  another file or whose faces (count, surface type, area) are not the part's. A feature's `n` is
  its rank among its kind ordered by faces; `recognition` records the recogniser and the frame
  quiddity's parameters are in. Every quiddity record type has one home: a feature, evidence
  (`recognition.evidence`, e.g. face levels and risers) or a refusal (`recognition.refusals`,
  with quiddity's reason); a record type the bridge does not know is refused. `recognition` and
  `features` were added to `draftwright.analysis/1` as optional fields, so analyses written
  before them still read. Rust quiddity's CLI (quiddity-rust at c9ce1cd) emits, beside its
  records, fingerprints that name each feature's defining faces, but its records themselves do
  not carry their defining faces, so the bridge does not read it yet.
- **Slot patterns** are features: quiddity's derived `slot_patterns` (`SlotArray`, `SlotGrid`)
  become `slot_pattern` features, `f:slot_pattern:<n>` ranked by faces, on their member slots'
  faces, with quiddity's record (its pitch and direction, or its rows, columns, pitches, angle and
  centre) whose `slots` names the members by feature ID in quiddity's order. A member must be one
  slot record, and the members' own pattern links must agree, or the read is refused; members
  and pattern carry the pattern's count and pitch in their fingerprints. No new field: analyses
  written before read as they did (with no slot patterns).
- **What the bridge knows** (ADR3-4): it reads quiddity's document format `quiddity-recognition`
  version 3, written by a quiddity release it is checked against (`bridge::QUIDDITY_VERSIONS`:
  0.3.9 and 0.4.0.dev0, and the one the wrapper names), in local coordinates, with only the
  families, family/record-type pairs (`bridge::RECORDS`) and derived collections it knows.
  Anything else is refused with the mismatch named, never read past.

### Spec: what the author decided

The spec is the source of truth. It is small, edited, and stored beside the STEP file as
`part.drawing.json`, and inside every output so the work can be resumed. Anything it doesn't
mention takes its default.

```json
{
  "schema": "draftwright.spec/1",
  "binding": { "source_sha256": "…" },
  "sheet": { "size": "A3", "scale": "2:1", "projection": "first-angle",
             "title": { "title": "BRACKET", "number": "DWG-001", "material": "6082-T6" } },
  "views": { "front": {}, "plan": {}, "side": { "omit": true }, "iso": {},
             "sections": [ { "id": "A", "through": "f:hole:3" } ] },
  "features": { "f:hole:3": { "anchor": { "…": "fingerprint" }, "fit": "H7", "note": "REAM" } },
  "dimensions": {
    "d:f:hole:3/diameter@plan": { "tolerance": "±0.05" },
    "d:envelope/width@front": { "place": { "side": "below", "offset": 8 } },
    "d:f:pocket:1/depth@front": { "omit": true }
  },
  "notes": [ { "id": "n:1", "text": "BREAK ALL SHARP EDGES" } ]
}
```

- **Fields read so far:** `sheet` (`size`, `scale`, `title`), `views/<name>/omit`,
  `dimension_source`, `dimensions/<id>` (`omit`, `tolerance`), `features/<id>/fit` (see
  Dimensions below) and `notes` (see General notes below). `anchor`, `note` and `place` above
  are the design, not yet read: an unknown field is refused. All are optional, so a spec
  written before a field existed reads unchanged.
- **The spec stores intent, never geometry.** An entry says "this hole's fit is H7" or "this
  dimension sits 8 mm below its feature". It never says "Ø8.1 at x = 41.2". Every value is
  measured again from the part when the plan is made, so a revised part flows through by
  itself.
- **Placement is relative to an anchor** (a side, an offset, an order in a stack). Placements
  therefore move with their features, and a collision becomes an ordinary problem, never a
  silent overlap.

### Plan: what the sheet will say

`plan(analysis, spec)` produces it. The plan is the preview, and renderers draw only from it.

```json
{
  "schema": "draftwright.plan/1",
  "sheet": { "size": "A3", "scale": "2:1", "page": [420, 297] },
  "views": [ { "id": "v:front", "direction": [0, -1, 0], "at": [120, 180], "default": true,
               "basis": "front shows the most features" } ],
  "dimensions": [
    { "id": "d:f:hole:3/diameter@plan", "view": "v:plan", "value": 8.1, "label": "ø8.1 H7",
      "from_spec": ["fit"], "default": false, "basis": "a hole's size is called out where it is seen end-on",
      "rebind": "carried" }
  ],
  "problems": [
    { "id": "p:overlap:d:…", "code": "annotation_overlap", "severity": "warning",
      "message": "ø8.1 H7 overlaps the plan view's outline",
      "subjects": ["d:f:hole:3/diameter@plan"],
      "fix": { "op": "set", "path": "dimensions/d:f:hole:3/diameter@plan/place/side", "value": "left" } }
  ]
}
```

- **Every item has a stable ID**, its resolved value, whether it is a default, and a `basis`
  sentence saying why it is there.
- **Every problem names its subjects** and, where one exists, an edit that would fix it.
- **Nothing the plan approves vanishes silently.** An item that can't be drawn is reported
  against its ID, never dropped. This is carried over from the Python version's ADR 4
  invariant 9.
- **The sheet and scale the spec leaves open are re-planned when the layout fails** (ISSUE-1299-a,
  `model::plan_recovered`, which every caller of the library is given): a plan with a hard
  layout finding (ink off the sheet, overlapping labels, a line through a label, an overflowing
  title field, views that do not fit) or hole locations too close to read is planned again on
  draftwright's bounded ladder (a scale each way, or two larger, on its sheet; then each larger
  sheet), and the first valid, complete try that lays no more ink over the views is taken
  (`layout_replanned`, info, and the sheet's basis say so); none found keeps the first plan and
  says so (`layout_recovery_exhausted`). A scale the spec names is never re-planned and a sheet
  it names never left. Nothing is clipped.
- **A hole item with no place on the sheet is dropped and escalated** (the maintainer's decision
  of 2026-10-09, draftwright's `_prepare_hole_table_run`; `src/hole_table.rs`, applied to every
  plan the recovery judges by `model::plan_escalated`). A hole callout or hole location that
  placement put off the sheet keeps its ID in the plan, unplaced, with a problem
  (`callout_dropped`, `location_ref_dropped`, or `off_axis_location_dropped` at info for a hole
  the plan view does not see end-on); every such drop is a required loss the recovery tries to
  repair. A dropped callout or plan-view location escalates as draftwright's does: when the plan
  view has at least 16 holes outside a pattern, a hole table (`hole_table`: one row per hole,
  `TAG | ⌀ | DEPTH | X | Y`, in up to four side-by-side blocks towards the top right) replaces
  their plain callouts and locations, each row with a balloon (`balloons`: a tagged ring leadered
  from the hole); a dropped pattern callout in the plan view gets one `n×TAG` balloon that states
  nothing, so its drop stays. The table is kept only whole, every row's balloon placed; else it
  is withdrawn and the drops stay reported (`table_dropped`, `balloon_dropped`). The drawing
  records each row as an annotation claiming what its cells state, so lint credits the rows as
  it credits callouts, never reading the plan.

### Dimensions (first cut)

`src/dims/` plans the dimensions draftwright's defaults would state for the families ported
so far, each a Plan item `{id, kind, view, value, label, terms, measurements, default, from_spec,
basis, placed}`:

- **Overall extents:** `d:envelope/width.length@front`, `depth.length@side`,
  `height.length@front` for a prismatic part (each falling back to the plan or side view when its
  view is omitted). A part turned whole states `od.diameter` (printed `ø…`) and its length along
  the axis instead, in either profile view; the diameter is drawn across whichever radial axis
  that view shows. A view that looks along a dimension never carries it.
- **Hole callouts** in draftwright's grammar (`4× ⌀10 THRU ⌴ ⌀16 ↧ 5 (2×2)`), in the view that
  looks along the hole. Identical holes drilled the same way are grouped and counted; quiddity's
  hole patterns (kept by the bridge in `recognition.patterns` as `f:hole_pattern:<n>`) are called
  out as grids `(r×c)` or bolt circles `EQ SP ON øD BC` (five or more members), or as linear
  patterns. THRU is the feature's fact (`bottom: through`). A spotface is stated with ASME
  Y14.5-2018's spotface symbol, the counterbore symbol with SF inside (`⌴SF` in the text, drawn as
  one symbol with the font's `SF` in the box), where draftwright prints the counterbore's `⌴`. A
  hole with both a counterbore and a spotface states both (`⌴ ⌀11 ↧ 6.4 ⌴SF ⌀20 ↧ 1`, counterbore
  first), where draftwright prints only the counterbore and loses the spotface its own lint
  requires.
- **Locations** (`location.length.<axis>`) from the part's box, in the hole's view, one
  dimension per distance; patterns by their centre (or a linear pattern by its first member), with
  their pitches (`grid_pitch.length.row|col`, `pitch.length`). A hole or bolt circle on a turned
  part's axis is not located across it.

- **Slot patterns** (`src/dims/slots.rs`), as draftwright's `render_slot_patterns`: one
  `N× SLOT W × L` callout, an obround's `2× R`, the pitch (`2× 25`) or the grid's column and row
  pitches (quiddity's), and the centre located on both axes (`location_slot_pattern.length.<axis>`),
  in the view that looks through the slots; the members are not dimensioned again. A pattern whose
  slots do not stand as quiddity describes them is reported and its slots dimensioned singly.

- **Fillets and chamfers** (`src/dims/edges.rs`): `R3` and `C1` or `4.5 × 42.1°` callouts, equal
  ones counted (`4× R3`), where the edge is seen end-on, or in the profile when quiddity marks
  them turned. A record whose sizes or axis cannot be read is reported (`feature_not_dimensioned`).
  Oriented chamfers (always oblique in quiddity) are left undimensioned, as in draftwright (#1365).

- **Tolerances and fits** (`src/dims/tolerance.rs`; ARCH0011-1: a tolerance or fit belongs to a
  dimension) come only from the spec, never from the part (ADR5-D2), and reach the label
  verbatim, as draftwright prints them:
  - `dimensions/<id>/tolerance` is `"±0.05"` or `"+0.1 -0.02"` (an upper and a lower deviation,
    both magnitudes, as draftwright's `.tolerance(t)` and `.tolerance(lo, hi)`). It is printed
    after the term that states the value (`304.8 ±0.02`, `4× ⌀7.1 ±0.03 THRU …`, `HEX 100 ±0.2
    A/F`), with its authored digits and at least the drawn precision of one decimal (`±1` prints
    `±1.0`): an authored magnitude is never rounded into another requirement. Any other form is
    refused naming the dimension (`"+-0.05"` reads as `"±0.05"`).
  - `features/<id>/fit` is an ISO 286 class from draftwright's table (`fits.py`): on a hole an
    upper-case hole class (H, G, F), on a cylindrical boss a lower-case shaft class (h, g, f, js,
    k, n, p), grades 5 to 11, for a nominal size up to 250 mm. It is printed after the diameter
    of the callout that states the feature's size (`2× ⌀6.3 H7 ↧ 12.7`, `ø44.5 h6`), as the class
    code. A fit on any other feature, on a hole pattern (set it on its holes), on a feature the
    part does not have, of the wrong case (`"h7" reads as "H7"`) or outside the table is
    refused; so is a size given both a tolerance and a fit.
  - The item lists what the spec gave it in `from_spec` (`tolerance`, `fit`) and is not a
    default. A band is withheld, with a `tolerance_withheld` warning on the item's ID, when it
    is a band of nothing (`±0`: ARCH0019-2; draftwright prints `±0.0`) or when an `n×` callout's
    members do not all have the same fit (ARCH0019-3; draftwright splits such a group of bosses
    instead). A tolerance on a dimension the plan leaves out, or a fit on a feature no planned
    callout states, is a `tolerance_not_shown` warning on its ID. Across a revision, feature
    entries are kept inactive like the other feature-keyed entries.

Each family is a planner module in `src/dims/` (`envelope`, `turned`, `holes`, `slots`, `bosses`,
`edges`), listed in `dims::PLANNERS` with the feature kinds it dimensions. Every other recognised
feature (blends, turned steps, …) gets one info problem, `dimensions_not_ported`, on its
ID: draftwright dimensions those kinds and the port does not yet, so this is a gap in the port,
not in the part. Record types draftwright itself leaves undimensioned (freeform surfaces,
thin-wall and sheet-metal bodies, …; `detect.py`'s unconsumed and orchestrated records) are not
reported, except an angled step: draftwright cannot dimension one, so it gets a warning
`angled_step_requirement_unsupported` on its ID (ISSUE-1247-a; lint's code and wording).

`placed` is the item's ink on the page (lines, arrowheads, the label's baseline origin, rotation
and box); renderers draw it and the terms, and nothing else. Placement is simple: lanes outward
from each view (draftwright's strips: below, and above the plan; right, and left of the plan),
sizes nearest, locations by distance, overall extents, then callouts with 45° leaders. The side
view's bands are reserved from the lanes its dimensions take (`dims::lanes`, `Demand::lanes`,
`layout::Strips::reserve`); the front's and the plan's are still draftwright's estimates. Every
item the part would have is placed before the spec chooses what is drawn, so omitting or adding
one moves no other (ARCH0003-2). A strip that cannot hold its linear dimensions on the page
passes the lanes it has no room for to the strip across the view (`dims::placement::fall_through`,
ISSUE-1215-e), an overall extent claiming the free lanes first; one that fits on neither side keeps
its lane. Then labels settle off other items' ink
(`dims::placement::settle`, a first part of ADR2-10's bounded local solve): a linear label
slides along its line, or moves to the other end of a span too short for it; a callout's leader
may turn to the left, its tip moving to the rim's left 45° point so the leader stays radial; a
callout whose lane does not fit the page (off the sheet, or its leader across another view) goes
out beside its view, as draftwright's plan callouts do. A moved label stays on the sheet and off
the views and the title block. Whatever then collides is
a problem on the item's ID (`annotation_overlap`, `annotation_ink_overlap`,
`annotation_over_view`, `annotation_line_over_view` for a line across a view not its own,
`annotation_over_title_block`, `annotation_off_sheet`, and the info
`annotation_outside_area` for ink past its view's area and bands); an item whose view is omitted
stays unplaced and is reported (`view_plan_incomplete`, naming every view that could carry it and
why none does). Placement problems on the corpus are a ratchet (`tests/placement.rs`,
`tests/fixtures/known_placement.json`).

What the collision checks cover, and what they do not: label boxes against label boxes, a
dimension's lines against other labels' boxes, labels against view outlines and the title block,
and all ink against the drawing area. **Not checked yet:** extension, dimension and leader lines
crossing another view's outline or other dimensions' lines (on dense sheets such as ftc06 the
front view's right-hand lanes run into the side view and callout leaders cross dimension lines,
unreported). Labels are measured and drawn with one bundled font (`dims::FONT`), so the boxes
are the ink. Each label's text run is the plan's string, symbols included, laid invisibly over
the ink in the PDF and the SVG so it can be searched and copied, and read by lint.

### General notes

`src/notes.rs` reads the spec's `notes`, a list of `{id, text}` in the order they are numbered,
and draws them as draftwright draws its document notes (`render_document_notes`):

- **Refused** (by `plan` and `edit`, with the next free ID as the choice): an ID not of the form
  `n:<k>` (`k` a whole number from 1, no leading zeros), two notes with one ID, a note whose text
  is empty or only whitespace. An absent or empty list is not serialised, so a spec without
  notes reads and writes as before.
- **Plan items** (`notes`): `{id, text, number, rows, default: false, basis, placed}`. `number`
  is the note's place in the list (draftwright numbers by position, so the IDs stay while the
  numbers follow the order); `rows` are `document_note_rows`' (`1  BREAK ALL SHARP EDGES`, wrapped
  at 48 characters as Python's `textwrap` wraps them, never splitting a word, continuation rows
  indented three spaces); `placed` is each row's ink centre and the note's cells. The block
  (`notes_block`: the `GENERAL NOTES` heading, its ink centre and the table's box) is drawn
  ruled, one row of 3 mm text and 2 mm padding above and below for the heading and for each
  note row, in IBM Plex Mono.
- **Placement.** The block's measured size is reserved when the sheet is chosen, top right
  (draftwright's `layout_required_tables`), so a long list moves the sheet or scale on; then it
  is placed after the dimensions by `fit_box` towards the top right, with 6 mm of breathing room
  and then at the 2 mm text padding, clear of the views' areas (their bands, where the
  dimension lanes run), the isometric's drawn box, every dimension's ink and the title block. A
  block no layout at a standard scale can hold is not reserved, so it never shrinks the views;
  a block that then has no place is a `notes_not_placed` error naming every note, which stays
  in the plan undrawn.
- **Render.** Each row is drawn as glyphs and is one invisible text run of the row as the plan
  states it (continuation rows without their indent), so the notes can be searched and copied
  from the PDF and the SVG; `⌀` is drawn as `ø`, as draftwright's `_font_safe_text` does.
- **Edits.** `{"op": "add", "path": "notes", "value": {"text": "DEBURR"}}` appends a note; one
  given no `id` takes the next free `n:<k>`. A note is changed or removed by setting `notes`.
- **Differences from draftwright**, with verdicts (`tests/notes.rs` compares the block with
  draftwright's on m10127 and ctc01, `tests/fixtures/notes.json` from `tools/capture_notes.py`):
  the rows, their numbering and wrapping, the block's height and its top-right corner are
  draftwright's; a continuation row keeps its indent, where draftwright's left alignment on each
  row's ink drops it, so the block is wider by the indent (rust-correct: draftwright writes the
  indent into the row and loses it in the drawing); the block keeps clear of the views' bands as
  well as the drawn ink (equivalent on the parts compared: both land in the corner the sheet
  reserved); a block no sheet can hold at a standard scale is not reserved (draftwright would let
  it drive the sheet to a computed scale; undetermined, a question in `docs/next-session.md`);
  the text layer keeps `⌀` where draftwright's has `ø` (rust-correct, as for callouts). The
  per-feature `note` and hole tables are not ported.

### Tables

`src/tables.rs` reads the spec's `tables`, a list of `{id, rows, prefer?}` placed in order, and
draws each as draftwright's `Sheet.table` draws a declared corner-block table (revision blocks,
BOMs, schedules) through `Drawing.add_table` and `_build_table`. The pieces the notes block
shares with a table (text height, padding, rows, font-safe ink, what a block keeps clear of,
placement and rules) live there too; the notes block is draftwright's single-column table.

- **Refused** (by `plan` and `edit`): an ID not of the form `t:<k>` or two tables with one ID
  (with the next free ID as the choice); and, as `Sheet.table` refuses them, a table with no
  rows, a row that is a string (pointing to `notes`), rows not all of one non-zero length; and,
  beyond draftwright, a row that is not a list, a cell that is neither text nor a number (`null`,
  `true`, a list), and a `prefer` other than `tr`, `tl`, `br`, `bl` (with those as the choices).
  A number cell is printed as JSON writes it (`4`, `0.5`). An absent or empty list is not
  serialised.
- **Plan items** (`tables`): `{id, rows, default: false, basis, placed}`, `rows` the cells as
  text, row 0 the header; `placed` is each cell's ink centre (none for an empty cell, which
  draws nothing), each column's left edge and the right edge, and the table's box. Each column
  is as wide as its widest cell's ink plus 2 mm each side, at least 7.5 mm (`_table_metrics`);
  each row 7 mm (3 mm IBM Plex Mono and 2 mm padding above and below); each cell's ink centred
  in its cell; ruled at every column edge and at the top and bottom of every row.
- **Placement.** Each table's measured size is reserved towards its corner when the sheet is
  chosen, ahead of the notes block, as draftwright's `layout_required_tables` orders them (a
  reservation that only a fallback would make room for is dropped, with the notes', so it never
  shrinks the views). Each is then placed after the dimensions and the notes block, in the
  spec's order, by `fit_box` towards its `prefer` corner (default `tr`; the corner ranks the
  places, it does not exclude the others), with 6 mm of breathing room and then at the 2 mm
  padding, clear of the views' areas, the isometric's drawn box, every dimension's ink, the
  title block, the notes block and the tables before it. A table with no place is a
  `table_not_placed` error naming it (draftwright's `table_dropped`), and stays in the plan
  undrawn.
- **Render.** Each non-empty cell is drawn as glyphs and is one invisible text run of the cell
  (trimmed) in the PDF and the SVG text layer; `⌀` is drawn as `ø`.
- **Edits.** `{"op": "add", "path": "tables", "value": {"rows": [["REV", "DESCRIPTION"], ["A",
  "INITIAL RELEASE"]]}}` appends a table; one given no `id` takes the next free `t:<k>`.
- **Differences from draftwright**, with verdicts (`tests/tables.rs` compares a four-column
  revision block towards `tr`, with an empty cell, and a four-column BOM towards `bl` on m10127
  and ctc01, `tests/fixtures/tables.json` from `tools/capture_tables.py`, which declares them as
  `Sheet.build` does): rows, height, every column's width and every cell's ink centre agree to
  1e-4 mm on both parts, and an empty cell draws nothing in both; on ctc01, where the port
  chooses draftwright's sheet with and without the tables, both tables sit flush in the same
  corners; on m10127 the port's sheet already differs without them (A4 1:1 against A3 2:1,
  `tests/fixtures/known_views.json`), and the BOM's bottom-left corner is taken by the front
  view's area, so it goes to the next free place (not comparable). An unknown `prefer` is
  refused, where `fit_box` reads anything but `br`, `tl`, `tr` as `bl` (rust-correct: a typo
  would silently move the table); a non-list row and a cell that is not text or a number are
  refused, where Python's `str()` would print `None`, `True` or a list's repr (rust-correct);
  `table_not_placed` is an error, where `table_dropped` is a warning (rust-correct:
  draftwright's own `build_policy._is_required_scale_drop` treats a sourced `table_dropped`,
  which every declared table is, as a lost requirement); each cell's text run is centred on its
  ink, where draftwright's PDF overlay starts every cell's run at the column's left padding
  although the ink is centred (rust-correct: the run lies over the glyphs it stands for); the
  views' bands are kept clear as well as their ink (as for notes; equivalent where compared).
  Not ported: `block_cols` (side-by-side blocks, which only the hole table uses through
  `_wrap_rows`), `left_align_cols`, a table's `name`, and `Sheet.schedule`'s measured tables.

### Feature control frames and datum feature symbols (PMI)

`src/pmi_lowering.rs` lowers the Analysis's PMI as draftwright's `build_pmi_features` does, and
`src/dims/gdt.rs` places and draws it. Plan items (`gdt`): `{id: "pmi:<source>", kind:
"control_frame" | "datum_feature", view, side, source, faces, cells, modifiers?, default, basis,
placed}`.

- **A feature control frame** for each geometric tolerance read without blockers: its
  characteristic, magnitude (`_fmt_pmi_magnitude`), the zone's `⌀` or `S⌀`, `Ⓜ` or `Ⓛ`, the
  datum letters (repeats removed) and the all-around or all-over circle at its leader's bend. It is
  drawn in the view that sees its axis end-on (its faces' common cylinder axis, else their box's
  longest extent, else z: x → side, y → front, z → plan), below the front and the side, above
  the plan, its leader from the centre of its faces' box. Only the modifiers draftwright draws
  are drawn (all-around, all-over, the zone, the material condition); any other is a
  `pmi_not_lowered` error.
- **A datum feature symbol** for each datum whose faces are all planes, or all cylinders, on one
  principal plane or axis: in the view that sees it edge-on (x and z: front; y: side; a y-normal
  plane goes to the plan when the side view is not drawn), on the side its outward normal points
  to (a cylinder's: its site's), from a proven site on its first face. Drawn as ISO 5459 draws it:
  the filled triangle on the surface, a line along the normal, the letter in a square frame.
- **Placement**: in its strip beyond every dimension of its view there, nearest tier first,
  clear of the sheet's edge, the views, the title block, other labels and leaders; a frame with
  no clear tier tries draftwright's other strips, a datum keeps its side; one with no clear place
  keeps its first tier and its collisions are reported as the dimensions' are. Notes and tables
  keep clear of the frames.
- **Render**: strokes for the frame, the symbols and rings, the font's glyphs for letters and
  numbers (`⌀` drawn as a ring and a stroke, `S⌀` as `Sø`), and one invisible text run per
  compartment (`⌖`, `⌀0.05Ⓜ`, `A`), so the PDF and SVG text layers can be searched; lint reads
  them as annotations that claim no measurement.
- **Reported, not drawn**: a record whose reading left blockers or whose faces prove no
  attachment, or whose faces lie in several placements of its part (`pmi_not_lowered`, error); a
  source not read (`pmi_not_extracted`: error for a requirement, info for a shape aspect or
  property); a file whose PMI cannot be read (`pmi_not_read`); presentation
  (`pmi_presentation_not_read`, info); an item whose view is not drawn (`pmi_not_drawn`). PMI
  dimensions are the next section's. draftwright draws PMI only
  with `pmi="annotate"`; the port draws it whenever the part has some.

Compared item by item with draftwright on 14 AP242 files (`tests/pmi_lowering.rs`,
`tests/fixtures/known_pmi_lowering.json`).

### Toleranced and basic dimensions (PMI)

`src/dims/pmi_dimensions.rs` lowers each AP242 dimension record as draftwright's
`lower_ap242_*_tolerances`/`_nominal_*` and `render_pmi` do, from where its faces put it (the
Analysis's `geometry` on a dimension: a linear or thickness dimension's two proven `stations` and
their axis, its one witness `view`; a diameter's or radius's exact `cylinders`, a radius's site on
its arc; `src/pmi_lowering.rs`'s `dimension_geometry`, draftwright's `_proved_planar_linear` and
`_cylindrical_references`). **The rule** (draftwright's): a requirement on a quantity a recognised
feature's dimension already states annotates that dimension, which states it once: a bore whose
cylinders are a hole callout's holes (axis lines, axis, diameter) gives the callout its band
(`6× ⌀10 +0.09 -0.0 THRU`; symmetric limits as `±`, asymmetric ones as limits, a band wholly above
or below the nominal with its signs), and a callout whose holes carry different requirements is
split by requirement (a bolt circle is not: reported); an external cylinder gives its band to the
turned step, boss or turned part whose diameter it is; a plain diameter or step length is stated
by its owner. Otherwise it **stands beside** them, an ordinary planned dimension `d:pmi:<source>/
<kind>[.basic]@<view>` (a length along x, y or z in draftwright's view; a diameter or radius by a
leader where it is seen end-on), its label the record's, a basic one boxed (its label box widened
to take the frame). Every item that states a PMI dimension claims it (a measurement of feature
`pmi:<source>`); a spec's own tolerance or fit on that size (a hole's or a boss's) wins
(`pmi_not_lowered`; when the spec's is withheld, the label states neither and the report says
so), and an omitted item's PMI is `pmi_not_drawn`. Not drawn, with the reason: reading blockers, faces in
several placements, faces that prove no witness (`pmi_not_lowered`); angles, oblique lengths,
non-principal cylinders (`pmi_dimensions_not_ported`). The open questions (supplemental geometry,
an ISO 286 class without limits) stay refused. Compared record by record on the 14 AP242 files
(`tests/pmi_dimensions.rs`, `tests/fixtures/known_pmi_dimensions.json`).

**IDs and views.** The `@<view>` in an ID is the view that carries the dimension. Omitting that
view moves the dimension to its next view and renames it (the basis says so); a spec still keyed
by the old ID is refused, with the new ID named first in the refusal's choices. Take IDs from
the latest plan.

The spec chooses the source (ADR4-5): `"dimension_source": "auto"` (the default: the defaults
less `dimensions/<id>/omit`) or `"authored"` (exactly the IDs listed in `dimensions`, a complete
set). Omitted items are listed in the Plan's `omitted` with their basis. In an edit path, the
`/` inside a dimension's ID is written `~1`, as in a JSON pointer:
`{"op": "set", "path": "dimensions/d:envelope~1width.length@front/omit", "value": true}`.

## The five operations

These are one Rust core: pure functions over the documents, deterministic, with no state between
calls.

| Operation | In | Out |
| --- | --- | --- |
| `analyse` | STEP file | Analysis |
| `plan` | Analysis, Spec | Plan (with problems); rebinds first if the spec's binding differs |
| `edit` | Spec, ops | Spec, or a validation error |
| `render` | Plan, formats, dpi | SVG, DXF and PDF by default, and PNG at `dpi` (150) when asked, each with the spec embedded |
| `review` | Plan | a rendered image, the problems and lint's findings on the drawing, for a check |

**`edit` operations** are small and addressed by path:

```json
[ { "op": "set", "path": "features/f:hole:3/fit", "value": "H7" },
  { "op": "remove", "path": "dimensions/d:f:pocket:1/depth@front/omit" },
  { "op": "add", "path": "notes", "value": { "text": "DEBURR" } } ]
```

`add` appends its value to the list at its path (only `notes` is a list).

Each operation is validated against the analysis:

- **Wrong value:** the error says what would be accepted, e.g. "Choose one of: H6, H7, H8 …".
- **Close but wrong:** "'h7' reads as 'H7'".
- **Unknown ID:** the error lists the nearest valid IDs.

A caller may equally replace the whole spec.

**Adapters.** All three call one module, `src/api.rs`: the operations typed, and `api::call`,
the same operations by name with JSON arguments. Requests carry their documents and name files by
path, so no adapter keeps state between calls. Errors are `{"code": "invalid" | "failed",
"message", "choices"?}`.

`plan`, `render` and `review` take the part either as `step` (the STEP file's path) with
`features`? (Python quiddity's recognition JSON), which they analyse, or as `analysis`: the
Analysis document that `analyse` returned, carried inline as the spec is, so the caller need not
analyse again (typed: `api::plan_analysed`, `api::render_analysed`, `api::review_analysed`). The
output is byte-identical to analysing the same STEP with the same features.

- `plan` with `analysis` reads no file: `step` may be omitted, and plans any Analysis version
  `plan` reads (`draftwright.analysis/2` or `/3`).
- `render` and `review` still need the part's geometry, so they take `step` beside `analysis`
  and read it without recognition. They take only the version this build writes
  (`draftwright.analysis/3`), and refuse, as `invalid`, an Analysis whose binding's
  `source_sha256` or `face_count` is not the STEP file's, naming both digests: one part is never
  drawn with another's analysis. `plan` given both checks them the same way.
- `analysis` beside `features` is refused (the analysis already carries the features it was made
  with), and so is an Analysis that does not parse (unknown or missing fields, not an object).

- **CLI.** `draftwright analyse part.step`, `draftwright plan part.step --spec part.drawing.json`,
  `draftwright review …` and `draftwright render … -o out` all print JSON. `--analysis a.json`
  names an Analysis file in place of `--features` (`draftwright plan --analysis a.json` needs no
  STEP file). Exit code 2 means
  invalid input; exit code 1 means a write failed and the output was left as it was.
- **JSON-lines service.** `draftwright serve` takes one request and gives one reply per line, in
  the shape of specify-core's `serve`: `{"id": 1, "op": "plan", "step": "part.step", "spec": {…}}`
  gives `{"id": 1, "ok": true, "result": …}` or `{"id": 1, "ok": false, "error": {…}}`, and
  `{"op": "hello"}` gives the protocol version and the operations. A malformed line is answered,
  and the service carries on.
- **MCP server.** `draftwright-mcp` speaks MCP over stdio (JSON-RPC 2.0, one message per line).
  Its tools map one-to-one onto the operations, each with an input schema and a description
  written for an LLM; `plan`, `render` and `review` take `analysis` as an object, and `plan`
  requires no `step`. Results are the documents as JSON text; `review` adds the drawing as SVG
  text. A refused request is a tool error (`isError`) carrying the message and the choices.

**The spec in each output** (invariant 8): in the SVG's metadata, in the PDF's document
information, and in the DXF as comments (group code 999) ahead of the first section. AutoCAD,
ezdxf and the `dxf` crate read past such comments; a re-save drops them. `draftwright spec`
reads it back from any of the three.

## IDs

| Item | Form | Stable across |
| --- | --- | --- |
| Feature | `f:<kind>:<n>`, derived from its faces | the same file |
| Dimension | `d:<feature>/<parameter>@<view>` | the same file; carried to revisions through its feature |
| View | `v:<name>` | always |
| Note, table, section | `n:<k>`, `t:<k>`, `s:<letter>` | always (set in the spec) |
| PMI frame or symbol | `pmi:<source>` (the record's `source_id`, `geometric_tolerance:#<instance>`) | the same file |
| Problem | `p:<code>:<subject>` | the same plan |

Parameter names follow the Python version's role vocabulary (`role.kind[.discriminator]`). A
name that matches more than one measurement is refused rather than guessed (Python ADR 4
invariant 6).

## Rebinding: when the STEP changes

Feature IDs are stable only within one file. When the spec's `source_sha256` differs from the
analysis, `plan` first rebinds the spec using quiddity's `correspond(old, new)` (see
quiddity-rust's `docs/correspondence.md`). Every spec entry keyed to a feature carries that
feature's fingerprint, so the old STEP file is not needed. Each entry then becomes one of these:

- **carried:** it applies unchanged;
- **adapted:** it applies, and the plan says what changed in the feature (e.g. "ø6.6 → ø8.4,
  fit H7 kept"). A choice that no longer fits, such as a fit for the old size, becomes a problem
  with a suggested replacement;
- **ambiguous:** it is kept, and a question lists the candidates;
- **orphaned:** its feature is gone. It is kept, marked inactive and reported, so a later
  revision or a manual re-anchor can revive it;
- **new:** the feature has no entries; it takes its defaults and is listed for review.

Every plan item records which of these it is, so nothing changes without a trace.

## How the port is checked

- **The rules catalogue** (`docs/rules/`) lists every rule the Python version learnt, from its
  ADRs, its lint rules and its issue regressions. Each rule is classified and tracked as pending,
  ported, translated or retired.
- **A differential comparison** checks Rust's plan against Python's sheets, fact by fact
  (measurement identities, values, labels, views, findings), with a verdict for each
  difference. Layout is judged by rules, not by matching Python's positions.
- **The ported lint rules** (`src/lint/`) run on the rendered drawing, independently of the plan:
  they read only the Analysis and what the drawing prints and claims (its annotations), so they
  judge Python's sheets and Rust's alike. A guard test fails if lint names the Plan or the Spec.
  `review` replies with lint's findings (`LintIssue`: code, severity, message, subjects, the
  requirements, evidence and a fix). `tests/lint.rs` runs them on Python's captured sheets and
  lists every difference from Python's own lint, with a verdict, in
  `tests/fixtures/known_lint.json`.
