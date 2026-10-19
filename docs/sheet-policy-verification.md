# Round 5 sheet-policy verification — 2026-10-19

Current `origin/main` is `51d8f9f5`, already merged by `71452a78`. The merge retains #107–#115, including compact manufacturing notes, derived PMI geometry and the final gutter pass. `CHANGELOG.md` has no branch diff against main. Nothing was pushed.

## Corrected drawing choices

`corpus/mfcadpp/10098.step` is one A4 sheet at 1:1, with its controlling profile in a 2:1 detail. Its existing hole leader was frozen when derived recess geometry obstructed it. The bounded joint solve now reroutes obstructed existing controls while preserving already clear callouts and all measurement facts. The old A1 choice was caused by rejected routes on smaller paper; the saved A1 page dimensions were correct. The residual native-opening advisory remains explicit.

`corpus/cadgenbench/threaded_connector_109.step` stays on one A3 sheet at 1:5. All measured controls, openings, text and dimension lanes are readable. Dense unannotated thread depiction remains an explicit advisory and cannot purchase larger paper by itself.

A4 is tried first; larger paper requires a smaller-paper fit or measured-readability refusal. A larger single sheet precedes extra sheets, and no census part gains sheets. Gutters run after paper/scale selection, preserve all scales and sheet counts, and reserve 2 mm of frame clearance. Frame constraints use complete ink rather than empty reserved bands, and infeasible reservations fall back to complete ink, so unused padding cannot freeze a feasible inward move. Shared table/note placement also reserves 2 mm outside its rulings, and table reservations use the same inset region as final placement. Cutting-plane strokes and letters belong to the marked source view, so their actual ink participates in gutters. Canonical label coordinates survive bare pins exactly when the leader geometry is unchanged. Inter-view reservations and authored/pinned constraints remain binding.

Both tapped-post fixtures remain one A3 sheet at 5:1. The separately supplied GRM-01 shank remains one A4 sheet at 1:1; its final native render is included below.

## Border audit

The existing structural review emits error `drawable_border_contact` for view, dimension, label, callout, hidden or thin ink on or within 0.5 mm of the drawable frame (the combined half-widths of the heaviest frame and content pens); intentional title-block attachment is exempt. It checks full ink bounds, including witnesses and arrows, without clipping at the frame. `tools/quality.py` runs this review for ordinary views too and rejects border errors even when an old baseline contains the same defect. The saved-plan census independently audits complete footprints on every physical sheet and fails on contacts.

[The saved regression evidence](sheet-policy-border-regressions.json) binds both reviewed drawings from `0c5bfb41` to their STEP and Plan hashes: `10098` fails with six thin strokes crossing the frame and the connector with one near-frame callout. The final census has zero border contacts. Tests cover all four borders, crossing and near-edge strokes, complete witness ink, the title-block exemption, gutter overrides, and empty reservations near the frame. No quality baseline or readability threshold was loosened.

## Census against main

Both sides use the same content-hash manifest: 227 drawable inputs and one identical refusal. Before is the frozen main executable; all after Plans and PNGs use the frozen Round 5 executable.

| Paper | Main | Round 5 |
| --- | ---: | ---: |
| A4 | 114 | 164 |
| A3 | 88 | 37 |
| A2 | 6 | 9 |
| A1 | 13 | 11 |
| A0 | 6 | 6 |
| Physical sheets | 227 | 227 |

No part changes sheet count. 67 drawings change paper, scale, sheet count or detail count; fully readable drawings: 154 → 183. Every remaining measured readability failure has an explicit refusal. No measurement facts, PMI/GDT claims or non-diagnostic finding identities regress. See [every paper/scale/count change](sheet-policy-changes.md), [all 227 rows](sheet-policy-results.csv), and [the generated summary](sheet-policy-summary.md).

## Every move to larger paper

Every after-PNG below was opened and inspected. The smaller main drawing has the measured failures stated here; larger paper clears those failures while retaining one sheet.

| Part | Main → Round 5 | Justification | PNGs |
| --- | --- | --- | --- |
| corpus/mfcadpp/10049.step | A4 1:1 / 1 / 0 → A3 2:1 / 1 / 0 | A3 2:1 makes the 1.5 mm opening readable; the A4 1:1 trial remains below the 2 mm printed-opening floor. | [before](../target/sheet-policy/round5/qualified/larger-renders/before-9b831e372dbe-10049.png) · [after](../target/sheet-policy/round5/qualified/larger-renders/after-9b831e372dbe-10049.png) |
| corpus/mfcadpp/10146.step | A3 2:1 / 1 / 0 → A2 2:1 / 1 / 0 | A2 2:1 restores the 19 mm pocket-location witness clearance; the A3 trial leaves only 3.6 mm. | [before](../target/sheet-policy/round5/qualified/larger-renders/before-8b6a5959e000-10146.png) · [after](../target/sheet-policy/round5/qualified/larger-renders/after-8b6a5959e000-10146.png) |
| corpus/mfcadpp_holdout/119.step | A4 1:1 / 1 / 0 → A3 2:1 / 1 / 0 | A3 2:1 clears A4’s short step, two text/arrow failures, witness-lane failure and near-frame ink. | [before](../target/sheet-policy/round5/qualified/larger-renders/before-3ede6a86f773-119.png) · [after](../target/sheet-policy/round5/qualified/larger-renders/after-3ede6a86f773-119.png) |
| fixtures/parts/boss_circle.step | A4 1:2 / 1 / 0 → A3 1:1 / 1 / 0 | A3 1:1 makes the six small holes and short step readable; A4 1:2 fails those measured floors. | [before](../target/sheet-policy/round5/qualified/larger-renders/before-e1f12e3a4fb8-boss_circle.png) · [after](../target/sheet-policy/round5/qualified/larger-renders/after-e1f12e3a4fb8-boss_circle.png) |

## Reported regressions and frame repairs: inspected PNGs

| Part | Result | PNG |
| --- | --- | --- |
| corpus/cadgenbench/threaded_connector_109.step | Retain A3 1:5: left callouts and the bottom diameter dimension clear the frame; unannotated thread density does not purchase larger paper. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-connector.png) |
| corpus/mfcadpp/10098.step | A4 1:1, with the controlling profile at 2:1; all dimensions, thin ink and the isometric clear the frame on one sheet. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-10098.png) |
| corpus/mfcadpp/10155.step | Retain A4 1:1: release empty reservation padding so the isometric and its caption move inward without changing scales. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-10155.png) |
| corpus/mfcadpp_holdout/196.step | Retain A4 1:1: complete-ink gutter constraints give the isometric frame clearance while preserving readable principal controls. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-196.png) |
| fixtures/pmi/step/nist_ctc_01_asme1_ap242-e1.stp.gz | Retain one A3 sheet at 1:5: the dimension strip has positive stroke clearance, with all controls readable. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-ctc01.png) |
| fixtures/samples/specify-sample-spindle.step.gz | Retain A4 1:1: cutting-plane letters and strokes clear the left frame, with the section and isometric on the same sheet. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-spindle.png) |
| external/GRM-01_shank.step | Retain one A4 sheet at 1:1: principal controls and section remain readable, and the orientation view clears the title. | [PNG](../target/sheet-policy/round5/qualified/key-renders/after-GRM-01.png) |

The [hash-bound visual index](sheet-policy-render-review.json) records each inspected image and decision. Native renders use 150 dpi and the original STEP basename.

## Local checks

| Check | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Core / dimensions / rendering / audits, release binaries | PASS: 279 / 332 / 505 / 159; 1,275 total |
| Unfiltered `cargo nextest list` + `tools/check_test_binaries.py` | PASS: 8 binaries, 142 reachable Rust files, 1,334 integration tests; docs match |
| `python3 -I tools/test_quality.py` | PASS: 42 tests |
| `tools/quality.py` on both reported parts | PASS: no border contacts or off-sheet ink |
| Paired 227-part census and complete-ink border gate | PASS |

Initial local runs exposed stale exact snapshots for table insets, frame-safe gutters and a resolved-problem count. They also caught a safe annotation band left outside its reservation: off-sheet blank padding had vetoed recording that band. The guard now uses the same clipped reservation as the final frame. All four binaries were then rerun in full on the final source. A native replay of all 228 inputs confirms that the final reservation fix changes metadata in three cases and leaves every drawing’s ink, paper, scales and measurement facts unchanged.

The full suite was not run. Round 5 requested these local checks; earlier release acceptance receipts are retained separately and are not claimed as rerun here. Logs and frozen native survey artifacts are under `target/sheet-policy/round5/qualified/`. Environment: `CARGO_BUILD_JOBS=6`, required corpus `/Users/paul/repos/quiddity/tests/corpus`.

Executable SHA-256:

- Main: `eb3a0bfe0529dc4f7e8bdd4e12e113054d3a9f0ccf8e9200453064aac0ea6f48`.
- Round 5: `41a42f3dd6ea05ea0841fc4eeb695316718be5343bb9a0524678b89f821317da`.

Implementation: `7c68bb5a`, `caa7ce6a`, `2456fcce`, exact gutter snapshots `506715ca`, and reservation metadata `7c5a3ec8`.
