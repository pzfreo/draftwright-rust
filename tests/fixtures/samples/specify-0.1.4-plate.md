# Specify 0.1.4 plate (#265)

`specify-0.1.4-plate.step.gz` is draftwright-editor's `public/samples/specify-sample-plate.step`
(the editor's Plate sample; SHA-256 `f1ad7d88…b210`) written by specify-core-rust v0.1.4
(`fd184f0`) with `specify-core write --accept-defaults` and the single answer
`{"part.material": "Aluminium 6082-T6"}`, then gzipped with `gzip -n -9`. Nothing was edited.

Uncompressed SHA-256: 76169350ee7f22bc368c6394896b94e92924635fc0b4b51e9db8099146359570

Specify 0.1.4 writes the ten sizes and plane levels the general tolerance covers as their own
nominal-only dimensions (`specify-core:written:v1`, no tolerance), and a part-level
`specify characteristic numbers` attribute set holding its numbering history. The regression
(`tests/pmi_dimensions.rs`) checks that those dimensions are drawn as draftwright plans that
geometry, not on top of it, that one with no planned equivalent is planned as draftwright's own, that strict mode still draws them as the file's PMI, and that the
history set is bookkeeping.

It is kept in `samples/`, outside the single-sheet byte guard's `parts/` and `pmi/step/`.

`specify-0.1.4-plate-plus-minus.step.gz` is the same part written by the same Specify through
draftwright-editor's WASM build (`writePmi(acceptDefaults(answers))`), with the answers the
editor's Plate sample starts with: `{"part.locating": "± dimensions",
"hole.function:holes:13.25": "general", "hole.function:hole_patterns.rectangle:6.12.15.24":
"general"}`. Locating by ± makes Specify write the holes' sizes and locations as general
dimensions too, and the drawing states them in its hole table. The regression
(`tests/editor_session.rs`) checks that the PMI report lowers each to the item that draws it,
the hole table included.
