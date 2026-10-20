# Local fixes and sheet intent

A local fix sets the existing `sheet/size`, `sheet/scale` and `sheets/max` fields
(the current implementation offers local fixes only on one sheet). It also sets
`sheet/keep_layout: true`: keep those explicit choices without entering automatic
sheet selection, view repacking, measured refits or final gutters. Measured linear
reservation bounds still include the complete drawn ink, without re-packing carriers.
Paper and scale alone do not bypass those later layout stages, so the boolean is necessary.
The Spec contains no captured page coordinates or required-item lists. It remains
readable author intent and reconstructs independently of an editor session.

This is not a stored snapshot of view frames. The fixed-pair planner derives views
from the Spec and geometry deterministically. If that changes any view's origin,
area, scale or sheet, the edit notice names the moved views. A local edit must keep
paper, scale and count; a mismatch or lost required information refuses the edit.
Preview and apply compare before/after shown items and conveyed dimensions, including
radius-centre table rows, in memory. Nothing from that inventory enters the Spec.
Stateless preview/apply performs the same comparison by reconstructing the input Spec.
Independent offers can conflict when combined; preview checks the combined result.

Moving items on the carrier of an automatic radius-centre table cannot cheaply prove
that its withdrawn leaders would still require the same table. Those alternatives
are conservatively sheet scope; edits on independent carriers can remain local.

Existing bounded placement trials compare the before/after inventory and findings
on settled ink without invoking a recovered planner per alternative. Offers on the same
immutable drawing share their baseline collision check and material footprints. Repeated
item/placement trials reuse their complete findings, in a cache capped at 128 entries
for that drawing. New findings
of any code are included in the alternative's description, with their messages and
codes. After finding a trade, the search checks at most three further candidates for
a clean alternative before returning that disclosed trade. Errors are rejected rather
than offered. This limits verification cost. Actual apply remains authoritative for
interactions with other placements and information retention.

A later explicit paper, scale or sheet-count edit releases an inherited
`sheet/keep_layout` guard. Setting the boolean explicitly in that same batch keeps
the local promise and refuses conflicting sheet choices. Thus stateless edits can
progress from a local repair to an intentional sheet change without a hidden lock.
Set `sheet/keep_layout` to false or remove it to resume automatic layout. A sheet fix
includes removal when necessary and says: “The whole sheet is laid out again, so
placements you have moved by hand may change.” Multi-sheet or conflicting authored
paper/scale remedies are conservatively offered as sheet fixes.

`layout_replanned` in `Delta.notices` (MCP event `notice`) compares actual before/after
paper, scale, count and view frames. Preview reports changes without committing.
Notices describe the edit and are separate from reproducible drawing findings;
delta replay and reconstruction from the Spec remain identical.

MCP problem pages retain the preferred operations, scope and complete description
when the full list of alternatives exceeds a row's byte budget. `detail_ref` reads
all alternatives and explanations; a verbose trade must not hide the problem's
code or actionable bundle behind an ID-only row.
