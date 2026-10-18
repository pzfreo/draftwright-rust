# Default view gutters

Automatic final drawings prefer 12 mm of blank paper between adjacent annotation
bands, spending only slack on the selected sheet at the selected scales. Gutters
shrink continuously to zero when necessary. Projection origins stay aligned;
explicit `views/<name>/place/gap` stays an authored distance, including zero.
`sheet/view_gutter` overrides the preference (finite, non-negative paper mm);
zero reproduces the drawing without the final spacing pass.

The common counted sheet allocator applies spacing after recovery, measured
annotation packing, detail selection, relative/obstacle placement and multi-sheet
allocation. This covers sessions as well as the library, stateless API and
exports. Enabling the historical `layout::Request::view_gutters` flag would spend
its hard 6 mm floor during sheet selection, so the Python replay profiles retain
that flag and their recorded arithmetic. The new final pass cannot select a
sheet, change a scale, reroute a dimension or drop a view.

The solver translates complete view blocks and their settled annotation ink,
GD&T, line work, balloons and scale captions. It preserves projection alignment,
explicit relative placement groups, pinned annotation stations and the canonical
frames retained by annotation text edits. It shares constrained slack before
improving individual pairs, and tries lifting the row above the title block
before sacrificing a gutter. Frames and fixed sheet furniture remain obstacles.
An isometric uses its actual ink/caption band rather than reserving its empty
fitting rectangle, with 1 mm of clearance inside the frame for its ink.

## Comparison

Baseline: `9422945b691a798ef97b569a783ab8d51188c3b3` (`origin/main` at start).
The baseline binary was built before editing and retained under
`artifacts/view-gutters/draftwright-main`. Its ten projection-alignment hashes
match the original fixture exactly. The branch binary is retained alongside it, from implementation commit
`500ad83`. Binary SHA-256 values and the environment are recorded in
`artifacts/view-gutters/manifest.json`.

The table reports the minimum signed horizontal/vertical separating strip
between two bands on the same sheet, in paper mm. Both runs normalize the iso to
its actual outline plus caption; other views retain their annotation bands.
Negative means overlapping bands (the pre-existing caption overlap on 10038).
Every sheet/scale, individual view scale and view inventory is checked unchanged
by `tools/check_view_gutters.py`.

| Part | Sheet | Scale | Gap before → after (mm) |
| --- | --- | --- | ---: |
| cadgenbench_inputs/cgb203.step | A4 | 1:5 | 8.50 → 12.00 |
| cadgenbench_inputs/cgb207.step | A3 | 1:2 | 0.00 → 12.00 |
| cadgenbench_inputs/cgb241.step | A3 | 1:5 | 0.00 → 2.13 |
| cadgenbench_inputs/cgb245.step | A2 | 1:2 | 0.00 → 12.00 |
| gramel/GRM-03_thumbwheel_drive_screw.step | A3 | 10:1 | 1.21 → 10.40 |
| gramel/GRM-05_depth_lock_bolt.step | A3 | 5:1 | 0.00 → 5.30 |
| gramel/string_post.step | A4 | 5:1 | 0.00 → 13.30 |
| nist/nist_ctc_01_asme1_rd.stp | A3 | 1:5 | 0.00 → 7.50 |
| nist/nist_ctc_02_asme1_rc.stp | A0 | 1:5 | 0.00 → 12.00 |
| nist/nist_ctc_03_asme1_rc.stp | A3 | 1:5 | 0.00 → 12.00 |
| nist/nist_ctc_04_asme1_rd.stp | A1 | 1:5 | 0.00 → 12.00 |
| nist/nist_ctc_05_asme1_rd.stp | A2 | 1:5 | 0.00 → 12.00 |
| nist/nist_ftc_06_asme1_rd.stp | A1 | 1:2 | 0.00 → 12.00 |
| nist/nist_ftc_07_asme1_rd.stp | A2 | 1:2 | 0.00 → 12.00 |
| nist/nist_ftc_08_asme1_rc.stp | A1 | 1:2 | 0.00 → 12.00 |
| nist/nist_ftc_09_asme1_rd.stp | A1 | 1:1 | 0.00 → 12.00 |
| nist/nist_ftc_10_asme1_rb.stp | A1 | 1:1 | 0.00 → 12.00 |
| ttt_inputs/sm-hanger.step | A3 | 1:2 | 0.00 → 12.00 |
| mfcadpp/1000.step | A4 | 2:1 | 0.00 → 6.26 |
| mfcadpp/10000.step | A4 | 1:1 | 0.00 → 11.42 |
| mfcadpp/10007.step | A3 | 2:1 | 0.00 → 12.00 |
| mfcadpp/10020.step | A3 | 2:1 | 0.00 → 12.00 |
| mfcadpp/10033.step | A3 | 2:1 | 18.00 → 18.00 |
| mfcadpp/10038.step | A4 | 2:1 | -1.22 → 12.00 |
| mfcadpp/1004.step | A4 | 1:1 | 0.00 → 12.00 |
| mfcadpp/10046.step | A3 | 2:1 | 0.00 → 12.00 |
| mfcadpp/10047.step | A3 | 1:1 | 0.00 → 12.00 |
| mfcadpp/10049.step | A4 | 1:1 | 0.00 → 12.00 |
| mfcadpp/10060.step | A4 | 1:1 | 0.00 → 12.00 |
| mfcadpp/10063.step | A4 | 1:1 | 0.00 → 9.96 |

All 30 sheets/scales and all view scales/inventories are unchanged. Twenty-three
parts meet or exceed 12 mm; seven use smaller positive gutters.

## Constrained gutters

| Part | Gap (mm) | Limiting room at the retained sheet and scales |
| --- | ---: | --- |
| cgb241 | 2.13 | The aligned column, side and section annotation blocks share the horizontal row with the iso/note-block obstacles. |
| GRM-03 | 10.40 | The aligned end view reaches the upper frame; the iso and its caption need the lower frame clearance. |
| GRM-05 | 5.30 | Front and plan bands occupy 271.7 mm of the A3 sheet's 277 mm usable height. |
| NIST CTC-01 | 7.50 | The tall plan band reaches the upper frame while the aligned side view must clear the title block below. |
| 1000 | 6.26 | Iso, front and plan share A4's height; the two vertical gutters share the remaining room after caption clearance. |
| 10000 | 11.42 | The front/plan stack reaches the upper frame while the aligned side band clears the title block. |
| 10063 | 9.96 | Column, side and iso fill A4's width, including the iso's frame clearance; two horizontal gutters share the slack. |

The second title-block arrangement restores a full 12 mm on 10046: the row can
lift above the title instead of squeezing the side view against its left edge.

## Render review

Before/after PNGs were rendered and inspected for all eight parts below. The new
spacing separates the view bands without changing geometry scale, leaving the
frame, title block and fixed furniture clear. The high-enlargement GRM-03 and A4
1000 are tight: their caption-to-frame clearance takes precedence over 12 mm.

All paths are under `artifacts/view-gutters/`; each stem has both
`main/<stem>.png` and `branch/<stem>.png`:

- `nist_ctc_01_asme1_rd`: plate with dense dimension bands and radius table.
- `nist_ctc_04_asme1_rd`: large plate, section and long hole schedule.
- `nist_ftc_06_asme1_rd`: bracket with section and isometric.
- `GRM-03_thumbwheel_drive_screw`: turned part at 10:1.
- `string_post`: Gramel turned part with end view and section.
- `sm-hanger`: bent sheet-metal part.
- `1000`: small prismatic part on A4 at 2:1.
- `cgb245`: broad plate with hole schedule.

## Checks and reproduction

All Rust checks use:

```sh
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
export DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus
export DRAFTWRIGHT_CORPUS_REQUIRED=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --release -- -D warnings
cargo test --release --test rendering --test core
python3 tools/check_test_binaries.py
python3 tools/test_mcp_spec_outline.py
```

The full test suite was not run. Five new rendering regressions cover roomy first
and third angle layouts, unchanged sheets and scales, shrinking, explicit gaps,
override validation/round-trip and rigid annotation transport. Existing no-op pin,
editor fragment, detail, relative iso, section and multi-sheet assertions remain
active. Expected-position groups carry a one-line reason: final gutters follow
raw recovery, existing view pins now repeat the reported default gap, and the ten
alignment hashes include final spacing while retaining authored alignments.

```sh
python3 -I tools/quality.py artifacts/view-gutters/draftwright-main artifacts/view-gutters/main $(sed "s|^|$DRAFTWRIGHT_CORPUS/|" tools/quality_parts.txt)
python3 -I tools/quality.py artifacts/view-gutters/draftwright-branch artifacts/view-gutters/branch $(sed "s|^|$DRAFTWRIGHT_CORPUS/|" tools/quality_parts.txt)
python3 tools/check_view_gutters.py artifacts/view-gutters/main artifacts/view-gutters/branch artifacts/view-gutters/comparison.json
python3 tools/check_leader_routes.py artifacts/view-gutters/branch --report artifacts/view-gutters/branch-leaders.json
python3 tools/check_quality_table.py docs/status-2026-10-10.md artifacts/view-gutters/branch/baseline.json
```

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo test --release --test core --test rendering` | PASS: core 270/270 (52.79 s), rendering 465/465 (121.55 s) |
| `python3 tools/check_test_binaries.py` | PASS: 8 binaries, 140 reachable Rust files, 1,273 integration tests in source; docs counts updated |
| `python3 tools/test_mcp_spec_outline.py` | PASS: 3 tests; advertised schemas unchanged |
| `tools/quality.py`, main and branch | PASS: all 30 parts; zero off-sheet annotations, view overlaps or views on title block |
| `tools/check_view_gutters.py` | PASS: all sheet/view scales and inventories unchanged |
| `tools/check_leader_routes.py` | PASS: 144 callouts; no longer/new bad routes; 8 unchanged pre-existing runs |
| `tools/check_quality_table.py` | PASS: 450 cells, no part worse than the maintained status table |
| Direct main/branch quality comparison | PASS: all 450 tracked cells equal main |

The final quality totals remain: 7 repeated labels, 6 ink findings, 63 feature
crossings, 1 off-span text, and 18 leader/extension contacts. These are unchanged
from main; gutters do not reroute or remove annotations to hide existing findings.

Raw plan/analysis JSON, SVGs, the 16 PNGs, comparison JSON, binary copies and
check logs remain in the ignored `artifacts/view-gutters/` directory.
