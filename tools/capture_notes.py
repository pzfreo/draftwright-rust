"""Record draftwright's general notes block on sampled corpus parts, for `tests/notes.rs`.

    ~/repos/draftwright/.venv/bin/python -I tools/capture_notes.py <draftwright-src> <corpus>

Builds each part as draftwright does by default, then again with the notes below added to its
model as document notes (`DocumentNote`, the source of `render_document_notes`), and records the
`general_notes` table: its rows, its measured size and its box on the page, with the sheet and
scale. Also records `document_note_rows` for wrapping cases. Writes
``tests/fixtures/notes.json``; deterministic (floats rounded to 1e-6).
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, sys.argv[1])

from draftwright import build_drawing  # noqa: E402
from draftwright.auxiliary_layout import document_note_rows  # noqa: E402
from draftwright.model.ir import DocumentNote, Frame  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "notes.json"
NOTES = [
    "BREAK ALL SHARP EDGES",
    "REMOVE ALL BURRS AND MACHINE ALL INTERNAL CORNERS TO R0.5 MAX UNLESS OTHERWISE STATED ON THIS DRAWING",
]
PARTS = {"m10127": "mfcadpp/10127.step", "ctc01": "nist/nist_ctc_01_asme1_rd.stp"}
WRAPS = [
    "DEBURR",
    "A" * 60,
    "SHORT " + "B" * 50 + " TAIL",
    "TWO  SPACES  KEPT  INSIDE  A  ROW  OF  THIS  NOTE  WHICH  WRAPS",
    "TAB\tAND\nNEWLINE AND HYPHENATED-WORDS-STAY-WHOLE-WHEN-WRAPPING-AT-FORTY-EIGHT",
    "EXACTLY FORTY EIGHT CHARACTERS LONG IN TOTAL OK!",
    "⌀6 H7 REAMED HOLES ARE INSPECTED WITH A GO / NO-GO GAUGE AFTER PLATING",
]


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def capture(corpus: Path, rel: str) -> dict:
    step = str(corpus / rel)
    with tempfile.TemporaryDirectory() as tmp:
        plain = build_drawing(step, out=str(Path(tmp) / "a"))
        notes = [
            DocumentNote(frame=Frame((0, 0, 0), "z"), text=t, note_kind="edge_condition")
            for t in NOTES
        ]
        d = build_drawing(step, out=str(Path(tmp) / "b"), model=list(plain.model().features) + notes)
    t = d.get_annotation("general_notes")
    bb = t.bounding_box()
    return {
        "file": rel,
        "page": [_num(d.page_w), _num(d.page_h)],
        "scale": _num(d.scale),
        "rows": [r[0] for r in t.table_rows],
        "size": [_num(v) for v in t.table_size],
        "box": [_num(v) for v in (bb.min.X, bb.min.Y, bb.max.X, bb.max.Y)],
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    out = {
        "notes": NOTES,
        "parts": {name: capture(corpus, rel) for name, rel in PARTS.items()},
        "wraps": [
            {"text": w, "rows": [r[0] for r in document_note_rows([SimpleNamespace(text=w)])[1:]]}
            for w in WRAPS
        ],
    }
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
