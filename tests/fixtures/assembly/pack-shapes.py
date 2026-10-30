"""Build neutral copies of catalogue shapes and deliberately custom negatives.

Requires build123d (including its OCP dependency). Run from the repository root.
Names carry no supply hints; geometry exercises recognised-feature rules.
"""
from pathlib import Path
import gzip
from build123d import Axis, Compound, Cylinder, Location, chamfer, fillet, export_step
from build123d import Align, Plane, RegularPolygon, Solid, extrude
import math
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakePolygon, BRepBuilderAPI_MakeWire
from OCP.BRepLib import BRepLib
from OCP.BRepOffsetAPI import BRepOffsetAPI_MakePipe
from OCP.Geom2d import Geom2d_Line, Geom2d_TrimmedCurve
from OCP.Geom import Geom_CylindricalSurface
from OCP.GeomFill import GeomFill_IsFrenet
from OCP.gp import gp_Ax3, gp_Dir, gp_Pnt, gp_Pnt2d, gp_Dir2d

root = Path(__file__).resolve().parent
washer = Cylinder(4, 1) - Cylinder(2, 2)
dowel = Cylinder(2, 12)
dowel = chamfer(dowel.edges().filter_by(Axis.Z, reverse=True), length=0.3)
rounded = Cylinder(2.5, 14)
rounded = fillet(rounded.edges().filter_by(Axis.Z, reverse=True), radius=0.3)
plain = Cylinder(2, 10)
stepped = Cylinder(2, 12).fuse(Cylinder(3, 3).moved(Location((0, 0, 5))))
# Modelled helical flanks, cylindrical head, hexagonal socket; no cosmetic threads.
# A constant trapezoidal helical cut, adapted from quiddity's original
# tools/make_thread_fixtures.py construction. Flat crests/roots and opposed flanks.
radius, lead, turns = 10.0, -2.0, 6
surface = Geom_CylindricalSurface(gp_Ax3(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), radius)
segment = Geom2d_TrimmedCurve(Geom2d_Line(gp_Pnt2d(0, 0), gp_Dir2d(2 * math.pi, lead)), 0, turns * math.hypot(2 * math.pi, lead))
edge = BRepBuilderAPI_MakeEdge(segment, surface).Edge()
BRepLib.BuildCurves3d_s(edge)
path = BRepBuilderAPI_MakeWire(edge).Wire()
h = lead / (2 * math.pi)
unit = math.hypot(radius, h)
polygon = BRepBuilderAPI_MakePolygon()
for dr, width in [(-0.5, -0.125), (-0.5, 0.125), (0.5, 0.3), (0.5, -0.3)]:
    polygon.Add(gp_Pnt(radius + dr, -width * h / unit, width * radius / unit))
polygon.Close()
profile = BRepBuilderAPI_MakeFace(polygon.Wire()).Face()
tool = Solid(BRepOffsetAPI_MakePipe(path, profile, GeomFill_IsFrenet).Shape()).moved(Location((0, 0, 12)))
core = Cylinder(radius, abs(lead) * turns, align=(Align.CENTER, Align.CENTER, Align.MIN)) - tool
shank = Cylinder(radius, 3, align=(Align.CENTER, Align.CENTER, Align.MIN)).moved(Location((0, 0, 12)))
head = Cylinder(16, 6, align=(Align.CENTER, Align.CENTER, Align.MIN)).moved(Location((0, 0, 15)))
socket = extrude(Plane.XY.offset(18) * RegularPolygon(6, 6), amount=4)
screw = core.fuse(shank, head - socket)
thick_annulus = Cylinder(4, 6) - Cylinder(2, 8)
one_end = Cylinder(2, 10)
one_end = chamfer(one_end.edges().filter_by(Axis.Z, reverse=True).sort_by(Axis.Z)[:1], length=0.3)
shapes = [washer, dowel, rounded, plain, stepped, screw, core, thick_annulus, one_end]
for i, shape in enumerate(shapes):
    shape.label = f"body_{i + 1}"
    shape.location = Location((i * 20, 0, 0))
assembly = Compound(children=shapes)
assembly.label = "neutral shape candidates"
path = root / "pack-shapes.step"
export_step(assembly, path)
with gzip.GzipFile(filename="", fileobj=(root / "pack-shapes.step.gz").open("wb"), mode="wb", mtime=0) as out:
    out.write(path.read_bytes())
path.unlink()
