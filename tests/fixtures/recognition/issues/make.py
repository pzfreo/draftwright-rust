"""Build the parts of draftwright's issue regressions that `tests/issue_regressions.rs` ports, as
the cited Python tests build them (draftwright's `tests/test_issue_<n>_*.py`, AGPL-3.0). Run from
this directory with draftwright's venv, then record quiddity's recognition for each:

    ~/repos/draftwright/.venv/bin/python make.py . [name ...]   # every part, or those named
    ~/repos/draftwright/.venv/bin/python ../../../../tools/recognise.py <name>.step -o <name>.json
    gzip -9 -n <name>.json                # for each name below

and draftwright's own whistle frame (the part its #1598 and #1613 tests read), kept gzipped:

    git -C <draftwright> show origin/main:tests/fixtures/whistle_frame_reference.step \
        > whistle_frame_reference.step
    ~/repos/draftwright/.venv/bin/python ../../../../tools/recognise.py \
        whistle_frame_reference.step -o whistle_frame_reference.json
    gzip -9 -n whistle_frame_reference.step whistle_frame_reference.json
"""

import sys
from pathlib import Path

from build123d import Align, Box, Cylinder, Pos, Rot, Rotation, export_step

out = Path(sys.argv[1])
MIN_Z = (Align.CENTER, Align.CENTER, Align.MIN)


def blind_pocket(x_size=20, y_size=20):
    # test_issue_916_pocket_leader.py::_blind_pocket: a real blind recess.
    return Box(50, 50, 30) - Pos(10, 10, 10) * Box(x_size, y_size, 10)


def curved(case):
    # test_issue_1485_curved_pockets.py::curved: a pocket cut into a cylinder's side, so its
    # mouth is the cylinder (an x-axis cylinder of radius 20).
    y, length = (9, 6) if case == "offset" else (0, 24)
    part = Rotation(0, 90, 0) * Cylinder(20, 80) - Pos(0, y, 14) * Box(6, length, 12)
    if case == "negative":
        part = Rotation(180, 0, 0) * part
    if case == "rotated":
        part = Pos(11, 22, 33) * Rotation(0, 90, 0) * part
    return part


def u_channel(lower_wall=12.5, upper_wall=12.5):
    # test_issue_917_open_channel.py::_u_channel: a base and two walls, pierced by a
    # counterbored hole through the base and a cross hole through both walls.
    width = 50.0
    lower_center = -width / 2 + lower_wall / 2
    upper_center = width / 2 - upper_wall / 2
    part = (
        Box(50, width, 12)
        + Pos(0, lower_center, 15) * Box(50, lower_wall, 18)
        + Pos(0, upper_center, 15) * Box(50, upper_wall, 18)
    )
    part -= Cylinder(2, 12)
    part -= Pos(0, 0, 4) * Cylinder(6, 4)
    part -= (
        Pos(0, 0, 15)
        * Rot(90, 0, 0)
        * Cylinder(4, 60, align=(Align.CENTER, Align.CENTER, Align.CENTER))
    )
    return part


PARTS = {
    # test_issue_1593_title_block_keepout.py::CASES.
    "tb_plain_box": lambda: Box(40, 30, 12),
    "tb_tall_box": lambda: Box(80, 60, 30),
    "tb_stepped": lambda: Box(80, 60, 30) - Pos(0, -20, 7.5) * Box(80, 20, 15),
    "tb_bored_plate": lambda: Box(90, 60, 10) - Cylinder(5, 10),
    # test_issue_916_pocket_leader.py.
    "pocket_square": lambda: blind_pocket(),
    "pocket_z": lambda: blind_pocket(22, 14),
    "pocket_x": lambda: Rot(0, 90, 0) * blind_pocket(22, 14),
    "pocket_y": lambda: Rot(90, 0, 0) * blind_pocket(22, 14),
    # test_issue_1485_curved_pockets.py.
    "curved_centered": lambda: curved("centered"),
    "curved_offset": lambda: curved("offset"),
    "curved_negative": lambda: curved("negative"),
    "curved_rotated": lambda: curved("rotated"),
    # test_issue_917_open_channel.py.
    "u_channel": lambda: u_channel(),
    "u_channel_asymmetric": lambda: u_channel(lower_wall=10.0, upper_wall=15.0),
    "bounded_pocket": lambda: Box(80, 60, 20) - Pos(0, 0, 6) * Box(30, 20, 8),
    "centred_rebate": lambda: Box(80, 60, 30) - Pos(0, 0, 7.5) * Box(80, 20, 15),
    # test_issue_1143_hole_completeness.py::_single_hole.
    "single_hole": lambda: Box(80, 50, 10, align=MIN_Z)
    - Pos(12, 7, 0) * Cylinder(4, 10, align=MIN_Z),
    # test_issue_1351_structured_note_coverage.py::_part: a counterbored through hole.
    "counterbored_block": lambda: Box(60, 40, 20) - Cylinder(4, 40) - Pos(0, 0, 6) * Cylinder(7, 8),
}

for name, build in PARTS.items():
    if sys.argv[2:] and name not in sys.argv[2:]:
        continue
    export_step(build(), str(out / f"{name}.step"))
