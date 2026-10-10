# PMI fixtures

The files `tests/pmi.rs` reads AP242 semantic PMI from, and draftwright's reading of each.

- `step/*.stp.gz`, `step/*.step.gz`: byte-for-byte copies of quiddity-rust's AP242 fixtures at
  commit f7a1141 (unchanged at 4cd5494, where this repository pins haecceity),
  `tests/fixtures/ap242/nist/*.stp.gz` (7 NIST MBE PMI test models, AP242 editions 1 to 4, mm
  and inch) and `tests/fixtures/ap242/specify/*.step.gz` (7 files specify-core wrote on corpus parts: fits,
  threads, knurls, datums, positions, a two-part assembly, and a merge into NIST CTC-01). Their
  provenance (NIST release and sha256, specify-core commit and answers) is quiddity-rust's
  `tests/fixtures/ap242/README.md`. Copied, not referenced, so the test needs neither a
  quiddity-rust checkout nor the cargo git checkout of it (CI runs archived test binaries).
- `step/hw_*.step.gz`: byte-for-byte copies of the 8 files haecceity's PMI writer wrote,
  quiddity-rust `tests/fixtures/ap242/write/<name>.step.gz` at 694dc17 (where this repository
  pins haecceity), stem `hw_<name>`: `every_kind` (every kind of item the writer makes, on a
  specify-core part), the intents specify-core wrote written again by haecceity
  (`assembly_plate_pin_intent`, `spool_fits_intent`, `string_post_tapped_intent`,
  `thumbwheel_thread_knurl_intent`), a written thread replaced and removed (`thread_replaced`,
  `thread_removed`, U14) and an add beside specify-core's notes on faces
  (`thumbwheel_notes_add`, U10). Their provenance and OpenCascade cross-check (`.occt.json.gz`)
  are quiddity-rust's `docs/step-ap242.md` and `crates/haecceity/tests/pmi_write.rs`.
- `<stem>.json.gz`: draftwright's `extract_pmi_report` on each of them and on the corpus's NIST
  parts (`nist/*.stp`, geometry only, read from `DRAFTWRIGHT_CORPUS`), written by
  `tools/capture_pmi.py` from draftwright `origin/main` 4dfc63f (OCP's OpenCascade, as
  draftwright's venv has it). `sha256` is that of the STEP text.
  The `hw_*` captures (and those below and in `pmi_drawings/` and `pmi_coverage/`) were added
  later from the same draftwright commit, 4dfc63f.

Regenerate with
`~/repos/draftwright/.venv/bin/python tools/capture_pmi.py <draftwright>/src <quiddity>/tests/corpus`.

- `lowering/<stem>.json.gz`: how draftwright lowers and draws each AP242 file's geometric
  tolerances and datums (`tests/pmi_lowering.rs`): `build_pmi_features` on `extract_pmi_report`'s
  records in the file's own STEP space, one item per geometric-tolerance or datum record (what it
  became, its view, strip, site and what it states), and what `build_drawing(..., pmi="annotate")`
  placed and dropped (or why it could not draw the part: on bracket_positions its projection
  raises). The part's box comes from draftwright's own STEP reader (`_import_step`), as
  build123d's XCAF import crashes on NIST CTC-02 (draftwright #20). Written by `tools/capture_pmi_lowering.py` from
  draftwright `origin/main` 4dfc63f. Regenerate with
  `~/repos/draftwright/.venv/bin/python tools/capture_pmi_lowering.py <draftwright>/src`.
- `dimensions/<stem>.json.gz`: how draftwright lowers and draws each AP242 file's dimensions
  (`tests/pmi_dimensions.rs`): each dimension record of `extract_pmi_report` (its kind, value,
  label, basic flag, stations, cylinders and blockers), the placed terms of
  `build_drawing(..., pmi="annotate")` that cite it (`Drawing.dimension_sources`: its own
  standalone dimension, or a recognised feature's callout that took its tolerance), and the lint
  issues naming it; or why draftwright could not draw the part. Written by
  `tools/capture_pmi_dimensions.py` from draftwright `origin/main`. Regenerate with
  `~/repos/draftwright/.venv/bin/python tools/capture_pmi_dimensions.py <draftwright>/src`.
