# PMI/source and precision: main merge — 2026-10-16

Merged `origin/main` at `717def3ee87676400f8c1ca6a476fbb80fbbaa62` into the batch at `941b1d6c0f620c25a08c2d298ee00b471f6ac487`.

## Resolution

All 509 entries outside the six verified batch ports are copied verbatim from main. This keeps its approved retired and translated verdicts, tests, notes and `port_note` tags. The batch changed no pending entry, so there is no batch blocker sentence to append.

The six entries below retain the batch status (`ported`), test, note and owner where present; their `port_note` comes from main. Each named test definition exists in the merged checkout.

| Entry | Retained regression |
| --- | --- |
| LINT-code-pmi_source_site_mismatch | `tests/surface_texture.rs::a_finish_leader_must_stay_on_its_source_face` |
| ISSUE-1209-a | `tests/pmi_dimensions.rs::oblique_pmi_draws_its_exact_projected_station_span` |
| ISSUE-1215-a | `tests/issue_drafting.rs::envelope_wide_bands_and_extent_fits_reach_the_ink` |
| ISSUE-1349-a | `tests/precision.rs::requested_precision_reaches_extents_chains_ladders_and_locations` |
| ISSUE-1349-b | `tests/precision.rs::invalid_and_conflicting_precision_fail_closed_at_both_inputs` |
| ISSUE-1351-a | `tests/spec_notes.rs::structured_feature_notes_cover_only_explicit_roles_of_their_placed_owner` |

## Entries where main wins

Of the 509 entries inherited from main, 189 differ from the pre-merge branch (123 change status; the others retain status with updated evidence, notes or tags). All are listed below by main’s resulting status. The remaining 320 entries already matched main.

### Pending (61)

**adr.yaml:** `ADR1-8`, `ADR4-U1`, `ARCH0004-3`, `ARCH0009-2`, `ARCH0014-5`, `ARCH0015-3`, `ARCH0017-6`, `ARCH0018-1`, `ARCH0020-1`, `ARCH0020-2`.

**lint.yaml:** `LINT-hole_coverage-thread_and_profile`, `LINT-hole_coverage-second_countersink_unverifiable`, `LINT-code-feature_not_dimensioned`, `LINT-code-unrecognised_defining_geometry-prismatic`, `LINT-pocket_pattern_coverage-requirements`, `LINT-code-pocket_pattern_requirement_state`, `LINT-angular-profile_angle_requirement`, `LINT-gear_coverage-data_table`, `LINT-code-gear_requirement_mismatch`, `LINT-code-gear_axis_mismatch`, `LINT-code-gear_repeat_count_mismatch`, `LINT-code-gear_correspondence_unverifiable`, `LINT-code-interior_label_on_narrow_material`, `LINT-code-leader_shaft_unverified`, `LINT-code-detail_caption_clearance`, `LINT-code-pmi_source_text_mismatch`.

**issues.yaml:** `ISSUE-367-a`, `ISSUE-443-a`, `ISSUE-563-b`, `ISSUE-885-a`, `ISSUE-885-b`, `ISSUE-915-a`, `ISSUE-958-a`, `ISSUE-1058-a`, `ISSUE-1086-a`, `ISSUE-1086-b`, `ISSUE-1143-e`, `ISSUE-1144-b`, `ISSUE-1215-c`, `ISSUE-1217-a`, `ISSUE-1262-a`, `ISSUE-1276-a`, `ISSUE-1298-a`, `ISSUE-1298-b`, `ISSUE-1332-a`, `ISSUE-1357-c`, `ISSUE-1360-a`, `ISSUE-1370-a`, `ISSUE-1370-b`, `ISSUE-1370-c`, `ISSUE-1371-b`, `ISSUE-1374-a`, `ISSUE-1374-b`, `ISSUE-1374-d`, `ISSUE-1432-a`, `ISSUE-1485-a`, `ISSUE-1485-c`, `ISSUE-1505-a`, `ISSUE-1505-b`, `ISSUE-1530-a`, `ISSUE-1604-a`.

### Ported (27)

**adr.yaml:** `ADR2-U1`, `ADR3-8`, `ADR5-16`, `ARCH0009-5`, `ARCH0015-1`, `ARCH0015-2`.

**lint.yaml:** `LINT-_coverage_common-assembly_severity`, `LINT-_registry-structured_note_satisfaction`, `LINT-schedule_evidence-verified_cells`, `LINT-turned_step_coverage-length_and_od`, `LINT-code-annotation_overlap`, `LINT-code-annotation_ink_overlap`, `LINT-code-datum_leader_remote`, `LINT-code-manufacturing_reference_unresolved`.

**issues.yaml:** `ISSUE-827-a`, `ISSUE-917-a`, `ISSUE-1144-a`, `ISSUE-1146-b`, `ISSUE-1154-a`, `ISSUE-1155-a`, `ISSUE-1202-a`, `ISSUE-1215-f`, `ISSUE-1296-a`, `ISSUE-1334-a`, `ISSUE-1353-b`, `ISSUE-1543-c`, `ISSUE-1555-a`.

### Translated (89)

**adr.yaml:** `ADR2-D1`, `ADR2-D4`, `ADR2-D5`, `ADR2-2`, `ADR2-13`, `ADR2-17`, `ADR3-7`, `ADR5-1`, `ADR5-5`, `ADR5-6`, `ADR5-9`, `ADR5-17`, `ADR5-U1`, `ARCH0002-1`, `ARCH0002-2`, `ARCH0008-1`, `ARCH0009-1`, `ARCH0009-3`, `ARCH0009-4`, `ARCH0014-6`, `ARCH0016-3`, `ARCH0016-9`, `ARCH0017-1`, `ARCH0017-2`, `ARCH0017-3`, `ARCH0018-3`, `ARCH0018-6`, `ARCH0019-4`, `ARCH0019-5`.

**lint.yaml:** `LINT-_parameter_coverage-outcome_states`, `LINT-_parameter_coverage-report_uncovered_only`, `LINT-issues-lintissue_schema`, `LINT-requirements-source_families`, `LINT-hole_coverage-location`, `LINT-code-gear_semantics_missing`, `LINT-code-claimed_value_absent`, `LINT-code-claimed_measurement_not_compiled`, `LINT-code-pmi_not_rendered`, `LINT-pmi_coverage-stage_summary`, `LINT-quality-four_components`, `LINT-quality-fidelity_codes`, `LINT-quality-legibility_codes`, `LINT-orchestration-finished_drawing_order`, `LINT-suggest-fix_snippets`.

**issues.yaml:** `ISSUE-367-b`, `ISSUE-798-c`, `ISSUE-883-a`, `ISSUE-955-a`, `ISSUE-1006-a`, `ISSUE-1058-b`, `ISSUE-1073-a`, `ISSUE-1082-b`, `ISSUE-1126-a`, `ISSUE-1130-a`, `ISSUE-1132-a`, `ISSUE-1143-a`, `ISSUE-1143-b`, `ISSUE-1143-c`, `ISSUE-1146-a`, `ISSUE-1166-a`, `ISSUE-1166-b`, `ISSUE-1166-c`, `ISSUE-1187-a`, `ISSUE-1190-b`, `ISSUE-1204-a`, `ISSUE-1216-a`, `ISSUE-1216-b`, `ISSUE-1229-b`, `ISSUE-1245-a`, `ISSUE-1246-a`, `ISSUE-1260-a`, `ISSUE-1312-a`, `ISSUE-1325-a`, `ISSUE-1357-a`, `ISSUE-1369-a`, `ISSUE-1371-a`, `ISSUE-1392-a`, `ISSUE-1396-a`, `ISSUE-1432-b`, `ISSUE-1433-b`, `ISSUE-1460-a`, `ISSUE-1460-b`, `ISSUE-1460-c`, `ISSUE-1485-b`, `ISSUE-1511-a`, `ISSUE-1543-a`, `ISSUE-1543-b`, `ISSUE-1593-a`, `ISSUE-1613-a`.

### Retired (12)

**adr.yaml:** `ARCH0016-8`.

**lint.yaml:** `LINT-issues-pair_aggregation_ledger`, `LINT-_registry-requirement_carriers`, `LINT-coverage-coverage_state`, `LINT-coverage-examinable_declared_kinds`, `LINT-structural-duck_typed_dispatch`, `LINT-quality-unscored_and_stage_routed`.

**issues.yaml:** `ISSUE-676-a`, `ISSUE-1176-b`, `ISSUE-1308-b`, `ISSUE-1532-a`, `ISSUE-1544-a`.

## Final counts

| Catalogue | Pending | Ported | Translated | Retired | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| adr.yaml | 10 | 62 | 62 | 26 | 160 |
| lint.yaml | 16 | 115 | 15 | 19 | 165 |
| issues.yaml | 35 | 78 | 51 | 26 | 190 |
| All | 61 | 255 | 128 | 71 | 515 |

## Code and quality baseline

Read the commits introducing both sides of the code conflicts. Retained the batch’s source-site lint and main’s independent source-matched manufacturing-row checks. Rendering keeps both the precision metadata and main’s compiler-approved table claims. Auto-merged placement, datum follow-up, sheet selection and review changes are retained. Main’s new side-only callout reservation now also formats authored precision before measuring text, matching the final solver. The regression `dims::tests::side_only_callout_corridors_grow_with_authored_precision` checks that both left and right corridors grow for a 15-place nominal. A direct run of the existing symbolic-reference regression also reproduced an auto-merge interaction with main’s positive thread detection (`SEE MFG 10` became `SEE MFG 10.000`). Precision now excludes manufacturing-reference numbers explicitly while keeping main’s thread parser. The expanded existing regression checks reference-only text, thread-only text, and numeric nominals beside references. The initial full build was stopped after reproducing that failure; final validation runs on the corrected source.

Compared all 300 quality cells, taking the minimum per cell. Main already holds every better or equal value; the resulting JSON, including all metadata, is identical to main.

## Validation

All final checks passed. Commands sourced `$HOME/.cargo/env`; Cargo used
`CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`. Corpus checks used
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1`.
No latency gate was disabled or changed. `--no-fail-fast` collects all harness results without
waiving failures.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --release -- -D warnings` | PASS |
| Targeted catalogue, batch and interacting datum/table tests | PASS: 132 tests in 10 harnesses |
| `cargo test --release --lib` on the corrected source | PASS: 206 tests, including both integration regressions |
| Corpus-required `cargo test --release --no-fail-fast` on the corrected source | PASS: 1,071 passed, 0 failed, 0 ignored; 108 harnesses including doc tests |
| `tests/rules_catalogue.rs` in the final full run | PASS: all 6 tests |
| Final multi-sheet latency harness | PASS: its native and MCP gates remain enabled |
| Final session harness | PASS: all 8 tests, including the native latency gates |

The final full run includes every batch regression and all targeted interacting tests again.
The existing symbolic-reference regression was reproduced failing before the explicit reference
exclusion, then passed in both the final unit run and final full suite. No verification blocker
remains. Logs are local: `/tmp/pmi-merge-focused.log`, `/tmp/pmi-merge-final-{fmt,clippy,unit,full}.log`,
and `/tmp/pmi-merge-final-checks.json`. Commits remain local and have not been pushed.
