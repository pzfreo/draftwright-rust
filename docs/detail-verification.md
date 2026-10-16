# Detail views and turned inspection baseline — verification, 2026-10-16

## Merge onto main `53aa9f3f`

This verification supersedes the earlier implementation's check and delivery status below.
The merge combines detail commit `7e29f66b` with `origin/main` at `53aa9f3f`;
`git log 253b7601..origin/main` and the F1, K4 and K5 implementation/review changes were
read before resolving the conflicts. The original source trace and comparison artifacts are
retained below as historical evidence against base `253b7601`.

### Conflict resolutions

* `src/model.rs`: retained F1's outer `rule` loop, lazy `is_small` / `fits_a3_at_full_size`,
  invalid-large-seed handling and the fallback to large sheets only when A4/A3 have no valid
  drawing. Derived details are excluded from whole-principal envelope tests, including the
  full-size A3 test. Detail-aware demand fitting, `iso_factor_with_detail`, head-sheet retention
  and detail quality gates run within the new scale loop. Kept both the detail markers/captions
  and F1's thin manufacturing layer. F1's independent section lift and fixed GD&T corridors remain.
* `src/dims/leaders.rs`: kept K4's shared material re-entry predicate and depth, F1's
  grazing-angle guard, and the detail branch's extension clearance, shorter-route gates,
  opposite-silhouette candidates and protection against new text contacts. Refinement may
  increase neither the original hard/crossing scores nor the grazing score. Pinned detail
  callouts now participate in the same re-entry report as principal-view callouts.
* `src/dims/turned.rs`: kept the default common inspection end for a whole connected multi-step
  turned profile, with PMI first, otherwise the larger flat end (low end for a tie). Axial
  feature locations use that same end. K5's separate-body profile grouping, separating view,
  body silhouette witnesses and side strips remain intact; separate bodies do not acquire
  a shared whole-part datum. K5's groove ownership, pad, slot and lint rules are unchanged.

The Z-axis automatic head extension is restricted to whole turned parts: a connected pin on
an assembly plate must not introduce a head crop that displaces main's valid small-sheet
candidate. Automatic turned crops preserve established GD&T frame corridors and principal
scale; explicitly authored crops retain precedence. A refused optional crop is not by itself
an active detail quality veto. Thread/knurl ink now uses the detail's centre and clips to its
crop. Drawn frames record the equivalent whole-part-centre offset, so independent K4 lint
projects the same geometry as the drawing.

F1's toleranced string-post test used the identical saved spec but expected its pre-detail
5:1 mains. Its exact merged assertion is now A4 2:1 **and** a present head B at 10:1.
Every original hard-finding, tolerance-coverage and sheet-rule assertion remains. The GD&T
control still asserts A3 5:1. No test is disabled, no threshold relaxed, and no CI quality or
leader-route baseline is re-recorded. A new external-thread detail regression verifies
nonempty conventional ink, crop containment and the recorded world-to-page transform.

The full run exposed a genuine R5 integration failure: the optional head displaced the
inspection schedule added by F1's overflow fixture. Automatic crops now release their
reservations when required tables/notes cannot be placed. A declared schedule combined with
an explicit minimum main scale takes precedence over an automatic turned crop; authored
details keep precedence. The table, two overflow prefixes, complete dimension inventory,
three sheet steps, one plan per edit, wire limits and 150 ms timing assertions remain intact.

Stateless reconstruction then exposed repeated placement and table-metric work. Bounded
process memos retain exact sheet candidates (64), dimension placements (64), and fixed-font
cell ink spans (128). Candidate keys cover the complete Analysis and Spec. Placement keys
cover the complete Analysis, physical sheet/view geometry, spec intent, held ink, obstacles
and exclusions; only unread title cell text and explanatory metadata are omitted. The actual
title-block rectangle stays in the key. Results are owned clones; edited PMI or geometry
invalidates reuse. Table metrics use exact outlines, unchanged sums and fixed font/height.
The allocator/recovery still run and check every prefix; no planner counter is fabricated.
Shared-cache locks only update reference counts and queues; large clones, replacements and
evictions run outside the locks. A re-entrant clone/drop probe enforces that rule. Recovery
reuses the complete Analysis digest already computed in the call instead of serializing the
same Analysis once per overflow prefix.
Warm persisted projections also track changes to binding, projected views, cuts and placed
geometry, so unchanged caches avoid a redundant 3.3 MB serialization/read on R5 title edits.
Successful projection-file parses use a four-entry content-keyed memo for inputs up to 8 MiB.
Every file byte remains in its SHA-256 key; changed or malformed files are parsed again.
Returned caches are owned clones with zero restored work counters and clean persisted state.
The regression also checks that editing a loaded cache cannot mutate a later reload.
The disk format and durability guarantees remain unchanged. Independent immutable Spec, delta
and history files flush concurrently, with `sync_all` on each; only after all succeed is the
head published. A failure-path regression obstructs the artifact directory, verifies that the
parent remains live, removes the obstruction and successfully retries. A cache regression verifies clean
reconstruction/title edits, dirty scale edits, exact cache bytes and fresh-render parity.

The 150 ms session and R5 wall-clock gates, operation counts, complete delta replay, HTTP
limits and cold/warm/fresh-process parity assertions remain unchanged. Diagnostic timing
runs exposed intermittent waits: one NIST edit took 338.012 ms wall time but only 20.195 ms
of its thread's CPU time; an isolated NIST run passed at 48.913 ms. Temporary instruction,
CPU and stage traces were removed from the final source. The final required full-suite result
is recorded below. A keep-awake experiment did not eliminate the waits, and moving
the process out of background priority was not permitted, so no scheduling promotion is used.
Session now retains one exact assembled preview drawing, keyed by its Spec and PMI. The
session's Part, font and recognized geometry are immutable; PMI is the only Analysis field
its editing API replaces. Every evaluation still runs the planner, geometry placement,
annotation work and delta calculation. Only identical assembled drawing ink is reused.
Existing preview/commit, undo/redo, changed-PMI and fresh-render parity tests enforce this.
The final-source focused run passes R5 (34.219 ms session p90, 78.962 ms MCP p90), all eight
session cases (NIST p90 44.629 ms), and all fourteen stateless cases, with six test threads.
Evidence: `target/detail-merge/latency-preview-render.log`.
The full PMI drawing comparison also found two obsolete view-difference exceptions:
`manufacturing_requirement:#722` on `hw_thumbwheel_notes_add.step` and
`thumbwheel_thread_knurl.step` now matches the reference front view. Those two exceptions
were removed; no new exception was added (349 remaining instead of 351).

The merged X-head placement exposed a genuine 1.1 mm band overrun on GRM-03's 2 mm
local length. Its top reservation now includes the first dimension gap plus two text heights
(16 mm instead of 12 mm); the label remains clear without relaxing lint. The existing GRM-03
regression now also forbids `annotation_outside_area`. Its corrected diagnostic has zero ink
findings, and the 119-callout route guard passes.

### Drawings inspected

All four PNGs were opened and inspected for supported witnesses, common origins, text/line
contacts, dimension/section identification, readable scales and sheet boundaries. SVGs,
plan JSON and render reports accompany each PNG under `target/detail-merge/renders/`.
All four final PNGs are byte-identical to the inspected images before the performance changes;
`target/detail-merge/png-final-comparison.json` records their hashes.

| Case | Result | Visual inspection |
| --- | --- | --- |
| string_post default | A4 2:1; detail B 10:1 | Front heights 3.8, 6.5, 7.6 and 14.8 share the high flat end; the 1 mm flange and 2× R0.2 are legible in B. |
| exact saved 14-tolerance spec | One A4 at 2:1; B 10:1; every item on sheet 1 | All 14 tolerances remain on their original quantities. Independent toleranced band lengths sit opposite the inspection baseline. No crossed text or off-sheet labels. |
| `cnc-review-spindle.step.gz` | A4 1:1 | PMI baseline, datums A/B, fits, internal thread, diamond knurl and section remain readable. |
| turned.step | A3 2:1 | 40 and 70 locate shoulders from the larger low end; no redundant reference chain. |

The saved-spec fixture key is
`string_post.step:spec:1c29f39db68e8b009a719d83ec1389d96a1e350a2d6b4bc142069453c99e3569`.
The spindle was decompressed directly from `tests/fixtures/samples/cnc-review-spindle.step.gz`
into `target/detail-merge/spindle.step`. The spec still reports its four informational
reservation overruns. The spindle retains exactly the previous detail commit's finding counts,
including five leader crossings, three reservation overruns and one leader-through-feature
finding; these are reported, not removed or waived.

* string_post: [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/renders/string_post.png)
* string_post_spec: [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/renders/string_post_spec.png)
* spindle: [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/renders/spindle.png)
* turned: [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/renders/turned.png)

The full-size sheet rule is evaluated with main's exact 20 mm dimension row on each side
of each principal/section view, against A3's usable 400 × 277 mm. The diagnostic comparison
finds no newly large-sheet **small** part among all 30. Hanger is the only new large-sheet
choice relative to main (A2 1:1 with its crop versus main's reduced A3 1:2): its full-size row
is 427.3268 mm and its column 280.52 mm, so F1 classifies it as eligible for large paper.
It is deliberately not reclassified as small merely because a reduced A3 drawing exists.
The final per-part calculation and main comparison are linked in the checks below.

### Required checks on the merged tree

The final required commands ran one at a time with `CARGO_BUILD_JOBS=6` and
`RUST_TEST_THREADS=6`. The release suite used the required corpus at
`/Users/paul/repos/quiddity/tests/corpus` with `DRAFTWRIGHT_CORPUS_REQUIRED=1`.
Normal process scheduling and the original 150 ms gates were used.

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --check` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/fmt.log) |
| `cargo clippy --all-targets --release -- -D warnings` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/clippy.log) |
| `cargo test --release`, required corpus | PASS: 1,016 tests across 103 targets; zero failures | [full log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/tests.log), [summary](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/test-summary.json) |
| `tools/quality.py`, all 30 manifest parts | PASS: all parts analysed, planned, rendered and reviewed | [survey](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality.log), [exact command](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-command.json) |
| `tools/ci_acceptance.py --check-results quality` | PASS: all 300 per-part metric comparisons, nothing worse | [comparison](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-acceptance.log), [results](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality/baseline.json) |
| `python3 -I tools/check_leader_routes.py <quality dir>` | PASS: 119 callouts on 30 parts | [guard log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-routes.log), [route report](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-routes.json) |
| Full-size A3 rule against main `53aa9f3f`, all 30 parts | PASS: zero new small-part A2+ choices | [per-part calculation](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-sheet-rule.json), [summary](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/quality-sheet-rule.log) |
| Four final PNGs versus the inspected images | PASS: all byte-identical | [hash comparison](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/png-final-comparison.json), [render results](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/render-refresh.log) |

The leader guard preserves the original base + 2 mm length limit and forbids new hidden,
section or cut crossings and new material runs over 2 mm. Fourteen unchanged pre-existing
runs remain; no route baseline was re-recorded. The quality baseline is byte-identical to
`origin/main:tools/ci-quality-baseline.json` (SHA-256
`7ca8cd45c69561f94e0ad1322bd1c7fb71a21029ac740d9e6639b11b32d046bb`).

| Quality metric, summed over 30 parts | Baseline | Merged |
| --- | ---: | ---: |
| `off_sheet` | 0 | 0 |
| `repeated_label` | 7 | 6 |
| `repeated_measure` | 0 | 0 |
| `views_overlap` | 0 | 0 |
| `view_on_title_block` | 0 | 0 |
| `ink` | 52 | 49 |
| `repeated_span` | 0 | 0 |
| `line_through_feature` | 73 | 69 |
| `hidden_on_visible` | 2232 | 0 |
| `section_hidden` | 0 | 0 |

Every individual part/metric cell is no worse, as well as these totals. String-post default
has zero findings in all ten quality metrics. The saved spec and spindle retain the pre-existing findings described in the
inspection section above.

The final release binary SHA-256 is
`f0941a18e07ad938cb70a618864a8ea42edd4bfe0a67bcb4f5b4f4e233eaae42`.
[Artifact metadata](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/cx-detail/target/detail-merge/final-artifacts.json) records the sheet/scale results
and unchanged PNGs. The local merge includes this report; no push is performed.


## Original implementation report — base `253b7601`, commit `7e29f66b`

The default string post now uses A4 at 2:1, with head detail B at 10:1 and section A–A.
The fitting isometric stays at sheet scale (2:1); it therefore needs no NTS caption.
Default front-view heights are 3.8 to the cross hole, 6.5 and 7.6 to shoulders, and 14.8 overall,
all from the high flat end. Shortest ordinates stand nearest the part. The 1 mm flange and
2× R0.2 callout belong to the head detail. Its teeth are visible in the enlarged exact geometry.

The exact saved spec `string_post.step:spec:1c29f39db68e8b009a719d83ec1389d96a1e350a2d6b4bc142069453c99e3569`
retains all 14 tolerances on their original quantities. It also uses A4 at 2:1 with detail B
at 10:1 and has no reported label/ink crossing or off-sheet annotation. Its explicitly toleranced
local band lengths stay independent on the opposite side from the inspection baseline.
Applying a tolerance on a 1.1 mm band to a cumulative 7.6 mm position would change manufacturing
intent; the port deliberately retains J4's independent controlling requirement instead.
Before this change the same saved spec consumed A2 at 10:1.
Four informational `annotation_outside_area` findings remain on that saved spec: its long
tolerance labels extend 3.8–19 mm beyond estimated front-view bands. They remain inside the
sheet and clear of other annotation ink; those reservation overruns are reported, not hidden.

## Python source traced

All paths below are relative to `/Users/paul/repos/draftwright/src/draftwright/`.

| Python source | Ported behaviour |
| --- | --- |
| `_core.py:1071`, `:1154`, `:1167`, `:1231` | Request metadata, caption, Y-chain readability and supported secondary crops |
| `analysis.py:115`, `:255` | Scale-dependent Y-chain and X-head footprints |
| `annotations/sections.py:791`, `:833`, `:867`, `:1136`, `:1188`, `:1369` | Crops, parent marker, drawing, decisions, resolution and approved prismatic rung recovery |
| `annotations/sections.py:85`, `:100`, `:110` | Reservation lifetime and ownership |
| `annotations/_axial_render.py:337`, `annotations/from_model.py:2366` | Queue exact approved axial dimensions |
| `annotations/orchestrator.py:228` | Authored detail requests |
| `auxiliary_layout.py:55` | Rectangular late-furniture space |
| `build_policy.py:538`, `builder.py:285` | Detail presence and iso keep-sheet-scale rule |
| `linting/structural.py:918`, `:1577`, `linting/ink_overlap.py:183` | Shared derived identifiers and annotation comparisons across scale groups |

Python tests consulted include `test_detail_views.py`, `test_derived_view_planning.py`,
`test_auxiliary_layout.py`, `test_compiled_plan_boundary.py:1060`, and the GRM03 axial/detail
scenarios. Exact approved requirement IDs pass through the ordinary Rust dimension planner;
no secondary ladder is reconstructed in rendering. Details have stable `v:detail_a`–`v:detail_h`
IDs, a source camera, model crop, absolute scale, owned IDs and authored/default metadata.
Authored feature crops retain an exact multiple of main scale. Omission does not rename details.
Section and detail letters share an allocation, including remote sections.

## Deliberate drawing choices and limits

* A thin terminal flange on a Z-turned whole part also requests a head detail. Python's automatic
  head producer only covers X; this extension is required for the reported string-post problem.
* PMI datum ends and authored baseline lengths take precedence. Otherwise the larger flat end
  (end diameter as the available geometric proxy) wins; the low end breaks a tie.
* Automatic details cannot force a larger initial sheet, or add annotation conflicts or longer
  leaders. An explicit smallest-scale constraint keeps its precedence. A declined optional crop
  releases its reservations; required dimensions remain on the source. Head details keep their
  compact sheet; prismatic layouts can still use the existing fill search.
* Prismatic details use the minimum readable absolute scale, at least 1.2 times parent scale.
  Python also searches preferred larger/half-step scales. Different supporting geometry and
  page reservations therefore produce different detail scales; the survey below records them.
* The requested parent circle replaces Python's present rectangular crop marker. Its exact arcs
  break behind annotation text and at the drawable boundary; the identifying letter avoids view
  and dimension ink. Captions and scale fidelity use the same independent structural lint pass.
* The Rust kernel has no solid Boolean crop. Exact projected curve intervals are clipped instead;
  this preserves existing outlines but creates no new cut-face caps. Supported strips are labelled
  PARTIAL PROFILE. This remains a geometric difference from Python's cropped solid.
* Native string-post recognition supplies no tooth/knurl feature or manufacturing pitch record.
  The enlarged teeth are drawn, but no numeric pitch/count requirement is invented. Existing
  authored/PMI knurl requirements continue through their diameter owner into a detail.

## Regression evidence

`tests/detail_views.rs` covers the A4/head result, common axial origin, extension/text clearance,
no stray detail (with a positive head witness), authored exact scale, saved chain tolerance IDs,
the complete 14-tolerance saved spec, approved prismatic/Y-chain ownership, letter allocation,
and omission identity. The pre-change executable rejects the authored `detail` field and produces
no head view, mixed origins and the old sheet choices; its outputs are retained below.

The executable-level witnesses in `regression_cli.py` exercise the five requested conditions
without relying on the new Rust types: all five fail against the saved pre-change executable
(`regression-before.log`), and all five pass against the final executable
(`regression-after.log`). The no-detail condition includes a positive fine-region witness,
so a generator that never produces details cannot pass it.

Existing connected-profile assertions now expect baseline positions. Original chain/reference,
omission and pin invariants are also exercised on explicitly independent profiles; none of those
invariants was removed. Crop-aware test bounds replace the former whole-part envelopes. Captured
bad section ink retains its original carrier frame so it is translated, rather than erased, when
layout moves. Comparison catalogues enumerate exact moved quantities and views; they do not waive
coverage or collisions. The single-sheet byte fixtures and CI quality baseline are unchanged.

## Selected PNG inspection

I inspected each before/after/Python PNG for geometry, dimension origins, witness support,
text/line contacts, detail identification and scale. The simple turned part loses its redundant
reference link; the spindle retains its PMI baseline, datums, thread and knurl requirements.
The two stair specimens retain A3 at 5:1 in the default fill search: their rungs are readable
at that main scale, so the optional detail disappears. With A4 at 1:1 explicitly selected, both
recover approved height rungs into supported details (2.333:1 and 3.5:1); the isometric
stays at the 1:1 sheet scale. Python defaults to A4 at 2:1 with preferred details at 3:1 and 4:1.
This remaining default packing difference is visible in the artifacts. The two controls keep no detail;
pre-existing recognition differences from Python remain visible in their dimension inventories.
Python's string-post spec comparison applies its 13 supported size tolerances through the Python
API; its cross-hole location tolerance is unavailable through that handle and is not claimed as
an exact 14-tolerance replay. Rust replays the exact saved spec.

All render artifacts are under `/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work`. SVGs and plan JSON accompany the Rust PNGs.

| Case | Rust before | Rust after | Python |
| --- | --- | --- | --- |
| string_post | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/string_post.png) | [A4 2:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/string_post.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/string_post/string_post.png) |
| string_post_spec | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/string_post_spec.png) | [A4 2:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/string_post_spec.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/string_post_spec/string_post_spec.png) |
| turned | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/turned.png) | [A3 2:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/turned.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/turned/turned.png) |
| spindle | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/spindle.png) | [A4 1:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/spindle.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/spindle/spindle.png) |
| crowded_prism | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/crowded_prism.png) | [A3 5:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/crowded_prism.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/crowded_prism/crowded_prism.png) |
| unequal_prism | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/unequal_prism.png) | [A3 5:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/unequal_prism.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/unequal_prism/unequal_prism.png) |
| 1000 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/1000.png) | [A4 2:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/1000.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/1000/1000.png) |
| 1004 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/1004.png) | [A4 1:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/1004.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/1004/1004.png) |

Two corpus prismatic defaults were also rendered and inspected: 10060 retains its supported height crop on A4 (detail 3.5:1 versus Python 4:1); sm-hanger uses A2 at 1:1, with detail 1.75:1 versus Python 2:1. Hanger's supported strip is legible. On 10060 the exact projection crop leaves disconnected outline fragments where the solid has no projected edge; Python's Boolean crop supplies connecting cap edges. The PARTIAL PROFILE caption states that limitation. Recognised height inventories also differ (Rust's approved 11.3 mm rung versus Python's 13.3 mm rung).

| Case | Rust before | Rust after | Python |
| --- | --- | --- | --- |
| 10060 | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/10060.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/10060.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/10060/10060.png) |
| sm-hanger | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/sm-hanger.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/sm-hanger.png) | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/python/sm-hanger/sm-hanger.png) |

The controlled A4 1:1 prismatic replays use the same explicit sheet spec in both Rust executables.

| Case | Before at A4 1:1 | After at A4 1:1 |
| --- | --- | --- |
| crowded_prism | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/crowded_prism_a4.png) | [Detail A 2.333:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/crowded_prism_a4.png) |
| unequal_prism | [PNG](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/before/unequal_prism_a4.png) | [Detail A 3.5:1](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/after/unequal_prism_a4.png) |

## Thirty quality parts: Rust versus Python

All 30 Rust plans/reviews/SVGs were regenerated. Python completed 29 corresponding builds.
Six slow Python cases completed on SVG-only retries; cgb207 fails in Python before producing
a drawing. The machine-readable survey retains every Python request/refusal and Rust owner.
Python reports are under `python/` and `python-retry/` in the scratch root;
Rust final reports are under `quality-final-10/`.

Scale values below are absolute paper/model ratios. `—` means no placed detail.

| Part | Python details | Rust details | Rust mains | Explanation |
| --- | --- | --- | --- | --- |
| cadgenbench_inputs/cgb203.step | — | — | A4 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| cadgenbench_inputs/cgb207.step | FAILED: empty plan | — | A4 1:2 | Python fails with an empty plan projection (ValueError); no completed Python drawing is available. Rust completes without a detail. |
| cadgenbench_inputs/cgb241.step | — | — | A3 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| cadgenbench_inputs/cgb245.step | — | — | A2 1:2 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| gramel/GRM-03_thumbwheel_drive_screw.step | — | A 10:1 | A4 2:1 | Rust chooses 2:1 mains and recovers the crowded X head at 10:1; Python enlarges the mains instead. |
| gramel/GRM-05_depth_lock_bolt.step | — | — | A3 5:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| gramel/string_post.step | — | B 10:1 | A4 2:1 | Intentional Z-axis terminal-head extension; Python has no automatic vertical-head request. |
| nist/nist_ctc_01_asme1_rd.stp | — | A 0.24:1 | A3 1:5 | Rust already approved the supported 50 mm rung that Python withholds. It now moves to its own readable crop. |
| nist/nist_ctc_02_asme1_rc.stp | — | — | A1 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ctc_03_asme1_rc.stp | — | — | A3 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ctc_04_asme1_rd.stp | — | — | A1 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ctc_05_asme1_rd.stp | — | — | A2 1:5 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ftc_06_asme1_rd.stp | — | — | A1 1:2 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ftc_07_asme1_rd.stp | — | — | A2 1:2 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ftc_08_asme1_rc.stp | — | — | A2 1:2 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ftc_09_asme1_rd.stp | — | — | A1 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| nist/nist_ftc_10_asme1_rb.stp | B 4.5:1 | B 3.5:1 | A1 1:1 | Same prismatic recovery; Rust uses the minimum readable 3.5 rather than Python’s preferred fitted 4.5. |
| ttt_inputs/sm-hanger.step | A 2:1 | A 1.75:1 | A2 1:1 | Same supported height recovery; Rust uses required 1.75, Python fits a preferred 2.0. Rust retains 1:1 mains on A2 to keep the established leaders clear. |
| mfcadpp/1000.step | — | — | A4 2:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/10000.step | — | — | A4 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/10007.step | A 6:1 | — | A4 1:1 | Rust requests the crop but cannot reserve its minimum footprint in the current principal/annotation layout. Python’s different packing fits 6:1. Source requirements remain drawn; this is a remaining layout difference. |
| mfcadpp/10020.step | — | — | A4 2:1 | Both finish without a detail. Python refuses minimum-footprint fit; Rust declines trials that lengthen leaders or conflict with annotations. |
| mfcadpp/10033.step | A 1.454:1 | — | A3 2:1 | Rust retains A3 mains at 2:1, where these rungs pass the source gate and the original short leader survives. Its smaller-paper detail trial needed a 17 mm leader versus the established 10 mm route. Python fits its 1.454 detail on A4. |
| mfcadpp/10038.step | — | — | A4 2:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/1004.step | — | — | A4 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/10046.step | — | A 1.2:1 | A4 1:1 | Native Rust approval retains a supported 1.8 mm height that Python does not queue. Its detail moves that same original requirement. |
| mfcadpp/10047.step | — | — | A4 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/10049.step | — | — | A4 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |
| mfcadpp/10060.step | A 4:1 | A 3.5:1 | A4 1:1 | Same prismatic recovery; minimum 3.5 in Rust versus fitted preferred 4.0 in Python. |
| mfcadpp/10063.step | — | — | A4 1:1 | Both finish without a detail; request/refusal evidence is retained in the machine-readable survey. |

## Quality and route gates

No cell among the 30 × 10 CI metrics is worse than `tools/ci-quality-baseline.json`.
The unchanged pre-change executable reproduced all 300 baseline cells before the port.
The baseline was not re-recorded. Every improvement is listed below. Visible-edge precedence
now applies to the isometric too, so hidden strokes no longer duplicate its visible ink.

| Part | Metric | Before | After |
| --- | --- | ---: | ---: |
| 1000 | hidden_on_visible | 20 | 0 |
| 10000 | hidden_on_visible | 7 | 0 |
| 10007 | hidden_on_visible | 20 | 0 |
| 10020 | hidden_on_visible | 31 | 0 |
| 10033 | hidden_on_visible | 15 | 0 |
| 10038 | hidden_on_visible | 26 | 0 |
| 1004 | hidden_on_visible | 16 | 0 |
| 10046 | hidden_on_visible | 22 | 0 |
| 10047 | hidden_on_visible | 18 | 0 |
| 10049 | hidden_on_visible | 21 | 0 |
| 10060 | ink | 6 | 4 |
| 10060 | hidden_on_visible | 17 | 0 |
| 10063 | hidden_on_visible | 17 | 0 |
| GRM-03_thumbwheel_drive_screw | hidden_on_visible | 14 | 0 |
| GRM-05_depth_lock_bolt | hidden_on_visible | 7 | 0 |
| cgb203 | hidden_on_visible | 61 | 0 |
| cgb207 | hidden_on_visible | 195 | 0 |
| cgb241 | hidden_on_visible | 46 | 0 |
| cgb245 | hidden_on_visible | 172 | 0 |
| nist_ctc_01_asme1_rd | repeated_label | 1 | 0 |
| nist_ctc_01_asme1_rd | hidden_on_visible | 55 | 0 |
| nist_ctc_02_asme1_rc | hidden_on_visible | 426 | 0 |
| nist_ctc_03_asme1_rc | hidden_on_visible | 148 | 0 |
| nist_ctc_04_asme1_rd | hidden_on_visible | 205 | 0 |
| nist_ctc_05_asme1_rd | hidden_on_visible | 66 | 0 |
| nist_ftc_06_asme1_rd | hidden_on_visible | 32 | 0 |
| nist_ftc_07_asme1_rd | hidden_on_visible | 136 | 0 |
| nist_ftc_08_asme1_rc | hidden_on_visible | 218 | 0 |
| nist_ftc_09_asme1_rd | hidden_on_visible | 17 | 0 |
| nist_ftc_10_asme1_rb | line_through_feature | 11 | 7 |
| nist_ftc_10_asme1_rb | hidden_on_visible | 121 | 0 |
| sm-hanger | hidden_on_visible | 57 | 0 |
| string_post | ink | 1 | 0 |
| string_post | hidden_on_visible | 26 | 0 |

`python3 -I tools/check_leader_routes.py /private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/quality-final-10 --report /private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/leader-routes-final-10.json`
passes: 119 callouts on all 30 parts, no extra hidden/section/cutting-plane crossings,
no new padded text/head/outline contacts, routes at most 2 mm longer, and no new runs
beside ink over 2 mm. Fifteen unchanged pre-existing runs remain explicitly reported.

The only part newly using A2 or larger is sm-hanger: A3 at 1:2 before, A2 at 1:1 now.
The independent pinned-1:1 replay also selects A2; its actual principal/annotation layout
needs that paper at full size. No small part acquires A2 merely for enlargement.
The evidence is `/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/sheet-size-check.json`. String_post’s saved spec improves from A2 at
10:1 to A4 at 2:1.

The semantic comparison catalogues were updated for exact relocated quantities/views and
new inspection-arrow/leader bands: `known_dimensions.json`, `known_views.json`,
`known_bands.json`, two PMI manufacturing riders, and the removed CTC01 repeated label.
The connector109 baseline removes five previously allowed leader crossings; its changed
joint nine-callout search now truthfully reports the existing 200000-step cap. Every item
remains drawn; optimality is unproven. No ink threshold, coverage check, route guard,
single-sheet byte fixture, or quality baseline was relaxed.

The first full verification run exposed a NIST session latency regression under six test
threads (one 442 ms sample; a complete-target retry also exceeded 150 ms). The gate stayed
enabled. Detail requests and identical dimension-placement inputs now share preparation
within one immutable planning call; leader refinement rejects routes already over its
length budget and computes ranking scores once. No geometry or clearance predicate is
relaxed. Temporary profiling instrumentation was removed. The focused session and multi-sheet
gates then passed (NIST p90 135 ms, MCP p90 99 ms); the final corpus run below uses this code.
All 12 regenerated Rust PNGs are byte-identical to the inspected pre-optimization renders
(`render-performance-comparison.json` in the scratch directory).
All 30 quality-part plan JSON files are also byte-identical, and every final quality result
matches the pre-optimization audit (`plan-performance-comparison.json`).

## Original checks and delivery (historical)

The final source was checked in the requested order, with the cargo environment loaded,
`CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6`:

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --check` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/fmt-final.log) |
| `cargo clippy --all-targets --release -- -D warnings` | PASS | [log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/clippy-final.log) |
| Corpus-required `cargo test --release --no-fail-fast` | 979 passed; 7 HTTP sandbox failures; all other targets pass | [full log](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/tests-final.log) |
| Unchanged 150 ms session and multi-sheet gates | PASS in the full run | [focused timing detail](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/latency-cached-placement.log) |
| All 300 CI quality cells | PASS; no baseline update | [acceptance](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/quality-acceptance-final.log) |
| Leader-route guard | PASS, 119 callouts / 30 parts | [report](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/leader-routes-final-10.json) |
| Five requested executable-level regressions | All five fail before; all five pass after | [before](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/regression-before.log), [after](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/regression-after.log) |

The full test command used
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1`.
It exited 101 solely for the seven HTTP tests in `tests/stateless.rs`: their server cannot
bind a socket in this sandbox, so the listener never supplies a port. All six non-HTTP
stateless tests pass, including warm-cache/fresh-process identity and published schemas.
The unchanged session target passes all eight tests, including NIST; the isometric/NTS
target passes all 21 tests; all nine new detail tests pass. No gate was disabled.
The single-sheet byte guard remains opt-in and was not enabled or re-recorded.

The test-run CLI is byte-identical to the final quality/render CLI:
`706d7f8d0c5258dd4b765bc7b9c5b2e847d90c35e7c8510737fd0a1fb0ccaf88`. Machine-readable check results are [here](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/final-checks.json).

The sandbox repeatedly denies staging:
`/Users/paul/repos/draftwright-rust/.git/worktrees/wt-detail/index.lock: Operation not permitted`.
Consequently **no commits were possible**, and nothing was pushed. Changes remain in the
authorised worktree on `cx/detail-views`. Cumulative patch checkpoints preserve the work;
the final complete patch, including new gzipped fixtures, is
[06-detail-views.patch](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/checkpoints/06-detail-views.patch).
Its [manifest](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/checkpoints/manifest.json) records the branch, base and hashes.
The prepared [commit message](/private/tmp/claude-501/-Users-paul-repos-quiddity-rust/4f6aac0c-d5a0-4c82-bc67-2d2552947882/scratchpad/detail-work/checkpoints/commit-message.txt) ends with the required
`Co-Authored-By: Codex <codex@openai.com>` line.
