"""Record draftwright's drawings of a few corpus parts: what it exports and what it writes.

    ~/repos/draftwright/.venv/bin/python tools/capture_output.py <draftwright-src> <corpus>

For each part, builds the drawing as draftwright does and records, in page coordinates (mm):

- the content its exporters are handed, layer by layer (``part``, ``hidden``, ``dims``): each
  loose edge, and each face as its wires, every edge as its analytic curve (line, circle,
  ellipse, B-spline) with the parameter range and orientation drawn, and the Bézier pieces
  build123d's SVG exporter cuts it into;
- the page size, the attribution link's rectangle and the PDF's searchable text runs;
- the SVG, DXF and PDF draftwright writes from it (``reproducible``).

Writes ``tests/fixtures/output/<name>.{json.gz,svg.gz,dxf.gz,pdf}``.
"""

from __future__ import annotations

import gzip
import json
import shutil
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

from build123d import Edge, Face, GeomType  # noqa: E402
from OCP.GeomConvert import GeomConvert_BSplineCurveToBezierCurve  # noqa: E402
from OCP.TopAbs import TopAbs_REVERSED  # noqa: E402

from draftwright import build_drawing  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "output"
PARTS = {
    "m10127": "mfcadpp/10127.step",
    "string_post": "gramel/string_post.step",
    "ctc01": "nist/nist_ctc_01_asme1_rd.stp",
}


def _xy(p) -> list[float]:
    return [p.X(), p.Y()] if callable(p.X) else [p.X, p.Y]


def _edge(edge: Edge) -> dict:
    adaptor = edge.geom_adaptor()
    u0, u1 = adaptor.FirstParameter(), adaptor.LastParameter()
    record = {
        "reversed": edge.wrapped.Orientation() == TopAbs_REVERSED,
        "range": [u0, u1],
        "start": _xy(adaptor.Value(u0)),
        "end": _xy(adaptor.Value(u1)),
    }
    kind = edge.geom_type
    if kind == GeomType.LINE:
        record["kind"] = "line"
    elif kind in (GeomType.CIRCLE, GeomType.ELLIPSE):
        conic = adaptor.Circle() if kind == GeomType.CIRCLE else adaptor.Ellipse()
        position = conic.Position()
        record.update(
            kind="circle" if kind == GeomType.CIRCLE else "ellipse",
            centre=_xy(conic.Location()),
            radii=[conic.Radius()] * 2 if kind == GeomType.CIRCLE else [conic.MajorRadius(), conic.MinorRadius()],
            x_axis=_xy(position.XDirection()),
            y_axis=_xy(position.YDirection()),
        )
    else:
        record["kind"] = "bspline" if kind == GeomType.BSPLINE else str(kind)
    # The pieces build123d's SVG exporter cuts it into (any curve, through to_splines()).
    spline_edge = edge.to_splines()
    a = spline_edge.geom_adaptor()
    spline = a.Curve().Curve().Copy()
    spline.Transform(spline_edge.location.wrapped.Transformation())
    if kind == GeomType.BSPLINE:
        b = adaptor.BSpline()
        record["spline"] = {
            "degree": b.Degree(),
            "poles": [_xy(b.Pole(i)) for i in range(1, b.NbPoles() + 1)],
            "weights": [b.Weight(i) for i in range(1, b.NbPoles() + 1)],
            "knots": [b.Knot(i) for i in range(1, b.NbKnots() + 1)],
            "multiplicities": [b.Multiplicity(i) for i in range(1, b.NbKnots() + 1)],
            "periodic": b.IsPeriodic(),
        }
    converter = GeomConvert_BSplineCurveToBezierCurve(spline, a.FirstParameter(), a.LastParameter(), 1e-9)
    record["beziers"] = [
        [_xy(p) for p in converter.Arc(i).Poles()] for i in range(1, converter.NbArcs() + 1)
    ]
    return record


def _element(shape) -> dict:
    if isinstance(shape, Face):
        return {"face": [[_edge(e) for e in wire.order_edges()] for wire in [shape.outer_wire(), *shape.inner_wires()]]}
    return {"edge": _edge(shape)}


class _Recorder:
    """Stands in for an exporter: draftwright hands it each face and loose edge in turn."""

    def __init__(self):
        self.layers: dict[str, list] = {"part": [], "hidden": [], "dims": []}

    def add_shape(self, shape, layer):
        self.layers[layer].append(_element(shape))


def main() -> None:
    corpus = Path(sys.argv[2])
    OUT.mkdir(parents=True, exist_ok=True)
    for name, rel in PARTS.items():
        with tempfile.TemporaryDirectory() as tmp:
            stem = str(Path(tmp) / name)
            drawing = build_drawing(str(corpus / rel), out=stem)
            drawing.export(stem, formats=("svg", "dxf", "pdf"))
            recorder = _Recorder()
            drawing._add_shapes(recorder, ordered=False)
            link = getattr(drawing.get_annotation("title_block"), "draftwright_link_rect", None)
            runs = [
                {**run.__dict__, "font_path": Path(run.font_path).name if run.font_path else None}
                for run in drawing._pdf_text_runs()
            ]
            content = {
                "part": rel,
                "page": [drawing.page_w, drawing.page_h],
                "link": list(link) if link is not None else None,
                "layers": recorder.layers,
                "text_runs": runs,
            }
            text = json.dumps(content, allow_nan=False, ensure_ascii=False) + "\n"
            (OUT / f"{name}.json.gz").write_bytes(gzip.compress(text.encode(), mtime=0))
            for ext in ("svg", "dxf"):
                (OUT / f"{name}.{ext}.gz").write_bytes(gzip.compress(Path(f"{stem}.{ext}").read_bytes(), mtime=0))
            shutil.copy(f"{stem}.pdf", OUT / f"{name}.pdf")
        print(name, {k: len(v) for k, v in recorder.layers.items()}, len(runs), "text runs", file=sys.stderr)


if __name__ == "__main__":
    main()
