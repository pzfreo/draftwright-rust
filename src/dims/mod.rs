//! Dimensions and hole callouts: what the sheet states about sizes, planned from the Analysis's
//! features and drawn only from the plan (ADR R1 invariant 2).
//!
//! This first cut covers:
//! - **the overall extents**: width, depth and height of a prismatic part; the diameter and the
//!   axial length of a part that is turned whole;
//! - **whole-part polygonal stock**: its size across flats and its length, in place of the extents;
//! - **hole callouts**: draftwright's compound grammar (`4× ⌀10 THRU ⌴ ⌀16 ↧ 5 (2×2)`), with
//!   identical holes grouped and counted, and quiddity's hole patterns called out as patterns;
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
//!   front view, and its shoulders' positions along x and y.
//!
//! Each is a [`PlannedDimension`] with a stable ID, `d:<feature>/<parameter>@<view>`, its value,
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

mod bosses;
mod edges;
mod envelope;
mod flats;
pub mod gdt;
mod grooves;
mod holes;
mod leaders;
pub(crate) mod levels;
mod placement;
mod pmi_dimensions;
mod recesses;
mod slots;
mod steps;
mod stock;
mod tolerance;
mod turned;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{Analysis, DimensionSource, Invalid, PlannedSheet, PlannedView, Problem, Spec};
use crate::output::{Annotation, Claim, Curve, HAlign, Item, TextRun, VAlign};
use crate::text::{Font, Segment};
pub use leaders::clear_routes;
use placement::{Frame, across, moved, to_page};
pub use placement::{LabelInk, label_ink};
pub(crate) use tolerance::check as check_tolerances;

/// Annotation text height (the em), mm: draftwright's `_FONT_SIZE`.
pub const TEXT: f64 = 3.0;
/// Arrowhead length, mm: draftwright's draft preset at its text height.
pub const ARROW: f64 = 2.7;
/// Half the arrowhead's width: a closed head of 1:3.
const ARROW_HALF: f64 = ARROW / 6.0;
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
const CAP: f64 = 0.7 * TEXT;
const SHOULDER: f64 = 3.0;
/// The font labels are measured and drawn with (ARCH0006-1: one file for both).
pub const FONT: &str = "IBMPlexMono-Regular.ttf";
const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

pub(crate) fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| Font::new(MONO.to_vec()).expect("the bundled font parses"))
}

/// A dimension or callout the sheet will carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedDimension {
    /// `d:<feature>/<parameter>@<view>`; the feature is `envelope` for the overall extents.
    pub id: String,
    /// `linear` or `callout`.
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
    /// Its ink on the page; absent when it cannot be drawn, and a problem says why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<Placed>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub feature: String,
    /// draftwright's role vocabulary, `role.kind[.discriminator]`.
    pub parameter: String,
    pub value: f64,
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
    pub basis: String,
}

/// draftwright's number format (`_fmt`): a whole number bare, anything else to one decimal.
/// Geometric noise below 1e-6 mm is removed first, so 247.64999999 reads as 247.65 does.
pub fn fmt(v: f64) -> String {
    let v = (v * 1e6).round() / 1e6;
    let r = v.round();
    if (v - r).abs() < 1e-6 {
        format!("{}", if r == 0.0 { 0 } else { r as i64 })
    } else {
        format!("{v:.1}")
    }
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
#[derive(Clone, Debug)]
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
    /// A leader to one of these circles (centre, radius), world mm.
    Callout { sites: Vec<(P3, f64)> },
}

#[derive(Clone, Debug)]
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
    /// whichever radial axis the carrying view shows.
    turning: Option<usize>,
}

fn want_id(feature: &str, parameter: &str, view: &str) -> String {
    format!("d:{feature}/{parameter}@{view}")
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
const PLANNERS: [Planner; 12] = [
    turned::PLANNER,
    envelope::PLANNER,
    stock::PLANNER,
    holes::PLANNER,
    slots::PLANNER,
    bosses::PLANNER,
    edges::PLANNER,
    grooves::PLANNER,
    flats::PLANNER,
    steps::PLANNER,
    levels::PLANNER,
    recesses::PLANNER,
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

/// quiddity record types (`Fingerprint::subtype`) draftwright gives no dimensions of their own
/// (`model/detect.py`'s `_ORCHESTRATED_RECORDS` and `_UNCONSUMED_RECORDS`): a countersink rides on
/// its hole's callout, and the others have no reviewed drafting grammar there (#1365, #1430,
/// #1557). Leaving them undimensioned is no gap in the port, so they are not reported as one. An
/// angled step (#1247) has no grammar either, but is reported as unsupported ([`unsupported`]).
const UNDRAFTED: [&str; 10] = [
    "CircularFacePattern",
    "CounterSink",
    "FreeformSurface",
    "InteriorVoid",
    "ObliqueThroughStep",
    "OrientedChamfer",
    "OrientedSlotArray",
    "OrientedSlotGrid",
    "SheetMetalBody",
    "ThinWallBody",
];

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
        .filter(|f| !UNDRAFTED.contains(&f.fingerprint.subtype.as_str()))
        .map(|f| {
            unsupported(f).unwrap_or_else(|| Problem {
                id: format!("p:dimensions_not_ported:{}", f.id),
                code: "dimensions_not_ported".into(),
                severity: "info".into(),
                message: format!(
                    "{} is not dimensioned: no dimensions for {} are ported yet.",
                    f.id, f.kind
                ),
                subjects: vec![f.id.clone()],
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
            "angled step {} is not represented by Draftwright dimensions; review and define its \
             manufacturing geometry outside automatic drawing approval.",
            f.id
        ),
        subjects: vec![f.id.clone()],
    })
}

fn levenshtein(a: &str, b: &str) -> usize {
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

/// The view that carries each of *wants*: its first choice that is shown (in *frames*) and does
/// not look along it (a turned part's diameter turns to the radial axis the view shows). A later
/// choice renames it. Those with none keep the views that were shown but look along them.
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
        if let Some(v) = view
            && v != w.views[0]
        {
            let (stem, _) = w.id.rsplit_once('@').expect("an ID names its view");
            w.id = format!("{stem}@{v}");
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
    let (mut wants, ..) = wants(analysis);
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
    let (chosen, _) = route(&mut wants, &frames, analysis);
    let owner = vec![None; wants.len()];
    let (sides, lane) = placement::lanes(&frames, wants.iter().zip(chosen.iter().copied()), &owner);
    placement::band_lanes(chosen.iter().zip(sides).zip(lane))
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
    let (mut wants, mut problems, notes) = wants(analysis);
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            NAMES
                .contains(&name)
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    let (chosen, end_on) = route(&mut wants, &frames, analysis);
    let ids: BTreeSet<&str> = wants.iter().map(|w| w.id.as_str()).collect();
    // ARCH0016-5: each dimension is in exactly one view, under one ID. A duplicate is a planner
    // bug, checked in every build.
    assert_eq!(ids.len(), wants.len(), "dimension IDs are unique");
    for (id, entry) in &spec.dimensions {
        if !ids.contains(id.as_str()) {
            let mut near: Vec<&str> = ids.iter().copied().collect();
            near.sort_by_key(|c| (levenshtein(id, c), *c));
            near.truncate(5);
            // An ID names the view that carries the dimension, so omitting that view renames it.
            let stem = id.rsplit_once('@').map_or(id.as_str(), |(s, _)| s);
            let moved = wants
                .iter()
                .find(|w| w.id.rsplit_once('@').is_some_and(|(s, _)| s == stem) && w.id != *id);
            let mut choices: Vec<String> = moved.iter().map(|w| w.id.clone()).collect();
            for c in near {
                if !choices.iter().any(|x| x == c) {
                    choices.push(c.into());
                }
            }
            choices.truncate(5);
            let message = match moved {
                Some(w) => format!(
                    "There is no dimension {id:?} on this part: an ID names the view that carries it, and that dimension is now {:?} ({}).",
                    w.id, w.basis
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
    // The spec chooses what is drawn, and states the tolerances and fits on the kept labels.
    let all = wants.clone();
    let owner = placement::owners(analysis, &all, &chosen);
    let mut omitted = Vec::new();
    let mut kept = Vec::new();
    for ((w, view), along) in wants.into_iter().zip(chosen.iter().copied()).zip(end_on) {
        let entry = spec.dimensions.get(&w.id);
        match spec.dimension_source {
            DimensionSource::Auto if entry.is_some_and(|e| e.omit) => omitted.push(Omitted {
                id: w.id,
                basis: "authored: the spec omits it".into(),
            }),
            DimensionSource::Authored if entry.is_none() => omitted.push(Omitted {
                id: w.id,
                basis: "authored: not in the spec's complete set (dimension_source authored)"
                    .into(),
            }),
            _ => kept.push((w, entry.is_none(), view, along)),
        }
    }
    bosses::consolidate(analysis, &mut kept, &mut omitted);
    pmi_dimensions::yield_to_spec(spec, &notes, &mut kept, &mut problems);
    let mut given = tolerance::apply(analysis, spec, &mut kept, &omitted, &mut problems)?;
    holes::consolidate(analysis, &mut kept, &mut given, &mut omitted);
    share(&mut kept, &mut given, &mut omitted);
    problems.extend(pmi_dimensions::omitted(&all, &kept, &omitted));

    // Then everything the part would have is placed: each item's lane, and where its label
    // settles, come from the full set, so omitting or restoring one leaves the others where they
    // were (ARCH0003-2). A kept item is placed with its label as drawn (its tolerance or fit
    // included); one the spec omits keeps its default label and its place, so it leaves a gap.
    // An item the defaults merge away (a height the overall extent conveys among them) takes no
    // lane: it shares its survivor's.
    let labels: BTreeMap<&str, &[String]> = kept
        .iter()
        .map(|(w, ..)| (w.id.as_str(), w.terms.as_slice()))
        .collect();
    let all: Vec<Want> = all
        .into_iter()
        .map(|mut w| {
            if let Some(terms) = labels.get(w.id.as_str()) {
                w.terms = terms.to_vec();
            }
            w
        })
        .collect();
    let (mut sides, mut lane) =
        placement::lanes(&frames, all.iter().zip(chosen.iter().copied()), &owner);
    placement::fall_through(
        &frames, &all, &chosen, &owner, &mut sides, &mut lane, analysis, sheet, views,
    );
    let drawn: Vec<bool> = all
        .iter()
        .map(|w| labels.contains_key(w.id.as_str()))
        .collect();
    let alone = placement::separate(&owner, &drawn, &chosen, &sides, &mut lane);
    let mut placed: Vec<Option<Placed>> = all
        .iter()
        .zip(&chosen)
        .zip(sides.iter().zip(&lane))
        .zip(&owner)
        .map(|(((w, view), (side, lane)), owner)| {
            view.zip(*side)
                .filter(|_| owner.is_none())
                .and_then(|(v, side)| placement::place(&frames[v], w, side, *lane))
        })
        .collect();
    placement::settle(&mut placed, analysis, sheet, views);
    let page = placement::Page::new(analysis, sheet, views);
    let leader_problems = leaders::place(&frames, &all, &chosen, &sides, &mut placed, &page);
    placement::place_merged(
        &frames,
        &all,
        &chosen,
        &owner,
        &alone,
        &sides,
        &lane,
        &mut placed,
        &page,
    );
    let mut positions: BTreeMap<String, Option<Placed>> =
        all.into_iter().map(|w| w.id).zip(placed).collect();

    // Undrawn items by the views that could carry them and those of them that look along them.
    type Undrawn = (Vec<&'static str>, Vec<&'static str>);
    let mut missing: BTreeMap<Undrawn, Vec<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for ((w, default, chosen, along), from_spec) in kept.into_iter().zip(given) {
        let placed = positions.remove(&w.id).flatten();
        let view = chosen.unwrap_or(w.views[0]);
        if placed.is_none() {
            missing
                .entry((w.views.clone(), along))
                .or_default()
                .push(w.id.clone());
        }
        let basis = if default {
            w.basis
        } else {
            format!("listed in the spec; {}", w.basis)
        };
        out.push(PlannedDimension {
            id: w.id,
            kind: match w.shape {
                Shape::Linear { .. } => "linear",
                Shape::Callout { .. } => "callout",
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
            placed,
        });
    }
    for ((candidates, along), ids) in missing {
        let n = ids.len();
        let s = if n == 1 { "" } else { "s" };
        let named = |v: &[&str]| v.join(" or ");
        let omitted: Vec<&str> = candidates
            .iter()
            .copied()
            .filter(|v| !along.contains(v))
            .collect();
        let mut why = Vec::new();
        if !omitted.is_empty() {
            why.push(format!(
                "the {} view{} omitted",
                named(&omitted),
                if omitted.len() == 1 { " is" } else { "s are" }
            ));
        }
        if !along.is_empty() {
            why.push(format!(
                "the {} view looks along {}",
                named(&along),
                if n == 1 { "it" } else { "them" }
            ));
        }
        let fix = if omitted.is_empty() {
            "Omit them, or show a view across them.".to_string()
        } else {
            format!(
                "Restore the view (remove views/{}/omit) or omit them.",
                omitted[0]
            )
        };
        problems.push(Problem {
            id: format!("p:view_plan_incomplete:v:{}", candidates.join("+v:")),
            code: "view_plan_incomplete".into(),
            severity: "error".into(),
            message: format!(
                "{n} dimension{s} can only be drawn in the {} view, but {}; they stay in the plan undrawn. {fix}",
                named(&candidates),
                why.join(" and ")
            ),
            subjects: ids,
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
    problems.extend(placement::collisions(analysis, &out, sheet, views));
    Ok((out, omitted, problems))
}
/// The sizes that one dimension states for every feature it spans ([`share`]), and whether its
/// label then counts them (`n×`).
const SHARED: [(&str, bool); 4] = [
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

/// One span, one dimension, for sizes (ARCH0009-6, which the location merge in
/// [`holes::consolidate`] applies to locations): kept items stating the same [`SHARED`] size of
/// different features across the same span (the same view and axis, raw ends within 1e-6 mm,
/// and the same label, tolerance included, so different stated sizes or bands never collapse)
/// are drawn once. The survivor is one the spec lists over a default, then the smaller ID; it
/// claims every member's measurement and, for a counted size, prints their number (`2× 6`).
/// Each other moves to *omitted*, `conveyed by` the survivor. *given* stays aligned with *kept*.
fn share(
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
) {
    const SAME_SPAN: f64 = 1e-6;
    type Span = (String, &'static str, usize, f64, f64, String);
    let span = |(w, _, view, _): &(Want, bool, Option<&'static str>, Vec<&'static str>)| {
        let parameter = &w.measurements.first()?.parameter;
        SHARED.iter().find(|(p, _)| p == parameter)?;
        match w.shape {
            Shape::Linear { a, b, axis, .. } => Some((
                parameter.clone(),
                view.unwrap_or(w.views[0]),
                axis,
                a[axis].min(b[axis]),
                a[axis].max(b[axis]),
                w.terms.join(" "),
            )),
            Shape::Callout { .. } => None,
        }
    };
    let same = |s: &Span, t: &Span| {
        s.0 == t.0
            && s.1 == t.1
            && s.2 == t.2
            && (s.3 - t.3).abs() <= SAME_SPAN
            && (s.4 - t.4).abs() <= SAME_SPAN
            && s.5 == t.5
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
                basis: format!(
                    "conveyed by {owner}: it states the same {} ({}) across the same span in {}, in the same view (one span, one dimension: ARCH0009-6)",
                    s.0, s.5, AXES[s.2]
                ),
            });
            claims.extend(w.measurements.iter().cloned());
            gone.insert(i);
        }
        let w = &mut kept[*winner].0;
        w.measurements.extend(claims);
        let features: BTreeSet<&str> = w.measurements.iter().map(|m| m.feature.as_str()).collect();
        let n = features.len();
        let counted = SHARED.iter().any(|(p, c)| *p == s.0 && *c);
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
        if pmi_dimensions::basic(&d.measurements) {
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

#[cfg(test)]
mod tests {
    use super::*;

    type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

    /// A default size of *feature* in the plan, along x from *lo* to *hi*, labelled *label*.
    fn size(feature: &str, parameter: &str, lo: f64, hi: f64, label: &[&str]) -> Kept {
        let want = Want {
            id: want_id(feature, parameter, "plan"),
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
                ("d:f:slot:1/slot_width.length@plan".into(), "2× 6".into(), 2),
                ("d:f:slot:3/slot_width.length@plan".into(), "6".into(), 1),
            ]
        );
        assert_eq!(
            omitted,
            [(
                "d:f:slot:2/slot_width.length@plan".into(),
                "d:f:slot:1/slot_width.length@plan".into()
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
            [(
                "d:f:hole_pattern:1/pitch.length@plan".into(),
                "2× 75".into(),
                2
            )]
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
            [(
                "d:f:boss:2/boss_height.length@plan".into(),
                "2× 5".into(),
                2
            )]
        );
        assert_eq!(omitted[0].0, "d:f:boss:1/boss_height.length@plan");
    }
}
