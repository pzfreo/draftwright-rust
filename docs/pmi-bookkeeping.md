# STEP bookkeeping

Draftwright recognises these part-owned bookkeeping records by their exact names and forms:

- Specify's `resume::attribute_set`: `pmi-assist answers`, with one text item of the same name.
- Specify 0.1.4's characteristic-number history (#265): `specify characteristic numbers`, with one text item `history` (the numbering's high-water mark; Draftwright does not read it).
- The editor and MCP project exporter: `draftwright editor project`, with one text item of the same name.
- Python Specify's earlier form: a `pmi-assist answers` property described `schema 1` on a product definition, with one representation holding one descriptive item of the same name.

They remain extracted `other` sources in the analysis, with their instance provenance, and produce no `pmi_not_extracted` problem. Names alone do not exempt an attribute set with other items or a feature owner. Foreign attributes and unrecognised properties remain reportable.

## Characteristic numbers (producer convention, #254)

Specify owns inspection characteristic numbers and stores them in the standard derived id
slot: `ID_ATTRIBUTE('<positive decimal integer>', #requirement)` (CAx-IF PMI RP v4.1
§5.1). Draftwright reads `PmiRead.identifiers` from haecceity 0.2.6, index for index with
semantic PMI provenance, and never allocates, changes or infers a number.

Each requirement has exactly one owner:

| Requirement | Id carrier |
| --- | --- |
| Dimension | Its `DIMENSIONAL_SIZE` or `DIMENSIONAL_LOCATION` |
| Geometric tolerance | Its `GEOMETRIC_TOLERANCE` root, including complex roots |
| Datum requirement | Its `DATUM`, reached from the datum feature |
| Surface texture, including part default | Its `PROPERTY_DEFINITION('surface texture', ...)` |
| Thread or knurl | Its own exclusive `PRODUCT_DEFINITION_SHAPE` |
| Semantic note | Its own requirement property, when explicitly identified |

Supporting faces, shared shape aspects, parameter properties and part ids do not supply
requirement numbers. Positive decimal ids retain their file spelling, including leading
zeros; other valid AP242 ids such as `thread-01` are quietly ignored for numbering.
Unnumbered derived dimensions and general CSV rows stay unnumbered. Specify's nominal-only
general-tolerance dimensions carry their numbers onto the planned items credited by #265.

The interim CAx-IF UDA remains a fallback: an attribute set named exactly `characteristic
number`, with one item of that name, containing positive decimal text or a positive integer,
on the exact size, tolerance or datum-feature owner. An id wins if both routes supply
numbers. Different integer values produce `characteristic_number_conflict` on the item;
equivalent leading-zero spellings do not conflict. Duplicate `ID_ATTRIBUTE`s produce
`characteristic_id_ambiguous` on the item and no number, even when a UDA is present or the
ids agree. This preserves haecceity's finding without preventing the requirement itself
from being drawn.

PMI record `fields` retain the numeric `id` with its file spelling, an `id ambiguity` diagnostic when present, and the
legacy `characteristic number`. These are metadata, not manufacturing parameters. JSON
containers and WASM/MCP signatures are unchanged.

Inspection rings attach to settled dimension labels, tolerance/datum frames, face-texture
symbols and thread/knurl callouts. A shared internal-thread requirement is applied only
after every source cylinder proves one distinct hole owner. Identical holes sharing
that source use a counted callout; independently authored thread requirements retain
independent callouts. A hole schedule retains the native drilling size/depth beside
the thread in each applicable row and
repeats the same requirement number beside those rows, including rows also stated by
retained size callouts. A part-default texture's ring sits beside its default
texture symbol near the title block. A numbered semantic note uses its own general-note
line; ordinary notes remain unnumbered. Missing carriers or clear ring positions are
reported; no identity is borrowed from an adjacent requirement.
