# Oriented chamfers after native-recess main integration

Merged `origin/main` at `45a65043` (#85 native section recesses and #86 release CI)
into `cx/oriented-chamfers`, starting at accepted `410ccfca`. The merge is
`5c6910ba`. Both geometry dependencies remain pinned to
`quiddity-rust v0.1.0-alpha.5`. The plan-time budget, quality baseline and latest
completeness reference table are byte-for-byte main's versions.

## Resolution and ownership

The recognition/3 reader retains native recess occurrences, patterns, refusals,
level/riser evidence and oriented bevels. Both label helpers, physical visible
attachment checks, notch exterior-void proofs, recess corridor reservations,
notch arc reservations, established routing and authored-item ownership survive.
The dimension parity ledger retains both branches' complete entries. MCP schemas
were regenerated with `python3 tools/generate_mcp_schemas.py`: 14 tools,
34,477 compact advertised bytes, with no generated changes.

Main's optimized label scoring assumes a candidate's text is on its measured
span. The notch path also permits retaining an existing detached station. Their
initial combination incorrectly scored that unchanged detached text as repaired,
stopped searching, and caused CTC-01 to shrink to 1:10. The merged scorer uses
the actual span tier for bounded-notch drawings, retaining the prepared contact
cache and every clearance guard. The existing slot-width regression passes;
CTC-01 retains 1:5 and its four-row radius-centre table. The duplicate preparation
of the same interior proofs was also removed.

Notch support evidence remains independent of section-recess recognition. Native
triangular/general recesses outside the supported recess grammars remain explicit
unsupported requirements; they acquire no guessed pocket dimensions or duplicate
notch credit. Supported native pocket callouts remain intact. The 10020 notch
depth has one physical owner, its existing bevel first leg; hiding that carrier
restores the independent depth. Source PMI still owns matching CTC-01 angles
across normal-section heights. Every printed `(feature, parameter)` claim is
unique in all five drawings. The final inventory audit also requires every main
dimension claim/value and every main radius-centre occurrence to survive.

The 10020 regression now requires main's A3 2:1 drawing and checks all twelve
native pocket size/depth/location claims exactly once. Its former A4 expectation
predated native recesses; returning to that drawing by discarding the new pocket
information would lose main's behavior. The CTC-01 regression additionally fixes
A3 1:5 and all four R25 table rows. No assertion, checker, threshold or baseline
was removed or relaxed.

## Five-part drawing review

Fresh 150 dpi PNGs, SVGs, plans and analyses for main and the merged release
binary are under `target/oriented-main-merge/`. Main's reference executable has
SHA-256 `f93071df191b4c973b4df04bc7cb2014a05e71ec63bf3a7237596e32b11f3762`,
matching #85's recorded final binary. #86 changes CI routing only. All five PNGs
were visually inspected, including close views of CTC-04's bevel leader and
10190's angle. `drawing-audit.json` records preserved claims, additions, changed
labels, completeness and PNG hashes.

| Part | Merged drawing relative to main | Sheet / scale | Claimed faces, main → merged |
| --- | --- | --- | --- |
| CTC-01 | Two readable 60° angles, one `2× 50` depth, X=290/510 and Y=100 stations; `2× C5` becomes `6× C5`. Independent C50, both slot-width labels, bore statements and four-row R25 table remain. | A3 / 1:5 | 107 → 115 / 139 |
| CTC-04 | `8× C5` becomes `12× C5` on its existing side-view carrier; tables, section and all main measurements remain. | A1 / 1:5 | 240 → 244 / 518 |
| 1007 | Readable C0.1 leader added beside the hole; existing radius/depth and hole callouts retained. | A4 / 1:1 | 17 → 18 / 26 |
| 10020 | Readable 21.2° angle and 20.0 station added. Existing `10.6 × 10.6°` owns the notch depth once. Both native pocket callouts, their locations and the hex-pocket depth remain. | A3 / 2:1 | 44 → 45 / 57 |
| 10190 | Readable 75.0° angle, 2 mm depth and 20.4 station added in side view. Native pocket size/depth and both locations remain. | A1 / 10:1 | 23 → 23 / 39 |

All five retain main's exact sheet size and scale. No main dimension claim/value
or located radius occurrence is lost; no printed claim repeats. Completeness
uses face unions, so the extra notch statements on already claimed 10190 faces
do not inflate its claimed-face count.

## Local verification

Commands run sequentially with `CARGO_BUILD_JOBS=12`, `RUST_TEST_THREADS=12`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Detailed commands, exit status, elapsed times,
source commit and executable hash are recorded under
`target/oriented-main-merge/checks/`.

The first nextest run exposed three file-deletion races in the existing PMI
scratch helper: separate test processes each used counter zero and the same STEP
path. Commit `6100349a` isolates scratch paths by process and call counter in the
four PMI helpers with that pattern. Assertions and fixture bytes are unchanged.
The initial report is retained under `initial-checks/`. Its fourth failure was
the unchanged CTC-01 tolerance regression: canonical restoration could reinsert
an untouched pitch line through the grown slot-width label. The notch restoration
guard now protects linear peers as well as callouts, excludes self-comparison,
and retains the already solved geometry when restoration would add a text
contact. The exact problem-list equality assertion is retained. The complete
suite is rerun after these fixes.

The next complete run passed 1,307 tests and failed only the native NIST
session wall-clock gate while the corpus-wide PMI tests were running in other
nextest processes (p90 362.781 ms against the unchanged 150 ms bound). The same
source passes that test alone at p90 43.866 ms. That report is retained under
`contention-checks/`; the isolated log is in `followup-checks/isolated-session.log`.

The final full nextest invocation uses an artifact-local configuration with
`threads-required = 12` for `binary(=session) | binary(=multi_sheet_latency)`.
Every test remains selected; latency tests reserve all 12 slots so their
wall-clock measurements do not compete with corpus-wide work. Other tests use
12-way execution. The repository's nextest configuration, every timing bound and
all assertions remain unchanged, and `DRAFTWRIGHT_LATENCY_GATE` is unset.

| Check | Final result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo nextest run --release --workspace --test-threads 12 --no-fail-fast --config-file target/oriented-main-merge/nextest-local.toml` | PASS: 1,308 tests across 122 binaries, 0 failed, 0 skipped; 727.829 seconds of test execution |
| Locked release rebuild | PASS |
| Fresh five-part render and inventory audit | PASS: every main claim/value and radius occurrence retained, no duplicate claims; all five PNGs byte-identical to the visually inspected drawings |
| Fresh `python3 -I tools/quality.py` on `tools/quality_parts.txt` | PASS: 30 parts, all 300 CI-baseline cells no worse, 8 improved |
| `python3 -I tools/check_quality_table.py docs/status-2026-10-10.md <baseline.json>` | PASS: all 450 cells no worse, including completeness, text-to-span and leader/extension columns |

The final native session and multi-sheet timing tests pass within this complete
suite, with all 12 nextest slots reserved for each. No latency gate was disabled.
The validation source is `bbc44ef6` (following merge `5c6910ba` and scratch fix
`6100349a`); the final report commit changes documentation only.

Against the unchanged CI baseline, ink findings decrease **17 → 8** and
feature crossings **73 → 65**. Repeated labels remain 7; the other eight gated
metrics remain zero. Against main's refreshed status table, ink decreases
**9 → 8**, claimed faces increase **2,045 → 2,058**, unplanned features remain
98, text-off-span findings remain 4, and leader/extension contacts decrease
31 → 28. No individual part/cell is worse in either comparison.

The final executable SHA-256 is
`772e7c675221ceed02af0112915a007019a82438f6abf0ef62829edb57fcb986`.
`target/oriented-main-merge/final-validation.json` binds the source commit,
commands and successful exit codes, binary hash, all 1,308 tests, both quality
comparisons, inspected PNG identities and unchanged main inputs. The final
quality artifacts are in `target/oriented-main-merge/quality/`.

The survey manifest audit uses the same literal-test-input and sheet-facts
selection as `survey_repeated_spans.py`. All 39 named corpus inputs are already
in `tools/repeated_span_parts.txt`, including 10190. No corpus input was added.
No CI acceptance job (quality-pins, repeated-spans, render-tail, MCP or plan-time)
was run, and no planning budget was re-recorded. No push was performed.
