# Default title fields: #101 and #103

The drawing number uses the first available value in this order:

1. An explicit Spec `sheet/title/number`, including an empty or whitespace-only value.
2. A leading part-number token in the selected STEP product name.
3. A leading part-number token in the source file stem (without STEP/gzip extensions).
4. `DWG-001`.

A token consists of ASCII letters/digits separated by single internal hyphens,
contains at least one digit, and keeps its source spelling. Leading whitespace is
ignored; spaces, underscores and other punctuation end the token. A later number
in a description is not a leading token. Purely numeric tokens qualify.

| Product name | File stem | Default number |
| --- | --- | --- |
| `GRM-02 Shaft` | `GRM-03_thumbwheel_drive_screw_ap242_pmi` | `GRM-02` |
| `Thumbwheel drive screw` | `GRM-03_thumbwheel_drive_screw_ap242_pmi` | `GRM-03` |
| absent | `10000` | `10000` |
| `Bracket 42` | `bracket` | `DWG-001` |
| `GRM--03 Thumbwheel` | `bracket` | `DWG-001` |

The readable title rule from #108 is unchanged: use a meaningful product name,
otherwise a readable file stem, replacing underscores/hyphens with spaces and
uppercasing. Keep the whole name, including its part number. Explicit Spec titles
still win, including blanks. A selected assembly member retains the original
source stem for fallback; older Analyses use their binding name when that field
is absent.

## GRM-03 material

The tester's original AP242 file uses a CAx-IF §4.1 material-name property:
`PROPERTY_DEFINITION` #1996 → `PROPERTY_DEFINITION_REPRESENTATION` #1997 →
`REPRESENTATION` #1994 → `DESCRIPTIVE_REPRESENTATION_ITEM` #1995. This is the same
form as the working Brass CW614N and Stainless steel 303 fixtures. Haecceity reads
identification `CZ121 / CW614N / CuZn39Pb3`, description `Free-machining leaded brass`,
no density, and provenance `[1994, 1995, 1996, 1997]` on part 0.

At starting main `1e5a87a1`, draftwright already projects this material to its
separate `Material` fact and prints the designation in the title block. The census
source `material:0:0` says it has no annotation record because materials use that
separate fact. #108 already suppresses its undrawn warning when the title block
states it. No additional material-reader or kernel change is needed for this file.
The new regression checks the original file's kernel reading, material fact,
planned cell, rendered text, source annotation and coverage lint. See the
[fixture provenance and exact STEP entities](../tests/fixtures/material/README.md).

The rebuilt binary plans that file with number `GRM-03`, title
`GRM 03 THUMBWHEEL DRIVE SCREW`, and material `CZ121 / CW614N / CuZn39Pb3`, without
an undrawn-material warning. Historical Python captures remain unchanged; live
regressions assert the new drawing-number defaults directly.

## Verification (2026-10-18)

All checks ran sequentially with `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. No assertion, latency gate, threshold or baseline
was weakened. No active `DWG-001` expectation needed replacement.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo test --release --test rendering` | PASS: 456 tests, 389.08 s |
| `cargo test --release --test core` | PASS: 269 tests, 156.02 s |
| `python3 tools/check_test_binaries.py` | PASS: 8 integration binaries, 138 reachable Rust files, 1,263 tests |
| Same inventory guard with unfiltered compiled workspace nextest JSON | PASS: all 1,263 integration test names match |
| `cargo nextest run --release --workspace --no-fail-fast --test-threads 8` (once) | 1,514 passed, 2 failed, 0 skipped; 1,323.168 s |
| `cargo test --release --workspace --doc` | PASS: 0 doc-tests in either crate |
| `python3 tools/ci_acceptance.py --job mcp` | PASS: all 7 MCP tasks and supporting Python/Node checks; 22.93 s |

The full run's only failures were unchanged 150 ms performance gates: R5 MCP
p90 **715.391 ms**, and NIST session native p90 **922.351 ms**. Separate unchanged
binary reruns also failed: R5 MCP p90 **758.514 ms** (native p90 **100.485 ms**
passed), and NIST p90 **215.960 ms** (the session binary's other seven tests passed).
These failures are retained as failures, separately from the passing functional
checks and MCP acceptance.

To check whether these failures were introduced here, a detached worktree of
starting main `1e5a87a1` ran the same two unmodified binaries separately with the
same environment, release profile and corpus. Main also failed both gates:

| Starting-main check | Result |
| --- | --- |
| `multi_sheet_latency` | FAIL: MCP p90 **527.332 ms**; native p90 **42.299 ms** passed |
| `session` | 7 passed, NIST native p90 **552.688 ms** failed |

This reproduces both performance failures before the drawing-number change. It
does not turn the branch's failed full run into a pass. The comparison worktree
was removed afterward and release binaries were rebuilt from this branch; no
performance-test implementation, threshold, baseline or kernel code changed.
