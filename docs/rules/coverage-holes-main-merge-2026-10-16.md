# Coverage/holes merge with main, 2026-10-16

Merged `origin/main` at `717def3e` into the coverage/holes batch at `81f61298`. No push was made.

## Batch statuses retained

These nine entries retain the batch's ported status, Rust tests and Rust notes. All other fields, including main's triage `port_note` tags, come from main.

`ARCH0014-5`, `ARCH0020-2`, `LINT-hole_coverage-second_countersink_unverifiable`, `LINT-code-unrecognised_defining_geometry-prismatic`, `ISSUE-1058-a`, `ISSUE-1357-c`, `ISSUE-1370-a`, `ISSUE-1370-b`, `ISSUE-1370-c`.

## Main decisions retained

All other 506 entries retain main's status and notes, including all 128 translated and 71 retired verdicts. The only appended note is the concrete angular-placement blocker for `LINT-angular-profile_angle_requirement`, which main still marks pending. No batch test citation or partial implementation note replaces main's assessment of that rule.

The following 29 entries changed on either branch since the common ancestor and take main's decisions; the other 477 entries already agree with main.

| Entry | Final status |
| --- | --- |
| `ADR2-U1` | ported |
| `ADR3-8` | ported |
| `ADR5-16` | ported |
| `ARCH0009-2` | pending |
| `ARCH0009-5` | ported |
| `ARCH0015-3` | pending |
| `ARCH0020-1` | pending |
| `LINT-_coverage_common-assembly_severity` | ported |
| `LINT-schedule_evidence-verified_cells` | ported |
| `LINT-code-feature_not_dimensioned` | pending |
| `LINT-angular-profile_angle_requirement` | pending |
| `LINT-gear_coverage-data_table` | pending |
| `LINT-code-gear_requirement_mismatch` | pending |
| `LINT-code-annotation_overlap` | ported |
| `LINT-code-manufacturing_reference_unresolved` | ported |
| `ISSUE-827-a` | ported |
| `ISSUE-1086-a` | pending |
| `ISSUE-1086-b` | pending |
| `ISSUE-1143-e` | pending |
| `ISSUE-1144-a` | ported |
| `ISSUE-1144-b` | pending |
| `ISSUE-1155-a` | ported |
| `ISSUE-1202-a` | ported |
| `ISSUE-1215-c` | pending |
| `ISSUE-1215-f` | ported |
| `ISSUE-1217-a` | pending |
| `ISSUE-1353-b` | ported |
| `ISSUE-1543-c` | ported |
| `ISSUE-1555-a` | ported |

## Final counts

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| `adr.yaml` | 8 | 64 | 62 | 26 |
| `lint.yaml` | 15 | 116 | 15 | 19 |
| `issues.yaml` | 35 | 78 | 51 | 26 |
| **All** | **58** | **258** | **128** | **71** |

## Code and baseline resolution

Read both branches' commits and patches before resolving code and test conflicts. Keep min-cost balloon assignment and dimension/extension crossing rejection from the batch, plus main's retained-leader ring checks, overlap/T-contact rejection and whole-table rollback. Both appended test groups in `tests/completeness.rs` and `tests/hole_tables.rs` are retained. The other code changes merged automatically.

All 300 quality-baseline cells use the minimum of the branch values. The result equals main's baseline, including every metadata field. No latency gate or threshold was changed.

## Validation

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- Corpus-required targeted release tests (`rules_catalogue`, `coverage_holes`, `completeness`, `hole_tables`): **49 passed**, including all six catalogue tests and all 13 coverage/holes scenarios.
- `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`: **657 passed, one failed** before Cargo stopped at `multi_sheet_latency`.
- Continued all 50 integration targets not reached by that command, using the same corpus-required environment and `cargo test --release --no-fail-fast --test <target> ...`: **415 passed, zero failed**.
- `cargo test --release --doc` in the same environment: passed (no doc tests).
- Complete release coverage: **1,072 passed, one failed, zero ignored**. All 104 integration targets ran, plus library and binary targets. The library's 204 passing tests include the batch's min-cost assignment regression and main's retained-shaft contact regression.

The sole failure is the unchanged wall-clock gate `r5_multi_sheet_latency`: MCP p90 **184.212 ms** exceeds **150 ms**. Session p90 was **128.292 ms**; every edit still used one plan, and MCP wire size was 1,132 bytes. The gate, its thresholds and its test source are untouched.

A separate entry-by-entry audit verified all 515 catalogue decisions, preserved batch status/test/note fields for exactly nine entries, retained main's other fields and `port_note` tags, and checked all 300 baseline metric cells and all metadata.
