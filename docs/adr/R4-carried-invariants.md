# ADR R4: Carried invariants

**Status:** Proposed. The maintainer has not yet reviewed the rules catalogue.

**Covers:** every rule of the Python ADRs 1–5 in [`docs/rules/adr.yaml`](../rules/adr.yaml) (94
entries: numbered invariants `ADR<n>-<k>`, decision-level rules `ADR<n>-D<k>` and unguarded rules
`ADR<n>-U<k>`), and in summary the 66 rules taken from the archived ADRs (`ARCH*`).

## Decision

Each Python rule gets one disposition:

- **kept:** the rule holds in draftwright-rust as it stands. Either it is already an R1, R2 or
  R3 invariant (named), or it is a **pending port**: it is owed, and is checked against its
  catalogue id when the code it governs lands.
- **translated:** the rule's purpose holds, restated in terms of the Analysis, Spec and Plan
  documents ([model](../model.md)).
- **retired:** it governed Python machinery that the new model does not have.

The catalogue's `port_note` was the starting point. Where this record disagrees with it, the row
says so. The catalogue's `status` fields stay `pending` until this record is accepted; the
dispositions are then copied into it (`rust_rule:` for translated, `why:` for retired), and each
port sets `ported` with its `rust_test:` as it lands.

Totals for the 94 live rules: **kept 54** (seven already R1 or R2 invariants or decisions, one a
documented limitation, 46 pending ports), **translated 32**, **retired 8**.

## ADR 1: the compiler pipeline

| Id | Rule | Disposition | Where it lands / why |
| --- | --- | --- | --- |
| ADR1-D1 | One engine; every surface is a front door onto one build path. | translated | R1 decision: the CLI, service and MCP adapters are thin over the five operations; no surface has its own pipeline. |
| ADR1-1 | The import graph is a ranked DAG. | translated | Module dependencies point downward (`output` and `text` know nothing of `model`). Differs from port note: a crate split for its own sake is not needed while the code is one crate; the one boundary that must be hard is lint's (ADR1-9). |
| ADR1-2 | Drawing is not a state bus. | retired | Operations are pure functions over documents; there is no mutable drawing object to protect. |
| ADR1-3 | One feature inventory per build, detected once. | translated | The Analysis is the only feature inventory; `plan` (including page and scale) reads Analysis and Spec only. |
| ADR1-4 | Detection and declaration feed one waist. | translated | Analysis features and Spec-declared or overridden features resolve to one feature set before planning; nothing downstream branches on producer. |
| ADR1-5 | Recognition records cross at one fail-closed adapter. | translated | One adapter in `analyse` (`src/bridge.rs`) from quiddity records to the Analysis: a closed `match` gives each known record type one home (feature, evidence or refusal), and a read that meets any other type fails. The records arrive as JSON, so the failure is at read time, not compile time. |
| ADR1-6 | Orientation is data: a feature's view follows from its axis by one rule. | kept | Pending port: view routing is a pure function of axis and measurement kind, stated in the item's basis. |
| ADR1-7 | Dimension passes consume planner output; exceptions enumerated. | translated | R1 invariant 2. The exceptions (PMI, GD&T, the gear table) become Plan items with a source-authored basis, so there is no second channel to enumerate. |
| ADR1-8 | One owner per physical measurement; never dedupe by value. | kept | Pending port: planner reconciliation; the yielding dimension is a Plan omission with basis `conveyed_by <id>`. |
| ADR1-9 | Lint does not import the drafting model. | kept | Pending port. Lint takes Analysis and the rendered sheet, never Plan or Spec, as its denominator. Decided (2026-10-07): a module inside draftwright, guarded by a test that fails if lint reads the Plan or Spec; split into a crate if it grows. |
| ADR1-10 | Expired compatibility surfaces stay deleted; remaining ones carry a removal date. | retired | None of the Python surfaces exist. Differs from port note: the dated-shim principle is replaced by schema versions, and an unknown schema is refused (ADR3-4). |
| ADR1-11 | Reporting sits beside lint on one requirement ledger. | translated | Report and completeness read one requirement ledger computed from Analysis and the rendered sheet. |
| ADR1-12 | Private test reads are a shrinking allowlist. | retired | Rust visibility makes it a compile-time property. |

## ADR 2: sheet layout and view planning

| Id | Rule | Disposition | Where it lands / why |
| --- | --- | --- | --- |
| ADR2-D1 | A view earns its place by carrying measurements no other view can. | kept | Pending port: each Plan view's basis names the requirements that justify it. Today the four default views are fixed and their basis is a sentence. |
| ADR2-D2 | Page, scale, view set and arrangement are one choice judged on the finished placement. | kept | Pending port: planner candidate selection. |
| ADR2-D3 | Infeasibility is a result, never a silent relaxation of an authored constraint. | kept | Pending port: an infeasible Spec constraint is a Plan problem or a refused request naming it (R1 invariants 3 and 9). Partly guarded by `invalid_requests_say_what_would_do`. |
| ADR2-D4 | Compose then pack: each view block includes its annotations. | kept | Pending port. |
| ADR2-D5 | Collect then solve: annotations are placed by one solve per strip. | kept | Pending port: over capacity is a ranked selection recorded per item. |
| ADR2-D6 | Policy B: keep the annotation and log the crossing rather than search unboundedly or drop it. | kept | Pending port: a retained crossing is an info problem on the annotation's ID. |
| ADR2-D7 | Users edit whole view blocks, never annotation coordinates. | translated | R1 invariant 1: Spec views carry presence, relations and an optional origin pin; the Spec has no coordinates. |
| ADR2-1 | Footprints are boxes; the corrective repack is bounded. | translated | The estimate-then-verify structure and its iteration bound stay, for termination and determinism; the OpenCascade cost that motivated it is gone (R2). |
| ADR2-2 | An explicit scale may cramp but never silently lose a requirement. | kept | Pending port: Spec scale policy (default, strict, permissive); the Plan records requested and effective scale, attempts and blockers. |
| ADR2-3 | Scale before sheet: try a larger scale before dropping the isometric or enlarging the sheet. | kept | Pending port: escalation order, every attempt recorded in the Plan. |
| ADR2-4 | Every strip occupant is a solve candidate; exemptions are an allowlist. | translated | One strip solver in the planner; any other placement path is a listed exception in this record. |
| ADR2-5 | The strip solve is exact, dependency-free and deterministic (PAVA, lower-median ties). | kept | Pending port with the same tie rule; property tests for crossing-freedom and determinism. The tie rule is a convention (an R2 "equivalent"), kept so differential comparison is meaningful. |
| ADR2-6 | One pass sequence serves the build and the edit drain. | retired | There is no drain: a Spec edit is re-planned whole by the one `plan`. |
| ADR2-7 | Feature leaders are assigned jointly; a fired budget is a named capability loss. | kept | Pending port: a hit budget is a Plan problem naming the lost capability. |
| ADR2-8 | Material re-entry is one predicate, shared by router and critique, and a cost not a gate. | kept | Pending port: one function for planner and lint. Its field must be deterministic (R2 invariant 3); exact haecceity geometry may replace the tessellated field. |
| ADR2-9 | A retained Policy B crossing is a finding with provenance. | kept | Pending port: a Plan problem naming the annotation and the obstruction. |
| ADR2-10 | Same-batch ink overlaps are resolved by a bounded local solve or reported. | kept | Pending port: an unresolved overlap is an `annotation_ink_overlap` problem. |
| ADR2-11 | Authored view request and resolved view plan are distinct immutable states. | translated | Spec `views` (request) and Plan `views` (result) share one vocabulary in separate documents. True by construction in `src/model.rs`. |
| ADR2-12 | Authored constraints are never relaxed; a dimension no authored view can carry fails, naming the view to add. | kept | Pending port: a `view_plan_incomplete` problem listing the dimension IDs and the view that would carry them, raised before rendering. |
| ADR2-13 | View removal is requirement-aware; no principal view is removed automatically. | kept | Pending port: any removal records its triggering requirement in the basis. |
| ADR2-14 | Detail and isometric scales are exact; unequal principal scales are refused. | kept | Pending port. |
| ADR2-15 | The placement solve can be traced; a strip-full drop names its occupants. | kept | Pending port: an optional trace beside the Plan; drop problems name occupants by ID. |
| ADR2-16 | Page validity is judged before completeness when comparing candidates. | kept | Pending port: an exhausted ladder gives a Plan marked invalid with the violations. |
| ADR2-17 | Interior labels are typed solver candidates, used only after exterior corridors are exhausted. | kept | Pending port. |
| ADR2-18 | An alternative layout is chosen only by strict measured improvement on a finished drawing. | retired | The port has one layout algorithm. If a second is ever added, this returns as a Spec option whose Plan records the comparison. |
| ADR2-19 | The normal layout is one build that never silently re-renders with another algorithm. | translated | R1 invariant 7: `plan` is one deterministic computation with no hidden retry under a different algorithm. |
| ADR2-U1 | The optimiser never shrinks text to fit. | kept | Pending port: typography is fixed paper-space input (`src/text.rs`); a guard test is owed (Python had none). |

## ADR 3: the recognition boundary

| Id | Rule | Disposition | Where it lands / why |
| --- | --- | --- | --- |
| ADR3-D1 | Recognition reasons about geometry, drafting about drafting; no rescanning to second-guess a record. | translated | R2 and quiddity-rust: quiddity owns recognition, `analyse` consumes its records, and `plan` never scans the B-rep for features. |
| ADR3-1 | Recognition executes in the provider. | translated | The crate boundary: no recognition code in draftwright-rust. |
| ADR3-2 | One recognition aggregate per build; none for a declared render. | translated | `analyse` runs recognition once; `plan` and `render` never do. The "none for a declared render" half is retired with ADR4-3. |
| ADR3-3 | Consumers project the one result; they do not rescan. | translated | The Analysis is passed to `plan` and lint by reference. |
| ADR3-4 | The provider join is fail-closed on family, record type and schema version. | kept | Pending port: the Analysis records quiddity's version and schema (`binding.loader`, `schema` today); `plan` refuses an unknown schema. No guard yet. |
| ADR3-5 | Every record type has exactly one home. | translated | A closed `match` over quiddity's record types that fails the read on an unknown one (same mechanism as ADR1-5). |
| ADR3-6 | No provider object crosses the waist. | translated | Spec and Plan name features by draftwright IDs. A quiddity fingerprint may sit in a Spec anchor as data (R3); a quiddity reference or face index never keys a Spec entry. |
| ADR3-7 | Occurrence ownership is captured at conversion, by identity. | kept | Pending port: the Analysis carries, per recognised occurrence, its disposition and owning feature; an unaccounted occurrence is reported, never dropped. |
| ADR3-8 | Ownerless policy is explicit per occurrence, from one capability table. | kept | Pending port. |
| ADR3-9 | Recognition is geometry-only; PMI lowering may change ownership, never what was recognised. | translated | quiddity records serialise into the Analysis; PMI is merged by draftwright, not quiddity. |
| ADR3-10 | The framed boundary returns frame, solid and result together, or a typed refusal. | kept | Pending port: a frame refusal is a typed Analysis outcome, not a fallback hidden in the adapter. |
| ADR3-11 | Frame gauge limits semantic claims. | kept | Pending port: the Analysis records the gauge; the planner derives no side or handedness the gauge cannot support. |
| ADR3-12 | Plural turned profiles are all planned; a groove with no single owner is refused. | kept | Pending port. |
| ADR3-13 | A second consumer of one recognition run costs no second run. | translated | Reports and spec export are pure functions of an existing Analysis. |

## ADR 4: declared intent

| Id | Rule | Disposition | Where it lands / why |
| --- | --- | --- | --- |
| ADR4-D1 | Detect-then-override is first-class. | translated | R1 decision: Spec entries override or add to Analysis features by stable ID. |
| ADR4-D2 | The editable source records intent, never placement outcomes. | kept | R1 decision (the Spec is the source of truth) with invariant 1: `plan` never writes outcomes back into the Spec; a dimension that does not fit stays in the Spec and is a problem in the Plan. Guard pending. |
| ADR4-D3 | A view or side on a dimension is a routing hint, not a placement. | translated | An optional view or side hint on a Spec dimension entry; the Plan records whether it was honoured. |
| ADR4-D4 | Recognition breadth equals editable-vocabulary breadth. | translated | The Spec's feature kinds are the Analysis's feature kinds. |
| ADR4-1 | No authoring surface carries page geometry. | translated | R1 invariant 1. Anchor-relative placement (`side`, `offset`) is allowed: an offset is paper millimetres from the anchor, a request the layout honours where it can and otherwise reports as a problem, never a bounds override (decided 2026-10-07). |
| ADR4-2 | Detection and declaration emit the same IR and the same furniture. | translated | Parity test: a Spec that declares exactly the detected features gives the same Plan. |
| ADR4-3 | A declared build performs no recognition. | retired | Differs from port note. Every `plan` starts from an Analysis, which is also where the geometry is read; there is no separate declared mode to keep free of recognition, and haecceity makes recognition cheap (R2). Confirmed 2026-10-07: every plan starts from an analysed part. |
| ADR4-4 | Renderers emit dimensional content only from the compiled plan. | kept | R1 invariant 2. Source-authored PMI text is carried verbatim in the Plan with a source-authored basis. |
| ADR4-5 | Dimension intent has two explicit sources: automatic, or a complete authored set. | translated | A Spec `dimension_source` (auto, authored); an authored set is complete; invalid combinations are validation errors. Decided 2026-10-07: keep both modes, defaults by default and an opt-in complete set. Not yet in the Spec schema. |
| ADR4-6 | Dimension identity is the derived parameter ID; an ambiguous selector is refused. | kept | R1 invariant 5, with the `role.kind[.discriminator]` vocabulary ([model](../model.md#ids)). |
| ADR4-7 | Suppression changes what is shown, never what the drawing asserts. | kept | R1 invariant 6. A compound callout's head without its dependents is a Spec validation error. |
| ADR4-8 | Authored omission and planner suppression are distinct rows. | kept | Pending port: a Plan omission's basis says `authored` or `rule:<name>` (R1 invariant 4). Same rule as ADR5-8. |
| ADR4-9 | Nothing the plan approves vanishes silently. | kept | R1 invariant 3. The guard joins Plan items against the rendered output. |
| ADR4-10 | A contingency is compiled, carried inactive, and released only when the primary places nothing. | kept | Pending port: contingent Plan items with basis `contingent_on <id>`. |
| ADR4-11 | Edits are pin and priority in the shared solve, never raw coordinates. | translated | Spec placement is anchor-relative (side, offset, order) plus pin and priority, and stays a request to the solver (ARCH0012-1). There is no raw-coordinate escape hatch and no drain. |
| ADR4-12 | Sheet identity is invariant between declaration and build. | retired | The Spec is an immutable JSON value; there is no fluent builder to mutate. |
| ADR4-13 | Generated editable output executes what it emits and fabricates no geometry. | translated | The script emitter is gone. A Spec exported from a Plan re-plans to the same Plan, and cannot hold geometry (R1 invariant 1). |
| ADR4-14 | Authored views with automatic dimensions is refused. | translated | Differs from port note, which would refuse the combination. Omitting a view is already allowed with default dimensions; instead, a dimension no remaining view can carry is a `view_plan_incomplete` problem naming the view to restore (ADR2-12). Decided 2026-10-07. |
| ADR4-15 | Automatic dimensioning carries no warning; automatic on the declarative surface is soft-deprecated. | retired | The deprecation mechanics have no counterpart. `analyse` then `plan` with an empty Spec is the automatic drawing (R1). |
| ADR4-U1 | Over-dimensioning that is neither identical nor coincident should be reported. | kept | Pending port, as a target: a redundancy check over Plan dimensions. Python never built it. |

## ADR 5: trust and honest failure

| Id | Rule | Disposition | Where it lands / why |
| --- | --- | --- | --- |
| ADR5-D1 | A guard is load-bearing only when breaking the rule on purpose fails it. | kept | Pending, as practice: each guard named here is shown to fail under a deliberate mutation when it lands. |
| ADR5-D2 | No document infers material, process, finish, thread, fit or tolerance intent. | kept | Pending port: such intent comes only from the Spec or source PMI. R3's "suggested replacement" for a fit is a problem's suggestion, never written by `plan`. |
| ADR5-1 | Same input, same sheet, every platform. | kept | R1 invariant 7, R2 invariant 3. Guard: `the_same_documents_give_the_same_bytes` (one platform; a cross-platform check is still owed). |
| ADR5-2 | Lint derives no completeness denominator from the plan. | kept | Pending port: completeness is a function of Analysis and the rendered sheet; checking a claim may read the Plan. Same boundary as ADR1-9. |
| ADR5-3 | Repair is an allowlisted, idempotent safety net; the cure is a placement rule. | kept | Decided 2026-10-07: the safe repairs are ported as an automatic step in `plan`. They are allow-listed and idempotent, accepted only on strict improvement with no new problems, and each is recorded in the Plan with what it changed. Python's repairs are `dim_inside_part` and the axis-aligned ink-overlap class. Any other fix stays a problem's suggested `fix` edit for the caller. Pending port. |
| ADR5-4 | Provenance is recorded once, at the render seam. | kept | Pending port: each Plan item carries its feature and dimension IDs; rendered elements carry the item's ID. |
| ADR5-5 | Quality is reported as components; completeness names its bound. | kept | Pending port, with the same field names (`audited_score`) once a report exists. |
| ADR5-6 | A claim on the sheet is checked against the plan, not a pattern. | kept | Pending port: lint checks rendered text against Plan text by ID. |
| ADR5-7 | An approved dimension not drawn is reported against its measurement and does not gate the build. | kept | R1 invariant 3. "Retracted if a later pass draws it" no longer applies: the Plan is computed whole and reports final outcomes only. |
| ADR5-8 | An authored omission is distinguishable from a planner omission. | kept | Pending port; same as ADR4-8. |
| ADR5-9 | A report refuses rather than shrinks, and serialises no identity meant to survive another run. | kept | Pending port, read as in [the section below](#reading-adr5-9-report-identity). Also: non-finite numbers are refused. `serde_json` writes NaN as `null`, so this needs an explicit check. |
| ADR5-10 | Report persistence is atomic. | kept | Pending port: every file the CLI writes is staged beside and renamed into place. `render` does this today; there is no guard yet. |
| ADR5-11 | Direct CLI rendering writes the report beside its output by default. | translated | Proposed 2026-10-07, built around what an LLM needs to fix a drawing. `plan` and `review` reply with everything needed: the problems, with their subjects and a suggested `fix` edit; the plan items they refer to, with their default flags and basis; and the spec. `review` adds a picture of the sheet with the problem regions marked, and what changed since the previous plan. `render` replies with the problems beside its files, and writes a sidecar only on request (`--report`). |
| ADR5-12 | Evidence comes from one hashed byte snapshot. | kept | Pending port. Implemented: `analyse` hashes the exact bytes it parses. `analysis_binds_to_its_file` checks the binding but not that the hash and the geometry share one read. |
| ADR5-13 | A clear result names its bound (`bounded-clear`, `needs-attention`). | kept | Pending port: the same status vocabulary in the Plan or report summary. |
| ADR5-14 | Read-only evidence is a projection, never a build. | translated | `analyse` alone is the inspection operation; the Analysis is the evidence. |
| ADR5-15 | Evidence lives beside the editable intent, not inside it. | translated | Analysis and Spec are separate documents. Outputs embed the Spec (R1 invariant 8), never the Analysis. |
| ADR5-16 | A document records the run options that determined it. | kept | Pending port: Analysis and Plan headers carry the source hash, producer version and every option that changes the result. |
| ADR5-17 | An evidence document states what it cannot say. | kept | Pending port: explicit "unavailable" fields in the Analysis, not empty lists. |
| ADR5-U1 | Lint's coverage is a goal, not a property; visual defects are outside lint. | kept | A documented limitation. R1's `review` operation is the visual channel. |

## Reading ADR5-9: report identity

ADR5-9 forbids an identity that *recognition or topology* hands out from outliving the run that
produced it: a `FeatureRef`, a `FaceRef`, a topology index or an object address. It does not
forbid every ID:

- **Within one file**, draftwright-owned IDs (`f:hole:3`, `d:f:hole:3/diameter@plan`) are
  allowed in all three documents. They are derived, not positional (R1 invariant 5). An Analysis
  may list a feature's face indices as data, because the Analysis is bound to one file by its
  `source_sha256`.
- **Across revisions**, no face index or quiddity reference carries anything. A Spec entry that
  must survive a new STEP file carries quiddity's fingerprint of its anchor, and `plan` rebinds
  through `correspond` (R3). Draftwright IDs are re-derived from the new Analysis, never trusted
  from the old one.
- **A report** keeps the refusal half unchanged: an unclassified occurrence, a foreign Analysis
  (wrong hash or schema), or a missing ownership row is an error, not a smaller report.

## Archived ADRs

The 66 `ARCH*` entries (36 drafting, 24 trust, 6 machinery) are rules the archived ADRs held that
the live ones did not restate. None is retired. **Six are translated**: semantic roles in place
of rendered labels (ARCH0008-1); decorations keyed by dimension or feature ID (ARCH0011-1);
edit locality as "a small Spec edit changes only the affected Plan items" (ARCH0003-2);
part-space spans in Plan dimensions (ARCH0016-9); view intent as a Spec view source with additions
(ARCH0018-6); and immutable Plans re-planned from the Spec (ARCH0018-7). **The other 60 are
kept.** Three already hold: text is measured with the font it is drawn with (ARCH0006-1,
`src/text.rs`); correctness over byte-parity (ARCH0008-3, R2); and display-complete labels in
the Plan (ARCH0019-1, R1 invariant 2). One belongs to quiddity-rust rather than here: the Z/Y-first
tie rule for a dominant axis (ARCH0013-1). The remaining 56 are pending ports, mostly planner
and formatter rules: projection alignment, escalation ladders, collapse wording, tolerance
rounding, per-body step chains, and the regression floors on placed counts.

## Decisions (maintainer, 2026-10-07)

1. **Lint's independence (ADR1-9, ADR5-2):** a module inside draftwright, guarded by a test that
   fails if lint reads the Plan or Spec, until it grows enough to be its own crate.
2. **Dimension and view sources (ADR4-5, ADR4-14, ARCH0018-6):** both modes are kept.
   - Defaults by default, with an opt-in complete set.
   - A dimension no remaining view can carry is a problem, not a refusal.
3. **Declared builds (ADR4-3, ADR3-2):** not supported. Every plan starts from an analysed part.
4. **Placement offsets (ADR4-1, ADR4-11):** paper millimetres from the anchor. An offset is a
   request the layout honours where it can, and otherwise reports as a problem; it never
   overrides the sheet's bounds.
5. **Reports (ADR5-11):** settled by what an LLM needs to fix a drawing, all in the reply:
   - the problems, with subjects and a suggested `fix` edit;
   - the plan items they refer to, with their defaults and basis;
   - the spec;
   - from `review`, a picture with the problem regions marked, and the changes since the
     previous plan.

   A sidecar is written only on request. (This one was proposed in answer to the maintainer's
   question, and stands unless changed.)
6. **Repair (ADR5-3):** the safe repairs are ported as an automatic, recorded step in `plan`.
   They are allow-listed, idempotent and accepted only on strict improvement. Anything else
   stays a suggested `fix`.
7. **Catalogue:** the edit-surface rules get their own class, `spec`. The README's `ADR2-7` is
   corrected by that change, since the list now names `ADR2-D7`.

**ADR5-9's reading is accepted:** recognition and topology identities never outlive a run.
Draftwright's own IDs are fine within one file, and revisions carry quiddity's fingerprints.

With these settled, this record awaits acceptance. On acceptance, the dispositions are copied into
the catalogue's `status` fields.
