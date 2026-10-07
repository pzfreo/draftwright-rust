"""Record draftwright's sheet-layout calls on a sample of corpus parts, for a parity check of the
port's layout arithmetic (``src/layout.rs``):

    ~/repos/draftwright/.venv/bin/python tools/capture_layout.py <draftwright-src> <corpus> [name ...]

Builds each part as draftwright does by default (the parts of ``capture_sheet_facts.PARTS``) and
records every top-level call of the two layout authorities, with their inputs and results:

- ``choose_scale``: the part's extents, the annotation strip depths, the requested scale, page and
  arrangements, the view set, the section count, the tables and the derived-view footprints; and
  the (scale, page, title-block width, arrangement) it picked;
- ``_layout_geometry``: the same inputs at one scale and page, the measured view blocks of a
  repack, and the view centres, the isometric's rectangle and the fit verdicts it returned.

Scale-dependent inputs (angular reservations, derived-view footprints) are evaluated at every
standard scale. Floats are written at full precision, so the port can be held to 1e-9. Identical
calls are recorded once, in first-call order. Writes ``tests/fixtures/layout/<name>.json.gz``.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

import draftwright.analysis as analysis_mod  # noqa: E402
import draftwright.builder as builder_mod  # noqa: E402
import draftwright.compose as compose  # noqa: E402
import draftwright.section_scale_recovery as recovery_mod  # noqa: E402
from draftwright import build_drawing  # noqa: E402
from draftwright._core import _SCALES, SheetMargins  # noqa: E402
from draftwright.annotation_layout_profile import layout_flag  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
from capture_sheet_facts import PARTS  # noqa: E402

SCHEMA = "draftwright.layout-calls/1"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "layout"
STRIP_FIELDS = (
    "right left top pv_halo fv_top fv_bottom front_hole_below pv_authored_top pv_bottom "
    "sv_top sv_bottom sv_right pv_location_top rv_right"
).split()
RESULT_FIELDS = (
    "FV_X FV_Y PV_X PV_Y SV_X SV_Y SECTION_X SECTION_Y iso_left iso_bottom iso_right iso_top "
    "ISO_X ISO_Y iso_valid iso_fits fits auto_fits arrangement vertical_gutter side_gutter "
    "x_offset view_gutters"
).split()


def _margins(m):
    if m is None:
        return None
    if isinstance(m, SheetMargins):
        return [m.left, m.right, m.top, m.bottom]
    return [float(m)] * 4


def _strips(s, scales):
    if s is None:
        return None
    out = {f: float(getattr(s, f)) for f in STRIP_FIELDS}
    scheme = s.scheme
    demands = scheme.demands if scheme is not None else ()
    # The two facts the layout reads from the annotation scheme.
    out["side_ladder"] = sum(
        d.view == "side" and d.side == "above" and d.family != "feature_leader" for d in demands
    )
    out["plan_demand"] = any(d.view == "plan" for d in demands)
    out["side_step_y"] = any(
        d.view == "side"
        and d.side == "above"
        and any(
            i.parameter == "step.length" and i.feature.frame.axis == "y" for i in d.measurements
        )
        for d in demands
    )
    out["angular"] = {
        repr(float(k)): [[r.view, [float(c) for c in r.footprint(k)]] for r in s.angular]
        for k in scales
    }
    return out


def _derived(fn, scales):
    if fn is None:
        return None
    return {
        repr(float(k)): [[i, float(w), float(h)] for i, w, h in fn(k)] for k in scales
    }


def _common(kw, scales):
    return {
        "strips": _strips(kw.get("strips"), scales),
        "n_steps": int(kw.get("n_steps", 0)),
        "section": int(kw.get("section", False)),
        "table_sizes": [[float(a), float(b)] for a, b in kw.get("table_sizes", ())],
        "required_tables": [
            [[float(a), float(b)], p] for (a, b), p in kw.get("required_tables", ())
        ],
        "margin": _margins(kw.get("margin", 10.0)),
        "title_block_margins": _margins(kw.get("title_block_margins")),
        "views": list(kw["views"]) if kw.get("views") is not None else None,
        "include_iso": bool(kw.get("include_iso", True)),
        "iso_scale_factor": kw.get("iso_scale_factor"),
        "convention": kw.get("convention", "third"),
        "view_gutters": layout_flag("view_gutters", "DRAFTWRIGHT_EXPERIMENT_VIEW_GUTTERS"),
    }


calls: list[dict] = []
depth = 0


def _record(entry):
    text = json.dumps(entry, sort_keys=True)
    if all(json.dumps(c, sort_keys=True) != text for c in calls):
        calls.append(entry)


def _wrap_choose(real):
    def choose_scale(x_size, y_size, z_size, n_steps=0, scale=None, page=None, **kw):
        global depth
        depth += 1
        try:
            pick = real(x_size, y_size, z_size, n_steps, scale, page, **kw)
        finally:
            depth -= 1
        if depth == 0:
            kw["n_steps"] = n_steps
            entry = {
                "call": "choose_scale",
                "size": [x_size, y_size, z_size],
                "scale": scale,
                "page": list(page) if isinstance(page, tuple) else page,
                "arrangements": list(kw["arrangements"]) if kw.get("arrangements") else None,
                "title_block_width": kw.get("title_block_width"),
                **_common(kw, _SCALES),
                "derived": _derived(kw.get("derived_view_footprints_for_scale"), _SCALES),
                "result": [*map(float, tuple(pick)), getattr(pick, "arrangement", "columns")],
            }
            _record(entry)
        return pick

    return choose_scale


def _wrap_layout(real):
    def _layout_geometry(x_size, y_size, z_size, scale, page_w, page_h, tb_w, strips, *args, **kw):
        global depth
        depth += 1
        try:
            g = real(x_size, y_size, z_size, scale, page_w, page_h, tb_w, strips, *args, **kw)
        finally:
            depth -= 1
        if depth == 0:
            names = ("n_steps", "blocks")
            kw.update(dict(zip(names, args)))
            kw["strips"] = strips
            blocks = kw.get("blocks")
            entry = {
                "call": "layout_geometry",
                "size": [x_size, y_size, z_size],
                "scale": scale,
                "page": [page_w, page_h],
                "tb_w": tb_w,
                "arrangement": kw.get("arrangement", "columns"),
                **_common(kw, [scale]),
                "blocks": {
                    k: [b.hw, b.hh, b.top, b.right, b.bottom, b.left] for k, b in blocks.items()
                }
                if blocks is not None
                else None,
                "derived_view_footprints": [
                    [i, float(w), float(h)] for i, w, h in kw.get("derived_view_footprints", ())
                ],
                "result": {
                    f: (float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else v)
                    for f in RESULT_FIELDS
                    for v in [getattr(g, f)]
                },
            }
            _record(entry)
        return g

    return _layout_geometry


choose = _wrap_choose(compose.choose_scale)
layout = _wrap_layout(compose._layout_geometry)
for module in (compose, analysis_mod):
    module.choose_scale = choose
for module in (compose, analysis_mod, builder_mod, recovery_mod):
    module._layout_geometry = layout


def capture(corpus: Path, rel: str) -> dict:
    path = corpus / rel
    calls.clear()
    with tempfile.TemporaryDirectory() as tmp:
        d = build_drawing(str(path), out=str(Path(tmp) / "sheet"))
    return {
        "schema": SCHEMA,
        "binding": {"file": rel, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()},
        "final": {
            "scale": float(d.scale_decision.get("effective_scale") or 0.0)
            if getattr(d, "scale_decision", None)
            else None,
        },
        "calls": list(calls),
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    names = sys.argv[3:] or list(PARTS)
    OUT.mkdir(parents=True, exist_ok=True)
    failed = 0
    for name in names:
        try:
            record = capture(corpus, PARTS[name])
        except Exception as e:  # noqa: BLE001 - report every failure and carry on
            print(f"{name}: draftwright failed: {e}", file=sys.stderr)
            failed += 1
            continue
        data = json.dumps(record, sort_keys=True, indent=1).encode() + b"\n"
        with gzip.GzipFile(OUT / f"{name}.json.gz", "wb", mtime=0) as f:
            f.write(data)
        print(f"{name}: {len(record['calls'])} calls")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
