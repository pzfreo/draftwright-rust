# Specify 0.1.4 plate (#265)

`specify-0.1.4-plate.step.gz` is draftwright-editor's `public/samples/specify-sample-plate.step`
(the editor's Plate sample; SHA-256 `f1ad7d88…b210`) written by specify-core-rust v0.1.4
(`fd184f0`) with `specify-core write --accept-defaults` and the single answer
`{"part.material": "Aluminium 6082-T6"}`, then gzipped with `gzip -n -9`. Nothing was edited.

Uncompressed SHA-256: 76169350ee7f22bc368c6394896b94e92924635fc0b4b51e9db8099146359570

Specify 0.1.4 writes the ten sizes and plane levels the general tolerance covers as their own
nominal-only dimensions (`specify-core:written:v1`, no tolerance), and a part-level
`specify characteristic numbers` attribute set holding its numbering history. The original #265 regression replanned all ten. They all reference datum A, B or C;
the maintainer's 2026-10-31 decision now retains their authored PMI baselines. The
regressions in `tests/pmi_dimensions.rs` check those sources, keep the no-datum #265
policy independently, and retain the history set as bookkeeping. Keeping these ten
references intentionally costs paper; they must not be restated to recover the old sheet.

It is kept in `samples/`, outside the single-sheet byte guard's `parts/` and `pmi/step/`.

`specify-0.1.4-plate-plus-minus.step.gz` is the same part written by the same Specify through
draftwright-editor's WASM build (`writePmi(acceptDefaults(answers))`), with the answers the
editor's Plate sample starts with: `{"part.locating": "± dimensions",
"hole.function:holes:13.25": "general", "hole.function:hole_patterns.rectangle:6.12.15.24":
"general"}`. Locating by ± makes Specify write the holes' sizes and locations as general
dimensions too. The no-datum sizes remain in the hole table; datum-referenced
locations now keep their own PMI dimensions. The regression in
`tests/editor_session.rs` checks that the report names each actual drawing item.
