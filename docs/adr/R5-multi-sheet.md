# ADR R5: Multi-sheet drawings

**Status:** Proposed (2026-10-12). The allocation policy in the first section is **Accepted
(maintainer, 2026-10-11)**. The rest of this record, the technical design, needs the
maintainer's sign-off.

**Builds on:** R1 (the document model: the spec stores intent, renderers draw only from the plan,
IDs are derived), the layout recovery (ISSUE-1299-a, `model::recover`) and drop-and-escalate
(`src/hole_table.rs`, the maintainer's decision of 2026-10-09).

**Translates:** Python draftwright's `Document` (`src/draftwright/document.py`). There a document
is a set of separately authored `Sheet`s sharing one drawing number and revision, and each sheet's
title-block `sheet` cell gets `"n/m"`, or else all sheets get custom labels. Here, the planner
allocates the sheets from one spec. The cell reads `n OF m`, as the policy says, not `n/m`.

## The agreed policy (maintainer's decision, 2026-10-11, verbatim)

> # Sheet allocation policy (agreed by the maintainer, 2026-10-11)
>
> ## Default allocation
> | Content | Default sheet | Rule |
> |---|---|---|
> | Title block, general notes, general tolerance, projection symbol | 1 | Notes on sheet 1 only; every sheet repeats the title block with "SHEET n OF m". |
> | Principal views (front, plan, side) and overall sizes | 1 | Principal views are never split across sheets. |
> | Datum feature symbols | where the datum feature is shown best, normally sheet 1 | Every datum a feature control frame references must be identified on sheet 1 or on the same sheet as that frame. |
> | Feature control frames | with the feature's dimension, on the view that shows the feature best, on whatever sheet that view is (ASME Y14.5 / ISO 129-1 practice) | A frame never leaves its feature. |
> | Hole table | 1, with tags on the plan view | A long table moves to its own sheet; tags stay on the views. |
> | Sections and enlarged details | the next sheet with room | Cross-referenced both ways with sheet and grid zone ("SECTION A-A, SHEET 2, ZONE C4" / "FROM SHEET 1, ZONE B3"). |
> | Dense regions | a detail view on a later sheet | Preferred to shrinking the whole sheet below the smallest legible scale. |
>
> ## Controls
> - Spec: maximum sheets, sheet size, smallest scale allowed; pin a view or item to a sheet; keep-together groups (e.g. a section with its parent view); "all GD&T on sheet 1" (forces frames onto sheet-1 views, for customers or inspectors who require it).
> - MCP and editor: "move view to sheet n" op; sheet-qualified item addresses. Editor: no sheet UI in these stages (the maintainer will redesign the editor after them); keep the editor building and working on sheet 1 only.
> - Checks (problems): frame references a datum not identified on sheet 1 or the frame's sheet; section/detail missing its cross-reference; a sheet below the smallest scale allowed.

## Decision (technical design)

### 1. The spec

Everything is additive and absent by default. `draftwright.spec/2` is kept: an absent field
serialises as nothing, so an existing spec's bytes are unchanged. A build without sheets refuses a
spec that has them (`deny_unknown_fields`), as it should.

```json
{
  "sheet":  { "size": "A3", "scale": "1:1", "title": { "number": "DW-100" } },
  "sheets": {
    "max": 3,
    "smallest_scale": "1:2",
    "gdt_on_sheet_1": true,
    "keep_together": [["front", "section_aa"]],
    "hole_table": 2
  },
  "views":       { "section_aa": { "sheet": 2 }, "iso": { "sheet": 2 } },
  "dimensions":  { "d:f:hole:3/diameter": { "sheet": 1 } },
  "annotations": { "pmi:geometric_tolerance.position:f12,13:ref=A,B": { "sheet": 1 } },
  "tables":      [ { "id": "t:1", "rows": [], "sheet": 2 } ]
}
```

| Field | Meaning | Absent |
| --- | --- | --- |
| `sheets.max` | The number of sheets the planner may use, 1 to 9 | 1: today's planner, unchanged |
| `sheet.size` | One size for every sheet of the document (existing field) | As today: the recovery's ladder |
| `sheets.smallest_scale` | The smallest ISO 5455 scale sheet 1's principal views may take | No floor |
| `views.<name>.sheet` | Pins the view to a sheet. A principal view only to 1 | The planner decides |
| `dimensions.<id>.sheet`, `annotations.<id>.sheet` | Pins the item to a sheet: its carrying view must be on that sheet. Refused (with the views that could carry it) when no view there can | The view that shows it best |
| `tables[k].sheet`, `sheets.hole_table` | Pins a corner table or the hole table | Sheet 1; a long hole table moves |
| `sheets.keep_together` | Groups of view names (and `hole_table`) that share one sheet | None |
| `sheets.gdt_on_sheet_1` | Every frame and datum symbol goes on a sheet-1 view that shows its feature | `false` |

`edit` validates each of these as it validates every path. A sheet number above `sheets.max` is
refused, and the choices are `1..max`. Its `fix` also offers `set sheets/max`. A pin that goes
against a keep-together group is refused, naming the group. A pin is an *intent*, never a page
coordinate, so R1 invariant 1 holds.

### 2. The plan

- `plan.sheet` stays as it is and is always **sheet 1**, with ID `sheet`. Every existing reader,
  the editor included, keeps reading it.
- `plan.sheets: Vec<PlannedSheet>` holds sheets 2..m, with IDs `sheet:2` … `sheet:m`. It is
  omitted when empty.
- `PlannedSheet` gains `number: u32` (omitted when 1) and `zones: Option<Zones>` (the grid,
  section 5, present only when m > 1).
- `PlannedView`, `PlannedTable` and `HoleTable` gain `sheet: u32`, omitted when it is 1.
  Dimensions, frames, datum symbols, labels and balloons take the sheet of their carrying view.
  Notes and drawing-wide requirements are on sheet 1. `items::Item` gains `sheet`, derived the
  same way, so nothing is stored twice.
- `PlannedView` gains `scale_note: Option<String>` (for example `"SCALE 2:1"`) when its scale is
  not its sheet's, and `xref: Option<CrossRef>` (section 5).
- **IDs do not change** when an item moves between sheets, just as a dimension's ID does not
  change when it moves between views (docs/model.md, IDs). R1 invariant 5 holds.
- **Sessions and deltas:** `PlanDelta.collections` gains `sheets`. A view that moves lists the
  view and each of its items as `changed`, because their `sheet` field changed. `removed` and
  `added` are unaffected. No new delta kind is needed.

### 3. Allocation (`src/sheets.rs`, new; called from `model::plan_recovered`)

1. **Today's plan first.** `plan_escalated` + `recover` run exactly as now, on one sheet, with
   every view, at the spec's size. With `sheets.max` absent or 1, that plan is final. This is the
   migration guarantee in section 9. It is also final when it is clean (no hard finding, no
   required drop), its scale is at or above `smallest_scale`, and no pin or `hole_table` names a
   later sheet.
2. **Otherwise, move content off sheet 1** one candidate at a time, in this overflow order:
   the isometric, then section views, then a hole table that does not fit (a *long* table: one
   that sheet 1 cannot place whole), then (stream I5) detail views of dense regions. Pinned
   content is placed first and is never a candidate. A keep-together group moves as one. A
   principal view never moves.
3. **Each move re-plans sheet 1** through the same `recover`, with the moved views omitted from
   sheet 1. The recovery's paper ladder is unchanged: a larger paper still comes before a further
   sheet, unless `sheet.size` fixes the paper. A further sheet is preferred **only to a scale
   below `smallest_scale`, or to a recovery that ran out of options**. The first move that gives
   a clean sheet 1 at or above the floor is taken. Moves are bounded by
   `(sheets.max − 1) × candidates`, at most 8 re-plans, and each try is recorded in the sheet's
   `basis`, as `layout_replanned` records them now.
4. **The moved content is packed** onto sheets 2..m in order, each sheet filled before the next
   ("the next sheet with room"). `layout::plan_sheet(views, page, scale)` (new) packs the moved
   view blocks, each with its annotation bands, as a row that wraps, with the title block at the
   lower right and no notes block. A section keeps its parent view's scale. A view that cannot
   take its parent's scale takes the largest one that fits and gets a `scale_note`. A long hole
   table gets a sheet of its own; its balloons stay on the plan view.
5. **The datum rule.** A view is not taken as a candidate if moving it would leave a frame
   referencing a datum that is identified neither on sheet 1 nor on the frame's own sheet. When
   every remaining candidate breaks the rule, the move is still made and the check in section 7
   reports it. No datum symbol is ever repeated.
6. **`gdt_on_sheet_1`** limits the carrier choice for frames and datum symbols
   (`dims::gdt`'s carrier) to sheet-1 views that show the feature. A frame that no sheet-1 view
   can carry stays on its best view and gets a `gdt_not_on_sheet_1` warning. A frame never leaves
   its feature.
7. **Placement checks run per sheet.** Collision and off-sheet audits
   (`dims::placement`, `dims::audit`, `dims::gdt`, `hole_table::escalate`) compare only items on
   the same sheet, against that sheet's page and title block.

### 4. The title block on every sheet

Every sheet draws the full title block (`title_block::cells`) with the same title values.
`format` is the shared size. The projection symbol and the general notes are drawn on sheet 1
only. When m > 1:

- the `SHEET` cell holds `n OF m`. Measured in the title-block face at 3 mm, `1 OF 2` is 7.60 mm
  wide and `10 OF 12` is 10.84 mm. Today's cell is 6 % of the block: 7.2 mm on A4 and 9.0 mm on
  A3. In a multi-sheet document, therefore, the bottom row widens the SHEET cell to 12 % and
  narrows APPROVED BY from 35 % to 29 %. The single-sheet grid does not change. This is a
  judgement call (see the open questions);
- a spec `sheet.title.sheet` value is replaced by `n OF m`, with a `sheet_label_replaced` info
  problem. Python's custom page labels are not carried.

### 5. Grid zones and cross-references

- **Zones (ISO 5457):** in a multi-sheet document only, each sheet's border is divided into an
  even number of fields of about 50 mm: A4 6×4, A3 8×6, A2 12×8, A1 16×12, A0 24×16. Numbers run
  left to right and letters run top to bottom (no I or O). Ticks and labels are drawn in the
  10 mm margin outside the border, so the drawable area and every layout result are unchanged.
  A point's zone is `<letter><number>` (`sheets::zone(page, p)`).
- **Cross-references:** they are needed only when a section, or a detail view, is on a different
  sheet from the view that marks it.
  - On the marking view, beside the cutting-plane letter, the text reads `SECTION A–A, SHEET 2,
    ZONE C4`, the zone being that of the section's label.
  - Under the section's label, the text reads `FROM SHEET 1, ZONE B3`, the zone being that of
    the cutting-plane letter.
  - Both are `CrossRef { sheet, zone, at }` on the `PlannedView` and the `Cut`. They are placed
    and collision-checked like any label, and drawn as text runs so that lint and PDF search can
    read them. A view on the same sheet is drawn as today.

### 6. Rendering

- `model::render` keeps its signature and draws sheet 1. `model::render_sheets` (new) returns
  `Vec<Drawing>`, one per sheet. Every `Drawing` embeds the whole spec (R1 invariant 8).
- **SVG:** one file per sheet. The sheet-1 file keeps its name; sheet n is `<stem>.sheet<n>.svg`.
  `api::rendered` adds the entries `sheet<n>.svg`, and `api::write` writes them like any format.
- **PDF:** one file, one page per sheet (`Drawing::pdf_pages(&[Drawing], fonts)`). With one
  sheet it is `pdf()`, byte for byte.
- **DXF:** one file per sheet, `<stem>.sheet<n>.dxf`, each with the spec comments. Paperspace
  layouts in one file were considered and not chosen, because the existing writer and its
  readers (ezdxf, the `dxf` crate) are model-space only.
- **PNG:** one per sheet, `<stem>.sheet<n>.png`. `review` reviews every sheet. `view_sheet` and
  `render` take `sheet` (default 1).

### 7. Checks (new problem codes, `src/sheets.rs::check`)

| Code | Severity | Subjects / refs | Fix |
| --- | --- | --- | --- |
| `datum_not_identified` | error | the frame / `ref` the missing letter | `set views/<datum's view>/sheet` to the frame's sheet, or `set annotations/<frame>/sheet 1` |
| `cross_reference_missing` | error | the section or detail view, its marking view | none (the planner always writes it; this guards a drawing that lost it). Also a lint structural check over each sheet's drawn text |
| `sheet_below_smallest_scale` | warning | `sheet` | `set sheets/max` to m+1 when below 9, else none |
| `gdt_not_on_sheet_1` | warning | the frame | none |
| `sheet_label_replaced` | info | `sheet`, ref `title_block/sheet` | `remove sheet/title/sheet` |

`sheet_below_smallest_scale` is also raised on a single-sheet plan that sets `smallest_scale`.

### 8. MCP and the library

- **Sheet-qualified addresses:** `sheet:<n>/<id>` is accepted anywhere an item ID is
  (`get_item`, `item_options`, overlays and `list_items` `near`). If the item is on another
  sheet, the request is refused with "`<id>` is on sheet 2", and the choices include the correct
  address. Bare IDs keep working: IDs are unique across sheets.
- **Move view to sheet n:** this is the edit `{"op": "set", "path": "views/<name>/sheet",
  "value": n}`. RFC 6902 already has a `move` op with other meaning, so no new op kind is added.
  `item_options` for a view lists it as the operation `move_to_sheet`, with choices `1..max` and
  the keep-together consequences.
- **Results:** `list_items`, `get_item` and delta items carry `sheet`. It is omitted when the plan
  has one sheet, which keeps today's MCP baselines and the 4096-byte pages unchanged; the tool
  descriptions say "absent means sheet 1". `list_items` takes a `filter.sheet`. `inspect_part`'s
  summary gains `sheets: m` and a per-sheet size and scale when m > 1. The analysis itself has no
  sheets. The schemas under `schemas/mcp/` are regenerated with
  `tools/generate_mcp_schemas.py`.

### 9. Migration

A spec without `sheets` and without any `sheet` pin plans, renders and serialises **byte for
byte** as before. Each new field is `skip_serializing_if` its single-sheet value, step 1 of the
allocation is today's code path, zones and cross-references exist only when m > 1, and `pdf()`
is unchanged. The plan schema stays `draftwright.plan/1`. Old plans read back, because every new
field is `#[serde(default)]`.

### 10. The editor meanwhile

The editor (`draftwright-editor`, thin-slice) vendors draftwright `v0.1.0-alpha.1` and reads
`plan.sheet`, `plan.views` and `render_svg` (`crates/engine/src/lib.rs`,
`src/components/DrawingStage.tsx`, `web/app.js`). It gets no sheet UI in these stages. When it
re-vendors:

- `plan.sheet` is still sheet 1, and `render_svg` draws sheet 1;
- `crates/engine` filters `plan.views` to `sheet == 1` before its own `dims::plan` and
  `dims::audit` calls;
- its Paper and Scale selectors keep writing `sheet.size` and `sheet.scale`. A multi-sheet spec
  written elsewhere is shown as sheet 1 only, and its `sheets` fields are kept unchanged.

The maintainer redesigns the editor after these stages.

## Invariants

1. **A single-sheet spec is unchanged byte for byte:** its plan JSON, SVG, PDF, DXF and PNG.
   Guards: `tests/sheets.rs::single_sheet_documents_are_byte_identical`, together with the
   existing `tests/fixtures/known_*.json` and the CI quality baseline, unchanged.
2. **The principal views are on sheet 1, never split.** Guard:
   `sheets::principal_views_stay_on_sheet_1`.
3. **Every sheet has the title block, and SHEET reads `n OF m` without overflow on A4 to A0.**
   Guard: `sheets::every_sheet_has_its_title_block`.
4. **Every frame's datums are identified on sheet 1 or on the frame's own sheet, or the problem
   says so.** Guard: `sheets::datum_rule`.
5. **A section or detail view on another sheet is cross-referenced both ways with the correct
   zones.** Guard: `sheets::cross_references_both_ways`.
6. **A further sheet is preferred to a scale below `smallest_scale`; the paper ladder is
   otherwise unchanged.** Guard: `sheets::a_further_sheet_before_a_smaller_scale`.
7. **IDs are independent of sheets.** Guard: `sheets::moving_a_view_keeps_ids`.

## Tests that prove it (`tests/sheets.rs`, new, plus the existing suites)

- **Byte identity:** every slice part and AP242 fixture, with the default spec and with each
  existing test spec, gives plan, SVG, PDF and DXF bytes equal to the pre-change build. The hashes
  are recorded in `tests/fixtures/known_single_sheet.json` before any Stage I code lands. The CI
  quality baseline (300 cells) and the pin grid (270 runs) are unchanged.
- **Allocation:** on a corpus part with section A–A, `views/section_aa/sheet = 2` gives two
  sheets, with the section on sheet 2 at its parent's scale. `keep_together [front, section_aa]`
  refuses that pin. `sheets.max = 2` with a `smallest_scale` above what one sheet allows moves the
  isometric, then the section, and sheet 1 lands at or above the floor. The same spec without
  `max` reports `sheet_below_smallest_scale`. On a dense-hole part (≥ 16 plan holes), a forced
  `hole_table = 2` puts the table on sheet 2 and the balloons on the plan view on sheet 1.
- **GD&T:** on NIST FTC-07 or CTC-01 (AP242), moving the view that carries datum A away from a
  frame that references it either is avoided by the planner or gives `datum_not_identified`.
  `gdt_on_sheet_1` puts every frame on a sheet-1 view.
- **Title block and zones:** each sheet's `SHEET` cell reads `n OF m`. Zones are 6×4 on A4 up to
  24×16 on A0. `sheets::zone` maps known points. The cross-reference texts appear in the drawn
  text of both sheets.
- **Formats:** the SVG count is m, the PDF page count is m, the DXF count is m, and each file's
  embedded spec round-trips through `draftwright spec`.
- **MCP:** a qualified address is accepted on its own sheet and refused with the correct address
  on another. The move op is listed in `item_options`, applied by `apply_ops`, and shown in the
  delta as the view and its items `changed`. `list_items` `filter.sheet` works. The schema check
  (`tools/generate_mcp_schemas.py` and the jsonschema validation) passes. The scripted MCP
  evaluation stays at 7/7.
- **Editor:** `draftwright-editor` builds and its `crates/engine/tests/session.rs` passes against
  the new engine, with the editor showing sheet 1 of a two-sheet spec.

## Build plan: Stage I

Every stream runs on its own branch from `main` and keeps invariant 1. Stream I1 lands first;
I2 to I4 build on it and can run in parallel; I5 is last.

| Stream | Changes | Acceptance |
| --- | --- | --- |
| **I1: model and allocation** | `src/model.rs` (`Spec.sheets`, pins on `ViewSpec`, `DimensionSpec`, `TableSpec`; `Plan.sheets`; `PlannedView.sheet`, `scale_note`, `xref`), `src/placement_intent.rs` (`AnnotationSpec.sheet`), `src/sheets.rs` (new: allocation, `zone`, `check`), `src/layout.rs` (`plan_sheet`), `src/hole_table.rs` (`sheet`, long-table move), `src/items.rs` (`Item.sheet`), `src/dims/{placement,mod,gdt}.rs` (per-sheet audits, sheet-limited carriers), `src/tables.rs`, `docs/model.md` | `known_single_sheet.json` recorded first and unchanged; the allocation, GD&T and check tests above pass; the full corpus suite, fmt and strict clippy pass |
| **I2: sheet furniture and rendering** | `src/title_block.rs` (multi-sheet bottom row, `n OF m`), `src/model.rs` render (`render_sheets`, border zones, cross-reference text), `src/section.rs` (cross-reference under the label), `src/output.rs` (`pdf_pages`), `src/png.rs`, `src/api.rs` (sheet files, `write`), `src/lint/structural.rs` (`cross_reference_missing` on drawn text) | Title block, zone, cross-reference and format tests pass; single-sheet bytes unchanged; a two-sheet PDF opens with 2 pages, and its text search finds both cross-references |
| **I3: MCP and sessions** | `src/session.rs` (delta `sheets`, `filter.sheet`, qualified addresses), `src/stateless.rs` (`render(sheet)`, summary), `src/mcp.rs`, `schemas/mcp/*.json`, `tools/generate_mcp_schemas.py`, `docs/mcp.md` | The MCP tests above pass; existing MCP baselines and the 4096-byte budgets unchanged for single-sheet parts; the scripted evaluation stays 7/7 |
| **I4: editor compatibility** | In `draftwright-editor`: `vendor/draftwright` re-vendored, `crates/engine/src/lib.rs` (sheet-1 view filter). No UI change | Editor build, `crates/engine/tests/session.rs` and `web/smoke.cjs` pass; a two-sheet spec shows sheet 1 and survives a round trip unchanged |
| **I5: dense-region detail views** | `src/demand.rs` (dense region: the hole-location spacing and step-legibility findings that today force a larger scale), `src/sheets.rs` (detail candidate), `src/model.rs` (detail view: circle on the parent, `DETAIL B`, `SCALE n:1`, cross-references), `src/section.rs`-like furniture in a new `src/detail.rs` | On a part whose recovery today goes below a set `smallest_scale` because of one dense region, the plan instead keeps sheet 1 at the floor and puts a detail view of that region on sheet 2, enlarged, cross-referenced and with every dropped item restored |

## Open

- **Detail views (I5)** are new to the port: it has no detail views today. They may be large
  enough to deserve their own stage after I1 to I4. Until they land, a dense region falls back to
  the ladder plus `sheet_below_smallest_scale`.
- **The SHEET cell:** widening it in multi-sheet documents (section 4) departs from Python's grid
  and from ISO 7200's widths. The alternative is ISO 7200's own `n/m` (3.95 mm, fits today's
  cell), but the policy says "SHEET n OF m".
