# Release sheet-policy verification — 2026-10-19

Current main was fetched again and remains `51d8f9f5`. Merge `71452a78` retains #107–#115: trimmed arc leaders and perpendicular witnesses, user-facing wording, projection alignment, drawing numbers, compact manufacturing labels and numbered notes, derived PMI geometry and datum separation, final view gutters, and table symbols. `CHANGELOG.md` has no branch diff against main. Nothing was pushed.

Automatic recovery searches clean readable single-sheet A4 through A0 before R5 overflow. One larger sheet beats multiple smaller sheets; low fill never purchases another sheet. Explicit paper, scale, sheet limits and assignments remain binding. Gutters run after sheet/scale selection and allocation, consume available slack, and preserve all scales and paper. The final pass records actual annotation reservations only when they clear the border, title and other views.

A compact-callout retry preserves the readability of a clean fitted input. Invalid initial estimates still recover against the established default routes. Regression snapshots bind exact crossing pairs and coordinates to current main and STEP hashes. Pinned crops retain their already qualified canonical frame; explicit dimension views take precedence over held carriers, and unchanged hole tables keep their clear canonical cells. The identity variants keep their existing A4 routes and all coordinates; FTC-09 keeps its frame and pinned schedule while its reserved band includes the existing height witness. Automatic details also retain the independently recovered whole-part main scale, paper and sheet count; a smaller parent drawing cannot win by moving its controls into a crop. Explicit-scale retries retain their legibility warning. No readability threshold, quality baseline, instruction budget or latency gate was lowered.

## Key review parts

| Part | Main | Release | Sheets |
| --- | --- | --- | ---: |
| string_post_tapped | A3 5:1 | A3 5:1 | 1 → 1 |
| hw_string_post_tapped_intent | A3 5:1 | A3 5:1 | 1 → 1 |
| GRM-01_shank | A3 2:1 | A4 1:1 | 1 → 1 |

Both tapped posts retain the section and isometric on sheet 1. Tests also replay zero and oversized view gutters without changing paper, main scale, individual view scales or sheet count. GRM-01 uses the maintainer-supplied STEP under `scratchpad/review-parts/`.

## Census

Both sides use the same immutable 227-part content manifest plus one identically refused input. Native STEP planning and rendering preserve basenames and drawing titles. GRM-01 is reviewed separately. Primary paper counts:

| Paper | Main | Release |
| --- | ---: | ---: |
| A4 | 114 | 151 |
| A3 | 88 | 48 |
| A2 | 6 | 10 |
| A1 | 13 | 12 |
| A0 | 6 | 6 |
| Physical sheets | 227 | 227 |

No part changes sheet count; none gains sheets. [Every paper/scale/count change](sheet-policy-changes.md), [all 227 rows with reasons](sheet-policy-results.csv), and [the summary](sheet-policy-summary.md) use current main as before. The paired audit finds no lost measurement facts, no lost PMI/GDT claims and no new non-diagnostic drawing-finding identities. Every remaining measured readability failure has an explicit refusal.

## Native visual review

Every changed-paper or changed-count part in the requested real-part families is rendered before/after at 150 dpi and inspected, plus both tapped posts and GRM-01. [The committed hash-bound review index](sheet-policy-render-review.json) records the decisions and original PNG paths. Unchanged-paper/scale GRM and spindle cases remain covered by the complete census.

| Pair | Decision | PNGs |
| --- | --- | --- |
| hw_string_post_tapped_intent | Keep A3 5:1: preserve the large readable whole-part drawing, section and isometric on one sheet. | [before](../target/sheet-policy/release/final/key-renders/before-hw_string_post_tapped_intent.png) / [after](../target/sheet-policy/release/final/key-renders/after-hw_string_post_tapped_intent.png) |
| GRM-01_shank | Keep A4 1:1: principal views and section fit with legible dimensions; orientation view clears the title. | [before](../target/sheet-policy/release/final/key-renders/before-GRM-01_shank.png) / [after](../target/sheet-policy/release/final/key-renders/after-GRM-01_shank.png) |
| 301cb3ce914f-threaded_connector_109 | Keep A2 1:2: the ordinary main separates the repeated connector profiles; all dimensions remain on one sheet. | [before](../target/sheet-policy/release/final/real-renders/before-301cb3ce914f-threaded_connector_109.png) / [after](../target/sheet-policy/release/final/real-renders/after-301cb3ce914f-threaded_connector_109.png) |
| 523a9621070b-turned-step-compound | Keep A4 1:1: both turned components, end views, controls and orientation views remain distinct and readable. | [before](../target/sheet-policy/release/final/real-renders/before-523a9621070b-turned-step-compound.png) / [after](../target/sheet-policy/release/final/real-renders/after-523a9621070b-turned-step-compound.png) |
| 4b7011b0b86c-nist_ftc_08_asme1_rc | Keep A2 1:2: principal views retain scale, the schedule clears the views and title, and the local controlling detail fits on the same sheet. | [before](../target/sheet-policy/release/final/real-renders/before-4b7011b0b86c-nist_ftc_08_asme1_rc.png) / [after](../target/sheet-policy/release/final/real-renders/after-4b7011b0b86c-nist_ftc_08_asme1_rc.png) |
| 971dca9ef243-turned | Keep A4 1:1: full-size main and clear exterior dimensions; no extra sheet or unnecessary crop. | [before](../target/sheet-policy/release/final/real-renders/before-971dca9ef243-turned.png) / [after](../target/sheet-policy/release/final/real-renders/after-971dca9ef243-turned.png) |
| string_post_tapped | Retain one A3 sheet at 5:1, including the section and isometric; a smaller-paper crop cannot shrink the independent main-view scale. | [before](../target/sheet-policy/release/final/key-renders/before-string_post_tapped.png) / [after](../target/sheet-policy/release/final/key-renders/after-string_post_tapped.png) |
| 59bbc09a3462-nist_ctc_05_asme1_ap242 | Keep A2 1:5: principal and section scales are unchanged; the two schedules, datum frames and orientation view remain distinct with clear title space. | [before](../target/sheet-policy/release/final/real-renders/before-59bbc09a3462-nist_ctc_05_asme1_ap242.png) / [after](../target/sheet-policy/release/final/real-renders/after-59bbc09a3462-nist_ctc_05_asme1_ap242.png) |
| bf3df8a4ece1-specify-sample-spindle | Keep A4 1:1: full-size spindle controls, through-hole section and isometric remain readable on one sheet, with separated dimension lanes. | [before](../target/sheet-policy/release/final/real-renders/before-bf3df8a4ece1-specify-sample-spindle.png) / [after](../target/sheet-policy/release/final/real-renders/after-bf3df8a4ece1-specify-sample-spindle.png) |
| profile-419 | Retain the complete A3 1:1 profile and readable controls; no circle-only crop is needed. | [before](../target/sheet-policy/release/final/supplemental-renders/before-419.png) / [after](../target/sheet-policy/release/final/supplemental-renders/after-419.png) |
| profile-10098 | Choose the fully readable A1 5:1 whole profile; both controlling openings remain measured in the principal views, without a refused aperture crop. | [before](../target/sheet-policy/release/final/supplemental-renders/before-10098.png) / [after](../target/sheet-policy/release/final/supplemental-renders/after-10098.png) |
| profile-10049 | Choose A3 2:1 with the independently recovered main scale; the narrow slot remains readable without shrinking the parent to add a crop. | [before](../target/sheet-policy/release/final/supplemental-renders/before-10049.png) / [after](../target/sheet-policy/release/final/supplemental-renders/after-10049.png) |

## Required checks

| Check | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| Core / dimensions / rendering / audits, release shared-process binaries | PASS: 279 / 332 / 505 / 158; 1,274 total |
| Unfiltered `cargo nextest list` and `tools/check_test_binaries.py` | PASS: eight integration binaries, 1,333 tests; docs match |
| `tools/ci_acceptance.py --job mcp` | PASS |
| `tools/quality.py` and both leader-route / quality-table checkers | PASS |
| `python3 -I tools/ci_acceptance.py --job quality-pins --pin-cell 1-1 --out target/acceptance` | PASS |

The quality-pins job invokes `tools/quality.py` on all 30 parts, checks the unchanged accepted quality baseline and leader routes, tests deliberate-regression rejection and label-lane locality, then runs cell 1-1. The separate status-table checker reads `target/acceptance/quality-pins/quality/baseline.json`. The full suite was not run.

Environment: `CARGO_BUILD_JOBS=6`, `RUST_TEST_THREADS=6` (rendering uses four workers), required corpus `/Users/paul/repos/quiddity/tests/corpus`. Logs, exact acceptance commands, executable hash and source hashes are under `target/sheet-policy/release/final/`; acceptance cell receipts are under `target/acceptance/`. Superseded interrupted checks remain under `target/sheet-policy/release/resume/`.

Executable SHA-256:

- Before: `eb3a0bfe0529dc4f7e8bdd4e12e113054d3a9f0ccf8e9200453064aac0ea6f48`.
- Release: `d5ef71966b6c5b98e447793f4740dfe43a8983ebc88eaa7020dd4b25970c3130`.

Implementation commits: `71452a78` (merge), `07a41464` (single-sheet policy), `1571557c` (compact-callout references and final reserved bands), `c9c797ee` (canonical pinned detail frames and explicit carriers), `52b9b942` (clear canonical schedule cells), and `6913df84` (independent main scale and explicit-scale diagnostics).
