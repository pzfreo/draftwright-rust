"""Record draftwright's declared corner-block tables on sampled corpus parts, for `tests/tables.rs`.

    ~/repos/draftwright/.venv/bin/python -I tools/capture_tables.py <draftwright-src> <corpus>

Builds each part as draftwright does by default, then again with the tables below declared as
`Sheet.build` declares `Sheet.table`'s: each measured by `_est_table_size` and reserved
(`_required_tables`), then placed after the build by `Drawing.add_table` (`_post_build`), in
order. Records each table: its rows, corner, measured size, its box on the page (or none, with
its `table_dropped` lint, when it has no place), its column edges and each cell's ink box (the
glyph faces whose centres fall in the cell), with the sheet and scale with and without the
tables. Writes ``tests/fixtures/tables.json``; deterministic (floats rounded to 1e-6).
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

from draftwright import build_drawing  # noqa: E402
from draftwright._core import _FONT_SIZE, _table_metrics  # noqa: E402
from draftwright.compose import _est_table_size  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "tables.json"
PAD = 2.0  # the draft preset's pad_around_text
TABLES = [
    {
        "name": "revisions",
        "prefer": "tr",
        "rows": [
            ["REV", "DESCRIPTION", "DATE", "APPROVED"],
            ["A", "INITIAL RELEASE", "2026-10-08", "PF"],
            ["B", "⌀6 HOLES REAMED H7", "2026-10-09", ""],
        ],
    },
    {
        "name": "bom",
        "prefer": "bl",
        "rows": [
            ["ITEM", "PART NO.", "DESCRIPTION", "QTY"],
            ["1", "DW-1001", "PLATE", "1"],
            ["2", "ISO 4762 M6x20", "SOCKET HEAD CAP SCREW", "4"],
        ],
    },
]
PARTS = {"m10127": "mfcadpp/10127.step", "ctc01": "nist/nist_ctc_01_asme1_rd.stp"}


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def _box(bb):
    return [_num(v) for v in (bb.min.X, bb.min.Y, bb.max.X, bb.max.Y)]


def _table(dwg, table) -> dict:
    rows = [tuple(r) for r in table["rows"]]
    w, h = _est_table_size(rows)
    out = {
        "name": table["name"],
        "prefer": table["prefer"],
        "rows": table["rows"],
        "size": [_num(w), _num(h)],
    }
    if table["name"] not in dwg.annotations():
        out["box"] = None
        out["lint"] = [
            i.message for i in dwg.registry.issues if i.code == "table_dropped"
        ]
        return out
    t = dwg.get_annotation(table["name"])
    bb = t.bounding_box()
    x0, y0 = bb.min.X, bb.min.Y
    lefts, rights, _w, total_h, row_h, _bc = _table_metrics(rows, _FONT_SIZE, PAD)
    out["box"] = _box(bb)
    out["columns"] = [_num(x0 + v) for v in lefts] + [_num(x0 + rights[-1])]
    # Each glyph face to the cell its centre lies in; a cell's ink is the union of its faces.
    ink = [[None for _ in r] for r in rows]
    for f in t.faces():
        fb = f.bounding_box()
        cx, cy = (fb.min.X + fb.max.X) / 2 - x0, (fb.min.Y + fb.max.Y) / 2 - y0
        ri = int((total_h - cy) // row_h)
        ci = next(c for c in range(len(lefts)) if lefts[c] <= cx <= rights[c])
        b = ink[ri][ci]
        box = (fb.min.X, fb.min.Y, fb.max.X, fb.max.Y)
        ink[ri][ci] = box if b is None else (
            min(b[0], box[0]), min(b[1], box[1]), max(b[2], box[2]), max(b[3], box[3])
        )
    out["ink"] = [[None if b is None else [_num(v) for v in b] for b in r] for r in ink]
    return out


def capture(corpus: Path, rel: str) -> dict:
    step = str(corpus / rel)
    required = tuple(
        (_est_table_size([tuple(r) for r in t["rows"]]), t["prefer"]) for t in TABLES
    )

    def place(dwg):
        for t in TABLES:
            dwg.add_table(
                [tuple(r) for r in t["rows"]],
                prefer=t["prefer"],
                name=t["name"],
                _source_id=f"sheet.table:{t['name']}",
            )
        return dwg

    with tempfile.TemporaryDirectory() as tmp:
        plain = build_drawing(step, out=str(Path(tmp) / "a"))
        d = build_drawing(
            step, out=str(Path(tmp) / "b"), _post_build=place, _required_tables=required
        )
    return {
        "file": rel,
        "bare": {"page": [_num(plain.page_w), _num(plain.page_h)], "scale": _num(plain.scale)},
        "page": [_num(d.page_w), _num(d.page_h)],
        "scale": _num(d.scale),
        "tables": [_table(d, t) for t in TABLES],
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    out = {
        "tables": TABLES,
        "parts": {name: capture(corpus, rel) for name, rel in PARTS.items()},
    }
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
