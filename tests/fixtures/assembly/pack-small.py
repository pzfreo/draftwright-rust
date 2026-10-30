"""Regenerate the tiny synthetic pack fixtures; run from the repository root.

Requires build123d. Generated STEP bytes are gzip-compressed without timestamps.
"""
from pathlib import Path
import gzip

from build123d import Box, Compound, Cylinder, Location, Plane, export_step

ROOT = Path(__file__).resolve().parent


def save(shape, name):
    path = ROOT / (name + ".step")
    export_step(shape, path)
    with path.with_suffix(".step.gz").open("wb") as out:
        with gzip.GzipFile(filename="", fileobj=out, mode="wb", mtime=0) as compressed:
            compressed.write(path.read_bytes())
    path.unlink()


plate = Box(40, 30, 4)
plate.label = "plate"
pin = Cylinder(2, 12)
pin.label = "pin A"
pin2 = Cylinder(2, 12)
pin2.label = "pin B"
pin2.location = Location((12, 0, 10))
screw = Cylinder(1, 8).fuse(Cylinder(2, 2).moved(Location((0, 0, 5))))
screw.label = "M2 screw"
screw.location = Location((20, 0, 10))
assembly = Compound(children=[plate, pin, pin2, screw])
assembly.label = "pack fixture"
save(assembly, "pack-small")

# A blind hole gives a top side; the asymmetric pattern cannot be reflected by
# a proper rotation. Through holes alone would permit flipping the plate over.
right = Box(20, 16, 5).cut(
    Cylinder(1, 3).moved(Location((4, 3, 2))),
    Cylinder(2, 20).moved(Location((-5, 2, 0))),
    Cylinder(3, 20).moved(Location((1, -4, 0))),
)
right.label = "right handed"
left = right.mirror(Plane.YZ)
left.label = "left handed"
left.location = Location((40, 0, 0))
save(Compound(children=[right, left]), "pack-handed")
