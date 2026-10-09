//! A part's AP242 semantic PMI read into draftwright's PMI records: stage 1 of drawing PMI
//! (draftwright `pmi.py`, `extract_pmi_report`). The Analysis carries the report ([`attach`],
//! [`Reading`]), each geometric tolerance and datum with where it attaches ([`Geometry`]);
//! [`crate::pmi_lowering`] lowers it to Plan items. The PMI coverage lint is a later stage.
//!
//! draftwright reads PMI through OpenCascade's XCAF document, with Part 21 overlays where XCAF
//! loses something (magnitudes, datum supports, dimension groups). The port reads it through
//! haecceity's semantic reader ([`haecceity::pmi::read`]), which walks the Part 21 instances
//! from each part and reports every instance it did not read as a finding. The report keeps
//! draftwright's shape:
//!
//! - [`Record`] is `PmiRecord`, for the three categories this stage projects: `dimension`,
//!   `geometric_tolerance` and `datum`. `kind`, `type_code` (XCAF's type number for the same
//!   kind), `value` (mm, or degrees for an angle), the deviations, the limits, `label` (as
//!   `_pmi_schema._make_label` and `pmi._geometric_tolerance_record` write it), `datum_refs`,
//!   `gtol_modifiers` (draftwright's names), `basic` and `part21_id` keep draftwright's
//!   meaning, with two differences of convention: a deviation is signed (`lower_tol` -0.2, not
//!   draftwright's unsigned 0.2, so a g6 shaft's two negative deviations survive), and an
//!   unstated nominal is `None`, not 0. One datum record per datum, as `_coalesce_datum_records`
//!   projects XCAF's occurrences onto their datum feature. A record's `blockers` are its
//!   `lowering_blockers`: what the record does not carry (a dimension's qualifier, a tolerance's
//!   projected zone, a datum's targets, …) and every haecceity finding on an instance the record
//!   was read from.
//! - The faces a record applies to replace draftwright's XCAF shapes: `faces`, the drawing's face
//!   indices (haecceity's, the file's own order, as [`crate::bridge`] uses), `face_ids` their
//!   `ADVANCED_FACE` instances, and `face_groups`, a location's two sides or a size's one
//!   (draftwright's `reference_item_groups`). `ref_bbox` (their box) and `dominant_axis` are in
//!   the frame the drawing uses, the file's own: the port draws a part where the file puts it,
//!   which is draftwright's frame-less extraction (`frame=None`). A geometric tolerance's and a
//!   datum's `geometry` (draftwright's `ref_pts`, `reference_axis`, `reference_surface_kind` and
//!   `reference_normal`) is derived from the faces for lowering, and so is a dimension's (its
//!   proven stations, or its cylinders: [`crate::pmi_lowering::dimension_geometry`]).
//! - [`Source`] is `PmiSourceEntity`: what was not read, and why, as a `not_extracted` source;
//!   every read record is a source too (`extracted`, or `partially_extracted` with its
//!   blockers). Not read: every haecceity finding not on a record's instances; every item
//!   haecceity reads that this stage does not project (general tolerances, threads, knurls,
//!   notes, attribute sets, material, standards, datum targets, tolerance relations: draftwright's
//!   `manufacturing_requirement` and `surface_label` categories, its common labels and material
//!   facts, are not ported yet); and every record whose face or edge does not resolve to the
//!   drawing's part, which is never guessed. Presentation is counted ([`Accounting`]), as
//!   draftwright's `presentation_only` dimensions are, not read.
//!
//! The census is per Part 21 instance, not per XCAF label, so its sources do not pair one for one
//! with draftwright's; records are compared one for one (`tests/pmi.rs`). Output order is the
//! category (dimensions, tolerances, datums, as draftwright's report), then the part, then the
//! reader's order.

use std::collections::{BTreeMap, BTreeSet};

use haecceity::Part;
use haecceity::geom::{Bounds, Surface};
use haecceity::p21::{Document, RawEntity};
use haecceity::pmi::{
    self, Anchor, AngleUnit, AuxiliaryClassification, DatumId, DimTolerance, Dimension,
    DimensionId, DimensionKind, Feature, FeatureId, GeometricTolerance, GroupKind, LengthUnit,
    LocationKind, PartPmi, PartProvenance, Quantity, SizeKind, ToleranceKind, ToleranceModifier,
    ToleranceTarget, Value,
};
use haecceity::step::{PartDefinition, read_part_definitions};
use serde::{Deserialize, Serialize};

pub const PMI_SCHEMA: &str = "draftwright.pmi/1";

/// draftwright's source categories (`PmiSourceCategory`), and `other` for what haecceity reads
/// or reports that belongs to none of them (shape aspects, properties, standards).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Dimension,
    GeometricTolerance,
    Datum,
    ManufacturingRequirement,
    SurfaceLabel,
    Other,
}

/// draftwright's extraction outcomes (`PmiExtractionOutcome`), less `presentation_only`:
/// presentation is counted, not inventoried ([`Accounting::presentation`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Extracted,
    PartiallyExtracted,
    NotExtracted,
}

/// One source and the outcome of reading it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// `<category>:#<instance>` for a record (the record's `source_id`), `finding:#<instance>`
    /// for a finding, `<item>:<part>:<index>` for an item this stage does not project.
    pub id: String,
    pub category: Category,
    /// The subject's entity type, lower case (a complex instance's leaves joined by `+`).
    pub entity: String,
    /// The instances it concerns (the subject first for a finding; ascending otherwise).
    pub instances: Vec<u64>,
    pub outcome: Outcome,
    pub reason: String,
}

/// One semantic PMI requirement of a part (draftwright's `PmiRecord`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub source_id: String,
    pub category: Category,
    /// draftwright's kind: `linear`, `diameter`, `radius`, `angular`, `thickness`, … for a
    /// dimension (`_pmi_schema._DIM_TYPE`), `position`, `flatness`, … for a tolerance
    /// (`_GTOL_TYPE`), `datum`.
    pub kind: String,
    /// XCAF's type number for the kind (`XCAFDimTolObjects_DimensionType` or
    /// `…_GeomToleranceType`), where it has one.
    pub type_code: Option<u32>,
    /// The part (haecceity's `PartId`: `read_part_definitions`' order).
    pub part: usize,
    /// The record's own instance: the dimension, the tolerance, or the datum's feature (its
    /// `datum` when it has none), as draftwright's `part21_id`.
    pub part21_id: u64,
    /// Nominal (dimension) or magnitude (tolerance), mm or degrees; `None` where the file
    /// states none.
    pub value: Option<f64>,
    /// Signed deviations from nominal, in the value's unit.
    pub upper_tol: Option<f64>,
    pub lower_tol: Option<f64>,
    /// Limits, in the value's unit.
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    /// An ISO 286 tolerance class (`H7`, `g6`).
    pub fit: Option<String>,
    pub label: String,
    /// The datum letters of a tolerance's datum system, compartment by compartment.
    pub datum_refs: Vec<String>,
    pub gtol_modifiers: Vec<String>,
    /// Theoretically exact (basic) dimension.
    pub basic: bool,
    /// The drawing's faces the record applies to, ascending.
    pub faces: Vec<usize>,
    /// Their `ADVANCED_FACE` instances, ascending.
    pub face_ids: Vec<u64>,
    /// A dimension's sides: a location's from and to, a size's feature (drawing faces).
    pub face_groups: Vec<Vec<usize>>,
    /// A dimension's sides' edges, group by group as `face_groups` (draftwright's reference
    /// groups take edges as well as faces).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_groups: Vec<Vec<usize>>,
    /// The drawing's edges the record applies to, ascending.
    pub edges: Vec<usize>,
    /// The box of `faces` in the drawing's frame, `[xmin, ymin, zmin, xmax, ymax, zmax]`.
    pub ref_bbox: Option<[f64; 6]>,
    /// `X`, `Y`, `Z`, or `?`: the box's longest extent (draftwright `_dominant_from_bbox`).
    pub dominant_axis: String,
    /// Every instance the record was read from, ascending.
    pub instances: Vec<u64>,
    pub blockers: Vec<String>,
    /// Where a geometric tolerance or datum attaches, derived from its faces for lowering
    /// (draftwright's `ref_pts`, `reference_axis`, `reference_surface_kind` and
    /// `reference_normal`); absent on a dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
}

/// How the file's semantic-PMI instances were accounted for (haecceity's accounting).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accounting {
    pub semantic: usize,
    pub consumed: usize,
    pub reported: usize,
    pub presentation: usize,
    pub validation: usize,
}

/// A part's PMI as draftwright reads it (`PmiExtractionReport`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    /// The parts of the file, in haecceity's order (`Record::part` indexes it).
    pub parts: Vec<String>,
    pub sources: Vec<Source>,
    pub records: Vec<Record>,
    pub accounting: Accounting,
}

/// Where a geometric tolerance or datum attaches to the part, in the drawing's frame (the
/// file's), as draftwright measures it from the record's faces (`_datum_geometry_from_shapes`,
/// `_datum_reference_normal` and `_common_principal_cylinder_axis`), and where a dimension does.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    /// A datum's attachment site on each face, face by face (`ref_pts`): a point inside a plane,
    /// or on a cylinder's projected silhouette (`_datum_face_site`); a radius's proven point on
    /// its arc. Empty for a tolerance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<[f64; 3]>,
    /// Other proven sites on a datum's first face, ranked as draftwright ranks its choice and
    /// spread over the face (on a cylinder, on the first site's side): where the port's placement
    /// may attach the symbol when the first site's has no clear place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<[f64; 3]>,
    /// `X`, `Y` or `Z` (`reference_axis`): a datum's common plane normal or cylinder axis; a
    /// tolerance's common principal cylinder axis, when every face is a cylinder on one; a
    /// dimension's measured direction or cylinder axis (below).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<String>,
    /// A datum's surface: `plane` or `cylinder` (`reference_surface_kind`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    /// A planar datum's outward unit normal (`reference_normal`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal: Option<[f64; 3]>,
    /// A linear or thickness dimension's two proven reference stations (draftwright's
    /// `ref_pts` for them), from and to; its `axis` is then the direction they are measured
    /// along, when it is x, y or z ([`crate::pmi_lowering::dimension_geometry`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<[f64; 3]>,
    /// A diameter's or radius's exact cylinders (draftwright's `cylindrical_refs`), identical
    /// ones once; its `axis` is then their common axis, when it is x, y or z.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cylinders: Vec<crate::pmi_lowering::Cylinder>,
    /// The one view a linear dimension's faces prove a witness in (`plan` or `side`), when only
    /// one does (draftwright's `support_view`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Why the attachment is not proven: the record is then not lowered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

/// A part's PMI as the Analysis carries it: the report, or why the file's PMI could not be
/// read at all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reading {
    Read(Report),
    Refused(String),
}

/// The entity types of semantic PMI (ISO 10303-242's dimensions, tolerances and datums), by
/// which a file that cannot be read is known to carry PMI that is then lost.
const SEMANTIC: [&str; 6] = [
    "DIMENSIONAL_SIZE",
    "DIMENSIONAL_LOCATION",
    "GEOMETRIC_TOLERANCE",
    "DATUM",
    "DATUM_FEATURE",
    "DATUM_SYSTEM",
];

/// What the Analysis carries of the STEP file *bytes*' PMI, read on the drawing's *part*: the
/// report when the file has any PMI to account for (a record, a source, or a counted instance),
/// none when it has none. A file that cannot be read is refused with the reason when its text
/// names a semantic PMI entity, so PMI the reader loses is said, never dropped.
pub fn attach(bytes: &[u8], part: &Part) -> Option<Reading> {
    match read(bytes, part) {
        Ok(r) => {
            let a = &r.accounting;
            let any = !r.records.is_empty()
                || !r.sources.is_empty()
                || a.semantic + a.presentation + a.reported > 0;
            any.then_some(Reading::Read(r))
        }
        Err(e) => {
            let text = String::from_utf8_lossy(bytes);
            let named = SEMANTIC.iter().any(|n| {
                text.match_indices(n).any(|(i, _)| {
                    let after = text[i + n.len()..].trim_start();
                    let before = text[..i].trim_end();
                    after.starts_with('(') && (before.ends_with('=') || before.ends_with('('))
                })
            });
            named.then_some(Reading::Refused(e))
        }
    }
}

/// Reads the semantic PMI of the STEP file *bytes*, whose part the drawing read as *part*
/// (`haecceity::read_step` of the same bytes). Refused when the file's parts cannot be read or
/// do not match *part*'s faces; everything within a part that cannot be read is a source with a
/// reason.
pub fn read(bytes: &[u8], part: &Part) -> Result<Report, String> {
    let defs = read_part_definitions(bytes).map_err(|e| format!("cannot read the parts: {e}"))?;
    let doc = Document::parse(bytes.to_vec()).map_err(|e| format!("cannot read Part 21: {e}"))?;
    let read = pmi::read(&doc, &defs).map_err(|e| e.to_string())?;
    report(&doc, &defs, part, &read)
}

/// The report of a haecceity reading *read* of *doc* (whose parts are *defs*), on the drawing's
/// *part*.
pub fn report(
    doc: &Document,
    defs: &[PartDefinition],
    part: &Part,
    read: &pmi::PmiRead,
) -> Result<Report, String> {
    let drawing = Drawing::new(part, defs)?;
    let mut records = Vec::new();
    let mut sources = Vec::new();
    for (p, (pmi, prov)) in read.parts.iter().zip(&read.provenance.parts).enumerate() {
        let r = PartReader {
            doc,
            p,
            pmi,
            prov,
            def: &defs[p],
            drawing: &drawing,
        };
        r.read(&mut records, &mut sources);
    }
    let mut attached = vec![false; read.findings.len()];
    for record in &mut records {
        for (i, f) in read.findings.iter().enumerate() {
            let on = f.part == Some(pmi::PartId(record.part))
                && f.ids
                    .first()
                    .is_some_and(|id| record.instances.binary_search(id).is_ok());
            if on {
                attached[i] = true;
                record.blockers.push(format!(
                    "haecceity {} on #{} ({}): {}",
                    f.kind.as_str(),
                    f.ids[0],
                    f.entity,
                    f.detail
                ));
            }
        }
    }
    for (f, _) in read.findings.iter().zip(&attached).filter(|(_, a)| !**a) {
        let subject = f.ids.first().copied().unwrap_or(0);
        sources.push(Source {
            id: format!("finding:#{subject}"),
            category: category_of_entity(&f.entity),
            entity: f.entity.clone(),
            instances: f.ids.clone(),
            outcome: Outcome::NotExtracted,
            reason: format!("haecceity {}: {}", f.kind.as_str(), f.detail),
        });
    }
    for r in &mut records {
        r.geometry = match r.category {
            Category::GeometricTolerance => Some(tolerance_geometry(part, r)),
            Category::Datum => Some(datum_geometry(part, r)),
            Category::Dimension => Some(crate::pmi_lowering::dimension_geometry(part, r)),
            _ => None,
        };
    }
    records.sort_by_key(|r| (r.category, r.part));
    let mut record_sources: Vec<Source> = records
        .iter()
        .map(|r| Source {
            id: r.source_id.clone(),
            category: r.category,
            entity: entity_name(doc, r.part21_id),
            instances: r.instances.clone(),
            outcome: if r.blockers.is_empty() {
                Outcome::Extracted
            } else {
                Outcome::PartiallyExtracted
            },
            reason: r.blockers.join("; "),
        })
        .collect();
    record_sources.append(&mut sources);
    let a = &read.accounting;
    Ok(Report {
        schema: PMI_SCHEMA.into(),
        parts: defs.iter().map(|d| d.name.clone()).collect(),
        sources: record_sources,
        records,
        accounting: Accounting {
            semantic: a.semantic,
            consumed: a.consumed,
            reported: a.reported,
            presentation: a.presentation,
            validation: a.validation,
        },
    })
}

/// The drawing's part, indexed by the file instances its faces and edges were read from.
struct Drawing<'a> {
    part: &'a Part,
    /// `ADVANCED_FACE` instance → (placed instance, drawing face).
    faces: BTreeMap<u64, Vec<(usize, usize)>>,
    edges: BTreeMap<u64, Vec<(usize, usize)>>,
}

impl<'a> Drawing<'a> {
    fn new(part: &'a Part, defs: &[PartDefinition]) -> Result<Self, String> {
        let mut faces: BTreeMap<u64, Vec<(usize, usize)>> = BTreeMap::new();
        for f in 0..part.faces.len() {
            let s = part.face_source(f).ok_or_else(|| {
                format!("face {f} of the drawing's part was not read from a file")
            })?;
            faces.entry(s.entity).or_default().push((s.instance, f));
        }
        let mut edges: BTreeMap<u64, Vec<(usize, usize)>> = BTreeMap::new();
        for e in 0..part.edges.len() {
            if let Some(s) = part.edge_source(e) {
                edges.entry(s.entity).or_default().push((s.instance, e));
            }
        }
        let counted: usize = defs
            .iter()
            .map(|d| d.faces.len() * d.placements.len())
            .sum();
        if counted != part.faces.len() {
            return Err(format!(
                "the file's parts have {counted} placed faces and the drawing's part {}: they \
                 were not read from the same file",
                part.faces.len()
            ));
        }
        Ok(Drawing { part, faces, edges })
    }

    /// The drawing's faces (or edges) read from *entity* in one of *def*'s placements.
    fn resolve(
        map: &BTreeMap<u64, Vec<(usize, usize)>>,
        def: &PartDefinition,
        entity: u64,
    ) -> Vec<usize> {
        let placed: BTreeSet<usize> = def
            .placements
            .iter()
            .flat_map(|p| p.instances.iter().copied())
            .collect();
        map.get(&entity)
            .into_iter()
            .flatten()
            .filter(|(i, _)| placed.contains(i))
            .map(|&(_, f)| f)
            .collect()
    }
}

/// What a set of features is made of, on the drawing.
#[derive(Default)]
struct Support {
    faces: BTreeSet<usize>,
    face_ids: BTreeSet<u64>,
    edges: BTreeSet<usize>,
    instances: BTreeSet<u64>,
    /// Supplemental geometry anchors (no drawing face), by entity.
    geometry: BTreeSet<String>,
    all_around: bool,
    /// Why a face or edge could not be resolved: the record is not read.
    unresolved: Vec<String>,
}

struct PartReader<'a> {
    doc: &'a Document,
    p: usize,
    pmi: &'a PartPmi,
    prov: &'a PartProvenance,
    def: &'a PartDefinition,
    drawing: &'a Drawing<'a>,
}

impl PartReader<'_> {
    fn read(&self, records: &mut Vec<Record>, sources: &mut Vec<Source>) {
        for d in 0..self.pmi.dimensions.len() {
            self.keep(self.dimension(DimensionId(d)), records, sources);
        }
        for t in 0..self.pmi.tolerances.len() {
            self.keep(self.tolerance(t), records, sources);
        }
        for d in 0..self.pmi.datums.len() {
            self.keep(self.datum(DatumId(d)), records, sources);
        }
        self.not_projected(sources);
    }

    /// Keeps a read record, or reports the one whose support does not resolve.
    fn keep(&self, record: Record, records: &mut Vec<Record>, sources: &mut Vec<Source>) {
        if let Some(why) = record.blockers.iter().find(|b| b.starts_with(UNRESOLVED)) {
            sources.push(Source {
                id: record.source_id.clone(),
                category: record.category,
                entity: entity_name(self.doc, record.part21_id),
                instances: record.instances.clone(),
                outcome: Outcome::NotExtracted,
                reason: why.clone(),
            });
        } else {
            records.push(record);
        }
    }

    /// Items haecceity reads that this stage does not project, each a source not read.
    fn not_projected(&self, sources: &mut Vec<Source>) {
        let pmi = self.pmi;
        let prov = self.prov;
        let groups: [(&str, Category, usize, &[Vec<u64>]); 8] = [
            (
                "standard",
                Category::Other,
                pmi.standards.len(),
                &prov.standards,
            ),
            (
                "general_tolerance",
                Category::ManufacturingRequirement,
                pmi.general.len(),
                &prov.general,
            ),
            (
                "thread",
                Category::ManufacturingRequirement,
                pmi.threads.len(),
                &prov.threads,
            ),
            (
                "knurl",
                Category::ManufacturingRequirement,
                pmi.knurls.len(),
                &prov.knurls,
            ),
            ("note", Category::Other, pmi.notes.len(), &prov.notes),
            (
                "attribute_set",
                Category::Other,
                pmi.attributes.len(),
                &prov.attributes,
            ),
            (
                "datum_target",
                Category::Datum,
                pmi.datum_targets.len(),
                &prov.datum_targets,
            ),
            (
                "tolerance_relation",
                Category::GeometricTolerance,
                pmi.tolerance_relations.len(),
                &prov.tolerance_relations,
            ),
        ];
        for (item, category, n, ids) in groups {
            for i in 0..n {
                let instances = ids.get(i).cloned().unwrap_or_default();
                sources.push(Source {
                    id: format!("{item}:{}:{i}", self.p),
                    category,
                    entity: instances
                        .first()
                        .map_or_else(String::new, |&id| entity_name(self.doc, id)),
                    instances,
                    outcome: Outcome::NotExtracted,
                    reason: format!("{NOT_PROJECTED} ({})", item.replace('_', " ")),
                });
            }
        }
        if pmi.material.is_some() {
            sources.push(Source {
                id: format!("material:{}:0", self.p),
                category: Category::Other,
                entity: prov
                    .material
                    .first()
                    .map_or_else(String::new, |&id| entity_name(self.doc, id)),
                instances: prov.material.clone(),
                outcome: Outcome::NotExtracted,
                reason: format!("{NOT_PROJECTED} (material)"),
            });
        }
    }

    /// The faces, edges and instances of *features*, resolved to the drawing.
    fn support(&self, features: &[FeatureId]) -> Support {
        let mut s = Support::default();
        let mut seen = BTreeSet::new();
        let mut todo: Vec<FeatureId> = features.to_vec();
        while let Some(f) = todo.pop() {
            if !seen.insert(f) {
                continue;
            }
            if let Some(ids) = self.prov.features.get(f.0) {
                s.instances.extend(ids);
            }
            match self.pmi.features.get(f.0) {
                Some(Feature::Items(anchors)) => {
                    for a in anchors {
                        self.anchor(*a, &mut s);
                    }
                }
                Some(Feature::Group { members, kind }) => {
                    s.all_around |= *kind == GroupKind::AllAround;
                    todo.extend(members);
                }
                Some(Feature::Derived { from, .. }) => todo.extend(from),
                None => s
                    .unresolved
                    .push(format!("{UNRESOLVED}: feature {} is not the part's", f.0)),
            }
        }
        s
    }

    fn anchor(&self, a: Anchor, s: &mut Support) {
        let p = self.p;
        match a {
            Anchor::Face(i) => match self.def.faces.get(i.0) {
                Some(&entity) => {
                    let found = Drawing::resolve(&self.drawing.faces, self.def, entity);
                    if found.is_empty() {
                        s.unresolved.push(format!(
                            "{UNRESOLVED}: face {} of part {p} (#{entity}) is none of the \
                             drawing's faces",
                            i.0
                        ));
                    }
                    s.faces.extend(found);
                    s.face_ids.insert(entity);
                }
                None => s.unresolved.push(format!(
                    "{UNRESOLVED}: part {p} has no face {} ({} faces)",
                    i.0,
                    self.def.faces.len()
                )),
            },
            Anchor::Edge(i) => match self.def.edges.get(i.0) {
                Some(&entity) => {
                    let found = Drawing::resolve(&self.drawing.edges, self.def, entity);
                    if found.is_empty() {
                        s.unresolved.push(format!(
                            "{UNRESOLVED}: edge {} of part {p} (#{entity}) is none of the \
                             drawing's edges",
                            i.0
                        ));
                    }
                    s.edges.extend(found);
                }
                None => s.unresolved.push(format!(
                    "{UNRESOLVED}: part {p} has no edge {} ({} edges)",
                    i.0,
                    self.def.edges.len()
                )),
            },
            Anchor::Geometry(g) => {
                let entity = self
                    .pmi
                    .geometry
                    .get(g.0)
                    .map_or_else(|| format!("geometry {}", g.0), |x| x.item.entity());
                if let Some(ids) = self.prov.geometry.get(g.0) {
                    s.instances.extend(ids);
                }
                s.geometry.insert(entity);
            }
        }
    }

    /// A record's common fields from its support.
    fn record(
        &self,
        category: Category,
        part21_id: u64,
        own: &[u64],
        support: Support,
        mut blockers: Vec<String>,
    ) -> Record {
        let Support {
            faces,
            face_ids,
            edges,
            mut instances,
            geometry,
            unresolved,
            ..
        } = support;
        instances.extend(own);
        blockers.splice(0..0, unresolved);
        if !geometry.is_empty() {
            blockers.push(format!(
                "applies to supplemental geometry ({}), not only the part's faces and edges",
                geometry.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        let faces: Vec<usize> = faces.into_iter().collect();
        let ref_bbox = bbox(self.drawing.part, &faces);
        Record {
            source_id: format!("{}:#{part21_id}", category_name(category)),
            category,
            kind: String::new(),
            type_code: None,
            part: self.p,
            part21_id,
            value: None,
            upper_tol: None,
            lower_tol: None,
            lower_bound: None,
            upper_bound: None,
            fit: None,
            label: String::new(),
            datum_refs: Vec::new(),
            gtol_modifiers: Vec::new(),
            basic: false,
            dominant_axis: ref_bbox.map_or("?", dominant).into(),
            ref_bbox,
            faces,
            face_ids: face_ids.into_iter().collect(),
            face_groups: Vec::new(),
            edge_groups: Vec::new(),
            edges: edges.into_iter().collect(),
            instances: instances.into_iter().collect(),
            blockers,
            geometry: None,
        }
    }

    /// The instance among *ids* whose entity has one of *leaves* (upper case), the first of
    /// them when several do.
    fn subject(&self, ids: &[u64], leaves: &dyn Fn(&str) -> bool) -> Option<u64> {
        ids.iter().copied().find(|&id| {
            self.doc.get(id).is_some_and(|e| match e {
                RawEntity::Simple { name, .. } => leaves(name),
                RawEntity::Complex { parts, .. } => parts.iter().any(|p| leaves(&p.name)),
            })
        })
    }

    fn dimension(&self, id: DimensionId) -> Record {
        let d: &Dimension = &self.pmi.dimensions[id.0];
        let own = self
            .prov
            .dimensions
            .get(id.0)
            .map_or(&[][..], Vec::as_slice);
        let part21_id = self
            .subject(own, &|n| {
                matches!(
                    n,
                    "DIMENSIONAL_SIZE"
                        | "ANGULAR_SIZE"
                        | "DIMENSIONAL_SIZE_WITH_PATH"
                        | "DIMENSIONAL_LOCATION"
                        | "ANGULAR_LOCATION"
                        | "DIMENSIONAL_LOCATION_WITH_PATH"
                        | "DIRECTED_DIMENSIONAL_LOCATION"
                )
            })
            .or_else(|| own.first().copied())
            .unwrap_or(0);
        let mut blockers = Vec::new();
        let (groups, kind, code): (Vec<FeatureId>, &str, Option<u32>) = match &d.kind {
            DimensionKind::Size {
                feature,
                kind,
                path,
                angle,
            } => {
                if path.is_some() {
                    blockers.push("the dimension's path is not carried".into());
                }
                let (k, c) = if angle.is_some() {
                    ("angular", Some(28))
                } else {
                    size_kind(kind)
                };
                (vec![*feature], k, c)
            }
            DimensionKind::Location {
                from,
                to,
                kind,
                path,
                directed,
                angle,
            } => {
                if path.is_some() {
                    blockers.push("the dimension's path is not carried".into());
                }
                if *directed {
                    blockers.push("the dimension's direction (from, to) is not carried".into());
                }
                let (k, c) = if angle.is_some() {
                    ("angular", Some(11))
                } else {
                    location_kind(kind)
                };
                (vec![*from, *to], k, c)
            }
        };
        if let DimensionKind::Size {
            kind: SizeKind::Other(n),
            ..
        }
        | DimensionKind::Location {
            kind: LocationKind::Other(n),
            angle: None,
            ..
        } = &d.kind
        {
            blockers.push(format!("dimension kind {n:?} is not one draftwright knows"));
        }
        if let DimensionKind::Size { angle: Some(a), .. }
        | DimensionKind::Location { angle: Some(a), .. } = &d.kind
            && *a != pmi::AngleSelection::Equal
        {
            blockers.push(format!("the angle selection {a:?} is not carried"));
        }
        if let Some(q) = &d.qualifier {
            blockers.push(format!("the value qualifier {q:?} is not carried"));
        }
        for m in &d.modifiers {
            blockers.push(format!("the dimension modifier {m:?} is not carried"));
        }
        if let Some(p) = &d.principle {
            blockers.push(format!("the tolerance principle {p:?} is not carried"));
        }
        let face_groups: Vec<Vec<usize>> = groups
            .iter()
            .map(|g| self.support(&[*g]).faces.into_iter().collect())
            .collect();
        let edge_groups: Vec<Vec<usize>> = groups
            .iter()
            .map(|g| self.support(&[*g]).edges.into_iter().collect())
            .collect();
        let support = self.support(&groups);
        let mut r = self.record(Category::Dimension, part21_id, own, support, blockers);
        r.kind = kind.into();
        r.type_code = code;
        r.face_groups = face_groups;
        if edge_groups.iter().any(|g| !g.is_empty()) {
            r.edge_groups = edge_groups;
        }
        r.value = d.nominal.as_ref().map(measure);
        if d.nominal.is_none() {
            r.blockers
                .push("the file states no nominal value (haecceity reads it without one)".into());
        }
        match &d.tolerance {
            DimTolerance::None => {}
            DimTolerance::Basic => r.basic = true,
            DimTolerance::Deviations(b) => {
                r.upper_tol = Some(measure(b.upper()));
                r.lower_tol = Some(measure(b.lower()));
            }
            DimTolerance::Limits(b) => {
                r.lower_bound = Some(measure(b.lower()));
                r.upper_bound = Some(measure(b.upper()));
            }
            DimTolerance::Fit { class, limits } => {
                r.fit = Some(class.to_string());
                match limits {
                    Some(b) => {
                        r.lower_bound = Some(measure(b.lower()));
                        r.upper_bound = Some(measure(b.upper()));
                    }
                    None => r.blockers.push(format!(
                        "the ISO 286 class {class} is stated without limits; they are not computed"
                    )),
                }
            }
        }
        r.label = dimension_label(&r, d);
        r
    }

    fn tolerance(&self, t: usize) -> Record {
        let g: &GeometricTolerance = &self.pmi.tolerances[t];
        let own = self.prov.tolerances.get(t).map_or(&[][..], Vec::as_slice);
        let part21_id = self
            .subject(own, &|n| {
                n == "GEOMETRIC_TOLERANCE"
                    || ToleranceKind::of_entity(&n.to_ascii_lowercase()).is_some()
            })
            .or_else(|| own.first().copied())
            .unwrap_or(0);
        let mut blockers = Vec::new();
        let (features, dimension): (Vec<FeatureId>, Option<DimensionId>) = match &g.target {
            ToleranceTarget::Feature(f) => (vec![*f], None),
            ToleranceTarget::Dimension(d) => {
                (dimension_features(&self.pmi.dimensions[d.0]), Some(*d))
            }
            ToleranceTarget::Relation {
                relating, related, ..
            } => (vec![*relating, *related], None),
            ToleranceTarget::WholePart => (Vec::new(), None),
        };
        if let ToleranceTarget::Relation { name, .. } = &g.target {
            blockers.push(format!("applies to a shape aspect relationship {name:?}"));
        }
        let support = self.support(&features);
        let all_around = support.all_around;
        let mut modifiers: Vec<String> = Vec::new();
        if all_around {
            modifiers.push("all_around".into());
        }
        // On the part's shape, it "applies to all surfaces" (§6.3): all over, which XCAF and
        // draftwright name as a modifier.
        if g.target == ToleranceTarget::WholePart
            || g.auxiliary.contains(&AuxiliaryClassification::AllOver)
        {
            modifiers.push("all_over".into());
        }
        let material = |m: &ToleranceModifier| {
            matches!(
                m,
                ToleranceModifier::MaximumMaterialRequirement
                    | ToleranceModifier::LeastMaterialRequirement
            )
        };
        for m in g.modifiers.iter().filter(|m| !material(m)) {
            modifiers.push(m.as_enum().to_ascii_lowercase());
        }
        if let Some(z) = &g.zone {
            if z.form.is_diametral() {
                modifiers.push("diameter_zone".into());
            } else if z.form.is_spherical() {
                modifiers.push("spherical_diameter_zone".into());
            }
            if z.projected.is_some() {
                blockers.push("the projected tolerance zone is not carried".into());
            }
            if z.non_uniform.is_some() {
                blockers.push("the non-uniform tolerance zone is not carried".into());
            }
            if z.runout_angle.is_some() {
                blockers.push("the runout zone angle is not carried".into());
            }
            if z.affected_plane.is_some() {
                blockers.push("the affected plane is not carried".into());
            }
        }
        for m in g.modifiers.iter().filter(|m| material(m)) {
            modifiers.push(m.as_enum().to_ascii_lowercase());
        }
        let mut unique = BTreeSet::new();
        modifiers.retain(|m| unique.insert(m.clone()));
        if g.unit_basis.is_some() {
            blockers.push("the unit-basis (per unit area or length) is not carried".into());
        }
        if g.maximum.is_some() {
            blockers.push("the maximum tolerance value is not carried".into());
        }
        if g.unequal.is_some() {
            blockers.push("the unequally disposed zone value is not carried".into());
        }
        if g.auxiliary
            .contains(&AuxiliaryClassification::UnlessOtherwiseSpecified)
        {
            blockers.push("the 'unless otherwise specified' classification is not carried".into());
        }
        let mut datum_refs = Vec::new();
        if let Some(system) = &g.datums {
            for c in system.compartments() {
                if c.references.len() > 1 {
                    blockers.push(format!(
                        "a compartment of {} datums (common datum) is carried as separate letters",
                        c.references.len()
                    ));
                }
                if !c.modifiers.is_empty() {
                    blockers.push("datum compartment modifiers are not carried".into());
                }
                for reference in &c.references {
                    if !reference.modifiers.is_empty() {
                        blockers.push("datum reference modifiers are not carried".into());
                    }
                    match self.pmi.datums.get(reference.datum.0) {
                        Some(d) => datum_refs.push(d.label().to_string()),
                        None => blockers.push(format!(
                            "{UNRESOLVED}: datum {} is not the part's",
                            reference.datum.0
                        )),
                    }
                }
            }
        }
        if self
            .pmi
            .tolerance_relations
            .iter()
            .any(|rel| rel.relating.0 == t || rel.related.0 == t)
        {
            blockers.push(
                "it is related to another tolerance (composite, …), which is not carried".into(),
            );
        }
        let mut r = self.record(
            Category::GeometricTolerance,
            part21_id,
            own,
            support,
            blockers,
        );
        if let Some(d) = dimension {
            r.instances
                .extend(self.prov.dimensions.get(d.0).into_iter().flatten());
            r.instances.sort_unstable();
            r.instances.dedup();
        }
        let (kind, code) = tolerance_kind(g.kind);
        r.kind = kind.into();
        r.type_code = Some(code);
        r.value = g.magnitude.as_ref().map(measure);
        match r.value {
            Some(v) => r.label = format!("{kind} {}", fmt_g3(v)),
            None => {
                r.label = kind.into();
                r.blockers.push("the tolerance states no magnitude".into());
            }
        }
        r.datum_refs = datum_refs;
        r.gtol_modifiers = modifiers;
        r.blockers.dedup();
        r
    }

    fn datum(&self, id: DatumId) -> Record {
        let d = &self.pmi.datums[id.0];
        let own = self.prov.datums.get(id.0).map_or(&[][..], Vec::as_slice);
        let mut features: Vec<FeatureId> = d.feature().into_iter().collect();
        let mut blockers = Vec::new();
        for t in d.targets() {
            match self.pmi.datum_targets.get(t.0) {
                Some(target) => features.extend(target.on()),
                None => blockers.push(format!(
                    "{UNRESOLVED}: datum target {} is not the part's",
                    t.0
                )),
            }
        }
        if !d.targets().is_empty() {
            blockers.push(format!(
                "established by {} datum target(s), which are not carried",
                d.targets().len()
            ));
        }
        let support = self.support(&features);
        let feature_ids: Vec<u64> = d
            .feature()
            .and_then(|f| self.prov.features.get(f.0))
            .cloned()
            .unwrap_or_default();
        let part21_id = self
            .subject(&feature_ids, &|n| n == "DATUM_FEATURE")
            .or_else(|| self.subject(own, &|n| n == "DATUM_FEATURE"))
            .or_else(|| self.subject(own, &|n| n == "DATUM"))
            .or_else(|| own.first().copied())
            .unwrap_or(0);
        let mut r = self.record(Category::Datum, part21_id, own, support, blockers);
        r.kind = "datum".into();
        r.label = d.label().to_string();
        r
    }
}

/// The reason prefix of a record whose face, edge or reference does not resolve: not read.
const UNRESOLVED: &str = "unresolved";

/// The reason of an item read but not projected by this stage.
const NOT_PROJECTED: &str = "read by haecceity; not projected to a draftwright record yet (PMI \
                             reading stage 1 projects dimensions, geometric tolerances and datums)";

fn category_name(c: Category) -> &'static str {
    match c {
        Category::Dimension => "dimension",
        Category::GeometricTolerance => "geometric_tolerance",
        Category::Datum => "datum",
        Category::ManufacturingRequirement => "manufacturing_requirement",
        Category::SurfaceLabel => "surface_label",
        Category::Other => "other",
    }
}

/// The category of a finding's entity (lower case, `+`-joined leaves).
fn category_of_entity(entity: &str) -> Category {
    let leaves: Vec<&str> = entity.split('+').collect();
    let has = |f: &dyn Fn(&str) -> bool| leaves.iter().any(|l| f(l));
    if has(&|l| {
        l.starts_with("dimensional_")
            || l.starts_with("angular_")
            || l == "directed_dimensional_location"
    }) {
        Category::Dimension
    } else if has(&|l| l.ends_with("_tolerance") || l.starts_with("geometric_tolerance")) {
        Category::GeometricTolerance
    } else if has(&|l| l.starts_with("datum")) {
        Category::Datum
    } else {
        Category::Other
    }
}

/// An instance's entity type, lower case, a complex instance's leaves joined by `+`.
fn entity_name(doc: &Document, id: u64) -> String {
    match doc.get(id) {
        Some(RawEntity::Simple { name, .. }) => name.to_ascii_lowercase(),
        Some(RawEntity::Complex { parts, .. }) => parts
            .iter()
            .map(|p| p.name.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("+"),
        None => String::new(),
    }
}

/// The features a dimension measures.
fn dimension_features(d: &Dimension) -> Vec<FeatureId> {
    match &d.kind {
        DimensionKind::Size { feature, .. } => vec![*feature],
        DimensionKind::Location { from, to, .. } => vec![*from, *to],
    }
}

/// A value in mm, or degrees for an angle.
fn measure(v: &Value) -> f64 {
    match &v.quantity {
        Quantity::Length(l) => l.mm(),
        Quantity::Angle(a) => match a.unit {
            AngleUnit::Degree => a.value.to_f64(),
            _ => a.rad().to_degrees(),
        },
    }
}

/// draftwright's kind and XCAF's type number for a size (`_DIM_TYPE`, 14–27).
fn size_kind(k: &SizeKind) -> (&'static str, Option<u32>) {
    match k {
        SizeKind::CurveLength => ("curve_length", Some(14)),
        SizeKind::Diameter => ("diameter", Some(15)),
        SizeKind::SphericalDiameter => ("diameter", Some(16)),
        SizeKind::Radius => ("radius", Some(17)),
        SizeKind::SphericalRadius => ("radius", Some(18)),
        // XCAF's toroidal types, which draftwright names by number (`type19` …).
        SizeKind::ToroidalMinorDiameter => ("type19", Some(19)),
        SizeKind::ToroidalMajorDiameter => ("type20", Some(20)),
        SizeKind::ToroidalMinorRadius => ("type21", Some(21)),
        SizeKind::ToroidalMajorRadius => ("type22", Some(22)),
        SizeKind::ToroidalHighMajorDiameter => ("type23", Some(23)),
        SizeKind::ToroidalLowMajorDiameter => ("type24", Some(24)),
        SizeKind::ToroidalHighMajorRadius => ("type25", Some(25)),
        SizeKind::ToroidalLowMajorRadius => ("type26", Some(26)),
        SizeKind::Thickness => ("thickness", Some(27)),
        SizeKind::Other(_) => ("size", None),
    }
}

/// draftwright's kind and XCAF's type number for a location (`_DIM_TYPE`, 1–10). The three
/// one-sided linear distances have no XCAF type.
fn location_kind(k: &LocationKind) -> (&'static str, Option<u32>) {
    match k {
        LocationKind::CurvedDistance => ("curved_dist", Some(1)),
        LocationKind::LinearDistance => ("linear", Some(2)),
        LocationKind::LinearDistanceCentreOuter => ("linear", Some(3)),
        LocationKind::LinearDistanceCentreInner => ("linear", Some(4)),
        LocationKind::LinearDistanceOuterCentre => ("linear", Some(5)),
        LocationKind::LinearDistanceOuterOuter => ("linear", Some(6)),
        LocationKind::LinearDistanceOuterInner => ("linear", Some(7)),
        LocationKind::LinearDistanceInnerCentre => ("linear", Some(8)),
        LocationKind::LinearDistanceInnerOuter => ("linear", Some(9)),
        LocationKind::LinearDistanceInnerInner => ("linear", Some(10)),
        LocationKind::LinearDistanceCentre
        | LocationKind::LinearDistanceInner
        | LocationKind::LinearDistanceOuter => ("linear", None),
        LocationKind::Other(_) => ("location", None),
    }
}

/// draftwright's kind and XCAF's type number for a tolerance (`_GTOL_TYPE`).
fn tolerance_kind(k: ToleranceKind) -> (&'static str, u32) {
    match k {
        ToleranceKind::Angularity => ("angularity", 1),
        ToleranceKind::CircularRunout => ("circular_runout", 2),
        ToleranceKind::Roundness => ("circularity", 3),
        ToleranceKind::Coaxiality => ("coaxiality", 4),
        ToleranceKind::Concentricity => ("concentricity", 5),
        ToleranceKind::Cylindricity => ("cylindricity", 6),
        ToleranceKind::Flatness => ("flatness", 7),
        ToleranceKind::Parallelism => ("parallelism", 8),
        ToleranceKind::Perpendicularity => ("perpendicularity", 9),
        ToleranceKind::Position => ("position", 10),
        ToleranceKind::LineProfile => ("profile_line", 11),
        ToleranceKind::SurfaceProfile => ("profile_surface", 12),
        ToleranceKind::Straightness => ("straightness", 13),
        ToleranceKind::Symmetry => ("symmetry", 14),
        ToleranceKind::TotalRunout => ("total_runout", 15),
    }
}

/// Why a record's faces give no single attachment: they lie in several placements of their
/// part (an assembly placing it more than once), so which one to annotate is not known.
pub(crate) fn placements_reason(r: &Record) -> Option<String> {
    (r.faces.len() > r.face_ids.len()).then(|| {
        format!(
            "its {} faces are drawn in {} placements of the part; which placement to annotate is \
             an open question",
            r.face_ids.len(),
            r.faces.len().div_ceil(r.face_ids.len().max(1))
        )
    })
}

/// The axis (0, 1, 2) a unit direction lies along, when it is principal to within
/// *tolerance*: draftwright's tests, a component ≥ 1e-6 with the others summing to at most
/// *tolerance* times it.
fn principal(d: [f64; 3], tolerance: f64) -> Option<usize> {
    let c = d.map(f64::abs);
    let k = (0..3).fold(0, |k, i| if c[i] > c[k] { i } else { k });
    let others = c[0] + c[1] + c[2] - c[k];
    (c[k] >= 1e-6 && others <= tolerance * c[k]).then_some(k)
}

/// A geometric tolerance's attachment: its faces' common principal cylinder axis, when every
/// face is a cylinder and they share one (`_common_principal_cylinder_axis`, which reads no axis
/// when any reference is not a cylindrical face). The leader's site is its faces' box centre,
/// which the record carries (`ref_bbox`).
fn tolerance_geometry(part: &Part, r: &Record) -> Geometry {
    let reasons: Vec<String> = placements_reason(r).into_iter().collect();
    let mut axes = BTreeSet::new();
    let mut cylinders = r.edges.is_empty() && !r.faces.is_empty();
    for &f in &r.faces {
        match &part.faces[f].surface {
            Surface::Cylinder { frame, .. } => {
                let d = frame.z;
                let k = (0..3).fold(0, |k, i| if d[i].abs() > d[k].abs() { i } else { k });
                let exact = (d[k].abs() - 1.0).abs() <= 1e-6
                    && (0..3).all(|i| i == k || d[i].abs() <= 1e-6);
                if exact {
                    axes.insert(k);
                } else {
                    cylinders = false;
                }
            }
            _ => cylinders = false,
        }
    }
    let axis = (cylinders && axes.len() == 1)
        .then(|| AXIS_NAMES[*axes.first().expect("one axis")].to_string());
    Geometry {
        sites: Vec::new(),
        alternatives: Vec::new(),
        axis,
        surface: None,
        normal: None,
        reasons,
        ..Geometry::default()
    }
}

const AXIS_NAMES: [&str; 3] = ["X", "Y", "Z"];

/// A datum's attachment (`_datum_geometry_from_shapes` and `_datum_reference_normal`): every
/// face a plane or cylinder whose axis is principal, all of one kind, coplanar or coaxial; a
/// site on each; and a plane's outward normal, the faces agreeing.
fn datum_geometry(part: &Part, r: &Record) -> Geometry {
    let mut reasons: Vec<String> = placements_reason(r).into_iter().collect();
    let mut sites = Vec::new();
    let mut axes = Vec::new();
    let mut kinds = Vec::new();
    let mut supports: Vec<Vec<f64>> = Vec::new();
    let mut alternatives = Vec::new();
    if r.faces.is_empty() {
        reasons.push("referenced geometry is unavailable".into());
    }
    for &f in &r.faces {
        let b = part.face_bounds(f);
        sites.push(b.centre());
        let (kind, frame) = match &part.faces[f].surface {
            Surface::Plane { frame } => ("plane", frame),
            Surface::Cylinder { frame, .. } => ("cylinder", frame),
            _ => {
                reasons.push("one datum reference is neither planar nor cylindrical".into());
                continue;
            }
        };
        let Some(k) = principal(frame.z, 1e-6) else {
            reasons.push("one datum reference surface is not axis-aligned".into());
            continue;
        };
        let o = frame.origin;
        supports.push(if kind == "plane" {
            vec![o[k]]
        } else {
            (0..3).filter(|&i| i != k).map(|i| o[i]).collect()
        });
        axes.push(k);
        kinds.push(kind);
        let found = face_sites(part, f, &b, kind, k);
        match found.first() {
            Some(site) => {
                *sites.last_mut().expect("pushed") = *site;
                if alternatives.is_empty() && sites.len() == 1 {
                    alternatives = spread(&found, &b, 8);
                }
            }
            None => reasons
                .push("one datum reference face has no proven interior attachment point".into()),
        }
    }
    let mut axis = None;
    if axes.is_empty() {
        reasons.push("datum reference surface is unavailable".into());
    } else if kinds.iter().any(|k| *k != kinds[0]) {
        reasons.push("datum reference faces mix planar and cylindrical surfaces".into());
    } else if axes.iter().any(|a| *a != axes[0])
        || supports[1..].iter().any(|s| {
            s.iter()
                .zip(&supports[0])
                .any(|(a, b)| (a - b).abs() > 1e-4)
        })
    {
        let qualifier = if kinds[0] == "plane" {
            "coplanar"
        } else {
            "coaxial"
        };
        reasons.push(format!("datum reference faces are not {qualifier}"));
    } else {
        axis = Some(AXIS_NAMES[axes[0]].to_string());
    }
    let surface =
        (!kinds.is_empty() && kinds.iter().all(|k| *k == kinds[0])).then(|| kinds[0].to_string());
    let mut normal = None;
    if reasons.is_empty() && surface.as_deref() == Some("plane") {
        let normals: Vec<[f64; 3]> = r
            .faces
            .iter()
            .filter_map(|&f| part.face_normal(f, 0.0, 0.0))
            .collect();
        if normals.len() != r.faces.len() {
            reasons.push("datum reference surface has no usable normal".into());
        } else if normals[1..]
            .iter()
            .any(|n| n[0] * normals[0][0] + n[1] * normals[0][1] + n[2] * normals[0][2] < 0.999)
        {
            reasons.push("datum reference faces disagree about outward normal".into());
        } else {
            normal = Some(normals[0]);
        }
    }
    reasons.dedup();
    Geometry {
        sites,
        alternatives,
        axis,
        surface,
        normal,
        reasons,
        ..Geometry::default()
    }
}

/// The proven sites on face *f* (box *b*, of *kind*, its normal or axis along *k*), the one
/// `_datum_face_site` chooses first: on a cylinder, points of the silhouette the edge-on view
/// projects (`_cylindrical_datum_site`'s stations, then others along the axis on the same side);
/// on a plane, the points of a 13 × 13 grid over its parameter range that lie on the face, nearest
/// the box's centre across the view (then in space) first. Where no grid point lies on the face
/// (a narrow ring), draftwright takes mesh triangles' centroids; the port takes finer grids (52,
/// then 208, per side). Empty when none is proven.
fn face_sites(part: &Part, f: usize, b: &Bounds, kind: &str, k: usize) -> Vec<[f64; 3]> {
    let surface = &part.faces[f].surface;
    let Some(domain) = part.domain(f) else {
        return Vec::new();
    };
    let centre = b.centre();
    if kind == "cylinder" {
        let Surface::Cylinder { frame, radius } = surface else {
            return Vec::new();
        };
        let radial = if k < 2 { 2 } else { 0 };
        let signs = if k < 2 { [-1.0, 1.0] } else { [1.0, -1.0] };
        let mut stations = vec![centre[k]];
        if k == 2 {
            stations.insert(0, centre[k] + ((b.max[2] - b.min[2]) / 4.0).min(10.0));
        }
        let on = |station: f64, sign: f64| {
            let mut s = frame.origin;
            s[k] = station;
            s[radial] += sign * radius;
            if !b.contains(s, 1e-6) {
                return None;
            }
            let (u, v) = surface.parameters(s, None)?;
            let p = surface.value(u, v);
            let d = ((p[0] - s[0]).powi(2) + (p[1] - s[1]).powi(2) + (p[2] - s[2]).powi(2)).sqrt();
            // On the face, its boundary included (draftwright measures the distance to the face,
            // 1e-6): a silhouette on the seam between two halves of a split cylinder is on both.
            let e = 1e-9;
            let inside = [(0.0, 0.0), (e, 0.0), (-e, 0.0), (0.0, e), (0.0, -e)]
                .iter()
                .any(|(du, dv)| domain.contains(u + du, v + dv));
            (d <= 1e-6 && inside).then_some(s)
        };
        let Some((first, sign)) = stations
            .iter()
            .flat_map(|&st| signs.map(|sg| (st, sg)))
            .find_map(|(st, sg)| on(st, sg).map(|s| (s, sg)))
        else {
            return Vec::new();
        };
        let mut out = vec![first];
        for t in [0.3, 0.7, 0.15, 0.85, 0.05, 0.95] {
            if let Some(s) = on(b.min[k] + t * (b.max[k] - b.min[k]), sign) {
                out.push(s);
            }
        }
        return out;
    }
    let Some((u0, u1, v0, v1)) = part.uv_bounds(f) else {
        return Vec::new();
    };
    if ![u0, u1, v0, v1].iter().all(|x| x.is_finite()) {
        return Vec::new();
    }
    let hidden = if k == 1 { 0 } else { 1 };
    for n in [13usize, 52, 208] {
        let mut found: Vec<([f64; 2], [f64; 3])> = Vec::new();
        for ui in 0..n {
            let u = u0 + (ui as f64 + 0.5) * (u1 - u0) / n as f64;
            for vi in 0..n {
                let v = v0 + (vi as f64 + 0.5) * (v1 - v0) / n as f64;
                if domain.contains(u, v) {
                    let p = surface.value(u, v);
                    let key = [
                        (p[hidden] - centre[hidden]).abs(),
                        (0..3).map(|i| (p[i] - centre[i]).powi(2)).sum(),
                    ];
                    found.push((key, p));
                }
            }
        }
        if !found.is_empty() {
            found.sort_by(|(ka, pa), (kb, pb)| {
                ka[0]
                    .total_cmp(&kb[0])
                    .then(ka[1].total_cmp(&kb[1]))
                    .then(pa[0].total_cmp(&pb[0]))
                    .then(pa[1].total_cmp(&pb[1]))
                    .then(pa[2].total_cmp(&pb[2]))
            });
            return found.into_iter().map(|(_, p)| p).collect();
        }
    }
    Vec::new()
}

/// Up to *n* of *sites* (ranked) after the first, each at least a tenth of the face's box
/// diagonal from those kept before it: other places a symbol may attach.
fn spread(sites: &[[f64; 3]], b: &Bounds, n: usize) -> Vec<[f64; 3]> {
    let gap = 0.1 * b.diagonal();
    let mut kept: Vec<[f64; 3]> = sites.iter().take(1).copied().collect();
    for s in sites.iter().skip(1) {
        if kept.len() > n {
            break;
        }
        let far = kept
            .iter()
            .all(|k| ((0..3).map(|i| (k[i] - s[i]).powi(2)).sum::<f64>()).sqrt() >= gap);
        if far {
            kept.push(*s);
        }
    }
    kept.split_off(1.min(kept.len()))
}

/// The box of *faces* on *part*.
fn bbox(part: &Part, faces: &[usize]) -> Option<[f64; 6]> {
    let mut out: Option<[f64; 6]> = None;
    for &f in faces {
        let b = part.face_bounds(f);
        if !(b.min.iter().chain(&b.max).all(|x| x.is_finite())) {
            continue;
        }
        let o = out.get_or_insert([b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]]);
        for i in 0..3 {
            o[i] = o[i].min(b.min[i]);
            o[i + 3] = o[i + 3].max(b.max[i]);
        }
    }
    out
}

/// draftwright's `_dominant_from_bbox`: the axis of the largest extent, `?` when none is.
fn dominant(b: [f64; 6]) -> &'static str {
    let spans = [("X", b[3] - b[0]), ("Y", b[4] - b[1]), ("Z", b[5] - b[2])];
    // Python's max keeps the first of equal spans.
    let mut best = spans[0];
    for s in &spans[1..] {
        if s.1.abs() > best.1.abs() {
            best = *s;
        }
    }
    if best.1.abs() > 1e-6 { best.0 } else { "?" }
}

/// The label draftwright writes for a dimension (`pmi._dimension_label`): in the file's own
/// length unit with its stated decimal places and the unit's name, for a diameter, thickness or
/// linear distance stated in a conversion-based unit (draftwright's `DimensionDisplayFact`);
/// otherwise from the values in mm (degrees), with draftwright's number formatting. A deviation
/// keeps its sign.
fn dimension_label(r: &Record, d: &Dimension) -> String {
    let Some(value) = r.value else {
        return String::new();
    };
    let prefix = match r.kind.as_str() {
        "diameter" => "ø",
        "radius" => "R",
        _ => "",
    };
    let displayed = matches!(
        &d.kind,
        DimensionKind::Size {
            kind: SizeKind::Diameter | SizeKind::Thickness,
            angle: None,
            ..
        } | DimensionKind::Location {
            kind: LocationKind::LinearDistance,
            angle: None,
            ..
        }
    );
    let authored = d.nominal.as_ref().and_then(|n| match &n.quantity {
        Quantity::Length(l) => unit_name(&l.unit).map(|name| (n, l, name)),
        Quantity::Angle(_) => None,
    });
    if displayed && let Some((nominal, length, name)) = authored {
        let per = length.unit.millimetres();
        let in_unit = |v: &Value| measure(v) / per;
        let value_decimals = nominal.decimal_places.map(usize::from);
        let tolerances = |a: &Value, b: &Value| {
            let set: BTreeSet<u8> = [a.decimal_places, b.decimal_places]
                .into_iter()
                .flatten()
                .collect();
            if set.len() == 1 {
                set.first().map(|&d| usize::from(d))
            } else {
                None
            }
        };
        let base = match &d.tolerance {
            DimTolerance::Limits(b)
            | DimTolerance::Fit {
                limits: Some(b), ..
            } => {
                let places = tolerances(b.lower(), b.upper()).or(value_decimals);
                format!(
                    "{prefix}{} - {prefix}{}",
                    fmt_magnitude(in_unit(b.lower()), places),
                    fmt_magnitude(in_unit(b.upper()), places)
                )
            }
            DimTolerance::Deviations(b) => format!(
                "{prefix}{}{}",
                fmt(length.value.to_f64(), value_decimals),
                deviations(
                    in_unit(b.upper()),
                    in_unit(b.lower()),
                    tolerances(b.upper(), b.lower())
                )
            ),
            _ => format!("{prefix}{}", fmt(length.value.to_f64(), value_decimals)),
        };
        return format!("{base} {name}");
    }
    match (r.lower_bound, r.upper_bound, r.upper_tol, r.lower_tol) {
        (Some(lo), Some(hi), _, _) => format!(
            "{prefix}{} - {prefix}{}",
            fmt_magnitude(lo, None),
            fmt_magnitude(hi, None)
        ),
        (_, _, Some(up), Some(lo)) => {
            format!("{prefix}{}{}", fmt(value, None), deviations(up, lo, None))
        }
        _ => format!("{prefix}{}", fmt(value, None)),
    }
}

/// The name draftwright appends for a conversion-based length unit (its `unit_name`, the
/// `conversion_based_unit`'s name, which the NIST files spell in capitals); `None` for an SI
/// unit, which draftwright labels in mm without a name.
fn unit_name(u: &LengthUnit) -> Option<String> {
    match u {
        LengthUnit::Inch => Some("INCH".into()),
        LengthUnit::Foot => Some("FOOT".into()),
        LengthUnit::Other { name, .. } => Some(name.clone()),
        LengthUnit::Millimetre
        | LengthUnit::Micrometre
        | LengthUnit::Centimetre
        | LengthUnit::Metre => None,
    }
}

/// The deviation suffix (`_make_label`): ` ±t`, ` +u/0`, ` 0/-l`, ` +u/-l`, each sign as stated.
fn deviations(upper: f64, lower: f64, places: Option<usize>) -> String {
    let signed = |v: f64| {
        let sign = if v < 0.0 { "-" } else { "+" };
        format!("{sign}{}", fmt_magnitude(v.abs(), places))
    };
    if upper > 1e-9 && upper == -lower {
        format!(" ±{}", fmt_magnitude(upper, places))
    } else if lower.abs() <= 1e-9 {
        format!(" {}/0", signed(upper))
    } else if upper.abs() <= 1e-9 {
        format!(" 0/{}", signed(lower))
    } else {
        format!(" {}/{}", signed(upper), signed(lower))
    }
}

/// draftwright's `_fmt`: an integer when whole, else one decimal place; or exactly *decimals*
/// places (no negative zero).
pub fn fmt(v: f64, decimals: Option<usize>) -> String {
    match decimals {
        Some(d) => {
            let s = format!("{v:.d$}");
            if s.trim_start_matches('-')
                .chars()
                .all(|c| c == '0' || c == '.')
            {
                format!("{:.d$}", 0.0)
            } else {
                s
            }
        }
        None => {
            let r = v.round_ties_even();
            if (v - r).abs() < 1e-6 {
                format!("{}", r as i64)
            } else {
                format!("{v:.1}")
            }
        }
    }
}

/// draftwright's `_fmt_pmi_magnitude`: at most 13 significant digits without trailing zeros, or
/// exactly *decimals* places of the value's shortest decimal form, rounded half to even.
pub fn fmt_magnitude(v: f64, decimals: Option<usize>) -> String {
    match decimals {
        Some(d) => round_decimal(&format!("{v}"), d),
        None if v == 0.0 => format!("{v:.1}"),
        None => {
            let sci = format!("{v:.12e}");
            plain(&sci)
        }
    }
}

/// `%.3g`, as draftwright labels a tolerance (`f"{value:.3g}"`).
pub fn fmt_g3(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let sci = format!("{v:.2e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent");
    if !(-4..3).contains(&exp) {
        let m = mantissa.trim_end_matches('0').trim_end_matches('.');
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{m}e{sign}{:02}", exp.abs());
    }
    let places = usize::try_from(2 - exp).unwrap_or(0);
    let s = format!("{v:.places$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        s
    }
}

/// A `d.ddde±x` string as a plain decimal without trailing zeros.
fn plain(sci: &str) -> String {
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let exp: i64 = exp.parse().expect("exponent");
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    // The point sits after the first digit, moved by the exponent.
    let point = 1 + exp;
    let mut s = if point <= 0 {
        format!(
            "0.{}{digits}",
            "0".repeat(usize::try_from(-point).unwrap_or(0))
        )
    } else if usize::try_from(point).unwrap_or(0) >= digits.len() {
        format!(
            "{digits}{}",
            "0".repeat(usize::try_from(point).unwrap_or(0) - digits.len())
        )
    } else {
        let p = usize::try_from(point).unwrap_or(0);
        format!("{}.{}", &digits[..p], &digits[p..])
    };
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    let s = s.trim_start_matches('0');
    let s = if s.is_empty() || s.starts_with('.') {
        format!("0{s}")
    } else {
        s.to_string()
    };
    if negative { format!("-{s}") } else { s }
}

/// A decimal string (as Rust prints an `f64`, no exponent) rounded half to even to *d* places.
fn round_decimal(text: &str, d: usize) -> String {
    let negative = text.starts_with('-');
    let body = text.trim_start_matches('-');
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
    let keep = int.len() + d;
    if digits.len() < keep {
        digits.resize(keep, 0);
    }
    let rest = digits.split_off(keep);
    let up = match rest.first() {
        Some(&r) if r > 5 => true,
        Some(&5) => rest[1..].iter().any(|&x| x > 0) || digits.last().is_some_and(|x| x % 2 == 1),
        _ => false,
    };
    if up {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, 1);
                break;
            }
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let n_int = digits.len() - d;
    let int: String = digits[..n_int]
        .iter()
        .map(|x| char::from(b'0' + x))
        .collect();
    let frac: String = digits[n_int..]
        .iter()
        .map(|x| char::from(b'0' + x))
        .collect();
    let sign = if negative && digits.iter().any(|&x| x > 0) {
        "-"
    } else {
        ""
    };
    if d == 0 {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format_as_draftwright_formats_them() {
        assert_eq!(fmt(35.0, None), "35");
        assert_eq!(fmt(3.175, None), "3.2");
        assert_eq!(fmt(0.25, Some(3)), "0.250");
        assert_eq!(fmt(-0.0001, Some(2)), "0.00");
        assert_eq!(fmt_magnitude(0.508, None), "0.508");
        assert_eq!(fmt_magnitude(0.30000000000000004, None), "0.3");
        assert_eq!(fmt_magnitude(100.0, None), "100");
        assert_eq!(fmt_magnitude(0.0254, None), "0.0254");
        assert_eq!(fmt_magnitude(0.02, Some(2)), "0.02");
        assert_eq!(fmt_magnitude(0.125, Some(2)), "0.12");
        assert_eq!(fmt_magnitude(0.135, Some(2)), "0.14");
        assert_eq!(fmt_magnitude(9.995, Some(2)), "10.00");
        assert_eq!(fmt_g3(0.381), "0.381");
        assert_eq!(fmt_g3(1.016), "1.02");
        assert_eq!(fmt_g3(0.38099999999999995), "0.381");
        assert_eq!(fmt_g3(0.75), "0.75");
        assert_eq!(fmt_g3(100.0), "100");
        assert_eq!(fmt_g3(1234.0), "1.23e+03");
        assert_eq!(fmt_g3(0.00001), "1e-05");
    }

    #[test]
    fn deviations_keep_their_signs() {
        assert_eq!(deviations(0.5, -0.5, None), " ±0.5");
        assert_eq!(deviations(0.0, -0.2, None), " 0/-0.2");
        assert_eq!(deviations(0.2, 0.0, None), " +0.2/0");
        assert_eq!(deviations(0.05, -0.1, None), " +0.05/-0.1");
        // A g6 shaft: both below nominal, which draftwright's unsigned label cannot say.
        assert_eq!(deviations(-0.009, -0.025, None), " -0.009/-0.025");
    }

    #[test]
    fn entities_fall_in_draftwright_categories() {
        assert_eq!(category_of_entity("dimensional_size"), Category::Dimension);
        assert_eq!(
            category_of_entity(
                "geometric_tolerance+geometric_tolerance_with_datum_reference+position_tolerance"
            ),
            Category::GeometricTolerance
        );
        assert_eq!(category_of_entity("datum_feature"), Category::Datum);
        assert_eq!(category_of_entity("shape_aspect"), Category::Other);
    }
}
