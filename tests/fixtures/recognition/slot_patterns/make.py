"""Build the slot-pattern fixture parts (no corpus part has a slot array): plates with arrays of
through obround slots, in build123d. Run from this directory with draftwright's venv, then record
quiddity's recognition and draftwright's sheet facts for them:

    ~/repos/draftwright/.venv/bin/python make.py .
    ~/repos/draftwright/.venv/bin/python ../../../../tools/recognise.py slot_array.step -o slot_array.json
    gzip -9 -n slot_array.json            # and the same for slot_grid
    PYTHONPATH=<draftwright>/src ~/repos/draftwright/.venv/bin/python capture.py \\
        ../../../../tools/capture_sheet_facts.py <draftwright>/src . . slot_array slot_grid
"""

import sys
from pathlib import Path

from build123d import (
    Box,
    BuildPart,
    BuildSketch,
    Locations,
    Mode,
    Plane,
    Pos,
    SlotCenterToCenter,
    export_step,
    extrude,
)

out = Path(sys.argv[1])


def plate(name, size, centres, c2c, width, rotate=0):
    length, depth, thickness = size
    with BuildPart() as p:
        Box(length, depth, thickness)
        with BuildSketch(Plane.XY.offset(thickness / 2)):
            with Locations(*centres):
                SlotCenterToCenter(c2c, width, rotation=rotate)
        extrude(amount=-thickness, mode=Mode.SUBTRACT)
    part = p.part.moved(Pos(length / 2, depth / 2, thickness / 2))
    export_step(part, str(out / f"{name}.step"))


# Three obround slots 6 wide, 20 long (centres 14 apart), at 25 mm pitch along x, running along y.
plate("slot_array", (120, 60, 8), [(-25, 0), (0, 0), (25, 0)], 14, 6, rotate=90)
# A 2 x 3 grid of obround slots 8 wide, 24 long, running along x: 40 mm apart along x, 30 along y.
plate("slot_grid", (160, 100, 10), [(x, y) for y in (-15, 15) for x in (-40, 0, 40)], 16, 8)
