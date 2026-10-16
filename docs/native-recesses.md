# Native section recesses

The default reader consumes `quiddity-rust/recognition/3`. `section_recesses`
records use Python's `SectionRecess` geometry, classification and evidence. Their
constituent faces anchor drawing IDs; source projection refusals go to
`Recognition.refusals` and remain explicit lint findings. Step levels and risers
are retained as evidence. Source pocket, channel, blind-slot and passage families
are represented by their unified projection and no longer produce duplicate
unrecognised-family warnings.

Both readers retain `section_recess_patterns`, resolving their occurrence indices
to member feature IDs. Missing, repeated, multiply-owned, ambiguous and non-pocket
members are refused. The drawing verifies member sizes and the lattice before
crediting a grouped callout, count, pitch and absolute centre locations. Pattern
members retain their source records and share one requirement set. Missing pitch
reopens the member-location finding. Rounded pockets retain four corner-radius
requirements per occurrence, independently of their size/depth annotation. Pattern corners use one counted radius callout carrying their matching blend
claims; the same blends are excluded from blanket radius notes. Lone pockets
reuse four matching vertical blend owners through their note, callout or located
table. Without such records, a counted pocket-radius callout owns all four corners.
Matching uses faces, axis and radius and prevents duplicate physical statements.

Python has no location compiler owner for non-Z pocket patterns; those remain
explicitly unverifiable here too. Lattices outside the drawing's principal axes
also remain unverifiable rather than receiving invented locations. Individual
side-opening pockets get both in-plane locations in the view looking into them.

Five pending catalogue rules are ported: LINT-pocket_pattern_coverage-requirements,
LINT-code-pocket_pattern_requirement_state, ISSUE-885-a, ISSUE-1485-a and
ISSUE-1485-c. The default-path tests also exercise ISSUE-1245-a, ISSUE-1246-a,
ISSUE-917-a and ARCH0015-1. Existing bridge oracle tests stay in place.

The dependency tag in Cargo.toml remains v0.1.0-alpha.4. Development uses an
uncommitted .cargo/config.toml path override for both quiddity and haecceity.
Merge quiddity first, release it (or use its final commit), then move both pins
together and regenerate Cargo.lock before merging draftwright. This branch
requires the new document and cannot merge against the old release pin.

For this quiddity commit, the exact development-to-merge change for **both**
dependencies is:

```toml
haecceity = { git = "https://github.com/pzfreo/quiddity-rust", rev = "1b171f4d678966945193047f871988295a2667c1" }
quiddity = { git = "https://github.com/pzfreo/quiddity-rust", rev = "1b171f4d678966945193047f871988295a2667c1" }
```

Alternatively pin both to the release tag containing that commit. Remove the
local override and run `cargo update -p quiddity -p haecceity`, then commit the two
pin changes and Cargo.lock together. The released tag pin is intentionally kept
in this branch until that upstream commit is available; neither repository is
pushed by this task.

All required families are published. Families still published but not consumed by
this drawing reader are `oriented_chamfers`, `circular_face_patterns`,
`thin_wall_bodies`, `interior_voids`, `sheet_metal_bodies`,
`repeating_radial_profiles`, and `freeform_surfaces`. Their per-part records remain
reported; this change does not imply drafting grammars for them.

Verification and the remaining merge blockers are recorded in
[native-recesses-qa.md](native-recesses-qa.md).
