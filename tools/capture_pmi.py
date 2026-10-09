"""Record what draftwright reads from a part's AP242 semantic PMI, for the port's PMI reader.

    ~/repos/draftwright/.venv/bin/python tools/capture_pmi.py <draftwright-src> <corpus>

<draftwright-src> is draftwright's ``src`` directory (put first on the import path) and <corpus>
is quiddity's ``tests/corpus``. The inputs are every ``tests/fixtures/pmi/step/*.gz`` (copies of
quiddity-rust's AP242 fixtures, ``tests/fixtures/pmi/README.md``) and the corpus's NIST parts
(``nist/*.stp``, geometry only). For each, ``tests/fixtures/pmi/<stem>.json.gz`` gets
``draftwright.pmi.extract_pmi_report`` on the file in its own STEP space (no part frame, as the
port draws a part in the file's frame):

- ``sources``: its census, every source with its category, OCCT type code, outcome and reason;
- ``records``: every record with every field except those holding coordinates (``ref_pts``,
  ``ref_bbox``, ``reference_bboxes``, ``cylindrical_refs``, ``circular_refs``,
  ``angular_reference``, ``angular_references``, ``reference_normal``: lowering's geometry,
  which the port derives from the faces), and two fields this tool adds to join a record to the
  port's, which reads Part 21 instances rather than XCAF labels:
  - ``faces``: the ``ADVANCED_FACE`` instances (``#N``, ascending) of the shapes XCAF attaches
    to the record's source labels (``GetRefShapeLabel``, both sides; a datum's every
    occurrence), found as quiddity-rust's ``capture_pmi_occt.py`` finds them
    (``TransferReader::EntityFromShapeResult``, rank to id); ``edges`` likewise for edges;
  - ``key``: the Part 21 instance draftwright itself names for the record (``part21_id``), or
    for a geometric tolerance without one, the one its own correspondence would name
    (``match_geometric_tolerance`` on the XCAF semantic name, as ``pmi.py`` does for a missing
    magnitude), else "".

``sha256`` is the STEP text's (decompressed). Values are draftwright's, as returned; nothing is interpreted. The XCAF pass that finds the
faces reads the file as ``pmi.py`` does (GD&T and names on), so its label entries are the
census's; the tool checks each label's category and type against the record's and stops
otherwise.
"""

from __future__ import annotations

import dataclasses
import enum
import gzip
import hashlib
import json
import math
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, sys.argv[1])

from OCP.IFSelect import IFSelect_RetDone  # noqa: E402
from OCP.STEPCAFControl import STEPCAFControl_Reader  # noqa: E402
from OCP.TCollection import TCollection_AsciiString, TCollection_ExtendedString  # noqa: E402
from OCP.TDF import TDF_Label, TDF_LabelSequence, TDF_Tool  # noqa: E402
from OCP.TDocStd import TDocStd_Document  # noqa: E402
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE  # noqa: E402
from OCP.TopExp import TopExp_Explorer  # noqa: E402
from OCP.XCAFDoc import (  # noqa: E402
    XCAFDoc_Datum,
    XCAFDoc_Dimension,
    XCAFDoc_DocumentTool,
    XCAFDoc_GeomTolerance,
)

from draftwright import _pmi_part21 as p21  # noqa: E402
from draftwright._pmi_schema import _GTOL_TYPE  # noqa: E402
from draftwright.pmi import extract_pmi_report  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests" / "fixtures" / "pmi"
CORPUS = Path(sys.argv[2])

#: Record fields holding coordinates in the extraction space, left out (see the docstring).
GEOMETRY = {
    "ref_pts",
    "ref_bbox",
    "reference_bboxes",
    "cylindrical_refs",
    "circular_refs",
    "angular_reference",
    "angular_references",
    "reference_normal",
}


def plain(value):
    """JSON form of draftwright's values: dataclasses as objects, tuples as lists."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {f.name: plain(getattr(value, f.name)) for f in dataclasses.fields(value)}
    if isinstance(value, enum.Enum):
        return value.name
    if isinstance(value, (list, tuple)):
        return [plain(v) for v in value]
    if isinstance(value, dict):
        return {str(k): plain(v) for k, v in value.items()}
    if isinstance(value, float) and not math.isfinite(value):
        return repr(value)
    return value


def instances(data: bytes) -> list[tuple[int, str]]:
    """``(id, first keyword)`` of every instance of the DATA section, in file order, scanning
    the text outside strings and comments (quiddity-rust ``capture_pmi_occt.py``)."""
    text = data.decode("latin-1")
    out, n = [], len(text)
    start = text.find("DATA;")
    i = start + 5 if start >= 0 else n
    statement: list[str] = []
    while i < n:
        c = text[i]
        if c == "'":
            j = i + 1
            while j < n:
                if text[j] == "'":
                    if j + 1 < n and text[j + 1] == "'":
                        j += 2
                        continue
                    break
                j += 1
            statement.append("''")
            i = j + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        if c == ";":
            head = "".join(statement).strip()
            m = re.match(r"#(\d+)\s*=\s*\(?\s*([A-Za-z_][A-Za-z0-9_]*)", head)
            if m:
                out.append((int(m.group(1)), m.group(2).upper()))
            statement = []
            i += 1
            continue
        statement.append(c)
        i += 1
    return out


class Faces:
    """The XCAF document ``pmi.py`` reads, and the file instances behind its labels' shapes."""

    def __init__(self, path: Path) -> None:
        self.doc = TDocStd_Document(TCollection_ExtendedString("XCAF"))
        self.reader = STEPCAFControl_Reader()
        self.reader.SetGDTMode(True)
        self.reader.SetNameMode(True)
        if self.reader.ReadFile(str(path)) != IFSelect_RetDone:
            raise ValueError(f"cannot read {path}")
        if not self.reader.Transfer(self.doc):
            raise ValueError(f"cannot transfer {path}")
        session = self.reader.Reader().WS()
        model = session.Model()
        self.transfer = session.TransferReader()
        self.ranks = {model.Value(k): k for k in range(1, model.NbEntities() + 1)}
        self.instances = instances(path.read_bytes())
        if len(self.instances) != model.NbEntities():
            raise ValueError(
                f"{path}: {len(self.instances)} instances in the text, {model.NbEntities()} read"
            )
        main = self.doc.Main()
        self.shapes = XCAFDoc_DocumentTool.ShapeTool_s(main)
        self.tool = XCAFDoc_DocumentTool.DimTolTool_s(main)
        self.data = self.doc.GetData()

    def _instance(self, shape) -> int | None:
        entity = self.transfer.EntityFromShapeResult(shape, 1)
        rank = self.ranks.get(entity) if entity is not None else None
        if rank is None:
            return None
        number, keyword = self.instances[rank - 1]
        found = entity.DynamicType().Name().split("_", 1)[-1]
        if keyword.replace("_", "") != found.upper():
            raise ValueError(f"rank {rank} is #{number} {keyword}, OpenCascade read {found}")
        return number

    def label(self, entry: str):
        label = TDF_Label()
        TDF_Tool.Label_s(self.data, TCollection_AsciiString(entry), label)
        if label.IsNull():
            raise ValueError(f"no XCAF label {entry}")
        return label

    def shapes_of(self, label) -> tuple[set[int], set[int]]:
        """The ``ADVANCED_FACE`` and ``EDGE_CURVE`` instances of a label's reference shapes."""
        first, second = TDF_LabelSequence(), TDF_LabelSequence()
        self.tool.GetRefShapeLabel_s(label, first, second)
        faces: set[int] = set()
        edges: set[int] = set()
        for seq in (first, second):
            for i in range(1, seq.Length() + 1):
                shape = self.shapes.GetShape_s(seq.Value(i))
                if shape.IsNull():
                    continue
                for kind, into in ((TopAbs_FACE, faces), (TopAbs_EDGE, edges)):
                    explorer = TopExp_Explorer(shape, kind)
                    while explorer.More():
                        number = self._instance(explorer.Current())
                        if number is not None:
                            into.add(number)
                        explorer.Next()
        return faces, edges


def _check_label(faces: Faces, category: str, label, record: dict) -> None:
    """The label is the census's: the same category and, where it has one, the same type."""
    if category == "dimension":
        found = int(XCAFDoc_Dimension.Set_s(label).GetObject().GetType())
    elif category == "geometric_tolerance":
        found = int(XCAFDoc_GeomTolerance.Set_s(label).GetObject().GetType())
    else:
        XCAFDoc_Datum.Set_s(label).GetObject()
        return
    if found != record["type_code"]:
        raise ValueError(f"{record['source_id']}: type {found} here, {record['type_code']} there")


def _semantic_name(label) -> str:
    name = XCAFDoc_GeomTolerance.Set_s(label).GetObject().GetSemanticName()
    return str(name.ToCString()).strip() if name is not None else ""


def capture(step: Path) -> dict:
    report = extract_pmi_report(step)
    faces = Faces(step)
    try:
        tolerance_facts = p21.read_geometric_tolerances(step)
    except Exception:  # draftwright's own correspondence fails the same way; no key
        tolerance_facts = ()
    records = []
    for rec in report.records:
        out = plain(rec)
        for key in GEOMETRY:
            out.pop(key, None)
        face_ids: set[int] = set()
        edge_ids: set[int] = set()
        key = rec.part21_id
        for source in rec.source_ids or (rec.source_id,):
            category, _, entry = source.partition(":")
            if category not in ("dimension", "geometric_tolerance", "datum"):
                continue  # Part 21 projections and unrepresented definitions have no label
            label = faces.label(entry)
            _check_label(faces, category, label, out)
            f, e = faces.shapes_of(label)
            face_ids |= f
            edge_ids |= e
            if category == "geometric_tolerance" and not key:
                name = _semantic_name(label)
                if name:
                    fact, reason = p21.match_geometric_tolerance(
                        tolerance_facts, name, _GTOL_TYPE.get(rec.type_code, "")
                    )
                    if fact is not None and not reason:
                        key = fact.entity_id
        out["faces"] = sorted(face_ids)
        out["edges"] = sorted(edge_ids)
        out["key"] = key
        records.append(out)
    return {
        "error": report.error,
        "sources": plain(report.sources),
        "records": records,
    }


def inputs() -> list[tuple[str, Path]]:
    fixtures = [(p.name, p) for p in sorted((FIXTURES / "step").glob("*.gz"))]
    corpus = [(f"nist/{p.name}", p) for p in sorted((CORPUS / "nist").glob("*.stp"))]
    return fixtures + corpus


def main() -> None:
    for name, path in inputs():
        data = path.read_bytes()
        if path.suffix == ".gz":
            data = gzip.decompress(data)
        plain_name = path.name.removesuffix(".gz")
        with tempfile.TemporaryDirectory() as tmp:
            step = Path(tmp) / plain_name
            step.write_bytes(data)
            body = capture(step)
        body = {"file": name, "sha256": hashlib.sha256(data).hexdigest(), **body}
        stem = plain_name.rsplit(".", 1)[0]
        text = json.dumps(body, allow_nan=False, sort_keys=True, separators=(",", ":")) + "\n"
        (FIXTURES / f"{stem}.json.gz").write_bytes(gzip.compress(text.encode(), mtime=0))
        print(name, len(body["sources"]), "sources", len(body["records"]), "records")


if __name__ == "__main__":
    main()
