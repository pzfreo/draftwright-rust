# Whole-part paper policy verification — 2026-10-18

A4 is the default. Paper and the main scale are selected from the principal views,
their complete annotation, and general geometric legibility. The general geometry
floor remains a 20 mm longest extent and an absolute minimum scale of 1:10.
Text/arrow fit, the 10/7 mm dimension lanes, off-sheet ink, principal fit and retained
annotation/feature associations remain whole-drawing constraints.

A dimensioned opening or native pocket/channel mouth below 2 mm on paper, a short
step interval below its existing 2/2.1 mm floor, or a thread/repetition gap below
0.7 mm cannot purchase A2/A1/A0. These are local constraints. After trying larger
clean main views on the whole part's paper, the planner tries automatic enlarged
crops with their own scale, caption, parent circle and letter. Nearby requirements
share a crop when their complete witnesses fit. Original measurements retain their
values and move through the existing detail ownership/placement pipeline.

For adjacent baseline heights, the original two coordinates remain on the parent.
Their exact difference is explicitly shown as a reference dimension in a local
crop around both recorded supports. The baseline interval is qualified only when that particular
interval has a placed dimension in the matching crop and meets the same 2 mm
paper-space threshold. A crop without its interval annotation does not qualify.

If no clean detail fits, the smaller-paper drawing and all measurements remain,
with `layout_recovery_exhausted` reporting the measured refusal. An A4 drawing may
try A3 only when A4 cannot accommodate a clean detail and the A3 detailed result
has zero remaining readability failures and no new annotation/route finding.
Other whole-part paper sizes cannot advance for a local floor. Low fill cannot
purchase paper. Existing cumulative auxiliary-sheet allocation remains available.

The enlarged crop keeps its own paper-space annotation: interior clearance uses
its crop centre, short reference text may move outside its witnesses under the
existing bounded repair, and pins retain the selected carrier. Dense modelled
repetition uses a partial root-and-crest crop over several complete periods; it
does not need a fabricated thread specification or a full component diameter.
Parent marker letters also avoid all previously placed captions and marker letters;
520 is an explicit rendered regression with two letters separated by at least 1 mm.
Clean A4/named documents stop before unnecessary detail trials, and R5 finishes
pending whole-document table allocation when no local floor is failing.

Explicit paper, scale, authored details, assignments and pins remain authoritative.
A local recovery must also retain already placed Spec fit/tolerance text as drawn
dimension ink; nominal schedule coverage cannot silently substitute for that callout.
The crowded H7 test and all its original text/clearance assertions remain unchanged.
Refusal diagnostics retain the local feature scale separately from the general
selection floor; the STC-09 exact-view and diagnostic assertions remain unchanged.
`sheet.automatic_details: false` disables automatic crops, including local
readability recovery; authored details remain active. The setting is described in
`tools/mcp_spec.py`, the full `schemas/spec.json`, and sheet item options. The
advertised tools/list outline stays at 14 tools and 34,477 compact schema bytes.

## Paired survey and evidence

The portable [manifest](sheet-policy-manifest.json) is unchanged: 227 drawable
unique STEP contents and one intentionally unresolved input. Original decompressed
basenames are retained because title text affects layout. Both sides were rerun
natively using `tools/detail_revision_survey.py --audit-legibility` on this exact
manifest. Before is `origin/main`, `0e393afcda459277deecf78dff5fce1d3758e599`.
Its archived executable is bound to that source by
`target/sheet-policy/revision/before/source.json`.

The final production source is `cec28bb4b38ee5b26ef37eb79cbf6e23175a6e1f`; subsequent commits record
only this evidence. After executable SHA-256:
`bf509d0a5672fda592089e3f453568102babe95dab86d2236beda1d49258c172`.
Before executable SHA-256:
`f93071df191b4c973b4df04bc7cb2014a05e71ec63bf3a7237596e32b11f3762`.
MCP acceptance and quality verification use that same frozen after executable.

Full Plans, native Analysis, executable hashes, command logs and PNGs are retained
under `target/sheet-policy/revision/`. `tools/compare_sheet_policy_survey.py` produces
the [227-row CSV](sheet-policy-results.csv) and the [summary with every paper
up-move](sheet-policy-summary.md). The current `sheet_readability` example measures
both sides with identical thresholds, including native mouths separately.

The paired result has 90 changes in paper, scale, sheet count or detail count.
A4 rises from 99 to 164 parts; A3 falls from 102 to 38; A2 remains 8; A1 falls
from 12 to 11; A0 remains 6. There are 230 sheets (227 before) and 38 details
(3 before). The three extra sheets are A4 allocations for 10212,
`hw_string_post_tapped_intent`, and `string_post_tapped`. Their auxiliary
sheets hold ISO/section views; the automatic local details remain on their
parent’s physical sheet, independently asserted across the full survey.

No part moves above A3. Every paper up-move is listed here:

| Part | Before | After | Reason |
| --- | --- | --- | --- |
| mfcadpp/1004 | A4 1:1 | A3 2:1 | The settled A4 candidate adds crossing pairs against the independently fitted route reference. Preserve the whole-drawing association guard and larger clean main views. No local floor or fill target buys this paper. |
| boss_circle | A4 1:2 | A3 1:1 | Permitted A4→A3 exception: A4 enlarged-detail trials fail annotation/feature association or actual drawn bounds. A3 is clean with **zero** remaining readability failures. Larger main views resolve every feature, so no detail is necessary there. |

The complete audit finds no lost nominal measurement values, no lost placed
PMI/GDT/requirement claims and no new warning/error drawing-finding identities.
The plain spool's two former tabulated bore-centre coordinates (65, 65) remain
proved by the placed 130 mm OD, centred end-view cross and drawn common axis;
all other measurement facts remain directly stated. That geometric proof is
checked against the actual native envelope and bore centre, not just a Covered
ledger entry. All 33 drawings with a remaining readability failure have an
explicit `layout_recovery_exhausted` refusal. Fully readable drawings rise
from 158 to 194 under identical measurements on both saved Plan sets.

## MCP acceptance repair

The rejected local run failed before its timeout-evidence test reached the timed
subprocesses. Three corpus inputs newly referenced by Rust tests were absent from
the repeated-span coverage manifest: `mfcadpp/10103.step`, `mfcadpp/1013.step`, and
`mfcadpp_holdout/419.step`. Supplemental coverage now includes those inputs and
requires all 74 coverage cases, including six additional inputs exercised by this
revision's small-part regression test. The instruction-budget cohort remains the same
43 corpus plus 22 AP242 inputs; no instruction budget or baseline was rewritten.
Final verification runs only the MCP acceptance job for this revision; quality-pins, repeated-spans, render-tail and plan-time
acceptance jobs are left to CI as requested.

## Changed expectations

Every change below follows paper priority or improves a previously refused local
feature. No lane, arrow, opening, step, depiction, route-length, crossing-identity,
off-sheet, pin-geometry or source-measurement limit was increased.

| Test/case | Rejected expectation | New expectation | Reason and retained checks |
| --- | --- | --- | --- |
| Native 10000, page escalation and recovery | A1 5:1, no detail | A4 1:1 with enlarged local carriers, zero readability failures | The pocket mouth and adjacent heights are local. Original nominal claims, exact rendered finding identities and the recorded +2 mm leader allowances remain checked. Removing the actual interval annotation must restore its failure. |
| Native 229 | A0 10:1 | A4 1:1, exactly four local interval failures and an explicit refusal | Cropped trials fail the retained crossing-pair guard. Preserve the complete smaller drawing; an attempted detail cannot purchase paper or new findings. |
| 10046 raw-seed refusal | One text/arrow failure; aggregate readability ranking could buy a larger main drawing | A4 1:2 with exactly two local step failures, zero whole-drawing failures and a warning refusal | Replace the two-mm global enlargement with the measured local refusal. Generic refusal tests still preserve every hard-finding and source-outcome guard; all other cases retain the aggregate assertion. |
| cgb245 raw-seed refusal | Ten local step failures and two lane failures | Established legacy-recognition A3 1:5, exactly 12 step and 15 opening failures, zero whole-drawing failures and a warning refusal | Fix the whole drawing’s lanes on its established fitting paper, report remaining local floors, and retain all printed source outcomes and exact bounds. No up-move versus origin/main. |
| 10046 bare pins | A3 2:1, no detail | A4 1:2, no detail, exactly two local failures and a refusal | A readable crop cannot preserve all fitted routes. Bare pins still preserve every dimension and view's exact ink. 11512 remains the positive crop case. |
| 10063 | A4 1:1 with an exhausted step refusal | A4 1:1 with a readable detail | The local reference interval now fits. All readability failures must be zero; no required drop or off-sheet/title-block ink is allowed. |
| GRM-03 | A3 10:1 at 60–85% fill without a detail | A4 2:1 with a local flange detail and zero readability failures | Saturating A4 below the fill guide cannot purchase A3. The captured old drawing's 60–85% measurement guard, all placed dimensions and route/ink guards remain. Native recognition requires the 5:1 flange carrier; the captured legacy recognition has its separately measured refusal below. |
| GRM-03 captured legacy recognition | A3 10:1, zero local failures | A4 2:1, no detail, exactly one local step failure and a warning refusal | Its crop does not preserve the retained routes. Keep the smaller paper and the explicit refusal. All eight original shoulder outcomes, controlling stations, isometric and source nominal checks remain. |
| threaded_connector_109 | A2 1:2 without a detail | A3 1:5 with a root-and-crest detail and zero readability failures | The local depiction gap cannot buy A2. Removing the crop must restore the measured depiction failure. Every original dimension remains placed. |
| Plain spool | A3 1:2 with a hole table and zero local failures | A4 1:5, exactly 26 local failures and an explicit refusal | No complete clean detail solution survives the route guards. The 69 original measurement facts remain: 67 direct statements and two centre coordinates proved by the 130 mm OD, centred bore cross and common axis. The intended spool's A3 1:2 table and existing crossing caps remain checked. |
| FTC-07 and bounded search diagnostics | Trials on larger papers to pursue local floors | Retained A2 1:2 with its measured refusal and trials bounded by whole-part paper | Existing paper/scale, loss, fit and route assertions remain; diagnostic strings now name actual whole-part and bounded-detail trials. |

The independent rule checker now distinguishes ordinary whole-part selection,
same-paper details, the qualified A4→A3 exception and explicit local refusal. It
still replays the original candidate ranking and crossing ratchet for ordinary
choices. New negative checks cover missing interval annotations, absent depiction
crops, underscaled actual size owners and the sheet-level off switch.

## Render inspection

I inspected all 28 PNGs at 150 dpi, including the nine requested cases and five additional changed parts. Every enlarged detail has its own scale/caption and a parent circle/letter. The 14 paired native reviews add no lint, structural or Plan finding identity (apart from the explicitly reported selection/readability diagnostics). All depicted measurements are retained; the complete 227-part audit independently checks measurement values and PMI/GDT claims. None of these 14 after drawings uses A2/A1/A0.

| Part | Before PNG | After PNG | After details / failures | Inspection |
| --- | --- | --- | --- | --- |
| 10000 | [A4 1:1](../target/sheet-policy/revision/before/renders/10000.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/10000.png) | 2 / 0 | 2:1 mouth and exact-interval details; parent datum heights retained. |
| 229 | [A4 1:1](../target/sheet-policy/revision/before/renders/229.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/229.png) | 0 / 4 | Retained A4; four local interval failures reported; crop trial adds crossing pairs. |
| 520 | [A4 1:1](../target/sheet-policy/revision/before/renders/520.png) | [A4 1:2](../target/sheet-policy/revision/after/renders/520.png) | 2 / 0 | Separate 5:1 and 2:1 carriers; parent letters A/B clear after the rendered regression fix. |
| 808 | [A4 1:1](../target/sheet-policy/revision/before/renders/808.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/808.png) | 1 / 0 | Two nearby intervals share one 2:1 detail. |
| 10167 | [A4 1:1](../target/sheet-policy/revision/before/renders/10167.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/10167.png) | 1 / 0 | 2:1 interval detail; all parent coordinates retained. |
| 1004 | [A4 1:1](../target/sheet-policy/revision/before/renders/1004.png) | [A3 2:1](../target/sheet-policy/revision/after/renders/1004.png) | 0 / 0 | Larger main views; A4 crossing-pair veto retained. |
| boss_circle | [A4 1:2](../target/sheet-policy/revision/before/renders/boss_circle.png) | [A3 1:1](../target/sheet-policy/revision/after/renders/boss_circle.png) | 0 / 0 | Qualified A3 exception; larger principals resolve the holes and bottom wall without a detail. |
| tuner_jig_blind_obround_pockets | [A4 1:2](../target/sheet-policy/revision/before/renders/tuner_jig_blind_obround_pockets.png) | [A4 1:2](../target/sheet-policy/revision/after/renders/tuner_jig_blind_obround_pockets.png) | 0 / 2 | Retained A4; two local step failures reported; pocket pattern, locations, depths and stock remain. |
| threaded_connector_109 | [A3 1:5](../target/sheet-policy/revision/before/renders/threaded_connector_109.png) | [A3 1:5](../target/sheet-policy/revision/after/renders/threaded_connector_109.png) | 1 / 0 | 1:2 root-and-crest crop makes modelled repetition legible; no fabricated thread specification. |
| 257 | [A4 1:1](../target/sheet-policy/revision/before/renders/257.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/257.png) | 1 / 0 | 2:1 interval carrier; complete global bracket dimensions retained. |
| 10245 | [A4 1:1](../target/sheet-policy/revision/before/renders/10245.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/10245.png) | 1 / 0 | Two short intervals share one 2:1 carrier; all aperture callouts retained. |
| 10119 | [A3 5:1](../target/sheet-policy/revision/before/renders/10119.png) | [A3 2:1](../target/sheet-policy/revision/after/renders/10119.png) | 1 / 0 | 5:1 crop groups bore and interval; pin test preserves the same placed ink. |
| 10155 | [A3 2:1](../target/sheet-policy/revision/before/renders/10155.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/10155.png) | 0 / 0 | Complete A4 principals; no local detail needed. |
| 10007 | [A3 2:1](../target/sheet-policy/revision/before/renders/10007.png) | [A4 1:1](../target/sheet-policy/revision/after/renders/10007.png) | 1 / 0 | 2:1 interval carrier; repaired reference text fits its measured reservation. |
