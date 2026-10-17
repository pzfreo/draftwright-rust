# Detail integration with main #80 and #81 — 2026-10-17

Fetched origin and merged `origin/main` at `54704307` into `cx/detail-views`,
starting from the accepted `3cc87e4e`. No push was made.

Resolved both modules in `src/lib.rs`; retained both the explicit-scale
`legibility_floor_breached` warning and `recognition_corrections::report` in
`src/model.rs`. Every test function and body from both MCP efficiency files
survives intact. Both changelog entries and WASM JSON notes are retained.

The generator combines typed detail records and placement names with main's
recognition declarations, label text/style fields, shared map shapes, iterative
compaction and root reference targets. JSON schemas were regenerated exclusively
with the script. No JSON conflict was hand-merged.

## Schema accounting

**46,656 compact UTF-8 bytes** across 26 standalone schemas; both generator and
served-byte Rust assertion use the exact minimum strict bound **< 46,657**.
Main regenerates to 42,465 bytes; the same merged compaction without the detail
record is 42,553 bytes. Thus the record contributes **4,103 bytes** and detail
placement names add **88 bytes**. Main's compaction saves 13 bytes of the previously
justified 4,116-byte record. See [the full accounting](detail-schema-budget.md).

## Drawing preservation

Replanned and rerendered each original corpus STEP using both the preserved
accepted executable (SHA-256
`77cfc40ea0d18bf4eaa9f0194639600f9b51849d579229a47ec9c7cb35592e6b`)
and the freshly built merged executable. All four full Plan JSON objects compare
equal, and every 150 dpi PNG is byte-identical. All have zero enlarged details.

| Drawing | Sheet | Scale | Plan / PNG |
| --- | --- | --- | --- |
| string_post | A4 | 5:1 | identical / byte-identical |
| GRM-03 | A3 | 10:1 | identical / byte-identical |
| sm-hanger | A3 | 1:2 | identical / byte-identical |
| threaded_connector_109 | A3 | 1:5 | identical / byte-identical |

## Verification

Checks run sequentially with `CARGO_BUILD_JOBS=4` and `RUST_TEST_THREADS=4`.
Corpus-dependent tests use `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus`
and `DRAFTWRIGHT_CORPUS_REQUIRED=1`. The latency gate remains enabled by default.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Release build | PASS |
| `uv run --with jsonschema python tools/check_mcp_schemas.py` | PASS; 13 tools, 26 published/served schemas, input/output/refusal validation |
| Optimized web WASM build, locked, `wasm-release` | PASS |
| Native reference build and Node WASM/native parity | PASS; 4 tests, zero skips |
| `tools/check_wasm_api.py --base-ref origin/main` | PASS; reviewed snapshot matches |
| WASM size guard | PASS; 14,294,855 < 15,700,000 raw bytes |
| MCP efficiency | PASS; 11 tests |
| Engine version / served-byte assertion | PASS; 3 tests, 46,656 bytes |
| Detail views | PASS; 21 tests |
| Recognition corrections | PASS; 21 tests |
| Label text | PASS; 17 tests |
| Schema compaction equivalence | PASS; 4 tests |
| WASM label text/style smoke check | PASS |
| Quality vs `tools/ci-quality-baseline.json` | PASS; all 300 cells exactly equal across 30 parts |
| Leader routes | PASS; 119 callouts / 30 parts; 14 unchanged pre-existing runs |
| `tools/check_plan_time.py` | PASS; 2,748,483,515,098 < 3,165,755,533,285 instructions; all 65 parts |
| `cargo test --workspace --release --no-fail-fast`, corpus required | PASS; 1,228 tests, 0 failed, 0 ignored, 120 harness/doc results |
| Session and multi-sheet latency gates | PASS in the full suite; limits unchanged |

The first `uv` invocation could not open its default cache in the sandbox; the
successful run uses `UV_CACHE_DIR=/private/tmp/draftwright-detail-uv-cache`.
WASM packaging uses the existing writable
`WASM_PACK_CACHE=/private/tmp/draftwright-wasm-pack-cache` and its normal optimizer.
No acceptance baseline, latency threshold, routing threshold, instruction budget,
or WASM ceiling was changed.

Local logs, generated drawings, byte-comparison hashes and machine-readable
accounting are under `target/detail-main-81/` (ignored build evidence).
