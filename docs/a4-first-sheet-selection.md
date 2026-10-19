# A4-first sheet selection — 2026-10-19

The automatic planner now supplies the unchanged baseline, followed by the small-sheet policy documented in [model.md](model.md#sheet-allocation-r5-stage-i). `sheet/preferred_printer` accepts `A4` (default) and `A3`. Explicit size, scale, and sheet-count limits bypass the policy. No detail recovery, readability reference drawings, or candidate-specific layout repair was added.

**Performance remains outside the requested budget.** Exact forced-paper planning is substantially more expensive for several annotation-heavy parts. The policy keeps the required candidate order and existing allocator. See the timing table below.

## Implementation and preservation

The baseline and each distinct forced Spec share the existing per-call preparation scope. Complete results are memoized with the Spec and existing nominal/layout policy inputs. R5 can reuse its one-sheet attempt; the wrapper never runs recursively inside the baseline planner. A baseline already meeting the next preference is reused. Outline dimensions, principal gutters, and summed outline area at the existing readability floor reject impossible formats before planning.

The failing Problem codes are centralized in `src/sheet_policy.rs`. Comparison uses code plus sorted subjects. Baseline findings, including informational ones, are tolerated. The existing warning `layout_recovery_exhausted` captures unreadable scale/step exhaustion; its informational summary does not itself reject paper. Visible view strokes must also clear the frame, reported through the existing off-sheet code. Candidate summaries and the final reason appear in `sheet_candidates` and `sheet.basis`.

Sixteen archived-CLI comparisons on prismatic, 10098, 10146, and GRM-03, with explicit A3, explicit 1:2, max=1, or max=2, produced byte-identical Plans against `origin/main`. The explicit-heavy FTC-06 A4/max=2 comparison was also byte-identical. Artifacts: `/tmp/a4first-explicit-proof-final.json`, `/tmp/a4first-heavy-explicit-proof.json`.

All 14 changed corpus Plans also match independently planned explicit-paper Specs after removing only the wrapper’s restored intent/default flags, basis text, and candidate summary. Every other Plan field compares exactly (`/tmp/a4first-forced-equivalence.json`).

## Every corpus sheet change

Compared default-Spec Plans for all 100 unique corpus inputs (including compressed inputs) with `origin/main` (`ecb9f179`). All changed corpus drawings remain one sheet. Each linked after-PNG was visually inspected: views clear the frame, callouts stay within it, and no new overlaps were observed. Existing baseline findings are described below.

| Corpus part | Before (paper, scale) | After (paper, scale) | Reviewed PNG |
| --- | --- | --- | --- |
| `cadgenbench_inputs/cgb241.step` | A3, 1:5 | A4, 1:10 | [after](/tmp/a4first-changed-renders/after/80fb0fbffea3-cgb241.png) |
| `cadgenbench_inputs/cgb245.step` | A2, 1:2 | A3, 1:5 | [after](/tmp/a4first-changed-renders/after/034d4dec7b69-cgb245.png) |
| `gramel/GRM-03_thumbwheel_drive_screw.step` | A3, 10:1 | A4, 5:1 | [after](/tmp/a4first-changed-renders/after/b4d455810969-GRM-03_thumbwheel_drive_screw.png) |
| `gramel/GRM-05_depth_lock_bolt.step` | A3, 5:1 | A4, 2:1 | [after](/tmp/a4first-changed-renders/after/73c7ee3e136a-GRM-05_depth_lock_bolt.png) |
| `inventory_refusal/14052.step.gz` | A3, 5:1 | A4, 2:1 | [after](/tmp/a4first-changed-renders/after/91456bd801b9-14052.png) |
| `mfcadpp/10020.step` | A3, 2:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/88310a9fa790-10020.png) |
| `mfcadpp/10033.step` | A3, 2:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/d3501c8c991a-10033.png) |
| `mfcadpp/10046.step` | A3, 2:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/fc891f27e0be-10046.png) |
| `mfcadpp/10103.step` | A3, 5:1 | A4, 2:1 | [after](/tmp/a4first-changed-renders/after/d165fab2c375-10103.png) |
| `mfcadpp/10163.step` | A3, 1:1 | A4, 1:2 | [after](/tmp/a4first-changed-renders/after/1985e7ec6b8b-10163.png) |
| `mfcadpp/10247.step` | A3, 2:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/5252edd39905-10247.png) |
| `mfcadpp/10278.step` | A0, 10:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/2e974154e0f2-10278.png) |
| `mfcadpp_holdout/20273.step` | A3, 2:1 | A4, 1:1 | [after](/tmp/a4first-changed-renders/after/c4684e5b6ee2-20273.png) |
| `nist/nist_ftc_07_asme1_rd.stp` | A2, 1:2 | A3, 1:5 | [after](/tmp/a4first-changed-renders/after/a0bc55165678-nist_ftc_07_asme1_rd.png) |

The native survey is archived in `/tmp/a4first-native-before` and `/tmp/a4first-native-final`; each contains the manifest, summary, and complete Plans. `/tmp/a4first-final-corpus-changes.json` is the machine-readable list. Drawings were generated with the repository tool `tools/render_sheet_selection.py` (native CLI SVG and PNG at 150 dpi). After the frame check, 10170 retains A3 at 1:1: its proposed A4 section touched the right frame. The other 99 corpus Plans have identical drawing data to the reviewed survey after removing only the policy explanation/metadata. The final readability-warning classification changes none of these 100 choices (`/tmp/a4first-visual-proof.json`).

## Named key parts

| Part | Before → after | Visual review |
| --- | --- | --- |
| Demo bracket | A4 1:2 → A4 1:2 | Clear; original sample read only. |
| 10098 | A4 1:2 → A4 1:2 | Clear. |
| threaded_connector_109 | A3 1:5 → A3 1:5 | Clear; smaller candidates rejected. |
| GRM-01 shank | A3 2:1 → A4 1:1 | [Final PNG](/tmp/a4first-grm01-final.png), clear. |
| cgb243 | A1 1:1 → A1 1:1 | Clear; retains large automatic baseline. |
| Plain string_post_tapped | A3 5:1 → two A4 sheets at 5:1 | Both pages clear; section references and page numbers reviewed. |
| Same plain tapped post, A3 printer | A3 5:1 → one A3 at 5:1 | Clear. |
| hw_string_post_tapped_intent | A3 5:1 → A3 5:1 | A4 candidates report unreadable-step exhaustion and are rejected. Final A3 inspected. |

Named PNGs and both stages are indexed in `/tmp/a4first-key-renders/index.json`. The final intent fixture supersedes that index’s earlier A4 trial: [final A3](/tmp/a4first-tapped-final.png). The A3-printer comparison is [here](/tmp/a4first-tapped-a3.png). The GRM-01 input is `/Users/paul/steps/GRM-01_shank.step`; the demo input is the requested frontend sample.

`tools/quality.py` completed all 30 review parts: zero off-sheet, overlapping-view, or title-block/view findings. Four existing ink findings remain on 10060 (one), cgb245 (one), and CTC-01 (two); the baseline code/subject rule intentionally tolerates those. The changed cgb245 drawing retains its one inherited ink contact; visual review found no additional contacts. All eight changed parts in the route baseline were regenerated with the scoped repository recorder; unrelated historical records were preserved. `check_leader_routes.py` passes 143 callouts on 30 parts, retaining four pre-existing long-run observations.

## Timing

Release CLI timings against `origin/main` (`ecb9f179`), 125 inputs: 100 corpus parts, 23 PMI fixtures, the demo bracket, and external GRM-01. Measured on Darwin arm64 with rustc 1.99.0 (b940084d7, 2026-09-28). The table sums three-run per-part medians, using identical cached Analysis files; analysis/render time is excluded. All rows are in [plan-times.csv](a4-first/plan-times.csv).

The policy executable is archived as `/tmp/a4first-f5c4b00`; the final `1f952be3` commit adds two rejection codes that occur in none of the 244 native survey Plans or 125 timing Plans. It does not change any measured selection. The earlier partial timing run is not used in this table.

Wall time is noisy because tests and other timing jobs shared this sandbox. Child CPU time shows the regressions independently of scheduling. Instruction counters were unavailable. The aggregate wall and CPU ratios exceed 1.5×, with many parts over the 2× ceiling.

| Part | main wall (ms) | Policy wall (ms) | Wall × | main CPU (ms) | Policy CPU (ms) | CPU × |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **Corpus 100 (sum)** | 330,821.8 | 1,178,350.2 | **3.56** | 180,630.0 | 1,006,780.0 | **5.57** |
| **All 125 (sum)** | 646,512.2 | 1,361,872.4 | **2.11** | 295,840.0 | 1,162,870.0 | **3.93** |
| threaded_connector_109 | 1,983.3 | 3,881.0 | 1.96 | 1,970.0 | 3,460.0 | 1.76 |
| cgb243 | 59,263.8 | 319,165.6 | 5.39 | 44,450.0 | 273,130.0 | 6.14 |
| cgb242 | 140,518.9 | 231,099.8 | 1.64 | 36,810.0 | 180,690.0 | 4.91 |
| cgb217 | 28,070.4 | 64,368.0 | 2.29 | 25,480.0 | 47,960.0 | 1.88 |
| cgb202 | 28,680.1 | 272,691.5 | 9.51 | 25,730.0 | 272,320.0 | 10.58 |
| nist_ftc_06_asme1_rd | 1,010.6 | 40,417.9 | 39.99 | 540.0 | 31,500.0 | 58.33 |
| nist_ftc_07_asme1_rd | 1,018.9 | 5,357.9 | 5.26 | 540.0 | 4,660.0 | 8.63 |
| nist_ftc_08_asme1_rc | 11,329.6 | 4,426.1 | 0.39 | 4,410.0 | 4,220.0 | 0.96 |
| nist_ftc_09_asme1_rd | 2,555.7 | 1,759.9 | 0.69 | 1,690.0 | 1,760.0 | 1.04 |
| nist_ftc_10_asme1_rb | 1,855.7 | 27,851.6 | 15.01 | 1,800.0 | 23,270.0 | 12.93 |
| nist_ctc_02_asme1_rc | 7,810.2 | 58,027.4 | 7.43 | 4,910.0 | 49,910.0 | 10.16 |
| nist_ctc_04_asme1_rd | 7,064.7 | 98,067.4 | 13.88 | 3,590.0 | 66,940.0 | 18.65 |

In the corpus, 20 of 100 parts exceed 2× wall time and 21 exceed 2× CPU time. Across all 125 inputs, 23 exceed 2× wall time and 24 exceed 2× CPU time. See the full CSV for every row, including the additional FTC AP242 inputs.

The expensive cases have small enough outlines to pass the mandated outline-only bound, then spend time in the existing explicit-paper scale search and R5 overflow. A direct FTC-06 A4/max=2 probe takes 27,830 ms CPU on main and 28,900 ms on the policy build and produces byte-identical Plans. Main’s automatic FTC-06 costs only 540 ms CPU. Required forced planning alone therefore exceeds the requested per-part allowance; memo reuse cannot remove its first run. No time cutoff, annotation heuristic, alternate allocator, or recovery machinery was introduced to hide this cost.

Timing artifacts: `/tmp/a4first-before/{timing.txt,timing-extra.txt,analysis,plans}` and `/tmp/a4first-timing-head/{timing.txt,plans}`. Input list: `/tmp/a4first-files.txt`.

## Validation

All required native tests use the corpus-required environment described below. Production revision: `1f952be3`; the final rendering fixture adjustments are `5f3c6a5` and `53bee3b1`. The report commit changes documentation only.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | Passed |
| `cargo nextest run --release --lib` | 280 passed, zero skipped |
| `cargo test --release --test audits` | 158 passed |
| `cargo test --release --test core` | 279 passed |
| `cargo test --release --test dimensions` | 336 passed |
| `cargo test --release --test rendering` | 473 passed |
| WASM parity (`node --test tools/wasm-parity.test.mjs`) | Six passed |
| `cargo test --release -p draftwright-wasm` | Passed (zero Rust unit/doc tests; parity exercises the built WASM) |
| WASM API snapshot check | Passed |
| MCP Spec outline tests | Three passed |
| Integration-binary reachability | Eight binaries, 141 reachable files, 1,305 expected integration tests |
| Leader routes | Passed, 143 callouts / 30 parts |

The twelve policy tests cover baseline A4 reuse, default two-A4 allocation, the same drawing on one A3, A3 need and baseline reuse, automatic fallback, explicit size/scale/count passthrough, baseline-reference isolation, validation, baseline code/subject comparison, and frame strokes. Existing tests that exercise geometry at a fixed frame now author paper, scale, or a sheet limit explicitly; automatic-policy assertions were updated to the new selected paper. Logs are `/tmp/a4first-{lib-final,all-head,rendering-final,clippy-head,fmt-head}.log` and `/tmp/a4first-wasm-head-*.log`.

## Reproduction

Use `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and `DRAFTWRIGHT_CORPUS_REQUIRED=1` for native checks, so corpus-backed tests do not silently skip. For timings, run the repository `tools/plan_timing.py` on archived release executables, with `--runs 3 --plans --no-instructions` and the same cached Analyses. On this sandbox `/usr/bin/time -l` cannot read its sysctl counters, hence the explicit option; wall and child CPU measurements remain available. Large gzipped inputs were decompressed into temporary files, without altering corpus sources.

The native render helper takes before/after survey directories and an output directory, plus `--corpus-only` or `--part PATH` for additional inputs. WASM was built with `wasm-pack build --no-opt --profile wasm-release --target web crates/draftwright-wasm --locked`, followed by `node --test tools/wasm-parity.test.mjs`; the profile still compiles optimized Rust, while optional wasm-opt is skipped.

No changes were made to CHANGELOG. Nothing was pushed.
