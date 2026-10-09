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
- `<stem>.json.gz`: draftwright's `extract_pmi_report` on each of them and on the corpus's NIST
  parts (`nist/*.stp`, geometry only, read from `DRAFTWRIGHT_CORPUS`), written by
  `tools/capture_pmi.py` from draftwright `origin/main` 4dfc63f (OCP's OpenCascade, as
  draftwright's venv has it). `sha256` is that of the STEP text.

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
