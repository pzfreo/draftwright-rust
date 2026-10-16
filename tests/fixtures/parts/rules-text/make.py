"""Exact #958 solids from Python draftwright tests/test_issue_958_cross_solid_recognition.py.

Regenerate with draftwright's venv. Fixtures are below parts/ so the opt-in byte guard's
parts/*.step roster does not change. No drawing or recognition expectations are recorded.
"""
from pathlib import Path
from build123d import Box, Compound, Pos, export_step

out = Path(__file__).parent
base = Box(80, 60, 10)
detached = Pos(0, 0, 10) * Box(30, 20, 4)
wall = Pos(0, -10.5, 6.5) * Box(30, 1, 3)
slotted = Box(60, 30, 10) - Box(30, 8, 20)
padded = base + Pos(0, 0, 7) * Box(30, 20, 4)
pocketed = Box(80, 60, 20) - Pos(0, 0, 6) * Box(30, 20, 8)
channel = (Box(50, 50, 12) + Pos(0, -18.75, 15) * Box(50, 12.5, 18)
           + Pos(0, 18.75, 15) * Box(50, 12.5, 18))
cases = {
    "detached": Compound(children=[base, detached]),
    "three-body-pocket": Compound(children=[base, detached, wall]),
    "three-body-channel": Compound(children=[Box(30, 60, 10), detached, wall]),
    "two-slots": Compound(children=[slotted, Pos(0, 60, 0) * slotted]),
    "two-pads": Compound(children=[padded, Pos(120, 0, 0) * padded]),
    "two-pockets": Compound(children=[pocketed, Pos(120, 0, 0) * pocketed]),
    "two-channels": Compound(children=[channel, Pos(0, 0, 60) * channel]),
    "two-channels-reversed": Compound(children=[Pos(0, 0, 60) * channel, channel]),
}
for name, part in cases.items():
    export_step(part, out / f"{name}.step")
