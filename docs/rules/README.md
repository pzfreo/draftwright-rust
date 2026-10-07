# Rules catalogue

This catalogue lists what the Python draftwright learnt, in drafting terms rather than code, so the
port can track what it owes ([plan](../plan.md); ADR R4 will cite it). It was extracted from
pzfreo/draftwright at `586628b96db6779454d499c7d92d37f1aa8f1df9`.

| File | Source | Entries | Drafting | Trust | Machinery |
| --- | --- | --- | --- | --- | --- |
| `adr.yaml` | live ADRs 1–5 (invariants, decisions, unguarded rules) and archived ADRs (rules not already live) | 160 | 57 | 61 | 42 |
| `lint.yaml` | `linting/`: per-feature coverage requirements and every issue code | 165 | 121 | 33 | 11 |
| `issues.yaml` | the 142 issue-numbered regression test files (`test_issue_*.py`) | 190 | 93 | 85 | 12 |
| **All** | | **515** | **271** | **179** | **65** |

## An entry

```yaml
- id: ADR5-1                      # ADR<n>-<k>, ADR<n>-D<k> (decision), ADR<n>-U<k> (unguarded), ARCH<nnnn>-<k>,
                                  # LINT-<module>-<slug>, ISSUE-<n>-<letter>
  statement: "Same input, same sheet, every platform."
  detail: "…"                     # optional
  class: trust                    # drafting | trust | machinery
  source: { adr: "…", lint_code: "…", module: "…", issues: [1196], tests: ["…"], corpus_parts: ["…"] }
  status: pending                 # pending | ported | translated | retired
  port_note: "Keep: …"            # the extractor's suggestion
```

- **drafting:** what the sheet must say, show or avoid. These are ported as checks on the plan
  or the rendered sheet, or as scenario tests (a STEP and a spec in, a property of the plan or
  its problems out).
- **trust:** determinism, honest failure, provenance, lint independence and reporting. These are
  kept.
- **machinery:** rules about the Python compiler's own structure. These are retired, or
  translated into the Analysis/Spec/Plan model.

When a rule is ported, its status becomes `ported` and it gains `rust_test:`, the test that
checks it. When it is translated, it gains `rust_rule:`, the rule it became. When it is
retired, it gains `why:`.

## Review

The extraction was done by agents reading the Python. Every entry cites its source, and the
citations were checked against origin/main: tests exist, and IDs are unique. The
classifications and port notes are judgement calls, worth a maintainer's look, especially:
- **The 65 machinery entries:** "translate" or "retire" decides what the new model must still
  guarantee.
- **Edit-surface rules** (`ADR4-1`, `ADR4-5`, `ADR4-14`, `ADR2-7`), marked machinery/translate.
  They carry straight into the Spec's schema, and could be a class of their own.
- **`ADR5-9`**, "serialises no identity meant to survive another run". It is read as forbidding
  recognition and topology identities. The Spec's draftwright-owned IDs are allowed within one
  file. Across files the Spec carries quiddity's fingerprints (ADR R3), never face indices.
- **Recognition lessons:** those about not claiming an unproven feature are classed trust; those
  about how a recognised feature is dimensioned are classed drafting.
