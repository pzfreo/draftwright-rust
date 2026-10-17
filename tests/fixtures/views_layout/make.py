"""Issue 1530 shapes, from the Python section provenance tests; run with draftwright's venv.
The internal station fixture is Python's issue_1595_whistle_key_frame.step.
"""
from pathlib import Path
from build123d import Box, Compound, Cylinder, Pos, Rot, export_step

out = Path(__file__).parent
base = Box(20, 16, 8)
boss = Pos(5, 4, 7) * Box(4, 8, 6)
parts = {
    'section_attached': base + boss,
    'section_detached': Compound(children=[base, Pos(30, 0, 0) * boss]),
    'section_adjacent': base + Pos(10, 4, 0) * Box(4, 8, 8),
    'section_notch': base - Pos(5, -4, 2) * Box(10, 8, 4),
    'section_gap': Compound(children=[Pos(0, y, 0) * Box(20, 4, 8) for y in (-6, 6)]),
    'section_regions': Compound(children=[
        (base - Pos(-5, 0, 0) * Rot(90, 0, 0) * Cylinder(2, 16))
        + Pos(5, 5, 7) * Box(4, 4, 6), Pos(18, 0, 0) * Box(4, 16, 4)]),
}
for name, part in parts.items():
    export_step(part, out / f'{name}.step')
