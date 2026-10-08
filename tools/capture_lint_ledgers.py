"""Record Python draftwright's completeness ledgers for the captured sheets' parts.

    ~/repos/draftwright/.venv/bin/python tools/capture_lint_ledgers.py <draftwright-src> <corpus> [name ...]

Python's lint reports no finding for the envelope's extents or the turned steps' bands: they are
completeness ledgers (``linting/envelope_coverage.py``, ``linting/turned_step_coverage.py``),
read by the quality report. So the sheet facts (``tools/capture_sheet_facts.py``) do not carry
them. For each part this builds the drawing as draftwright does by default (as the sheet facts
were captured) and records each ledger's outcomes:

- ``envelope``: each non-zero axis of the part's box, its parameter, extent and state;
- ``turned_steps``: each band's axis, axis line and span (quiddity's frame, as Python reads the
  record), its parameter and state.

Writes ``tests/fixtures/lint_ledgers.json``, deterministically (floats rounded to 1e-6, parts in
the facts' order). The parts are the sheet facts' (``capture_sheet_facts.PARTS``).
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])
sys.path.insert(0, str(Path(__file__).resolve().parent))

from capture_sheet_facts import PARTS  # noqa: E402

from draftwright import build_drawing  # noqa: E402
from draftwright.linting.envelope_coverage import envelope_requirement_outcomes  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "lint_ledgers.json"


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def ledgers(corpus: Path, rel: str) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        d = build_drawing(str(corpus / rel), out=str(Path(tmp) / "sheet"))
    snapshot = d.requirement_snapshot()
    envelope = [
        {
            "axis": o.axis,
            "parameter": o.parameter_id,
            "extent": _num(o.extent),
            "state": o.state,
        }
        for o in envelope_requirement_outcomes(snapshot.part, snapshot.registry, snapshot.omissions)
    ]
    turned = [
        {
            "axis": o.source_axis,
            "line": None if o.source_line is None else [_num(v) for v in o.source_line],
            "span": None if o.source_span is None else [_num(v) for v in o.source_span],
            "parameter": o.parameter_id,
            "state": o.state,
        }
        for o in snapshot.outcomes.get("turned_steps", ())
    ]
    return {"file": rel, "envelope": envelope, "turned_steps": turned}


def main() -> None:
    corpus = Path(sys.argv[2])
    names = sys.argv[3:] or list(PARTS)
    out = json.loads(OUT.read_text()) if OUT.exists() else {}
    failed = []
    for name in names:
        try:
            out[name] = ledgers(corpus, PARTS[name])
        except Exception as e:  # reported, and the run fails at the end
            print(name, "FAILED:", type(e).__name__, e, file=sys.stderr)
            failed.append(name)
            continue
        print(name, out[name]["envelope"], len(out[name]["turned_steps"]), file=sys.stderr)
    ordered = {name: out[name] for name in PARTS if name in out}
    OUT.write_text(json.dumps(ordered, allow_nan=False, indent=1, sort_keys=True) + "\n")
    if failed:
        sys.exit(f"not captured: {', '.join(failed)}")


if __name__ == "__main__":
    main()
