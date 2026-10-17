# Rules catalogue

This catalogue lists what the Python draftwright learnt, in drafting terms rather than code, so the
port can track what it owes ([plan](../plan.md); ADR R4 will cite it). It was extracted from
pzfreo/draftwright at `586628b96db6779454d499c7d92d37f1aa8f1df9`.

| File | Source | Entries | Drafting | Trust | Spec | Machinery |
| --- | --- | --- | --- | --- | --- | --- |
| `adr.yaml` | 5 | 68 | 61 | 26 |
| `lint.yaml` | 7 | 125 | 14 | 19 |
| `issues.yaml` | 20 | 93 | 51 | 26 |
| **All** | **32** | **286** | **126** | **71** |

Status after the maintainer-approved triage of 2026-10-16, including the earlier Stage G/H
and K1b/K4/K5/K7 integrations, merged with PR #56 (datum placement) and PR #57
(K6, one owner per measurement), PR #61 (datum follow-up), the pins/lint batch
(nine rules ported with tests), and PR #64 (table-rule ports). The table-rule batch adds
verified schedule and manufacturing evidence, transactional hole-table guards and
drawing-consumer checks. The PMI/source and precision batch retains its six verified ports
(exact aligned PMI, source-site fidelity, extent bands and fits, display precision and structured
feature-note authority). The coverage/holes stream retains its nine ported rules (body-local
corridors, prismatic shoulders, countersink seats, compound double-D callouts, pattern
counts and balloon assignment). Automatic profile-angle placement remains pending after
failing the unchanged drawing-quality bar. The rules-text merge retains its two completed
ports and pending blockers ([merge report](text-main-merge-2026-10-16.md)).
The views-and-layout batch retains six verified ports: ARCH0004-3, ARCH0018-1,
ISSUE-563-b, ISSUE-1262-a, ISSUE-1530-a and ISSUE-1604-a. ISSUE-563-b includes the
maintainer-approved reservation of every linear corridor, permitting smaller scales.
Counts are computed from the merged YAML entries:

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| `adr.yaml` | 5 | 68 | 61 | 26 |
| `lint.yaml` | 7 | 125 | 14 | 19 |
| `issues.yaml` | 20 | 93 | 51 | 26 |
| **All** | **32** | **286** | **126** | **71** |

The triage covered all 190 rules then pending: 80 PORT, 98 TRANSLATE and 12 RETIRE.
The original PORT triage recorded value, size and stream in `port_note`. Of the TRANSLATE
group, seven rules already met in Rust are marked ported with verified test citations:
ARCH0015-1, LINT-_registry-structured_note_satisfaction, LINT-schedule_evidence-verified_cells,
LINT-turned_step_coverage-length_and_od, ISSUE-917-a, ISSUE-1215-f and ISSUE-1296-a.
At triage, the other 91 were translated into the Rust model; the 12 retirements carry the
approved reason.

PR #57 now supplies the K6 evidence that was unavailable at triage: ARCH0015-2 and
ISSUE-1154-a are ported, and ADR1-8, ADR4-U1 and ISSUE-1505-b gain evidence for their
implemented portions. ISSUE-1505-a retains K6's updated assessment and remains pending.
The table-rule batch keeps six verified ported entries, promoting four beyond main's triage:
LINT-code-manufacturing_reference_unresolved, ISSUE-1144-a, ISSUE-1202-a and ISSUE-1543-c.
The approved triage is otherwise retained, including ISSUE-883-a's translation. Test
citations name files and tests found in the merged checkout.

The gear batch ports all eight selected gear rules and promotes the earlier
`gear_semantics_missing` translation to its exact Python diagnostic. Evidence is in
`tests/gears.rs`, including the real 13-tooth wheel and a smooth declared blank.

The [pins/lint batch evidence](pins-lint-2026-10-16.md) records its regression pins and
implementation checks. The [main merge report](pins-lint-main-merge-2026-10-16.md) lists
retained batch statuses, main's catalogue decisions and merged validation results.
The [PMI/precision merge report](pmi-precision-main-merge-2026-10-16.md) records this batch’s
six retained ports, the entries inherited from main, final counts and required checks.
The [coverage/holes merge report](coverage-holes-main-merge-2026-10-16.md) records its nine
retained ports, main's remaining decisions, final counts and validation results.

The [fit-label integration report](pmi-precision-fit-main-merge-2026-10-16.md) records the
latest merge, composed precision/fit behavior, catalogue audit and validation.

## An entry

```yaml
- id: ADR5-1                      # ADR<n>-<k>, ADR<n>-D<k> (decision), ADR<n>-U<k> (unguarded), ARCH<nnnn>-<k>,
                                  # LINT-<module>-<slug>, ISSUE-<n>-<letter>
  statement: "Same input, same sheet, every platform."
  detail: "…"                     # optional
  class: trust                    # drafting | trust | spec | machinery
  source: { adr: "…", lint_code: "…", module: "…", issues: [1196], tests: ["…"], corpus_parts: ["…"] }
  status: pending                 # pending | ported | translated | retired
  port_note: "Keep: …"            # the extractor's suggestion
  owner: draftwright              # set when ported, for recognition lessons: quiddity | draftwright
```

- **drafting:** what the sheet must say, show or avoid. These are ported as checks on the plan
  or the rendered sheet, or as scenario tests (a STEP and a spec in, a property of the plan or
  its problems out).
- **trust:** determinism, honest failure, provenance, lint independence and reporting. These are
  kept.
- **spec:** rules the Spec and its edit operations must uphold. Examples: it carries no page
  geometry, edits address items by stable ID, and a view request is distinct from the planned
  view. These are guarded by the Spec's schema, `edit`'s validation and tests.
- **machinery:** rules about the Python compiler's own structure. These are retired, or
  translated into the Analysis/Spec/Plan model.

When a rule is ported, its status becomes `ported` and it gains `rust_test:`, the test that
checks it. A rule ported only in part stays `pending`, with `rust_test:` for the part that is
checked and `rust_note:` saying what is ported and what is not. When it is translated, it gains
`rust_rule:`, the rule it became, and `rust_note:` describing its Rust mechanism and limits,
with `rust_test:` when a test or test suite is named. When it is retired, it gains `why:`;
the 2026-10-16 triage also records the reason and dated maintainer approval in `rust_note:`.
A rule ported in full may also carry `rust_note:`, saying what the port does differently.
Multiple test citations use a semicolon-separated scalar, as in the existing catalogue.

## Review

The extraction was done by agents reading the Python. Every entry cites its source, and the
citations were checked against origin/main: tests exist, and IDs are unique. The
classifications and port notes are judgement calls. Settled with the maintainer (2026-10-07):
- **The `spec` class:** the edit-surface rules were moved out of machinery into it (16 rules:
  ADR2-D7, ADR2-11, ADR3-6, ADR4-D1, ADR4-D3, ADR4-D4, ADR4-1, ADR4-5, ADR4-11, ADR4-14,
  ARCH0011-1, ARCH0018-6, ARCH0018-7, ISSUE-1000-a, ISSUE-1350-a, ISSUE-1432-c).
- **`ADR5-9`**, "serialises no identity meant to survive another run", forbids recognition and
  topology identities. The Spec's draftwright-owned IDs are allowed within one file. Across
  revisions the Spec carries quiddity's fingerprints (ADR R3), never face indices.

- **Recognition lessons:** the split stands.
  - Lessons about not claiming an unproven feature are classed trust.
  - Lessons about how a recognised feature is dimensioned are classed drafting.
  - As each "don't claim it" lesson is ported, it gains `owner:`:
    - `quiddity` when recognition itself should refuse the feature, with its test in
      quiddity-rust;
    - `draftwright` when the drawing must say that recognition refused or was unsure, rather
      than guess.

H7 (2026-10-12) revived ADR2-17 and LINT-code-interior_label_on_narrow_material
with tests for the implemented portion; both were retired only because the port formerly
placed no interior labels. The 2026-10-16 triage translates ADR2-17's placement rule;
the independent narrow-material lint check remains pending.

The native recess-reader batch ports five pending recess rules and verifies four
previously bridge-only rules on recognition/3. See `tests/native_recesses.rs`.
