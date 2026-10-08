//! Dimensions and hole callouts: what the sheet states about sizes, planned from the Analysis's
//! features and drawn only from the plan (ADR R1 invariant 2).
//!
//! This first cut covers:
//! - **the overall extents**: width, depth and height of a prismatic part; the diameter and the
//!   axial length of a part that is turned whole;
//! - **hole callouts**: draftwright's compound grammar (`4× ⌀10 THRU ⌴ ⌀16 ↧ 5 (2×2)`), with
//!   identical holes grouped and counted, and quiddity's hole patterns called out as patterns;
//! - **hole locations** from the part's box, in the view that looks along each hole, and the
//!   pitches that locate a pattern's members.
//!
//! Each is a [`PlannedDimension`] with a stable ID, `d:<feature>/<parameter>@<view>`, its value,
//! its printed terms and label, the measurements it states, a default flag and a basis.
//!
//! **Placement is simple and honest**, not draftwright's solver: linear dimensions stack in lanes
//! below (horizontal) and right of (vertical) their view, sizes nearest, then locations by
//! distance, then the overall extent outermost; callouts stack above their view with 45° leaders.
//! Whatever then collides (labels with labels, lines through labels, labels over views, ink off
//! the sheet) is reported as a problem on the dimension's ID, never silently moved or dropped.
//! Lines crossing other views' outlines or other dimensions' lines are not checked yet. An item
//! no shown view can carry is kept in the plan, unplaced, and reported (R1 invariant 3).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use haecceity::hlr::View;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bridge::{Feature, HolePattern, holes};
use crate::layout::MARGIN;
use crate::model::{Analysis, DimensionSource, Invalid, PlannedSheet, PlannedView, Problem, Spec};
use crate::output::{Annotation, Claim, Curve, HAlign, Item, TextRun, VAlign};
use crate::text::{Font, Segment};

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
const MONO: &[u8] = include_bytes!("../fonts/IBMPlexMono-Regular.ttf");

fn font() -> &'static Font {
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
/// Member sites projected into a view, as integer keys.
type Sites = Vec<(i64, i64)>;
/// Strip lanes: (rank, sub-rank, item index), per view and strip.
type Lanes<'a> = BTreeMap<(&'a str, Side), Vec<(u8, i64, usize)>>;
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
    holes::size_key(x)
}

/// A recognised hole the callout grammar can state: read by [`holes::Hole::read`], along a
/// principal axis, with its depth when it is blind.
#[derive(Clone, Debug)]
struct Hole<'a> {
    read: holes::Hole<'a>,
    /// The world axis it lies along.
    axis: usize,
}

impl<'a> std::ops::Deref for Hole<'a> {
    type Target = holes::Hole<'a>;
    fn deref(&self) -> &holes::Hole<'a> {
        &self.read
    }
}

impl Hole<'_> {
    /// The recess the grammar prints (the counterbore, else the spotface): role, diameter, depth.
    fn recess(&self) -> Option<(&'static str, holes::Recess)> {
        self.counterbore
            .map(|r| ("counterbore", r))
            .or(self.spotface.map(|r| ("spotface", r)))
    }

    /// The terms after the count: `⌀d`, `THRU` or `↧ depth`, the recess, the countersink.
    fn terms(&self) -> Vec<String> {
        let mut t = vec![format!("⌀{}", fmt(self.diameter))];
        match self.depth {
            Some(depth) if !self.through => t.extend(["↧".into(), fmt(depth)]),
            _ => t.push("THRU".into()),
        }
        if let Some((_, r)) = self.recess() {
            t.extend(["⌴".into(), format!("⌀{}", fmt(r.diameter))]);
            if let Some(depth) = r.depth {
                t.extend(["↧".into(), fmt(depth)]);
            }
        }
        if let Some(c) = self.countersink {
            t.extend(["⌵".into(), format!("⌀{}", fmt(c.diameter))]);
            if let Some(a) = c.angle {
                t.extend(["×".into(), format!("{}°", fmt(a))]);
            }
        }
        t
    }

    fn measurements(&self) -> Vec<Measurement> {
        let id = &self.feature.id;
        let m = |p: &str, v: f64| Measurement {
            feature: id.clone(),
            parameter: p.into(),
            value: v,
        };
        let mut out = vec![m("bore.diameter", self.diameter)];
        if let Some(depth) = self.depth.filter(|_| !self.through) {
            out.push(m("bore.depth", depth));
        }
        if let Some((role, r)) = self.recess() {
            out.push(m(&format!("{role}.diameter"), r.diameter));
            if let Some(depth) = r.depth {
                out.push(m(&format!("{role}.depth"), depth));
            }
        }
        if let Some(c) = self.countersink {
            out.push(m("countersink.diameter", c.diameter));
            if let Some(a) = c.angle {
                out.push(m("countersink.angle", a));
            }
        }
        out
    }
}

/// A hole as the callout grammar reads it, or why it cannot state it.
fn hole<'a>(read: Result<holes::Hole<'a>, String>) -> Result<Hole<'a>, String> {
    let read = read?;
    let (axis, _) = read
        .principal
        .ok_or("its axis is not along x, y or z, so no standard view looks along it")?;
    if !read.through && read.depth.is_none() {
        return Err("it is blind and has no depth".into());
    }
    Ok(Hole { read, axis })
}

/// A part turned whole: its axis (a world axis), a point on it, and its largest diameter, when
/// every turned step shares one principal axis and the largest step spans the part's box across
/// it.
struct Turned {
    axis: usize,
    on: P3,
    diameter: f64,
}

fn turned(analysis: &Analysis, frame: &QFrame) -> Option<Turned> {
    let steps: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .collect();
    let mut found: Option<(usize, P3)> = None;
    let mut diameter: f64 = 0.0;
    for s in &steps {
        let p = &s.parameters;
        let k = holes::principal(frame.letter(p.get("axis")?)?)?;
        let on = frame.point(point(p.get("profile")?.get("axis_origin"))?);
        if let Some((k0, on0)) = found
            && (k0 != k || (0..3).any(|j| j != k && (on[j] - on0[j]).abs() > 1e-3))
        {
            return None;
        }
        found = Some((k, on));
        diameter = diameter.max(p.get("diameter")?.as_f64()?);
    }
    let (axis, on) = found?;
    let [lo, hi] = analysis.envelope;
    let whole = (0..3)
        .filter(|&j| j != axis)
        .all(|j| ((hi[j] - lo[j]) - diameter).abs() < 1e-3 * diameter.max(1.0));
    whole.then_some(Turned { axis, on, diameter })
}

/// Everything the defaults would dimension, with the problems met on the way.
fn wants(analysis: &Analysis) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let [lo, hi] = analysis.envelope;
    let frame = QFrame(analysis.recognition.as_ref().map_or(
        [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        |r| r.frame,
    ));
    let turned = turned(analysis, &frame);

    // The overall extents.
    let names = ["width", "depth", "height"];
    let profile_views = |axis: usize| -> Vec<&'static str> {
        match axis {
            0 => vec!["front", "plan"],
            1 => vec!["side", "plan"],
            _ => vec!["front", "side"],
        }
    };
    let corner = |k: usize, at: f64, other: P3| {
        let mut p = other;
        p[k] = at;
        p
    };
    match &turned {
        Some(t) => {
            let views = profile_views(t.axis);
            // Across the axis, in the first of its profile views: the radial axis that view shows.
            let radial = (0..3)
                .find(|&j| j != t.axis && profile_views(j).contains(&views[0]))
                .unwrap_or((t.axis + 1) % 3);
            out.push(Want {
                id: want_id("envelope", "od.diameter", views[0]),
                views: views.clone(),
                value: t.diameter,
                terms: vec![format!("ø{}", fmt(t.diameter))],
                measurements: vec![Measurement {
                    feature: "envelope".into(),
                    parameter: "od.diameter".into(),
                    value: t.diameter,
                }],
                turning: Some(t.axis),
                basis: format!(
                    "the part is turned whole about {}: its largest diameter is its overall size across the axis, in a view that shows the profile",
                    AXES[t.axis]
                ),
                shape: Shape::Linear {
                    a: corner(radial, lo[radial], lo),
                    b: corner(radial, hi[radial], lo),
                    axis: radial,
                    a_edge: true,
                    b_edge: true,
                    order: 2,
                },
            });
            let k = t.axis;
            let name = names[k];
            out.push(Want {
                id: want_id("envelope", &format!("{name}.length"), views[0]),
                views,
                value: hi[k] - lo[k],
                terms: vec![fmt(hi[k] - lo[k])],
                measurements: vec![Measurement {
                    feature: "envelope".into(),
                    parameter: format!("{name}.length"),
                    value: hi[k] - lo[k],
                }],
                turning: None,
                basis: "the overall length along the turning axis, in a view that shows the profile; no step chain is planned yet to state it (as draftwright releases the overall length when its chain is dropped)".into(),
                shape: Shape::Linear {
                    a: corner(k, lo[k], lo),
                    b: corner(k, hi[k], lo),
                    axis: k,
                    a_edge: true,
                    b_edge: true,
                    order: 2,
                },
            });
        }
        None => {
            let views: [Vec<&'static str>; 3] = [
                vec!["front", "plan"],
                vec!["side", "plan"],
                vec!["front", "side"],
            ];
            for (k, name) in names.iter().enumerate() {
                let v = hi[k] - lo[k];
                if v <= 1e-6 {
                    continue;
                }
                out.push(Want {
                    id: want_id("envelope", &format!("{name}.length"), views[k][0]),
                    views: views[k].clone(),
                    value: v,
                    terms: vec![fmt(v)],
                    measurements: vec![Measurement {
                        feature: "envelope".into(),
                        parameter: format!("{name}.length"),
                        value: v,
                    }],
                    turning: None,
                    basis: format!(
                        "every overall extent is stated once, outermost: the {name} ({} extent) in the {} view",
                        AXES[k], views[k][0]
                    ),
                    shape: Shape::Linear {
                        a: corner(k, lo[k], lo),
                        b: corner(k, hi[k], lo),
                        axis: k,
                        a_edge: true,
                        b_edge: true,
                        order: 2,
                    },
                });
            }
        }
    }

    // Holes.
    let mut holes: Vec<Hole> = Vec::new();
    for (f, read) in holes::read(analysis) {
        match hole(read) {
            Ok(h) => {
                let bottom = h.bottom;
                if !matches!(bottom, Some("through" | "flat" | "drill_point")) {
                    // Called out blind to its recognised depth, as draftwright does, but the
                    // recogniser did not establish that it stops there.
                    problems.push(Problem {
                        id: format!("p:hole_bottom_unknown:{}", f.id),
                        code: "hole_bottom_unknown".into(),
                        severity: "info".into(),
                        message: format!(
                            "{} is called out blind to its recognised depth ({}), but recognition did not establish its bottom ({}); check whether it is through.",
                            f.id,
                            fmt(h.depth.expect("a blind hole has its depth")),
                            bottom.unwrap_or("none given")
                        ),
                        subjects: vec![f.id.clone()],
                    });
                }
                holes.push(h);
            }
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!("{} is not dimensioned: {why}.", f.id),
                subjects: vec![f.id.clone()],
            }),
        }
    }
    let index: BTreeMap<&str, usize> = holes
        .iter()
        .enumerate()
        .map(|(i, h)| (h.feature.id.as_str(), i))
        .collect();
    let patterns: &[HolePattern] = analysis
        .recognition
        .as_ref()
        .map_or(&[], |r| r.patterns.as_slice());

    // Patterns first: each that holds together takes its members.
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    let mut callouts: Vec<Want> = Vec::new();
    let mut located: Vec<(String, P3, usize, String)> = Vec::new(); // owner, point, view axis…
    let mut pitches: Vec<Want> = Vec::new();
    let mut declined: BTreeMap<usize, String> = BTreeMap::new();
    let mut coincident: Vec<(usize, P3, Sites)> = Vec::new(); // callout index, station
    let reads: Vec<&holes::Hole> = holes.iter().map(|h| &h.read).collect();
    for p in patterns {
        let members = match holes::established(p, &reads) {
            Ok(members) => members,
            Err(why) => {
                for m in &p.members {
                    if let Some(&i) = index.get(m.as_str()) {
                        declined.entry(i).or_insert_with(|| why.clone());
                    }
                }
                continue;
            }
        };
        if members.iter().any(|m| taken.contains(m)) {
            continue;
        }
        let first = &holes[members[0]];
        let k = first.axis;
        let view = end_on(k);
        let (u, v) = in_view(k);
        let centres: Vec<P3> = members.iter().map(|&m| holes[m].at).collect();
        let n = members.len();
        let (suffix, bcd, why): (Option<String>, Option<f64>, String) = match p.arrangement.as_str()
        {
            "bolt_circle" => {
                let d = p.parameters.get("diameter").and_then(Value::as_f64);
                let c = point(p.parameters.get("center")).map(|c| frame.point(c));
                match (d, c) {
                    (Some(d), Some(c)) => {
                        located_pattern(&mut located, &p.id, c, k, turned.as_ref());
                        (
                            Some(format!("EQ SP ON ø{} BC", fmt(d))),
                            Some(d),
                            "a bolt circle of five or more holes is self-evident (ISSUE-1596-a): it is called out with its circle and located by its centre".to_string(),
                        )
                    }
                    _ => {
                        for &m in &members {
                            declined.insert(m, format!("quiddity's bolt circle {} gives no diameter or centre, so its holes are dimensioned singly", p.id));
                        }
                        continue;
                    }
                }
            }
            "grid" => match lattice(&centres, u, v) {
                Some((cols, rows)) if cols.len() * rows.len() == n => {
                    let centre = {
                        let mut c = centres[0];
                        c[u] = 0.5 * (cols[0] + cols[cols.len() - 1]);
                        c[v] = 0.5 * (rows[0] + rows[rows.len() - 1]);
                        c
                    };
                    located_pattern(&mut located, &p.id, centre, k, turned.as_ref());
                    for (along, values, name) in [(u, &cols, "col"), (v, &rows, "row")] {
                        if values.len() > 1 {
                            pitches.push(pitch_want(
                                &p.id,
                                &format!("grid_pitch.length.{name}"),
                                view,
                                centres[0],
                                along,
                                values,
                            ));
                        }
                    }
                    (
                        Some(format!("({}×{})", rows.len(), cols.len())),
                        None,
                        format!(
                            "a grid of {}×{} identical holes is called out once with its size, located by its centre and spaced by its pitches",
                            rows.len(),
                            cols.len()
                        ),
                    )
                }
                _ => {
                    for &m in &members {
                        declined.insert(m, format!("quiddity's grid {} is not a square lattice in the {view} view, so its holes are dimensioned singly", p.id));
                    }
                    continue;
                }
            },
            _ => match line(&centres, u, v) {
                Some((along, values)) => {
                    let mut start = centres[0];
                    start[along] = values[0];
                    if let Some(c) = centres.iter().find(|c| (c[along] - values[0]).abs() < 1e-4) {
                        start = *c;
                    }
                    located_pattern(&mut located, &p.id, start, k, turned.as_ref());
                    pitches.push(pitch_want(
                        &p.id,
                        "pitch.length",
                        view,
                        start,
                        along,
                        &values,
                    ));
                    (
                        None,
                        None,
                        "a linear pattern is called out once, located by its first member and spaced by its pitch".to_string(),
                    )
                }
                None => {
                    for &m in &members {
                        declined.insert(m, format!("quiddity's linear pattern {} is not evenly spaced along one axis of the {view} view, so its holes are dimensioned singly", p.id));
                    }
                    continue;
                }
            },
        };
        taken.extend(members.iter().copied());
        let mut terms = vec![format!("{n}×")];
        terms.extend(first.terms());
        let mut measurements: Vec<Measurement> = members
            .iter()
            .flat_map(|&m| holes[m].measurements())
            .collect();
        terms.extend(suffix);
        if let Some(d) = bcd {
            measurements.push(Measurement {
                feature: p.id.clone(),
                parameter: "bolt_circle.diameter".into(),
                value: d,
            });
        }
        let sites: Vec<(P3, f64)> = members
            .iter()
            .map(|&m| (holes[m].at, 0.5 * holes[m].diameter))
            .collect();
        let mut projected: Vec<(i64, i64)> =
            sites.iter().map(|(c, _)| (key(c[u]), key(c[v]))).collect();
        projected.sort();
        if k == 2 {
            coincident.push((callouts.len(), first.at, projected));
        }
        callouts.push(Want {
            id: want_id(&p.id, "bore.diameter", view),
            views: vec![view],
            value: first.diameter,
            terms,
            measurements,
            turning: None,
            basis: format!("{why}; called out where it is seen end-on"),
            shape: Shape::Callout { sites },
        });
    }

    // Draftwright names the axial face of z patterns whose members coincide in the plan.
    let mut by_sites: BTreeMap<&Sites, Vec<(usize, f64)>> = BTreeMap::new();
    for (i, c, sites) in &coincident {
        by_sites.entry(sites).or_default().push((*i, c[2]));
    }
    for group in by_sites.values().filter(|g| g.len() > 1) {
        let mut stations: Vec<f64> = group.iter().map(|g| g.1).collect();
        stations.sort_by(f64::total_cmp);
        stations.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        if stations.len() < 2 {
            continue;
        }
        for (i, z) in group {
            let n = stations
                .iter()
                .position(|s| (s - z).abs() < 1e-6)
                .unwrap_or(0);
            let which = if stations.len() == 2 {
                ["LOWER FACE", "UPPER FACE"][n].to_string()
            } else {
                format!("FACE {} OF {} FROM LOWER END", n + 1, stations.len())
            };
            let w = &mut callouts[*i];
            match w.terms.last_mut() {
                Some(last) if last.starts_with("EQ SP") || last.starts_with('(') => {
                    *last = format!("{last} {which}")
                }
                _ => w.terms.push(which),
            }
            w.basis += "; its members project onto another pattern's, so the face it is drilled from is named";
        }
    }

    // The holes left: identical ones grouped and counted (ARCH0008-2).
    let mut groups: Vec<(Vec<i64>, Vec<usize>)> = Vec::new();
    for (i, h) in holes.iter().enumerate() {
        if taken.contains(&i) {
            continue;
        }
        let req = h.requirement();
        match groups.iter_mut().find(|g| g.0 == req) {
            Some(g) => g.1.push(i),
            None => groups.push((req, vec![i])),
        }
    }
    for (_, members) in &groups {
        let first = &holes[members[0]];
        let view = end_on(first.axis);
        let mut terms = Vec::new();
        if members.len() > 1 {
            terms.push(format!("{}×", members.len()));
        }
        terms.extend(first.terms());
        let note = members
            .iter()
            .find_map(|m| declined.get(m))
            .map_or(String::new(), |d| format!("; {d}"));
        callouts.push(Want {
            id: want_id(&first.feature.id, "bore.diameter", view),
            views: vec![view],
            value: first.diameter,
            terms,
            measurements: members.iter().flat_map(|&m| holes[m].measurements()).collect(),
            turning: None,
            basis: if members.len() > 1 {
                format!("identical holes drilled from one face are called out once and counted, where they are seen end-on{note}")
            } else {
                format!("a hole's size is called out where it is seen end-on{note}")
            },
            shape: Shape::Callout {
                sites: members
                    .iter()
                    .map(|&m| (holes[m].at, 0.5 * holes[m].diameter))
                    .collect(),
            },
        });
        for &m in members {
            let h = &holes[m];
            let (u, v) = in_view(h.axis);
            for k in [u, v] {
                if let Some(t) = &turned
                    && k != t.axis
                    && (h.at[k] - t.on[k]).abs() < 1e-3
                {
                    continue; // on the turning axis: the centre line locates it
                }
                located.push((h.feature.id.clone(), h.at, k, view.into()));
            }
        }
    }
    out.extend(callouts);
    out.extend(pitches);

    // Locations from the part's box: one dimension per distinct distance (ARCH0009-6).
    let mut by: BTreeMap<(String, usize, i64), Vec<(String, P3)>> = BTreeMap::new();
    for (owner, p, k, view) in located {
        let d = p[k] - lo[k];
        if d.abs() < 1e-6 {
            problems.push(Problem {
                id: format!("p:location_on_datum:{owner}/{}", AXES[k]),
                code: "location_on_datum".into(),
                severity: "info".into(),
                message: format!(
                    "{owner} lies on the part's {} face, so its {} location measures nothing and is not dimensioned.",
                    ["left", "front", "bottom"][k],
                    AXES[k]
                ),
                subjects: vec![owner],
            });
            continue;
        }
        by.entry((view, k, key(d))).or_default().push((owner, p));
    }
    let rank = |id: &str| {
        analysis
            .features
            .iter()
            .position(|f| f.id == id)
            .unwrap_or(usize::MAX)
    };
    for ((view, k, _), mut owners) in by {
        owners.sort_by_key(|(o, _)| (rank(o), o.clone()));
        owners.dedup_by(|a, b| a.0 == b.0);
        let (first, p) = &owners[0];
        let d = p[k] - lo[k];
        let parameter = format!("location.length.{}", AXES[k]);
        let view: &'static str = NAMES.iter().find(|n| **n == view).expect("a view");
        out.push(Want {
            id: want_id(first, &parameter, view),
            views: vec![view],
            value: d,
            terms: vec![fmt(d)],
            measurements: owners
                .iter()
                .map(|(o, _)| Measurement {
                    feature: o.clone(),
                    parameter: parameter.clone(),
                    value: d,
                })
                .collect(),
            turning: None,
            basis: format!(
                "located in {} from the part's {} face, in the view that looks along the hole{}",
                AXES[k],
                ["left", "front", "bottom"][k],
                if owners.len() > 1 {
                    "; one dimension serves every feature at this distance"
                } else {
                    ""
                }
            ),
            shape: Shape::Linear {
                a: {
                    let mut a = *p;
                    a[k] = lo[k];
                    a
                },
                b: *p,
                axis: k,
                a_edge: true,
                b_edge: false,
                order: 1,
            },
        });
    }
    (out, problems)
}

/// The world axes a view looking along *k* shows across and up the page.
fn in_view(k: usize) -> (usize, usize) {
    match k {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

fn distinct(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut v: Vec<f64> = values.collect();
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    v
}

fn even(values: &[f64]) -> bool {
    values.len() < 3
        || values
            .windows(2)
            .all(|w| ((w[1] - w[0]) - (values[1] - values[0])).abs() < 1e-4)
}

/// A grid's distinct column and row positions, when its members are an evenly spaced lattice
/// along the view's axes.
fn lattice(centres: &[P3], u: usize, v: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    let cols = distinct(centres.iter().map(|c| c[u]));
    let rows = distinct(centres.iter().map(|c| c[v]));
    (even(&cols) && even(&rows)).then_some((cols, rows))
}

/// A linear pattern's axis and its members' positions along it, when they lie evenly along one
/// of the view's axes.
fn line(centres: &[P3], u: usize, v: usize) -> Option<(usize, Vec<f64>)> {
    for (along, across) in [(u, v), (v, u)] {
        let a = distinct(centres.iter().map(|c| c[along]));
        let b = distinct(centres.iter().map(|c| c[across]));
        if b.len() == 1 && a.len() == centres.len() && even(&a) {
            return Some((along, a));
        }
    }
    None
}

fn located_pattern(
    located: &mut Vec<(String, P3, usize, String)>,
    id: &str,
    centre: P3,
    k: usize,
    turned: Option<&Turned>,
) {
    let (u, v) = in_view(k);
    for a in [u, v] {
        if let Some(t) = turned
            && a != t.axis
            && (centre[a] - t.on[a]).abs() < 1e-3
        {
            continue;
        }
        located.push((id.into(), centre, a, end_on(k).into()));
    }
}

fn pitch_want(
    pattern: &str,
    parameter: &str,
    view: &'static str,
    at: P3,
    along: usize,
    values: &[f64],
) -> Want {
    let p = values[1] - values[0];
    let intervals = values.len() - 1;
    let mut terms = Vec::new();
    let text = if intervals > 1 {
        format!("{intervals}× {}", fmt(p))
    } else {
        fmt(p)
    };
    terms.push(text);
    let (mut a, mut b) = (at, at);
    a[along] = values[0];
    b[along] = values[1];
    Want {
        id: want_id(pattern, parameter, view),
        views: vec![view],
        value: p,
        terms,
        measurements: vec![Measurement {
            feature: pattern.into(),
            parameter: parameter.into(),
            value: p,
        }],
        turning: None,
        basis: format!(
            "the pattern's spacing in {}, between its first two members, counted across its {intervals} interval{}",
            AXES[along],
            if intervals == 1 { "" } else { "s" }
        ),
        shape: Shape::Linear {
            a,
            b,
            axis: along,
            a_edge: false,
            b_edge: false,
            order: 0,
        },
    }
}

/// A view as the dimensions see it: world to page, and the part's box on the page.
struct Frame {
    view: View,
    at: P2,
    s: f64,
    centre: P2,
    /// The projected envelope's box on the page.
    outline: [f64; 4],
}

impl Frame {
    fn new(analysis: &Analysis, v: &PlannedView) -> Frame {
        let view = View::new(v.direction, v.up).expect("a planned view");
        let (size, centre) = crate::model::extent(analysis, &view);
        let s = v.scale;
        Frame {
            view,
            at: v.at,
            s,
            centre,
            outline: [
                v.at[0] - 0.5 * s * size[0],
                v.at[1] - 0.5 * s * size[1],
                v.at[0] + 0.5 * s * size[0],
                v.at[1] + 0.5 * s * size[1],
            ],
        }
    }

    fn page(&self, p: P3) -> P2 {
        let q = self.view.map(p);
        [
            self.at[0] + self.s * (q[0] - self.centre[0]),
            self.at[1] + self.s * (q[1] - self.centre[1]),
        ]
    }
}

/// A label laid out in its own frame: the baseline on y = 0 from x = 0.
pub struct LabelInk {
    /// Each text token's glyph outlines.
    pub glyphs: Vec<Vec<Vec<Segment>>>,
    /// The drawn symbols' strokes.
    pub strokes: Vec<[P2; 2]>,
    /// The text layer's runs: text and where it starts on the baseline.
    pub runs: Vec<(String, f64)>,
    pub min: P2,
    pub max: P2,
}

/// Lays out *terms* as draftwright's callout recipe does: each term's leading symbol in a cell of
/// one text height (`⌀` as the font's `ø`, the others drawn), each token a gap apart.
pub fn label_ink(terms: &[String], font: &Font) -> LabelInk {
    let h = TEXT;
    let mut ink = LabelInk {
        glyphs: Vec::new(),
        strokes: Vec::new(),
        runs: Vec::new(),
        min: [f64::INFINITY; 2],
        max: [f64::NEG_INFINITY; 2],
    };
    let grow = |ink: &mut LabelInk, a: P2, b: P2| {
        for p in [a, b] {
            ink.min = [ink.min[0].min(p[0]), ink.min[1].min(p[1])];
            ink.max = [ink.max[0].max(p[0]), ink.max[1].max(p[1])];
        }
    };
    let mut x = 0.0;
    let mut tokens: Vec<(Option<char>, String)> = Vec::new();
    for t in terms {
        let mut chars = t.chars();
        match chars.next() {
            Some(c @ ('⌀' | '↧' | '⌴' | '⌵')) => {
                tokens.push((Some(c), String::new()));
                let rest: String = chars.collect();
                if !rest.is_empty() {
                    tokens.push((None, rest));
                }
            }
            _ => tokens.push((None, t.clone())),
        }
    }
    for (i, (symbol, text)) in tokens.iter().enumerate() {
        if i > 0 {
            x += TOKEN_GAP;
        }
        let glyphs = |s: &str, at: f64, ink: &mut LabelInk| {
            let laid = font.baseline(s, h);
            if !laid.contours.is_empty() {
                grow(
                    ink,
                    [at + laid.min[0], laid.min[1]],
                    [at + laid.max[0], laid.max[1]],
                );
            }
            let shift = |p: P2| [p[0] + at, p[1]];
            ink.glyphs.push(
                laid.contours
                    .into_iter()
                    .map(|c| c.into_iter().map(|s| moved(&s, &shift)).collect())
                    .collect(),
            );
            ink.runs.push((s.to_string(), at));
        };
        match symbol {
            Some('⌀') => {
                let at = x + 0.5 * (h - font.advance("ø", h));
                glyphs("ø", at, &mut ink);
                x += h;
            }
            Some(c) => {
                let (l, m, r) = (x + 0.15 * h, x + 0.5 * h, x + 0.85 * h);
                let strokes: Vec<[P2; 2]> = match c {
                    '↧' => vec![
                        [[l, CAP], [r, CAP]],
                        [[m, CAP], [m, 0.0]],
                        [[m - 0.2 * h, 0.25 * h], [m, 0.0]],
                        [[m, 0.0], [m + 0.2 * h, 0.25 * h]],
                    ],
                    '⌴' => vec![
                        [[l, CAP], [l, 0.0]],
                        [[l, 0.0], [r, 0.0]],
                        [[r, 0.0], [r, CAP]],
                    ],
                    _ => vec![[[l, CAP], [m, 0.0]], [[m, 0.0], [r, CAP]]],
                };
                for s in &strokes {
                    grow(&mut ink, s[0], s[1]);
                }
                ink.strokes.extend(strokes);
                x += h;
            }
            None => {
                glyphs(text, x, &mut ink);
                x += font.advance(text, h);
            }
        }
    }
    if !ink.min[0].is_finite() {
        (ink.min, ink.max) = ([0.0; 2], [0.0; 2]);
    }
    ink
}

fn moved(s: &Segment, f: &dyn Fn(P2) -> P2) -> Segment {
    match *s {
        Segment::Line(a, b) => Segment::Line(f(a), f(b)),
        Segment::Quad(a, b, c) => Segment::Quad(f(a), f(b), f(c)),
        Segment::Cubic(a, b, c, d) => Segment::Cubic(f(a), f(b), f(c), f(d)),
    }
}

/// A label's frame on the page: its baseline origin and direction.
fn to_page(origin: P2, rotation: f64) -> impl Fn(P2) -> P2 {
    let (sin, cos) = rotation.to_radians().sin_cos();
    move |p: P2| {
        [
            origin[0] + cos * p[0] - sin * p[1],
            origin[1] + sin * p[0] + cos * p[1],
        ]
    }
}

fn label_box(ink: &LabelInk, origin: P2, rotation: f64) -> [f64; 4] {
    let f = to_page(origin, rotation);
    let corners = [
        f(ink.min),
        f([ink.max[0], ink.min[1]]),
        f(ink.max),
        f([ink.min[0], ink.max[1]]),
    ];
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for c in corners {
        b = [
            b[0].min(c[0]),
            b[1].min(c[1]),
            b[2].max(c[0]),
            b[3].max(c[1]),
        ];
    }
    b
}

fn arrow(tip: P2, from: P2) -> [P2; 2] {
    let d = [from[0] - tip[0], from[1] - tip[1]];
    let n = d[0].hypot(d[1]).max(1e-12);
    [tip, [tip[0] + ARROW * d[0] / n, tip[1] + ARROW * d[1] / n]]
}

/// A strip beside a view's outline, where its annotations stack outward in lanes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    Below,
    Above,
    Left,
    Right,
}

/// The strips draftwright reserves (`layout::Strips`): a view's horizontal dimensions below it,
/// except the plan's, which go above it (its x-location tiers); vertical ones right of the front
/// and the side, and left of the plan (the front/plan column's left band). Callouts take the
/// outermost lanes: below the front, above the plan and the side.
fn strip(view: &str, horizontal: bool) -> Side {
    match (view, horizontal) {
        ("plan", true) => Side::Above,
        (_, true) => Side::Below,
        ("plan", false) => Side::Left,
        _ => Side::Right,
    }
}

fn callout_strip(view: &str) -> Side {
    if view == "front" {
        Side::Below
    } else {
        Side::Above
    }
}

/// The page position of lane *lane* in strip *side*, and the outward sign.
fn lane_at(o: [f64; 4], side: Side, lane: usize) -> (f64, f64) {
    let d = FIRST + lane as f64 * LANE;
    match side {
        Side::Below => (o[1] - d, -1.0),
        Side::Above => (o[3] + d, 1.0),
        Side::Left => (o[0] - d, -1.0),
        Side::Right => (o[2] + d, 1.0),
    }
}

/// Whether world axis *axis* runs across the page in this view (else up it); none when the
/// view looks along it.
fn across(frame: &Frame, axis: usize) -> Option<bool> {
    let mut e = [0.0; 3];
    e[axis] = 1.0;
    let u = frame.view.map(e);
    if u[0].abs() > 0.5 {
        Some(true)
    } else if u[1].abs() > 0.5 {
        Some(false)
    } else {
        None
    }
}

/// Places a linear dimension in lane *lane* of strip *side* of its view.
fn place_linear(
    frame: &Frame,
    shape: &Shape,
    terms: &[String],
    side: Side,
    lane: usize,
) -> Option<Placed> {
    let Shape::Linear {
        a,
        b,
        a_edge,
        b_edge,
        ..
    } = shape
    else {
        return None;
    };
    let horizontal = matches!(side, Side::Below | Side::Above);
    let ink = label_ink(terms, font());
    let w = ink.max[0] - ink.min[0];
    let o = frame.outline;
    let (pa, pb) = (frame.page(*a), frame.page(*b));
    let (t_line, out) = lane_at(o, side, lane);
    // Work along the line (s) and across it (t), then map to the page.
    let k = usize::from(horizontal); // the page coordinate across the line
    let map = |s: f64, t: f64| if horizontal { [s, t] } else { [t, s] };
    let edge = match side {
        Side::Below => o[1],
        Side::Above => o[3],
        Side::Left => o[0],
        Side::Right => o[2],
    };
    let mut lines = Vec::new();
    let (s0, s1) = (pa[1 - k], pb[1 - k]);
    for (p, s, on_edge) in [(pa, s0, *a_edge), (pb, s1, *b_edge)] {
        let from = if on_edge { edge } else { p[k] };
        lines.push([map(s, from + out * GAP), map(s, t_line + out * OVERSHOOT)]);
    }
    let (lo, hi) = (s0.min(s1), s0.max(s1));
    let mut arrows = Vec::new();
    let mid = if hi - lo >= w + 2.0 * ARROW + 2.0 {
        lines.push([map(lo, t_line), map(hi, t_line)]);
        arrows.push(arrow(map(lo, t_line), map(lo + 1.0, t_line)));
        arrows.push(arrow(map(hi, t_line), map(hi - 1.0, t_line)));
        0.5 * (lo + hi) - 0.5 * w - ink.min[0]
    } else {
        // Too short for the label between the arrows: the arrows point in from outside, and the
        // label sits beyond the far end.
        lines.push([map(lo - ARROW - 2.0, t_line), map(hi + ARROW + 2.0, t_line)]);
        arrows.push(arrow(map(lo, t_line), map(lo - 1.0, t_line)));
        arrows.push(arrow(map(hi, t_line), map(hi + 1.0, t_line)));
        hi + ARROW + 3.0 - ink.min[0]
    };
    // The label reads from below or from the right, above its line (ISO 129-1, aligned).
    let (label_at, rotation) = if horizontal {
        ([mid, t_line + LIFT], 0.0)
    } else {
        ([t_line - LIFT, mid], 90.0)
    };
    Some(Placed {
        lines,
        arrows,
        label_at,
        rotation,
        label_box: label_box(&ink, label_at, rotation),
    })
}

/// Places a callout in lane *lane* of strip *side* (above or below its view), with a 45° leader
/// to the site nearest the strip's right-hand end.
fn place_callout(
    frame: &Frame,
    sites: &[(P3, f64)],
    terms: &[String],
    side: Side,
    lane: usize,
) -> Placed {
    let up = side == Side::Above;
    let tip = callout_tip(frame, sites, up);
    let (y, _) = lane_at(frame.outline, side, lane);
    let elbow = [tip[0] + (y - tip[1]).abs(), y];
    let end = [elbow[0] + SHOULDER, y];
    let ink = label_ink(terms, font());
    let label_at = [end[0] + 1.0 - ink.min[0], y - 0.5 * CAP];
    Placed {
        lines: vec![[tip, elbow], [elbow, end]],
        arrows: vec![arrow(tip, elbow)],
        label_at,
        rotation: 0.0,
        label_box: label_box(&ink, label_at, 0.0),
    }
}

/// The leader's tip: on the rim, at 45° towards the strip and right, of the site nearest the
/// strip's right-hand end.
fn callout_tip(frame: &Frame, sites: &[(P3, f64)], up: bool) -> P2 {
    let r = std::f64::consts::FRAC_1_SQRT_2;
    let v = if up { 1.0 } else { -1.0 };
    sites
        .iter()
        .map(|(c, radius)| {
            let p = frame.page(*c);
            let rr = frame.s * radius;
            ([p[0] + r * rr, p[1] + v * r * rr], p[0] + v * p[1])
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a callout has a site")
        .0
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

/// The planned dimensions under *spec*, placed around *views* on *sheet*, with what the spec
/// left out and the problems found. A spec naming a dimension the part does not have is refused
/// with the nearest IDs.
pub fn plan(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) -> Result<Planned, Invalid> {
    let (mut wants, mut problems) = wants(analysis);
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            NAMES
                .contains(&name)
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    // The view that carries each: its first choice that is shown and does not look along it
    // (a turned part's diameter turns to the radial axis the view shows). A later choice renames
    // it. Those with none keep the views that were shown but look along them.
    let [lo, hi] = analysis.envelope;
    let mut chosen: Vec<Option<&'static str>> = Vec::new();
    let mut end_on: Vec<Vec<&'static str>> = Vec::new();
    for w in &mut wants {
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
    let mut omitted = Vec::new();
    let mut kept = Vec::new();
    for ((w, view), along) in wants.into_iter().zip(chosen).zip(end_on) {
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

    // Lanes: per view and strip, sizes nearest, then locations by distance, overall outermost
    // (ARCH0009-5), then callouts by their tips, right to left.
    let mut strips: Lanes = BTreeMap::new();
    let mut sides: Vec<Option<Side>> = vec![None; kept.len()];
    for (i, (w, _, view, _)) in kept.iter().enumerate() {
        let Some(view) = *view else { continue };
        let f = &frames[view];
        let (side, rank) = match &w.shape {
            Shape::Linear {
                a, b, axis, order, ..
            } => {
                let Some(horizontal) = across(f, *axis) else {
                    continue;
                };
                (
                    strip(view, horizontal),
                    (*order, key((b[*axis] - a[*axis]).abs())),
                )
            }
            Shape::Callout { sites } => {
                let side = callout_strip(view);
                let tip = callout_tip(f, sites, side == Side::Above);
                (side, (3, -key(tip[0])))
            }
        };
        sides[i] = Some(side);
        strips
            .entry((view, side))
            .or_default()
            .push((rank.0, rank.1, i));
    }
    let mut lane = vec![0usize; kept.len()];
    for list in strips.values_mut() {
        list.sort();
        for (n, (_, _, i)) in list.iter().enumerate() {
            lane[*i] = n;
        }
    }

    // Undrawn items by the views that could carry them and those of them that look along them.
    type Undrawn = (Vec<&'static str>, Vec<&'static str>);
    let mut missing: BTreeMap<Undrawn, Vec<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for (i, (w, default, chosen, along)) in kept.into_iter().enumerate() {
        let view = chosen.unwrap_or(w.views[0]);
        let placed = chosen.zip(sides[i]).and_then(|(v, side)| {
            let f = &frames[v];
            match &w.shape {
                Shape::Linear { .. } => place_linear(f, &w.shape, &w.terms, side, lane[i]),
                Shape::Callout { sites } => Some(place_callout(f, sites, &w.terms, side, lane[i])),
            }
        });
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
            default,
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
    problems.extend(collisions(analysis, &out, sheet, views));
    Ok((out, omitted, problems))
}

fn boxes_overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

/// Whether segment *s* passes through the open box *b*.
fn crosses(s: [P2; 2], b: [f64; 4]) -> bool {
    // Liang–Barsky clipping against the box.
    let (p, q) = (s[0], s[1]);
    let d = [q[0] - p[0], q[1] - p[1]];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (num, den) in [
        (p[0] - b[0], -d[0]),
        (b[2] - p[0], d[0]),
        (p[1] - b[1], -d[1]),
        (b[3] - p[1], d[1]),
    ] {
        if den == 0.0 {
            if num <= 0.0 {
                return false;
            }
        } else {
            let t = num / den;
            if den < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t1 - t0 > 1e-9
}

fn inside(p: P2, b: [f64; 4]) -> bool {
    p[0] >= b[0] - 1e-9 && p[0] <= b[2] + 1e-9 && p[1] >= b[1] - 1e-9 && p[1] <= b[3] + 1e-9
}

/// What the simple placement collides with: each a problem on the dimension's ID.
fn collisions(
    analysis: &Analysis,
    dims: &[PlannedDimension],
    sheet: &PlannedSheet,
    views: &[PlannedView],
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let [w, h] = sheet.page;
    let drawable = [MARGIN, MARGIN, w - MARGIN, h - MARGIN];
    let outlines: Vec<(&str, [f64; 4])> = views
        .iter()
        .map(|v| {
            let b = if v.id == "v:iso" {
                let ib = analysis.iso_bounds;
                [
                    v.at[0] + v.scale * ib[0],
                    v.at[1] + v.scale * ib[1],
                    v.at[0] + v.scale * ib[2],
                    v.at[1] + v.scale * ib[3],
                ]
            } else {
                Frame::new(analysis, v).outline
            };
            (v.id.as_str(), b)
        })
        .collect();
    let placed: Vec<(&PlannedDimension, &Placed)> = dims
        .iter()
        .filter_map(|d| d.placed.as_ref().map(|p| (d, p)))
        .collect();
    let mut problem = |code: &str, severity: &str, subjects: Vec<String>, message: String| {
        problems.push(Problem {
            id: format!("p:{code}:{}", subjects.join("|")),
            code: code.into(),
            severity: severity.into(),
            message,
            subjects,
        });
    };
    for (d, p) in &placed {
        let lb = p.label_box;
        let ends = p
            .lines
            .iter()
            .chain(&p.arrows)
            .flat_map(|s| s.iter().copied());
        if !inside([lb[0], lb[1]], drawable)
            || !inside([lb[2], lb[3]], drawable)
            || ends.clone().any(|e| !inside(e, drawable))
        {
            problem(
                "annotation_off_sheet",
                "error",
                vec![d.id.clone()],
                format!("{} ({}) runs off the sheet's drawing area.", d.id, d.label),
            );
        }
        let tb = sheet.title_block;
        if boxes_overlap(lb, tb) || p.lines.iter().any(|s| crosses(*s, tb)) {
            problem(
                "annotation_over_title_block",
                "warning",
                vec![d.id.clone()],
                format!("{} ({}) runs into the title block.", d.id, d.label),
            );
        }
        for (view, b) in &outlines {
            if boxes_overlap(lb, *b) {
                problem(
                    "annotation_over_view",
                    "warning",
                    vec![d.id.clone(), view.to_string()],
                    format!(
                        "The label of {} ({}) lies over the {view} view.",
                        d.id, d.label
                    ),
                );
            }
        }
    }
    for (i, (a, pa)) in placed.iter().enumerate() {
        for (b, pb) in &placed[i + 1..] {
            if boxes_overlap(pa.label_box, pb.label_box) {
                problem(
                    "annotation_overlap",
                    "warning",
                    vec![a.id.clone(), b.id.clone()],
                    format!(
                        "The labels of {} ({}) and {} ({}) overlap.",
                        a.id, a.label, b.id, b.label
                    ),
                );
            }
        }
    }
    for (a, pa) in &placed {
        for (b, pb) in &placed {
            if a.id == b.id {
                continue;
            }
            if pa.lines.iter().any(|s| crosses(*s, pb.label_box)) {
                problem(
                    "annotation_ink_overlap",
                    "warning",
                    vec![a.id.clone(), b.id.clone()],
                    format!(
                        "A line of {} ({}) runs through the label of {} ({}).",
                        a.id, a.label, b.id, b.label
                    ),
                );
            }
        }
    }
    problems
}

/// Draws the planned dimensions: lines, arrowheads and labels into *items* (the dims layer), and
/// each label's text runs into *text*. Everything comes from the plan; nothing is measured or
/// formatted here (R1 invariant 2).
///
/// Labels are drawn with the same bundled font placement measured them with ([`FONT`]), so the
/// label boxes the collision problems describe are the ink drawn. `⌀` is drawn as the font's `ø`,
/// and its text run is `ø` too: the searchable text is what is printed. `↧`, `⌴` and `⌵` are
/// strokes with no text run.
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
        for (run, x) in &ink.runs {
            text.push(TextRun {
                text: run.clone(),
                at: f([*x, 0.0]),
                size: TEXT,
                rotation: p.rotation,
                font: FONT.into(),
                h_align: HAlign::Left,
                v_align: VAlign::Baseline,
            });
        }
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

/// Where a feature or pattern sits in the part's frame: a hole's point on its axis, a pattern's
/// centre (its members' mean), for comparing with draftwright's feature keys.
pub fn feature_origin(analysis: &Analysis, id: &str) -> Option<P3> {
    let frame = QFrame(analysis.recognition.as_ref()?.frame);
    if let Some(f) = analysis.features.iter().find(|f| f.id == id) {
        return point(f.parameters.get("location")).map(|l| frame.point(l));
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
