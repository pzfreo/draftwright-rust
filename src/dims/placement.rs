//! Where the planned dimensions go on the page, and what then collides: linear dimensions and
//! callouts stacked in lanes beside their view, each collision a problem on the dimension's ID.

use std::collections::BTreeMap;

use haecceity::hlr::View;

use super::{
    ARROW, ARROW_HALF, CAP, FIRST, GAP, LANE, LIFT, OVERSHOOT, P2, P3, Placed, PlannedDimension,
    SHOULDER, Shape, TEXT, TOKEN_GAP, Want, font, key,
};
use crate::layout::{Lanes, MARGIN};
use crate::model::{Analysis, PlannedSheet, PlannedView, Problem};
use crate::text::{Font, Segment};

/// Strip lanes: (rank, sub-rank, item index), per view and strip.
type Strips<'a> = BTreeMap<(&'a str, Side), Vec<(u8, i64, usize)>>;

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
        Frame::looking(analysis, v.direction, v.up, v.at, v.scale)
    }

    /// The view looking along *direction* with *up*, the part's box centred at *at*, at scale *s*.
    pub(super) fn looking(analysis: &Analysis, direction: P3, up: P3, at: P2, s: f64) -> Frame {
        let view = View::new(direction, up).expect("a planned view");
        let (size, centre) = crate::model::extent(analysis, &view);
        Frame {
            view,
            at,
            s,
            centre,
            outline: [
                at[0] - 0.5 * s * size[0],
                at[1] - 0.5 * s * size[1],
                at[0] + 0.5 * s * size[0],
                at[1] + 0.5 * s * size[1],
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
    let mut strips: Strips = BTreeMap::new();
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
                let (tip, _) = callout_tip(f, sites, side == Side::Above);
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

/// The lanes each strip holds (one more than its outermost item's lane), from each item's view,
/// strip and lane, as the layout's bands name them. A strip [`strip`] and [`callout_strip`] never
/// give (above the front, below the plan, left of the front or the side, right of the plan) is a
/// bug.
pub(super) fn band_lanes<'a>(
    items: impl Iterator<Item = ((&'a Option<&'static str>, Option<Side>), usize)>,
) -> Lanes {
    let mut out = Lanes::default();
    for ((view, side), lane) in items {
        let (Some(view), Some(side)) = (view, side) else {
            continue;
        };
        let band = match (*view, side) {
            ("front", Side::Below) => &mut out.front_below,
            ("front", Side::Right) => &mut out.front_right,
            ("plan", Side::Above) => &mut out.plan_above,
            ("plan", Side::Left) => &mut out.plan_left,
            ("side", Side::Below) => &mut out.side_below,
            ("side", Side::Above) => &mut out.side_above,
            ("side", Side::Right) => &mut out.side_right,
            other => unreachable!("no band holds {other:?}"),
        };
        *band = (*band).max(lane + 1);
    }
    out
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

/// The step a label slides by along its dimension line, mm; the clearance it keeps from the
/// arrowheads at the line's ends; and how far past the line's end a label beyond it may go.
const SLIDE_STEP: f64 = 0.5;
const SLIDE_CLEAR: f64 = 0.5;
const SLIDE_REACH: f64 = 20.0;

/// Moves labels off other items' ink: an item whose label another item's line crosses (an outer
/// lane's extension line, a leader) or meets another label, or whose leader crosses another
/// label, takes the nearest place clear of both. A linear dimension's label moves along its own
/// line: between its arrows it slides between them, right (or up) before left; beyond the line's
/// end (a span too short for it) it tries the other end, then further out from either, the line
/// running on to it, up to [`SLIDE_REACH`]. A callout's leader may turn the other way: 45° up or
/// down to the left, its label left of the shoulder. With no clear place an item stays where it
/// was, and the collision is reported. *items* are every item the part would have, in plan order,
/// so where a label settles does not depend on what the spec omits (ARCH0003-2); *rims* gives
/// each callout's site radius on the page ([`callout_rim`]) and is none for a linear dimension.
pub(super) fn settle(
    items: &mut [Option<Placed>],
    rims: &[Option<f64>],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) {
    let drawable = drawable(sheet);
    // What a moved label must keep off as well: the views and the title block.
    let mut keep_off: Vec<[f64; 4]> = outlines(analysis, views).into_iter().map(|o| o.1).collect();
    keep_off.push(sheet.title_block);
    for i in 0..items.len() {
        let Some(p) = items[i].as_ref() else {
            continue;
        };
        let linear = rims[i].is_none();
        let candidates = match rims[i] {
            None => slides(p),
            Some(rim) => vec![turned(p, rim)],
        };
        let others = || {
            items
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .filter_map(|(_, q)| q.as_ref())
        };
        // A linear dimension's own lines are fixed by what it measures, so only a line run on to
        // its label must clear other labels; a callout's leader and shoulder are its choice.
        let clear = |c: &Placed| {
            let own: Vec<&[P2; 2]> = c
                .lines
                .iter()
                .filter(|l| !linear || !p.lines.contains(l))
                .collect();
            others().all(|q| {
                !q.lines.iter().any(|l| crosses(*l, c.label_box))
                    && !boxes_overlap(c.label_box, q.label_box)
                    && !own.iter().any(|l| crosses(**l, q.label_box))
            })
        };
        if clear(p) {
            continue;
        }
        // A new place stays on the sheet (unless the item was off it already, which stays
        // reported), its label off the views and the title block, and its lines out of the
        // title block.
        let on_sheet = |c: &Placed| ink_points(c).all(|q| inside(q, drawable));
        let off_already = !on_sheet(p);
        let fits = |c: &Placed| {
            (off_already || on_sheet(c))
                && keep_off.iter().all(|o| !boxes_overlap(c.label_box, *o))
                && !c.lines.iter().any(|l| crosses(*l, sheet.title_block))
        };
        if let Some(c) = candidates.into_iter().find(|c| fits(c) && clear(c)) {
            items[i] = Some(c);
        }
    }
}

/// A linear dimension's other label places along its line, nearest first (see [`settle`]).
fn slides(p: &Placed) -> Vec<Placed> {
    // Along the line: x for a horizontal dimension, y for a vertical one.
    let k = usize::from(p.rotation != 0.0);
    let Some(&line) = p.lines.get(2) else {
        return Vec::new();
    };
    let (l0, l1) = (line[0][k].min(line[1][k]), line[0][k].max(line[1][k]));
    let b = p.label_box;
    let width = b[k + 2] - b[k];
    // Each a shift of the label and the line's new extent.
    let mut shifts: Vec<(f64, [f64; 2])> = Vec::new();
    let (lo, hi) = (l0 + ARROW + SLIDE_CLEAR, l1 - ARROW - SLIDE_CLEAR);
    if b[k] >= lo && b[k + 2] <= hi {
        let steps = ((hi - lo - width) / SLIDE_STEP).ceil() as i64;
        for d in (1..=steps)
            .flat_map(|n| [n, -n])
            .map(|n| n as f64 * SLIDE_STEP)
        {
            if b[k] + d >= lo && b[k + 2] + d <= hi {
                shifts.push((d, [l0, l1]));
            }
        }
    } else if b[k] > l1 {
        // Beyond the far end, 1 mm past the line; the near end mirrors it.
        let back = (l0 - 1.0 - width) - b[k];
        for n in 0..=(SLIDE_REACH / SLIDE_STEP) as i64 {
            let d = n as f64 * SLIDE_STEP;
            if n > 0 {
                shifts.push((d, [l0, b[k] + d - 1.0]));
            }
            shifts.push((back - d, [b[k + 2] + back - d + 1.0, l1]));
        }
    }
    shifts
        .into_iter()
        .map(|(d, extent)| {
            let mut c = p.clone();
            c.label_box[k] += d;
            c.label_box[k + 2] += d;
            c.label_at[k] += d;
            c.lines[2][0][k] = extent[0];
            c.lines[2][1][k] = extent[1];
            c
        })
        .collect()
}

/// A callout with its leader turned the other way: mirrored about the vertical through its site's
/// centre, so the tip moves to the rim's point at 45° to the left and the leader still runs
/// radially from it (a leader tangent to the rim would not point at the hole); the shoulder and
/// label to its left. *rim* is the site's radius on the page.
fn turned(p: &Placed, rim: f64) -> Placed {
    let [tip, elbow] = p.lines[0];
    let axis = tip[0] - std::f64::consts::FRAC_1_SQRT_2 * rim;
    let tip = [2.0 * axis - tip[0], tip[1]];
    let elbow = [2.0 * axis - elbow[0], elbow[1]];
    let end = [elbow[0] - SHOULDER, elbow[1]];
    let b = p.label_box;
    let shift = (end[0] - 1.0) - b[2];
    Placed {
        lines: vec![[tip, elbow], [elbow, end]],
        arrows: vec![arrow(tip, elbow)],
        label_at: [p.label_at[0] + shift, p.label_at[1]],
        rotation: p.rotation,
        label_box: [b[0] + shift, b[1], b[2] + shift, b[3]],
    }
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
    let (tip, _) = callout_tip(frame, sites, up);
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
/// strip's right-hand end; and that site's radius on the page.
fn callout_tip(frame: &Frame, sites: &[(P3, f64)], up: bool) -> (P2, f64) {
    let r = std::f64::consts::FRAC_1_SQRT_2;
    let v = if up { 1.0 } else { -1.0 };
    sites
        .iter()
        .map(|(c, radius)| {
            let p = frame.page(*c);
            let rr = frame.s * radius;
            (([p[0] + r * rr, p[1] + v * r * rr], rr), p[0] + v * p[1])
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a callout has a site")
        .0
}

/// The page radius of the site a callout in strip *side* points to (see [`callout_tip`]), which
/// [`settle`] needs to turn its leader.
pub(super) fn callout_rim(frame: &Frame, sites: &[(P3, f64)], side: Side) -> f64 {
    callout_tip(frame, sites, side == Side::Above).1
}
/// Every extreme point of a placed item's ink: its lines' ends, its arrowheads' corners (as
/// `dims::render` draws them) and its label box's corners.
fn ink_points(p: &Placed) -> impl Iterator<Item = P2> + '_ {
    let heads = p.arrows.iter().flat_map(|[tip, tail]| {
        let n = [tail[1] - tip[1], tip[0] - tail[0]];
        let k = ARROW_HALF / n[0].hypot(n[1]).max(1e-12);
        [
            *tip,
            [tail[0] + k * n[0], tail[1] + k * n[1]],
            [tail[0] - k * n[0], tail[1] - k * n[1]],
        ]
    });
    let b = p.label_box;
    p.lines
        .iter()
        .flat_map(|l| l.iter().copied())
        .chain(heads)
        .chain([[b[0], b[1]], [b[2], b[1]], [b[0], b[3]], [b[2], b[3]]])
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

/// The sheet's drawing area, inside its margin.
fn drawable(sheet: &PlannedSheet) -> [f64; 4] {
    let [w, h] = sheet.page;
    [MARGIN, MARGIN, w - MARGIN, h - MARGIN]
}

/// Each view's drawn box on the page: a principal view's projected envelope, the isometric's
/// projected bounds.
fn outlines<'a>(analysis: &Analysis, views: &'a [PlannedView]) -> Vec<(&'a str, [f64; 4])> {
    views
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
        .collect()
}

/// What the simple placement collides with: each a problem on the dimension's ID.
pub(super) fn collisions(
    analysis: &Analysis,
    dims: &[PlannedDimension],
    sheet: &PlannedSheet,
    views: &[PlannedView],
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let drawable = drawable(sheet);
    let outlines = outlines(analysis, views);
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
        // Its ink past its view's reserved area (the box and its bands).
        if let Some(v) = views.iter().find(|v| v.id == d.view) {
            let a = v.area;
            let past = ink_points(p)
                .map(|q| {
                    (a[0] - q[0])
                        .max(q[0] - a[2])
                        .max(a[1] - q[1])
                        .max(q[1] - a[3])
                })
                .fold(0.0, f64::max);
            if past > 1e-6 {
                problem(
                    "annotation_outside_area",
                    "info",
                    vec![d.id.clone(), d.view.clone()],
                    format!(
                        "{} ({}) runs up to {past:.1} mm past the area reserved for the {} view and its bands; it may crowd what lies beyond.",
                        d.id, d.label, d.view
                    ),
                );
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A turned leader still points at the hole: its tip is on the rim and the leader runs
    /// along the radius through it, never tangent to the rim (review of d-placement).
    #[test]
    fn a_turned_leader_stays_radial() {
        let (centre, rim) = ([50.0, 40.0], 6.0);
        for up in [1.0, -1.0] {
            let r = std::f64::consts::FRAC_1_SQRT_2 * rim;
            let tip = [centre[0] + r, centre[1] + up * r];
            let elbow = [tip[0] + 12.0, tip[1] + up * 12.0];
            let end = [elbow[0] + SHOULDER, elbow[1]];
            let p = Placed {
                lines: vec![[tip, elbow], [elbow, end]],
                arrows: vec![arrow(tip, elbow)],
                label_at: [end[0] + 1.0, elbow[1]],
                rotation: 0.0,
                label_box: [end[0] + 1.0, elbow[1] - 1.0, end[0] + 20.0, elbow[1] + 2.0],
            };
            let t = turned(&p, rim);
            let [tip, elbow] = t.lines[0];
            let radius = [tip[0] - centre[0], tip[1] - centre[1]];
            assert!(
                (radius[0].hypot(radius[1]) - rim).abs() < 1e-9,
                "tip on the rim"
            );
            assert!(radius[0] < 0.0, "tip on the left of the rim");
            let leader = [elbow[0] - tip[0], elbow[1] - tip[1]];
            let cross = radius[0] * leader[1] - radius[1] * leader[0];
            let dot = radius[0] * leader[0] + radius[1] * leader[1];
            assert!(
                cross.abs() < 1e-9 && dot > 0.0,
                "leader runs out along the radius"
            );
            assert!(t.label_box[2] < elbow[0], "label left of the shoulder");
        }
    }
}
