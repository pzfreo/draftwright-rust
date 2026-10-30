"""Build123d gusset array made for #256 by draftwright's maintainers."""
import sys
import build123d as bd

with bd.BuildPart() as p:
    bd.Box(100, 50, 8, align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN))
    with bd.Locations((0, 21, 8)):
        bd.Box(100, 8, 50, align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN))
    for x in (-30, -5, 20):
        with bd.BuildSketch(bd.Plane.YZ.offset(x - 3)):
            bd.Polygon([(17, 8), (-5, 8), (17, 34)], align=None)
        bd.extrude(amount=6)
bd.export_step(p.part, sys.argv[1])
