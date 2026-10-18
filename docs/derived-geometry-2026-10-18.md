# Derived PMI geometry, 2026-10-18

Draftwright side of [#99](https://github.com/pzfreo/draftwright-rust/issues/99).
Both kernel dependencies consume merge commit
`6ab5f29df418a06ee26e86ce1f7b010b653e1ade` through Cargo. No kernel checkout was
edited or vendored. Return the pins to a tag at the next quiddity release.
The unchanged release-pin comparison reports the difference from specify-core-rust's
published pin as a warning; `artifacts/derived-dw/release-pin.log` retains it.

The consumer follows the kernel's `Feature::Derived.geometry` links into
`PartPmi.derived_geometry` (PMI document format 2, reader `haecceity-pmi-read/7`).
It keeps the axis/plane, optional cylinder radius, and deriving-face support in the
Analysis, transforming local coordinates with the part occurrence. A selected part
keeps its own coordinates. Supplemental points never select drawing witnesses.
Missing geometry and missing size radii remain refused with their reasons retained.

Plane pairs use normal separation; axis–plane pairs use perpendicular separation;
axis pairs use their closest points. Infinite-target witness stations are chosen near
the deriving faces. The existing projection and nominal-agreement checks still apply.
Tolerance frames use their resolved target's direction and a site on that target.
A complete chain of derived plane-to-plane profile lengths takes precedence over automatic
cumulative shoulder locations. A partial source chain stands beside the automatic baseline
rather than claiming a hidden local length. This keeps source dimensions on the sheet.
Authored nominal values, tolerance bands and MAX/MIN/AVG qualifiers survive lowering
and ownership by recognised feature dimensions. Lint independently compares drawn
requirements with the retained target geometry and checks the qualifier.

For the unattended drawing choice, resolved semantic geometry takes precedence over
supplemental presentation points, while unsupported face-only bore witnesses keep their
existing refusal. No baseline or acceptance threshold is relaxed.

The unchanged GRM-03 fixture contains ten semantic nominal dimensions and Maximum on
#914; it contains no per-dimension semantic deviations or limit pairs. Separate planted
band regressions verify tolerance preservation on both absorbed and standalone dimensions.
The original fixture's decompressed SHA-256 is
`4b6462b9cc9f0d419250933bd77fb305f9cfebb7ec2b3f377008732876010a21`.
The new fixture has independent kernel-contract regressions rather than a Python
comparison capture; every existing Python comparison fixture remains required. The
full repeated-span and planning-cost survey inventories expand from 43 + 22 to
43 + 23 fixtures. The planning instruction budget and its margin are unchanged.

## Drawing and plan evidence

Before PNGs and complete plans for all 23 PMI fixtures are retained in
`artifacts/derived-dw/`. The baseline executable was built from the initial source and
alpha.6 tag dependency before implementation. The two before sheets are GRM-03 A4,
5:1, and bracket_positions A2, 1:1.

GRM-03 changes from **0/10 semantic dimensions drawn (10 refused) to 10/10 drawn
(0 refused or dropped)**: diameters 4, 6, 10 MAX, 5 and 1.6 mm, and lengths
3.2, 0.5, 2, 3 and 20 mm. Recognised diameters retain their callouts and source claims;
the complete PMI length chain replaces the automatic cumulative shoulder locations.
The drawing has 14 dimensions instead of 13, with no duplicate geometric annotation for
any of the ten semantic sizes. Both sheets are A4 at 5:1. Inspection confirms readable
labels, short-length outside arrows and witnesses on the intended shoulders/cylinders.
Datum-leader crossing warnings remain visible (3 before, 7 after); these are not hidden
or re-baselined.

Bracket_positions is unchanged: the five location records #31343, #31359, #31375,
#31401 and #31417 still have no face-supported plane-to-bore axis witness. These are
direct face targets, not resolved derived aspects, so the kernel adds no axis/plane target
for them. Their bounding-box centre differences include unrelated transverse and axial
offsets; those centres cannot truthfully replace the missing witnesses. They remain
refused, with the existing reason. The complete plan, apart from its Analysis hash,
is identical; the before/after PNG raster is identical, on A2 at 1:1.

All **38** committed STEP fixtures carrying semantic dimensions/tolerances were compared:
the 23 PMI fixtures and 15 other committed fixtures (`extra-fixtures.json`). Only
`analysis_sha256` is excluded from the comparison. **31 plans are identical.** Every
changed fixture is listed here:

| Fixture | Reason |
| --- | --- |
| `GRM-03_thumbwheel_drive_screw_AP242_PMI` | Ten resolved PMI dimensions and source ownership; authored profile chain replaces cumulative lengths; MAX retained; annotation bands, table and datums reflow on the same sheet. |
| `hw_assembly_plate_pin_intent` | Remove the obsolete maintainer-question reference from one refusal; drawing unchanged. |
| `hw_every_kind` | Same reference cleanup on one refusal; drawing unchanged. |
| `nist_ftc_10_asme1_ap242-e2` | Same reference cleanup on six refusals; drawing unchanged. |
| `nist_stc_10_asme1_ap242-e2` | Same reference cleanup on nine refusals; drawing unchanged. |
| `assembly/assembly_pin_pmi.step.gz` | Same reference cleanup on one refusal; drawing unchanged. |
| `rule_pins/nist_ctc_05_asme1_ap242.stp.gz` | Same reference cleanup on four refusals; drawing unchanged. |

`plan-comparison.json` records every changed JSON path. There are no unexplained plan
changes, and no existing comparison capture or accepted difference was changed.

The four inspected PNGs are:

- `artifacts/derived-dw/GRM-03_thumbwheel_drive_screw_AP242_PMI-before.png`
- `artifacts/derived-dw/GRM-03_thumbwheel_drive_screw_AP242_PMI-after.png`
- `artifacts/derived-dw/bracket_positions-before.png`
- `artifacts/derived-dw/bracket_positions-after.png`

## Verification

All Rust and acceptance checks use `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus`, and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Check logs and PNGs are under
`artifacts/derived-dw/` (generated evidence, ignored by Git).

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Focused derived geometry and repaired guards | PASS: 7/7 |
| `tools/check_test_binaries.py`, source and compiled inventory | PASS: 8 integration binaries, 138 reachable Rust files, 1,260 tests with exact matching names |
| `cargo nextest run --release --workspace --ignore-default-filter --no-fail-fast --test-threads 8` | PASS: 1,514/1,514, 0 skipped; 595.322 s; includes all 1,201 cases in the four affected binaries and both native latency gates |
| `cargo test --release --workspace --doc` | PASS: 0 doc tests in both workspace crates |
| `tools/ci_acceptance.py --job mcp` | PASS: Python quality tests, acceptance tests, Node client tests and MCP scenarios; 11.353 s |
| `tools/quality.py` on `tools/quality_parts.txt` | PASS: 30 parts; no worse than the unchanged CI baseline |
| `tools/check_leader_routes.py` | PASS: 144 callouts on 30 parts; eight unchanged pre-existing runs remain |
| `tools/check_quality_table.py` | PASS: 30 parts, 300 CI metric cells, no part worse |
| Direct `ci_acceptance.py --check-results quality` comparison | PASS against `tools/ci-quality-baseline.json` |
| Release-pin comparison | WARNING: different published specify-core-rust pin; checker unchanged |

The exploratory affected-binary run executed all 1,201 cases: 1,198 passed and three
failed. Two test setup corrections registered the independent fixture and checked complete
rendered marks against the three exact pre-existing manufacturing refusals. The fingerprint
guard also correctly rejected an executable after source edits during that run. All three
checks passed after a frozen-source rebuild; the final workspace run rechecked every affected
binary as well as all remaining unit, corpus and latency targets. No existing comparison
fixture, assertion threshold, quality baseline, instruction budget or test exclusion changed.

The quality table's reference is an exact transcription of the ten metric columns in
`tools/ci-quality-baseline.json`, with its source SHA-256 retained in
`artifacts/derived-dw/ci-quality-reference.md`. Totals are off-sheet 0, repeated labels 7,
repeated measurements 0, view overlaps 0, title-block overlaps 0, ink findings 6,
repeated spans 0, lines through features 63, hidden-on-visible 0 and section-hidden 0.
The JSON baseline, leader-route baseline, assertion thresholds and budget files are unchanged.
Final PNGs were regenerated from the tested release binary; bracket_positions still has
an identical raster. Both code commits and this evidence remain local; nothing was pushed.
