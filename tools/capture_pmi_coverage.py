"""Record draftwright's PMI coverage lint (``linting/pmi_coverage.py``) on each AP242 fixture,
for the port's coverage lint.

    ~/repos/draftwright/.venv/bin/python tools/capture_pmi_coverage.py <draftwright-src> [stem ...]

<draftwright-src> is draftwright's ``src`` directory (put first on the import path). The inputs
are the AP242 files of ``tests/fixtures/pmi/step/`` (``tests/fixtures/pmi/README.md``), or the
stems named. For each, ``tests/fixtures/pmi_coverage/<stem>.json.gz`` gets:

- ``issues``: every finding of ``build_drawing(..., pmi="annotate").lint()`` (draftwright's
  default build, A-series sheet, third angle) whose code ``pmi_coverage.py`` emits (``CODES``),
  in lint order: its code, severity, message, source IDs and annotation name;
- or ``error``: why draftwright could not draw the part.

``sha256`` is the STEP text's (decompressed). Values are draftwright's, as returned.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

from draftwright.builder import build_drawing  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
STEPS = ROOT / "tests" / "fixtures" / "pmi" / "step"
OUT = ROOT / "tests" / "fixtures" / "pmi_coverage"

# Every code `linting/pmi_coverage.py` emits.
CODES = frozenset(
    {
        "pmi_present_but_ignored",
        "pmi_not_extracted",
        "pmi_not_lowered",
        "pmi_not_rendered",
        "pmi_value_mismatch",
        "pmi_source_text_mismatch",
        "pmi_source_site_mismatch",
        "pmi_unreconciled",
        "pmi_source_unknown",
        "manufacturing_reference_unresolved",
        "step_material_mismatch",
        "step_material_disagreement",
        "step_material_unavailable",
        "step_material_ambiguous",
        "step_general_tolerance_mismatch",
        "step_general_tolerance_disagreement",
    }
)


def drawn(path: Path) -> dict:
    if os.environ.get("CAPTURE_NO_DRAWING"):
        return {"error": os.environ["CAPTURE_NO_DRAWING"]}
    with tempfile.TemporaryDirectory() as tmp:
        try:
            d = build_drawing(str(path), out=str(Path(tmp) / "sheet"), pmi="annotate")
        except Exception as exc:  # noqa: BLE001 - draftwright's failure is the record
            return {"error": f"{type(exc).__name__}: {exc}"}
        issues = [
            {
                "code": str(i.code),
                "severity": str(i.severity),
                "message": str(i.message),
                "source_ids": list(getattr(i, "source_ids", ()) or ()),
                "annotation_name": getattr(i, "annotation_name", None),
            }
            for i in d.lint()
            if str(i.code) in CODES
        ]
    return {"issues": issues}


def capture(path: Path) -> dict:
    data = path.read_bytes()
    out = {"file": path.name, "sha256": hashlib.sha256(data).hexdigest()}
    out.update(drawn(path))
    return out


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[2:])
    if not wanted:
        # One process per file: draftwright's drawing can crash OpenCascade outright, and a
        # crash is then recorded as the drawing's outcome.
        for gz in sorted(STEPS.glob("*.gz")):
            stem = gz.name.split(".")[0]
            run = subprocess.run([sys.executable, __file__, sys.argv[1], stem], check=False)
            if run.returncode != 0:
                env = dict(os.environ)
                env["CAPTURE_NO_DRAWING"] = f"draftwright crashed drawing it (exit {run.returncode})"
                subprocess.run([sys.executable, __file__, sys.argv[1], stem], env=env, check=True)
        return
    with tempfile.TemporaryDirectory() as tmp:
        for gz in sorted(STEPS.glob("*.gz")):
            stem = gz.name.split(".")[0]
            if stem not in wanted:
                continue
            path = Path(tmp) / gz.name[: -len(".gz")]
            path.write_bytes(gzip.decompress(gz.read_bytes()))
            record = capture(path)
            target = OUT / f"{stem}.json.gz"
            text = json.dumps(record, indent=1, sort_keys=True) + "\n"
            target.write_bytes(gzip.compress(text.encode(), mtime=0))
            print(stem, record.get("error") or len(record["issues"]), flush=True)


if __name__ == "__main__":
    main()
