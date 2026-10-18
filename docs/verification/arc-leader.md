# Trimmed radius leaders and perpendicular location witnesses

The 10000 side-view radius callout used a complete construction circle for a
quarter-cylinder edge. Its preferred label direction selected the missing upper
half of the circle. Analysis now records the actual circular boundary edges of
the feature's faces, including their exact angular ranges. Callout sites retain
those ranges through projection, radial selection, authored placement, interior
placement and the clear-route sweep. Short concave profiles also offer proven
interior arc stations, following the point-site contract already used by blends.
Full-circle holes retain their existing geometry and selection rule.

Exact edges are matched to their carrying cylinders rather than rounded feature
sizes. NIST CTC-03 slots 1/2 and FTC-09 slots 1/2/3 exposed this distinction
(R3.17 can mean an exact R3.175 cylinder). Both NIST parts now retain their
physical boundary arcs, with a regression checking the unrounded source radii
and the finished callouts. Unverifiable radius targets also count as quality
findings, so missing evidence cannot pass the quality gate silently.

The callout audit found the same full-circle construction in lone obround end
radii and slot-pattern end radii. Both now use their actual trimmed outward arcs (semicircles on uncut obrounds). Native
circular-seat callouts, native pocket corner-radius callouts, round-bottom blind
slots, authored radius/diameter callouts and blend callouts already use physical
surface/wall points; these do not
have this full-circle fault.

The recess-context placement search previously ranked diagonal witnesses together
with perpendicular candidates. A diagonal could win by reducing a crossing count
even though a perpendicular lane remained available. The search now exhausts
ordinary lanes on both sides, through the paper's available outer lanes, and
finite-start-gap alternatives before trying an oblique fallback. Longer finite
start gaps keep the same projected measurement stations while omitting witness
ink through the body. Further finite-gap alternatives start beyond settled
callout corridors using the same padding as the leader router. This preserves
clear established leaders instead of displacing them after the witnesses settle.
Captured recognition already contains its recess inventory. The new long-gap
and alternate-support repairs therefore apply only to native geometry, keeping
captured step-height rungs on their recorded supports and preserving their
neighbours' witness gaps through omission edits. Native repairs may start from
the nearer face or another proven support edge.
Oblique fallbacks use parallel witnesses at 30 degrees to
the perpendicular, may not add either feature or model-edge crossings relative
to their perpendicular base, and may not traverse material or cross another
dimension's line, witness, arrowhead or label. Location retries also stay outside
settled size lanes before ranking the datum ladder. Semantic PMI retains its established
corridor policy. Obstructed radius exceptions retain the established centre
schedule policy; clear counted radius leaders stay on the recovered view. Hole schedules
and their tags include an existing radius table among their furniture obstacles. When an omission promotes a counted
linear alias, it inherits its owner’s settled lane while retaining its own
measurement stations. This keeps edits local after an outward witness repair.

`radius_leader_target_mismatch` now covers circular blind steps and obround end
radii against the Analysis's exact boundary spans. `oblique_witness_crossing`
checks the finished drawing's witness ink against visible boundary curves and
other dimensions. Mismatch, unverifiable-target and oblique findings participate in `quality.py`'s ink metric. Lint's
independence guard also audits the shared arc geometry module; it still prohibits
reading plans or specifications.

The corpus regression checks the 10000 radius tip against samples of its exact
edge parameter interval and verifies that its 17.5 hole coordinate has vertical
witnesses. Deliberately moving the tip to the missing sector and skewing the
witnesses must produce the respective lint findings. Unit tests cover interval
clamping, wrapping, reflected projection and preservation of full-circle targets.

Generated artifacts are in `artifacts/arc-leader/` (not committed): before/after
PNG and SVG drawings, saved quality analyses/plans/reviews, witness-count JSON,
leader-route reports, and verification logs. `count-oblique.py` counts placed
linear dimensions with a nonzero dot product between their dimension axis and
either witness, with a relative numerical tolerance of 1e-6. Counts refer to the
same 30 parts listed in `tools/quality_parts.txt`. Before: **7 parts / 16
dimensions**. After: **0 parts / 0 dimensions**. Changed parts are 10000, 10020,
10046, nist_ctc_02, nist_ftc_06, nist_ftc_08 and nist_ftc_09 (the shortened
NIST names identify the corresponding entries in that inventory).

## Visual inspection

All images below were rendered and inspected. The additional circular-step parts
come from the quality inventory (`10063`) and analysed survey candidates
(`1007`, `363`); `circular-step-survey.json` retains their native feature IDs and
exact boundary spans.

| Part | Inspection | Before PNG | After PNG |
| --- | --- | --- | --- |
| mfcadpp/10000 | R8.5 touches the quarter arc, with room around its label; 17.5 has vertical witnesses from the nearer bottom face, below 3.4 and inside 19.7; 7.8 uses the clear upper lane; A4 at 1:1 | [before](../../artifacts/arc-leader/before/10000.png) | [after](../../artifacts/arc-leader/after/10000.png) |
| mfcadpp/10020 | Height witnesses are perpendicular; the drafting-angle annotation remains an angle | [before](../../artifacts/arc-leader/before/10020.png) | [after](../../artifacts/arc-leader/after/10020.png) |
| mfcadpp/10046 | The 32.4 coordinate has perpendicular witnesses below its view | [before](../../artifacts/arc-leader/before/10046.png) | [after](../../artifacts/arc-leader/after/10046.png) |
| mfcadpp/10063 | R2.8 touches its quarter-arc endpoint; the label clears the neighbouring pocket labels | [before](../../artifacts/arc-leader/before/10063.png) | [after](../../artifacts/arc-leader/after/10063.png) |
| mfcadpp/1007 | R2.7 touches its quarter arc; the label sits above the chamfer and bore callouts | [before](../../artifacts/arc-leader/before/1007.png) | [after](../../artifacts/arc-leader/after/1007.png) |
| mfcadpp_holdout/363 | R8.6 touches the arc endpoint at the outer profile; its label stays clear below the front view | [before](../../artifacts/arc-leader/before/363.png) | [after](../../artifacts/arc-leader/after/363.png) |

Further route verification preserves established leaders through the existing
no-increase gates for physical crossings, labels, witnesses and arrowheads,
before shortening them after witness reflow. It also retries hole leaders that
cross a newly recognised feature and requires a strict reduction in feature
crossings. The existing 10020 test checks its boss route against finely sampled
rendered hidden edges and its arrowhead against the settled witnesses. Fixed-ink
refinement also compares directed arrow/leader contacts, so clearing a witness
cannot introduce a close parallel peer at the tip. Existing captured-bridge
leader and edit-locality checks cover both cases; no recorded exception was added.

## Verification

Final checks use `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Checks run sequentially. The target directory is
`../target`; the base is `d96f067b` (`origin/main` at the start).

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Affected library / dimensions / rendering / core / audits binaries | PASS: 253 / 321 / 445 / 269 / 155 tests |
| Direct captured leader, placement and omission checks | PASS: 3 tests; unchanged recorded findings |
| Direct size-before-location audit | PASS |
| Final captured support, native boss route, omission locality and placement regressions | PASS: all four unchanged checks; final rendering and drafting binaries also pass |
| Rounded NIST slot-radius regressions | PASS: both parts, five slots; full leader-rules group 8 tests |
| New physical lint findings across all quality reviews | PASS: zero mismatch, unverifiable-target and oblique-crossing findings |
| Final refreshed PNGs after support-scope fix | PASS: all six hashes identical to the inspected images |
| `python3 -m unittest discover -s tools -p 'test_quality.py'` | PASS: 40 tests |
| `tools/quality.py` on `tools/quality_parts.txt` | PASS: all 30 parts; ink 6, line-through-feature 63 |
| `tools/check_leader_routes.py` on that output | PASS: 142 callouts; no extra hidden/section/cut crossings, excess length or new runs; 8 unchanged pre-existing runs |
| `tools/check_quality_table.py` against an exact table exported from `tools/ci-quality-baseline.json` | PASS: 30 parts, 300 cells, no part worse |
| Full workspace suite, once | 1,499 passed, 3 failed, 0 skipped; 1,252.595 s. Failures and unchanged-limit reruns recorded below |
| Drafting binary after support-anchor fix | PASS: 37 tests |
| Multi-sheet latency binary rerun | PASS: 1 test, original 150 ms limit |
| Session binary rerun | PASS: 8 tests, original 150 ms limits |
| Workspace doc-tests | PASS: both workspace crates; 0 doc-tests |

An initial full-suite launch was interrupted before any test completed when the
additional rounded-slot evidence gap was found. The one completed full run then
found a step-height support regression and two latency failures. The rung's
finite start gap had detached its witness from the recorded level support;
restricting the new long-gap and alternate-support repairs to native geometry
fixes that regression while preserving the captured support selection. A first,
broader rung exclusion disturbed the native 10020 boss route and a captured
omission edit; the narrower native-only rule passes all four focused regressions
without changing their assertions. All 37 drafting tests pass afterward.

The full run's unchanged latency gates failed at MCP p90 635.044 ms in
`r5_multi_sheet_latency` and native p90 475.862 ms in `nist_benchmark_deltas`,
both against 150 ms. The separate latency binary passes its one test on rerun,
and the complete session binary passes all eight tests on rerun, with no timing
code or limit changed. These successful reruns are distinct from the failed
full-run result. The full suite was not repeated; the final support-scope fix
passes all 37 drafting, 321 dimension, 445 rendering and 155 audit tests. Workspace doc-tests
also pass. The final 30-part quality output passes both checkers and a direct
comparison of all 300 CI JSON baseline cells. The six refreshed PNGs are
byte-identical to the inspected images (`after/png-final-check.json`).

No assertion, threshold, known-finding fixture or CI quality baseline was weakened.
The CI-only quality-pins, repeated-spans, render-tail, MCP and plan-time acceptance
tools were not run. Logs and full quality data accompany the PNGs in the artifact
directory. The explicit visual choice for 10000 is a perpendicular finite start
gap at the nearer bottom face; its 7.8 size uses the opposite strip to clear the
short 3.4 label.

One slow-header HTTP test encountered a connection reset during the grouped core
run. It passed unchanged in isolation and in the complete 269-test core retry;
no HTTP code, assertion or timing limit changed.


## Reviewer follow-up: preserve paper and radius presentation

The follow-up starts at `22e6d2ec` and compares the exported, recovered plans
with main at `d96f067b`. Before/after artifacts for this follow-up live in
`artifacts/arc-leader/fix/`; `before/` is the rejected branch drawing and `main/`
is the main drawing. The original verification above records the earlier round.

CTC-02 lost its clear `4× R15` and `6× R18` plan-view leaders because the branch
forced multiple counted radius groups in a recess view into a centre schedule,
without requiring an obstructed leader. Complete-view recovery now applies the
original obstruction-based schedule policy. The bare, unrecovered planner keeps
its established estimated centre schedule; its existing row-credit and table
corruption tests remain unchanged. The counted exported leaders retain every
occurrence claim and attach to proven sites on their trimmed corner surfaces.
FTC-08 retains its existing radius-centre table.

FTC-09's A1 failure was not a larger principal view or a changed geometry scale.
Its scheduled layout reserved an isometric region 289 mm high, then restored
the established isometric scale, producing 363.897 mm of projected ink. That
footprint intruded into the 30-hole schedule's placement space. The schedule
transaction failed, restored the coordinate ladder, and its close 38.1/39.37
ordinates caused recovery to choose A0. The schedule installer now retries an
automatic isometric that outgrows its reservation beside the complete schedules,
using the same isometric scale and the existing furniture clearances. The retry
commits only after every schedule row and tag fits. Principal view frames and
witnesses stay fixed, and authored placements and held frames keep their existing
policy. The choice is to move the automatic orientation view and keep A1 at 1:1.

Two new exported-plan regressions check CTC-02's counted leaders and physical
trimmed sites, and FTC-09's A1/1:1 sheet, 30 schedule rows, unchanged isometric
scale, bounded isometric footprint and perpendicular witnesses. No existing
assertion, threshold, known-finding fixture or quality baseline is changed.

All **30/30** exported plans retain main's sheet dimensions, sheet scale and
every individual view scale.
Parts gaining or losing a radius-centre table relative to main: **none**.
CTC-01 and FTC-08 retain their tables; FTC-08's complete radius-table data is
identical to main. The full comparison is saved in
`fix/presentation-comparison.json`.

| Quality part | Main → final sheet | Main → final scale | Radius-centre table |
| --- | --- | --- | --- |
| cadgenbench_inputs/cgb203.step | A4 → A4 | 1:5 → 1:5 | absent in both |
| cadgenbench_inputs/cgb207.step | A3 → A3 | 1:2 → 1:2 | absent in both |
| cadgenbench_inputs/cgb241.step | A3 → A3 | 1:5 → 1:5 | absent in both |
| cadgenbench_inputs/cgb245.step | A2 → A2 | 1:2 → 1:2 | absent in both |
| gramel/GRM-03_thumbwheel_drive_screw.step | A3 → A3 | 10:1 → 10:1 | absent in both |
| gramel/GRM-05_depth_lock_bolt.step | A3 → A3 | 5:1 → 5:1 | absent in both |
| gramel/string_post.step | A4 → A4 | 5:1 → 5:1 | absent in both |
| nist/nist_ctc_01_asme1_rd.stp | A3 → A3 | 1:5 → 1:5 | retained |
| nist/nist_ctc_02_asme1_rc.stp | A0 → A0 | 1:5 → 1:5 | absent in both |
| nist/nist_ctc_03_asme1_rc.stp | A3 → A3 | 1:5 → 1:5 | absent in both |
| nist/nist_ctc_04_asme1_rd.stp | A1 → A1 | 1:5 → 1:5 | absent in both |
| nist/nist_ctc_05_asme1_rd.stp | A2 → A2 | 1:5 → 1:5 | absent in both |
| nist/nist_ftc_06_asme1_rd.stp | A1 → A1 | 1:2 → 1:2 | absent in both |
| nist/nist_ftc_07_asme1_rd.stp | A2 → A2 | 1:2 → 1:2 | absent in both |
| nist/nist_ftc_08_asme1_rc.stp | A1 → A1 | 1:2 → 1:2 | retained |
| nist/nist_ftc_09_asme1_rd.stp | A1 → A1 | 1:1 → 1:1 | absent in both |
| nist/nist_ftc_10_asme1_rb.stp | A1 → A1 | 1:1 → 1:1 | absent in both |
| ttt_inputs/sm-hanger.step | A3 → A3 | 1:2 → 1:2 | absent in both |
| mfcadpp/1000.step | A4 → A4 | 2:1 → 2:1 | absent in both |
| mfcadpp/10000.step | A4 → A4 | 1:1 → 1:1 | absent in both |
| mfcadpp/10007.step | A3 → A3 | 2:1 → 2:1 | absent in both |
| mfcadpp/10020.step | A3 → A3 | 2:1 → 2:1 | absent in both |
| mfcadpp/10033.step | A3 → A3 | 2:1 → 2:1 | absent in both |
| mfcadpp/10038.step | A4 → A4 | 2:1 → 2:1 | absent in both |
| mfcadpp/1004.step | A4 → A4 | 1:1 → 1:1 | absent in both |
| mfcadpp/10046.step | A3 → A3 | 2:1 → 2:1 | absent in both |
| mfcadpp/10047.step | A3 → A3 | 1:1 → 1:1 | absent in both |
| mfcadpp/10049.step | A4 → A4 | 1:1 → 1:1 | absent in both |
| mfcadpp/10060.step | A4 → A4 | 1:1 → 1:1 | absent in both |
| mfcadpp/10063.step | A4 → A4 | 1:1 → 1:1 | absent in both |

The inspected CTC-02 plan detail keeps both counted leaders with their trimmed
corner targets and ordinary perpendicular witnesses. FTC-09 keeps the full front
hole schedule and the perpendicular location ladders on an **841 × 594 mm A1**
sheet at **1:1**. Its orientation projection keeps scale 1.3 and is repacked with
6 mm of breathing room inside the sheet frame, in addition to the existing
furniture clearance. The initial trial touched the frame; visual QA caught that
and the final placement reserves the shared preferred block gutter.

| Part | Main PNG | Rejected branch PNG | Fixed PNG | Fixed detail |
| --- | --- | --- | --- | --- |
| CTC-02 | [main](../../artifacts/arc-leader/fix/main/nist_ctc_02_asme1_rc.png) | [before](../../artifacts/arc-leader/fix/before/nist_ctc_02_asme1_rc.png) | [after](../../artifacts/arc-leader/fix/after/nist_ctc_02_asme1_rc.png) | [plan leaders](../../artifacts/arc-leader/fix/after/ctc02-plan-detail.png) |
| FTC-09 | [main](../../artifacts/arc-leader/fix/main/nist_ftc_09_asme1_rd.png) | [before](../../artifacts/arc-leader/fix/before/nist_ftc_09_asme1_rd.png) | [after](../../artifacts/arc-leader/fix/after/nist_ftc_09_asme1_rd.png) | full sheet above |

The first follow-up dimensions run passed 319 tests and failed the two bare
CTC-02 radius-table expectations. Those tests use `model::plan`, whose estimated
presentation already had the table on main; the exported command-line drawing
uses `model::plan_recovered`. The final fix passes the existing complete-view
policy to radius scheduling, retaining the bare planner's schedule while requiring
an actual obstruction before tabulating recovered counted leaders. Both existing
tests, including all row-credit and corrupted-locator assertions, are unchanged.
The initial failure log remains at `fix/dimensions-initial.log`.

### Follow-up verification

The final source is `8c0bd6a1`, following `2359e935` and `69a6b52f`. Checks use
the same local environment and unchanged limits recorded above. The final
before/main/after PNGs and CTC-02 plan-detail crops have been inspected; final PNG
hashes are saved in `fix/png-sha256.json`.

| Check on final source | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Workspace library tests | PASS: 253 tests; wasm crate has 0 library tests |
| Affected binaries: dimensions / rendering / core / audits / drafting | PASS: 321 / 447 / 269 / 155 / 37 tests |
| `python3 -m unittest discover -s tools -p 'test_quality.py'` | PASS: 40 tests |
| Final `tools/quality.py`, all 30 quality parts | PASS: ink 6, line-through-feature 63, no off-sheet items |
| `tools/check_leader_routes.py` | PASS: 144 callouts, no additional crossings, excess length or new runs; 8 unchanged pre-existing runs |
| `tools/check_quality_table.py`, unchanged CI table | PASS: 30 parts, 300 cells, no part worse |
| Main comparison: sheet dimensions, sheet scale, every view scale | PASS: unchanged on all 30 parts |
| Radius-centre table gains / losses against main | PASS: none; FTC-08 table exactly identical |
| Oblique witnesses across all 30 parts | PASS: 0 parts / 0 dimensions |
| Independent radius-target and oblique findings, fresh reviews of all 30 parts | PASS: zero in both lint and structural results |
| Accepted dimension placements on 10000, 10020, 10046, FTC-06, FTC-08 | PASS: exactly identical to the accepted previous-round plans |
| Full workspace suite, once in this follow-up | PASS: 1,504 passed, 0 failed, 0 skipped; 572.413 s |
| Workspace release doc-tests | PASS: both crates; 0 doc-tests |

The full follow-up command is
`cargo nextest run --workspace --release --no-fail-fast --ignore-default-filter --test-threads 8`.
Its result is kept separately from the earlier round's full run and the first
follow-up dimensions failure. Full data and logs are under `fix/`, including
`presentation-comparison.json`, `new-physical-lints.json`, `oblique.json`,
`leader-routes.json`, `quality-table.log`, and the per-binary test logs.

No existing assertion, limit, known-finding fixture or baseline was weakened.
No remote CI job was started and no commit was pushed.

The one completed follow-up full run passes all **1,504 tests**, including the
unchanged native latency gates. No full-suite rerun or latency retry was needed.
The 10000 radius tip still touches its quarter arc and its 17.5 witnesses remain
vertical; both regressions pass in the focused rendering binary and in the full
run. The accepted 10000, 10020, 10046, FTC-06 and FTC-08 dimension records are
identical to the previous accepted output.
