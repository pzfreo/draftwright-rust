//! Where the planned dimensions go on the page, and what then collides: linear dimensions and
//! callouts stacked in lanes beside their view, each collision a problem on the dimension's ID.

use std::collections::BTreeMap;

use haecceity::hlr::View;

use super::{
    ARROW, CAP, FIRST, GAP, LANE, LIFT, OVERSHOOT, P2, P3, Placed, PlannedDimension, SHOULDER,
    Shape, TEXT, TOKEN_GAP, Want, font, key,
};
use crate::layout::MARGIN;
use crate::model::{Analysis, PlannedSheet, PlannedView, Problem};
use crate::text::{Font, Segment};

/// Strip lanes: (rank, sub-rank, item index), per view and strip.
type Lanes<'a> = BTreeMap<(&'a str, Side), Vec<(u8, i64, usize)>>;

/// A view as the dimensions see it: world to page, and the part's box on the page.
pub(super) struct Frame {
    view: View,
    at: P2,
    s: f64,
    centre: P2,
    /// The projected envelope's box on the page.
    outline: [f64; 4],
}

impl Frame {
    pub(super) fn new(analysis: &Analysis, v: &PlannedView) -> Frame {
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

pub(super) fn moved(s: &Segment, f: &dyn Fn(P2) -> P2) -> Segment {
    match *s {
        Segment::Line(a, b) => Segment::Line(f(a), f(b)),
        Segment::Quad(a, b, c) => Segment::Quad(f(a), f(b), f(c)),
        Segment::Cubic(a, b, c, d) => Segment::Cubic(f(a), f(b), f(c), f(d)),
    }
}

/// A label's frame on the page: its baseline origin and direction.
pub(super) fn to_page(origin: P2, rotation: f64) -> impl Fn(P2) -> P2 {
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
pub(super) enum Side {
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
pub(super) fn across(frame: &Frame, axis: usize) -> Option<bool> {
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

/// Each item's strip and lane in its carrying view (none when it has no view, or its view looks
/// along it): per view and strip, sizes nearest, then locations by distance, overall outermost
/// (ARCH0009-5), then callouts by their tips, right to left.
pub(super) fn lanes<'a>(
    frames: &BTreeMap<&str, Frame>,
    items: impl ExactSizeIterator<Item = (&'a Want, Option<&'static str>)>,
) -> (Vec<Option<Side>>, Vec<usize>) {
    let n = items.len();
    let mut strips: Lanes = BTreeMap::new();
    let mut sides: Vec<Option<Side>> = vec![None; n];
    for (i, (w, view)) in items.enumerate() {
        let Some(view) = view else { continue };
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
    let mut lane = vec![0usize; n];
    for list in strips.values_mut() {
        list.sort();
        for (n, (_, _, i)) in list.iter().enumerate() {
            lane[*i] = n;
        }
    }
    (sides, lane)
}

/// Places *w* in lane *lane* of strip *side* of its view.
pub(super) fn place(frame: &Frame, w: &Want, side: Side, lane: usize) -> Option<Placed> {
    match &w.shape {
        Shape::Linear { .. } => place_linear(frame, &w.shape, &w.terms, side, lane),
        Shape::Callout { sites } => Some(place_callout(frame, sites, &w.terms, side, lane)),
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
pub(super) fn collisions(
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
