"""Record what draftwright's coverage backstop reads and finds, for lint's coverage checks.

    ~/repos/draftwright/.venv/bin/python tools/capture_lint_coverage.py <draftwright-src> <corpus> <name>=<path>[!<remove>] ...

<draftwright-src> is draftwright's ``src`` directory (put first on the import path) and <corpus>
is quiddity's ``tests/corpus``; each part is named and given by its path in the corpus
(``m10127=mfcadpp/10127.step``). For each, builds the drawing as draftwright does by default,
removes the annotations ``<remove>`` names, if any (comma-separated; an exact name, or a prefix
ending in ``*``: ``cgb245_bare=cadgenbench_inputs/cgb245.step!hc_plan0,m_cm*``, as draftwright's
own tests take a callout off a finished drawing), lints it (``Drawing.lint()``, physical), and
records the arguments and findings of that lint's
one ``lint_feature_coverage`` call and one ``lint_location_coverage`` call
(``linting/orchestration.py``):

- ``feature``: the holes it reads (each bore's, counterbore's and spotface's diameter, with the
  hole's location and axis, to join it to the port's Analysis), the bosses it reads and those it
  keeps after the blend-owned exclusion (``feature_diameters``' ``bosses``), the inventory, each
  annotation's structured and text diameters (``_annotation_diameter_sources``; title blocks
  skipped, as the function skips them) with its quantity (``covers_count``) and owner, the
  diameters placed structured notes and approved cells provide (with their feature's count and
  owner; owners are numbered in the order met), the excluded (capped) diameters, the severity
  and the findings; and the findings of the same call with the count check on
  (``check_hole_counts=True``, as a standalone caller makes it; the drawing's lint turns it off);
- ``location``: each hole and double-D bore it reads (location, axis, its position key, whether
  a pattern holds it, its projection into each view it may be marked in), the bounding box's
  centre, each view's centre marks and dimension witness points (a dimension with structured
  location facts gives none, as in the function), the structured location facts (their
  feature's axis and member keys, parameter and point key), the features placed notes locate,
  the severity and the findings.

Writes ``tests/fixtures/lint_coverage/python.json.gz``, deterministically (floats rounded to 1e-6,
parts in the order given, every list in draftwright's order); parts already in it and not named
are kept.
"""

from __future__ import annotations

import gzip
import json
import sys
import tempfile
from pathlib import Path

USAGE = "usage: capture_lint_coverage.py <draftwright-src> <corpus> <name>=<path>[!<remove>] ..."

if __name__ == "__main__":
    if sys.argv[1:2] in (["-h"], ["--help"]):
        print(__doc__.strip())
        sys.exit(0)
    if len(sys.argv) < 4:
        sys.exit(f"{USAGE}\n(--help for more)")

sys.path.insert(0, sys.argv[1])

from build123d_drafting.helpers import CenterMark, Dimension, TitleBlock  # noqa: E402

import draftwright.linting.coverage as coverage  # noqa: E402
import draftwright.linting.orchestration as orchestration  # noqa: E402
from draftwright import build_drawing  # noqa: E402
from draftwright._core import (  # noqa: E402
    _END_ON,
    _annotation_diameter_sources,
    _axis_letter,
    _decode_hole_location_fact,
    _xyz,
)
from draftwright.linting._registry import annotation_owner, cell_approvals_of, satisfaction_ids  # noqa: E402

SCHEMA = "draftwright.lint-coverage/1"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures" / "lint_coverage" / "python.json.gz"


def _num(v):
    r = round(float(v), 6)
    return 0.0 if r == 0 else r


def _pt(p):
    return [_num(c) for c in p]


_feature_calls: list = []
_location_calls: list = []
_diameter_calls: list = []

_original_feature = orchestration.lint_feature_coverage
_original_location = orchestration.lint_location_coverage
_original_diameters = coverage.feature_diameters


def _recording_diameters(part, cyls=None, holes=None, bosses=None):
    result = _original_diameters(part, cyls=cyls, holes=holes, bosses=bosses)
    _diameter_calls.append((list(holes or ()), list(bosses or ()), list(result)))
    return result


def _recording_feature(part, annotations, *args, **kwargs):
    _diameter_calls.clear()
    issues = _original_feature(part, annotations, *args, **kwargs)
    diameters = list(_diameter_calls)
    # The same call with the count check on, as a standalone caller makes it.
    counted = _original_feature(part, annotations, *args, **{**kwargs, "check_hole_counts": True})
    _feature_calls.append((part, list(annotations), kwargs, list(issues), diameters, list(counted)))
    return issues


def _recording_location(part, dwg, *args, **kwargs):
    issues = _original_location(part, dwg, *args, **kwargs)
    _location_calls.append((part, dwg, kwargs, list(issues)))
    return issues


orchestration.lint_feature_coverage = _recording_feature
orchestration.lint_location_coverage = _recording_location
coverage.feature_diameters = _recording_diameters


def _issues(issues) -> list:
    return [{"code": i.code, "severity": i.severity, "message": i.message} for i in issues]


def _hole(h) -> dict:
    return {
        "diameter": _num(h.diameter),
        "cbore": None if h.cbore is None else _num(h.cbore.diameter),
        "spotface": None if h.spotface is None else _num(h.spotface.diameter),
        "location": _pt(_xyz(h.location)),
        "axis": _axis_letter(h),
    }


def _boss(b) -> dict:
    return {"diameter": _num(b.diameter), "location": _pt(_xyz(b.location)), "axis": _axis_letter(b)}


def feature(call, names) -> dict:
    part, annotations, kwargs, issues, diameters, counted = call
    if len(diameters) != 1:
        raise RuntimeError(f"expected one feature_diameters call, saw {len(diameters)}")
    holes, kept, inventory = diameters[0]
    registry = kwargs.get("registry")
    owners: dict = {}

    def owner(feature):
        return None if feature is None else owners.setdefault(id(feature), len(owners))

    def provided(value, feature):
        return {"value": _num(value), "count": int(getattr(feature, "count", 1) or 1), "owner": owner(feature)}

    notes = []
    for identity in satisfaction_ids(registry):
        feature = getattr(identity, "feature", None)
        parameter_id = getattr(identity, "parameter", None)
        for item in getattr(feature, "parameters", lambda: ())():
            if item.parameter_id == parameter_id and item.kind == "diameter":
                notes.append(provided(item.value, feature))
    cells = []
    if registry is not None:
        for name in registry.names():
            for reference, cell in cell_approvals_of(registry, name):
                if cell.measurement.kind == "diameter" and cell.measurement.role != "bolt_circle":
                    cells.append(provided(cell.measurement.value, reference.measurement.feature))
    marks = []
    for ann in annotations:
        if isinstance(ann, TitleBlock):
            continue
        structured, text = _annotation_diameter_sources(ann)
        if not structured and not text:
            continue
        marks.append(
            {
                "name": names.get(id(ann)),
                "type": type(ann).__name__,
                "label": getattr(ann, "label", None) or "",
                "structured": [_num(v) for v in structured],
                "text": [_num(v) for v in text],
                "count": int(getattr(ann, "covers_count", 1) or 1),
                "owner": owner(annotation_owner(registry, ann)) if structured else None,
            }
        )
    assembly = kwargs.get("assembly")
    if assembly is None:
        assembly = len(part.solids()) > 1
    bosses = kwargs.get("bosses")
    return {
        "holes": [_hole(h) for h in holes],
        "bosses": [_boss(b) for b in (bosses or ())],
        "kept_bosses": [_boss(b) for b in kept],
        "inventory": [_num(d) for d in inventory],
        "annotations": marks,
        "notes": sorted(notes, key=lambda n: (n["value"], n["count"], n["owner"])),
        "cells": sorted(cells, key=lambda n: (n["value"], n["count"], n["owner"])),
        "exclude": [_num(d) for d in (kwargs.get("exclude") or ())],
        "assembly": bool(assembly),
        "check_hole_counts": kwargs.get("check_hole_counts", True),
        "issues": _issues(issues),
        "counted_issues": _issues(counted),
    }


def _ref(owner, point) -> list:
    r = coverage._location_ref(owner, point)
    return [_num(r.x), _num(r.y), _num(r.z)]


def location(call) -> dict:
    part, dwg, kwargs, issues = call
    source_holes = kwargs.get("holes") or []
    profiled = kwargs.get("profiled_bores") or []
    patterns = kwargs.get("patterns") or []
    patterned = {coverage._location_ref(h, h.location) for pat in patterns for h in pat.holes}
    registry = kwargs.get("registry")
    if registry is None:
        registry = getattr(dwg, "registry", None)
    dimension_spec_of = getattr(registry, "dimension_spec_of", None)
    named = getattr(registry, "named", None)
    marks: dict = {}
    witnesses: dict = {}
    facts = []
    for name, ann in dwg.iter_annotations():
        view = dwg.view_of(name)
        if view is not None and isinstance(ann, CenterMark):
            c = ann.center()
            marks.setdefault(view, []).append([_num(c.X), _num(c.Y)])
        evidence = named(name) if callable(named) else ann
        decoded = tuple(
            parsed
            for fact in getattr(evidence, "covers_hole_locations", ())
            if (parsed := _decode_hole_location_fact(fact)) is not None
        )
        if view is not None and isinstance(ann, Dimension) and not decoded:
            spec = dimension_spec_of(name) if callable(dimension_spec_of) else coverage._UNSET
            witnesses.setdefault(view, []).extend(
                [_num(x), _num(y)] for x, y in coverage._dim_vertices(ann, spec)
            )
        for owner, parameter, point in decoded:
            if getattr(owner, "kind", None) != "hole":
                continue
            frame = getattr(owner, "frame", None)
            members = getattr(owner, "members", ()) or ((frame.origin,) if frame else ())
            facts.append(
                {
                    "annotation": name,
                    "feature_axis": None if frame is None else frame.axis,
                    "members": [_ref(owner, p) for p in members],
                    "parameter": str(parameter),
                    "point": _ref(owner, point),
                }
            )
    noted = []
    for identity in satisfaction_ids(registry):
        if getattr(identity, "parameter", None) != "location":
            continue
        owner = identity.feature
        frame = getattr(owner, "frame", None)
        if frame is None:
            continue
        members = getattr(owner, "members", ()) or (frame.origin,)
        noted.append({"feature_axis": frame.axis, "members": [_ref(owner, p) for p in members]})
    available = getattr(dwg, "views", None)
    holes = []
    for kind, h in [*(("hole", h) for h in source_holes), *(("double_d_bore", h) for h in profiled)]:
        x, y, z = _xyz(h.location)
        axis = _axis_letter(h)
        views = ("front", "rear") if axis == "y" else (_END_ON.get(axis, "plan"),)
        if available is None:
            views = (_END_ON.get(axis, "plan"),)
        projections = {
            view: _pt(dwg.at(view, x, y, z))[:2]
            for view in views
            if available is None or view in available
        }
        ref = coverage._location_ref(h, h.location)
        holes.append(
            {
                "kind": kind,
                "location": _pt((x, y, z)),
                "axis": axis,
                "ref": [_num(ref.x), _num(ref.y), _num(ref.z)],
                "patterned": ref in patterned,
                "projections": projections,
            }
        )
    bb = part.bounding_box()
    assembly = kwargs.get("assembly")
    if assembly is None:
        assembly = len(part.solids()) > 1
    return {
        "holes": holes,
        "centre": _pt((bb.center().X, bb.center().Y, bb.center().Z)),
        "views": None if available is None else sorted(available),
        "marks": {v: marks[v] for v in sorted(marks)},
        "witnesses": {v: witnesses[v] for v in sorted(witnesses)},
        "facts": facts,
        "noted": noted,
        "assembly": bool(assembly),
        "issues": _issues(issues),
    }


def capture(corpus: Path, rel: str, remove: list) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        d = build_drawing(str(corpus / rel), out=str(Path(tmp) / "sheet"))
    removed = sorted(
        name
        for name in d.annotations()
        if any(name.startswith(r[:-1]) if r.endswith("*") else name == r for r in remove)
    )
    if remove and not removed:
        raise RuntimeError(f"nothing named {remove}")
    for name in removed:
        d.remove(name)
    _feature_calls.clear()
    _location_calls.clear()
    found = d.lint()
    if len(_feature_calls) != 1 or len(_location_calls) != 1:
        raise RuntimeError(
            f"expected one call of each, saw {len(_feature_calls)} and {len(_location_calls)}"
        )
    names = {id(obj): name for name, obj in d._registry.iter_named()}
    return {
        "file": rel,
        "removed": removed,
        "feature": feature(_feature_calls[0], names),
        "location": location(_location_calls[0]),
        "lint_codes": sorted(
            {i.code for i in found if i.code.startswith("feature_") and "leader" not in i.code}
        ),
    }


def main() -> None:
    corpus = Path(sys.argv[2])
    pairs = [a.split("=", 1) for a in sys.argv[3:]]
    if any(len(p) != 2 for p in pairs):
        sys.exit(USAGE)
    out = json.loads(gzip.decompress(OUT.read_bytes())) if OUT.exists() else {"schema": SCHEMA, "parts": {}}
    failed = []
    for name, spec in pairs:
        rel, _, remove = spec.partition("!")
        try:
            out["parts"][name] = capture(corpus, rel, [r for r in remove.split(",") if r])
        except Exception as e:  # reported, and the run fails at the end
            print(name, "FAILED:", type(e).__name__, e, file=sys.stderr)
            failed.append(name)
            continue
        record = out["parts"][name]
        print(
            name,
            [i["message"] for i in record["feature"]["issues"]],
            [i["message"] for i in record["location"]["issues"]],
            file=sys.stderr,
        )
    OUT.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(out, allow_nan=False, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    OUT.write_bytes(gzip.compress(text.encode(), mtime=0))
    if failed:
        sys.exit(f"not captured: {', '.join(failed)}")


if __name__ == "__main__":
    main()
