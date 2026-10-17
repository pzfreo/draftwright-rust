# Oriented bevels and bounded V-notch dimensions, 2026-10-17

Baseline: `adf92b4c`, the parent of `bdd7e4a9`. Evidence is under
`target/oriented-chamfers/` in this checkout; PNGs are rendered at 150 dpi with the
same default Spec before and after. `binary-hashes.json` records both executables.
The family scan (`scan.json`) found one oriented record on mfcadpp/1007 and four
each on CTC-01 and CTC-04.

Implementation source: `ee8f98d` (including `105398e1`, `7620815`, `b2b9bc59`
and `3e82826`).

## Source behaviour and drafting choices

Python quiddity's `OrientedChamfer` publishes the support-plane legs, their unit
directions, the run and its length, support spans, a virtual sharp corner at the
run midpoint, and the bevel face's centre. Its angle is `atan2(leg2, leg1)`.
Python draftwright defers the family (`consumer_semantics_deferred`, issue 1365).
CTC-01's four records are the hex boss's 5 × 5 mm, 45° top-edge bevels, with
corner z=50 and run 51.96 mm, independently of the plate's notches. Python's
front view states only `2× C5`.

Python's `profile_angles.py` selects only the foremost camera-facing planar face
per body/view. On CTC-01's plan that is the hex top: the HEX callout defines its
120° corners, and the plate notches never enter that rule. General automatic
profile angles remain pending in `docs/rules/lint.yaml`; no catalogue completion
count changes. The bounded notch grammar deliberately adds information Python
currently omits, without reinstating stream K5's general angular planner.

The bevel consumer claims the defining faces and states true section sizes:
equal legs at 45° keep C notation; other bevels state first leg × section angle.
Equal principal and oblique bevels share one counted callout, keeping the existing
principal owner's ID. Bevels remain edge treatments located by their drawn
supports. A run midpoint is not a design datum, so no automatic station is
invented there; an explicit dimension scheme may still target the virtual corner.

Symmetric, non-right outer-profile V recesses with a principal mouth and depth direction
receive an included angle in the true normal view, theoretical-sharp depth
(including tangent rounded roots), and minimum-datum mouth-centre stations.
Coincident statements share ink while retaining every claim. Angular footprints
and the added vertical notch lanes are reserved before sheet packing, including
acute angles. A silhouette-mouth datum leaves its witnesses on that free side,
preserving short existing leaders; notch arcs do not evict established dimensions.
Mouth stations distribute ink between true aligned projections: the left X=290 station
uses plan, the right X=510 station uses front, and their shared Y=100 station uses side.
This retains CTC-01's A3 sheet and leaves its bore-leader corridors clear. When front
carries GD&T, both X stations stay there, preserving its existing shared frame band.
On AP242 CTC-01 that keeps the perpendicularity frame above front and datum B's
short route, rather than displacing the frame left and making B remote. Angles can occupy clear exterior voids within the plan envelope
only with whole-label exclusion from every projected planar face and every curved
face's full bounds, plus normal edge, shaft and arrow clearances. Extra physical
quarter-flat sites on a bevelled polygonal host are established by shared source
edges. Radius leaders displaced by the new bands consider model strokes before
assignment. Existing concave radius-centre schedules with at least four exact, end-on centres
in one view are tried within the already chosen sheet and scale, allowing another
existing arrangement. Principal fillet owners and authored radius presentations are
excluded. CTC-01 keeps its four-row R25 table.
Asymmetric profiles, inner wires and arbitrary sloping pairs grant no
notch authority.

## Drawing inspection

| Part | Before / after PNGs in `target/oriented-chamfers/` | Drafting assessment |
| --- | --- | --- |
| NIST CTC-01 | `ctc01-before.png`, `ctc01-after.png` | Both 60° angles read on the notch-support extensions in clear plan space; `2× 50` measures theoretical-sharp depths. Mouth centres are at X=290 in plan, X=510 in front and Y=100 at the side, from minimum datums. `2× C5` becomes `6× C5`; `C50` remains independent. Claimed faces increase from 77 to 85 of 139. Existing slot-width text stays inside its own spans. A3, 1:5. |
| NIST CTC-04 | `ctc04-before.png`, `ctc04-after.png` | Four oblique bevels join the eight existing bevels as `12× C5`. The existing principal callout carries them without extra datum clutter. A1, 1:5 and existing tables/layout retained. |
| mfcadpp/1007 | `1007-before.png`, `1007-after.png` | A new `C0.1` leader identifies the tiny bevel. Existing hole and recess labels remain readable, with no annotation findings. A4, 1:1 retained; no run-midpoint datum is invented. |
| mfcadpp/10020 | `10020-before.png`, `10020-after.png` | The new 21.2° V angle and 20.0 mouth station complete the notch; its existing `10.6 × 10.6°` callout owns the same 10.6 depth, so no duplicate depth line is printed. Reserving the exterior arc keeps it inside its band and clears the old chamfer leader crossing. A4, 2:1 retained. |
| mfcadpp/10190 | `10190-before.png`, `10190-after.png` | A new 75.0° angle, 2 mm depth and 20.4 mm mouth station define the lower V in the true side view. A1, 10:1 retained, with no ink, off-sheet or detached-text findings. The depth/station witnesses pass through projected hole/slot regions while remaining clear of annotation text. |
| mfcadpp/10046 | `10046-before.png`, `10046-after.png` | No bounded notch candidates; before/after PNG bytes are identical. |
| NIST FTC-08 | `ftc08-before.png`, `ftc08-after.png` | No bounded notch candidates; before/after PNG bytes are identical. |

The 65-part survey also draws these bounded notch statements on CTC-01's
`nist_ctc_01_asme1_ap242-e1` and `nist_ctc_01_merge` fixtures. Across that survey,
the other physical parts with notch statements are 10020 and 10190; 10190 is
outside the 30-part quality manifest. The additional PNG pair documents its new
information separately from the 300-cell comparison.

## Ownership and tests

`tests/oriented_chamfers.rs` exercises omit, tolerance, placement and label text
on the new angular, depth, notch-station and counted-bevel dimensions. It checks
that authored angles transfer ownership, auto mode names the duplicate owner,
and a datum scheme replaces the automatic notch station exactly once. Lint
remeasures the support geometry, rejects wrong nominals even when a tolerance
contains the right digits, and derives scheme coverage only through truthful
printed links. A corrupted scheme link loses its derived notch coverage. AP242 CTC-01's source
`60 ±0.5` on faces 15/17 owns the right-notch angle across Z=−25/0 normal sections;
its automatic duplicate is covered. The exact physical flank pair, projected vertex,
support directions and remeasured angle establish ownership. Its implicit degree unit
requires that independently proved typed PMI claim in the true normal view. Wrong
nominals and removed type claims lose credit; omitting PMI restores the automatic angle.
The existing PMI angular test now requires exactly both measurement claims and exactly
two printed angular items, retaining every ink geometry assertion. A second ownership
test checks 10020's existing bevel first leg: its physical flank, mouth-end corner,
principal run, both leg sizes and half-angle establish the same depth span within the
provider's three-decimal legs/corners and two-decimal angle. The callout retains its ID;
wrong first nominals lose depth credit and hiding the callout restores the depth line. The source-only
notch and bevel derivations live inside the unchanged lint independence guard;
planner caching and the conversion to angular ink remain outside lint.

The targeted eleven-test suite passes, including editor placement-view options and
proof that distinct pinned angular lanes produce distinct ink. A unit test rejects
an allegedly empty recess covered by an oblique planar face or an end-on cylinder.
The existing leader, structural, MCP, hole-table, trust and drafting assertions
also pass unchanged. Overall tolerances do not alter profile statements. Profile IDs
use the envelope owner with stable support stamps, independently of the carrying view;
measurement claims retain each source notch identity. Exact-count parity ledgers require the new CTC-01 dimensions
and the four missing bevel claims on Python's unchanged sheet. Failed optional
schedule probes do not introduce a drawing problem when all original hole
requirements remain legible; explicit table requests and losses still report them.

Edited long labels can retain their
settled along-span position when a new notch lane lets the whole dimension move
clear of a callout. Existing text-to-span relations may be retained; no new relation
or contact is allowed. The movement remains bounded to two paper lanes. Unchanged
callouts are not restored into a new contact with edited linear ink, and qualified
HEX labels can use shorter shelves through the same collision gates.

The CI locality check also protects CTC-01's existing slot widths. Their labels
return inside their own spans before a contact-only improvement may retain an
old detached station. A single extra 1 mm text lift can clear the neighbouring
R20 label while retaining the dimension line, arrows and source span. All original
contact, page, witness and movement predicates still apply; this additional
choice is restricted to bounded-notch parts and displaced labels. The CTC-01
regression requires both width labels wholly between their own arrow tips.


## Quality and dense-part comparison

The final 30-part comparison against `tools/ci-quality-baseline.json` passes all
300 cells, without exceptions. Ink findings decrease from 17 to 15: CTC-01 goes
from 3 to 2, and mfcadpp/10020 from 1 to 0. Every other cell is unchanged. In
particular, 10046 remains at 0 ink findings and FTC-08 at 1, with no off-sheet
items or new angles on either. Their before/after PNG SHA-256 hashes are identical
(`target/oriented-chamfers/png-hashes.json`).

`tools/check_leader_routes.py` passes all 119 callouts on all 30 parts: no length
increase over the existing +2 mm limit, no added hidden/section/cutting-plane
crossings, no new runs over 2 mm, and no part gains extension contacts. Twelve unchanged
pre-existing runs remain. Source: `checks/quality-comparison.log` and
`leader-routes-verified.json` under `target/oriented-chamfers/`.

## Verification

The final serial run uses `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`,
`DRAFTWRIGHT_CORPUS=/Users/paul/repos/quiddity/tests/corpus` and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`. Node v22.23.3 matches CI's major version;
Rust is 1.99.0 on macOS arm64. `checks/runtime.json` binds the source commit.
Commands, exit status and timing are recorded in `checks/results.json`; each
acceptance directory also retains its fixture hashes, binary hash and step results.

All native checks passed on the implementation source above. The full suite
passed **1,269 tests, zero failed, zero ignored**, with local latency gates enabled.
No tests, thresholds, quality/performance baselines, leader-route allowances or CI
workflow were weakened. The parity ledgers require the newly drawn information
and missing claims on Python's unchanged sheet.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --release -- -D warnings` | PASS |
| `cargo test --workspace --release --no-fail-fast` | PASS: 1,269 tests, including both workspace crates and doc-test harnesses |
| Fresh 30-part `tools/quality.py` and `ci_acceptance.py --check-results quality` | PASS: all 300 cells no worse; two improved |
| `tools/check_leader_routes.py` | PASS: 119 callouts, all 30 parts |
| `tools/check_plan_time.py` through its CI acceptance job | PASS: all 65 parts, below the unchanged instruction limit |
| Required renders and dense-part PNG comparison | PASS: seven before/after pairs inspected or matched to inspected bytes; dense pair byte-identical |

Every acceptance job uses `python3 -I tools/ci_acceptance.py` with the workflow's
`--job`, `--pin-cell` or `--survey-shard` arguments and a fresh output directory.
The nine pin cells include 270 part runs, fresh same-binary baselines, leader
routes, locality checks and deliberate-regression rejection. The four survey
shards partition all 65 inputs (17, 16, 16 and 16 parts), with zero repeated spans.

| CI acceptance job | Result | Wall seconds |
| --- | --- | ---: |
| `quality-pins-1-1` | PASS | 193.21 |
| `quality-pins-1-3` | PASS | 173.33 |
| `quality-pins-1-8` | PASS | 180.56 |
| `quality-pins-2-1` | PASS | 215.60 |
| `quality-pins-2-3` | PASS | 218.25 |
| `quality-pins-2-8` | PASS | 160.19 |
| `quality-pins-3-1` | PASS | 188.56 |
| `quality-pins-3-3` | PASS | 187.34 |
| `quality-pins-3-8` | PASS | 154.64 |
| `mcp` | PASS on unchanged retry | 13.04 |
| `plan-time` | PASS | 220.64 |
| `render-tail` | PASS | 352.35 |
| `repeated-spans-1-4` | PASS | 33.66 |
| `repeated-spans-2-4` | PASS | 48.58 |
| `repeated-spans-3-4` | PASS | 39.11 |
| `repeated-spans-4-4` | PASS | 352.56 |

All 16 accepted reports bind the same release executable,
`c48277fbbff2a18f0b1cde97748e5989d535e42cac66bba1fe8d475cc74f5049`.
`target/oriented-chamfers/final-validation.json` checks that binding, the complete
job set, full-suite counts and all seven PNG pairs. The initial MCP report and its
passing retry are both retained.

The 65-part `tools/check_plan_time.py` gate passes on `darwin-arm64`: measured
2,848,351,077,895 retired instructions against the unchanged
3,165,755,533,285.35 limit (1.035 times the recorded baseline; allowed 1.15).
Wall time was 220.5 seconds. Evidence:
`target/oriented-chamfers/acceptance-plan-time/plan-time/plan-time.json`.

The first MCP job's Python counter self-test reported 40,177,743,847 paused-child
instructions against its required value above 41,921,719,518.6 (90% of its
uninterrupted child's count). Its functional MCP run, Node client tests and
Python quality tests passed. The counter test and its bound remain unchanged;
the complete unchanged retry passed all 24 Python acceptance tests, the Python
quality tests, Node client tests and functional MCP checks. Its report is
`acceptance-mcp-retry/summary.json`; the initial report remains in
`acceptance-mcp/summary.json`. Both are under `target/oriented-chamfers/`.

The `render-tail` acceptance job also passes its unchanged instruction limits:
cgb243 uses 2,830,658,999,458 instructions (1.055 times its baseline; limit
3,085,278,430,843.35), and threaded_connector_109 uses 4,115,440,055,392
(1.008 times its baseline; limit 4,694,428,036,262). Evidence:
`acceptance-render-tail/render-tail/render-tail.json` under the same artifact root.

The final changes are committed in coherent steps with the requested Codex
co-author trailer. No push was performed.

## Native-recess main integration

The later merge with #85/#86, five-part render review and local verification are
recorded in [the main integration report](oriented-chamfers-main-merge-2026-10-17.md).
