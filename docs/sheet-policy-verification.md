# Sheet policy and automatic detail verification — 2026-10-18

Before is the freshly fetched `origin/main`, `d96f067b`, including #91's eight
integration test binaries. After uses production source `7b16a06e`, including the
native 419 profile correction, independently readable ordinary main views,
page-marker boundary clipping, material-edit preparation reuse and bounded
Session main preparation, and the final FTC-07 span-label rejection. Later
commits record evidence and documentation;
they do not change the drawing implementation. Both sides use the unchanged
227-part portable content manifest plus the same intentionally unresolved input. STEP basenames are
preserved because drawing titles affect layout.

Plans, native Analysis, source bindings, immutable executables, hashes, logs,
policy audits and PNGs are under `target/sheet-policy/final/`. PNGs are rendered
natively from STEP on each side, matching the survey. They do not use the saved
Analysis rendering path: rereading a bound Analysis can choose a different
planning inventory. Saved Analysis is used for the identical readability audit.

## Implemented rules

1. Solve the whole-part drawing without automatic details first. An automatic
   detail cannot lower its main scale, enlarge its paper or add physical sheets.
   An independent native replay with `sheet.automatic_details: false` verifies
   every final automatic-detail drawing against this literal switch setting.
2. Require a placed, non-reference controlling dimension or callout transferred
   from the parent. Bore crops sized only to fit a long label are refused.
   Reference intervals can accompany a geometric control; they cannot
   earn a crop alone. Every child label must also pass the existing off-span
   association check, including reference companions. Unannotated depiction crops are refused. Existing parent
   references remain where appropriate or are omitted as before.
3. Crop and parent marker use the same bounds. New local crops include adjoining
   outline and have aspect ratio at most 3:1. Complete long feature profiles can
   retain their connected silhouette; the visual review records that exception.
4. Reserve caption widths and place every caption before choosing marker letters.
   Automatic captions and letters clear other text by at least 1.75 mm. Circle
   strokes are interrupted behind annotation text. Readability captions use the
   standard `DETAIL A (2:1)` form; clipped outlines retain their break marks.
5. Nearby needs share a crop. The generator and final eligibility check allow at
   most two automatic details per document, also enforcing the per-sheet limit.

A refused crop keeps an otherwise fitting, single-sheet whole-part drawing.
Dropping the crop from a larger-paper trial cannot make that trial eligible.
Ordinary paper recovery still addresses general whole-part floor/fit failures or
an existing multi-sheet allocation. An otherwise readable ordinary seed is also
protected from a reduction that would introduce opening or depiction failures;
its readable-scale ordinary solve must independently pass every fit, fact and
route gate. Useful fitted profile controls are preserved
on the ordinary drawing's paper and main scale when a replacement would lose
that profile's context. An unfitted or differently scaled initial proposal is
not the whole-part comparator.

Reductions that cut a clean readable main view by more than half are refused.
Halving a principal view with at least eight real controls and unresolved local
geometry is also refused. These guards preserve the large main views requested
in review; readability counters do not override the visual comparison.

`sheet.automatic_details: false` remains available. Explicit detail authoring,
paper, scale and pins remain authoritative. Held labels preserve their actual
canonical schedule membership and covered scalars, including rows whose size
callout changes carrier. The unchanged exact row and placement assertions test
this behavior.

All existing numerical limits remain: 2 mm openings and prismatic intervals,
2.1 mm turned intervals, 0.7 mm depiction gaps, 10/7 mm dimension lanes, the
retained +2 mm leader allowance, and the existing padded contact and material
association checks. Rejected reference-only, label-only and unannotated crops
now require explicit refusal assertions instead of artificial zero-failure detail recovery.
Measurement, pin geometry, exact crossing-pair and original clearance assertions
remain. No quality baseline, instruction budget or threshold was lowered.

## Verification environment and order

```sh
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
export DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus
export DRAFTWRIGHT_CORPUS_REQUIRED=1
```

The recorded order is formatting, strict workspace/all-target release Clippy,
the affected grouped binaries, one final full-suite run, MCP acceptance, then
quality generation and its leader-route and quality-table checks. The
quality-pins, repeated-spans, render-tail and plan-time acceptance jobs are left
to CI. Supplemental CI coverage grows from 74 to 77 inputs for the new native
regressions; the original 65-input instruction cohort and its budget are unchanged.

An earlier full audit ran 1,505 nextest tests: 1,503 passed and two failed.
The structural audit found a marker arc 4.7 billionths of a millimetre below
the drawing border;
marker clipping now includes a numerical inset, with an exact-border regression
and the original lint tolerance retained. R5 title edits exceeded the existing
150 ms gate. Short material declarations that fit every reserved cell, with no
STEP material precedence, now share the material-free main preparation; the
actual material is restored before allocation scoring and drawing. Allocation
still executes its real prefixes and counts. Oversized material text uses the
full path and retains overflow and off-sheet findings. An exact-plan comparison
tests both paths. These failures are fixed before the final full run below.

The affected NIST session benchmark also exposed repeated main preparation for
placement edits (561–1,008 ms against the unchanged 150 ms gate). A Session now
owns a separate 32-entry memo keyed by complete Analysis, Spec, trial scale and
planning policy. Only pure main solves are retained: nested allocation or a
scope-policy change prevents insertion. Allocation still executes its real
prefixes. Existing preview/delta/fresh-Plan/SVG/DXF/annotation comparisons and
PMI edits remain authoritative. Diagnostic instrumentation was removed before
these final checks; no latency gate was disabled or raised.

MCP preflight initially rejected the expanded inventory because its guard still
required 74 inputs. The runner now requires exactly 55 corpus inputs plus 22 PMI
fixtures, hashes the supplemental files too, and checks exact 20/19/19/19 shard
sizes. Five focused acceptance inventory/document tests pass. All original input
requirements, the 65-part instruction cohort and every quality/performance limit
are retained. This correction changes no Rust or rendering code.

The first final quality-table check found one new off-span reference label in
FTC-07's automatic detail. The existing settled-label predicate now rejects
such settled crops, without changing its
5.7 mm limit or the quality baseline. FTC-07 keeps its ordinary A2 1:2 drawing,
all original controls and an explicit local readability refusal. The independent
automatic-detail replay also checks this unchanged Python quality predicate.
A native 520 regression checks a useful controlling crop, rejection when only
its reference companion is displaced, and unchanged authored-detail treatment. The corrected source is re-surveyed and re-rendered;
its library, rendering and Session/latency checks precede one new full-suite run.
The earlier successful full run and failed quality table remain archived at
`target/sheet-policy/proof-3b0af09/`.

An initial implementation also counted these labels in general recovery scores.
That triggered unnecessary NIST Session searches and failed the unchanged
150 ms p90 gate (464 ms, then 919 ms in isolation). The final correction keeps
span association at the strict automatic-detail eligibility gate and restores
the original general recovery counters. Its focused native Session run passes
at 110.908 ms; the final serial check and full suite below recheck the same gate.
The failed run is archived under `target/sheet-policy/proof-span-counter-diagnostic/`.
No existing assertion, recovery threshold, latency gate or quality baseline changed.

The general-counter correction passed its first focused timing run but the next
serial Session check still failed at 264 ms. Phase measurements exposed an
oversized preview arena discarding the established reference and repeated
nominal seed preparation. The final implementation trims all disposable
candidates and policies at the unchanged 256-entry boundary, retaining only
the Analysis digest and at most eight comparison recipes keyed by presentation.
Changed complete Analysis still invalidates these references. Policies are
rebuilt normally. A separate sixteen-entry Session seed memo follows comparison
preparation, under complete effective Analysis, Spec and policy keys. Only
seeds with no nested allocation and no policy transition qualify; actual
allocation runs and counts remain real. The native reference-retention unit,
520 eligibility regression, R5 count/latency check and FTC-07 zero off-span
quality check pass. Focused NIST p90 falls to 66.050 ms, and the final checks
below remeasure the unchanged gate. Temporary instrumentation was removed.
The failed intermediate runs and phase timings remain archived separately.

The final eligibility audit also forces automatic details on FTC-07 with
explicit A2 paper and 1:2 scale. This exposed a fixed-main early return that
bypassed crop eligibility. That path now compares against literal switch-off
recovery and refuses the bad crop, preserving every placed real control, the
explicit frame, and the original enabled Spec switch. Authored details remain
in both passes. The new native regression passes; the interrupted prior survey
and checks remain archived. The complete evidence below is regenerated on
this corrected source, including the full final suite.

The first timing check on this source failed while aggregate host load rose
above 50 on 18 logical CPUs; other Session cases slowed too. Its receipts and
logs remain in `timing-contention/`. Functional library/rendering checks and
the native survey are completed first. The unchanged Session and R5 timing
checks then run after native work finishes and aggregate load falls below 12.
No latency assertion is disabled, increased or skipped. The final full suite
uses its existing exclusive benchmark scheduling.

The preceding full run completed 1,510 nextest tests with 1,509 passing; its
NIST scale-edit latency reached 298 ms. The shared 37/13-test harnesses and
doctests passed. A new deterministic regression then reproduced unkeyed
routing policies surviving small retained arenas but disappearing when the
256-entry budget was crossed. Each invocation now starts with fresh policy
inputs while preserving keyed preparation and the unchanged budget. The same
complete Analysis digest is reused for prefix keys. Phase measurements also
showed commit recomputing preview geometry despite an existing drawing memo.
The one-entry render memo now retains real geometry and its actually prepared
projection cache, keyed by the freshly planned result and mutable PMI. Planning,
actual allocation prefixes, annotation ink and delta computation still execute.
The deterministic scope regression, native crop regressions and R5 count gate
pass; focused NIST p90 is 36.959 ms with exact fresh-Plan/SVG/DXF/evidence/history
assertions unchanged. Profiling is removed. The full run below rechecks this
corrected source; the earlier failed run remains archived at
`target/sheet-policy/proof-fixed-main-details-policy-diagnostic/`.

## Final paired results

| Paper | Before | After |
| --- | ---: | ---: |
| A4 | 97 | 143 |
| A3 | 103 | 56 |
| A2 | 9 | 11 |
| A1 | 12 | 11 |
| A0 | 6 | 6 |
| Sheets | 227 | 229 |
| Details | 6 | 13 |

8 parts gain details; 60 change paper, scale, sheet count or detail count. Fully readable drawings: 157 → 186. All remaining failures carry an explicit recovery refusal. The same unresolved fixture is refused on both sides.

No measurement facts or PMI/GDT/requirement claims are lost, and no non-diagnostic drawing-finding identities are added. The paired audit checks exact nominal values. See [CSV](sheet-policy-results.csv) for every part and the paper-selection reason.

Parts gaining details: `corpus/inventory_refusal/13975.step.gz`, `corpus/mfcadpp/10000.step`, `corpus/mfcadpp/10049.step`, `corpus/mfcadpp_holdout/163.step`, `corpus/mfcadpp_holdout/520.step`, `corpus/mfcadpp_regressions/11512.step`, `corpus/nist/nist_ctc_03_asme1_rc.stp`, `fixtures/rule_pins/tuner_jig_blind_obround_pockets.step.gz`.

Every paper increase: `corpus/cadgenbench/threaded_connector_109.step`: A3 1:5 / 1 / 0 → A2 1:2 / 1 / 0; `corpus/mfcadpp/1004.step`: A4 1:1 / 1 / 0 → A3 2:1 / 1 / 0; `fixtures/parts/boss_circle.step`: A4 1:2 / 1 / 0 → A3 1:1 / 1 / 0.

Independent native switch-off replay passes for all 13 drawings carrying 13 automatic details. Every retained automatic detail transfers a real placed control from its parent, preserves at least the independently solved main scale, uses no larger paper or extra physical sheets, and matches its marker bounds.

Executable SHA-256 bindings:

| Stage | SHA-256 |
| --- | --- |
| before | `02c3fd4e93032a2db92d6c433e296c7955e3401e366728cd56223bd42251fbed` |
| after | `97a6bb096cd9763cc24c4dd49a223d5a8797c4dc7417510f2805c26a1254b2f2` |

## Rejected readability changes and their remedies

The review covers all requested parts, every remaining A3→A4 reduction with a
lower main scale, every automatic detail, and the additional cases below. PNGs
are compared at the same 150 dpi; each pair keeps the actual paper dimensions.
The inspection ledger binds every judgment to the exact before/after PNG hashes.

| Part | Less readable or unhelpful trial | Remedy |
| --- | --- | --- |
| 520 | Two thin crops, touching captions, main reduced to 1:2 | Keep A4 1:1. One 5:1 crop carries real 2.0 and reference 0.9, with the bottom outline and adjoining arc. One clear caption and marker. |
| 10167 | A long strip carrying only reference (1.7), with poor correspondence | Refuse the crop; keep A4 1:1 and report the remaining local need. |
| 10000 | Reference-only (1.1) strip | Remove that strip. Keep A4 1:1 and the useful 2:1 slot crop with its real 1.3 × 2.3 × 6.5 DEEP callout. It is now the sole Detail A. The separate remaining wall need stays explicitly refused. |
| 10049 | Main shrank when a crop was added | Keep A4 1:1 with the useful 2:1 pocket crop. The larger initial A3 estimate is not the independent drawing without automatic details. |
| 163 | Main shrank when a crop was added | Keep A4 1:1 with a grouped 2:1 crop carrying real 1.8 and reference 1.9. |
| 10119 | Reduced main views lose the separation of the original A3 drawing | Keep A3 5:1 without an automatic detail; repair the overlapping exterior lanes within the unchanged limits. |
| 274 | Reduced main, or a larger plain A3 trial after the crop failed | Keep the ordinary A4 1:1 drawing without the crop, with an explicit readability refusal. An empty detail trial cannot purchase A3. |
| GRM-03 | A3 10:1→A4 2:1 makes the small main features much harder to distinguish | Keep A3 10:1. |
| sm-hanger | A3 1:2→A4 1:5 compresses the main geometry and dimensions | Keep A3 1:2. |
| 10212 | A4 reduction loses useful main-view scale | Keep A3 2:1. |
| 10007 | My inspection found the halved, densely annotated main harder to read | Reject A4 1:1; keep A3 2:1, with no automatic detail and zero readability failures. The same conservative guard also retains 1017 at A3 2:1. |
| threaded_connector_109 | The A3 1:5 ordinary reduction introduces two depiction failures relative to its readable seed; an unannotated crop cannot qualify | Independently recover the ordinary drawing at A2 1:2. Both gaps become readable without any automatic crop or invented thread callout. The switch-off solve has the same paper and main scale. |
| boss_circle | The smaller ordinary A4 1:2 trial failed seven local floors despite a readable A3 main | Independently recover the ordinary no-detail drawing at its readable opening scale. Keep A3 1:1, also with the switch off, and preserve the unchanged strict drafting assertions. |
| 419 | My final native inspection found two bore-only crops replacing the useful enlarged profile, with little locating outline | Keep A3 1:1 with one complete 2:1 profile and its real 8.2 control. Refuse the font-width-only bore crops; parent annotation problems do not invalidate an independently useful profile. |
| nist_ftc_07 | My quality review found the reference (1.3) label displaced from its span in the automatic detail; the enlargement therefore adds a misleading association | Refuse that crop using the unchanged span-label rule. Keep the ordinary A2 1:2 drawing, its original controls, and the explicit refusal for seven short local spans. The quality baseline remains zero off-span labels. |
| 10098 | My inspection found that an aperture-only replacement lost the useful enlarged profile; dropping that profile also lost useful context | Keep A4 1:1 and restore the 2:1 profile with its real 4.6 and 10.4 controls. Retain the settled parent routes and refuse the separate aperture need. |

The 1013 regression also keeps A3, enlarging its main from 1:1 to 2:1 with no
automatic detail. Its crossed height control uses another orthogonal carrier
that sees the same measured span. Every measurement, witness attachment and
original leader budget is retained; the unchanged zero-crossing assertion passes. Its prior detail is removed once the main view reaches 2:1. This accounts for the net increase of seven details while eight parts gain a detail.

10063 refuses the three reference-only step needs on A4 1:1. The modelled-thread
connector 109 now uses an ordinary A2 1:2 main: both depiction gaps pass at that
scale, without an unannotated crop or invented thread specification. The
explicitly compressed A3 1:5 case still fails the unchanged depiction floor.
On 11512 the controlling front
profile remains at 2:1 in the detail while the simple principal views fit A4 1:1.
NIST CTC-03 keeps its A3 1:5 main and gains a controlled 2:1 local crop. Remaining
local failures are retained in the counters and refusals.

The long-profile exceptions are reviewed explicitly: 10170 retains its approximately
4.14:1 complete connected front profile, with real 5.1 and 10.4 controls.
13975's approximately 3.28:1 detail shows the complete connected long feature and its controlling 1.6 value,
with enough outline to identify it. It is not an isolated reference strip.

## Final visual inspection and PNG index

I personally inspected all 72 native before/after PNG pairs, including all 16 requested parts, all 46 remaining A3→A4 lower-main-scale changes, every automatic-detail drawing, and the additional regressions. Pairs preserve physical paper size at 150 dpi. The final images below are bound to the inspection ledger by SHA-256; earlier inspected images are reused only when both hashes match exactly.

After the remedies above, I judge none of these final drawings less readable than its origin/main counterpart. The retained reductions keep adequate main geometry and dimension separation; 11512 preserves its controlled profile at 2:1 in its detail. This visual judgment does not erase the explicit unresolved local readability refusals.

| Part | Before → after | Native PNGs | Inspection |
| --- | --- | --- | --- |
| corpus/cadgenbench/threaded_connector_109.step | A3 1:5 → A2 1:2 | [before](../target/sheet-policy/final/before/renders/301cb3ce914f-threaded_connector_109.png) · [after](../target/sheet-policy/final/after/renders/301cb3ce914f-threaded_connector_109.png) | The ordinary main grows from A3 1:5 to A2 1:2. Native thread-root and crest gaps are now distinguishable, with clearer band controls and zero depiction failures. No unannotated detail or invented thread callout is added; the switch-off solve selects the same paper and scale. |
| corpus/gramel/GRM-03_thumbwheel_drive_screw.step | A3 10:1 → A3 10:1 | [before](../target/sheet-policy/final/before/renders/b4d455810969-GRM-03_thumbwheel_drive_screw.png) · [after](../target/sheet-policy/final/after/renders/b4d455810969-GRM-03_thumbwheel_drive_screw.png) | A3 10:1 is retained; the 0.5 and 0.3 features and main dimension lanes remain separated. |
| corpus/inventory_refusal/13975.step.gz | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/20226562f507-13975.png) · [after](../target/sheet-policy/final/after/renders/20226562f507-13975.png) | A4 1:1 main is retained. The 2:1 detail shows the complete connected long profile and its controlling real 1.6; its approximately 3.28:1 aspect is justified by the feature itself, with matching parent marker and clear caption. |
| corpus/mfcadpp/10000.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5c37a23fd7b9-10000.png) · [after](../target/sheet-policy/final/after/renders/5c37a23fd7b9-10000.png) | The sole 2:1 crop carries the real 1.3 x 2.3 x 6.5 DEEP slot callout, matching its circled region and showing surrounding outline. Main remains A4 1:1; no reference-only 1.1 strip. |
| corpus/mfcadpp/10007.step | A3 2:1 → A3 2:1 | [before](../target/sheet-policy/final/before/renders/ad95c3b0af77-10007.png) · [after](../target/sheet-policy/final/after/renders/ad95c3b0af77-10007.png) | The same A3 2:1 main views and exterior dimension lanes are retained, avoiding the dense halved trial. |
| corpus/mfcadpp/10033.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/d3501c8c991a-10033.png) · [after](../target/sheet-policy/final/after/renders/d3501c8c991a-10033.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| corpus/mfcadpp/1004.step | A4 1:1 → A3 2:1 | [before](../target/sheet-policy/final/before/renders/5a7cfcf49a1d-1004.png) · [after](../target/sheet-policy/final/after/renders/5a7cfcf49a1d-1004.png) | The A3 2:1 drawing enlarges the thin stock and aperture details, with clear exterior dimensions and no automatic crop. |
| corpus/mfcadpp/10049.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/9b831e372dbe-10049.png) · [after](../target/sheet-policy/final/after/renders/9b831e372dbe-10049.png) | Main remains A4 1:1. The crop matches the circled lower plan pocket, includes its nearby bottom and side outlines, and moves the real 1.5 x 2.5 x 6.1 DEEP callout at 2:1. |
| corpus/mfcadpp/10063.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/b3c3ca453dc2-10063.png) · [after](../target/sheet-policy/final/after/renders/b3c3ca453dc2-10063.png) | Main remains A4 1:1 with the original exterior lanes; no reference-only sliver is added. The local step needs are refused explicitly. |
| corpus/mfcadpp/10098.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/4069220db61e-10098.png) · [after](../target/sheet-policy/final/after/renders/4069220db61e-10098.png) | The useful complete lower profile and real 4.6 and 10.4 controls remain enlarged at 2:1, with the corresponding circled region and clear caption. Parent views remain A4 1:1. |
| corpus/mfcadpp/10119.step | A3 5:1 → A3 5:1 | [before](../target/sheet-policy/final/before/renders/4273d2baaba4-10119.png) · [after](../target/sheet-policy/final/after/renders/4273d2baaba4-10119.png) | A3 5:1 main is retained. The 12.6 and 17.9 coordinate lanes are separated, with no reduction or extra detail. |
| corpus/mfcadpp/1013.step | A3 1:1 → A3 2:1 | [before](../target/sheet-policy/final/before/renders/cf60fc55069e-1013.png) · [after](../target/sheet-policy/final/after/renders/cf60fc55069e-1013.png) | A3 is retained and main grows 1:1 to 2:1. The controlling measurements are clear in the main and section, allowing the old automatic profile crop to be removed without losing readability. |
| corpus/mfcadpp/10155.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/dfcd4d15c66f-10155.png) · [after](../target/sheet-policy/final/after/renders/dfcd4d15c66f-10155.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| corpus/mfcadpp/10167.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/fa26ed75ac96-10167.png) · [after](../target/sheet-policy/final/after/renders/fa26ed75ac96-10167.png) | A4 1:1 main and original clear exterior controls are retained; the reference-only 1.7 strip is refused. |
| corpus/mfcadpp/1017.step | A3 2:1 → A3 2:1 | [before](../target/sheet-policy/final/before/renders/8740d288d903-1017.png) · [after](../target/sheet-policy/final/after/renders/8740d288d903-1017.png) | The original A3 2:1 main views and clear exterior lanes are retained; the crowded halving guard avoids a smaller main. |
| corpus/mfcadpp/10170.step | A3 1:1 → A3 1:1 | [before](../target/sheet-policy/final/before/renders/aa34b9b5ba6b-10170.png) · [after](../target/sheet-policy/final/after/renders/aa34b9b5ba6b-10170.png) | A3 1:1 main and established 2:1 complete front-profile detail are retained. The real 5.1 and 10.4 controls remain enlarged. This long feature itself justifies the elongated profile; captions, marker and section letters are separated. |
| corpus/mfcadpp/10212.step | A3 2:1 → A3 2:1 | [before](../target/sheet-policy/final/before/renders/20de40173fe1-10212.png) · [after](../target/sheet-policy/final/after/renders/20de40173fe1-10212.png) | A3 2:1 is retained with its section and separated controls. No scale loss or automatic crop is introduced. |
| corpus/mfcadpp/10247.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5252edd39905-10247.png) · [after](../target/sheet-policy/final/after/renders/5252edd39905-10247.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| corpus/mfcadpp_holdout/163.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/af3b6fef1ff0-163.png) · [after](../target/sheet-policy/final/after/renders/af3b6fef1ff0-163.png) | A4 1:1 main is retained. One grouped 2:1 crop shows the lower aperture and adjoining bottom outline, with real 1.8 and reference 1.9; caption and marker are separated. |
| corpus/mfcadpp_holdout/274.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/9fc6dd58170a-274.png) · [after](../target/sheet-policy/final/after/renders/9fc6dd58170a-274.png) | Native before and after retain A4 1:1 main views and clear exterior controls; the unhelpful detail is refused without buying larger paper. |
| corpus/mfcadpp_holdout/408.step | A3 1:1 → A3 1:1 | [before](../target/sheet-policy/final/before/renders/0b8a39b594b3-408.png) · [after](../target/sheet-policy/final/after/renders/0b8a39b594b3-408.png) | A3 1:1 main and its useful 5:1 complete shallow-profile detail remain unchanged in scale. Real 3.3 and 5.7 controls are clear, with matching marker and separated caption. |
| corpus/mfcadpp_holdout/419.step | A3 1:1 → A3 1:1 | [before](../target/sheet-policy/final/before/renders/d4356c584a02-419.png) · [after](../target/sheet-policy/final/after/renders/d4356c584a02-419.png) | The useful complete profile and real 8.2 control are restored at 2:1 on A3 1:1. Bore controls remain on settled parent exterior routes. One corresponding marker and clear caption replace the two unhelpful bore-only crops. |
| corpus/mfcadpp_holdout/520.step | A4 1:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/f63dd97d1197-520.png) · [after](../target/sheet-policy/final/after/renders/f63dd97d1197-520.png) | One 5:1 crop shows the circled bottom region, including adjoining arc and bottom outline. Real 2.0 and accompanying reference 0.9 are readable; one caption and marker clear other text. Main stays A4 1:1. |
| corpus/mfcadpp_regressions/11512.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5dc3ff1ba730-11512.png) · [after](../target/sheet-policy/final/after/renders/5dc3ff1ba730-11512.png) | A4 1:1 mains remain clear for the simple principal envelope and 3.0 hole. The controlling front profile with real 1.3 and 8.3 stays at the original 2:1 in the useful complete detail, with clear matching marker. |
| corpus/nist/nist_ctc_03_asme1_rc.stp | A3 1:5 → A3 1:5 | [before](../target/sheet-policy/final/before/renders/95a775b1acdb-nist_ctc_03_asme1_rc.png) · [after](../target/sheet-policy/final/after/renders/95a775b1acdb-nist_ctc_03_asme1_rc.png) | A3 1:5 main and hole schedule remain readable at their original scales. One 2:1 crop shows the circled flange-end thickness with real 1.9 and surrounding corner outline; caption is isolated. Other needs remain explicit refusals. |
| corpus/nist/nist_ftc_07_asme1_rd.stp | A2 1:2 → A2 1:2 | [before](../target/sheet-policy/final/before/renders/a0bc55165678-nist_ftc_07_asme1_rd.png) · [after](../target/sheet-policy/final/after/renders/a0bc55165678-nist_ftc_07_asme1_rd.png) | A2 1:2 principal views, controlling dimensions and hole schedule retain their size and separation. The unqualified crop is absent. The short-height stack remains locally crowded as before and carries an explicit refusal; no detached reference detail remains. |
| corpus/ttt_inputs/sm-hanger.step | A3 1:2 → A3 1:2 | [before](../target/sheet-policy/final/before/renders/8247dc2eb7ed-sm-hanger.png) · [after](../target/sheet-policy/final/after/renders/8247dc2eb7ed-sm-hanger.png) | A3 1:2 is retained; bend geometry, holes and dimension lanes keep their original separation. |
| fixtures/coverage-holes/profile-pentagon.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/a4b2a36fd4a2-profile-pentagon.png) · [after](../target/sheet-policy/final/after/renders/a4b2a36fd4a2-profile-pentagon.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/coverage-holes/turned-step-compound.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/523a9621070b-turned-step-compound.png) · [after](../target/sheet-policy/final/after/renders/523a9621070b-turned-step-compound.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/boss_circle.step | A4 1:2 → A3 1:1 | [before](../target/sheet-policy/final/before/renders/e1f12e3a4fb8-boss_circle.png) · [after](../target/sheet-policy/final/after/renders/e1f12e3a4fb8-boss_circle.png) | The independently recovered ordinary drawing is A3 1:1 with no automatic detail, enlarging the controlled openings and separating their dimension lanes. Both default and switch-off preserve zero readability failures. |
| fixtures/parts/issue_1247_angled_blind_step.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/f56db19a5763-issue_1247_angled_blind_step.png) · [after](../target/sheet-policy/final/after/renders/f56db19a5763-issue_1247_angled_blind_step.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/k5/groove-compound.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/74f64e175c5f-groove-compound.png) · [after](../target/sheet-policy/final/after/renders/74f64e175c5f-groove-compound.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/k5/pad-compound-equal.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/0b8604ddb4f9-pad-compound-equal.png) · [after](../target/sheet-policy/final/after/renders/0b8604ddb4f9-pad-compound-equal.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/k5/pad-x-positive.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5175a9364e5f-pad-x-positive.png) · [after](../target/sheet-policy/final/after/renders/5175a9364e5f-pad-x-positive.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/rules-text/central-prismatic-square.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/0e491b009f34-central-prismatic-square.png) · [after](../target/sheet-policy/final/after/renders/0e491b009f34-central-prismatic-square.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/rules-text/central-prismatic-wide.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/d219df11e856-central-prismatic-wide.png) · [after](../target/sheet-policy/final/after/renders/d219df11e856-central-prismatic-wide.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/rules-text/central-prismatic.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/3e05041a154d-central-prismatic.png) · [after](../target/sheet-policy/final/after/renders/3e05041a154d-central-prismatic.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/rules-text/three-body-pocket.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/ef369ad15c4e-three-body-pocket.png) · [after](../target/sheet-policy/final/after/renders/ef369ad15c4e-three-body-pocket.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/rules-text/two-channels.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/85cbc1f17558-two-channels.png) · [after](../target/sheet-policy/final/after/renders/85cbc1f17558-two-channels.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/parts/turned.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/971dca9ef243-turned.png) · [after](../target/sheet-policy/final/after/renders/971dca9ef243-turned.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/pmi/step/nist_ctc_02_asme1_ap242-e2.stp.gz | A0 1:5 → A0 1:5 | [before](../target/sheet-policy/final/before/renders/99a0a2079dde-nist_ctc_02_asme1_ap242-e2.png) · [after](../target/sheet-policy/final/after/renders/99a0a2079dde-nist_ctc_02_asme1_ap242-e2.png) | A0 1:5 mains and the established 1:2.5 full-profile Detail B remain at baseline scales. I inspected the full pair plus full-resolution detail and principal-view crops: the real 8 and 30 controls, locating outline and caption are retained. The dense existing PMI and hole schedules are unchanged. |
| fixtures/recognition/golden/polygonal_stock.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/46c48aea9688-polygonal_stock.png) · [after](../target/sheet-policy/final/after/renders/46c48aea9688-polygonal_stock.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/bounded_pocket.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/a72d1308242e-bounded_pocket.png) · [after](../target/sheet-policy/final/after/renders/a72d1308242e-bounded_pocket.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/centred_rebate.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/1e7236880c89-centred_rebate.png) · [after](../target/sheet-policy/final/after/renders/1e7236880c89-centred_rebate.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/counterbored_block.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/7667c69558ac-counterbored_block.png) · [after](../target/sheet-policy/final/after/renders/7667c69558ac-counterbored_block.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/curved_centered.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/45907a67440f-curved_centered.png) · [after](../target/sheet-policy/final/after/renders/45907a67440f-curved_centered.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/curved_negative.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5e5b8ad97184-curved_negative.png) · [after](../target/sheet-policy/final/after/renders/5e5b8ad97184-curved_negative.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/curved_offset.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/fe583a5a7863-curved_offset.png) · [after](../target/sheet-policy/final/after/renders/fe583a5a7863-curved_offset.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/curved_rotated.step | A3 1:1 → A4 1:2 | [before](../target/sheet-policy/final/before/renders/18435ce99344-curved_rotated.png) · [after](../target/sheet-policy/final/after/renders/18435ce99344-curved_rotated.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/pocket_square.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/411262cba615-pocket_square.png) · [after](../target/sheet-policy/final/after/renders/411262cba615-pocket_square.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/pocket_x.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/ce19568938b8-pocket_x.png) · [after](../target/sheet-policy/final/after/renders/ce19568938b8-pocket_x.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/pocket_z.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/ec928f335fcf-pocket_z.png) · [after](../target/sheet-policy/final/after/renders/ec928f335fcf-pocket_z.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/tb_bored_plate.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/a626e89b6288-tb_bored_plate.png) · [after](../target/sheet-policy/final/after/renders/a626e89b6288-tb_bored_plate.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/tb_stepped.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/735c8425a3b6-tb_stepped.png) · [after](../target/sheet-policy/final/after/renders/735c8425a3b6-tb_stepped.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/tb_tall_box.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/a6665d3f94a2-tb_tall_box.png) · [after](../target/sheet-policy/final/after/renders/a6665d3f94a2-tb_tall_box.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/issues/u_channel.step | A3 1:1 → A4 1:2 | [before](../target/sheet-policy/final/before/renders/0dfecfecb534-u_channel.png) · [after](../target/sheet-policy/final/after/renders/0dfecfecb534-u_channel.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/polygonal_stock/plain.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/5c0debe81a20-plain.png) · [after](../target/sheet-policy/final/after/renders/5c0debe81a20-plain.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/polygonal_stock/roll17.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/333e414e5488-roll17.png) · [after](../target/sheet-policy/final/after/renders/333e414e5488-roll17.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/polygonal_stock/translated.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/e8eeba466dff-translated.png) · [after](../target/sheet-policy/final/after/renders/e8eeba466dff-translated.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/polygonal_stock/x.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/eb69b8617d32-x.png) · [after](../target/sheet-policy/final/after/renders/eb69b8617d32-x.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/polygonal_stock/y.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/4bc9710ddcd1-y.png) · [after](../target/sheet-policy/final/after/renders/4bc9710ddcd1-y.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/steps/plates_pads_levels_and_slanted_steps.step | A3 1:1 → A4 1:2 | [before](../target/sheet-policy/final/before/renders/4d504aa58d58-plates_pads_levels_and_slanted_steps.png) · [after](../target/sheet-policy/final/after/renders/4d504aa58d58-plates_pads_levels_and_slanted_steps.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/recognition/steps/rectangular_through_step.step | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/95ae8bff5625-rectangular_through_step.png) · [after](../target/sheet-policy/final/after/renders/95ae8bff5625-rectangular_through_step.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/rule_pins/framed-moved.step.gz | A3 1:1 → A4 1:2 | [before](../target/sheet-policy/final/before/renders/90501a6deb18-framed-moved.png) · [after](../target/sheet-policy/final/after/renders/90501a6deb18-framed-moved.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/rule_pins/framed.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/02444d743414-framed.png) · [after](../target/sheet-policy/final/after/renders/02444d743414-framed.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/rule_pins/opposed.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/3fbe0c46722f-opposed.png) · [after](../target/sheet-policy/final/after/renders/3fbe0c46722f-opposed.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/rule_pins/tuner_jig_blind_obround_pockets.step.gz | A4 1:2 → A4 1:2 | [before](../target/sheet-policy/final/before/renders/75655195470b-tuner_jig_blind_obround_pockets.png) · [after](../target/sheet-policy/final/after/renders/75655195470b-tuner_jig_blind_obround_pockets.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/samples/specify-sample-spindle.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/bf3df8a4ece1-specify-sample-spindle.png) · [after](../target/sheet-policy/final/after/renders/bf3df8a4ece1-specify-sample-spindle.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/samples/turned_spindle_fits.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/415fd38127ee-turned_spindle_fits.png) · [after](../target/sheet-policy/final/after/renders/415fd38127ee-turned_spindle_fits.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/turned_knurl/spindle_diamond_knurl.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/cf642e0da264-spindle_diamond_knurl.png) · [after](../target/sheet-policy/final/after/renders/cf642e0da264-spindle_diamond_knurl.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/turned_knurl/spindle_external_thread.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/ed45a7b6f69a-spindle_external_thread.png) · [after](../target/sheet-policy/final/after/renders/ed45a7b6f69a-spindle_external_thread.png) | Clear main geometry and separated dimensions at the final paper/scale. |
| fixtures/turned_knurl/spindle_turned_knurl.step.gz | A3 2:1 → A4 1:1 | [before](../target/sheet-policy/final/before/renders/7810703748d5-spindle_turned_knurl.png) · [after](../target/sheet-policy/final/after/renders/7810703748d5-spindle_turned_knurl.png) | Clear main geometry and separated dimensions at the final paper/scale. |

Inspection ledger: [manual-review.json](../target/sheet-policy/final/manual-review.json). Physical-size comparison boards: `target/sheet-policy/final/boards/*.png`.

## Completed checks

All commands below ran in the stated environment, sequentially within this check sequence. The final affected library, rendering, Session and latency binaries completed before the final full-suite run. Earlier core, dimensions, audits and issue_drafting passes are archived under `target/sheet-policy/proof-3b0af09/`; the final full run below rechecks all eight integration binaries on the corrected source. Native survey/render processes finished before the full suite, avoiding competition with its unchanged latency gates.

| Check | Result | Tests passed | Seconds |
| --- | --- | ---: | ---: |
| `cargo fmt --check` | PASS | — | 2.13 |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS | — | 11.29 |
| `cargo nextest run --release --lib --test-threads 8 --no-fail-fast` | PASS | 284 | 300.45 |
| `cargo nextest run --release --test rendering --test-threads 8 --no-fail-fast` | PASS | 473 | 1137.68 |
| `cargo nextest run --release --test session --test-threads 8 --no-fail-fast --success-output immediate` | PASS | 8 | 39.76 |
| `cargo test --release --test multi_sheet_latency -- --nocapture` | PASS | 1 | 1.99 |
| `python3 -I tools/test_ci_acceptance.py Acceptance.test_survey_requires_all_77_parts_and_zero_repeated_spans Acceptance.test_survey_shards_partition_the_77_parts Acceptance.test_mcp_requires_seven_unique_successful_tasks Acceptance.test_mcp_requires_readable_judge_documents_and_rendered_artifacts Acceptance.test_missing_fixtures_fail_before_starting_mcp` | PASS | 5 | 0.502 |
| `cargo nextest run --release --workspace --profile ci --test-threads 8 --no-fail-fast` | PASS | 1511 | 1262.95 |
| `cargo test --release --test issue_drafting` | PASS | 37 | 86.48 |
| `cargo test --release --test issue_trust` | PASS | 13 | 4.9 |
| `cargo test --workspace --release --doc` | PASS | 0 | 0.75 |
| `python3 -I tools/ci_acceptance.py --job mcp --binary target/sheet-policy/proof-policy-scope/after/draftwright --out target/sheet-policy/proof-policy-scope/acceptance-final` | PASS | — | 13.31 |
| `python3 -I tools/quality.py target/sheet-policy/proof-policy-scope/after/draftwright target/sheet-policy/proof-policy-scope/quality-final /Users/paul/repos/quiddity/tests/corpus/cadgenbench_inputs/cgb203.step /Users/paul/repos/quiddity/tests/corpus/cadgenbench_inputs/cgb207.step /Users/paul/repos/quiddity/tests/corpus/cadgenbench_inputs/cgb241.step /Users/paul/repos/quiddity/tests/corpus/cadgenbench_inputs/cgb245.step /Users/paul/repos/quiddity/tests/corpus/gramel/GRM-03_thumbwheel_drive_screw.step /Users/paul/repos/quiddity/tests/corpus/gramel/GRM-05_depth_lock_bolt.step /Users/paul/repos/quiddity/tests/corpus/gramel/string_post.step /Users/paul/repos/quiddity/tests/corpus/nist/nist_ctc_01_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ctc_02_asme1_rc.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ctc_03_asme1_rc.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ctc_04_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ctc_05_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ftc_06_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ftc_07_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ftc_08_asme1_rc.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ftc_09_asme1_rd.stp /Users/paul/repos/quiddity/tests/corpus/nist/nist_ftc_10_asme1_rb.stp /Users/paul/repos/quiddity/tests/corpus/ttt_inputs/sm-hanger.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/1000.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10000.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10007.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10020.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10033.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10038.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/1004.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10046.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10047.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10049.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10060.step /Users/paul/repos/quiddity/tests/corpus/mfcadpp/10063.step` | PASS | — | 420.86 |
| `python3 -I tools/check_leader_routes.py target/sheet-policy/proof-policy-scope/quality-final --report target/sheet-policy/proof-policy-scope/leader-routes-final.json` | PASS | — | 6.83 |
| `python3 -I tools/check_quality_table.py docs/status-2026-10-10.md target/sheet-policy/proof-policy-scope/quality-final/baseline.json` | PASS | — | 0.02 |

The unchanged 150 ms p90 latency gates pass with one planner call per edit and real cumulative allocation steps:

```text
R5_LATENCY samples=10 p50_ms=9.979 p90_ms=10.073 max_ms=29.107 max_delta_bytes=6807 sheet_steps=[3, 3, 3, 3, 3, 3, 3, 3, 3, 3] plans_per_edit=1
R5_MCP_LATENCY samples=10 p50_ms=41.031 p90_ms=45.960 max_ms=46.265 max_wire_bytes=1132 plans_per_edit=1
```

NIST placement-edit p90: 35.242 ms with the unchanged 150 ms gate. The same tests verify preview equality, exact Plan delta replay, fresh SVG/DXF rendering, annotation evidence, undo and redo.

The full suite uses the existing CI nextest profile, then runs its two excluded shared-analysis binaries under their normal harnesses, then doctests. This executes all eight grouped integration binaries; nothing is silently skipped. Full logs and exact test counts are retained in the receipt/log paths below.

| Evidence | Path |
| --- | --- |
| Affected checks | [affected-check-receipts.json](../target/sheet-policy/final/affected-check-receipts.json) |
| Full suite (one final run) | [full-suite-receipts.json](../target/sheet-policy/final/full-suite-receipts.json) |
| MCP and quality checks | [check-receipts.json](../target/sheet-policy/final/check-receipts.json) |
| Paired information/readability audit | [audit.json](../target/sheet-policy/final/report/audit.json) |
| Independent no-automatic-detail audit | [automatic-policy-audit.json](../target/sheet-policy/final/automatic-policy-audit.json) |
| Leader routes | [leader-routes-final.json](../target/sheet-policy/final/leader-routes-final.json) |
| Quality output | `target/sheet-policy/final/quality-final/` |

No quality-pins, repeated-spans, render-tail or plan-time acceptance job was run. No threshold, numeric assertion or quality baseline was relaxed. The baseline 227-part content manifest and 65-part instruction cohort remain unchanged. Commits are local; nothing was pushed.
