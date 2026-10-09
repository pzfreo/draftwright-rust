# Alpha 1: the plan and its gates

The maintainer's plan (2026-10-09): bring draftwright-rust to an alpha 1 with PMI and the quality
fixes; then get the quiddity-rust (haecceity) and specify-core-rust changes in; then cut an alpha 1
release of each; then carry on with draftwright-rust improvements.

Until every gate below is met, the rolling run dispatches **only alpha 1 work** in all three repos
(and fixes to it). Everything else in the backlog waits until the releases are cut.

## 1. draftwright-rust alpha 1

- **PMI, drawn from the part's AP242 semantic PMI** (haecceity's reader):
  - reading: done (stage 1); every `rust-wrong` entry in `known_pmi.json` (55 at the time of
    writing) fixed, or re-examined and given a settled verdict with evidence;
  - feature control frames and datum feature symbols: done (stage 2a);
  - **toleranced and basic dimensions from PMI** (stage 2b; Python's
    annotations/_pmi_dimensions.py);
  - **PMI coverage lint** (stage 3; linting/pmi_coverage.py): every PMI item is on the drawing
    or reported;
  - checked on NIST CTC/FTC and on specify-core-written files against Python draftwright's
    drawings, every difference with a verdict.
- **Quality**, measured with `tools/quality.py` on `tools/quality_parts.txt` (baseline in
  `docs/quality-2026-10-09.md`):
  - annotations off the sheet: **0**;
  - overlapping view pairs and views on the title block: **0**;
  - repeated dimension labels: each one remaining justified with a verdict (identical features
    or quantities draftwright also states separately), none unexplained;
  - ink problems: **at least 80% fewer** than the baseline's 506, with no annotation over a view
    and no leader crossing a view; what remains listed with verdicts.
- **Performance:** the maintainer's PR #1 merged (render cache and straight-piece boxes); `plan`
  back to fast (NIST FTC 06 under 50 ms natively, from about 490 ms), drawings unchanged.
- **Release hygiene:** CI green; README states what alpha 1 does and does not do; a short
  CHANGELOG entry.

## 2. quiddity-rust (with haecceity) alpha 1

- haecceity's PMI writer, from the maintainer's decisions of 2026-10-09: no
  `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` for datum feature symbols (OpenCascade crashes on
  it); one `geometric_item_specific_usage` per face for multi-face sizes and tolerances; notes
  and surface texture in standard forms. Each verified by read-back and an OpenCascade
  cross-check.
- All 46 recognisers ported: section recesses wired into recognition.
- specify-core-rust's upstream needs (its docs/upstream-needs.md) done or explicitly deferred
  with a reason.
- CI green on Linux and macOS, each around 10 minutes.

## 3. specify-core-rust alpha 1

(Its own run, driven from the main session.)

- The maintainer's decisions merged: the move to `prototype/rust-port`, fingerprint schema 2,
  ISO 286 to 3150 mm, faceless items written around, knurl parameters.
- The new haecceity writer adopted: OpenCascade (and so specify-core's Python) opens every
  Rust-written file; multi-face tolerances cross; notes and surface finish written.
- CI green, including the Linux-only cross-engine test.

## 4. Cutting the releases

Done by the main session, in dependency order, once 1-3 are met:

1. quiddity-rust: version `0.1.0-alpha.1`, tag `v0.1.0-alpha.1`.
2. draftwright-rust and specify-core-rust: pin quiddity-rust to that tag, CI green, then
   version `0.1.0-alpha.1` and tag `v0.1.0-alpha.1`.
3. GitHub releases with notes, drafted for the maintainer to approve before they are published.

## 5. After alpha 1

Back to draftwright-rust improvements from the backlog.
