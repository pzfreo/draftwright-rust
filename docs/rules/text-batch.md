# Text and feature rule batch, 2026-10-16

This records the batch before main's approved catalogue triage was merged. The
[current merge report](text-main-merge-2026-10-16.md) records the final catalogue
verdicts and validation against main.

The 29 assigned rules were reviewed against the Python catalogue revision
`586628b96db6779454d499c7d92d37f1aa8f1df9`, its cited regression tests and the Rust
Analysis, Spec, planning, output and lint implementations. The read-only Python
checkout is newer (`8ad716b6`); deleted or changed source/tests were read with
`git show` at the catalogue revision. The YAML notes give the detailed remainder
and owner for each pending entry.

Batch outcomes: **2 ported, 3 translated, 1 retired, 23 pending**. The symbol test
pins an existing behavior under its translated rule. Pending includes the partial
PMI text implementation and the passing portions of the cross-solid probes; neither
is counted as a fully ported rule.

| Rule | Outcome | Test or concrete remainder |
| --- | --- | --- |
| ADR2-17 | pending | One joint interior/exterior candidate assignment and resource-floor/commit identity need a placement redesign; bounded H7 tests remain. |
| ADR3-7 | pending | Protected dimension-ownership stream: per-occurrence conversion ledger, absorption and remapping. |
| ARCH0008-1 | translated | `tests/rules_text.rs::callout_symbols_are_geometry_and_claims_are_semantic_roles`; semantic Analysis/Spec roles, display text in Plan, symbol strokes and searchable Unicode text. |
| ARCH0009-3 | ported | `tests/rules_text.rs::prismatic_central_bores_keep_their_row_and_turned_bores_clear_axis_ink` (three blocks and two round/turned controls). |
| ARCH0009-4 | pending | Shared strip keep-out intervals and a proved farthest-edge fallback; GD&T placement belongs to its own stream. |
| ARCH0016-3 | pending | Protected dimension-ownership stream: compound heads, independently addressable dependents and validation. |
| ARCH0017-2 | pending | Protected dimension-ownership stream: independent manufacturing requirement cardinality. |
| LINT-hole_coverage-thread_and_profile | pending | Across-flats is checked; independent thread-depth requirement/head identity is protected ownership work. |
| LINT-coverage-examinable_declared_kinds | retired | Rust has no replacement-geometric-feature declaration API or declared-feature truth-check roster. |
| LINT-pocket_pattern_coverage-requirements | pending | G7b recess-reader must supply patterns and original members; requirement ownership follows. |
| LINT-code-pocket_pattern_requirement_state | pending | No pocket-pattern denominator or author-suppression provenance reaches the coverage ledger. |
| LINT-code-interior_label_on_narrow_material | pending | Independent connected-band/curved-material lint needs a signed filled-material output field. |
| LINT-structural-duck_typed_dispatch | translated | Typed `output::Kind`/`Drawn`/`DrawnView` replace attribute probes and repeated geometry queries; captured structural-input parity test. |
| LINT-code-pmi_source_text_mismatch | pending | New source-word/finish-parameter checks pass; independent filled-glyph-path proof needs output binding evidence. |
| LINT-suggest-fix_snippets | translated | `LintIssue.fix` advice and `Problem.fix` typed `items::Op` edits; `tests/items.rs::a_problems_fix_resolves_it`. |
| ISSUE-885-a | pending | Pads are drawn; G7b pockets and protected footprint/location ownership remain. |
| ISSUE-885-b | pending | Protected removal locality, sparse-recognition denominator and slot/pocket supersession. |
| ISSUE-917-a | pending | Legacy independent-wall/base tests exist; native recess reader and physical chain ownership remain. |
| ISSUE-958-a | pending | The same two-channel compound yields one or two native channels depending on child order; partial slot/pad/pocket/no-false-feature tests pass. |
| ISSUE-1246-a | pending | G7b prismatic-recess evidence, specific unsupported-pocket disposition and independent unrelated-profile accounting. |
| ISSUE-1298-a | pending | Protected boss-owned thread/knurl attachment; source-preserving table-to-full-suffix fallback. |
| ISSUE-1332-a | ported | `src/dims/placement.rs::tests::an_overlapping_pair_has_one_diagnostic_with_a_consistent_remedy`; existing rendered-sheet remedy test. |
| ISSUE-1360-a | pending | Protected addressable tap-depth dimension and head/dependent contract; Spec thread length already prints. |
| ISSUE-1432-a | pending | Protected pattern-member disposition and rigid-motion contract; rim callout test already exists. |
| ISSUE-1432-b | pending | Full passage/provider-witness validation and protected occurrence/provenance ledger must fail both requirements closed. |
| ISSUE-1485-a | pending | G7b repeated curved-pocket evidence and original-occurrence grouping; upstream coordinate precision limitation remains. |
| ISSUE-1485-c | pending | G7b rounded-pocket patterns; protected individual-corner/blend and pitch/location ownership. |
| ISSUE-1505-a | pending | Protected OD reuse ownership must account for tolerance/decoration. |
| ISSUE-1613-a | pending | Protected seat/coaxial requirement identities; G6 axis furniture and G7b native recess evidence. |

## Implemented behavior and test evidence

An overlapping pair with line-work also crossing a label now produces one planner
`annotation_overlap`. Its message says that moving only text cannot fix the line-work.
The former extra `annotation_ink_overlap` for that pair is gone. Non-overlapping labels
retain ink-crossing diagnostics; ordinary overlaps retain their ordinary message.
Both annotation orders are tested. Planner problems offer no invented executable repair;
the sheet lint retains its existing consistent advice.

Source-scoped edge notes now compare their own words, independently of the note formatter,
allowing numbering and whitespace changes from wrapping. Face finishes compare each named
parameter and its numeric value in micrometres against the source's structured fields. The
independent check accepts the exact source value or its declared three-significant-digit
presentation, with no interval of nearby values. Valid `0.12` values survive unit conversion,
and `1.016` correctly prints as `1.02`; corrupt nearby values such as `0.8001` for `0.8` fail.
An unrelated annotation cannot rescue a corrupted source-bearing annotation. The source
IDs stay intact during corruption tests. This is deliberately a partial port: the invisible
searchable text runs do not independently prove that the filled glyph paths say the same
thing. Datum, GD&T, detail-view and measurement-ownership implementations were unchanged.

A central bore in prismatic stock now prefers horizontal rim-normal interior candidates whose
label midpoint stays within the 3 mm font height of the view centre. Candidate generation
retains the 64-alternative bound and all existing material, feature, furniture and peer-ink
checks. The row preference is ranked after clearance: if that row is blocked, another safe
interior route remains eligible. If no interior route fits, the exterior fallback remains. Turning profiles and analytic round stock are excluded, including plain cylinders
without native `turned_step` records. Their coaxial labels still clear the centre axis. The
new geometry regression failed before this change with a 10.266 mm row displacement. A separate
blocked-row regression failed with a hard row filter; the final policy ranks row-aligned
candidates after clearance and retains other safe interior alternatives within the same budget.

The new tests are:

- `src/dims/placement.rs::tests::an_overlapping_pair_has_one_diagnostic_with_a_consistent_remedy`
- `src/lint/pmi_coverage.rs::tests::manufacturing_source_text_is_checked_independently_of_its_identity`
- `tests/pmi_coverage.rs::a_source_edge_note_must_keep_every_word_when_wrapped`
- `tests/surface_texture.rs::a_face_finish_text_cannot_disagree_with_its_source`
- `src/dims/interior.rs::tests::a_blocked_centre_row_keeps_safe_interior_alternatives`
- `tests/rules_text.rs::prismatic_central_bores_keep_their_row_and_turned_bores_clear_axis_ink`
- `tests/rules_text.rs::callout_symbols_are_geometry_and_claims_are_semantic_roles`
- `tests/rules_text.rs::separate_solids_do_not_invent_features_and_preserve_real_slots_pads_and_pockets`

The overlap and manufacturing-source unit tests were run against the original implementations
and failed: respectively two diagnostics instead of one, and no diagnostic for missing source
words. They pass after the implementation changes. The same manufacturing-source regression also
failed on the valid conversion `0.00012 mm` → `Ra 0.12` with a raw-float comparison; the
independent display-precision check fixes that false positive and correctly accepts
`1.016` → `1.02`, while still rejecting nearby wrong values. Existing tests and quality thresholds were
not weakened. The full cross-solid channel contract was probed separately and left pending
because it fails upstream; the committed test names the passing subset it actually protects.

## Upstream cross-solid blocker

`tests/fixtures/parts/rules-text/make.py` builds the exact #958 Python solids. In particular,
`two-channels.step` contains channels at z=6..24 and z=66..84. Quiddity at pinned revision
`7cb26d8` publishes one channel in normal child order and two in reversed order. Its reader is outside this repository and native
Draftwright also does not yet consume its recess families. The full rule therefore stays
pending; reporting an unread family as absent would make a negative test vacuous. The raw evidence is
`target/rules-text-validation/two-channels.native.json` (one channel, three plates) and
`target/rules-text-validation/two-channels-reversed.native.json` (two channels, six plates).

Inspect the native evidence, including unread families, with:

```sh
cargo run --release --example recognition_document -- tests/fixtures/parts/rules-text/two-channels.step
cargo run --release --example recognition_document -- tests/fixtures/parts/rules-text/two-channels-reversed.step
```

The pending positive assertions are **two** native channels and equal physical geometry/order
regardless of compound child order. These expectations are not replaced with the order-dependent
observed results. The negative fixtures and the two real slots/pads/pockets exercise the raw
native document before checking any reader/planner behavior.

## Drawing and validation evidence

Before and after SVG, PNG, plan and render-report artifacts are in
`target/rules-text-row-before/` and `target/rules-text-row-after/`, respectively.
The before files were regenerated with an independently clean-built original parent
`53aa9f3f`; binary provenance is in
`target/rules-text-validation/render-parent-provenance.json`. All three before/after PNG
pairs were inspected as drawings: the short horizontal rim-normal leader makes the text
read on the bore row, with clear text, location dimensions and adjacent views. Sheet size,
scale, IDs, labels and measurement inventory are unchanged.

| Part stem (each has `.svg`, `.png`, `.plan.json`, `.render.json`) | Before row offset (mm) | After row offset (mm) |
| --- | ---: | ---: |
| `central-prismatic` | -10.266278 | -0.045 |
| `central-prismatic-wide` | -13.059755 | -0.045 |
| `central-prismatic-square` | -8.921441 | -0.063 |

The five bore STEP fixtures were regenerated with the committed generator and compared by
Boolean subtraction in both directions: every difference had zero volume. The round and
stepped-shaft controls assert an off-axis label midpoint and check the actual drawing for
foreign horizontal lines crossing the label. The blocked-row unit test preserves a clear
upward route and the 64-candidate limit.

Three corpus controls were also rendered with the original parent and current binary and
inspected: `cgb203`, `GRM-03_thumbwheel_drive_screw`, `nist_ctc_01_asme1_rd`. Their PNGs are
byte-identical before/after. Artifacts are in `target/rules-text-before/` and
`target/rules-text-after/`, with the same four suffixes. Comparison results are recorded in
`target/rules-text-validation/render-corpus-controls.json`. Their existing blade hidden-line
density and dense plate layout remain visible; this batch introduces no new contact there.

All checks used four build jobs and four Rust test threads. The corpus was required at
`/Users/paul/repos/quiddity/tests/corpus`. Checks ran one at a time; `--no-fail-fast` kept the
remaining release targets running after a latency assertion failed. Logs are under
`target/rules-text-validation/`.

| Check | Result | Evidence file |
| --- | --- | --- |
| `cargo fmt --check` | PASS | `fmt-final.log` |
| `cargo clippy --all-targets --release -- -D warnings` | PASS | `clippy-final.log` |
| Required-corpus `cargo test --release --no-fail-fast` | **FAIL: 1,005 passed, 4 latency assertions failed; 103 target results** | `tests-release-complete.log` |
| `tools/quality.py` on all 30 manifest parts | PASS | `quality.log`; drawings in `target/rules-text-quality/` |
| `tools/ci_acceptance.py --check-results quality` against original committed baseline | PASS: all 300 cells no worse | `quality-acceptance-original.log` |
| Same acceptance check after recording improvement | PASS | `quality-acceptance-recorded.log` |
| `python3 -I tools/check_leader_routes.py target/rules-text-quality` | PASS: 119 callouts, 30 parts; no new crossings or long runs; 14 unchanged pre-existing runs | `leader-routes.log`, `leader-routes.json` |

The measured baseline improvement is `string_post.hidden_on_visible: 26 → 25` (overall
`2232 → 2231`). All other checked cells are unchanged. It was already present in the original
parent, confirmed with `target/rules-text-parent-string-post/baseline.json`; it is an inherited
improvement against the older committed reference, not a new hidden-line rendering change in
this batch. `tools/ci-quality-baseline.json` records the measured values, source commit and
verified binary hash. No metric definition, threshold or single-sheet byte baseline changed.
Large quality sheets already have a minimum fit of A2 or larger; no A2+ sheet was introduced
just to enlarge a part that fits a smaller sheet at 1:1.

## Remaining validation blocker

All new rule tests and all other correctness tests pass. The full release command is not
green: four unchanged **150 ms p90** assertions failed:

| Gate in the complete release run | p90 (ms) |
| --- | ---: |
| `multi_sheet_latency::r5_multi_sheet_latency`, MCP path | 610.738 |
| `session::plate_benchmark_deltas` | 154.007 |
| `session::turned_benchmark_deltas` | 167.097 |
| `session::nist_benchmark_deltas` | 223.024 |

Direct R5 session edits passed at 66.139 ms p90. All functional delta, replay, plan-count and
byte-budget assertions reached before these timing checks passed. A same-four-thread recheck
of the two failed targets still failed: R5 direct/MCP p90 was 180.769/314.005 ms and NIST
282.678 ms, while plate and turned cases passed at 27.258/69.889 ms
(`latency-final-targets.log`). No latency gate was disabled and no threshold was relaxed.

The independently rebuilt original parent also fails: R5 direct/MCP p90
161.746/303.178 ms (`latency-parent.log`), and its full eight-test session target at four
threads fails NIST p90 191.527 ms (`session-parent-full-control.log`). The earlier isolated
parent NIST run passed at 79.501 ms (`session-parent.log`); results therefore demonstrate
variable timing, rather than proving every run of the parent fails. The first parent attempt
that reused the active target cache is retained as `latency-parent-cache-reuse.log` and is
**not** used as control evidence.

An all-green release result remains blocked by these pre-existing timing gates. Resolving
that performance/environment issue is separate work; no drawing, correctness check, planner
budget or metric was weakened to obtain a pass. The other unfinished work is the 23 pending
rules above, each with its concrete prerequisite and owner in the catalogue.

The unsettled symbol mechanism is resolved by retaining Python's geometric counterbore,
countersink and depth symbols with the existing searchable Unicode text layer. This symbol
choice changes no ink. Datum/GD&T placement, detail views and measurement ownership were not
changed. The opt-in single-sheet byte guard was left opt-in and was not re-recorded.
