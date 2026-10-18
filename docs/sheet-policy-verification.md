# Screen-first sheet policy verification — 2026-10-18

The implementation starts on the detail-view stream and is integrated with main
`0e393afc` (#85, native section recesses). All **before** results in the completed
comparison are native runs of that exact main source; **after** results use the
merged source recorded with the final executable. The portable manifest is kept
unchanged. Historical intermediate experiments remain in `target/sheet-policy/`
and are excluded from the final tables and check ledger.

No push, quality-baseline rewrite, instruction-budget re-recording or latency
override is used. Main's recorded instruction figure is 3,166,420,524,176 with the
unchanged 15% margin (limit 3,641,383,602,802 instructions).

## Policy and the unattended choice

A4 is considered first, including the existing cumulative multi-sheet allocation,
before A3, then A2/A1/A0. Within a paper size the largest clean readable standard
scale wins. Explicit paper, scale, sheet limits, assignments and pins remain intent.
Optional details are considered only when the ordinary drawing cannot deliver the
smaller readable paper. The original material-crossing reference remains in force
when an optional crop is omitted or declined.
An identical pre-existing `pmi_not_lowered` refusal (same source, subjects,
severity and explanation) cannot be repaired with larger paper and therefore
does not block an otherwise clean readable allocation. It remains reported.
Any new refusal, changed source refusal or physical/layout error still blocks it.

The existing 60% fill target is a floor **triggering a larger-scale trial on the same
paper**, including allocation. If no enlarged clean readable allocation fits, retain
the largest clean readable drawing on that paper and record saturation. This is the
unattended choice: buying larger paper for a marginal fill gain contradicts the
maintainer's paper-cost order. The original GRM-03 60–85% footprint guard remains on both the captured
before Plan and the actual automatic result. Its readable A4 5:1 trial is declined
because it introduces a padded text/extension contact; the A3 10:1 drawing remains. No quality or latency threshold is lowered.

## Measured readability

| Measure | Paper-space limit | Reason |
| --- | --- | --- |
| Dimensioned hole/slot opening and native pocket/channel mouth, including scheduled holes | 2.0 mm | The former 0.8 mm opening floor could produce filled-in or hard-to-match openings. |
| Dimensioned turned interval | 2.1 mm | Existing `MIN_STEP_SPAN`, the font's printed capital height; preserves the flange guard and baseline-interval ownership. |
| Prismatic step/height interval | 2.0 mm | Detects compressed primary steps, including intervals determined by adjacent baseline heights; a real 2 mm wall remains acceptable at 1:1. |
| Linear text and arrows | Actual ink width + two 2.7 mm arrowheads | Clean outside arrows/text remain valid; 0.1 mm numerical clearance tolerance accommodates settled coordinates. |
| First exterior dimension line / parallel lines | 10.0 / 7.0 mm | Reuses `dims::FIRST` and `dims::LANE`; interior dimensions keep their existing material-clearance checks. |
| Located ordinates | 1.0 mm span / 4.7 mm separation | Reuses `layout::crowded_locations`, evaluated at the owning view's scale. |
| Thread/repetitive depiction | 0.7 mm | White space between root/crest strokes or repeated modelled ink; four uniformly spaced coaxial strips identify repetition rather than tessellation chords. |
| Cosmetic knurl depiction | Existing 3.0 mm minimum | The renderer already enforces a larger paper-space gap than 0.7 mm. |
| Automatic geometric extent / absolute scale floor | 20.0 mm longest extent / 1:10 | Existing floors remain. |

Measurements belong to their actual carrier, including enlarged details and the
balloon view of a hole schedule. Calibration includes the 2 mm wall in `framed`
at 1:1, the 6 mm slot in `curved_rotated` at 1:2, GRM-03's 0.5 mm interval
at 5:1 (2.5 mm on paper), and 10046's 1.842 mm step: its ordinary views need 2:1; A4 then fails settled dimension lanes.
The connector's two depiction failures at 1:5 disappear at 1:2. Native pocket and channel widths use the same 2 mm acceptance floor; the older preparation floor remains independent. The established automatic and authored detail-scale policies remain unchanged. 10000 now meets the native-mouth floor in its ordinary views and needs no oversized crop. Small edge
chamfers/radii remain unambiguous treatments stated by their callouts; they do
not shrink the whole drawing's acceptance floor. A stronger floor can expose an already compressed
legacy drawing with no clean readable solution on the supported standard sheets.
Such a case retains its established fitted ink and measurements, reports a readability
refusal, and never labels the fallback as a clean readable candidate. The historical
comparator reconstructs the established fitted route reference and refusal geometry;
it never ranks the qualified paper choices. Its 0.8 mm frame-preparation
limit remains separate from the new 2 mm acceptance floor. Aligned authored
dimensions use rotated ink and lane projections. Unresolved small-part
fallbacks do not buy large paper for only a partial improvement.

## Reproduction and evidence

The accepted survey is [the content-hash manifest](sheet-policy-manifest.json):
227 drawable unique STEP contents and the same intentionally unresolved fixture.
Original decompressed STEP basenames are preserved because title text affects layout.
Before and final survey Plans and Analysis are native runs on that exact manifest.
Intermediate sheet-search-only experiments reused immutable, hash-checked Analysis
files; the final survey and all required gates use native analysis normally.

Evidence lives under `target/sheet-policy/`: archived executables, full Plans and
Analysis, candidate histories, rendered SVG/PNG and review reports, intermediate
regression audits, and command logs. `tools/detail_revision_survey.py` supplies the
native survey workflow; `examples/sheet_readability.rs` audits the new measures.
The survey tool's historical legibility fields remain historical; the new
`*-readability.json` reports use the constants above.

All required checks set `CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8`. Corpus checks also set
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1`.
The final check commands run one at a time.

### Re-running the exact native survey

The portable manifest contains names and content hashes, rather than machine-local
paths. From the repository root, hydrate it without changing STEP basenames:

```sh
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
export DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus
export DRAFTWRIGHT_CORPUS_REQUIRED=1
python3 - <<'PYTHON'
import gzip, hashlib, json, os
from pathlib import Path
rows = json.loads(Path('docs/sheet-policy-manifest.json').read_text())
out = Path('target/sheet-policy-recheck')
for digest, row in rows.items():
    prefix, relative = row['name'].split('/', 1)
    root = Path(os.environ['DRAFTWRIGHT_CORPUS']) if prefix == 'corpus' else Path('tests/fixtures')
    source = root / relative
    data = gzip.decompress(source.read_bytes()) if source.suffix == '.gz' else source.read_bytes()
    assert hashlib.sha256(data).hexdigest() == digest, source
    name = source.name[:-3] if source.suffix == '.gz' else source.name
    target = out / 'inputs' / digest / name
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    row['path'] = str(target)
(out / 'manifest.json').write_text(json.dumps(rows, indent=2))
PYTHON
cargo build --release --bin draftwright --example sheet_readability
python3 -I tools/detail_revision_survey.py --binary target/release/draftwright --corpus "$DRAFTWRIGHT_CORPUS" --manifest target/sheet-policy-recheck/manifest.json --out target/sheet-policy-recheck/native --audit-legibility
target/release/examples/sheet_readability target/sheet-policy-recheck/native > target/sheet-policy-recheck/readability.json
```

`sheet_readability` is the current six-component audit; the survey tool's older
`geometric_floor` field intentionally retains its historical definition. The
before executable is built from `0e393afc`, with the same manifest and corpus.
For the completed run, `target/sheet-policy/checks85/results.json` records the exact
argv, environment, executable hash, elapsed time and exit code of every final gate.

## Survey result

The before executable is main at #85 (`0e393afcda459277deecf78dff5fce1d3758e599`).
The final validated source and executable hash are recorded with the check results below.
The complete [227-row comparison](sheet-policy-results.csv) gives paper, scale,
sheet count, detail count and the measured A4 failure for every larger-paper result.
Candidate histories and exact Plans remain in `target/sheet-policy/after/survey/`.

| Measure | Before | After |
| --- | ---: | ---: |
| A4 drawings | 99 | 141 |
| A3 drawings | 102 | 49 |
| A2 drawings | 8 | 16 |
| A1 drawings | 12 | 14 |
| A0 drawings | 6 | 7 |
| Total sheets | 227 | 230 |
| Multi-sheet drawings | 0 | 3 |
| Details / drawings with details | 3 / 3 | 10 / 10 |
| Drawings failing a new readability test | 69 | 31 |
| Total measured readability failures | 376 | 300 |

79 drawings change paper, scale, sheet count or detail count. No part gets worse
in any of the six readability components (steps, existing openings, native mouths,
text/arrows, depiction or lanes). No required measurement, placed claim or PMI item is lost; nominal
measurement values are unchanged. No drawing-defect count or exact finding
identity increases, including multiplicity and severity. 30 cases acquire
an explicit readability-refusal diagnostic exposing an existing failure.
All 31 remaining failing drawings report recovery exhaustion. No unresolved
readability fallback moves to larger paper for only partial improvement.

The exact-subject audit is stricter than a count comparison: no new drawing
finding appears by `(code, subjects, refs)`, including multiplicity. The fitted
reference preserves crossing identities independently of its count. A readable
trial cannot trade an old crossing for a newly obstructed dimension or feature.
The 18 archived assembly-pin route identities are pinned in
`tests/fixtures/sheet_policy_routes.json`. An otherwise readable enlargement
which changes those pairs is rejected; the complete fitted A4 drawing remains
with its eight undersized openings and an explicit recovery warning.

The final larger-paper choices include 1004 at A3 2:1: its A4 drawing fails one
10/7 mm lane test. The tuner jig uses A2 1:1 to clear two compressed steps and
one text/arrow failure; A4 allocation cannot retain clean routes at that scale.
Boss_circle uses A3 1:1, clearing one compressed boss interval and all six
undersized holes. Their before/after PNGs were inspected directly, and every
measurement remains. These choices buy measurable readability rather than fill.

## Full-suite regression repairs

Automatic trials retain the ordinary named-pair frame for authored witnesses.
Angular annotation ink participates in corridor reservation. An infeasible edited
frame gets a bounded trial at the same paper and scale through the existing
recovered named-pair layout; clean geometry, measurement retention and finding
limits still govern acceptance. The unchanged authored tests check the angle's
sector, border clearance and scale, and every diagonal/extension/arrow crossing.
Located dimensions that become inverted at a newly readable scale can exchange
occupied perpendicular rows while retaining their physical witness origins.
Component readability, exact findings and leader associations must remain no worse;
the unchanged rules catalogue checks the actual drawn order. Existing slot
size/location chains retain their shared row through the original semantic and
geometric chain predicates; the original no-table FTC-08/FTC-09 assertion remains
unchanged and passes.
A physically fitted input to public `recover` is retained when it supplies a
stronger readability reference than the historical fallback. Readability refusal
text preserves the original GD&T reservation rationale, one fill measurement,
and an explicit below-floor statement when true. Smaller-paper allocation keeps
warnings about an explicitly requested subfloor scale. No assertion or baseline
was relaxed for these repairs.

## Changed test expectations

| Test / part | Previous expectation | New expectation and reason |
| --- | --- | --- |
| Recognition correction split-grid mounting plate | A3 2:1 | A3 2:1 remains: A4 at 1:1 lengthens a leader to 31.7 mm versus the established 10 mm; the retained route guard allows only 2 mm growth. A small plate stays larger only because that association test fails. |
| GRM-03 default / recovered issue test | Exact main #85 A3 10:1; the pre-#85 branch issue test pinned A4 5:1 | Main result unchanged: A4 5:1 introduces a 1 mm padded text/extension contact. The original 60–85% fill guard tests both before and current Plans. |
| Bare pins on 10046 and 11512 | A pre-#85 branch expectation of A4 1:1 plus one detail for 10046 | Exact main #85 has 10046 at A3 2:1 without a detail; retain that readable result because 1:1 fails its 1.842 mm step floor and A4 2:1 fails lanes. The full bare-pin/ink equality test now also covers 11512 at A4 1:1 with its real detail, preserving positive crop coverage. |
| GD&T string-post section correction | A3 5:1, one sheet | A4 5:1, two sheets; the existing allocation preserves all original control frames and measurements, with zero readability failures and the same physical-fit and route limits. |
| Plain assembly_plate_pin | Automatic A4 1:5 within the former A3 cap | A4 1:5 retained as a reported refusal: all eight openings fail 2 mm, and larger readable trials change established crossing pairs. All 18 routes, frames and measurements remain. |
| hw_assembly_plate_pin_intent | A3 cap | Still A3; its installed rows fit at a readable scale, with no new physical findings. |
| threaded_connector_109 | A3 1:5 | A2 1:2; A3's modelled thread/repetition gap is below 0.7 mm. |
| Constrained CTC03 A2 1:5 | No recovery warning | Same physical bounds/ink thresholds; a measured readability failure must now be reported as recovery exhaustion. |
| Projection cache scale-edit unit | Always edit to 1:1 | Choose a different scale dynamically, because the new default can already be 1:1. The cache invalidation assertion remains. |
| Spindle fit label | A3 2:1 | A4 1:1, zero readability failures; actual glyph width, 2 mm clear pad and the original 25 mm leader limit remain. |
| Default 10063 / 10000 | A4 1:1 / A4 1:1 | 10063 retains A4 1:1 with three measured step refusals because enlargement fails the original leader-length limit. 10000 uses A1 5:1 without an automatic detail: its 1.286 mm native pocket mouth must draw at least 2 mm, and the smaller-paper trials fail settled layout or leader association. Its short circular-bore leader remains within the original +2 mm allowance. A4 compressed-step/native-mouth trials and allocation refusals are recorded. The original optional-capacity geometry equality test is unchanged. |
| FTC-10 centreline label encounter | Automatic A1 1:1 | Explicit historical A1 1:1 keeps the exact axis/label encounter; all original trimming distances remain. Automatic recovery and the section carrier are tested separately. |
| FTC-10 section hatch tests | Render sheet 1 | Render the actual section carrier with `render_sheets`; all original material and hidden-line checks remain. |
| No-NTS isometric case | Automatic isometric equal to sheet scale | Explicit A3 2:1 and isometric factor 1.0 exercise the same no-caption condition; the separate strict 5% boundary tests remain. |
| Forced assembly-pin material crossing | Automatic former A4 1:5 | Explicit A4 1:5 preserves the original ±15 mm obstruction, depth and planner/lint equality checks. |
| STC-09 GD&T refusal | Below 1:2 | Below 1:1; the stronger 2 mm opening floor, with the same paper, carrier and bounds checks. |
| FTC-07 / cgb245 exhausted search | Informational old-policy fallback | Same established A2 1:2 drawing, now a warning with measured readability and candidate evidence. All retained outcomes and physical bounds are checked. |
| R5 paper order | Larger paper before another sheet | A4 5:1 on two sheets before larger paper; table retention, zero readability failures and all physical fit checks remain. |
| Located-hole corridor test | Location always a linear witness | Assert its retained table row when the default uses a schedule, then disable tables to run the original exact linear-corridor checks. |
| Service problem ledger | Three problems including fill replan | Two original source-completeness problems; no obsolete fill-driven paper replan. All remaining messages and fixes are checked exactly. |
| Typography recovery exercise | Unchanged automatic seed | Explicit former A3 2:1 seed for GRM-04 ensures recovery actually runs; all original fixed-font checks remain. |
| Explicitly framed A2 assembly seed passed to automatic recovery | Same blanket small-paper cap as the default estimate | A2 1:1 when A4/A3 cannot preserve this seed's different established routes; zero readability failures, no new exact route findings and all placed GD&T frames retained. The ordinary default still retains A4 1:5 with its reported floor failure. |
| Native 1013 | Pre-merge branch A3 1:1 | A3 2:1, retaining main #85’s native spans at the readable scale. |
| Holdout 419 | Pre-merge branch A3 2:1 | A4 1:1; the explicit historical reference now has zero material crossings. Witness-ray and dimension-span checks remain. |

10224 and 10255 remain one A3 sheet at 2:1; their original placement, overlap and
bounds checks remain, with zero readability failures additionally required.
New tests separately exercise paper order, same-paper allocation, fill enlargement,
each readability limit (including aligned text and inward-arrow collisions),
explicit Spec controls, preserved pattern coordinates and annotation-quality
regressions. No quality baseline or instruction/latency budget changes.




The native integration also pins main #85’s exact warning/error identities,
references and multiplicity for 10119, 10127 and whistle_frame in
`tests/fixtures/sheet_policy_native_routes.json`, together with placed measurement
claims and all six readability counts (native mouths are separate). The fitted pre-recess reference is chosen
independently of A4 ranking. A raw seed violating native corridor/material budgets
cannot veto a better fitted candidate. Both sides’ recess dimensions and routing
protections remain active.
Installed native hole schedules use the same historical entry point and guarded
return as main before any inline reconstruction. The qualified search still runs
all applicable readability tests; a retained undersized opening is reported as a
refusal. This removes accidental reference reconstruction without relaxing a
drawing, coverage, corridor or material gate.
The quality guard also includes the existing same-view, same-direction linear
label count. FTC-06's rejected reconstructed table added seven inline dimensions
and two repeated labels; the complete native reference retains its rows and
measurements with zero repeats. No quality baseline was changed.

<!-- GENERATED RESULTS -->
