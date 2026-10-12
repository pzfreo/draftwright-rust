# H6 sheet and scale choice, 2026-10-12

The planner finds the smallest valid complete standard fit, permits only its
immediate larger sheet with an enlargement, and selects by drawn-item fill.
Explicit Spec choices and pin frames remain authoritative. The named target
`sheet_fit::FILL_BAND` is 60–85%; no qualifying drawing means the fullest valid
allowed pair wins. Invalid/incomplete candidates never win merely on fill.

Fill is the union of each item's actual bounding footprint with 1 mm paper
clearance, including witnesses, arrows, GD&T, notes/tables, section furniture
and the scale/projection furniture outside the drawn title grid. Reservation
bands do not count. The denominator is the bordered drawing area less the
32 mm drawn title grid, rather than its 40 mm reservation.

The conservative automatic scale floor is 1:10 until the Spec gains a smallest
scale control. If nothing fits legibly, the drawing keeps honest overflow
findings at that floor. A bounded measured correction can lift a free
orthographic row and refit its orientation view. GD&T view blocks can be
composed from a same-scale A0 template and packed with every witness, leader
and frame moving with its feature; no leader route is recalculated by packing.
New material crossings veto a candidate. A 16-entry process-local memo keyed
by all three pure inputs keeps identical previews/commits fast.

These are documented, overturnable judgement calls in `next-session.md`. The
existing glyph-size calculation already measured GD&T frames; this change
uses those footprints in finished candidate choice and measured composition.

No rule catalogue YAML, MCP task condition, commit or push was changed. Python
`compose.choose_scale` and the historical six-case recovery remain references;
the maintainer's new allocation policy deliberately supersedes their first-fit
stopping rule. Evidence/verdicts are in `known_layout.json` and
`known_sheet_policy.json`.



## Evidence and verification

The archived pre-change release is `target/h6-before/draftwright`. Final native
measurements use the release binary also frozen at
`target/h6-verification/draftwright`, SHA-256
`dca69e46d5b398840086d7bafe17ebd351c6f82f45bb6e0696adf71bd2c9f2d8`.
Before fill was remeasured from the archived plans, rather than inferred from
reservation bands. `tools/sheet_choices.py` independently computes rectangle
unions, checks against the Rust measure to 1e-9, and verifies candidate ranking,
the smallest fit's largest scale and the one-step enlargement limit. Its rank
check respects the basis's 0.1% rounding at band endpoints; the selected fill
is independently checked at full precision.

Seven new Rust regressions cover smallest fit/one-step enlargement, fill
selection, named sheet/scale authority, validity over fill, native GRM-03's band,
the supplied six-item PMI plate and the legibility floor. Six fail against the
original Rust source (0/6); the seventh's identical huge-envelope scenario
fails through the original CLI, which chooses A0 at 1:500. The new Python
quality-column test also fails against original quality.py. Red evidence is
`target/h6-before/six-tests.log`, `target/h6-before/floor-test.log` and
`target/h6-before-python/tests.log`. No original capture is overwritten.

Existing tests for the superseded recovery order now enforce bounded paper and
retained measurements. Placement locality still compares every geometry field;
only recomputed fill and its basis suffix are excluded. The crowding test keeps
a real starved A4 case and checks m1000's newly fitting height ladder's overall
extent outside every rung. Service/review tests assert the new informational
sheet-choice finding. Python differences have explicit verdicts in
`known_sheet_policy.json` and `known_layout.json`.

Formatting and strict Clippy pass. Python quality tests: 24/24; acceptance tests:
15/15; Node client tests: 8/8. The scripted MCP evaluation succeeds on all seven
unchanged tasks (16 calls, 6,399 estimated tokens). The required random-pin grid
runs with --require-go against the same-binary baseline: all 270 part/cell
measurements pass, with no new off-sheet items. The independent acceptance
validator also accepts all nine grid reports and the MCP report.

FULL_RUST_CHECK

| Seed | Pins N | Parts | Unpinned ink | Pinned ink | New off-sheet | Verdict |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 1 | 30 | 262 | 252 | 0 | GO |
| 1 | 3 | 30 | 262 | 244 | 0 | GO |
| 1 | 8 | 30 | 262 | 242 | 0 | GO |
| 2 | 1 | 30 | 262 | 260 | 0 | GO |
| 2 | 3 | 30 | 262 | 211 | 0 | GO |
| 2 | 8 | 30 | 262 | 215 | 0 | GO |
| 3 | 1 | 30 | 262 | 258 | 0 | GO |
| 3 | 3 | 30 | 262 | 247 | 0 | GO |
| 3 | 8 | 30 | 262 | 228 | 0 | GO |


Full logs and machine reports are under `target/h6-verification`: fmt.log,
clippy.log, tests.log, build.log, quality-check.log, pmi-check.log, choices-30.log,
choices-pmi.log, pins-*.log, pin-validation.log, mcp.json and mcp-validation.log.

## Quality before and after

All 30 native parts pass the latest pre-H6 status-table ratchet: 390 cells, no
part worse. All 14 AP242 fixtures plus the supplied plate pass an independent
195-cell comparison, no part worse. Claimed faces and unplanned-feature counts
are unchanged on every part.

| Metric | Native before | Native after | AP242 + plate before | AP242 + plate after |
|---|---:|---:|---:|---:|
| off_sheet | 0 | 0 | 0 | 0 |
| repeated_label | 17 | 17 | 16 | 16 |
| repeated_measure | 0 | 0 | 0 | 0 |
| views_overlap | 0 | 0 | 0 | 0 |
| view_on_title_block | 0 | 0 | 0 | 0 |
| ink | 273 | 262 | 1612 | 1593 |
| repeated_span | 0 | 0 | 0 | 0 |
| line_through_feature | 211 | 208 | 447 | 446 |
| hidden_on_visible | 2263 | 2245 | 1425 | 1425 |
| section_hidden | 0 | 0 | 0 | 0 |
| claimed_faces | 1390 | 1390 | 988 | 988 |
| faces | 4268 | 4268 | 2492 | 2492 |
| unplanned_features | 277 | 277 | 149 | 149 |

Per-part counts are below. Every unlisted metric (`off_sheet`,
`repeated_measure`, `views_overlap`, `view_on_title_block`, `repeated_span`,
`section_hidden`) is 0 before and after for every part. The fill column and
paper choices follow in their own tables.

### Native 30

| Part | Repeated labels B→A | Ink B→A | Material B→A | Hidden B→A | Claimed/total B→A | Unplanned B→A |
|---|---:|---:|---:|---:|---|---:|
| 1000 | 0→0 | 1→0 | 0→0 | 20→20 | 3/52→3/52 | 0→0 |
| 10000 | 0→0 | 5→5 | 0→0 | 7→7 | 5/27→5/27 | 1→1 |
| 10007 | 0→0 | 0→0 | 0→0 | 20→20 | 7/32→7/32 | 4→4 |
| 10020 | 0→0 | 1→1 | 0→0 | 31→31 | 3/57→3/57 | 0→0 |
| 10033 | 0→0 | 0→0 | 0→0 | 15→15 | 5/24→5/24 | 0→0 |
| 10038 | 0→0 | 1→0 | 0→0 | 26→26 | 4/51→4/51 | 1→1 |
| 1004 | 0→0 | 0→0 | 2→2 | 16→16 | 2/28→2/28 | 0→0 |
| 10046 | 0→0 | 0→0 | 0→0 | 22→22 | 0/35→0/35 | 0→0 |
| 10047 | 0→0 | 0→0 | 0→0 | 18→18 | 3/34→3/34 | 1→1 |
| 10049 | 0→0 | 0→0 | 0→0 | 21→21 | 5/33→5/33 | 1→1 |
| 10060 | 1→1 | 6→6 | 0→0 | 17→17 | 8/32→8/32 | 0→0 |
| 10063 | 0→0 | 1→1 | 0→0 | 17→17 | 5/41→5/41 | 1→1 |
| GRM-03_thumbwheel_drive_screw | 0→0 | 0→0 | 0→0 | 14→14 | 11/14→11/14 | 0→0 |
| GRM-05_depth_lock_bolt | 0→0 | 0→0 | 0→0 | 7→7 | 5/9→5/9 | 0→0 |
| cgb203 | 0→0 | 0→0 | 0→0 | 61→61 | 2/156→2/156 | 0→0 |
| cgb207 | 0→0 | 7→6 | 4→4 | 215→203 | 31/286→31/286 | 5→5 |
| cgb241 | 0→0 | 2→2 | 0→0 | 46→46 | 12/52→12/52 | 6→6 |
| cgb245 | 1→1 | 6→6 | 11→11 | 172→172 | 168/450→168/450 | 82→82 |
| nist_ctc_01_asme1_rd | 1→1 | 11→11 | 5→5 | 55→55 | 66/139→66/139 | 32→32 |
| nist_ctc_02_asme1_rc | 1→1 | 36→36 | 36→36 | 426→426 | 240/664→240/664 | 35→35 |
| nist_ctc_03_asme1_rc | 0→0 | 24→21 | 8→5 | 148→148 | 56/139→56/139 | 15→15 |
| nist_ctc_04_asme1_rd | 5→5 | 54→54 | 62→62 | 205→205 | 205/518→205/518 | 15→15 |
| nist_ctc_05_asme1_rd | 0→0 | 8→8 | 16→16 | 66→66 | 98/209→98/209 | 10→10 |
| nist_ftc_06_asme1_rd | 2→2 | 11→9 | 3→3 | 32→32 | 56/144→56/144 | 5→5 |
| nist_ftc_07_asme1_rd | 0→0 | 9→7 | 9→9 | 136→136 | 77/269→77/269 | 14→14 |
| nist_ftc_08_asme1_rc | 1→1 | 46→46 | 18→18 | 218→218 | 93/270→93/270 | 13→13 |
| nist_ftc_09_asme1_rd | 3→3 | 20→20 | 14→14 | 17→17 | 91/158→91/158 | 11→11 |
| nist_ftc_10_asme1_rb | 2→2 | 22→22 | 23→23 | 121→121 | 81/214→81/214 | 0→0 |
| sm-hanger | 0→0 | 0→0 | 0→0 | 68→62 | 37/103→37/103 | 23→23 |
| string_post | 0→0 | 2→1 | 0→0 | 26→26 | 11/28→11/28 | 2→2 |

### AP242 14 + plate

| Part | Repeated labels B→A | Ink B→A | Material B→A | Hidden B→A | Claimed/total B→A | Unplanned B→A |
|---|---:|---:|---:|---:|---|---:|
| assembly_plate_pin | 1→1 | 9→9 | 9→9 | 9→9 | 13/27→13/27 | 3→3 |
| bolt_thread_knurl | 0→0 | 4→3 | 0→0 | 7→7 | 5/9→5/9 | 0→0 |
| bracket_positions | 2→2 | 17→17 | 6→6 | 215→215 | 31/286→31/286 | 5→5 |
| manufacturing-pmi | 1→1 | 18→8 | 4→4 | 2→2 | 8/14→8/14 | 0→0 |
| nist_ctc_01_asme1_ap242-e1 | 1→1 | 26→26 | 7→7 | 46→46 | 72/117→72/117 | 29→29 |
| nist_ctc_01_merge | 1→1 | 28→28 | 7→7 | 46→46 | 72/117→72/117 | 29→29 |
| nist_ctc_02_asme1_ap242-e2 | 1→1 | 64→64 | 59→59 | 416→416 | 244/637→244/637 | 34→34 |
| nist_ftc_07_asme1_ap242-e2 | 0→0 | 65→65 | 25→25 | 151→151 | 109/306→109/306 | 19→19 |
| nist_ftc_10_asme1_ap242-e2 | 2→2 | 438→438 | 123→123 | 123→123 | 119/282→119/282 | 0→0 |
| nist_stc_06_asme1_ap242-e3 | 2→2 | 104→104 | 11→11 | 98→98 | 56/144→56/144 | 5→5 |
| nist_stc_09_asme1_ap242-e4 | 3→3 | 521→521 | 114→114 | 6→6 | 91/163→91/163 | 11→11 |
| nist_stc_10_asme1_ap242-e2 | 1→1 | 278→278 | 71→71 | 154→154 | 93/256→93/256 | 12→12 |
| spool_fits | 1→1 | 20→20 | 10→10 | 112→112 | 53/92→53/92 | 0→0 |
| string_post_tapped | 0→0 | 8→8 | 0→0 | 26→26 | 11/28→11/28 | 2→2 |
| thumbwheel_thread_knurl | 0→0 | 12→4 | 1→0 | 14→14 | 11/14→11/14 | 0→0 |

## Sheet, scale and fill for every part

The before binary draws native GRM-03 at A4 2:1; the task described another
A4 1:1 capture. The comparison below uses the measured archived binary.
CTC-03's no-complete-fit fallback changes from A2 to A3 at the same 1:5 scale
because the title-block lift improves the initial estimate. It retains honest
findings and spends less paper; it is not reported as a complete fit.

Every paper increase in these tables has a valid complete candidate basis and
only one step from the smallest fit with a larger scale. There are no paper
increases among exhausted-search fallbacks. The complete basis records all
measured candidates; below-band choices are the fullest eligible valid pair.

### Native 30

| Part | Before sheet | Scale | Fill % | After sheet | Scale | Fill % |
|---|---|---|---:|---|---|---:|
| 1000 | A4 | 1:1 | 19.2 | A4 | 2:1 | 34.74 |
| 10000 | A4 | 1:1 | 23.97 | A4 | 1:1 | 23.97 |
| 10007 | A4 | 1:1 | 21.83 | A3 | 2:1 | 22.91 |
| 10020 | A4 | 2:1 | 29.53 | A4 | 2:1 | 29.53 |
| 10033 | A4 | 1:1 | 25.71 | A3 | 2:1 | 39.9 |
| 10038 | A4 | 1:1 | 20.05 | A4 | 2:1 | 42.3 |
| 1004 | A4 | 1:1 | 20.54 | A4 | 1:1 | 20.54 |
| 10046 | A4 | 1:1 | 20.03 | A3 | 2:1 | 20.69 |
| 10047 | A4 | 1:1 | 15.18 | A4 | 1:1 | 15.18 |
| 10049 | A4 | 1:1 | 17.14 | A4 | 1:1 | 17.14 |
| 10060 | A4 | 1:1 | 17.04 | A4 | 1:1 | 17.04 |
| 10063 | A4 | 1:1 | 22.62 | A4 | 1:1 | 22.62 |
| GRM-03_thumbwheel_drive_screw | A4 | 2:1 | 13.94 | A3 | 10:1 | 60.06 |
| GRM-05_depth_lock_bolt | A4 | 2:1 | 12.54 | A3 | 5:1 | 16.64 |
| cgb203 | A3 | 1:5 | 19.89 | A4 | 1:5 | 43.22 |
| cgb207 | A2 | 1:1 | 29.51 | A1 | 2:1 | 33.57 |
| cgb241 | A3 | 1:5 | 47.37 | A3 | 1:5 | 47.37 |
| cgb245 | A2 | 1:2 | 40.27 | A2 | 1:2 | 40.27 |
| nist_ctc_01_asme1_rd | A3 | 1:5 | 57.56 | A3 | 1:5 | 57.56 |
| nist_ctc_02_asme1_rc | A1 | 1:5 | 38.12 | A1 | 1:5 | 38.12 |
| nist_ctc_03_asme1_rc | A2 | 1:5 | 17.61 | A3 | 1:5 | 37.01 |
| nist_ctc_04_asme1_rd | A1 | 1:5 | 24.32 | A1 | 1:5 | 24.32 |
| nist_ctc_05_asme1_rd | A2 | 1:5 | 38.89 | A2 | 1:5 | 38.89 |
| nist_ftc_06_asme1_rd | A2 | 1:5 | 14.43 | A1 | 1:2 | 19.5 |
| nist_ftc_07_asme1_rd | A2 | 1:2 | 44.85 | A1 | 1:1 | 50.33 |
| nist_ftc_08_asme1_rc | A2 | 1:2 | 42.19 | A2 | 1:2 | 42.19 |
| nist_ftc_09_asme1_rd | A2 | 1:1 | 69.09 | A2 | 1:1 | 69.09 |
| nist_ftc_10_asme1_rb | A1 | 1:1 | 19.71 | A1 | 1:1 | 19.71 |
| sm-hanger | A2 | 1:1 | 47.39 | A4 | 1:2 | 53.83 |
| string_post | A4 | 2:1 | 8.97 | A3 | 5:1 | 11.59 |

### AP242 14 and supplied plate

| Part | Before sheet | Scale | Fill % | After sheet | Scale | Fill % |
|---|---|---|---:|---|---|---:|
| assembly_plate_pin | A2 | 1:1 | 24.54 | A2 | 1:1 | 24.54 |
| bolt_thread_knurl | A4 | 1:1 | 15.1 | A4 | 2:1 | 19.62 |
| bracket_positions | A2 | 1:1 | 33.34 | A2 | 1:1 | 33.34 |
| manufacturing-pmi | A2 | 1:1 | 17.99 | A3 | 1:1 | 38.01 |
| nist_ctc_01_asme1_ap242-e1 | A3 | 1:5 | 60.72 | A3 | 1:5 | 60.72 |
| nist_ctc_01_merge | A3 | 1:5 | 61.92 | A3 | 1:5 | 61.92 |
| nist_ctc_02_asme1_ap242-e2 | A1 | 1:5 | 39.12 | A1 | 1:5 | 39.12 |
| nist_ftc_07_asme1_ap242-e2 | A2 | 1:2 | 47.8 | A2 | 1:2 | 47.8 |
| nist_ftc_10_asme1_ap242-e2 | A1 | 1:1 | 23.85 | A1 | 1:1 | 23.85 |
| nist_stc_06_asme1_ap242-e3 | A0 | 1:1 | 42.46 | A0 | 1:1 | 42.46 |
| nist_stc_09_asme1_ap242-e4 | A1 | 1:5 | 12.07 | A1 | 1:5 | 12.07 |
| nist_stc_10_asme1_ap242-e2 | A1 | 1:1 | 23.61 | A1 | 1:1 | 23.61 |
| spool_fits | A2 | 1:1 | 47.53 | A2 | 1:1 | 47.53 |
| string_post_tapped | A4 | 2:1 | 14.87 | A4 | 2:1 | 14.87 |
| thumbwheel_thread_knurl | A4 | 2:1 | 25.99 | A3 | 10:1 | 49.2 |

## Visual review and remaining work

All 30 native and 15 AP242/plate SVG sheets were rendered with the release CLI
and rasterised with resvg at 2400 px. All eight contact sheets were inspected,
with individual close inspection of the supplied plate, GRM-03, cgb207 and
CTC-03. Evidence is in `target/h6-verification/visual-quality` and `visual-pmi`.
The plate keeps all six GD&T items clear below/alongside their carriers, with
no right view across the title block and no clipped labels. GRM-03's larger
views and shoulder dimensions are legible and clear of the title block.
CTC-03 gains useful view size on smaller paper. cgb207's orientation view stays
subordinate to its dimensioned views.

The dense NIST sheets still show long, difficult annotation ladders and existing
conflicts. They need the other streams' tables/multi-sheet work, not a visual
approval from H6. The existing GRM axial closed chain and hidden isometric ink
are also visible; neither was changed in this stream. resvg logs include its
existing Courier New fallback and path-dashing warnings; the SVG line-work
ratchet is measured before rasterisation.

The 1:10 floor, 1 mm clearance, per-item axis-aligned bounding footprint and
16-entry pure-input memo are overturnable choices recorded in next-session.md.
Cold searches on dense PMI fixtures remain substantially more expensive than
a memoised identical preview; the existing session p90 bar is unchanged.

Not done: multi-sheet allocation/controls, sections/details, first-angle layout,
and unsupported surface-texture/attribute-set extraction on the supplied plate.
Existing dense NIST annotation conflicts, missing recognition/dimension coverage,
axial chains and hidden-line defects retain their findings. These belong to the
other streams; H6 does not reroute leaders to conceal them.

Rules covered: ADR2-D2; ADR2-3 / ISSUE-1299-a (new maintainer policy); ADR2-16;
ARCH0018-2; ISSUE-1338-a; LINT-quality-hard_layout_codes. Partial: ADR2-D4 /
ISSUE-1216-a (measured composition and glyph sizing); ISSUE-563-b (existing lane
reservations); ISSUE-1396-a (automatic floor). Catalogue YAML is untouched.

## Files changed

- Planner/geometry: src/model.rs, src/sheet_fit.rs, src/layout.rs,
  src/dims/mod.rs (shared arrow footprint constant), src/lib.rs.
- Measurement: tools/quality.py, tools/test_quality.py, tools/sheet_choices.py,
  examples/sheet_fill.rs.
- Regressions: tests/sheet_fill.rs; existing identity, issue_drafting,
  issue_regressions, model, page_escalation, placement and serve tests.
- Fixtures/verdicts: tests/fixtures/known_layout.json, known_sheet_policy.json,
  parts/manufacturing_pmi.step.gz and its provenance markdown.
- Documentation: docs/model.md, docs/next-session.md, docs/status-2026-10-10.md,
  this report.

No commit, push, rules YAML or tools/mcp-eval/tasks.json edit.

## H6 fix round, 2026-10-12

The earlier implementation record above is historical. The review correctly found
that a lower material-crossing count could veto the only valid complete drawing,
and that a constant 1:10 floor did not measure legibility. Recovery now judges the
seed's legibility, drawn bounds, hard findings and GD&T completeness before using
its secondary crossing count as a veto. An invalid seed cannot veto a complete
valid candidate. Failed seeds may try the existing columns, stacked-isometric and
staggered-side arrangements; no leader-routing strategy was added.

The automatic floor derives from the smallest positive hole/slot opening (0.8 mm
on paper) and a 20 mm longest extent, rounded up to a standard scale, with 1:10
as an additional lower bound. At the renderer's 0.5 mm outline weight, an 0.8 mm
diameter leaves 0.3 mm of visible interior. Explicit Spec scales override it.
An unavailable standard legible scale reports exhaustion at warning severity.
These thresholds are overturnable geometric proxies, documented in next-session.

The largest valid scale on the smallest fit defines its competing pair. Its next
larger sheet may compete only at a larger scale. Among complete eligible drawings,
avoidable annotation conflicts are excluded before the fill ranking. This extra
admission constraint is an explicit conservative judgement to retain the quality
ratchet and the plate's A3 acceptance; literal fill without that constraint could
choose A2 2:1 with more leader crossings than A3 1:1. The named 60–85% band, scale
preference, paper preference and fullest fallback have direct numeric unit tests.

A frozen reviewed CTC-03 Plan is a regression input, with its source hash checked
against fresh corpus Analysis. The regression forces A2 and requires complete valid
recovery, exercising the invalid seed's crossing veto independently of wording.
The normal automatic result is also checked against the latest quality table.
The title-overflow regression uses the PMI plate and a value fitting the A0 template
but overflowing its A3 recipient; packed GD&T rechecks paper geometry, title cells,
completeness and status on the target sheet. Quality.py independently measures its
Python footprint union, including furniture and bundled-font glyph boxes, instead
of copying sheet.fill. The Python regression supplies a deliberately false fill.

Named sheet-and-scale choices retain their preferred arrangement. A satisfied view
pin relation within the existing paper-boundary tolerance preserves exact coordinates;
the unchanged margin-pin regression caught a 10^-14 mm adjustment on 10000.
The dead migration branch and stray H6_RUST_GATE token are removed.

Two review minors remain explicit: the measured correction's isometric growth/top
clearance is retained because removing it produces GRM-03 fill 44.7% and breaks the
unchanged 60–85% acceptance; the existing bounded memo remains because its removal
failed the unchanged 150 ms native gate twice (175.995/180.386 ms). Neither review
finding is dismissed. Paper-economy tuning for one-percentage-point fill gains is
also left to the maintainer; no unagreed minimum-gain rule was added.

AP242 STC-09 still has no complete clean single-sheet candidate at its feature-based
floor. It reports unresolved layout at A0 1:2. Its original band regression replays
plan_escalated with every old assertion retained; a separate automatic regression
requires visible opening size and honest needs-attention/exhaustion. Tables, details
and additional sheets are the next-stage remedy, not further leader routing.

Final verification and before/after tables follow once the finishing runner completes.
