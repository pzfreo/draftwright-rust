# STEP bookkeeping

Draftwright recognises these part-owned bookkeeping records by their exact names and forms:

- Specify's `resume::attribute_set`: `pmi-assist answers`, with one text item of the same name.
- Specify 0.1.4's characteristic-number history (#265): `specify characteristic numbers`, with one text item `history` (the numbering's high-water mark; Draftwright does not read it).
- The editor and MCP project exporter: `draftwright editor project`, with one text item of the same name.
- Python Specify's earlier form: a `pmi-assist answers` property described `schema 1` on a product definition, with one representation holding one descriptive item of the same name.

They remain extracted `other` sources in the analysis, with their instance provenance, and produce no `pmi_not_extracted` problem. Names alone do not exempt an attribute set with other items or a feature owner. Foreign attributes and unrecognised properties remain reportable.

## Characteristic numbers (producer convention, #254)

Specify owns inspection characteristic numbers and stores them in AP242; Draftwright must
never allocate or infer them. The interim convention is a CAx-IF User Defined Attribute
(UDA practice §5–7) named exactly `characteristic number`, with one item of the same name,
whose value is text such as `"17"` or an integer such as `17`. Edition 4's
`characterized_item` SELECT carries direct size/tolerance owners. Attach it to the PMI entity
that states the requirement, rather than its supporting faces or the part. The attribute's
`property_definition` is associated with a `general_property` by a
`general_property_association`; its representation holds the named value. Read it through
haecceity's extracted `PartPmi.attributes`, retaining the owner and provenance. Unnumbered
derived dimensions and general CSV rows remain unnumbered.

The pinned haecceity 0.2.3 (`ced75a5`) can write and read both value forms with
`NoteOwner::Dimension`, `NoteOwner::Tolerance` and `NoteOwner::Feature` (including datum
features). **Known limit: surface textures and semantic notes remain unnumbered.** Its
`NoteOwner` has no surface-texture or semantic-note owner. Surface textures and semantic
notes are themselves `PROPERTY_DEFINITION` records: directly using one as another
`property_definition.definition` fails AP242's `characterized_definition` SELECT, and the
reader does not reach the attached UDA. A feature-owned number cannot substitute: several
requirements can share the same feature.

Dimensions, geometric tolerance frames and datum features use this convention for inspection
balloons. Surface textures and semantic notes keep their existing drawing behavior until a
schema-valid attachment and reader/writer ownership support exists in quiddity/haecceity.
Their attributes retain the existing unsupported-PMI accounting; no number is inferred from
an adjacent requirement or a shared face.
