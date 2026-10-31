"""Synthetic GA section/detail fixture. Regenerate with build123d from repo root."""
from pathlib import Path
import gzip
from build123d import Box, Compound, Cylinder, export_step

root = Path(__file__).resolve().parent
plate = Box(60, 40, 6).cut(Cylinder(5, 20))
plate.label = "mount plate"
bushing = Cylinder(5, 6).cut(Cylinder(4, 20))
bushing.label = "bushing"
liner = Cylinder(4, 6).cut(Cylinder(3.75, 20))
liner.label = "liner"
assembly = Compound(children=[plate, bushing, liner])
assembly.label = "ASSEMBLY"
path = root / "pack-nested.step"
export_step(assembly, path)
with path.with_suffix(".step.gz").open("wb") as out:
    with gzip.GzipFile(filename="", fileobj=out, mode="wb", mtime=0) as compressed:
        compressed.write(path.read_bytes())
path.unlink()
