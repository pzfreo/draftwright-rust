# PMI/precision merge with fit labels — 2026-10-16

Merged origin/main `0af1323e` into PMI/precision `f2ed0a09` after reading both
commit histories and the changes introducing the conflicting behavior.

## Resolutions

- `src/dims/mod.rs` keeps both precision and profiled-bore modules. Lane preparation
  retains the fallible aligned-PMI return value and resolves the same printed terms
  as placement: PMI overrides and fit display, nominal precision, authored tolerances,
  consolidation and references. Its memo key includes the entire Spec, so changes to
  precision, fit display and tolerances cannot reuse incompatible reservations.
- `src/model.rs` carries aligned demands into layout while retaining #72's separate
  nominal anchors for authored linear tolerances, full printed extents, and the
  callout settlement pass after GD&T. Dimension lines and arrow flares remain obstacles.
- `src/lint/pmi_coverage.rs` keeps both independent source-site fidelity and main's
  manufacturing source-text checks, including its declared finish display precision.
- `tests/surface_texture.rs` keeps both mutation regressions and one atomic scratch
  directory counter, preventing concurrent tests from removing each other's fixtures.
- The workspace, WASM package, gear rules, text rules, and coverage/holes work remain.

## Precision and fits

Extent fits use #72's shared formatting decision: class only by default. Opting into
`sheet/fit_deviations` prints upper/lower deviations stacked at the shared smaller glyph
size; the searchable label encodes the stack with a newline. No inline slash pair is
introduced. The new field takes precedence over the legacy `fit_limits` alias.
Precision formats only nominal measurements; it preserves explicit tolerance digits,
ISO fit deviation precision, symbolic thread/reference text, and measurement provenance.
The extent-wide band remains a fallback; a per-extent tolerance or fit overrides it.

`tests/precision.rs::extent_precision_composes_with_default_and_stacked_fit_labels`
checks all three extent IDs with three nominal places, the default, enabled stacks,
and explicit new-setting false overriding legacy true. It checks searchable rendered
text, full reserved label dimensions, independent structural fidelity, and bands on
the other extents. Existing tests from both branches remain intact.

## Trust-fixture follow-up

The first complete release run reached `tests/trust_rules.rs` with 1,108 passes and
one failure: `the_hash_and_the_geometry_come_from_one_read` read two concatenated
STEP documents from its FIFO. Its timed fallback writer could reopen the original
FIFO before a descheduled first reader had consumed EOF. Production analysis still
reads the source exactly once (`read_step_file` then in-memory parsing).

The fixture now atomically replaces the FIFO path while the first writer is still
open, then closes that writer. A reopened path receives the second document, but
an outstanding first descriptor can receive only the first. Unique per-process
scratch directories prevent cross-run interference. The fallback's five-second
interval and every original hash, geometry and provenance assertion are retained.
The writer is joined and its errors are asserted. An additional control leaves EOF
unread until the second offer, checks the first stream has no trailing bytes, and
verifies a second open receives the different document. The corrected 15-test trust
suite, the remaining five view tests, and doc tests passed. The full release command
is repeated on the final files; the initial failure log is retained unchanged.

## Catalogue

Main's decisions are retained for all 509 entries outside the branch's six completed
ports. Every entry ported on either side remains ported. The six retained entries are
`LINT-code-pmi_source_site_mismatch`, `ISSUE-1209-a`, `ISSUE-1215-a`, `ISSUE-1349-a`,
`ISSUE-1349-b`, and `ISSUE-1351-a`. Main's `port_note` is retained on those entries;
the extent-fit evidence now documents the composed display behavior.

Counts recomputed from the YAML (515 entries):

| File | Pending | Ported | Translated | Retired |
| --- | ---: | ---: | ---: | ---: |
| adr.yaml | 7 | 66 | 61 | 26 |
| lint.yaml | 9 | 123 | 14 | 19 |
| issues.yaml | 27 | 86 | 51 | 26 |
| All | 43 | 275 | 126 | 71 |

The class totals remain 271 drafting, 179 trust, 16 spec, and 49 machinery.
README tables match these computed counts. A field-level audit is retained at
`target/pmi-precision-merge/catalogue-audit.json`.

## Validation

Commands source `$HOME/.cargo/env` and use `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`.
Cargo test runs require the corpus with
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1`.
Logs and drawings are retained under `target/pmi-precision-merge/`. No quality baseline,
latency threshold, or original assertion is relaxed.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Focused PMI/precision, fit-layout, catalogue and source-finish suites | PASS: 80 tests in six harnesses |
| Final corpus-required `cargo test --release --no-fail-fast` | 1,112 passed, two failed, zero ignored across 112 result blocks; exit 101. Failures detailed below. |
| Unchanged `cargo test --release --test stateless` rerun | PASS: all 13 tests, including both slow-request deadline cases |
| Corrected trust fixture suite | PASS: all 15 tests, including the added two-open/delayed-EOF control |
| Remaining view suite and doc tests | PASS: five view tests, zero doc tests |
| `python3 -I tools/quality.py` on the exact 30-part manifest | PASS |
| `tools/ci_acceptance.py --check-results quality` against `tools/ci-quality-baseline.json` | PASS: all 300 cells identical, zero regressions |
| `python3 -I tools/check_leader_routes.py target/pmi-precision-merge/quality` | PASS: 119 callouts on 30 parts, 14 unchanged pre-existing runs |

The initial full run (`full-tests.log`) records the FIFO fixture failure described above;
both its multi-sheet edit/MCP timing gate and all eight session tests passed. The final
complete repeat is recorded in `full-tests-final.log` after the fixture correction.

Quality totals before and after are identical: off-sheet 0, repeated labels 7, repeated
measurements 0, overlapping views 0, views on title block 0, ink conflicts 52, repeated
spans 0, lines through features 73, hidden-on-visible 2,231, section-hidden 0. The baseline
file is unchanged. Leader routes add no hidden/section/cutting-plane crossings, extra
length beyond the existing 2 mm allowance, or new runs over 2 mm.

### Final release failures and diagnostic evidence

The complete final run (`full-tests-final.log`, `test-summary.json`) finished every
harness. Its corrected trust suite passed all 15 tests, including the delayed-EOF
control. Its multi-sheet edit/MCP latency gate passed. The two failures were:

- `tests/session.rs::nist_benchmark_deltas`: NIST CTC01 native p90 **627.891 ms**
  against the unchanged **150 ms** gate. The nine samples were 15.766, 16.256,
  18.981, 19.059, 19.189, 20.067, 25.658, 45.150 and 627.891 ms.
  The other seven session tests passed. All eight had passed in the initial run.
  No latency assertion, calibration, threshold or benchmark was changed.
- `tests/stateless.rs::http_slow_head_has_absolute_deadline_and_serves_queued_ping`:
  `http_response` received `ConnectionReset` (OS error 54) while reading the response.
  The test and server remain unchanged. The initial full run passed all 13 stateless
  tests; the final run passed 12. An unchanged rerun of the entire 13-test suite passed
  in 30.56 seconds (`stateless-rerun.log`). The reset assertion was not relaxed.

The repeated PMI corpus checks were slow but passed. During the long FTC10 native
analysis, a byte comparison proved the temporary input identical to the committed
fixture (1,879,881 bytes; SHA-256
`9d7711f48e81831c5d1838fdac5c11c0de1aa9b61f3c1d67930a7638b3b25e69`).
An isolated diagnostic analysis hit its own 60-second bound (`ftc10-diagnostic.json`);
this did not stop or alter the test. The full PMI coverage, dimension, drawing and
label comparisons subsequently passed. No production workaround was introduced.
