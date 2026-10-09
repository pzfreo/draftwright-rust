# PMI drawings fixtures

What draftwright's drawing shows of each AP242 file's PMI (`tests/fixtures/pmi/step/`), for
`tests/pmi_drawings.rs` (alpha 1's PMI gate).

- `<stem>.json.gz`: `build_drawing(..., pmi="annotate")` on the file (default build, A-series
  sheet, third angle), one item per record of `extract_pmi_report` (its `source_id` joins
  draftwright's reading in `tests/fixtures/pmi/<stem>.json.gz`): what the drawing states of it
  (a dimension term citing it with its label, view, parameter and box; a frame's or datum
  symbol's text tokens and view; a general note; the title block's general tolerance or the
  default surface finish) and the codes of the lint issues naming it; or `error`, why
  draftwright could not draw the part. No positions. `sha256` is the STEP text's. Written by
  `tools/capture_pmi_drawings.py` from draftwright `origin/main` 4dfc63f. Regenerate with
  `~/repos/draftwright/.venv/bin/python tools/capture_pmi_drawings.py <draftwright>/src`.
- `known_pmi_drawings.json`: every difference between draftwright's drawing and the port's, with
  a verdict and evidence. A difference already judged at an earlier stage carries that stage's
  verdict and evidence, named (`known_pmi.json`, `known_pmi_lowering.json`,
  `known_pmi_dimensions.json`).
