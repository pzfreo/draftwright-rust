# Rules-text merge with main, 2026-10-16

Merged main at `717def3e` after reading both branches' placement and PMI commits.
Main's catalogue entries, notes and `port_note` tags are the base. Only the two
batch entries marked ported with real regression tests retain the batch status,
test and note. Pending batch blockers are appended only where main remains pending;
main's translated, retired and already ported verdicts keep their original notes.
All other 513 entries retain main's status. The table below lists the 29 entries
reviewed by this batch: two batch ports and 27 main verdicts. The other 486 entries
match main in full.

The source-text checks, display-precision regression, bore-row placement and overlap
diagnostics are preserved alongside main's placement clearance, pins, datum behavior
and manufacturing table provenance checks. The PMI code conflict was documentation;
both implementations and all tests remain.

| Entry | Batch status | Main status | Merged status | Winner |
| --- | --- | --- | --- | --- |
| ADR2-17 | pending | translated | translated | main |
| ADR3-7 | pending | translated | translated | main |
| ARCH0008-1 | translated | translated | translated | main |
| ARCH0009-3 | ported | translated | ported | batch |
| ARCH0009-4 | pending | translated | translated | main |
| ARCH0016-3 | pending | translated | translated | main |
| ARCH0017-2 | pending | translated | translated | main |
| LINT-hole_coverage-thread_and_profile | pending | pending | pending | main |
| LINT-coverage-examinable_declared_kinds | retired | retired | retired | main |
| LINT-pocket_pattern_coverage-requirements | pending | pending | pending | main |
| LINT-code-pocket_pattern_requirement_state | pending | pending | pending | main |
| LINT-code-interior_label_on_narrow_material | pending | pending | pending | main |
| LINT-structural-duck_typed_dispatch | translated | retired | retired | main |
| LINT-code-pmi_source_text_mismatch | pending | pending | pending | main |
| LINT-suggest-fix_snippets | translated | translated | translated | main |
| ISSUE-885-a | pending | pending | pending | main |
| ISSUE-885-b | pending | pending | pending | main |
| ISSUE-917-a | pending | ported | ported | main |
| ISSUE-958-a | pending | pending | pending | main |
| ISSUE-1246-a | pending | translated | translated | main |
| ISSUE-1298-a | pending | pending | pending | main |
| ISSUE-1332-a | ported | pending | ported | batch |
| ISSUE-1360-a | pending | pending | pending | main |
| ISSUE-1432-a | pending | pending | pending | main |
| ISSUE-1432-b | pending | translated | translated | main |
| ISSUE-1485-a | pending | pending | pending | main |
| ISSUE-1485-c | pending | pending | pending | main |
| ISSUE-1505-a | pending | pending | pending | main |
| ISSUE-1613-a | pending | translated | translated | main |

Blocker sentences appended: LINT-hole_coverage-thread_and_profile, LINT-pocket_pattern_coverage-requirements, LINT-code-pocket_pattern_requirement_state, LINT-code-interior_label_on_narrow_material, LINT-code-pmi_source_text_mismatch, ISSUE-885-a, ISSUE-885-b, ISSUE-958-a, ISSUE-1298-a, ISSUE-1360-a, ISSUE-1432-a, ISSUE-1485-a, ISSUE-1485-c, ISSUE-1505-a.

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| adr.yaml | 10 | 63 | 61 | 26 |
| lint.yaml | 17 | 114 | 15 | 19 |
| issues.yaml | 39 | 74 | 51 | 26 |
| All | 66 | 251 | 127 | 71 |

The quality baseline keeps main metadata and the lower finding count in each cell.
Cells improved relative to main: none.

Validation uses `DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus`
and `DRAFTWRIGHT_CORPUS_REQUIRED=1` for both release-test commands.

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --release -- -D warnings`: passed.
- Targeted `cargo test --release --test rules_catalogue --test rules_text --test pmi_coverage --test surface_texture`: **17 passed**, zero failed or ignored (6 catalogue, 3 rules-text, 4 PMI coverage, 4 surface-texture).
- All eight batch additions passed: the five integration regressions above and the three library regressions for overlap remedies, manufacturing source text/display precision and safe blocked-centre-row alternatives.
- Exact full required-corpus `cargo test --release`: **1,065 passed, zero failed or ignored**. All 104 integration targets, library and binary tests, and doc tests completed. Both `multi_sheet_latency::r5_multi_sheet_latency` and all eight session tests (including the plate, turned, dense and NIST benchmarks) passed with the original 150 ms gates enabled. No latency gate, threshold or fixture was changed. No continuation run or follow-up code fix was needed.

A separate field-by-field audit verified merge precedence for every catalogue entry,
all main `port_note` tags, and minimum baseline values with metadata from main.

Validation logs are retained locally under `target/rules-text-main-merge/`: `clippy.log`, `targeted.log` and `release.log`.
