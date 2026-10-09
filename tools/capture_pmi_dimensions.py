"""Record how draftwright lowers and draws a part's AP242 dimensions (toleranced, limit, fit and
basic sizes and locations), for the port's PMI dimensions (stage 2b).

    ~/repos/draftwright/.venv/bin/python tools/capture_pmi_dimensions.py <draftwright-src> [stem ...]

<draftwright-src> is draftwright's ``src`` directory (put first on the import path). The inputs
are the AP242 files of ``tests/fixtures/pmi/step/`` (``tests/fixtures/pmi/README.md``), or the
stems named. For each, ``tests/fixtures/pmi/dimensions/<stem>.json.gz`` gets one entry per
dimension record of ``extract_pmi_report`` (in the file's own STEP space, as
``tools/capture_pmi.py`` reads them), by ``source_id``:

- what the record states and where it attaches: kind, value, label, basic, dominant axis, the
  reference stations (``ref_pts``), box, exact cylinder and circle supports, and its
  rendering and lowering blockers;
- ``drawn``: ``build_drawing(..., pmi="annotate")``'s placed terms that cite it
  (``Drawing.dimension_sources``): the annotation, its view, the parameter it states (``None``
  for a standalone PMI dimension, ``bore.diameter`` and the like when draftwright's lowering
  gave the requirement to a recognised feature's callout), and the annotation's label;
- ``issues``: every lint issue naming it (code, severity, message).

Or ``error``: why draftwright could not draw the part (default build, A-series sheet, third
angle). ``sha256`` is the STEP text's (decompressed). Values are draftwright's, as returned.
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
from draftwright.pmi import extract_pmi_report  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests" / "fixtures" / "pmi"
OUT = FIXTURES / "dimensions"


def _pt(p) -> list[float]:
    return [float(x) for x in p]


def record(r) -> dict:
    return {
        "source_id": r.source_id,
        "kind": r.kind,
        "value": r.value,
        "label": r.label,
        "basic": bool(getattr(r, "basic", False)),
        "upper_tol": r.upper_tol,
        "lower_tol": r.lower_tol,
        "lower_bound": r.lower_bound,
        "upper_bound": r.upper_bound,
        "dominant_axis": r.dominant_axis,
        "ref_pts": [_pt(p) for p in r.ref_pts],
        "ref_bbox": list(r.ref_bbox) if r.ref_bbox is not None else None,
        "cylinders": [
            {
                "axis_direction": _pt(c.axis_direction),
                "axis_origin": _pt(c.axis_origin),
                "midpoint": _pt(c.midpoint),
                "radius": c.radius,
                "principal_axis": c.principal_axis,
            }
            for c in getattr(r, "cylindrical_refs", ())
        ],
        "circles": [
            {"center": _pt(c.center), "normal": _pt(c.normal), "radius": c.radius}
            for c in getattr(r, "circular_refs", ())
        ],
        "angular": getattr(r, "angular_reference", None) is not None
        or bool(getattr(r, "angular_references", ())),
        "rendering_blockers": list(
            dict.fromkeys((*r.rendering_blockers, *getattr(r, "source_value_blockers", ())))
        ),
        "lowering_blockers": list(r.lowering_blockers),
        "view": getattr(r, "view", None),
    }


def drawing(path: Path, ids: set[str]) -> dict:
    if os.environ.get("CAPTURE_NO_DRAWING"):
        return {"error": os.environ["CAPTURE_NO_DRAWING"]}
    with tempfile.TemporaryDirectory() as tmp:
        try:
            d = build_drawing(str(path), out=str(Path(tmp) / "sheet"), pmi="annotate")
        except Exception as exc:  # noqa: BLE001 - draftwright's failure is the record
            return {"error": f"{type(exc).__name__}: {exc}"}
    labels = {}
    for name, ann in d.iter_annotations():
        labels[name] = getattr(ann, "label", None)
    drawn: dict[str, list] = {}
    for row in d.dimension_sources():
        for sid in row["source_ids"]:
            if sid in ids:
                entry = {
                    "annotation": row["annotation"],
                    "view": row["view"],
                    "parameter": row["parameter_id"],
                    "label": labels.get(row["annotation"]),
                }
                if entry not in drawn.setdefault(sid, []):
                    drawn[sid].append(entry)
    issues: dict[str, list] = {}
    for i in d.lint():
        for sid in sorted(set(i.source_ids) & ids):
            issues.setdefault(sid, []).append(
                {"code": i.code, "severity": i.severity, "message": i.message}
            )
    return {"drawn": drawn, "issues": issues}


def capture(path: Path) -> dict:
    data = path.read_bytes()
    report = extract_pmi_report(str(path))
    records = [record(r) for r in report.records if r.source_category == "dimension"]
    out = drawing(path, {r["source_id"] for r in records}) if records else {"drawn": {}, "issues": {}}
    for r in records:
        if "error" in out:
            continue
        r["drawn"] = out["drawn"].get(r["source_id"], [])
        r["issues"] = sorted(
            out["issues"].get(r["source_id"], []), key=lambda i: (i["code"], i["message"])
        )
    result = {"file": path.name, "sha256": hashlib.sha256(data).hexdigest(), "records": records}
    if "error" in out:
        result["error"] = out["error"]
    return result


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[2:])
    if not wanted:
        # One process per file: draftwright's drawing can crash OpenCascade outright, and a
        # crash is then recorded as the drawing's outcome, the records still captured.
        for gz in sorted((FIXTURES / "step").glob("*.gz")):
            stem = gz.name.split(".")[0]
            run = subprocess.run([sys.executable, __file__, sys.argv[1], stem], check=False)
            if run.returncode != 0:
                env = dict(os.environ)
                env["CAPTURE_NO_DRAWING"] = f"draftwright crashed drawing it (exit {run.returncode})"
                subprocess.run([sys.executable, __file__, sys.argv[1], stem], env=env, check=True)
        return
    with tempfile.TemporaryDirectory() as tmp:
        for gz in sorted((FIXTURES / "step").glob("*.gz")):
            stem = gz.name.split(".")[0]
            if stem not in wanted:
                continue
            path = Path(tmp) / gz.name[: -len(".gz")]
            path.write_bytes(gzip.decompress(gz.read_bytes()))
            rec = capture(path)
            target = OUT / f"{stem}.json.gz"
            text = json.dumps(rec, indent=1, sort_keys=True) + "\n"
            target.write_bytes(gzip.compress(text.encode(), mtime=0))
            print(stem, len(rec["records"]), rec.get("error", "drawn"), flush=True)


if __name__ == "__main__":
    main()
