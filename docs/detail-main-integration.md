# Detail/main integration, 2026-10-17

Latest integration: [recognition corrections and label text (#80/#81)](detail-main-81-verification.md).
The first integration below is historical; current schema accounting is in
[detail-schema-budget.md](detail-schema-budget.md).

Fetched origin and merged `origin/main` at `2eb328ce` into `cx/detail-views`.
This includes #77, #78 and #79, the alpha.4 release, PMI precision/source fixes and
WASM safeguards. Recognition corrections #80 were not in origin/main at fetch time.
The merge commit is `4d7979ce`; no changes were pushed.

Read the incoming implementation commits and their validation reports before resolving
shared planning, placement, projection and schema code. The resolution retains:

- #77's view-before-sheet line-work switches, provenance and projection-cache invalidation,
  including explicit marks/lines on orientation views. Detail views keep their automatic
  furniture convention unless the same explicit switches request it.
- #78's geometry-authored quantities, ownership, true/aligned dimensions, physical angle
  references and independent lint. Both ordinary planning and editor placement options route
  eligible authored geometry through requested crops. Aligned supports must lie in the crop.
  Precision conflicts remain errors for equal shared quantities; arithmetic baseline intervals
  retain independent display policies when their station owners have different nominals.
- #79's complete-view base Specs, measured linear corridors, transported settled annotations,
  section material/hatch proofs and caches, section refusal, fitting incomplete fallbacks
  through optional-detail recovery, one visible-precedence pass per
  nonsection view including iso, and the larger-paper schedule transaction.
- Detail crops, physical baseline intervals, break boundaries, captions, optional-detail
  rejection and principal-first recovery from this branch. Complete-frame and required-detail
  switches remain separate fields of one internal view policy.

The corridor composer uses crop extents for detail blocks. A section partner follows an
expanded side corridor beyond its final edge, preserving any existing positive gap and
adding a positive numerical separation to touching estimates. Collision predicates and
clearance thresholds are unchanged. This prevents an otherwise readable A3 assembly from
falling through to large paper when the side corridor grows into the section's old band.

Generated MCP JSON was discarded as merge material and regenerated exclusively with
`tools/generate_mcp_schemas.py`. Main's `spec_read`, `hoist_objects` and `group_properties`
are retained together with typed detail records. Exact byte accounting is in
[detail-schema-budget.md](detail-schema-budget.md).

Both sides' changelog entries and catalogue ports are retained. Counts recomputed from the
merged YAML are 35 pending, 284 ported, 125 translated and 71 retired (515 entries).
`tools/ci-quality-baseline.json` is exactly main's incoming baseline; the leader-route
baseline and instruction budget have not been changed.

The connector recovery adds a general printed-step floor: every dimensioned turned-step
length or baseline replacement must span at least the 2.1 mm printed capital height (0.7 of the 3 mm annotation em) at its owning
view's scale. Notes for small edge treatments do not determine the whole-sheet scale;
a detail can satisfy the floor at its own enlarged scale. Automatic candidate, seed and
multi-sheet supplement gates all enforce it. Explicit scales remain authored choices and
report `legibility_floor_breached`, extending ARCH0004-3's existing warning behavior.
A complete, readable drawing on A3 keeps practical paper rather than moving to A2 solely
to improve low fill. Named paper and scales remain explicit overrides.
