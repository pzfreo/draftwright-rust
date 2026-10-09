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

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use haecceity::Part;
use haecceity::classify::{Classifier, State};
use haecceity::hlr::{self, Exact, Plane, Unresolved, View, project, project_section};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bridge::{Feature, Recognition};
use crate::dims::{Omitted, PlannedDimension};
use crate::layout::{self, Corner, Fallback, MARGIN, Principals, Request, Scaled};
use crate::notes::{NoteSpec, NotesBlock, PlannedNote};
use crate::output::{
    Curve, Drawing, Drawn, DrawnField, DrawnView, HAlign, Item, Kind, Layer, Sheet, TextRun,
    VAlign, items_bbox, layer_styles, projected,
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
pub const SPEC_SCHEMA: &str = "draftwright.spec/1";
pub const PLAN_SCHEMA: &str = "draftwright.plan/1";

/// Which file, and which reading of it, a document was made from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_sha256: String,
    pub part: String,
    pub face_count: usize,
    pub loader: String,
}

/// What the part is: regenerable from its STEP file, never edited.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub schema: String,
    pub binding: Binding,
    /// The part's box in its own frame, mm.
    pub envelope: [[f64; 3]; 2],
    /// Where the features came from; absent when none were recognised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recognition: Option<Recognition>,
    #[serde(default)]
    pub features: Vec<Feature>,
    /// The isometric's projected box (visible and hidden edges), `[x0, y0, x1, y1]` mm at 1:1,
    /// about the projection of the envelope's centre: what the isometric is fitted by. All zero
    /// when the projection has no edges (the plan then reports `iso_projection_empty`).
    pub iso_bounds: [f64; 4],
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
    /// Per standard view (`front`, `plan`, `side`, `iso`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, ViewSpec>,
    /// Where the dimensions come from: `auto` (the defaults, less what `dimensions` omits) or
    /// `authored` (exactly the dimensions listed, a complete set: ADR4-5).
    #[serde(skip_serializing_if = "DimensionSource::is_auto")]
    pub dimension_source: DimensionSource,
    /// Per dimension, by its ID (`d:<feature>/<parameter>@<view>`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dimensions: BTreeMap<String, DimensionSpec>,
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
            views: BTreeMap::new(),
            dimension_source: DimensionSource::Auto,
            dimensions: BTreeMap::new(),
            features: BTreeMap::new(),
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
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ViewSpec {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub omit: bool,
}

/// What the sheet will say.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub schema: String,
    pub binding: Binding,
    pub sheet: PlannedSheet,
    pub views: Vec<PlannedView>,
    /// Dimensions and callouts, each with its ID, value, label, default flag and basis.
    #[serde(default)]
    pub dimensions: Vec<PlannedDimension>,
    /// Dimensions the spec leaves out, and why (ADR4-8: an authored omission says so).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omitted: Vec<Omitted>,
    /// General notes, each with its ID, text, rows, default flag, basis and place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<PlannedNote>,
    /// The block the notes are drawn in, when it has a place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_block: Option<NotesBlock>,
    /// Corner-block tables, each with its ID, cells, default flag, basis and place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<PlannedTable>,
    /// The part's AP242 geometric tolerances and datums as feature control frames and datum
    /// feature symbols, each with its ID, view, compartments, basis and place
    /// ([`crate::dims::gdt`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gdt: Vec<crate::dims::gdt::PlannedGdt>,
    pub problems: Vec<Problem>,
    /// The spec the plan was made from, embedded in every output.
    pub spec: Spec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedSheet {
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
    pub title: BTreeMap<String, String>,
    /// The title block's cells, each with its box and the value drawn in it (ISSUE-1585-a: a
    /// supplied field is recorded as drawn, in its own cell).
    #[serde(default)]
    pub title_cells: Vec<TitleCell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedView {
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
    /// A section view's cutting plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<Cut>,
}

/// A section's cutting plane: the material on the side `normal` points to is kept and seen from
/// the side removed. The plane is marked on the `marked_in` view, its ends lettered `label`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    pub label: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub marked_in: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub code: String,
    pub severity: String,
    pub message: String,
    pub subjects: Vec<String>,
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
fn scale_name(s: f64) -> String {
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

/// Reads the STEP file at *path*, with its features from *recognition*: Python quiddity's JSON
/// for the same file, written by `tools/recognise.py` ([`crate::bridge`]). Refused when the
/// kernel did not resolve a face or edge the isometric draws.
pub fn analyse_with(
    path: &Path,
    recognition: Option<&Path>,
) -> Result<(Part, Analysis), Unanalysed> {
    let bytes = std::fs::read(path)
        .map_err(|e| unanalysed(false, format!("cannot read {}: {e}", path.display())))?;
    let part = haecceity::read_step(&bytes).map_err(|e| {
        // A file that does not parse is not STEP; one that parses but holds what the kernel
        // does not model (or lost on reading) is the kernel's failure.
        let failed = !matches!(e, haecceity::step::StepError::Parse(_));
        unanalysed(
            failed,
            format!("cannot read {} as STEP: {e}", path.display()),
        )
    })?;
    let b = part.bounds();
    let (level_supports, risers) = crate::dims::levels::evidence(&part);
    let mut analysis = Analysis {
        schema: ANALYSIS_SCHEMA.into(),
        binding: Binding {
            source_sha256: Sha256::digest(&bytes)
                .iter()
                .map(|x| format!("{x:02x}"))
                .collect(),
            part: path.file_stem().map_or(String::new(), |s| {
                s.to_string_lossy().trim_end_matches(".step").to_string()
            }),
            face_count: part.faces.len(),
            loader: LOADER.into(),
        },
        envelope: [b.min, b.max],
        recognition: None,
        features: Vec::new(),
        iso_bounds: [0.0; 4],
        levels: crate::demand::face_levels(&part),
        level_supports,
        risers,
        principal_profiles: crate::lint::profiles::evidence(&part),
        unrecognised: Vec::new(),
        pmi: crate::pmi::attach(&bytes, &part),
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

/// Takes *analysis*'s features from Python quiddity's recognition JSON for the same file
/// ([`crate::bridge`]).
pub fn recognise(part: &Part, analysis: &mut Analysis, json: &str) -> Result<(), String> {
    let (recognition, features) = crate::bridge::read(part, &analysis.binding.source_sha256, json)?;
    analysis.recognition = Some(recognition);
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
    let (part, mut analysis) = analyse_with(path, None)?;
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
const HARD_LAYOUT: [&str; 6] = [
    "annotation_off_sheet",
    "annotation_overlap",
    "annotation_ink_overlap",
    "title_field_overflow",
    "page_fit_uncertain",
    "views_do_not_fit",
];

/// A plan's findings for the recovery. The port drops nothing for want of room (an item that
/// collides is placed and reported; one no shown view carries is kept unplaced, which no sheet
/// or scale changes), so its completeness tier reads the one required loss draftwright's
/// blockers have that a scale decides: hole locations too close to dimension legibly
/// ([`layout::crowded_locations`], draftwright's `location_ref_dropped`), counted per view and
/// axis, a pattern's centre ordinate apart (draftwright keeps it whatever its spacing).
fn findings(plan: &Plan) -> layout::Findings {
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
        hard: plan
            .problems
            .iter()
            .filter(|p| HARD_LAYOUT.contains(&p.code.as_str()) && p.severity != "info")
            .count(),
        dropped: axes
            .values()
            .map(|o| layout::crowded_locations(o, scale))
            .sum(),
    }
}

/// What a plan's recovery findings are, in words.
fn said(f: layout::Findings) -> String {
    let hard = match f.hard {
        1 => "1 hard layout finding".to_string(),
        n => format!("{n} hard layout findings"),
    };
    let crowded = match f.dropped {
        1 => "1 hole location too close to read".to_string(),
        n => format!("{n} hole locations too close to read"),
    };
    match (f.hard, f.dropped) {
        (0, 0) => "valid and complete".into(),
        (_, 0) => hard,
        (0, _) => crowded,
        _ => format!("{hard}, {crowded}"),
    }
}

/// draftwright's bounded layout recovery on a finished plan (ISSUE-1299-a, [`layout::recover`]):
/// a plan with a hard layout failure is re-planned at the nearest smaller, then the nearest
/// larger standard scale on its sheet, then on each larger standard sheet; one without, whose
/// hole locations are too close to read, at the next two larger scales, then each larger sheet;
/// the first valid and complete one is taken. A scale the spec names is never re-planned
/// (draftwright resolves an explicit scale apart), a sheet it names is never left, and a valid,
/// complete plan is kept. The change and what was tried are said (`layout_replanned`), as is a
/// recovery that found nothing better (`layout_recovery_exhausted`). Each try is a whole
/// [`plan`] (no projection): six at most.
///
/// Unlike draftwright's retries (`try_scales_on_selected_page`, `try_larger_standard_pages`),
/// which rebuild with the first drawing's settled arrangement and principal views, each try
/// here is planned afresh at the sheet and scale named, so its arrangement and view set are
/// chosen again for them. Not ported: draftwright's other candidate gates (a retained recovery
/// detail, a recognised-requirement regression, the structural replay), whose inputs the port
/// does not plan; a principal view too large for the page is the try's `views_do_not_fit`.
///
/// [`plan`] does not call it yet (an integration decision: docs/next-session.md); applying it
/// there means re-planning through the unrecovered plan, never through itself.
pub fn recover(analysis: &Analysis, spec: &Spec, first: Plan) -> Result<Plan, Invalid> {
    if spec.sheet.scale.is_some() {
        return Ok(first);
    }
    let found = findings(&first);
    let mut tried = Vec::new();
    let mut last = None;
    let attempts = layout::recover(
        first.sheet.scale_factor,
        first.sheet.page,
        spec.sheet.size.is_some(),
        found,
        &mut |retry| {
            let mut s = spec.clone();
            match retry {
                layout::Retry::Scale(k) => {
                    s.sheet.size = Some(first.sheet.size.clone());
                    s.sheet.scale = Some(scale_name(k));
                }
                layout::Retry::Sheet(n) => s.sheet.size = Some(n.into()),
            }
            let Ok(p) = plan(analysis, &s) else {
                let at = s.sheet.scale.as_deref().unwrap_or("its scale");
                tried.push(format!(
                    "{} at {at} (could not be planned)",
                    s.sheet.size.as_deref().unwrap_or_default()
                ));
                return None;
            };
            let f = findings(&p);
            tried.push(format!(
                "{} at {} ({})",
                p.sheet.size,
                p.sheet.scale,
                said(f)
            ));
            last = Some(p);
            Some(f)
        },
    );
    if attempts.is_empty() {
        return Ok(first);
    }
    let codes: BTreeSet<&str> = first
        .problems
        .iter()
        .filter(|p| HARD_LAYOUT.contains(&p.code.as_str()) && p.severity != "info")
        .map(|p| p.code.as_str())
        .collect();
    let why = if found.hard > 0 {
        format!(
            "{} at {} has {} ({})",
            first.sheet.size,
            first.sheet.scale,
            said(found),
            codes.into_iter().collect::<Vec<_>>().join(", ")
        )
    } else {
        format!(
            "{} at {} has {}",
            first.sheet.size,
            first.sheet.scale,
            said(found)
        )
    };
    let scales = if found.hard > 0 {
        "the nearest smaller, then larger standard scale"
    } else {
        "the next two larger standard scales"
    };
    let order = if spec.sheet.size.is_some() {
        format!("{scales} on the sheet the spec names")
    } else {
        format!("{scales} on the same sheet, then each larger standard sheet")
    };
    let tried = tried.join(", ");
    match (attempts.last().is_some_and(|a| a.taken), last) {
        (true, Some(mut p)) => {
            let to = format!("{} at {}", p.sheet.size, p.sheet.scale);
            p.sheet.basis = format!(
                "draftwright's layout recovery: {why}, so {order} was tried, the first valid and complete one taken: {to}"
            );
            p.sheet.default = true;
            p.spec = first.spec;
            p.problems.push(Problem {
                id: "p:layout_replanned:sheet".into(),
                code: "layout_replanned".into(),
                severity: "info".into(),
                message: format!(
                    "The sheet was re-planned: {why}. Tried {tried}; {to} is used. Name a sheet and scale to keep another."
                ),
                subjects: vec!["sheet".into()],
            });
            Ok(p)
        }
        _ => {
            let mut p = first;
            p.problems.push(Problem {
                id: "p:layout_recovery_exhausted:sheet".into(),
                code: "layout_recovery_exhausted".into(),
                severity: "info".into(),
                message: format!(
                    "{why}; re-planning ({order}) found nothing valid and complete: tried {tried}. The first choice is kept."
                ),
                subjects: vec!["sheet".into()],
            });
            Ok(p)
        }
    }
}

/// Plans the sheet: which views, where, at what scale and on what sheet, as draftwright's layout
/// does ([`layout`]): each principal view is a block of its box and the annotation bands reserved
/// around it, packed third-angle; the isometric takes the largest space left. Each choice the
/// spec leaves open is marked as a default, with the reason for it.
///
/// The bands, the turned-part view set and section A–A come from the features
/// ([`crate::demand`]); the height ladder is sized for the steps legible at the scale chosen.
/// The scale and sheet are draftwright's first choice, before it re-plans on its finished
/// drawing's findings. That recovery is [`recover`], not yet applied here: on the captured parts
/// it moves only ctc01, for a label the port's placement leaves off the sheet
/// (docs/next-session.md, ISSUE-1299-a).
pub fn plan(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
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
    if spec.schema != SPEC_SCHEMA {
        return Err(invalid(
            format!(
                "The spec is {:?}, which this draftwright does not read.",
                spec.schema
            ),
            &[SPEC_SCHEMA],
        ));
    }
    title_block::check(&spec.sheet.title)?;
    crate::notes::check(&spec.notes)?;
    crate::tables::check(&spec.tables)?;
    let mut problems = Vec::new();
    // The spec the plan follows: across a revision, without the entries that cannot be trusted.
    let mut effective = spec.clone();
    let mut inactive = Vec::new();
    if let Some(b) = &spec.binding
        && b.source_sha256 != analysis.binding.source_sha256
    {
        // Until rebinding (ADR R3) lands, a feature's ID is rank-based and may name another
        // feature in this revision: its entries are kept in the spec but inactive (R3 invariant
        // 3), and listed. The sheet, the views and the envelope's dimensions carry over.
        inactive = spec
            .dimensions
            .keys()
            .filter(|id| id.starts_with("d:f:"))
            .chain(spec.features.keys())
            .cloned()
            .collect::<Vec<_>>();
        effective.dimensions.retain(|id, _| !id.starts_with("d:f:"));
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
        });
        if !inactive.is_empty() {
            problems.push(Problem {
                id: "p:spec_entries_inactive:part".into(),
                code: "spec_entries_inactive".into(),
                severity: "warning".into(),
                message: format!(
                    "The spec was made for a different revision of {}, and its entries for features ({}) cannot be matched to this one's until revisions are rebound: they are kept in the spec but not applied, and the spec stays bound to its revision while they remain. Remove them (then set what is wanted by this revision's IDs), or edit the spec's binding to this revision's to apply them as they are.",
                    b.part,
                    inactive.join(", ")
                ),
                subjects: inactive.clone(),
            });
        }
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
            subjects: analysis
                .unrecognised
                .iter()
                .map(|u| u.family.clone())
                .collect(),
        });
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
    // A turned part drops the view that repeats its profile, unless the spec asks for it, or
    // omits the profile view it keeps (front): the dropped view is then the only profile left.
    let omitted = |name: &str| spec.views.get(name).is_some_and(|v| v.omit);
    let reduction = crate::demand::turned_views(&demand)
        .filter(|(kept, dropped, _)| !spec.views.contains_key(*dropped) && !omitted(kept[0]));
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
    demand.lanes = crate::dims::lanes(analysis, &shown_views);
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

    let [lo, hi] = analysis.envelope;
    // Section A-A is marked on the plan and sits in the front's row.
    let section = demand
        .section
        .clone()
        .filter(|_| shown(SECTION) && principals.plan && (principals.front || principals.side));
    // A section left out is said, with why and what would bring it back (draftwright records
    // every section decision: ISSUE-1190-b); one the author omits needs no saying.
    let section_not_shown = |why: String| Problem {
        id: format!("p:section_not_shown:v:{SECTION}"),
        code: "section_not_shown".into(),
        severity: "warning".into(),
        message: format!("Section A\u{2013}A is not drawn: {why}."),
        subjects: vec![format!("v:{SECTION}")],
    };
    match &demand.section {
        Some(_) if section.is_none() && shown(SECTION) => problems.push(section_not_shown(if !principals.plan {
            let how = if omitted("plan") {
                "the spec omits it; remove views/plan/omit to show it"
            } else {
                "it is left out as the turned part's repeated profile; set views/plan to {} to keep it"
            };
            format!(
                "a z-axis hole with a counterbore, spotface or bottom needs it, but its cutting plane is marked on the plan view, and {how}"
            )
        } else {
            "it sits in the row of the front and side views, and the spec omits both; remove views/front/omit or views/side/omit to show it".into()
        })),
        None if spec.views.get(SECTION).is_some_and(|v| !v.omit) => problems.push(section_not_shown(
            "the spec asks for it, but the part has no z-axis hole with a counterbore, spotface or bottom whose profile it would show, so there is no plane to cut; remove views/section_aa".into(),
        )),
        _ => {}
    }
    let mut request = Request::new(
        [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]],
        principals,
        shown("iso"),
    );
    request.sections = usize::from(section.is_some());
    // The tables are reserved towards their corners and the general notes block top right, in
    // draftwright's order, unless only a fallback would make room for them: then each is placed
    // where it can be, or reported (crate::tables, crate::notes).
    let notes_size = crate::notes::size(&spec.notes);
    request.required_tables = crate::tables::sizes(&spec.tables);
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
    let size = layout::PAGES
        .iter()
        .find(|(_, w, h)| [*w, *h] == pick.page)
        .map_or_else(
            || format!("{}x{}", pick.page[0], pick.page[1]),
            |p| p.0.into(),
        );
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
        }),
        Some(Fallback::Overflow) => problems.push(Problem {
            id: "p:views_do_not_fit:sheet".into(),
            code: "views_do_not_fit".into(),
            severity: "error".into(),
            message: format!("The views do not fit {size} at {scale}."),
            subjects: vec!["sheet".into()],
        }),
        Some(Fallback::Uncertain) => problems.push(Problem {
            id: "p:page_fit_uncertain:sheet".into(),
            code: "page_fit_uncertain".into(),
            severity: "warning".into(),
            message: format!("The views may not fit {size} at {scale}, as the spec asks."),
            subjects: vec!["sheet".into()],
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
                    "third-angle: the plan sits above the front, past both views' bands".into(),
                ),
                "side" => (
                    g.side,
                    sv.footprint(g.side[0], g.side[1]),
                    pick.scale,
                    "third-angle: the right side sits right of the front, past both views' bands"
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
                id: format!("v:{name}"),
                direction: *direction,
                up: *up,
                at,
                scale,
                area,
                default: !spec.views.contains_key(*name),
                basis: basis + &turned(name),
                cut: None,
            }
        })
        .collect();
    if let (Some(cut), Some(area)) = (&section, g.section_boxes.first()) {
        views.push(PlannedView {
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
            cut: Some(Cut {
                label: "A".into(),
                origin: [0.5 * (lo[0] + hi[0]), cut.cut_y, 0.5 * (lo[2] + hi[2])],
                normal: [0.0, 1.0, 0.0],
                marked_in: "v:plan".into(),
            }),
        });
    } else if section.is_some() {
        problems.push(section_not_shown(
            "the layout made no place for it on the sheet".into(),
        ));
    }
    let mut title = spec.sheet.title.clone();
    title
        .entry("title".into())
        .or_insert_with(|| analysis.binding.part.to_uppercase());
    title
        .entry("number".into())
        .or_insert_with(|| "DWG-001".into());
    let title_cells = title_block::cells(g.title_block, &title, &size);
    problems.extend(title_block::problems(&title_cells, pick.page));
    let sheet = PlannedSheet {
        size,
        page: pick.page,
        scale,
        scale_factor: pick.scale,
        arrangement: pick.arrangement.name().into(),
        title_block: g.title_block,
        default: spec.sheet.size.is_none() || spec.sheet.scale.is_none(),
        basis,
        title,
        title_cells,
    };
    let (dimensions, mut omitted, more) = crate::dims::plan(analysis, spec, &sheet, &views)?;
    problems.extend(more);
    // The part's PMI, placed beyond the dimensions; the notes and tables keep clear of both.
    let (gdt, more) = crate::dims::gdt::plan(analysis, &sheet, &views, &dimensions);
    problems.extend(more);
    let mut ink = dimensions.clone();
    ink.extend(gdt.iter().filter_map(crate::dims::gdt::proxy));
    let (notes, notes_block, more) =
        crate::notes::plan(&spec.notes, analysis, &sheet, &views, &ink);
    problems.extend(more);
    let (tables, more) = crate::tables::plan(
        &spec.tables,
        analysis,
        &sheet,
        &views,
        &ink,
        notes_block.as_ref().map(|b| b.cells),
    );
    problems.extend(more);
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
    Ok(Plan {
        schema: PLAN_SCHEMA.into(),
        binding: analysis.binding.clone(),
        sheet,
        views,
        dimensions,
        omitted,
        notes,
        notes_block,
        tables,
        gdt,
        problems,
        spec,
    })
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
            let Some(object) = at.as_object_mut() else {
                return Err(invalid(format!("{key:?} is not inside an object."), &[]));
            };
            at = if kind != "remove" {
                object
                    .entry(key.to_string())
                    .or_insert_with(|| serde_json::json!({}))
            } else {
                object.get_mut(key).ok_or_else(absent)?
            };
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
    crate::dims::check_tolerances(&spec)?;
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
    // Section furniture (hatch, cutting plane), and its text: what, where.
    let mut furniture: Vec<Item> = Vec::new();
    let mut captions: Vec<(String, [f64; 2])> = Vec::new();
    // What lint's sheet checks read: each view's visible box, and each thing drawn, by name.
    let mut sheet = Sheet::default();
    let mut caption_names: Vec<String> = Vec::new();
    let mut layers: Vec<Layer> = layer_styles()
        .into_iter()
        .map(|(name, style)| Layer {
            name: name.into(),
            style,
            items: Vec::new(),
        })
        .collect();
    let mut undrawn = BTreeMap::new();
    for v in &plan.views {
        let view = View::new(v.direction, v.up).expect("a planned view");
        let s = v.scale;
        let centre = extent(analysis, &view).1;
        let place = |q: [f64; 2]| {
            [
                v.at[0] + s * (q[0] - centre[0]),
                v.at[1] + s * (q[1] - centre[1]),
            ]
        };
        let plane = v.cut.as_ref().and_then(|c| Plane::new(c.origin, c.normal));
        let pieces = match &plane {
            Some(plane) => project_section(part, &view, plane),
            None => project(part, &view),
        }?;
        let visible_from = layers[0].items.len();
        for piece in pieces {
            let mut moved = piece.clone();
            moved.points = piece.points.iter().map(|q| place(*q)).collect();
            moved.exact = piece.exact.as_ref().map(|e| placed(e, s, centre, v.at));
            match projected(&moved) {
                Some(curve) => layers[usize::from(!piece.visible)]
                    .items
                    .push(Item::Edge(curve)),
                None => *undrawn.entry(v.id.clone()).or_default() += 1,
            }
        }
        let visible = &layers[0].items[visible_from..];
        if let Some(bbox) = items_bbox(visible) {
            sheet.views.push(DrawnView {
                name: v.id.clone(),
                bbox,
                edges: visible
                    .iter()
                    .filter_map(|i| match i {
                        Item::Edge(c) => Some(c.clone()),
                        Item::Face(_) => None,
                    })
                    .collect(),
            });
        }
        if let (Some(plane), Some(cut)) = (&plane, &v.cut) {
            let f = section_furniture(part, analysis, plan, v, (plane, cut))?;
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
            caption_names.extend(
                [
                    "section_caption".into(),
                    format!("section_{l}_left"),
                    format!("section_{l}_right"),
                ]
                .into_iter()
                .take(f.text.len()),
            );
            furniture.extend(f.ink);
            captions.extend(f.text);
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
        let from = layers[2].items.len();
        let drawn = crate::dims::render(std::slice::from_ref(d), &mut layers[2].items, &mut text);
        if let (Some(a), Some(ink)) = (drawn.first(), items_bbox(&layers[2].items[from..])) {
            let placed = d.placed.as_ref();
            sheet.items.push(Drawn {
                name: a.name.clone(),
                kind: Kind::Dimension,
                ink,
                label: placed.map(|p| p.label_box),
                lines: placed.map(|p| p.lines.clone()).unwrap_or_default(),
                // A callout's leader runs from its tip to its elbow, then along its shoulder.
                elbow: placed
                    .filter(|_| d.kind == "callout")
                    .and_then(|p| p.lines.first())
                    .map(|l| l[1]),
                interior: false,
            });
        }
        annotations.extend(drawn);
    }
    // The feature control frames and datum feature symbols, likewise.
    for g in &plan.gdt {
        let from = layers[2].items.len();
        let drawn = crate::dims::gdt::render(g, &mut layers[2].items, &mut text);
        if let (Some(a), Some(ink), Some(p)) = (
            drawn,
            items_bbox(&layers[2].items[from..]),
            g.placed.as_ref(),
        ) {
            sheet.items.push(Drawn {
                name: a.name.clone(),
                kind: Kind::Gdt,
                ink,
                label: Some(p.label_box),
                lines: p.lines.clone(),
                elbow: p.lines.first().map(|l| l[1]),
                interior: false,
            });
            annotations.push(a);
        }
    }
    // Glyphs *laid* out drawn moved to *at*, and the *run* (text, font file, size), if any, as
    // invisible text centred there.
    let mut label = |laid: Laid,
                     at: [f64; 2],
                     layer: &mut Vec<Item>,
                     run: Option<(&str, &str, f64)>| {
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
        if let Some((s, font, size)) = run {
            text.push(TextRun {
                text: s.into(),
                at,
                size,
                rotation: 0.0,
                font: font.into(),
                h_align: HAlign::Center,
                v_align: VAlign::Middle,
            });
        }
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
    for ((s, at), name) in captions.iter().zip(&caption_names) {
        let size = crate::section::FONT_SIZE;
        let run = (s.as_str(), "IBMPlexMono-Regular.ttf", size);
        let from = dims.len();
        label(font.text(s, size), *at, dims, Some(run));
        sheet
            .items
            .extend(labelled(name, Kind::Caption, &dims[from..]));
    }
    // Each cell: its dividers, its label at its lower left, and its value (none drawn for a
    // field not supplied) centred in it, whole, and searchable. The ink is the font-safe value.
    let title_from = dims.len();
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
            label(
                title_block::lay(v, title_block::TEXT),
                centre,
                dims,
                Some(run),
            );
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
    let notes_from = dims.len();
    if let Some(block) = &plan.notes_block {
        for [a, b] in crate::notes::rules(block) {
            dims.push(Item::Edge(Curve::Line(a, b)));
        }
        for (row, at) in crate::notes::texts(block, &plan.notes) {
            let run = (row.trim_start(), crate::dims::FONT, crate::notes::TEXT);
            let laid = font.text(&crate::notes::ink(row), crate::notes::TEXT);
            label(laid, at, dims, Some(run));
        }
    }
    if let Some(ink) = items_bbox(&dims[notes_from..]) {
        sheet.items.push(Drawn {
            name: "general_notes".into(),
            kind: Kind::Notes,
            ink,
            label: None,
            lines: Vec::new(),
            elbow: None,
            interior: false,
        });
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
    Ok(Drawing {
        page: plan.sheet.page,
        layers,
        link: None,
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
) -> Result<Furniture, Unresolved> {
    use crate::section::{HATCH_SPACING, OVERRUN, cutting_plane, hatch};
    let mut out = Furniture {
        ink: Vec::new(),
        hatch: 0,
        chain: None,
        text: Vec::new(),
    };
    let view = &View::new(v.direction, v.up).expect("a planned view");
    let (s, centre) = (v.scale, extent(analysis, view).1);
    let page: Vec<Vec<[f64; 2]>> = hlr::section(part, plane)?
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
    // Each hatch stretch is kept where the part has material just behind the cut, on the kept
    // side: a face lying in the cutting plane leaves a doubled piece in the cut's boundary.
    let classifier = Classifier::new(part);
    let n = cut.normal;
    let inside = |q: [f64; 2]| {
        let (u, w) = (
            centre[0] + (q[0] - v.at[0]) / s,
            centre[1] + (q[1] - v.at[1]) / s,
        );
        let base = [0, 1, 2].map(|k| view.x[k] * u + view.y[k] * w);
        let along = (0..3)
            .map(|k| (cut.origin[k] - base[k]) * n[k])
            .sum::<f64>()
            / (0..3).map(|k| view.toward[k] * n[k]).sum::<f64>();
        let p = [0, 1, 2].map(|k| base[k] + view.toward[k] * along + n[k] * 1e-2);
        classifier.classify(p) == State::In
    };
    for [a, b] in hatch(&page, HATCH_SPACING, Some(&inside)) {
        out.ink.push(Item::Edge(Curve::Line(a, b)));
    }
    out.hatch = out.ink.len();
    let half_h = 0.5 * extent(analysis, view).0[1] * v.scale;
    let l = &cut.label;
    out.text.push((
        format!("SECTION {l}\u{2013}{l}"),
        [v.at[0], v.at[1] - half_h - 7.0],
    ));
    if let Some(m) = plan.views.iter().find(|w| w.id == cut.marked_in)
        && let Some(mv) = View::new(m.direction, m.up)
    {
        let centre = extent(analysis, &mv).1;
        let to_page = |p: [f64; 3]| {
            let q = mv.map(p);
            [
                m.at[0] + m.scale * (q[0] - centre[0]),
                m.at[1] + m.scale * (q[1] - centre[1]),
            ]
        };
        let [lo, hi] = analysis.envelope;
        // The line runs across the part's extent along the view's x, past it each side.
        let (a, b) = (
            to_page([lo[0], cut.origin[1], lo[2]]),
            to_page([hi[0], cut.origin[1], hi[2]]),
        );
        let (x0, x1) = (a[0].min(b[0]) - OVERRUN, a[0].max(b[0]) + OVERRUN);
        let y = to_page(cut.origin)[1];
        let (ink, letters) = cutting_plane(x0, x1, y);
        out.ink.extend(ink);
        out.chain = Some([[x0, y], [x1, y]]);
        out.text.extend(letters.map(|at| (l.clone(), at)));
    }
    Ok(out)
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
