# Catalogue regression pins

All `.gz` files are gzip with `mtime=0`. STEP fixtures are lossless copies from Python
`draftwright/tests/fixtures` (read on 2026-10-16):

- `tuner_jig_blind_obround_pockets.step`: five pockets, centres y = 21.2, 48.4, 75.6,
  102.8, 130 mm, x = -5, z = 8.5. Width 7.9, length 13.6, depth 19, corner R3.94.
- `grm04_drive_plate.step`: GRM-04, including its Ø2.4 through hole.
- `nist_ctc_05_asme1_ap203.stp` and `nist_ctc_05_asme1_ap242.stp`: the same NIST CTC-05
  solid in two encodings. Python's current #1555 test pins **three** physical steps on z,
  after equal-diameter adjacent bands were coalesced; the catalogue's five-step note is old.

`tuner.recognition.json.gz`, `grm04.recognition.json.gz` and
`opposed.recognition.json.gz` were written with this repo's
`tools/recognise.py`, using `/Users/paul/repos/draftwright/.venv/bin/python` (quiddity 0.3.9),
against the exact STEP bytes in this directory. They make the pocket scenario independent of native
section-recess document support. The tuner is intentionally dimensioned as five pockets,
with a shared x location and five datum y locations; Rust has no pocket-pattern adapter.
The expected measurements are hand-listed fixture facts, never recognition-derived floors.

The remaining fixtures were exported by build123d using the same Python environment:

```python
from build123d import *
C = (Align.CENTER,) * 3
opposed = Box(40, 40, 20, align=C) - Pos(0, 0, 4) * Cylinder(1, 6) - Pos(0, 0, -10) * Cylinder(2, 6)
export_step(opposed, "opposed.step")
framed = (Box(80, 55, 20, align=C)
          - Pos(17, 8, 0) * Cylinder(4, 30, align=C)
          - Pos(-22, -11, 4) * Box(10, 8, 8, align=C))
export_step(framed, "framed.step")
export_step(Pos(123, -47, 91) * Rot(17, 31, 23) * framed, "framed-moved.step")
```

The opposed-tap recipe is Python #1353's (the second cut intersects the box by 3 mm).
Its captured Python recognition also retains the enclosed cavity, avoiding unrelated native
capability/unclaimed-face warnings while the test requires all lint and plan findings to be clear
or informational. Native before/after render evidence uses the same original native inputs.
The rigid-motion pair is Python #1357's. Its intrinsic-size regression is only a partial pin:
raw Rust planning still dimensions the moved caller-space box (91.844 × 91.173 × 68.912)
instead of the physical 80 × 55 × 20 box. Full ARCH0020-1 needs one frame shared by analysis,
projection, PMI and rendering and stays pending.
