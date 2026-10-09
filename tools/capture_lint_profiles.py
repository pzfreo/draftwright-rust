"""Record what draftwright's principal profile scan reads and finds, for lint's profile check.

    ~/repos/draftwright/.venv/bin/python tools/capture_lint_profiles.py <draftwright-src> <root> <name>=<path> ...

<draftwright-src> is draftwright's ``src`` directory (put first on the import path); each part is
named and given by its path under <root> (quiddity's ``tests/corpus``, or this repository, for
``m10127=mfcadpp/10127.step`` or ``golden_dd=tests/fixtures/recognition/golden/double_d_bore.step``;
a ``.gz`` file is read decompressed). For each, reads the part as draftwright reads it
(``import_step``), recognises it with quiddity (``build_raw_recognition_result``, the aggregate
lint falls back to), and runs ``lint_principal_profile_coverage`` with that aggregate's double-D
bores and section recesses, as ``linting/orchestration.py`` does. Recorded:

- the solids it scans (each solid of a part of several, else the part), with their boxes;
- every inner wire of each principal boundary face (``_principal_boundary_plane``) of each, in
  draftwright's order (solids, faces, ``inner_wires()``): the face's axis and plane position, the
  wire's box across the plane, its edges' kinds, and four verdicts, taken in the order
  ``_supported_inner_profile`` takes them: whether it is an endpoint of a section recess record
  (``recess``), whether it claims a double-D bore's mouth not claimed before (``double_d``),
  whether its shape alone is in draftwright's vocabulary (``vocabulary``: circle, axis-aligned
  rectangle, obround or rounded rectangle; judged for every wire) and whether it is supported;
- each solid's proven radial outer arc count (``_radial_outer_arc_count``), or none;
- the aggregate's double-D bores, section recesses, and the closed ones among them (the only
  ones a wire can match), counted to bind the port's Analysis;
- the findings.

Every wire is judged, where draftwright's ``any`` stops at the first unsupported one: the
finding depends only on whether one is unsupported, and the double-D claims are made in the same
order, so the verdicts up to that wire are the ones draftwright makes.

Writes ``tests/fixtures/lint_coverage/profiles.json.gz`` (or the file ``CAPTURE_OUT`` names, to
capture in parallel and merge) deterministically (floats rounded to 1e-6); parts already in it and
not named are kept.
"""

from __future__ import annotations

import gzip
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

USAGE = "usage: capture_lint_profiles.py <draftwright-src> <root> <name>=<path> ..."

if __name__ == "__main__":
    if sys.argv[1:2] in (["-h"], ["--help"]):
        print(__doc__.strip())
        sys.exit(0)
    if len(sys.argv) < 4:
        sys.exit(f"{USAGE}\n(--help for more)")

sys.path.insert(0, sys.argv[1])

from build123d import GeomType, import_step  # noqa: E402
from quiddity import build_raw_recognition_result  # noqa: E402

import draftwright.linting.coverage as coverage  # noqa: E402

SCHEMA = "draftwright.lint-profiles/1"
ROOT = Path(__file__).resolve().parent.parent
OUT = Path(os.environ.get("CAPTURE_OUT") or ROOT / "tests" / "fixtures" / "lint_coverage" / "profiles.json.gz")
KINDS = {GeomType.LINE: "line", GeomType.CIRCLE: "circle"}


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def _box(bb) -> list:
    return [_num(bb.min.X), _num(bb.min.Y), _num(bb.min.Z), _num(bb.max.X), _num(bb.max.Y), _num(bb.max.Z)]


def scan(solid, double_d_bores, section_recesses, claimed: set, index: int) -> list:
    bbox = solid.bounding_box()
    extent = (float(bbox.size.X), float(bbox.size.Y), float(bbox.size.Z))
    tol = max(1e-5, max(extent) * 1e-5)
    wires = []
    for face_i, face in enumerate(solid.faces()):
        boundary = coverage._principal_boundary_plane(face, bbox)
        if boundary is None:
            continue
        axis, plane_axes, at = boundary
        for wire in face.inner_wires():
            recess = any(
                coverage._recess_matches_principal_wire(r, wire, axis, plane_axes, at, tol)
                for r in section_recesses
            )
            double_d = not recess and coverage._claim_double_d_bore_mouth(
                double_d_bores, claimed, wire, axis, plane_axes, at, bbox, tol
            )
            vocabulary = coverage._supported_inner_profile(wire, plane_axes, tol)
            wbb = wire.bounding_box()
            wires.append(
                {
                    "solid": index,
                    "face": face_i,
                    "axis": axis,
                    "at": _num(at),
                    "box": [
                        _num(getattr(wbb.min, plane_axes[0].upper())),
                        _num(getattr(wbb.min, plane_axes[1].upper())),
                        _num(getattr(wbb.max, plane_axes[0].upper())),
                        _num(getattr(wbb.max, plane_axes[1].upper())),
                    ],
                    "edges": [KINDS.get(e.geom_type, e.geom_type.name.lower()) for e in wire.edges()],
                    "recess": bool(recess),
                    "double_d": bool(double_d),
                    "vocabulary": bool(vocabulary),
                    "supported": bool(recess or double_d or vocabulary),
                }
            )
    return wires


def capture(root: Path, rel: str) -> dict:
    path = root / rel
    with tempfile.TemporaryDirectory() as tmp:
        if path.suffix == ".gz":
            plain = Path(tmp) / path.stem
            with gzip.open(path, "rb") as src, open(plain, "wb") as dst:
                shutil.copyfileobj(src, dst)
            path = plain
        part = import_step(str(path))
    recognition = build_raw_recognition_result(part)
    double_d = list(recognition.double_d_bores)
    # The aggregate's section recesses, refusals among them (no refusal matches a wire); only
    # the records are counted, as the port's Analysis keeps refusals apart.
    recesses = list(recognition.section_recesses)
    solids = list(part.solids())
    sources = solids if len(solids) > 1 else [part]
    claimed: set = set()
    wires = []
    for i, solid in enumerate(sources):
        wires += scan(solid, double_d, recesses, claimed, i)
    radial = [coverage._radial_outer_arc_count(s, s.bounding_box()) for s in sources]
    issues = coverage.lint_principal_profile_coverage(
        part, double_d_bores=double_d, section_recesses=recesses
    )
    unsupported = any(not w["supported"] for w in wires)
    if unsupported != any("internal profile" in i.message for i in issues):
        raise RuntimeError("the wire verdicts disagree with the finding")
    return {
        "file": rel,
        "solids": [_box(s.bounding_box()) for s in sources],
        "wires": wires,
        "radial": radial,
        "double_d_bores": len(double_d),
        "section_recesses": sum(type(r).__name__ == "SectionRecess" for r in recesses),
        "closed_section_recesses": sum(
            type(r).__name__ == "SectionRecess" and r.geometry.profile.closure == "closed"
            for r in recesses
        ),
        "issues": [{"code": i.code, "severity": i.severity, "message": i.message} for i in issues],
    }


def main() -> None:
    root = Path(sys.argv[2])
    pairs = [a.split("=", 1) for a in sys.argv[3:]]
    if any(len(p) != 2 for p in pairs):
        sys.exit(USAGE)
    out = json.loads(gzip.decompress(OUT.read_bytes())) if OUT.exists() else {"schema": SCHEMA, "parts": {}}
    failed = []
    for name, rel in pairs:
        try:
            out["parts"][name] = capture(root, rel)
        except Exception as e:  # reported, and the run fails at the end
            print(name, "FAILED:", type(e).__name__, e, file=sys.stderr)
            failed.append(name)
            continue
        record = out["parts"][name]
        print(
            name,
            len(record["wires"]),
            sum(not w["supported"] for w in record["wires"]),
            [i["message"][-60:] for i in record["issues"]],
            file=sys.stderr,
        )
    OUT.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(out, allow_nan=False, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    OUT.write_bytes(gzip.compress(text.encode(), mtime=0))
    if failed:
        sys.exit(f"not captured: {', '.join(failed)}")


if __name__ == "__main__":
    main()
