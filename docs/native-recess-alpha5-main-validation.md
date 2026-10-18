# Native recesses: alpha.5 and main integration

This work starts at `61222902` on `cx/native-section-recesses`. Nothing is pushed.
All validation uses `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Checks run serially; latency gates remain enabled.

## Pin and merge

`195dbe0` pins both `haecceity` and `quiddity` to the released
`tag = "v0.1.0-alpha.5"`, commit `6494e07f2e0241e60663bf4e603335d27409b5f6`.
Those are the only two package entries changed in `Cargo.lock`.
`python3 tools/check_quiddity_pin.py` exits successfully. Its first invocation still
warned about an external mismatch: specify-core-rust's latest published alpha.4
uses `f5ddb726…`. The local manifest uses the requested release tag; the checker
and sibling release are not modified.

The incoming commits for #82 (dimension schemes), #83 (Spec discovery) and #73
(detail views, inspection origins and printed legibility) were read before merging.
`8570c75` merges fetched main `adf92b4ce39bab26140f247ac40775ca7e00d849`.

The resolutions retain native recess placement stages, measured corridor and
historical-route guards alongside main's detail recovery, common-line chains,
authored networks, inspection-end baselines and legibility checks. Inactive
internal detail omissions do not veto established principal-view repairs.
Comparison recipes keep their established routes; ordinary inspection repair
retains main's uncertain-ink checks, while recess and edited-label repair retain
the historical-path proof.

Schemas are regenerated with `python3 tools/generate_mcp_schemas.py`, without
`-I`. No schema JSON is hand-merged. Main's `tools/mcp_spec.py` remains the full
Spec vocabulary; no new Spec fields are introduced. The advertised total remains
exactly **34,477 bytes**, including main's detail-view choices.

The drawing catalogues were merged structurally, retaining both sides' additions
and removal of obsolete exceptions. Catalogue status counts are recomputed from
the merged YAML. The two quality-baseline files were already byte-identical
(`c4584ed962c84bc4e0cedeb139de35fb102fe445`); their per-cell minimum and main's
metadata therefore require no baseline edit.

## Drawing choice and coverage

FTC08 keeps its clear **A1 at 1:2** drawing with the hole schedule. The first
merged search instead retained a congested A2 seed because a slot location's
controlling hole location moved into that schedule. The new coverage check
follows that ownership chain to actually placed ink. An unplaced table or an
unrooted cycle cannot satisfy coverage; a regression proves both refusals.

10060 keeps its established vertical ramp-angle route. GRM05's ordinary inspection
repair keeps its leader clear without overriding the inspection-end baseline.
`5eb98a8` records these integration repairs.

The measured thirty-part quality comparison has **0 worse cells and 7 better**
than main, across all 300 cells. Ink falls **17 to 9**, and feature crossings
**73 to 65**. Repeated labels remain 7. Off-sheet items, repeated measures/spans,
view overlaps, title-block overlaps, hidden-on-visible and section-hidden counts
remain zero. Leader routes pass for **144 callouts**, with **10 unchanged
pre-existing runs**.

Evidence: `target/alpha5-main-validation/quality-repaired/`,
`quality-comparison.json` and `leader-routes-repaired.json`.

## Final integration regressions

The first complete required-corpus workspace run found **1,287 passed, nine failed,
zero ignored** across 123 groups. Eight failures were functional; one was the
unchanged NIST session latency gate (CTC01 p90 **523.993 ms**, limit 150 ms).
The functional failures exposed canonical peer ink being recomputed against
already edited furniture, rejected crops reappearing in A0 packing templates,
and stale captured-drawing expectations. The initial log and summary are retained
as `full-suite-initial.log` and `full-suite-initial-summary.json`.

Bare pins and annotation text edits now reuse the actual settled canonical peer
ink. Single-pass text edits use their single-pass canonical drawing; recovered
edits retain the recovered detail choice. Multi-item tolerance edits still run
full recovery, including the saved fourteen-tolerance string-post spec.

Captured m10046 retains its clear **A4 at 1:1 detail** by fitting complete callout
columns on the same sheet and scale. Captured m1000 retains **A4 at 1:1** with all
band overruns removed; its roomy template omits crops the recipient declined.
Reservations use the actual crop extent and explicitly protect caption clearance.
No larger paper is accepted merely to fix that initial estimate.

Captured m10127 keeps the complete **A3 at 2:1 principal drawing**. Its two height
rungs have 8.16 mm paper separation, above the unchanged 7 mm spacing floor.
The existing test now checks every original through-step and rung assertion in
both that default drawing and an explicit **A3 at 1:1** constrained drawing. The
original detail-ownership assertion remains in the constrained case; the default
adds exact sheet/scale, no-detail, spacing and clear-annotation assertions.
Only three obsolete comparison exceptions are removed: the two unmatched
32.4 mm dimension rows and the redundant view-set difference. No new exception
or relaxed physical comparison is added.

All **57 tests** in `detail_views`, `dimensions`, `dims_plates`, `dims_steps`,
`placement` and `views` pass in `integration-repairs-final.log`. The native
workspace suite and workflow acceptance jobs were then repeated on this source.

## Planning work

The original Darwin arm64 budget is **2,752,830,898,509 instructions**, with
a 15% limit of **3,165,755,533,285 instructions** (recorded 16 October).

| Measurement | Retired instructions | Result |
| --- | ---: | --- |
| Accepted starting branch | 3,132,566,556,944 | PASS |
| First merged release (recesses + accumulated main work) | 3,537,154,547,705 | FAIL |
| Coverage and initial exact-query reuse | 3,466,229,923,462 | FAIL |
| Shared projected trees and query preparation | 3,384,152,670,200 | FAIL |
| Early guards and constant span checks | 3,221,611,569,654 | FAIL |
| Lazy peer lists and PMI classification | 3,204,717,313,660 | FAIL |
| Bounded extension contacts and recess preparation | 3,167,175,161,459 | FAIL |
| Exact traversal and short-circuit guards | 3,165,975,917,897 | FAIL |
| Bounded model contacts (`a6fa9bb`) | **3,165,503,533,631** | **PASS** |
| Final repairs, no sparse remap (old budget) | 3,166,188,461,378 | FAIL, 1.150157267x old budget |
| One final-source re-record | **3,166,420,524,176** | **PASS; 1.150241566x old budget; new 15% limit 3,641,383,602,802** |
| Final workflow acceptance (new budget) | 3,166,228,578,313 | PASS; 0.999939381x new budget |

The sparse-owner remapping experiment passed its exact full-scan regression but
measured **3,166,317,477,055** instructions versus **3,166,188,461,378**
without remapping (a 0.0041% increase). It is reverted under the maintainer
criterion; the sparse-ID regression remains. No standalone measurement of the
merged main revision is available, so the first merged row must not be read as
recess cost alone.

The maintainer authorizes one budget re-record on the final source: the old
record predates five intended feature merges. The new figure accounts for native
recesses on top of main's accumulated feature work. The margin stays **15%**;
no checker, other budget, threshold or quality baseline changes. The record moves
from **2,752,830,898,509** to **3,166,420,524,176** instructions (**1.150241566x**),
reflecting native recesses' measured cost on top of main's accumulated feature work.

By `a6fa9bb`, the retained optimisations removed **371,651,014,074 instructions**
from the first merged build. The final repairs are measured separately above. An additional capability-result memo experiment was discarded
after it increased the count; its evidence is retained. The workflow acceptance
job measures the final source again without re-recording.

The profiling instrumentation is removed from the final source. No native or
WASM release profile, search bound or clearance threshold changes. The sole
instruction-budget re-record is described above. The optimisations share exact
projected ink, geometry queries and pure placement results; reject candidates only once an existing bound already rules
them out; prepare peer obstacle lists lazily; and reuse immutable PMI lowering
and inspection-datum preparation. Contact queries stop when the incumbent's
existing bound has already rejected a candidate; incomplete cached counts retain
their lower-bound status. CTC02 AP242's complete Plan was checked equal
to the saved reference while its measured cost fell from about 824B to 647B.

Full-scan regressions cover shared versus local ink, cached contacts, uncertain
boxes and changed crop centres. PMI preparation is compared with uncached
lowering for all eight principal-view combinations, including every diagnostic.
The frozen view-conflict regression retains its unsafe candidate and original
assertions under the expanded memo key and both route-policy states.

## Validation evidence

The complete command, exit-code and elapsed-time ledger is
`target/alpha5-main-validation/checks.jsonl`; each command has its own log there.
Initial failed measurements and diagnostic checks are retained, including the
first quality-pins job whose pin bar passed but whose quality and route checks
exposed the integration defects above.

## Final checks and acceptance results

All runs below use the final release source, with sparse-owner remapping reverted.
The final release binary SHA-256 is
`f93071df191b4c973b4df04bc7cb2014a05e71ec63bf3a7237596e32b11f3762`.
`DRAFTWRIGHT_LATENCY_GATE` is unset (the native gate remains enabled), and all
checks run one at a time with the required corpus and both concurrency values at 8.

| Check | Result |
| --- | --- |
| Sparse-owner exact full-scan regression after reverting remapping | PASS |
| Locked release build before the one budget recording | PASS |
| One `check_plan_time.py --record` on final source | PASS; 3,166,420,524,176 instructions |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo test --workspace --release --no-fail-fast` | PASS; 1,296 passed, 0 failed, 0 ignored in 123 groups (2,077.486 s); includes doc-tests and native latency gates |
| Separate `cargo test --release --test audits rules_catalogue::` | PASS; 6 tests |
| Locked release rebuild for schema validation | PASS |
| `python3 tools/generate_mcp_schemas.py` | PASS; 14 tools, **34,477 compact advertised schema bytes**; no generated file changes |
| `uv run --with jsonschema python tools/check_mcp_schemas.py target/release/draftwright` | PASS; 28 published/served schemas agree; all tool input/output/refusal checks and 1,872 checked-in Spec examples |
| WASM safeguard Python tests | PASS; 16 tests |
| `wasm-pack build crates/draftwright-wasm --target web --profile wasm-release --locked` | PASS |
| Locked native `wasm_reference` example build | PASS |
| Node 22 `--test tools/wasm-parity.test.mjs` | PASS; 5 tests |
| `python3 tools/check_wasm_api.py --base-ref origin/main` | PASS; reviewed API unchanged against merged main `adf92b4c` |
| `python3 tools/wasm_size.py crates/draftwright-wasm/pkg/draftwright_wasm_bg.wasm` | PASS; raw 15,025,924 / 15,700,000 bytes; gzip 5,027,611; Brotli 3,354,406 |

### Workflow acceptance

Each job runs the workflow command
`python3 -I tools/ci_acceptance.py --job JOB [matrix arguments] --out target/acceptance`.
The runner moves that fresh output directory to
`target/alpha5-main-validation/final-JOB/` (including the cell/shard suffix)
after each job, so every run retains its own `summary.json`, fixture hashes,
binary hash, step commands and logs. No timeout override or acceptance bypass is used.

The CI quality comparison has **0 worse cells and 7 better cells** across 300
cells against unchanged `tools/ci-quality-baseline.json`: ink **17 → 9**,
feature crossings **73 → 65**, repeated labels **7 → 7**, and every other gated
metric remains zero. Leader routes pass for **144 callouts on 30 parts**, with
**10 unchanged pre-existing runs**. Evidence: `final-quality-comparison.json`
and each cell's `leader-routes.json` / `leader-routes.log`.

The quality-pins jobs also invoke `check_label_lanes.py --current`, which first
checks the older `docs/status-2026-10-10.md` table through
`check_quality_table.py`. That historical comparison fails on
`unplanned_features` in **21 parts** (for example 1000: 0 → 5, CTC03: 12 → 16).
The gate exits there, before its own-span and STC09 locality assertions; those
later assertions are therefore **not validated by this acceptance step**.
The job failures are retained and reported. Neither the historical table nor
any checker, test, threshold or quality baseline is changed to accept them.

| Workflow job / matrix arguments | Result | Wall seconds |
| --- | --- | ---: |
| quality-pins `--pin-cell 1-1` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 162.389 |
| quality-pins `--pin-cell 1-3` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 172.294 |
| quality-pins `--pin-cell 1-8` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 184.281 |
| quality-pins `--pin-cell 2-1` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 287.650 |
| quality-pins `--pin-cell 2-3` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 252.866 |
| quality-pins `--pin-cell 2-8` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 254.673 |
| quality-pins `--pin-cell 3-1` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 253.982 |
| quality-pins `--pin-cell 3-3` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 184.591 |
| quality-pins `--pin-cell 3-8` | FAIL; historical-table locality only; quality, routes, deliberate-regression rejection and pins PASS | 215.026 |
| mcp | PASS; Python quality tests, acceptance tests, Node client tests; 7/7 scripted tasks, 16 tool calls, 6,280 estimated tokens | 11.629 |
| plan-time | PASS; 3,166,228,578,313 instructions; 0.999939381x new budget; 3,641,383,602,802 limit | 271.613 |
| render-tail | PASS; cgb243 2,824,815,165,405 instructions (1.052915487x); threaded_connector_109 4,098,471,484,059 (1.004007766x); original 15% limits unchanged | 169.965 |
| repeated-spans `--survey-shard 1/4` | PASS; 17 parts; 0 repeated spans | 27.460 |
| repeated-spans `--survey-shard 2/4` | PASS; 16 parts; 0 repeated spans | 102.544 |
| repeated-spans `--survey-shard 3/4` | PASS; 16 parts; 0 repeated spans | 40.263 |
| repeated-spans `--survey-shard 4/4` | PASS; 16 parts; 0 repeated spans | 317.560 |

All **270 random-pin runs pass**. All nine fresh quality measurements have no
cell worse than the CI baseline, and all nine leader-route checks pass. The
acceptance matrix totals **7 PASS / 9 FAIL**; every failure is the historical
status-table comparison above, with no other failed step. No latency gate fails
in this final run. The four survey slices partition **65 actual inputs** exactly
once; their legacy `of: 63` summary metadata is left unchanged.

The complete aggregate is `final-acceptance-summary.json`, with pin details in
`final-pin-grid-summary.json` and native counts in `final-full-suite-summary.json`.
All 16 job summaries report the same final binary hash. An audit confirms one
`--record` invocation, only the authorized plan-budget file changed among budgets,
and no checker, quality baseline, workflow, profile or other numerical threshold changed.
The only fixture removals strengthen comparisons by deleting obsolete exceptions;
the through-step test retains its original assertions in the constrained drawing
and checks the complete default drawing as well.

### Commits from the resumed session

- `650295c2`: preserve canonical peers and complete crop columns through integration recovery.
- `4d0d0c2`: retain exact sparse-owner contact coverage and satisfy Clippy; discard sparse remapping.
- `3f83bba`: record alpha.5/main integration repairs and planning evidence.
- `167c26f`: re-record only the final plan-time budget, with old/new figures, ratio and rationale.
- This final documentation commit records all checks and acceptance outcomes.

Every commit ends with `Co-Authored-By: Codex <codex@openai.com>`. No commits are pushed.
