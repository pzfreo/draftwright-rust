"""Record draftwright's sheets of a sample of corpus parts as facts, for a semantic comparison.

    ~/repos/draftwright/.venv/bin/python tools/capture_sheet_facts.py <draftwright-src> <corpus> [name ...]

For each part (all of ``PARTS``, or the names given), builds the drawing as draftwright does by
default and records what the sheet says, not how it looks (``docs/facts.md`` has the schema):

- the binding (corpus path, sha256), the sheet (format, page, scale, projection convention);
- each view: its kind, its camera as a world basis (page x, page y, towards the viewer), its
  scale, and where it sits on the page (the look-at point's page position and the projected
  geometry's box);
- each annotation the registry names: its type, view, printed text, label box, and the
  measurements it draws, each with its feature, parameter, compiled value and tolerance;
- tables' rows, the title block's fields, the section and detail decisions, and the lint
  findings.

Writes ``tests/fixtures/sheets/<name>.json.gz``. The capture is deterministic: floats are rounded
to 1e-6 and every list is in draftwright's own (deterministic) order.
"""

from __future__ import annotations

import dataclasses
import enum
import gzip
import hashlib
import json
import math
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

from build123d_drafting.helpers import format_drawing_scale  # noqa: E402

from draftwright import build_drawing  # noqa: E402
from draftwright.annotations._sheet_furniture import _sheet_format  # noqa: E402
from draftwright.drawing import feature_key  # noqa: E402
from draftwright.pdf_text import pdf_text_runs  # noqa: E402

SCHEMA = "draftwright.sheet-facts/1"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "sheets"
PARTS = {
    "m10127": "mfcadpp/10127.step",
    "m1000": "mfcadpp/1000.step",
    "m10046": "mfcadpp/10046.step",
    "string_post": "gramel/string_post.step",
    "grm03": "gramel/GRM-03_thumbwheel_drive_screw.step",
    "grm05": "gramel/GRM-05_depth_lock_bolt.step",
    "spool132": "cadgenbench/flanged_spool_132.step",
    "connector109": "cadgenbench/threaded_connector_109.step",
    "cgb245": "cadgenbench_inputs/cgb245.step",
    "ctc01": "nist/nist_ctc_01_asme1_rd.stp",
    "ctc04": "nist/nist_ctc_04_asme1_rd.stp",
    "ftc06": "nist/nist_ftc_06_asme1_rd.stp",
    "hanger": "ttt_inputs/sm-hanger.step",
}


def _num(v: float) -> float:
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def _plain(v):
    """A JSON-safe, deterministic rendering of a value draftwright holds."""
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, enum.Enum):
        return v.name
    if isinstance(v, int):
        return v
    if isinstance(v, float):
        return _num(v) if math.isfinite(v) else str(v)
    if dataclasses.is_dataclass(v) and not isinstance(v, type):
        return {"type": type(v).__name__, **{f.name: _plain(getattr(v, f.name)) for f in dataclasses.fields(v)}}
    if isinstance(v, dict):
        return {str(k): _plain(x) for k, x in v.items()}
    if isinstance(v, (list, tuple, frozenset, set)):
        items = [_plain(x) for x in v]
        return sorted(items, key=json.dumps) if isinstance(v, (set, frozenset)) else items
    # Anything else (model objects, shapes) is named rather than guessed at.
    return f"<{type(v).__name__}>"


def _feature(f) -> dict | None:
    if f is None:
        return None
    frame = getattr(f, "frame", None)
    origin = getattr(frame, "origin", None)
    return {
        "key": feature_key(f),
        "kind": getattr(f, "kind", None) or type(f).__name__,
        "origin": None if origin is None else [_num(origin[i]) for i in range(3)],
        "axis": getattr(frame, "axis", None) or None,
    }


def _cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def _views(d) -> list[dict]:
    plan = d.view_plan
    # Section and detail views are added after the view plan is resolved; their kind comes from
    # the section mark or the detail decision that names them.
    derived = {decision.get("view"): "detail" for decision in d.detail_decisions}
    for name, _ in d.iter_annotations():
        if (mark := d.registry.section_of(name)) is not None:
            derived[mark.view] = "section"
    views = []
    for name in d.views:
        coords = d._coords.get(name)
        spec = plan.spec(name) if plan is not None else None
        record = {"name": name, "kind": spec.kind if spec is not None else derived.get(name)}
        bounds = d.view_bounds(name)
        record["bbox"] = None if bounds is None else [_num(b) for b in bounds]
        record["centre"] = None if bounds is None else [_num((bounds[0] + bounds[2]) / 2), _num((bounds[1] + bounds[3]) / 2)]
        if coords is not None:
            x, y = coords._page_x_vec, coords._page_y_vec
            c = coords._centers
            record.update(
                scale=_num(coords._scale),
                origin=[_num(coords._view_x), _num(coords._view_y)],
                look_at=[_num(c["world_X"]), _num(c["world_Y"]), _num(c["world_Z"])],
                x_axis=None if x is None else [_num(v) for v in x],
                up=None if y is None else [_num(v) for v in y],
                direction=None if x is None or y is None else [_num(v) for v in _cross(x, y)],
            )
        views.append(record)
    return views


def _claims(d) -> tuple[dict, list]:
    """The compiled meanings (value, tolerance, span) behind each named measurement, and the
    annotations whose meaning draftwright could not confirm."""
    snapshot = d.measurement_snapshot()
    claims: dict = {}
    for c in snapshot.claims:
        found = claims.setdefault((c.annotation, feature_key(c.owner), c.parameter, c.cell), [])
        for m in c.meaning:
            meaning = {"value": _plain(m[0]), "tolerance": _plain(m[1]), "span": _plain(m[2])}
            if meaning not in found:
                found.append(meaning)
    unknown = sorted({(name, reason) for name, reason in snapshot.unknown})
    return claims, [{"annotation": n, "reason": r} for n, r in unknown]


def _measurement(mid, compiled, span) -> dict:
    """One drawn measurement. A correlated ladder shares one identity, so its compiled meanings
    are told apart by the annotation's own span; ``value`` stays null when that cannot settle it."""
    record = {"feature": _feature(mid.feature), "parameter": mid.parameter, "compiled": compiled}
    chosen = compiled or []
    if len({json.dumps(m["value"]) for m in chosen}) > 1 and span is not None:
        chosen = [m for m in chosen if m["span"] == span]
    values = {json.dumps([m["value"], m["tolerance"]]) for m in chosen}
    record["value"], record["tolerance"] = json.loads(values.pop()) if len(values) == 1 else (None, None)
    return record


def _annotations(d, claims) -> list[dict]:
    registry = d.registry
    out = []
    for name, ann in d.iter_annotations():
        type_name = type(ann).__name__
        runs = pdf_text_runs(d.draft, [(name, ann)], dimension_spec_of=registry.dimension_spec_of)
        record = {
            "name": name,
            "type": type_name,
            "view": registry.view_of(name),
            "text": [r.text for r in runs],
            "label": _plain(getattr(ann, "label", None)),
            "label_bbox": _plain(getattr(ann, "label_bbox", None)),
            "features": [_feature(f) for f in registry.features_of(name)],
        }
        span = registry.measurement_span_of(name)
        span = None if span is None else _plain(span)
        record["measurements"] = [
            _measurement(mid, claims.get((name, feature_key(mid.feature), mid.parameter, None)), span)
            for mid in registry.measurement_of(name)
        ]
        cells = []
        for cell in registry.cells_of(name):
            mid = cell.measurement
            compiled = claims.get((name, feature_key(mid.feature), mid.parameter, (cell.row, cell.column)))
            cells.append({"cell": [cell.row, cell.column], **_measurement(mid, compiled, None)})
        if cells:
            record["cells"] = cells
        rows = getattr(ann, "table_rows", None)
        if rows:
            record["rows"] = [[str(c) for c in row] for row in rows]
        mark = registry.section_of(name)
        if mark is not None:
            record["section_mark"] = {"cut_y": _num(mark.cut_y), "view": mark.view}
        if span is not None:
            record["span"] = span
        out.append(record)
    return out


def _lint(d) -> list[dict]:
    return [
        {
            "code": i.code,
            "severity": i.severity,
            "message": i.message,
            "annotation": i.annotation_name,
            "related": list(i.related_annotation_names),
            "view": i.view,
            "measurements": _plain(i.measurement_ids),
            "source_ids": list(i.source_ids),
            "stage": i.outcome_stage,
        }
        for i in d.lint()
    ]


def capture(corpus: Path, rel: str) -> dict:
    path = corpus / rel
    with tempfile.TemporaryDirectory() as tmp:
        d = build_drawing(str(path), out=str(Path(tmp) / "sheet"))
    claims, unconfirmed = _claims(d)
    annotations = _annotations(d, claims)
    tb = d.get_annotation("title_block")
    title = {}
    for field, value, *_ in getattr(tb, "title_field_specs", ()):
        title[field] = value
    plan = d.view_plan
    return {
        "schema": SCHEMA,
        "binding": {"file": rel, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()},
        "producer": _plain(d.report()["producer"]),
        "sheet": {
            "format": _sheet_format(d.page_w, d.page_h),
            "page": [_num(d.page_w), _num(d.page_h)],
            "scale": format_drawing_scale(d.scale),
            "scale_factor": _num(d.scale),
            "projection": plan.convention if plan is not None else None,
        },
        "decisions": {
            "scale": _plain(d.scale_decision),
            "views": _plain(d.view_decision),
            "arrangement": _plain(d.arrangement_decision),
            "section": _plain(d.section_decision),
            "details": _plain(d.detail_decisions),
        },
        "views": _views(d),
        "annotations": annotations,
        "unconfirmed": unconfirmed,
        "title_block": title,
        "suppressions": _plain(d.suppressions()),
        "lint": _lint(d),
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    names = sys.argv[3:] or list(PARTS)
    OUT.mkdir(parents=True, exist_ok=True)
    failed = []
    for name in names:
        try:
            facts = capture(corpus, PARTS[name])
        except Exception as e:  # reported, and the run fails at the end; nothing is written
            print(name, "FAILED:", type(e).__name__, e, file=sys.stderr)
            failed.append(name)
            continue
        text = json.dumps(facts, allow_nan=False, ensure_ascii=False, sort_keys=True, indent=0) + "\n"
        (OUT / f"{name}.json.gz").write_bytes(gzip.compress(text.encode(), mtime=0))
        kinds = {}
        for a in facts["annotations"]:
            kinds[a["type"]] = kinds.get(a["type"], 0) + 1
        print(
            name,
            facts["sheet"]["format"],
            facts["sheet"]["scale"],
            [v["name"] for v in facts["views"]],
            kinds,
            len(facts["lint"]),
            "lint",
            file=sys.stderr,
        )
    if failed:
        sys.exit(f"not captured: {', '.join(failed)}")


if __name__ == "__main__":
    main()
