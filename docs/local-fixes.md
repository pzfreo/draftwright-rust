# Local fixes and retained layouts

A local fix keeps paper size, scale, sheet count and the view layout. Its operations
include `sheet/keep_layout`, the single sheet and view frames on which the placement
candidate was checked. This is persistent author intent, like a placement pin, and
works in Session, MCP, WASM and stateless preview/apply. Remove it explicitly to permit
sheet selection and view packing again. Sheet fixes include that removal when needed
and say: “The whole sheet is laid out again, so placements you have moved by hand may
change.”

The override contains `size`, `scale` and `views`, keyed by view ID. Each frame has
`at` (two paper coordinates), `area` (four bounds) and `scale`. The `required` list
retains shown items, including notes and tables; a covered dimension may still be
conveyed by its drawn owner. Radius-table groups remain tabulated, retaining their
centre coordinates even if an edit frees a radius-leader corridor. A batch that loses required information is refused. It constrains the
single-sheet solver, suppresses view repacking and final gutters, and checks the final
paper, scale, count and view origins/scales. Conflicting authored paper/scale or edits
that cannot retain the view set are refused with “This edit cannot keep the retained
sheet layout. No local fix is available for this edit”. No information is suppressed
to meet this constraint; ordinary overlap, fit and undrawn-item findings still apply.
Multiple local fixes carry the same frame and can be combined. Use fresh offers after
changing the sheet; a stale offer may be refused.

Offering fixes does not run recovered planning per alternative. Existing bounded
placement trials check the new ink on the settled sheet; attaching the retained frame
copies the view frames and required-item IDs once per offer; verification invokes no
additional planner. Applying the edit uses one fixed-pair solve and
verifies its frame. Allocation across multiple sheets is not yet constrained, so an
otherwise local remedy there is conservatively offered as a sheet fix. The author can
express the retained-layout choice with `sheet/keep_layout`, or remove it to restore
automatic selection; no new search policy is implicit.

`layout_replanned` in `Delta.notices` (MCP event `notice`) compares actual before/after paper, scale and sheet
count, independently of recovery's candidate diagnostics. Preview reports the change
without committing it. Notices describe the edit and are not inserted into reproducible
drawing findings; delta replay and reconstruction from the Spec remain identical. A generic edit remains free to change a sheet; its report says
so. Scope classifies the offered alternative; it does not guarantee that several
individually valid placements clear every finding when combined.
