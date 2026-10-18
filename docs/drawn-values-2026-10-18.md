# Faithful nominal values and interrupted single flats

The starting revision was `d96f067b`. The report's bugs 3 and 4 reproduce using
`/Users/paul/steps/GRM-02_shaft.step` (SHA-256
`e5babf6b7de9137adc35f90a0087530ee07143ec3a56c68982125068f672a757`).
The unchanged STEP is retained as `tests/fixtures/parts/grm02.step` for regression tests.

Default nominal formatting intentionally departs from Python's `_fmt`: remove noise by
rounding to 1e-6, then choose the shortest decimal within 1e-6, with at most three places.
`4.75`, `7.4`, `12`, and `247.65000001 -> 247.65` are examples. If no faithful decimal fits, all three places are printed (for example, `4.0001 -> 4.000`).
A live numeric carrier
that needs more than three places produces an info `nominal_rounded` problem naming
its dimension/measurement, modelled value, and printed value. Spec precision still wins;
tolerance magnitudes keep their independent formatting. Callouts, dimensions, ordinate
and hole-table cells, chamfers and generated nominal notes use the same policy. The lint
coverage/parameter and PMI formatters independently implement that policy; profiled-bore
lint uses the lint parameter formatter. Flat auto-decimal padding is replaced by the common
nominal policy so it cannot silently round a third decimal.

Quiddity's pinned `features/flats.rs::discover` emits an occurrence for every candidate
face (`defining: vec![cand.face]`); it calculates `across` as opposed-plane distances when
an opposite face exists, otherwise plane-to-axis plus stock radius. GRM-02 therefore has
two provider records, faces `[6]` and `[2]`, both the z=-3.4 plane. Draftwright previously
admitted both records and unconditionally suffixed every flat callout with A/F.

Draftwright's bridge now combines coplanar faces on the same stock, before exposing
stable flat IDs, and retains both face claims and a rebuilt fingerprint. Stock direction
and span are canonicalized, and different stock regions remain separate. Both the native
and Python bridge paths perform the repair. No quiddity source or dependency pin changed.
The bridge records whether parallel flats actually oppose each other across that stock's
axis. A/F requires this proof. Single flats keep the existing Python `FlatFeature` D-height
convention (flat-to-opposite-OD, 7.4 here), without A/F. Python's `model/ir.py` documents that
convention; its `annotations/from_model.py::_flat_label` incorrectly applies A/F even to it.

Live linear dimensions with an endpoint on the single flat's actual plane can carry its
height or axis-offset measurement. GRM-02's 7.4 envelope height and 3.4 hole-axis location
therefore state the one flat when its callout is omitted. The claim reaches both completeness
and lint. Regression tests remove the height too to verify axis-location coverage alone,
and then remove every carrier to ensure a real missing flat is still reported.

The archived Python captures remain unchanged. Exact label assertions and the dimension/PMI
parity ledgers now record the faithful nominal text; the ledgers retain exact measurement,
view, count, coverage and tolerance checks. Hole tables additionally compare every cell
against explicit new nominal snapshots before replaying the historical-format comparison.
Old `nominal_rounded` findings that merely described one-decimal rounding are removed;
three-place-limit information remains required where appropriate.

Longer labels exercise the existing layout rules. Linear text can use outside arrows without
moving measured tips, and a short label can overhang a tip slightly while respecting peer
witness clearances. Dimension-line run-ons stop just past their arrow tails. Final label promotions preserve semantic lane order. An inward overall extent may spend
the same two-lane movement bound beyond the old footprint to restore its outermost position;
page fit, contact and clearance gates remain mandatory. Final callout repair checks the complete live
inventory, including near-parallel and arrow-shaft contacts, without adding material,
furniture, text or extension contacts. Interior attachments and visible-profile proofs remain
protected. Retained session caches separate prepared reference data from policies activated
by the current planning invocation; an earlier automatic solve cannot change a later explicit-scale edit. No new accepted leader or ink defects are added.

M10000 retains A4 at 1:1 and its size/location/overall order. A centred outside-arrow
alternative must fit its own span, and no new slide may cross its own witnesses. Longer labels reserve their actual footprint by reclaiming vacant neighbouring
bands; every neighbouring view, mark, dimension, GD&T glyph and isometric caption remains protected. This
removes the outside-area findings for string_post (its longer `3.75` ordinate and the
previous 1 mm head overrun now fit the measured reservation), and reduces cgb245's
old estimated-band findings from four to one. The strict placement ledger removes those
obsolete findings; no new geometric exception is added.

Early established reservations use live nominal widths. A separate private late
comparison keeps compact text only for geometry and paper-length budgets; it is
never rendered. Incoming placements are remeasured using live terms. Inside-arrow
placements that a longer label outgrows stay free for the accurate solve. The
comparison includes furniture settlement, restores its formatter after panics,
and has separate cache keys. Imported recognition and semantic PMI keep their
usual layout.

A late comparison for automatic native dimensions contributes current and established
attachment rays after the complete ink inventory has settled. It keeps paper shoulders
and length budgets independent of projection scale, and respects actual pinned items.
Short shoulders and bounded local moves can fit longer labels between witnesses or into
vacant neighbouring paper. New text contacts are checked by owner, so exchanging one
collision for another cannot pass just because their counts match. Arrowheads, padded
labels, outlines, material, furniture, and protected neighbouring ink are checked on the
live geometry. Already clear routes stay fixed. The rescue hole table has a separate
regression verifying its actual rendered cells and precision-limit information. A revived
coincident dimension inherits its survivor's final lane while retaining its own physical
witness origins.

The axis-overrun regression prescribes its label across the live axis end because automatic
stations can change with longer neighboring labels. Its shelf gap, physical span, opposite
overrun, minimum retained overrun, and axis-column assertions are retained. The MCP fit
scenario identifies the same through bore by its new printed `8.094` rather than `8.1`;
its feature identity, H7 requirement, and preservation checks remain unchanged.

The resumed run merges `origin/main` at `9422945b`, including arc leader,
user-facing text, projection alignment and title-field fixes. The growth-repair
attachment carries the complete arc span, and restored coincident lanes retain
the upstream author-edit and view guards. Verification results and the final
label census are recorded below.


Merge validation exposed a GD&T improvement on the imported STC-09 drawing:
only one control frame remains outside its reserved band, down from two.
`gdt_band` now requires exactly one; its frame-kind, clear-strip explanation,
datum, on-sheet and foreign-view assertions remain intact. The split-grid
fill expectation stays at main's exact 52.2% after integration.

Growth repairs require an outward shoulder and use the initial router's profile
sites, including section/detail transformations. A retained proven attachment
cannot jump to a different occurrence when its elbow moves. A shorter route that
still crosses a peer does not preempt the existing bounded clear-route retry.
Physical-ink, page, length and author-pin checks remain mandatory.

Native perpendicular witnesses use the nearest already-claimed member of a shared
span. A straight row's constant-coordinate location can likewise use any member;
its along-row location still uses its first member. Each new witness is a subset
of the old stroke: the measured stations, value, arrows and text stay fixed.
This removes unnecessarily long witnesses blocking CTC-04's callouts. Authored
placements, oblique witnesses, sections and details retain their geometry.

New-crossing comparisons retain original CAD-edge identity across sampled chords.
Moving across another chord of the same curve is not a new physical edge crossing;
foreign CAD edges remain forbidden. A regression requires both outcomes while
retaining the independent raw-chord intersection checks. Metadata stores only
curve bounds and chords, without a cache per curve.

Unchanged fillet callouts are compared with complete canonical claims after blend
consolidation, preserving their rays during a tolerance edit. Merely permitting
another sheet (`sheets.max`) keeps the same nominal placement comparison; other
sheet-allocation constraints retain their own solve.

The CTC-04 leader ledger removes exactly two obsolete pattern-9 crossings
(pitch of pattern 2 and length of slot 4). No new crossing is accepted. Exterior
shoulder repairs explicitly reject a new crossing of their own view box, even
when another hard blocker would disappear.
