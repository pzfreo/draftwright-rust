//! The document model ([ADR R1](../docs/adr/R1-document-model.md), [docs/model.md]):
//! - **Analysis**: what the part is;
//! - **Spec**: what the author decided, keyed by stable IDs;
//! - **Plan**: what the sheet will say, every item with its ID, default flag and basis.
//!
//! The operations over them are [`analyse`], [`plan`], [`edit`] and [`render`]: pure, stateless
//! and deterministic.
//!
//! This first cut covers the views (which views, the sheet, the scale and the placement) and a
//! title block. Features, dimensions and annotations join the same documents as they are ported.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use haecceity::Part;
use haecceity::classify::{Classifier, State};
use haecceity::hlr::{self, Exact, Plane, Projected, Unresolved, View, project, project_section};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bridge::{Feature, Recognition};
use crate::dims::{Omitted, PlannedDimension};
use crate::items::State as ItemState;
use crate::layout::{self, Corner, Fallback, MARGIN, Principals, Request, Scaled};
use crate::notes::{NoteSpec, NotesBlock, PlannedNote};
use crate::output::{
    Annotation, Claim, Curve, Drawing, Drawn, DrawnField, DrawnView, HAlign, Item, Kind, Layer,
    Sheet, TextRun, VAlign, items_bbox, layer_styles, projected,
};
use crate::tables::{PlannedTable, TableSpec};
use crate::text::{Font, Laid, Segment};
use crate::title_block::{self, TitleCell};

/// Version 2 added the isometric box (`iso_bounds`) and the recognised features, version 3 the
/// horizontal face levels (`levels`), version 4 their supports and the step risers
/// (`level_supports`, `risers`), version 5 the families the recogniser did not give
/// (`unrecognised`), version 6 the principal boundary faces' loops lint's profile check reads
/// (`principal_profiles`), version 7 the part's AP242 PMI (`pmi`).
pub const ANALYSIS_SCHEMA: &str = "draftwright.analysis/7";
/// The analyses [`plan`] reads: this version, and versions 2 to 6 (it reads them with none of
/// what they lack).
pub const ANALYSIS_SCHEMAS: [&str; 6] = [
    "draftwright.analysis/2",
    "draftwright.analysis/3",
    "draftwright.analysis/4",
    "draftwright.analysis/5",
    "draftwright.analysis/6",
    ANALYSIS_SCHEMA,
];
/// Version 2 names a dimension by what it measures, `d:<feature>/<parameter>` (version 1 added
/// the view that carries it, `@<view>`), and a PMI item by its record's semantic identity
/// (version 1 by its Part 21 instance, `pmi:<category>:#<n>`); the carrying view is the entry's
/// `view`. A version 1 spec is read through [`migrate`].
pub const SPEC_SCHEMA: &str = "draftwright.spec/2";
/// The specs [`plan`] reads: this version, and version 1 through [`migrate`].
pub const SPEC_SCHEMAS: [&str; 2] = ["draftwright.spec/1", SPEC_SCHEMA];
pub const PLAN_SCHEMA: &str = "draftwright.plan/1";

/// Which file, and which reading of it, a document was made from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_sha256: String,
    pub part: String,
    pub face_count: usize,
    pub loader: String,
    /// The part of an assembly read alone, by its index in the file's distinct parts
    /// (haecceity's order, as `draftwright parts` lists them): `part` is then that part's name,
    /// and the Analysis is in the part's own frame. Absent when the whole file was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_index: Option<usize>,
}

/// One distinct part of a STEP file, as `draftwright parts` lists it (specify-core's
/// `{part, name, faces, instances}`, with the product's id): what a caller chooses a part of an
/// assembly by. The index is the identity; the name and product id are reported beside it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilePart {
    pub part: usize,
    pub name: String,
    pub product_id: String,
    pub faces: usize,
    /// How many times the part is placed.
    pub instances: usize,
}

/// What the part is: regenerable from its STEP file, never edited.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub schema: String,
    pub binding: Binding,
    /// The part's box in its own frame, mm.
    pub envelope: [[f64; 3]; 2],
    /// Absent in older analyses; several solids make completeness findings informational.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid_count: Option<usize>,
    /// Principal outer stock faces, credited only when the overall sizes are stated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock_faces: Vec<usize>,
    /// Where the features came from; absent when none were recognised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recognition: Option<Recognition>,
    #[serde(default)]
    pub features: Vec<Feature>,
    /// Exact trimmed-surface attachment points for recognised blends, in the part frame.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub blend_sites: BTreeMap<String, Vec<[f64; 3]>>,
    /// Small, grouped shallow relief regions whose profiles are governed by the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub radius_reliefs: Vec<crate::blends::Relief>,
    /// The isometric's projected box (visible and hidden edges), `[x0, y0, x1, y1]` mm at 1:1,
    /// about the projection of the envelope's centre: what the isometric is fitted by. All zero
    /// when the projection has no edges (the plan then reports `iso_projection_empty`).
    pub iso_bounds: [f64; 4],
    /// The part's edges and faces, compactly, from which the planner proves a region of a view
    /// blank before placing a label inside it ([`projected_ink`]). Absent in older analyses,
    /// which then keep every label outside the views.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_ink: Option<projected_ink::EdgeInk>,
    /// The heights (z, mm) of the part's horizontal face levels: what a prismatic part's height
    /// ladder dimensions ([`crate::demand::face_levels`]).
    #[serde(default)]
    pub levels: Vec<f64>,
    /// Each horizontal face level's support (quiddity's `FaceLevel` records, read in the part's
    /// frame): its height and the x and y extents of its faces, per solid.
    #[serde(default)]
    pub level_supports: Vec<LevelSupport>,
    /// The walls and ramps that rise between the levels (quiddity's `RiserEvidence`, read in the
    /// part's frame): where a prismatic part's steps are located along x and y.
    #[serde(default)]
    pub risers: Vec<Riser>,
    /// Each solid's principal boundary faces (each solid of a part of several, else the whole
    /// part), with their loops' edges: what lint's principal profile check judges
    /// ([`crate::lint::profiles`]), read from the part as draftwright's lint reads it.
    #[serde(default)]
    pub principal_profiles: Vec<crate::lint::profiles::Source>,
    /// The families of features the recogniser did not give, so the part may have features of
    /// them that the Analysis lacks: empty for Python quiddity's recognition, which the bridge
    /// reads in full; for Rust quiddity's ([`analyse_native`]), the families its document has
    /// that [`crate::bridge::native`] does not read yet, and those it does not carry. The plan
    /// reports them (`features_not_recognised`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unrecognised: Vec<Unrecognised>,
    /// The part's AP242 semantic PMI ([`crate::pmi::attach`]): its records, each with where it
    /// attaches, and every source not read; or why the file's PMI could not be read. Absent
    /// when the file has none. The plan lowers it to feature control frames and datum feature
    /// symbols ([`crate::pmi_lowering`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pmi: Option<crate::pmi::Reading>,
    /// The file's distinct parts, when it holds more than one (an assembly): absent for a file
    /// of one part. With no part chosen (`binding.part_index` absent) they are drawn together
    /// as one, and the plan warns (`assembly_drawn_whole`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<FilePart>,
}

/// A family of features the recogniser did not give the Analysis (quiddity's family name).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unrecognised {
    pub family: String,
    /// How many records of it the recogniser's document has on this part; absent when the
    /// document does not carry the family at all, so whether the part has any is not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub records: Option<usize>,
}

/// A horizontal face level's support: its height and the extents of its faces, mm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelSupport {
    pub z: f64,
    pub x_span: [f64; 2],
    pub y_span: [f64; 2],
}

/// A step riser as quiddity records it (`RiserEvidence`): a wall (`vertical`) or ramp facing
/// along `axis`, at `positions` along it (and, for a bounded ramp, `other_positions` along
/// `other_axis`), rising from `z_lo` to `z_hi`; whether each end lies on the part's top or bottom;
/// the tolerance it was read with; and the heights of its solid's levels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Riser {
    pub vertical: bool,
    pub axis: String,
    pub positions: Vec<f64>,
    pub other_axis: String,
    pub other_positions: Vec<f64>,
    pub z_lo: f64,
    pub z_hi: f64,
    pub lo_at_envelope: bool,
    pub hi_at_envelope: bool,
    pub tol: f64,
    pub body_levels: Vec<f64>,
}

/// What the author decided. Anything absent takes its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Spec {
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<Binding>,
    pub sheet: SheetSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheets: Option<crate::sheets::SheetsSpec>,
    /// Per standard view (`front`, `plan`, `side`, `iso`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, ViewSpec>,
    /// Where the dimensions come from: `auto` (the defaults, less what `dimensions` omits) or
    /// `authored` (exactly the dimensions listed, a complete set: ADR4-5).
    #[serde(skip_serializing_if = "DimensionSource::is_auto")]
    pub dimension_source: DimensionSource,
    /// Plan-time hole presentation; rescue tables also obey `never`.
    #[serde(skip_serializing_if = "crate::hole_table::Setting::is_auto")]
    pub hole_table: crate::hole_table::Setting,
    /// Per dimension, by its ID (`d:<feature>/<parameter>`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dimensions: BTreeMap<String, DimensionSpec>,
    /// Placement and omission of semantic PMI frames, datums and labels, by stable ID.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, crate::placement_intent::AnnotationSpec>,
    /// Per feature, by its ID (`f:<kind>:<n>`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, FeatureSpec>,
    /// General notes, in the order they are numbered ([`crate::notes`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<NoteSpec>,
    /// Corner-block tables (revision blocks, BOMs, schedules), placed in order
    /// ([`crate::tables`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<TableSpec>,
}

/// The two sources of dimension intent (ADR4-5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DimensionSource {
    /// The defaults, less what the spec omits.
    #[default]
    Auto,
    /// Exactly the dimensions the spec lists.
    Authored,
}

impl DimensionSource {
    fn is_auto(&self) -> bool {
        *self == DimensionSource::Auto
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DimensionSpec {
    /// Leave it off the sheet (in `auto` only; an authored set leaves out by not listing).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub omit: bool,
    /// Its tolerance, printed after its value: `±T` or `+U -L` ([`crate::dims`]'s tolerance
    /// rules; ARCH0011-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<String>,
    /// The view that carries it (`front`, `plan` or `side`), where the default would put it
    /// elsewhere: no longer part of its ID (`draftwright.spec/2`). One that cannot carry it is
    /// refused; one that is omitted leaves it in its default view, with a warning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Position relative to its feature and carrying view; never page coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::placement_intent::Place>,
}

/// What the author decided of a feature.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FeatureSpec {
    /// An ISO 286 fit class on a hole's bore or a cylindrical boss's diameter (`H7`, `g6`),
    /// printed after the diameter of the callout that states it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            schema: SPEC_SCHEMA.into(),
            binding: None,
            sheet: SheetSpec::default(),
            sheets: None,
            views: BTreeMap::new(),
            dimension_source: DimensionSource::Auto,
            hole_table: crate::hole_table::Setting::Auto,
            dimensions: BTreeMap::new(),
            features: BTreeMap::new(),
            annotations: BTreeMap::new(),
            notes: Vec::new(),
            tables: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SheetSpec {
    /// `A4` … `A0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// `n:m`, from ISO 5455's series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub title: BTreeMap<String, String>,
    /// Whether a fit prints its ISO 286 limits beside its class (`⌀10 H7 (+0.015/0)`): shown
    /// unless false, which prints the class alone (`⌀10 H7`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit_limits: Option<bool>,
    /// ISO 5456-2's projection method, `first` or `third` (angle): where the plan and side sit
    /// about the front, and the symbol beside the title block. Third angle unless set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ViewSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<u32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub omit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::placement_intent::ViewPlace>,
}

/// What the sheet will say.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub schema: String,
    pub binding: Binding,
    #[serde(default)]
    pub status: crate::completeness::Status,
    #[serde(default)]
    pub completeness: crate::completeness::Completeness,
    pub sheet: PlannedSheet,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<PlannedSheet>,
    /// Escalation evidence for allocation; never inferred from a problem sentence.
    #[serde(skip)]
    pub table_outcome: crate::hole_table::TableOutcome,
    #[serde(skip)]
    pub(crate) table_restore: Option<Box<Plan>>,
    pub views: Vec<PlannedView>,
    /// Dimensions and callouts, each with its ID, value, label, default flag and basis.
    #[serde(default)]
    pub dimensions: Vec<PlannedDimension>,
    /// Canonical widths for the editor's snap guide; not part of the drawing document.
    #[serde(skip)]
    pub(crate) callout_widths: BTreeMap<String, f64>,
    /// Dimensions the spec leaves out, and why (ADR4-8: an authored omission says so).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omitted: Vec<Omitted>,
    /// General notes, each with its ID, text, rows, default flag, basis and place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<PlannedNote>,
    /// The block the notes are drawn in, when it has a place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_block: Option<NotesBlock>,
    /// A separate compact radius block when authored/PMI notes keep their ruled table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_notes_block: Option<NotesBlock>,
    /// Obstructed radius groups, located by every occurrence's centre in its named view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_table: Option<crate::dims::blends::RadiusTable>,
    /// Corner-block tables, each with its ID, cells, default flag, basis and place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<PlannedTable>,
    /// The part's AP242 geometric tolerances and datums as feature control frames and datum
    /// feature symbols, each with its ID, view, compartments, basis and place
    /// ([`crate::dims::gdt`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gdt: Vec<crate::dims::gdt::PlannedGdt>,
    /// The part's drawing-wide AP242 requirements (general tolerance, default surface finish,
    /// edge conditions), each with what carries it ([`crate::pmi_lowering::plan_sheet`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<crate::pmi_lowering::PlannedRequirement>,
    /// The plan-view hole table a dropped hole item escalated to ([`crate::hole_table`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_table: Option<crate::hole_table::HoleTable>,
    /// Hole schedules chosen for legibility, one per end-on principal view.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hole_tables: Vec<crate::hole_table::HoleTable>,
    /// The balloons beside the plan view: the hole table's rows' and dropped patterns'.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub balloons: Vec<crate::hole_table::Balloon>,
    pub problems: Vec<Problem>,
    /// The spec the plan was made from, embedded in every output.
    pub spec: Spec,
}

impl Plan {
    /// Widest canonical callout in a view, in paper mm. None for a view without
    /// callouts or a deserialised Plan: this metadata is retained only by planning.
    pub fn canonical_callout_width(&self, view: &str) -> Option<f64> {
        self.callout_widths.get(view).copied()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedSheet {
    #[serde(
        default = "crate::sheets::one",
        skip_serializing_if = "crate::sheets::is_one"
    )]
    pub number: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zones: Option<crate::sheets::Zones>,
    /// Union of drawn-item footprints as a share of usable paper (not reserved bands).
    #[serde(default)]
    pub fill: f64,
    pub size: String,
    /// Width and height, mm.
    pub page: [f64; 2],
    pub scale: String,
    /// The principal views' scale as a number (page mm per world mm).
    pub scale_factor: f64,
    /// Where the isometric sits relative to the orthographic row: `columns`, `stacked-iso`.
    pub arrangement: String,
    /// The title block's box, `[x0, y0, x1, y1]` mm.
    pub title_block: [f64; 4],
    pub default: bool,
    pub basis: String,
    /// The sheet's state (its ID is `sheet`): always shown.
    pub state: crate::items::State,
    pub title: BTreeMap<String, String>,
    /// The title block's cells, each with its box and the value drawn in it (ISSUE-1585-a: a
    /// supplied field is recorded as drawn, in its own cell).
    #[serde(default)]
    pub title_cells: Vec<TitleCell>,
    /// Resolved projection furniture; absent in plans written before the symbol was ported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_symbol: Option<title_block::ProjectionPlaced>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedView {
    #[serde(
        default = "crate::sheets::one",
        skip_serializing_if = "crate::sheets::is_one"
    )]
    pub sheet: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xref: Option<crate::sheets::CrossRef>,
    pub id: String,
    /// Towards the viewer, and the view's up.
    pub direction: [f64; 3],
    pub up: [f64; 3],
    /// Where the centre of the part's box lands on the page, mm.
    pub at: [f64; 2],
    /// The view's own scale: the sheet's, except an isometric resized to fill its space.
    pub scale: f64,
    /// The page area reserved for the view, `[x0, y0, x1, y1]` mm: a principal view's box with
    /// its annotation bands, or the isometric's rectangle.
    pub area: [f64; 4],
    pub default: bool,
    pub basis: String,
    /// Always shown: a view the plan does not draw is not among its views.
    pub state: crate::items::State,
    /// Automatic chain-line furniture, in paper coordinates; never Spec-addressable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_work: Vec<crate::centrelines::LineWork>,
    /// A section view's cutting plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<Cut>,
}

/// A section's cutting plane: the material on the side `normal` points to is kept and seen from
/// the side removed. The plane is marked on the `marked_in` view, its ends lettered `label`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xref: Option<crate::sheets::CrossRef>,
    pub label: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub marked_in: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    pub message: String,
    /// The items it is about, by their IDs: the plan's items ([`crate::items::items`], the sheet
    /// as `sheet`) and the Analysis's features.
    pub subjects: Vec<String>,
    /// What else it names that is no item of the plan or feature of the Analysis: a PMI
    /// record's source (its Part 21 provenance, `geometric_tolerance:#123`), a source not read,
    /// a family the recogniser does not give, a view the plan does not draw, a spec entry not
    /// applied. Pinned leader crossings also repeat the callout ID as pin provenance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    /// The edit that resolves it, as [`edit`] takes it, where there is one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fix: Vec<crate::items::Op>,
}

/// A request that cannot be met as asked: what was wrong, and what would be accepted.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Invalid {
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.choices.is_empty() {
            write!(f, " Choose one of: {}.", self.choices.join(", "))?;
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>, choices: &[&str]) -> Invalid {
    Invalid {
        message: message.into(),
        choices: choices.iter().map(|c| c.to_string()).collect(),
    }
}

/// The standard views, third-angle (draftwright's default projection): towards the viewer, up.
const VIEWS: [(&str, [f64; 3], [f64; 3]); 4] = [
    ("front", [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
    ("plan", [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ("side", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ("iso", [1.0, -1.0, 1.0], [0.0, 0.0, 1.0]),
];
/// Section A-A's view name (draftwright's `section_aa`).
pub const SECTION: &str = "section_aa";
const LOADER: &str = concat!("haecceity via draftwright ", env!("CARGO_PKG_VERSION"));

/// A scale as printed: `n:1` or `1:n`.
/// The Spec's projection methods (`sheet/projection`), ISO 5456-2's first and third angle.
pub const PROJECTIONS: [&str; 2] = ["first", "third"];

pub(crate) fn scale_name(s: f64) -> String {
    if s >= 1.0 {
        format!("{}:1", fmt_ratio(s))
    } else {
        format!("1:{}", fmt_ratio(1.0 / s))
    }
}

fn fmt_ratio(r: f64) -> String {
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round())
    } else {
        format!("{r:.3}")
    }
}

/// Why a STEP file gave no Analysis. It reads as its message.
#[derive(Clone, Debug, PartialEq)]
pub struct Unanalysed {
    /// `false`: the request named input that is not what it should be (a file that cannot be
    /// read, bytes that do not parse as STEP, recognition JSON not made from this file).
    /// `true`: the input is what it should be, and the kernel or the bridge could not use it (a
    /// STEP entity the kernel does not model, geometry it could not resolve, a face whose area
    /// it measures differently from the recognition's).
    pub failed: bool,
    pub message: String,
}

impl std::ops::Deref for Unanalysed {
    type Target = str;
    fn deref(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for Unanalysed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn unanalysed(failed: bool, message: String) -> Unanalysed {
    Unanalysed { failed, message }
}

/// Reads the STEP file at *path*: the part, and its analysis, without features.
pub fn analyse(path: &Path) -> Result<(Part, Analysis), Unanalysed> {
    analyse_with(path, None)
}

/// The distinct parts of the STEP file *bytes* (named *path*, for messages), in haecceity's
/// order: what [`analyse_step_part`]'s *part* indexes.
pub fn parts(path: &Path, bytes: &[u8]) -> Result<Vec<FilePart>, Unanalysed> {
    let defs = haecceity::step::read_part_definitions(bytes).map_err(|e| step_refused(path, e))?;
    Ok(file_parts(&defs))
}

fn file_parts(defs: &[haecceity::step::PartDefinition]) -> Vec<FilePart> {
    defs.iter()
        .enumerate()
        .map(|(part, d)| FilePart {
            part,
            name: d.name.clone(),
            product_id: d.product_id.clone(),
            faces: d.faces.len(),
            instances: d.placements.len(),
        })
        .collect()
}

/// `0: plate, 1: pin`: an assembly's parts as a message names them.
pub fn part_list(parts: &[FilePart]) -> String {
    parts
        .iter()
        .map(|p| format!("{}: {}", p.part, p.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why *path* could not be read as STEP: a file that does not parse is not STEP; one that parses
/// but holds what the kernel does not model (or lost on reading) is the kernel's failure.
fn step_refused(path: &Path, e: haecceity::step::StepError) -> Unanalysed {
    let failed = !matches!(e, haecceity::step::StepError::Parse(_));
    unanalysed(
        failed,
        format!("cannot read {} as STEP: {e}", path.display()),
    )
}

/// Reads the STEP file at *path*, with its features from *recognition*: Python quiddity's JSON
/// for the same file, written by `tools/recognise.py` ([`crate::bridge`]). Refused when the
/// kernel did not resolve a face or edge the isometric draws.
pub fn analyse_with(
    path: &Path,
    recognition: Option<&Path>,
) -> Result<(Part, Analysis), Unanalysed> {
    analyse_with_part(path, recognition, None)
}

/// [`analyse_with`], reading one part of an assembly alone when *part* is given
/// ([`analyse_step_part`]).
pub fn analyse_with_part(
    path: &Path,
    recognition: Option<&Path>,
    part: Option<usize>,
) -> Result<(Part, Analysis), Unanalysed> {
    let bytes = std::fs::read(path)
        .map_err(|e| unanalysed(false, format!("cannot read {}: {e}", path.display())))?;
    let (part, mut analysis) = analyse_step_part(path, &bytes, part)?;
    if let Some(r) = recognition {
        let json = std::fs::read_to_string(r)
            .map_err(|e| unanalysed(false, format!("cannot read {}: {e}", r.display())))?;
        recognise(&part, &mut analysis, &json).map_err(|e| {
            // JSON of the right format made from this very file is what the request should
            // name; the bridge refusing it after that (a face it measures differently, a record
            // it has no home for) is the port's failure, not the caller's.
            let header = serde_json::from_str::<serde_json::Value>(&json).ok();
            let field = |k: &str| {
                header
                    .as_ref()
                    .and_then(|h| h.get(k)?.as_str().map(String::from))
            };
            let failed = field("format").as_deref() == Some(crate::bridge::RECOGNITION_FORMAT)
                && field("source_sha256").as_ref() == Some(&analysis.binding.source_sha256);
            unanalysed(failed, format!("{}: {e}", r.display()))
        })?;
    }
    Ok((part, analysis))
}

/// Reads *bytes*, the STEP file named *path* (which is not read: the name is for the part and
/// for messages), as [`analyse_with`] reads a file, without features: for callers that have the
/// file's bytes and no file system, such as a browser.
pub fn analyse_step(path: &Path, bytes: &[u8]) -> Result<(Part, Analysis), Unanalysed> {
    analyse_step_part(path, bytes, None)
}

/// [`analyse_step`], of one part of the file when *part* is given: part *k* of [`parts`], read
/// alone in its own frame (`haecceity::StepFile::part`, its first placement's shape with no
/// placement applied), its PMI only (other parts' records are dropped). The binding names the
/// part and its index. With none, the whole file is read as one part, as it always was; an
/// assembly's parts are then listed in `parts` and the plan warns that they are drawn together.
/// Refused, naming the parts, when there is no part *k*.
pub fn analyse_step_part(
    path: &Path,
    bytes: &[u8],
    chosen: Option<usize>,
) -> Result<(Part, Analysis), Unanalysed> {
    // The file's part definitions are read in both cases (the PMI reading needs them too, and
    // is given these rather than reading them again).
    let (part, defs, name) = match chosen {
        None => (
            haecceity::read_step(bytes).map_err(|e| step_refused(path, e))?,
            haecceity::step::read_part_definitions(bytes)
                .map_err(|e| format!("cannot read the parts: {e}")),
            path.file_stem().map_or(String::new(), |s| {
                s.to_string_lossy().trim_end_matches(".step").to_string()
            }),
        ),
        Some(k) => {
            let file = haecceity::step::StepFile::read(bytes).map_err(|e| step_refused(path, e))?;
            if k >= file.parts.len() {
                return Err(unanalysed(
                    false,
                    format!(
                        "{} has {} part(s) ({}); there is no part {k}",
                        path.display(),
                        file.parts.len(),
                        part_list(&file_parts(&file.parts))
                    ),
                ));
            }
            let part = file.part(k).map_err(|e| step_refused(path, e))?;
            let name = file.parts[k].name.clone();
            (part, Ok(file.parts), name)
        }
    };
    let b = part.bounds();
    let (level_supports, risers) = crate::dims::levels::evidence(&part);
    let mut analysis = Analysis {
        schema: ANALYSIS_SCHEMA.into(),
        binding: Binding {
            source_sha256: Sha256::digest(bytes)
                .iter()
                .map(|x| format!("{x:02x}"))
                .collect(),
            part: name,
            face_count: part.faces.len(),
            loader: LOADER.into(),
            part_index: chosen,
        },
        envelope: [b.min, b.max],
        solid_count: Some(part.solids.len()),
        stock_faces: (0..part.faces.len())
            .filter(|&f| {
                if !matches!(
                    part.faces[f].surface,
                    haecceity::geom::Surface::Plane { .. }
                ) {
                    return false;
                }
                let fb = part.face_bounds(f);
                (0..3).any(|k| {
                    fb.max[k] - fb.min[k] <= 1e-5
                        && ((fb.min[k] - b.min[k]).abs() <= 1e-5
                            || (fb.max[k] - b.max[k]).abs() <= 1e-5)
                })
            })
            .collect(),
        recognition: None,
        features: Vec::new(),
        blend_sites: BTreeMap::new(),
        radius_reliefs: crate::blends::reliefs(&part),
        iso_bounds: [0.0; 4],
        edge_ink: None,
        levels: crate::demand::face_levels(&part),
        level_supports,
        risers,
        principal_profiles: crate::lint::profiles::evidence(&part),
        unrecognised: Vec::new(),
        pmi: crate::pmi::attach_with(
            bytes,
            &part,
            defs.as_deref().map_err(String::as_str),
            chosen,
        ),
        parts: match &defs {
            Ok(defs) if defs.len() > 1 => file_parts(defs),
            _ => Vec::new(),
        },
    };
    let (_, toward, up) = VIEWS[3];
    let iso = View::new(toward, up).expect("standard view");
    let centre = extent(&analysis, &iso).1;
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in project(&part, &iso)
        .map_err(|e| {
            unanalysed(
                true,
                format!(
                    "cannot draw {}: the kernel did not resolve its {e} on reading",
                    path.display()
                ),
            )
        })?
        .iter()
        .flat_map(|piece| piece.points.iter())
    {
        let q = [p[0] - centre[0], p[1] - centre[1]];
        bounds = [
            bounds[0].min(q[0]),
            bounds[1].min(q[1]),
            bounds[2].max(q[0]),
            bounds[3].max(q[1]),
        ];
    }
    if bounds[0].is_finite() {
        analysis.iso_bounds = bounds;
    }
    analysis.edge_ink = Some(projected_ink::evidence(&part));
    Ok((part, analysis))
}

/// [`analyse_step`], with its features from Rust quiddity's recognition, as [`analyse_native`].
pub fn analyse_native_step(path: &Path, bytes: &[u8]) -> Result<(Part, Analysis), Unanalysed> {
    analyse_native_step_part(path, bytes, None)
}

/// [`analyse_native_step`] of one part of the file when *part* is given ([`analyse_step_part`]).
pub fn analyse_native_step_part(
    path: &Path,
    bytes: &[u8],
    part: Option<usize>,
) -> Result<(Part, Analysis), Unanalysed> {
    let (part, mut analysis) = analyse_step_part(path, bytes, part)?;
    recognise_native(&part, &mut analysis).map_err(|e| {
        unanalysed(
            true,
            format!(
                "Rust quiddity's recognition of {} was refused: {e}",
                path.display()
            ),
        )
    })?;
    Ok((part, analysis))
}

/// Takes *analysis*'s features from Python quiddity's recognition JSON for the same file
/// ([`crate::bridge`]).
pub fn recognise(part: &Part, analysis: &mut Analysis, json: &str) -> Result<(), String> {
    let (recognition, features) = crate::bridge::read(part, &analysis.binding.source_sha256, json)?;
    analysis.recognition = Some(recognition);
    analysis.blend_sites = crate::blends::evidence(part, &features);
    analysis.features = features;
    analysis.unrecognised.clear();
    Ok(())
}

/// The families draftwright reads from Python quiddity that Rust quiddity's recognition document
/// does not carry: section recesses (pockets, channels, blind slots and circular recesses, as
/// draftwright reads them).
pub const NOT_IN_NATIVE_DOCUMENT: [&str; 1] = ["section_recesses"];

/// Reads the STEP file at *path*, as [`analyse_with`] reads it, with its features from Rust
/// quiddity's recognition document ([`crate::bridge::native`]) in place of Python quiddity's JSON.
/// Refused, as the kernel's failure, when the native reader refuses the document; never read as
/// a part without features.
pub fn analyse_native(path: &Path) -> Result<(Part, Analysis), Unanalysed> {
    analyse_native_part(path, None)
}

/// [`analyse_native`] of one part of the file when *part* is given ([`analyse_step_part`]).
pub fn analyse_native_part(
    path: &Path,
    part: Option<usize>,
) -> Result<(Part, Analysis), Unanalysed> {
    let (part, mut analysis) = analyse_with_part(path, None, part)?;
    recognise_native(&part, &mut analysis).map_err(|e| {
        unanalysed(
            true,
            format!(
                "Rust quiddity's recognition of {} was refused: {e}",
                path.display()
            ),
        )
    })?;
    Ok((part, analysis))
}

/// Takes *analysis*'s features from Rust quiddity's recognition of *part*
/// ([`recognise_document`]).
pub fn recognise_native(part: &Part, analysis: &mut Analysis) -> Result<(), String> {
    let recognition = quiddity::correspondence::recognise(part);
    recognise_document(
        part,
        analysis,
        &quiddity::recognition::document(&recognition, part.faces.len()),
    )
}

/// Takes *analysis*'s features from *doc*, Rust quiddity's recognition document for *part*
/// ([`crate::bridge::native`]), in the part's own frame, and records in `unrecognised` the
/// families *doc* has that are not read, and those it does not carry
/// ([`NOT_IN_NATIVE_DOCUMENT`]). A refused document leaves *analysis* as it was.
pub fn recognise_document(
    part: &Part,
    analysis: &mut Analysis,
    doc: &quiddity::recognition::RecognitionDocument,
) -> Result<(), String> {
    use crate::bridge::native;
    let read = native::read(part, doc)?;
    let mut features = read.holes;
    features.extend(read.features);
    // As the bridge orders them: by kind, then faces (each kind's IDs are ranked so already).
    features.sort_by(|a, b| (&a.kind, &a.faces).cmp(&(&b.kind, &b.faces)));
    let mut counts = BTreeMap::<&str, usize>::new();
    for r in doc
        .records
        .iter()
        .filter(|r| !native::READ.contains(&r.family.as_str()))
    {
        *counts.entry(r.family.as_str()).or_default() += 1;
    }
    let mut unrecognised: Vec<Unrecognised> = NOT_IN_NATIVE_DOCUMENT
        .iter()
        .filter(|f| !native::READ.contains(f) && !counts.contains_key(*f))
        .map(|f| Unrecognised {
            family: f.to_string(),
            records: None,
        })
        .collect();
    unrecognised.extend(counts.into_iter().map(|(f, n)| Unrecognised {
        family: f.to_string(),
        records: Some(n),
    }));
    analysis.unrecognised = unrecognised;
    analysis.recognition = Some(Recognition {
        recogniser: read.recogniser,
        frame: native::IDENTITY,
        evidence: Vec::new(),
        refusals: Vec::new(),
        patterns: read.patterns,
    });
    analysis.blend_sites = crate::blends::evidence(part, &features);
    analysis.features = features;
    Ok(())
}

/// The box (width, height) and centre of the part's envelope seen in *view*, mm at 1:1.
pub(crate) fn extent(analysis: &Analysis, view: &View) -> ([f64; 2], [f64; 2]) {
    let [lo, hi] = analysis.envelope;
    let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for i in 0..8 {
        let corner = [0, 1, 2].map(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] });
        let p = view.map(corner);
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    (
        [max[0] - min[0], max[1] - min[1]],
        [0.5 * (min[0] + max[0]), 0.5 * (min[1] + max[1])],
    )
}

/// The problems that make a plan's layout invalid for the recovery: draftwright's hard layout
/// codes (`linting.quality._HARD_LAYOUT_CODES`) as the port reports them (`annotation_off_sheet`
/// is its `annotation_out_of_bounds`; views that do not fit are its `view_out_of_bounds` and
/// `view_overlap`). `annotation_over_view` is not one: the port judges it by the view's box,
/// draftwright's `view_annotation_overlap` by the view's line-work.
const HARD_LAYOUT: [&str; 9] = [
    "annotation_off_sheet",
    "annotation_over_title_block",
    "annotation_overlap",
    "annotation_ink_overlap",
    "title_field_overflow",
    "page_fit_uncertain",
    "views_do_not_fit",
    "notes_not_placed",
    "table_not_placed",
];

/// The tail of the `notes_not_placed` message for a blanket radius note withdrawn because an
/// exception is undrawn: a completeness refusal that no sheet or scale changes, so the sheet
/// choice does not count it as a hard layout finding.
const RADIUS_DEFAULT_WITHDRAWN: &str =
    "cannot assert a blanket radius while an exception is undrawn.";

/// A plan's findings for the recovery. The port drops for want of room only the hole items it
/// placed off the sheet ([`crate::hole_table`]: an item that collides is placed and reported; one
/// no shown view carries is kept unplaced, which no sheet or scale changes), so its completeness
/// tier reads those drops still reported (each a `*_dropped` blocker of draftwright's
/// `_is_required_scale_drop`; a hole table's rows clear the ones they state) and the one other
/// required loss draftwright's blockers have that a scale decides: hole locations too close to
/// dimension legibly ([`layout::crowded_locations`], draftwright's `location_ref_dropped`),
/// counted per view and axis, a pattern's centre ordinate apart (draftwright keeps it whatever
/// its spacing).
pub(crate) fn findings(plan: &Plan) -> layout::Findings {
    let mut axes: BTreeMap<(&str, &str), Vec<f64>> = BTreeMap::new();
    for d in &plan.dimensions {
        let Some(m) = d.measurements.first() else {
            continue;
        };
        let located = matches!(
            m.parameter.as_str(),
            "location.length.x" | "location.length.y"
        );
        let centre = d
            .measurements
            .iter()
            .any(|m| m.feature.starts_with("f:hole_pattern:"));
        if d.kind == "linear" && located && !centre {
            axes.entry((d.view.as_str(), m.parameter.as_str()))
                .or_default()
                .push(d.value);
        }
    }
    let scale = plan.sheet.scale_factor;
    layout::Findings {
        hard: usize::from(crate::sheets::floor(&plan.spec).is_some_and(|f| scale < f))
            + plan
                .problems
                .iter()
                .filter(|p| HARD_LAYOUT.contains(&p.code.as_str()) && p.severity != "info")
                .filter(|p| !p.message.ends_with(RADIUS_DEFAULT_WITHDRAWN))
                .count(),
        dropped: axes
            .values()
            .map(|o| layout::crowded_locations(o, scale))
            .sum::<usize>()
            + drops(plan),
    }
}

/// A plan's annotations over a view not their own: a label on a view's box or a line across
/// another view (`annotation_over_view`, `annotation_line_over_view`), at warning or error.
fn over_views(plan: &Plan) -> usize {
    plan.problems
        .iter()
        .filter(|p| {
            matches!(
                p.code.as_str(),
                "annotation_over_view" | "annotation_line_over_view"
            ) && p.severity != "info"
        })
        .count()
}

/// The hole items a plan dropped for want of room and still reports ([`crate::hole_table`]).
fn drops(plan: &Plan) -> usize {
    plan.problems
        .iter()
        .filter(|p| crate::hole_table::DROPS.contains(&p.code.as_str()))
        .count()
}

/// What a plan's recovery findings are, in words; *dropped* of them are hole items dropped.
fn said(f: layout::Findings, dropped: usize) -> String {
    let hard = match f.hard {
        1 => "1 hard layout finding".to_string(),
        n => format!("{n} hard layout findings"),
    };
    let crowded = match f.dropped - dropped {
        0 => None,
        1 => Some("1 hole location too close to read".to_string()),
        n => Some(format!("{n} hole locations too close to read")),
    };
    let lost = match dropped {
        0 => None,
        1 => Some("1 hole item dropped for want of room".to_string()),
        n => Some(format!("{n} hole items dropped for want of room")),
    };
    let parts: Vec<String> = (f.hard > 0)
        .then_some(hard)
        .into_iter()
        .chain(crowded)
        .chain(lost)
        .collect();
    if parts.is_empty() {
        "valid and complete".into()
    } else {
        parts.join(", ")
    }
}

/// Material crossings are counted by leader/feature pair and by witness line, as quality.py.
fn feature_crossings(plan: &Plan) -> usize {
    let mut leaders = BTreeSet::new();
    let mut extensions = BTreeSet::new();
    for p in &plan.problems {
        if p.code == "leader_through_feature" {
            leaders.insert(p.subjects.clone());
        }
        if p.code == "extension_through_feature"
            && let Some(owner) = p.subjects.first()
        {
            extensions.insert((
                owner.clone(),
                p.id.rsplit(":line:").next().unwrap_or_default().to_string(),
            ));
        }
    }
    leaders.len() + extensions.len()
}

fn ink_findings(plan: &Plan) -> usize {
    plan.problems
        .iter()
        .filter(|p| {
            matches!(
                p.code.as_str(),
                "annotation_ink_overlap"
                    | "annotation_over_view"
                    | "annotation_line_over_view"
                    | "annotation_leader_crossing"
                    | "annotation_outside_area"
            )
        })
        .count()
}

fn rect_overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// Supported measurements already carried by the seed, including a table's replacements.
fn carried(plan: &Plan) -> BTreeSet<(String, String)> {
    let mut out: BTreeSet<(String, String)> = plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .flat_map(|d| &d.measurements)
        .chain(
            plan.hole_table
                .iter()
                .chain(&plan.hole_tables)
                .flat_map(|t| {
                    t.rows
                        .iter()
                        .flat_map(|r| &r.measurements)
                        .chain(&t.covered)
                }),
        )
        // A placed general radius note and the radius-centre table carry radii as well.
        .chain(
            plan.notes
                .iter()
                .filter(|n| n.placed.is_some())
                .flat_map(|n| &n.measurements),
        )
        .map(|m| (m.feature.clone(), m.parameter.clone()))
        .collect();
    for r in plan.radius_table.iter().flat_map(|t| &t.rows) {
        if let Some(kind) = r
            .feature
            .strip_prefix("f:")
            .and_then(|f| f.split(':').next())
        {
            out.insert((r.feature.clone(), format!("{kind}.radius")));
        }
    }
    out
}

fn drawn_items_fit(analysis: &Analysis, p: &Plan) -> bool {
    let inside = |b: [f64; 4]| {
        b[0] >= layout::MARGIN - 1e-6
            && b[1] >= layout::MARGIN - 1e-6
            && b[2] <= p.sheet.page[0] - layout::MARGIN + 1e-6
            && b[3] <= p.sheet.page[1] - layout::MARGIN + 1e-6
    };
    let mut title = p.sheet.title_block;
    title[3] = title_block::top(title);
    crate::sheet_fit::footprints(analysis, p)
        .into_iter()
        .all(inside)
        && crate::sheet_fit::furniture_footprints(analysis, p)
            .into_iter()
            .all(|b| !rect_overlap(b, title))
        && p.views
            .iter()
            .all(|v| inside(v.area) && !rect_overlap(v.area, p.sheet.title_block))
        && p.views.iter().enumerate().all(|(i, a)| {
            p.views[i + 1..]
                .iter()
                .all(|b| !rect_overlap(a.area, b.area))
        })
}

pub(crate) fn undrawn_gdt(p: &Plan) -> usize {
    p.gdt
        .iter()
        .filter(|g| {
            g.placed.is_none()
                && !matches!(
                    g.state,
                    ItemState::OmittedByUser | ItemState::NotAuthored | ItemState::Covered { .. }
                )
        })
        .count()
}

/// Select from finished, complete drawings under the maintainer's sheet policy (2026-10-12).
/// Find the smallest fit and its largest scale; only its immediate larger sheet, with an
/// enlargement, may compete. Pins keep their canonical sheet; explicit choices override.
pub fn recover(analysis: &Analysis, spec: &Spec, first: Plan) -> Result<Plan, Invalid> {
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    // Preview and commit often plan identical intent. Keep a small process-local memo whose
    // key includes all three pure inputs, including injected seed findings and pin geometry.
    type Memo = VecDeque<([u8; 32], Plan)>;
    static MEMO: OnceLock<Mutex<Memo>> = OnceLock::new();
    if spec.sheet.scale.is_some() || crate::placement_intent::has_pins(spec) {
        return recover_uncached(analysis, spec, first);
    }
    let bytes = serde_json::to_vec(&(analysis, spec, &first)).expect("plan inputs serialize");
    let key: [u8; 32] = Sha256::digest(&bytes).into();
    let memo = MEMO.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Some((_, plan)) = memo
        .lock()
        .expect("sheet memo lock")
        .iter()
        .find(|(k, _)| *k == key)
    {
        return Ok(plan.clone());
    }
    let plan = crate::planning::with(analysis, || recover_uncached(analysis, spec, first))?;
    let mut cached = memo.lock().expect("sheet memo lock");
    cached.retain(|(k, _)| *k != key);
    if cached.len() == 16 {
        cached.pop_front();
    }
    cached.push_back((key, plan.clone()));
    Ok(plan)
}

/// The nearest sheet and scale that hold the views when a pinned pair does not (#13), as the
/// edit that sets them, chosen by the engine's own search: the pinned sheet at the scale the
/// recovery picks for it, else the pinned scale on the smallest sheet that holds it, else the
/// automatic pair. A pair is offered only when, pinned, its finished drawing fits. Only for a
/// spec that pins both (each search unpins one, so this never recurses).
fn fit_fix(
    analysis: &Analysis,
    spec: &Spec,
) -> Result<Option<(Vec<crate::items::Op>, String, String)>, Invalid> {
    let (Some(size), Some(scale)) = (&spec.sheet.size, &spec.sheet.scale) else {
        return Ok(None);
    };
    let unpinned = |size: bool, scale: bool| {
        let mut open = spec.clone();
        if size {
            open.sheet.size = None;
        }
        if scale {
            open.sheet.scale = None;
        }
        open
    };
    let mut tried = BTreeSet::new();
    for (open_size, open_scale) in [(false, true), (true, false), (true, true)] {
        let open = unpinned(open_size, open_scale);
        let found = if open_scale {
            recover(analysis, &open, plan_escalated(analysis, &open)?)?
        } else {
            plan_escalated(analysis, &open)?
        };
        let pair = (found.sheet.size.clone(), found.sheet.scale.clone());
        if (&pair.0, &pair.1) == (size, scale) || !tried.insert(pair.clone()) {
            continue;
        }
        let mut pinned = spec.clone();
        pinned.sheet.size = Some(pair.0.clone());
        pinned.sheet.scale = Some(pair.1.clone());
        let mut p = plan_escalated(analysis, &pinned)?;
        let doubted = p
            .problems
            .iter()
            .any(|q| matches!(q.code.as_str(), "page_fit_uncertain" | "views_do_not_fit"));
        let fits = !doubted || drawn_items_fit(analysis, &p) || {
            crate::sheet_fit::correct_estimate(analysis, &mut p);
            drawn_items_fit(analysis, &p)
        };
        if !fits {
            continue;
        }
        let mut ops = Vec::new();
        if pair.0 != *size {
            ops.push(crate::items::Op::set("sheet/size", pair.0.clone().into()));
        }
        if pair.1 != *scale {
            ops.push(crate::items::Op::set("sheet/scale", pair.1.clone().into()));
        }
        return Ok(Some((ops, pair.0, pair.1)));
    }
    Ok(None)
}

fn recover_uncached(analysis: &Analysis, spec: &Spec, mut first: Plan) -> Result<Plan, Invalid> {
    use crate::sheet_fit::FILL_BAND;
    let min_legible_scale = crate::sheets::floor(spec).map_or_else(
        || crate::sheet_fit::min_legible_scale(analysis),
        |floor| floor.max(crate::sheet_fit::min_legible_scale(analysis)),
    );
    first.sheet.fill = crate::sheet_fit::fill(analysis, &first);
    if spec.sheet.scale.is_some() || crate::placement_intent::has_pins(spec) {
        // A pinned sheet and scale are judged as the automatic search judges its candidates
        // (#13): the packed estimate's doubt is withdrawn when the finished drawing's items and
        // views all fit the sheet, so the pair the engine would choose itself raises nothing.
        // When they really do not fit, the problem carries the nearest pair that does.
        if let Some(i) = first
            .problems
            .iter()
            .position(|q| q.code == "page_fit_uncertain")
        {
            // The drawing as planned first; failing that, after the measured correction the
            // automatic search gives its candidates (so the pinned pair is drawn as the search
            // would draw it; a no-op for pinned placements).
            let mut corrected = first.clone();
            if !drawn_items_fit(analysis, &corrected) {
                crate::sheet_fit::correct_estimate(analysis, &mut corrected);
            }
            if drawn_items_fit(analysis, &corrected) {
                first = corrected;
                first.problems.remove(i);
                first.sheet.fill = crate::sheet_fit::fill(analysis, &first);
                if let Some((basis, _)) = first.sheet.basis.rsplit_once("; drawn-item fill ") {
                    first.sheet.basis =
                        format!("{basis}; drawn-item fill {:.1}%", first.sheet.fill * 100.0);
                }
                first.sheet.basis = first.sheet.basis.replacen(
                    "; the views may not fit",
                    "; the finished drawing fits",
                    1,
                );
                first.status = crate::completeness::status(&first.problems);
            } else if let Some((fix, size, scale)) = fit_fix(analysis, spec)? {
                let p = &mut first.problems[i];
                p.message = format!(
                    "{}; {size} at {scale} fits.",
                    p.message.trim_end_matches('.')
                );
                p.fix = fix;
            }
        }
        return Ok(first);
    }
    if !first.hole_tables.is_empty() {
        return recover_tables(analysis, spec, first);
    }
    let seed_clean =
        findings(&first).clean() && undrawn_gdt(&first) == 0 && drawn_items_fit(analysis, &first);
    let required = carried(&first);
    let mut tried = Vec::new();
    let mut candidates = Vec::new();
    let mut smallest = None;
    let mut smallest_iso: Option<f64> = None;
    let mut recipes = BTreeMap::new();
    for (i, (name, _, _)) in layout::PAGES.iter().enumerate() {
        if spec.sheet.size.as_ref().is_some_and(|n| n != name) {
            continue;
        }
        if smallest.is_some_and(|(j, _)| i > j + 1) {
            break;
        }
        for scale in layout::SCALES
            .into_iter()
            .filter(|s| *s >= min_legible_scale)
        {
            // On the one larger sheet only an enlargement of the smallest fit is allowed.
            if smallest.is_some_and(|fit| !crate::sheet_fit::allowed(fit, i, scale)) {
                continue;
            }
            // A view larger than the usable paper cannot be repaired by annotation placement.
            // Reject it before running the finished solver (especially important on dense PMI).
            let (_, w, h) = layout::PAGES[i];
            if first.views.iter().filter(|v| v.id != "v:iso").any(|v| {
                let view = View::new(v.direction, v.up).expect("planned view");
                let (size, _) = extent(analysis, &view);
                size[0] * scale > w - 2.0 * layout::MARGIN
                    || size[1] * scale > h - 2.0 * layout::MARGIN
            }) {
                tried.push(format!(
                    "{name} {} (principal ink exceeds usable paper; not a fit candidate)",
                    scale_name(scale)
                ));
                continue;
            }
            let mut named = spec.clone();
            named.sheet.size = Some((*name).into());
            named.sheet.scale = Some(scale_name(scale));
            let mut p =
                if first.sheet.size == *name && first.sheet.scale_factor == scale && seed_clean {
                    first.clone()
                } else {
                    plan_escalated_with(analysis, &named, true, None)?
                };
            // A failed seed must not restrict the complete drawing to its first arrangement.
            // Try the other established arrangement, with no new leader-routing strategy.
            if !seed_clean {
                let quality = |p: &Plan| {
                    let f = findings(p);
                    (
                        f.hard,
                        f.dropped,
                        over_views(p),
                        ink_findings(p),
                        feature_crossings(p),
                    )
                };
                for arrangement in [
                    layout::Arrangement::Columns,
                    layout::Arrangement::StackedIso,
                    layout::Arrangement::StaggeredSide,
                ] {
                    if p.sheet.arrangement == arrangement.name() {
                        continue;
                    }
                    let alternative =
                        plan_escalated_with(analysis, &named, true, Some(arrangement))?;
                    if quality(&alternative) < quality(&p) {
                        p = alternative;
                    }
                }
            }
            if !p.gdt.is_empty() {
                let recipe = recipes.entry(scale.to_bits()).or_insert_with(|| {
                    let mut roomy = spec.clone();
                    roomy.sheet.size = Some("A0".into());
                    roomy.sheet.scale = Some(scale_name(scale));
                    plan_escalated_with(analysis, &roomy, true, None)
                        .ok()
                        .filter(|q| findings(q).clean())
                });
                if let Some(recipe) = recipe
                    && let Some(packed) = crate::sheet_fit::pack_gdt(analysis, recipe, &p)
                    && carried(&p).is_subset(&carried(&packed))
                    && (
                        findings(&packed).hard,
                        findings(&packed).dropped,
                        over_views(&packed),
                        feature_crossings(&packed),
                        ink_findings(&packed),
                    ) < (
                        findings(&p).hard,
                        findings(&p).dropped,
                        over_views(&p),
                        feature_crossings(&p),
                        ink_findings(&p),
                    )
                {
                    p = packed;
                }
            }
            // Enlargement is for the dimensioned views. On the smallest fit's larger neighbour
            // the orientation view keeps the smallest fit's paper scale, rather than spending the
            // extra sheet on decorative growth.
            if !spec.views.contains_key("iso")
                && let Some((fit, Some(cap))) = smallest.map(|(j, _)| (j, smallest_iso))
                && i > fit
                && let Some(iso) = p.views.iter_mut().find(|v| v.id == "v:iso")
                && iso.scale > cap
            {
                // Shrink about the drawn box's centre, so it stays inside its old footprint.
                let b = analysis.iso_bounds;
                let centre = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
                iso.at = [
                    iso.at[0] + (iso.scale - cap) * centre[0],
                    iso.at[1] + (iso.scale - cap) * centre[1],
                ];
                iso.scale = cap;
                iso.basis +=
                    "; the smallest fit's orientation scale kept while dimensioned views enlarge";
            }
            crate::sheet_fit::correct_estimate(analysis, &mut p);
            // An estimate's iso budget can refuse a sheet whose finished drawing fits.
            // Only discharge that estimate when all actual item bounds and reserved views fit.
            let actual_fit = drawn_items_fit(analysis, &p);
            if actual_fit {
                p.problems.retain(|q| q.code != "page_fit_uncertain");
            }
            p.status = crate::completeness::status(&p.problems);
            let mut f = findings(&p);
            f.hard += usize::from(!actual_fit);
            f.dropped += required.difference(&carried(&p)).count();
            f.dropped += undrawn_gdt(&p);
            let over = over_views(&p).saturating_sub(over_views(&first));
            let crossings = feature_crossings(&p).saturating_sub(feature_crossings(&first));
            p.sheet.fill = crate::sheet_fit::fill(analysis, &p);
            tried.push(format!(
                "{name} {} fill {:.1}% ({}, {over} additional view conflicts, {crossings} additional material crossings, {} annotation conflicts, {})",
                p.sheet.scale,
                p.sheet.fill * 100.0,
                said(f, drops(&p)),
                ink_findings(&p),
                p.sheet.arrangement
            ));
            // Valid and complete: no hard finding, nothing dropped or undrawn, and no more
            // annotations over another view than the seed (the port's own gate).
            if !f.clean() || over > 0 {
                continue;
            }
            if smallest.is_none() {
                smallest = Some((i, scale));
                smallest_iso = p.views.iter().find(|v| v.id == "v:iso").map(|v| v.scale);
            }
            candidates.push((
                crate::sheet_fit::Candidate {
                    sheet: i,
                    scale,
                    width: p.sheet.page[0],
                    fill: p.sheet.fill,
                    extra_crossings: crossings,
                },
                p,
            ));
        }
    }
    let keys: Vec<_> = candidates.iter().map(|(c, _)| *c).collect();
    let Some((choice, (i, s))) =
        crate::sheet_fit::choose(&keys, spec.sheet.size.is_some(), seed_clean)
    else {
        // No valid complete pair: keep the initial drawing and its findings, on its own paper.
        // Only a seed below the absolute floor (1:10, the smallest scale the automatic choice
        // ever offers) is redrawn at that floor; a seed between it and the feature floor is
        // kept and the shortfall reported, as no legible candidate drew more validly.
        let floor = crate::sheet_fit::ABSOLUTE_FLOOR;
        if first.sheet.scale_factor < floor && crate::sheets::floor(spec).is_none_or(|f| floor >= f)
        {
            let mut named = spec.clone();
            named.sheet.size = Some(first.sheet.size.clone());
            named.sheet.scale = Some(scale_name(floor));
            first = plan_escalated_with(analysis, &named, true, None)?;
            first.sheet.default = true;
            first.spec = spec.clone();
            first.sheet.basis = format!(
                "{} at {}, the smallest automatic scale, as nothing smaller is legible",
                first.sheet.size, first.sheet.scale
            ) + first
                .sheet
                .basis
                .rsplit_once("; drawn-item fill ")
                .map(|(_, f)| format!("; drawn-item fill {f}"))
                .unwrap_or_default()
                .as_str();
        }
        let illegible = first.sheet.scale_factor < min_legible_scale;
        first.sheet.basis += &format!(
            "; no valid complete smallest fit at or above {}; candidates: {}",
            scale_name(min_legible_scale),
            tried.join("; ")
        );
        let below = if illegible {
            format!(
                "; it is below {}, the smallest scale at which its smallest opening stays legible",
                scale_name(min_legible_scale)
            )
        } else {
            String::new()
        };
        first.problems.push(Problem {
            id: "p:layout_recovery_exhausted:sheet".into(),
            code: "layout_recovery_exhausted".into(),
            severity: if illegible { "warning" } else { "info" }.into(),
            message: format!(
                "{} at {} has {}{below}; no standard sheet and legible scale yields a valid complete drawing, so this drawing and its findings are kept. Tried {}.",
                first.sheet.size,
                first.sheet.scale,
                said(findings(&first), drops(&first)),
                tried.join("; ")
            ),
            subjects: vec!["sheet".into()],
            ..Default::default()
        });
        first.status = crate::completeness::status(&first.problems);
        return Ok(first);
    };
    let estimate = format!(
        "first estimate {} at {} ({})",
        first.sheet.size,
        first.sheet.scale,
        if seed_clean {
            "valid and complete"
        } else {
            "not valid and complete"
        }
    );
    let (mut chosen, kept) = match choice {
        Some(index) => (candidates.swap_remove(index).1, ""),
        None => (
            first.clone(),
            "; every allowed pair adds material crossings to the valid first estimate, which is kept",
        ),
    };
    chosen.sheet.basis = format!(
        "smallest fit {} at {}; at most one larger sheet only with enlargement; drawn-item fill {:.1}%, target {:.0}–{:.0}% (validity and completeness first); {estimate}{kept}; candidates: {}",
        layout::PAGES[i].0,
        scale_name(s),
        chosen.sheet.fill * 100.0,
        FILL_BAND[0] * 100.0,
        FILL_BAND[1] * 100.0,
        tried.join("; ")
    );
    chosen.sheet.default = true;
    chosen.spec = first.spec;
    if chosen.sheet.size != first.sheet.size || chosen.sheet.scale != first.sheet.scale {
        chosen.problems.push(Problem {
            id: "p:layout_replanned:sheet".into(),
            code: "layout_replanned".into(),
            severity: "info".into(),
            // The measured fill stays in the sheet basis: an edit that moves it by a fraction
            // must not churn this problem in an edit's delta.
            message: format!(
                "Sheet choice changed from {} at {} to {} at {} by drawn-item fill; the sheet basis lists the candidates.",
                first.sheet.size, first.sheet.scale, chosen.sheet.size, chosen.sheet.scale,
            ),
            subjects: vec!["sheet".into()],
            ..Default::default()
        });
    }
    Ok(chosen)
}

/// [`plan`] with draftwright's bounded layout recovery applied ([`recover`]): the plan every
/// caller (the library's [`crate::api`], so the command line, the JSON-lines service and MCP)
/// is given. A plan that cannot be made is refused as [`plan`] refuses it.
pub fn plan_recovered(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    crate::planning::with(analysis, || crate::sheets::allocate(analysis, spec))
}

fn recover_tables(analysis: &Analysis, spec: &Spec, first: Plan) -> Result<Plan, Invalid> {
    // Establish the ordinary recovered geometry before changing presentation. A table must
    // not undo a scale recovery (nor spend one merely because its ladder has disappeared).
    let mut baseline_spec = spec.clone();
    baseline_spec.hole_table = crate::hole_table::Setting::Never;
    let mut seed = first.spec.clone();
    seed.hole_table = crate::hole_table::Setting::Never;
    let mut baseline = recover(analysis, &baseline_spec, plan_escalated(analysis, &seed)?)?;
    baseline.spec = spec.clone();
    let selection = crate::hole_table::select(analysis, &baseline);
    if selection.tables.is_empty() {
        return Ok(baseline);
    }
    // The table is drawn on the sheet the recovery chose, so it never undoes that choice.
    let replanned = baseline
        .problems
        .iter()
        .any(|p| p.code == "layout_replanned");
    let mut at = spec.clone();
    if replanned {
        at.sheet.size = Some(baseline.sheet.size.clone());
        at.sheet.scale = Some(scale_name(baseline.sheet.scale_factor));
    }
    let mut scheduled =
        plan_with_presentation(analysis, &at, false, replanned, None, Some(&selection))?;
    if replanned {
        scheduled.spec = spec.clone();
        scheduled.sheet.default = true;
    }
    if crate::hole_table::install(analysis, &mut scheduled, &selection) {
        // A genuine geometry recovery still belongs to the final plan; a resolved ladder's
        // exhausted-loss message does not. Preserve the successful recovery's trace.
        scheduled.problems.extend(
            baseline
                .problems
                .iter()
                .filter(|p| p.code == "layout_replanned")
                .cloned(),
        );
        if baseline
            .problems
            .iter()
            .any(|p| p.code == "layout_replanned")
        {
            scheduled.sheet.basis = baseline.sheet.basis.clone()
                + "; hole presentation changed at the recovered geometry";
        }
        crate::completeness::report(analysis, &mut scheduled);
        scheduled.sheet.fill = crate::sheet_fit::fill(analysis, &scheduled);
        Ok(scheduled)
    } else {
        baseline.problems.push(Problem { id:"p:hole_table_unavailable:sheet".into(), code:"hole_table_unavailable".into(), severity:if spec.hole_table == crate::hole_table::Setting::Always { "warning" } else { "info" }.into(), message:"The selected schedule has no clear place at the recovered geometry scale; the original dimensions are kept.".into(), subjects:vec!["sheet".into()], ..Default::default() });
        Ok(baseline)
    }
}

/// [`plan`] with draftwright's drop and escalate applied ([`crate::hole_table`]): a hole callout
/// or location placed off the sheet is dropped and reported, and may escalate to a plan-view hole
/// table with balloons. What the recovery judges, as draftwright's escalation runs inside every
/// build its recovery compares.
///
/// A plan that leaves feature control frames, datum feature symbols or PMI labels off the sheet
/// or over another view is made again with their room reserved beyond every dimension lane of
/// their strips (`layout::Strips::reserve_gdt`'s *beyond_lanes*), and that plan is taken when it
/// leaves fewer: the first reserves within draftwright's estimate where the lane count overstates
/// the dimensions, which on most parts keeps a larger scale, but where the dimensions run on
/// past it an item can find no place (nist_stc_09 e4 at 1:1: 13 frames across the plan).
pub fn plan_escalated(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    plan_escalated_with(analysis, spec, false, None)
}

fn plan_escalated_with(
    analysis: &Analysis,
    spec: &Spec,
    automatic_pair: bool,
    candidate_arrangement: Option<layout::Arrangement>,
) -> Result<Plan, Invalid> {
    let key = serde_json::to_vec(&(
        spec,
        automatic_pair,
        candidate_arrangement.map(|a| a.name()),
    ))
    .expect("candidate inputs serialize");
    crate::planning::memo(analysis, "candidate", key, || {
        plan_escalated_uncached(analysis, spec, automatic_pair, candidate_arrangement)
    })
}

fn plan_escalated_uncached(
    analysis: &Analysis,
    spec: &Spec,
    automatic_pair: bool,
    candidate_arrangement: Option<layout::Arrangement>,
) -> Result<Plan, Invalid> {
    let escalated = |beyond_lanes: bool| {
        plan_with(
            analysis,
            spec,
            beyond_lanes,
            automatic_pair,
            candidate_arrangement,
        )
        .and_then(|mut p| {
            let selection = crate::hole_table::select(analysis, &p);
            if selection.tables.is_empty() && spec.hole_table == crate::hole_table::Setting::Always
                && p.dimensions.iter().any(|d| d.measurements.iter().any(|m| m.parameter == "bore.diameter")) {
                // About the holes, not the sheet (#25): its subjects are the hole callouts kept.
                let callouts = p.dimensions.iter().filter(|d| d.measurements.iter().any(|m| m.parameter == "bore.diameter")).map(|d| d.id.clone()).collect();
                p.problems.push(Problem { id:"p:hole_table_unavailable:sheet".into(), code:"hole_table_unavailable".into(), severity:"warning".into(), message:"A hole table was requested, but no eligible holes have a proven datum corner in a carrying view; the original dimensions are kept.".into(), subjects:callouts, ..Default::default() });
            }
            if !selection.tables.is_empty() {
                let mut scheduled =
                    plan_with_presentation(
                    analysis,
                    spec,
                    beyond_lanes,
                    automatic_pair,
                    candidate_arrangement,
                    Some(&selection),
                )?;
                if crate::hole_table::install(analysis, &mut scheduled, &selection) {
                    p = scheduled;
                } else {
                    p.problems.push(Problem { id:"p:hole_table_unavailable:sheet".into(), code:"hole_table_unavailable".into(), severity:if spec.hole_table == crate::hole_table::Setting::Always { "warning" } else { "info" }.into(), message:"The selected hole table or one of its tags has no clear place; the original dimensions are kept.".into(), subjects:vec!["sheet".into()], ..Default::default() });
                }
            }
            p.table_outcome = crate::hole_table::escalate(analysis, &mut p,
                spec.sheets.as_ref().and_then(|s| s.hole_table).filter(|n| *n > 1));
            crate::completeness::report(analysis, &mut p);
            p.sheet.fill = crate::sheet_fit::fill(analysis, &p);
            if let Some((basis, _)) = p.sheet.basis.rsplit_once("; drawn-item fill ") {
                p.sheet.basis = format!("{basis}; drawn-item fill {:.1}%", p.sheet.fill * 100.0);
            }
            Ok(p)
        })
    };
    let first = escalated(false)?;
    let misplaced = gdt_misplaced(&first);
    if misplaced == (0, 0) {
        return Ok(first);
    }
    let mut again = escalated(true)?;
    if gdt_misplaced(&again) >= misplaced {
        return Ok(first);
    }
    again.sheet.basis += &format!(
        "; the GD&T's room reserved beyond every dimension lane of its strips, as reserved within \
         draftwright's estimate, on {} at {}, {} of its items had no place on the sheet \
         off the other views",
        first.sheet.size,
        first.sheet.scale,
        misplaced.0 + misplaced.1
    );
    Ok(again)
}

/// A plan's feature control frames, datum feature symbols and PMI labels off the sheet or over
/// another view (`annotation_off_sheet`, `annotation_over_view`, `annotation_line_over_view`),
/// at warning or error.
fn gdt_misplaced(plan: &Plan) -> (usize, usize) {
    let ids: BTreeSet<&str> = plan.gdt.iter().map(|g| g.id.as_str()).collect();
    let mut found = (0, 0);
    for p in &plan.problems {
        if p.severity == "info" || !p.subjects.iter().any(|s| ids.contains(s.as_str())) {
            continue;
        }
        match p.code.as_str() {
            "annotation_off_sheet" | "annotation_over_view" | "annotation_line_over_view" => {
                found.0 += 1
            }
            "annotation_ink_overlap" | "annotation_overlap" => found.1 += 1,
            _ => {}
        }
    }
    // Never trade ink off the sheet or over another view for fewer label collisions.
    found
}

/// Plans the sheet: which views, where, at what scale and on what sheet, as draftwright's layout
/// does ([`layout`]): each principal view is a block of its box and the annotation bands reserved
/// around it, packed third-angle; the isometric takes the largest space left. Each choice the
/// spec leaves open is marked as a default, with the reason for it.
///
/// The bands, the turned-part view set and section A–A come from the features
/// ([`crate::demand`]); the height ladder is sized for the steps legible at the scale chosen.
/// The scale and sheet are draftwright's first choice, before it re-plans on its finished
/// drawing's findings. That recovery is [`recover`], applied by [`plan_recovered`], never here:
/// its tries are plans made by this function (ISSUE-1299-a).
pub fn plan(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    plan_with(analysis, spec, false, false, None)
}

/// [`plan`], the GD&T's room reserved beyond every dimension lane of its strips when
/// *gdt_beyond_lanes* ([`crate::demand::Demand::gdt_beyond_lanes`]).
fn plan_with(
    analysis: &Analysis,
    spec: &Spec,
    gdt_beyond_lanes: bool,
    automatic_pair: bool,
    candidate_arrangement: Option<layout::Arrangement>,
) -> Result<Plan, Invalid> {
    plan_with_presentation(
        analysis,
        spec,
        gdt_beyond_lanes,
        automatic_pair,
        candidate_arrangement,
        None,
    )
}

fn plan_with_presentation(
    analysis: &Analysis,
    spec: &Spec,
    gdt_beyond_lanes: bool,
    automatic_pair: bool,
    candidate_arrangement: Option<layout::Arrangement>,
    presentation: Option<&crate::hole_table::Selection>,
) -> Result<Plan, Invalid> {
    let excluded = presentation
        .map(|p| &p.replaced)
        .cloned()
        .unwrap_or_default();
    // A document of a version this build does not know is refused, never read as the nearest
    // one it does (ADR3-4).
    if !ANALYSIS_SCHEMAS.contains(&analysis.schema.as_str()) {
        return Err(invalid(
            format!(
                "The analysis is {:?}, which this draftwright does not read; analyse the STEP file again.",
                analysis.schema
            ),
            &ANALYSIS_SCHEMAS,
        ));
    }
    let spec = &migrate(spec, analysis)?;
    crate::sheets::validate(spec)?;
    crate::dims::check_views(spec)?;
    crate::placement_intent::check(spec)?;
    title_block::check(&spec.sheet.title)?;
    crate::notes::check(&spec.notes)?;
    crate::tables::check(&spec.tables)?;
    let mut problems = Vec::new();
    // The spec the plan follows: across a revision, without the entries that cannot be trusted.
    let canonical = if crate::placement_intent::has_pins(spec) {
        let mut base = Spec::default();
        base.sheet.size = spec.sheet.size.clone();
        base.sheet.scale = spec.sheet.scale.clone();
        base.sheet.projection = spec.sheet.projection.clone();
        base.hole_table = spec.hole_table;
        Some(plan_recovered(analysis, &base)?)
    } else {
        None
    };
    let mut effective = spec.clone();
    if let Some(base) = &canonical {
        effective.sheet.size = Some(base.sheet.size.clone());
        effective.sheet.scale = Some(base.sheet.scale.clone());
        for (id, d) in &mut effective.dimensions {
            if let Some(p) = d.place.as_mut()
                && p.pinned
                && p.view.is_none()
            {
                p.view = d.view.clone().or_else(|| {
                    crate::placement_intent::carrier(base, id)
                        .map(|v| v.trim_start_matches("v:").into())
                });
            }
        }
        for (id, d) in &mut effective.annotations {
            if let Some(p) = d.place.as_mut()
                && p.pinned
                && p.view.is_none()
            {
                p.view = base
                    .gdt
                    .iter()
                    .find(|d| d.id == *id)
                    .map(|d| d.view.trim_start_matches("v:").into());
            }
        }
    }
    let mut inactive = Vec::new();
    if let Some(b) = &spec.binding
        && b.source_sha256 != analysis.binding.source_sha256
    {
        // Until rebinding (ADR R3) lands, a feature's ID is rank-based and may name another
        // feature in this revision: its entries are kept in the spec but inactive (R3 invariant
        // 3), and listed. The sheet, the views and the envelope's dimensions carry over.
        let inactive_entries: Vec<(&str, &String)> = spec
            .dimensions
            .keys()
            .filter(|id| id.starts_with("d:f:") || id.starts_with("d:pmi:"))
            .map(|id| ("dimensions", id))
            .chain(spec.features.keys().map(|id| ("features", id)))
            .chain(spec.annotations.keys().map(|id| ("annotations", id)))
            .collect();
        inactive = inactive_entries
            .iter()
            .map(|(_, id)| id.to_string())
            .collect::<Vec<_>>();
        effective
            .dimensions
            .retain(|id, _| !id.starts_with("d:f:") && !id.starts_with("d:pmi:"));
        effective.annotations.clear();
        effective.features.clear();
        problems.push(Problem {
            id: "p:rebind:part".into(),
            code: "spec_from_other_revision".into(),
            severity: "info".into(),
            message: format!(
                "The spec was made for a different revision of {}; its sheet, view and envelope dimension choices are carried over.",
                b.part
            ),
            subjects: vec![],
            ..Default::default()
        });
        if !inactive.is_empty() {
            problems.push(Problem {
                id: "p:spec_entries_inactive:part".into(),
                code: "spec_entries_inactive".into(),
                severity: "warning".into(),
                message: format!(
                    "The spec was made for a different revision of {}, and its {} entr{} for features and PMI (this problem's refs) cannot be matched to this one's until revisions are rebound: they are kept in the spec but not applied, and the spec stays bound to its revision while they remain. Remove them (then set what is wanted by this revision's IDs), or edit the spec's binding to this revision's to apply them as they are.",
                    b.part,
                    inactive.len(),
                    if inactive.len() == 1 { "y" } else { "ies" }
                ),
                subjects: Vec::new(),
                refs: inactive.clone(),
                // The message's first remedy: the entries removed, the spec binds to this
                // revision.
                fix: inactive_entries
                    .iter()
                    .map(|(at, id)| {
                        crate::items::Op::remove(format!("{at}/{}", crate::items::key(id)))
                    })
                    .collect(),
            });
        }
    }
    // An assembly drawn whole (no part chosen): its parts are drawn together as one, as they
    // always were (the maintainer's D4), and the plan says how to draw one alone.
    if !analysis.parts.is_empty() && analysis.binding.part_index.is_none() {
        problems.push(Problem {
            id: "p:assembly_drawn_whole:part".into(),
            code: "assembly_drawn_whole".into(),
            severity: "warning".into(),
            message: format!(
                "{} is an assembly of {} parts ({}); they are drawn together as one. To draw one part alone, choose it by its index: --part K on the command line, or \"part\": K in a request.",
                analysis.binding.part,
                analysis.parts.len(),
                part_list(&analysis.parts)
            ),
            subjects: Vec::new(),
            refs: analysis.parts.iter().map(|p| p.name.clone()).collect(),
            ..Default::default()
        });
    }
    if !analysis.unrecognised.is_empty() {
        let named = analysis
            .unrecognised
            .iter()
            .map(|u| match u.records {
                Some(n) => format!("{} ({n} on this part)", u.family),
                None => format!("{} (not in its document)", u.family),
            })
            .collect::<Vec<_>>();
        let recogniser = analysis
            .recognition
            .as_ref()
            .map_or("the recogniser", |r| r.recogniser.as_str());
        problems.push(Problem {
            id: "p:features_not_recognised:part".into(),
            code: "features_not_recognised".into(),
            severity: "warning".into(),
            message: format!(
                "The features come from {recogniser}, which does not give these families: {}. Features of them are not dimensioned, sectioned or linted; Python quiddity's recognition (tools/recognise.py) gives them.",
                named.join(", ")
            ),
            subjects: Vec::new(),
            refs: analysis
                .unrecognised
                .iter()
                .map(|u| u.family.clone())
                .collect(),
            ..Default::default()
        });
    }
    if !effective.annotations.is_empty() {
        let mut annotation_ids: Vec<String> = crate::pmi_lowering::lower(analysis, &|_| true)
            .0
            .into_iter()
            .map(|i| i.id)
            .collect();
        annotation_ids.extend(crate::dims::blends::note_ids(analysis));
        for id in effective.annotations.keys() {
            if !annotation_ids.contains(id) {
                return Err(Invalid {
                    message: format!("There is no annotation {id:?} on this part."),
                    choices: annotation_ids.clone(),
                });
            }
        }
    }
    let given = spec;
    let spec = &effective;
    for name in spec.views.keys() {
        if name != SECTION && !VIEWS.iter().any(|(v, ..)| v == name) {
            let mut names = VIEWS.map(|v| v.0).to_vec();
            names.push(SECTION);
            return Err(invalid(
                format!("There is no view called {name:?}."),
                &names,
            ));
        }
    }
    let mut demand = crate::demand::read(analysis);
    demand.gdt_beyond_lanes = gdt_beyond_lanes;
    // A turned part drops the view that repeats its profile, unless the spec asks for it, or
    // omits the profile view it keeps (front): the dropped view is then the only profile left;
    // or a PMI item is drawn in it (draftwright's `annotation_view_required`).
    let omitted = |name: &str| spec.views.get(name).is_some_and(|v| v.omit);
    let pmi_views = crate::pmi_lowering::views(analysis);
    let reduction = crate::demand::turned_views(&demand).filter(|(kept, dropped, _)| {
        !spec.views.contains_key(*dropped) && !omitted(kept[0]) && !pmi_views.contains(dropped)
    });
    let shown = |name: &str| match spec.views.get(name) {
        Some(v) => !v.omit,
        None => !reduction.as_ref().is_some_and(|r| r.1 == name),
    };
    let principals = Principals {
        front: shown("front"),
        plan: shown("plan"),
        side: shown("side"),
    };
    // The bands hold the lanes the dimensions will take in the views shown (review M3).
    let shown_views: Vec<(&str, [f64; 3], [f64; 3])> = VIEWS
        .iter()
        .filter(|(name, ..)| shown(name))
        .copied()
        .collect();
    demand.lanes = crate::dims::lanes_excluding(analysis, &shown_views, &excluded);
    if !(principals.front || principals.plan || principals.side) {
        return Err(invalid(
            "Every principal view is omitted; a sheet needs front, plan or side.",
            &["front", "plan", "side"],
        ));
    }
    let page = match &spec.sheet.size {
        Some(s) => Some(
            layout::PAGES
                .iter()
                .find(|(n, ..)| n == s)
                .map(|(_, w, h)| [*w, *h])
                .ok_or_else(|| {
                    invalid(
                        format!("{s:?} is not a sheet size."),
                        &layout::PAGES.map(|p| p.0),
                    )
                })?,
        ),
        None => None,
    };
    let scale = match &spec.sheet.scale {
        Some(s) => Some(
            layout::SCALES
                .iter()
                .copied()
                .find(|k| scale_name(*k) == *s)
                .ok_or_else(|| {
                    let names = layout::SCALES.map(scale_name);
                    invalid(
                        format!("{s:?} is not a standard scale."),
                        &names.iter().map(String::as_str).collect::<Vec<_>>(),
                    )
                })?,
        ),
        None => None,
    };
    // ISO 5456-2's two methods: third angle (draftwright's) unless the spec says first.
    let first_angle = match spec.sheet.projection.as_deref() {
        None | Some("third") => false,
        Some("first") => true,
        Some(p) => {
            return Err(invalid(
                format!("{p:?} is not a projection method."),
                &PROJECTIONS,
            ));
        }
    };

    let page = page.or_else(|| presentation.map(|p| p.page));
    // A schedule removes annotation demand; it must not shrink or enlarge the proven feature ink.
    let scale = presentation.map(|p| p.scale).or(scale);
    let [lo, hi] = analysis.envelope;
    // Section A-A is marked on the plan and sits in the front's row.
    let section = demand
        .section
        .clone()
        .filter(|_| shown(SECTION) && principals.plan && (principals.front || principals.side));
    // A section left out is said, with why and what would bring it back (draftwright records
    // every section decision: ISSUE-1190-b); one the author omits needs no saying.
    let section_not_shown = |why: String, fix: Vec<crate::items::Op>| Problem {
        id: format!("p:section_not_shown:v:{SECTION}"),
        code: "section_not_shown".into(),
        severity: "warning".into(),
        message: format!("Section A\u{2013}A is not drawn: {why}."),
        subjects: Vec::new(),
        refs: vec![format!("v:{SECTION}")],
        fix,
    };
    match &demand.section {
        Some(_) if section.is_none() && shown(SECTION) => problems.push(if !principals.plan {
            let (how, fix) = if omitted("plan") {
                (
                    "the spec omits it; remove views/plan/omit to show it",
                    vec![crate::items::Op::remove("views/plan/omit")],
                )
            } else {
                (
                    "it is left out as the turned part's repeated profile; set views/plan to {} to keep it",
                    Vec::new(),
                )
            };
            section_not_shown(
                format!(
                    "a z-axis hole with a counterbore, spotface or bottom needs it, but its cutting plane is marked on the plan view, and {how}"
                ),
                fix,
            )
        } else {
            section_not_shown(
                "it sits in the row of the front and side views, and the spec omits both; remove views/front/omit or views/side/omit to show it".into(),
                vec![crate::items::Op::remove("views/front/omit")],
            )
        }),
        None if spec.views.get(SECTION).is_some_and(|v| !v.omit) => problems.push(section_not_shown(
            "the spec asks for it, but the part has no z-axis hole with a counterbore, spotface or bottom whose profile it would show, so there is no plane to cut; remove views/section_aa".into(),
            vec![crate::items::Op::remove(format!("views/{SECTION}"))],
        )),
        _ => {}
    }
    let mut request = Request::new(
        [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]],
        principals,
        shown("iso"),
    );
    request.automatic_pair = automatic_pair;
    request.candidate_arrangement = candidate_arrangement;
    request.first_angle = first_angle;
    request.sections = usize::from(section.is_some());
    // The tables are reserved towards their corners and the general notes block top right, in
    // draftwright's order, unless only a fallback would make room for them: then each is placed
    // where it can be, or reported (crate::tables, crate::notes).
    // The part's drawing-wide PMI requirements (crate::pmi_lowering): its edge conditions are
    // general notes after the spec's.
    let (requirements, more) = crate::pmi_lowering::sheet_requirements(analysis);
    problems.extend(more);
    let mut notes_given = spec.notes.clone();
    notes_given.extend(
        requirements
            .iter()
            .filter(|r| r.kind == "document_note")
            .map(|r| NoteSpec {
                id: r.id.clone(),
                text: r.value.clone(),
            }),
    );
    let mut radius_notes = crate::dims::blends::notes(analysis);
    if spec.dimension_source == DimensionSource::Authored {
        radius_notes.retain(|(n, _)| spec.annotations.contains_key(&n.id));
    }
    notes_given.extend(
        radius_notes
            .iter()
            .filter(|(n, _)| {
                !(spec.dimension_source == DimensionSource::Authored
                    && !spec.annotations.contains_key(&n.id))
                    && !spec.annotations.get(&n.id).is_some_and(|s| {
                        s.omit
                            || s.place.as_ref().is_some_and(|p| {
                                p.side.is_some() || p.lane.is_some() || p.along.is_some()
                            })
                    })
            })
            .map(|(n, _)| n.clone()),
    );
    let notes_size = crate::notes::size(
        &notes_given
            .iter()
            .filter(|n| !radius_notes.iter().any(|(r, _)| r.id == n.id))
            .cloned()
            .collect::<Vec<_>>(),
    );
    request.required_tables = crate::tables::sizes(&spec.tables);
    if let Some(p) = presentation {
        request.required_tables.extend(p.sizes());
    }
    // The part's manufacturing requirements table, when its threads and knurls are referred to
    // one (crate::pmi_lowering::manufacturing), before the notes, as draftwright reserves it.
    request.required_tables.extend(
        crate::tables::manufacturing_size(&crate::pmi_lowering::manufacturing(analysis).rows())
            .map(|s| (s, Corner::TopRight)),
    );
    request
        .required_tables
        .extend(notes_size.map(|s| (s, Corner::TopRight)));
    // The height ladder is sized for the steps legible at the scale it leads to.
    let (mut pick, mut strips, _) = crate::demand::size(&demand, &request, scale, page, lo[2]);
    if !request.required_tables.is_empty() && pick.fallback.is_some() {
        let mut bare = request.clone();
        bare.required_tables.clear();
        let (p, s, _) = crate::demand::size(&demand, &bare, scale, page, lo[2]);
        if p.fallback.is_none() {
            (request, pick, strips) = (bare, p, s);
        }
    }
    request.strips = strips;
    let page_name = |page: [f64; 2]| {
        layout::PAGES
            .iter()
            .find(|(_, w, h)| [*w, *h] == page)
            .map_or_else(|| format!("{}x{}", page[0], page[1]), |p| p.0.into())
    };
    let size = page_name(pick.page);
    let scale = scale_name(pick.scale);
    match pick.fallback {
        Some(Fallback::Computed) => problems.push(Problem {
            id: "p:scale_fallback_applied:sheet".into(),
            code: "scale_fallback_applied".into(),
            severity: "warning".into(),
            message: format!(
                "No standard scale fits the views on {size}; the largest that fits, {scale}, is used."
            ),
            subjects: vec!["sheet".into()],
            ..Default::default()
        }),
        Some(Fallback::Overflow) => problems.push(Problem {
            id: "p:views_do_not_fit:sheet".into(),
            code: "views_do_not_fit".into(),
            severity: "error".into(),
            message: format!("The views do not fit {size} at {scale}."),
            subjects: vec!["sheet".into()],
            ..Default::default()
        }),
        Some(Fallback::Uncertain) => problems.push(Problem {
            id: "p:page_fit_uncertain:sheet".into(),
            code: "page_fit_uncertain".into(),
            severity: "warning".into(),
            message: format!("The views may not fit {size} at {scale}, as the spec asks."),
            subjects: vec!["sheet".into()],
            ..Default::default()
        }),
        None => {}
    }
    // A fallback is said as what it is, never as the rule that found nothing.
    let basis = match (pick.fallback, &spec.sheet.size, &spec.sheet.scale) {
        (Some(Fallback::Computed), None, _) => format!(
            "past draftwright's ladder, as no sheet and standard scale fits the views and their annotation bands: the largest scale that fits the largest sheet, {size}, found by bisection: {scale}, not a standard scale"
        ),
        (Some(Fallback::Computed), Some(_), _) => format!(
            "no standard scale fits the views on {size}: the largest scale that fits, found by bisection: {scale}, not a standard scale"
        ),
        (Some(Fallback::Overflow), _, None) => format!(
            "nothing fits: no scale, standard or computed, fits the views on {size}; {size} at {scale} is used, and the views do not fit"
        ),
        (Some(Fallback::Overflow), _, Some(_)) => format!(
            "no sheet holds the views at {scale}: the largest, {size}, is used, and the views do not fit"
        ),
        (Some(Fallback::Uncertain), ..) => "as the spec asks; the views may not fit".into(),
        (None, None, None) => format!(
            "the first sheet and scale on draftwright's ladder (every enlargement on the smallest sheet first, then reductions) whose views and the annotation bands their features need fit: {size} at {scale}"
        ),
        (None, Some(_), None) => {
            format!("the largest standard scale at which the views fit {size}")
        }
        (None, None, Some(_)) => format!("the smallest sheet that holds the views at {scale}"),
        (None, Some(_), Some(_)) => "as the spec asks".into(),
    };

    let g = layout::geometry(
        &request,
        &Scaled::default(),
        pick.scale,
        pick.page,
        pick.tb_w,
        pick.arrangement,
        None,
    );
    let ib = analysis.iso_bounds.map(|b| b * pick.scale);
    let iso_factor = layout::iso_factor(g.iso, g.iso_centre, ib);
    if shown("iso") && analysis.iso_bounds == [0.0; 4] {
        problems.push(Problem {
            id: "p:iso_projection_empty:v:iso".into(),
            code: "iso_projection_empty".into(),
            severity: "warning".into(),
            message: "The isometric projection of the part has no edges, so the isometric cannot be fitted to its space: it is planned at the sheet's scale and draws nothing. Omit it with views/iso/omit.".into(),
            subjects: vec!["v:iso".into()],
            fix: vec![crate::items::Op::set("views/iso/omit", serde_json::json!(true))],
            ..Default::default()
        });
    }
    let turned = |name: &str| match &reduction {
        Some((kept, dropped, why)) if kept.contains(&name) => {
            format!("; {why}, so the {dropped} view is left out")
        }
        _ => String::new(),
    };
    let mut views: Vec<PlannedView> = VIEWS
        .iter()
        .filter(|(name, ..)| shown(name))
        .map(|(name, direction, up)| {
            let [fv, pv, sv] = g.blocks;
            let (at, area, scale, basis) = match *name {
                "front" => (
                    g.front,
                    fv.footprint(g.front[0], g.front[1]),
                    pick.scale,
                    "the principal view, its box and annotation bands packed from the lower left"
                        .to_string(),
                ),
                "plan" => (
                    g.plan,
                    pv.footprint(g.plan[0], g.plan[1]),
                    pick.scale,
                    if first_angle {
                        "first-angle: the plan sits below the front, past both views' bands"
                    } else {
                        "third-angle: the plan sits above the front, past both views' bands"
                    }
                    .into(),
                ),
                "side" => (
                    g.side,
                    sv.footprint(g.side[0], g.side[1]),
                    pick.scale,
                    if first_angle {
                        "first-angle: the right side sits left of the front, past both views' bands"
                    } else {
                        "third-angle: the right side sits right of the front, past both views' bands"
                    }
                    .into(),
                ),
                _ => (
                    g.iso_centre,
                    g.iso,
                    pick.scale * iso_factor,
                    if iso_factor == 1.0 {
                        "the isometric, centred in the largest space the other views leave".into()
                    } else {
                        format!(
                            "the isometric, centred in the largest space the other views leave and scaled ×{iso_factor} to fill it (not to scale)"
                        )
                    },
                ),
            };
            PlannedView {
                sheet: 1, scale_note: None, xref: None,
                id: format!("v:{name}"),
                direction: *direction,
                up: *up,
                at,
                scale,
                area,
                default: !spec.views.contains_key(*name),
                basis: basis + &turned(name),
                state: ItemState::Shown,
                cut: None,
                line_work: Vec::new(),
            }
        })
        .collect();
    if let (Some(cut), Some(area)) = (&section, g.section_boxes.first()) {
        views.push(PlannedView {
            sheet: 1, scale_note: None, xref: None,
            id: format!("v:{SECTION}"),
            direction: [0.0, -1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            at: g.section,
            scale: pick.scale,
            area: *area,
            default: !spec.views.contains_key(SECTION),
            basis: format!(
                "draftwright's section rule: a z-axis hole with a counterbore, spotface or bottom shows its profile in no standard view; the plane y = {} cuts the densest row of them ({} on it), seen from the front and placed right of the side view",
                fmt_ratio(cut.cut_y),
                cut.holes
            ),
            state: ItemState::Shown,
            line_work: Vec::new(),
            cut: Some(Cut {
                xref: None,
                label: "A".into(),
                origin: [0.5 * (lo[0] + hi[0]), cut.cut_y, 0.5 * (lo[2] + hi[2])],
                normal: [0.0, 1.0, 0.0],
                marked_in: "v:plan".into(),
            }),
        });
    } else if section.is_some() {
        problems.push(section_not_shown(
            "the layout made no place for it on the sheet".into(),
            Vec::new(),
        ));
    }
    let mut title = spec.sheet.title.clone();
    title
        .entry("title".into())
        .or_insert_with(|| analysis.binding.part.to_uppercase());
    title
        .entry("number".into())
        .or_insert_with(|| "DWG-001".into());
    // The file's one usable material, unless the spec states one, even blank (draftwright's
    // `_resolve_title_document_defaults`).
    if let Some(crate::pmi::Reading::Read(r)) = &analysis.pmi
        && let Some(m) = r.title_material()
    {
        if let Some(v) = spec.sheet.title.get("material")
            && v.trim() != m.designation.trim()
        {
            problems.push(Problem {
                id: format!("p:step_material_disagreement:{}", m.source_id),
                code: "step_material_disagreement".into(),
                severity: "warning".into(),
                message: format!(
                    "The title block uses the Spec's material {v:?} (sheet/title/material), which wins; the STEP material {} states {:?}. The two values disagree.",
                    m.source_id, m.designation
                ),
                subjects: vec!["sheet".into()],
                refs: vec![m.source_id.clone(), "sheet/title/material".into()],
                ..Default::default()
            });
        }
        title
            .entry("material".into())
            .or_insert_with(|| m.designation.trim().to_string());
    }
    let (mut requirements, more) = crate::pmi_lowering::plan_sheet(
        &requirements,
        &spec.sheet.title,
        &mut title,
        g.title_block,
    );
    problems.extend(more);
    let title_cells = title_block::cells(g.title_block, &title, &size);
    problems.extend(title_block::problems(&title_cells, pick.page));
    let mut sheet = PlannedSheet {
        number: 1,
        zones: None,
        fill: 0.0,
        size,
        page: pick.page,
        scale,
        scale_factor: pick.scale,
        arrangement: pick.arrangement.name().into(),
        title_block: g.title_block,
        default: given.sheet.size.is_none() || given.sheet.scale.is_none(),
        basis,
        state: ItemState::Shown,
        title,
        title_cells,
        projection_symbol: Some(title_block::projection(g.title_block, first_angle)),
    };
    if let Some(base) = &canonical {
        sheet.page = base.sheet.page;
        sheet.scale_factor = base.sheet.scale_factor;
        sheet.title_block = base.sheet.title_block;
        sheet.basis = base.sheet.basis.clone()
            + "; anchor frames and sheet choice kept fixed for placement pins";
        sheet.title_cells = title_block::cells(sheet.title_block, &sheet.title, &sheet.size);
        sheet.projection_symbol = Some(title_block::projection(sheet.title_block, first_angle));
    }
    if canonical.is_some() || spec.views.values().any(|v| v.place.is_some()) {
        crate::placement_intent::views(
            spec,
            &mut views,
            canonical.as_ref(),
            &sheet,
            &[],
            &mut problems,
        );
    }
    if let Some(presentation) = presentation
        && let Some(base) = &presentation.iso
        && spec.views.get("iso").is_none_or(|v| v.place.is_none())
        && let Some(v) = views.iter_mut().find(|v| v.id == "v:iso")
    {
        v.scale = base.scale;
        let outside = (0..2).any(|k| {
            v.at[k] + v.scale * analysis.iso_bounds[k] < MARGIN
                || v.at[k] + v.scale * analysis.iso_bounds[k + 2] > sheet.page[k] - MARGIN
        });
        // Bare pins retain their canonical frame. Holding an automatic scale must not leave
        // its newly packed centre outside the page; in that case retain the proven frame too.
        if let Some(base) = canonical
            .as_ref()
            .and_then(|p| p.views.iter().find(|v| v.id == "v:iso"))
        {
            (v.at, v.area, v.scale) = (base.at, base.area, base.scale);
        } else if outside {
            (v.at, v.area, v.scale) = (base.at, base.area, base.scale);
        }
    }
    crate::dims::gdt::validate_placements(analysis, spec, &views)?;
    // Different arrangement requests can resolve to identical geometry. Solve that ink once,
    // retaining each request's sheet and view explanations in its returned Plan.
    let reusable = canonical.is_none()
        && crate::planning::enabled(analysis)
        && !crate::placement_intent::has_pins(spec);
    let key = if reusable {
        let mut geometry_sheet = sheet.clone();
        geometry_sheet.basis.clear();
        geometry_sheet.arrangement.clear();
        let mut geometry_views = views.clone();
        for view in &mut geometry_views {
            view.basis.clear();
        }
        serde_json::to_vec(&(
            spec,
            given,
            &geometry_sheet,
            &geometry_views,
            &problems,
            &excluded,
            &requirements,
            &notes_given,
            &radius_notes,
            &inactive,
            presentation.is_some(),
        ))
        .expect("resolved candidate inputs serialize")
    } else {
        Vec::new()
    };
    let sheet_basis = sheet.basis.clone();
    let arrangement = sheet.arrangement.clone();
    let view_bases: Vec<_> = views.iter().map(|v| v.basis.clone()).collect();
    if reusable
        && let Some(mut plan) =
            crate::planning::lookup::<Plan>(analysis, "resolved_candidate", &key)
    {
        plan.sheet.basis = format!(
            "{sheet_basis}; drawn-item fill {:.1}%",
            plan.sheet.fill * 100.0
        );
        plan.sheet.arrangement = arrangement;
        for (view, basis) in plan.views.iter_mut().zip(view_bases) {
            view.basis = basis;
        }
        return Ok(plan);
    }
    let footprint_cache = crate::dims::footprints::Cache::default();
    let held = crate::placement_intent::held_dimensions(canonical.as_ref(), &views);
    let mut fixed_spec = spec.clone();
    fixed_spec
        .annotations
        .retain(|_, a| a.place.as_ref().is_some_and(|p| p.pinned));
    let fixed = if fixed_spec.annotations.is_empty() {
        Vec::new()
    } else {
        crate::dims::gdt::plan_anchored(
            analysis,
            &fixed_spec,
            &sheet,
            &views,
            &[],
            canonical.as_ref(),
            &footprint_cache,
        )
        .0
        .iter()
        .filter(|g| fixed_spec.annotations.contains_key(&g.id))
        .filter_map(crate::dims::gdt::proxy)
        .collect()
    };
    crate::sheets::line_work(analysis, spec, &mut views);
    let mut furniture = crate::centrelines::obstacles(&views);
    furniture.extend(fixed.iter().cloned());
    let ((mut dimensions, mut omitted, mut more), mut inventory) = crate::dims::plan_inventory(
        analysis,
        spec,
        &sheet,
        &views,
        &furniture,
        &held,
        &footprint_cache,
        &excluded,
    )?;
    // A bare pin whose ink is already settled beside a schedule adds no packing obstacle.
    // Repacking the isometric around its reserved band can consume the table's clear space.
    // Changed ink and explicit placements still reserve their full obstacle as before.
    let packing_dimensions: Vec<_> = dimensions
        .iter()
        .filter(|d| {
            !(presentation.is_some()
                && spec
                    .dimensions
                    .get(&d.id)
                    .and_then(|e| e.place.as_ref())
                    .is_some_and(crate::placement_intent::automatic_pin)
                && canonical.as_ref().is_some_and(|base| {
                    base.dimensions
                        .iter()
                        .any(|before| before.id == d.id && before.placed == d.placed)
                }))
        })
        .cloned()
        .collect();
    let obstacles = crate::placement_intent::obstacles(spec, &packing_dimensions, &fixed);
    if !obstacles.is_empty() {
        let before: Vec<_> = views.iter().map(|v| (v.at, v.area, v.scale)).collect();
        problems.retain(|p| {
            !matches!(
                p.code.as_str(),
                "pinned_view_overlap" | "pinned_view_off_sheet" | "placement_anchor_not_shown"
            )
        });
        // Reserve explicit pin ink before packing the free views around it. A pin's carrier
        // keeps its origin and scale; the additional band is a real packing obstacle.
        let mut reserved = canonical.clone();
        for d in dimensions.iter().chain(&fixed) {
            let place = spec
                .dimensions
                .get(&d.id)
                .and_then(|d| d.place.as_ref())
                .or_else(|| spec.annotations.get(&d.id).and_then(|a| a.place.as_ref()));
            if place.is_some_and(|p| p.pinned && !crate::placement_intent::automatic_pin(p))
                && let Some(b) = crate::dims::ink_box(d)
            {
                for v in views
                    .iter_mut()
                    .chain(reserved.iter_mut().flat_map(|p| &mut p.views))
                    .filter(|v| v.id == d.view)
                {
                    v.area = [
                        v.area[0].min(b[0]),
                        v.area[1].min(b[1]),
                        v.area[2].max(b[2]),
                        v.area[3].max(b[3]),
                    ];
                }
            }
        }
        crate::placement_intent::views(
            spec,
            &mut views,
            reserved.as_ref(),
            &sheet,
            &obstacles,
            &mut problems,
        );
        if before
            != views
                .iter()
                .map(|v| (v.at, v.area, v.scale))
                .collect::<Vec<_>>()
        {
            crate::sheets::line_work(analysis, spec, &mut views);
            furniture = crate::centrelines::obstacles(&views);
            furniture.extend(fixed.iter().cloned());
            let held = crate::placement_intent::held_dimensions(canonical.as_ref(), &views);
            ((dimensions, omitted, more), inventory) = crate::dims::plan_inventory(
                analysis,
                spec,
                &sheet,
                &views,
                &furniture,
                &held,
                &footprint_cache,
                &excluded,
            )?;
        }
    }
    problems.extend(more);
    // A label that had to stay across an axis end shortens that axis's overrun; what follows
    // keeps clear of the trimmed furniture.
    crate::centrelines::trim(&mut views, &dimensions);
    furniture = crate::centrelines::obstacles(&views);
    furniture.extend(fixed.iter().cloned());
    // The part's PMI, placed beyond the dimensions; the notes and tables keep clear of both.
    let mut annotation_obstacles = dimensions.clone();
    let furniture_ink: Vec<_> = furniture.into_iter().filter(|d| d.kind == "ink").collect();
    annotation_obstacles.extend(furniture_ink.iter().cloned());
    let (gdt, more) = crate::dims::gdt::plan_anchored(
        analysis,
        spec,
        &sheet,
        &views,
        &annotation_obstacles,
        canonical.as_ref(),
        &footprint_cache,
    );
    problems.extend(more);
    // Settle labels after GD&T has claimed its corridors. Frames and datum attachments remain
    // fixed obstacles; the full dimension inventory preserves omission locality.
    for d in &mut inventory {
        if let Some(shown) = dimensions.iter().find(|shown| shown.id == d.id) {
            *d = shown.clone();
        }
    }
    let reserved: Vec<_> = gdt.iter().filter_map(crate::dims::gdt::proxy).collect();
    let mut label_spec = spec.clone();
    if spec.dimensions.values().any(|d| d.tolerance.is_some())
        || spec.features.values().any(|f| f.fit.is_some())
    {
        // Text edits retain unaffected peers' canonical settlement, just as lane assignment
        // does. Fix peers on identical witnesses before solving the edited/restored items.
        let base = Spec::default();
        let (_, mut natural) = crate::dims::plan_inventory(
            analysis,
            &base,
            &sheet,
            &views,
            &[],
            &[],
            &footprint_cache,
            &excluded,
        )?;
        let before = natural.clone();
        crate::dims::settle_inventory(analysis, &base, &sheet, &views, &mut natural, &reserved);
        for d in &mut inventory {
            if spec
                .dimensions
                .get(&d.id)
                .is_some_and(|e| e.place.is_some())
            {
                continue;
            }
            let original = before.iter().find(|b| {
                b.id == d.id
                    && b.terms == d.terms
                    && b.view == d.view
                    && b.placed
                        .as_ref()
                        .zip(d.placed.as_ref())
                        .is_some_and(|(a, b)| {
                            a.arrows == b.arrows
                                && a.lines.iter().take(2).map(|l| l[0]).eq(b
                                    .lines
                                    .iter()
                                    .take(2)
                                    .map(|l| l[0]))
                        })
            });
            let settled = natural.iter().find(|b| b.id == d.id);
            if original.is_some()
                && let (Some(_), Some(c)) =
                    (d.placed.as_ref(), settled.and_then(|d| d.placed.as_ref()))
            {
                label_spec
                    .dimensions
                    .entry(d.id.clone())
                    .or_default()
                    .place
                    .get_or_insert_with(Default::default)
                    .pinned = true;
                d.placed = Some(c.clone());
            }
        }
    }
    // Settled labels also keep clear of the (trimmed) centre-line furniture: a move never adds
    // a contact with a centre or cutting-plane stroke.
    let mut settle_fixed = reserved.clone();
    settle_fixed.extend(furniture_ink);
    crate::dims::settle_inventory(
        analysis,
        &label_spec,
        &sheet,
        &views,
        &mut inventory,
        &settle_fixed,
    );
    for d in &mut dimensions {
        if let Some(settled) = inventory.iter().find(|settled| settled.id == d.id) {
            d.placed = settled.placed.clone();
        }
    }
    let settled_ids: BTreeSet<&str> = dimensions
        .iter()
        .chain(&reserved)
        .map(|d| d.id.as_str())
        .collect();
    problems.retain(|p| {
        !matches!(
            p.code.as_str(),
            "annotation_outside_area"
                | "annotation_off_sheet"
                | "annotation_over_title_block"
                | "annotation_over_view"
                | "annotation_line_over_view"
                | "annotation_overlap"
                | "annotation_ink_overlap"
                | "annotation_leader_crossing"
                | "leader_through_feature"
                | "extension_through_feature"
        ) || !p
            .subjects
            .iter()
            .any(|id| settled_ids.contains(id.as_str()))
    });
    let mut ink = dimensions.clone();
    ink.extend(reserved);
    problems.extend(crate::dims::label_collisions(
        analysis,
        &ink,
        &sheet,
        &views,
        &footprint_cache,
    ));
    crate::dims::blends::consolidate(&mut dimensions, spec, given);
    crate::dims::blends::clear_covered_findings(&dimensions, &mut problems);
    let radius_table = crate::dims::blends::tabulate_obstructed(
        analysis,
        spec,
        &sheet,
        &views,
        &mut dimensions,
        &gdt,
        &mut problems,
    );
    for d in dimensions
        .iter()
        .filter(|d| matches!(&d.state, crate::items::State::Covered { by } if by.starts_with("n:") || (by.starts_with("d:") && d.measurements.iter().any(|m| m.parameter == "blend.radius"))))
    {
        let crate::items::State::Covered { by } = &d.state else {
            unreachable!()
        };
        omitted.push(crate::dims::Omitted {
            id: d.id.clone(),
            label: d.label.clone(),
            basis: format!("conveyed by {by}: {}", d.basis),
            state: d.state.clone(),
        });
    }
    dimensions.retain(|d| !matches!(&d.state, ItemState::Covered { by } if by.starts_with("n:") || (by.starts_with("d:") && d.measurements.iter().any(|m| m.parameter == "blend.radius"))));
    let mut ink = dimensions.clone();
    ink.extend(
        annotation_obstacles
            .iter()
            .filter(|d| d.kind == "ink")
            .cloned(),
    );
    ink.extend(gdt.iter().filter_map(crate::dims::gdt::proxy));
    if let Some(table) = &radius_table {
        ink.extend(crate::tables::proxy(&table.table));
    }
    // The default finish's symbol beside the title block: what it meets, and kept clear of.
    for r in &requirements {
        problems.extend(crate::dims::gdt::finish_collisions_cached(
            analysis,
            &sheet,
            &views,
            &ink,
            r,
            &footprint_cache,
        ));
        ink.extend(crate::dims::gdt::finish_proxy(r));
    }
    // The manufacturing requirements table goes before the notes, which keep clear of it.
    let (schedule, more) = crate::tables::manufacturing(analysis, &sheet, &views, &ink);
    problems.extend(more);
    ink.extend(schedule.iter().filter_map(crate::tables::proxy));
    let (mut automatic, authored): (Vec<_>, Vec<_>) = notes_given
        .into_iter()
        .partition(|n| radius_notes.iter().any(|(r, _)| r.id == n.id));
    let (mut notes, mut notes_block, more) =
        crate::notes::plan(&authored, analysis, &sheet, &views, &ink);
    problems.extend(more);
    // Authored tables keep their established priority; automatic radii use the space left.
    let (mut tables, more) = crate::tables::plan(
        &spec.tables,
        analysis,
        &sheet,
        &views,
        &ink,
        notes_block.as_ref().map(|b| b.cells),
    );
    problems.extend(more);
    tables.extend(schedule);
    ink.extend(tables.iter().filter_map(crate::tables::proxy));
    let exceptions_unstated =
        crate::dims::blends::exceptions_unstated(analysis, &dimensions, &radius_notes);
    if exceptions_unstated {
        automatic.retain(|n| n.id == crate::dims::blends::RELIEF_NOTE);
    }
    let (extra, block, mut more) = crate::notes::plan_radii(
        &automatic,
        analysis,
        &sheet,
        &views,
        &ink,
        notes_block.as_ref(),
    );
    if !authored.is_empty() {
        for p in &mut more {
            p.id += ":blend-radii";
        }
    }
    notes.extend(extra);
    problems.extend(more);
    let mut radius_notes_block = if authored.is_empty() {
        notes_block = block;
        None
    } else {
        block
    };
    problems.extend(crate::notes::radius_notes(
        &mut notes,
        &radius_notes,
        spec,
        &sheet,
        &views,
    ));
    let stated: BTreeSet<_> = dimensions
        .iter()
        .filter(|d| d.placed.is_some() || matches!(&d.state, ItemState::Covered { by } if by == crate::dims::blends::RADIUS_TABLE))
        .flat_map(|d| d.measurements.iter().map(|m| m.feature.as_str()))
        .chain(
            notes
                .iter()
                .filter(|n| n.placed.is_some())
                .flat_map(|n| n.measurements.iter().map(|m| m.feature.as_str())),
        )
        .collect();
    // A default must explicitly leave proven relief profiles to the model. Hiding or
    // losing that companion note cannot silently assign the default to their small arcs.
    let unresolved = (!analysis.radius_reliefs.is_empty()
        && !notes
            .iter()
            .any(|n| n.id == crate::dims::blends::RELIEF_NOTE && n.placed.is_some()))
        || analysis.features.iter().any(|f| {
            matches!(f.kind.as_str(), "blend" | "fillet")
                && !crate::lint::blends::is_relief(analysis, f)
                && !stated.contains(f.id.as_str())
                && !radius_notes.iter().flat_map(|(_, ms)| ms).any(|m| {
                    f.kind == "fillet"
                        && f.parameters
                            .get("radius")
                            .and_then(|v| v.as_f64())
                            .is_some_and(|r| crate::dims::fmt(r) == crate::dims::fmt(m.value))
                })
        });
    if unresolved {
        for note in notes.iter_mut().filter(|n| {
            (n.placed.is_some()
                || (exceptions_unstated && matches!(n.state, ItemState::OmittedByUser)))
                && !spec.annotations.get(&n.id).is_some_and(|e| e.omit)
                && !n.measurements.is_empty()
                && n.text.starts_with("ALL UNSPECIFIED RADII")
        }) {
            note.placed = None;
            note.state = ItemState::NotDrawable {
                reason_code: "notes_not_placed".into(),
            };
            problems.push(Problem {
                id: format!("p:notes_not_placed:{}", note.id),
                code: "notes_not_placed".into(),
                severity: "error".into(),
                message: format!(
                    "{} {RADIUS_DEFAULT_WITHDRAWN}",
                    crate::wording::capital(&crate::wording::note(&note.text))
                ),
                subjects: vec![note.id.clone()],
                ..Default::default()
            });
        }
    }
    for note in notes
        .iter()
        .filter(|n| matches!(n.state, ItemState::NotDrawable { .. }))
    {
        for m in &note.measurements {
            problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", m.feature),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!(
                    "{} is not dimensioned: {} cannot safely state its radius.",
                    crate::wording::capital(&crate::wording::feature_id(analysis, &m.feature)),
                    crate::wording::note(&note.text)
                ),
                subjects: vec![m.feature.clone()],
                ..Default::default()
            });
        }
    }
    if !notes
        .iter()
        .any(|n| n.placed.is_some() && radius_notes.iter().any(|(r, _)| r.id == n.id))
    {
        if authored.is_empty() {
            notes_block = None;
        } else {
            radius_notes_block = None;
        }
    }
    crate::dims::blends::cover_fillets(&mut notes, &mut dimensions, spec, given);
    // The radius note/table is placed late. Its exact claims and the explicit relief
    // exclusions now resolve the earlier dimension-only coverage findings.
    let stated_radii: BTreeSet<_> = dimensions.iter()
        .filter(|d| d.placed.is_some() || matches!(&d.state, ItemState::Covered { by } if by == crate::dims::blends::RADIUS_TABLE))
        .flat_map(|d| &d.measurements)
        .chain(notes.iter().filter(|n| n.placed.is_some()).flat_map(|n| &n.measurements))
        .filter(|m| matches!(m.parameter.as_str(), "blend.radius" | "fillet.radius"))
        .map(|m| m.feature.as_str()).collect();
    problems.retain(|p| {
        p.code != "feature_not_dimensioned"
            || p.subjects.is_empty()
            || !p.subjects.iter().all(|id| {
                analysis.features.iter().any(|f| {
                    f.id == *id
                        && matches!(f.kind.as_str(), "blend" | "fillet")
                        && (stated_radii.contains(id.as_str())
                            || crate::lint::blends::is_relief(analysis, f))
                })
            })
    });

    for d in dimensions
        .iter()
        .filter(|d| matches!(&d.state, ItemState::Covered { by } if by.starts_with("n:") || by == crate::dims::blends::RADIUS_TABLE))
    {
        let ItemState::Covered { by } = &d.state else {
            unreachable!()
        };
        omitted.push(crate::dims::Omitted {
            id: d.id.clone(),
            label: d.label.clone(),
            basis: format!("conveyed by {by}: {}", d.basis),
            state: d.state.clone(),
        });
    }
    dimensions.retain(|d| !matches!(&d.state, ItemState::Covered { by } if by.starts_with("n:") || by == crate::dims::blends::RADIUS_TABLE));

    // A drawing-wide requirement carried by a general note is drawn as that note is.
    for r in requirements.iter_mut().filter(|r| r.carrier == "notes") {
        if let Some(n) = notes.iter().find(|n| n.id == r.id) {
            r.state = n.state.clone();
        }
    }
    for o in &mut omitted {
        if inactive.contains(&o.id) {
            o.basis =
                "inactive: the spec lists it, for another revision (spec_entries_inactive)".into();
        }
    }
    let mut spec = given.clone();
    spec.schema = SPEC_SCHEMA.into();
    if inactive.is_empty() {
        spec.binding = Some(analysis.binding.clone());
    }
    let mut plan = Plan {
        schema: PLAN_SCHEMA.into(),
        binding: analysis.binding.clone(),
        status: crate::completeness::Status::NeedsAttention,
        completeness: crate::completeness::Completeness::default(),
        sheet,
        sheets: Vec::new(),
        table_outcome: crate::hole_table::TableOutcome::None,
        table_restore: None,
        views,
        callout_widths: {
            let mut widths: BTreeMap<String, f64> = BTreeMap::new();
            for d in canonical.as_ref().map_or(&inventory, |p| &p.dimensions) {
                if d.kind == "callout"
                    && let Some(p) = &d.placed
                {
                    let width = widths.entry(d.view.clone()).or_default();
                    *width = width.max(p.label_box[2] - p.label_box[0]);
                }
            }
            widths
        },
        dimensions,
        omitted,
        notes,
        notes_block,
        radius_notes_block,
        radius_table,
        tables,
        gdt,
        requirements,
        hole_table: None,
        hole_tables: Vec::new(),
        balloons: Vec::new(),
        problems,
        spec,
    };
    crate::placement_intent::finish(&mut plan, analysis);
    let [width, height] = plan.sheet.page;
    for v in &plan.views {
        for w in v.line_work.iter().filter(|w| w.kind != "cutting_plane") {
            if w.curves.iter().any(|c| {
                let b = c.bbox();
                b[0] < MARGIN - 1e-9
                    || b[1] < MARGIN - 1e-9
                    || b[2] > width - MARGIN + 1e-9
                    || b[3] > height - MARGIN + 1e-9
            }) {
                plan.problems.push(Problem {
                    id: format!("p:annotation_off_sheet:{}", w.id),
                    code: "annotation_off_sheet".into(),
                    severity: "error".into(),
                    message: format!(
                        "{} in {} extends past the drawable area; choose a larger sheet or a smaller scale.",
                        crate::wording::capital(&crate::wording::line_work(&w.kind)),
                        crate::wording::view(&v.id)
                    ),
                    subjects: vec![v.id.clone()],
                    refs: vec![w.id.clone()],
                    ..Default::default()
                });
            }
        }
    }
    for p in &mut plan.problems {
        for subject in &mut p.subjects {
            if let Some(v) = plan.views.iter().find(|v| {
                annotation_obstacles
                    .iter()
                    .any(|w| w.kind == "ink" && w.view == v.id && w.id == *subject)
            }) {
                p.refs.push(subject.clone());
                *subject = v.id.clone();
            }
        }
        let mut seen = BTreeSet::new();
        p.subjects.retain(|s| seen.insert(s.clone()));
    }
    crate::completeness::report(analysis, &mut plan);
    plan.sheet.fill = crate::sheet_fit::fill(analysis, &plan);
    plan.sheet.basis += &format!("; drawn-item fill {:.1}%", plan.sheet.fill * 100.0);
    if reusable {
        crate::planning::store(analysis, "resolved_candidate", key, plan.clone());
    }
    Ok(plan)
}

/// *spec* as this version reads it (`draftwright.spec/2`): a version 2 spec as it is; a version
/// 1 spec with each dimension's key rewritten to the form it now has (R1 invariant 9: a change
/// of form is read, never guessed at):
/// - `d:<feature>/<parameter>@<view>` becomes `d:<feature>/<parameter>`: the view was where the
///   plan routed it, not a decision, so the entry does not keep it as its `view`;
/// - a PMI dimension's feature, `pmi:<category>:#<n>` (its record's Part 21 instance), becomes
///   the record's item ID in *analysis* ([`crate::pmi::Report::identities`]), when the spec is
///   bound to the analysis's file (or to none): a Part 21 instance names a record only in its
///   own file. A key whose record is not in the analysis, has no ID, or is of another file, is
///   kept as it was, for [`plan`] to refuse with the nearest IDs.
///
/// Two keys that come to one are ambiguous, and refused with both as the candidates. Anything
/// but a version 1 or 2 spec is refused.
pub fn migrate(spec: &Spec, analysis: &Analysis) -> Result<Spec, Invalid> {
    if spec.schema == SPEC_SCHEMA {
        return Ok(spec.clone());
    }
    if !SPEC_SCHEMAS.contains(&spec.schema.as_str()) {
        return Err(invalid(
            format!(
                "The spec is {:?}, which this draftwright does not read.",
                spec.schema
            ),
            &SPEC_SCHEMAS,
        ));
    }
    let same_file = spec
        .binding
        .as_ref()
        .is_none_or(|b| b.source_sha256 == analysis.binding.source_sha256);
    let ids = match &analysis.pmi {
        Some(crate::pmi::Reading::Read(r)) if same_file => r.identities(),
        _ => crate::pmi::Identities::default(),
    };
    let mut out = spec.clone();
    out.schema = SPEC_SCHEMA.into();
    out.dimensions.clear();
    let mut from: BTreeMap<String, String> = BTreeMap::new();
    for (old, entry) in &spec.dimensions {
        let mut key = match old.rsplit_once('@') {
            Some((stem, view)) if old.starts_with("d:") && !view.contains('/') => stem.to_string(),
            _ => old.clone(),
        };
        if let Some((feature, parameter)) = key.strip_prefix("d:").and_then(|k| k.rsplit_once('/'))
            && let Some(source) = feature.strip_prefix("pmi:")
            && let Some(id) = ids.ids.get(source)
        {
            key = format!("d:{id}/{parameter}");
        }
        if let Some(first) = from.get(&key) {
            return Err(Invalid {
                message: format!(
                    "The spec is {:?}, whose dimension keys {first:?} and {old:?} both read as {key:?} in {SPEC_SCHEMA} (a dimension's ID no longer names its view, and a PMI dimension is named by its record's identity): keep one of them.",
                    spec.schema
                ),
                choices: vec![first.clone(), old.clone()],
            });
        }
        from.insert(key.clone(), old.clone());
        out.dimensions.insert(key, entry.clone());
    }
    Ok(out)
}

/// Applies edit operations to a spec. Each operation is `{"op": "set" | "remove" | "add",
/// "path": "a/b/c", "value": …}`, with paths into the spec's JSON; a `/` inside a key (a
/// dimension's ID) is written `~1` and a `~` is written `~0`, as in a JSON pointer (RFC 6901).
/// `add` appends its value to the list at its path; a note added without an ID takes the next
/// free `n:<k>`, a table the next free `t:<k>`. The result is validated whole, so an unknown field or a value of the wrong kind
/// is refused with the reason.
pub fn edit(spec: &Spec, ops: &serde_json::Value) -> Result<Spec, Invalid> {
    let mut doc = serde_json::to_value(spec).expect("a spec serialises");
    let ops = ops
        .as_array()
        .ok_or_else(|| invalid("Edits are a list of operations.", &[]))?;
    for op in ops {
        let kind = op["op"].as_str().unwrap_or("");
        let path: Vec<String> = op["path"]
            .as_str()
            .ok_or_else(|| invalid("An edit needs a \"path\".", &[]))?
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.replace("~1", "/").replace("~0", "~"))
            .collect();
        let Some((last, parents)) = path.split_last() else {
            return Err(invalid("An edit's path is empty.", &[]));
        };
        if !["set", "remove", "add"].contains(&kind) {
            return Err(invalid(
                format!("{kind:?} is not an edit."),
                &["set", "remove", "add"],
            ));
        }
        let absent = || {
            invalid(
                format!(
                    "There is nothing at {:?} to remove.",
                    op["path"].as_str().unwrap_or("")
                ),
                &[],
            )
        };
        // `set` and `add` make the objects on their path; `remove` needs every one of them there.
        let mut at = &mut doc;
        for key in parents {
            if at.is_array() {
                if path.len() != 3 || path[0] != "tables" || path[2] != "sheet" || kind == "add" {
                    return Err(invalid(
                        "List-index edits are supported only for tables/<index>/sheet.",
                        &[],
                    ));
                }
                let index = key
                    .parse::<usize>()
                    .ok()
                    .ok_or_else(|| invalid(format!("{key:?} is not a list index."), &[]))?;
                at = at.get_mut(index).ok_or_else(|| {
                    invalid(format!("{key:?} is not an existing list index."), &[])
                })?;
            } else {
                let object = at
                    .as_object_mut()
                    .ok_or_else(|| invalid(format!("{key:?} is not inside an object."), &[]))?;
                at = if kind != "remove" {
                    object
                        .entry(key.to_string())
                        .or_insert_with(|| serde_json::json!({}))
                } else {
                    object.get_mut(key).ok_or_else(absent)?
                };
            }
        }
        let object = at
            .as_object_mut()
            .ok_or_else(|| invalid(format!("{:?} is not an object.", parents.join("/")), &[]))?;
        if kind == "set" {
            object.insert(last.to_string(), op["value"].clone());
        } else if kind == "add" {
            let Some(list) = object
                .entry(last.to_string())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
            else {
                return Err(invalid(
                    format!("{:?} is not a list to add to.", path.join("/")),
                    &[],
                ));
            };
            let mut value = op["value"].clone();
            if parents.is_empty()
                && (last == "notes" || last == "tables")
                && let Some(item) = value.as_object_mut()
                && !item.contains_key("id")
            {
                let ids = list.iter().filter_map(|n| n["id"].as_str());
                let id = if last == "notes" {
                    crate::notes::next_id(ids)
                } else {
                    crate::tables::next_id(ids)
                };
                item.insert("id".into(), id.into());
            }
            list.push(value);
        } else {
            object.remove(last.as_str()).ok_or_else(absent)?;
        }
    }
    let spec: Spec = serde_json::from_value(doc)
        .map_err(|e| invalid(format!("The edited spec is not valid: {e}."), &[]))?;
    crate::sheets::validate(&spec)?;
    crate::dims::check_tolerances(&spec)?;
    crate::dims::check_views(&spec)?;
    crate::placement_intent::check(&spec)?;
    title_block::check(&spec.sheet.title)?;
    crate::notes::check(&spec.notes)?;
    crate::tables::check(&spec.tables)?;
    Ok(spec)
}

/// Draws the plan: each view projected with hidden lines, placed and scaled, and the title
/// block. The spec travels in the output. Refused when a drawn face or edge of *part* did not
/// resolve on reading.
pub fn render(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
) -> Result<Drawing, Unresolved> {
    render_with(part, analysis, plan, font, &mut Projections::default())
}

/// What renders share: hidden-line projections and cuts, and the latest placed edges and
/// visible-ink bounds per projection. A different part binding discards the cached geometry.
#[derive(Clone, Default)]
pub struct Projections {
    binding: Option<String>,
    views: HashMap<Vec<u64>, Vec<Projected>>,
    cuts: HashMap<Vec<u64>, Vec<Vec<[f64; 3]>>>,
    geometry: HashMap<Vec<u64>, (Vec<u64>, ViewGeometry)>,
    stats: ProjectionStats,
}

/// Expensive passes actually performed by a kept cache, including unsuccessful projections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionStats {
    pub projections: usize,
    pub bounds: usize,
}

/// One view's projected edges on the page, with the bounds of its visible ink.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewGeometry {
    pub visible: Vec<Item>,
    pub hidden: Vec<Item>,
    pub bbox: Option<[f64; 4]>,
    pub undrawn: usize,
}

/// The geometry layer, in plan view order. Sheet furniture belongs to the annotation layer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeometryLayer {
    pub views: Vec<(String, ViewGeometry)>,
}

fn projection_key(v: &PlannedView) -> Vec<u64> {
    let cut = v.cut.as_ref();
    bits(&[
        &v.direction,
        &v.up,
        cut.map_or(&[], |c| &c.origin[..]),
        cut.map_or(&[], |c| &c.normal[..]),
    ])
}

/// Numbers as a map key, by their bits.
fn bits(numbers: &[&[f64]]) -> Vec<u64> {
    numbers
        .iter()
        .flat_map(|n| n.iter().map(|x| x.to_bits()))
        .collect()
}

impl Projections {
    pub fn stats(&self) -> ProjectionStats {
        self.stats
    }

    fn bind(&mut self, analysis: &Analysis) {
        if self.binding.as_ref() != Some(&analysis.binding.source_sha256) {
            self.views.clear();
            self.cuts.clear();
            self.geometry.clear();
            self.binding = Some(analysis.binding.source_sha256.clone());
        }
    }

    fn geometry(
        &mut self,
        part: &Part,
        analysis: &Analysis,
        v: &PlannedView,
    ) -> Result<ViewGeometry, Unresolved> {
        let view = View::new(v.direction, v.up).expect("a planned view");
        let centre = extent(analysis, &view).1;
        let key = projection_key(v);
        let placement = bits(&[&[v.scale, f64::from(v.id == "v:iso")], &centre, &v.at]);
        if let Some((previous, geometry)) = self.geometry.get(&key)
            && *previous == placement
        {
            return Ok(geometry.clone());
        }
        let mut geometry = ViewGeometry {
            visible: Vec::new(),
            hidden: Vec::new(),
            bbox: None,
            undrawn: 0,
        };
        let place = |q: [f64; 2]| {
            [
                v.at[0] + v.scale * (q[0] - centre[0]),
                v.at[1] + v.scale * (q[1] - centre[1]),
            ]
        };
        for piece in self.view(part, v)? {
            let mut moved = piece.clone();
            moved.points = piece.points.iter().map(|q| place(*q)).collect();
            moved.exact = piece
                .exact
                .as_ref()
                .map(|e| placed(e, v.scale, centre, v.at));
            match projected(&moved) {
                Some(curve) => {
                    let curve = if v.id == "v:iso" {
                        curve
                    } else {
                        crate::linework::stable_curve(curve)
                    };
                    let items = if piece.visible {
                        &mut geometry.visible
                    } else {
                        &mut geometry.hidden
                    };
                    items.push(Item::Edge(curve));
                }
                None => geometry.undrawn += 1,
            }
        }
        if v.cut.is_some() {
            geometry.hidden.clear();
        } else if v.id != "v:iso" {
            geometry.hidden = crate::linework::hidden(&geometry.visible, geometry.hidden);
        }
        self.stats.bounds += 1;
        geometry.bbox = items_bbox(&geometry.visible);
        self.geometry.insert(key, (placement, geometry.clone()));
        Ok(geometry)
    }

    fn view(&mut self, part: &Part, v: &PlannedView) -> Result<Vec<Projected>, Unresolved> {
        let cut = v.cut.as_ref();
        let key = projection_key(v);
        if let Some(pieces) = self.views.get(&key) {
            return Ok(pieces.clone());
        }
        let view = View::new(v.direction, v.up).expect("a planned view");
        self.stats.projections += 1;
        let pieces = match cut.and_then(|c| Plane::new(c.origin, c.normal)) {
            Some(plane) => project_section(part, &view, &plane),
            None => project(part, &view),
        }?;
        self.views.insert(key, pieces.clone());
        Ok(pieces)
    }

    fn cut(
        &mut self,
        part: &Part,
        cut: &Cut,
        plane: &Plane,
    ) -> Result<Vec<Vec<[f64; 3]>>, Unresolved> {
        let key = bits(&[&cut.origin, &cut.normal]);
        if let Some(lines) = self.cuts.get(&key) {
            return Ok(lines.clone());
        }
        let lines = hlr::section(part, plane)?;
        self.cuts.insert(key, lines.clone());
        Ok(lines)
    }
}

/// Draws only the views' edges, reusing projections, placed curves and visible bounds.
pub fn render_geometry(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    projections: &mut Projections,
) -> Result<GeometryLayer, Unresolved> {
    projections.bind(analysis);
    let views = plan
        .views
        .iter()
        .map(|v| Ok((v.id.clone(), projections.geometry(part, analysis, v)?)))
        .collect::<Result<_, Unresolved>>()?;
    Ok(GeometryLayer { views })
}

/// An annotation that can be rendered independently from every other item and the geometry.
pub enum AnnotationItem<'a> {
    Dimension(&'a PlannedDimension),
    Gdt(&'a crate::dims::gdt::PlannedGdt),
}

/// One item's ink and provenance. Run indices are local until appended to a drawing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RenderedItem {
    pub items: Vec<Item>,
    pub text: Vec<TextRun>,
    pub annotations: Vec<Annotation>,
    pub drawn: Vec<Drawn>,
    /// Session ink's owning items; absent for the unchanged single-sheet wire format.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<crate::items::Item>,
}

impl RenderedItem {
    /// Appends an item to the annotation layer, rebasing its searchable text's run indices.
    pub fn append(
        self,
        items: &mut Vec<Item>,
        text: &mut Vec<TextRun>,
        annotations: &mut Vec<Annotation>,
        sheet: &mut Sheet,
    ) {
        let offset = text.len();
        items.extend(self.items);
        text.extend(self.text);
        annotations.extend(self.annotations.into_iter().map(|mut a| {
            for run in &mut a.runs {
                *run += offset;
            }
            a
        }));
        sheet.items.extend(self.drawn);
    }
}

/// Draws a dimension, callout, feature control frame, datum symbol or PMI label from the plan.
pub fn render_item(item: AnnotationItem<'_>) -> RenderedItem {
    let mut out = RenderedItem::default();
    let interior = match &item {
        AnnotationItem::Dimension(d) => d.region == Some(crate::dims::CandidateRegion::Interior),
        AnnotationItem::Gdt(_) => false,
    };
    let (kind, label, lines, elbow) = match item {
        AnnotationItem::Dimension(d) => {
            out.annotations =
                crate::dims::render(std::slice::from_ref(d), &mut out.items, &mut out.text);
            let p = d.placed.as_ref();
            (
                Kind::Dimension,
                p.map(|p| p.label_box),
                p.map(|p| p.lines.clone()).unwrap_or_default(),
                p.filter(|_| d.kind == "callout")
                    .and_then(|p| p.lines.first())
                    .map(|l| l[1]),
            )
        }
        AnnotationItem::Gdt(g) => {
            out.annotations
                .extend(crate::dims::gdt::render(g, &mut out.items, &mut out.text));
            let p = g.placed.as_ref();
            (
                Kind::Gdt,
                p.map(|p| p.label_box),
                p.map(|p| p.lines.clone()).unwrap_or_default(),
                p.and_then(|p| p.lines.first()).map(|l| l[1]),
            )
        }
    };
    if let (Some(a), Some(ink)) = (out.annotations.first(), items_bbox(&out.items)) {
        out.drawn.push(Drawn {
            name: a.name.clone(),
            kind,
            ink,
            label,
            lines,
            elbow,
            interior,
        });
    }
    out
}

/// [`render`], projecting each view through *projections*: one kept across a part's renders
/// draws the same drawing without projecting a view again.
pub fn render_with(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
    projections: &mut Projections,
) -> Result<Drawing, Unresolved> {
    if !plan.sheets.is_empty() {
        return render_sheet_with(part, analysis, plan, &plan.sheet, font, projections);
    }
    let geometry = render_geometry(part, analysis, plan, projections)?;
    render_annotations(part, analysis, plan, font, geometry, projections)
}

/// Draw every planned sheet in order, each carrying the whole Spec. Projections are shared
/// across the document; [`render`] remains the sheet-1 entry point.
pub fn render_sheets(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
) -> Result<Vec<Drawing>, Unresolved> {
    let mut projections = Projections::default();
    let mut drawings = vec![render_with(part, analysis, plan, font, &mut projections)?];
    for sheet in &plan.sheets {
        drawings.push(render_sheet_with(
            part,
            analysis,
            plan,
            sheet,
            font,
            &mut projections,
        )?);
    }
    Ok(drawings)
}

fn render_sheet_with(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    sheet: &PlannedSheet,
    font: &Font,
    projections: &mut Projections,
) -> Result<Drawing, Unresolved> {
    let mut selected = plan.clone();
    selected.sheet = sheet.clone();
    selected.views.retain(|v| v.sheet == sheet.number);
    selected.tables.retain(|t| t.sheet == sheet.number);
    selected.hole_table = selected.hole_table.filter(|t| t.sheet == sheet.number);
    selected.hole_tables.retain(|t| t.sheet == sheet.number);
    let carries = |id: &str| selected.views.iter().any(|v| v.id == id);
    selected.dimensions.retain(|d| carries(&d.view));
    selected.gdt.retain(|g| carries(&g.view));
    selected.balloons.retain(|b| carries(&b.view));
    if sheet.number > 1 {
        selected.notes.clear();
        selected.notes_block = None;
        selected.radius_notes_block = None;
        selected.radius_table = None;
        selected.requirements.clear();
        selected.omitted.clear();
        selected.sheet.projection_symbol = None;
    }
    let geometry = render_geometry(part, analysis, &selected, projections)?;
    let remote: Vec<_> = plan
        .views
        .iter()
        .filter(|v| {
            v.sheet != sheet.number && v.cut.as_ref().is_some_and(|c| carries(&c.marked_in))
        })
        .collect();
    render_annotations_with_cuts(
        part,
        analysis,
        &selected,
        font,
        geometry,
        projections,
        &BTreeMap::new(),
        &remote,
    )
}

/// Assembles a geometry layer with the plan's annotations and sheet furniture. Each dimension
/// and PMI item uses [`render_item`], so a caller can draw a changed item alone.
pub fn render_annotations(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
    geometry: GeometryLayer,
    projections: &mut Projections,
) -> Result<Drawing, Unresolved> {
    render_annotations_with_items(
        part,
        analysis,
        plan,
        font,
        geometry,
        projections,
        &BTreeMap::new(),
    )
}

/// Assembles the same drawing, reusing independently rendered annotations by ID.
pub fn render_annotations_with_items(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
    geometry: GeometryLayer,
    projections: &mut Projections,
    items: &BTreeMap<String, RenderedItem>,
) -> Result<Drawing, Unresolved> {
    render_annotations_with_cuts(
        part,
        analysis,
        plan,
        font,
        geometry,
        projections,
        items,
        &[],
    )
}

/// Session assembly selects the already allocated page and keeps geometry in Plan order.
/// Sheet furniture remains the renderer's responsibility (R5 I2).
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_annotations_on_sheet(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
    mut geometry: GeometryLayer,
    projections: &mut Projections,
    items: &BTreeMap<String, RenderedItem>,
    number: u32,
) -> Result<Drawing, Unresolved> {
    if plan.sheets.is_empty() && number == 1 {
        return render_annotations_with_items(
            part,
            analysis,
            plan,
            font,
            geometry,
            projections,
            items,
        );
    }
    let mut selected = plan.clone();
    selected.sheet = std::iter::once(&plan.sheet)
        .chain(&plan.sheets)
        .find(|s| s.number == number)
        .expect("the session validates its sheet number")
        .clone();
    selected.views.retain(|v| v.sheet == number);
    let local = |id: &str| selected.views.iter().any(|v| v.id == id);
    geometry.views.retain(|(id, _)| local(id));
    selected.dimensions.retain(|d| local(&d.view));
    selected.gdt.retain(|g| local(&g.view));
    selected.balloons.retain(|b| local(&b.view));
    selected.tables.retain(|t| t.sheet == number);
    selected.hole_table = selected.hole_table.filter(|t| t.sheet == number);
    selected.hole_tables.retain(|t| t.sheet == number);
    if number != 1 {
        selected.notes.clear();
        selected.requirements.clear();
        selected.radius_table = None;
        selected.sheet.projection_symbol = None;
    }
    let remote = plan
        .views
        .iter()
        .filter(|v| v.sheet != number && v.cut.as_ref().is_some_and(|c| local(&c.marked_in)))
        .collect::<Vec<_>>();
    render_annotations_with_cuts(
        part,
        analysis,
        &selected,
        font,
        geometry,
        projections,
        items,
        &remote,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_annotations_with_cuts(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    font: &Font,
    geometry: GeometryLayer,
    projections: &mut Projections,
    items: &BTreeMap<String, RenderedItem>,
    remote_sections: &[&PlannedView],
) -> Result<Drawing, Unresolved> {
    // Section furniture (hatch, cutting plane), and its text: what, where.
    let mut furniture: Vec<Item> = Vec::new();
    let mut captions: Vec<(String, [f64; 2])> = Vec::new();
    // What lint's sheet checks read: each view's visible box, and each thing drawn, by name.
    let mut sheet = Sheet::default();
    let mut caption_names: Vec<String> = Vec::new();
    let mut references = Vec::new();
    let mut layers: Vec<Layer> = layer_styles()
        .into_iter()
        .map(|(name, style)| Layer {
            name: name.into(),
            style,
            items: Vec::new(),
        })
        .collect();
    layers.push(Layer {
        name: "centres".into(),
        style: crate::output::centre_style(),
        items: Vec::new(),
    });
    let mut undrawn = BTreeMap::new();
    assert_eq!(
        plan.views.len(),
        geometry.views.len(),
        "geometry covers every planned view"
    );
    for (v, (name, geometry)) in plan.views.iter().zip(geometry.views) {
        assert_eq!(v.id, name, "geometry follows plan view order");
        if geometry.undrawn > 0 {
            undrawn.insert(v.id.clone(), geometry.undrawn);
        }
        if let Some(bbox) = geometry.bbox {
            sheet.views.push(DrawnView {
                name: v.id.clone(),
                bbox,
                edges: geometry
                    .visible
                    .iter()
                    .filter_map(|i| match i {
                        Item::Edge(c) => Some(c.clone()),
                        Item::Face(_) => None,
                    })
                    .collect(),
            });
        }
        sheet.frames.insert(
            v.id.trim_start_matches("v:").into(),
            crate::output::DrawnFrame {
                direction: v.direction,
                up: v.up,
                at: v.at,
                scale: v.scale,
            },
        );
        for w in v.line_work.iter().filter(|w| w.kind != "cutting_plane") {
            let lines = crate::centrelines::segments(w);
            let ink: Vec<_> = w.curves.iter().cloned().map(Item::Edge).collect();
            if let Some(ink_box) = items_bbox(&ink) {
                sheet.items.push(Drawn {
                    name: w.id.clone(),
                    kind: Kind::Centreline,
                    ink: ink_box,
                    label: None,
                    lines,
                    elbow: None,
                    interior: false,
                });
            }
            if let Some(c) = w.centre {
                sheet
                    .centre_marks
                    .entry(v.id.trim_start_matches("v:").into())
                    .or_default()
                    .push(c);
            }
            layers[3].items.extend(ink);
        }
        layers[0].items.extend(geometry.visible);
        layers[1].items.extend(geometry.hidden);
    }
    for v in plan.views.iter().chain(remote_sections.iter().copied()) {
        let plane = v.cut.as_ref().and_then(|c| Plane::new(c.origin, c.normal));
        if let (Some(plane), Some(cut)) = (&plane, &v.cut) {
            let remote = v.sheet != plan.sheet.number;
            let f = if remote {
                let mut f = Furniture {
                    ink: Vec::new(),
                    hatch: 0,
                    chain: None,
                    text: Vec::new(),
                };
                cutting_furniture(plan, cut, &mut f);
                f
            } else {
                section_furniture(part, analysis, plan, v, (plane, cut), projections)?
            };
            // draftwright's pieces, each by its ink: the hatch, the chain line by its line, and
            // each end's arrowhead and wing stub (`section::cutting_plane`'s order).
            let mut piece = |name: String, kind: Kind, ink: &[Item], lines: Vec<[[f64; 2]; 2]>| {
                if let Some(ink) = items_bbox(ink) {
                    sheet.items.push(Drawn {
                        name,
                        kind,
                        ink,
                        label: None,
                        lines,
                        elbow: None,
                        interior: false,
                    });
                }
            };
            piece(
                "section_hatch".into(),
                Kind::Hatch,
                &f.ink[..f.hatch],
                Vec::new(),
            );
            let plane_ink = &f.ink[f.hatch..];
            if let Some(line) = f.chain {
                let chain = plane_ink.len() - 4;
                piece(
                    "section_line".into(),
                    Kind::Centreline,
                    &plane_ink[..chain],
                    vec![line],
                );
                for (k, side) in ["left", "right"].into_iter().enumerate() {
                    let at = chain + 2 * k;
                    let (wing, head) = (&plane_ink[at..at + 1], &plane_ink[at + 1..at + 2]);
                    piece(
                        format!("section_arrow_{side}"),
                        Kind::CuttingPlane,
                        head,
                        Vec::new(),
                    );
                    piece(
                        format!("section_wing_{side}"),
                        Kind::CuttingPlane,
                        wing,
                        Vec::new(),
                    );
                }
            }
            // draftwright's names: the caption, then the cutting plane's letters.
            let l = cut.label.to_lowercase();
            if !remote {
                caption_names.push("section_caption".into());
            }
            if f.chain.is_some() {
                caption_names.extend([format!("section_{l}_left"), format!("section_{l}_right")]);
            }
            furniture.extend(f.ink);
            captions.extend(f.text);
            for (reference, prefix, name, target) in [
                (
                    v.xref.as_ref().filter(|_| !remote),
                    "FROM".to_string(),
                    format!("{}/xref", v.id),
                    format!("section_{l}_left"),
                ),
                (
                    cut.xref.as_ref().filter(|_| f.chain.is_some()),
                    format!("SECTION {}–{},", cut.label, cut.label),
                    format!("{}/cut/xref", v.id),
                    "section_caption".into(),
                ),
            ] {
                if let Some(r) = reference {
                    references.push((
                        captions.len(),
                        name.clone(),
                        prefix.clone(),
                        target,
                        r.clone(),
                    ));
                    captions.push((format!("{prefix} SHEET {}, ZONE {}", r.sheet, r.zone), r.at));
                    caption_names.push(name);
                }
            }
        }
    }
    for v in &plan.views {
        if let Some(note) = &v.scale_note {
            let view = View::new(v.direction, v.up).expect("a planned view");
            let half_h = 0.5 * extent(analysis, &view).0[1] * v.scale;
            let gap = if v.cut.is_some() { 17.0 } else { 7.0 };
            captions.push((note.clone(), [v.at[0], v.at[1] - half_h - gap]));
            caption_names.push(format!("{}/scale_note", v.id));
        }
    }
    // The border and the title block, lower right.
    let [w, h] = plan.sheet.page;
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
            .windows(2)
            .map(|p| Item::Edge(Curve::Line(p[0], p[1])))
            .collect::<Vec<_>>()
    };
    let part_layer = &mut layers[0].items;
    part_layer.extend(rect(MARGIN, MARGIN, w - MARGIN, h - MARGIN));
    sheet.drawable = Some([MARGIN, MARGIN, w - MARGIN, h - MARGIN]);
    // The block is the grid's four rows at the bottom of the box the layout reserves for it.
    let block = plan.sheet.title_block;
    let [tx0, ty0, tx1, _] = block;
    part_layer.extend(rect(tx0, ty0, tx1, title_block::top(block)));
    let mut text = Vec::new();
    // The dimensions and callouts, drawn from the plan alone, one at a time so each one's ink
    // is measured.
    let mut annotations = Vec::new();
    for d in &plan.dimensions {
        items
            .get(&d.id)
            .cloned()
            .unwrap_or_else(|| render_item(AnnotationItem::Dimension(d)))
            .append(
                &mut layers[2].items,
                &mut text,
                &mut annotations,
                &mut sheet,
            );
    }
    // The feature control frames and datum feature symbols, likewise.
    for g in &plan.gdt {
        items
            .get(&g.id)
            .cloned()
            .unwrap_or_else(|| render_item(AnnotationItem::Gdt(g)))
            .append(
                &mut layers[2].items,
                &mut text,
                &mut annotations,
                &mut sheet,
            );
    }
    // Glyphs *laid* out drawn moved to *at*, and the *run* (text, font file, size), if any, as
    // invisible text centred there; the run's index in the drawing's text.
    let mut label = |laid: Laid,
                     at: [f64; 2],
                     layer: &mut Vec<Item>,
                     run: Option<(&str, &str, f64)>|
     -> Option<usize> {
        let shift = |p: [f64; 2]| [p[0] + at[0], p[1] + at[1]];
        let curves: Vec<Curve> = laid
            .contours
            .iter()
            .flat_map(|c| c.iter())
            .map(|seg| match *seg {
                Segment::Line(a, b) => Curve::Line(shift(a), shift(b)),
                Segment::Quad(a, b, c) => Curve::Beziers(vec![vec![shift(a), shift(b), shift(c)]]),
                Segment::Cubic(a, b, c, d) => {
                    Curve::Beziers(vec![vec![shift(a), shift(b), shift(c), shift(d)]])
                }
            })
            .collect();
        if !curves.is_empty() {
            // One path of every contour: TrueType's winding makes the holes.
            let wires = laid
                .contours
                .iter()
                .scan(0usize, |start, c| {
                    let wire: Vec<Curve> = curves[*start..*start + c.len()].to_vec();
                    *start += c.len();
                    Some(wire)
                })
                .filter(|w| !w.is_empty())
                .collect();
            layer.push(Item::Face(wires));
        }
        let (s, font, size) = run?;
        text.push(TextRun {
            text: s.into(),
            at,
            size,
            rotation: 0.0,
            font: font.into(),
            h_align: HAlign::Center,
            v_align: VAlign::Middle,
        });
        Some(text.len() - 1)
    };
    let dims = &mut layers[2].items;
    dims.extend(furniture);
    // A text-only thing as drawn: the box of its ink *items*, which is also its label.
    let labelled = |name: &str, kind: Kind, items: &[Item]| {
        items_bbox(items).map(|b| Drawn {
            name: name.into(),
            kind,
            ink: b,
            label: Some(b),
            lines: Vec::new(),
            elbow: None,
            interior: false,
        })
    };
    for (index, ((s, at), name)) in captions.iter().zip(&caption_names).enumerate() {
        let size = crate::section::FONT_SIZE;
        let run = (s.as_str(), "IBMPlexMono-Regular.ttf", size);
        let from = dims.len();
        if let Some(run) = label(font.text(s, size), *at, dims, Some(run))
            && let Some((_, name, prefix, target, r)) =
                references.iter().find(|(i, ..)| *i == index)
        {
            sheet.references.push(crate::output::DrawnReference {
                name: name.clone(),
                target: target.clone(),
                prefix: prefix.clone(),
                sheet: r.sheet,
                zone: r.zone.clone(),
                run,
            });
        }
        sheet
            .items
            .extend(labelled(name, Kind::Caption, &dims[from..]));
    }
    if let Some(grid) = &plan.sheet.zones {
        // All zone furniture lives outside the drawable border, preserving every view band.
        for n in 0..grid.columns {
            let x = (f64::from(n) + 0.5) * w / f64::from(grid.columns);
            let value = (n + 1).to_string();
            for y in [MARGIN / 2., h - MARGIN / 2.] {
                label(
                    font.text(&value, 3.),
                    [x, y],
                    dims,
                    Some((&value, "IBMPlexMono-Regular.ttf", 3.)),
                );
            }
            if n > 0 {
                let x = f64::from(n) * w / f64::from(grid.columns);
                for (a, b) in [(0., MARGIN), (h - MARGIN, h)] {
                    dims.push(Item::Edge(Curve::Line([x, a], [x, b])));
                }
            }
        }
        for n in 0..grid.rows {
            let y = h - (f64::from(n) + 0.5) * h / f64::from(grid.rows);
            let value = (b"ABCDEFGHJKLMNPQRSTUVWXYZ"[n as usize] as char).to_string();
            for x in [MARGIN / 2., w - MARGIN / 2.] {
                label(
                    font.text(&value, 3.),
                    [x, y],
                    dims,
                    Some((&value, "IBMPlexMono-Regular.ttf", 3.)),
                );
            }
            if n > 0 {
                let y = h - f64::from(n) * h / f64::from(grid.rows);
                for (a, b) in [(0., MARGIN), (w - MARGIN, w)] {
                    dims.push(Item::Edge(Curve::Line([a, y], [b, y])));
                }
            }
        }
    }
    // Each cell: its dividers, its label at its lower left, and its value (none drawn for a
    // field not supplied) centred in it, whole, and searchable. The ink is the font-safe value.
    let title_from = dims.len();
    // The runs that state the sheet's PMI requirements, by requirement ID.
    let mut stated: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let title_material = match &analysis.pmi {
        Some(crate::pmi::Reading::Read(r)) => r.title_material(),
        _ => None,
    };
    for c in &plan.sheet.title_cells {
        let [x0, y0, x1, y1] = c.cell;
        if x0 > tx0 {
            dims.push(Item::Edge(Curve::Line([x0, y0], [x0, y1])));
        } else if y1 < title_block::top(block) {
            // A row's first cell draws the line along the row's top.
            dims.push(Item::Edge(Curve::Line([tx0, y1], [tx1, y1])));
        }
        let laid = title_block::lay(&c.label, title_block::LABEL_TEXT);
        let at = [
            x0 + title_block::LABEL_PAD - laid.min[0],
            y0 + title_block::LABEL_PAD - laid.min[1],
        ];
        label(laid, at, dims, None);
        if let Some(v) = &c.value {
            let centre = [0.5 * (x0 + x1), 0.5 * (y0 + y1)];
            let run = (v.as_str(), title_block::FONT, title_block::TEXT);
            let from = dims.len();
            let at = label(
                title_block::lay(v, title_block::TEXT),
                centre,
                dims,
                Some(run),
            );
            for r in &plan.requirements {
                if r.carrier == "title_block" && c.field == "general_tolerance" {
                    stated.entry(r.id.clone()).or_default().extend(at);
                }
            }
            // A material cell that says the file's material states its source.
            if c.field == "material"
                && let Some(m) = &title_material
                && v.as_str() == m.designation.trim()
            {
                stated
                    .entry(format!("pmi:{}", m.census_id()))
                    .or_default()
                    .extend(at);
            }
            if let Some(ink) = items_bbox(&dims[from..]) {
                sheet.fields.push(DrawnField {
                    field: c.field.clone(),
                    cell: c.cell,
                    ink,
                });
            }
        }
    }
    // The block's outline and every cell's ink.
    let outline = [tx0, ty0, tx1, title_block::top(block)];
    let ink = items_bbox(&dims[title_from..]).map_or(outline, |b| {
        [
            b[0].min(outline[0]),
            b[1].min(outline[1]),
            b[2].max(outline[2]),
            b[3].max(outline[3]),
        ]
    });
    // Its rules: the outline, and each cell's divider.
    let outline_rules = [[tx0, ty0], [tx1, ty0], [tx1, outline[3]], [tx0, outline[3]]];
    let mut rules: Vec<[[f64; 2]; 2]> = (0..4)
        .map(|k| [outline_rules[k], outline_rules[(k + 1) % 4]])
        .collect();
    rules.extend(dims[title_from..].iter().filter_map(|i| match i {
        Item::Edge(Curve::Line(a, b)) => Some([*a, *b]),
        _ => None,
    }));
    sheet.items.push(Drawn {
        name: "title_block".into(),
        kind: Kind::TitleBlock,
        ink,
        label: None,
        lines: rules,
        elbow: None,
        interior: false,
    });
    // The general notes: the block's rules, and each row centred on its planned ink, searchable.
    let mixed = plan.notes.iter().any(|n| !n.id.starts_with("n:f:blend:"));
    for (i, block) in [plan.notes_block.as_ref(), plan.radius_notes_block.as_ref()]
        .into_iter()
        .enumerate()
    {
        if i == 1 && !mixed {
            continue;
        }
        let notes_from = dims.len();
        if let Some(block) = block {
            for [a, b] in crate::notes::rules(block) {
                dims.push(Item::Edge(Curve::Line(a, b)));
            }
            let row = block.title.as_str();
            let laid = font.text(&crate::notes::ink(row), crate::notes::TEXT);
            label(
                laid,
                block.title_at,
                dims,
                Some((row, crate::dims::FONT, crate::notes::TEXT)),
            );
        }
        for n in plan.notes.iter().filter(|n| {
            n.placed.is_some() && (!mixed || (i == 1) == n.id.starts_with("n:f:blend:"))
        }) {
            let p = n.placed.as_ref().unwrap();
            let runs: Vec<usize> = n
                .rows
                .iter()
                .zip(&p.rows_at)
                .filter_map(|(row, at)| {
                    let laid = font.text(&crate::notes::ink(row), crate::notes::TEXT);
                    label(
                        laid,
                        *at,
                        dims,
                        Some((row.trim_start(), crate::dims::FONT, crate::notes::TEXT)),
                    )
                })
                .collect();
            if n.id.starts_with("pmi:") {
                stated.entry(n.id.clone()).or_default().extend(&runs);
            }
            if !n.measurements.is_empty() || n.id == crate::dims::blends::RELIEF_NOTE {
                annotations.push(Annotation {
                    coordinate_origin: None,
                    claims: Vec::new(),
                    name: n.id.clone(),
                    view: None,
                    runs,
                    satisfies: n
                        .measurements
                        .iter()
                        .map(|m| crate::output::Claim {
                            feature: m.feature.clone(),
                            parameter: m.parameter.clone(),
                        })
                        .collect(),
                });
            }
        }
        if let Some(ink) = items_bbox(&dims[notes_from..]) {
            sheet.items.push(Drawn {
                name: if i == 0 {
                    "general_notes"
                } else {
                    "blend_radius_notes"
                }
                .into(),
                kind: Kind::Notes,
                ink,
                label: None,
                lines: Vec::new(),
                elbow: None,
                interior: false,
            });
        }
    }
    // The default surface finish beside the title block: its symbol and its requirement.
    for r in &plan.requirements {
        let Some(p) = &r.placed else { continue };
        let from = dims.len();
        for [a, b] in &p.strokes {
            dims.push(Item::Edge(Curve::Line(*a, *b)));
        }
        let run = (r.text.as_str(), title_block::FONT, title_block::TEXT);
        let at = label(
            title_block::lay(&r.text, title_block::TEXT),
            p.text_at,
            dims,
            Some(run),
        );
        stated.entry(r.id.clone()).or_default().extend(at);
        if let Some(ink) = items_bbox(&dims[from..]) {
            sheet.items.push(Drawn {
                name: r.id.clone(),
                kind: Kind::Gdt,
                ink,
                label: Some(p.label_box),
                lines: p.strokes.clone(),
                elbow: None,
                interior: false,
            });
        }
    }
    // The tables: each one's rules, and each cell centred on its planned ink, searchable.
    for table in &plan.tables {
        let Some(p) = &table.placed else { continue };
        let table_from = dims.len();
        for [a, b] in crate::tables::ruling(p.cells, &p.columns) {
            dims.push(Item::Edge(Curve::Line(a, b)));
        }
        for (cell, at) in crate::tables::texts(table) {
            let run = (cell.trim(), crate::dims::FONT, crate::tables::TEXT);
            let laid = font.text(&crate::tables::ink(cell), crate::tables::TEXT);
            label(laid, at, dims, Some(run));
        }
        if let Some(ink) = items_bbox(&dims[table_from..]) {
            sheet.items.push(Drawn {
                name: table.id.clone(),
                kind: Kind::Table,
                ink,
                label: None,
                lines: Vec::new(),
                elbow: None,
                interior: false,
            });
        }
    }
    // Each radius-centre row locates one occurrence, unlike a representative group site.
    if let Some(t) = &plan.radius_table
        && let Some(p) = &t.table.placed
    {
        let start = dims.len();
        for [a, b] in crate::tables::ruling(p.cells, &p.columns) {
            dims.push(Item::Edge(Curve::Line(a, b)));
        }
        for (i, (row, at)) in t.table.rows.iter().zip(&p.cells_at).enumerate() {
            let mut runs = Vec::new();
            for (cell, at) in row.iter().zip(at) {
                let Some(at) = at else { continue };
                let laid = font.text(&crate::tables::ink(cell), crate::tables::TEXT);
                runs.extend(label(
                    laid,
                    *at,
                    dims,
                    Some((cell, crate::dims::FONT, crate::tables::TEXT)),
                ));
            }
            if i > 0
                && let Some(radius) = t.rows.get(i - 1)
            {
                annotations.push(Annotation {
                    coordinate_origin: None,
                    name: format!("{}/{}", t.table.id, radius.feature),
                    view: Some(radius.view.clone()),
                    runs,
                    claims: vec![Claim {
                        feature: radius.feature.clone(),
                        parameter: "blend.radius".into(),
                    }],
                    satisfies: Vec::new(),
                });
            }
        }
        if let Some(ink) = items_bbox(&dims[start..]) {
            sheet.items.push(Drawn {
                name: t.table.id.clone(),
                kind: Kind::Table,
                ink,
                label: None,
                lines: Vec::new(),
                elbow: None,
                interior: false,
            });
        }
    }
    // The hole table, drawn as the tables are, block by block. Each row is an annotation for
    // lint: its cells' text (the tag apart) and the measurements they state, as draftwright
    // registers the table's measurements.
    for t in plan.hole_table.iter().chain(&plan.hole_tables) {
        let table_from = dims.len();
        let mut row_runs: Vec<Vec<Vec<usize>>> = t
            .rows
            .iter()
            .map(|r| vec![Vec::new(); r.cells.len()])
            .collect();
        let headers = if t.view.is_some() { 2 } else { 1 };
        let per = t
            .blocks
            .first()
            .map_or(0, |b| b.rows.len().saturating_sub(headers));
        for (k, block) in t.blocks.iter().enumerate() {
            let Some(p) = &block.placed else { continue };
            for [a, b] in crate::tables::ruling(p.cells, &p.columns) {
                dims.push(Item::Edge(Curve::Line(a, b)));
            }
            for (r, (row, at)) in block.rows.iter().zip(&p.cells_at).enumerate() {
                for (c, (cell, at)) in row.iter().zip(at).enumerate() {
                    let Some(at) = at else { continue };
                    let run = (cell.trim(), crate::dims::FONT, crate::tables::TEXT);
                    let laid = if t.view.is_some() && r >= headers && c == 1 {
                        // Reuse the established callout symbols in scheduled size cells;
                        // a font's missing-glyph box must not stand for recess or depth.
                        let ink = crate::hole_table::size_ink(cell, font);
                        let centre = [0, 1].map(|k| 0.5 * (ink.min[k] + ink.max[k]));
                        let local = |p: [f64; 2]| [p[0] - centre[0], p[1] - centre[1]];
                        for [a, b] in ink.strokes {
                            let a = local(a);
                            let b = local(b);
                            dims.push(Item::Edge(Curve::Line(
                                [at[0] + a[0], at[1] + a[1]],
                                [at[0] + b[0], at[1] + b[1]],
                            )));
                        }
                        Laid {
                            contours: ink
                                .glyphs
                                .into_iter()
                                .flatten()
                                .map(|c| {
                                    c.into_iter()
                                        .map(|s| match s {
                                            Segment::Line(a, b) => {
                                                Segment::Line(local(a), local(b))
                                            }
                                            Segment::Quad(a, b, c) => {
                                                Segment::Quad(local(a), local(b), local(c))
                                            }
                                            Segment::Cubic(a, b, c, d) => Segment::Cubic(
                                                local(a),
                                                local(b),
                                                local(c),
                                                local(d),
                                            ),
                                        })
                                        .collect()
                                })
                                .collect(),
                            min: local(ink.min),
                            max: local(ink.max),
                        }
                    } else {
                        font.text(&crate::tables::ink(cell), crate::tables::TEXT)
                    };
                    let index = label(laid, *at, dims, Some(run));
                    if r >= headers
                        && c > 0
                        && let Some(runs) = row_runs.get_mut(k * per + r - headers)
                    {
                        runs[c].extend(index);
                    }
                }
            }
        }
        if let Some(ink) = items_bbox(&dims[table_from..]) {
            sheet.items.push(Drawn {
                name: t.id.clone(),
                kind: Kind::Table,
                ink,
                label: None,
                lines: Vec::new(),
                elbow: None,
                interior: false,
            });
        }
        for (row, cells) in t.rows.iter().zip(row_runs) {
            if t.view.is_none() {
                annotations.push(Annotation {
                    satisfies: Vec::new(),
                    coordinate_origin: None,
                    name: format!("{}/{}", t.id, row.tag),
                    view: Some("v:plan".into()),
                    runs: cells.into_iter().flatten().collect(),
                    claims: row
                        .measurements
                        .iter()
                        .map(|m| Claim {
                            feature: m.feature.clone(),
                            parameter: m.parameter.clone(),
                        })
                        .collect(),
                });
            } else {
                // A coordinate cell proves only its own axis. A matching number elsewhere in
                // this row or another row cannot certify a changed coordinate.
                for (c, runs) in cells.into_iter().enumerate().take(5).skip(1) {
                    if c == 4 {
                        annotations.push(Annotation {
                            satisfies: Vec::new(),
                            coordinate_origin: None,
                            name: format!("{}/{}/entry", t.id, row.tag),
                            view: t.view.clone(),
                            runs,
                            claims: vec![Claim {
                                feature: row.feature.clone(),
                                parameter: format!(
                                    "entry.view.{}",
                                    t.view
                                        .as_ref()
                                        .and_then(|v| plan.views.iter().find(|w| &w.id == v))
                                        .map_or_else(String::new, |v| {
                                            v.direction
                                                .iter()
                                                .map(|d| format!("{d:.6}"))
                                                .collect::<Vec<_>>()
                                                .join(",")
                                        })
                                ),
                            }],
                        });
                        continue;
                    }
                    let location = row
                        .measurements
                        .iter()
                        .filter(|m| m.parameter.starts_with("location.length."))
                        .nth(c.saturating_sub(2));
                    let claims = row
                        .measurements
                        .iter()
                        .filter(|m| {
                            if c == 1 {
                                !m.parameter.starts_with("location.length.")
                            } else {
                                location.is_some_and(|l| m.parameter == l.parameter)
                            }
                        })
                        .map(|m| Claim {
                            feature: m.feature.clone(),
                            parameter: m.parameter.clone(),
                        })
                        .collect();
                    annotations.push(Annotation {
                        satisfies: Vec::new(),
                        coordinate_origin: if c > 1 { t.origin } else { None },
                        name: format!("{}/{}/{}", t.id, row.tag, ["tag", "size", "x", "y"][c]),
                        view: t.view.clone(),
                        runs,
                        claims,
                    });
                }
            }
        }
    }

    // The balloons: each a ring with its tag, and its leader with an arrowhead at the hole's
    // rim. Furniture to lint's sheet checks, as draftwright marks a balloon a centreline: judged
    // by its leader and its ring's box.
    for b in &plan.balloons {
        let from = dims.len();
        let [cx, cy] = b.at;
        let circle = |r: f64, turn: f64| Curve::Conic {
            centre: b.at,
            radii: [r, r],
            x_axis: [1.0, 0.0],
            y_axis: [0.0, turn],
            t0: 0.0,
            t1: 2.0 * std::f64::consts::PI,
        };
        if b.radius > 0.0 {
            dims.push(Item::Face(vec![
                vec![circle(b.radius, 1.0)],
                vec![circle(b.radius - crate::hole_table::RING, -1.0)],
            ]));
        }
        let run = (b.tag.as_str(), crate::dims::FONT, crate::dims::TEXT);
        label(font.text(&b.tag, crate::dims::TEXT), b.at, dims, Some(run));
        let r = b.radius;
        let mut lines = vec![
            [[cx - r, cy - r], [cx + r, cy - r]],
            [[cx + r, cy - r], [cx + r, cy + r]],
            [[cx + r, cy + r], [cx - r, cy + r]],
            [[cx - r, cy + r], [cx - r, cy - r]],
        ];
        if b.radius == 0.0 {
            lines.clear();
        }
        if let Some([tip, end]) = b.leader {
            dims.push(Item::Edge(Curve::Line(tip, end)));
            let d = [end[0] - tip[0], end[1] - tip[1]];
            let len = d[0].hypot(d[1]).max(1e-12);
            let u = [d[0] / len, d[1] / len];
            let a = crate::dims::ARROW.min(len);
            let tail = [tip[0] + a * u[0], tip[1] + a * u[1]];
            let h = crate::dims::ARROW / 6.0;
            let b1 = [tail[0] - h * u[1], tail[1] + h * u[0]];
            let b2 = [tail[0] + h * u[1], tail[1] - h * u[0]];
            dims.push(Item::Face(vec![vec![
                Curve::Line(tip, b1),
                Curve::Line(b1, b2),
                Curve::Line(b2, tip),
            ]]));
            lines.insert(0, [tip, end]);
        }
        if let Some(ink) = items_bbox(&dims[from..]) {
            sheet.items.push(Drawn {
                name: b.id.clone(),
                kind: if b.id.starts_with("hole_schedule_") {
                    Kind::Dimension
                } else {
                    Kind::Centreline
                },
                ink,
                label: b.label_box,
                lines,
                elbow: None,
                interior: b.id.starts_with("hole_schedule_"),
            });
        }
    }
    // The scale, outside the block (ISO 7200 §4), above its right end.
    let scale = format!("SCALE {}", plan.sheet.scale);
    let at = title_block::scale_note(block, &scale);
    let run = (scale.as_str(), title_block::FONT, title_block::TEXT);
    let from = dims.len();
    label(
        title_block::lay(&scale, title_block::TEXT),
        at,
        dims,
        Some(run),
    );
    sheet
        .items
        .extend(labelled("scale_note", Kind::ScaleNote, &dims[from..]));
    // Each drawing-wide PMI requirement by the runs that state it, for lint and the PMI checks.
    annotations.extend(stated.into_iter().map(|(name, runs)| Annotation {
        coordinate_origin: None,
        satisfies: Vec::new(),
        name,
        view: None,
        runs,
        claims: Vec::new(),
    }));
    // Authored plate omissions are consumer provenance, with no ink or claimed value.
    annotations.extend(
        plan.omitted
            .iter()
            .filter(|o| {
                o.id.ends_with("/thickness.length")
                    && matches!(
                        o.state,
                        crate::items::State::OmittedByUser | crate::items::State::NotAuthored
                    )
            })
            .filter_map(|o| {
                let (feature, parameter) = o.id.strip_prefix("d:")?.split_once('/')?;
                Some(Annotation {
                    coordinate_origin: None,
                    name: format!("suppressed:{}", o.id),
                    view: None,
                    runs: Vec::new(),
                    claims: vec![crate::output::Claim {
                        feature: feature.into(),
                        parameter: parameter.into(),
                    }],
                    satisfies: Vec::new(),
                })
            }),
    );
    if let Some(symbol) = &plan.sheet.projection_symbol {
        let ink: Vec<Item> = symbol.curves.iter().cloned().map(Item::Edge).collect();
        if let Some(bbox) = items_bbox(&ink) {
            sheet.items.push(Drawn {
                name: "projection_symbol".into(),
                kind: Kind::ProjectionSymbol,
                ink: bbox,
                label: None,
                lines: symbol
                    .curves
                    .iter()
                    .filter_map(|c| match c {
                        Curve::Line(a, b) => Some([*a, *b]),
                        _ => None,
                    })
                    .collect(),
                elbow: None,
                interior: false,
            });
        }
        let mut style = layer_styles()[0].1.clone();
        style.width = title_block::PROJECTION_LINE;
        style.weight = 15;
        layers.push(Layer {
            name: "projection_symbol".into(),
            style,
            items: ink,
        });
    }
    Ok(Drawing {
        page: plan.sheet.page,
        layers,
        // The attribution link covers the "drawn by" cell, as draftwright's does.
        link: plan
            .sheet
            .title_cells
            .iter()
            .find(|c| c.field == "designed_by")
            .map(|c| c.cell),
        text,
        annotations,
        sheet,
        problems: crate::output::undrawn(&undrawn),
        spec: Some(embeddable(&plan.spec)),
    })
}

/// A section view's furniture: ink (the hatch's first lines, then the cutting plane's), and
/// texts with where each is centred.
struct Furniture {
    ink: Vec<Item>,
    /// How many of the ink's first items are the hatch.
    hatch: usize,
    /// The cutting plane's chain line, end to end, when it is drawn.
    chain: Option<[[f64; 2]; 2]>,
    text: Vec<(String, [f64; 2])>,
}

/// Section view *v*'s furniture: its cut face hatched, its caption, and the cutting-plane line
/// with its arrows and letters on the view that marks it (draftwright's `_add_section_view`).
fn section_furniture(
    part: &Part,
    analysis: &Analysis,
    plan: &Plan,
    v: &PlannedView,
    (plane, cut): (&Plane, &Cut),
    projections: &mut Projections,
) -> Result<Furniture, Unresolved> {
    use crate::section::{HATCH_SPACING, hatch};
    let mut out = Furniture {
        ink: Vec::new(),
        hatch: 0,
        chain: None,
        text: Vec::new(),
    };
    let view = &View::new(v.direction, v.up).expect("a planned view");
    let (s, centre) = (v.scale, extent(analysis, view).1);
    let page: Vec<Vec<[f64; 2]>> = projections
        .cut(part, cut, plane)?
        .iter()
        .map(|l| {
            l.iter()
                .map(|p| {
                    let q = view.map(*p);
                    [
                        v.at[0] + s * (q[0] - centre[0]),
                        v.at[1] + s * (q[1] - centre[1]),
                    ]
                })
                .collect()
        })
        .collect();
    // Only material crossing the plane is hatched. Coplanar faces and stray cut
    // pieces must not flip parity or conceal a boundary inside a candidate span.
    let faces: Vec<usize> = part
        .solids
        .iter()
        .flat_map(|solid| solid.faces.iter().copied())
        .collect();
    let classifier = if faces.is_empty() {
        Classifier::new(part)
    } else {
        Classifier::for_faces(part, faces)
    };
    let n = plane.normal;
    let to_model = |q: [f64; 2], side: f64| {
        let (u, w) = (
            centre[0] + (q[0] - v.at[0]) / s,
            centre[1] + (q[1] - v.at[1]) / s,
        );
        let base = [0, 1, 2].map(|k| view.x[k] * u + view.y[k] * w);
        let along = (0..3)
            .map(|k| (cut.origin[k] - base[k]) * n[k])
            .sum::<f64>()
            / (0..3).map(|k| view.toward[k] * n[k]).sum::<f64>();
        [0, 1, 2].map(|k| base[k] + view.toward[k] * along + side * n[k] * 1e-2)
    };
    for [a, b] in hatch(&page, HATCH_SPACING, Some(&|_| true)) {
        let point = |t: f64| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
        let (p, q) = (to_model(a, 1.0), to_model(b, 1.0));
        let length = (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f64>().sqrt();
        let direction = [0, 1, 2].map(|k| (q[k] - p[k]) / length);
        let mut stops = vec![0.0, 1.0];
        for side in [-1.0, 1.0] {
            let hits = classifier
                .rays()
                .hits(to_model(a, side), direction, length)
                .ok_or_else(|| Unresolved {
                    faces: part
                        .solids
                        .iter()
                        .flat_map(|solid| solid.faces.iter().copied())
                        .collect(),
                    edges: Vec::new(),
                })?;
            stops.extend(hits.iter().map(|hit| (hit.t / length).clamp(0.0, 1.0)));
        }
        stops.sort_by(f64::total_cmp);
        stops.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for pair in stops.windows(2) {
            let middle = point((pair[0] + pair[1]) / 2.0);
            if (pair[1] - pair[0]) * length * s > 0.2
                && [-1.0, 1.0]
                    .into_iter()
                    .all(|side| classifier.classify(to_model(middle, side)) == State::In)
            {
                out.ink
                    .push(Item::Edge(Curve::Line(point(pair[0]), point(pair[1]))));
            }
        }
    }
    out.hatch = out.ink.len();
    let half_h = 0.5 * extent(analysis, view).0[1] * v.scale;
    let l = &cut.label;
    out.text.push((
        format!("SECTION {l}\u{2013}{l}"),
        [v.at[0], v.at[1] - half_h - 7.0],
    ));
    cutting_furniture(plan, cut, &mut out);
    Ok(out)
}

fn cutting_furniture(plan: &Plan, cut: &Cut, out: &mut Furniture) {
    use crate::section::cutting_plane;
    let l = &cut.label;
    if let Some(w) = plan
        .views
        .iter()
        .find(|w| w.id == cut.marked_in)
        .and_then(|v| v.line_work.iter().find(|w| w.kind == "cutting_plane"))
        && let Curve::Line(a, b) = w.curves[0]
    {
        let (ink, letters) = cutting_plane(a[0], b[0], a[1]);
        out.ink.extend(ink);
        out.chain = Some([a, b]);
        out.text.extend(letters.map(|at| (l.clone(), at)));
    }
}

/// Where the cutting-plane line of *cut* runs on the view *m* that marks it (page mm): from
/// `x0` to `x1` at height `y`, across the part's extent along the view's x and past it each
/// side. The renderer draws it there; the planner keeps interior labels off it.
pub(crate) fn cutting_line(
    analysis: &Analysis,
    m: &PlannedView,
    mv: &View,
    cut: &Cut,
) -> (f64, f64, f64) {
    let centre = extent(analysis, mv).1;
    let to_page = |p: [f64; 3]| {
        let q = mv.map(p);
        [
            m.at[0] + m.scale * (q[0] - centre[0]),
            m.at[1] + m.scale * (q[1] - centre[1]),
        ]
    };
    let [lo, hi] = analysis.envelope;
    let (a, b) = (
        to_page([lo[0], cut.origin[1], lo[2]]),
        to_page([hi[0], cut.origin[1], hi[2]]),
    );
    let overrun = crate::section::OVERRUN;
    (
        a[0].min(b[0]) - overrun,
        a[0].max(b[0]) + overrun,
        to_page(cut.origin)[1],
    )
}

/// A projected curve scaled by *s* about *centre* and moved to *at*.
fn placed(e: &Exact, s: f64, centre: [f64; 2], at: [f64; 2]) -> Exact {
    let m = |q: [f64; 2]| {
        [
            at[0] + s * (q[0] - centre[0]),
            at[1] + s * (q[1] - centre[1]),
        ]
    };
    match e.clone() {
        Exact::Line(a, b) => Exact::Line(m(a), m(b)),
        Exact::Conic {
            centre: c,
            radii,
            x_axis,
            y_axis,
            t0,
            t1,
        } => Exact::Conic {
            centre: m(c),
            radii: [radii[0] * s, radii[1] * s],
            x_axis,
            y_axis,
            t0,
            t1,
        },
        Exact::Nurbs {
            degree,
            poles,
            weights,
            knots,
            t0,
            t1,
        } => Exact::Nurbs {
            degree,
            poles: poles.into_iter().map(m).collect(),
            weights,
            knots,
            t0,
            t1,
        },
    }
}

/// *spec* as the JSON a drawing carries: `<`, `>` and `&` written as `\u` escapes, so no markup
/// (an end tag, a CDATA section's end, an entity) can be read inside it.
fn embeddable(spec: &Spec) -> String {
    serde_json::to_string(spec)
        .expect("a spec serialises")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

/// The spec embedded in a rendered SVG, DXF or PDF, so a drawing can be resumed from the file
/// alone: `None` when the drawing carries none. The format is told by the file's signature; a
/// file that is none of the three, or a spec that does not read, is an error saying why.
pub fn embedded_spec(file: &[u8]) -> Result<Option<Spec>, String> {
    let text = String::from_utf8_lossy(file);
    let start = text.trim_start_matches('\u{feff}').trim_start();
    let json = if crate::png::is_png(file) {
        crate::png::spec(file)?
    } else if file.starts_with(b"%PDF-") {
        pdf_spec(&text)?
    } else if start.starts_with('<') {
        svg_spec(&text)?
    } else if start
        .lines()
        .next()
        .is_some_and(|l| !l.trim().is_empty() && l.trim().parse::<i32>().is_ok())
    {
        // A DXF opens with a group code.
        dxf_spec(&text)
    } else {
        return Err("it is not an SVG, DXF, PDF or PNG drawing".into());
    };
    json.map(|j| {
        serde_json::from_str(&j)
            .map_err(|e| format!("it carries a draftwright spec that is not valid: {e}"))
    })
    .transpose()
}

/// The JSON of a DXF's spec: the 999 comments after the marker, joined. The marker counts only
/// as a whole comment line.
fn dxf_spec(text: &str) -> Option<String> {
    let i = ["999\r\n", "999\n"].iter().find_map(|code| {
        let i = text.find(&format!("{code}{}", crate::output::DXF_SPEC))?;
        let end = i + code.len() + crate::output::DXF_SPEC.len();
        matches!(text.as_bytes().get(end), Some(b'\r' | b'\n')).then_some(i + code.len())
    })?;
    let mut lines = text[i..]
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l));
    lines.next();
    let mut json = String::new();
    while let (Some(code), Some(piece)) = (lines.next(), lines.next())
        && code.trim() == "999"
    {
        json += piece;
    }
    Some(json)
}

/// The JSON of an SVG's `draftwright:spec` element: its character data, CDATA sections taken as
/// they are and the rest with XML's predefined entities undone (a spec a re-saving editor wrote
/// without CDATA still reads).
fn svg_spec(text: &str) -> Result<Option<String>, String> {
    let Some(i) = text.find("<draftwright:spec") else {
        return Ok(None);
    };
    let broken = || "its draftwright:spec element is not closed".to_string();
    let mut rest = &text[i + text[i..].find('>').ok_or_else(broken)? + 1..];
    let mut json = String::new();
    loop {
        if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r.find("]]>").ok_or_else(broken)?;
            json += &r[..end];
            rest = &r[end + "]]>".len()..];
        } else if rest.starts_with("</draftwright:spec>") {
            return Ok(Some(json));
        } else if rest.starts_with('<') || rest.is_empty() {
            return Err(broken());
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            json += &rest[..end]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&amp;", "&");
            rest = &rest[end..];
        }
    }
}

/// The JSON of a PDF's `/DraftwrightSpec` document information string.
fn pdf_spec(text: &str) -> Result<Option<String>, String> {
    let broken = || "its /DraftwrightSpec string does not read".to_string();
    if let Some(i) = text.find("/DraftwrightSpec (") {
        return crate::output::pdf_unescape(&text[i + "/DraftwrightSpec (".len()..])
            .map(Some)
            .ok_or_else(broken);
    }
    // pdf-writer writes a string that is not all ASCII in hex.
    let Some(i) = text.find("/DraftwrightSpec <") else {
        return Ok(None);
    };
    let i = i + "/DraftwrightSpec <".len();
    let digits: Vec<u8> = text[i..i + text[i..].find('>').ok_or_else(broken)?]
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let bytes = digits
        .chunks(2)
        .map(|d| u8::from_str_radix(std::str::from_utf8(d).ok()?, 16).ok())
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(broken)?;
    String::from_utf8(bytes).map(Some).map_err(|_| broken())
}

mod projection_store;

pub mod projected_ink;
