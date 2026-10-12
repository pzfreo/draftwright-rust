# Rules catalogue

This catalogue lists what the Python draftwright learnt, in drafting terms rather than code, so the
port can track what it owes ([plan](../plan.md); ADR R4 will cite it). It was extracted from
pzfreo/draftwright at `586628b96db6779454d499c7d92d37f1aa8f1df9`.

| File | Source | Entries | Drafting | Trust | Spec | Machinery |
| --- | --- | --- | --- | --- | --- | --- |
| `adr.yaml` | live ADRs 1–5 (invariants, decisions, unguarded rules) and archived ADRs (rules not already live) | 160 | 57 | 61 | 13 | 29 |
| `lint.yaml` | `linting/`: per-feature coverage requirements and every issue code | 165 | 121 | 33 | 0 | 11 |
| `issues.yaml` | the 142 issue-numbered regression test files (`test_issue_*.py`) | 190 | 93 | 85 | 3 | 9 |
| **All** | | **515** | **271** | **179** | **16** | **49** |

Status, as brought up to date from the rules assessment of 2026-10-11:

| File | Pending | Ported | Translated | Retired |
| --- | --- | --- | --- | --- |
| `adr.yaml` | 55 | 46 | 33 | 26 |
| `lint.yaml` | 64 | 86 | 0 | 15 |
| `issues.yaml` | 117 | 46 | 6 | 21 |
| **All** | **236** | **178** | **39** | **62** |

The assessment rated each of the 353 rules then pending; 20 rules pending today were not rated.
Rules it found done but with no test to name, or with a real remainder, stay pending with a
`rust_note` saying so.

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
`rust_rule:`, the rule it became. When it is retired, it gains `why:`. A rule
ported in full may also carry `rust_note:`, saying what the port does differently.

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
