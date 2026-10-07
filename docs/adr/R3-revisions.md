# ADR R3: Revisions

**Status:** Accepted (maintainer, 2026-10-07). Implementation follows once feature overrides exist.

## Decision

When the STEP file changes, the author's decisions carry over to the new revision.

**quiddity does the matching** (`correspond(old, new)`):
- It owns revision-stable **fingerprints** of features and faces: a geometric signature in the
  part's frame.
- It works in two levels:
  - it aligns the two parts by a rigid motion found from distinctive features;
  - it assigns features of each type one to one, with an explicit "unmatched" option, and
    classifies each match as carried, adapted, ambiguous, orphaned or new;
  - it then matches faces by propagation over the adjacency graph, seeded from the matched
    features.

**draftwright applies the result.** It rebinds its spec and labels every plan item with its
class. specify-core can apply the same correspondence to its answers.

## Why

- **There is no construction history to name entities through.** History-based persistent naming
  (Capoyleas, Chen & Hoffmann 1996; Kripac 1997; OCCT TNaming; FreeCAD 1.0) needs one, and we
  have only two STEP files.
- **History-free matching is a known problem.** Onshape's re-import matcher (Baran & Schulz,
  US 11,288,411) and Jones et al. (SIGGRAPH 2023) both match from exact seeds, then by
  propagation, and leave what stays ambiguous unmatched rather than guessing.
- **Recognised features are the natural anchors.** They are the persistent semantic entities
  Bidarra et al. (2005) argue for, and are far less ambiguous than faces.
- **Rigid motion needs its own step.** The published matchers handle translation at most;
  aligning by distinctive features first lets a moved or reoriented revision rebind.
- **quiddity is the right home.** Both draftwright and specify-core need it, and specify-core
  currently refuses a changed file outright.

## Invariants

1. **Nothing changes without a trace.** Every rebound item is labelled carried, adapted,
   ambiguous, orphaned or new.
2. **Ambiguity is asked, never guessed.** An ambiguous match keeps the entry and raises a
   question listing the candidates.
3. **An orphaned entry is kept inactive**, never deleted.
4. **A rigid motion of the whole part carries every entry.** Guard: the corpus's invariance
   variants.
5. **An adapted choice that no longer fits is a problem with a suggested replacement**, never a
   silent change.
