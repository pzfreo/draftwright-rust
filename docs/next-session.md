# Next session: handoff (2026-10-07)

This is written for a fresh session continuing the port with parallel workflows ("ultracode").
Read it first, then the documents it points to.

## Where things stand

**quiddity-rust** (`~/repos/quiddity-rust`, branch `prototype/rust-port`, pushed at 590cb89):

- `crates/haecceity` is the kernel. It covers:
  - STEP import, rays, classification, volume probes and coverage;
  - surface recovery;
  - hidden lines with exact projected curves, and sections without booleans;
  - prism probes and face sweeps.
- 22 recogniser families are ported. The latest batch added through steps, oblique through
  steps, circular blind steps, turned steps, round-bottom blind slots, gusset ribs and gusset
  rib patterns. Each family is checked:
  - call by call against Python's captured test calls;
  - over the 100-part corpus, including defining faces;
  - under rigid motion.
- `docs/correspondence.md` designs revision matching: fingerprints with semantics, placement and
  AAG neighbourhood, and `correspond`. It is not implemented yet.
- Verify with
  `QUIDDITY_CORPUS_REQUIRED=1 cargo test --workspace --release`, fmt and clippy. The corpus is at
  `~/repos/quiddity/tests/corpus`. Set `QUIDDITY_CORPUS` when working from a worktree.

**draftwright-rust** (`~/repos/draftwright-rust`, `main`, pushed):

- **The document model** (ADR R1, `docs/model.md`):
  - Analysis, Spec and Plan;
  - `analyse`, `plan`, `edit`, `render`, `review` and `spec`, through the CLI, the JSON-lines
    service and the MCP server (`src/api.rs`, `src/bin/`);
  - the spec embedded in SVG, PDF and DXF;
  - output deterministic to the byte.
- **Layout:** draftwright's sheet, scale and view layout arithmetic (`src/layout.rs`), exact
  against 285 recorded Python calls. 86 differences from Python's final sheets remain, listed in
  `tests/fixtures/known_views.json`.
- **Bridge:** quiddity's features come in through `src/bridge.rs`, with stable IDs and
  fingerprints that include the AAG neighbourhood.
- **Facts:** Python draftwright's sheets are recorded as facts (`tools/capture_sheet_facts.py`,
  `tests/fixtures/sheets/`, `docs/facts.md`) for 13 parts.
- **ADRs R1–R4** are accepted. `docs/rules/` has 515 rules learnt from Python, classed drafting,
  trust, spec or machinery, with statuses. **Update a rule's status when you port it.**
- `docs/plan.md` is the overall plan.
- Verify with `cargo test --release`, fmt and clippy.

**Python references (read-only):**

- draftwright at origin/main 586628b. Read it with
  `git -C ~/repos/draftwright show origin/main:<path>` (the working tree is on a stale branch).
- Run it with `~/repos/draftwright/.venv/bin/python` and `PYTHONPATH` pointing at a source
  export (`git -C ~/repos/draftwright archive origin/main src | tar -x -C <dir>`).
- quiddity is at `~/repos/quiddity`, with its venv in `.venv`.

## How we work

- **Accuracy over parity.** Matching Python is a stepping stone. Every difference gets a verdict
  in a known-differences list: rust-correct, rust-wrong, equivalent, undetermined or
  not-applicable, with evidence.
- **One worktree and branch per stream.** In quiddity-rust, `isolation: 'worktree'` works when
  the session starts there. For draftwright-rust, create worktrees under the scratchpad.
- **Each stream gets a skeptical review and one fix round.** An integrator then merges locally
  and runs the full checks.
- **The main session re-verifies before pushing.** Never push unverified, and never publish to
  anyone without the maintainer's explicit permission.
- **Commit attribution:** messages end with the Co-Authored-By and Claude-Session lines the
  session gives.
- **Maintainer decisions to respect:**
  - lint is a module with a guard test that it never reads the Plan;
  - both dimension modes stay: defaults, and an opt-in complete set;
  - there are no declared builds;
  - placement offsets are paper millimetres and a request;
  - the safe repairs run automatically and are recorded;
  - replies carry everything an LLM needs to fix a drawing, as in R4's decision 5.

## Next spikes, briefed as parallel streams

**A. First dimensions** (draftwright-rust).

- **Plan:** linear and envelope dimensions, and hole callouts, from the bridged features. Each
  is a Plan item with an ID (`d:<feature>/<parameter>@<view>`), a value, a label, a default flag
  and a basis.
- **Render:** extension lines, dimension lines, arrows, and text through `src/text.rs`.
- **Check:** compare each item with the captured facts (measurement identity, value, label
  text, view), with verdicts.
- **Rules:** port the matching drafting rules from `docs/rules/lint.yaml` and
  `docs/rules/issues.yaml`, and set their status.

**B. View planning gaps** (draftwright-rust, stage 2).

- **Turned parts:** drop the side view (ADR2-13).
- **Layout:** reserve the annotation bands around features in the layout.
- **Sections:** plan section views, with hatching from `hlr::section` and draftwright's 45°
  even-odd hatch.
- **Check:** shrink `known_views.json` with evidence.

**C. Lint module skeleton** (draftwright-rust).

- **Guard test:** lint never reads the Plan or Spec (ADR1-9, ADR5-2).
- **Issue type:** a `LintIssue` type with the catalogue's fields.
- **First rules:** the coverage rules for holes, run on the rendered drawing together with the
  Analysis.
- **Check:** compare the findings with Python's on the captured sheets.

**D. Next recogniser families** (quiddity-rust). These build on what has just landed:

- grooves and plates, on `turned_steps`;
- rectangular blind slots, on the `regions.rs` helpers from round-bottom slots;
- double-D bores;
- edge-open circular pockets and edge-open prismatic recesses, together, since they share
  Python's `_rings` helper.

Follow the README's "How a family is ported".

**E. Correspondence** (quiddity-rust).

- **Implement:** fingerprints and `correspond(old, new)` as in `docs/correspondence.md`.
- **Revision pairs:** add a capture tool that builds them in build123d (a hole moved, a boss
  resized, a feature added or removed, the whole part moved or turned), each with its expected
  classes.
- **Invariance:** the corpus's invariance variants must correspond with every entry carried.

**F. A haecceity fix.** `face_mass` gives an area of 0 on faces 209 and 174 of nist_ftc_07 and
nist_ftc_10, where OpenCascade gives about 37 mm². The bridge refuses those files because of it.

## Small leftovers from the last reviews

- Views: the basis sentence is misleading for a computed or overflow scale fallback, and an
  empty iso projection leaves `iso_bounds` at zero without reporting it.
- Facts: `tests/facts.rs` has a check that can never fail ("look-at lands on its origin").
  `capture_sheet_facts.py` gives no usage message. `docs/facts.md` doesn't explain the
  identity-collision nulls.
- The MCP server hasn't been tried in a real client.
- PDF fonts are embedded whole, not subset.
- PNG output isn't written yet.
- quiddity-rust: the volume module's general `Probe::Prism` has no direct unit test for the
  convex case gussets use. Python's `local_degradation` path is not ported in any family.
