# Pins/lint merge with main, 2026-10-16

Merged `origin/main` at `8e5552e3` into the pins/lint batch at `c12d081e`.

The batch keeps its ported status, Rust test and Rust note for these nine entries; main's other fields, including triage `port_note` tags, are retained:

`ADR2-U1`, `ADR3-8`, `ADR5-16`, `ARCH0009-5`, `LINT-_coverage_common-assembly_severity`, `ISSUE-827-a`, `ISSUE-1155-a`, `ISSUE-1353-b`, `ISSUE-1555-a`.

Every other entry uses main's status and notes. Main keeps both batch remainders pending, with the batch's concrete blocker appended to main's Rust note:

`ARCH0020-1`, `LINT-code-feature_not_dimensioned`.

Main's changes win for 185 entries since the common ancestor, including 111 status changes.
The table below names each; the other entries already agreed with main. The nine retained
batch ports are excluded:

| Entry | Batch status | Main / final status |
| --- | --- | --- |
| `ADR1-8` | pending | pending |
| `ADR2-D1` | pending | translated |
| `ADR2-D4` | pending | translated |
| `ADR2-D5` | pending | translated |
| `ADR2-2` | pending | translated |
| `ADR2-13` | pending | translated |
| `ADR2-17` | pending | translated |
| `ADR3-7` | pending | translated |
| `ADR4-U1` | pending | pending |
| `ADR5-1` | pending | translated |
| `ADR5-5` | pending | translated |
| `ADR5-6` | pending | translated |
| `ADR5-9` | pending | translated |
| `ADR5-17` | pending | translated |
| `ADR5-U1` | pending | translated |
| `ARCH0002-1` | pending | translated |
| `ARCH0002-2` | pending | translated |
| `ARCH0004-3` | pending | pending |
| `ARCH0008-1` | pending | translated |
| `ARCH0009-1` | pending | translated |
| `ARCH0009-2` | pending | pending |
| `ARCH0009-3` | pending | translated |
| `ARCH0009-4` | pending | translated |
| `ARCH0014-5` | pending | pending |
| `ARCH0014-6` | pending | translated |
| `ARCH0015-1` | pending | ported |
| `ARCH0015-2` | pending | ported |
| `ARCH0015-3` | pending | pending |
| `ARCH0016-3` | pending | translated |
| `ARCH0016-8` | pending | retired |
| `ARCH0016-9` | pending | translated |
| `ARCH0017-1` | pending | translated |
| `ARCH0017-2` | pending | translated |
| `ARCH0017-3` | pending | translated |
| `ARCH0017-6` | pending | pending |
| `ARCH0018-1` | pending | pending |
| `ARCH0018-3` | pending | translated |
| `ARCH0018-6` | pending | translated |
| `ARCH0019-4` | pending | translated |
| `ARCH0019-5` | pending | translated |
| `ARCH0020-1` | pending | pending |
| `ARCH0020-2` | pending | pending |
| `LINT-_parameter_coverage-outcome_states` | pending | translated |
| `LINT-_parameter_coverage-report_uncovered_only` | pending | translated |
| `LINT-_registry-structured_note_satisfaction` | pending | ported |
| `LINT-schedule_evidence-verified_cells` | pending | ported |
| `LINT-issues-lintissue_schema` | pending | translated |
| `LINT-issues-pair_aggregation_ledger` | pending | retired |
| `LINT-_registry-requirement_carriers` | pending | retired |
| `LINT-requirements-source_families` | pending | translated |
| `LINT-hole_coverage-thread_and_profile` | pending | pending |
| `LINT-hole_coverage-location` | pending | translated |
| `LINT-hole_coverage-second_countersink_unverifiable` | pending | pending |
| `LINT-code-feature_not_dimensioned` | pending | pending |
| `LINT-code-unrecognised_defining_geometry-prismatic` | pending | pending |
| `LINT-coverage-coverage_state` | pending | retired |
| `LINT-coverage-examinable_declared_kinds` | pending | retired |
| `LINT-turned_step_coverage-length_and_od` | pending | ported |
| `LINT-pocket_pattern_coverage-requirements` | pending | pending |
| `LINT-code-pocket_pattern_requirement_state` | pending | pending |
| `LINT-angular-profile_angle_requirement` | pending | pending |
| `LINT-gear_coverage-data_table` | pending | pending |
| `LINT-code-gear_requirement_mismatch` | pending | pending |
| `LINT-code-gear_semantics_missing` | pending | translated |
| `LINT-code-gear_axis_mismatch` | pending | pending |
| `LINT-code-gear_repeat_count_mismatch` | pending | pending |
| `LINT-code-gear_correspondence_unverifiable` | pending | pending |
| `LINT-code-claimed_value_absent` | pending | translated |
| `LINT-code-claimed_measurement_not_compiled` | pending | translated |
| `LINT-code-annotation_ink_overlap` | ported | ported |
| `LINT-code-interior_label_on_narrow_material` | pending | pending |
| `LINT-code-leader_shaft_unverified` | pending | pending |
| `LINT-code-datum_leader_remote` | ported | ported |
| `LINT-code-detail_caption_clearance` | pending | pending |
| `LINT-structural-duck_typed_dispatch` | pending | retired |
| `LINT-code-pmi_not_rendered` | pending | translated |
| `LINT-code-pmi_source_text_mismatch` | pending | pending |
| `LINT-code-pmi_source_site_mismatch` | pending | pending |
| `LINT-code-manufacturing_reference_unresolved` | pending | pending |
| `LINT-pmi_coverage-stage_summary` | pending | translated |
| `LINT-quality-four_components` | pending | translated |
| `LINT-quality-fidelity_codes` | pending | translated |
| `LINT-quality-legibility_codes` | pending | translated |
| `LINT-quality-unscored_and_stage_routed` | pending | retired |
| `LINT-orchestration-finished_drawing_order` | pending | translated |
| `LINT-suggest-fix_snippets` | pending | translated |
| `ISSUE-367-a` | pending | pending |
| `ISSUE-367-b` | pending | translated |
| `ISSUE-443-a` | pending | pending |
| `ISSUE-563-b` | pending | pending |
| `ISSUE-676-a` | pending | retired |
| `ISSUE-798-c` | pending | translated |
| `ISSUE-883-a` | pending | translated |
| `ISSUE-885-a` | pending | pending |
| `ISSUE-885-b` | pending | pending |
| `ISSUE-915-a` | pending | pending |
| `ISSUE-917-a` | pending | ported |
| `ISSUE-955-a` | pending | translated |
| `ISSUE-958-a` | pending | pending |
| `ISSUE-1006-a` | pending | translated |
| `ISSUE-1058-a` | pending | pending |
| `ISSUE-1058-b` | pending | translated |
| `ISSUE-1073-a` | pending | translated |
| `ISSUE-1082-b` | pending | translated |
| `ISSUE-1086-a` | pending | pending |
| `ISSUE-1086-b` | pending | pending |
| `ISSUE-1126-a` | pending | translated |
| `ISSUE-1130-a` | pending | translated |
| `ISSUE-1132-a` | pending | translated |
| `ISSUE-1143-a` | pending | translated |
| `ISSUE-1143-b` | pending | translated |
| `ISSUE-1143-c` | pending | translated |
| `ISSUE-1143-e` | pending | pending |
| `ISSUE-1144-a` | pending | pending |
| `ISSUE-1144-b` | pending | pending |
| `ISSUE-1146-a` | pending | translated |
| `ISSUE-1146-b` | ported | ported |
| `ISSUE-1154-a` | pending | ported |
| `ISSUE-1166-a` | pending | translated |
| `ISSUE-1166-b` | pending | translated |
| `ISSUE-1166-c` | pending | translated |
| `ISSUE-1176-b` | pending | retired |
| `ISSUE-1187-a` | pending | translated |
| `ISSUE-1190-b` | pending | translated |
| `ISSUE-1202-a` | pending | translated |
| `ISSUE-1204-a` | pending | translated |
| `ISSUE-1209-a` | pending | pending |
| `ISSUE-1215-a` | pending | pending |
| `ISSUE-1215-c` | pending | pending |
| `ISSUE-1215-f` | pending | ported |
| `ISSUE-1216-a` | pending | translated |
| `ISSUE-1216-b` | pending | translated |
| `ISSUE-1217-a` | pending | pending |
| `ISSUE-1229-b` | pending | translated |
| `ISSUE-1245-a` | pending | translated |
| `ISSUE-1246-a` | pending | translated |
| `ISSUE-1260-a` | pending | translated |
| `ISSUE-1262-a` | pending | pending |
| `ISSUE-1276-a` | pending | pending |
| `ISSUE-1296-a` | pending | ported |
| `ISSUE-1298-a` | pending | pending |
| `ISSUE-1298-b` | pending | pending |
| `ISSUE-1308-b` | pending | retired |
| `ISSUE-1312-a` | pending | translated |
| `ISSUE-1325-a` | pending | translated |
| `ISSUE-1332-a` | pending | pending |
| `ISSUE-1334-a` | ported | ported |
| `ISSUE-1349-a` | pending | pending |
| `ISSUE-1349-b` | pending | pending |
| `ISSUE-1351-a` | pending | pending |
| `ISSUE-1357-a` | pending | translated |
| `ISSUE-1357-c` | pending | pending |
| `ISSUE-1360-a` | pending | pending |
| `ISSUE-1369-a` | pending | translated |
| `ISSUE-1370-a` | pending | pending |
| `ISSUE-1370-b` | pending | pending |
| `ISSUE-1370-c` | pending | pending |
| `ISSUE-1371-a` | pending | translated |
| `ISSUE-1371-b` | pending | pending |
| `ISSUE-1374-a` | pending | pending |
| `ISSUE-1374-b` | pending | pending |
| `ISSUE-1374-d` | pending | pending |
| `ISSUE-1392-a` | pending | translated |
| `ISSUE-1396-a` | pending | translated |
| `ISSUE-1432-a` | pending | pending |
| `ISSUE-1432-b` | pending | translated |
| `ISSUE-1433-b` | pending | translated |
| `ISSUE-1460-a` | pending | translated |
| `ISSUE-1460-b` | pending | translated |
| `ISSUE-1460-c` | pending | translated |
| `ISSUE-1485-a` | pending | pending |
| `ISSUE-1485-b` | pending | translated |
| `ISSUE-1485-c` | pending | pending |
| `ISSUE-1505-a` | pending | pending |
| `ISSUE-1505-b` | pending | pending |
| `ISSUE-1511-a` | pending | translated |
| `ISSUE-1530-a` | pending | pending |
| `ISSUE-1532-a` | pending | retired |
| `ISSUE-1543-a` | pending | translated |
| `ISSUE-1543-b` | pending | translated |
| `ISSUE-1543-c` | pending | pending |
| `ISSUE-1544-a` | pending | retired |
| `ISSUE-1593-a` | pending | translated |
| `ISSUE-1604-a` | pending | pending |
| `ISSUE-1613-a` | pending | translated |

Counts recomputed from the merged YAML:

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| adr.yaml | 10 | 62 | 62 | 26 |
| lint.yaml | 18 | 113 | 15 | 19 |
| issues.yaml | 42 | 70 | 52 | 26 |
| All | 70 | 245 | 129 | 71 |

Quality baseline: the minimum (better) value is retained per part/metric; metadata is exactly main's. Both branches already agree on `string_post.hidden_on_visible = 25`.

Code and tests merged automatically. Both branches' commit messages and overlapping diffs were reviewed: the merged dimension planner retains occurrence dispositions and side-constrained callouts alongside K6 correlated sets/authored parity; the lint collector retains assembly coverage severity alongside reference checks; trust tests retain both provenance and correlated-set coverage. Main's datum placement and search guards remain intact.

Both release-test runs set `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus`
and `DRAFTWRIGHT_CORPUS_REQUIRED=1`.

Validation:

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- Required-corpus targeted release run (library plus `rules_catalogue`, `rule_pins`, `completeness` and `trust_rules` (now modules of `--test audits`)): 242 passed, zero failed or ignored. This includes all 11 tests added by the batch, all six catalogue checks/scenarios, main's correlated-set trust checks and the existing library regressions.
- The exact required-corpus `cargo test --release` command stopped at `multi_sheet_latency::r5_multi_sheet_latency`: MCP p90 810.875 ms exceeded its unchanged 150 ms wall-clock limit (direct p90 90.583 ms). Its preceding targets had 637 tests pass. All 49 remaining integration targets passed under `--no-fail-fast` (393 tests), and doc tests passed (zero tests). Together the full run and its continuation covered all 102 integration targets plus library/binary tests: **1030 passed, 1 wall-clock failure, 0 ignored**. The session target, including `nist_benchmark_deltas`, passed. No latency gate, threshold or fixture was changed.

A field-by-field audit of all 515 catalogue entries confirms the requested merge precedence; the baseline audit confirms minimum values per cell and metadata from main.
