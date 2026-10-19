# Changelog

All notable changes to draftwright-rust are recorded here, in the style of
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### WASM API

- `sheetFragments(sheet)` and `sheetSvg(sheet)` give any sheet's fragments and SVG, as
  `fragments()` and `renderSvg()` give sheet 1's; `renderPdf()` has a page per sheet and
  throws an API error instead of returning bytes unconditionally.

### Added

- Pin a projected view's gap with `views/<name>/place/align: "projection"` while keeping
  geometry on its projection line despite asymmetric dimension bands. Valid front/plan
  and front/side pairs offer it first in `item_options`; other pairs refuse it with choices.

### Fixed

- Draw AP242 dimensions and tolerances on resolved derived axes and planes, preserving
  authored values, tolerances and MAX/MIN/AVG qualifiers. GRM-03 now carries its ten
  semantic sizes and lengths instead of geometric fallback dimensions.
- Default drawing numbers to a leading part-number token in the STEP product name, then
  the file stem (including numeric names such as `10000`), before `DWG-001`. Explicit Spec
  numbers, including blanks, win; readable titles retain the part number (#103).
- Cover the tester's original GRM-03 AP242 material (`CZ121 / CW614N / CuZn39Pb3`)
  through reading, title-block rendering and coverage lint. Its CAx-IF property already
  projects on main; #108's warning fix applies without a new reader or kernel change (#101).
- Keep radius leaders on the drawn part of trimmed arcs (quarter-cylinder edges, obround
  and slot-pattern ends) instead of the full construction circle.
- Prefer perpendicular location witnesses on either side and finite-gap witnesses before
  any oblique fallback; a fallback uses parallel 30° witnesses clear of material and other
  dimensions. On the 30 quality parts oblique witnesses fall from 16 dimensions to none.
- Lint radius-leader targets on circular steps and slot ends, and oblique witness ink.
- Stop reporting a STEP material as undrawn when the title block states it; retain the
  warning when a blank or conflicting Spec material replaces it.
- Describe missed feature recognition and unsupported STEP annotations in drawing language,
  with Spec remedies; retain tool/version and reader diagnostics in structured references.
- Prefer meaningful STEP product names for default titles, otherwise print readable file
  stems with spaces. Explicit Spec titles, including blanks, keep precedence.
- Explain that hole tables require X/Y locations from a datum corner and recommend the
  retained individual dimensions when those locations are absent.
- Clarify that view placement gaps measure between annotation bands, not part outlines,
  in the Spec schema, MCP item options and documentation. Tools/list stays at 34,477
  compact schema bytes.

## 0.1.0-alpha.5 (2026-10-18)

### Added

- Consume oriented chamfers from native recognition, claim their faces, and include equal
  oblique and principal bevels in one counted true-section C/size-and-angle callout.
- Dimension symmetric V-notches from ordered profile evidence with true included angles,
  theoretical sharp depths and datum locations. Their ordinary IDs support dimension edits,
  authored ownership and location schemes; lint verifies live values against source geometry
  and printed scheme links. Reserve acute-notch arcs before layout; bevel run midpoints do
  not introduce artificial automatic datum stations. Existing PMI angles on the same
  physical flank faces own the notch angle across normal-section heights, retaining their
  tolerance and implicit degree notation without a duplicate automatic arc. An existing
  bevel first leg owns a matching physical notch depth, keeping its callout without an
  extra depth line.
- Read native recognition/3 unified section recesses, source refusals, face levels
  and risers. The default path now uses the existing pocket/channel/blind-slot/
  passage drafting grammars and no longer warns that section recesses are absent
  from the document. Native and Python 0.4 readers retain unified pocket patterns: grouped count
  and size/depth callouts, pitch and absolute locations, independent coverage
  outcomes, and four retained radius requirements per rounded pocket. Five pending
  recess-reader rules and four formerly bridge-only rules have native regressions.
  The kernel is pinned to quiddity-rust release `v0.1.0-alpha.6`.
  See docs/native-recesses.md for reader and drafting details.

- Authored detail `from` selects an existing principal or section parent; `around` accepts
  a feature/face ID or a nonempty list whose crops are united. Missing members and
  degenerate projections in an explicit parent are refused. Mixed natural sources choose
  the view with most visible members and explain the choice in the basis. Saved strings
  remain compatible.

- Enlarged orthographic detail views for crowded turned chains and heads, thin terminal
  flanges, prismatic height ladders and authored feature crops. Details participate in sheet
  planning, carry stable view IDs and their own scale, and draw parent circles, letters,
  captions and relocated dimensions. A fitting iso keeps sheet scale beside a detail.
  Verification and Python differences: [detail views](docs/detail-verification.md).
- MCP `spec_schema {}` returns the full draft-2020-12 author Spec schema and its
  named `get_spec` read shape on demand. Tool schemas retain explicit typed top-level
  outlines. Against alpha.4 the per-session schema context falls from 42,464 to 34,477 bytes
  (−18.8%), including this release's new Spec fields.
  Nested Spec additions no longer grow tools/list; server validation and corrective
  guidance are unchanged.

- Spec dimensioning schemes by feature pattern, station set or principal view axis: chain,
  chosen-datum baseline and signed running ordinate, with stable references, session edits,
  one-owner networks and independent printed-link lint (#76 item 4).

- Spec label wording under `dimensions/<id>/text` and prose `annotations/<id>/text`:
  prefix/suffix or one `{value}` template retain every generated measured term. Explicit
  dimension `style: reference | basic` draws parentheses or a box. Full ink drives placement;
  plans expose the overrides and lint still reads live measured text (#76, item 5).
- Session/editor text and style options use existing Spec operations, preview and history;
  saved Specs round-trip and stale/merged label IDs are refused with a correction.
  Large MCP option replies retain full structured data and a lossless text-channel resource.
- Recognition corrections in the Spec (#76 item 2): reclassify supported cylindrical and
  rectangular features, group holes as a linear/grid/bolt-circle pattern, split recognised
  patterns, suppress spurious recognitions, and declare missed features from face fingerprints.
  Geometry supplies every size; saved face anchors and authored IDs survive re-analysis.
  Corrections share ordinary dimensions, schedules and independent lint, with `from_spec`
  provenance, transactional editor operations and MCP schemas inside the existing byte budget.
- Geometry-authored dimensions (`dimensions/d:authored:<name>`): principal or true distances,
  cylindrical/spherical diameters and radii, and planar-face/straight-edge angles, with stable
  fingerprint references, ordinary tolerance/placement, editor/session operations and independent
  geometry lint (#76 item 1). Duplicate drawn quantities are refused with their existing owner.
- Spec line-work switches per view and drawing-wide sheet defaults: hidden lines,
  centre marks, centre lines, thread depiction and knurl depiction (#76 item 6).
  Session options/edits support them; effective paths are reported in view `from_spec`.

### Changed

- Planning is faster on PMI-heavy AP242 files (#90): linear-dimension trials reuse their
  stroke geometry and footprint witness counts are memoised; with quiddity-rust alpha.6's
  bounded emptiness probe, NIST FTC-10's AP242 plan falls from 203 s to about 5 s. Every
  plan JSON and SVG on the 65-part survey is byte-identical to the output before this change.
- The plan-time budget is re-recorded at 2,352,592,623,176 instructions on the 65-part
  survey (from 3,166,420,524,176 after native recesses; margin still 15%).
- Development: integration tests are grouped into eight binaries, halving CI's build step,
  with a guard against unreachable test files (#91); long test loops are split into
  individual cases (#89); release-only PRs skip the build in CI (#86).

### Fixed

- Preserve archived Python 0.3.9 pocket callouts and individual datum locations.
- Repair displaced location witnesses through foreign features and overlapping
  callout text with bounded route moves that retain the existing physical guards.

- Avoid repeated projection and collision work during complete sheet searches with native
  recesses, retaining the original native and WASM release settings.


- Automatic sheet candidates require dimensioned turned-step intervals to span at least
  the printed annotation capital height in their owning view, including baseline replacements.
  Explicit scales retain their geometry and warn below this floor. A complete, legible
  A3 drawing keeps practical paper instead of growing to A2 solely for fill.
- Keep geometry-authored angle arcs local to their support intersection and place aligned
  dimensions at a clear parallel offset, moving unpinned linear dimensions out of their way
  before escalating the sheet.

### WASM API

- Additive Spec `dimension_schemes` map and scheme measurement claims; native and browser
  sessions share the same JSON vocabulary. Scheme fragment edits refresh placement options.
  Engine method signatures are unchanged.
- Additive recognition-correction Spec fields and Plan provenance use the shared native
  session model; binding methods and TypeScript signatures are unchanged.

- Establish the reviewed TypeScript API snapshot and changelog gate, a measured WASM size
  ceiling, and warning-only cross-package quiddity pin checks in CI and release notes.
- Additive Spec line-work switches and optional Plan view `line_style`/`from_spec`
  fields, shared with native sessions; no binding method changes.
- Spec JSON accepts authored feature crops (`detail`, `around`, `from`, `scale_factor`)
  and optional Plan view `detail` records; engine method signatures are unchanged.
- Additive Spec text/style fields and annotation/note `from_spec` JSON provenance
  through the existing session methods; no new JavaScript methods or signatures.
- Spec JSON accepts authored dimension fields; Analysis/7 exposes optional exact
  dimension-reference evidence. Engine method signatures are unchanged.

## 0.1.0-alpha.4 (2026-10-16)

### Added

- Browser `draftwright-wasm` workspace crate, moving the editor's live-session wrapper here
  (#69, #66). JSON-string operations and errors, SVG/PDF bytes, bundled fonts and panic hook;
  byte-based analysis and native-only adapter gates keep the browser independent of files.
- Dedicated `wasm-release` profile, Node/native session parity CI, and a published-release
  workflow attaching the web package and SHA-256 checksum. See `docs/wasm.md` for builds,
  package sizes and switching the editor to a pinned release asset.
- Spec-authored external spur gears, with an ISO 1328 gear data table, validated anchors
  and gear coverage and reconciliation lint; gears are declared, not recognised (#67).

### Changed

- Small parts try a measured single A4/A3 sheet, then a second A4/A3 sheet, before larger
  paper; second-sheet views are centred and section references name the right zones (#63).
- ISO 286 fit callouts show the class alone by default; `sheet/fit_deviations: true` draws
  stacked upper and lower deviations after the class (#72).

### Fixed

- Datum placement skips searches that cannot improve the result, restoring planning cost;
  planar datum extensions require the face to end within 1 mm of the view outline (#61).
- Pinned labels obey the shared-line guard, omitted authored-set callouts report a restoring
  fix, dimension edits cannot add extension crossings through the part, and thread inference
  requires a positive designation (#62).
- Schedule cells verify tolerance, precision and THRU evidence; manufacturing references
  require complete source-matched ink, and a retained leader crossing rolls back the whole
  table. Damaged cells lose evidence and descriptive tables cannot replace measurements (#64).
- Opposed coaxial taps retain their authored callout sides and GRM-04 retains its small-hole
  callout; plan provenance binds the full recognition input, ownerless occurrences state
  closed policy reasons, and assembly severity reaches every feature coverage family (#65).
- Prismatic bore labels keep their view row and turned bore labels clear the axis; lint
  checks overlaps and manufacturing source text, and finish values respect declared precision
  and multi-word parameters (#68).
- Middle turned bodies use their own corridors; prismatic shoulders need truthful evidence,
  omitted separating views report a restoring fix, double-D callouts preserve their roles,
  countersink seats stay independent, and hole-pattern counts and balloon allocation are
  checked (#70).
- Finish leaders stay on their source faces, oblique PMI uses its exact projected span,
  envelope bands and extent fits reach the ink, and requested precision reaches dimension
  chains and locations. Invalid or conflicting precision is refused; structured notes cover
  only their placed owner's explicit roles (#71).
- Full fit-label ink drives reservation and placement; dimension lines and arrowheads are
  obstacles, final settlement includes callouts, and lint counts linear arrow flares (#72).

### Fixed

- Optional detail requests no longer veto valid ordinary drawings or cap principal scale.
  Cropped edges meet thin zigzag break lines; captions follow real projected ink. Prismatic
  details keep the feature silhouette and use preferred enlargement scales. Extent queries
  map only the envelope, once per projected view, and large-paper omissions precede planning.
- Multi-step turned profiles use one inspection end by default, preserving PMI datum
  precedence and saved chain IDs. Cross-hole axial locations use the same end; baseline
  lanes run shortest nearest the part. Valid complete larger principal views take precedence
  over optional details under the practical-paper sheet rules.
- Visible edges take precedence over hidden edges in the isometric as well as principal
  views, including when a detail changes the isometric scale.

## 0.1.0-alpha.3 (2026-10-16)

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

- Draw one chosen assembly part through the CLI (`parts`, `--part`), API, sessions and MCP,
  with its own geometry, PMI and material; drawing an assembly whole reports it (#28).
- AP242 external threads replace turned-diameter callouts with their designations; profile
  views draw thread minor-diameter lines and knurl bands (#35).
- AP242 surface textures draw as ISO 1302 face symbols or a part-wide default finish (#38).
- Pinned callouts accept a paper-space `place.offset`, including positions inside their view;
  sessions report the current offset and pinning it leaves the callout in place (#39).
- First- and third-angle projection via `sheet/projection` (#37); off-scale isometric views
  carry `ISO VIEW (NTS)` (#49).
- ISO shaft fits on turned diameters print their ISO 286 limits; `sheet/fit_limits: false`
  keeps the class alone (#42).
- Spec-authored feature and face datums and GD&T controls, sharing the AP242 drawing path;
  spec entries override STEP PMI and report undeclared datum letters (#47).
- Spec-authored surface finish, threads, knurls and leadered notes on features and faces,
  plus `sheet/default_finish`, through the same paths as AP242 PMI (#52).
- Structural lint checks labels against measured ink, angles against their rays and visible
  edges, datum-symbol distance, and nominal rounding (#50).
- Per-body length chains for parallel turned bodies, raised-pad sizes, heights and locations,
  and counted free-direction slot callouts; lint credits full extents and exact adjoining
  groups, and ambiguous groove ownership is refused (#55).

### Changed

- The Python-rules catalogue (`docs/rules`) applies the maintainer-approved triage of every
  pending rule: 12 retired, 91 translated with the Rust mechanism that meets each, 7 already
  met marked ported, and the 80 left to port tagged with value, size and stream; stale notes
  describe current Rust. `tests/rules_catalogue.rs` checks that every rule says what backs its
  status, every cited test exists and the README tables match the files.

- Pinned haecceity and quiddity to quiddity-rust `v0.1.0-alpha.4` (commit `f5ddb726`), from rev `7cb26d85`.
- Plan problems use drawing language and identify the affected faces and callouts;
  reference dimensions carry `reference` and `determined_by` (#29).
- Turned profiles measured from a datum end use baseline shoulder positions; existing band
  dimension IDs remain covered by those positions (#48).
- Faster hidden-line rendering through quiddity-rust, with render-tail acceptance budgets
  for cgb243 and threaded_connector_109 (#51).
- The single-sheet byte guard is opt-in (`DRAFTWRIGHT_SINGLE_SHEET=1`); its fixtures are kept
  without maintenance and CI no longer records them. Quality, leader-route, lint and test
  gates remain (#53).
- One owner per measurement: authored sets preserve covered items, ladder and shoulder sets
  are approved or omitted whole unless an explicit member entry conflicts, and closed boss
  heights become references. Lint reports redundant pattern locations and toleranced
  references (#57).

### Fixed

- Pinned sheet size and scale are judged on the finished drawing, with a verified fix for
  real misfits (#37).
- The third-angle projection symbol follows ISO 5456-2; basic cross-hole locations work
  across principal bore axes and replace the plain default with a boxed dimension (#40).
- Applied leader fixes re-space overlapping dimension lanes and keep outside text beside
  its line; plan problems and lint report shared lines and stray text (#46).
- Leader routing and lint share the material re-entry check; pinned callouts report re-entry
  with an unpin fix, diameter and radius leaders are checked against their targets, and
  callout arrowhead flares count as collision ink (#54).

- Datum feature symbols try the space inside dimension lanes before outer tiers, can use
  either proven cylinder silhouette, and can stand on drawn planar face extensions. Shared
  GD&T placement counts leader crossings with dimension and extension lines for frames,
  labels and surface finishes too. Complete arrangements cannot introduce a new annotation
  contact to clear an earlier leader; placement intents and pins retain priority.
  Empty placement resets restore automatic tiers. The CNC-review spindle's A and B clear
  the dimension stack.

- NTS captions retry with reduced clearance and nearby vertical positions before retaining
  a clash; structural lint reports shallow caption overlaps with text and line ink. Pinned
  placement refits use the same strict 5% sheet-scale snap as measured refits. An authored
  iso scale only skips infeasible-iso candidates during recovery; other errors propagate.

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
