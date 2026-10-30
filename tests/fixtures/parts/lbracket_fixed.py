"""The t2c L-bracket rebuilt with build123d alone, holes moved clear of the gussets (x = ±30)."""
import sys
import build123d as bd
BL, BW, BT = 80.0, 50.0, 8.0
BCY, BTH, BH = 21.0, 8.0, 50.0
HX = float(sys.argv[2])
with bd.BuildPart() as p:
    bd.Box(BL, BW, BT, align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN))
    with bd.Locations((0, BCY, BT)):
        bd.Box(BL, BTH, BH, align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN))
    for x in (-HX, HX):
        with bd.Locations((x, -10.0, -1.0)):
            bd.Cylinder(3, BT + 2, align=(bd.Align.CENTER, bd.Align.CENTER, bd.Align.MIN), mode=bd.Mode.SUBTRACT)
        with bd.Locations((x, BCY, 30.0)):
            bd.Cylinder(3, BTH + 2, rotation=(90, 0, 0), mode=bd.Mode.SUBTRACT)
    for xc in (-20.0, 20.0):
        fy = BCY - BTH / 2
        with bd.BuildSketch(bd.Plane.YZ.offset(xc - 4)):
            bd.Polygon([(fy, BT), (fy - 30, BT), (fy, BT + 30)], align=None)
        bd.extrude(amount=8)
bd.export_step(p.part, sys.argv[1])
