"""Four true bolt-circle holes, corroborated by a central bore, for authored grouping.

Run with Python draftwright's environment (read-only):
  /Users/paul/repos/draftwright/.venv/bin/python -I -B tests/fixtures/recognition_corrections/make.py
"""
import gzip
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from build123d import Box, Cylinder, Pos, export_step

part = Box(80, 60, 8) - Cylinder(6, 12)
for x, y in [(20, 0), (0, 20), (-20, 0), (0, -20)]:
    part -= Pos(x, y, 0) * Cylinder(2, 12)
parts = {'bolt_circle': part, 'below_boss': Box(40, 30, 6) + Pos(0, 0, -5) * Cylinder(5, 4),
         'closed_void': Box(40,30,20) - Box(10,8,6),
         'local_pocket': (Box(60,50,12) + Pos(-20,0,12)*Cylinder(5,12)) - Pos(10,0,3)*Box(12,10,6)}
for name, shape in parts.items():
    if sys.argv[1:] and name not in sys.argv[1:]:
        continue
    with TemporaryDirectory() as temp:
        path = Path(temp) / (name + '.step')
        export_step(shape, path)
        Path(__file__).with_name(name + '.step.gz').write_bytes(gzip.compress(path.read_bytes(), mtime=0))
