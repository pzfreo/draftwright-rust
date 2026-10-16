# Changelog

All notable changes to draftwright-rust are recorded here, in the style of
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- Gzipped STEP input (maintainer decision 2026-10-15): every command (`analyse`, `plan`,
  `render`, `review`, `parts`), serve and MCP request, cached registration and library entry
  point that reads a STEP file or its bytes (`api`, `model::analyse_step`, `model::parts`,
  `pmi::read`, `pmi::Reader::attach`, `Session::open`, `stateless::Cache::register`) accepts
  gzip, told by its magic bytes (`1f 8b`), not the file name, and decompressed in memory
  (`model::step_bytes`, `model::read_step_file`). The binding's `source_sha256` and the cache
  key are of the decompressed bytes, so a `.step` and its `.step.gz` bind, cache and draw
  identically (a `.step.gz` is titled by its stem without `.gz`). A gzip that does not
  decompress is refused as invalid input, saying so.

### Fixed

- `--part 0` (`"part": 0`) of a file of one part is the whole file: drawn byte for byte as without
  a part chosen, titled by the file's stem rather than the STEP product's name (`plate.step`
  gave `COMPOUND`). A part of a real assembly is still titled by its product's name.

## 0.1.0-alpha.2 (2026-10-14)

### Changed

- Pinned to quiddity-rust `v0.1.0-alpha.3` (from `v0.1.0-alpha.1`). Its recognition document's
  new `section_recesses` family (`SectionRecess`, `SectionRecessRefusal`) is admitted as known
  and not yet read, as the other recess families are; without that every part with a section
  recess was refused. No drawing changes: across the single-sheet guard's 1876 cases the new pin
  leaves SVG, PDF and PNG bytes unchanged and each plan differs only in its
  `features_not_recognised` warning (the recogniser's version, and section recesses counted from
  the document); the version bump then changes only the loader version in each plan's binding
  and each file's embedded Spec. The guard is re-baselined for this (`quiddity-alpha3`).
- Multiple sheets: a plan allocates views and tables across sheets (SHEET n/m), every sheet is
  rendered, sessions and MCP work on multi-sheet plans, and sheets carry ISO 5457 zones and
  section cross-references (`lint_sheets`).
- Leader routes keep callout text, arrowheads and elbows off extension lines and the outline;
  pinned callout labels keep their place and side.
- Fits print their ISO 286 limits (`sheet/fit_limits: false` prints the class alone); repeated
  spans are drawn once; the title block credits draftwright in DRAWN BY.
- DXF export is disabled until it is fixed.
- Stable identity (`draftwright.spec/2`): a dimension's ID is `d:<feature>/<parameter>`, never
  its view, so moving it to another view, omitting a view or changing the sheet or scale keeps
  it; the view is the item's `view` and may be chosen in the spec (`dimensions/<id>/view`). A
  PMI item's ID is its record's semantic identity (`pmi:<category>.<kind>:f…:ref=…`), never its
  Part 21 instance, so the same intent written again keeps it; records that cannot be told apart
  are refused. A `draftwright.spec/1` spec is migrated when planned; ambiguous keys are refused
  with their candidates (`docs/model.md`, "IDs").

### Added

- Typed state and problems: every plan item carries a `state` (`shown`, `omitted_by_user`,
  `not_authored`, `covered` by another item, or `not_drawable` with the code of the problem that
  says why), beside its basis sentence; every problem's `subjects` are live item IDs, with what
  else it names in `refs`, and a machine-readable `fix` (edit operations) where one resolves it;
  `items::items` and `items::label` list every item with the text it draws, an omitted
  dimension's and a GD&T item's label included (`docs/model.md`, "Plan").
- Placement as intent, a session core (apply, preview, undo and redo, queries) and a stateless
  MCP core with its cache and schemas.
- AP242 semantic `turned_knurl` PMI is drawn.

## 0.1.0-alpha.1 (2026-10-10)

The first alpha. Its gates and their state are in `docs/alpha1.md`.

### Added

- The document model (ADR R1): Analysis, Spec and Plan, with `analyse`, `plan`, `edit`,
  `render`, `review` and `spec` on the command line, as a JSON-lines service (`draftwright
  serve`) and as an MCP server (`draftwright-mcp`).
- Native feature recognition through quiddity-rust (`quiddity-rust/recognition/2`), read
  fail-closed; Python quiddity's JSON through `--features`.
- Layout with draftwright's sheet, scale and view arithmetic, reserved annotation bands, and
  page escalation and recovery; section A–A without a boolean.
- Dimensions per family (envelope, turned steps, holes and patterns, slots, bosses, fillets and
  chamfers, grooves, flats, steps and the height ladder, section recesses, hexagonal stock),
  with spec tolerances and ISO 286 fits; hole tables by drop-and-escalate.
- Lint of the drawing against the recognised features, independent of the Plan.
- AP242 semantic PMI: read through haecceity; feature control frames, datum feature symbols,
  and toleranced and basic dimensions drawn; surface and common labels drawn with leaders, and
  the general tolerance, default surface finish and edge-condition notes on the sheet; thread
  and knurl requirements on their owning hole's or turned step's callout, or `SEE MFG n` with a
  manufacturing requirements table.
- The title block's material from the STEP file's one usable material property when the spec
  states none, as draftwright does.
- Angular PMI dimensions between two planar faces, drawn about their vertex in the view that
  shows the angle true (nist_ctc_01's `60 ±0.5` in the plan); one no view shows true is refused.
- The AP242 files haecceity's PMI writer writes (8, from quiddity-rust's written fixtures) in the
  PMI reading, drawing and coverage comparisons against draftwright, every difference verdicted.
- Band space for GD&T: the layout reserves room for feature control frames, datum feature
  symbols and PMI labels, and places them inside their views' bands (draftwright reserves none).
- PMI on hole occurrences (AP242 edition 4 `basic_round_hole`), now that haecceity reads them:
  nist_stc_09's hole callouts and their position frames are drawn, counted over the pattern.
- For callers with no file system, such as a browser build: `model::analyse_step` and
  `analyse_native_step` analyse a STEP file from its bytes, and the bundled font is public
  (`api::MONO`); haecceity runs hidden lines on one thread where none can be spawned.
- SVG, DXF, PDF (subset fonts, searchable text) and PNG output, each carrying its Spec;
  deterministic to the byte in every time zone.
- The PMI coverage lint: every PMI item is on the drawing or reported.
- `plan` from a saved Analysis is fast again (the maintainer's render cache and straight-piece
  boxes, PR #1): about 16 ms on NIST FTC 06, from about 490 ms; drawings unchanged.
- Drawing quality on the 30 measured parts (`tools/quality_parts.txt`), against the 86ffcbc
  baseline: annotations off the sheet 34 to 0, overlapping views 34 to 0, views on the title
  block 1 to 0, repeated labels 50 to 18 (each remaining one verdicted), ink problems 506 to 282,
  with none over a view.

### Known limitations

- Oblique PMI lengths, a cone's angle, a thread or knurl on a boss and face-finish
  requirements are read and reported, not drawn.
- Section recesses and some other families are not read from quiddity-rust's document
  (`features_not_recognised`); an angled step is reported unsupported.
- Revision rebinding (ADR R3) is not implemented.
- 282 ink problems remain on the 30 measured parts (109 leader crossings, 155 cases of ink
  past a view's area, 18 lines through labels), four parts have accepted cells worse than the
  baseline, dense AP242 parts carry much more ink, and hole tables are drawn only by
  drop-and-escalate; the README's "Known limitations" lists each with its counts and verdict.
