# J3 shaft-fit regression (#30)

`spindle_shaft_fit.step.gz` is a deterministic gzip copy of the maintainer's editor
CNC-review sample `cnc-review/spindle.step` (draftwright-editor's turned spindle with the
PMI its reviewer wrote: a g6 fit on the Ø15 journal, datums A/B, a position tolerance on the
Ø4 H7 cross hole, heights from A), supplied for stream J3 on 2026-10-15. No model geometry or
PMI was edited.

Uncompressed SHA-256: 3322957335fecbdc051b583c6ce763cdfbd356a5ef3349c22c3e3fd9a4b9d39a

The regression (`tests/pmi_dimensions.rs`) checks that the Ø15 journal's `LIMITS_AND_FITS('g','shaft','6')` is drawn on
the turned diameter as `ø15 g6` by default, with no refusal for it. Per the maintainer's
2026-10-16 decision, `sheet/fit_deviations: true` adds stacked ISO 286-2 deviations
`-0.006` above `-0.017` at smaller text height; the regression checks both forms.

It is kept in `samples/`, outside the single-sheet byte guard's `parts/` and `pmi/step/`, so
adding it does not change the guard's case set.
