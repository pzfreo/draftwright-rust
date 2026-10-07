# ADR R2: The kernel

**Status:** Accepted (maintainer, 2026-10-07).

**Supersedes:** the OpenCascade and build123d dependency assumed throughout the Python records,
in particular the geometry and recognition boundaries of Python ADRs 1 and 3.

## Decision

All geometry comes from **haecceity**, a pure-Rust B-rep kernel shared with quiddity-rust:

- STEP import;
- rays, point classification, volumes and surface recovery;
- hidden-line projection with exact projected curves;
- sections without a boolean.

Text comes from the vendored fonts through `src/text.rs`. Output is written by `src/output.rs`.

Matching the Python implementation is a stepping stone, not the goal. The goal is being right.
Where Rust and OpenCascade disagree, the case is examined and given a verdict:

- **rust-correct:** OpenCascade is wrong;
- **rust-wrong:** a Rust bug, to be fixed;
- **equivalent:** a convention, either way is right;
- **undetermined:** not settled.

Each verdict is recorded with its evidence in a known-differences list beside the test that
found it.

## Why

The spikes recorded in the README and in quiddity-rust's history found three things:

- **OpenCascade's own output is often the wrong thing to copy.** Its hidden-line output
  approximates projected curves, up to 0.28 mm off the true curve. Its booleans lose or skip cut
  faces. Its visible lines sometimes contradict its own sections.
- **haecceity was exact where OpenCascade approximated.**
- **haecceity was faster:** about 150 times on the volume questions recognition asks.

## Invariants

1. **Every difference from the reference is listed with a verdict and a reason**, and the list
   has no stale entries. Guards: `crates/haecceity/tests/drawings.rs` and `probes.rs` in
   quiddity-rust.
2. **Curves are exact where the geometry is exact.** Edges are evaluated on their own curves, and
   projected edges carry their exact curves. Guard: `exact_curves_follow_their_points`.
3. **The kernel is deterministic.** No result depends on hash order or thread timing.
