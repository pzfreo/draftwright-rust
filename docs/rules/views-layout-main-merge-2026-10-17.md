# Views and layout merge validation

Merged `origin/main` at `923bab69` into the views-and-layout batch. Read both branches’ code changes before combining corridor reservation, callout and precision bands, section allocation, and projection behavior.

## Catalogue decisions

Retained this batch’s `status: ported`, `rust_test`, and `rust_note` for `ARCH0004-3`, `ARCH0018-1`, `ISSUE-563-b`, `ISSUE-1262-a`, `ISSUE-1530-a`, `ISSUE-1604-a`. Main supplies every other entry, including all `port_note` tags. No pending blocker note was added by this batch relative to the merge base, so none needed appending.

The following entries changed on main relative to this branch; main’s verdict and notes win:

| Entry | Final status |
| --- | --- |
| `ADR2-U1` | ported |
| `ADR3-8` | ported |
| `ADR5-16` | ported |
| `ARCH0009-2` | pending |
| `ARCH0009-3` | ported |
| `ARCH0009-5` | ported |
| `ARCH0014-5` | ported |
| `ARCH0015-3` | ported |
| `ARCH0020-1` | pending |
| `ARCH0020-2` | ported |
| `ISSUE-1058-a` | ported |
| `ISSUE-1086-a` | ported |
| `ISSUE-1086-b` | ported |
| `ISSUE-1143-e` | pending |
| `ISSUE-1144-a` | ported |
| `ISSUE-1144-b` | pending |
| `ISSUE-1155-a` | ported |
| `ISSUE-1202-a` | ported |
| `ISSUE-1209-a` | ported |
| `ISSUE-1215-a` | ported |
| `ISSUE-1215-c` | pending |
| `ISSUE-1215-f` | ported |
| `ISSUE-1217-a` | pending |
| `ISSUE-1298-a` | pending |
| `ISSUE-1332-a` | ported |
| `ISSUE-1349-a` | ported |
| `ISSUE-1349-b` | ported |
| `ISSUE-1351-a` | ported |
| `ISSUE-1353-b` | ported |
| `ISSUE-1357-c` | ported |
| `ISSUE-1360-a` | pending |
| `ISSUE-1370-a` | ported |
| `ISSUE-1370-b` | ported |
| `ISSUE-1370-c` | ported |
| `ISSUE-1432-a` | pending |
| `ISSUE-1485-a` | pending |
| `ISSUE-1485-c` | pending |
| `ISSUE-1505-a` | pending |
| `ISSUE-1543-c` | ported |
| `ISSUE-1555-a` | ported |
| `ISSUE-827-a` | ported |
| `ISSUE-885-a` | pending |
| `ISSUE-885-b` | pending |
| `ISSUE-958-a` | pending |
| `LINT-_coverage_common-assembly_severity` | ported |
| `LINT-angular-profile_angle_requirement` | pending |
| `LINT-code-feature_not_dimensioned` | pending |
| `LINT-code-gear_axis_mismatch` | ported |
| `LINT-code-gear_correspondence_unverifiable` | ported |
| `LINT-code-gear_repeat_count_mismatch` | ported |
| `LINT-code-gear_requirement_mismatch` | ported |
| `LINT-code-gear_semantics_missing` | ported |
| `LINT-code-interior_label_on_narrow_material` | pending |
| `LINT-code-manufacturing_reference_unresolved` | ported |
| `LINT-code-pmi_source_site_mismatch` | ported |
| `LINT-code-pmi_source_text_mismatch` | pending |
| `LINT-code-pocket_pattern_requirement_state` | pending |
| `LINT-code-unrecognised_defining_geometry-prismatic` | ported |
| `LINT-gear_coverage-data_table` | ported |
| `LINT-hole_coverage-second_countersink_unverifiable` | ported |
| `LINT-hole_coverage-thread_and_profile` | pending |
| `LINT-pocket_pattern_coverage-requirements` | pending |
| `LINT-schedule_evidence-verified_cells` | ported |

All other entries retain main verbatim.

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| adr.yaml | 5 | 68 | 61 | 26 |
| lint.yaml | 9 | 123 | 14 | 19 |
| issues.yaml | 23 | 90 | 51 | 26 |
| **All** | **37** | **281** | **126** | **71** |

## Code and baseline decisions

- Keep both requirement-preserving view reduction and main’s measured callout bands; reserve authored linear corridors alongside main’s precision-aware/aligned demands and stable tolerance anchors.
- Keep section material/hatch caches and refusal behavior alongside main’s section compaction and sheet allocation.
- Honor per-view and sheet hidden-line switches, including projection-cache invalidation. Apply visible precedence once to every non-section view, including iso. Remove the duplicate iso call; the iso overlap regression passes with one call. Extend the cache regression with alternating sheet defaults and iso overrides.
- An explicitly requested schedule can need more paper once all linear corridors are measured. If its first transaction fails and neither paper nor placements are pinned, retry larger automatic paper at the same scale before retaining the original dimensions. Keep all transactional PMI and tag checks. The semantic-PMI schedule regression now also checks that the Spec is unchanged and retained frames stay on paper.
- Use the lower defect count in every quality-baseline cell. Preserve main’s metadata exactly. Keep main’s plan-time budget unchanged.

## Validation

- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --release -- -D warnings`: pass.
- Focused release run with required corpus: `rules_catalogue` 6/6, `views_layout_rules` 12/12, `linework_spec` 6/6 pass. The final full run also covers the added iso cache override assertions.
- `tools/check_plan_time.py`: **PASS**; 2,611,567,805,125 retired instructions, limit 3,165,755,533,285, 0.949 of the recorded budget; 280.0 s wall time (final binary). All 63 parts measured. Budget unchanged.
- `tools/check_wasm_api.py --base-ref origin/main`: pass; generated declarations match the reviewed snapshot, so no API or changelog update is required.
- WASM release compilation and package generation: pass with `wasm-pack build --no-opt`; the default wasm-opt packaging step was blocked by sandbox permissions. No build configuration changed.
- WASM safeguard unit tests: 16/16 pass.
- Required-corpus `cargo test --release` initially stopped at the multi-sheet MCP wall-time gate (p90 576.784 ms versus 150 ms). The continuation exposed the semantic-PMI schedule regression, which is fixed above.
- Final release suite: **1,135 passed, 1 failed** across every rebuilt harness (including all 212 library tests). Built with `cargo test --release --no-run`, then ran the same harnesses in four parallel groups with `RUST_TEST_THREADS=8`, required corpus enabled and latency gates enabled. The zero-test CLI harness was also run separately. Documentation tests pass (0 tests).
- The **only final failure** is `multi_sheet_latency::r5_multi_sheet_latency`: MCP p90 **580.832 ms > 150 ms** (p50 173.398 ms, max 676.869 ms). Native p90 **71.065 ms** passes. Neither the latency tests nor their thresholds were changed.
- Final focused counts within that suite: catalogue **6/6**, views/layout **12/12**, line-work Spec **6/6**, planned-hole-table **27/27**, stateless **13/13**, session **8/8**. Both additional batch unit regressions pass among the 212 library tests.
