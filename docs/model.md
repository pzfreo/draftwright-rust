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
  before them still read. Rust quiddity's JSON carries no faces yet, so it cannot anchor
  features.

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

## The five operations

These are one Rust core: pure functions over the documents, deterministic, with no state between
calls.

| Operation | In | Out |
| --- | --- | --- |
| `analyse` | STEP file | Analysis |
| `plan` | Analysis, Spec | Plan (with problems); rebinds first if the spec's binding differs |
| `edit` | Spec, ops | Spec, or a validation error |
| `render` | Plan, formats | SVG, DXF, PDF (and PNG), each with the spec embedded |
| `review` | Plan | a rendered image and the problems, for a visual check |

**`edit` operations** are small and addressed by path:

```json
[ { "op": "set", "path": "features/f:hole:3/fit", "value": "H7" },
  { "op": "remove", "path": "dimensions/d:f:pocket:1/depth@front/omit" },
  { "op": "add", "path": "notes", "value": { "text": "DEBURR" } } ]
```

Each operation is validated against the analysis:

- **Wrong value:** the error says what would be accepted, e.g. "Choose one of: H6, H7, H8 …".
- **Close but wrong:** "'h7' reads as 'H7'".
- **Unknown ID:** the error lists the nearest valid IDs.

A caller may equally replace the whole spec.

**Adapters.** All three call one module, `src/api.rs`: the operations typed, and `api::call`,
the same operations by name with JSON arguments. Requests carry their documents and name files by
path, so no adapter keeps state between calls. Errors are `{"code": "invalid" | "failed",
"message", "choices"?}`.

- **CLI.** `draftwright analyse part.step`, `draftwright plan part.step --spec part.drawing.json`,
  `draftwright review …` and `draftwright render … -o out` all print JSON. Exit code 2 means
  invalid input; exit code 1 means a write failed and the output was left as it was.
- **JSON-lines service.** `draftwright serve` takes one request and gives one reply per line, in
  the shape of specify-core's `serve`: `{"id": 1, "op": "plan", "step": "part.step", "spec": {…}}`
  gives `{"id": 1, "ok": true, "result": …}` or `{"id": 1, "ok": false, "error": {…}}`, and
  `{"op": "hello"}` gives the protocol version and the operations. A malformed line is answered,
  and the service carries on.
- **MCP server.** `draftwright-mcp` speaks MCP over stdio (JSON-RPC 2.0, one message per line).
  Its tools map one-to-one onto the operations, each with an input schema and a description
  written for an LLM. Results are the documents as JSON text; `review` adds the drawing as SVG
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
- **The ported lint rules** run on the rendered plan, independently of it: they judge Python's
  sheets and Rust's alike.
