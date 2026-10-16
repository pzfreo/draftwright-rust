# Fit-label layout regression (2026-10-16)

The maintainer supplied `turned_spindle.step`, exported from the specify-core editor with
AP242 PMI (ø10 g6, ø15 h6, threads, knurl and Ra 3.2). The exact bytes are preserved in
`tests/fixtures/samples/turned_spindle_fits.step.gz`; its adjacent Markdown file records the
provenance and uncompressed SHA-256. No geometry or PMI was synthesized for this reproduction.

On main (`0f871b16`), native analysis and default rendering select A3 at 2:1. The g6 label
spans the 18/52 dimension stack: its glyph box starts at y=183.314 mm, just 1.314 mm above
the upper extension line at y=182 mm. This reproduces the screenshot's crowding; the exact
glyph bounds do not literally intersect that line. The h6 shaft is 49.25 mm long and follows
the groove and knurled base to its label below the view. Neither raises a text overlap.

## Cause and correction

PMI lowering already inserts the fit and deviations before `label_ink` measures placement.
The stale-width hypothesis was not confirmed at that stage. Three downstream gaps caused
this drawing:

* Packing counted lanes but left only a generic margin for a turned diameter's alternate
  shelf. The complete label could therefore eliminate the other-side candidates before
  routing. Reservation now resolves the same printed terms as placement, including authored
  tolerances, fit display settings, consolidation counts and references, and measures the
  entire label with the drawing font. Turned diameters reserve their alternate shelf.
  Its allowance uses the router's elbow clearance, shoulder, text gap and 2 mm of paper
  to the border. The finished-fit correction can trim an unused off-paper reserved edge
  only after the complete rendered footprints prove no ink occupies it. This avoids
  unnecessary sheet growth for the toleranced string post (still A4 at 5:1).
  Authored linear tolerances keep the established projection anchors while their full
  printed extents widen the reserved blocks; unrelated dimensions are not recentered.
* The callout router tested fixed dimension/extension strokes and arrow flares against exact
  glyph bounds, while label/label contacts had a margin. Compound fit/tolerance text now seeks 2 mm of paper
  from annotation strokes. Clearance is scored separately below actual ink contacts, so
  actual collisions and material crossings take priority over breathing room. Ordinary text
  retains its established corridors and pin locality. The existing side,
  row, tier and escape search chooses a clear shelf. No fit string or fixture-specific coordinates select it.
* Semantic GD&T is placed after callouts, while the final settlement pass moved only linear
  labels. A free callout whose text meets that late ink now retries the full existing candidate
  search with all settled dimensions, extension lines, arrow flares and GD&T fixed. It must
  reduce text contacts without increasing hard conflicts, crossings or material crossings.
  Authored placements and unaffected routes stay fixed. This keeps both fit display modes of
  the intent spool readable at A3 1:2 rather than falling back to a tiny A4 1:5 drawing.

The final-plan collision audit now includes every dimension's filled arrowhead, rather than
only callout arrows. Rendered evidence retains arrowhead outlines separately from witness
and leader strokes, so structural lint reports them without changing dimension-reading
grammar. An arrow flare over text is reported even when the centre shaft misses it.

## Fit display decision

Per the maintainer's 2026-10-16 decision, the default is the ISO 286 class alone: `ø10 g6`,
`⌀4 H7`. `sheet/fit_deviations: true` adds upper over lower at 70% text height, after the
class, without brackets. Signs and precision remain unchanged and zero is `0`.
The plan label and searchable text encode the stack with a newline, e.g.
`ø10 g6 -0.005\n-0.014`. The same renderer measures and draws the stack, including its
increased height. Scheduled hole size cells use that renderer too.

The setting applies to PMI turned diameters, PMI hole fits, spec fits, editor sessions and
MCP. The old `fit_limits` setting remains an explicit compatibility spelling; the new setting
takes precedence. Explicit ± and band tolerances keep their text regardless of this setting.
The field is documented with the Spec in `docs/model.md` and exposed through MCP schemas
and sheet item options. Canonical plans used by editor/MCP pins retain the chosen display
setting, so an enabled stack keeps its measured shelf when pinned. Redundant inherited
schema type declarations were removed to keep
the existing schema byte limit unchanged.

Intentional text expectation changes:

* `tests/fixtures/known_pmi_dimensions.json` and
  `tests/fixtures/pmi_drawings/known_pmi_drawings.json`: native shaft and hole fit expectations now keep
  the class alone. The evidence records this decision; Python/source observations are retained.
* `tests/pmi_dimensions.rs`, `tests/pmi_thread_knurl.rs`: the spindle and authored bore fit
  defaults now state only their classes. An explicit opt-in checks the computed upper/lower
  deviations as well.
* `tests/editor_session.rs`: checks class-only defaults and opted-in stacked hole deviations.
* `tests/tolerances.rs`: the format round-trip explicitly enables deviations and checks the
  literal newline stack in SVG/PDF/DXF searchable text. The explicit-off assertion remains.
* `tests/planned_hole_tables.rs`: explicitly enables deviations when checking their retention
  in every scheduled size cell. Internal PMI formatting tests pin the new stack representation.

## Regressions and visual review

`tests/fit_label_layout.rs` checks the real g6 export (default and enabled deviations), a
crowded NIST CTC04 H7 hole pattern, and CTC01 with a `+0.03 -0.0` band. Each checks complete
label width against the drawing font and all foreign linear strokes and arrow flares with
2 mm clearance. The spindle also checks a short h6 route and unchanged A3 2:1 selection.
All three geometry regressions failed on unmodified main; the actual failures are retained
in `target/fit-label-review/regressions-before-current.log`. Additional tests check the tall
stack's bounds, searchable text, setting precedence and structural lint's arrow-flare evidence.
The intent-spool regression also pins its legible A3 1:2 scale and checks that neither fit
display mode has a label/label or stroke/text overlap after GD&T placement.

Before/after SVG, PNG and plan JSON evidence is under `target/fit-label-review/before/` and
`target/fit-label-review/after/`, with these stems:

* `turned_spindle_fits`
* `spindle_shaft_fit`
* `hw_spool_fits_intent`
* `spool_fits`
* `nist_ctc_01_band` (CTC01 with the checked asymmetric band spec)

`target/fit-label-review/after-deviations/` also holds all five with deviations enabled. The before binary is unmodified main; fixture paths, native
recognition and the NIST band spec are the same on both sides. The render script and logs
are retained in the evidence directory. `turned_spindle_editor_export` in the before/after
directories also renders the original supplied `.step` path directly, retaining its filename
and title, rather than reading the gzip fixture.

The chosen layout uses short shelves beside the journals. On the real export, default g6
reads from the left and h6 from the right; enabled deviations can use two left shelves. Both
keep the dimension stack readable without increasing sheet size. The intent spool's f7
callout moves beside its neck and R10 clears datum A at the bottom left. The plain spool's
explicit bands retain their text. The NIST band's label moves left of the right-hand stack.
The plain spool now draws more legibly on A3 at 1:2 instead of A4 at 1:5.
Datum placement keeps its existing compact-first search inside dimension corridors. The
existing rule restricting A2 and larger to parts needing them at 1:1 is unchanged.

## Final verification

With the Cargo environment sourced, `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`:

* `cargo fmt --check`: pass.
* `cargo clippy --all-targets --release -- -D warnings`: pass.
* `cargo test --release --no-fail-fast`: **1,044 passed, zero failed, zero ignored**,
  across 107 test targets/results. This includes `tests/rules_catalogue.rs`, the original
  sheet policy and placement assertions, all six new regressions, and the multi-sheet latency gate.
* `python3 -I tools/quality.py` over the exact 30 parts in `tools/quality_parts.txt`, followed by
  `tools/ci_acceptance.py --check-results quality` against `tools/ci-quality-baseline.json`:
  pass. All **300 accepted metric cells are identical** to the original baseline. There is
  no baseline improvement or regression to record; the baseline is unchanged.
* `python3 -I tools/check_leader_routes.py target/fit-label-review/quality`: pass for
  **119 callouts on 30 parts**. No additional hidden/section/cutting-plane crossings,
  longer routes beyond the existing allowance, or new runs over 2 mm. The 14 pre-existing
  runs remain unchanged.

Logs are `fmt-check.log`, `clippy.log`, `full-tests.log`, `quality.log`,
`quality-acceptance.log` and `leader-routes.log` under `target/fit-label-review/`.
The final drawings were visually inspected in both fit display modes.

Earlier diagnostic runs hit the unchanged 150 ms multi-sheet wall-clock gate: edit/MCP
p90 376.706/246.990 ms (`full-tests-before-style-pin.log`), and MCP p90 198.803 ms
(`full-tests-before-linear-anchors.log`, edit p90 78.280 ms). These failures are retained;
the final complete run passed the gate. No timing threshold, quality baseline or test
assertion was relaxed.
