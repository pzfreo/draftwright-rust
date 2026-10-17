# Native recess layout repair

The comparison is against `699acae7` (released recognition), using the unchanged
30-part manifest and quality baseline. The initial native results are retained in
`/tmp/section-recess-review/after-final`; the original drawings are in `before`.
New information does not excuse moving an existing item into clutter. Counts below
are the initial regression cells, not approved baseline changes.

The repaired revision `8083adb` has no worse cell across all 30 parts and passes
both leader-route comparisons without a quality baseline change. The attribution,
repair mechanisms, measured results and inspected drawings are recorded here;
the required final serial test results are recorded at the end.

| Part | Initial worsened cells (before → native) | New information | Existing-item regression / cause |
| --- | --- | --- | --- |
| 1000 | ink 0→1; extension contacts 0→1 | Rectangular pocket size/depth and two locations; hex pocket; circular seat and axis locations | New hex callout escapes the side band; new location/leader contact. Existing envelope carries the coincident 48.4 span with both claims. |
| 10000 | ink 5→6; through-feature 0→1 | Pocket size/depth and x/y locations | Existing x shoulder exceeds its band after lane insertion; existing hole y witness crosses the newly recognised pocket. |
| 10007 | ink 0→1 | Two seat arcs, common diameter/run and axis locations | Sheet changes A3 2:1→A4 1:1; existing x shoulder exceeds its band. |
| 10020 | ink 1→4; through-feature 0→3; extension contacts 5→7 | Two side pockets with size/depth and in-plane locations; hex pocket | New locations exceed side band; new pocket leader crosses its peer location and pocket. Existing hole/rung witnesses cross newly recognised recesses. |
| 1004 | through-feature 2→4 | Edge-open side pocket size/depth | Existing hole x/y witnesses cross newly recognised recess footprints; no new leader is responsible for these two findings. |
| 10046 | ink 0→1 | Side-opening pocket size/depth and two locations | Sheet changes A3 2:1→A4 1:1; new front callout exceeds its band. |
| 10047 | ink 0→4; through-feature 0→1; extension contacts 0→3; off-span 1→3 | Two round-bottom blind slots and their locations, pocket and hex pocket | Three new callouts exceed bands; pocket leader crosses overall width; new slot location crosses an existing slot; linear label settlement loses span association. |
| 10049 | ink 0→1; through-feature 0→4; extension contacts 0→2 | Three pockets, two with explicit locations | New side callout exceeds band and crosses another pocket; new location crosses the large pocket. Existing height rungs also cross that newly recognised footprint. |
| 10060 | ink 6→7; extension contacts 5→6 | Side pocket size/depth and two locations | New side location exceeds band; existing hole leader length grows 19.547→25.606 mm. |
| 10063 | ink 1→3; through-feature 0→2; extension contacts 3→4 | Front and plan pockets with locations; hex pocket | Existing envelope width and new front callout exceed band. Existing hole leader and witness cross the newly recognised plan pocket. |
| NIST CTC02 | through-feature 6→8; extension contacts 2→4 | Two seat arcs and their axis locations | New axis witnesses cross holes. Existing blend leader grows 56.134→64.139 mm; extra extension contacts are not justified by the new information. |
| NIST FTC06 | repeats 0→2; ink 5→9; through-feature 3→5; extension contacts 3→4; off-span 0→2 | Four channel widths: two 3.1 side, two 12.7 plan | Equal widths create two repeated-label groups. Existing hole pitch leaves band and chamfer text gains an extension contact; new widths leave bands, cross holes and lose text association. |
| NIST FTC08 | repeats 1→2; ink 1→6; through-feature 11→13; extension contacts 3→12 | Two seat arcs and axis locations; two 3.1 channel widths | Sheet changes A2 1:2→A0 1:1. Existing slot-radius leader grows 10→55.251 mm, gains hidden crossings and text contacts; new axis witnesses cross existing holes/slots. Equal channel widths repeat. |
| NIST FTC09 | through-feature 2→7 | Four seat arcs and shared axis locations | Three existing height-witness findings count their own newly recognised recess wall as an obstacle; the exact support-face ownership fixes those false positives. New side axis location crosses the hole bank; new front locations cross holes/slots. New seat leader runs beside ink 13.428 mm. |

Every non-NIST row requires correction, including intersections with newly
recognised footprints. Dense NIST increases may only be accepted for extra
information after hard findings and existing-item regressions are removed.
No baseline cell has been re-recorded.

## Counted attribution of the initial findings

The following event comparison reconciles every worsened cell above. `N` means
an added finding involves a dimension ID introduced by the native recess port;
`E` means its dimension IDs already existed. The last number is old findings
removed, so `N + E − removed` is exactly the cell's increase. A mixed contact
between a new witness and an old leader counts under `N`; that does not make it
acceptable. The cause column above distinguishes actual movement from a newly
recognised footprint intersecting an unchanged witness. Leader length, hidden
crossings and parallel travel are checked separately by the route ratchets.

| Part | Ink | Through feature | Extension contact | Repeated label | Off span |
| --- | --- | --- | --- | --- | --- |
| 1000 | +1 N / +0 E / −0 | — | +1 N / +0 E / −0 | — | — |
| 10000 | +0 N / +1 E / −0 | +0 N / +1 E / −0 | — | — | — |
| 10007 | +0 N / +1 E / −0 | — | — | — | — |
| 10020 | +3 N / +0 E / −0 | +1 N / +2 E / −0 | +2 N / +0 E / −0 | — | — |
| 1004 | — | +0 N / +2 E / −0 | — | — | — |
| 10046 | +1 N / +0 E / −0 | — | — | — | — |
| 10047 | +4 N / +0 E / −0 | +1 N / +0 E / −0 | +3 N / +0 E / −0 | — | +1 N / +1 E / −0 |
| 10049 | +1 N / +0 E / −0 | +2 N / +2 E / −0 | +2 N / +0 E / −0 | — | — |
| 10060 | +1 N / +0 E / −0 | — | +1 N / +0 E / −0 | — | — |
| 10063 | +1 N / +1 E / −0 | +0 N / +2 E / −0 | +1 N / +0 E / −0 | — | — |
| NIST CTC02 | — | +2 N / +0 E / −0 | +2 N / +0 E / −0 | — | — |
| NIST FTC06 | +3 N / +1 E / −0 | +2 N / +0 E / −0 | +0 N / +1 E / −0 | +2 N / +0 E / −0 | +2 N / +0 E / −0 |
| NIST FTC08 | +4 N / +2 E / −1 | +2 N / +0 E / −0 | +8 N / +3 E / −2 | +1 N / +0 E / −0 | — |
| NIST FTC09 | — | +3 N / +2 E / −0 | — | — | — |

The item IDs and event differences are retained in
`/tmp/section-recess-review/repair-initial-event-attribution.json`. The event
counts independently match the original quality totals for all fourteen rows.
They are attribution evidence, not baseline allowances.

## Placement changes

Equal channel widths share explicitly counted carriers. Each original width claim
and covered item ID remains in the plan; omitting a carrier promotes a remaining
channel. The bridge regression now checks all seven independent width claims as
well as the exact counted labels.

The lane budget now includes recess linear items and measured callout text on both
sides of each carrying view. The choice is to give the annotations real paper,
including a larger sheet where needed. A pocket callout can leave sideways near
its pocket instead of crossing an existing slot merely because its text did not
fit in the old margin. Existing slot datum legs may share their own length's lane
when their full ink is disjoint and their measured ends meet.

Linear corridor alternatives consider recess footprints and settled callouts,
with existing callouts placed before recess callouts. Text remains adjacent to
its own arrow or within its span. Covered dimensions inherit their carrier's
settled lane. Final label settlement protects the full 1 mm leader/extension
corridor, the padded label and the arrowhead, rather than only intersecting
strokes. A second bounded route check uses the final witnesses and GD&T obstacles.

GD&T reservation uses the consolidated linear inventory on recess drawings.
Outside-band GD&T participates in recovery. A bounded automatic scale reduction
is retained only if it improves the GD&T and keeps the carried measurements and
placed frames without increasing hard layout findings or dropped items. Its
basis identifies an automatic choice rather than attributing it to the user.
Eligible view blocks may be packed using their measured annotation bounds while
retaining their witness and leader geometry within each block.

No recognition record is hidden, and no acceptance threshold, route fixture or
quality baseline is relaxed. The final check results and remaining findings are
recorded below.

The captured-drawing placement ratchet is tightened: FTC06's fourteen original
outside-band allowances are removed. The full-ink reservation and unused-band
transfer now keep the counted side heights and channel widths inside their own
bands. Removed m1000, m10046 and m10127 overrun entries are improvements too.
No hard-finding allowance is added. The counted-width oracle entries retain all
seven independent width claims and are covered by the carrier-promotion tests.

The view-conflict gate keeps its original two-view counterexample and every
assertion using the complete historical recognition/2 Analysis, source SHA checked
against a fresh native reading. A separate recognition/3 replay returns the real
unsafe proposal through the production candidate memo and proves that recovery
rejects it even though the current lane solver clears the live stand-in. Neither
fixture removes a feature from its recorded recognition document.

The joint solve also distinguishes centre-line furniture from leaders: a cross
mark has no arrow and must not reserve a leader/extension corridor. Actual
callouts retain the full 1 mm corridor and the 0.2 mm arrow-flare clearance used
by route acceptance. The corridor search caches identical witness measurements
when trying several label positions.

Short existing interior radius leaders claim their corridors before new recess
witnesses. Radial retries sample finer directions around the physical end caps.
A bounded additional schedule temporarily leaves movable witness strokes out of
the new leader trial, restores every dimension, and solves its lanes and witnesses
around the proposed route; only the fully adjudicated drawing can win. New native pockets
also offer quarter stations on their straight walls, clear of rounded corners.

Channel mouths use the existing 0.8 mm opening-legibility criterion when choosing
a scale. This chooses readable geometry and additional paper when needed. PMI
reservation accounts for the complete label/symbol ink, including its leader and
arrow, rather than its text box alone.

The automatic hole presentation is kept stable when recesses are added. The
established automatic schedule remains eligible under the same rules and must
still pass installation. This avoids replacing an existing table with unrelated
inline hole locations as the annotation bands change. The comparison is cached
within the same planning call and also supplies the established scale cap.

The final corridor pass can retry the opposite physical wall or hole rim, using
the existing bounded radial sweep. It retains the existing length, hidden/model
crossing, text and arrow-clearance gates. GD&T frame trials may use proven clear
inner tiers as datums already do, with the same full obstacle grading.

Face-level witnesses retain ownership of their actual recess walls. The match
requires a principal planar face in that recess's face evidence and exact level
support bounds; equal height alone is insufficient. The FTC09 regression keeps
an unrelated crossed hole reported and rejects the same exemption for a different
height. Recognised features and their geometry remain in the drawing.

After GD&T settles, new recess linear items remain in the corridor solve while
existing slot and datum lanes stay fixed. New recess callouts can yield to settled
frame or symbol ink; existing leaders retain their original movement/length gate.

The established route comparison is prepared before the first lane solve, and
clear exterior routes can claim corridors in the first schedule. These are
preferences rather than pins: other complete schedules can win, and refinement
still checks the settled ink. A successful automatic fit must not add material
crossings to existing dimensions or add leader/extension contacts relative to the
established drawing. The practical-sheet fallback must meet those checks too.
New recess witnesses remain distinct when assessing added information.

Dense edge scans use a conservative bounding-box tree before the exact crossing,
contact and parallel-run predicates. A randomized oracle compares indexed results
with complete scans, including a run assembled from many short strokes. The wider
final radial sweep is limited to drawings with actual new recess dimensions,
preserving the unrelated turned-part route test unchanged.

The readable opening reservation applies to newly native channels. Captured
bridge documents retain their established scale policy; the original dimension
oracle passes, including FTC06's plate thickness in the side view. A thickness
must witness its actual exposed profile edge: the opposite strip cannot reverse
its witnesses across the body just because its label fits there.

The slot's location and its own length reserve one end-to-end corridor. Later
linear repair, interior trials and text promotion must preserve that relationship.
The FTC08 probe retains the exact common line and meeting endpoints after the
new recess dimensions are restored. An alias separated by a tolerance occupies
its own lane before corridor repair starts, so its carrier cannot move into that
supposedly empty lane.

Identical located seats can use one explicitly counted form callout. Every seat
retains its diameter, run, sweep and individual axis-location claims. All physical
arc attachment stations participate in the carrier's route search; omission of a
carrier promotes a surviving seat. This reduces redundant text without hiding a
recognised feature. Dimensions retain the same unrounded form values.

The captured FTC06 diameter pair may extend its established straight shaft in
empty paper while retaining a horizontal shelf. Every candidate must prove that
its complete path inside each uncertain material box is already covered by the
established segment. The original length and per-edge crossing gates remain;
the shared leader assignment counts both directed contacts of a reciprocal
shaft crossing. It clears the captured FTC06 pair entirely, allowing its last
one-crossing allowance to be removed from both independent ratchets.

Oblique linear witnesses now account for their overshoot when shifted. Their
arrow tips lie exactly on the witness strokes. The shared-line detector measures
their true intersections with the dimension line instead of skipping oblique
witnesses. A unit counterexample checks overlapping measured spans, and the
10046 regression checks every linear arrow against its physical witness.


An oblique witness's arrow-clearance strip is measured in the witness's own
axes with the same 0.2 mm pad. Its axis-aligned bounding rectangle had rejected
heads several millimetres from the stroke, incorrectly ruling out clear corridors.
The regression includes a far corner inside that bounding rectangle and actual
contacts both along the stroke and at its endpoint.

The late solve can keep established dimension and GD&T ink fixed while placing
new seat locations. An along-run midpoint may use either physical profile view;
all aliases follow its carrier's selected view and retain their individual claims.
The bounded retry varies which new location claims a corridor first. Covered
callout aliases do not acquire phantom leaders during that retry. Actual counted
carriers, promoted members and authored omission gaps retain their reservations.

Where existing ink is held, the candidate search can accept a transverse crossing
as reported crowding to keep a new witness off text. It continues to forbid
parallel runs, text contacts and arrowhead contacts. Final problem reporting and
all quality and route acceptance predicates remain unchanged. Semantic PMI keeps
its established leader-assignment objective and reservation policy.

Conservative silhouette bounds remain obstacles for interior labels and strokes.
They cannot by themselves prove a physical callout attachment: that proof now
requires a recorded model edge. The rendered-ink oracle independently checks the
actual drawing, including the leader tip, and is not relaxed.

A circular boss attachment must also be visible: a nearer planar face cannot
cover its physical rim. When all recorded end-view rims are occluded, a native
boss can state its diameter on a visible profile generator or cap rim, preserving
every counted diameter claim. Existing exterior boss statements retain their
established view; this prevents an unrelated callout from moving simply because
the newly available profile is usable. The rendered-edge regression independently
verifies the four-boss FTC06 statement's visible attachment.

The established comparison replays the earlier circular presentation only to
reserve existing routes. It cannot grant a hidden attachment to the final
native drawing. Profile routes participate in the same extension corridors,
including the route checker's full 0.2 mm arrow-flare clearance.

A route repair can also retry the implicated existing leader and the linear label
that displaced it as one complete candidate. Other established items retain
their current geometry. The full inventory is solved around those reservations;
acceptance requires fewer new physical-edge crossings, preserved claims, no new
hard findings and no increase in extension contacts, band overruns or reported
feature crossings. This prevents a shorter leader from buying clearance by
crossing extra hidden edges.

The full-suite follow-up distinguishes newly native recesses from captured bridge
inventories when retaining the established scale and hole-table presentation.
Captured inputs keep their initial column estimate, while every complete recovery
candidate reserves measured callout columns. The view oracle removes six stale
m10127 exceptions (sheet, sheet scale and all four view scales); no new exception
is added. Its centre differences remain documented independently.

Visible profile attachment is an eligibility condition. The complete label,
shaft, shelf and arrow ink also participates in candidate grading, including
extension corridors. An invalid station cannot win merely because another real
route has a reported obstruction. The independent rendered-edge test retains
its original assertions.

Existing location ladders are repaired in increasing datum distance after the
new recess witnesses choose their least obstructed physical corridors. Movable
outer peers yield during that joint pass, then return in datum order so a whole
ladder can move together onto usable paper. The ordering repair applies to the
pre-existing ladder; new recess coordinates do not constrain one another before
their routes settle. Held or authored pins and established slot chains remain
obstacles. The complete inventory and physical measurement endpoints are preserved.

Section packing recomputes centre-line page-bound findings after moving the view
and its line work together. It uses the same predicate as the initial plan; an
obsolete off-sheet finding is replaced by a fresh check, rather than exempted.
Both original 10224 single-sheet tests pass with A3 at 2:1 and no added sheet.

Native recess extension-line crossings now trigger the same bounded joint
schedules as leader crossings. A clear short form leader can otherwise cause an
early return while its location witness still crosses a hole. The shared solve
can keep the clear witness corridor first and select a longer form leader when
that reduces complete-drawing findings. The FTC09 regression retains all six
seat claims and requires the 39.6 location witness to avoid crossed features.

When a hole schedule removes the original hole locations but a new witness
still crosses a feature, one bounded complete alternative preserves the remaining
existing location corridors and reserves the new witnesses before fitting their
form leaders. Existing leader preferences are retained. Its full hard-finding,
text-span, extension-contact and material-crossing grade must beat the current
complete drawing. A clear longer new leader can then win over a shorter leader
that blocks its own location corridor.

Complete view-block packing refreshes the required isometric scale disclaimer
against the final recipient sheet, settled views and annotation ink before grading
and fill measurement. A caption cannot retain its A0 template position after its
view moves to smaller paper. Its display condition and the original structural
assertions remain unchanged.

A native recess drawing with a shared linear stack can use the same complete
settled-block trial even without GD&T. This extra trial excludes page-anchored
note, radius and hole-table blocks. It makes 10000's A4 drawing at 1:1 valid,
with its 21.0 and 25.6 shoulders on distinct lines, so the unchanged fill policy
selects the fullest allowed drawing. The all-sixteen-test sheet-selection oracle
passes, including its unsafe-view counterexample.

Required lower-corner tables reserve their paper before the views claim it. The
aligned orthographic and section row may lift together at the same scale, using
the existing preferred clearance, only when the complete row still fits. The
ordinary auxiliary placement remains available where there is no height for that
lift. The unchanged m10127 oracle now keeps its BOM at the lower-left corner on
A3 at 2:1, with all rows, columns and text preserved.

## Repaired quality comparison

The source revision `8083adb` passes the unchanged 30-part quality baseline.
The final serial artifacts are in
`/tmp/section-recess-review/repair-verified3-outputs/`. The checks below were run
one at a time against the local quiddity override. Every one of the 360
part/metric comparisons is unchanged or improved, including the two additional
route/span metrics. This is true for both non-NIST and dense NIST parts.

No dense-NIST baseline cell needs an allowance for new information. **Baseline
cells re-recorded: none.** All original and initially native placed measurement
claims remain carried, including counted aliases, notes and table measurements.
No new hard layout finding is introduced. The three existing withdrawn-radius
completeness refusals remain explicit; they are not newly introduced findings.

| Metric | Released before | Initial native | Repaired |
| --- | ---: | ---: | ---: |
| off_sheet | 0 | 0 | 0 |
| repeated_label | 7 | 10 | 7 |
| repeated_measure | 0 | 0 | 0 |
| views_overlap | 0 | 0 | 0 |
| view_on_title_block | 0 | 0 | 0 |
| ink | 52 | 76 | 31 |
| repeated_span | 0 | 0 | 0 |
| line_through_feature | 73 | 97 | 66 |
| hidden_on_visible | 2231 | 2139 | 2227 |
| section_hidden | 0 | 0 | 0 |
| leader_on_extension | 53 | 71 | 31 |
| text_off_span | 5 | 9 | 4 |

The initial fourteen regressed parts now have the following counts. Each cell
is **released before → initial native → repaired**. The event attribution above
remains the explanation of the initial regression; none is retained as an
approved regression. Existing repeated-label verdicts remain independently tested.

| Part | Ink | Through feature | Extension contact | Repeated label | Off span |
| --- | --- | --- | --- | --- | --- |
| 1000 | 0→1→0 | 0→0→0 | 0→1→0 | 0→0→0 | 0→0→0 |
| 10000 | 5→6→0 | 0→1→0 | 2→2→1 | 0→0→0 | 0→0→0 |
| 10007 | 0→1→0 | 0→0→0 | 0→0→0 | 0→0→0 | 0→0→0 |
| 10020 | 1→4→0 | 0→3→0 | 5→7→2 | 0→0→0 | 0→0→0 |
| 1004 | 0→0→0 | 2→4→2 | 0→0→0 | 0→0→0 | 0→0→0 |
| 10046 | 0→1→0 | 0→0→0 | 0→0→0 | 0→0→0 | 0→0→0 |
| 10047 | 0→4→0 | 0→1→0 | 0→3→0 | 0→0→0 | 1→3→0 |
| 10049 | 0→1→0 | 0→4→0 | 0→2→0 | 0→0→0 | 0→0→0 |
| 10060 | 6→7→3 | 0→0→0 | 5→6→2 | 1→1→1 | 0→0→0 |
| 10063 | 1→3→0 | 0→2→0 | 3→4→0 | 0→0→0 | 0→0→0 |
| nist_ctc_02_asme1_rc | 2→2→2 | 6→8→5 | 2→4→0 | 0→0→0 | 0→0→0 |
| nist_ftc_06_asme1_rd | 5→9→0 | 3→5→2 | 3→4→2 | 0→2→0 | 0→2→0 |
| nist_ftc_08_asme1_rc | 1→6→0 | 11→13→6 | 3→12→0 | 1→2→1 | 0→0→0 |
| nist_ftc_09_asme1_rd | 6→6→1 | 2→7→2 | 6→3→0 | 1→1→1 | 3→3→3 |

The complete per-part, per-metric comparison is retained in
`/tmp/section-recess-review/repair-thirty-part-quality.csv`. The raw claim and
new-hard-finding comparison is in `repair-verified3-comparison.json`. Both full
30-part route checks pass: the committed route fixture and the original released
drawings. Existing leader length, hidden/section/cutting-plane crossings, parallel
runs, extension corridors and arrowheads retain their acceptance predicates.

## Drawing review and render paths

I inspected all six before/after pairs, plus FTC09 and 10000 diagnostic pairs.
The better drawing preserves the established view scale, gives long recess
statements real paper and counts identical forms while retaining each claim.
The complete render manifest includes source SVG hashes and the individual
draftsman observations:
`/tmp/section-recess-review/repair-final-render-manifest.json`.

| Part | Initial native before PNG | Repaired after PNG |
| --- | --- | --- |
| 1000 | `/tmp/section-recess-review/repair-visuals/native-before/1000.png` | `/tmp/section-recess-review/repair-visuals/verified-after/1000.png` |
| 10046 | `/tmp/section-recess-review/repair-visuals/native-before/10046.png` | `/tmp/section-recess-review/repair-visuals/verified-after/10046.png` |
| nist_ftc_06_asme1_rd | `/tmp/section-recess-review/repair-visuals/native-before/nist_ftc_06_asme1_rd.png` | `/tmp/section-recess-review/repair-visuals/verified-after/nist_ftc_06_asme1_rd.png` |
| 10020 | `/tmp/section-recess-review/repair-visuals/native-before/10020.png` | `/tmp/section-recess-review/repair-visuals/verified-after/10020.png` |
| 10047 | `/tmp/section-recess-review/repair-visuals/native-before/10047.png` | `/tmp/section-recess-review/repair-visuals/verified-after/10047.png` |
| nist_ftc_08_asme1_rc | `/tmp/section-recess-review/repair-visuals/native-before/nist_ftc_08_asme1_rc.png` | `/tmp/section-recess-review/repair-visuals/verified-after/nist_ftc_08_asme1_rc.png` |

The initial native SVG sources are in `after-final/`; the repaired sources are
in `repair-verified3-outputs/`, both under `/tmp/section-recess-review/`. All eight
final serial-run SVGs match the inspected source SVGs byte-for-byte.

- **1000**: The hex statement now sits directly beside its side-view feature, clear of the height ladder. The seat form statement occupies its own lower corridor; the shared 48.4 span remains readable.
- **10046**: Retained 2:1 scale on A3 makes the recess and short height tiers legible. The oblique 23.0 witnesses have real arrow stations, and the pocket statement is below the view rather than over the upper band.
- **nist_ftc_06_asme1_rd**: The two equal channel-width pairs are counted rather than repeated. Existing location ladders progress outward from the datum. The four-boss diameter attaches to a visible lower profile cap; all seven channel-width claims remain. The new thin-channel witness uses a longer oblique corridor in clear paper.
- **10020**: The two pocket statements leave sideways beside their own openings. Their location labels stay on their spans, separate from the side-view outline and the front-view datum ladder. Existing hole callouts remain distinct.
- **10047**: Long blind-slot statements occupy separate horizontal corridors on A3 at the established 1:1 scale. The front pocket statement clears the overall-width tier; the reference and slot-location labels remain beside their own spans.
- **nist_ftc_08_asme1_rc**: The established 1:2 scale and hole schedule are retained. One counted seat form statement replaces duplicate form text; short existing slot-radius leaders remain close to their arcs. Added axis witnesses use outside paper and leave their labels on the dimension spans.

Additional FTC09 diagnostic pair: `/tmp/section-recess-review/repair-visuals/native-before/nist_ftc_09_asme1_rd.png` and
`/tmp/section-recess-review/repair-visuals/verified-after/nist_ftc_09_asme1_rd.png`. One counted four-seat form statement retains the diameter, run and sweep of every seat. The 39.6 witness uses the clear upper corridor rather than crossing the hole bank; existing height and slot ladders remain ordered. A longer new form leader is preferable to blocking that witness.

Additional 10000 diagnostic pair: `/tmp/section-recess-review/repair-visuals/native-before/10000.png` and
`/tmp/section-recess-review/repair-visuals/verified-after/10000.png`. A4 at the retained 1:1 scale carries the complete inventory. The 21.0 shoulder is between the two left views and 25.6 is above the plan, on distinct lines. The pocket statement leaves to the left, clear of its location witnesses. The 17.5 hole location uses a clear oblique corridor below the side view. Extra paper or a reduced scale is unnecessary.

## Final serial verification

Verified source: `8083adb1af68b3c44dafa13ffd12062572528873`; local quiddity override: `2709a331a7c0e3d80734768834ff12e36caf92f5` (clean). The source snapshot did not change during verification. The local override and `.recess-rules.json` remain uncommitted.

Every Cargo stage sourced `$HOME/.cargo/env` and used `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`. The required corpus was `/Users/paul/repos/quiddity/tests/corpus`, with `DRAFTWRIGHT_CORPUS_REQUIRED=1`. The stages below ran serially. `--no-fail-fast` collects all failures without changing test assertions.

| Required check | Result | Seconds |
| --- | --- | ---: |
| `cargo fmt --check` | PASS | 1.392 |
| `cargo clippy --all-targets --release -- -D warnings` | PASS | 0.236 |
| Required-corpus `cargo test --release --no-fail-fast` | PASS | 3282.064 |
| Release drawing binary build | PASS | 0.134 |
| `tools/quality.py`, all 30 manifest parts | PASS | 214.618 |
| `tools/ci_acceptance.py --check-results quality`, unchanged baseline | PASS | 0.055 |
| `python3 -I tools/check_leader_routes.py`, committed fixture | PASS | 8.117 |
| Leader routes against all 30 original released drawings | PASS | 8.046 |
| Explicit `cargo test --release --test rules_catalogue` | PASS | 1.669 |

The complete suite finished 107 test groups: **1076 passed, 0 failed, 0 ignored**. The separately requested catalogue run also passed all six tests.

All seven original failing tests pass with their four test source files unchanged from the task start:

| Original regression | Result |
| --- | --- |
| `frames_datum_symbols_and_labels_are_on_the_sheet_and_off_the_other_views` | PASS |
| `ctc_02_frames_and_datum_symbols_are_inside_their_bands_and_on_the_sheet` | PASS |
| `stc_09_datum_symbols_are_inside_their_bands_and_frames_past_them_had_no_clear_strip` | PASS |
| `stc09_without_a_complete_fit_keeps_its_first_drawing_and_reports_the_floor` | PASS |
| `stc09_slot_length_stays_beside_its_peer_and_gdt_corridors` | PASS |
| `a_location_that_fits_on_neither_side_shares_a_held_lane` | PASS |
| `every_repeated_label_on_the_quality_parts_has_a_verdict` | PASS |

Both wall-clock gates (`r5_multi_sheet_latency` and `nist_benchmark_deltas`) pass in the full run. Neither threshold nor test file was changed.

The final comparison confirms no worse cell in all **360 comparisons**, no new hard layout finding, and no lost measurement from either the released drawings or the initial native drawings. **Baseline cells re-recorded: none.** Both route checks retain all 119 common original callouts and accept 25 additional callouts. The eight final SVG hashes match the explicitly inspected PNG sources.

The original seven-test files, quality baseline, route fixture, quality and route predicates, latency tests, Cargo pins and lockfile retain their task-start hashes. Their proof is `/tmp/section-recess-review/repair-unchanged-acceptance-files.json`. All serial commands, exit codes, timings and logs are indexed in `/tmp/section-recess-review/repair-final-verification.md`; the machine-readable report is `repair-final-report.json`. The eight rendered pairs and draftsman observations have clickable paths in `/tmp/section-recess-review/repair-final-render-table.md`.

**Merge readiness:** the drawing repair meets the layout, information-retention, quality and route bar. The branch still requires upstream publication of the local quiddity revision, both dependency pins updated together, and a regenerated `Cargo.lock`, as specified in [native-recesses.md](native-recesses.md). It is not merge-ready against the checked-in alpha.4 pins. No push was performed.
