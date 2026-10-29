//! Datum feature symbols and feature control frames on holes, attached to the hole's mark where
//! the hole is seen end-on (#224). Lowering draws a cylinder's datum symbol on its silhouette in
//! the view that sees the cylinder edge-on, and hangs a frame's leader from the centre of its
//! faces ([`crate::pmi_lowering`]); for a hole that silhouette is hidden and that centre is often
//! blank material, so symbols stack on one rim and frames converge on points that do not say
//! which hole they control. ASME Y14.5 and ISO 5459 let a feature of size carry both on its size
//! callout instead, so once the drawing is finished:
//!
//! - **A frame** whose faces are all bores of holes one shown size callout names, and no others,
//!   moves beneath that callout's text in the view that sees the holes end-on, left-aligned with
//!   it and without a leader, as the size callout's own frame (over the text when beneath has no
//!   room); a second frame on the same callout continues the first's stack. A frame on one hole
//!   a table states takes its tag the same way.
//! - **A datum symbol** whose faces are one hole's bore moves onto that hole's mark in the view
//!   that sees it end-on: its own size callout when that callout names this hole alone (a
//!   pattern's `4×` callout would make the whole pattern the datum), else its hole table tag. The
//!   triangle's base stands just off the mark's text, its line running away from the hole, the
//!   frame in the nearest of a few tiers.
//! - **Without a clear place on a mark** (a hole table row replaced the callout, or the callout
//!   has no clear paper beside it), a frame takes a leader to the hole itself in that view
//!   (ASME Y14.5: a frame attached to its feature by a leader): its arrowhead on the bore's
//!   circle, the leader radial to it, then a shelf and the frame; one on several holes leads to
//!   the hole their callout points at and carries their count over it (`4×`, ISO 1101). A datum
//!   symbol without a clear place on its mark stands on a frame so attached to its hole.
//! - **Visible holes first** (ASME Y14.2, ISO 128-22: a leader ends on a visible outline): the
//!   leader goes to a hole the view draws visible. Where none is, the holes' callout kept beside
//!   its table row carries the frame; failing that, the leader goes to one of its holes drawn
//!   hidden, with a `leader_to_hidden_edge` warning. A frame whose leader to its faces ends on
//!   another hole's centre (so it reads as that hole's) does not keep it: it re-aims that leader
//!   at the rim of one of its holes, or takes a new one, either crossing other leaders no more
//!   often than the old one. A second frame on the same holes may stack on the first's leader.
//!
//! Each takes a place only when it is clear of every other item, other views and sheet
//! furniture, on blank paper of its own view, with no finding of its own (a leader to the hole
//! may pass through material no more often than the leader it replaces); otherwise it stays
//! where lowering put it. A place past its view's reserved area alone grows that area, where the
//! grown area stays clear of the margins, the title block, every other view's area and every
//! table, notes block and caption it did not already meet. A frame with an all-around or
//! all-over mark keeps its leader (the mark sits at its bend). One the Spec places in a view or
//! side (`annotations/<id>/place`) is the Spec's, so placing it keeps it on its surface or
//! leader; a bare pin holds the attached ink.

use std::collections::{BTreeMap, BTreeSet};

use super::{P2, PlannedDimension, callout_routes, gdt, placement};
use crate::bridge::holes::Hole;
use crate::model::{Analysis, Plan, Spec};

/// The few tiers a datum symbol tries out from its mark before it stays on its surface.
const TIERS: usize = 3;
/// The gap between a mark's text and the frame or triangle attached to it, paper mm.
const GAP: f64 = 0.5;

/// A hole's mark in its end-on view: its text box, where its leader starts (on the hole), its
/// item ID and what the basis calls it.
struct Mark {
    id: String,
    label_box: [f64; 4],
    from: P2,
    tag: bool,
    name: String,
}

/// Each automatic frame and datum symbol on holes moved onto the holes' size callout or tag in
/// the view that sees them end-on, where it has a clear place; the problems of what moved are
/// checked again.
pub(crate) fn attach(analysis: &Analysis, spec: &Spec, plan: &mut Plan) {
    if !plan
        .gdt
        .iter()
        .any(|g| matches!(g.kind.as_str(), "datum_feature" | "control_frame") && g.placed.is_some())
    {
        return;
    }
    let read = crate::bridge::holes::read(analysis);
    let holes: Vec<&Hole> = read.iter().filter_map(|(_, h)| h.as_ref().ok()).collect();
    let exterior = super::interior::exterior_proofs(analysis, &plan.views);
    let interior = super::interior::callout_proofs(analysis, &plan.views);
    let cache = super::footprints::Cache::default();
    let mut changed = BTreeSet::new();
    let mut rechecked_codes = BTreeSet::new();
    // The frames already on each callout, so the next one continues their stack.
    let mut stacks: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // The frames attached to each set of holes, which a datum symbol on one of them may take.
    let mut framed: BTreeMap<BTreeSet<&str>, Vec<String>> = BTreeMap::new();
    // Frames first: a datum symbol on the same callout then keeps clear of them.
    for kind in ["control_frame", "datum_feature"] {
        for gi in 0..plan.gdt.len() {
            let g = &plan.gdt[gi];
            if g.kind != kind
                || g.placed.is_none()
                || g.state != crate::items::State::Shown
                || !g.modifiers.is_empty()
            {
                continue;
            }
            let frame = kind == "control_frame";
            let Some((vi, named)) = target(analysis, spec, plan, &holes, g) else {
                continue;
            };
            let view = &plan.views[vi];
            let own = mark(plan, &named, &view.id);
            // A datum symbol stands on its hole's mark or on a frame of its hole; a frame may
            // also take a leader to the hole itself.
            if own.is_none() && !frame && !framed.contains_key(&named) {
                continue;
            }
            // The marks to try, in order: the holes' own mark, then (for a datum symbol) a frame
            // already attached to the same hole, which the triangle may stand on (ASME Y14.5,
            // ISO 5459: a datum feature symbol attached to the feature's control frame). A frame
            // may as a last resort stack on another frame's leader to the same holes.
            let mut marks: Vec<Mark> = own.into_iter().collect();
            let mut on_frames: Vec<Mark> = Vec::new();
            for id in framed.get(&named).into_iter().flatten() {
                let Some(q) = plan.gdt.iter().find(|q| &q.id == id) else {
                    continue;
                };
                let Some(p) = q.placed.as_ref() else {
                    continue;
                };
                if frame && q.side != LEADER {
                    continue;
                }
                on_frames.push(Mark {
                    id: id.clone(),
                    label_box: p.label_box,
                    from: p.lines.last().map_or(p.tip, |l| l[1]),
                    tag: false,
                    // On a frame that leads to a hidden hole, it is as far from its mark.
                    name: if q.basis.contains(HIDDEN) {
                        format!("{ON_FRAME}{HIDDEN}")
                    } else {
                        ON_FRAME.into()
                    },
                });
            }
            if !frame {
                marks.append(&mut on_frames);
            }
            let mut settled = None;
            // A frame without a place on its mark leads to one of its holes the view draws
            // visible, else (rather than keep a leader to faces that may read as another
            // feature's) to one it draws hidden.
            let steps = marks
                .iter()
                .map(|m| (Some(m), false))
                .chain(frame.then_some((None, false)))
                .chain(frame.then_some((None, true)))
                .chain(on_frames.iter().map(|m| (Some(m), false)))
                .map(|(m, hidden)| (m, hidden, false));
            // Where its leader to its faces ends on another hole's centre, so that it reads as
            // that hole's frame, a leader to its own hole may cross as many leaders as that one
            // does.
            let misleading = frame && points_elsewhere(analysis, plan, &holes, &named, g);
            let steps = steps.chain(
                [(None, false, true), (None, true, true)]
                    .into_iter()
                    .filter(|_| misleading),
            );
            let taken = tips(plan, &view.id, &g.id);
            for (mark, hidden, relaxed) in steps {
                let candidates = match (frame, mark) {
                    (true, Some(mark)) => frame_candidates(plan, &stacks, g, mark),
                    // Its own leader first, ending on one of its holes instead.
                    (true, None) if relaxed => {
                        retargeted(analysis, view, &holes, &named, g, hidden, &taken)
                            .into_iter()
                            .chain(leader_candidates(
                                analysis,
                                view,
                                &holes,
                                &named,
                                g,
                                marks.first(),
                                hidden,
                                &taken,
                            ))
                            .collect()
                    }
                    (true, None) => leader_candidates(
                        analysis,
                        view,
                        &holes,
                        &named,
                        g,
                        marks.first(),
                        hidden,
                        &taken,
                    ),
                    (false, Some(mark)) => datum_candidates(g, mark),
                    (false, None) => Vec::new(),
                };
                // The mark, and the frames stacked on it, may touch it.
                let mut attached: BTreeSet<String> = BTreeSet::new();
                if let Some(mark) = mark {
                    attached.extend(stacks.get(&mark.id).cloned().unwrap_or_default());
                    attached.insert(mark.id.clone());
                }
                // Its own leader re-aimed, it may still touch what already stood beside it.
                if relaxed && let Some(b) = g.placed.as_ref().map(|p| p.label_box) {
                    attached.extend(
                        plan.gdt
                            .iter()
                            .filter(|q| q.id != g.id && q.view == g.view)
                            .filter(|q| {
                                q.placed.as_ref().is_some_and(|p| {
                                    let pad = [b[0] - 1.0, b[1] - 1.0, b[2] + 1.0, b[3] + 1.0];
                                    placement::boxes_overlap(pad, p.label_box)
                                })
                            })
                            .map(|q| q.id.clone()),
                    );
                }
                if let Some(found) = settle(
                    analysis,
                    plan,
                    gi,
                    view,
                    candidates,
                    &attached,
                    (&exterior, &interior),
                    &cache,
                    relaxed,
                ) {
                    settled = Some((mark, hidden, found));
                    break;
                }
            }
            let Some((mark, hidden, (side, next, area, before))) = settled else {
                continue;
            };
            let id = g.id.clone();
            rechecked_codes.extend(before.into_iter().map(|p| p.code));
            let g = &mut plan.gdt[gi];
            g.view.clone_from(&view.id);
            g.side = side.into();
            g.placed = Some(next);
            g.basis += &match (frame, mark) {
                (true, Some(mark)) => {
                    stacks
                        .entry(mark.id.clone())
                        .or_default()
                        .push(g.id.clone());
                    format!(
                        "; {} {} in the {} view{ATTACHED}s end-on, in place of a leader to its faces (a view or side in annotations/{}/place keeps the leader)",
                        if side == "below" { "beneath" } else { "over" },
                        mark.name,
                        view.id.trim_start_matches("v:"),
                        g.id
                    )
                }
                (true, None) => format!(
                    "; on a leader to {}{} in the {} view{ATTACHED}s end-on, in place of a leader to its faces (a view or side in annotations/{}/place keeps that leader)",
                    if named.len() > 1 {
                        "one of its holes"
                    } else {
                        "its hole"
                    },
                    if hidden { HIDDEN } else { "" },
                    view.id.trim_start_matches("v:"),
                    g.id
                ),
                (false, mark) => format!(
                    "; attached to {} in the {} view{ATTACHED} end-on where its surface is hidden (a view or side in annotations/{}/place keeps it on its surface)",
                    mark.map_or("its mark", |m| m.name.as_str()),
                    view.id.trim_start_matches("v:"),
                    g.id
                ),
            };
            if frame {
                framed.entry(named.clone()).or_default().push(g.id.clone());
            }
            changed.insert(id);
            if let Some((vi, area)) = area {
                plan.views[vi].area = area;
            }
        }
    }
    if changed.is_empty() {
        return;
    }
    let mut all = plan.dimensions.clone();
    all.extend(plan.gdt.iter().filter_map(gdt::proxy));
    callout_routes::refresh(analysis, plan, &all, &cache, &changed, &rechecked_codes);
}

/// What a datum symbol standing on its hole's frame calls that frame.
const ON_FRAME: &str = "the control frame of its hole";
/// What the basis of a frame on a leader to a hole its view draws hidden says: the last resort,
/// after its holes' mark ([`attachments`] counts it as not attached).
const HIDDEN: &str = " (drawn hidden: no view shows its holes visible)";
/// The `side` a frame on a leader to its hole settles with.
const LEADER: &str = "leader";
/// How far beyond the hole's rim a leader's bend may stand, paper mm, nearest first.
const REACH: [f64; 11] = [
    4.0, 6.0, 8.0, 11.0, 15.0, 20.0, 27.0, 36.0, 48.0, 64.0, 85.0,
];

/// Where frame *g* may sit on *mark*: beneath its text, or over it when beneath has no room; a
/// second frame continues the first's stack (*stacks*).
fn frame_candidates(
    plan: &Plan,
    stacks: &BTreeMap<String, Vec<String>>,
    g: &gdt::PlannedGdt,
    mark: &Mark,
) -> Vec<(&'static str, gdt::GdtPlaced)> {
    let stacked: Vec<[f64; 4]> = stacks
        .get(&mark.id)
        .into_iter()
        .flatten()
        .filter_map(|id| plan.gdt.iter().find(|q| &q.id == id)?.placed.as_ref())
        .map(|p| p.label_box)
        .collect();
    let b = mark.label_box;
    let below = stacked.iter().map(|q| q[1]).fold(b[1] - GAP, f64::min);
    let above = stacked.iter().map(|q| q[3]).fold(b[3] + GAP, f64::max);
    let over = stacked.iter().any(|q| q[1] > b[1]);
    // Invariant: only placed glyphs attach.
    let old = g.placed.as_ref().expect("placed");
    let h = old.label_box[3] - old.label_box[1];
    let under = ("below", beneath(old, [b[0], below]));
    let atop = ("above", beneath(old, [b[0], above + h]));
    match (stacked.is_empty(), over) {
        (true, _) => vec![under, atop],
        (false, false) => vec![under],
        (false, true) => vec![atop],
    }
}

/// Where datum symbol *g* may stand on *mark*: the nearest tier first, wherever along the text.
fn datum_candidates(g: &gdt::PlannedGdt, mark: &Mark) -> Vec<(&'static str, gdt::GdtPlaced)> {
    let letter = g.cells.first().cloned().unwrap_or_default();
    let b = mark.label_box;
    let mut out = Vec::new();
    for side in sides(mark) {
        for tier in 0..TIERS {
            for at in along(mark, side) {
                // The base stands GAP off the text, as a frame beneath it does.
                let tip = match side {
                    "below" => [at, b[1] - GAP],
                    "above" => [at, b[3] + GAP],
                    "left" => [b[0] - GAP, at],
                    _ => [b[2] + GAP, at],
                };
                out.push((side, gdt::datum_at(&letter, tip, side, tier)));
            }
        }
    }
    out
}

/// Where frame *g* may sit on a leader to one of its holes *named* in *view*, which sees them
/// end-on (ASME Y14.5: a frame attached to its feature by a leader): the arrowhead on the bore's
/// circle, the leader radial to it out to its bend, then a shelf and the frame. The hole *mark*
/// points at first (so a callout and its frame name the same hole), then the others by distance
/// from it; the nearest bend first, pointing away from the view's middle first. Only the holes
/// the view draws visible ([`seen`]) take the leader (ASME Y14.2, ISO 128-22: a leader ends on
/// a visible outline, not on dashed line-work), or with *hidden* only those it draws hidden.
#[allow(clippy::too_many_arguments)]
fn leader_candidates(
    analysis: &Analysis,
    view: &crate::model::PlannedView,
    holes: &[&Hole],
    named: &BTreeSet<&str>,
    g: &gdt::PlannedGdt,
    mark: Option<&Mark>,
    hidden: bool,
    taken: &[P2],
) -> Vec<(&'static str, gdt::GdtPlaced)> {
    let Some(old) = g.placed.as_ref() else {
        return Vec::new();
    };
    let frame = placement::Frame::new(analysis, view);
    let o = frame.outline();
    let middle = [0.5 * (o[0] + o[2]), 0.5 * (o[1] + o[3])];
    let mut circles: Vec<(P2, f64)> = holes
        .iter()
        .filter(|h| named.contains(h.feature.id.as_str()) && seen(analysis, view, h) != hidden)
        .map(|h| (frame.page(h.at), 0.5 * h.diameter * frame.scale()))
        .collect();
    if let Some(m) = mark {
        let d = |c: P2| (c[0] - m.from[0]).hypot(c[1] - m.from[1]);
        circles.sort_by(|a, b| d(a.0).total_cmp(&d(b.0)));
    }
    circles.truncate(3);
    // Every other hole seen end-on here, at its outermost step: a leader through one would seem
    // to point at it.
    let axis = (0..3).max_by(|&i, &j| view.direction[i].abs().total_cmp(&view.direction[j].abs()));
    let others: Vec<(P2, f64)> = holes
        .iter()
        .filter(|h| h.principal.map(|(k, _)| k) == axis)
        .map(|h| {
            let outer = h
                .counterbore
                .iter()
                .chain(&h.outer_steps)
                .chain(&h.spotface)
                .map(|r| r.diameter)
                .chain(h.countersink.map(|c| c.diameter))
                .fold(h.diameter, f64::max);
            (frame.page(h.at), 0.5 * outer * frame.scale())
        })
        .collect();
    let size = [
        old.label_box[2] - old.label_box[0],
        old.label_box[3] - old.label_box[1],
    ];
    let mut out = Vec::new();
    for reach in REACH {
        for &(c, r) in &circles {
            let mut directions: Vec<P2> = (0..8)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::FRAC_PI_4;
                    [a.cos(), a.sin()]
                })
                .collect();
            let away = [c[0] - middle[0], c[1] - middle[1]];
            directions.sort_by(|a, b| {
                (b[0] * away[0] + b[1] * away[1]).total_cmp(&(a[0] * away[0] + a[1] * away[1]))
            });
            for u in directions {
                let tip = [c[0] + r * u[0], c[1] + r * u[1]];
                if crowded(tip, taken) {
                    continue;
                }
                let elbow = [c[0] + (r + reach) * u[0], c[1] + (r + reach) * u[1]];
                if others.iter().any(|&(o, q)| {
                    (o[0] - c[0]).hypot(o[1] - c[1]) > 1e-6 && near([tip, elbow], o, q)
                }) {
                    continue;
                }
                // The shelf runs on away from the hole; a vertical leader's to either side.
                let shelves: &[f64] = if u[0].abs() < 1e-9 {
                    &[1.0, -1.0]
                } else if u[0] > 0.0 {
                    &[1.0]
                } else {
                    &[-1.0]
                };
                for &dir in shelves {
                    let end = [elbow[0] + dir * gdt::SHELF, elbow[1]];
                    let x0 = if dir > 0.0 { end[0] } else { end[0] - size[0] };
                    let mut next = beneath(old, [x0, end[1] + 0.5 * size[1]]);
                    next.tip = tip;
                    next.lines = vec![[tip, elbow], [elbow, end]];
                    next.arrows = vec![placement::arrow(tip, elbow)];
                    // On one hole of several, the frame says how many it controls.
                    if named.len() > 1 {
                        gdt::with_count(&mut next, named.len());
                    }
                    out.push((LEADER, next));
                }
            }
        }
    }
    out
}

/// Whether *view* sees hole *h* end-on through an open end: no planar face of the part lies
/// nearer the viewer over the bore's axis at its opening (or, for a through hole of known depth,
/// at its exit). The planner has no hidden-line removal (ADR R1), so this tests the stored faces
/// directly; curved faces are not tested, and a part without stored faces counts as seen.
fn seen(analysis: &Analysis, view: &crate::model::PlannedView, h: &Hole) -> bool {
    let Some(ink) = analysis.edge_ink.as_ref() else {
        return true;
    };
    let Some(camera) = haecceity::hlr::View::new(view.direction, view.up) else {
        return true;
    };
    let t = camera.toward;
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut ends = vec![h.at];
    if let (true, Some(depth)) = (h.through, h.depth) {
        ends.push([0, 1, 2].map(|k| h.at[k] + depth * h.direction[k]));
    }
    let covered = |end: [f64; 3]| {
        let q = camera.map(end);
        ink.planes.iter().any(|f| {
            let along = dot(f.normal, t);
            let Some(v0) = f.edges.first().and_then(|&e| ink.edges.get(e)?.first()) else {
                return false;
            };
            if along.abs() < 1e-6 {
                return false;
            }
            // How far toward the viewer the face's plane lies over the end.
            if (dot(f.normal, *v0) - dot(f.normal, end)) / along <= 0.01 {
                return false;
            }
            // Over the end, by the even-odd rule across all its loops' edges.
            let mut inside = false;
            for e in f.edges.iter().filter_map(|&e| ink.edges.get(e)) {
                for w in e.windows(2) {
                    let (a, b) = (camera.map(w[0]), camera.map(w[1]));
                    if (a[1] > q[1]) != (b[1] > q[1])
                        && q[0] < a[0] + (q[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                    {
                        inside = !inside;
                    }
                }
            }
            inside
        })
    };
    ends.into_iter().any(|end| !covered(end))
}

/// Frame *g* where it stands in *view*, its leader's first leg ending on the rim of one of its
/// holes *named* instead (those the view draws visible, or with *hidden* those it draws hidden),
/// nearest its old end first; none when *g* stands in another view.
fn retargeted(
    analysis: &Analysis,
    view: &crate::model::PlannedView,
    holes: &[&Hole],
    named: &BTreeSet<&str>,
    g: &gdt::PlannedGdt,
    hidden: bool,
    taken: &[P2],
) -> Vec<(&'static str, gdt::GdtPlaced)> {
    let Some(old) = g.placed.as_ref().filter(|_| g.view == view.id) else {
        return Vec::new();
    };
    let Some(&[_, next]) = old.lines.first().map(|l| l.as_slice()) else {
        return Vec::new();
    };
    let frame = placement::Frame::new(analysis, view);
    let mut circles: Vec<(P2, f64)> = holes
        .iter()
        .filter(|h| named.contains(h.feature.id.as_str()) && seen(analysis, view, h) != hidden)
        .map(|h| (frame.page(h.at), 0.5 * h.diameter * frame.scale()))
        .collect();
    let d = |c: P2| (c[0] - old.tip[0]).hypot(c[1] - old.tip[1]);
    circles.sort_by(|a, b| d(a.0).total_cmp(&d(b.0)));
    circles
        .into_iter()
        .filter_map(|(c, r)| {
            let (dx, dy) = (next[0] - c[0], next[1] - c[1]);
            let n = dx.hypot(dy);
            if n <= r {
                return None;
            }
            let tip = [c[0] + r * dx / n, c[1] + r * dy / n];
            if crowded(tip, taken) {
                return None;
            }
            let mut p = old.clone();
            p.tip = tip;
            p.lines[0][0] = tip;
            p.arrows = vec![placement::arrow(tip, next)];
            if named.len() > 1 {
                gdt::with_count(&mut p, named.len());
            }
            Some((LEADER, p))
        })
        .collect()
}

/// Where the leaders of *view*'s items other than *id* end: a second leader ending at one of
/// them would read as the same leader.
fn tips(plan: &Plan, view: &str, id: &str) -> Vec<P2> {
    let dims = plan
        .dimensions
        .iter()
        .filter(|d| d.view == view && d.id != id)
        .filter_map(|d| Some(d.placed.as_ref()?.lines.first()?[0]));
    let frames = plan
        .gdt
        .iter()
        .filter(|q| q.view == view && q.id != id)
        .filter_map(|q| q.placed.as_ref())
        .filter(|p| !p.lines.is_empty())
        .map(|p| p.tip);
    dims.chain(frames).collect()
}

/// Whether *tip* lies within a millimetre of one of *taken*, the ends of other leaders.
fn crowded(tip: P2, taken: &[P2]) -> bool {
    taken
        .iter()
        .any(|t| (t[0] - tip[0]).hypot(t[1] - tip[1]) < 1.0)
}

/// Whether frame *g*'s leader ends on the centre of a hole other than its own (*named*) that its
/// view sees end-on, so that the frame reads as that hole's (FTC-07's frames on two holes, led
/// to the middle of their faces, on the hole between them).
fn points_elsewhere(
    analysis: &Analysis,
    plan: &Plan,
    holes: &[&Hole],
    named: &BTreeSet<&str>,
    g: &gdt::PlannedGdt,
) -> bool {
    let (Some(p), Some(view)) = (
        g.placed.as_ref(),
        plan.views.iter().find(|v| v.id == g.view),
    ) else {
        return false;
    };
    if p.lines.is_empty() {
        return false;
    }
    let frame = placement::Frame::new(analysis, view);
    let axis = (0..3).max_by(|&i, &j| view.direction[i].abs().total_cmp(&view.direction[j].abs()));
    holes
        .iter()
        .filter(|h| !named.contains(h.feature.id.as_str()))
        .filter(|h| h.principal.map(|(k, _)| k) == axis)
        .any(|h| {
            let c = frame.page(h.at);
            (c[0] - p.tip[0]).hypot(c[1] - p.tip[1]) < 0.3
        })
}

/// Whether segment *s* comes within *r* (and a half-millimetre gap) of point *c*.
fn near(s: [P2; 2], c: P2, r: f64) -> bool {
    let d = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
    let l = d[0] * d[0] + d[1] * d[1];
    let t = if l > 0.0 {
        (((c[0] - s[0][0]) * d[0] + (c[1] - s[0][1]) * d[1]) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let p = [s[0][0] + t * d[0], s[0][1] + t * d[1]];
    (p[0] - c[0]).hypot(p[1] - c[1]) < r + 0.5
}

/// Where glyph *gi* of *plan* goes in *view*: the first of *candidates* clear of every item but
/// those it is *attached* to, of other views and of sheet furniture, on blank paper of its own
/// view (*proofs*: its exterior and interior proofs), with no finding of its own, or past its
/// view's reserved area alone where that area can grow (the view and grown area returned); with
/// the findings its old place had, to check again. None when no candidate is clear. A leader to
/// a hole may pass through material as often as the old place did, and *relaxed* (in place of a
/// leader that reads as another feature's) cross other leaders as often too.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn settle(
    analysis: &Analysis,
    plan: &Plan,
    gi: usize,
    view: &crate::model::PlannedView,
    candidates: Vec<(&'static str, gdt::GdtPlaced)>,
    attached: &BTreeSet<String>,
    proofs: (
        &BTreeMap<String, std::rc::Rc<super::interior::Proof>>,
        &BTreeMap<String, std::rc::Rc<super::interior::Proof>>,
    ),
    cache: &super::footprints::Cache,
    relaxed: bool,
) -> Option<(
    &'static str,
    gdt::GdtPlaced,
    Option<(usize, [f64; 4])>,
    Vec<crate::model::Problem>,
)> {
    let sheet = std::iter::once(&plan.sheet)
        .chain(&plan.sheets)
        .find(|s| s.number == view.sheet)?;
    let (exterior, interior) = proofs;
    // Other views' outlines are closed to it; its own view's only on proven blank paper.
    let outlines: Vec<(bool, [f64; 4])> = plan
        .views
        .iter()
        .filter(|v| v.sheet == sheet.number)
        .filter(|v| {
            let name = v.id.trim_start_matches("v:");
            super::NAMES.contains(&name) || crate::detail::NAMES.contains(&name)
        })
        .map(|v| {
            (
                v.id == view.id,
                placement::Frame::new(analysis, v).outline(),
            )
        })
        .collect();
    let paper = |b: [f64; 4]| {
        exterior
            .get(&view.id)
            .is_some_and(|p| p.exterior_clear(view, b, &[], &[]))
            || interior.get(&view.id).is_some_and(|p| p.blank(view, b))
    };
    let on_sheet = |id: &str| {
        plan.views
            .iter()
            .find(|v| v.id == id)
            .is_none_or(|v| v.sheet == sheet.number)
    };
    let id = &plan.gdt[gi].id;
    let own_triangle = format!("{id}:triangle:");
    let mut all: Vec<PlannedDimension> = plan.dimensions.clone();
    all.extend(plan.gdt.iter().filter_map(gdt::proxy));
    let i = all
        .iter()
        .position(|d| d.id == *id)
        // Invariant: a placed glyph has a proxy, appended above.
        .expect("the glyph's proxy");
    let peers: Vec<PlannedDimension> = all
        .iter()
        .enumerate()
        .filter(|(j, q)| *j != i && on_sheet(&q.view))
        .map(|(_, q)| q.clone())
        .chain(
            callout_routes::furniture(plan)
                .into_iter()
                .filter(|q| !q.id.starts_with(&own_triangle) && on_sheet(&q.view)),
        )
        .collect();
    let before = placement::collisions_for(analysis, &all, sheet, &plan.views, cache, id);
    let mut chosen = None;
    for (side, next) in candidates {
        if !clear(&next, attached, &peers, &outlines, &paper) {
            continue;
        }
        let mut trial = plan.gdt[gi].clone();
        trial.view.clone_from(&view.id);
        trial.placed = Some(next.clone());
        all[i] = gdt::proxy(&trial).expect("just placed");
        let problems = placement::collisions_for(analysis, &all, sheet, &plan.views, cache, id);
        // A leader to the hole may pass through material as often as the leader it replaces
        // did: the frame then says which hole it controls at no new cost. *relaxed*, it may
        // cross other leaders as often too.
        let tolerated = |ps: &[crate::model::Problem]| -> Vec<crate::model::Problem> {
            let count = |ps: &[crate::model::Problem], code: &str| {
                ps.iter().filter(|p| p.code == code).count()
            };
            let mut codes = vec!["leader_through_feature"];
            if relaxed {
                codes.push("annotation_leader_crossing");
            }
            codes.retain(|c| side == LEADER && count(ps, c) <= count(&before, c));
            ps.iter()
                .filter(|p| !codes.contains(&p.code.as_str()))
                .cloned()
                .collect()
        };
        let problems = tolerated(&problems);
        if problems.is_empty() {
            chosen = Some((side, next, None));
            break;
        }
        // Past its view's reserved area alone: that area grows to hold it where the
        // sheet has vacant paper, and nothing else may change.
        if problems.iter().all(|p| p.code == "annotation_outside_area")
            && let Some(area) =
                super::ink_box(&all[i]).and_then(|ink| grown(view, ink, sheet, &plan.views, &peers))
        {
            let mut views = plan.views.clone();
            // Invariant: `view` is one of the plan's views.
            let vi = views
                .iter()
                .position(|v| v.id == view.id)
                .expect("its view");
            views[vi].area = area;
            if tolerated(&placement::collisions_for(
                analysis, &all, sheet, &views, cache, id,
            ))
            .is_empty()
            {
                chosen = Some((side, next, Some((vi, area))));
                break;
            }
        }
    }
    let (side, next, area) = chosen?;
    Some((side, next, area, before))
}

/// Whether placement intent *p* keeps a glyph on its lowered attachment: it names a view other
/// than *end_on* (where the glyph attaches), or a side, lane or position along a lowered strip.
/// A bare pin, an offset, or a pin in *end_on* holds the attached ink instead.
fn lowered(p: &crate::placement_intent::Place, end_on: &str) -> bool {
    if p.view.as_deref() == Some(end_on) {
        return !p.pinned;
    }
    p.view.is_some() || p.side.is_some() || p.lane.is_some() || p.along.is_some()
}

/// Whether *place* pins lowered glyph *item* in the view it attaches in on its hole's mark (an
/// editor's pin of the attached glyph, where it is drawn). That view is not one the glyph is
/// lowered in, so lowering takes it as a bare pin and [`attach`] holds it on its mark.
pub(crate) fn pinned_on_mark(
    analysis: &Analysis,
    item: &crate::pmi_lowering::Lowered,
    place: &crate::placement_intent::Place,
) -> bool {
    use crate::pmi_lowering::Glyph;
    let frame = match &item.glyph {
        Glyph::ControlFrame {
            all_around,
            all_over,
            ..
        } => !*all_around && !*all_over,
        Glyph::Datum { .. } => false,
        _ => return false,
    };
    if !place.pinned || place.view.is_none() {
        return false;
    }
    let read = crate::bridge::holes::read(analysis);
    let holes: Vec<&Hole> = read.iter().filter_map(|(_, h)| h.as_ref().ok()).collect();
    let Some(owners) = bore_owners(analysis, &holes, &item.faces) else {
        return false;
    };
    if !frame && owners.len() != 1 {
        return false;
    }
    // The principal view down the holes' common axis: side, front or plan down x, y or z.
    let Some(k) = owners[0].principal.map(|(k, _)| k) else {
        return false;
    };
    owners
        .iter()
        .all(|h| h.principal.map(|(a, _)| a) == Some(k))
        && place.view.as_deref() == Some(["side", "front", "plan"][k])
}

/// Where glyph *g* would attach: the index of the principal view down its holes' common axis and the holes
/// whose bores its faces are (one, for a datum symbol); none when it stays where lowering put it,
/// or when the Spec places it in that view or side.
fn target<'h>(
    analysis: &Analysis,
    spec: &Spec,
    plan: &Plan,
    holes: &[&'h Hole<'h>],
    g: &gdt::PlannedGdt,
) -> Option<(usize, BTreeSet<&'h str>)> {
    let owners = bore_owners(analysis, holes, &g.faces)?;
    if g.kind != "control_frame" && owners.len() != 1 {
        return None;
    }
    // The principal view down the holes' common axis.
    let k = owners[0].principal.map(|(k, _)| k)?;
    if owners
        .iter()
        .any(|h| h.principal.map(|(a, _)| a) != Some(k))
    {
        return None;
    }
    let vi = plan.views.iter().position(|v| {
        super::NAMES.contains(&v.id.trim_start_matches("v:")) && v.direction[k].abs() > 0.999
    })?;
    if spec
        .annotations
        .get(&g.id)
        .and_then(|a| a.place.as_ref())
        .is_some_and(|p| lowered(p, plan.views[vi].id.trim_start_matches("v:")))
    {
        return None;
    }
    Some((vi, owners.iter().map(|h| h.feature.id.as_str()).collect()))
}

/// The frames and datum symbols of *plan* that attach to their holes ([`attach`]) when the
/// drawing is finished as it stands, each with the size callout it attaches to: none for a table
/// tag, a frame on the same hole or a leader to the hole itself. One that cannot attach, or only
/// by a leader to a hole drawn hidden (directly or through the frame it stands on), is left out:
/// its holes' callout, where one exists, is the better place.
pub(crate) fn attachments(analysis: &Analysis, plan: &Plan) -> BTreeMap<String, Option<String>> {
    let mut finished = plan.clone();
    attach(analysis, &plan.spec, &mut finished);
    let read = crate::bridge::holes::read(analysis);
    let holes: Vec<&Hole> = read.iter().filter_map(|(_, h)| h.as_ref().ok()).collect();
    finished
        .gdt
        .iter()
        .filter(|g| attached(g) && !g.basis.contains(HIDDEN))
        .map(|g| {
            let on_callout = g.side != LEADER && !g.basis.contains(ON_FRAME);
            let callout = target(analysis, &plan.spec, &finished, &holes, g)
                .filter(|_| on_callout)
                .and_then(|(vi, named)| mark(&finished, &named, &finished.views[vi].id))
                .filter(|m| !m.tag)
                .map(|m| m.id);
            (g.id.clone(), callout)
        })
        .collect()
}

/// Whether glyph *g* was attached to a hole's mark here, so a pin on it holds that ink rather
/// than naming the view it was lowered into.
pub(crate) fn attached(g: &gdt::PlannedGdt) -> bool {
    g.basis.contains(ATTACHED)
}

/// Whether frame *g* leads to one of its holes where its view draws it hidden, as no view
/// shows its holes visible and it has no place on their mark (a frame stacked on such a frame's
/// shelf shares that leader).
pub(crate) fn led_to_hidden(g: &gdt::PlannedGdt) -> bool {
    attached(g) && g.basis.contains(HIDDEN)
}

/// What the basis of an attached glyph says.
const ATTACHED: &str = ", which sees the hole";

/// The holes whose bores *faces* are, each face a cylinder of one hole at its bore's diameter
/// (not a counterbore, spotface or countersink), or none.
fn bore_owners<'a>(
    analysis: &Analysis,
    holes: &[&'a Hole<'a>],
    faces: &[usize],
) -> Option<Vec<&'a Hole<'a>>> {
    if faces.is_empty() {
        return None;
    }
    let mut owners: Vec<&Hole> = Vec::new();
    for &face in faces {
        let a = analysis.face_attachments.get(face)?;
        let (Some("cylinder"), Some(axis), Some(b)) = (a.surface.as_deref(), a.axis, a.bounds)
        else {
            return None;
        };
        // Its diameter from a proven site's distance to its axis, else its box: a bore split in
        // halves at an angle spans less than its diameter across either principal axis.
        let across: Vec<usize> = (0..3).filter(|&j| j != axis).collect();
        let across = match (a.sites.first(), a.support.as_slice()) {
            (Some(p), &[u, v]) => 2.0 * (p[across[0]] - u).hypot(p[across[1]] - v),
            _ => across.iter().map(|&j| b[j + 3] - b[j]).fold(0.0, f64::max),
        };
        let mut of = holes
            .iter()
            .filter(|h| h.feature.faces.binary_search(&face).is_ok());
        let (Some(h), None) = (of.next(), of.next()) else {
            return None;
        };
        if (across - h.diameter).abs() > 1e-3 * h.diameter.max(1.0) {
            return None;
        }
        if !owners.iter().any(|o| o.feature.id == h.feature.id) {
            owners.push(h);
        }
    }
    Some(owners)
}

/// The mark of holes *named* in *view*: a shown size callout naming exactly them, else one hole's
/// table tag.
fn mark(plan: &Plan, named: &BTreeSet<&str>, view: &str) -> Option<Mark> {
    let callout = plan.dimensions.iter().find(|d| {
        let holes: BTreeSet<&str> = d
            .measurements
            .iter()
            .filter(|m| m.parameter == "bore.diameter")
            .map(|m| m.feature.as_str())
            .collect();
        d.view == view
            && d.kind == "callout"
            && d.state == crate::items::State::Shown
            && d.placed.is_some()
            && holes == *named
            // Its holes' own measurements, and the PMI dimensions that tolerance them.
            && d.measurements
                .iter()
                .all(|m| named.contains(m.feature.as_str()) || m.feature.starts_with("pmi:"))
            // A count (`4×`) makes the whole pattern the mark's subject; `⌵ ⌀12 × 90°` and
            // `M6×1` are one hole's.
            && (named.len() > 1 || !counted(&d.label))
    });
    if let Some(d) = callout {
        let p = d.placed.as_ref()?;
        return Some(Mark {
            id: d.id.clone(),
            label_box: p.label_box,
            from: p.lines.first().map_or(p.label_at, |l| l[0]),
            tag: false,
            name: format!(
                "{} size callout",
                if named.len() > 1 { "the holes'" } else { "its" }
            ),
        });
    }
    let b = plan.balloons.iter().find(|b| {
        b.view == view
            && b.certifying
            && b.features.len() == 1
            && named.len() == 1
            && named.contains(b.features[0].as_str())
            && b.state == crate::items::State::Shown
    })?;
    Some(Mark {
        id: b.id.clone(),
        label_box: b.label_box?,
        from: b.leader.map_or(b.at, |l| l[0]),
        tag: true,
        name: format!("its hole table tag {}", b.tag),
    })
}

/// Whether *label* opens with a count, `4×`.
fn counted(label: &str) -> bool {
    label
        .split_once('×')
        .map(|(n, _)| n.trim())
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Frame *old* moved rigidly so its box's top left corner is *corner*, without its leader.
fn beneath(old: &gdt::GdtPlaced, corner: P2) -> gdt::GdtPlaced {
    let b = old.label_box;
    let d = [corner[0] - b[0], corner[1] - b[3]];
    let shift = |p: P2| [p[0] + d[0], p[1] + d[1]];
    let mut next = old.clone();
    next.tip = corner;
    next.lines.clear();
    next.arrows.clear();
    next.label_box = [b[0] + d[0], b[1] + d[1], b[2] + d[0], b[3] + d[1]];
    for s in &mut next.strokes {
        *s = [shift(s[0]), shift(s[1])];
    }
    for a in &mut next.arcs {
        [a[0], a[1]] = shift([a[0], a[1]]);
    }
    for t in next.letters.iter_mut().chain(&mut next.runs) {
        t.at = shift(t.at);
    }
    next
}

/// The sides of *mark* a datum symbol tries, away from the hole first and never back toward it:
/// a callout's text above or below only, so its line never runs along the text.
fn sides(mark: &Mark) -> Vec<&'static str> {
    let b = mark.label_box;
    let away = [
        0.5 * (b[0] + b[2]) - mark.from[0],
        0.5 * (b[1] + b[3]) - mark.from[1],
    ];
    let vertical = if away[1] < 0.0 { "below" } else { "above" };
    let opposite = if vertical == "below" {
        "above"
    } else {
        "below"
    };
    let horizontal = if away[0] < 0.0 { "left" } else { "right" };
    if !mark.tag {
        return vec![vertical, opposite];
    }
    if away[0].abs() > away[1].abs() {
        vec![horizontal, "above", "below"]
    } else {
        vec![vertical, "left", "right"]
    }
}

/// Where along *side* of *mark* the triangle may stand: a tag's middle; along a callout's text,
/// its middle first, then out toward either end, the triangle wholly on the text.
fn along(mark: &Mark, side: &str) -> Vec<f64> {
    let b = mark.label_box;
    let (lo, hi) = if matches!(side, "above" | "below") {
        (b[0], b[2])
    } else {
        (b[1], b[3])
    };
    let middle = 0.5 * (lo + hi);
    if mark.tag {
        return vec![middle];
    }
    let reach = (0.5 * (hi - lo) - gdt::TRIANGLE).max(0.0);
    [0.0, -0.5, 0.5, -1.0, 1.0]
        .iter()
        .map(|f| middle + f * reach)
        .collect()
}

/// The reserved area of *view* grown to hold *ink* with the half-millimetre band
/// [`crate::sheet_fit`] reserves around ink, when the grown area stays inside the drawing margins,
/// off the title block, clear of every other view's area on its sheet and of every table, notes
/// block and caption among *peers* the area did not already meet; else none.
fn grown(
    view: &crate::model::PlannedView,
    ink: [f64; 4],
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
    peers: &[PlannedDimension],
) -> Option<[f64; 4]> {
    let a = view.area;
    let next = [
        a[0].min(ink[0] - 0.5),
        a[1].min(ink[1] - 0.5),
        a[2].max(ink[2] + 0.5),
        a[3].max(ink[3] + 0.5),
    ];
    let m = crate::layout::margin();
    let inside = next[0] >= m - 1e-6
        && next[1] >= m - 1e-6
        && next[2] <= sheet.page[0] - m + 1e-6
        && next[3] <= sheet.page[1] - m + 1e-6;
    let vacant = !placement::boxes_overlap(next, sheet.title_block)
        && views
            .iter()
            .filter(|v| v.id != view.id && v.sheet == view.sheet)
            .all(|v| !placement::boxes_overlap(next, v.area))
        && peers
            .iter()
            .filter(|q| matches!(q.kind.as_str(), "table" | "notes" | "caption"))
            .filter_map(super::ink_box)
            .all(|b| !placement::boxes_overlap(next, b) || placement::boxes_overlap(a, b));
    (inside && vacant).then_some(next)
}

/// Whether glyph *next* keeps the 1 mm text gap from every item but those it is *attached* to
/// (which it may touch, never overlap), and stays off every view but on blank paper of its own:
/// the checks the route repair makes beyond the collision report.
fn clear(
    next: &gdt::GdtPlaced,
    attached: &BTreeSet<String>,
    peers: &[PlannedDimension],
    outlines: &[(bool, [f64; 4])],
    paper: &dyn Fn([f64; 4]) -> bool,
) -> bool {
    let b = next.label_box;
    let pad = [b[0] - 1.0, b[1] - 1.0, b[2] + 1.0, b[3] + 1.0];
    let triangle = next.fills.iter().flatten().fold(b, |t, p| {
        [
            t[0].min(p[0]),
            t[1].min(p[1]),
            t[2].max(p[0]),
            t[3].max(p[1]),
        ]
    });
    if outlines
        .iter()
        .any(|(own, o)| placement::boxes_overlap(triangle, *o) && !(*own && paper(triangle)))
    {
        return false;
    }
    // A datum's triangle stands on its mark: it may touch the mark, nothing else.
    let fills: Vec<[f64; 4]> = next
        .fills
        .iter()
        .map(|f| {
            let t = f.iter().fold(
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                |t, p| {
                    [
                        t[0].min(p[0]),
                        t[1].min(p[1]),
                        t[2].max(p[0]),
                        t[3].max(p[1]),
                    ]
                },
            );
            [t[0] + 0.05, t[1] + 0.05, t[2] - 0.05, t[3] - 0.05]
        })
        .collect();
    peers
        .iter()
        .filter_map(|q| Some((attached.contains(&q.id), q.placed.as_ref()?)))
        .all(|(own, q)| {
            let room = if own { b } else { pad };
            let frame_clear = !placement::boxes_overlap(room, q.label_box)
                && !q
                    .lines
                    .iter()
                    .chain(&q.arrows)
                    .any(|s| placement::crosses(*s, room))
                && !next.lines.iter().any(|s| {
                    // Its line keeps the text gap too, from all but its own mark.
                    let l = q.label_box;
                    let g = if own { 0.0 } else { 1.0 };
                    placement::crosses(*s, [l[0] - g, l[1] - g, l[2] + g, l[3] + g])
                });
            let triangle_clear = own
                || fills.iter().all(|t| {
                    !placement::boxes_overlap(*t, q.label_box)
                        && !q
                            .lines
                            .iter()
                            .chain(&q.arrows)
                            .any(|s| placement::crosses(*s, *t))
                });
            frame_clear && triangle_clear
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_leading_count_counts() {
        assert!(super::counted("4× ⌀7 THRU"));
        assert!(super::counted("12 × ⌀3"));
        assert!(!super::counted("⌀12 THRU ⌵ ⌀20 × 90°"));
        assert!(!super::counted("M6×1 THRU"));
    }
}
