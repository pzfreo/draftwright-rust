# Integration test binaries

Integration tests use eight explicit Cargo targets instead of one executable per
`tests/*.rs` file. Six target roots include the original files with `#[path]`:
file history, relative helper paths and fixture paths are retained. Function names
are unchanged; grouped test names gain their original file stem as a module prefix.
The two corpus-wide harness targets remain separate.

## Grouping

| Target | Original source modules |
| --- | --- |
| `analysis` | `api_analysis`, `assembly_parts`, `bridge`, `cli_native`, `facts`, `gears`, `gzip`, `identity`, `model`, `native`, `native_recesses`, `pmi`, `pmi_lowering`, `pmi_thread_knurl`, `recognition_corrections`, `test_support` |
| `dimensions` | `angled_steps`, `authored_dimensions`, `coverage_holes`, `datum_placement`, `dimension_rules`, `dimension_schemes`, `dimensions`, `dims_blends`, `dims_edges`, `dims_grooves_flats`, `dims_plates`, `dims_recesses`, `dims_slots`, `dims_spotface`, `dims_steps`, `dims_stock`, `dims_turned`, `fit_label_layout`, `hole_tables`, `planned_hole_tables`, `plate_coverage`, `pmi_angular`, `pmi_dimensions`, `precision`, `repeated_dimensions`, `tolerances` |
| `drawing` | `bands`, `centrelines`, `detail_views`, `fix_relayout`, `gdt_band`, `h5_chains`, `interior_placement`, `iso_nts`, `label_lanes`, `label_rows`, `layout`, `leader_rules`, `leaders`, `line_through_feature`, `linework`, `oriented_chamfers`, `page_escalation`, `pinned_callouts`, `pinned_offset`, `placement`, `placement_intent`, `pmi_labels`, `repeated_labels`, `sheet_fill`, `sheet_projection`, `views`, `views_layout_rules` |
| `documents` | `label_text`, `linework_spec`, `notes`, `output`, `pmi_drawings`, `png`, `render_caches`, `render_panics`, `sections`, `sheets`, `sheets_lint`, `sheets_render`, `sheets_xref`, `spec_gdt`, `spec_notes`, `surface_texture`, `tables`, `text`, `timezone`, `title_block`, `title_block_material` |
| `audits` | `completeness`, `completeness_rules`, `issue_regressions`, `items`, `lint`, `lint_coverage`, `lint_family_rest`, `lint_fidelity`, `lint_structural`, `ownership_rules`, `pmi_coverage`, `problem_wording`, `review_leftovers`, `rule_pins`, `rules_catalogue`, `rules_text`, `trust_rules` |
| `interface` | `editor_session`, `engine_version`, `mcp`, `mcp_efficiency`, `mcp_spec_schema`, `multi_sheet_latency`, `serve`, `session`, `sheets_mcp`, `stateless` |
| `issue_drafting` | `issue_drafting` (whole-binary corpus harness) |
| `issue_trust` | `issue_trust` (whole-binary corpus harness) |

## Running tests

```sh
export CARGO_BUILD_JOBS=12 RUST_TEST_THREADS=12
export DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus
export DRAFTWRIGHT_CORPUS_REQUIRED=1
cargo nextest run --release --test drawing -E 'test(/^oriented_chamfers::/)'
cargo test --release --test drawing oriented_chamfers::
cargo test --release --test drawing oriented_chamfers::a_test_name -- --exact
cargo test --release --test issue_drafting --test issue_trust
```

Use the grouping above to replace `--test old_file` with `--test group old_file::`
for Cargo's harness, or `--test group -E 'test(/^old_file::/)'` for nextest. An
`--exact` selector must include the entire module path. Historical validation
reports retain their recorded counts; these commands describe the current layout.

## Inventory guard

`autotests = false` makes new root files opt-in. Add a module to the appropriate
target root when adding a test source. Lint runs
`python3 tools/check_test_binaries.py`, which walks every explicit target's module
graph and rejects any unreachable `.rs` file anywhere under `tests/`, including
helper files. It handles ordinary modules, path modules, inline modules and the
existing `part_cases!` / `pmi_cases!` generators. A new generator must extend the
inventory reader and its regression checks.

After archiving, CI also runs:

```sh
cargo nextest list --release --ignore-default-filter --message-format json > nextest-list.json
python3 tools/check_test_binaries.py --nextest-list nextest-list.json
```

The second check compares exact per-target test-name sets inferred from source
with the compiled inventory, including the two harness targets. A count drop,
missing target, omitted macro case or replacement with an unrelated test fails.
There is no manually maintained count to forget to update.

## Process-local state

Each original module still declares its own helper modules. The caches and counters
in `common/cache.rs` are therefore still separate instances per original module,
with the original filename/content/recognition-mode and whole-Analysis cache keys.
`OnceLock` fixture/plan caches remain private to their original modules. Scratch
paths use their original area namespaces, plus process IDs and counters where
parallel calls need distinct files. Tests pass environment settings to child
commands rather than mutating the parent environment. Production memo keys include
the full inputs and are synchronized or thread-local. Shared-process libtest runs
exercise the grouped modules as well as nextest's per-test processes.

The `ci` profile still excludes only `issue_drafting` and `issue_trust`; the workflow
continues deriving those names from its default filter and running their archived
executables whole. Exact-name priorities and reservations now use qualified module
names; former binary-wide rules select the corresponding module within its group.

