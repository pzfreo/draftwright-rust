//! Where the planned dimensions go on the page, and what then collides: linear dimensions and
//! callouts stacked in lanes beside their view, each collision a problem on the dimension's ID.

use std::collections::BTreeMap;

use haecceity::hlr::View;

use super::holes::SPOTFACE;
use super::{
    ARROW, ARROW_HALF, CAP, FIRST, GAP, LANE, LIFT, OVERSHOOT, P2, P3, Placed, PlannedDimension,
    Shape, TEXT, TOKEN_GAP, Want, font, key,
};
use crate::layout::{Lanes, MARGIN};
use crate::model::{Analysis, PlannedSheet, PlannedView, Problem};
use crate::text::{Font, Segment};

/// A held lane an item may share: its strip, the lane, and the item's extent along it there.
type Shared = (Side, usize, [f64; 2]);

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

    /// The projected envelope's box on the page.
    pub(super) fn outline(&self) -> [f64; 4] {
        self.outline
    }

    /// The view's scale, page mm per world mm.
    pub(super) fn scale(&self) -> f64 {
        self.s
    }

    pub(super) fn page(&self, p: P3) -> P2 {
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
/// one text height (`⌀` as the font's `ø`, the others drawn), each token a gap apart. The
/// spotface symbol (the term [`SPOTFACE`], ASME Y14.5-2018's counterbore symbol with SF inside)
/// is the counterbore's strokes widened to hold the font's own `SF`, in a cell of [`SF_CELL`].
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
            _ if t == SPOTFACE => tokens.push((Some('⌴'), "SF".into())),
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
            Some('⌴') if text == "SF" => {
                let (l, r) = (x + 0.15 * h, x + SF_CELL - 0.15 * h);
                let strokes = [
                    [[l, CAP], [l, 0.0]],
                    [[l, 0.0], [r, 0.0]],
                    [[r, 0.0], [r, CAP]],
                ];
                for s in &strokes {
                    grow(&mut ink, s[0], s[1]);
                }
                ink.strokes.extend(strokes);
                let at = x + 0.5 * (SF_CELL - font.advance(text, SF_TEXT));
                let laid = font.baseline(text, SF_TEXT);
                grow(
                    &mut ink,
                    [at + laid.min[0], SF_RAISE + laid.min[1]],
                    [at + laid.max[0], SF_RAISE + laid.max[1]],
                );
                let shift = |p: P2| [p[0] + at, p[1] + SF_RAISE];
                ink.glyphs.push(
                    laid.contours
                        .into_iter()
                        .map(|c| c.into_iter().map(|s| moved(&s, &shift)).collect())
                        .collect(),
                );
                x += SF_CELL;
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

/// The spotface symbol's cell: the counterbore's box widened from 0.7 to 1.1 text heights so the
/// letters fit inside with a margin.
const SF_CELL: f64 = 1.4 * TEXT;
/// The height the spotface symbol's `SF` is set at, and its baseline above the box's floor: its
/// capitals (0.7 of the size in the bundled mono font) stand clear of the floor and below the
/// box's sides.
const SF_TEXT: f64 = 0.6 * TEXT;
const SF_RAISE: f64 = 0.12 * TEXT;

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

pub(super) fn label_box(ink: &LabelInk, origin: P2, rotation: f64) -> [f64; 4] {
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

pub(super) fn arrow(tip: P2, from: P2) -> [P2; 2] {
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

/// Exterior callout elbow rows, in paper mm; keys are the four Spec side names.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LabelRows {
    pub outline: [f64; 4],
    pub first_row: BTreeMap<String, f64>,
    pub row_spacing: BTreeMap<String, f64>,
}

fn callout_step(side: Side, width: f64, canonical_width: f64) -> f64 {
    if matches!(side, Side::Left | Side::Right) {
        canonical_width.max(width) + super::SHOULDER + LANE
    } else {
        LANE
    }
}

/// Rows for a view and the greater of its canonical and edited callout widths.
pub fn label_rows(analysis: &Analysis, view: &PlannedView, width: f64) -> LabelRows {
    let outline = Frame::new(analysis, view).outline();
    let sides = [
        ("above", Side::Above),
        ("below", Side::Below),
        ("left", Side::Left),
        ("right", Side::Right),
    ];
    LabelRows {
        outline,
        first_row: sides
            .iter()
            .map(|(name, side)| (name.to_string(), lane_at(outline, *side, 0).0))
            .collect(),
        row_spacing: sides
            .iter()
            .map(|(name, side)| (name.to_string(), callout_step(*side, width, width)))
            .collect(),
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

/// A plate thickness's default strip: Python's fixed axis map, with the front's vertical walls
/// on its left and the side's horizontal walls above it.
pub(super) fn thickness_strip(view: &str, axis: usize, horizontal: bool) -> Side {
    match (view, axis) {
        ("front", 2) => Side::Left,
        ("side", 1) => Side::Above,
        _ => strip(view, horizontal),
    }
}

/// How near (page mm) a point must be to the view's silhouette to lie on it.
pub(super) const ON_SILHOUETTE: f64 = 0.05;

/// How far (page mm) a plate thickness's witnesses may run over the part before leaving it at
/// the silhouette, when no pair of its solid edges reaches the silhouette in any view: about an
/// arrowhead, as where a flange ends just short of a chamfer that forms the outline.
pub(super) const NEAR_SILHOUETTE: f64 = 3.0;

/// The view's silhouette edge nearest page point *p* across a dimension that runs horizontally
/// (*horizontal*) or vertically, and the distance to it: the strip whose extension lines leave
/// the part there.
pub(super) fn silhouette_gap(frame: &Frame, p: P3, horizontal: bool) -> (Side, f64) {
    let o = frame.outline();
    let c = usize::from(horizontal);
    let q = frame.page(p)[c];
    let (low, high) = if horizontal {
        (Side::Below, Side::Above)
    } else {
        (Side::Left, Side::Right)
    };
    let (dl, dh) = ((q - o[c]).abs(), (q - o[c + 2]).abs());
    if dl <= dh { (low, dl) } else { (high, dh) }
}

/// The strip a plate thickness takes: where its witnesses meet the silhouette when they are
/// anchored on solid profile edges ([`super::plates::prefer_readable`]), else its default strip.
fn plate_strip(frame: &Frame, view: &str, shape: &Shape, horizontal: bool) -> Side {
    match shape {
        Shape::Linear {
            a,
            axis,
            a_edge,
            b_edge,
            ..
        } => (!a_edge && !b_edge)
            .then(|| silhouette_gap(frame, *a, horizontal))
            .filter(|(_, d)| *d <= NEAR_SILHOUETTE)
            .map_or_else(
                || thickness_strip(view, *axis, horizontal),
                |(side, _)| side,
            ),
        _ => strip(view, horizontal),
    }
}

/// For each item, the item whose lane it shares, when the defaults merge it away: one span, one
/// dimension (ARCH0009-6; [`super::bosses::consolidate`], [`super::holes::consolidate`],
/// [`super::share`]) leaves one dimension on a span that several items measure, so the others
/// take no lane of their own (draftwright's corridor dedup: a displaced duplicate is never
/// placed while its winner is). The merges are decided among every item the part would have,
/// under the defaults' labels, so no spec edit changes them (ARCH0003-2): an item the spec brings
/// back by omitting its survivor is drawn in the survivor's lane, where the span already was.
/// Counted walls on different projected axes keep a parallel strip of their own.
pub(super) fn owners(
    analysis: &Analysis,
    items: &[Want],
    views_of: &[Option<&'static str>],
) -> Vec<Option<usize>> {
    let mut kept: Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)> = items
        .iter()
        .zip(views_of)
        .map(|(w, view)| (w.clone(), true, *view, Vec::new()))
        .collect();
    let mut given = vec![Vec::new(); kept.len()];
    let mut omitted = Vec::new();
    super::plates::consolidate(analysis, &mut kept, &mut omitted);
    super::bosses::consolidate(analysis, &mut kept, &mut omitted);
    super::holes::consolidate(analysis, &mut kept, &mut given, &mut omitted);
    super::plates::share(analysis, &mut kept, &mut given, &mut omitted);
    super::share(&mut kept, &mut given, &mut omitted);
    super::redundancy::consolidate(analysis, &mut kept, &mut given, &mut omitted);
    let index: BTreeMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, w)| (w.id.as_str(), i))
        .collect();
    // Covered items name their survivor independently of the explanatory text.
    let mut owner = vec![None; items.len()];
    for o in &omitted {
        let survivor = match &o.state {
            crate::items::State::Covered { by } => index.get(by.as_str()),
            _ => None,
        };
        // An owner outside the linear inventory reserves no shared lane here.
        if let (Some(&i), Some(&s)) = (index.get(o.id.as_str()), survivor) {
            owner[i] = Some(s);
        }
    }
    // A survivor merged away in turn passes on its own survivor.
    for i in 0..owner.len() {
        let mut s = owner[i];
        for _ in 0..owner.len() {
            match s.and_then(|s| owner[s]) {
                Some(next) => s = Some(next),
                None => break,
            }
        }
        owner[i] = s;
    }
    owner
}

/// Each item's strip and lane in its carrying view (none when it has no view, or its view looks
/// along it): per view and strip, sizes nearest, then locations by distance, overall outermost
/// (ARCH0009-5), then callouts by their tips, right to left. An item merged away ([`owners`])
/// takes its survivor's lane number and, when parallel, its strip.
pub(super) fn lanes<'a>(
    frames: &BTreeMap<&str, Frame>,
    items: impl ExactSizeIterator<Item = (&'a Want, Option<&'static str>)>,
    owner: &[Option<usize>],
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
                    if w.measurements
                        .iter()
                        .any(|m| m.parameter == "thickness.length")
                    {
                        plate_strip(f, view, &w.shape, horizontal)
                    } else {
                        strip(view, horizontal)
                    },
                    (*order, key((b[*axis] - a[*axis]).abs())),
                )
            }
            Shape::Callout { sites } => {
                let side = callout_strip(view);
                let (tip, _) = callout_tip(f, sites, side == Side::Above);
                (side, (3, -key(tip[0])))
            }
            // An angle is placed about its vertex, in no strip ([`super::angular`]).
            Shape::Angular { .. } => continue,
        };
        sides[i] = Some(side);
        if owner[i].is_none() {
            strips
                .entry((view, side))
                .or_default()
                .push((rank.0, rank.1, i));
        }
    }
    let mut lane = vec![0usize; n];
    for list in strips.values_mut() {
        list.sort();
        for (n, (_, _, i)) in list.iter().enumerate() {
            lane[*i] = n;
        }
    }
    share_lanes(owner, &mut sides, &mut lane);
    (sides, lane)
}

/// Gives each item merged away its survivor's lane number and a parallel strip.
/// Equal walls on different projected axes keep their own strip orientation.
fn share_lanes(owner: &[Option<usize>], sides: &mut [Option<Side>], lanes: &mut [usize]) {
    for (i, s) in owner.iter().enumerate() {
        if let Some(s) = *s {
            if let (Some(a), Some(b)) = (sides[i], sides[s])
                && matches!(a, Side::Above | Side::Below) == matches!(b, Side::Above | Side::Below)
            {
                sides[i] = Some(b);
            }
            lanes[i] = lanes[s];
        }
    }
}

/// Gives an item merged away under the defaults' labels ([`owners`]) that the plan draws beside
/// the one whose lane it shares (the spec's labels broke the merge: a tolerance on one of them,
/// say) the next free lane of its strip, outside every lane the strip holds, so two drawn items
/// never share a lane; every other item keeps its lane (ARCH0003-2). Items with lanes of their
/// own claim theirs first, then merged items in order. True for each item moved.
pub(super) fn separate(
    owner: &[Option<usize>],
    drawn: &[bool],
    views_of: &[Option<&'static str>],
    sides: &[Option<Side>],
    lanes: &mut [usize],
) -> Vec<bool> {
    let strip = |i: usize| views_of[i].zip(sides[i]);
    let mut next: BTreeMap<(&str, Side), usize> = BTreeMap::new();
    for (i, lane) in lanes.iter().enumerate() {
        if let Some(key) = strip(i) {
            let n = next.entry(key).or_default();
            *n = (*n).max(lane + 1);
        }
    }
    let mut claimed = std::collections::BTreeSet::new();
    for (i, lane) in lanes.iter().enumerate() {
        if let (Some(key), None, true) = (strip(i), owner[i], drawn[i]) {
            claimed.insert((key, *lane));
        }
    }
    let mut moved = vec![false; lanes.len()];
    for i in 0..lanes.len() {
        let (Some(key), Some(_), true) = (strip(i), owner[i], drawn[i]) else {
            continue;
        };
        if !claimed.insert((key, lanes[i])) {
            let n = next.get_mut(&key).expect("every strip has its next lane");
            lanes[i] = *n;
            *n += 1;
            moved[i] = true;
        }
    }
    moved
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
            ("front", Side::Left) => &mut out.plate_front_left,
            ("front", Side::Above) => &mut out.plate_front_above,
            ("plan", Side::Right) => &mut out.plate_plan_right,
            ("plan", Side::Below) => &mut out.plate_plan_below,
            ("side", Side::Left) => &mut out.plate_side_left_lanes,
            ("side", Side::Below) => &mut out.side_below,
            ("side", Side::Above) => &mut out.side_above,
            ("side", Side::Right) => &mut out.side_right,
            other => unreachable!("no band holds {other:?}"),
        };
        *band = (*band).max(lane + 1);
    }
    out
}

/// The strip across a view from *side*.
fn opposite(side: Side) -> Side {
    match side {
        Side::Below => Side::Above,
        Side::Above => Side::Below,
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

/// A strip that cannot hold its lanes on the page passes them to the opposite strip
/// (draftwright's fall-through, ISSUE-1215-e: a dimension its strip has no room for retries on the
/// strip across the view). A strip holds its linear dimensions up to the first lane whose item
/// does not fit the page ([`Page::fits`]: on the sheet, its label off the views and the title
/// block, no line through the title block or another view); that item and every one outside it
/// move to the next free lanes of the opposite strip, an overall extent first (draftwright moves
/// only the extent), each that fits there taking the next lane; the extent then goes outside the
/// rest where that still fits. One that fits on neither side shares a lane already held where it
/// runs clear of that lane's items, and otherwise keeps its own lane, where the collision checks
/// report it: nothing is dropped. Callouts take no lanes ([`super::leaders`] places them), and an
/// item merged away follows its survivor ([`owners`]). The strips' lanes come from every item the
/// part would have, and the page from the sheet and the views, which no omission changes, so
/// where an item falls is edit-local (ARCH0003-2).
#[allow(clippy::too_many_arguments)]
pub(super) fn fall_through(
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    owner: &[Option<usize>],
    sides: &mut [Option<Side>],
    lanes: &mut [usize],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) {
    let page = Page::new(analysis, sheet, views);
    let linear = |i: usize| matches!(items[i].shape, Shape::Linear { .. });
    let mut strips: BTreeMap<(&str, Side), Vec<usize>> = BTreeMap::new();
    // Callouts hold no lanes: their leaders are placed around the dimensions afterwards.
    for i in 0..items.len() {
        if owner[i].is_some() {
            continue;
        }
        if let (Some(view), Some(side), true) = (views_of[i], sides[i], linear(i)) {
            strips.entry((view, side)).or_default().push(i);
        }
    }
    let fits = |i: usize, side: Side, lane: usize| {
        let view = views_of[i].expect("a strip's item has a view");
        place(&frames[view], &items[i], side, lane).is_some_and(|p| page.fits(&p, view))
    };
    // Each strip's overflow, and the lanes it keeps.
    let mut overflow: Vec<((&str, Side), Vec<usize>)> = Vec::new();
    let mut held: BTreeMap<(&str, Side), usize> = BTreeMap::new();
    for (key, list) in &mut strips {
        list.sort_by_key(|&i| lanes[i]);
        let cut = list
            .iter()
            .position(|&i| !fits(i, key.1, lanes[i]))
            .unwrap_or(list.len());
        let out: Vec<usize> = list[cut..].to_vec();
        let lanes_held = list[..cut].iter().map(|&i| lanes[i] + 1).max();
        held.insert(*key, lanes_held.unwrap_or(0));
        if !out.is_empty() {
            overflow.push((*key, out));
        }
    }
    let full: Vec<(&str, Side)> = overflow.iter().map(|(key, _)| *key).collect();
    // What fits on neither side, in strip order.
    let mut stranded: Vec<usize> = Vec::new();
    for ((view, side), list) in overflow {
        let across = opposite(side);
        // An opposite strip that overflows itself has no room: what it could not hold keeps its
        // lanes there.
        if full.contains(&(view, across)) {
            stranded.extend(list);
            continue;
        }
        let first = *held.get(&(view, across)).unwrap_or(&0);
        // An overall extent has first claim on the free lanes (draftwright falls through only
        // the extent, ISSUE-1215-e); the others follow in their order.
        let extent = |i: usize| items[i].id.starts_with("d:envelope/");
        let (mut order, rest): (Vec<usize>, Vec<usize>) =
            list.into_iter().partition(|&i| extent(i));
        order.extend(rest);
        let mut moved = Vec::new();
        for &i in &order {
            if fits(i, across, first + moved.len()) {
                moved.push(i);
            }
        }
        // The extent then stands outside the rest, as it did on its own strip, where that fits.
        let mut outer: Vec<usize> = moved.iter().copied().filter(|&i| !extent(i)).collect();
        outer.extend(moved.iter().copied().filter(|&i| extent(i)));
        if outer != moved
            && outer
                .iter()
                .enumerate()
                .all(|(k, &i)| fits(i, across, first + k))
        {
            moved = outer;
        }
        for (k, &i) in moved.iter().enumerate() {
            sides[i] = Some(across);
            lanes[i] = first + k;
        }
        held.insert((view, across), first + moved.len());
        stranded.extend(order.into_iter().filter(|i| !moved.contains(i)));
    }
    // One that fits on neither side then shares a lane either strip already holds, where it fits
    // the page and runs clear of every item in that lane along it (draftwright's lateral tier
    // reuse, `_adjust_strip_candidate_labels`: a dimension shares an inner tier where its whole
    // extent is disjoint from every occupant there). A slot's location so chains with the slot's
    // own length, end to end on one line. No lane is added. The item with the fewest such lanes
    // takes one first (the longest among as few, then strip order), its own strip's innermost
    // first, so one lane is not spent where another would have done.
    share_held(
        frames, items, views_of, owner, sides, lanes, &held, &stranded, &fits,
    );
    share_lanes(owner, sides, lanes);
}

/// The stranded items' shared lanes ([`fall_through`]): each lane a strip holds (*held*) that
/// *fits* the item on the page and whose items' extents along it are disjoint from the item's
/// (ends may meet).
#[allow(clippy::too_many_arguments)]
fn share_held(
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    owner: &[Option<usize>],
    sides: &mut [Option<Side>],
    lanes: &mut [usize],
    held: &BTreeMap<(&str, Side), usize>,
    stranded: &[usize],
    fits: &dyn Fn(usize, Side, usize) -> bool,
) {
    if stranded.is_empty() {
        return;
    }
    let along = |i: usize, side: Side, lane: usize| {
        let view = views_of[i].expect("a strip's item has a view");
        let k = usize::from(matches!(side, Side::Left | Side::Right));
        place(&frames[view], &items[i], side, lane).map(|p| {
            ink_points(&p).fold([f64::INFINITY, f64::NEG_INFINITY], |e, q| {
                [e[0].min(q[k]), e[1].max(q[k])]
            })
        })
    };
    // Each held lane's items' extents along it.
    let mut taken: BTreeMap<(&str, Side, usize), Vec<[f64; 2]>> = BTreeMap::new();
    for j in 0..items.len() {
        if let (Some(view), Some(side), None, Shape::Linear { .. }) =
            (views_of[j], sides[j], owner[j], &items[j].shape)
            && lanes[j] < *held.get(&(view, side)).unwrap_or(&0)
            && !stranded.contains(&j)
            && let Some(e) = along(j, side, lanes[j])
        {
            taken.entry((view, side, lanes[j])).or_default().push(e);
        }
    }
    // Each stranded item's places: a held lane of its own strip or the opposite one, innermost
    // first, that it fits, with its extent there.
    let mut places: Vec<(usize, Vec<Shared>)> = stranded
        .iter()
        .map(|&i| {
            let view = views_of[i].expect("a strip's item has a view");
            let own = sides[i].expect("a strip's item has a side");
            let list = [own, opposite(own)]
                .into_iter()
                .flat_map(|side| {
                    (0..*held.get(&(view, side)).unwrap_or(&0)).map(move |lane| (side, lane))
                })
                .filter(|&(side, lane)| fits(i, side, lane))
                .filter_map(|(side, lane)| along(i, side, lane).map(|e| (side, lane, e)))
                .collect();
            (i, list)
        })
        .collect();
    let clear = |taken: &BTreeMap<(&str, Side, usize), Vec<[f64; 2]>>,
                 view: &str,
                 (side, lane, e): Shared| {
        taken.get(&(view, side, lane)).is_none_or(|list| {
            list.iter()
                .all(|o| e[1] <= o[0] + 1e-6 || o[1] <= e[0] + 1e-6)
        })
    };
    loop {
        // The item with the fewest clear places, the longest among as few (first fit
        // decreasing), then first in strip order.
        let best = places
            .iter()
            .enumerate()
            .filter_map(|(k, (i, list))| {
                let view = views_of[*i].expect("a strip's item has a view");
                let mut open = list.iter().filter(|&&p| clear(&taken, view, p));
                let first = open.next()?;
                let n = 1 + open.count();
                Some((n, -key(first.2[1] - first.2[0]), k))
            })
            .min();
        let Some((.., k)) = best else { break };
        let (i, list) = places.remove(k);
        let view = views_of[i].expect("a strip's item has a view");
        let (side, lane, e) = *list
            .iter()
            .find(|&&p| clear(&taken, view, p))
            .expect("a clear place");
        sides[i] = Some(side);
        lanes[i] = lane;
        taken.entry((view, side, lane)).or_default().push(e);
    }
}

/// Places *w* in lane *lane* of strip *side* of its view; none for a callout.
pub(super) fn place(frame: &Frame, w: &Want, side: Side, lane: usize) -> Option<Placed> {
    match &w.shape {
        Shape::Linear { .. } => place_linear(frame, &w.shape, &w.terms, side, lane),
        // A callout's leader is placed with the others' ([`super::leaders`]), an angle about its
        // vertex ([`super::angular`]).
        Shape::Callout { .. } | Shape::Angular { .. } => None,
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
    place_linear_at(
        frame,
        shape,
        terms,
        side,
        lane_at(frame.outline, side, lane),
    )
}

pub(super) fn interior_lanes(frame: &Frame, w: &Want, side: Side) -> Vec<Placed> {
    let o = frame.outline();
    let (edge, inward) = match side {
        Side::Below => (o[1], 1.0),
        Side::Above => (o[3], -1.0),
        Side::Left => (o[0], 1.0),
        Side::Right => (o[2], -1.0),
    };
    (1..=8)
        .filter_map(|lane| {
            place_linear_at(
                frame,
                &w.shape,
                &w.terms,
                side,
                (edge + inward * lane as f64 * LANE, inward),
            )
        })
        .collect()
}

fn place_linear_at(
    frame: &Frame,
    shape: &Shape,
    terms: &[String],
    side: Side,
    position: (f64, f64),
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
    let (t_line, out) = position;
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
/// Moves labels off other items' ink: a linear dimension whose label another item's line
/// crosses (an outer lane's extension line) or meets another label takes the nearest place clear
/// of both, along its own line: between its arrows it slides between them, right (or up) before
/// left; beyond the line's end (a span too short for it) it tries the other end, then further out
/// from either, the line running on to it, up to [`SLIDE_REACH`]. With no clear place an item
/// stays where it was, and the collision is reported. *items* are every linear item the part
/// would have, in plan order (callouts are placed after, around them, by [`super::leaders`]), so
/// where a label settles does not depend on what the spec omits (ARCH0003-2).
pub(super) fn settle(
    items: &mut [Option<Placed>],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) {
    let page = Page::new(analysis, sheet, views);
    settle_where(items, &page, |_| true, |i, j| j != i);
}

pub(super) fn settle_fixed(
    items: &mut [Option<Placed>],
    fixed: &[bool],
    obstacles: &[PlannedDimension],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
) {
    if fixed.iter().all(|f| !f) && obstacles.is_empty() {
        settle(items, analysis, sheet, views);
        return;
    }
    let n = items.len();
    let mut all = items.to_vec();
    all.extend(obstacles.iter().map(|d| d.placed.clone()));
    let page = Page::new(analysis, sheet, views);
    settle_where(&mut all, &page, |i| i < n && !fixed[i], |i, j| i != j);
    items.clone_from_slice(&all[..n]);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn avoid_fixed(
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    chosen: &[Option<&str>],
    sides: &mut [Option<Side>],
    lanes: &mut [usize],
    fixed: &[bool],
    drawn: &[bool],
    placed: &mut [Option<Placed>],
    obstacles: &[PlannedDimension],
    page: &Page,
) {
    let held: Vec<(Placed, bool)> = placed
        .iter()
        .zip(fixed)
        .zip(items)
        .filter_map(|((p, f), w)| {
            if *f {
                Some((p.clone()?, matches!(w.shape, Shape::Callout { .. })))
            } else {
                None
            }
        })
        .chain(
            obstacles
                .iter()
                .filter_map(|d| Some((d.placed.clone()?, d.kind == "callout"))),
        )
        .collect();
    if held.is_empty() {
        return;
    }
    let callout_pin = items
        .iter()
        .zip(fixed)
        .any(|(w, fixed)| *fixed && matches!(w.shape, Shape::Callout { .. }));
    let blocked = |p: &Placed| {
        held.iter().any(|(q, leader)| {
            boxes_overlap(p.label_box, q.label_box)
                || p.lines.iter().any(|l| crosses(*l, q.label_box))
                || q.lines.iter().any(|l| crosses(*l, p.label_box))
                || (*leader
                    && q.lines.iter().any(|l| {
                        p.lines
                            .iter()
                            .chain(&p.arrows)
                            .any(|t| super::leaders::lines_meet(*l, *t))
                    }))
        })
    };
    for i in 0..items.len() {
        if fixed[i] {
            continue;
        }
        let (Some(p), Some(view), Some(side)) = (&placed[i], chosen[i], sides[i]) else {
            continue;
        };
        if !blocked(p) {
            continue;
        }
        let slid = slides(p, page.drawable).find(|p| page.fits(p, view) && !blocked(p));
        if let Some(p) = slid {
            placed[i] = Some(p);
            continue;
        }
        let original = p;
        let opposite = match side {
            Side::Above => Side::Below,
            Side::Below => Side::Above,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        let candidates = (0..64)
            .map(|n| (side, n))
            .chain((0..64).map(|n| (opposite, n)));
        for (side, lane) in candidates {
            if let Some(p) = place(&frames[view], &items[i], side, lane)
                && page.fits(&p, view)
                && !blocked(&p)
                && !placed.iter().enumerate().any(|(j, q)| {
                    callout_pin
                        && drawn[i]
                        && drawn[j]
                        && j != i
                        && chosen[j] == Some(view)
                        && matches!(items[j].shape, Shape::Linear { .. })
                        && q.as_ref().is_some_and(|q| {
                            dimension_lines_overlap(&p, q) && !dimension_lines_overlap(original, q)
                        })
                })
            {
                placed[i] = Some(p);
                sides[i] = Some(side);
                lanes[i] = lane;
                break;
            }
        }
    }
}

// Linear placements store their dimension line after the two extension lines.
// Dimensions sharing a datum must stay on separate lines when moving around a pin.
fn dimension_lines_overlap(p: &Placed, q: &Placed) -> bool {
    let (Some(a), Some(b)) = (p.lines.get(2), q.lines.get(2)) else {
        return false;
    };
    (0..2).any(|k| {
        let along = 1 - k;
        (a[0][k] - a[1][k]).abs() < 1e-6
            && (b[0][k] - b[1][k]).abs() < 1e-6
            && (a[0][k] - b[0][k]).abs() < 1e-6
            && p.arrows.iter().any(|a| {
                q.arrows
                    .iter()
                    .any(|b| (a[0][along] - b[0][along]).abs() < 1e-6)
            })
            && a[0][along]
                .min(a[1][along])
                .max(b[0][along].min(b[1][along]))
                < a[0][along]
                    .max(a[1][along])
                    .min(b[0][along].max(b[1][along]))
                    - 1e-6
    })
}

/// A held (bare-pin) placement re-laid for the item's current label: the lines stay where the
/// automatic layout put them, and only the label is re-set from *terms*. A linear or angular label
/// keeps its centre along its direction; a callout's keeps the edge at its shoulder and the
/// shoulder line follows the new width. Unchanged terms keep the held ink byte for byte.
pub(super) fn relabel(held: &Placed, was: &[String], w: &Want) -> Placed {
    if was == w.terms.as_slice() {
        return held.clone();
    }
    let ink = label_ink(&w.terms, font());
    let mut p = held.clone();
    let fresh = label_box(&ink, p.label_at, p.rotation);
    let old = held.label_box;
    let r = p.rotation.rem_euclid(180.0);
    let axis = if r.abs() < 1e-9 {
        Some(0)
    } else if (r - 90.0).abs() < 1e-9 {
        Some(1)
    } else {
        None
    };
    let mut d = [0.0; 2];
    match (&w.shape, axis) {
        (Shape::Callout { .. }, Some(k)) => {
            // The shoulder is the leader's last segment; the label sits beyond its far end.
            let shoulder = p.lines.last().copied();
            let right = shoulder.is_none_or(|[a, b]| b[k] >= a[k]);
            d[k] = if right {
                old[k] - fresh[k]
            } else {
                old[k + 2] - fresh[k + 2]
            };
        }
        (_, Some(k)) => d[k] = 0.5 * (old[k] + old[k + 2] - fresh[k] - fresh[k + 2]),
        (_, None) => {
            for k in 0..2 {
                d[k] = 0.5 * (old[k] + old[k + 2] - fresh[k] - fresh[k + 2]);
            }
        }
    }
    p.label_at = [p.label_at[0] + d[0], p.label_at[1] + d[1]];
    p.label_box = label_box(&ink, p.label_at, p.rotation);
    if let (Shape::Callout { .. }, Some(k), Some(last)) = (&w.shape, axis, p.lines.last_mut()) {
        // A shoulder that ran under the old label runs under the new one, as far past its end.
        let [a, b] = *last;
        if b[k] >= a[k] && b[k] > old[k] {
            last[1][k] = p.label_box[k + 2] + (b[k] - old[k + 2]);
        } else if b[k] < a[k] && b[k] < old[k + 2] {
            last[1][k] = p.label_box[k] + (b[k] - old[k]);
        }
    }
    p
}

/// Resolve intent independently of the other annotations, then let the solver fill around it.
pub(super) fn intent(
    frame: &Frame,
    w: &Want,
    p: &crate::placement_intent::Place,
    side: Side,
    lane: usize,
    callout_width: f64,
) -> Option<Placed> {
    if !p.pinned && p.side.is_none() && p.lane.is_none() && p.along.is_none() {
        return None;
    }
    let side = match p.side {
        Some(crate::placement_intent::Side::Above) => Side::Above,
        Some(crate::placement_intent::Side::Below) => Side::Below,
        Some(crate::placement_intent::Side::Left) => Side::Left,
        Some(crate::placement_intent::Side::Right) => Side::Right,
        None => side,
    };
    let lane = p.lane.unwrap_or(lane);
    let mut placed = match &w.shape {
        Shape::Linear { axis, .. } => {
            let horizontal = across(frame, *axis)?;
            if horizontal != matches!(side, Side::Above | Side::Below) {
                return None;
            }
            place_linear(frame, &w.shape, &w.terms, side, lane)?
        }
        Shape::Callout { sites } => {
            let (site, _) = sites.first()?;
            let c = frame.page(*site);
            let o = frame.outline();
            let (mut line, out) = lane_at(o, side, 0);
            let ink = label_ink(&w.terms, font());
            let width = ink.max[0] - ink.min[0];
            // Horizontal callout labels need their measured width in a side column. Use the
            // canonical width so editing another item's text cannot move a pin's lane.
            let step = callout_step(side, width, callout_width);
            line += out * lane as f64 * step;
            let k = usize::from(matches!(side, Side::Above | Side::Below));
            let f = p
                .along
                .unwrap_or(crate::placement_intent::Along::Fraction(0.5))
                .fraction();
            let mut elbow = c;
            elbow[k] = line;
            elbow[1 - k] = o[1 - k] + f * (o[3 - k] - o[1 - k]);
            let tip = super::leaders::intent_tip(frame, sites, elbow);
            let left = match side {
                Side::Left => true,
                Side::Right => false,
                Side::Above | Side::Below => p.pinned && elbow[0] < tip[0] - 1e-9,
            };
            let end = [
                elbow[0]
                    + if left {
                        -width - super::SHOULDER
                    } else {
                        width + super::SHOULDER
                    },
                elbow[1],
            ];
            let at = [
                if left {
                    end[0] - ink.min[0]
                } else {
                    elbow[0] + super::SHOULDER - ink.min[0]
                },
                elbow[1] + LIFT,
            ];
            Placed {
                lines: vec![[tip, elbow], [elbow, end]],
                arrows: vec![arrow(tip, elbow)],
                label_at: at,
                rotation: 0.0,
                label_box: label_box(&ink, at, 0.0),
            }
        }
        Shape::Angular { angle } => super::angular::intent(frame, angle, &w.terms, p)?,
    };
    if matches!(w.shape, Shape::Linear { .. })
        && let Some(along) = p.along
    {
        let k = usize::from(placed.rotation != 0.0);
        let lo = placed
            .arrows
            .iter()
            .map(|a| a[0][k])
            .fold(f64::INFINITY, f64::min);
        let hi = placed
            .arrows
            .iter()
            .map(|a| a[0][k])
            .fold(f64::NEG_INFINITY, f64::max);
        let ink = label_ink(&w.terms, font());
        let width = ink.max[0] - ink.min[0];
        placed.label_at[k] = lo + along.fraction() * (hi - lo) - 0.5 * width - ink.min[0];
        placed.label_box = label_box(&ink, placed.label_at, placed.rotation);
    }
    Some(placed)
}

/// Places each item merged away ([`owners`]) in its survivor's lane, once everything else is
/// placed (callouts included), and moves its label as [`settle`] would, clear of every other
/// item but its survivor and the items merged with it: their ink is the one span drawn once. One
/// [`separate`] moved to a lane of its own (*alone*) keeps clear of every other item. The others
/// never see it, so it moves nothing (it is drawn only when the spec omits its survivor or breaks
/// the merge).
#[allow(clippy::too_many_arguments)]
pub(super) fn place_merged(
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    owner: &[Option<usize>],
    alone: &[bool],
    sides: &[Option<Side>],
    lanes: &[usize],
    placed: &mut [Option<Placed>],
    page: &Page,
) {
    for i in 0..items.len() {
        if owner[i].is_some() {
            placed[i] = views_of[i]
                .zip(sides[i])
                .and_then(|(v, side)| place(&frames[v], &items[i], side, lanes[i]));
        }
    }
    settle_where(
        placed,
        page,
        |i| owner[i].is_some(),
        |i, j| j != i && (alone[i] || (owner[j].is_none() && Some(j) != owner[i])),
    );
}

/// [`settle`] over the items *moves* picks, each against the others *sees* picks for it.
fn settle_where(
    items: &mut [Option<Placed>],
    page: &Page,
    moves: impl Fn(usize) -> bool,
    sees: impl Fn(usize, usize) -> bool,
) {
    for i in 0..items.len() {
        if !moves(i) {
            continue;
        }
        let Some(p) = items[i].as_ref() else {
            continue;
        };
        let others = || {
            items
                .iter()
                .enumerate()
                .filter(|(j, _)| sees(i, *j))
                .filter_map(|(_, q)| q.as_ref())
        };
        // A linear dimension's own lines are fixed by what it measures, so only a line run on to
        // its label must clear other labels.
        let clear = |c: &Placed| {
            let own: Vec<&[P2; 2]> = c.lines.iter().filter(|l| !p.lines.contains(l)).collect();
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
        let off_already = !page.on_sheet(p);
        let found = slides(p, page.drawable)
            .find(|c| (off_already || page.on_sheet(c)) && page.clear_of_furniture(c) && clear(c));
        if let Some(c) = found {
            items[i] = Some(c);
        }
    }
}

/// A linear dimension's other label places along its line, nearest first (see [`settle`]),
/// made as they are tried: a line far longer than the sheet (a part drawn at a scale too large
/// for it) has more places than memory holds.
fn slides(p: &Placed, drawable: [f64; 4]) -> Box<dyn Iterator<Item = Placed> + '_> {
    // Along the line: x for a horizontal dimension, y for a vertical one.
    let k = usize::from(p.rotation != 0.0);
    let Some(&line) = p.lines.get(2) else {
        return Box::new(std::iter::empty());
    };
    let (l0, l1) = (line[0][k].min(line[1][k]), line[0][k].max(line[1][k]));
    let b = p.label_box;
    let width = b[k + 2] - b[k];
    // Each a shift of the label and the line's new extent.
    let (lo, hi) = (l0 + ARROW + SLIDE_CLEAR, l1 - ARROW - SLIDE_CLEAR);
    let shifts: Box<dyn Iterator<Item = (f64, [f64; 2])>> = if b[k] >= lo && b[k + 2] <= hi {
        // Only shifts that keep the label on the drawing area along the line: their count is
        // bounded by the sheet, however long the line (a part drawn at a scale far too large).
        let (lo, hi) = (lo.max(drawable[k]), hi.min(drawable[k + 2]));
        let (d_min, d_max) = (lo - b[k], hi - b[k + 2]);
        // The steps n >= 1 for which +n or -n steps may lie in [d_min, d_max], a step wider.
        let up = (
            (d_min / SLIDE_STEP).floor().max(1.0),
            (d_max / SLIDE_STEP).ceil(),
        );
        let down = (
            (-d_max / SLIDE_STEP).floor().max(1.0),
            (-d_min / SLIDE_STEP).ceil(),
        );
        let spans: Vec<(f64, f64)> = [up, down].into_iter().filter(|s| s.0 <= s.1).collect();
        let first = spans.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
        let last = spans.iter().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
        let (first, last) = if first <= last {
            (first as i64, last as i64)
        } else {
            (1, 0)
        };

        Box::new(
            (first..=last)
                .flat_map(|n| [n, -n])
                .map(|n| n as f64 * SLIDE_STEP)
                .filter(move |d| b[k] + d >= lo && b[k + 2] + d <= hi)
                .map(move |d| (d, [l0, l1])),
        )
    } else if b[k] > l1 {
        // Beyond the far end, 1 mm past the line; the near end mirrors it.
        let back = (l0 - 1.0 - width) - b[k];
        Box::new((0..=(SLIDE_REACH / SLIDE_STEP) as i64).flat_map(move |n| {
            let d = n as f64 * SLIDE_STEP;
            (n > 0)
                .then_some((d, [l0, b[k] + d - 1.0]))
                .into_iter()
                .chain([(back - d, [b[k + 2] + back - d + 1.0, l1])])
        }))
    } else {
        Box::new(std::iter::empty())
    };
    Box::new(shifts.map(move |(d, extent)| {
        let mut c = p.clone();
        c.label_box[k] += d;
        c.label_box[k + 2] += d;
        c.label_at[k] += d;
        c.lines[2][0][k] = extent[0];
        c.lines[2][1][k] = extent[1];
        c
    }))
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

/// Every extreme point of a placed item's ink: its lines' ends, its arrowheads' corners (as
/// `dims::render` draws them) and its label box's corners.
pub(super) fn ink_points(p: &Placed) -> impl Iterator<Item = P2> + '_ {
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

pub(super) fn boxes_overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

/// Whether segment *s* passes through the open box *b*.
pub(super) fn crosses(s: [P2; 2], b: [f64; 4]) -> bool {
    let (p, q) = (s[0], s[1]);
    // Wholly to one side of the box: the clipping below finds nothing there either (each bound
    // it divides out is then at least 1 or below 0), so the answer is the same, found sooner.
    if (p[0] < b[0] && q[0] < b[0])
        || (p[0] > b[2] && q[0] > b[2])
        || (p[1] < b[1] && q[1] < b[1])
        || (p[1] > b[3] && q[1] > b[3])
    {
        return false;
    }
    // Liang–Barsky clipping against the box.
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

/// What a placed item's ink must stay inside or off on the page: the drawing area, each view's
/// drawn box ([`outlines`]) and the title block.
pub(super) struct Page<'a> {
    drawable: [f64; 4],
    views: Vec<(&'a str, [f64; 4])>,
    /// Each view's reserved area (its box and its annotation bands), by its ID.
    areas: Vec<(&'a str, [f64; 4])>,
    title_block: [f64; 4],
}

impl<'a> Page<'a> {
    pub(super) fn new(
        analysis: &Analysis,
        sheet: &PlannedSheet,
        views: &'a [PlannedView],
    ) -> Page<'a> {
        Page {
            drawable: drawable(sheet),
            views: outlines(analysis, views)
                .into_iter()
                .filter(|(id, _)| views.iter().any(|v| v.id == *id && v.sheet == sheet.number))
                .collect(),
            areas: views
                .iter()
                .filter(|v| v.sheet == sheet.number)
                .map(|v| (v.id.as_str(), v.area))
                .collect(),
            title_block: sheet.title_block,
        }
    }

    /// The reserved area of the view named *view* (`plan`), when the sheet has it.
    pub(super) fn area(&self, view: &str) -> Option<[f64; 4]> {
        self.areas
            .iter()
            .find(|(id, _)| id.strip_prefix("v:") == Some(view))
            .map(|(_, a)| *a)
    }

    pub(super) fn drawable(&self) -> [f64; 4] {
        self.drawable
    }

    pub(super) fn title_block(&self) -> [f64; 4] {
        self.title_block
    }

    /// Each view's drawn box, by its ID (`v:plan`).
    pub(super) fn views(&self) -> &[(&'a str, [f64; 4])] {
        &self.views
    }

    fn on_sheet(&self, p: &Placed) -> bool {
        ink_points(p).all(|q| inside(q, self.drawable))
    }

    /// The views other than *own* (a view's name, `plan`) that a line of *p* runs across.
    fn crossed<'b>(&'b self, p: &'b Placed, own: &'b str) -> impl Iterator<Item = &'a str> + 'b {
        self.views
            .iter()
            .filter(move |(id, b)| {
                id.strip_prefix("v:") != Some(own) && p.lines.iter().any(|l| crosses(*l, *b))
            })
            .map(|(id, _)| *id)
    }

    /// Whether *p*, an item of view *own*, fits the page: its ink on the sheet, its label off
    /// every view and the title block, and no line through the title block or another view.
    pub(super) fn fits(&self, p: &Placed, own: &str) -> bool {
        self.on_sheet(p) && self.clear_of_furniture(p) && self.crossed(p, own).next().is_none()
    }

    pub(super) fn fits_interior(&self, p: &Placed, own: &str) -> bool {
        self.on_sheet(p)
            && !boxes_overlap(p.label_box, self.title_block)
            && !p.lines.iter().any(|l| crosses(*l, self.title_block))
            && self.crossed(p, own).next().is_none()
            && self.views.iter().all(|(id, b)| {
                id.strip_prefix("v:") == Some(own) || !boxes_overlap(p.label_box, *b)
            })
    }

    /// Whether *p*'s label is off every view and the title block, and no line of it runs through
    /// the title block.
    fn clear_of_furniture(&self, p: &Placed) -> bool {
        !boxes_overlap(p.label_box, self.title_block)
            && self
                .views
                .iter()
                .all(|(_, b)| !boxes_overlap(p.label_box, *b))
            && !p.lines.iter().any(|l| crosses(*l, self.title_block))
    }
}

/// What the simple placement collides with: each a problem on the dimension's ID.
#[cfg(test)]
pub(super) fn collisions(
    analysis: &Analysis,
    dims: &[PlannedDimension],
    sheet: &PlannedSheet,
    views: &[PlannedView],
) -> Vec<Problem> {
    collisions_cached(
        analysis,
        dims,
        sheet,
        views,
        &super::footprints::Cache::default(),
    )
}

/// A placed item as a problem's message names it ([`crate::wording::item`]).
fn named(d: &PlannedDimension) -> String {
    crate::wording::item(&d.id, &d.kind, &d.label)
}

pub(crate) fn collisions_cached(
    analysis: &Analysis,
    dims: &[PlannedDimension],
    sheet: &PlannedSheet,
    views: &[PlannedView],
    cache: &super::footprints::Cache,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let drawable = drawable(sheet);
    let interior_proofs = super::interior::proofs(analysis, views);
    let outlines: Vec<_> = outlines(analysis, views)
        .into_iter()
        .filter(|(id, _)| views.iter().any(|v| v.id == *id && v.sheet == sheet.number))
        .collect();
    let footprints: BTreeMap<_, _> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            super::NAMES
                .contains(&name)
                .then(|| (v.id.as_str(), cache.get(analysis, v)))
        })
        .collect();
    let placed: Vec<(&PlannedDimension, &Placed)> = dims
        .iter()
        .filter(|d| {
            views
                .iter()
                .find(|v| v.id == d.view)
                .map_or(sheet.number == 1, |v| v.sheet == sheet.number)
        })
        .filter_map(|d| d.placed.as_ref().map(|p| (d, p)))
        .collect();
    let mut feature_problems = Vec::new();
    let mut problem = |code: &str, severity: &str, subjects: Vec<String>, message: String| {
        problems.push(Problem {
            id: format!("p:{code}:{}", subjects.join("|")),
            code: code.into(),
            severity: severity.into(),
            message,
            subjects,
            ..Default::default()
        });
    };
    for (d, p) in &placed {
        if let Some(f) = footprints.get(d.view.as_str()) {
            let (code, shafts): (&str, &[[P2; 2]]) = match d.kind.as_str() {
                "callout" => ("leader_through_feature", &p.lines[..p.lines.len().min(1)]),
                "linear" => (
                    "extension_through_feature",
                    &p.lines[..p.lines.len().min(2)],
                ),
                _ => ("", &[]),
            };
            for (n, shaft) in shafts.iter().enumerate() {
                let crossed = if d.kind == "callout" {
                    f.crossed(*shaft, &d.measurements)
                } else {
                    f.crossed_extension(*shaft, &d.measurements)
                };
                for feature in crossed {
                    let subjects = vec![d.id.clone(), feature];
                    feature_problems.push(Problem {
                        id: format!("p:{code}:{}:line:{n}", subjects.join("|")),
                        code: code.into(),
                        severity: "warning".into(),
                        message: format!(
                            "The {} of {} crosses {}.",
                            if d.kind == "callout" {
                                "leader"
                            } else if n == 0 {
                                "first extension line"
                            } else {
                                "second extension line"
                            },
                            named(d),
                            crate::wording::feature_id(analysis, &subjects[1])
                        ),
                        subjects,
                        ..Default::default()
                    });
                }
            }
        }
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
                        "{} runs up to {past:.1} mm past the area reserved for {} and its bands; it may crowd what lies beyond.",
                        crate::wording::capital(&named(d)),
                        crate::wording::view(&d.view)
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
                format!(
                    "{} runs off the sheet's drawing area.",
                    crate::wording::capital(&named(d))
                ),
            );
        }
        let tb = sheet.title_block;
        if boxes_overlap(lb, tb) || p.lines.iter().any(|s| crosses(*s, tb)) {
            problem(
                "annotation_over_title_block",
                "warning",
                vec![d.id.clone()],
                format!(
                    "{} runs into the title block.",
                    crate::wording::capital(&named(d))
                ),
            );
        }
        for (view, b) in &outlines {
            if boxes_overlap(lb, *b)
                && !(*view == d.view
                    && views.iter().find(|v| v.id == d.view).is_some_and(|v| {
                        interior_proofs
                            .get(&v.id)
                            .is_some_and(|proof| proof.blank(v, lb))
                    }))
            {
                problem(
                    "annotation_over_view",
                    "warning",
                    vec![d.id.clone(), view.to_string()],
                    format!(
                        "The label of {} lies over {}.",
                        named(d),
                        crate::wording::view(view)
                    ),
                );
            }
            // A line (a leader, an extension or dimension line) across a view not its own.
            if *view != d.view && p.lines.iter().any(|s| crosses(*s, *b)) {
                problem(
                    "annotation_line_over_view",
                    "warning",
                    vec![d.id.clone(), view.to_string()],
                    format!(
                        "A line of {} runs across {}.",
                        named(d),
                        crate::wording::view(view)
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
                    format!("The labels of {} and {} overlap.", named(a), named(b)),
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
                        "A line of {} runs through the label of {}.",
                        named(a),
                        named(b)
                    ),
                );
            }
            // A callout's leader or shoulder across, or laid along, another item's line (its
            // arrowheads' shafts included): Policy B keeps the leader and logs the crossing
            // (ADR2-D6). Lines of two linear dimensions crossing are not checked yet.
            let theirs: Vec<&[P2; 2]> = pb.lines.iter().chain(&pb.arrows).collect();
            if a.kind == "callout"
                && pa
                    .lines
                    .iter()
                    .any(|s| theirs.iter().any(|t| super::leaders::lines_meet(*s, **t)))
            {
                problem(
                    "annotation_leader_crossing",
                    "warning",
                    vec![a.id.clone(), b.id.clone()],
                    format!("The leader of {} crosses a line of {}.", named(a), named(b)),
                );
            }
        }
    }
    problems.extend(feature_problems);
    problems
}

#[cfg(test)]
mod tests {
    #[test]
    fn front_left_lanes_do_not_reserve_the_plan_band() {
        let front = Some("front");
        let bands = super::band_lanes([((&front, Some(super::Side::Left)), 2)].into_iter());
        assert_eq!(bands.plate_front_left, 3);
        assert_eq!(bands.plan_left, 0);
    }

    use super::*;

    /// `slides` stays bounded by the sheet however long the dimension line: a 200 km line at 1:1
    /// (`tests/views.rs`'s fallback part) gives no more places than the drawing area has steps,
    /// each with the label on it, and the nearest first.
    #[test]
    fn slides_are_bounded_by_the_sheet() {
        let drawable = [10.0, 10.0, 584.0, 410.0];
        let (y, len) = (50.0, 2.0e8);
        let p = Placed {
            lines: vec![
                [[0.0, 40.0], [0.0, 52.0]],
                [[len, 40.0], [len, 52.0]],
                [[0.0, y], [len, y]],
            ],
            arrows: vec![],
            label_at: [100.0, y + LIFT],
            rotation: 0.0,
            label_box: [100.0, y + LIFT, 110.0, y + LIFT + CAP],
        };
        let bound = (2.0 * (drawable[2] - drawable[0]) / SLIDE_STEP) as usize + 4;
        let places: Vec<Placed> = slides(&p, drawable).take(bound + 1).collect();
        assert!(
            !places.is_empty() && places.len() <= bound,
            "{}",
            places.len()
        );
        assert!(
            places
                .iter()
                .all(|c| c.label_box[0] >= drawable[0] && c.label_box[2] <= drawable[2])
        );
        assert_eq!(places[0].label_box[0], 100.0 + SLIDE_STEP);
    }

    /// ISSUE-367-a, planted (the drawings of the parts here no longer have a case since callout
    /// leaders are placed clear of labels, stream d-callout-leaders): an item whose arrowhead
    /// reaches into another item's label, its shaft through the label, is reported as
    /// `annotation_ink_overlap` naming both; and a callout's leader across another item's line is
    /// `annotation_leader_crossing`.
    #[test]
    fn an_arrowhead_and_a_leader_through_other_ink_are_reported() {
        let analysis: Analysis = serde_json::from_value(serde_json::json!({
            "schema": "draftwright.analysis/5",
            "binding": {"source_sha256": "", "part": "", "face_count": 0, "loader": ""},
            "envelope": [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
            "iso_bounds": [0.0, 0.0, 0.0, 0.0]
        }))
        .unwrap();
        let sheet: PlannedSheet = serde_json::from_value(serde_json::json!({
            "size": "A4", "page": [297.0, 210.0], "scale": "1:1", "scale_factor": 1.0,
            "arrangement": "columns", "title_block": [200.0, 10.0, 287.0, 40.0],
            "default": true, "basis": "", "state": { "kind": "shown" }, "title": {}
        }))
        .unwrap();
        let item = |id: &str, kind: &str, placed: Placed| PlannedDimension {
            reference: false,
            determined_by: Vec::new(),
            region: None,
            id: id.into(),
            kind: kind.into(),
            view: "v:plan".into(),
            value: 1.0,
            label: id.into(),
            terms: vec![id.into()],
            measurements: vec![],
            default: true,
            from_spec: vec![],
            basis: String::new(),
            state: crate::items::State::Shown,
            placed: Some(placed),
        };
        // A dimension line ending in an arrowhead whose tip lies in the label at x 50..60.
        let dim = item(
            "d:a",
            "linear",
            Placed {
                lines: vec![[[30.0, 100.0], [55.0, 100.0]]],
                arrows: vec![[[55.0, 100.0], [52.3, 100.0]]],
                label_at: [35.0, 101.0],
                rotation: 0.0,
                label_box: [35.0, 101.0, 45.0, 103.0],
            },
        );
        let label = item(
            "d:b",
            "linear",
            Placed {
                lines: vec![],
                arrows: vec![],
                label_at: [50.0, 99.0],
                rotation: 0.0,
                label_box: [50.0, 99.0, 60.0, 101.5],
            },
        );
        // A leader up across the dimension line at x 40.
        let callout = item(
            "d:c",
            "callout",
            Placed {
                lines: vec![
                    [[40.0, 90.0], [42.0, 110.0]],
                    [[42.0, 110.0], [45.0, 110.0]],
                ],
                arrows: vec![[[40.0, 90.0], [40.3, 92.7]]],
                label_at: [46.0, 109.0],
                rotation: 0.0,
                label_box: [46.0, 109.0, 56.0, 111.5],
            },
        );
        let problems = collisions(&analysis, &[dim, label, callout], &sheet, &[]);
        let has = |code: &str, a: &str, b: &str| {
            problems
                .iter()
                .any(|p| p.code == code && p.subjects == [a.to_string(), b.to_string()])
        };
        assert!(has("annotation_ink_overlap", "d:a", "d:b"), "{problems:#?}");
        assert!(
            has("annotation_leader_crossing", "d:c", "d:a"),
            "{problems:#?}"
        );
        assert!(!has("annotation_leader_crossing", "d:a", "d:c"));
    }
}
