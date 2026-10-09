"""Write `tests/fixtures/parts/scattered_holes.step` for `tests/hole_tables.rs`: a plate with 18
through holes of three sizes placed irregularly, so no pattern is recognised and its plan view
has enough holes outside a pattern (16) to warrant draftwright's hole table.

    ~/repos/draftwright/.venv/bin/python -I tools/make_scattered_holes.py tests/fixtures/parts/scattered_holes.step
"""

import sys

from build123d import Box, Cylinder, Pos, export_step, Align
pts = [(-95,55,6),(-71,-48,8),(-52,20,5),(-30,-62,6),(-18,41,8),(-4,-15,5),(9,63,6),(23,-40,8),
       (37,8,5),(48,51,6),(61,-58,8),(74,-9,5),(88,36,6),(99,-41,8),(-84,-6,5),(-60,62,6),(15,25,8),(-40,-30,5)]
p = Box(240,160,12)
for x,y,d in pts:
    p -= Pos(x,y,0)*Cylinder(d/2, 20)
export_step(p, sys.argv[1])
