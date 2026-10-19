# STEP bookkeeping and graphic PMI reports

Draftwright recognises these part-owned bookkeeping records by their exact names and forms:

- Specify's `resume::attribute_set`: `pmi-assist answers`, with one text item of the same name.
- The editor and MCP project exporter: `draftwright editor project`, with one text item of the same name.
- Python Specify's earlier form: a `pmi-assist answers` property described `schema 1` on a product definition, with one representation holding one descriptive item of the same name.

They remain extracted `other` sources in the analysis, with their instance provenance, and produce no `pmi_not_extracted` problem. Names alone do not exempt an attribute set with other items or a feature owner. Foreign attributes and unrecognised properties remain reportable.

The part-note support discussed in #127 is absent from this branch. Its part-owned `semantic text` attributes remain accounted for once as unsupported attributes; they are not bookkeeping. A regression includes that form alongside foreign attributes and both current bookkeeping forms.

The geometry-styling correction for #125 belongs to haecceity's `Accounting::presentation`. Draftwright's presentation-problem wording is unchanged. Draftwright's committed haecceity/quiddity dependencies stay pinned to `v0.1.0-alpha.7`; updating that pin follows a quiddity release. Testing against the companion local quiddity checkout uses an uncommitted Cargo patch for both packages.
