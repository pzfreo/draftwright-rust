# PR #73 independent review fixes

Starting point: `83dc22dc` on `cx/detail-views`. Each reported path was checked
against the source before editing. No test assertion, acceptance baseline, latency
limit, instruction budget or schema budget was relaxed.

| Item | Resolution and verification |
| --- | --- |
| 1 | Centre-work mapping uses `detail::crop_extent`. Detail curves are clipped after merging, with empty work removed. Cropped axes cannot be trimmed as feature overruns; intact feature overruns remain available. Regressions cover crop-centred point mapping and both inherited sheet and explicit view centre-line switches on a string_post head detail. Before the fix its 158 mm axis escaped the 20 mm crop. |
| 2 | Isometric-caption view obstacles and the hole-table frame use the crop extent. Both modules have regressions comparing the obstacle to the detail footprint; hole-table mapping also centres the crop. Before the fix the 15 × 15 mm crop reserved a 100 × 60 mm box. |
| 3 | Small-sheet rescue permits no more illegible steps than its required seed. The repeated-length turned fixture at explicit 1:10 now rescues an A2 seed to A4/A3 without changing scale or losing measurements. Its regression independently checks that the A3 candidate already passes every other gate. Before the fix both fitting small-paper candidates were rejected. |
| 4 | GRM-03 A3 10:1 measures 60.9474% fill. Restored the original 60–85% band, with its measured value documented in the test. |
| 5 | Removed the placement test's debugging `eprintln!`. |
| 6 | Detail furniture is filtered by the current sheet number. A regression verifies that foreign-sheet details cannot reserve local caption or marker furniture. |
| 7 | The extra settle pass now uses `!local_tolerances.is_empty()`, reusing the existing exclusion of explicitly placed dimensions. Existing pin and tolerance tests remain intact. Isolated before/after guard comparisons of pinned placements showed no drawing change. |
| 8 | Positive-rim shoulder witnesses are tried on copies and committed only if both fit. The updated comment describes the full head silhouette. The regression rejects a crop without changing the parent witnesses, then verifies successful positive-rim routing into a fitting crop. |
| 9 | Removed the unused model-space point from `detail_levels`; its two-field consumers are unchanged. |
| 10 | Both caption reservation and drawing use the crop-based slot, eight mm below the footprint. A padded-crop rendering regression failed before the fix because the caption sat 10 mm above its reservation. |

Eight added behavioral regressions were observed failing before their respective
fixes. The GRM-03 assertion restores deleted coverage. Debug output removal, the
settle guard and unused tuple cleanup are additionally checked by compilation,
lint and the existing placement/tolerance suite.

Local development evidence, including the pre-fix failures, is in
`target/review-fixes/`. The exploratory pin comparison preserves binaries differing
only in the settle guard; it is not an acceptance baseline.

## Sequential validation

Tested application source: `bf0f00b` (detail/dimension fixes in `4f5e2c9`).
Checks ran one at a time with `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Latency gates were enabled at their existing limits.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo test --workspace --release --no-fail-fast` | PASS: 1,252 tests, 0 failed, 0 ignored; 121 harness/doc results |
| Session and multi-sheet latency gates | PASS in the full suite; limits unchanged |
| Quality vs `tools/ci-quality-baseline.json` | PASS: all 300 cells unchanged across 30 parts |
| `tools/check_leader_routes.py` | PASS: 119 callouts / 30 parts; 14 unchanged pre-existing runs |
| `tools/check_plan_time.py` | PASS: 2,753,897,793,569 instructions < 3,165,755,533,285 limit; all 65 parts |
| `tools/generate_mcp_schemas.py` | PASS: 46,872 compact UTF-8 bytes; budget unchanged at 46,872 |
| `uv run --with jsonschema python tools/check_mcp_schemas.py` | PASS: 13 tool inputs, outputs and refusals; all 26 published and served schemas agree |

Schema files, generator and served-byte assertion have no diff from `83dc22dc`.
The schema checker used the writable `UV_CACHE_DIR=/private/tmp/draftwright-detail-uv-cache`.

Machine-readable results and full logs are in `target/review-fixes/checks.json`,
`verification.json`, `plan-time.json` and the adjacent `.log` files.


## Drawing preservation

Compared the preserved incoming release executable with the freshly built fixed
executable on the original corpus STEP files. All four complete Plan JSON objects
are equal, and all four rendered 150 dpi PNGs are byte-identical.

| Drawing | Sheet | Scale | Plan / PNG |
| --- | --- | --- | --- |
| string_post | A4 | 5:1 | equal / byte-identical |
| GRM-03 | A3 | 10:1 | equal / byte-identical |
| sm-hanger | A3 | 1:2 | equal / byte-identical |
| threaded_connector_109 | A3 | 1:5 | equal / byte-identical |

Exact PNG hashes and fill measurements are in
`target/review-fixes/drawing-comparison.json`; both sets of Plans and PNGs are
under `target/review-fixes/drawings/`.

The changes are committed in two coherent application/test steps, followed by
this verification report. Every commit has the requested Codex co-author footer.
Nothing was pushed.
