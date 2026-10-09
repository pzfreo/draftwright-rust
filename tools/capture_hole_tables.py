"""Record draftwright's drop-and-escalate on the parts the port drops hole items on, for
`tests/hole_tables.rs`.

    ~/repos/draftwright/.venv/bin/python -I tools/capture_hole_tables.py <draftwright-src> <corpus>

Builds each part as draftwright does by default and records: the sheet; every hole table its
escalation attempted (`orchestrator._maybe_tabulate_holes_impl`'s `TAG | ⌀ | DEPTH | X | Y` rows,
read where they are wrapped, `_wrap_rows`, before the table is placed), with whether the
finished drawing keeps it; each balloon on the finished drawing, by name and box; and the drop,
table and balloon findings (`*_dropped`). Writes ``tests/fixtures/hole_tables.json``;
deterministic (floats rounded to 1e-6). Slow: each part is a whole build (minutes for NIST
CTC-02).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[1])

import draftwright.annotations.orchestrator as orchestrator  # noqa: E402
from draftwright import build_drawing  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "hole_tables.json"
PARTS = {
    "ftc09": "nist/nist_ftc_09_asme1_rd.stp",
    "ftc08": "nist/nist_ftc_08_asme1_rc.stp",
    "ctc02": "nist/nist_ctc_02_asme1_rc.stp",
    "ctc04": "nist/nist_ctc_04_asme1_rd.stp",
    "cgb245": "cadgenbench_inputs/cgb245.step",
}

attempts: list = []
_wrap = orchestrator._wrap_rows


def _recording_wrap(header, data, ncols):
    rows = [list(header)] + [list(r) for r in data]
    if ncols == 1 and rows not in attempts:
        attempts.append(rows)
    return _wrap(header, data, ncols)


orchestrator._wrap_rows = _recording_wrap


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def main():
    corpus = Path(sys.argv[2])
    out = {}
    for name, rel in PARTS.items():
        attempts.clear()
        dwg = build_drawing(str(corpus / rel))
        names = dwg.annotations()
        balloons = []
        for n in names:
            if n.startswith("balloon"):
                bb = dwg.get_annotation(n).bounding_box()
                balloons.append(
                    {"name": n, "box": [_num(v) for v in (bb.min.X, bb.min.Y, bb.max.X, bb.max.Y)]}
                )
        out[name] = {
            "sheet": [_num(dwg.page_w), _num(dwg.page_h)],
            "scale": _num(dwg.scale),
            "attempted_tables": list(attempts),
            "table_kept": "hole_table_plan" in names,
            "balloons": balloons,
            "findings": [
                [i.code, i.message]
                for i in dwg.registry.issues
                if i.code.endswith("_dropped")
                and i.code
                in {
                    "callout_dropped",
                    "location_ref_dropped",
                    "off_axis_location_dropped",
                    "table_dropped",
                    "balloon_dropped",
                }
            ],
        }
        print(name, len(attempts), "attempted", out[name]["table_kept"], file=sys.stderr)
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
