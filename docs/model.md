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
  Principal faces now also retain optional `outer_lines` (straight outer boundary edges).
  Plate planning can choose a boundary profile whose two thickness faces have overlapping
  solid edges, and anchor its witnesses there. This is model-space Analysis evidence;
  the Plan does not copy the projected line-work inventory. Each profile source also retains optional
  `boundary_segments`: model-space B-rep chords used as tag-clearance obstacles.
  They include internal face edges and analytic cylinder/cone generators tangent
  to principal datum directions. Optional `curved_faces` retain the remaining
  curved surfaces' bounds and revolution axes for conservative clearance. Older analyses default to no straight-edge
  evidence and keep the axis-map fallback. Profile visibility is proved only for one solid;
  body bounds are transformed from the recognition frame before matching.

- **Edge evidence** (optional `edge_ink`, `draftwright.analysis/7`): every edge as a
  polyline in the part's frame (within 0.05 mm, coordinates to 0.001 mm), planar faces by
  normal and loop edges, and curved faces by box (cylinders also by axis and radius). The
  planner projects it into a view to prove a region blank before placing a label inside
  (`src/model/projected_ink.rs`, `src/dims/interior.rs`); the projection is a superset of the
  drawn ink. Rendering does not read it. Older documents default to none and keep every label
  outside its view. The Plan records only the resolved annotation's `region: interior` and
  basis. See [R1's decision note](adr/R1-document-model.md).

- **PMI** (`draftwright.analysis/7`, `pmi`): the part's AP242 semantic PMI as `src/pmi.rs`
  reads it (`read`, the report: records with their faces, and every source not read with its
  reason), each geometric tolerance and datum with where it attaches (`geometry`: a datum's site
  on each face, its plane normal or cylinder axis and its outward normal; a tolerance's common
  cylinder axis), derived from the faces as draftwright's `_datum_geometry_from_shapes` derives
  them; or `refused` with the reason when a file naming semantic PMI cannot be read. Absent when
  the file has none (presentation alone is counted). `plan` lowers it (below); older analyses
  read with none. Manufacturing requirements (threads and knurls with their structured fields
  and faces, general tolerances, surface texture and edge notes; AP242 surface textures, a
  part's as a default `surface_texture`, a face's as a `surface_finish` with its material removal
  condition and parameters in `fields`), surface labels and common
  labels are records too; a surface or common label carries its proven sites (`geometry`, below)
  and is drawn as a label; the drawing-wide requirements are the sheet's (below); a thread or
  knurl carries its cylinders (`geometry.cylinders`) and is printed on its owner's callout
  (below).
- **Assemblies: one chosen part** (draftwright-rust #8, slice 3; no schema bump, every field is
  optional and absent for a file of one part, whose Analysis is byte-identical). A STEP file's
  distinct parts are listed by `draftwright parts part.step` (serve/MCP `parts`):
  `{"parts": [{"part": 0, "name": "plate", "product_id": "plate", "faces": 13, "instances": 1},
  …]}`, in haecceity's order (`read_part_definitions`). A part is chosen by its **index**
  (`--part K`; `"part": K` in a request), with the name and product id reported beside it (the
  maintainer's D1). Part K is read alone, in its own frame (`haecceity::StepFile::part`: its
  first placement's shape with no placement applied), so a part placed twice is drawn once.
  Its `binding.part` is the part's name and `binding.part_index` is K; `face_count` and
  `envelope` are the part's own. Its PMI is the part's only: other parts' records, findings and
  material are dropped, not refused (`pmi.drawn` = K), and a material it states fills the title
  block. With no part chosen an assembly is read whole, as before (D4): the Analysis lists the
  file's parts in `parts` (only when there are two or more) and `plan` warns
  `assembly_drawn_whole`, naming the parts and how to choose one. An Analysis made from one
  part is re-read from that part when passed back (`--analysis`), and a `part` beside it must be
  the same. An index the file does not have is refused (`invalid`), naming the parts. Nested
  sub-assemblies are flattened to their leaf parts; a sub-assembly is not selectable (D2).

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
  "schema": "draftwright.spec/2",
  "binding": { "source_sha256": "…" },
  "sheet": { "size": "A3", "scale": "2:1",
             "title": { "title": "BRACKET", "number": "DWG-001", "material": "6082-T6" } },
  "views": {
    "plan": { "place": { "relative_to": "front", "side": "above",
                         "gap": 10, "align": "middle", "pinned": true } },
    "side": { "omit": true }
  },
  "features": { "f:hole:3": { "fit": "H7" } },
  "dimensions": {
    "d:f:hole:3/bore.diameter": { "tolerance": "±0.05" },
    "d:envelope/width.length": {
      "place": { "view": "front", "side": "above", "lane": 1,
                 "along": "middle", "pinned": true }
    }
  },
  "annotations": {
    "pmi:datum.A:f12:t=A": { "place": { "view": "front", "side": "left",
                                      "lane": 0, "pinned": true } }
  },
  "notes": [ { "id": "n:1", "text": "BREAK ALL SHARP EDGES" } ]
}
```

The design also includes `views/sections` (for example `"sections": [{"id": "A", "through": "f:hole:3"}]`). Feature `anchor` is read for recognition corrections, below; a feature's `note` is read since K1b (below).

- **Fields read:** `sheet` (`size`, `scale`, `title`, `fit_deviations`, `fit_limits`, `projection`, `default_finish`, `hidden`, `centre_marks`, `centre_lines`, `thread_depiction`, `knurl_depiction`), `sheets` (below), `views/<name>` (`omit`, `place`, `sheet`, `hidden`, `centre_marks`, `centre_lines`, `thread_depiction`, `knurl_depiction`),
  `iso_scale_factor`, `dimension_source`, `hole_table` (`auto`, `always`, `never`), `dimensions/<id>` (`omit`, `tolerance`, `fit`, `text`, `style`, `place`, `kind`, `between`, `along`, and the earlier `view`
  spelling), `annotations/<id>` (`omit`, `place`, `text` for prose callouts and automatic notes), `features/<id>` (`fit`, `datum`, `controls`, `finish`, `thread`, `knurl`, `note`, `anchor`, `reclassify`, `suppress`, `split`, `pattern`), `feature_declarations`,
  `faces/<face>` (`datum`, `controls`, `finish`, `note`: "Datums and controls authored in the Spec" and "Finish, threads,
  knurls and notes authored in the Spec", below), `notes` and `tables`.
  Unknown fields, including page coordinates, are refused. Absent fields keep their defaults.
- **The spec stores intent, never geometry.** A fit, lane, side or fractional label position is
  resolved against the part and its anchors on every plan. Measured values and resolved paper
  coordinates occur only in the Analysis and Plan.

`iso_scale_factor` is an optional finite positive multiplier of the sheet scale, kept exactly
through automatic fitting and recovery. An infeasible authored iso scale is refused rather
than reduced. Without an override the orientation view fills its available space, retaining
the sheet scale within Python's strict 5% no-op band. A fitting sheet-scale iso stays at that
scale when a detail claims space; detail-view generation itself is not yet ported.

When the drawn iso differs from its sheet scale, `Plan.captions` carries `note_iso_nts`,
text `ISO VIEW (NTS)` at 3 mm, with its view, sheet, resolved position and ink bounds. It is
an automatic read-only `caption` item, reserved before late notes and tables. It tries
Python's positions below, further below, left, right and above the iso, keeping 2 mm clear
of settled view and annotation ink and the title block. If every position is blocked, it
retries without the extra text clearance, then slides the candidates vertically to clear
obstacle edges. Only an entirely blocked search retains Python's natural-position fallback;
structural lint reports even shallow caption contacts with labels, lines and view edges.
A sheet-scale or omitted iso carries no NTS caption. This furniture does not change the sheet-choice ladder or fill.

#### Recognition corrections (#76 item 2)

The automatic reading remains the default. Corrections sit beside a feature's other authored
requirements; missed features have a separate declaration list. They carry intent and stable
face fingerprints, never measured sizes or page coordinates:

```json
{
  "features": {
    "f:boss:1": { "reclassify": "turned_step" },
    "f:hole:9": { "suppress": true },
    "f:hole_pattern:1": { "split": true },
    "f:hole_pattern:mounting": {
      "pattern": { "arrangement": "grid", "members": ["f:hole:1", "f:hole:2", "f:hole:3", "f:hole:5", "f:hole:6", "f:hole:8"] }
    }
  },
  "feature_declarations": [
    { "id": "f:declared:pilot", "kind": "hole", "faces": ["face:sha256:<64 hex digits>"] }
  ]
}
```

The example's actions are independent; a grouped member must remain an active hole. `pattern`
accepts `linear`, `grid` or `bolt_circle`; all count, pitch, centre and diameter data is derived
from its physical members. Authored pattern IDs require a nonnumeric name, keeping them
separate from automatic pattern ranks across re-analysis. A line must be evenly spaced along a principal axis; a grid must
fill an evenly spaced rectangular lattice; a bolt circle needs at least three noncollinear,
equally spaced holes on one circle. Members have identical machining sizes, depths and drilling
direction and open on one plane. The ordinary planner's corroboration still applies to bolt
circles with fewer than five holes: a bore or boss must prove the centre. A new group replaces
intersecting automatic groups; holes may belong to only one authored group. `split` dissolves
a recognised hole or slot pattern and keeps its members. Normal sharing of identical hole
sizes continues; their locations are individual and the pattern's pitch statements disappear.

`reclassify` accepts `hole`, `boss`, `turned_step`, `slot` or `pocket`. Supported changes are
boss ↔ turned step and slot ↔ pocket, when defining faces prove the target grammar. Cylindrical
interpretations need complete, coaxial, equal-radius principal cylinders with contiguous axial
spans; opening evidence distinguishes a bore from an external band. Rectangular interpretations
need four inward-facing planar walls with matching rectangular spans. A pocket also needs
its capped planar floor and a proved open planar mouth; a through slot needs two proved
open ends. Exact principal profiles or the existing planar edge graph on a single solid
prove the opening, including local stock planes below a higher boss. An omitted cap cannot
turn a closed void or a blind pocket into an open slot. Curved/unproved mouths are refused. Rust's existing
`section_recess` feature is the pocket representation. Unsupported, partial, oblique or
ambiguous geometry is refused with its reason; the correction never invents dimensions.
Derived fingerprints retain their principal axes, so existing view-demand rules apply. A named
boss end selects its outward direction; without one unique end, a boss interpretation uses
the positive principal direction. Name the free end to select a negative-facing boss. Hole
and boss vectors can still be read when a bridge's working frame is oblique.
A suppressed feature cannot also carry fit, thread, datum, control, finish, knurl or note
requirements. Choose only one of `reclassify`, `suppress`, `split` and `pattern` on a key.

Declarations use the same five kinds and geometry checks. `id` is a unique authored
`f:declared:<name>`; the list carries no numerical parameters. For a bore, name the cylindrical
wall and its planar blind floor when present; without a floor the cylinder must reach both
stock planes to prove throughness. Equivalent automatic recognition yields to the declared ID, including when a later
recogniser learns the previously missed feature. Automatic hole/slot pattern links follow
that identity, so a physical occurrence is never counted twice. Extra planar end caps may
belong to the same proven scope. A different/incomplete scope, another authored declaration,
or authored requirements/references still naming the retired automatic owner is refused;
move those references to the declared ID, or suppress/reclassify the conflicting reading
explicitly. The declaration then
accepts ordinary `features/<declared-id>` requirements and `dimensions/d:<declared-id>/…` edits.

`recognition_corrections::face_fingerprint(analysis, face_index)` supplies each face name;
`item_options` with `id: "face:<index>"` reads one in
`settable.feature_declarations.fields`; sheet options explain this lookup. Indices are
Analysis discovery addresses only: the Spec stores the resulting fingerprint. Each response
stays bounded rather than returning every face on a large part.
Signatures hash the existing attachment's surface, principal axis, support, bounds and outward
planar normal, rounded to 1e-6 mm; they exclude face indices and sample order. They are
model-space identity fingerprints, never authored geometry. An Analysis lacking face evidence
must be regenerated. A fingerprint must resolve to exactly one face: missing or ambiguous
faces are refused, naming the saved path and fingerprint and advising reattachment to current
fingerprints or removal. Identical signatures are deliberately ambiguous.

When saving a correction, the bound Plan Spec fills `features/<id>/anchor` with
`{"faces":["face:sha256:…", …]}` for the target and every grouped member. Explicit anchors are
never silently replaced. This lets a saved correction resolve by its defining faces on
re-analysis rather than trusting a changed rank. Exact matches refresh feature keys,
pattern-member references, and the associated dimension and precision keys when recognition
ranks change. Many-to-one collisions are refused for the author to combine explicitly. Authored declaration and group IDs are the
chosen names, independent of recognition order. Re-analysis does not renumber them. A
geometric change outside the exact signature's tolerance needs an explicit reattachment;
this stream does not guess a nearest face or implement general revision correspondence. When a
source hash changes but all correction anchors still resolve exactly, a correction-only Spec
can bind to the new source. A Spec also containing unrelated, unanchored revision-specific
edits is refused until those edits are reattached; they cannot inherit the correction's proof.

Corrections resolve before sheet demand, dimensioning, callouts, schedules and completeness.
The same effective Analysis is passed to independent lint by `spec_gdt::effective` (the existing
merge entry point used by API review and sessions). Lint continues to read only Analysis and
rendered evidence. Suppression removes a spurious feature from that inventory; it does not
acknowledge unrelated findings, remove PMI or silence the independent profile checks.
Declarations and reclassifications acquire the normal measurement requirements. Omitting or
misprinting their output still raises lint findings. Library callers who invoke lint directly
must pass this effective Analysis, as for authored GD&T.

`Plan.recognition_corrections` records every applied decision as `{id, from_spec, basis}`, even
suppression, which has no resulting dimension. Affected dimensions also carry the correction
path in `from_spec`, set `default: false`, and explain the change in their basis. With no
corrections the resolver borrows the existing Analysis, emits no new Plan fields and performs
no face lookup, hashing, geometry scan or recognition run. Sheet selection, detail policy,
datum position and fits defaults continue through the existing planner.

This is authored intent within R4's **one analysed part** decision (ADR4-3), consistent with
ADR1-4's single feature waist. `analyse` continues to report recognition unchanged; it never
automatically generates Spec declarations. The author names a missed feature and its faces;
resolution reads their already analysed geometry. As in Python's `model/declare.py`, these
features feed the same IR and planning as recognition. Rust deliberately offers no second
recognition-free build mode and no explicit numerical geometry constructors.

`item_options` with a recognised feature or pattern ID (`f:…`) returns its correction
controls. Keeping these on feature queries preserves the dimension options' wire budget.

Session `apply`/preview, undo/redo, stateless operations and WASM's existing `apply`/`setSpec`
all use the common Spec editor and validation. Use `set`/`remove` at `features/<id>/<field>`;
use `add` at `feature_declarations`, or `set` the whole declaration list to replace/re-anchor it.
List-index editing remains reserved for the existing table sheet operation. Invalid edits
leave the committed session unchanged. MCP declaration shapes are published by
`tools/generate_mcp_schemas.py`; supported kinds, face resolution and geometric semantics are
validated by the common Rust model, as with other feature requirements.

Reproduce three before/after PNG, SVG, Plan, Spec and lint pairs with
`cargo run --release --example recognition_corrections` in
`target/recognition-corrections/renders/`. These use unchanged STEP geometry: split a native
grid, declare a real native pocket omission, and reinterpret a boss as a turned band.
See [the validation and drafting review](recognition-corrections-validation.md) for
results, retained warnings and the measured performance gates.

#### Projection method and a pinned sheet's fit

`sheet/projection` is ISO 5456-2's method, `first` or `third` (angle); absent means third, and the
plan is then byte for byte what it was before the field existed (`third` set explicitly differs only
in the spec it echoes). First angle puts the plan below the front and the right side view left of
it, the same blocks and bands mirrored within the packed group, and draws the first-angle symbol
(cone left, circles right, the cone's large end beside them) where the third-angle symbol
(circles left, cone right, its small end beside them) goes, both ISO 5456-2's figures;
`sheet.projection_symbol.method` says `first-angle` or `third-angle`, and the views' bases say
which side each sits. The session's `item_options` for `sheet` lists the choices. Other sheets of a
multi-sheet drawing carry no projected views, so the setting only arranges sheet 1.

A spec that pins both `sheet/size` and `sheet/scale` is judged as the automatic search judges its
candidates: when the packed estimate doubts the fit, the finished drawing (after the same measured
correction the search applies) is checked, and if every drawn item and view lies on the sheet and
off the title block, `page_fit_uncertain` is not raised (the basis says "the finished drawing
fits"). So pinning the pair the engine would choose itself raises nothing. When the views really
do not fit, the problem's `fix` sets the nearest pair that does, found by the engine's own search:
the pinned sheet at the scale the recovery picks for it, else the pinned scale on the smallest sheet
that holds it, else the automatic pair; each is offered only when, pinned, its drawing fits. The
message names it (`…; A4 at 1:1 fits.`).

#### Line work per view (#76 item 6)

Each view accepts optional boolean `hidden`, `centre_marks`, `centre_lines`,
`thread_depiction` and `knurl_depiction`. The same fields under `sheet` set defaults
for **every view on every sheet**, including auxiliary views allocated to later sheets:

```json
{
  "sheet": {"hidden": false, "centre_marks": true, "thread_depiction": false},
  "views": {
    "side": {"hidden": true, "centre_marks": false},
    "front": {"centre_lines": false, "knurl_depiction": false}
  }
}
```

Precedence is **view, then sheet, then the view's automatic convention**. There are
currently no `sheets/<n>` settings: `sheets` controls allocation, not numbered sheet
styles. Removing a setting restores inheritance; absent fields preserve the original
output, including the isometric's own defaults. An explicit centre switch can request
eligible centre furniture in the isometric; it does not fabricate an axis or feature.
`centre_marks` controls end-on crosses, while `centre_lines` controls profile axes and
pitch circles. They are independent. Thread and knurl switches affect conventional
ink only: manufacturing requirements, dimensions, callouts and coverage remain.
Cosmetic depiction still requires a proven profile feature. Sections always omit hidden
edges and cosmetic threads/knurls, even with `true`; centre switches do not remove the
cutting-plane chain line, section arrows or hatching.

Settings name stable existing view keys (`front`, `plan`, `side`, `iso`, `section_aa`),
never page coordinates or projected edges. Unknown view references are refused with
available names and a replacement/removal instruction; misspelled fields and non-boolean
values are refused. As with other optional Spec fields, JSON `null` means absent.
A setting for an omitted view is saved for when that view is shown again. No new
feature-level switch or fingerprint address is introduced: furniture can be merged across
features, so per-feature suppression needs separate ownership work.

Planned views record effective authored switches in `line_style`, with the winning
Spec paths in `from_spec`, `default: false`, and an explanation in `basis`. The embedded
Spec retains all inherited and overridden entries. `set`/`remove` edits use the usual
paths, for example `views/side/hidden` or `sheet/centre_marks`; session options expose
all five booleans for views and sheets. Preview, undo/redo, geometry caching and delta
replay follow the ordinary Spec edit path.

These are drawing choices, not lint exemptions. Hidden curves switched off are absent
from rendered geometry, so `hidden_on_visible` and `section_hidden` measure only drawn
curves. Hidden-geometry completeness is not inferred from undrawn curves. Independent
lint still reports missing centre marks when the author disables them, as well as wrong
dimensions, missing manufacturing requirements and other drawing defects. Automatic
layout and its conservative projected-ink clearance remain in force; hiding edges does
not relax safety margins or change sheet-size, scale, detail, fit or datum policy.

#### Sheet allocation (R5, Stage I)

`sheets.max` allows 1–9 sheets. An explicit count is binding. With no count,
the normal search starts with one sheet; a small part with no valid complete A4/A3
single-sheet drawing tries R5 allocation on A4/A3 before A2 or larger (up to the
existing nine-sheet bound). Named paper keeps the original default of one.
See the [R5 small-part amendment](adr/R5-multi-sheet.md#small-part-overflow-amendment-2026-10-16). `sheets.smallest_scale` is an ISO
5455 scale floor, `keep_together` is a list of groups of view names (also allowing
`hole_table`), `hole_table` assigns a rescue hole table if escalation actually makes
one, and `gdt_on_sheet_1` records that requirement. A hole-table assignment does not
request or force a table. Existing per-view legibility schedules remain on sheet 1.
`views/<name>/sheet` assigns an auxiliary view; principal views accept only 1.
`tables/<index>/sheet` assigns a corner table. All sheets share the first sheet's
paper size, title and drawing number.

An assignment must be within `1..sheets.max`, must agree with its keep-together
group, and must put a view with `place.relative_to` on its anchor's sheet. Those
relations also form implicit keep-together groups. Placement pins are kept;
an explicit sheet assignment remains possible for an auxiliary group. A stated
`sheet.scale` below `sheets.smallest_scale` is refused with choices to remove the
scale or lower the floor. List-index edits are supported only for
`tables/<index>/sheet`, with `set` and `remove`.

The [2026-10-16 audit](second-sheet-audit.csv) lists all 68 small A2+ cases found
among 2,150 native default/spec cases over 190 corpus and fixture STEP inputs
(baseline `53aa9f3f`, origin/main at the start of the change). All 63 source hashes
in the captured single-sheet spec fixture are covered. Three selections
improved: 10224 and 10255 use one A3 sheet at 2:1 after measured section
correction; GRM-03's fixed 10:1 spec fits one A3. The
[follow-up review](second-sheet-review.md) records the original candidate failures,
auxiliary-page composition and references checked against drawn target positions.
The 44 named large-paper requests stay as authored. Every other small
large-paper case records both failed practical-paper allocations. The report's
required-drop count includes unreadably crowded location dimensions. When no
allocation prefix is clean, the reported best step may be the original one-sheet
step; each audit's `audit.json` retains every paper/scale and moved-prefix trial.
All 742 cases whose full-size views need larger paper retain their size, scale
and count.

Run `cargo run --release --example sheet_policy_audit -- CORPUS OUT` against each
revision, then `python3 -I tools/compare_sheet_policy.py BEFORE/audit.json
AFTER/audit.json OUT` to reproduce the case table and the unchanged-large-part gate.

Without sheet controls a valid single A4/A3 drawing keeps the existing planner path.
A floor alone enters the existing recovery. With `sheets.max > 1`, each allocation
prefix uses the existing finished-sheet recovery with the floor, including table
recovery. There are at most four prefixes; H6's exhaustive selection supersedes
R5's older six-retry estimate. Larger paper is tried before moving auxiliary content.
The cumulative overflow order is isometric, section A–A, then a rescue hole table
withdrawn for lack of table room. Assigned content moves first; a group containing a
principal or a placement pin does not move automatically. Sections that fit stay
on sheet 1. The first clean prefix wins; otherwise the fewest hard findings and
required drops, largest first-sheet scale, then fewest moves choose the result.
The sheet basis records every attempted or blocked prefix.

Assigned corner tables are placed first on later sheets, then complete view blocks
in wrapping rows, clear of the title block and each other. A view tries a fresh sheet
at its parent's scale before taking a smaller standard scale. A long rescue table
gets its own sheet; an explicit keep-together group shares that sheet and keeps clear
of the table. Every balloon must still land on the plan view on sheet 1. A table
that cannot fit even on its own sheet is withdrawn with its original dimension
states and geometry restored. Nothing is silently removed. Empty assigned sheets
are closed by renumbering, with `sheet_renumbered` and an edit to the actual sheet.

`plan.sheet` remains sheet 1 (`sheet`). Optional `plan.sheets` contains sheets 2..m
(`sheet:2` etc.); each carries `number` and, for a multi-sheet document, `zones`.
Views, corner tables and hole tables carry `sheet`, omitted when 1. A moved view's
`scale_note` records a scale different from its sheet's. A section's `xref` and its
cut's `xref` refer both ways, with sheet, grid zone and resolved text position.
Zones use 6×4 fields on A4, 8×6 on A3, 12×8 on A2, 16×12 on A1 and 24×16 on A0;
numbers run left to right and letters top to bottom, skipping I and O.

Item IDs survive moves. `items::Item.sheet` follows the carrying view or table;
notes and drawing-wide requirements stay on sheet 1. Every Stage I dimension,
frame and datum carrier is asserted to be on sheet 1. The generic datum check
requires each referenced symbol on sheet 1 or the frame's sheet. Collision audits
compare items only within their own sheet; later-sheet block audits check the
border, title block and other blocks. A missing section cross-reference is an error.
`sheet_below_smallest_scale` reports a floor that could not be reached, offering
more sheets only when a blocked allocation could help, otherwise larger fixed
paper or a lower floor. An unused rescue-table assignment reports
`sheet_assignment_unused` with a remove edit.

Stage I1 produces and checks these later-sheet plans. I2a adds
`model::render_sheets`, one Drawing per sheet, while `model::render` continues to
draw sheet 1, including the parent's cutting-plane furniture for a remote section.
Every sheet repeats the unchanged title-block grid with a planned `n/m` SHEET
value; an authored sheet label is replaced with `sheet_label_replaced` info.
The projection symbol and general notes stay on sheet 1. SVG and PNG keep the
sheet-1 filename and add `<stem>.sheet<n>.svg` / `.png` for later sheets; one PDF
contains every sheet in order (`Drawing::pdf_pages`, single-sheet bytes unchanged).
Every output embeds the whole Spec. Review keeps `svg` for sheet 1 and adds
`sheets: [svg…]` for sheets 2..m, omitted for one sheet. Coverage reads the marks of
every sheet once; structural checks run per sheet and
later-sheet issues carry their sheet number. Border zones and reciprocal section
addresses are drawn as searchable text, and structural lint checks that each
address reaches its rendered target in the printed grid. MCP sheet
operations are I3's scope. The plan and spec schema versions stay the same;
old documents read with single-sheet defaults.

#### Placement (`place`, S3)

Dimensions (linear dimensions and callouts, including holes, fillets and chamfers) and PMI
frames, datum symbols and labels share `{view?, side?, lane?, along?, pinned?}`; callouts
also take `offset` (below):

| Field | Meaning |
| --- | --- |
| `view` | `front`, `plan` or `side`; a dimension must be visible across that view. `place.view` takes precedence over the earlier `dimensions/<id>/view`. |
| `side` | `above`, `below`, `left` or `right` of the carrying view's projected envelope. A linear dimension uses the two sides parallel to its measured span; incompatible sides are refused with the choices. |
| `lane` | Zero-based order out from the view, 0–1000. Linear lanes use the existing paper spacing; callout columns on the left/right use the widest canonical label on that view plus shoulder and clearance, so labels at the same along position have distinct columns; PMI tiers use their glyph's size and clearance. Without strip/label fields a pin keeps the automatic ink. For an explicit strip placement, an omitted lane uses the automatic linear lane or the nearest PMI tier. |
| `along` | A fraction 0–1, or `start`, `middle`, `end`, `outside` (1.25). A linear label is centred at that fraction of its witness span; at `outside` it sits just past the line's far end, the dimension line run on to it, as a span too short for its label has it (#32: 1.25 of a long span stranded the text). Leadered labels use that fraction of the view's extent along the strip, keeping their original feature attachment. |
| `offset` | Callouts only: `[dx, dy]`, each within ±1000 paper mm (y up), the bend's offset from the callout's automatic leader tip. Requires `pinned: true` and excludes `side`, `lane` and `along`. |
| `pinned` | Defaults to false. True fixes the resolved geometry before automatic layout; without side, lane or along it keeps the canonical automatic ink (its lines and arrows; a label whose own text changes, such as a new tolerance, is re-set from its current text, centred where the old one was, or for a callout from the end of its shoulder); a free item can be adjusted around obstacles. |

An unpinned callout with only `side` (and optionally `view`) keeps participating in the
shared leader solve. Its candidates stay on that side, its printed label reserves paper space
before view packing, and the solver chooses a station clear of other ink. Supplying `lane`,
`along` or a pin continues to prescribe a station.

A pinned callout above or below a view puts its text on the side its leader runs to:
an elbow left of the tip carries text to the left, and a vertical leader defaults to the
right. Left/right columns keep their stated side. A pinned callout's bend and label are
never slid to avoid a feature crossing. The existing `leader_through_feature` warning
names the callout and crossed feature in `subjects`, also names the callout in `refs`,
and explains that the pinned label's leader crosses part features and keeps its position.

A pinned `offset` (#12) can put a callout's label anywhere, including the open space inside its
view, where the automatic solver often puts it and side/lane/along cannot. The bend goes at the
automatic leader tip plus the offset. That tip is the canonical plan's (the part's default
plan, as a bare pin keeps), moved with its view when packing moves the view, so unrelated edits
cannot move the pin. The leader re-attaches radially to the site the automatic tip lies on, so
a bend dragged round its hole keeps a radial shaft, and the text reads on the side the leader
runs to (a vertical leader keeps the automatic side). An offset that puts the bend where the
canonical plan has it is planned as a bare pin, so it is the automatic ink exactly whatever that
leader's direction: pinning a callout where it is moves nothing. A callout drawn without a
leader (no automatic placement, e.g. a pattern given by a hole table) has no tip to pin from:
`callout_offset` refuses it. With `place.view` naming a view other than the canonical one, the
offset is taken from the callout's first site centre in that view and the leader goes to the
nearest site; `callout_offset` reports only the canonical view and refuses the callout there. The bend and label are never slid; a crossing is the
`leader_through_feature` warning above, and a label over the view is `annotation_over_view`.

Existing angular PMI keeps its measured sector: `lane` increases the radius in paper steps and
`along` selects a fraction of its arc. A requested `side` is reported as
`placement_side_not_applicable`, with a fix removing that field; it cannot change the sector.

Views use `{relative_to, side, gap?, align?, pinned?}`. `relative_to` names another view
(`front`, `plan`, `side`, `iso`, `section_aa`); the page is never an anchor, so a view's place
is always relative to another view (ADR R1 invariant 1), typically its projection partner.
`gap` is the clear paper distance between view footprints (default 10 mm, 0–1000);
`align` is `start`, `middle` (default) or `end` along the other axis (`outside` also aligns
ends). Dependencies resolve before packing; cycles and self-anchors are refused. Missing anchors
are reported, with the entry retained. Unpinned views fit around fixed footprints; an automatic
isometric may be reduced to fit the space left. A pinned isometric first resolves the requested
relative region, clips it to the drawing area and fits its ink in the largest free rectangle
inside that region. Its resulting position and scale are then fixed. Principal view pins keep
the literal footprint relation and report any overrun.

A pin also fixes its carrying view and the view-anchor dependency chain for automatic packing.
Those frames and the automatic sheet/scale are resolved from the part's default plan and the
explicit sheet/scale choices, independent of unrelated text, omission and placement edits.
Recovery cannot change them to repair another item's collision. Editing the pin's own anchor
intent, sheet or scale deliberately re-resolves it. All original `place` entries remain in the
embedded Spec, including entries that are covered, omitted, or cannot fit.

Automatic linear labels settle against the completed dimension and GD&T ink. Text stays
between its own witness stations when it fits, otherwise just beside an arrow. If no such
label position clears ink, the whole dimension tries the next outward paper lane within the
existing view footprint. It may skip one occupied dimension rung, so the absolute movement
limit is two lanes (14 paper mm); a fixed-ink obstruction alone never permits that skip.
Settlement traverses the inventory once and tests span endpoints, centres and obstacle
boundaries rather than sampling the whole span. Cleared text/line contacts leave at least
1 paper mm, and a moved label (or its moved lines) may not come within 1 paper mm of another
item's stroke or label that was clear by 1 mm before the move. Inside the span comes first;
with outside arrows, text just past an arrow tail (ARROW + 1 mm) is the fallback. Its witness origins and measured value stay fixed. Candidates cannot add
label/line contact, share an overlapping measured span on another dimension line,
cross a settled GD&T leader, or introduce a band overrun. Pins stay fixed;
an infeasible label keeps its reported collision. `tools/quality.py` counts detached text
independently as `text_off_span`, with item IDs; this metric adds no MCP problem rows.

Pinned annotations are obstacles to the free linear, leader and PMI solvers. Explicit pin ink
reserves additional carrier bands before free views are packed; the carrier origin and scale
stay fixed. Free linear labels start from their canonical settled geometry, then try slides,
other lanes and the opposite side when a pin blocks them. Pinned callout shafts attach radially
to the nearest proven feature site, as automatic leaders do. A collision or
sheet overrun remains a problem against the pinned ID; its geometry is kept, including pinned
hole items that ordinary drop-and-escalate would withdraw. A covered measurement stays covered:
`omit: false` and a pin do not bypass consolidation or repeat information. A pin that cannot be
drawn has `pinned_item_not_drawn`, alongside the reason in its typed state. Pinned view collisions
and overruns have `pinned_view_overlap` and `pinned_view_off_sheet`.

An explicit (unpinned) side or lane for a linear dimension, such as a proposed fix sets, re-runs
that side's stack spacing (#32, `dims::placement::respace`): when its line would share a
dimension line with another linear dimension over an overlapping span (an automatic item at
its lane, or another explicit one), it moves out lane by lane to the nearest lane of its own
that fits the page (else the nearest free lane, its overrun reported), shorter spans inside.
An item that collides with nothing keeps its requested lane. Two lines sharing one offset over
overlapping spans are `dimension_line_shared` (error, both IDs), and a linear label more than a
text height (3 mm) from its own dimension line is `dimension_text_stray` (error); structural
lint reports the same two codes from the drawn ink. A fix that leaves either is therefore a new
problem in the edit's delta, not a resolved one.

Structural lint (`review`'s `structural`) also checks that each label says what its ink draws
(stream K7, `lint::fidelity`): a linear dimension's value against the span between its extension
lines divided by its view's scale (`label_vs_measured`: warning past 0.5%, error from 5%, a label
correctly rounded at its printed places excepted; an `INCH` label in millimetres; `N× v` per
member, as the port draws counted dimensions), with the correctly rounded ones noted once per
sheet (`nominal_rounded`, info); an angle's degrees against its drawn rays, its arc and extension
lines, and its rays along visible edges of the view (`angular_label_vs_geometry`,
`angular_geometry_mismatch`, `angular_support_mismatch`, errors); and a PMI datum symbol more
than max(24 mm, 2.5 × its frame) from its surface (`datum_leader_remote`, warning). These are
findings, not plan problems: no wire format changes.

Lint (`review`'s `lint`) also checks each callout's leader against the part (stream K4,
`lint::leaders`), from its drawn leader, its view's recorded transform and the Analysis: a hole
callout's tip must lie within 0.002 mm of a boundary circle of a hole it names (a double-D bore's
trimmed profile, flats included) as projected in its view (`diameter_leader_target_mismatch`,
warning; `diameter_leader_target_unverifiable` when no named hole can be read), a straight
blend's radius callout's tip on that blend's trimmed profile arc seen end-on
(`radius_leader_target_mismatch` and `_unverifiable`, warnings), and no callout's shaft may cut
back into the part's material after leaving it at its tip (`leader_crosses_silhouette`, info,
with the depth cut). The material is each solid's projected box (`material::solids`) and the
measure `material::reentry`, the same predicate the leader router prices: a candidate that
re-enters is a hard blocker, its depth added to its length, and a chosen one is reported as
`annotation_line_over_view` with the depth in its message; a pinned callout's leader, which the
router does not route, is reported the same way ("The pinned leader of … runs back across its own
view, …, cutting … mm of it. Its bend and label are kept at the prescribed position.", with the
unpinning fix). A callout's `annotation_ink_overlap`
reads its leader's rendered ink, its arrowhead's flare included; a contact only the flare makes
is worded "The arrowhead of … reaches into the label of …".
These add findings and problem wording only: no wire format changes.

Placement problems provide `edit` operations addressing `place`. Resetting an annotation's
`place` to `{}` releases it to automatic placement; removing a view's `place` restores automatic
packing. Suggestions for free annotations choose another strip or label position and must be
reviewed by re-planning. Exports embed these entries, so extracting the Spec and re-rendering
reproduces the same geometry (`tests/placement_intent.rs`, SVG, DXF, PDF and PNG).

### Plan: what the sheet will say

`analysis_sha256` identifies the complete effective Analysis used for planning, including its
recognition records and recogniser, beside the STEP `binding` and embedded `spec`. It is a
SHA-256 digest of the deterministic JSON serialization, not merely the source-file hash;
changing `--features` therefore changes the run's provenance even for the same STEP and Spec.
Older saved plans read with no digest.

`feature_dispositions` reports each recognised occurrence without a dimension owner. Its
`subject` is a feature ID, or `recognition:evidence:<index>` / `recognition:refusal:<index>` in
the Analysis's same-run record arrays. Each informational entry has a closed `disposition`
(`unsupported`, `deferred`, `evidence_only`) and `reason_code`. The dimension planners and their
undrafted capability table supply the policy: a countersink is hole-callout evidence, while
unreviewed freeform, sheet-metal and similar grammars are deferred. Evidence-only step levels,
risers and repeating profiles keep their own reasons. These records do not remove completeness
or unsupported-feature warnings, and an authored omission is not an ownerless disposition.


`plan(analysis, spec)` produces it. The plan is the preview, and renderers draw only from it.

```json
{
  "schema": "draftwright.plan/1",
  "sheet": { "size": "A3", "scale": "2:1", "page": [420, 297] },
  "views": [ { "id": "v:front", "direction": [0, -1, 0], "at": [120, 180], "default": true,
               "basis": "front shows the most features" } ],
  "dimensions": [
    { "id": "d:f:hole:3/diameter", "view": "v:plan", "value": 8.1, "label": "ø8.1 H7",
      "from_spec": ["fit"], "default": false, "basis": "a hole's size is called out where it is seen end-on",
      "state": { "kind": "shown" }, "rebind": "carried" }
  ],
  "omitted": [
    { "id": "d:f:pocket:1/depth", "label": "↧ 5", "basis": "authored: the spec omits it",
      "state": { "kind": "omitted_by_user" } }
  ],
  "problems": [
    { "id": "p:tolerance_withheld:d:envelope/width.length", "code": "tolerance_withheld",
      "severity": "warning", "message": "±0 is not printed on d:envelope/width.length: …",
      "subjects": ["d:envelope/width.length"],
      "fix": [ { "op": "remove", "path": "dimensions/d:envelope~1width.length/tolerance" } ] }
  ]
}
```

- **Every item has a stable ID**, its resolved value, whether it is a default, and a `basis`
  sentence saying why it is there.
- **Every item has a typed `state`** (`src/items.rs`; the editor thin slice's S2), set where the
  plan decides it, so a caller never reads it from the `basis`: `{"kind": "shown"}`;
  `omitted_by_user` (the spec's `omit`) or `not_authored` (not listed in an authored set), in
  `omitted`; `covered` with `by`, the item that states it instead (a coincident location, a
  shared size, a boss height the overall extent states: `omitted`; a hole callout or location
  the hole table states: `dimensions`, unplaced); or `not_drawable` with `reason_code`, the code
  of the problem on its ID that says why it is kept undrawn (`view_plan_incomplete`,
  `callout_dropped`, `location_ref_dropped`, `off_axis_location_dropped`, `pmi_not_drawn`,
  `notes_not_placed`, `table_not_placed`). The sheet (its ID is `sheet`), the views, the hole
  table and the balloons are in the plan only when drawn, so always `shown`; a drawing-wide
  requirement carried by a general note is drawn as that note is. `tests/items.rs` checks every
  item's state against what the plan did with it, on the slice parts, every AP242 fixture and
  the quality parts.
- **One label formatter**: `items::items(plan)` lists every item once, with its kind, view, state
  and the text draftwright draws for it (`items::label`): a dimension's label, an omitted one's
  as it would print (`omitted[].label`), a frame's compartments joined by ` | ` (`gdt[].label`),
  a datum's letter, a requirement's text, a note's rows, a balloon's tag.
- **Every problem names its subjects** and, where one exists, an edit that would fix it.
  `subjects` are live item IDs only: the plan's items (the sheet as `sheet`), the Analysis's
  features and, for geometry no feature claims (`unclaimed_geometry`), the part's faces as
  `face:<index>`. `sheet` is the subject only of a problem about the paper, the scale, what fits
  on the sheet or the title block drawn on it (#25). What else a problem names is in `refs`: a PMI record's source (its Part 21
  provenance), a source not read, a recogniser family, a view not drawn (`v:section_aa`), the
  height ladder (`step_level`), the whole part (`part`, for PMI not read), a title-block cell (`title_block/<field>`, the sheet its
  subject), a spec entry not applied, a thread a schedule states. `fix` is a list of `edit`
  operations that resolve the problem, given only where one does: a band of nothing removed
  (`tolerance_withheld`); an omitted dimension brought back, else its tolerance or fit removed
  (`tolerance_not_shown`); an omitted view restored (`dimension_view_not_shown`,
  `view_plan_incomplete`, `pmi_not_drawn`, `section_not_shown`) or the omitted dimension
  stating a PMI dimension brought back (`pmi_not_drawn`); a section with no plane un-asked
  (`section_not_shown`); an empty isometric omitted (`iso_projection_empty`); another revision's entries removed (`spec_entries_inactive`). Placement
  problems use `place` (above). Non-placement fixes are applied and re-planned in
  `tests/items.rs` (the isometric's in `tests/views.rs`), and the problem is gone. Where the
  turned-part rule, not the spec, dropped a view, the message says so and gives no fix (restoring
  an omission the spec never made would fail).
- **A problem's message is in drawing language** (#24; `src/wording.rs`): it names a dimension
  by the text it prints (`2× 30`), a feature by what a draftsman sees of it (`the ⌀8 hole`), a
  view by its name (`the plan view`), a note by its text; never by an internal ID (`d:`, `f:`,
  `v:`, `pmi:`, `n:`, `t:`, `face:`), which stay in `subjects` and `refs`. A PMI record not
  drawn is still named by its source (`dimensional_size:#123`), as it is no item. Messages are
  for people; nothing should parse them. `tests/problem_wording.rs` scans every message of the
  test parts and AP242 fixtures.
- **Nothing the plan approves vanishes silently.** An item that can't be drawn is reported
  against its ID, never dropped. This is carried over from the Python version's ADR 4
  invariant 9.
- **Automatic sheet and scale are chosen on finished drawings** (maintainer, 2026-10-12,
  H6; `model::plan_recovered` / `model::recover`, policy in `sheet_fit::choose`). Each
  standard sheet and scale at or above the legibility floor is planned and judged. A
  candidate is valid and complete when it has no hard layout finding, its drawn items and
  views fit inside the border and off the title block, it drops or loses no measurement
  the first plan carried, every GD&T item is drawn, and it lays no more annotations over
  another view than the first plan. The smallest fit is the smallest sheet with a valid
  complete candidate, at its largest such scale. Only that pair and the next larger sheet
  at a larger scale compete. Against a valid first plan, an allowed pair that adds material
  crossings (a leader or witness through a feature) is set aside; this never changes which
  pair is the smallest fit, and if every allowed pair adds them the valid first plan is
  kept. Among the rest, in
  `sheet_fit::FILL_BAND` (initially 60–85%) the larger scale wins, then the smaller sheet;
  if none reaches the band, the fullest wins. A Spec-named sheet keeps its sheet and
  chooses its scale by the same ranking; a named scale or pinned placement is not
  re-planned. On the larger neighbour the orientation view keeps the smallest fit's
  paper scale, so the extra paper goes to the dimensioned views.
  `sheet.fill` is the union of each drawn item's bounding footprint (view envelopes,
  dimensions with witnesses, leaders and arrows, GD&T, notes, tables, balloons and the
  scale/projection/section furniture), with 1 mm clearance, inside the border and
  excluding the drawn title grid. Reserved bands never count. `sheet.basis` lists every
  measured candidate's fill and findings. The legibility floor
  (`sheet_fit::min_legible_scale`) keeps the smallest hole or slot opening at least
  0.8 mm and the longest extent at least 20 mm on paper, and never goes below 1:10. If
  no candidate is valid and complete, the first drawing and its findings are kept on
  its own sheet, with `layout_recovery_exhausted`: a warning when that drawing is below
  the legibility floor, otherwise info. Only a first drawing below 1:10 is redrawn at
  1:10 on its sheet. A GD&T layout may also be composed from settled view blocks of a
  same-scale A0 template, moving witnesses, leaders and frames with their view; one
  measured correction may lift a free projection row and refit its orientation view.
  Every title-block collision lifts the row, even when the resulting overflow must be
  reported. Nothing is clipped.
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

The title block also carries the resolved third-angle projection symbol in
`sheet.projection_symbol` (method and paper curves). Native view planning uses third
angle; first-angle planning remains unsupported. Rendering draws these planned curves
in the reserved band beside the scale note. Older plans may omit this optional field.
A blank GEN. TOL. cell is `general_tolerance_missing` (warning, subject `sheet`), with
a set fix at `sheet/title/tolerance`. The offered `UNSPECIFIED` explicitly acknowledges
the absence without asserting manufacturing limits; supply the intended requirement
to state limits. A nonblank Spec tolerance or the unique usable PMI tolerance fills
the cell. A Spec material different from the unique usable STEP material is
`step_material_disagreement` (warning, both values and source/path refs); the Spec
value still wins, including an explicit blank.

### Centre furniture

Each shown principal or section view carries automatic `line_work` in paper
coordinates: provenance ID, kind (`cross`, `axis`, `pitch_circle`, `cutting_plane`)
and exact curves, with a centre for end-on crosses and a `global_axis` flag only
for the whole turning axis. It contains no values and is not Spec-addressable.
Hole, boss, turned-band and circular-seat geometry supplies it independently of
dimensional omissions. Bolt-circle crosses run radially and tangentially.

The centre layer uses thin black long-dash-dot ink; a long dash meets at the
cross centre. Crosses extend approximately 3 mm beyond the edge, proportionately
less for very small holes and more for very large ones. Local axes extend beyond
their ends; the whole turning axis extends 5 mm. Visible and hidden ink is painted above
centre ink; this is paint order, so chain ink can remain in hidden-dash gaps. Sections omit axes wholly on the removed side and retain axes of
cut holes and cylinders.

This furniture, including the finite cutting-plane line, its arrowhead/wing
outlines and both letters measured in the rendering font, enters the existing
fixed-ink placement hook before dimensions and PMI. Labels and travel along a
line are hard conflicts; transverse crossings are routing costs, with only a
local transverse tip exemption on an explicitly marked global turning axis.
Furniture provenance appears in problem `refs`; its carrier view is a subject.
Drawing lint checks the actual marked centres against independent projections
of the Analysis's holes, including pattern members and double-D bores. Structural
review separately counts collinear leader travel along fixed chain strokes and
actual labels touching section letters or arrows; quality includes those in ink.
A callout label that must stay across the end of a hole axis (its leader search
found no place as good under the ordinary-ink and feature-crossing floors) is not
moved off its shelf: the axis's overrun past the feature is shortened instead
(ISO 128-24), stopping 0.5 mm short of the label and keeping at least 1 mm past
the feature (`centrelines::trim`, after dimension placement). A label across the
feature's own span is never trimmed around; lint still reports it.
A leader shaft running within 1 mm of a centre stroke at under ten degrees, for
more than 0.5 mm, reads as part of the centre line (`centrelines::runs_beside`).
Exact collinear travel is a hard blocker; near-parallel shaft travel is weighted
just below one crossing, so it is avoided whenever that costs no crossing. A
radial leader leaving its own hole along that hole's centre-mark arm is exempt.

### Dimensions (first cut)

`src/dims/` plans the dimensions draftwright's defaults would state for the families ported
so far, each a Plan item `{id, kind, view, value, label, terms, measurements, default, from_spec,
basis, placed, reference?, determined_by?}`:

- **Overall extents:** `d:envelope/width.length` (in the front view), `depth.length` (side),
  `height.length` (front) for a prismatic part (each falling back to the plan or side view when
  its view is omitted). A part turned whole states `od.diameter` (printed `ø…`) and its length along
  the axis instead, in either profile view; the diameter is drawn across whichever radial axis
  that view shows. A view that looks along a dimension never carries it.
- **Turned step lengths** are a chain of `d:f:turned_step:<n>/step.length`, except on a part
  turned whole whose PMI measures the profile from one end (#33): a planar datum feature on that
  end face, or PMI lengths from it to the profile's shoulders. The profile is then a baseline from
  that end: the band at the datum keeps its `step.length`, each further shoulder is
  `d:f:turned_step:<n>/step_position.length` (the distance from the datum to the shoulder that
  ends band `n`), and the overall length is the last rung (it also claims the last band's
  `step_position.length`), so no rung is reference. The other bands' `step.length` items stay
  in the inventory (a spec may name them) and are `covered` by the rung locating their band,
  unless the spec tolerances them or omits that rung. A PMI length across the same span takes its
  rung's place. Lint counts a band's length as placed when a baseline locates both its
  shoulders.
  On a part of several turned bodies along one axis (stream K5, ARCH0020-2), each body's
  chain goes in the profile view that separates its axis line from the others' and its witnesses
  start at its own silhouette, on the side of the view it is nearer; a groove whose record names
  none of the part's turned profiles is refused (`feature_not_dimensioned`, ADR3-12). Lint
  credits a grouped length (`3× 10`) only when it prints exactly the count of the adjoining bands
  it claims.
- **Raised pads** (`src/dims/pads.rs`, stream K5): `d:f:pad:<n>/pad_width.length` and
  `pad_length.length` between the pad's own edges, `pad_height.length` as a callout `5 HIGH`, and
  `location_pad.location.x|y` (a pad along z) or `location_pad.<axis>` from the part's corner to
  its centre, all in the view that looks down on it; a direction on the datum is an info
  `location_on_datum`.
- **Free-direction slots** (`src/dims/oriented_slots.rs`, stream K5): one callout
  `d:f:oriented_slot:<n>/oriented_slot_width.length`, `ORIENTED SLOT 2 WIDE × 4.5 LONG`, claiming
  width and length, in the view along the slot's dominant run axis, its leader to the slot's rim;
  equal slots in one view share a counted callout.
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
    of the callout that states the feature's size. The default is the class alone
    (`2× ⌀6.3 H7 ↧ 12.7`, `ø44.5 h6`), including classes imported from AP242 PMI.
    `sheet/fit_deviations: true` adds ISO 286 deviations stacked upper above lower after
    the class, at 70% text height (ISO 129-1). Signs and upper/lower order are retained;
    zero prints `0`. Plan labels and searchable text encode that stack with a newline,
    for example `ø10 g6 -0.005\n-0.014`. This applies to turned diameters, hole fits,
    spec fits, editor previews and MCP output. Explicit ± and band tolerances are unchanged.
    The legacy `sheet/fit_limits` spelling remains accepted; `fit_deviations` takes precedence
    when both are set. A fit on any other feature, on a hole pattern (set it on its holes), on a feature the
    part does not have, of the wrong case (`"h7" reads as "H7"`) or outside the table is
    refused; so is a size given both a tolerance and a fit.
  - The item lists what the spec gave it in `from_spec` (`tolerance`, `fit`) and is not a
    default. A band is withheld, with a `tolerance_withheld` warning on the item's ID, when it
    is a band of nothing (`±0`: ARCH0019-2; draftwright prints `±0.0`) or when an `n×` callout's
    members do not all have the same fit (ARCH0019-3; draftwright splits such a group of bosses
    instead). A tolerance on a dimension the plan leaves out, or a fit on a feature no planned
    callout states, is a `tolerance_not_shown` warning on its ID. Across a revision, feature
    entries are kept inactive like the other feature-keyed entries.

Plate and wall thicknesses (`src/dims/plates.rs`, ISSUE-1373-a) are ordinary
`d:f:plate:<n>/thickness.length` items: z slabs on the front-left strip, y walls
above the side view and x walls below front. Equal thicknesses on one recognised
body share a counted statement with every slab's claim. Proven envelope, step,
turned-band and channel statements may cover an item; supporting polygonal slabs
and slot-pattern webs require physical ownership evidence. Single-axis native
slabs are retained when the ladder does not already state their interval. Plate
coverage (`src/lint/plates.rs`) checks claimed, printed nominal values independently;
zero-ink `suppressed:<id>` consumer annotations record authored omissions without
stating a thickness or letting lint read the Spec. Outcomes use
`plate_requirement_missing`, `_suppressed` and `_unverifiable`.

Each family is a planner module in `src/dims/` (`envelope`, `turned`, `holes`, `slots`, `bosses`,
`edges`, `pads`, `oriented_slots`, …), listed in `dims::PLANNERS` with the feature kinds it dimensions. Every other recognised
feature gets one warning problem, `dimensions_not_ported`, on its
ID: draftwright dimensions those kinds and the port does not yet, so this is a gap in the port,
not in the part. Record types draftwright itself leaves undimensioned (freeform surfaces,
thin-wall and sheet-metal bodies, …; `detect.py`'s unconsumed and orchestrated records) are not
reported, except an angled step: draftwright cannot dimension one, so it gets a warning
`angled_step_requirement_unsupported` on its ID (ISSUE-1247-a; lint's code and wording). That
Problem is `p:angled_step_requirement_unsupported:<feature id>` (`src/dims/mod.rs`,
`unsupported`), by the maintainer's decision of 2026-10-09 (stream d-angled-step-unsupported),
until an angled step is dimensioned or the port has a completeness report.

One measurement has one owner (`src/dims/redundancy.rs`). Matching physical
support-plane intervals share a surviving statement and a typed `covered` omission;
a functional outside diameter or A/F can own an equal overall interval. Equal values
alone prove nothing. A stated overall length keeps one otherwise redundant chain
member as a parenthesized reference dimension; a member the spec tolerances, or one that states
the part's PMI, is never the one (#33). Independent drawing lint reports
`duplicate_measurement` and `closed_dimension_chain` from printed claims and Analysis
stations, excluding parenthesized references. Point ordinates, height-ladder datums
and counted/compound statements retain their existing ownership contracts. Reference
choice does not change when another member gains a tolerance. Omission can make an
unpinned reference controlling, reflowing its text at the same centre and witnesses.
A pinned reference keeps its ink and raises `pinned_reference_chain_changed` if its
controlling chain is edited; its offered fix releases the pin. A dimension drawn in brackets
carries `reference: true` and, in `determined_by`, the IDs of the dimensions that determine it,
in the order its basis names them: a chain's stated total and other members, the stated overall
and slot length for a floor interval, a slot's width and location for a shoulder on its wall
(#26; `tests/problem_wording.rs`). Both fields are optional on the wire and absent on a
controlling dimension; a pinned reference kept after its chain was edited has `reference: true`
and no `determined_by`. A flush boss height is credited from
the same stated envelope interval, without reading the omission.
An authored set decides what is drawn, not where the geometry states a fact (stream K6,
ISSUE-1154-a): an item the set leaves out that the automatic plan would cover by an owner the
set keeps is `covered` by that owner, with the automatic basis, rather than `not_authored`, and
the owner carries the same claims; an owner the set leaves out takes nothing. A boss height
closed by a stated ladder rung and the overall height (a boss standing on a face level) is the
chain's reference member, determined by the rung and the overall height. Lint reports a
bracketed label that prints a tolerance or a fit (`toleranced_reference`: a reference dimension
is only for a redundant, untoleranced one) and a linear or grid pattern's pitch printed beside
every member's own location along that axis (`over_dimensioned_pattern`, ADR4-U1).

The Plan also carries `status` (`clear` or `needs_attention`) and `completeness`:
`claimed_faces`, `faces` and `unplanned_features`. Face claims are the distinct valid
face indices on Analysis features, measured against `binding.face_count`; they describe
this Analysis's recognition inventory, not recognition recall. Optional `stock_faces` records principal boundary plane face IDs. On a single solid,
those faces receive credit when printed measurements prove all overall extents;
unrecognised interior/curved faces can still be unclaimed. Older analyses default
that evidence to empty; a six-face rectangular box can be proved from its profiles. An `unclaimed_geometry` warning names both counts and asks for review,
without inventing missed features. A recognised feature with no drawn measurement is
reported at warning; absorbed groove floors and bands, pattern members, dimensions
carried by a hole table, and the existing deliberately undrafted record types retain
their established dispositions. `unplanned_features` counts whole features, not each
missing parameter; independent lint covers missing extents, band sizes and pattern
locations/pitches.

New analyses include optional `solid_count`. Several solids make these completeness
findings informational, following Python's assembly coverage policy. Older analyses
without the count are conservatively assessed at warning. `clear` means no assessed
Plan problem needs attention (warning or error); it does not mean manufacturing readiness.
The inspect summary additionally includes render problems in that status. Independent
coverage lint still lives in `review.lint`, rather than being inferred from the Plan.

`lint::check` now runs the envelope and turned-step ledgers as findings:
`missing_principal_dimension`, `axial_length_missing`,
`turned_step_requirement_missing` and `turned_step_requirement_unverifiable`.
Pattern owners have `hole_pattern_requirement_missing` or
`hole_pattern_requirement_unverifiable`, with each location axis or pitch named in
`requirements`. These checks read Analysis geometry and printed drawing marks only.
The ledger-to-finding extension is a deliberate Python difference, authorised for G1;
its verdicts are in `tests/fixtures/known_lint.json` and
`tests/fixtures/known_completeness.json`. No dimension planner changes for these checks.

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
  per-feature `note` is drawn as a leadered label since K1b ("Finish, threads, knurls and
  notes authored in the Spec", below); hole tables are not ported here.

### External spur gears

`gears` is an ordered list of authored manufacturing requirements (ISO 21771-1:2024,
ISO 21771-2:2025, ISO 53:1998, ISO 54:1996 and ISO 1328-1:2013):

```json
{"gears": [{
  "on": "f:boss:1",
  "frame": {"origin": [0, 0, 0], "axis": "z"},
  "tooth_count": 13, "module": 1.25, "pressure_angle": 20,
  "profile_shift": 0, "face_width": 10,
  "tooth_thickness": 1.9634954084936207,
  "tooth_thickness_tolerance": [-0.03, 0.01],
  "flank_tolerance_class": 7
}]}
```

Every field is required. `on` anchors ownership to a cylindrical `boss`,
`cylindrical_boss` or `turned_step` feature, a `face:<n>`, or a named stock face
(`top`, `bottom`, `left`, `right`, `front`, `back`). Use a face anchor for a toothed
wheel. `frame` explicitly authors the gear centre and principal axis in the part's
coordinates, in mm; axis letters accept either case. The axial span is the centre
coordinate on that axis ± half the face width. Anchors on another revision are
refused until reattached.

Validation follows Python's `ExternalSpurGearFeature`: finite origin; integer tooth
count 5–1000; positive finite module, face width and tooth thickness; module
0.5–70 mm; width 4–1200 mm; reference diameter `module × tooth_count` 5–15000 mm;
finite pressure angle strictly between 0 and 90 degrees; finite profile shift;
two finite tooth-thickness deviations, lower ≤ upper, retaining a strictly positive
lower thickness limit; integer flank tolerance class 1–11. Booleans are refused
as numeric values. Deviations are lower then upper, in mm.

A declaration creates `gear_data_1`, `gear_data_2`, … table items. Each prints the
header and Python's nine fixed ISO rows with exact authored float formatting,
units, and signed deviations. These tables reserve space through the sheet rules
and are placed towards the top right as late furniture, clear of dimensions,
notes, captions and earlier tables. They carry rendered provenance to `gear:1`,
`gear:2`, …; they are never recognised from geometry. Edit the whole `gears` list
with `set`, or append a declaration with `add` on `gears`; the MCP Spec schemas
expose the same list, like `notes` and `tables`.

Lint reads declarations lowered into the Analysis's independent requirement
inventory and the actual rendered table text, without reading the embedded Spec.
It reports missing, ambiguous or stale tables. Full-wire repeating-profile evidence
from native or Python recognition is matched by centre and axial span (1e-6 mm),
then audited for axis and repeat count. Missing, displaced or ambiguous evidence
is `gear_correspondence_unverifiable`; an undeclared repeating profile is
`gear_semantics_missing`. Arc counts alone never prove correspondence. A smooth
blank can carry a declaration and table but remains unverifiable, as in Python.
Python imposes no derived tip-diameter or face-width agreement with a blank;
the port adds no such refusal. `tests/gears.rs` covers the blank, real wheel,
validation, export and all reconciliation outcomes.

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
`src/dims/gdt.rs` places and draws it. Plan items (`gdt`): `{id: "pmi:<identity>", kind:
"control_frame" | "datum_feature" | "label" | "surface_finish", view, side, source, faces, cells,
modifiers?, default, basis, placed}` (`surface_finish` added by F2, draftwright-rust #18).

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
  to, from a proven site on its face. A cylinder may use either silhouette proved against
  the trimmed face; a plane may use a drawn extension of its edge-on face beyond either end
  of the view outline, with the triangle standing on that extension. Drawn as ISO 5459 draws it:
  the filled triangle on the surface, a line along the normal, the letter in a square frame.
- **A surface texture symbol** (`surface_finish`) for each face's AP242 surface texture (ISO
  10303-1110, which draftwright does not read): attached as a datum's symbol is (its faces all
  planes or all cylinders on one principal plane or axis, from a proven site on the face's edge
  line in the view that sees it edge-on, on its outward side), drawn as ISO 1302's symbol
  standing on its leader's shelf, the leader's arrowhead on the edge line, the material removal
  condition's bar or circle, and its parameters under the bar, one per cell and line (`Ra 0.8`,
  µm). A face no principal view shows edge-on (a cone, a tilted plane) is `pmi_not_lowered`; one
  whose view is not drawn is `pmi_not_drawn`. It keeps its outward strip.
- **Placement**: automatic datums first try inside tiers between their face and the first
  dimension lane, with normal triangle, shaft, frame and lane clearances. All inside sites
  are searched before the existing tiers beyond the dimensions. Cylinder alternatives use
  either proven silhouette; planar alternatives draw an extension outside either end of the
  view, with a 1 mm outline gap and overrun past the triangle. Sites nearest the original win
  equal grades. Frames and labels also retry the other strips. Every glyph's obstacle grade
  includes the count of crossings with dimension and extension lines, arrow shafts and other
  leaders, using the collision reporter's predicate (including collinear travel). Grade zero
  requires zero crossings; equal clearance classes prefer fewer crossings. The original
  complete arrangement remains a candidate. A replacement cannot introduce an annotation
  contact with a new unordered pair of subjects (reporting direction does not affect the
  comparison), so a greedy move cannot crowd later frames into overlaps. If the full
  rearrangement fails this guard, datum changes are tried alone. Unavoidable
  collisions stay visible and are reported. Notes and tables keep clear of the frames.
  Explicit coordinates retain their strip/tier behavior; pins keep their resolved geometry.
  An empty `place: {}` reset restores automatic placement, including inside datum tiers.
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
`tests/fixtures/known_pmi_lowering.json`). Datum placement's
[base/head survey and render evidence](datum-placement.md) records the corpus/fixture
crossing and overlap counts and the spindle's inside-lane and face-extension placements.

### Datums and controls authored in the Spec

A part whose STEP carries no AP242 PMI can still carry datums and feature control frames
(`src/spec_gdt.rs`, draftwright-rust #43 item 1; Python draftwright's `Sheet.datum` and
`Sheet.control` are the reference):

```json
"faces":    { "bottom": { "datum": "A" } },
"features": { "f:hole_pattern:1": { "controls": [
  { "characteristic": "position", "tolerance": 0.1, "datums": ["A"] } ] } }
```

- **Targets.** `features/<id>` names a recognised feature or hole pattern by its ID, as
  `features/<id>/fit` does; a datum or control on a feature with cylinders applies to its
  cylinders (its axis, as draftwright's `gdt_target` takes a feature's), else to all its faces.
  `faces/<face>` names a face by the side of the part's box it lies on, facing out (`bottom`,
  `top`, `left`, `right`, `front`, `back`: -z, +z, -x, +x, -y, +y; every planar face lying
  there), or by its index, `face:<n>`, as the plan's problems name faces.
- **`datum`**: a datum letter, one to three capitals (ISO 5459).
- **`controls`**: a list of `{characteristic, tolerance, datums?, diameter?, modifier?}`: one of
  draftwright's 14 characteristics (`straightness`, `flatness`, `circularity`, `cylindricity`,
  `profile_line`, `profile_surface`, `angularity`, `perpendicularity`, `parallelism`,
  `position`, `concentricity`, `symmetry`, `circular_runout`, `total_runout`), a positive
  magnitude in mm, the datum letters in precedence order (none on a form tolerance), a `⌀` zone
  (the default for position and concentricity; accepted on straightness and the orientation
  characteristics too, refused on the others) and `M` or `L`. `add` appends a control.
- **Lowering.** Each is lowered into the same record (`pmi::Record`) an AP242 datum or geometric
  tolerance on those faces is read into, its attachment measured as `src/pmi.rs` measures it,
  from the Analysis's `face_attachments` (below), and merged into the Analysis's PMI reading
  (`spec_gdt::effective`, once per plan, and again by `api::review` for lint, which reads no spec).
  Lowering, placement, band reservation, drawing and lint then treat it as the file's own: its
  item is `pmi:<identity>`, `annotations/<id>` places or omits it, its `source` is
  `spec:<path>` (`spec:faces/bottom/datum`), its basis says `spec-authored` and it is not a
  default.
- **The Spec wins.** Where the file's PMI states the same (a datum of the same letter; a
  tolerance of the same characteristic on the same faces), the file's record is not drawn and a
  `spec_overrides_pmi` warning names both (`refs`: the AP242 source, then the spec's). The same
  requirement on the same faces keeps its item ID, so its placement carries over.
- **Reported:** a control referring to a letter neither the spec nor the file declares is drawn
  with a `datum_undeclared` warning (draftwright's `_validate_datums`); an attachment the faces
  do not prove is `pmi_not_lowered`, as for the file's records. **Refused** (`invalid`): an
  unknown feature (with the nearest IDs) or face, a characteristic, letter or modifier not listed
  (`P`, a projected zone, is not drawn for AP242 PMI either), a tolerance not positive, one letter
  on two targets, two controls of one characteristic on the same faces, and an Analysis without
  face attachments (analyse again). Across a revision, the entries are inactive like the other
  feature-keyed entries.

The Analysis carries `face_attachments` (added to version 7 as an optional field): per face,
its box, its surface (`plane` or `cylinder`), principal axis and support, its proven sites
(first, then up to eight spread over it), a plane's outward normal and a cylinder's exact
principal axis: what `pmi::datum_geometry` and `tolerance_geometry` read of the part, which they
now read through the same record for the file's own PMI. `tests/spec_gdt.rs`.

### Finish, threads, knurls and notes authored in the Spec

The rest of what a part without PMI may need stated (`src/spec_notes.rs`, draftwright-rust #43
item 1, stream K1b; Python draftwright's `Sheet.finish`, `thread`, `knurl`, `note` and
`default_surface_finish` are the reference) is authored the way datums and controls are (above):

```json
"sheet":    { "default_finish": { "ra": 3.2 }, "title": { "tolerance": "ISO 2768-m" } },
"faces":    { "top": { "finish": { "ra": 0.8, "removal": "required" } } },
"features": { "f:hole_pattern:1": { "thread": { "designation": "M6" } },
              "f:turned_step:1":  { "thread": { "designation": "M10", "length": 15 } },
              "f:turned_step:3":  { "knurl": { "pattern": "diamond", "pitch": 0.8 } },
              "f:hole:1":         { "note": "DRILL AND TAP AFTER HARDENING" } },
"notes":    [ { "id": "n:1", "text": "BREAK ALL SHARP EDGES" } ]
```

- **`finish`** (`features/<id>`, `faces/<face>`): `{ra, removal?}`, Ra in µm, `removal` one of
  `required` (ISO 1302's bar), `prohibited` (its circle) or `any` (the default). Lowered into an
  AP242 face surface texture's record (`surface_finish`, on the feature's cylinders, else its
  faces) and drawn as F2 draws one: the symbol on a leader from the face's edge in the view that
  sees it edge-on. Faces no principal view shows edge-on, or several cylinders on different axes
  (a hole pattern's), are `pmi_not_lowered`, as for the file's own.
- **`sheet/default_finish`**: the same, the part's default finish: the record of an AP242
  surface texture on the part (`Ra <v> µm unless otherwise specified`), drawn beside the title
  block (`requirements`, `carrier: sheet`).
- **General tolerance**: `sheet/title/tolerance`, as before (it fills the title block's cell and
  wins over the file's, `step_general_tolerance_disagreement`). No new path.
- **`thread`** (`features/<id>`): `{designation, length?}`, an ISO metric designation
  (`M<size>[x<pitch>][-<class>][ LH|RH]`, printed as written; with no pitch, ISO 261's coarse
  pitch is used for the checks) and a minimum full-thread length in mm. On a hole the callouts
  state on its own, a hole pattern, or a turned step (or boss), it is lowered into an AP242
  thread's record (`internal_thread` on a hole or pattern, `external_thread` on a turned
  diameter) and printed on the owner's callout by the thread path: after a hole's terms
  (`⌀4.2 ↧ 9.2 M5x0.8-6H; 8 MIN FULL THREAD`), before a pattern's suffix (`4× ⌀6 THRU M6 (2×2)`,
  as draftwright folds a pattern member's thread), after a turned diameter
  (`ø10 M10; 15 MIN FULL THREAD`); the callout claims it. Reported (`pmi_not_lowered`, error):
  a member of a pattern the callouts state as one (set it on the pattern), a feature no callout
  states on its own, a hole larger than the nominal size or smaller than nominal less 1.5
  pitches, a turned diameter that is not the nominal size, a length past a blind hole's depth or
  a step's length (ISSUE-1360-a), a boss (as for AP242 threads).
- **`knurl`** (`features/<id>`): `{pattern, pitch}` (`straight` or `diamond`, mm), on a turned
  step, lowered into an AP242 knurl's record and printed after its diameter
  (`ø24 DIAMOND KNURL P0.8`; on the overall diameter when the step's is shared with it).
- **`note`** (`features/<id>`, `faces/<face>`): one line of text on a leader, lowered into a
  surface label's record (kind `note`, its leader from each face's first proven site) and drawn
  as an AP242 label is (`gdt` kind `label`). It claims no measurement, so it never silences
  coverage (ISSUE-1351-a). A note on the sheet is a general note (`notes`, above).
- **Lowered and merged** with the datums and controls (`spec_gdt::effective`): item IDs are the
  records' identities (`pmi:manufacturing_requirement.surface_finish:f2`,
  `pmi:surface_label.note:f3:t=…`), sources `spec:<path>`, bases `spec-authored`, never defaults;
  lint judges the drawing against the merged reading.
- **The Spec wins** with `spec_overrides_pmi` (warning) over the file's finish on any of the same
  faces, its default finish, and its thread (of either kind) or knurl on any of the same faces.
- **Refused** (`invalid`): an unknown feature or face, Ra not positive, an unknown removal or
  knurl pattern, a designation that is not ISO metric or states no pitch where ISO 261 has
  none, a length or pitch not positive, an empty or multi-line note, a thread on a feature of
  no threadable kind and a knurl on one that is not turned. `tests/spec_notes.rs`.

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
by its owner. Otherwise it **stands beside** them, an ordinary planned dimension `d:pmi:<identity>/
<kind>[.basic]` (a length along x, y or z in draftwright's view; a diameter or radius by a
leader where it is seen end-on), its label the record's, a basic one boxed (its label box widened
to take the frame). Every item that states a PMI dimension claims it (a measurement of feature
`pmi:<identity>`, its record's item ID); a spec's own tolerance or fit on that size (a hole's or a boss's) wins
(`pmi_not_lowered`; when the spec's is withheld, the label states neither and the report says
so), and an omitted item's PMI is `pmi_not_drawn`. Not drawn, with the reason: reading blockers, faces in
several placements, faces that prove no witness (`pmi_not_lowered`); angles, oblique lengths,
non-principal cylinders (`pmi_dimensions_not_ported`). An ISO 286 class stated without limits is
printed with its limits from draftwright's table on the item that owns its size: a hole class
on proven internal cylinders on the one bore callout they match, a shaft class (`g6`) on proven
external cylinders on the one turned diameter they match (`⌀15 g6 (-0.006/-0.017)`, #30); with
no unique owner, or a class of the other kind for the cylinder, it stays refused. The open
question of supplemental geometry stays refused. Compared record by record on the 14 AP242 files
(`tests/pmi_dimensions.rs`, `tests/fixtures/known_pmi_dimensions.json`).

**PMI coverage lint** (`src/lint/pmi_coverage.rs`, draftwright's `linting/pmi_coverage.py`): lint
checks the finished drawing against the Analysis's PMI, never the Plan. A record is drawn when an
annotation is named `pmi:<identity>` (a frame, symbol, label or sheet requirement) or claims a
measurement of feature `pmi:<identity>`, each read back to its record's source through the
Analysis's identities (a dimension, hole callout or turned size stating a PMI
dimension); otherwise it is `pmi_not_lowered` or `pmi_not_rendered`. A record lowers when
`pmi_lowering::lower` (a function of the Analysis alone) lowers it, a frame, symbol or label, when
`pmi_lowering::sheet_requirements` lowers it to the sheet (a general tolerance, default finish or
document note), or when it is a dimension of a kind stage 2b draws (a length along x, y or z, a
diameter or radius seen end-on) whose reading and faces prove it, or when it is a thread or knurl
`pmi_lowering::manufacturing` gives to its owner (whose callout then claims it); a face finish or
angle does not yet. Not lowered, it is `pmi_not_lowered` with the reason; lowered and not drawn,
`pmi_not_rendered` (reported even when the plan already says why, as lint cannot read it: an open
question). A source not read, or read in part, is `pmi_not_extracted` (error; info for a source of
no draftwright category); a refused reading is `pmi_not_extracted` on the part. A drawn frame
whose text runs do not state its source's magnitude, zone and datum letters is
`pmi_value_mismatch`; an annotation claiming a source the reading does not hold is
`pmi_source_unknown`, and one claiming a source on a part with no reading `pmi_unreconciled`
(warning). The parts' materials (`pmi::Report::materials`) give `step_material_ambiguous`
(several) and `step_material_unavailable` (one part's of several). Compared with draftwright's
lint on the 14 AP242 files, each drawn as the command line draws it (Rust quiddity's features, the
default spec) (`tests/pmi_coverage.rs`, `tests/fixtures/known_pmi_coverage.json`).

### PMI labels and drawing-wide requirements

A **surface or common label** (text on edges or faces: `A`, `F1.1`, `2X R`) is a `gdt` item of
kind `label`, its one cell the text, hung from a leader like a frame (draftwright's `Note`): from
its first proven site (`geometry.sites`: a surface label's edge at its parameter midpoint, a
common label's face at the vertex nearest the face's box centre, `pmi_lowering::label_geometry`)
in the front view, or the side when its references' box is longest along y, above or below by
the site against the part's centre; with no site it is `pmi_not_lowered`. The **drawing-wide
requirements** (`pmi_lowering::sheet_requirements`, draftwright's
`lower_ap242_document_requirements`) are the plan's `requirements`: `{id: "pmi:<identity>", kind,
source, text, carrier, basis, placed?}`. The part's one general tolerance fills the title
block's tolerance cell (`carrier: title_block`) unless the spec states one, which wins (a
different PMI one is `step_general_tolerance_disagreement`, a warning); its one `Ra <v> µm unless
otherwise specified` is ISO 1302's symbol at the left end of the band above the title block
(`sheet`, its collisions reported as a dimension's), as is an AP242 surface texture on the part
itself (F2: read as that default, its material removal condition drawn as ISO 1302's bar or
circle, `placed.arcs` holding the circle; the field is absent otherwise); each edge condition on no face is a general
note `pmi:<identity>` after the spec's notes (`notes`). Two of one default are ambiguous and
neither is drawn (`pmi_not_lowered`). Each is an annotation of the drawing by its ID, over the
runs that state it. A turned part keeps a view a PMI item is drawn in (draftwright's
`annotation_view_required`). Compared with draftwright's drawings by `tests/pmi_drawings.rs`.

### Threads and knurls (PMI)

An AP242 thread or knurl is no item of its own (`pmi_lowering::manufacturing`, draftwright's
`lower_ap242_manufacturing_requirements` and `manufacturing_schedule`): read from its words and its
structured statement, reconciled, its one cylinder the size its text states, it is given to the one
hole (blind to the drill depth, or through) or turned step that cylinder proves, and that
feature's callout prints its suffix after the size (`⌀1.6 ↧ 4 M2 x 0.4-6H RH; 2.5 MIN FULL
THREAD`; a through tap's `THRU` stands for the bore's) and claims it (feature `pmi:<identity>`,
parameter `thread` or `knurl`). With two or more owned and one suffix over 52 characters, each
callout prints `SEE MFG n` instead and the plan's table `manufacturing_requirements` (`default:
true`, `REF | MANUFACTURING REQUIREMENT`, the text wrapped at 44 characters, the second column
left-aligned) states them in their source IDs' order, reserved when the sheet is chosen and placed
before the notes; with no place it is `table_not_placed` (draftwright prints the suffixes in full
instead: an open question). A threaded owner keeps a callout of its own. No owner, several owners,
or text that disagrees with the cylinder is `pmi_not_lowered` (error) with draftwright's reason; a
requirement a boss owns is refused too, as the boss callout prints none yet
(`tests/pmi_thread_knurl.rs`).

A semantic AP242 `thread` (haecceity's, with its 'thread manufacturing' attribute set: pitch,
and a thread length for an external one) is read as draftwright's structured `internal_thread` or
`external_thread`; a `turned_knurl` as a structured `knurl`. Beyond draftwright (#23), an external
thread whose callout prints it (not `SEE MFG n`) prints its designation in place of the plain
diameter (`M10 x 1.5-6g RH, FULL AVAILABLE LENGTH`, not `ø10 M10 x …`), and a turned step's
external thread or knurl is drawn in its profile views (not section views) on the SVG/PDF/DXF
layer `thin` (thin continuous, 0.25 mm; only a drawing that has such lines has the layer): ISO
6410's thin line each side of the axis at the minor diameter d3 = d − 1.22687·P over the thread
length, and a knurl's band over the middle half of its length (lines along the axis for a
straight knurl, crossed at 30° for a diamond one, the pitch apart on paper but at least 3 mm).
The thread's limit is the step's own end (ownership requires the stated length to be the step's
cylinder or span), so the shoulder edge is its limit line and the step length dimensions it. The
plan is unchanged: these lines are render furniture, not plan items. In the session's SVG
fragments they travel in their view's fragment (layer `thin`), not in `furniture`, so they move
with the view. A turned step's diameter callout is led to the silhouette at 15° or more to it: a
leader along the silhouette would lie on the part's edge (ISO 128-22), so the placer counts one as
a hard blocker.

**IDs and views.** A dimension's ID names what it measures, never the view that carries it
(`draftwright.spec/2`; version 1 added `@<view>`). The view is the item's `view`. Omitting that
view moves the dimension to its next view under the same ID (the basis says so), and so does a
change of sheet or scale. The spec may choose the view, `dimensions/<id>/place/view` (the earlier
`dimensions/<id>/view` is still read) (`front`,
`plan` or `side`): it must be one that can carry the dimension (one of its views that does not
look along it), else the plan is refused with those that can; one the spec omits leaves the
dimension in its default view, with a warning (`dimension_view_not_shown`). The item then lists
`view` in `from_spec`. A version 2 spec keyed by a version 1 ID is refused, reading it as the ID
it now has (first among the choices). Every planner states each quantity once under one ID
(asserted in every build); a planner that drew one quantity in two views would give the second a
discriminator from what it states, as the height ladder's rungs are told apart by their heights
(`step_height.length.h12_001`), never from where it is drawn.

The spec chooses the source (ADR4-5): `"dimension_source": "auto"` (the default: the defaults
less `dimensions/<id>/omit`) or `"authored"` (exactly the IDs listed in `dimensions`, a complete
set). Omitted items are listed in the Plan's `omitted` with their basis. A correlated set, the
height ladder's rungs or the ladder's shoulders along one axis, is approved or omitted whole
(stream K6, ARCH0015-2): each member keeps its own ID, and the spec's decision on any member is
the set's. With `auto`, omitting one member omits the others too (`omitted_by_user`, the basis
naming the set); with `authored`, listing one member draws the others. Either way an `info`
problem `correlated_set_whole` names the members the decision spread to (with `auto`, its fix
removes the omission). An omission never overrides a member the spec decides itself (any entry
on it: a tolerance, a view, a place, `omit: false`): that set is drawn as the spec states it,
half omitted, with a `correlated_set_conflict` warning naming the omitted and the decided members,
its fix removing the omission. A turned profile's chain is not such a set. In an edit path, the
`/` inside a dimension's ID is written `~1`, as in a JSON pointer:
`{"op": "set", "path": "dimensions/d:envelope~1width.length/omit", "value": true}`.

### Dimensions authored from geometry (#76 item 1)

`dimensions/d:authored:<name>` adds a quantity independently of recognition. The name is
chosen by the author (letters, digits, `_` and `-`), remains its ID after moving it to another
view, and must not reuse an automatic dimension ID. For example (replace the abbreviated
fingerprints with complete keys from the Analysis):

```json
{
  "dimensions": {
    "d:authored:axis_offset": {
      "between": ["face:v1:<fingerprint>", "axis:face:v1:<fingerprint>"],
      "along": "x", "view": "front", "tolerance": "±0.1",
      "place": {"side": "below", "lane": 1, "along": "middle"}
    },
    "d:authored:diagonal": {
      "kind": "linear", "between": ["vertex:v1:<fingerprint>", "vertex:v1:<fingerprint>"],
      "along": "true", "view": "front"
    },
    "d:authored:seat_radius": {
      "kind": "radius", "between": ["face:v1:<fingerprint>"], "tolerance": "±0.05"
    },
    "d:authored:corner": {
      "kind": "angle", "between": ["edge:v1:<fingerprint>", "edge:v1:<fingerprint>"]
    }
  }
}
```

- `kind` is `linear` (default), `diameter`, `radius` or `angle`. `between` contains two
  references for linear/angle, one cylindrical or spherical face for diameter/radius.
  `along` is `x`, `y`, `z` or `true` (default), for linear dimensions only. It is distinct
  from `place.along`, which positions the label. No measured value or model/paper point is
  accepted in the Spec. Existing `tolerance`, `view`, `place`, omission and `precision`
  policies apply. Linear sizes and diameters also accept `fit` (ISO 286 class, without an
  explicit tolerance), class-only by default; radius and angle dimensions take explicit bands.
- **Discover references in `Analysis.edge_ink.references`.** This optional Analysis/7 field
  maps full `face:v1:<sha256>`, `edge:v1:<sha256>` and `vertex:v1:<sha256>` keys to lists of
  exact model-space records. A usable key has exactly one record. Plane, cylinder and sphere
  faces, straight B-rep edges and topological vertices are available. `axis:<face key>`
  selects a cylinder's axis; `axis:f:hole:3` selects the common cylinder axis of a recognized
  feature, refusing a feature with several axes. Prefer the face spelling when saving across
  revisions. Index-based `face:12`, `edge:12`, and page-coordinate references are not accepted.
  Older Analyses lacking evidence still plan their defaults; authoring requires analysing again.
- **Fingerprint contract.** The `v1` spelling and its content are fixed: SHA-256 of canonical
  sorted-key JSON containing the exact analytic record and sorted face-boundary endpoint
  signatures, with numbers rounded to 1e-6 mm. Plane normals and cylinder axes have their
  first nonzero component positive; their origins are the nearest point to the model origin
  on the plane/axis. Edge ends and boundary surface sites are sorted; a boundaryless sphere
  uses its six cardinal sites in x−, x+, y−, y+, z−, z+ order. Face traversal indices are absent.
  Fingerprints therefore survive face/edge/vertex enumeration and edge reversal. They identify
  unchanged geometry, not a guessed successor after a resize or rigid move. Coincident
  topological entities remain a multi-record ambiguity and are refused. A future fingerprint
  algorithm must retain `v1` resolution. A missing key names the reference and tells the author
  to copy a current Analysis key; it never silently binds to another entity.
- **Measurement.** Linear references are supporting planes, straight supporting lines (including
  cylinder axes), or vertices. A principal direction requires a unique station: a plane normal
  to that direction, an edge/axis perpendicular to it, or a vertex. `true` measures the shortest
  separation of the supports. Intersecting supports have zero distance and are refused.
  Angular references are two nonparallel planar faces or two coplanar, nonparallel straight
  edges; as for PMI, the actual face stations or edge ends select the half-rays and their
  minor included sector (0–180 degrees). Obtuse included angles remain obtuse.
  True lengths and angular sectors need a principal view that shows them true; a requested
  incompatible view is refused with the compatible choices. Angular placement uses view,
  radial lane and along; a side does not choose another measured sector. A size leader can attach to any
  cylindrical/spherical face, including an oblique cylinder, using exact surface sites.
- **One quantity, one owner (K6).** A new dimension duplicating a drawn automatic owner is
  refused, naming that owner ID in the message and choices. Edit its tolerance/view/place
  instead. Coincident principal spans, identical true spans/angular sectors, and the radius
  or diameter of a face already sized by its feature owner are duplicates; equal numeric
  sizes alone are not. A radius and diameter on the same face are one size quantity.
  In `dimension_source: authored`, only listed automatic IDs are owners; unlisted automatic
  candidates remain withheld, so a drawing can use entirely geometry-authored dimensions.
  Omitted dimensions do not own quantities. A replacement retains the matched numeric
  roles for independent completeness lint. A wholly restated automatic candidate is `covered`
  by the authored ID, with the basis naming that owner; a compound candidate's THRU, count,
  depth and other unstated roles remain required and are never inferred from its diameter.
- **Same planning and lint.** Authored quantities enter the ordinary dimension inventory before
  view routing, tolerance formatting and band reservation. They use its lanes, arrows, angular
  arcs, leaders, collision findings and sheet-selection rules. `default` is false, the basis
  says `spec-authored`, and `from_spec` includes `between`, plus explicit `kind`, `along`,
  tolerance and placement fields. A drawing claim carries only reference identities and the
  requested quantity (`geometry:<reference>|<reference>`, `linear.x`/`linear.true`, `diameter`,
  `radius`, `angle`), not a trusted nominal. Lint remeasures these against the Analysis and
  checks the leading printed value and size symbol; bad ink or unresolved references report
  `authored_dimension_invalid` errors. Existing structural and feature-completeness lint still
  applies; authored dimensions never silence missing manufacturing requirements.
- **Editing.** Existing `items::Op::set` / `remove` and session preview, apply, undo and redo
  operate on the entry and its fields, with atomic refusal. Set the complete entry to create
  it, for example `{"op":"set","path":"dimensions/d:authored:corner","value":{...}}`;
  remove the entry to delete it. Session/MCP item answers expose its reference anchors,
  placement options and editable fields. Sheet options describe the creation forms. MCP
  schemas describe the new fields without adding a tool. `get_spec` output declares the authored
  fields once; inputs retain the established sparse per-ID object maps and use the same Spec
  and geometry validators. Equivalent shared object-type constraints keep the 26 standalone
  schemas within the unchanged 42,500-byte session budget. Browser bindings accept the same
  Spec and operations; their method signatures are unchanged.

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
  { "op": "remove", "path": "dimensions/d:f:pocket:1~1depth/omit" },
  { "op": "add", "path": "notes", "value": { "text": "DEBURR" } } ]
```

`add` appends its value to the list at its path (only `notes` is a list).

Each operation is validated against the analysis:

- **Wrong value:** the error says what would be accepted, e.g. "Choose one of: H6, H7, H8 …".
- **Close but wrong:** "'h7' reads as 'H7'".
- **Unknown ID:** the error lists the nearest valid IDs.

A caller may equally replace the whole spec.

**Adapters.** All three call one module, `src/api.rs`: the operations typed, and `api::call`,
the same operations by name with JSON arguments. Batch requests carry their documents and name
files by path, so they need no session state; the Session additions are described below. Errors are `{"code": "invalid" | "failed",
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
  invalid input; exit code 1 means a write failed without that invocation replacing its destination.
  Earlier formats in the same export may already have been published.
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

**File exports.** `api::write` creates unique sibling staging files exclusively and publishes
with atomic replacement per destination. Concurrent callers use last-writer-wins per file
(the last successful rename), and cleanup removes only their own staged copy. A multi-format
export is not transactional: concurrent callers can publish different formats, and a later
failure does not roll back formats already published. Single-sheet destination names are unchanged.

**The spec in each output** (invariant 8): in the SVG's metadata, in the PDF's document
information, and in the DXF as comments (group code 999) ahead of the first section. AutoCAD,
ezdxf and the `dxf` crate read past such comments; a re-save drops them. `draftwright spec`
reads it back from any of the three.

## IDs

| Item | Form | Stable across |
| --- | --- | --- |
| Feature | `f:<kind>:<n>`, derived from its faces | the same file |
| Dimension | `d:<feature>/<parameter>`; never the view, which is the item's `view` | the same file, whichever view carries it and whatever the sheet or scale; carried to revisions through its feature |
| Sheet | `sheet` | always |
| View | `v:<name>` | always |
| Note, table, section | `n:<k>`, `t:<k>`, `s:<letter>` | always (set in the spec) |
| PMI frame, symbol, label, drawing-wide requirement, thread or knurl (and the feature of a PMI dimension's claim) | `pmi:<identity>`: the record's semantic identity, below; never its Part 21 instance (that is the item's `source`, e.g. `geometric_tolerance:#123`) | the same requirements on the same faces: the intent written again (new Part 21 numbers) keeps it |
| Problem | `p:<code>:<subject>` | the same plan |

Parameter names follow the Python version's role vocabulary (`role.kind[.discriminator]`). A
name that matches more than one measurement is refused rather than guessed (Python ADR 4
invariant 6).

**A PMI record's identity** (`pmi::identity`, `pmi::Report::identities`; the editor's
`gdt::editor_key`, upstreamed) is its category and kind (`.basic` for a basic dimension), then,
each tagged and left out when empty: its part in a file of several (`p<n>`), the drawing faces
(`f<i>,<j>`) and edges (`e<i>,<j>`) it applies to, its datum references (`ref=A,B,C`) and its
modifiers (`mod=…`), and a datum's letter or a label's text (`t=…`), every character but
letters, digits, `_`, `.` and `-` written `%XX`:
`pmi:geometric_tolerance.position:f12,13:ref=A,B:mod=diameter_zone`. No value enters it, so a
changed magnitude is the same item with another label. Records that share an identity (a
composite frame's two segments, NIST FTC-07's and FTC-10's) are each told apart by their label,
`:t=position%200.8`, a discriminator from what they state, never from their order; records whose
labels are the same too (NIST STC-09's two identical `0.03 ±0.01 INCH` records, FTC-10's three
`ø3` on no face) get no ID and are not drawn: `pmi_not_lowered`, naming the records they cannot
be told from (ADR R1 invariant 5). Problems about PMI records still name the record's source
(`p:pmi_not_lowered:<source>`), as the record is not an item.

**Spec versions.** `draftwright.spec/2` is the version this build writes. A
`draftwright.spec/1` spec is read through `model::migrate` (in `plan`, so every caller of the
library gets it; the plan's embedded spec is the migrated one): `d:<feature>/<parameter>@<view>`
becomes `d:<feature>/<parameter>` (the view was where the plan routed it, not a decision, so the
entry does not keep it as its `view`), and a PMI dimension's `pmi:<category>:#<n>` becomes its
record's identity, when the spec is bound to the analysis's file (a Part 21 instance names a
record only in its own file). Two keys that come to one are refused, with both as the
candidates; a key whose record has no identity, or is of another file, is kept for `plan` to
refuse with the nearest IDs. `edit` has no analysis and leaves a spec's version and keys as they
are. A version 1 spec plans as it did (`tests/identity.rs`, on fixtures the 0.1.0-alpha.1 build
planned).

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

## Rendering caches (thin slice S4 part A)

`model::render` remains a fresh deterministic render. A caller that keeps
`model::Projections` and uses `model::render_with` shares each hidden-line projection (direction,
up and cut), and its latest placed curves and visible-ink bounds (scale, projected centre and page
origin). Changing the part's source binding clears these caches. `Projections::stats` counts the
actual projection and view-bounds passes, so an annotation edit can demonstrate that neither ran.

The same path is available in two steps: `model::render_geometry` produces the views' visible and
hidden edges and bounds; `model::render_annotations` assembles that geometry with annotations and
sheet furniture. Pass the geometry of the same planned views, in the same order and at the same
placement and scale. For a slice item's own drawing, `model::render_item` takes
`AnnotationItem::Dimension` (including callouts) or `AnnotationItem::Gdt` (frames, datums and PMI
labels). Its `RenderedItem` holds only that item's ink, searchable text and lint provenance, with
local run indices. `RenderedItem::append` rebases those indices when combining fragments. The
whole-sheet renderer uses these same item functions; planning and placement are unchanged. Sheet
furniture (border, title, notes, tables and section furniture) is assembled in the whole-sheet path.

`pmi::Reader::default()` keeps the latest reading for one immutable part. Assign
`analysis.pmi = reader.attach(pmi_bytes, &part)` when PMI changes, and plan from that Analysis:
material, threads/knurls, dimensions, frames and datums consume the one report. Identical bytes
(including an empty or refused reading) do not read again; changed bytes replace the report.
`Reader::reads` counts reading attempts. Replace the reader when replacing the geometry. The
ordinary analysis API still attaches PMI once and carries the report in the Analysis; there are
no separate material, thread or frame reads in planning or rendering. Haecceity's geometry/part
inventory reader remains separate from its semantic PMI parser.

## Sessions (thin slice S4 part B)

`session::Session` (also exported as `draftwright::Session`) owns one immutable part and its
Analysis (including the already-read PMI report), current Spec/Plan, projections, annotation
ink and Spec history. `Session::new(part, analysis, spec)` takes trusted in-process geometry
and analysis; `Session::open(step, api::Features, spec)` loads them once. The batch operations
above retain their existing behaviour.

`apply(&[items::Op], commit)` uses exactly `edit`'s operation vocabulary and validates the
result through the recovered planner. `commit: false` returns the same delta as committing
from that state, while changing neither state, history nor cache counters. Refused edits also
leave the last good state intact. A batch is one undo step; `undo()` and `redo()` return deltas.
A successful new commit discards the redo branch; a preview or refusal preserves it. History
retains Specs, rather than whole drawings or projections.

Each edit makes one complete `plan_recovered` invocation. The existing planner is global, so
this is the conservative full-plan fallback: its internal layout candidate/recovery attempts
and canonical pin plan remain part of that invocation. `Session::stats().plans` counts those
invocations at the planning entry point, including undo/redo, and excludes discarded previews;
it does not claim to count individual internal layout candidates. `Delta.plans` reports the
measured invocations for a transaction, including a discarded preview. Queries never invoke it.
Projection and bounds counters count actual expensive passes as in S4 part A; the annotation
counter counts actual independent item renders. Unchanged label/geometry/provenance keeps its
ink, even when the item's basis or default changes. Sheet furniture is assembled afresh.

`Delta` reports `changed`, `added` and `removed` item IDs/typed answers, and complete problems
added/resolved (a changed problem is resolved and replaced). It also carries a lossless
`PlanDelta`: changed top-level fields and changed collection records by stable ID, with order
only when it changes. The other payloads are changed view geometry and independently rendered
annotation ink, using S4 part A's `RenderedItem`. Geometry and ink are typed, serialisable curve
records; no full SVG is returned on each edit. Omitting an item can keep its ID/state in the
Plan while removing its ink. `Delta::apply_to(&mut session::State)` replays without planning or
formatting annotation labels. `State::render` assembles those cached pieces and the plan's
furniture through the same renderer as a fresh sheet, including searchable text, lint provenance
and embedded Spec. `Session::drawing()` exposes the current assembled drawing for export.

The queries are `item(id)`, `items(&ItemFilter)`, `problems(&ProblemFilter)` and `options(id)`.
An item answer carries typed state, resolved value, the engine's label, placed geometry,
anchors, features, basis and allowed edit fields. Item filters combine view (with or without
`v:`), kind (including linear/callout/angular), exact typed state, feature, region (page millimetres) and proximity to another
item's ink (`near`, optional `radius`, default 10 mm). PMI feature associations follow overlap with the feature's drawing faces; their anchors also
name those faces. Problem filters combine code, severity
and subject ID or feature ID (all associated item subjects, plus the feature itself).
Unknown item IDs give the five nearest IDs. Options conservatively offer compatible view/side
placements, view anchors and the ISO scale series for the sheet. `settable` maps escaped Spec
paths to choices, ranges or forms, including tolerance strings, bare view keys, title fields,
and fit classes valid for each associated hole/boss's nominal size. A descriptor's `paths`
shares its choices across all listed member paths. Collisions are reported by preview/apply;
queries do not run trial plans.

`Session::label_rows(callout_address)` returns a serialisable map keyed by compatible
shown principal view IDs (`v:plan`, etc.) for an editor's exterior snap guide. Each
`dims::LabelRows` has `outline: [left, bottom, right, top]`, `first_row` and
`row_spacing`; the latter two are maps keyed by `above`, `below`, `left`, `right`.
All coordinates and spacings are paper millimetres, with y increasing upwards.
These are **elbow coordinates**, not text baselines. Lane `n` is
`first_row[side] + sign * n * row_spacing[side]`, with sign +1 above/right and
−1 below/left. The first row is 10 mm outside the projected envelope. Above/below
spacing is 7 mm; beside a view it is the greater of that view's widest canonical
callout and this callout's current ink width, plus its 3 mm shoulder and 7 mm
clearance. The resolver and query share the spacing calculation. Canonical widths
are retained as planning metadata, so another item's text edit cannot move
a pin's columns; the Plan's serialised shape is unchanged.

This query performs no planning or projection, accepts the usual bare/qualified
item addresses, and refuses non-callouts. Re-read it after applying an edit to a
view, sheet or this callout's text. It describes explicit exterior `side/lane/along`
placement. Automatic labels can lie inside the view or use the automatic solver's
other exterior candidates; read their actual geometry through `item`/`plan`.
A bare pin retains that automatic geometry. To pin a label dragged anywhere, including into
the view's open space, use `place.offset`: `Session::callout_offset(address)` returns
`CalloutOffset { view, tip, bend, offset }` (paper mm), where `tip` is the automatic leader tip
the offset is measured from, `bend` the bend as drawn now and `offset = bend - tip`. Pinning a
drop at `b` is `{"offset": b - tip, "pinned": true}`; pinning the returned `offset` keeps the
callout exactly where it is. `tip` is kept relative to its view's origin, so it is in the same
frame as the drawn bend after packing moves views. It reads planning metadata only and refuses
non-callouts, a callout drawn without a leader, and one pinned into a non-canonical view. Editors with their own Plan read the same tip with
`plan.automatic_callout_tip(id) -> Option<(view, [x, y])>` (`None` on a deserialised Plan).
These Rust session queries are available to editor bindings; they add no MCP tool or
JSON-lines operation.

Editors that keep their own session can call public
`dims::label_rows(&analysis, view, width)` directly. Pass the greater of the
edited callout's ink width and `plan.canonical_callout_width(&view.id)` (paper mm).
The latter returns `Option<f64>`: `None` means no canonical callout for that view,
or a deserialised Plan whose non-wire metadata is absent. Keep the freshly planned
Plan to use this API; do not infer canonical widths from edited placements.
This uses the same calculation as `Session::label_rows` without requiring a
`draftwright::Session`.

`draftwright serve` keeps one Session per connection, behind `api::Service`. The additional
operations advertised by `hello` are:

```json
{"id":1,"op":"open","step":"part.step","spec":{}}
{"id":2,"op":"apply","commit":false,"ops":[{"op":"set","path":"dimensions/d:envelope~1width.length/tolerance","value":"±0.1"}]}
{"id":3,"op":"apply","ops":[{"op":"set","path":"views/iso/omit","value":true}]}
{"id":4,"op":"undo"}
{"id":5,"op":"redo"}
{"id":6,"op":"item","item":"d:envelope/width.length"}
{"id":7,"op":"items","filter":{"kind":"dimension","view":"front"}}
{"id":8,"op":"problems","filter":{"severity":"error"}}
{"id":9,"op":"options","item":"sheet"}
```

`open` returns the initial replayable State, defaults to native recognition, accepts `features`
(`native`, `none` or Python recognition JSON path), or a bound `analysis` in its place. A new
successful `open` replaces the connection's part/history; a failed one preserves them.
`apply` defaults to committing. Item IDs use `item` so the request's `id` remains the reply
correlation ID. The original six operations and protocol/reply shapes are unchanged, and may
be interleaved with session requests. `api::call` remains stateless; `api::Service::call` owns
the connection state. Editor integration and MCP adapters belong to S5 and S7.


### Plan-time hole schedules (H1)

`hole_table` defaults to `auto`; `always` requests schedules and `never` disables
both plan-time schedules and the sixteen-hole rescue. The same `set`/`remove` Spec
operations and MCP `item_options` expose this field. `model::plan` remains the
raw layout diagnostic; `plan_escalated` and every recovered API plan honor it.

Each principal end-on view chooses its own presentation. Six or more holes use
a schedule when `layout::crowded_locations` finds illegible ordinates, there are
more than eight hole location statements, or a hole leader crosses a feature or
another annotation. Equal spacing on an established bolt circle keeps the
existing counted PCD/BC note in auto mode. A schedule uses the theoretical
intersection of the carrying solid's minimum datum planes, including tangents
at rounded corners. The solid's measured profile bounds must prove this origin.

`Plan.hole_tables` carries each view's schedule, resolved model-space origin,
rows and placed blocks. Columns are `TAG | ø / DEPTH / THREAD | X | Y | Z OUT / DIR`.
All three coordinates are in the carrying view's frame: X right, Y up, Z out of
the page toward the reader (the header says `Z OUT`). The entry is the
opening's distance from the part's rearmost face along view Z; the direction is
`−Z` (drilled into the page) or `+Z` (from the far side), for example `0 / +Z`.
The entry claim carries the view's out-of-page direction (`entry.view.x,y,z`),
which lint uses to check the cell. Each physical hole has a row;
coaxial openings share one tag such as `B1/B11`, with separate entry cells.
A tag sits beside its hole only where it is plainly nearer that hole's rim and
centre than any other scheduled hole's; otherwise it takes a short leader from
the hole's rim (shortest, fewest crossings, fanning outward from neighbours,
never through another scheduled rim, label or tag leader).
Tags are grouped by approved size wording; X/Y are the carrying view's two in-plane coordinates. The origin
and positive coordinate directions are marked in that view. Size and location
cells retain the planner's formatting and tolerances; an authored omission
is not resurrected. Rows retain typed measurement claims, and the rendered
coordinate cells certify only their own axis. Lint checks their printed values
against Analysis geometry; a wrong or missing entry cell reports
`hole_table_entry_invalid`. Five or more distinct equal-radius sites about a
recognized bore axis can use a `pcd` statement, a located circle centre and an
explicit `ANGLE` column (counterclockwise from local +X). Missing positions do
not imply equal spacing; the member X/Y remain supplementary locating evidence.
Tags and origin labels avoid boundaries, bore silhouettes, other annotation ink
and section furniture; their resolved `label_box` is also judged by structural
lint and the SVG quality instrument. Pins keep their callout or location ink. Semantic PMI keeps its native
source-carrying callout beside the schedule, and auto retains authored size
fits/tolerances. Coordinate tolerances keep their original span.

Table-owned callouts remain as stable covered dimensions; retired linear spans
are covered entries in `omitted`, as other superseded spans are. Their measurement
provenance is retained in the schedule's `covered` list for the completeness ledger.
They reserve no lanes; aliases previously consolidated into them remain consolidated. The ordinary recovered principal
and isometric geometry scale and sheet size are retained for the schedule
layout. If preserving the scale at a new automatic isometric centre would clip
the drawing, its proven full frame is retained; bare pins retain their canonical
frame and explicit isometric placements keep their fitted frame. Parenthesized fit limits survive; only the pattern lattice suffix is
removed when member coordinates replace its pitches. Table placement uses the existing clear corner-block inventory; a
failed complete-table/tag transaction restores the original drawing. An
unavailable explicitly requested table is a warning, with original dimensions
kept. The old sixteen-hole rescue remains a separate backstop. The deliberate
Python differences are recorded in `tests/fixtures/known_hole_tables.json`.


### Interior placement (Stage H7)

Automatic callouts contribute Python's bounded eight rays and eight text-lane
stations, sharing at most 64 alternatives across a grouped feature's anchors.
Bore targets stay radial; other features retain their physical attachment while
the ray rotates. Every tip must touch a projected edge. A complete interior label must fit over proven
projected material, with half a paper millimetre of clearance from every visible
and hidden edge and all settled annotation text/ink. The candidate's entire
leader/dimension geometry stays off other views and page furniture. Pins and
explicit placement retain precedence. Linear dimensions retry up to eight inward
lanes after their settled exterior placement fails; the whole dimension moves,
including witnesses, arrows and label. Every line and arrow independently clears
visible and hidden projected ink; only the first 0.6 mm of a callout at its
attachment may touch the target. Witnesses leave their own opening at its near
rim with the normal gap. Labels inside their own view are accepted
by placement checks only after re-proving blank face; rendered structural lint
still checks projected ink and suppresses the blank-region notice for a
deliberately interior annotation, including Python-read drawings without a
second annotation ownership record.

Interior placement currently requires a planar face square to the view. A curved face's
silhouette is bounded by its generators (a cylinder seen square) or its whole box (anything
else), so labels keep off it. Section views keep exterior labels; a cutting-plane line, its
arrows and letters are obstacles in the view that marks them (`model::cutting_line`, shared
with the renderer). These limits and the difference from Python's projected material field
are recorded in `tests/fixtures/known_interior.json` and `docs/next-session.md`.

### Blend radius notes (G3)

The Analysis optionally carries `blend_sites`, exact trimmed-cylinder/torus points in the part
frame, indexed by blend feature ID. Recognition derives them from the defining faces and checks
the rolling radius; the rolling path itself is not a leader attachment. Older version-7 analyses
read with no sites and cannot supply a new blend leader without that evidence.

Blends with equal displayed radii share exact occurrence credit. Fewer than five radius-bearing
blends outside grouped relief use compact `R` / `n× R` leaders. On a denser part, its most
frequent repeated radius is
one `ALL UNSPECIFIED RADII R…` note, including fillets with the same displayed value.
Other radii retain explicit exception leaders, combining equal blend and fillet groups.
Obstructed straight exception groups use an optional `radius_table` (`t:radius-centres`):
every occurrence has its own named view, radius and centre X/Y in model millimetres from
the left/bottom of that view's envelope. Centre locators print up to three decimals; radius
sizes retain the normal displayed precision. The blend's own trimmed-surface samples must
prove that centre. Dimensions represented there keep their IDs and become `covered` table rows.
Independent lint validates the printed view, X, Y and radius against each exact occurrence.
A wrong, missing or duplicated locator cannot borrow credit or unlock the default. If a
complete table cannot fit, the required leader remains with its routing findings (Policy B).
An undrawn or omitted exception withdraws the blanket default. Scoped counts and representative XYZ points cannot locate a group
and grant no coverage. Explicit dimension placements and tolerances remain author choices
and retain their geometry findings.

Automatic notes have stable IDs `n:f:blend:<id>/blend.radius` and accept
`annotations/<escaped-id>/{omit,place}`. Session/MCP item answers expose the claimed features
and the edit paths. Automatic radius-only blocks use the existing 3 mm font on compact 5 mm
rows, clear of actual view outlines and placed annotation ink; authored/PMI notes keep the
existing ruled table, with a separate optional `radius_notes_block` for automatic radii when
both are present. An omitted or unplaced note grants no radius credit. The rendered
annotation's `satisfies` list identifies the exact feature/parameter requirements; the printed default must agree with those claims and other radii need explicit callouts.
Text alone never infers authority. General authored notes still carry no satisfaction claims.

Analysis also optionally carries `radius_reliefs`: geometry-owned evidence of grouped tiny,
complex shallow profiles on a common broad carrier (`src/blends.rs`). Recognition features
remain intact. Every defining face of an excluded radius must belong to one proved region;
lone/simple/deep pockets and large faces are retained. Old Analyses have no exclusions.
The companion automatic note `n:f:blend:relief/profiles` says
`SHALLOW RELIEF PROFILES PER MODEL`, carries no fabricated radius claims and accepts the
same omit/place edits. A placed note reports the exact governed features through the
informational `relief_profiles_per_model` disposition. With relief present, a blanket radius
default requires that note to
be drawn; hiding/removing its annotation withdraws default authority in planning and lint.
The geometric thresholds and the single-solid restriction are overturnable drafting policy,
recorded with positive and negative evidence in `docs/next-session.md`.

Same-face boss/blend precedence uses one shared predicate for planning and lint. A uniquely
proven blend radius replaces that face's duplicate boss diameter; the independent boss height
remains required. Missing or ambiguous face evidence never deduplicates by scalar equality.
The differences and verdicts are recorded in `tests/fixtures/known_blends.json`.

Precision is a display policy in `precision`, a map from measurement IDs such as
`d:envelope/width.length` or `d:f:hole:1/counterbore.depth` to decimal places (0–15).
It retains trailing zeroes in the label and its measured reservation, without changing the
nominal or tolerance. Conflicting explicit policies for a shared printed term are refused;
they require separate dimension owners. Without a policy, formatting is unchanged.

`envelope_tolerance` supplies a band to all three overall extents. A dimension's own
`tolerance` or `fit` overrides that fallback. An extent fit uses the existing ISO 286 table
and prints the class alone by default. `sheet/fit_deviations: true` adds stacked upper/lower
deviations; the legacy `sheet/fit_limits` alias is supported, with the new setting taking
precedence. Authored precision controls the extent nominal in both modes and leaves the
fit deviations and explicit tolerance digits unchanged.

A feature note can explicitly satisfy its own measurement roles with
`features/<id>/note_satisfies`, for example `["counterbore.diameter", "counterbore.depth"]`.
Only its placed leadered note grants authority. This is separate from dimension claims:
prose grants no coverage, a diameter role implies neither THRU nor count, and sibling
features need their own explicit claims.


### Label wording and explicit dimension styles (#76, item 5)

Wording is authored under the existing stable dimension or annotation ID. No page
coordinates or measured values are stored in the Spec. These additive fields work in
`draftwright.spec/2`; saved Specs without them retain their automatic labels and layout.

```json
{
  "dimensions": {
    "d:envelope/width.length": {
      "text": {"prefix": "2×", "suffix": "TYP"},
      "style": "reference"
    },
    "d:f:hole:3/bore.diameter": {
      "text": {"template": "{value} AFTER PLATING"},
      "style": "basic"
    }
  },
  "annotations": {
    "pmi:common_label.example": {"text": {"template": "{value} REF"}}
  }
}
```

Use IDs returned by the current Plan or item queries; the example annotation ID is
illustrative. A dimension callout belongs in `dimensions`; unframed PMI prose labels
and the automatic radius/relief notes belong in `annotations`. Datum letters, geometric
tolerance frames and surface texture symbols retain their structured feature edits:
free wording on those symbols is refused.

`text` takes either `prefix`/`suffix`, or `template` with exactly one `{value}`.
The placeholder expands to **all generated terms**, including compound diameters,
depths, angles, counts, thread designations, precision, tolerances and fit classes.
It is deliberately more than a bare nominal: changing wording cannot remove another
measured requirement from a callout. Prefix and suffix are separated from the generated
terms by one space; surrounding literal whitespace is trimmed. For example `REF`,
`AFTER PLATING` and `(NOM)` are ordinary suffixes. A literal full replacement, even one
containing today's measured number, is refused: use `{value}` so the saved Spec remains
truthful when geometry or display precision changes.

Literal wording is one line, at most 240 UTF-8 bytes per field, without control characters,
extra numeric values, tolerance/measurement symbols or unknown placeholders. The exception
is one positive leading quantity `N×`; it cannot be added to an already counted label,
and independent quantity lint still judges it against the part. These rules refuse a
second nominal or a conflicting value instead of letting prose impersonate a measurement.
Unknown IDs are refused with current IDs or an explicit removal instruction. An override
on a merged dimension is refused with its owning label ID; apply wording to that owner.

`style: "reference"` encloses the entire label in parentheses; `style: "basic"` draws a
box around its full ink. The enum makes these choices mutually exclusive. Remove `style`
to restore the automatic choice. An imported basic PMI dimension cannot be made reference
without correcting its source PMI. A newly authored reference has no fabricated
`determined_by` chain. Basic/reference styling changes no nominal or measurement claim,
and lint continues checking the resulting output, including redundant controlling sizes.

Wording is applied after automatic ownership/consolidation and precision/tolerance
formatting, before placement and reservation. Full terms drive ink measurement, leader
shelves, label clearance and searchable SVG/PDF text. Automatic references keep their
parentheses around all authored wording. Plans report `text`, `style.reference` or
`style.basic` in `from_spec`, with the choice explained in `basis`; annotation and automatic
note wording likewise report `from_spec: ["text"]`. Unmentioned choices keep their defaults,
including paper/scale policy, automatic views, datum placement and class-only fits.

The existing `items::Op` `set`/`remove` operations, session previews, history and replay
handle these fields. Escape the slash inside an ID, for example:
`dimensions/d:envelope~1width.length/text`. Item options expose `text`'s fields/template
form and the two `style` choices. `get_spec`'s output schema describes the saved text shape;
other MCP tools retain the existing generic per-item Spec-map input convention. The schema
generator uses equivalent minimum-property counts for closed objects whose every field is
required, preserving the existing session byte cap.

Lint reads the live value-bearing text inside the compiled wording wrapper and the
original measured claims. It does not infer a new value from authored prose or trust an
approved nominal instead of the drawn text. Damaged wrappers or missing values lose claim
proof; linear fidelity compares the live numeric text against the drawn witnesses. Wording
never acknowledges or suppresses a lint finding.

Large MCP `item_options` replies keep the complete descriptor map in `structuredContent`.
When repeating it as text would exceed the existing 4,096-byte reply cap, the text channel
contains an `id`/`detail_ref` stub. Text-only clients read the identical complete JSON with
`resources/read {"uri":"draftwright:<detail_ref>"}`. Native/session options keep every field.
