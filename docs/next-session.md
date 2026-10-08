# Next session: handoff (2026-10-08)

This is written for a fresh session continuing the port with parallel workflows ("ultracode").
Read it first, then the documents it points to: `docs/review-2026-10-08.md` (the latest review,
its improvement plan and the maintainer questions) and `docs/plan.md`'s "Next steps
(2026-10-08)".

## Where things stand

**quiddity-rust** (`~/repos/quiddity-rust`, branch `prototype/rust-port`, at c9ce1cd):

- `crates/haecceity` is the kernel. It covers:
  - STEP import, rays, classification, volume probes and coverage;
  - surface recovery;
  - hidden lines with exact projected curves, and sections without booleans;
  - prism probes and face sweeps, and face mass.
- 28 recogniser entry points are ported (the README's table). Each family is checked:
  - call by call against Python's captured test calls;
  - over the 100-part corpus, including defining faces;
  - under rigid motion.
- Correspondence (`docs/correspondence.md`) is implemented in the library and the CLI
  (`quiddity part.step` writes records and fingerprints; `quiddity correspond old new`), not yet
  in a JSON-lines or MCP surface.
- Its own review (`docs/review-2026-10-08.md` there) found CI red at 2fa7207 on a pinned face
  count that depends on float round-off.
- Verify with
  `QUIDDITY_CORPUS_REQUIRED=1 cargo test --workspace --release`, fmt and clippy. The corpus is at
  `~/repos/quiddity/tests/corpus`. Set `QUIDDITY_CORPUS` when working from a worktree.

**draftwright-rust** (`~/repos/draftwright-rust`, `main`; the state below is at bbd3dd7, with
the corpus check, CI and these docs from stream d-ci-docs on top):

- **The document model** (ADR R1, `docs/model.md`):
  - Analysis, Spec and Plan;
  - `analyse`, `plan`, `edit`, `render`, `review` and `spec`, through the CLI, the JSON-lines
    service and the MCP server (`src/api.rs`, `src/bin/`);
  - the spec embedded in SVG, PDF and DXF;
  - output deterministic to the byte, in every time zone (`tests/timezone.rs`).
- **Layout:** draftwright's sheet, scale and view layout arithmetic (`src/layout.rs`), exact
  against 285 recorded Python calls. Differences from Python's final sheets are listed with
  verdicts in `tests/fixtures/known_views.json` (71 entries).
- **Bridge:** quiddity's features come in through `src/bridge.rs`, with stable IDs and
  fingerprints that include the AAG neighbourhood. haecceity is pinned at quiddity-rust 2fa7207
  (`Cargo.lock`); that pin lets the bridge read nist_ftc_07 and nist_ftc_10. nist_ctc_04 is
  still refused (one face's area differs from OpenCascade's by 0.15 %).
- **Merged streams** (2026-10-07):
  - **View gaps** (d707219): a turned part drops the view that repeats its profile (ISSUE-1262-a,
    pending: the veto is narrower than Python's); annotation bands reserved from the features
    (`src/demand.rs`, `known_bands.json`); section A–A planned and hatched at 45°.
  - **First dimensions** (5b7be0d, `src/dims.rs`): the envelope, turned steps and hole callouts
    with locations and pattern pitches, as Plan items with IDs, labels, defaults and bases;
    compared with the captured facts (`tests/dimensions.rs`, `known_dimensions.json`).
  - **Lint** (54ce4cc, `src/lint/`): a guarded module that never reads the Plan or Spec,
    `LintIssue`, and hole coverage, compared with Python's lint on the captured sheets
    (`tests/lint.rs`, `known_lint.json`).
- **Facts:** Python draftwright's sheets are recorded as facts (`tools/capture_sheet_facts.py`,
  `tests/fixtures/sheets/`, `docs/facts.md`) for 13 parts.
- **ADRs R1–R4** are accepted. `docs/rules/` has 515 rules learnt from Python, classed drafting,
  trust, spec or machinery, with statuses (18 ported after the d-ci-docs audit). **Update a
  rule's status when you port it;** a partial port stays pending with `rust_test` and
  `rust_note`.
- `docs/plan.md` is the overall plan.
- **Verify** with
  `DRAFTWRIGHT_CORPUS=~/repos/quiddity/tests/corpus DRAFTWRIGHT_CORPUS_REQUIRED=1 cargo test --release`,
  `cargo fmt --check` and `cargo clippy --all-targets --release -- -D warnings`. Without
  `DRAFTWRIGHT_CORPUS_REQUIRED=1`, corpus-backed tests skip (print SKIPPED and pass) when the
  corpus is missing; with it, they fail.

**Python references (read-only):**

- draftwright at origin/main 586628b. Read it with
  `git -C ~/repos/draftwright show origin/main:<path>` (the working tree is on a stale branch).
- Run it with `~/repos/draftwright/.venv/bin/python` and `PYTHONPATH` pointing at a source
  export (`git -C ~/repos/draftwright archive origin/main src | tar -x -C <dir>`).
- quiddity is at `~/repos/quiddity`, with its venv in `.venv`. At 6d544e0, the ref CI checks
  out, its corpus files match the sha256 bindings of every sheet-facts and corpus recognition
  fixture.

## Taking a new haecceity

haecceity comes by git from `pzfreo/quiddity-rust` (`Cargo.toml`: branch
`prototype/rust-port`), pinned to one commit in `Cargo.lock`. To move the pin:

1. `cargo update -p haecceity`, and note the old and new commits (the `source` line of
   `haecceity` in `Cargo.lock`).
2. Re-run the full verify line above.
3. Record every change it causes: a known-differences entry that went away or appeared, a part
   the bridge now reads or refuses, a changed count. Each gets a verdict with evidence in its
   list, and the commit message names the new pin and what changed (971e17e is an example).

## CI

`.github/workflows/ci.yml` runs on pushes to `main` and on pull requests: the toolchain from
`rust-toolchain.toml`, `cargo fmt --all --check`, clippy with `-D warnings`, and
`cargo test --release` with `DRAFTWRIGHT_CORPUS` at a sparse checkout of the public
`pzfreo/quiddity`'s `tests/corpus` (pinned to 6d544e060cd0eeb4442d2196e5df757b5a15ea55) and
`DRAFTWRIGHT_CORPUS_REQUIRED=1`.

haecceity's repository, `pzfreo/quiddity-rust`, is private, and the workflow's `GITHUB_TOKEN`
cannot read it. The workflow reads a repository secret **`HAECCEITY_READ_TOKEN`**: a token with
read access to the contents of `pzfreo/quiddity-rust` (for example a fine-grained personal
access token limited to that repository, Contents: read-only). It rewrites
`https://github.com/pzfreo/quiddity-rust` to use it (`git config url.….insteadOf`) and has cargo
fetch with the git CLI (`CARGO_NET_GIT_FETCH_WITH_CLI=true`). If the secret is missing, the
first step fails with an error naming it. **The secret does not exist yet:** creating it (or
choosing another way, such as a deploy key or publishing haecceity) is the maintainer's decision
(the review's first question). Until then CI fails at its first step. To set it:

1. Create the token (GitHub → Settings → Developer settings → Fine-grained tokens), repository
   access `pzfreo/quiddity-rust` only, permission Contents: read-only.
2. In `pzfreo/draftwright-rust` → Settings → Secrets and variables → Actions, add it as
   `HAECCEITY_READ_TOKEN`.
3. Re-run the workflow.

When fixtures are re-captured against a changed corpus file, move the workflow's `ref` to a
quiddity commit that has it.

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

## Open leftovers from earlier reviews

The 2026-10-08 review (`docs/review-2026-10-08.md`) supersedes the old spike list: its
improvement plan and questions are the current work. Still open from before it:

- Views: the basis sentence is misleading for a computed or overflow scale fallback, and an
  empty iso projection leaves `iso_bounds` at zero without reporting it.
- Facts: `capture_sheet_facts.py` gives no usage message. `docs/facts.md` doesn't explain the
  identity-collision nulls.
- The MCP server hasn't been tried in a real client.
- PDF fonts are embedded whole, not subset.
- PNG output isn't written yet.
- quiddity-rust: the volume module's general `Probe::Prism` has no direct unit test for the
  convex case gussets use. Python's `local_degradation` path is not ported in any family.
- From wave 1 (review H1–H5, M2 merged):
  - `Drawing::problems` (e.g. `edges_not_drawn` on cgb242) is not yet added to the problems
    `api::review` and `api::render` return, so the CLI, service and MCP do not show it.
  - demand groups holes by recognised pattern without `bridge::holes::established`, so a
    bolt circle of fewer than five holes is one feature there but loose holes in dims and lint.
  - A bolt circle of fewer than five holes with a hole or boss on its centre is a pattern in
    Python; not ported (LINT-hole_coverage-bolt_circle_corroboration stays pending).
  - dims states one recess per hole (counterbore, else spotface); a hole with both would lose
    its spotface and lint would report it. No fixture has one.
  - Question for haecceity: on cgb242's front view HLR emits a zero-span NURBS stretch (t0 =
    t1 = 0) at an edge's own end vertex; should it be filtered at source?
