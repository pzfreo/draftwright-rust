"""Record what draftwright's drawing shows of a part's AP242 PMI, item by item, for the port's
PMI drawings check (alpha 1's PMI gate).

    ~/repos/draftwright/.venv/bin/python tools/capture_pmi_drawings.py <draftwright-src> [stem ...]

<draftwright-src> is draftwright's ``src`` directory (put first on the import path). The inputs
are the AP242 files of ``tests/fixtures/pmi/step/`` (``tests/fixtures/pmi/README.md``), or the
stems named. For each, ``tests/fixtures/pmi_drawings/<stem>.json.gz`` gets one entry per record
of ``extract_pmi_report`` (in the file's own STEP space, as ``tools/capture_pmi.py`` reads them,
so the ``source_id`` joins draftwright's reading there), by ``source_id``: its category, kind and
label, and what ``build_drawing(..., pmi="annotate")`` (default build, A-series sheet, third
angle) made of it:

- ``drawn``: every placed annotation that states it, with its view and what it shows:
  - a dimension term citing it (``Drawing.dimension_sources``): the annotation's label, the
    parameter it rides on (``None`` for a standalone PMI dimension, ``bore.diameter`` and the
    like when the requirement went to a recognised feature's callout), and ``boxed`` (a basic
    record drawn standalone: draftwright boxes it, or refuses it, ``_basic_unboxed_producer``);
  - a GD&T glyph (``m_gdt<i>``: the ``i``-th control frame, datum symbol, finish or note of the
    part model, the order ``render_gdt`` names them in): the glyph's text tokens
    (``_gdt_pdf_text_specs``: zone sign, value, material letter, datum letters; a datum's
    letter; a note's text), and a frame's characteristic and how many of the tokens are its
    datum letters;
  - a drawing-wide note in the ``general_notes`` table (``render_document_notes``): its text;
  - a document default: the general tolerance the title block prints
    (``_resolve_title_document_defaults``), or the default surface finish drawn beside it
    (``_add_default_surface_finish``);
- ``issues``: the codes of every lint issue naming it (``pmi_dropped``, ``callout_dropped``,
  ``pmi_unsupported`` and the like).

Positions are not recorded: the check compares what the drawing states, not where. Or
``error``: why draftwright could not draw the part. ``sha256`` is the STEP text's
(decompressed). Values are draftwright's, as returned.
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

from draftwright.annotations._gdt import _GDT_KINDS  # noqa: E402
from draftwright.annotations._pmi_dimensions import _pmi_source_ids  # noqa: E402
from draftwright.builder import build_drawing  # noqa: E402
from draftwright.pmi import extract_pmi_report  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
STEP = ROOT / "tests" / "fixtures" / "pmi" / "step"
OUT = ROOT / "tests" / "fixtures" / "pmi_drawings"


def drawing(path: Path, records) -> dict:
    if os.environ.get("CAPTURE_NO_DRAWING"):
        return {"error": os.environ["CAPTURE_NO_DRAWING"]}
    ids = {r.source_id for r in records}
    basic = {r.source_id for r in records if getattr(r, "basic", False)}
    with tempfile.TemporaryDirectory() as tmp:
        try:
            d = build_drawing(str(path), out=str(Path(tmp) / "sheet"), pmi="annotate")
        except Exception as exc:  # noqa: BLE001 - draftwright's failure is the record
            return {"error": f"{type(exc).__name__}: {exc}"}
    anns = dict(d.iter_annotations())
    drawn: dict[str, list] = {}

    def add(sid: str, entry: dict) -> None:
        if sid in ids and entry not in drawn.setdefault(sid, []):
            drawn[sid].append(entry)

    for row in d.dimension_sources():
        ann = anns.get(row["annotation"])
        for sid in row["source_ids"]:
            add(
                sid,
                {
                    "kind": "dimension",
                    "annotation": row["annotation"],
                    "view": row["view"],
                    "parameter": row["parameter_id"],
                    "text": getattr(ann, "label", None),
                    "boxed": sid in basic and row["parameter_id"] is None,
                },
            )
    # render_gdt's items, in its order: every control frame, datum symbol, finish and note of
    # the part model (in annotate mode none is filtered out), named m_gdt<i>.
    items = [f for f in d._part_model.features if f.kind in _GDT_KINDS]
    for i, item in enumerate(items):
        name = f"m_gdt{i}"
        ann = anns.get(name)
        if ann is None:
            continue
        text = [s[0] for s in getattr(ann, "pdf_text_relative_specs", ()) or ()]
        if item.kind == "note" and not text:
            text = [str(getattr(item, "text", ""))]
        entry = {"kind": item.kind, "annotation": name, "view": d.view_of(name), "text": text}
        if item.kind == "control_frame":
            entry["characteristic"] = item.characteristic
            entry["datums"] = len(item.datums)
        for sid in _pmi_source_ids(item):
            add(sid, entry)
    if "general_notes" in anns:
        for f in d._part_model.features:
            if f.kind != "document_note":
                continue
            for sid in _pmi_source_ids(f):
                add(
                    sid,
                    {
                        "kind": "document_note",
                        "annotation": "general_notes",
                        "view": None,
                        "text": [str(getattr(f, "text", ""))],
                    },
                )
    # Document defaults the title block carries (``_resolve_title_document_defaults``) and the
    # default surface finish drawn beside it (``_add_default_surface_finish``).
    tolerance = getattr(d._build, "general_tolerance_source", None)
    if tolerance is not None:
        for sid in _pmi_source_ids(tolerance):
            add(
                sid,
                {
                    "kind": "general_tolerance",
                    "annotation": "title_block",
                    "view": None,
                    "text": [str(tolerance.designation)],
                },
            )
    finish = getattr(d._build, "default_surface_finish_source", None)
    if finish is not None:
        for sid in _pmi_source_ids(finish):
            add(
                sid,
                {
                    "kind": "default_surface_finish",
                    "annotation": "title_block",
                    "view": None,
                    "text": [f"Ra {finish.ra}"],
                },
            )
    issues: dict[str, set] = {}
    for i in d.lint():
        for sid in set(i.source_ids) & ids:
            issues.setdefault(sid, set()).add(i.code)
    return {"drawn": drawn, "issues": {k: sorted(v) for k, v in issues.items()}}


def capture(path: Path) -> dict:
    data = path.read_bytes()
    report = extract_pmi_report(str(path))
    out = drawing(path, report.records)
    items = []
    for r in report.records:
        item = {
            "source_id": r.source_id,
            "category": r.source_category,
            "kind": r.kind,
            "label": r.label,
            "basic": bool(getattr(r, "basic", False)),
        }
        if "error" not in out:
            item["drawn"] = out["drawn"].get(r.source_id, [])
            item["issues"] = out["issues"].get(r.source_id, [])
        items.append(item)
    result = {"file": path.name, "sha256": hashlib.sha256(data).hexdigest(), "items": items}
    if "error" in out:
        result["error"] = out["error"]
    return result


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[2:])
    if not wanted:
        # One process per file: draftwright's drawing can crash OpenCascade outright, and a
        # crash is then recorded as the drawing's outcome, the records still captured.
        for gz in sorted(STEP.glob("*.gz")):
            stem = gz.name.split(".")[0]
            run = subprocess.run([sys.executable, __file__, sys.argv[1], stem], check=False)
            if run.returncode != 0:
                env = dict(os.environ)
                env["CAPTURE_NO_DRAWING"] = f"draftwright crashed drawing it (exit {run.returncode})"
                subprocess.run([sys.executable, __file__, sys.argv[1], stem], env=env, check=True)
        return
    with tempfile.TemporaryDirectory() as tmp:
        for gz in sorted(STEP.glob("*.gz")):
            stem = gz.name.split(".")[0]
            if stem not in wanted:
                continue
            path = Path(tmp) / gz.name[: -len(".gz")]
            path.write_bytes(gzip.decompress(gz.read_bytes()))
            rec = capture(path)
            target = OUT / f"{stem}.json.gz"
            text = json.dumps(rec, indent=1, sort_keys=True) + "\n"
            target.write_bytes(gzip.compress(text.encode(), mtime=0))
            drawn = sum(1 for i in rec["items"] if i.get("drawn"))
            print(stem, len(rec["items"]), drawn, rec.get("error", "drawn"), flush=True)


if __name__ == "__main__":
    main()
