# Authored detail sources and unions (2026-10-16)

The Spec remains `draftwright.spec/2`. An authored detail can now select its parent and
name several semantic regions:

```json
{"views":{"detail_c":{"detail":{"from":"side","around":["f:groove:1","f:turned_step:3"],"scale_factor":2}}}}
```

`from` is optional. It names an existing principal or section view without `v:`.
`around` remains required and accepts the existing string form or a nonempty list of
feature/face IDs. Face IDs use the existing `faces/<face>` vocabulary, including side
names and `face:<n>`. The author supplies no geometry coordinates.

Every member must resolve. An explicit parent must exist in the actual plan and each
member must have a finite, non-degenerate projected crop there. Errors identify the
member and parent. Visibility means projected crop area; it does not assert absence
of occlusion. Feature crop margins remain unchanged; faces use attachment bounds.
The union combines the members' model-space crops. A section detail retains its
source plane in the Plan, including when rendered independently of the parent.

When natural sources differ without `from`, the most visible members determine the
available parent. The chosen tie order is front, plan, side, then section. Basis text
records that choice and the visible count. A shared natural source and the original
single-feature form retain their established behavior.

The eight added regressions cover plan/side cameras, projected parent marker circles,
edge-on faces (including one invalid member in an otherwise visible union), missing
and omitted parents, exact union bounds, a groove and adjoining shoulder, mixed-source
selection independent of list order, unknown members, empty lists, saved string/list
Specs and drawing metadata, and a section detail compared with an explicitly cut
projection. The extension initially contained 18 detail tests; the revision adds constrained
main-scale geometry, complete X-head profiles and omitted-seed recovery regressions,
bringing that target to 21.

All 26 MCP schemas for 13 served tools validate independently and match the served
schemas. The initial extension introduced a typed description of the entire detail object
in twelve standalone Spec schemas; previously each view was an opaque object. That accounts
for the 5,376-byte growth from 42,356 to 47,732, rather than two fields alone. There are no
redundant copies within a standalone schema. The revision uses a disjoint string/array type
and an exact compact source pattern, preserving nulls, list item types, minimum length,
source choices and strict detail properties. The total is **46,868 bytes**, 864 fewer, with
a strict **<46,869** cap in both generator and Rust guard. Final revision evidence and the
current default drawing policy are in [detail verification](detail-verification.md).

## Inspected PNG examples

- [Plan-sourced hole](/private/tmp/detail-extension/examples/plan-hole.png): mounting
  plate, sheet 1:2, detail C 2.5:1, circle and letter on the plan.
- [Side-sourced groove/shoulder union](/private/tmp/detail-extension/examples/side-union.png):
  two turned shafts, sheet 1:2, detail C 1:1, circle and letter on the side. The union
  retains the existing feature crop context, including both groove projections.

Both PNGs were opened and visually inspected. The input Specs and extracted PNG Spec
are beside the images. Check logs and the 30-part results are in
`/private/tmp/detail-extension/`.

## Original extension checks

Commands loaded `$HOME/.cargo/env` and used `CARGO_BUILD_JOBS=4` and
`RUST_TEST_THREADS=4`. Corpus tests set
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`.

- `cargo fmt --all --check`: passed ([log](/private/tmp/detail-extension/fmt.log)).
- `cargo clippy --workspace --all-targets --release -- -D warnings`: passed
  ([log](/private/tmp/detail-extension/clippy-final.log)).
- Final detail target: 18 passed, including the independent section projection oracle
  ([log](/private/tmp/detail-extension/detail-final.log)).
- `tools/check_mcp_schemas.py`: passed for all 13 tools and 26 served schemas
  ([log](/private/tmp/detail-extension/mcp-schemas.log)).
- Both final-executable PNGs were visually inspected; the union names its adjacent
  lower shoulder (`f:turned_step:3`) and reopening its saved Spec reproduces identical
  pixels and metadata ([hashes](/private/tmp/detail-extension/render-hashes-final.json)).

The first exact `cargo test --release` invocation caught a stale engine fingerprint:
its build began before the final source edit. The fingerprint guard was left intact;
the stable-source completion run uses `cargo test --release --no-fail-fast` to exercise
every target. The initial result is retained in
[tests.log](/private/tmp/detail-extension/tests.log).

`tools/quality.py` completed all 30 manifest parts, and
`tools/ci_acceptance.py --check-results quality` passed all 300 per-part metric cells
against the unchanged `tools/ci-quality-baseline.json`. Totals versus that baseline:
ink overlaps 52 → 49, line-through-feature findings 73 → 69, repeated labels 7 → 6,
and hidden-on-visible strokes 2,231 → 0. All other gated totals remain zero.
See [quality results](/private/tmp/detail-extension/quality/baseline.json),
[quality log](/private/tmp/detail-extension/quality.log) and
[acceptance](/private/tmp/detail-extension/quality-acceptance.log). These improvements
were already present on the detail-view branch; the extension preserves them.

The stable-source run passes `engine_version_matches_source_and_invalidates_on_content_changes`;
the earlier fingerprint mismatch is resolved without changing the guard.
The saved union Spec extracted from the PNG also reproduces a byte-identical reopened PNG.

The unchanged latency checks pass: all eight native Session tests (including NIST),
the multi-sheet latency test, and the dense invalid-batch check. All 14 stateless tests,
including HTTP transport, pass in this environment. No latency gate was changed.

Final corpus-required completion run: **1,126 passed, 0 failed, 0 ignored** across 112
reported test targets, exit 0 ([full log](/private/tmp/detail-extension/tests-final.log)).
The corrected adjacent-shoulder regression was also rerun in the final 18-test detail
target, which passes. The complete machine-readable summary is
[final-checks.json](/private/tmp/detail-extension/final-checks.json).
