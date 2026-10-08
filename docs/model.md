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
  "schema": "draftwright.analysis/3",
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
  patterns. THRU is the feature's fact (`bottom: through`). A hole with both a counterbore and a
  spotface states both (`⌴ ⌀11 ↧ 6.4 ⌴ ⌀20 ↧ 1`, counterbore first), where draftwright prints only
  the counterbore and loses the spotface its own lint requires.
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
reported.

`placed` is the item's ink on the page (lines, arrowheads, the label's baseline origin, rotation
and box); renderers draw it and the terms, and nothing else. Placement is simple: lanes outward
from each view (draftwright's strips: below, and above the plan; right, and left of the plan),
sizes nearest, locations by distance, overall extents, then callouts with 45° leaders. The side
view's bands are reserved from the lanes its dimensions take (`dims::lanes`, `Demand::lanes`,
`layout::Strips::reserve`); the front's and the plan's are still draftwright's estimates. Every
item the part would have is placed before the spec chooses what is drawn, so omitting or adding
one moves no other (ARCH0003-2). Then labels settle off other items' ink
(`dims::placement::settle`, a first part of ADR2-10's bounded local solve): a linear label
slides along its line, or moves to the other end of a span too short for it; a callout's leader
may turn to the left, its tip moving to the rim's left 45° point so the leader stays radial. A
moved label stays on the sheet and off the views and the title block. Whatever then collides is
a problem on the item's ID (`annotation_overlap`, `annotation_ink_overlap`,
`annotation_over_view`, `annotation_over_title_block`, `annotation_off_sheet`, and the info
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
