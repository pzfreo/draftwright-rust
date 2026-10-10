# PMI coverage fixtures

`<stem>.json.gz`: draftwright's PMI coverage lint (`linting/pmi_coverage.py`) on each AP242 file
of `tests/fixtures/pmi/step/`: every finding of a code that module emits on
`build_drawing(..., pmi="annotate")`'s drawing (draftwright's default build), with its severity,
message, source IDs and annotation name, or why draftwright could not draw the part (on
bracket_positions its projection raises). Written by `tools/capture_pmi_coverage.py` from
draftwright `origin/main` 4dfc63f; `sha256` is that of the STEP text. Read by
`tests/pmi_coverage.rs`.
The `hw_*` files (haecceity-written, `tests/fixtures/pmi/README.md`) were captured from the
same draftwright commit, 4dfc63f.

Regenerate with
`~/repos/draftwright/.venv/bin/python tools/capture_pmi_coverage.py <draftwright>/src`.
