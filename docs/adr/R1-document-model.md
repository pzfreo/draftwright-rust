# ADR R1: The document model

**Status:** Accepted (maintainer, 2026-10-07).

**Supersedes:**
- Python ADR 4 (declared intent): the authoring surface, which is the fluent `Sheet` facade, the
  script emitter, `authored_dimensions` and `auto_dimensions`, and edits as deferred intents;
- Python ADR 1 (compiler pipeline): the deferred-intent drain and its pass sequence.

**Keeps:** Python ADR 1's narrow waist (features → plan → renderers) and Python ADR 4's
invariants about the plan, translated below.

## Decision

A drawing is described by three JSON documents and produced by five stateless operations, as set
out in [docs/model.md](../model.md):

- **Analysis** is the part as recognised, and can always be regenerated.
- **Spec** is the author's intent, keyed by stable IDs. It is the source of truth.
- **Plan** is every view, dimension and annotation, with its ID, value, default flag and basis,
  plus structured problems.
- **The operations** are `analyse`, `plan`, `edit`, `render` and `review`.
- **The adapters**, a CLI, a JSON-lines service and an MCP server, are thin layers over that one
  core.

There is no authoring language. An LLM or a person changes a drawing by editing the spec through
small validated operations, or by replacing it whole, and reads the result back as a plan.

## Why

- **Editing a program is the expensive way to make small changes.** In the Python version, a
  callout change means editing a generated script, running it and resubmitting it.
- **The machinery that made the language work was the largest part of the system.** That
  includes the emitter's round trip, deferred-intent replay with transactional rollback, and two
  paths kept in step by one pass sequence.
- **An API can be just as effective.** pzfreo/specify-core shows that small stateless calls over
  plain JSON work for an LLM, given stable IDs, self-explaining defaults and corrective errors.

## Invariants

1. **The spec stores intent, never geometry values.** It never holds a page coordinate or a
   measured number. Placement is relative to an anchor. This translates Python ADR 4 invariant 1,
   "no authoring surface carries page geometry".
2. **Renderers draw only from the plan.** No renderer formats a number the plan did not resolve.
   This keeps Python ADR 4 invariant 4.
3. **Nothing the plan approves vanishes silently.** An item that can't be drawn is reported
   against its ID. This keeps Python ADR 4 invariant 9.
4. **Every plan item has a stable ID, a default flag and a basis sentence.** An item the spec did
   not decide is marked as a default and says why it is there.
5. **IDs are derived, never positional.** A selector that matches more than one item is refused,
   not guessed. This keeps Python ADR 4 invariant 6.
6. **Omission marks; it never infers.** An omitted parameter is never read as a fact; a blind
   hole does not read THRU because its depth was omitted. This keeps Python ADR 4 invariant 7.
7. **The operations are pure and deterministic.** The same documents give the same bytes on every
   platform. This keeps Python ADR 5 invariant 1.
8. **Every output embeds the spec it was made from**, so a drawing can be resumed from the file
   alone.
9. **Validation errors say what would be accepted.** That means a list of choices, a "reads as"
   correction, or the nearest valid IDs.

Each invariant gets its guard test as the code lands. Until then, R4 tracks it as pending.

## Open

- **Datums and GD&T:** whether they live in the spec or are taken from specify-core's intent
  (joining the two documents).
- **Whether the spec also lives inside the STEP file**, as specify-core's answers do.

## Decision note: projected evidence for interior placement (2026-10-12)

The planner may now read the Analysis's optional `projected_views`: the renderer's
HLR visible and hidden curves in each standard principal projection, plus complete
sampled boundaries of planar material faces parallel to that projection. Analysis
obtains these from the same kernel projection and curve lowering as rendering;
rendering reuses them and applies its existing paper-space hidden-line precedence.
Missing, incomplete or incompatible evidence cannot establish blank face.

This deliberately replaces the earlier exterior-only interpretation of "no
line-work in the plan": projected ink is regenerable Analysis evidence, not authored
Spec geometry or a second annotation renderer. It lets the planner prove a complete
label clear over material inside its own view, and the complete annotation's lines
and filled arrowheads clear of projected visible and hidden ink, retaining Python's
automatic interior callouts and whole-dimension corridor fallback. Resolved dimensions carry typed
`region: interior` provenance and a basis sentence; the Plan does not copy the HLR
inventory. All nine invariants above remain in force, including deterministic JSON,
relative author intent, stable IDs, and renderers drawing resolved annotations only.
