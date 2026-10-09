"""Record how draftwright lowers and draws a part's AP242 geometric tolerances and datums, for
the port's PMI lowering.

    ~/repos/draftwright/.venv/bin/python tools/capture_pmi_lowering.py <draftwright-src> [stem ...]

<draftwright-src> is draftwright's ``src`` directory (put first on the import path). The inputs
are the AP242 files of ``tests/fixtures/pmi/step/`` (``tests/fixtures/pmi/README.md``), or the
stems named. For each, ``tests/fixtures/pmi/lowering/<stem>.json.gz`` gets:

- ``items``: ``draftwright.model.build_pmi_features`` on ``extract_pmi_report``'s records, in
  the file's own STEP space (no part frame, as ``tools/capture_pmi.py`` reads them and as the
  port draws a part), one per geometric-tolerance or datum record, by ``source_id``: what the
  record became (``control_frame``, ``datum_ref``, or ``raw`` with its ``lowering_blockers``),
  its view and strip, its site (the frame origin its leader hangs from), and what it states: a
  frame's characteristic, its display tolerance, zone, material modifier, datums and scope; a
  datum's letter and surface kind;
- ``drawn``: ``build_drawing(..., pmi="annotate")``'s outcome for them: each placed GD&T
  annotation's view and glyph text (characteristic and tolerance, or letter), and the source
  IDs of every ``pmi_dropped`` issue its lint reports (draftwright's default build, A-series
  sheet, third angle); or ``error``, why draftwright could not draw the part.

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

from draftwright.analysis import _import_step  # noqa: E402
from draftwright.builder import build_drawing  # noqa: E402
from draftwright.model.detect import build_pmi_features  # noqa: E402
from draftwright.pmi import extract_pmi_report  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests" / "fixtures" / "pmi"
OUT = FIXTURES / "lowering"


def item(record, feature) -> dict:
    out = {
        "source_id": record.source_id,
        "category": record.source_category,
        "kind": record.kind,
        "lowered": feature.kind if feature.kind in ("control_frame", "datum_ref") else "raw",
    }
    if feature.kind == "control_frame":
        out.update(
            view=feature.view,
            side=feature.side,
            site=list(feature.frame.origin),
            characteristic=feature.characteristic,
            tolerance=feature.display_tolerance or feature.tolerance,
            diameter=feature.diameter,
            spherical_diameter=feature.spherical_diameter,
            modifier=feature.modifier,
            datums=list(feature.datums),
            all_around=feature.all_around,
            all_over=feature.all_over,
        )
    elif feature.kind == "datum_ref":
        out.update(
            view=feature.view,
            side=feature.side,
            site=list(feature.frame.origin),
            letter=feature.letter,
            surface=feature.reference_surface_kind,
        )
    else:
        out["blockers"] = list(getattr(feature, "lowering_blockers", ()))
    return out


def drawn(path: Path) -> dict:
    if os.environ.get("CAPTURE_NO_DRAWING"):
        return {"error": os.environ["CAPTURE_NO_DRAWING"]}
    with tempfile.TemporaryDirectory() as tmp:
        try:
            d = build_drawing(str(path), out=str(Path(tmp) / "sheet"), pmi="annotate")
        except Exception as exc:  # noqa: BLE001 - draftwright's failure is the record
            return {"error": f"{type(exc).__name__}: {exc}"}
    placed = []
    for name, ann in d.iter_annotations():
        if not name.startswith("m_gdt"):
            continue
        placed.append(
            {
                "name": name,
                "view": d.view_of(name),
                # The glyph's text tokens (`_gdt_pdf_text_specs`): the zone sign, the value,
                # the material letter and the datum letters, or a datum's letter.
                "text": [s[0] for s in getattr(ann, "pdf_text_relative_specs", ()) or ()],
                "visual_tolerance": getattr(ann, "gdt_visual_tolerance", None),
                "visual_zone": getattr(ann, "gdt_visual_zone", None),
            }
        )
    dropped = sorted(
        {s for i in d.lint() if i.code == "pmi_dropped" for s in i.source_ids}
    )
    return {"placed": sorted(placed, key=lambda p: p["name"]), "dropped": dropped}


def capture(path: Path) -> dict:
    data = path.read_bytes()
    report = extract_pmi_report(str(path))
    # draftwright's own reader (build123d's XCAF import crashes on NIST CTC-02, draftwright #20).
    part = _import_step(str(path))
    features = build_pmi_features(report.records, part.bounding_box())
    assert len(features) == len(report.records), "one feature per record"
    items = [
        item(r, f)
        for r, f in zip(report.records, features, strict=True)
        if r.source_category in ("geometric_tolerance", "datum")
    ]
    return {
        "file": path.name,
        "sha256": hashlib.sha256(data).hexdigest(),
        "items": items,
        "drawn": drawn(path),
    }


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[2:])
    if not wanted:
        # One process per file: draftwright's drawing can crash OpenCascade outright, and a
        # crash is then recorded as the drawing's outcome, the lowering still captured.
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
            if wanted and stem not in wanted:
                continue
            path = Path(tmp) / gz.name[: -len(".gz")]
            path.write_bytes(gzip.decompress(gz.read_bytes()))
            record = capture(path)
            target = OUT / f"{stem}.json.gz"
            text = json.dumps(record, indent=1, sort_keys=True) + "\n"
            target.write_bytes(gzip.compress(text.encode(), mtime=0))
            print(stem, len(record["items"]), record["drawn"].get("error", "drawn"), flush=True)


if __name__ == "__main__":
    main()
