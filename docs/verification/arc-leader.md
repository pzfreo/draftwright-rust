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
corridor policy. Multiple counted radius exceptions in one recess view retain
a located centre schedule; hole schedules and their tags now include that
radius table among their furniture obstacles. When an omission promotes a counted
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
