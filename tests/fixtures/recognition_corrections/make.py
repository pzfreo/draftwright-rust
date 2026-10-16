"""Four true bolt-circle holes, corroborated by a central bore, for authored grouping.

Run with Python draftwright's environment (read-only):
  /Users/paul/repos/draftwright/.venv/bin/python -I -B tests/fixtures/recognition_corrections/make.py
"""
import gzip
from pathlib import Path
from tempfile import TemporaryDirectory
from build123d import Box, Cylinder, Pos, export_step

part = Box(80, 60, 8) - Cylinder(6, 12)
for x, y in [(20, 0), (0, 20), (-20, 0), (0, -20)]:
    part -= Pos(x, y, 0) * Cylinder(2, 12)
with TemporaryDirectory() as temp:
    path = Path(temp) / 'bolt_circle.step'
    export_step(part, path)
    Path(__file__).with_name('bolt_circle.step.gz').write_bytes(gzip.compress(path.read_bytes(), mtime=0))
