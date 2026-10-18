//! Dimensions and hole callouts: what the sheet states about sizes, planned from the Analysis's
//! features and drawn only from the plan (ADR R1 invariant 2).
//!
//! This first cut covers:
//! - **the overall extents**: width, depth and height of a prismatic part; the diameter and the
//!   axial length of a part that is turned whole;
//! - **whole-part polygonal stock**: its size across flats and its length, in place of the extents;
//! - **hole callouts**: draftwright's compound grammar (`4× ⌀10 THRU ⌴ ⌀16 ↧ 5 (2×2)`), with
//!   identical holes grouped and counted, and quiddity's hole patterns called out as patterns;
//! - **double-D bores**: one compound major-diameter and across-flats callout;
//! - **hole locations** from the part's box, in the view that looks along each hole, and the
//!   pitches that locate a pattern's members;
//! - **through slots**: width, length, an obround's end radius, and location along the slot;
//!   **slot patterns** by one `N× SLOT W × L` callout with their pitches and centre;
//!   **blind slots** (rectangular and round-bottom section recesses) by one callout each;
//! - **section recesses**: pockets, hexagonal pockets and circular seats by one callout each,
//!   with their locations, and open channels by their width ([`recesses`]);
//! - **bosses**: diameter (or size across flats for a polygonal boss), height and location;
//! - **fillet and chamfer callouts** (`4× R3`, `C1`, `4.5 × 42.1°`) where the edge is seen end-on,
//!   or in the profile of a turned part;
//! - **planar steps**: a through step's two legs, a circular blind step's `R × DEEP` and a
//!   paired-ramp step's `2× θ° × RUN` callouts, in the view along the step's run;
//! - **the height ladder** of a prismatic part: its face levels' heights from its base, in the
//!   front view, and its shoulders' positions along x and y;
//! - **raised pads**: footprint, `HIGH` height callout and location, in the view down on each
//!   ([`pads`]);
//! - **free-direction slots**: one `ORIENTED SLOT W WIDE × L LONG` callout led to the rim
//!   ([`oriented_slots`]).
//!
//! Each is a [`PlannedDimension`] with a stable ID, `d:<feature>/<parameter>`, the view that
//! carries it, its value,
//! its printed terms and label, the measurements it states, a default flag and a basis. A
//! tolerance or fit the spec gives reaches the label verbatim ([`tolerance`]), before placement.
//! Each quantity is then stated once (ARCH0009-6): coincident locations merge
//! ([`holes::consolidate`]), and so do the same size of several features across one span
//! ([`share`]: parallel hole rows' pitch, and a slot width, slot length or boss height, counted).
//!
//! **Placement is simple and honest**, not draftwright's solver: linear dimensions stack in lanes
//! below (horizontal) and right of (vertical) their view, sizes nearest, then locations by
//! distance, then the overall extent outermost. A label another item's ink runs through then
//! moves along its own line to the nearest clear place, if there is one. Callouts are placed
//! after, around the dimensions, by draftwright's leader placement ([`leaders`]): a short leader
//! from the feature's rim to an elbow just outside its view, a horizontal shoulder, assigned
//! jointly to cross as little as it can. Whatever still collides (labels with labels, lines through labels, labels over views, ink off
//! the sheet, ink past the area its view reserved, lines across another view) is reported as a
//! problem on the dimension's ID, never silently moved or dropped. A strip without room on the
//! page passes its outer lanes to the strip across the view. A dimension's lines crossing other
//! dimensions' lines are not checked yet. Each item's lane is its place among everything the
//! defaults would dimension, so omitting one leaves the others where they were (ARCH0003-2), and
//! the layout reserves the side view's bands for its lanes ([`lanes`]). An item
//! no shown view can carry is kept in the plan, unplaced, and reported (R1 invariant 3).
//!
//! The module is a driver over one planner per family ([`PLANNERS`]): [`envelope`], [`turned`],
//! [`holes`], [`slots`], [`bosses`], [`edges`] (fillet and chamfer callouts), [`steps`] and
//! [`levels`] (the height ladder) each say what their family wants dimensioned, and
//! [`placement`] places it. A new family is a new planner module and an entry in the list.

pub mod angular;
pub(crate) mod authored;
pub(crate) mod blends;
mod bosses;
mod edges;
mod envelope;
mod established;
pub(crate) use established::retain as retain_established;
mod flats;
pub(crate) mod footprints;
pub mod gdt;
mod grooves;
mod holes;
pub(crate) mod interior;
mod label_lanes;
mod wording;
pub(crate) use pmi_dimensions::basic as is_basic;
mod leaders;
pub(crate) use label_lanes::{in_lane, settle_inventory};
pub(crate) use leaders::settle_after_furniture;
pub(crate) use placement::collisions_cached as label_collisions;
pub(crate) mod levels;
mod notches;
mod oriented_slots;
mod pads;
mod placement;
mod plates;
mod pmi_dimensions;
pub mod precision;
mod profiled_bores;
mod recesses;
mod redundancy;
mod schemes;
mod slots;
mod steps;
mod stock;
mod tolerance;
mod turned;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::items::{Op, State};
use crate::model::{Analysis, DimensionSource, Invalid, PlannedSheet, PlannedView, Problem, Spec};
use crate::output::{Annotation, Claim, Curve, HAlign, Item, TextRun, VAlign};
use crate::text::{Font, Segment};
pub use leaders::clear_routes;
pub use placement::label_rows;
pub use placement::{AlignedDemand, LabelRows};
use placement::{Frame, across, moved, to_page};
pub use placement::{LabelInk, label_ink};
pub(crate) use tolerance::{check as check_tolerances, choices as fit_choices};

/// Checks the view every dimension entry of the spec names (`dimensions/<id>/view`) is one a
/// dimension can be carried by: the form, never the part (that is [`plan`]'s).
pub(crate) fn check_views(spec: &Spec) -> Result<(), Invalid> {
    for (id, entry) in &spec.dimensions {
        if let Some(v) = entry
            .place
            .as_ref()
            .and_then(|p| p.view.as_ref())
            .or(entry.view.as_ref())
            && !NAMES.contains(&v.as_str())
            && !crate::detail::NAMES.contains(&v.as_str())
        {
            return Err(Invalid {
                message: format!(
                    "The view of {id:?} is {v:?}: a dimension is carried by a principal view."
                ),
                choices: NAMES.map(String::from).to_vec(),
            });
        }
    }
    Ok(())
}

/// Whether view *v* can carry *w*: it is one of *w*'s views and does not look along it (a
/// turned part's diameter turns to a radial axis the view shows), as [`route`] decides.
fn carries(w: &Want, v: &str) -> bool {
    if crate::detail::NAMES.contains(&v) {
        return w.views.contains(&v);
    }
    let Some((_, toward)) = TOWARD.iter().find(|(n, _)| *n == v) else {
        return false;
    };
    w.views.contains(&v)
        && match &w.shape {
            Shape::Linear { axis, .. } => {
                toward[*axis] == 0.0
                    || w.turning
                        .is_some_and(|t| (0..3).any(|j| j != t && j != *axis && toward[j] == 0.0))
            }
            Shape::Callout { .. } | Shape::Angular { .. } | Shape::Aligned { .. } => true,
        }
}

/// The spec's choice of the view that carries each dimension (`dimensions/<id>/view`): put
/// first among its views when it is shown, so [`route`] gives it that view; a view that is not
/// shown leaves it to its default view and says so (a warning on its ID). A view that cannot
/// carry it is refused, with the views that can.
fn chosen_views(
    wants: &mut [Want],
    spec: &Spec,
    frames: &BTreeMap<&str, Frame>,
) -> Result<Vec<Problem>, Invalid> {
    let mut problems = Vec::new();
    for w in wants.iter_mut() {
        let Some(v) = spec.dimensions.get(&w.id).and_then(|e| {
            e.place
                .as_ref()
                .and_then(|p| p.view.as_deref())
                .or(e.view.as_deref())
        }) else {
            continue;
        };
        let able: Vec<&'static str> = w.views.iter().copied().filter(|c| carries(w, c)).collect();
        let Some(&v) = able.iter().find(|c| **c == v) else {
            return Err(Invalid {
                message: format!(
                    "{} cannot be carried by the {v} view (dimensions/{}/view): it is drawn only in a view that sees it across.",
                    w.id,
                    w.id.replace('~', "~0").replace('/', "~1")
                ),
                choices: able.iter().map(|c| c.to_string()).collect(),
            });
        };
        if frames.contains_key(v) {
            w.views.retain(|c| *c != v);
            w.views.insert(0, v);
            w.basis += &format!("; the spec places it in the {v} view");
        } else {
            let pinned = spec
                .dimensions
                .get(&w.id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.pinned);
            if pinned {
                w.views.retain(|c| *c == v);
            }
            let (why, how, fix) = if spec.views.get(v).is_some_and(|s| s.omit) {
                (
                    "omitted",
                    format!(
                        "Restore the view (remove views/{v}/omit) or remove the dimension's view."
                    ),
                    vec![Op::remove(format!("views/{v}/omit"))],
                )
            } else {
                (
                    "left out as the turned part's repeated profile",
                    format!(
                        "Keep the view (set views/{v} to {{}}) or remove the dimension's view."
                    ),
                    Vec::new(),
                )
            };
            let outcome = if pinned {
                "its pinned placement is retained unplaced"
            } else {
                "it is drawn in its default view instead"
            };
            problems.push(Problem {
                id: format!("p:dimension_view_not_shown:{}", w.id),
                code: "dimension_view_not_shown".into(),
                severity: "warning".into(),
                message: format!(
                    "The spec places {} in the {v} view, which is {why}, so {outcome}. {how}",
                    wording_of(w)
                ),
                subjects: vec![w.id.clone()],
                fix,
                ..Default::default()
            });
        }
    }
    Ok(problems)
}

/// Annotation text height (the em), mm: draftwright's `_FONT_SIZE`.
pub const TEXT: f64 = 3.0;
/// Arrowhead length, mm: draftwright's draft preset at its text height.
pub const ARROW: f64 = 2.7;
/// Half the arrowhead's width: a closed head of 1:3.
pub(crate) const ARROW_HALF: f64 = ARROW / 6.0;
/// The gap between an extension line and the point it extends from, and how far it runs past
/// its dimension line (ISO 129-1: about 2 mm).
const GAP: f64 = 1.0;
const OVERSHOOT: f64 = 2.0;
/// The first dimension line's distance from the view's outline (draftwright's `_STRIP_GAP`) and
/// the distance between successive parallel lines.
pub const FIRST: f64 = 10.0;
pub const LANE: f64 = 7.0;
/// The gap between a label's terms (draftwright's callout recipe), a label's baseline above its
/// dimension line, the middle of a capital above its baseline, and a leader's shoulder.
const TOKEN_GAP: f64 = 0.45 * TEXT;
const LIFT: f64 = 1.0;
pub(crate) const CAP: f64 = 0.7 * TEXT;
const SHOULDER: f64 = 3.0;
/// The font labels are measured and drawn with (ARCH0006-1: one file for both).
pub const FONT: &str = "IBMPlexMono-Regular.ttf";
const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

pub(crate) fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| Font::new(MONO.to_vec()).expect("the bundled font parses"))
}

/// A dimension or callout the sheet will carry.
/// A resolved candidate's region; author intent still contains no page coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateRegion {
    Interior,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedDimension {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<CandidateRegion>,
    /// `d:<feature>/<parameter>`; the feature is `envelope` for the overall extents, and
    /// `pmi:<identity>` for a PMI dimension standing on its own. It never names the view.
    pub id: String,
    /// `linear`, `callout` or `angular`.
    pub kind: String,
    pub view: String,
    /// The measured value it leads with, mm (a callout's bore diameter).
    pub value: f64,
    /// What it prints: its terms joined by spaces.
    pub label: String,
    /// The printed terms in order. `⌀`, `↧`, `⌴` and `⌵` are drawn as symbols.
    pub terms: Vec<String>,
    /// Every measurement the label states, by feature and parameter.
    pub measurements: Vec<Measurement>,
    pub default: bool,
    /// What of it the spec gave: `tolerance`, `fit` (ARCH0011-1). Absent when nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from_spec: Vec<String>,
    pub basis: String,
    /// What became of it: shown, or why not ([`crate::items::State`]).
    pub state: crate::items::State,
    /// Its ink on the page; absent when it cannot be drawn, and a problem says why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<Placed>,
    /// A reference dimension, drawn in brackets (`(12)`): other dimensions determine it, so it
    /// is for information only (#26). Absent (false) for a controlling dimension.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reference: bool,
    /// For a reference dimension, the IDs of the dimensions that determine it (a chain's stated
    /// total and its other members; a slot's width and location), as its basis names them.
    /// Absent when none is recorded (a pinned reference kept after its chain was edited).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub determined_by: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub feature: String,
    /// draftwright's role vocabulary, `role.kind[.discriminator]`.
    pub parameter: String,
    pub value: f64,
}

pub(crate) fn ink_box(d: &PlannedDimension) -> Option<[f64; 4]> {
    let p = d.placed.as_ref()?;
    Some(placement::ink_points(p).fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, q| {
            [
                b[0].min(q[0]),
                b[1].min(q[1]),
                b[2].max(q[0]),
                b[3].max(q[1]),
            ]
        },
    ))
}

/// A dimension's ink, mm on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placed {
    /// Extension, dimension and leader lines.
    pub lines: Vec<[[f64; 2]; 2]>,
    /// Arrowheads, each `[tip, tail]` along its shaft.
    pub arrows: Vec<[[f64; 2]; 2]>,
    /// Where the label's first term starts, on its baseline.
    pub label_at: [f64; 2],
    /// The label's direction, degrees anticlockwise.
    pub rotation: f64,
    /// The label's ink box, `[x0, y0, x1, y1]`.
    pub label_box: [f64; 4],
}

/// A dimension the plan does not draw, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omitted {
    pub id: String,
    /// What it prints: its label as the plan had it when it was left out (one the spec omits,
    /// its default label).
    pub label: String,
    pub basis: String,
    /// Why it is left out: by the spec, or stated by another item.
    pub state: crate::items::State,
}

/// Shortest nominal within 1e-6 after noise removal, bounded to three decimal places.
/// Unlike Python's `_fmt`, this preserves modelled hundredths and thousandths.
pub fn fmt(v: f64) -> String {
    crate::nominal::format(v)
}

type P3 = [f64; 3];
type P2 = [f64; 2];
/// What [`plan`] returns: the dimensions, what the spec left out, and the problems.
pub type Planned = (Vec<PlannedDimension>, Vec<Omitted>, Vec<Problem>);

const NAMES: [&str; 3] = ["front", "plan", "side"];
const AXES: [&str; 3] = ["x", "y", "z"];
/// The standard views' directions towards the viewer, for routing before any view is placed.
const TOWARD: [(&str, P3); 3] = [
    ("front", [0.0, -1.0, 0.0]),
    ("plan", [0.0, 0.0, 1.0]),
    ("side", [1.0, 0.0, 0.0]),
];

/// The view that looks along world axis *k* (ADR1-6: a feature's view follows from its axis by
/// one rule): x → side, y → front, z → plan.
pub fn end_on(k: usize) -> &'static str {
    TOWARD
        .iter()
        .find(|(_, t)| t[k].abs() == 1.0)
        .map(|(n, _)| *n)
        .expect("each axis has a view")
}

/// What a dimension measures, before it is placed.
#[derive(Clone, Debug, Serialize)]
enum Shape {
    /// From *a* to *b* along world axis *axis*; an `edge` end extends from the view's outline.
    Linear {
        a: P3,
        b: P3,
        axis: usize,
        a_edge: bool,
        b_edge: bool,
        /// Its lane order: sizes 0, locations 1, overall extents 2.
        order: u8,
    },
    /// An exact span between two proven stations in a principal projection plane.
    Aligned { a: P3, b: P3 },
    /// A leader to circles/points (centre, radius), world mm. When `arcs` is
    /// nonempty it supplies the actual trimmed profiles instead of whole circles.
    Callout {
        sites: Vec<(P3, f64)>,
        arcs: Vec<crate::arc::ProfileArc>,
    },
    /// An angle between two sides, drawn about its vertex ([`angular`]).
    Angular { angle: angular::Angle },
}

#[derive(Clone, Debug, Serialize)]
struct Want {
    id: String,
    /// The views that can carry it, preferred first.
    views: Vec<&'static str>,
    value: f64,
    terms: Vec<String>,
    measurements: Vec<Measurement>,
    basis: String,
    shape: Shape,
    /// For a turned part's diameter: its turning axis. The diameter is then drawn across
    /// whichever radial axis the carrying view shows; a step diameter's callout, led to the
    /// silhouette, keeps its leader off the silhouette's direction ([`leaders`]).
    turning: Option<usize>,
}

/// A wanted item as a problem's message names it: by the terms it prints
/// ([`crate::wording::item`]), never by its ID.
fn wording_of(w: &Want) -> String {
    crate::wording::item(&w.id, "", &w.terms.join(" "))
}

/// Contact between annotation text and the other annotation's rendered ink.
pub(crate) fn text_contact(a: &Placed, b: &Placed) -> bool {
    placement::boxes_overlap(a.label_box, b.label_box)
        || placement::ink_through(a, b.label_box)
        || placement::ink_through(b, a.label_box)
}

/// The four overall sizes, excluding source-stamped profile requirements.
pub(crate) fn is_envelope_extent(id: &str) -> bool {
    matches!(
        id,
        "d:envelope/width.length"
            | "d:envelope/depth.length"
            | "d:envelope/height.length"
            | "d:envelope/od.diameter"
    )
}

/// A dimension's ID names what it measures, independently of the carrying view.
fn want_id(feature: &str, parameter: &str) -> String {
    format!("d:{feature}/{parameter}")
}

fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: P3, s: f64) -> P3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn point(v: Option<&Value>) -> Option<P3> {
    let c: Vec<f64> = v?
        .as_array()?
        .iter()
        .map(Value::as_f64)
        .collect::<Option<_>>()?;
    <[f64; 3]>::try_from(c).ok()
}

/// quiddity's frame (`Recognition::frame`): a point, or an axis letter, from it into the part's.
struct QFrame([[f64; 3]; 4]);

impl QFrame {
    fn point(&self, l: P3) -> P3 {
        (0..3).fold(self.0[0], |p, j| add(p, scale3(self.0[j + 1], l[j])))
    }

    fn letter(&self, axis: &Value) -> Option<P3> {
        let k = AXES.iter().position(|a| axis.as_str() == Some(a))?;
        Some(self.0[k + 1])
    }
}

fn key(x: f64) -> i64 {
    crate::bridge::holes::size_key(x)
}

/// What every planner reads: the Analysis, quiddity's frame, and whether the part is turned
/// whole.
struct Context<'a> {
    analysis: &'a Analysis,
    frame: QFrame,
    turned: Option<turned::Turned>,
}

/// A family's dimension planner: the feature kinds it dimensions, and what the defaults would
/// dimension for it, with the problems met on the way.
struct Planner {
    /// The `Feature::kind`s whose own sizes it plans. A feature of a kind no planner claims is
    /// reported (`dimensions_not_ported`).
    kinds: &'static [&'static str],
    wants: fn(&Context) -> (Vec<Want>, Vec<Problem>),
}

/// The planners, in the order their items are planned.
const PLANNERS: [Planner; 18] = [
    turned::PLANNER,
    envelope::PLANNER,
    stock::PLANNER,
    holes::PLANNER,
    slots::PLANNER,
    bosses::PLANNER,
    edges::PLANNER,
    notches::PLANNER,
    blends::PLANNER,
    grooves::PLANNER,
    flats::PLANNER,
    steps::PLANNER,
    levels::PLANNER,
    recesses::PLANNER,
    plates::PLANNER,
    pads::PLANNER,
    oriented_slots::PLANNER,
    profiled_bores::PLANNER,
];

/// Everything the defaults would dimension, with the problems met on the way, and what the part's
/// PMI dimensions put on the model's items ([`pmi_dimensions`]).
fn wants(
    analysis: &Analysis,
) -> (
    Vec<Want>,
    Vec<Problem>,
    BTreeMap<String, pmi_dimensions::Note>,
) {
    let mut result =
        crate::planning::memo(analysis, "wants", Vec::new(), || wants_uncached(analysis));
    bosses::prefer_established(analysis, &mut result.0);
    result
}

/// Captured bridge documents already carried their recess inventory. Only a
/// native reading adds that information relative to the released reader.
pub(crate) fn has_new_native_recess_dimensions(analysis: &Analysis) -> bool {
    analysis
        .recognition
        .as_ref()
        .is_some_and(|r| r.recogniser.starts_with("quiddity-rust "))
        && has_recess_dimensions(analysis)
}

pub(crate) fn has_recess_dimensions(analysis: &Analysis) -> bool {
    let (canonical, established) = crate::planning::policy_key(analysis);
    crate::planning::memo(
        analysis,
        "recess-presence",
        vec![u8::from(canonical), u8::from(established)],
        || has_recess_dimensions_uncached(analysis),
    )
}

fn has_recess_dimensions_uncached(analysis: &Analysis) -> bool {
    wants(analysis).0.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    })
}

fn wants_uncached(
    analysis: &Analysis,
) -> (
    Vec<Want>,
    Vec<Problem>,
    BTreeMap<String, pmi_dimensions::Note>,
) {
    let frame = QFrame(analysis.recognition.as_ref().map_or(
        [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        |r| r.frame,
    ));
    let cx = Context {
        analysis,
        turned: turned::turned(analysis, &frame),
        frame,
    };
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for p in &PLANNERS {
        let (w, more) = (p.wants)(&cx);
        out.extend(w);
        problems.extend(more);
    }
    // All axial locations use the same inspection end as the turned shoulder baseline.
    if let Some((axis, datum)) = turned::axial_datum(analysis) {
        for w in &mut out {
            if w.measurements
                .iter()
                .any(|m| m.parameter.starts_with("location.length."))
                && let Shape::Linear {
                    a,
                    b,
                    axis: k,
                    order,
                    ..
                } = &mut w.shape
                && *k == axis
            {
                a[axis] = datum;
                w.value = (b[axis] - datum).abs();
                w.terms = vec![fmt(w.value)];
                for m in &mut w.measurements {
                    m.value = w.value;
                }
                *order = 1;
                w.basis = format!(
                    "axial location in {} from the profile's common inspection end",
                    AXES[axis]
                );
            }
        }
    }
    flats::share_locations(&cx, &mut out);
    // A planner may dimension some records of a kind it does not claim (the section recesses
    // that are blind slots): those are not reported.
    let stated: BTreeSet<String> = out
        .iter()
        .flat_map(|w| w.measurements.iter().map(|m| m.feature.clone()))
        .collect();
    problems.extend(
        not_ported(analysis)
            .into_iter()
            .filter(|p| !p.subjects.iter().any(|s| stated.contains(s))),
    );
    // The part's PMI dimensions: on the model's items that state them, or their own.
    let (own, more, notes) = pmi_dimensions::lower(&cx, &mut out);
    out.extend(own);
    problems.extend(more);
    (out, problems, notes)
}

/// The consumer policy for an occurrence with no dimension owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Unsupported,
    Deferred,
    EvidenceOnly,
}

/// Closed reasons for recognised occurrences that have no dimension planner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionReason {
    ConsumerSemanticsUnsupported,
    ConsumerSemanticsDeferred,
    HoleCalloutEvidence,
    DimensionsNotPorted,
    StepLevelProjectionEvidence,
    RiserProjectionEvidence,
    GeometryOnlyCritique,
    HoleRecognitionEvidence,
    RecognitionRefused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureDisposition {
    pub subject: String,
    pub disposition: Disposition,
    pub reason_code: DispositionReason,
    pub severity: String,
}

/// quiddity record types (`Fingerprint::subtype`) draftwright gives no dimensions of their own
/// (`model/detect.py`'s `_ORCHESTRATED_RECORDS` and `_UNCONSUMED_RECORDS`): a countersink rides on
/// its hole's callout, and the others have no reviewed drafting grammar there (#1365, #1430,
/// #1557). Leaving them undimensioned is no gap in the port, so they are not reported as one. An
/// angled step (#1247) has no grammar either, but is reported as unsupported ([`unsupported`]).
const UNDRAFTED: &[(&str, Disposition, DispositionReason)] = &[
    (
        "CircularFacePattern",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "CounterSink",
        Disposition::EvidenceOnly,
        DispositionReason::HoleCalloutEvidence,
    ),
    (
        "FreeformSurface",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "InteriorVoid",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "ObliqueThroughStep",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "OrientedSlotArray",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "OrientedSlotGrid",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "SheetMetalBody",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
    (
        "ThinWallBody",
        Disposition::Deferred,
        DispositionReason::ConsumerSemanticsDeferred,
    ),
];

pub(crate) fn undrafted(f: &crate::bridge::Feature) -> bool {
    UNDRAFTED
        .iter()
        .any(|(subtype, ..)| *subtype == f.fingerprint.subtype)
}

/// The capability tables drive both the undrafted exemption and its occurrence report. A
/// planner can consume a kind it does not claim (section recesses); retain that actual proof.
/// Authored omissions are still planned capabilities, never ownerless policy.
pub(crate) fn feature_dispositions(analysis: &Analysis) -> Vec<FeatureDisposition> {
    let (wants, ..) = wants(analysis);
    let stated: BTreeSet<_> = wants
        .iter()
        .flat_map(|w| &w.measurements)
        .map(|m| m.feature.as_str())
        .collect();
    let mut out = Vec::new();
    let mut add = |subject: String, disposition, reason_code| {
        out.push(FeatureDisposition {
            subject,
            disposition,
            reason_code,
            severity: "info".into(),
        })
    };
    for f in &analysis.features {
        if let Some((_, disposition, reason)) = UNDRAFTED
            .iter()
            .find(|(subtype, ..)| *subtype == f.fingerprint.subtype)
        {
            add(f.id.clone(), *disposition, *reason);
        } else if !stated.contains(f.id.as_str())
            && !PLANNERS.iter().any(|p| p.kinds.contains(&f.kind.as_str()))
        {
            let (disposition, reason) = if f.kind == "angled_step" {
                (
                    Disposition::Unsupported,
                    DispositionReason::ConsumerSemanticsUnsupported,
                )
            } else {
                (
                    Disposition::Deferred,
                    DispositionReason::DimensionsNotPorted,
                )
            };
            add(f.id.clone(), disposition, reason);
        }
    }
    if let Some(r) = &analysis.recognition {
        for (i, record) in r.evidence.iter().enumerate() {
            let reason = match record.subtype.as_str() {
                "FaceLevel" => DispositionReason::StepLevelProjectionEvidence,
                "RiserEvidence" => DispositionReason::RiserProjectionEvidence,
                "RepeatingRadialProfile" => DispositionReason::GeometryOnlyCritique,
                // The bridge admits only these four evidence types (bridge::RECORDS).
                "HoleSpec" => DispositionReason::HoleRecognitionEvidence,
                _ => continue,
            };
            add(
                format!("recognition:evidence:{i}"),
                Disposition::EvidenceOnly,
                reason,
            );
        }
        for (i, _) in r.refusals.iter().enumerate() {
            add(
                format!("recognition:refusal:{i}"),
                Disposition::Unsupported,
                DispositionReason::RecognitionRefused,
            );
        }
    }
    out
}

/// One problem per recognised feature of a kind no planner claims, in the Analysis's feature
/// order. This reports a gap in the port, not in the part: draftwright converts these records to
/// features with dimension parameters (`detect.py`'s `_CONVERTERS`: fillets, chamfers, bosses,
/// slots, turned steps, …) and plans them, so it never emits such a problem; without it the
/// port's sheet would leave their sizes unstated with no trace (review M8). A kind not known to
/// be [`UNDRAFTED`] is reported, and a kind draftwright reports unsupported gets that warning
/// instead ([`unsupported`]).
fn not_ported(analysis: &Analysis) -> Vec<Problem> {
    analysis
        .features
        .iter()
        .filter(|f| !PLANNERS.iter().any(|p| p.kinds.contains(&f.kind.as_str())))
        .filter(|f| !undrafted(f))
        .map(|f| {
            unsupported(f).unwrap_or_else(|| Problem {
                id: format!("p:dimensions_not_ported:{}", f.id),
                code: "dimensions_not_ported".into(),
                severity: "info".into(),
                message: format!(
                    "{} is not dimensioned: no dimensions for {} are ported yet.",
                    crate::wording::capital(&crate::wording::feature(f)),
                    f.kind.replace('_', " ")
                ),
                subjects: vec![f.id.clone()],
                ..Default::default()
            })
        })
        .collect()
}

/// The warning for a recognised feature draftwright has decided not to dimension and reports as
/// unsupported, or `None` for any other. That is an angled step (ISSUE-1247-a; Python counts it as
/// an `unsupported` completeness outcome in `linting/quality.py`, and lint warns
/// `angled_step_requirement_unsupported`): its angle, legs and run have no drafting grammar, so it
/// is no gap in the port, but the sheet must not look complete without it (maintainer decision,
/// 2026-10-09, until it is dimensioned or the port has a completeness report). The code, severity
/// and wording are lint's.
fn unsupported(f: &crate::bridge::Feature) -> Option<Problem> {
    if f.kind != "angled_step" {
        return None;
    }
    let code = "angled_step_requirement_unsupported";
    Some(Problem {
        id: format!("p:{code}:{}", f.id),
        code: code.into(),
        severity: "warning".into(),
        message: format!(
            "{} is not represented by Draftwright dimensions; review and define its \
             manufacturing geometry outside automatic drawing approval.",
            crate::wording::capital(&crate::wording::feature(f))
        ),
        subjects: vec![f.id.clone()],
        ..Default::default()
    })
}

pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

pub(crate) fn inspection_datum(a: &Analysis) -> Option<(usize, f64)> {
    turned::axial_datum(a)
}

pub(crate) fn detail_levels(a: &Analysis) -> Vec<(String, f64)> {
    let (canonical, established) = crate::planning::policy_key(a);
    crate::planning::memo(
        a,
        "detail-linear-levels",
        vec![u8::from(canonical), u8::from(established)],
        || detail_levels_uncached(a),
    )
}

fn detail_levels_uncached(a: &Analysis) -> Vec<(String, f64)> {
    wants(a)
        .0
        .into_iter()
        .filter_map(|w| match w.shape {
            Shape::Linear { .. } => Some((w.id, w.value)),
            _ => None,
        })
        .collect()
}

/// Route the exact owned requirements through the same placement and tolerance pipeline.
/// A baseline with a witness outside the crop stays on the parent; its local band size
/// is recovered inside the detail under the saved chain ID.
fn detail_routes(a: &Analysis, wants: &mut [Want], views: &[PlannedView]) {
    for v in views {
        let Some(d) = &v.detail else { continue };
        let Some(&name) = crate::detail::NAMES
            .iter()
            .find(|n| v.id == format!("v:{n}"))
        else {
            continue;
        };
        let inside =
            |p: P3| (0..3).all(|k| d.bounds[0][k] - 1e-6 <= p[k] && p[k] <= d.bounds[1][k] + 1e-6);
        for w in wants.iter_mut() {
            let explicit = d.dimensions.contains(&w.id);
            let blend = w
                .measurements
                .iter()
                .any(|m| m.parameter == "blend.radius" || m.parameter == "fillet.radius");
            let sites = match &w.shape {
                Shape::Callout { sites, .. } => sites.iter().any(|(p, _)| inside(*p)),
                _ => false,
            };
            let feature = w
                .measurements
                .iter()
                .any(|m| crate::dims::feature_origin(a, &m.feature).is_some_and(inside));
            if !explicit && !(sites && blend && d.kind == "turned-head") && !(d.authored && feature)
            {
                continue;
            }
            match &mut w.shape {
                Shape::Linear {
                    a: pa,
                    b: pb,
                    axis,
                    a_edge,
                    b_edge,
                    ..
                } => {
                    // Dimension an x-turned head at its positive silhouette. Try the
                    // rim witnesses on copies: a rejected crop keeps the parent's ink.
                    let (mut next_a, mut next_b) = (*pa, *pb);
                    if w.id.ends_with("/step.length")
                        && d.kind == "turned-head"
                        && *axis == 0
                        && let Some(band) = w
                            .measurements
                            .first()
                            .and_then(|m| a.features.iter().find(|f| f.id == m.feature))
                            .and_then(|f| {
                                crate::lint::turned::band(&crate::bridge::holes::frame(a), f).ok()
                            })
                    {
                        next_a[2] = band.on[2] + band.diameter / 2.0;
                        next_b[2] = next_a[2];
                    }
                    if !inside(next_a) || !inside(next_b) {
                        continue;
                    }
                    (*pa, *pb) = (next_a, next_b);
                    if w.id.ends_with("/step.length") && *axis == 0 {
                        *a_edge = false;
                        *b_edge = false;
                    }
                }
                Shape::Callout { sites, .. } => {
                    sites.retain(|(p, _)| inside(*p));
                    if sites.is_empty() {
                        continue;
                    }
                }
                Shape::Aligned { a, b } => {
                    if !inside(*a) || !inside(*b) {
                        continue;
                    }
                }
                Shape::Angular { .. } => continue,
            }
            w.views.insert(0, name);
            w.basis += &format!("; recovered at enlarged scale in {name}");
        }
    }
}

/// The view that carries each of *wants*: its first choice that is shown (in *frames*) and does
/// not look along it (a turned part's diameter turns to the radial axis the view shows). A later
/// choice says so in its basis; its ID stays. Those with none keep the views that were shown but look along them.
fn route(
    wants: &mut [Want],
    frames: &BTreeMap<&str, Frame>,
    analysis: &Analysis,
) -> (Vec<Option<&'static str>>, Vec<Vec<&'static str>>) {
    let [lo, hi] = analysis.envelope;
    let mut chosen: Vec<Option<&'static str>> = Vec::new();
    let mut end_on: Vec<Vec<&'static str>> = Vec::new();
    for w in wants.iter_mut() {
        let mut view = None;
        let mut along = Vec::new();
        for v in w.views.clone() {
            let Some(f) = frames.get(v) else { continue };
            if let Shape::Linear { a, b, axis, .. } = &mut w.shape
                && across(f, *axis).is_none()
            {
                let radial = w
                    .turning
                    .and_then(|t| (0..3).find(|&j| j != t && j != *axis && across(f, j).is_some()));
                let Some(j) = radial else {
                    along.push(v);
                    continue;
                };
                *a = lo;
                *b = lo;
                b[j] = hi[j];
                *axis = j;
            }
            view = Some(v);
            break;
        }
        if let Some(v) = view {
            bosses::route_profile(analysis, w, v);
        }
        if let Some(v) = view
            && v != w.views[0]
        {
            let why = if frames.contains_key(w.views[0]) {
                "looks along it"
            } else {
                "is omitted"
            };
            w.basis += &format!(
                "; the {} view {why}, so the {v} view carries it",
                w.views[0]
            );
        }
        chosen.push(view);
        end_on.push(along);
    }
    (chosen, end_on)
}

/// A reduced principal set is permissible only if every planned requirement that
/// the full set can carry still has a carrier. This covers radial flats, keyways,
/// cross slots and PMI as well as holes, before placing or losing any item.
pub(crate) fn reduction_preserves_requirements(
    analysis: &Analysis,
    spec: &Spec,
    kept: &[&str],
    excluded: &BTreeSet<String>,
) -> bool {
    let (wants, _, _) = wants(analysis);
    wants
        .iter()
        .filter(|w| {
            !excluded.contains(&w.id) && !spec.dimensions.get(&w.id).is_some_and(|d| d.omit)
        })
        .all(|w| {
            // A dimension's view is a preference in Spec; asking to retain a
            // principal view belongs to views/<name>. Preserve that contract.
            !NAMES.iter().any(|v| carries(w, v))
                || kept
                    .iter()
                    .any(|v| carries(w, v) && !spec.views.get(*v).is_some_and(|v| v.omit))
        })
}

/// Paper-space room for unpinned, side-only callouts. Keep the same labels as the solver,
/// including threads and authored tolerances, and reserve the column before packing views.
pub(crate) fn callout_bands(
    analysis: &Analysis,
    spec: &Spec,
) -> Result<Vec<crate::layout::CalloutBand>, Invalid> {
    if !spec.dimensions.values().any(|d| {
        d.place
            .as_ref()
            .and_then(crate::placement_intent::side_hint)
            .is_some()
    }) {
        return Ok(Vec::new());
    }
    let (mut wants, ..) = wants(analysis);
    let scheme_covered = schemes::apply(analysis, spec, &mut wants)?;
    authored::append(analysis, spec, &mut wants)?;
    if !scheme_covered.is_empty() {
        wants.retain(|w| !scheme_covered.contains_key(&w.id));
    }
    let mut kept: Vec<_> = wants
        .into_iter()
        .map(|w| (w, false, None, Vec::new()))
        .collect();
    precision::labels(spec, kept.iter_mut().map(|(w, ..)| w))?;
    let mut given = tolerance::apply(analysis, spec, &mut kept, &[], &mut Vec::new())?;
    wording::apply(spec, &mut kept, &mut given)?;
    Ok(kept
        .iter()
        .filter_map(|(w, ..)| {
            if !matches!(w.shape, Shape::Callout { .. }) {
                return None;
            }
            let entry = spec.dimensions.get(&w.id)?;
            if entry.omit {
                return None;
            }
            let place = entry.place.as_ref()?;
            let side = crate::placement_intent::side_hint(place)?;
            let view = place
                .view
                .as_deref()
                .or(entry.view.as_deref())
                .unwrap_or(w.views[0]);
            let view = *NAMES.iter().find(|n| **n == view)?;
            if spec.views.get(view).is_some_and(|v| v.omit) {
                return None;
            }
            use crate::placement_intent::Side;
            let ink = label_ink(&w.terms, font());
            let (side, depth) = match side {
                Side::Left => ("left", ink.max[0] - ink.min[0] + SHOULDER),
                Side::Right => ("right", ink.max[0] - ink.min[0] + SHOULDER),
                Side::Above => ("above", TEXT + LIFT),
                Side::Below => ("below", TEXT + LIFT),
            };
            Some(crate::layout::CalloutBand {
                view,
                side,
                depth: FIRST + depth + 2.0,
            })
        })
        .collect())
}

/// The lanes the defaults' dimensions take beside the principal *views* (name, direction towards
/// the viewer, up) before they are placed: what the layout reserves for them
/// ([`crate::layout::Strips::reserve`]). Every item the part would have counts, whatever the
/// spec omits, as [`plan`] gives each its lane in that full set (edit locality, ARCH0003-2).
///
/// Items the defaults merge away count here with a lane each, though [`plan`] gives them their
/// survivor's ([`placement::owners`]): the band below the side is also the front's lower band,
/// which holds the front's own lanes without reserving them, and on ftc06 and ftc07 the side's
/// merged boss heights were what kept the front's lowest lanes inside it (counted without them,
/// ink 11 to 13 and 9 to 11; `docs/quality-2026-10-09.md`, "After merged-away lanes").
pub fn lanes(analysis: &Analysis, views: &[(&str, P3, P3)]) -> crate::layout::Lanes {
    lanes_excluding(analysis, views, &BTreeSet::new(), &Spec::default())
        .expect("default display policy cannot conflict")
        .0
}

pub(crate) fn lanes_excluding(
    analysis: &Analysis,
    views: &[(&str, P3, P3)],
    excluded: &BTreeSet<String>,
    spec: &Spec,
) -> Result<(crate::layout::Lanes, Vec<AlignedDemand>), Invalid> {
    lane_policy(analysis, views, excluded, spec, true)
}

/// A captured bridge drawing's first estimate precedes measured callout columns.
/// The complete recovery candidates always reserve those columns.
pub(crate) fn initial_lanes(
    analysis: &Analysis,
    views: &[(&str, P3, P3)],
    excluded: &BTreeSet<String>,
    spec: &Spec,
) -> Result<(crate::layout::Lanes, Vec<AlignedDemand>), Invalid> {
    lane_policy(analysis, views, excluded, spec, false)
}

fn lane_policy(
    analysis: &Analysis,
    views: &[(&str, P3, P3)],
    excluded: &BTreeSet<String>,
    spec: &Spec,
    callout_columns: bool,
) -> Result<(crate::layout::Lanes, Vec<AlignedDemand>), Invalid> {
    let key = serde_json::to_vec(&(views, excluded, spec, callout_columns))
        .expect("lane inputs serialize");
    crate::planning::memo(analysis, "lanes", key, || {
        lanes_uncached(analysis, views, excluded, spec, callout_columns)
    })
}

fn lanes_uncached(
    analysis: &Analysis,
    views: &[(&str, P3, P3)],
    excluded: &BTreeSet<String>,
    spec: &Spec,
    callout_columns: bool,
) -> Result<(crate::layout::Lanes, Vec<AlignedDemand>), Invalid> {
    let (mut wants, mut problems, notes) = wants(analysis);
    let scheme_covered = schemes::apply(analysis, spec, &mut wants)?;
    authored::append(analysis, spec, &mut wants)?;
    if scheme_covered.is_empty() {
        wants.retain(|w| !excluded.contains(&w.id));
    } else {
        wants.retain(|w| !excluded.contains(&w.id) && !scheme_covered.contains_key(&w.id));
    }
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter(|(name, ..)| NAMES.contains(name))
        .map(|(name, direction, up)| {
            (
                *name,
                Frame::looking(analysis, *direction, *up, [0.0; 2], 1.0),
            )
        })
        .collect();
    if spec.dimensions.values().any(|d| d.between.is_some()) {
        problems.extend(chosen_views(&mut wants, spec, &frames)?);
    }
    plates::prefer_readable(analysis, &mut wants, &frames, None);
    let (chosen, _) = route(&mut wants, &frames, analysis);
    let wants: Vec<_> = wants
        .into_iter()
        .zip(chosen)
        .filter(|(w, _)| !w.measurements.iter().any(|m| m.parameter == "blend.radius"))
        .collect();
    let (mut wants, chosen): (Vec<_>, Vec<_>) = wants.into_iter().unzip();
    let plate = |w: &Want| {
        w.measurements
            .iter()
            .any(|m| m.parameter == "thickness.length")
    };
    // #33: a turned band's length that a baseline from the datum conveys is planned only so a
    // spec naming it still reads; it draws no ink, so it reserves no lane of its own either.
    let baseline = wants
        .iter()
        .any(|w| w.id.ends_with("/step_position.length"));
    let conveyed = |w: &Want| baseline && w.id.ends_with("/step.length");
    let shared_bands = wants.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    let owner = if wants.iter().any(plate) || baseline || shared_bands {
        let merged = placement::owners(analysis, &wants, &chosen);
        wants
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if shared_bands || plate(w) || conveyed(w) {
                    merged[i]
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![None; wants.len()]
    };
    if wants
        .iter()
        .zip(&owner)
        .any(|(w, owner)| plate(w) && owner.is_none())
    {
        plates::default_terms(analysis, &mut wants, &chosen);
    }
    // Resolve the printed terms before measuring reservation, using placement's lowering order.
    let mut kept: Vec<_> = wants
        .iter()
        .cloned()
        .zip(chosen.iter().copied())
        .map(|(w, view)| (w, true, view, Vec::new()))
        .collect();
    let mut omitted = Vec::new();
    plates::consolidate(analysis, &mut kept, &mut omitted);
    bosses::consolidate(analysis, &mut kept, &mut omitted);
    pmi_dimensions::yield_to_spec(spec, &notes, &mut kept, &mut problems);
    pmi_dimensions::hide_fit_limits(spec, &notes, &mut kept);
    precision::labels(spec, kept.iter_mut().map(|(w, ..)| w))?;
    if let Ok(mut given) = tolerance::apply(analysis, spec, &mut kept, &omitted, &mut problems) {
        holes::consolidate(analysis, &mut kept, &mut given, &mut omitted);
        plates::share(analysis, &mut kept, &mut given, &mut omitted);
        share_statements(&mut kept, &mut given, &mut omitted, true);
        share_pmi(analysis, &mut kept, &mut given, &mut omitted);
        redundancy::consolidate(analysis, &mut kept, &mut given, &mut omitted);
        wording::apply(spec, &mut kept, &mut given)?;
    }
    for w in &mut wants {
        if let Some((printed, ..)) = kept.iter().find(|(p, ..)| p.id == w.id) {
            w.terms.clone_from(&printed.terms);
        }
    }
    let (sides, lane) = placement::lanes(&frames, wants.iter().zip(chosen.iter().copied()), &owner);
    let mut bands = placement::band_lanes(
        chosen
            .iter()
            .zip(sides.iter().copied())
            .zip(lane.iter().copied())
            .enumerate()
            .filter(|(i, _)| {
                owner[*i].is_none()
                    && (!shared_bands || matches!(wants[*i].shape, Shape::Linear { .. }))
            })
            .map(|(_, item)| item),
    );
    let front_linear = wants
        .iter()
        .enumerate()
        .filter(|(i, w)| {
            owner[*i].is_none()
                && chosen[*i] == Some("front")
                && sides[*i] == Some(placement::Side::Below)
                && matches!(w.shape, Shape::Linear { .. })
        })
        .map(|(i, _)| lane[i] + 1)
        .max()
        .unwrap_or(0);
    let plan_linear = wants
        .iter()
        .enumerate()
        .filter(|(i, w)| {
            owner[*i].is_none()
                && chosen[*i] == Some("plan")
                && sides[*i] == Some(placement::Side::Above)
                && matches!(w.shape, Shape::Linear { .. })
        })
        .map(|(i, _)| lane[i] + 1)
        .max()
        .unwrap_or(0);
    // A controlling overall adds a lane to the turned axial chain.
    if wants
        .iter()
        .any(|w| crate::dims::is_envelope_extent(&w.id) && w.id.ends_with(".length"))
        && turned::turned(analysis, &QFrame(crate::bridge::holes::frame(analysis))).is_some()
    {
        bands.plate_front_below = bands.plate_front_below.max(front_linear);
    }
    // A turned diameter can leave either side of the profile. Keep its full alternate shelf
    // available to the ordinary side/row/tier search, including tall fit deviations.
    for (i, w) in wants.iter().enumerate() {
        if chosen[i] == Some("front")
            && matches!(w.shape, Shape::Callout { .. })
            && w.measurements
                .iter()
                .any(|m| m.parameter == "step.diameter")
        {
            let ink = label_ink(&w.terms, font());
            bands.callout_front_left = bands.callout_front_left.max(leaders::shelf_width(&ink));
        }
        if owner[i].is_some()
            || !w
                .measurements
                .iter()
                .any(|m| m.parameter == "thickness.length")
        {
            continue;
        }
        match (chosen[i], sides[i]) {
            (Some("front"), Some(placement::Side::Left)) => {
                bands.plate_front_left = bands.plate_front_left.max(lane[i] + 1);
                let ink = label_ink(&w.terms, font());
                bands.plate_front_top = bands
                    .plate_front_top
                    .max(ink.max[0] - ink.min[0] + ARROW + 3.0);
            }
            (Some("front"), Some(placement::Side::Below)) => {
                bands.plate_front_below = bands.plate_front_below.max(front_linear)
            }
            // A horizontal side thickness's outside arrow reaches past the side's left edge,
            // above or below it.
            (Some("side"), Some(placement::Side::Above | placement::Side::Below)) => {
                bands.plate_side_left = ARROW + 2.0
            }
            (Some("side"), Some(placement::Side::Right)) => {
                let ink = label_ink(&w.terms, font());
                bands.plate_side_top = bands
                    .plate_side_top
                    .max(ink.max[0] - ink.min[0] + ARROW + 3.0);
            }
            (Some("plan"), Some(placement::Side::Above)) => {
                bands.plate_plan_above = bands.plate_plan_above.max(plan_linear)
            }
            _ => {}
        }
    }
    // Reserve canonical physical notch statements, independent of printed sharing.
    // A tolerance may split a counted depth without changing this plan-left allowance.
    // Reserving every historical lane would enlarge otherwise readable
    // sheets; omitting these new lanes can strand an existing centre ordinate.
    let notch_lanes = wants
        .iter()
        .zip(&chosen)
        .filter(|(w, view)| {
            **view == Some("plan")
                && (spec.dimension_source == DimensionSource::Auto
                    || spec.dimensions.contains_key(&w.id))
                && w.measurements
                    .iter()
                    .any(|m| m.feature.starts_with("notch:"))
                && matches!(w.shape, Shape::Linear { axis, .. }
                if placement::across(&frames["plan"], axis) == Some(false))
        })
        .count();
    if notch_lanes > 0 {
        let base = ((crate::layout::DIM_PAD - crate::layout::lane_band(1)).max(0.) / LANE).ceil()
            as usize
            + 1;
        bands.authored_plan_left = bands.authored_plan_left.max(base + notch_lanes);
    }
    bands.recess_lanes = wants.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    if bands.recess_lanes {
        // A recess carried by another view can still obstruct this view's
        // existing datum witnesses. Reserve their opposite strips before the
        // views and the iso are packed; a later corridor retry cannot create
        // paper between two views that were already placed against each other.
        let footprints: BTreeMap<_, _> = frames
            .iter()
            .map(|(name, f)| (*name, footprints::Footprints::new(analysis, f, name)))
            .collect();
        let mut extra = BTreeMap::new();
        for (i, w) in wants.iter().enumerate() {
            if owner[i].is_some() || !matches!(w.shape, Shape::Linear { .. }) {
                continue;
            }
            let (Some(view), Some(side)) = (chosen[i], sides[i]) else {
                continue;
            };
            let Some(p) = placement::place(&frames[view], w, side, lane[i]) else {
                continue;
            };
            if p.lines.iter().take(2).any(|line| {
                footprints[view]
                    .crossed_extension(*line, &w.measurements)
                    .iter()
                    .any(|id| id.starts_with("f:section_recess:"))
            }) {
                *extra.entry((view, side)).or_insert(0) += 1;
            }
        }
        for ((view, side), count) in extra {
            let target = match (view, side) {
                ("front", placement::Side::Below) => &mut bands.plate_front_above,
                ("front", placement::Side::Above) => &mut bands.front_below,
                ("front", placement::Side::Left) => &mut bands.front_right,
                ("front", placement::Side::Right) => &mut bands.plate_front_left,
                ("plan", placement::Side::Below) => &mut bands.plan_above,
                ("plan", placement::Side::Above) => &mut bands.plate_plan_below,
                ("plan", placement::Side::Left) => &mut bands.plate_plan_right,
                ("plan", placement::Side::Right) => &mut bands.plan_left,
                ("side", placement::Side::Below) => &mut bands.side_above,
                ("side", placement::Side::Above) => &mut bands.side_below,
                ("side", placement::Side::Left) => &mut bands.side_right,
                ("side", placement::Side::Right) => &mut bands.plate_side_left_lanes,
                _ => continue,
            };
            *target += count;
        }
    }
    for (i, w) in wants.iter().enumerate() {
        if callout_columns
            && matches!(w.shape, Shape::Callout { .. })
            && w.measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
            && let Some(view) = chosen[i]
            && let Some(k) = NAMES.iter().position(|v| *v == view)
        {
            let ink = label_ink(&w.terms, font());
            bands.recess_callouts[k] =
                bands.recess_callouts[k].max(FIRST + SHOULDER + ink.max[0] - ink.min[0] + 2.0);
        }
    }
    for (i, w) in wants.iter().enumerate() {
        if callout_columns
            && owner[i].is_none()
            && matches!(w.shape, Shape::Linear { .. })
            && w.measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
            && let Some(k) = chosen[i].and_then(|view| NAMES.iter().position(|v| *v == view))
        {
            let ink = label_ink(&w.terms, font());
            bands.recess_callouts[k] =
                bands.recess_callouts[k].max(ink.max[0] - ink.min[0] + ARROW + 3.0);
        }
    }
    if shared_bands
        && !excluded.is_empty()
        && wants.iter().enumerate().any(|(i, w)| {
            chosen[i] == Some("front")
                && matches!(w.shape, Shape::Linear { axis: 0, .. })
                && w.measurements
                    .iter()
                    .any(|m| m.parameter.contains("location"))
        })
    {
        // A table removes many plan ladders, but a front datum leg may need
        // the opposite corridor around the newly recognised recess footprint.
        // Reserve that real row before packing the aligned plan above it.
        bands.plate_front_above = bands.plate_front_above.max(1);
    }
    if shared_bands {
        bands.recess_chain_scale = recess_chain_scale(analysis);
        bands.plate_front_below = bands.plate_front_below.max(front_linear);
        bands.plate_plan_above = bands.plate_plan_above.max(plan_linear);
    }
    let mut aligned = Vec::new();
    for (i, w) in wants.iter().enumerate() {
        if !matches!(w.shape, Shape::Aligned { .. }) || owner[i].is_some() {
            continue;
        }
        if let (Some(view), Some(side)) = (chosen[i], sides[i]) {
            let mut display = w.clone();
            precision::labels(spec, std::iter::once(&mut display))?;
            aligned.push(AlignedDemand::new(
                &frames[view],
                &display,
                view,
                side,
                lane[i],
            ));
        }
    }
    Ok((bands, aligned))
}

/// Reserve explicitly requested linear strips before composition, at their
/// authored lane. The ordinary source lanes remain reserved for edit locality.
pub(crate) fn reserve_linear_intent(
    analysis: &Analysis,
    spec: &Spec,
    views: &[(&str, P3, P3)],
    excluded: &BTreeSet<String>,
    bands: &mut crate::layout::Lanes,
) -> Result<(), Invalid> {
    // A bare automatic placement is a no-op, including a bare pin. Its full
    // settled corridor is measured in composition; it requests no extra tier.
    if !spec.dimensions.values().any(|d| {
        d.place
            .as_ref()
            .is_some_and(|p| p.side.is_some() || p.lane.is_some())
    }) {
        return Ok(());
    }
    let (mut wants, _, _) = wants(analysis);
    let scheme_covered = schemes::apply(analysis, spec, &mut wants)?;
    if !spec.dimension_schemes.is_empty() {
        authored::append(analysis, spec, &mut wants)?;
    }
    if scheme_covered.is_empty() {
        wants.retain(|w| !excluded.contains(&w.id));
    } else {
        wants.retain(|w| !excluded.contains(&w.id) && !scheme_covered.contains_key(&w.id));
    }
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter(|(name, ..)| NAMES.contains(name))
        .map(|(name, direction, up)| {
            (
                *name,
                Frame::looking(analysis, *direction, *up, [0.0; 2], 1.0),
            )
        })
        .collect();
    chosen_views(&mut wants, spec, &frames)?;
    plates::prefer_readable(analysis, &mut wants, &frames, Some(spec));
    let (chosen, _) = route(&mut wants, &frames, analysis);
    let owner = placement::owners(analysis, &wants, &chosen);
    let (sides, lane) = placement::lanes(&frames, wants.iter().zip(chosen.iter().copied()), &owner);
    // Predict the same line-sharing reflow used by placement, before the
    // bands are composed. Scaling preserves shared stations and lane offsets.
    let mut occupied: BTreeMap<&str, Vec<Vec<[P2; 2]>>> = BTreeMap::new();
    let mut requested = Vec::new();
    for (i, w) in wants.iter().enumerate() {
        if !matches!(w.shape, Shape::Linear { .. }) || owner[i].is_some() {
            continue;
        }
        let (Some(view), Some(side)) = (chosen[i], sides[i]) else {
            continue;
        };
        if let Some(p) = spec
            .dimensions
            .get(&w.id)
            .filter(|d| !d.omit)
            .and_then(|d| d.place.as_ref())
            .filter(|p| p.side.is_some() || p.lane.is_some())
        {
            if let Some(ink) = placement::intent(&frames[view], w, p, side, lane[i], 0.0) {
                let span =
                    crate::output::linear_ink(&ink.lines).map_or(0.0, |l| l.span[1] - l.span[0]);
                requested.push((i, p.lane.unwrap_or(lane[i]), span));
            }
        } else if let Some(ink) = placement::place(&frames[view], w, side, lane[i]) {
            occupied.entry(view).or_default().push(ink.lines);
        }
    }
    requested.sort_by(|a, b| {
        (chosen[a.0], a.1)
            .cmp(&(chosen[b.0], b.1))
            .then(a.2.total_cmp(&b.2))
            .then(a.0.cmp(&b.0))
    });
    let mut resolved_lanes = BTreeMap::new();
    for (i, requested, _) in requested {
        let view = chosen[i].expect("routed linear");
        let side = sides[i].expect("linear strip");
        let p = spec.dimensions[&wants[i].id]
            .place
            .as_ref()
            .expect("explicit place");
        let taken = occupied.entry(view).or_default();
        for n in requested..requested + if p.pinned { 1 } else { 64 } {
            let mut trial = p.clone();
            trial.lane = Some(n);
            let Some(ink) = placement::intent(&frames[view], &wants[i], &trial, side, lane[i], 0.0)
            else {
                break;
            };
            if p.pinned
                || !taken
                    .iter()
                    .any(|q| crate::output::shared_dimension_line(&ink.lines, q))
            {
                taken.push(ink.lines);
                resolved_lanes.insert(i, n);
                break;
            }
        }
    }
    for (i, w) in wants.iter().enumerate() {
        let Some(p) = spec
            .dimensions
            .get(&w.id)
            .filter(|d| !d.omit)
            .and_then(|d| d.place.as_ref())
            .filter(|p| p.side.is_some() || p.lane.is_some())
        else {
            continue;
        };
        if !matches!(w.shape, Shape::Linear { .. }) {
            continue;
        }
        let (Some(view), Some(default_side)) = (chosen[i], sides[i]) else {
            continue;
        };
        let side = p.side.map_or(default_side, |s| match s {
            crate::placement_intent::Side::Above => placement::Side::Above,
            crate::placement_intent::Side::Below => placement::Side::Below,
            crate::placement_intent::Side::Left => placement::Side::Left,
            crate::placement_intent::Side::Right => placement::Side::Right,
        });
        let band = match (view, side) {
            ("front", placement::Side::Above) => &mut bands.plate_front_above,
            ("front", placement::Side::Below) => &mut bands.plate_front_below,
            ("front", placement::Side::Left) => &mut bands.plate_front_left,
            ("front", placement::Side::Right) => &mut bands.front_right,
            ("plan", placement::Side::Above) => &mut bands.plate_plan_above,
            ("plan", placement::Side::Below) => &mut bands.plate_plan_below,
            ("plan", placement::Side::Left) => &mut bands.authored_plan_left,
            ("plan", placement::Side::Right) => &mut bands.plate_plan_right,
            ("side", placement::Side::Above) => &mut bands.side_above,
            ("side", placement::Side::Below) => &mut bands.side_below,
            ("side", placement::Side::Left) => &mut bands.plate_side_left_lanes,
            ("side", placement::Side::Right) => &mut bands.side_right,
            _ => continue,
        };
        *band = (*band).max(
            resolved_lanes
                .get(&i)
                .copied()
                .unwrap_or_else(|| p.lane.unwrap_or(lane[i]))
                + 1,
        );
    }
    Ok(())
}

/// Judge the final drawn ink, including arrow flares, after all ordinary lane passes.
pub(crate) fn authored_placement_clear(spec: &Spec, dims: &[PlannedDimension]) -> bool {
    dims.iter()
        .filter(|d| {
            spec.dimensions.get(&d.id).is_some_and(|entry| {
                entry.between.is_some()
                    && (d.kind == "angular"
                        || (d.kind == "linear"
                            && entry.along.as_deref().is_none_or(|a| a == "true")))
            })
        })
        .all(|d| {
            d.placed.as_ref().is_some_and(|p| {
                dims.iter()
                    .filter(|q| q.id != d.id)
                    .filter_map(|q| q.placed.as_ref())
                    .all(|q| {
                        placement::annotations_clear(p, q) && placement::dimension_lines_clear(p, q)
                    })
            })
        })
}

/// A slot chain must have room for its arrows and its own text between its ends.
/// New recess annotations reserve extra paper rather than reducing these existing chains.
pub(crate) fn recess_chain_scale(analysis: &Analysis) -> f64 {
    let (canonical, established) = crate::planning::policy_key(analysis);
    crate::planning::memo(
        analysis,
        "recess-readable-scale",
        vec![u8::from(canonical), u8::from(established)],
        || recess_chain_scale_uncached(analysis),
    )
}

fn recess_chain_scale_uncached(analysis: &Analysis) -> f64 {
    if !gdt::demand(analysis, &|_| true).is_empty() {
        return 0.0;
    }
    let (items, ..) = wants(analysis);
    let native = analysis
        .recognition
        .as_ref()
        .is_some_and(|r| r.recogniser.starts_with("quiddity-rust "));
    if !items.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    }) {
        return 0.0;
    }
    items
        .iter()
        .filter_map(|w| {
            if w.value > 0.0
                && w.measurements.iter().any(|m| {
                    m.feature.starts_with("f:section_recess:")
                        && m.parameter == "channel_width.length"
                })
            {
                // Legacy bridged documents already carried these channels and
                // keep their established scale policy. This additional mouth
                // reservation is for the newly native recess information.
                return native.then_some(crate::sheet_fit::MIN_OPENING / w.value);
            }
            let feature = w.id.strip_suffix("/slot_length.length")?;
            if !items
                .iter()
                .any(|other| other.id == format!("{feature}/location_slot.length"))
                || w.value <= 0.0
            {
                return None;
            }
            let ink = label_ink(&w.terms, font());
            Some((ink.max[0] - ink.min[0] + 2.0 * ARROW + 2.0) / w.value)
        })
        .fold(0.0, f64::max)
}

/// The finite 1 mm leader/extension corridors used by the shared solve and QA.
pub(crate) fn extension_contacts(dims: &[PlannedDimension]) -> usize {
    let extensions: Vec<_> = dims
        .iter()
        .enumerate()
        .filter_map(|(i, d)| {
            let p = d.placed.as_ref()?;
            (d.kind == "linear" && p.arrows.len() == 2 && p.lines.len() >= 3).then_some((i, p))
        })
        .flat_map(|(i, p)| p.lines[..2].iter().map(move |e| (i, *e)))
        .collect();
    dims.iter()
        .filter(|d| d.kind == "callout")
        .filter_map(|d| d.placed.as_ref())
        .map(|p| leaders::extension_contacts(&extensions, &p.lines, p.label_box).0)
        .sum()
}

/// The planned dimensions under *spec*, placed around *views* on *sheet*, with what the spec
/// left out and the problems found. A spec naming a dimension the part does not have is refused
/// with the nearest IDs.
pub fn plan(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) -> Result<Planned, Invalid> {
    plan_with_obstacles(
        analysis,
        spec,
        sheet,
        views,
        &[],
        &[],
        &footprints::Cache::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_with_obstacles(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    obstacles: &[PlannedDimension],
    held: &[PlannedDimension],
    cache: &footprints::Cache,
) -> Result<Planned, Invalid> {
    plan_with_obstacles_excluding(
        analysis,
        spec,
        sheet,
        views,
        obstacles,
        held,
        cache,
        &BTreeSet::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_with_obstacles_excluding(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    obstacles: &[PlannedDimension],
    held: &[PlannedDimension],
    cache: &footprints::Cache,
    excluded: &BTreeSet<String>,
) -> Result<Planned, Invalid> {
    Ok(plan_inventory(
        analysis, spec, sheet, views, obstacles, held, cache, excluded,
    )?
    .0)
}

/// The complete placement inventory keeps omission edits local during label settlement.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_inventory(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    obstacles: &[PlannedDimension],
    held: &[PlannedDimension],
    cache: &footprints::Cache,
    excluded: &BTreeSet<String>,
) -> Result<(Planned, Vec<PlannedDimension>), Invalid> {
    if !crate::planning::enabled(analysis) {
        return plan_inventory_uncached(
            analysis, spec, sheet, views, obstacles, held, cache, excluded,
        );
    }
    // Detail-vs-parent proofs and hole-table trials can ask for identical placed ink.
    // Table selection happens after this stage; retain every placement input, but do
    // not solve again solely for that presentation policy or a frame's explanation.
    let mut geometry_spec = spec.clone();
    if !has_recess_dimensions(analysis) {
        geometry_spec.hole_table = crate::hole_table::Setting::Auto;
    }
    // The placer reads the title block rectangle, never its cells or their text.
    // Keep the actual rectangle below: an expanded title block still invalidates ink.
    geometry_spec.sheet.title.clear();
    // Placement reads the resolved sheet and frames below, not whether the caller
    // selected that same size/scale or the allocator selected it automatically.
    geometry_spec.sheet.size = None;
    geometry_spec.sheet.scale = None;
    geometry_spec.views.retain(|name, view| {
        let inactive = crate::detail::NAMES.contains(&name.as_str())
            && *view
                == crate::model::ViewSpec {
                    omit: true,
                    ..Default::default()
                }
            && !views.iter().any(|v| v.id.strip_prefix("v:") == Some(name));
        let named = spec.dimensions.values().any(|d| {
            d.view.as_ref() == Some(name)
                || d.place.as_ref().and_then(|p| p.view.as_ref()) == Some(name)
        }) || spec
            .annotations
            .values()
            .any(|d| d.place.as_ref().and_then(|p| p.view.as_ref()) == Some(name));
        !inactive || named
    });
    let mut geometry_sheet = sheet.clone();
    geometry_sheet.title.clear();
    geometry_sheet.title_cells.clear();
    geometry_sheet.basis.clear();
    geometry_sheet.arrangement.clear();
    geometry_sheet.default = false;
    geometry_sheet.fill = 0.0;
    let mut geometry_views = views.to_vec();
    for view in &mut geometry_views {
        view.basis.clear();
    }
    use sha2::{Digest, Sha256};
    let analysis_key: [u8; 32] =
        crate::planning::memo(analysis, "analysis_digest", Vec::new(), || {
            Sha256::digest(serde_json::to_vec(analysis).expect("Analysis serializes")).into()
        });
    let key = serde_json::to_vec(&(
        analysis_key,
        crate::planning::policy_key(analysis),
        geometry_spec,
        geometry_sheet,
        geometry_views,
        obstacles,
        held,
        excluded,
    ))
    .expect("dimension placement inputs serialize");
    crate::planning::memo(analysis, "dimension_placement", key.clone(), || {
        // Stateless reconstruction and title edits revisit the same physical placement.
        // This bounded process memo owns its results and hashes every read input, including
        // the complete Analysis, so edited PMI/geometry cannot reuse an old placement.
        type Inventory = Result<(Planned, Vec<PlannedDimension>), Invalid>;
        static MEMO: crate::planning::BoundedMemo<Inventory> =
            crate::planning::BoundedMemo::new(64);
        let digest: [u8; 32] = Sha256::digest(&key).into();
        if let Some(result) = MEMO.get(&digest) {
            return result;
        }
        let result = plan_inventory_uncached(
            analysis, spec, sheet, views, obstacles, held, cache, excluded,
        );
        MEMO.insert(digest, result.clone());
        result
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_inventory_uncached(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    obstacles: &[PlannedDimension],
    held: &[PlannedDimension],
    cache: &footprints::Cache,
    excluded: &BTreeSet<String>,
) -> Result<(Planned, Vec<PlannedDimension>), Invalid> {
    let first = plan_inventory_ordered(
        analysis, spec, sheet, views, obstacles, held, cache, excluded, 0,
    )?;
    let new = |d: &PlannedDimension| {
        !d.measurements.is_empty()
            && d.measurements
                .iter()
                .all(|m| m.feature.starts_with("f:section_recess:"))
    };
    if !first.0.0.iter().any(|d| d.kind == "linear" && new(d))
        || !first.0.0.iter().any(|d| d.kind == "callout" && new(d))
        || spec.dimensions.values().any(|d| d.place.is_some())
    {
        return Ok(first);
    }
    let score = |plan: &Planned| {
        let (dims, _, problems) = plan;
        let hard = problems
            .iter()
            .filter(|p| {
                matches!(
                    p.code.as_str(),
                    "annotation_off_sheet"
                        | "annotation_over_title_block"
                        | "annotation_overlap"
                        | "annotation_ink_overlap"
                        | "annotation_over_view"
                        | "annotation_line_over_view"
                        | "dimension_line_shared"
                        | "dimension_text_stray"
                )
            })
            .count();
        let extensions: Vec<_> = dims
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                d.placed
                    .as_ref()
                    .filter(|p| d.kind == "linear" && p.lines.len() >= 3)
                    .map(|p| (i, p))
            })
            .flat_map(|(i, p)| p.lines[..2].iter().map(move |e| (i, *e)))
            .collect();
        let contacts: usize = dims
            .iter()
            .filter(|d| d.kind == "callout")
            .filter_map(|d| d.placed.as_ref())
            .map(|p| leaders::extension_contacts(&extensions, &p.lines, p.label_box).0)
            .sum();
        let span = dims
            .iter()
            .filter(|d| d.kind == "linear")
            .filter_map(|d| d.placed.as_ref().map(|p| (d, p)))
            .filter(|(d, p)| {
                label_lanes::off_span(
                    p,
                    dims.iter()
                        .filter(|q| q.id != d.id && q.view == d.view && q.kind == "linear")
                        .filter_map(|q| q.placed.as_ref()),
                )
            })
            .count();
        let mut crossings = BTreeSet::new();
        for p in problems {
            if p.code == "leader_through_feature" {
                crossings.insert((p.code.clone(), p.subjects.join("|")));
            }
            if p.code == "extension_through_feature" {
                crossings.insert((
                    p.subjects[0].clone(),
                    p.id.rsplit(":line:").next().unwrap_or_default().into(),
                ));
            }
        }
        let ink = problems
            .iter()
            .filter(|p| {
                matches!(
                    p.code.as_str(),
                    "annotation_leader_crossing" | "annotation_outside_area"
                )
            })
            .count();
        let length: f64 = dims
            .iter()
            .filter(|d| d.kind == "callout")
            .filter_map(|d| d.placed.as_ref())
            .flat_map(|p| &p.lines)
            .map(|[a, b]| (a[0] - b[0]).hypot(a[1] - b[1]))
            .sum();
        (
            hard,
            span,
            contacts,
            crossings.len(),
            ink,
            (length * 1000.0).round() as u64,
        )
    };
    let native = analysis
        .recognition
        .as_ref()
        .is_some_and(|r| r.recogniser.starts_with("quiddity-rust "));
    let new_crossing = |plan: &Planned| {
        plan.2.iter().any(|p| {
            (p.code == "leader_through_feature"
                || (native && p.code == "extension_through_feature"))
                && p.subjects
                    .first()
                    .is_some_and(|id| plan.0.iter().any(|d| &d.id == id && new(d)))
        })
    };
    let a = score(&first.0);
    if a.0 == 0 && a.1 == 0 && !new_crossing(&first.0) {
        return Ok(first);
    }
    let alternative = plan_inventory_ordered(
        analysis, spec, sheet, views, obstacles, held, cache, excluded, 1,
    )?;
    let mut best = if score(&alternative.0) < a {
        alternative
    } else {
        first
    };
    if score(&best.0).0 > 0 || new_crossing(&best.0) {
        let mut joint_held = held.to_vec();
        joint_held.extend(
            best.0
                .0
                .iter()
                .filter(|d| {
                    d.kind == "callout"
                        && new(d)
                        && !best.0.2.iter().any(|p| {
                            p.subjects.contains(&d.id)
                                && matches!(
                                    p.code.as_str(),
                                    "annotation_ink_overlap"
                                        | "annotation_overlap"
                                        | "annotation_over_view"
                                        | "annotation_off_sheet"
                                        | "leader_through_feature"
                                )
                        })
                })
                .cloned(),
        );
        let joint = plan_inventory_ordered(
            analysis,
            spec,
            sheet,
            views,
            obstacles,
            &joint_held,
            cache,
            excluded,
            2,
        )?;
        if score(&joint.0) < score(&best.0) {
            best = joint;
        }
    }
    if native
        && !excluded.is_empty()
        && score(&best.0).0 == 0
        && best.0.2.iter().any(|p| {
            p.code == "extension_through_feature"
                && p.subjects
                    .first()
                    .is_some_and(|id| best.0.0.iter().any(|d| &d.id == id && new(d)))
        })
    {
        // A schedule removes its old hole locations. Compare preserving the
        // remaining existing corridors while new witnesses claim theirs first.
        // This avoids buying a short new form leader with a blocked datum leg.
        let preserved = plan_inventory_ordered(
            analysis, spec, sheet, views, obstacles, held, cache, excluded, 3,
        )?;
        if score(&preserved.0) < score(&best.0) {
            best = preserved;
        }
    }
    Ok(best)
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn plan_inventory_ordered(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    obstacles: &[PlannedDimension],
    held: &[PlannedDimension],
    cache: &footprints::Cache,
    excluded: &BTreeSet<String>,
    stage: u8,
) -> Result<(Planned, Vec<PlannedDimension>), Invalid> {
    let (mut wants, mut problems, notes) = wants(analysis);
    let scheme_covered = schemes::apply(analysis, spec, &mut wants)?;
    authored::append(analysis, spec, &mut wants)?;
    let retired: Vec<_> = if scheme_covered.is_empty() {
        Vec::new()
    } else {
        let retired = wants
            .iter()
            .filter(|w| scheme_covered.contains_key(&w.id))
            .cloned()
            .collect();
        wants.retain(|w| !scheme_covered.contains_key(&w.id));
        retired
    };
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            (NAMES.contains(&name) || crate::detail::NAMES.contains(&name))
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    detail_routes(analysis, &mut wants, views);
    let authored_ownership = authored::ownership(analysis, spec, &wants);
    problems.extend(chosen_views(&mut wants, spec, &frames)?);
    plates::prefer_readable(analysis, &mut wants, &frames, Some(spec));
    let (mut chosen, mut end_on) = route(&mut wants, &frames, analysis);
    // Display intent may not silently undo the established one-owner span grouping.
    let display_owners = if spec.precision.is_empty() {
        Vec::new()
    } else {
        placement::owners(analysis, &wants, &chosen)
    };
    precision::apply(spec, &mut wants)?;
    for (i, owner) in display_owners.iter().enumerate() {
        // Baseline arithmetic can convey a shorter interval through longer
        // stations. It shares placement ownership, not one printed scalar.
        if let Some(j) = owner
            && (wants[i].value - wants[*j].value).abs() <= 1e-6
            && (precision::leading(spec, &wants[i]).is_some()
                || precision::leading(spec, &wants[*j]).is_some())
            && precision::leading(spec, &wants[i]).map_or_else(
                || fmt(wants[i].value),
                |p| precision::format(wants[i].value, p),
            ) != precision::leading(spec, &wants[*j]).map_or_else(
                || fmt(wants[*j].value),
                |p| precision::format(wants[*j].value, p),
            )
        {
            return Err(Invalid {
                message: format!(
                    "conflicting precision for coincident dimensions {} and {}; shared measurements require matching display precision",
                    wants[i].id, wants[*j].id
                ),
                choices: vec![],
            });
        }
    }
    let ids: BTreeSet<&str> = wants
        .iter()
        .chain(&retired)
        .map(|w| w.id.as_str())
        .collect();
    // ARCH0016-5: each dimension is in exactly one view, under one ID. A duplicate is a planner
    // bug, checked in every build.
    assert_eq!(
        ids.len(),
        wants.len() + retired.len(),
        "dimension IDs are unique"
    );
    for (id, entry) in &spec.dimensions {
        if !ids.contains(id.as_str()) {
            let mut near: Vec<&str> = ids.iter().copied().collect();
            near.sort_by_key(|c| (levenshtein(id, c), *c));
            near.truncate(5);
            // A `draftwright.spec/1` ID named its view (`d:<feature>/<parameter>@<view>`); a spec
            // of version 2 is read with the IDs it has, so such a key reads as its stem.
            let stem = id.rsplit_once('@').map(|(s, _)| s);
            let reads = stem.filter(|s| ids.contains(s));
            let mut choices: Vec<String> = reads.iter().map(|s| s.to_string()).collect();
            for c in near {
                if !choices.iter().any(|x| x == c) {
                    choices.push(c.into());
                }
            }
            choices.truncate(5);
            let message = match reads {
                Some(s) => format!(
                    "There is no dimension {id:?} on this part: a dimension's ID no longer names its view (draftwright.spec/2), so it reads as {s:?}."
                ),
                None => format!("There is no dimension {id:?} on this part."),
            };
            return Err(Invalid { message, choices });
        }
        if entry.omit && spec.dimension_source == DimensionSource::Authored {
            return Err(Invalid {
                message: format!(
                    "{id:?} is omitted inside an authored set; in an authored set a dimension is left out by not listing it."
                ),
                choices: vec![],
            });
        }
    }
    // Table-owned measurements keep their IDs, but reserve no leader or ladder lane.
    let keep: Vec<_> = wants.iter().map(|w| !excluded.contains(&w.id)).collect();
    let mut index = 0;
    wants.retain(|_| {
        let k = keep[index];
        index += 1;
        k
    });
    let mut index = 0;
    chosen.retain(|_| {
        let k = keep[index];
        index += 1;
        k
    });
    let mut index = 0;
    end_on.retain(|_| {
        let k = keep[index];
        index += 1;
        k
    });
    // The spec chooses what is drawn, and states the tolerances and fits on the kept labels.
    let all = wants.clone();
    let owner = placement::owners(analysis, &all, &chosen);
    let mut omitted: Vec<_> = retired
        .into_iter()
        .map(|w| Omitted {
            state: State::Covered {
                by: scheme_covered[&w.id].clone(),
            },
            basis: format!(
                "conveyed by {}: the Spec station network determines this interval",
                scheme_covered[&w.id]
            ),
            id: w.id,
            label: w.terms.join(" "),
        })
        .collect();
    let mut kept = Vec::new();
    // The default consolidation of every item, independent of the spec: what an omission or an
    // authored set is measured against (lanes, reference ink, authored-set parity).
    let default_plan = (spec.dimension_source == DimensionSource::Authored
        || spec.dimensions.values().any(|d| d.omit))
    .then(|| {
        let mut defaults: Vec<_> = all
            .iter()
            .cloned()
            .zip(chosen.iter().copied())
            .map(|(w, view)| (w, true, view, Vec::new()))
            .collect();
        let mut unused = Vec::new();
        plates::consolidate(analysis, &mut defaults, &mut unused);
        bosses::consolidate(analysis, &mut defaults, &mut unused);
        let mut nominal = vec![Vec::new(); defaults.len()];
        holes::consolidate(analysis, &mut defaults, &mut nominal, &mut unused);
        recesses::consolidate(&mut defaults, &mut nominal, &mut unused);
        plates::share(analysis, &mut defaults, &mut nominal, &mut unused);
        share_statements(&mut defaults, &mut nominal, &mut unused, true);
        share_pmi(analysis, &mut defaults, &mut nominal, &mut unused);
        redundancy::consolidate(analysis, &mut defaults, &mut nominal, &mut unused);
        (defaults, unused)
    });
    // A rung the defaults hand to an item outside its set (a height the overall extent conveys)
    // is not the set's to approve: an authored set that leaves that owner out leaves it out too.
    let conveyed: BTreeSet<&str> = default_plan
        .as_ref()
        .map(|(_, unused)| {
            unused
                .iter()
                .filter(|u| {
                    matches!(&u.state, State::Covered { by }
                        if correlated_set(by) != correlated_set(&u.id))
                })
                .map(|u| u.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    let (with_set, set_problems) = correlated_sets(spec, &wants, &conveyed);
    problems.extend(set_problems);
    for ((w, view), along) in wants.into_iter().zip(chosen.iter().copied()).zip(end_on) {
        let entry = spec.dimensions.get(&w.id);
        if let Some(basis) = with_set.get(&w.id) {
            match spec.dimension_source {
                DimensionSource::Auto => omitted.push(Omitted {
                    label: w.terms.join(" "),
                    id: w.id,
                    basis: basis.clone(),
                    state: State::OmittedByUser,
                }),
                DimensionSource::Authored => kept.push((w, true, view, along)),
            }
            continue;
        }
        match spec.dimension_source {
            DimensionSource::Auto if entry.is_some_and(|e| e.omit) => omitted.push(Omitted {
                label: w.terms.join(" "),
                id: w.id,
                basis: "authored: the spec omits it".into(),
                state: State::OmittedByUser,
            }),
            DimensionSource::Authored if entry.is_none() => {
                if let Some(by) = authored_ownership.covered.get(&w.id) {
                    omitted.push(Omitted {
                        label: w.terms.join(" "), id: w.id,
                        basis: format!("spec-authored: this quantity is stated by {by}; the unlisted automatic candidate is withheld (one owner)"),
                        state: State::Covered { by: by.clone() },
                    });
                    continue;
                }
                // An authored set still keeps the automatic inventory. Surface every
                // unstated item by name, even when another measurement locates its feature.
                problems.push(Problem {
                    id: format!("p:dimension_not_authored:{}", w.id),
                    code: "dimension_not_authored".into(),
                    severity: "warning".into(),
                    message: format!(
                        "{} is not drawn: the authored dimension set does not list it. Add it to the set to draw it.",
                        crate::wording::capital(&wording_of(&w))
                    ),
                    subjects: vec![w.id.clone()],
                    fix: vec![Op::set(format!("dimensions/{}", crate::items::key(&w.id)), serde_json::json!({}))],
                    ..Default::default()
                });
                omitted.push(Omitted {
                    label: w.terms.join(" "),
                    id: w.id,
                    basis: "authored: not in the spec's complete set (dimension_source authored)"
                        .into(),
                    state: State::NotAuthored,
                });
            }
            _ => kept.push((w, entry.is_none(), view, along)),
        }
    }
    plates::consolidate(analysis, &mut kept, &mut omitted);
    bosses::consolidate(analysis, &mut kept, &mut omitted);
    pmi_dimensions::yield_to_spec(spec, &notes, &mut kept, &mut problems);
    pmi_dimensions::hide_fit_limits(spec, &notes, &mut kept);
    precision::labels(spec, kept.iter_mut().map(|(w, ..)| w))?;
    let mut given = tolerance::apply(analysis, spec, &mut kept, &omitted, &mut problems)?;
    holes::consolidate(analysis, &mut kept, &mut given, &mut omitted);
    recesses::consolidate(&mut kept, &mut given, &mut omitted);
    plates::share(analysis, &mut kept, &mut given, &mut omitted);
    share_statements(&mut kept, &mut given, &mut omitted, true);
    share_pmi(analysis, &mut kept, &mut given, &mut omitted);
    let references = redundancy::consolidate(analysis, &mut kept, &mut given, &mut omitted);
    problems.extend(pmi_dimensions::omitted(analysis, &all, &kept, &omitted));

    // Then everything the part would have is placed: each item's lane, and where its label
    // settles, come from the full set, so omitting or restoring one leaves the others where they
    // were (ARCH0003-2). A kept item is placed with its label as drawn (its tolerance or fit
    // included); one the spec omits keeps its default label and its place, so it leaves a gap.
    // An item the defaults merge away (a height the overall extent conveys among them) takes no
    // lane: it shares its survivor's.
    // Omission changes a reference back to a controlling dimension, but not its
    // witness lane or text centre. Reserve the default reference's ink during layout.
    let mut reference_terms = BTreeMap::new();
    if let Some((defaults, unused)) = default_plan {
        if spec.dimension_source == DimensionSource::Authored {
            redundancy::authored_parity(&all, &defaults, &unused, &mut kept, &mut omitted);
        }
        reference_terms.extend(defaults.into_iter().filter_map(|(w, ..)| {
            w.terms
                .first()
                .is_some_and(|t| t.starts_with('('))
                .then_some((w.id, w.terms))
        }));
    }
    wording::apply(spec, &mut kept, &mut given)?;
    wording::check_covered(spec, &kept, &omitted)?;
    let labels: BTreeMap<&str, &[String]> = kept
        .iter()
        .map(|(w, ..)| (w.id.as_str(), w.terms.as_slice()))
        .collect();
    let all: Vec<Want> = all
        .into_iter()
        .map(|mut w| {
            if let Some((statement, ..)) = kept.iter().find(|(statement, ..)| statement.id == w.id)
            {
                w.measurements = statement.measurements.clone();
                if w.measurements
                    .first()
                    .is_some_and(|m| m.parameter == "seat_diameter.diameter")
                {
                    w.shape = statement.shape.clone();
                }
            }
            if spec
                .dimensions
                .get(&w.id)
                .is_none_or(|d| d.tolerance.is_none() && d.text.is_none() && d.style.is_none())
                && let Some(terms) = reference_terms.get(&w.id)
            {
                w.terms = terms.clone();
            } else if let Some(terms) = labels.get(w.id.as_str()) {
                w.terms = terms.to_vec();
            }
            w
        })
        .collect();
    let (mut sides, mut lane) =
        placement::lanes(&frames, all.iter().zip(chosen.iter().copied()), &owner);
    for (i, w) in all.iter().enumerate() {
        if matches!(w.shape, Shape::Angular { .. })
            && spec
                .dimensions
                .get(&w.id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.side.is_some())
        {
            problems.push(Problem {id:format!("p:placement_side_not_applicable:{}",w.id),code:"placement_side_not_applicable".into(),severity:"warning".into(),message:format!("{} keeps its measured angular sector; place it with lane and along, without a side.",crate::wording::capital(&wording_of(w))),subjects:vec![w.id.clone()],fix:vec![Op::remove(format!("dimensions/{}/place/side",crate::items::key(&w.id)))],..Default::default()});
        }
        if spec
            .dimensions
            .get(&w.id)
            .and_then(|e| e.place.as_ref())
            .is_some_and(|p| p.offset.is_some())
            && !matches!(w.shape, Shape::Callout { .. })
        {
            return Err(Invalid {
                message: format!("The placement offset of {} applies only to callouts.", w.id),
                choices: vec!["side".into(), "lane".into(), "along".into()],
            });
        }
        if let Some(p) = spec.dimensions.get(&w.id).and_then(|e| e.place.as_ref())
            && let (Shape::Linear { axis, .. }, Some(side), Some(v)) = (&w.shape, p.side, chosen[i])
            && placement::across(&frames[v], *axis).is_some_and(|h| {
                h != matches!(
                    side,
                    crate::placement_intent::Side::Above | crate::placement_intent::Side::Below
                )
            })
        {
            let horizontal = placement::across(&frames[v], *axis) == Some(true);
            return Err(Invalid {
                message: format!(
                    "The side of {} must be parallel to its dimension line in {v}.",
                    w.id
                ),
                choices: if horizontal {
                    vec!["above".into(), "below".into()]
                } else {
                    vec!["left".into(), "right".into()]
                },
            });
        }
    }
    let mut intended: Vec<Option<Placed>> = all
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let p = spec.dimensions.get(&w.id)?.place.as_ref()?;
            let v = chosen[i]?;
            // #12: an offset pin is measured from the automatic leader tip in this view.
            if let Some(offset) = p.offset {
                let held = held
                    .iter()
                    .find(|d| d.id == w.id && d.view == format!("v:{v}"))
                    .and_then(|d| Some((d.placed.as_ref()?, d.terms.as_slice())));
                return leaders::offset_intent(&frames[v], w, offset, held);
            }
            if crate::placement_intent::automatic_pin(p)
                && let Some(d) = held
                    .iter()
                    .find(|d| d.id == w.id && d.view == format!("v:{v}"))
            {
                return d
                    .placed
                    .as_ref()
                    .map(|held| placement::relabel(held, &d.terms, w));
            }
            let callout_width = held
                .iter()
                .filter(|d| d.kind == "callout" && d.view == format!("v:{v}"))
                .filter_map(|d| d.placed.as_ref())
                .map(|p| p.label_box[2] - p.label_box[0])
                .fold(0.0, f64::max);
            // Angular ink has no strip. Its intent handler measures the requested
            // radius directly and does not use this linear/callout side argument.
            let side = sides[i].or_else(|| {
                matches!(w.shape, Shape::Angular { .. }).then_some(placement::Side::Below)
            })?;
            placement::intent(&frames[v], w, p, side, lane[i], callout_width)
        })
        .collect();
    // #32: an explicit side/lane (as a proposed fix sets) re-runs its side's stack spacing, so
    // two dimensions never share one dimension line; automatic linear items hold their lanes.
    let places: Vec<_> = all
        .iter()
        .map(|w| spec.dimensions.get(&w.id).and_then(|e| e.place.as_ref()))
        .collect();
    let mut occupied: BTreeMap<&'static str, Vec<Vec<[P2; 2]>>> = BTreeMap::new();
    for (i, w) in all.iter().enumerate() {
        if intended[i].is_none()
            && owner[i].is_none()
            && matches!(w.shape, Shape::Linear { .. })
            && let (Some(v), Some(side)) = (chosen[i], sides[i])
            && let Some(p) = placement::place(&frames[v], w, side, lane[i])
        {
            occupied.entry(v).or_default().push(p.lines);
        }
    }
    let page = placement::Page::new(analysis, sheet, views);
    placement::respace(
        &frames,
        &all,
        &chosen,
        &places,
        &sides,
        &lane,
        &mut occupied,
        &page,
        &mut intended,
    );
    let prescribed = intended.clone();
    let locked: Vec<_> = intended.iter().map(Option::is_some).collect();
    let pin_ink: Vec<_> = all
        .iter()
        .zip(&prescribed)
        .filter_map(|(w, p)| {
            spec.dimensions
                .get(&w.id)?
                .place
                .as_ref()?
                .pinned
                .then_some((w.id.as_str(), p.as_ref()?))
        })
        .chain(
            obstacles
                .iter()
                .filter_map(|d| Some((d.id.as_str(), d.placed.as_ref()?))),
        )
        .collect();
    for (i, w) in all.iter().enumerate() {
        if let (Some(side), Some(v), Some(placed)) = (
            spec.dimensions
                .get(&w.id)
                .and_then(|e| e.place.as_ref())
                .and_then(|p| p.side),
            chosen[i],
            intended[i].as_mut(),
        ) {
            let view = views
                .iter()
                .find(|view| view.id == format!("v:{v}"))
                .unwrap();
            leaders::route_intent(
                &cache.get(analysis, view),
                &frames[v],
                w,
                placed,
                side,
                &pin_ink,
                spec.dimensions[&w.id].place.as_ref().unwrap().pinned,
            );
        }
    }
    placement::fall_through(
        &frames, &all, &chosen, &owner, &mut sides, &mut lane, analysis, sheet, views,
    );
    let drawn: Vec<bool> = all
        .iter()
        .map(|w| labels.contains_key(w.id.as_str()))
        .collect();
    let alone = placement::separate(&owner, &drawn, &chosen, &sides, &mut lane);
    // Local tolerances measure band lengths, never cumulative positions. Keep those
    // explicit requirements in a separate stack opposite the inspection baseline.
    let has_baseline = all.iter().any(|w| w.id.ends_with("/step_position.length"));
    let local_tolerances: std::collections::BTreeSet<usize> = all
        .iter()
        .enumerate()
        .filter(|(i, w)| {
            has_baseline
                && owner[*i].is_some()
                && drawn[*i]
                && w.id.ends_with("/step.length")
                && spec
                    .dimensions
                    .get(&w.id)
                    .is_some_and(|d| d.tolerance.is_some() && d.place.is_none())
        })
        .map(|(i, _)| i)
        .collect();
    let mut locals: Vec<_> = local_tolerances.iter().copied().collect();
    locals.sort_by(|&i, &j| all[i].value.total_cmp(&all[j].value).then(i.cmp(&j)));
    let mut next = BTreeMap::new();
    for i in locals {
        let Some((view, side)) = chosen[i].zip(sides[i]) else {
            continue;
        };
        let opposite = match side {
            placement::Side::Right => placement::Side::Left,
            placement::Side::Left => placement::Side::Right,
            placement::Side::Above => placement::Side::Below,
            placement::Side::Below => placement::Side::Above,
        };
        let n = next.entry((view, opposite)).or_insert_with(|| {
            (0..all.len())
                .filter(|j| {
                    chosen[*j] == Some(view)
                        && sides[*j] == Some(opposite)
                        && matches!(all[*j].shape, Shape::Linear { .. })
                        && owner[*j].is_none()
                })
                .map(|j| lane[j] + 1)
                .max()
                .unwrap_or(0)
        });
        sides[i] = Some(opposite);
        lane[i] = *n;
        *n += 1;
    }
    let mut placed: Vec<Option<Placed>> = all
        .iter()
        .zip(&chosen)
        .zip(sides.iter().zip(&lane))
        .zip(&owner)
        .enumerate()
        .map(|(i, (((w, view), (side, lane)), owner))| {
            view.zip(*side)
                .filter(|_| owner.is_none() || local_tolerances.contains(&i))
                .and_then(|(v, side)| placement::place(&frames[v], w, side, *lane))
        })
        .collect();
    // A tolerance can separate a counted carrier from an alias. Both already
    // occupy their distinct lanes during the corridor solve, before leaders move.
    for i in 0..all.len() {
        if alone[i]
            && let (Some(view), Some(side)) = (chosen[i], sides[i])
        {
            placed[i] = placement::place(&frames[view], &all[i], side, lane[i]);
        }
    }
    for (i, p) in intended.iter().enumerate() {
        if p.is_some() {
            placed[i] = p.clone();
        } else if owner[i].is_none()
            && matches!(all[i].shape, Shape::Linear { .. })
            && let Some(d) = held.iter().find(|d| {
                d.id == all[i].id && chosen[i].is_some_and(|v| d.view == format!("v:{v}"))
            })
            && let Some(p) = &d.placed
        {
            // Start around pins from the settled layout, rather than reintroducing collisions
            // that the canonical label slides already removed.
            placed[i] = Some(placement::relabel(p, &d.terms, &all[i]));
        }
    }
    let fixed: Vec<bool> = all
        .iter()
        .map(|w| {
            spec.dimensions
                .get(&w.id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.pinned)
        })
        .collect();
    placement::avoid_fixed(
        &frames,
        &all,
        &chosen,
        &mut sides,
        &mut lane,
        &fixed,
        &drawn,
        &mut placed,
        obstacles,
        &page,
    );
    placement::settle_fixed(&mut placed, &fixed, obstacles, analysis, sheet, views);
    // Edited text can outgrow the canonical lanes. Keep the canonical layout stable
    // for pins and omission edits; their avoidance pass above owns that geometry.
    let recess_context = all.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    if matches!(stage, 0 | 3) && recess_context && intended.iter().all(Option::is_none) {
        leaders::seed_established(analysis, spec, views, &all, &chosen, &mut placed, &page);
    }
    if (recess_context
        || spec
            .dimensions
            .values()
            .any(|d| d.tolerance.is_some() || d.text.is_some() || d.style.is_some())
        || spec.features.values().any(|f| f.fit.is_some()))
        && intended.iter().all(Option::is_none)
    {
        placement::clear_linear(
            analysis,
            cache,
            views,
            &frames,
            &all,
            &chosen,
            &mut sides,
            &mut lane,
            &mut placed,
            obstacles,
            &locked,
            &page,
            stage != 3,
        );
    }
    let shared_owner: Vec<_> = owner
        .iter()
        .zip(&alone)
        .map(|(owner, alone)| if *alone { None } else { *owner })
        .collect();
    placement::share_lanes(&shared_owner, &mut sides, &mut lane);
    let body_fixed: Vec<bool> = all
        .iter()
        .enumerate()
        .map(|(i, w)| {
            fixed[i]
                || spec
                    .dimensions
                    .get(&w.id)
                    .is_some_and(|entry| entry.place.is_some())
        })
        .collect();
    problems.extend(placement::body_chains(
        analysis,
        &frames,
        &all,
        &chosen,
        &body_fixed,
        &mut placed,
        &page,
    ));
    let interior_proofs = interior::proofs(analysis, views);
    let notch_void = |view: &str, p: &Placed| {
        let Some(frame) = frames.get(view) else {
            return false;
        };
        let b = frame.outline();
        // Interior provenance covers the whole annotation, inside its own view.
        // An ordinary exterior sector must not inherit an own-view-only proof.
        if !page.fits_interior(p, view)
            || !placement::ink_points(p)
                .all(|q| q[0] >= b[0] && q[0] <= b[2] && q[1] >= b[1] && q[1] <= b[3])
        {
            return false;
        }
        views
            .iter()
            .find(|v| v.id == format!("v:{view}"))
            .is_some_and(|v| {
                interior_proofs
                    .get(&v.id)
                    .is_some_and(|proof| proof.exterior_annotation_clear(v, p))
            })
    };
    angular::place(
        &frames,
        &all,
        &chosen,
        &sides,
        &fixed,
        obstacles,
        &mut placed,
        &page,
        &notch_void,
    );
    // Radius exceptions are placed around the existing drawing ink. Their addition must not
    // reroute unrelated approved callouts into clutter.
    let mut callout_views = chosen.clone();
    for (i, w) in all.iter().enumerate() {
        if owner[i].is_some()
            && !drawn[i]
            && matches!(w.shape, Shape::Callout { .. })
            && w.measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
        {
            callout_views[i] = None;
        }
    }
    let mut prior = callout_views.clone();
    for (i, w) in all.iter().enumerate() {
        if w.measurements.iter().any(|m| m.parameter == "blend.radius")
            || matches!(w.shape, Shape::Callout { .. })
                && w.measurements
                    .iter()
                    .any(|m| m.feature.starts_with("f:section_recess:"))
        {
            prior[i] = None;
        }
    }
    // Existing leaders claim their corridors before new recess witnesses. Restore
    // those witnesses into the shared solve around the established leader ink.
    let recess_linear: Vec<_> = if recess_context {
        all.iter()
            .enumerate()
            .filter(|(i, w)| {
                !locked[*i]
                    && matches!(w.shape, Shape::Linear { .. })
                    && !w.measurements.is_empty()
                    && w.measurements
                        .iter()
                        .all(|m| m.feature.starts_with("f:section_recess:"))
            })
            .map(|(i, _)| (i, placed[i].take()))
            .collect()
    } else {
        Vec::new()
    };
    let mut leader_problems = if obstacles.is_empty() {
        leaders::place(
            analysis,
            spec,
            cache,
            views,
            &frames,
            &all,
            &prior,
            &sides,
            &mut placed,
            &page,
        )
    } else {
        leaders::place_with_obstacles(
            analysis,
            spec,
            cache,
            views,
            &frames,
            &all,
            &prior,
            &sides,
            &mut placed,
            &page,
            obstacles,
        )
    };
    if !recess_linear.is_empty() {
        let mut established = chosen.clone();
        for (i, w) in all.iter().enumerate() {
            if w.measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
            {
                established[i] = None;
            }
        }
        leader_problems.extend(leaders::place_with_obstacles(
            analysis,
            spec,
            cache,
            views,
            &frames,
            &all,
            &established,
            &sides,
            &mut placed,
            &page,
            obstacles,
        ));
    }
    // Settle the established interior leaders before new witnesses claim their
    // corridors. Otherwise a new datum leg can evict a short radius leader and
    // force it all the way out to the view envelope.
    let mut interior = if !recess_linear.is_empty() {
        let mut existing = chosen.clone();
        for (i, w) in all.iter().enumerate() {
            if !matches!(w.shape, Shape::Callout { .. })
                || w.measurements
                    .iter()
                    .any(|m| m.feature.starts_with("f:section_recess:"))
            {
                existing[i] = None;
            }
        }
        interior::place(
            analysis,
            spec,
            views,
            &frames,
            &all,
            &existing,
            &sides,
            &mut placed,
            &page,
            obstacles,
            cache,
        )
    } else {
        BTreeSet::new()
    };
    let restore = |placed: &mut [Option<Placed>],
                   sides: &mut [Option<placement::Side>],
                   lane: &mut [usize]| {
        if recess_linear.is_empty() {
            return;
        }
        for (i, p) in &recess_linear {
            placed[*i] = p.clone();
        }
        placement::clear_linear(
            analysis,
            cache,
            views,
            &frames,
            &all,
            &chosen,
            sides,
            lane,
            placed,
            obstacles,
            &locked,
            &page,
            stage != 3,
        );
        placement::share_lanes(&shared_owner, sides, lane);
    };
    if matches!(stage, 1 | 3) {
        restore(&mut placed, &mut sides, &mut lane);
    }
    // A bounded joint trial lets movable linear ink yield to a clear new
    // leader. Restore every dimension before repairing its lanes and witnesses;
    // only the complete, independently adjudicated result can win.
    if stage == 2 {
        for (i, w) in all.iter().enumerate() {
            if matches!(w.shape, Shape::Callout { .. })
                && w.measurements
                    .iter()
                    .any(|m| m.feature.starts_with("f:section_recess:"))
                && let Some(d) = held.iter().find(|d| {
                    d.id == w.id
                        && d.terms == w.terms
                        && chosen[i].is_some_and(|v| d.view == format!("v:{v}"))
                })
            {
                placed[i] = d.placed.clone();
            }
        }
    }
    let witnesses: Vec<_> = if stage == 2 {
        placed
            .iter_mut()
            .enumerate()
            .filter_map(|(i, p)| {
                if !matches!(all[i].shape, Shape::Linear { .. }) {
                    return None;
                }
                Some((i, p.take()?))
            })
            .collect()
    } else {
        Vec::new()
    };
    if prior != callout_views {
        leader_problems.extend(leaders::place_with_obstacles(
            analysis,
            spec,
            cache,
            views,
            &frames,
            &all,
            &callout_views,
            &sides,
            &mut placed,
            &page,
            obstacles,
        ));
    }
    for (i, p) in witnesses {
        placed[i] = Some(p);
    }
    if !matches!(stage, 1 | 3) {
        restore(&mut placed, &mut sides, &mut lane);
    }
    placement::place_merged(
        &frames,
        &all,
        &chosen,
        &owner,
        &alone,
        &drawn,
        &sides,
        &lane,
        &mut placed,
        &page,
    );
    // Explicit tolerances can revive local band lengths conveyed by a baseline. Settle
    // their combined ink once more: the outer overall witness must not run through a
    // toleranced local label. Keep the measured quantities and all pins intact.
    if !local_tolerances.is_empty() {
        placement::settle_fixed(&mut placed, &fixed, obstacles, analysis, sheet, views);
    }
    // Merged automatic items now have their final ink. Resolve oblique authored
    // quantities against it; the later leader refinement sees these settled positions.
    placement::clear_aligned(
        &frames,
        &all,
        &chosen,
        &sides,
        &fixed,
        &mut placed,
        obstacles,
        &page,
    );
    for (i, w) in all.iter().enumerate() {
        if !fixed[i]
            && matches!(w.shape, Shape::Angular { .. })
            && spec
                .dimensions
                .get(&w.id)
                .is_some_and(|d| d.between.is_some())
        {
            placed[i] = None;
        }
    }
    angular::place(
        &frames,
        &all,
        &chosen,
        &sides,
        &fixed,
        obstacles,
        &mut placed,
        &page,
        &notch_void,
    );
    interior.extend(interior::place(
        analysis,
        spec,
        views,
        &frames,
        &all,
        &chosen,
        &sides,
        &mut placed,
        &page,
        obstacles,
        cache,
    ));
    let refined = leaders::refine(
        analysis,
        spec,
        cache,
        sheet,
        views,
        &frames,
        &all,
        &chosen,
        &mut placed,
        &page,
        obstacles,
        &interior_proofs,
    );
    // Interior candidates already clear all settled ink. Remove only stale leader findings
    // involving replaced geometry; collisions below independently inspect the finished ink.
    let mut leader_problems: Vec<_> = leader_problems
        .into_iter()
        .filter(|p| {
            !p.subjects.iter().any(|s| {
                interior.contains(s)
                    || (matches!(
                        p.code.as_str(),
                        "annotation_line_over_view" | "annotation_leader_long"
                    ) && refined.contains_key(s))
            })
        })
        .collect();
    leader_problems.extend(refined.into_values().flatten());
    let inventory = all
        .iter()
        .enumerate()
        .filter_map(|(i, w)| {
            if owner[i].is_some()
                && !alone[i]
                && !(drawn[i] && matches!(w.shape, Shape::Callout { .. }))
            {
                return None;
            }
            let view = format!("v:{}", chosen[i]?);
            // Settlement moves exterior lane labels only; an interior label keeps its proof.
            let region = placed[i]
                .as_ref()
                .filter(|p| {
                    views.iter().find(|v| v.id == view).is_some_and(|v| {
                        interior_proofs.get(&v.id).is_some_and(|proof| {
                            proof.annotation_clear(v, p, matches!(w.shape, Shape::Callout { .. }))
                                || (w.measurements.iter().any(|m| m.parameter == "notch.angle")
                                    && notch_void(v.id.trim_start_matches("v:"), p))
                        })
                    })
                })
                .map(|_| CandidateRegion::Interior);
            Some(PlannedDimension {
                reference: false,
                determined_by: Vec::new(),
                region,
                id: w.id.clone(),
                kind: match w.shape {
                    Shape::Linear { .. } | Shape::Aligned { .. } => "linear",
                    Shape::Callout { .. } => "callout",
                    Shape::Angular { .. } => "angular",
                }
                .into(),
                view,
                value: w.value,
                label: w.terms.join(" "),
                terms: w.terms.clone(),
                measurements: w.measurements.clone(),
                default: true,
                from_spec: vec![],
                basis: w.basis.clone(),
                state: State::Shown,
                placed: placed[i].clone(),
            })
        })
        .collect();
    let geometry_terms: BTreeMap<_, _> = all
        .iter()
        .map(|w| (w.id.clone(), w.terms.clone()))
        .collect();
    let mut positions: BTreeMap<String, Option<Placed>> =
        all.into_iter().map(|w| w.id).zip(placed).collect();

    // Undrawn items by the views that could carry them and those of them that look along them.
    type Undrawn = (Vec<&'static str>, Vec<&'static str>);
    let mut missing: BTreeMap<Undrawn, Vec<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for ((mut w, default, chosen, along), mut from_spec) in kept.into_iter().zip(given) {
        if !spec.dimension_schemes.is_empty()
            && let Some(path) = schemes::from_spec(&w)
        {
            from_spec.push(path);
        }
        if spec
            .dimensions
            .get(&w.id)
            .is_some_and(|d| d.between.is_some())
        {
            from_spec.push("between".into());
            if spec.dimensions[&w.id].kind.is_some() {
                from_spec.push("kind".into());
            }
            if spec.dimensions[&w.id].along.is_some() {
                from_spec.push("along".into());
            }
        }
        if from_spec.is_empty()
            && spec
                .dimensions
                .get(&w.id)
                .and_then(|d| d.place.as_ref())
                .is_some_and(|p| p.pinned)
            && let Some(previous) = held.iter().find(|d| d.id == w.id)
            && previous.label == format!("({})", w.terms.join(" "))
        {
            // An unrelated chain edit cannot resize a pinned reference's glyphs.
            // Keep its ink and report that it no longer has a controlling chain.
            w.terms = previous.terms.clone();
            w.basis += "; reference retained by its pin after the controlling chain was edited";
            problems.push(Problem {
                id: format!("p:pinned_reference_chain_changed:{}", w.id),
                code: "pinned_reference_chain_changed".into(),
                severity: "warning".into(),
                message: format!("{} stays reference to preserve its pinned ink, but its controlling chain was edited. Release the pin to make it controlling, or restore the chain.", crate::wording::capital(&wording_of(&w))),
                subjects: vec![w.id.clone()],
                fix: vec![Op::remove(format!("dimensions/{}/place", crate::items::key(&w.id)))],
                ..Default::default()
            });
        }
        let placed = positions
            .remove(&w.id)
            .flatten()
            .map(|p| placement::relabel(&p, &geometry_terms[&w.id], &w))
            .map(|p| {
                if spec.dimension_schemes.is_empty() {
                    p
                } else {
                    schemes::ink(&w, p)
                }
            });
        if let Some(v) = spec.dimensions.get(&w.id).and_then(|e| e.view.as_deref())
            && chosen == Some(v)
        {
            from_spec.push("view".into());
        }
        if spec
            .dimensions
            .get(&w.id)
            .is_some_and(|e| e.place.is_some())
        {
            from_spec.push("place".into());
        }
        let view = chosen.unwrap_or(w.views[0]);
        if placed.is_none() {
            missing
                .entry((w.views.clone(), along))
                .or_default()
                .push(w.id.clone());
        }
        let mut basis = if default {
            w.basis
        } else {
            format!("listed in the spec; {}", w.basis)
        };
        let region = placed
            .as_ref()
            .filter(|p| {
                views
                    .iter()
                    .find(|v| v.id == format!("v:{view}"))
                    .is_some_and(|v| {
                        interior_proofs.get(&v.id).is_some_and(|proof| {
                            proof.annotation_clear(v, p, matches!(w.shape, Shape::Callout { .. }))
                                || (w.measurements.iter().any(|m| m.parameter == "notch.angle")
                                    && notch_void(v.id.trim_start_matches("v:"), p))
                        })
                    })
            })
            .map(|_| CandidateRegion::Interior);
        if region.is_some() {
            basis += interior::BASIS;
        }
        // Drawn in brackets: a reference, with what determines it where consolidation said.
        let reference = matches!(&w.terms[..], [t] if t.starts_with('(') && t.ends_with(')'));
        let determined_by = if reference {
            references.get(&w.id).cloned().unwrap_or_default()
        } else {
            Vec::new()
        };
        if let Some(claims) = authored_ownership.claims.get(&w.id) {
            w.measurements.extend(claims.iter().cloned());
            if !claims.is_empty() {
                basis += "; retains the measured roles of unlisted automatic candidates";
            }
        }
        out.push(PlannedDimension {
            reference,
            determined_by,
            region,
            id: w.id,
            kind: match w.shape {
                Shape::Linear { .. } | Shape::Aligned { .. } => "linear",
                Shape::Callout { .. } => "callout",
                Shape::Angular { .. } => "angular",
            }
            .into(),
            view: format!("v:{view}"),
            value: w.value,
            label: w.terms.join(" "),
            terms: w.terms,
            measurements: w.measurements,
            default: default && from_spec.is_empty(),
            from_spec,
            basis,
            // Unplaced only when no shown view can carry it (reported below).
            state: State::placed_or(placed.is_some(), "view_plan_incomplete"),
            placed,
        });
    }
    for ((candidates, along), ids) in missing {
        let n = ids.len();
        let s = if n == 1 { "" } else { "s" };
        let named = |v: &[&str]| v.join(" or ");
        let (omitted, dropped): (Vec<&str>, Vec<&str>) = candidates
            .iter()
            .copied()
            .filter(|v| !along.contains(v))
            .partition(|v| spec.views.get(*v).is_some_and(|s| s.omit));
        let mut why = Vec::new();
        if !omitted.is_empty() {
            why.push(format!(
                "the {} view{} omitted",
                named(&omitted),
                if omitted.len() == 1 { " is" } else { "s are" }
            ));
        }
        if !dropped.is_empty() {
            why.push(format!(
                "the {} view is left out as the turned part's repeated profile",
                named(&dropped)
            ));
        }
        if !along.is_empty() {
            why.push(format!(
                "the {} view looks along {}",
                named(&along),
                if n == 1 { "it" } else { "them" }
            ));
        }
        let (how, fix) = if let Some(v) = omitted.first() {
            (
                format!("Restore the view (remove views/{v}/omit) or omit them."),
                vec![Op::remove(format!("views/{v}/omit"))],
            )
        } else if let Some(v) = dropped.first() {
            (
                format!("Keep the view (set views/{v} to {{}}) or omit them."),
                Vec::new(),
            )
        } else {
            (
                "Omit them, or show a view across them.".to_string(),
                Vec::new(),
            )
        };
        problems.push(Problem {
            id: format!("p:view_plan_incomplete:v:{}", candidates.join("+v:")),
            code: "view_plan_incomplete".into(),
            severity: "error".into(),
            message: format!(
                "{n} dimension{s} can only be drawn in the {} view, but {}; they stay in the plan undrawn. {how}",
                named(&candidates),
                why.join(" and ")
            ),
            subjects: ids,
            fix,
            ..Default::default()
        });
    }
    // Only what the plan draws: the leaders were placed for every item the part would have.
    let drawn: BTreeSet<&str> = out.iter().map(|d| d.id.as_str()).collect();
    for mut p in leader_problems {
        p.subjects
            .retain(|s| s.starts_with("v:") || drawn.contains(s.as_str()));
        if p.subjects.iter().any(|s| !s.starts_with("v:")) {
            p.id = format!("p:{}:{}", p.code, p.subjects.join("|"));
            problems.push(p);
        }
    }
    pmi_dimensions::finish(&mut out);
    problems.extend(placement::collisions_cached(
        analysis, &out, sheet, views, cache,
    ));
    Ok(((out, omitted, problems), inventory))
}
/// The sizes that one dimension states for every feature it spans ([`share`]), and whether its
/// label then counts them (`n×`).
const SHARED: [(&str, bool); 5] = [
    ("notch_depth.length", true),
    // draftwright's `_coalesce_aligned_linear_pitch_dims`: parallel rows with the same stations
    // show one pitch, its label unchanged, claiming every row's.
    ("pitch.length", false),
    // draftwright's shared slot width (`_slot_shared_width_key`): `2× 6`.
    ("slot_width.length", true),
    // The same rule for the other sizes a span can share (beyond draftwright, which repeats
    // them: ftc06's four 38.1 boss heights on one span).
    ("slot_length.length", true),
    ("boss_height.length", true),
];

// Lane ownership uses same-family sharing. New cross-family statements retain their original
// reservations, so consolidating their ink does not move unrelated items.
fn share(
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
) {
    share_statements(kept, given, omitted, false);
}

/// The correlated set a dimension belongs to (ARCH0015-2): the height ladder's rungs (its counted
/// rise among them), or the ladder's shoulders along one axis. None for any other dimension: a
/// turned profile's chain is not one, since omitting one link of a chain is ordinary drafting
/// and its baseline already restores a band's length when a rung is omitted (#33).
fn correlated_set(id: &str) -> Option<&str> {
    const HEIGHTS: &str = "d:step_level/step_height.length";
    if id.starts_with(HEIGHTS) {
        return Some(HEIGHTS);
    }
    let rest = id.strip_prefix("d:step_level/step_position.length.")?;
    let axis = rest.get(..1).filter(|a| ["x", "y", "z"].contains(a))?;
    Some(&id[..id.len() - rest.len() + axis.len()])
}

/// Stream K6 (ARCH0015-2): a correlated set (the step-height ladder, the shoulders along an
/// axis) is approved or omitted whole, never half a staircase. Each member keeps its own ID
/// (ADR4-6), so the spec's decision on any member applies to the set: with the automatic source,
/// omitting one member omits the others too; with an authored set, listing one member approves
/// the others. An omission never overrides a member the spec decides itself (any entry on it):
/// that set is left as the spec states it, with a `correlated_set_conflict` warning. Returns, by
/// ID, each member the decision spreads to with its basis, and an `info` problem per set saying so.
/// With an authored set, a member in *conveyed* (one the defaults hand to an item outside its
/// set) is not approved by the set: it stays with its owner, conveyed or not authored as main's
/// authored-set parity decides.
fn correlated_sets(
    spec: &Spec,
    wants: &[Want],
    conveyed: &BTreeSet<&str>,
) -> (BTreeMap<String, String>, Vec<Problem>) {
    let mut sets: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let auto = spec.dimension_source == DimensionSource::Auto;
    for w in wants {
        if !auto && conveyed.contains(w.id.as_str()) {
            continue;
        }
        if let Some(set) = correlated_set(&w.id) {
            sets.entry(set).or_default().push(&w.id);
        }
    }
    let mut spread = BTreeMap::new();
    let mut problems = Vec::new();
    for (set, members) in sets {
        let decided: Vec<&str> = members
            .iter()
            .copied()
            .filter(|id| spec.dimensions.get(*id).is_some_and(|e| !auto || e.omit))
            .collect();
        let rest: Vec<&str> = members
            .iter()
            .copied()
            .filter(|id| !decided.contains(id))
            .collect();
        if decided.is_empty() || rest.is_empty() {
            continue;
        }
        let what = if set.contains("step_height") {
            "the height ladder".to_string()
        } else {
            format!("the ladder's shoulders along {}", &set[set.len() - 1..])
        };
        let named = decided.join(", ");
        let restore: Vec<Op> = decided
            .iter()
            .map(|id| Op::remove(format!("dimensions/{}/omit", crate::items::key(id))))
            .collect();
        // A member the spec decides itself (a tolerance, a view, a place, `omit: false`) keeps
        // that decision: the omission is not spread over it, and the conflict is reported, its
        // fix restoring the set (the author's requirement on the member must reach the sheet).
        let own: Vec<&str> = if auto {
            rest.iter()
                .copied()
                .filter(|id| spec.dimensions.contains_key(*id))
                .collect()
        } else {
            Vec::new()
        };
        if !own.is_empty() {
            let kept = own.join(", ");
            problems.push(Problem {
                id: format!("p:correlated_set_conflict:{set}"),
                code: "correlated_set_conflict".into(),
                severity: "warning".into(),
                message: format!(
                    "The spec omits {named} of {what} but states {kept} itself, so half the set is drawn: a correlated set is approved or omitted whole. Remove the omission to draw the set, or omit every member."
                ),
                subjects: decided.iter().chain(&own).map(|s| s.to_string()).collect(),
                fix: restore,
                ..Default::default()
            });
            continue;
        }
        let (basis, message, fix) = if auto {
            (
                format!(
                    "authored: omitted with {what}, which the spec omits ({named}): a correlated set is omitted whole"
                ),
                format!(
                    "Omitting {named} omits all of {what}: a correlated set is approved or omitted whole, never half a staircase. Remove the omission to restore the set."
                ),
                restore,
            )
        } else {
            (
                String::new(),
                format!(
                    "Listing {named} approves all of {what}: a correlated set is approved or omitted whole, never half a staircase."
                ),
                Vec::new(),
            )
        };
        for id in &rest {
            spread.insert(id.to_string(), basis.clone());
        }
        problems.push(Problem {
            id: format!("p:correlated_set_whole:{set}"),
            code: "correlated_set_whole".into(),
            severity: "info".into(),
            message,
            subjects: rest.iter().map(|s| s.to_string()).collect(),
            fix,
            ..Default::default()
        });
    }
    (spread, problems)
}

/// One span, one dimension (ARCH0009-6): kept items stating the same [`SHARED`] size or
/// datum distance across the same span (the same view and axis, raw ends within 1e-6 mm,
/// and the same label and basic status, so different stated sizes or bands never collapse).
/// Datum distances include different planners' statements, such as a location and a level.
/// A boss height can also state its face level; only the bosses enter the count.
/// A turned boss's linear diameter on the overall diameter also shares that statement.
/// These are drawn once. The survivor is one the spec lists over a default, then the smaller ID; it
/// claims every member's measurement and, for a counted size, prints their number (`2× 6`).
/// Each other moves to *omitted*, `conveyed by` the survivor. *given* stays aligned with *kept*.
fn share_statements(
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
    cross_family: bool,
) {
    const SAME_SPAN: f64 = 1e-6;
    type Span = (String, &'static str, usize, f64, f64, String, f64, bool);
    let coordinate = |p: &str| {
        p.starts_with("location")
            || p.starts_with("seat_location.")
            || p.starts_with("step_height.length.")
            || p.starts_with("step_position.length.")
    };
    let diameter = |p: &str| matches!(p, "od.diameter" | "boss.diameter");
    let level = |p: &str| p.starts_with("step_height.length.");
    let span = |(w, _, view, _): &(Want, bool, Option<&'static str>, Vec<&'static str>)| {
        let parameter = &w.measurements.first()?.parameter;
        // The recess consolidator owns these spans, including claims transferred to a
        // level or envelope carrier. Keep them out of point-location sharing so a hole's
        // table cells and a slot's independent datum identity remain intact.
        if w.measurements.iter().any(|m| {
            m.parameter.starts_with("seat_location.")
                || m.parameter.starts_with("location_pocket.")
                || m.parameter.starts_with("location_pocket_pattern.")
        }) {
            return None;
        }
        if !(SHARED.iter().any(|(p, _)| p == parameter)
            || cross_family && (coordinate(parameter) || diameter(parameter)))
        {
            return None;
        }
        match w.shape {
            Shape::Linear { a, b, axis, .. } => Some((
                parameter.clone(),
                view.unwrap_or(w.views[0]),
                axis,
                a[axis].min(b[axis]),
                a[axis].max(b[axis]),
                w.terms.join(" "),
                w.value,
                pmi_dimensions::basic(&w.measurements),
            )),
            Shape::Callout { .. } | Shape::Angular { .. } | Shape::Aligned { .. } => None,
        }
    };
    let same = |s: &Span, t: &Span| {
        (s.0 == t.0 && !diameter(&s.0)
            || cross_family
                && (coordinate(&s.0) && coordinate(&t.0)
                    || s.0 == "boss_height.length" && level(&t.0)
                    || t.0 == "boss_height.length" && level(&s.0)
                    || (s.0 == "od.diameter" || t.0 == "od.diameter")
                        && diameter(&s.0)
                        && diameter(&t.0)))
            && s.1 == t.1
            && s.2 == t.2
            && (s.3 - t.3).abs() <= SAME_SPAN
            && (s.4 - t.4).abs() <= SAME_SPAN
            && s.5 == t.5
            && (s.6 - t.6).abs() <= SAME_SPAN
            && s.7 == t.7
    };
    let mut groups: Vec<(Span, Vec<usize>)> = Vec::new();
    for (i, k) in kept.iter().enumerate() {
        let Some(s) = span(k) else { continue };
        match groups.iter_mut().find(|(t, _)| same(&s, t)) {
            Some((_, members)) => members.push(i),
            None => groups.push((s, vec![i])),
        }
    }
    let mut gone = BTreeSet::new();
    for (s, mut members) in groups.into_iter().filter(|(_, m)| m.len() > 1) {
        members.sort_by(|&x, &y| {
            kept[x]
                .1
                .cmp(&kept[y].1)
                .then_with(|| kept[x].0.id.cmp(&kept[y].0.id))
        });
        let (winner, losers) = members.split_first().expect("a group of two or more");
        let owner = kept[*winner].0.id.clone();
        let mut claims = Vec::new();
        for &i in losers {
            let w = &kept[i].0;
            omitted.push(Omitted {
                id: w.id.clone(),
                label: w.terms.join(" "),
                basis: format!(
                    "conveyed by {owner}: it states the same {} ({}) across the same span in {}, in the same view (one span, one dimension: ARCH0009-6)",
                    s.0, s.5, AXES[s.2]
                ),
                state: State::Covered { by: owner.clone() },
            });
            claims.extend(w.measurements.iter().cloned());
            gone.insert(i);
        }
        let counted_parameter = members.iter().find_map(|&i| {
            let p = &kept[i].0.measurements[0].parameter;
            SHARED
                .iter()
                .any(|(shared, count)| shared == p && *count)
                .then(|| p.clone())
        });
        let w = &mut kept[*winner].0;
        w.measurements.extend(claims);
        let features: BTreeSet<&str> = w
            .measurements
            .iter()
            .filter(|m| counted_parameter.as_ref().is_none_or(|p| &m.parameter == p))
            .map(|m| m.feature.as_str())
            .collect();
        let n = features.len();
        let counted = counted_parameter.is_some() && n > 1;
        w.basis += &format!(
            "; one span, one dimension: it states the same size of all {n} features it spans (ARCH0009-6){}",
            if counted { ", counted" } else { "" }
        );
        if counted {
            w.terms.insert(0, format!("{n}×"));
        }
    }
    let mut i = 0;
    kept.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
    let mut i = 0;
    given.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
}

/// One span, one dimension, for the part's PMI linear dimensions: a kept PMI linear dimension
/// across the same span as another (the same view and axis, raw ends within 1e-6 mm) conveys it.
/// - Two PMI dimensions with the same label (basic or not, band included) are drawn once: a
///   model that states each hole's basic location states the same span once per hole in a row or
///   column. The survivor is the smaller ID and claims every member's measurement.
/// - A default dimension with nothing from the spec (no tolerance or fit) and the same value
///   gives way to the PMI one, which is source-authored and carries the requirement; the PMI
///   dimension claims its measurement.
///
/// Each one that gives way moves to *omitted*, `conveyed by` the survivor. *given* stays aligned
/// with *kept*.
fn share_pmi(
    analysis: &Analysis,
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
) {
    notches::share_pmi(analysis, kept, given, omitted);
    const SAME_SPAN: f64 = 1e-6;
    let pmi = |w: &Want| {
        w.measurements
            .first()
            .is_some_and(|m| m.feature.starts_with("pmi:") && m.parameter.starts_with("linear"))
    };
    let span =
        |(w, _, view, _): &(Want, bool, Option<&'static str>, Vec<&'static str>)| match w.shape {
            Shape::Linear { a, b, axis, .. } => Some((
                view.unwrap_or(w.views[0]),
                axis,
                a[axis].min(b[axis]),
                a[axis].max(b[axis]),
            )),
            Shape::Callout { .. } | Shape::Angular { .. } | Shape::Aligned { .. } => None,
        };
    let mut gone = BTreeSet::new();
    let order: Vec<usize> = {
        let mut o: Vec<usize> = (0..kept.len()).filter(|&i| pmi(&kept[i].0)).collect();
        o.sort_by(|&x, &y| kept[x].0.id.cmp(&kept[y].0.id));
        o
    };
    for &p in &order {
        if gone.contains(&p) {
            continue;
        }
        let Some(sp) = span(&kept[p]) else { continue };
        let owner = kept[p].0.id.clone();
        let mut claims = Vec::new();
        for i in 0..kept.len() {
            if i == p || gone.contains(&i) {
                continue;
            }
            let Some(si) = span(&kept[i]) else { continue };
            let same = sp.0 == si.0
                && sp.1 == si.1
                && (sp.2 - si.2).abs() <= SAME_SPAN
                && (sp.3 - si.3).abs() <= SAME_SPAN;
            if !same {
                continue;
            }
            let (w, default, ..) = &kept[i];
            let how = if pmi(w) {
                if w.terms != kept[p].0.terms
                    || pmi_dimensions::basic(&w.measurements)
                        != pmi_dimensions::basic(&kept[p].0.measurements)
                {
                    continue;
                }
                "the part's PMI states the same span again"
            } else if *default
                && given[i].is_empty()
                && (w.value - kept[p].0.value).abs() <= SAME_SPAN
            {
                "a default dimension gives way to the part's PMI dimension of the same span"
            } else {
                continue;
            };
            omitted.push(Omitted {
                id: w.id.clone(),
                label: w.terms.join(" "),
                basis: format!(
                    "conveyed by {owner}: it states the same {} across the same span in {}, in the same view; {how} (one span, one dimension: ARCH0009-6)",
                    fmt(w.value),
                    AXES[sp.1]
                ),
                state: State::Covered { by: owner.clone() },
            });
            claims.extend(w.measurements.iter().cloned());
            gone.insert(i);
        }
        if !claims.is_empty() {
            let w = &mut kept[p].0;
            w.measurements.extend(claims);
            w.basis += "; one span, one dimension: it also states what the others across the same span stated (ARCH0009-6)";
        }
    }
    let mut i = 0;
    kept.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
    let mut i = 0;
    given.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
    notches::share_bevel_depths(analysis, kept, given, omitted);
}

/// Draws the planned dimensions: lines, arrowheads and labels into *items* (the dims layer), and
/// each label's text runs into *text*. Everything comes from the plan; nothing is measured or
/// formatted here (R1 invariant 2).
///
/// Labels are drawn with the same bundled font placement measured them with ([`FONT`]), so the
/// label boxes the collision problems describe are the ink drawn. `⌀` is drawn as the font's `ø`
/// and `↧`, `⌴` and `⌵` as strokes; the label's one text run is its terms as the plan states them,
/// symbols included, centred on its ink along the baseline, so the text searched and copied from
/// the PDF and SVG is the callout (`4× ⌀6 ↧ 5`) and lint reads the same.
///
/// Returns one annotation per drawn item, for lint: its ID, view and text runs, and the
/// measurements it states as its claims (provenance carried on the drawing, so lint reads the
/// drawing and never the plan).
pub fn render(
    dims: &[PlannedDimension],
    items: &mut Vec<Item>,
    text: &mut Vec<TextRun>,
) -> Vec<Annotation> {
    let font = font();
    let mut annotations = Vec::new();
    for d in dims {
        let Some(p) = &d.placed else { continue };
        let first = text.len();
        for l in &p.lines {
            items.push(Item::Edge(Curve::Line(l[0], l[1])));
        }
        for [tip, tail] in &p.arrows {
            let n = [tail[1] - tip[1], tip[0] - tail[0]];
            let len = n[0].hypot(n[1]).max(1e-12);
            let k = ARROW_HALF / len;
            let b1 = [tail[0] + k * n[0], tail[1] + k * n[1]];
            let b2 = [tail[0] - k * n[0], tail[1] - k * n[1]];
            items.push(Item::Face(vec![vec![
                Curve::Line(*tip, b1),
                Curve::Line(b1, b2),
                Curve::Line(b2, *tip),
            ]]));
        }
        let ink = label_ink(&d.terms, font);
        let f = to_page(p.label_at, p.rotation);
        for s in &ink.strokes {
            items.push(Item::Edge(Curve::Line(f(s[0]), f(s[1]))));
        }
        for contours in &ink.glyphs {
            let wires: Vec<Vec<Curve>> = contours
                .iter()
                .filter(|c| !c.is_empty())
                .map(|c| {
                    c.iter()
                        .map(|s| match moved(s, &f) {
                            Segment::Line(a, b) => Curve::Line(a, b),
                            Segment::Quad(a, b, c) => Curve::Beziers(vec![vec![a, b, c]]),
                            Segment::Cubic(a, b, c, e) => Curve::Beziers(vec![vec![a, b, c, e]]),
                        })
                        .collect()
                })
                .collect();
            if !wires.is_empty() {
                items.push(Item::Face(wires));
            }
        }
        // A basic dimension's box: its label box, which the plan widened to take it in.
        if crate::label_text::basic(d) {
            let [x0, y0, x1, y1] = p.label_box;
            for (a, b) in [
                ([x0, y0], [x1, y0]),
                ([x1, y0], [x1, y1]),
                ([x1, y1], [x0, y1]),
                ([x0, y1], [x0, y0]),
            ] {
                items.push(Item::Edge(Curve::Line(a, b)));
            }
        }
        text.push(TextRun {
            text: d.terms.join(" "),
            at: f([0.5 * (ink.min[0] + ink.max[0]), 0.0]),
            size: TEXT,
            rotation: p.rotation,
            font: FONT.into(),
            h_align: HAlign::Center,
            v_align: VAlign::Baseline,
        });
        annotations.push(Annotation {
            approved_claims: Vec::new(),
            approved_text: None,
            coordinate_origin: None,
            satisfies: Vec::new(),
            name: d.id.clone(),
            view: Some(d.view.clone()),
            runs: (first..text.len()).collect(),
            claims: d
                .measurements
                .iter()
                .map(|m| Claim {
                    feature: m.feature.clone(),
                    parameter: m.parameter.clone(),
                })
                .collect(),
        });
    }
    annotations
}

/// Where a feature or pattern sits in the part's frame: a hole's point on its axis, a fillet's or
/// chamfer's `at`, a pattern's centre (its members' mean), for comparing with draftwright's
/// feature keys.
pub fn feature_origin(analysis: &Analysis, id: &str) -> Option<P3> {
    let frame = QFrame(analysis.recognition.as_ref()?.frame);
    if let Some(f) = analysis.features.iter().find(|f| f.id == id) {
        let at = f.parameters.get("location").or(f.parameters.get("at"));
        return point(at).map(|l| frame.point(l));
    }
    let p = analysis
        .recognition
        .as_ref()?
        .patterns
        .iter()
        .find(|p| p.id == id)?;
    if let Some(c) = point(p.parameters.get("center")) {
        return Some(frame.point(c));
    }
    let members: Vec<P3> = p
        .members
        .iter()
        .filter_map(|m| feature_origin(analysis, m))
        .collect();
    (!members.is_empty()).then(|| {
        let n = members.len() as f64;
        members
            .iter()
            .fold([0.0; 3], |s, m| add(s, scale3(*m, 1.0 / n)))
    })
}

/// How far callout *d*'s leader shaft cuts back into its view's material after leaving it at
/// its tip, page mm: [`crate::material::reentry`] on the router's own material for the view
/// (ADR2-8), zero for none or for a view the router does not place in.
pub(crate) fn leader_reentry(
    analysis: &Analysis,
    views: &[PlannedView],
    d: &PlannedDimension,
) -> f64 {
    let (Some(v), Some(shaft)) = (
        views.iter().find(|v| {
            let name = v.id.strip_prefix("v:").unwrap_or(&v.id);
            v.id == d.view && (NAMES.contains(&name) || crate::detail::NAMES.contains(&name))
        }),
        d.placed.as_ref().and_then(|p| p.lines.first()),
    ) else {
        return 0.0;
    };
    crate::material::reentry(
        footprints::Cache::default().get(analysis, v).solids(),
        *shaft,
    )
}

/// Geometry-backed placement choices for the editor, including omitted dimensions.
pub fn placement_options(analysis: &Analysis, views: &[PlannedView]) -> BTreeMap<String, Value> {
    placement_options_with_spec(analysis, views, &Spec::default())
}

pub(crate) fn placement_options_with_spec(
    analysis: &Analysis,
    views: &[PlannedView],
    spec: &Spec,
) -> BTreeMap<String, Value> {
    let (mut wants, ..) = wants(analysis);
    if schemes::apply(analysis, spec, &mut wants).is_err()
        || authored::append(analysis, spec, &mut wants).is_err()
    {
        return BTreeMap::new();
    }
    detail_routes(analysis, &mut wants, views);
    let original = wants.clone();
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            (NAMES.contains(&name) || crate::detail::NAMES.contains(&name))
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    route(&mut wants, &frames, analysis);
    wants
        .iter()
        .zip(original)
        .map(|(w, original)| {
            let options = original
                .views
                .iter()
                .filter_map(|&name| {
                    let frame = frames.get(name)?;
                    let mut probe = original.clone();
                    probe.views = vec![name];
                    let (chosen, _) = route(std::slice::from_mut(&mut probe), &frames, analysis);
                    if chosen[0] != Some(name) {
                        return None;
                    }
                    let sides = match probe.shape {
                        Shape::Linear { axis, .. } => {
                            if across(frame, axis)? {
                                vec!["above", "below"]
                            } else {
                                vec!["left", "right"]
                            }
                        }
                        Shape::Callout { .. } | Shape::Aligned { .. } => {
                            vec!["above", "below", "left", "right"]
                        }
                        Shape::Angular { .. }
                            if probe.id.starts_with("d:authored:")
                                || probe
                                    .measurements
                                    .iter()
                                    .any(|m| m.parameter == "notch.angle") =>
                        {
                            Vec::new()
                        }
                        Shape::Angular { .. } => return None,
                    };
                    Some(serde_json::json!({"view": name, "sides": sides}))
                })
                .collect::<Vec<_>>();
            (
                w.id.clone(),
                serde_json::json!({"label": w.terms.join(" "), "views": options}),
            )
        })
        .collect()
}

/// A readable symbolic class with no numeric limits stated: a hole class on proven internal
/// cylinders, or a shaft class on proven external ones (#30).
pub(crate) fn symbolic_bore(r: &crate::pmi::Record) -> bool {
    pmi_dimensions::symbolic_bore(r)
}

/// Recheck free leaders after the linear labels and GD&T have settled together.
/// The full inventory keeps omission edits local; frames remain fixed obstacles.
pub(crate) fn refine_inventory(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    inventory: &mut [PlannedDimension],
    obstacles: &[PlannedDimension],
    cache: &footprints::Cache,
) -> BTreeMap<String, Vec<Problem>> {
    let recesses = inventory.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    // Restoring an unaffected peer after label edits can expose a callout
    // overlap which was absent in the initial joint solve. Recheck that final
    // ink even on drawings without new recess dimensions.
    let overlapping_text = !recesses
        && leaders::edited_labels(spec)
        && inventory.iter().enumerate().any(|(i, d)| {
            d.kind == "callout"
                && d.placed.as_ref().is_some_and(|p| {
                    inventory.iter().enumerate().any(|(j, q)| {
                        i != j
                            && q.placed
                                .as_ref()
                                .is_some_and(|q| placement::boxes_overlap(p.label_box, q.label_box))
                    })
                })
        });
    if !recesses && !overlapping_text {
        return BTreeMap::new();
    }
    let (raw, ..) = wants(analysis);
    let frames: BTreeMap<_, _> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            NAMES
                .contains(&name)
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    let mut items = Vec::new();
    let mut indices = Vec::new();
    let mut chosen = Vec::new();
    let mut placed = Vec::new();
    for (i, d) in inventory.iter().enumerate() {
        let Some(w) = raw.iter().find(|w| w.id == d.id) else {
            continue;
        };
        let mut w = w.clone();
        w.terms = d.terms.clone();
        w.measurements = d.measurements.clone();
        if w.measurements
            .first()
            .is_some_and(|m| m.parameter == "seat_diameter.diameter")
            && let Shape::Callout { sites, .. } = &mut w.shape
        {
            for peer in raw.iter().filter(|peer| {
                peer.id != w.id && peer.measurements.iter().any(|m| w.measurements.contains(m))
            }) {
                if let Shape::Callout { sites: more, .. } = &peer.shape {
                    sites.extend(more.iter().copied());
                }
            }
        }
        let name = d.view.trim_start_matches("v:");
        bosses::route_profile(analysis, &mut w, name);
        let view = NAMES
            .iter()
            .copied()
            .find(|v| *v == name && frames.contains_key(v));
        items.push(w);
        indices.push(i);
        chosen.push(view);
        placed.push(d.placed.clone());
    }
    let page = placement::Page::new(analysis, sheet, views);
    let mut sides = Vec::new();
    let mut lanes = Vec::new();
    for ((i, view), p) in indices.iter().zip(&chosen).zip(&placed) {
        let side = view.and_then(|v| {
            let p = p.as_ref()?;
            if inventory[*i].region == Some(CandidateRegion::Interior)
                || p.arrows.len() != 2
                || spec
                    .dimensions
                    .get(&inventory[*i].id)
                    .is_some_and(|d| d.place.is_some())
            {
                return None;
            }
            let o = frames[v].outline();
            Some(if p.rotation == 0.0 {
                if p.arrows[0][0][1] < o[1] {
                    placement::Side::Below
                } else {
                    placement::Side::Above
                }
            } else if p.arrows[0][0][0] < o[0] {
                placement::Side::Left
            } else {
                placement::Side::Right
            })
        });
        sides.push(side);
        lanes.push(0);
    }
    // New linear items get a final chance around the settled leaders and GD&T.
    // Every candidate clears that fixed ink and cannot add a feature crossing.
    if obstacles.iter().any(|d| d.kind == "callout") {
        for (i, w) in items.iter().enumerate() {
            if w.measurements.is_empty()
                || !w
                    .measurements
                    .iter()
                    .all(|m| m.feature.starts_with("f:section_recess:"))
            {
                sides[i] = None;
            }
        }
    }
    if recesses {
        placement::clear_linear(
            analysis,
            cache,
            views,
            &frames,
            &items,
            &chosen,
            &mut sides,
            &mut lanes,
            &mut placed,
            obstacles,
            &[],
            &page,
            true,
        );
    }
    let proofs = interior::proofs(analysis, views);
    let reports = leaders::refine(
        analysis,
        spec,
        cache,
        sheet,
        views,
        &frames,
        &items,
        &chosen,
        &mut placed,
        &page,
        obstacles,
        &proofs,
    );
    for (i, p) in indices.into_iter().zip(placed) {
        inventory[i].placed = p;
    }
    reports
}

/// Rejudge settled paper geometry after moving complete view blocks to another sheet.
pub(crate) fn recheck(analysis: &Analysis, plan: &crate::model::Plan) -> Vec<Problem> {
    recheck_cached(analysis, plan, &footprints::Cache::default())
}

pub(crate) fn recheck_cached(
    analysis: &Analysis,
    plan: &crate::model::Plan,
    cache: &footprints::Cache,
) -> Vec<Problem> {
    let mut all = plan.dimensions.clone();
    all.extend(plan.gdt.iter().filter_map(gdt::proxy));
    std::iter::once(&plan.sheet)
        .chain(&plan.sheets)
        .flat_map(|sheet| placement::collisions_cached(analysis, &all, sheet, &plan.views, cache))
        .collect()
}

/// Semantic roles available to a feature note, including compound callout terms.
pub(crate) fn measurement_ids(analysis: &Analysis) -> BTreeSet<String> {
    wants(analysis)
        .0
        .iter()
        .flat_map(|w| &w.measurements)
        .map(|m| want_id(&m.feature, &m.parameter))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_head_routes_leave_parent_witnesses_untouched() {
        let (_, mut a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/turned.step"),
        )
        .unwrap();
        let p = crate::model::plan(&a, &Spec::default()).unwrap();
        let mut v = p.views.iter().find(|v| v.id == "v:front").unwrap().clone();
        a.recognition = None;
        let f = &mut a.features[0];
        f.kind = "turned_step".into();
        f.parameters = serde_json::json!({"axis":"x","lo":0.,"hi":10.,"diameter":8.,"profile":{"axis_origin":[0.,0.,0.]}}).as_object().unwrap().clone();
        let feature = f.id.clone();
        v.id = "v:detail_a".into();
        v.detail = Some(serde_json::from_value(serde_json::json!({
            "label":"A","axis":0,"source_view":"v:front","kind":"turned-head",
            "bounds":[[0.,-5.,-5.],[5.,5.,5.]],"marked_bounds":[[0.,-5.,-5.],[5.,5.,5.]],
            "dimensions":[want_id(&feature,"step.length")],"scale":2.,"authored":false,"partial_profile":false
        })).unwrap());
        let mut w = size(&feature, "step.length", 0., 10., &["10"]).0;
        if let Shape::Linear { a, b, .. } = &mut w.shape {
            a[1] = 0.;
            b[1] = 0.;
        }
        let original = w.clone();
        detail_routes(&a, std::slice::from_mut(&mut w), std::slice::from_ref(&v));
        let witnesses = |w: &Want| match w.shape {
            Shape::Linear { a, b, .. } => [a, b],
            _ => unreachable!(),
        };
        assert_eq!(w.views, original.views);
        assert_eq!(w.basis, original.basis);
        assert_eq!(
            witnesses(&w),
            witnesses(&original),
            "a rejected crop must not change the parent's shoulders"
        );
        // A full shoulder crop still recovers the length at the positive silhouette.
        v.detail.as_mut().unwrap().bounds = [[0., -5., 3.], [10., 5., 5.]];
        detail_routes(&a, std::slice::from_mut(&mut w), &[v]);
        assert_eq!(w.views[0], "detail_a");
        assert_eq!(witnesses(&w), [[0., 0., 4.], [10., 0., 4.]]);
    }

    #[test]
    fn side_only_callout_corridors_grow_with_authored_precision() {
        let (_, analysis) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/plate.step"),
        )
        .unwrap();
        let id = "d:f:hole_pattern:1/bore.diameter";
        for side in ["left", "right"] {
            let mut spec: Spec = serde_json::from_value(serde_json::json!({
                "dimensions": {id: {"place": {"view": "plan", "side": side}}}
            }))
            .unwrap();
            let bare = callout_bands(&analysis, &spec).unwrap();
            assert_eq!(bare.len(), 1);
            spec.precision.insert(id.into(), 15);
            let precise = callout_bands(&analysis, &spec).unwrap();
            assert_eq!(precise.len(), 1);
            assert_eq!(precise[0].side, side);
            assert!(precise[0].depth > bare[0].depth + 20.0);
        }
    }

    type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

    /// A default size of *feature* in the plan, along x from *lo* to *hi*, labelled *label*.
    fn size(feature: &str, parameter: &str, lo: f64, hi: f64, label: &[&str]) -> Kept {
        let want = Want {
            id: want_id(feature, parameter),
            views: vec!["plan"],
            value: hi - lo,
            terms: label.iter().map(|t| t.to_string()).collect(),
            measurements: vec![Measurement {
                feature: feature.into(),
                parameter: parameter.into(),
                value: hi - lo,
            }],
            basis: String::new(),
            shape: Shape::Linear {
                a: [lo, 5.0, 0.0],
                b: [hi, 5.0, 0.0],
                axis: 0,
                a_edge: false,
                b_edge: false,
                order: 0,
            },
            turning: None,
        };
        (want, true, Some("plan"), vec![])
    }

    /// The kept items as (ID, label, claims), and the omitted IDs with their owners.
    type Shared = (Vec<(String, String, usize)>, Vec<(String, String)>);

    fn shared(mut kept: Vec<Kept>) -> Shared {
        let mut given = vec![Vec::new(); kept.len()];
        let mut omitted = Vec::new();
        share(&mut kept, &mut given, &mut omitted);
        assert_eq!(given.len(), kept.len());
        (
            kept.iter()
                .map(|(w, ..)| (w.id.clone(), w.terms.join(" "), w.measurements.len()))
                .collect(),
            omitted
                .iter()
                .map(|o| {
                    let owner = o.basis["conveyed by ".len()..o.basis.find(": ").unwrap()].into();
                    (o.id.clone(), owner)
                })
                .collect(),
        )
    }

    #[test]
    fn one_span_one_size_counted_and_claiming_every_feature() {
        let (kept, omitted) = shared(vec![
            size("f:slot:2", "slot_width.length", 10.0, 16.0, &["6"]),
            size("f:slot:1", "slot_width.length", 10.0, 16.0, &["6"]),
            size("f:slot:3", "slot_width.length", 20.0, 26.0, &["6"]),
        ]);
        assert_eq!(
            kept,
            [
                ("d:f:slot:1/slot_width.length".into(), "2× 6".into(), 2),
                ("d:f:slot:3/slot_width.length".into(), "6".into(), 1),
            ]
        );
        assert_eq!(
            omitted,
            [(
                "d:f:slot:2/slot_width.length".into(),
                "d:f:slot:1/slot_width.length".into()
            )]
        );
    }

    #[test]
    fn a_pitch_keeps_its_label_and_other_bands_or_parameters_never_merge() {
        let (kept, _) = shared(vec![
            size("f:hole_pattern:1", "pitch.length", 0.0, 75.0, &["2× 75"]),
            size("f:hole_pattern:2", "pitch.length", 0.0, 75.0, &["2× 75"]),
        ]);
        assert_eq!(
            kept,
            [("d:f:hole_pattern:1/pitch.length".into(), "2× 75".into(), 2)]
        );
        // A different band, a different parameter, or a size draftwright never shares.
        let (kept, omitted) = shared(vec![
            size("f:boss:1", "boss_height.length", 0.0, 5.0, &["5"]),
            size("f:boss:2", "boss_height.length", 0.0, 5.0, &["5", "±0.1"]),
            size("f:slot:1", "slot_length.length", 0.0, 5.0, &["5"]),
            size("f:step:1", "step.length", 0.0, 5.0, &["5"]),
            size("f:step:2", "step.length", 0.0, 5.0, &["5"]),
        ]);
        assert_eq!(kept.len(), 5, "{kept:?}");
        assert!(omitted.is_empty());
    }

    #[test]
    fn the_spec_listed_one_survives() {
        let mut listed = size("f:boss:2", "boss_height.length", 0.0, 5.0, &["5"]);
        listed.1 = false;
        let (kept, omitted) = shared(vec![
            size("f:boss:1", "boss_height.length", 0.0, 5.0, &["5"]),
            listed,
        ]);
        assert_eq!(
            kept,
            [("d:f:boss:2/boss_height.length".into(), "2× 5".into(), 2)]
        );
        assert_eq!(omitted[0].0, "d:f:boss:1/boss_height.length");
    }
}
