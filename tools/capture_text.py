"""Record build123d's text geometry, as draftwright draws and measures its labels.

    ~/repos/draftwright/.venv/bin/python tools/capture_text.py

For each of draftwright's fonts (``fonts/``, vendored from draftwright), each sample string and
size: ``Text(..., align=(CENTER, CENTER))`` as draftwright makes it, its bounding box (what
draftwright measures labels by) and each glyph face — its area and its boundary edges, each
sampled within 0.02 µm, in the text's plane. Writes ``tests/fixtures/text.json.gz``.
"""

from __future__ import annotations

import gzip
import json
import math
from pathlib import Path

from build123d import Align, Edge, GeomType, Mode, Text

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "text.json.gz"
FONTS = ["IBMPlexMono-Regular.ttf", "IBMPlexSansCondensed-Regular.ttf"]
#: Labels of the kinds draftwright draws: dimensions, callouts, notes, title blocks; a glyph
#: neither font has (U+2300, drawn as the missing-glyph box) and two lines.
STRINGS = [
    "0123456789",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "abcdefghijklmnopqrstuvwxyz",
    "4X ø5.5 THRU",
    "R5 ±0.05 45° ×2",
    "M6x1.0-6H",
    "SECTION A–A",
    "DETAIL B (2:1)",
    "Ra 3.2 / (%) [ok]; \"q\" 'q' @#&*",
    "⌀10",
    "A\nA",
]
SIZES = [3.5]
#: A second size, for the scaling alone.
SCALED = [("0123456789", 5.0), ("SECTION A–A", 2.5)]


def _polyline(edge: Edge) -> list[list[float]]:
    """An edge's points in the text's plane, its chords within 0.02 µm of it."""

    def at(t):
        p = edge.position_at(t)
        return (p.X, p.Y)

    if edge.geom_type == GeomType.LINE:
        ts = [0.0, 1.0]
    else:
        ts = [i / 8 for i in range(9)]
        for _ in range(16):
            refined, split = [ts[0]], False
            for a, b in zip(ts, ts[1:]):
                pa, pb, pm = at(a), at(b), at(0.5 * (a + b))
                if math.dist([(x + y) / 2 for x, y in zip(pa, pb)], pm) > 2e-5:
                    refined.append(0.5 * (a + b))
                    split = True
                refined.append(b)
            ts = refined
            if not split:
                break
    return [[round(x, 7), round(y, 7)] for x, y in map(at, ts)]


def _record(font: str, txt: str, size: float) -> dict:
    text = Text(
        txt=txt,
        font_size=size,
        font_path=str(ROOT / "fonts" / font),
        align=(Align.CENTER, Align.CENTER),
        mode=Mode.PRIVATE,
    )
    box = text.bounding_box()
    return {
        "font": font,
        "text": txt,
        "size": size,
        "box": [[box.min.X, box.min.Y], [box.max.X, box.max.Y]],
        "faces": [
            {"area": face.area, "edges": [_polyline(e) for e in face.edges()]}
            for face in text.faces()
        ],
    }


def main() -> None:
    cases = [(f, s, z) for f in FONTS for s in STRINGS for z in SIZES]
    cases += [(FONTS[0], s, z) for s, z in SCALED]
    records = [_record(*case) for case in cases]
    text = json.dumps({"cases": records}, allow_nan=False, ensure_ascii=False) + "\n"
    OUT.write_bytes(gzip.compress(text.encode(), mtime=0))
    print(len(records), "cases", sum(len(r["faces"]) for r in records), "faces")


if __name__ == "__main__":
    main()
