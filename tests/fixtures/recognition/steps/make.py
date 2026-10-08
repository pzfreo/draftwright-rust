"""Build the step fixture parts: quiddity's golden fixtures for the planar step families (no
captured corpus sheet has a circular blind step, a paired-ramp step or a through step running
along x or y), as quiddity's ``tests/golden/<name>/fixture.py`` builds them (Paul Fremantle,
Apache-2.0), and a block with a step along y whose legs draftwright leaves to its height ladder
and overall extents (``stepped_block``). Run from this directory with draftwright's venv, then record quiddity's recognition
and draftwright's sheet facts for them:

    ~/repos/draftwright/.venv/bin/python make.py .
    ~/repos/draftwright/.venv/bin/python ../../../../tools/recognise.py <name>.step -o <name>.json
    gzip -9 -n <name>.json                # for each name below
    PYTHONPATH=<draftwright>/src ~/repos/draftwright/.venv/bin/python ../slot_patterns/capture.py \\
        ../../../../tools/capture_sheet_facts.py <draftwright>/src . . <name> ...
"""

import sys
from pathlib import Path

from build123d import Box, Cylinder, Plane, Polygon, Pos, Rot, export_step, extrude

out = Path(sys.argv[1])

PARTS = {
    # A 20 x 20 rectangular through step along z, cut from one corner of a 40 x 30 x 20 block.
    "rectangular_through_step": lambda: Box(40, 30, 20) - Pos(15, 10, 0) * Box(20, 20, 30),
    # A quarter-cylinder (R4) blind step along x, cut into one top edge of a 40 x 30 x 20 block.
    "circular_blind_step": lambda: Box(40, 30, 20)
    - Pos(7.5, 15, 10) * Rot(0, 90, 0) * Cylinder(4, 25),
    # A V of two equal ramps (a 16 x 10 triangle) cut 25 deep into one side of a 40 x 40 x 30
    # block.
    "paired_ramp_step": lambda: Box(40, 40, 30)
    - Pos(20, 20, 0) * extrude(Plane.XZ * Polygon((0, -8), (0, 8), (-10, 0)), 25),
    # A base plate, an end wall, a lower step and a raised pad: face levels and plates.
    "plates_pads_levels_and_slanted_steps": lambda: Box(120, 70, 8)
    + Pos(-50, 0, 20) * Box(10, 70, 32)
    + Pos(10, 0, 8) * Box(45, 50, 8)
    + Pos(30, 0, 20) * Box(24, 18, 16),
    # A 20 x 10 step along y cut from one top edge of a 40 x 30 x 20 block.
    "stepped_block": lambda: Box(40, 30, 20) - Pos(10, 0, 5) * Box(20, 40, 10),
}

for name, build in PARTS.items():
    export_step(build(), str(out / f"{name}.step"))
