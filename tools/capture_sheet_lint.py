"""Record what draftwright's structural lint reads on a sample of corpus sheets, for lint's sheet checks.

    ~/repos/draftwright/.venv/bin/python tools/capture_sheet_lint.py <draftwright-src> <corpus> [name ...]

<draftwright-src> is draftwright's ``src`` directory (put first on the import path) and <corpus>
is quiddity's ``tests/corpus``. For each part (``capture_sheet_facts.PARTS``, or the names
given), builds the drawing as draftwright does by default, lints it, and records the arguments
of that lint's one ``lint_drawing`` call and what it returned:

- each item, in lint's order: its registry name (none for an unnamed item), type, label text,
  label box, exact label polygon, line-work (``segments``, page mm), whole ink box, a leader's
  tip and elbow, a centreline's own segments and component boxes, the placement region, and
  the riders lint dispatches on (centreline, sheet frame, zone grid, section hatch, datum
  target, scale note, dimension-like);
- each title-block field draftwright checks: its value, its cell's size and the value's measured
  text size;
- each view: its name, box, and visible edges, a straight edge by its ends and any other edge by
  the points lint samples it at (``_edges_intersect_rect``);
- the page's drawable area, and every finding the call returned.

Writes ``tests/fixtures/sheet_lint/<name>.json.gz``. Deterministic: floats rounded to 1e-6, every
list in draftwright's order.
"""

from __future__ import annotations

import gzip
import json
import sys
import tempfile
from pathlib import Path

USAGE = "usage: capture_sheet_lint.py <draftwright-src> <corpus> [name ...]"

if __name__ == "__main__":
    if sys.argv[1:2] in (["-h"], ["--help"]):
        print(__doc__.strip())
        sys.exit(0)
    if len(sys.argv) < 3:
        sys.exit(f"{USAGE}\n(--help for more)")

sys.path.insert(0, sys.argv[1])
sys.path.insert(0, str(Path(__file__).resolve().parent))

from build123d import GeomType  # noqa: E402

import draftwright.linting.orchestration as orchestration  # noqa: E402
from capture_sheet_facts import PARTS, _num  # noqa: E402
from draftwright import build_drawing  # noqa: E402
from draftwright._core import _text_size  # noqa: E402
from draftwright.linting.structural import _ann_box, is_dimension_like  # noqa: E402

SCHEMA = "draftwright.sheet-lint/1"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "sheet_lint"

_calls: list = []
_original = orchestration.lint_drawing


def _recording(items, **kwargs):
    issues = _original(items, **kwargs)
    _calls.append((list(items), kwargs, list(issues)))
    return issues


orchestration.lint_drawing = _recording


def _pt(p):
    if hasattr(p, "X"):
        return [_num(p.X), _num(p.Y)]
    return [_num(p[0]), _num(p[1])]


def _box(b):
    return None if b is None else [_num(v) for v in b]


def _segments(value):
    return [[_pt(a), _pt(b)] for a, b in (value or ())]


def _item(item, names, regions, cache) -> dict:
    record = {
        "name": names.get(id(item)),
        "type": type(item).__name__,
        "label": getattr(item, "label", None) or getattr(item, "_annotate_label", None) or "",
        "label_bbox": _box(getattr(item, "label_bbox", None)),
        "label_polygon": None,
        "segments": _segments(getattr(item, "segments", None)),
        "ink": _box(_ann_box(item, cache)),
        "region": regions.get(id(item)),
    }
    polygon = getattr(item, "label_polygon", None)
    if polygon is not None:
        record["label_polygon"] = [_pt(p) for p in polygon]
    for rider in (
        "is_centerline",
        "is_sheet_frame",
        "is_zone_grid",
        "is_zone_label",
        "is_section_hatch",
        "is_datum_target",
        "is_scale_note",
    ):
        if getattr(item, rider, False):
            record[rider] = True
    if is_dimension_like(item):
        record["dimension_like"] = True
    if getattr(item, "elbow", None) is not None:
        record["elbow"] = _pt(item.elbow)
        record["tip"] = _pt(item.tip)
    if getattr(item, "is_centerline", False):
        own = getattr(item, "centerline_segments", None)
        if own is not None:
            record["centerline_segments"] = _segments(own)
        record["centerline_boxes"] = [_box(b) for b in getattr(item, "centerline_boxes", ())]
    fields = []
    for field, value, font_size, font_path in getattr(item, "title_field_specs", ()):
        cell = item.cell_bbox(field)
        width, height = _text_size(value, font_size, font_path)
        fields.append(
            {
                "field": field,
                "value": value,
                "cell": [_num(cell["width"]), _num(cell["height"])],
                "text": [_num(width), _num(height)],
            }
        )
    if fields:
        record["title_fields"] = fields
    return record


def _edge(e) -> dict:
    if e.geom_type == GeomType.LINE:
        return {"line": [_pt(e.start_point()), _pt(e.end_point())]}
    n = min(200, max(8, int(e.length) + 1))
    return {"points": [_pt(e.position_at(i / n)) for i in range(n + 1)]}


def capture(corpus: Path, rel: str) -> dict:
    path = corpus / rel
    with tempfile.TemporaryDirectory() as tmp:
        d = build_drawing(str(path), out=str(Path(tmp) / "sheet"))
    _calls.clear()
    d.lint()
    if len(_calls) != 1:
        raise RuntimeError(f"expected one lint_drawing call, saw {len(_calls)}")
    items, kwargs, issues = _calls[0]
    names = kwargs.get("annotation_names") or {}
    regions = kwargs.get("annotation_regions") or {}
    cache = {}
    views = []
    for name, shape in zip(kwargs["view_names"], kwargs["view_shapes"], strict=True):
        views.append(
            {"name": name, "bbox": _box(_ann_box(shape, cache)), "edges": [_edge(e) for e in shape.edges()]}
        )
    return {
        "schema": SCHEMA,
        "file": rel,
        "page_bbox": _box(kwargs.get("page_bbox")),
        "scale": _num(kwargs.get("drawing_scale", 1.0)),
        "items": [_item(i, names, regions, cache) for i in items],
        "views": views,
        "issues": [
            {
                "code": i.code,
                "severity": i.severity,
                "message": i.message,
                "annotation": i.annotation_name,
                "related": list(i.related_annotation_names),
                "view": i.view,
            }
            for i in issues
        ],
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    names = sys.argv[3:] or list(PARTS)
    if unknown := [n for n in names if n not in PARTS]:
        sys.exit(f"{USAGE}\nunknown part: {', '.join(unknown)} (known: {', '.join(PARTS)})")
    OUT.mkdir(parents=True, exist_ok=True)
    failed = []
    for name in names:
        try:
            record = capture(corpus, PARTS[name])
        except Exception as e:  # reported, and the run fails at the end; nothing is written
            print(name, "FAILED:", type(e).__name__, e, file=sys.stderr)
            failed.append(name)
            continue
        text = json.dumps(record, allow_nan=False, ensure_ascii=False, sort_keys=True, indent=0) + "\n"
        (OUT / f"{name}.json.gz").write_bytes(gzip.compress(text.encode(), mtime=0))
        codes = {}
        for i in record["issues"]:
            codes[i["code"]] = codes.get(i["code"], 0) + 1
        print(name, len(record["items"]), "items", len(record["views"]), "views", codes, file=sys.stderr)
    if failed:
        sys.exit(f"not captured: {', '.join(failed)}")


if __name__ == "__main__":
    main()
