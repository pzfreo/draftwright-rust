//! Callout leaders: draftwright's shared feature-leader placement (`annotations/leaders.py`, with
//! the candidate producers of `holes.py`, `hole_leader_candidates.py`, `_edge_callouts.py` and
//! `_diameters.py`) for the exterior region: a simplified re-implementation of its candidates and
//! objective, not a line-by-line port (its occupancy grid, probe budgets and strip solve are not
//! here).
//!
//! Every callout is a job with a bounded inventory of candidate leaders around its view. A
//! candidate is a tip on a site's rim (radial to the elbow, `holes._rim_tip`), an elbow outside
//! the view, a horizontal shoulder and the label beyond it, on the side the leader runs to
//! (draftwright's `text_side`). The inventory unites draftwright's producers:
//! - **columns** beside the view (its plan and side hole callouts, `_leader_anchors`; its
//!   column of z-turned diameters, `_diameter_column_left`): the elbow just past the view's left
//!   or right edge, or past the dimensions that strip holds, on a row within the view's height,
//!   the site's own row first;
//! - **rows** above and below the view (its front hole callouts, `FrontHoleLeaderCandidateAdapter`;
//!   its row of x-turned diameters, `_diameter_row_below`): the elbow just past the top or bottom
//!   edge, or past that strip's dimensions, at a column within the view's width, the site's own
//!   column first, the label right or left;
//! - **escapes** (its corner and radial candidates for fillets, chamfers and flats,
//!   `_corner_escape_candidates`, `_radial_candidates`): from each site out along sixteen
//!   directions, the elbow a little past where the ray leaves the view.
//!
//! Each candidate is measured analytically (`leader_callout_geometry`): its cost is its ink's
//! length (leader and shoulder, draftwright's objective), in micrometres so sub-micron noise
//! cannot reorder candidates (ISSUE-740-b). Its blockers are counted in two tiers:
//! - **hard** (`_hard_fixed_blockers`): ink off the drawing area; the label or a line in the
//!   title block; the label or shoulder over any view (the exterior region); the leader across
//!   another view, or back into its own view's box after leaving it; a leader longer than
//!   [`MAX_LEADER`];
//! - **crossings** (Policy B, ADR2-D6): one per other item met, as the plan's collision checks
//!   report one problem per pair: an item whose line the leader or shoulder crosses or runs
//!   along, and one whose ink and the label meet (its line through the label, the label on its
//!   label, a leader line through its label). Of these, the ones that touch text are counted
//!   again, below the crossings: between as many crossings, a leader across a line is kept over
//!   text on a line (draftwright protects text both ways, `annotation_text_ink_clear`). The
//!   dimensions are placed first and fixed (ARCH0014-3: principal dimensions claim their strips;
//!   the callouts yield);
//! - **area**: ink outside the view's reserved area (its box and bands), below the crossings and
//!   above any length.
//!
//! The jobs are then assigned jointly (ISSUE-740-a, `_assign_by_view`): each takes one candidate,
//! minimising first the hard blockers, then the crossings (with the fixed ink and between the
//! chosen leaders), then the text touched, then the ink out of its area, then the total length.
//! The jobs split into independent groups that could conflict; each group is solved exactly by a
//! branch and bound whose work is bounded ([`NODE_BUDGET`]), from a greedy start that is its
//! floor when the budget runs out. A callout still blocked or crossing then tries the
//! clear-route sweep (ISSUE-1187-a, draftwright's `_clear_routes`: its leader leaving its sites
//! radially in 64 directions at ten reaches) and takes the best route better than its own that
//! meets no other leader, so a crossing is kept only when the sweep has no clear route
//! ([`clear_routes`] measures it). Nothing is dropped: a job whose every candidate is blocked
//! takes the least blocked (ISSUE-798-a), and the plan's collision checks report what it still
//! crosses; what they cannot see is reported here (a leader back across its own view, one over
//! [`MAX_LEADER`], a search stopped at its budget).
//!
//! The length limits ([`MIN_LEADER`], [`MAX_LEADER`]), the clearances ([`CLEAR`], [`ROW`]) and
//! the inventory's caps are calibrated from draftwright's captured sheets, not taken from its
//! code: draftwright minimises length with no hard cap and no angle limit (none in `leaders.py`
//! or `_leader_candidates.py`), and its `leader_policy.py` is the region policy only (auto,
//! exterior, interior). Its side rule, the label on the side the leader runs to (`text_side`), is
//! ported; the port has no angle limit either.
//!
//! Not ported: draftwright's interior candidates (labels inside a view over blank face,
//! `interior_leader_candidates`) need the view's projected line-work, which the plan, made from
//! the Analysis alone (ADR R1), does not have; the port keeps every label outside the views, as
//! draftwright's `--leader-region exterior`. For the same reason the view's material is its box:
//! draftwright's material re-entry cost (ADR2-8) is the leader re-entering the box. Arrowheads
//! are tested by their shafts (ISSUE-367-a).

use std::collections::BTreeMap;

use super::placement::{Frame, Page, Side, arrow, boxes_overlap, crosses, label_box};
use super::{CAP, P2, Placed, SHOULDER, Shape, Want, font, label_ink};
use crate::model::Problem;

/// The clearance an elbow keeps from the view's edge, or from the outermost dimension ink of the
/// strip it stands in, mm: one arrow and a little, so the leader has room for its head.
const CLEAR: f64 = 3.0;
/// The spacing of stacked callout rows, mm: a text height and two `pad_around_text`
/// (draftwright's `min_gap`).
const ROW: f64 = 7.0;
/// The shortest leader, mm: an arrowhead and about as much line again, so the leader reads as
/// one (draftwright's shortest on its captured sheets are 5.6 to 7 mm).
const MIN_LEADER: f64 = 6.0;
/// The longest leader taken while any shorter one is free of hard blockers, mm: a longer one
/// counts as one (draftwright's longest on its captured sheets is 73.5 mm, connector109's).
const MAX_LEADER: f64 = 75.0;
/// The step between candidate rows (beside a view) and columns (above and below it), mm.
const STEP: f64 = 1.0;
/// At most this many rows or columns are tried on each side of a view, however large the view.
const MAX_STATIONS: usize = 400;
/// The escapes' directions (draftwright's sixteen lead directions, `_END_DIAMETER_LEAD_DIRS`)
/// and how far past the view's edge their elbows go, mm.
const DIRECTIONS: usize = 16;
const ESCAPES: [f64; 3] = [CLEAR, 7.0, 11.0];
/// At most this many sites of one callout are tried (a pattern's members, nearest first).
const MAX_SITES: usize = 64;
/// The candidates each job keeps for the joint assignment, best first.
const KEEP: usize = 40;
/// The joint assignment's work bound per group of conflicting jobs, in search nodes.
pub(super) const NODE_BUDGET: usize = 200_000;
/// The descent passes (single jobs, then conflicting pairs) that improve the greedy start before
/// the search.
const DESCENT: usize = 50;
/// The clearance a callout's label keeps from other labels, mm (draftwright stacks callout rows
/// a text height and two `pad_around_text` apart; half a pad keeps them visibly apart here).
const LABEL_PAD: f64 = 1.0;
/// The weights of the objective's tiers, above any length in micrometres on a sheet.
const HARD: i64 = 1_000_000_000_000;
const CROSSING: i64 = 1_000_000_000;
/// The weight of ink outside the view's reserved area (its box and bands): below a crossing,
/// above any length (a metre, in micrometres).
const AREA: i64 = 1_000_000;
/// Between as many crossings, the fewer through text (draftwright protects text both ways,
/// `annotation_text_ink_clear`, and keeps shaft crossings under Policy B): above the area.
const TEXT_INK: i64 = 100 * AREA;
/// The clear-route sweep (ISSUE-1187-a, draftwright's `_clear_routes`): from each of a
/// callout's [`SWEEP_SITES`] sites nearest its leader's tip, the leader leaves radially in
/// [`SWEEP_DIRECTIONS`] directions, its elbow at each of [`SWEEP_REACH`] from the rim, mm.
const SWEEP_DIRECTIONS: usize = 64;
const SWEEP_REACH: [f64; 10] = [6.0, 8.0, 11.0, 15.0, 20.0, 27.0, 36.0, 48.0, 62.0, 75.0];
const SWEEP_SITES: usize = 8;

/// One leader a callout could take.
#[derive(Clone, Debug)]
struct Candidate {
    tip: P2,
    elbow: P2,
    end: P2,
    label_at: P2,
    label_box: [f64; 4],
    /// The ink's length, micrometres.
    cost: i64,
    hard: u32,
    crossings: u32,
    /// Of its crossings, those with other chosen leaders (in the repair, [`others`]).
    leaders: u32,
    /// Of its crossings, those that touch text: a line through a label, or two labels together.
    text: u32,
    /// Whether its ink leaves its view's reserved area (1) or not (0).
    outside: u32,
    /// The box of all its ink.
    bounds: [f64; 4],
}

impl Candidate {
    fn segments(&self) -> [[P2; 2]; 2] {
        [[self.tip, self.elbow], [self.elbow, self.end]]
    }

    fn score(&self) -> i64 {
        i64::from(self.hard) * HARD
            + i64::from(self.crossings) * CROSSING
            + i64::from(self.text) * TEXT_INK
            + i64::from(self.outside) * AREA
            + self.cost
    }

    /// Whether the label reads right of the elbow.
    fn right(&self) -> bool {
        self.end[0] > self.elbow[0]
    }

    /// Whether it is clear: no hard blocker and nothing crossed.
    fn clear(&self) -> bool {
        self.hard == 0 && self.crossings == 0
    }

    fn placed(&self) -> Placed {
        Placed {
            lines: vec![[self.tip, self.elbow], [self.elbow, self.end]],
            arrows: vec![arrow(self.tip, self.elbow)],
            label_at: self.label_at,
            rotation: 0.0,
            label_box: self.label_box,
        }
    }
}

/// What the callouts must keep clear of besides the page: the placed dimensions' lines (with
/// their arrowheads' shafts) and labels, each with the key of the item it belongs to, as the
/// plan's collision checks report one problem per pair of items.
struct Fixed {
    segments: Vec<(usize, [P2; 2])>,
    labels: Vec<(usize, [f64; 4])>,
    /// The arrowheads' outlines as `dims::render` draws them ([`head`]), which must stay out of
    /// the label: a flare can reach into it beside a shaft that does not.
    heads: Vec<(usize, [P2; 2])>,
}

/// The three edges of the arrowhead on *arrow* (tip, tail), as `dims::render` draws it.
fn head(arrow: [P2; 2]) -> [[P2; 2]; 3] {
    let [tip, tail] = arrow;
    let n = [tail[1] - tip[1], tip[0] - tail[0]];
    let k = super::ARROW_HALF / n[0].hypot(n[1]).max(1e-12);
    let (a, b) = (
        [tail[0] + k * n[0], tail[1] + k * n[1]],
        [tail[0] - k * n[0], tail[1] - k * n[1]],
    );
    [[tip, a], [a, b], [b, tip]]
}

/// One callout's job: its view's outline, its sites on the page (centre, radius), its label's ink
/// box about its baseline origin, and how far past each edge the strip's dimensions reach.
struct Job<'a> {
    view: &'a str,
    outline: [f64; 4],
    /// The view's reserved area (box and bands), when the sheet has it.
    area: Option<[f64; 4]>,
    sites: Vec<(P2, f64)>,
    ink_min: P2,
    ink_max: P2,
    /// Below, above, left, right: the distance past that edge clear of the strip's dimensions.
    beyond: [f64; 4],
}

fn side_index(side: Side) -> usize {
    match side {
        Side::Below => 0,
        Side::Above => 1,
        Side::Left => 2,
        Side::Right => 3,
    }
}

/// Places every callout of *items* (its view in *views_of*), whose entry in *placed* is empty,
/// around its view, clear of the dimensions already in *placed* (those with a strip in *sides*)
/// as far as it can be; see the module's notes. Every item the part would have is placed, so
/// where a callout goes does not depend on what the spec omits (ARCH0003-2).
///
/// Returns the problems the plan's collision checks cannot see: a callout that kept a leader
/// back across its own view or longer than [`MAX_LEADER`] (every candidate it had was blocked),
/// and a group of callouts whose search stopped at [`NODE_BUDGET`] (its assignment is the best
/// found, not proved the best).
pub(super) fn place(
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    sides: &[Option<Side>],
    placed: &mut [Option<Placed>],
    page: &Page,
) -> Vec<Problem> {
    let on = || {
        placed
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Some((i, p.as_ref()?)))
    };
    let fixed = Fixed {
        segments: on()
            .flat_map(|(i, p)| p.lines.iter().chain(&p.arrows).map(move |s| (i, *s)))
            .collect(),
        labels: on().map(|(i, p)| (i, p.label_box)).collect(),
        heads: on()
            .flat_map(|(i, p)| p.arrows.iter().flat_map(move |a| head(*a).map(|e| (i, e))))
            .collect(),
    };
    // How far each view's strips reach with dimension ink, past the view's edges.
    let mut reach: BTreeMap<(&str, usize), f64> = BTreeMap::new();
    for i in 0..items.len() {
        let (Some(view), Some(side), Some(p)) = (views_of[i], sides[i], placed[i].as_ref()) else {
            continue;
        };
        let o = frames[view].outline();
        let mut far = 0.0f64;
        for q in super::placement::ink_points(p) {
            far = far.max(match side {
                Side::Below => o[1] - q[1],
                Side::Above => q[1] - o[3],
                Side::Left => o[0] - q[0],
                Side::Right => q[0] - o[2],
            });
        }
        let e = reach.entry((view, side_index(side))).or_insert(0.0);
        *e = e.max(far);
    }
    let mut jobs: Vec<(usize, Job)> = Vec::new();
    for (i, w) in items.iter().enumerate() {
        let (Shape::Callout { sites }, Some(view)) = (&w.shape, views_of[i]) else {
            continue;
        };
        if placed[i].is_some() {
            continue;
        }
        let f = &frames[view];
        let ink = label_ink(&w.terms, font());
        let beyond = [0, 1, 2, 3].map(|k| {
            reach
                .get(&(view, k))
                .map_or(CLEAR, |r| if *r > 0.0 { r + CLEAR } else { CLEAR })
        });
        jobs.push((
            i,
            Job {
                view,
                outline: f.outline(),
                area: page.area(view),
                sites: page_sites(f, sites),
                ink_min: ink.min,
                ink_max: ink.max,
                beyond,
            },
        ));
    }
    let inventories: Vec<Vec<Candidate>> = jobs
        .iter()
        .map(|(_, job)| inventory(job, &fixed, page))
        .collect();
    let (choice, capped) = assign(&inventories);
    let mut chosen: Vec<Option<Candidate>> = inventories
        .iter()
        .zip(choice)
        .map(|(inv, c)| inv.get(c).cloned())
        .collect();
    repair(&jobs, &mut chosen, &fixed, page);
    let mut problems = Vec::new();
    let named = |i: usize| format!("{} ({})", items[i].id, items[i].terms.join(" "));
    for ((i, job), chosen) in jobs.iter().zip(&chosen) {
        let Some(chosen) = chosen else { continue };
        placed[*i] = Some(chosen.placed());
        let shaft = [chosen.tip, chosen.elbow];
        let view = format!("v:{}", job.view);
        if reenters(shaft, job.outline) {
            let subjects = vec![items[*i].id.clone(), view.clone()];
            problems.push(Problem {
                id: format!("p:annotation_line_over_view:{}", subjects.join("|")),
                code: "annotation_line_over_view".into(),
                severity: "warning".into(),
                message: format!(
                    "The leader of {} runs back across its own {view} view: every place it had was blocked.",
                    named(*i)
                ),
                subjects,
            });
        }
        let length = (shaft[1][0] - shaft[0][0]).hypot(shaft[1][1] - shaft[0][1]);
        if length > MAX_LEADER {
            let subjects = vec![items[*i].id.clone()];
            problems.push(Problem {
                id: format!("p:annotation_leader_long:{}", subjects.join("|")),
                code: "annotation_leader_long".into(),
                severity: "warning".into(),
                message: format!(
                    "The leader of {} is {length:.1} mm long, over the {MAX_LEADER} mm a leader may run: every shorter place it had was blocked.",
                    named(*i)
                ),
                subjects,
            });
        }
    }
    for group in capped {
        let subjects: Vec<String> = group.iter().map(|&k| items[jobs[k].0].id.clone()).collect();
        problems.push(Problem {
            id: format!("p:annotation_leader_search_capped:{}", subjects.join("|")),
            code: "annotation_leader_search_capped".into(),
            severity: "info".into(),
            message: format!(
                "The leaders of {} callouts were placed together by a search that stopped at its bound of {NODE_BUDGET} steps: their places are the best found, not proved the best.",
                subjects.len()
            ),
            subjects,
        });
    }
    problems
}

/// A callout's sites on the page, at most [`MAX_SITES`]. A silhouette site (radius 0) that lands
/// on another silhouette site lies on the view's line of sight (a turned band's two sites along
/// the view direction, both on its axis): it is no silhouette in this view, and is left out.
fn page_sites(f: &Frame, sites: &[(super::P3, f64)]) -> Vec<(P2, f64)> {
    let all: Vec<(P2, f64)> = sites
        .iter()
        .map(|(c, r)| (f.page(*c), f.scale() * r))
        .collect();
    let near = |a: P2, b: P2| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    let kept: Vec<(P2, f64)> = all
        .iter()
        .enumerate()
        .filter(|(i, (p, r))| {
            *r > 0.0
                || !all
                    .iter()
                    .enumerate()
                    .any(|(j, (q, s))| j != *i && *s == 0.0 && near(*p, *q))
        })
        .map(|(_, s)| *s)
        .take(MAX_SITES)
        .collect();
    // A site set wholly on the line of sight still names the feature: keep it rather than none.
    if kept.is_empty() {
        all.into_iter().take(MAX_SITES).collect()
    } else {
        kept
    }
}

/// The tip on *site*'s rim radial towards *elbow* (draftwright's `_rim_tip`).
fn rim_tip(site: (P2, f64), elbow: P2) -> P2 {
    let (c, r) = site;
    let d = [elbow[0] - c[0], elbow[1] - c[1]];
    let n = d[0].hypot(d[1]);
    if n <= r || n < 1e-12 {
        return c;
    }
    [c[0] + r * d[0] / n, c[1] + r * d[1] / n]
}

/// The candidate stations along one side: the natural one (clamped into `[lo, hi]`) first, then
/// alternately past and before it, [`STEP`] apart, at most [`MAX_STATIONS`].
fn stations(natural: f64, lo: f64, hi: f64) -> Vec<f64> {
    if lo > hi {
        return Vec::new();
    }
    let start = natural.clamp(lo, hi);
    let mut out = vec![start];
    let mut k = 1;
    while out.len() < MAX_STATIONS {
        let (up, down) = (start + k as f64 * STEP, start - k as f64 * STEP);
        if up > hi && down < lo {
            break;
        }
        if up <= hi {
            out.push(up);
        }
        if down >= lo && out.len() < MAX_STATIONS {
            out.push(down);
        }
        k += 1;
    }
    out
}

/// The distance from *p* along unit *u* to where the ray leaves box *b* (0 when *p* is outside).
fn exit_distance(p: P2, u: P2, b: [f64; 4]) -> f64 {
    if p[0] < b[0] || p[0] > b[2] || p[1] < b[1] || p[1] > b[3] {
        return 0.0;
    }
    let mut d = f64::INFINITY;
    if u[0] > 1e-12 {
        d = d.min((b[2] - p[0]) / u[0]);
    } else if u[0] < -1e-12 {
        d = d.min((b[0] - p[0]) / u[0]);
    }
    if u[1] > 1e-12 {
        d = d.min((b[3] - p[1]) / u[1]);
    } else if u[1] < -1e-12 {
        d = d.min((b[1] - p[1]) / u[1]);
    }
    if d.is_finite() { d.max(0.0) } else { 0.0 }
}

/// The site of *sites* nearest *elbow*.
fn nearest(sites: &[(P2, f64)], elbow: P2) -> (P2, f64) {
    *sites
        .iter()
        .min_by(|a, b| {
            let d = |s: &(P2, f64)| (s.0[0] - elbow[0]).hypot(s.0[1] - elbow[1]) - s.1;
            d(a).total_cmp(&d(b))
        })
        .expect("a callout has a site")
}

/// Every candidate of *job* measured, the best [`KEEP`] kept (fewest hard blockers, then fewest
/// crossings, then shortest, then generation order), near-duplicates dropped.
fn inventory(job: &Job, fixed: &Fixed, page: &Page) -> Vec<Candidate> {
    let o = job.outline;
    let d = page.drawable();
    let mut raw: Vec<(P2, P2, Option<bool>)> = Vec::new(); // (tip, elbow, label right?)
    // Columns beside the view: the site nearest that edge, rows within the view's height.
    for right in [true, false] {
        let site = *job
            .sites
            .iter()
            .max_by(|a, b| {
                let k = |s: &(P2, f64)| if right { s.0[0] } else { -s.0[0] };
                k(a).total_cmp(&k(b))
            })
            .expect("a callout has a site");
        let k = if right { 3 } else { 2 };
        for off in offsets(job.beyond[k]) {
            let x = if right { o[2] + off } else { o[0] - off };
            for y in stations(site.0[1], o[1].max(d[1]), o[3].min(d[3])) {
                let elbow = [x, y];
                raw.push((
                    rim_tip(nearest(&job.sites, elbow), elbow),
                    elbow,
                    Some(right),
                ));
            }
        }
    }
    // Rows above and below: the site nearest that edge, columns within the view's width.
    for up in [false, true] {
        let site = *job
            .sites
            .iter()
            .max_by(|a, b| {
                let k = |s: &(P2, f64)| if up { s.0[1] } else { -s.0[1] };
                k(a).total_cmp(&k(b))
            })
            .expect("a callout has a site");
        let k = usize::from(up);
        for off in offsets(job.beyond[k]) {
            let y = if up { o[3] + off } else { o[1] - off };
            for x in stations(site.0[0], o[0].max(d[0]), o[2].min(d[2])) {
                let elbow = [x, y];
                let tip = rim_tip(nearest(&job.sites, elbow), elbow);
                raw.push((tip, elbow, None));
            }
        }
    }
    // Escapes from each site.
    for &site in &job.sites {
        for j in 0..DIRECTIONS {
            let a = std::f64::consts::TAU * j as f64 / DIRECTIONS as f64;
            let u = [a.cos(), a.sin()];
            let exit = exit_distance(site.0, u, o);
            for off in ESCAPES {
                let dist = site.1.max(exit) + off;
                let elbow = [site.0[0] + u[0] * dist, site.0[1] + u[1] * dist];
                raw.push((rim_tip(site, elbow), elbow, None));
            }
        }
    }
    let mut measured: Vec<(usize, Candidate)> = Vec::new();
    for (tip, elbow, side) in raw {
        // The label reads on the side the leader runs to; a vertical leader may take either.
        let dx = elbow[0] - tip[0];
        let options: &[bool] = match side {
            Some(r) if (r && dx < -1e-9) || (!r && dx > 1e-9) => &[],
            Some(true) => &[true],
            Some(false) => &[false],
            None if dx > 1e-9 => &[true],
            None if dx < -1e-9 => &[false],
            None => &[true, false],
        };
        for &right in options {
            if let Some(c) = measure(job, tip, elbow, right, &[fixed], page) {
                measured.push((measured.len(), c));
            }
        }
    }
    measured.sort_by_key(|(n, c)| (c.hard, c.crossings, c.text, c.outside, c.cost, *n));
    let mut kept: Vec<Candidate> = Vec::new();
    for (_, c) in measured {
        if kept.len() >= KEEP {
            break;
        }
        let same = kept.iter().any(|k| {
            (0..4).all(|j| (k.label_box[j] - c.label_box[j]).abs() < 0.25)
                && (k.tip[0] - c.tip[0]).abs() < 0.25
                && (k.tip[1] - c.tip[1]).abs() < 0.25
        });
        if !same {
            kept.push(c);
        }
    }
    kept
}

/// The elbow's distances past an edge: just past it and two text rows further (draftwright's
/// lane expansion, `lane_candidates`, a text height and two pads apart), and the same past the
/// strip's dimensions when they reach further.
fn offsets(beyond: f64) -> Vec<f64> {
    let mut out: Vec<f64> = (0..3).map(|k| CLEAR + k as f64 * ROW).collect();
    if beyond > CLEAR + 1e-9 {
        out.extend((0..3).map(|k| beyond + k as f64 * ROW));
    }
    out
}

/// The candidate with its tip, elbow and label side, measured; none when the leader is too short
/// for its arrowhead.
fn measure(
    job: &Job,
    tip: P2,
    elbow: P2,
    right: bool,
    fixed: &[&Fixed],
    page: &Page,
) -> Option<Candidate> {
    let shaft = (elbow[0] - tip[0]).hypot(elbow[1] - tip[1]);
    if shaft < MIN_LEADER {
        return None;
    }
    let out = if right { 1.0 } else { -1.0 };
    let end = [elbow[0] + out * SHOULDER, elbow[1]];
    let at_x = if right {
        end[0] + 1.0 - job.ink_min[0]
    } else {
        end[0] - 1.0 - job.ink_max[0]
    };
    let label_at = [at_x, elbow[1] - 0.5 * CAP];
    let ink = super::LabelInk {
        glyphs: Vec::new(),
        strokes: Vec::new(),
        min: job.ink_min,
        max: job.ink_max,
    };
    let lb = label_box(&ink, label_at, 0.0);
    let segs = [[tip, elbow], [elbow, end]];
    let o = job.outline;
    let mut hard = 0u32;
    // On the drawing area.
    let dr = page.drawable();
    let inside = |p: P2| {
        p[0] >= dr[0] - 1e-9 && p[0] <= dr[2] + 1e-9 && p[1] >= dr[1] - 1e-9 && p[1] <= dr[3] + 1e-9
    };
    if ![tip, elbow, end, [lb[0], lb[1]], [lb[2], lb[3]]]
        .into_iter()
        .all(inside)
    {
        hard += 1;
    }
    let tb = page.title_block();
    if boxes_overlap(lb, tb) || segs.iter().any(|s| crosses(*s, tb)) {
        hard += 1;
    }
    for (id, b) in page.views() {
        let own = id.strip_prefix("v:") == Some(job.view);
        if boxes_overlap(lb, *b) || crosses(segs[1], *b) {
            hard += 1;
        }
        if !own && crosses(segs[0], *b) {
            hard += 1;
        }
    }
    if reenters(segs[0], o) {
        hard += 1;
    }
    if shaft > MAX_LEADER {
        hard += 1;
    }
    let padded = [
        lb[0] - LABEL_PAD,
        lb[1] - LABEL_PAD,
        lb[2] + LABEL_PAD,
        lb[3] + LABEL_PAD,
    ];
    let bounds = [
        tip[0].min(elbow[0]).min(end[0]).min(lb[0]),
        tip[1].min(elbow[1]).min(lb[1]),
        tip[0].max(elbow[0]).max(end[0]).max(lb[2]),
        tip[1].max(elbow[1]).max(lb[3]),
    ];
    let near = [
        bounds[0] - LABEL_PAD,
        bounds[1] - LABEL_PAD,
        bounds[2] + LABEL_PAD,
        bounds[3] + LABEL_PAD,
    ];
    // The items whose lines the leader crosses, and those whose ink and the label's text meet
    // (a line through either label, or the labels together): each pair one problem, as the
    // plan's collision checks report them.
    let mut crossed: Vec<usize> = Vec::new();
    let mut text: Vec<usize> = Vec::new();
    for (i, f) in fixed.iter().flat_map(|f| &f.segments) {
        if !segment_near(*f, near) {
            continue;
        }
        if segs.iter().any(|s| lines_meet(*s, *f)) {
            crossed.push(*i);
        }
        if crosses(*f, lb) {
            text.push(*i);
        }
    }
    for (i, e) in fixed.iter().flat_map(|f| &f.heads) {
        if segment_near(*e, near) && crosses(*e, lb) {
            text.push(*i);
        }
    }
    let own_head = head(arrow(tip, elbow));
    for (i, l) in fixed.iter().flat_map(|f| &f.labels) {
        if boxes_overlap(padded, *l) || segs.iter().chain(&own_head).any(|s| crosses(*s, *l)) {
            text.push(*i);
        }
    }
    for v in [&mut crossed, &mut text] {
        v.sort_unstable();
        v.dedup();
    }
    let crossings = (crossed.len() + text.len()) as u32;
    // The ink's extreme points (its arrowhead within a hair of its tip) against the view's area.
    let outside = job.area.is_some_and(|a| {
        [tip, elbow, end, [lb[0], lb[1]], [lb[2], lb[3]]]
            .iter()
            .any(|p| {
                p[0] < a[0] - 1e-6 || p[0] > a[2] + 1e-6 || p[1] < a[1] - 1e-6 || p[1] > a[3] + 1e-6
            })
    });
    let cost = ((shaft + SHOULDER) * 1000.0).round() as i64;
    Some(Candidate {
        tip,
        elbow,
        end,
        label_at,
        label_box: lb,
        cost,
        hard,
        crossings,
        text: text.len() as u32,
        leaders: crossed
            .iter()
            .chain(&text)
            .filter(|i| **i > usize::MAX / 2)
            .count() as u32,
        outside: u32::from(outside),
        bounds,
    })
}

/// The candidates of the clear-route sweep from *tip* (see [`SWEEP_DIRECTIONS`]): tip, elbow
/// and label side, the label on the side the leader runs to (either for a vertical one). The
/// sites are taken nearest *tip* first, so the sweep starts where the leader is.
fn sweep(job: &Job, tip: P2) -> Vec<(P2, P2, bool)> {
    let mut sites: Vec<(P2, f64)> = job.sites.clone();
    let d = |s: &(P2, f64)| (s.0[0] - tip[0]).hypot(s.0[1] - tip[1]);
    sites.sort_by(|a, b| d(a).total_cmp(&d(b)));
    let mut out = Vec::new();
    for (c, r) in sites.into_iter().take(SWEEP_SITES) {
        for j in 0..SWEEP_DIRECTIONS {
            let a = std::f64::consts::TAU * j as f64 / SWEEP_DIRECTIONS as f64;
            let u = [a.cos(), a.sin()];
            let tip = [c[0] + r * u[0], c[1] + r * u[1]];
            for reach in SWEEP_REACH {
                let elbow = [tip[0] + reach * u[0], tip[1] + reach * u[1]];
                let dx = elbow[0] - tip[0];
                if dx >= -1e-9 {
                    out.push((tip, elbow, true));
                }
                if dx <= 1e-9 {
                    out.push((tip, elbow, false));
                }
            }
        }
    }
    out
}

/// The ink of the chosen leaders other than job *k*'s, as fixed ink.
fn others(chosen: &[Option<Candidate>], k: usize) -> Fixed {
    let mut f = Fixed {
        segments: Vec::new(),
        labels: Vec::new(),
        heads: Vec::new(),
    };
    for (j, c) in chosen.iter().enumerate() {
        let Some(c) = c.as_ref().filter(|_| j != k) else {
            continue;
        };
        // Keys above half the range mark the chosen leaders (`Candidate::leaders`).
        let key = usize::MAX - j;
        f.segments.extend(c.segments().map(|s| (key, s)));
        f.segments.push((key, arrow(c.tip, c.elbow)));
        f.labels.push((key, c.label_box));
        f.heads
            .extend(head(arrow(c.tip, c.elbow)).map(|e| (key, e)));
    }
    f
}

/// Policy B's retention, made exact (ISSUE-1187-a): a callout whose chosen leader is blocked or
/// crosses ink (the dimensions', or another chosen leader's) takes the best route of its
/// clear-route [`sweep`] (fewest hard blockers, then crossings, then through text, then out of
/// its view's area, then shortest) that is better than its own and meets no other chosen
/// leader; a route clear of everything is the best there is. Otherwise it keeps its leader, and
/// what it crosses is reported. Repeated until no callout moves: a move lowers the mover's score
/// and raises no other's (it meets no other leader), so the passes end; and afterwards no callout
/// keeps a crossing while its sweep has a clear route.
fn repair(jobs: &[(usize, Job)], chosen: &mut [Option<Candidate>], fixed: &Fixed, page: &Page) {
    loop {
        let mut moved = false;
        for (k, (_, job)) in jobs.iter().enumerate() {
            let Some(now) = chosen[k].clone() else {
                continue;
            };
            let rest = others(chosen, k);
            let sets = [fixed, &rest];
            let now = measure(job, now.tip, now.elbow, now.right(), &sets, page).unwrap_or(now);
            if now.clear() {
                continue;
            }
            let best = sweep(job, now.tip)
                .into_iter()
                .filter_map(|(tip, elbow, right)| measure(job, tip, elbow, right, &sets, page))
                .filter(|c| c.leaders == 0 && c.score() < now.score())
                .enumerate()
                .min_by_key(|(n, c)| (c.score(), *n))
                .map(|(_, c)| c);
            if let Some(c) = best {
                chosen[k] = Some(c);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

/// How many routes of the clear-route sweep (ISSUE-1187-a) the planned callout *id* has that are
/// clear of the page's furniture, the views and every other planned dimension's ink: what the
/// plan measures before it keeps a blocked or crossing leader (Policy B), so for such a leader
/// it is zero. None when *id* is not a placed callout of the plan.
pub fn clear_routes(
    analysis: &crate::model::Analysis,
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
    dims: &[super::PlannedDimension],
    id: &str,
) -> Option<usize> {
    let d = dims.iter().find(|d| d.id == id && d.kind == "callout")?;
    let p = d.placed.as_ref()?;
    let view = d.view.strip_prefix("v:")?;
    let (mut wants, _) = super::wants(analysis);
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            super::NAMES
                .contains(&name)
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    super::route(&mut wants, &frames, analysis);
    let w = wants.iter().find(|w| w.id == id)?;
    let Shape::Callout { sites } = &w.shape else {
        return None;
    };
    let f = frames.get(view)?;
    let page = Page::new(analysis, sheet, views);
    let ink = label_ink(&d.terms, font());
    let job = Job {
        view,
        outline: f.outline(),
        area: page.area(view),
        sites: page_sites(f, sites),
        ink_min: ink.min,
        ink_max: ink.max,
        beyond: [CLEAR; 4],
    };
    let rest = || {
        dims.iter()
            .enumerate()
            .filter(|(_, o)| o.id != id)
            .filter_map(|(i, o)| Some((i, o.placed.as_ref()?)))
    };
    let fixed = Fixed {
        segments: rest()
            .flat_map(|(i, q)| q.lines.iter().chain(&q.arrows).map(move |s| (i, *s)))
            .collect(),
        labels: rest().map(|(i, q)| (i, q.label_box)).collect(),
        heads: rest()
            .flat_map(|(i, q)| q.arrows.iter().flat_map(move |a| head(*a).map(|e| (i, e))))
            .collect(),
    };
    let tip = p.lines.first()?[0];
    Some(
        sweep(&job, tip)
            .into_iter()
            .filter_map(|(tip, elbow, right)| measure(&job, tip, elbow, right, &[&fixed], &page))
            .filter(Candidate::clear)
            .count(),
    )
}

/// Whether segment *s*, leaving from its first point, runs into the box *b* from outside it: a
/// leader re-entering its own view (its tip on the view's edge, the elbow across the view).
fn reenters(s: [P2; 2], b: [f64; 4]) -> bool {
    let (p, d) = (s[0], [s[1][0] - s[0][0], s[1][1] - s[0][1]]);
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
        } else if den < 0.0 {
            t0 = t0.max(num / den);
        } else {
            t1 = t1.min(num / den);
        }
    }
    let len = d[0].hypot(d[1]);
    // Entering after it starts (beyond a hair at the tip), and running inside for a while.
    t0 * len > 0.05 && (t1 - t0) * len > 0.05
}

fn segment_near(s: [P2; 2], b: [f64; 4]) -> bool {
    s[0][0].max(s[1][0]) >= b[0]
        && s[0][0].min(s[1][0]) <= b[2]
        && s[0][1].max(s[1][1]) >= b[1]
        && s[0][1].min(s[1][1]) <= b[3]
}

/// The distance within which two parallel lines read as one, and how long they must run together
/// to, mm.
const ALONG: f64 = 1.0;
const ALONG_RUN: f64 = 0.5;

/// Whether two lines' ink meets: they cross ([`segments_cross`]), or run along each other (a
/// leader laid on an extension line or another leader, [`ALONG`] apart for [`ALONG_RUN`] or more).
pub(super) fn lines_meet(a: [P2; 2], b: [P2; 2]) -> bool {
    segments_cross(a, b) || segments_along(a, b)
}

/// Whether two segments run along each other: parallel, within [`ALONG`], overlapping by
/// [`ALONG_RUN`] or more.
pub(super) fn segments_along(a: [P2; 2], b: [P2; 2]) -> bool {
    let d = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
    let len = d[0].hypot(d[1]);
    if len < 1e-9 {
        return false;
    }
    let u = [d[0] / len, d[1] / len];
    let off = |p: P2| (p[0] - a[0][0]) * u[1] - (p[1] - a[0][1]) * u[0];
    let at = |p: P2| (p[0] - a[0][0]) * u[0] + (p[1] - a[0][1]) * u[1];
    if off(b[0]).abs() > ALONG || off(b[1]).abs() > ALONG {
        return false;
    }
    let (t0, t1) = (at(b[0]).min(at(b[1])), at(b[0]).max(at(b[1])));
    t1.min(len) - t0.max(0.0) >= ALONG_RUN
}

/// Whether two segments cross at a point inside both (touching ends and collinear runs do not
/// count).
pub(super) fn segments_cross(a: [P2; 2], b: [P2; 2]) -> bool {
    let orient =
        |p: P2, q: P2, r: P2| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let eps = 1e-9;
    let (d1, d2) = (orient(b[0], b[1], a[0]), orient(b[0], b[1], a[1]));
    let (d3, d4) = (orient(a[0], a[1], b[0]), orient(a[0], a[1], b[1]));
    ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
        && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
}

/// Whether two candidates' ink cannot coexist (`_candidate_conflict`): labels within
/// [`LABEL_PAD`] of each other, a line of one through the other's label, or their lines crossing.
fn conflict(a: &Candidate, b: &Candidate) -> bool {
    let grow = |x: [f64; 4]| {
        [
            x[0] - LABEL_PAD,
            x[1] - LABEL_PAD,
            x[2] + LABEL_PAD,
            x[3] + LABEL_PAD,
        ]
    };
    if !boxes_overlap(grow(a.bounds), grow(b.bounds)) {
        return false;
    }
    if boxes_overlap(grow(a.label_box), b.label_box) {
        return true;
    }
    let (sa, sb) = (a.segments(), b.segments());
    sa.iter().any(|s| crosses(*s, b.label_box))
        || sb.iter().any(|s| crosses(*s, a.label_box))
        || sa.iter().any(|s| sb.iter().any(|t| lines_meet(*s, *t)))
}

/// The joint assignment: one candidate index per inventory (none for an empty one), minimising the
/// sum of the chosen candidates' scores plus [`CROSSING`] per conflicting pair. Jobs are grouped
/// by possible conflicts; each group starts from its greedy assignment (jobs in order, each the
/// best given those before it) and is then searched exactly, best first, up to [`NODE_BUDGET`]
/// nodes; the best found is kept, and the groups whose search hit that bound are returned too
/// (their members' job indices). Deterministic: ties keep the earlier job's and candidate's
/// order.
fn assign(inventories: &[Vec<Candidate>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let n = inventories.len();
    // Pairwise conflicts, per pair of jobs that have any.
    let mut pairs: BTreeMap<(usize, usize), Vec<Vec<bool>>> = BTreeMap::new();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        parent[i] = r;
        r
    }
    for a in 0..n {
        for b in a + 1..n {
            let m: Vec<Vec<bool>> = inventories[a]
                .iter()
                .map(|ca| inventories[b].iter().map(|cb| conflict(ca, cb)).collect())
                .collect();
            if m.iter().flatten().any(|x| *x) {
                pairs.insert((a, b), m);
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut choice = vec![0usize; n];
    let mut capped = Vec::new();
    for members in groups.values() {
        let (picked, stopped) = solve_group(members, inventories, &pairs);
        for (k, &i) in members.iter().enumerate() {
            choice[i] = picked[k];
        }
        if stopped {
            capped.push(members.clone());
        }
    }
    (choice, capped)
}

fn pair_cost(
    pairs: &BTreeMap<(usize, usize), Vec<Vec<bool>>>,
    a: usize,
    ca: usize,
    b: usize,
    cb: usize,
) -> i64 {
    let (k, (x, y)) = if a < b {
        ((a, b), (ca, cb))
    } else {
        ((b, a), (cb, ca))
    };
    match pairs.get(&k) {
        Some(m) if m[x][y] => CROSSING,
        _ => 0,
    }
}

fn solve_group(
    members: &[usize],
    inventories: &[Vec<Candidate>],
    pairs: &BTreeMap<(usize, usize), Vec<Vec<bool>>>,
) -> (Vec<usize>, bool) {
    let m = members.len();
    let scores: Vec<Vec<i64>> = members
        .iter()
        .map(|&i| inventories[i].iter().map(Candidate::score).collect())
        .collect();
    // Greedy floor.
    let mut best: Vec<usize> = Vec::with_capacity(m);
    let mut best_total = 0i64;
    for k in 0..m {
        let mut pick = (i64::MAX, 0usize);
        for (c, s) in scores[k].iter().enumerate() {
            let t = s
                + (0..k)
                    .map(|j| pair_cost(pairs, members[k], c, members[j], best[j]))
                    .sum::<i64>();
            if t < pick.0 {
                pick = (t, c);
            }
        }
        best.push(pick.1);
        best_total += if scores[k].is_empty() { 0 } else { pick.0 };
    }
    if m < 2 {
        return (best, false);
    }
    // Improved by descent (each job, then each conflicting pair of jobs, re-picks its best given
    // all the others, while any improves, at most DESCENT passes): a better incumbent prunes the
    // search far sooner.
    let others = |k: usize, c: usize, skip: usize, best: &[usize]| {
        (0..m)
            .filter(|&j| j != k && j != skip)
            .map(|j| pair_cost(pairs, members[k], c, members[j], best[j]))
            .sum::<i64>()
    };
    for _ in 0..DESCENT {
        let mut improved = false;
        for k in (0..m).filter(|&k| !scores[k].is_empty()) {
            let now = scores[k][best[k]] + others(k, best[k], k, &best);
            let (t, c) = (0..scores[k].len())
                .map(|c| (scores[k][c] + others(k, c, k, &best), c))
                .min()
                .expect("a non-empty inventory");
            if t < now {
                best[k] = c;
                best_total += t - now;
                improved = true;
            }
        }
        for k in (0..m).filter(|&k| !scores[k].is_empty()) {
            for l in (k + 1..m).filter(|&l| !scores[l].is_empty()) {
                let key = (members[k].min(members[l]), members[k].max(members[l]));
                if !pairs.contains_key(&key) {
                    continue;
                }
                let joint = |c: usize, d: usize, best: &[usize]| {
                    scores[k][c]
                        + scores[l][d]
                        + pair_cost(pairs, members[k], c, members[l], d)
                        + others(k, c, l, best)
                        + others(l, d, k, best)
                };
                let now = joint(best[k], best[l], &best);
                let mut pick = (now, best[k], best[l]);
                for c in 0..scores[k].len() {
                    for d in 0..scores[l].len() {
                        let t = joint(c, d, &best);
                        if t < pick.0 {
                            pick = (t, c, d);
                        }
                    }
                }
                if pick.0 < now {
                    best[k] = pick.1;
                    best[l] = pick.2;
                    best_total += pick.0 - now;
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }
    // Exact search, bounded: the remaining jobs' best scores bound what is left.
    let floor: Vec<i64> = scores
        .iter()
        .map(|s| s.iter().copied().min().unwrap_or(0))
        .collect();
    let mut rest = vec![0i64; m + 1];
    for k in (0..m).rev() {
        rest[k] = rest[k + 1] + floor[k];
    }
    let mut nodes = 0usize;
    let mut current: Vec<usize> = Vec::with_capacity(m);
    #[allow(clippy::too_many_arguments)]
    fn search(
        k: usize,
        total: i64,
        current: &mut Vec<usize>,
        best: &mut Vec<usize>,
        best_total: &mut i64,
        nodes: &mut usize,
        members: &[usize],
        scores: &[Vec<i64>],
        rest: &[i64],
        pairs: &BTreeMap<(usize, usize), Vec<Vec<bool>>>,
    ) {
        if k == members.len() {
            if total < *best_total {
                *best_total = total;
                *best = current.clone();
            }
            return;
        }
        if scores[k].is_empty() {
            current.push(0);
            search(
                k + 1,
                total,
                current,
                best,
                best_total,
                nodes,
                members,
                scores,
                rest,
                pairs,
            );
            current.pop();
            return;
        }
        // Best first.
        let mut order: Vec<(i64, usize)> = scores[k]
            .iter()
            .enumerate()
            .map(|(c, s)| {
                let p: i64 = (0..k)
                    .map(|j| pair_cost(pairs, members[k], c, members[j], current[j]))
                    .sum();
                (s + p, c)
            })
            .collect();
        order.sort();
        for (t, c) in order {
            if *nodes >= NODE_BUDGET {
                return;
            }
            *nodes += 1;
            if total + t + rest[k + 1] >= *best_total {
                break;
            }
            current.push(c);
            search(
                k + 1,
                total + t,
                current,
                best,
                best_total,
                nodes,
                members,
                scores,
                rest,
                pairs,
            );
            current.pop();
        }
    }
    search(
        0,
        0,
        &mut current,
        &mut best,
        &mut best_total,
        &mut nodes,
        members,
        &scores,
        &rest,
        pairs,
    );
    (best, nodes >= NODE_BUDGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tip lies on the rim, radial towards the elbow (`holes._rim_tip`): the leader points at
    /// the hole's centre whatever side it leaves by.
    #[test]
    fn a_tip_is_on_the_rim_radial_to_the_elbow() {
        let site = ([50.0, 40.0], 6.0);
        for elbow in [[80.0, 40.0], [20.0, 70.0], [50.0, 10.0], [61.0, 52.0]] {
            let t = rim_tip(site, elbow);
            let r = [t[0] - 50.0, t[1] - 40.0];
            assert!((r[0].hypot(r[1]) - 6.0).abs() < 1e-9);
            let l = [elbow[0] - t[0], elbow[1] - t[1]];
            assert!((r[0] * l[1] - r[1] * l[0]).abs() < 1e-9 && r[0] * l[0] + r[1] * l[1] > 0.0);
        }
    }

    /// Stations start at the natural one, alternate outwards, stay within the side and are
    /// bounded however long the side.
    #[test]
    fn stations_are_bounded_and_nearest_first() {
        let s = stations(10.0, 0.0, 1.0e7);
        assert_eq!(s.len(), MAX_STATIONS);
        assert_eq!(&s[..3], &[10.0, 11.0, 9.0]);
        let s = stations(-5.0, 0.0, 2.5);
        assert_eq!(s, vec![0.0, 1.0, 2.0]);
        assert!(stations(0.0, 1.0, 0.0).is_empty());
    }

    fn cand(tip: P2, elbow: P2, cost: i64) -> Candidate {
        let end = [elbow[0] + SHOULDER, elbow[1]];
        let label_box = [end[0] + 1.0, elbow[1] - 1.0, end[0] + 10.0, elbow[1] + 1.5];
        Candidate {
            tip,
            elbow,
            end,
            label_at: [end[0] + 1.0, elbow[1]],
            label_box,
            cost,
            hard: 0,
            crossings: 0,
            text: 0,
            leaders: 0,
            outside: 0,
            bounds: [
                tip[0].min(elbow[0]),
                tip[1].min(elbow[1]).min(label_box[1]),
                label_box[2],
                tip[1].max(elbow[1]).max(label_box[3]),
            ],
        }
    }

    /// Two callouts whose cheapest leaders cross: the joint assignment gives one its second
    /// choice rather than letting them cross (ISSUE-740-a), and keeps the cheaper total.
    #[test]
    fn crossing_leaders_are_assigned_apart() {
        // Job 0's best runs up-right across job 1's best, which runs right.
        let a = vec![
            cand([0.0, 0.0], [10.0, 10.0], 14_000),
            cand([0.0, 0.0], [10.0, -10.0], 15_000),
        ];
        let b = vec![
            cand([0.0, 5.0], [20.0, 5.0], 20_000),
            cand([0.0, 5.0], [-20.0, 5.0], 30_000),
        ];
        assert!(conflict(&a[0], &b[0]));
        assert!(!conflict(&a[1], &b[0]));
        assert_eq!(assign(&[a, b]), (vec![1, 0], vec![]));
    }

    /// A small deterministic generator for the solver's tests.
    fn lcg(seed: &mut u64) -> f64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*seed >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Random leaders fanned around a few sites, short enough to meet often.
    fn random_inventories(seed: &mut u64, jobs: usize, each: usize) -> Vec<Vec<Candidate>> {
        (0..jobs)
            .map(|j| {
                let tip = [10.0 * j as f64, 0.0];
                (0..each)
                    .map(|_| {
                        let elbow = [
                            tip[0] + 30.0 * (lcg(seed) - 0.5),
                            tip[1] + 30.0 * (lcg(seed) - 0.5),
                        ];
                        let mut c = cand(tip, elbow, (lcg(seed) * 30_000.0) as i64);
                        c.crossings = u32::from(lcg(seed) < 0.2);
                        c
                    })
                    .collect()
            })
            .collect()
    }

    fn total(inventories: &[Vec<Candidate>], choice: &[usize]) -> i64 {
        let mut t: i64 = choice
            .iter()
            .zip(inventories)
            .map(|(c, inv)| inv[*c].score())
            .sum();
        for a in 0..choice.len() {
            for b in a + 1..choice.len() {
                if conflict(&inventories[a][choice[a]], &inventories[b][choice[b]]) {
                    t += CROSSING;
                }
            }
        }
        t
    }

    /// The bounded joint assignment equals an exhaustive oracle on small instances (ISSUE-740-a:
    /// the fewest crossings, then the least total length).
    #[test]
    fn the_assignment_matches_an_exhaustive_oracle() {
        let mut seed = 7u64;
        for _ in 0..40 {
            let inv = random_inventories(&mut seed, 4, 4);
            let got = assign(&inv).0;
            let mut best = i64::MAX;
            for k in 0..4usize.pow(4) {
                let choice: Vec<usize> = (0..4).map(|j| k / 4usize.pow(j) % 4).collect();
                best = best.min(total(&inv, &choice));
            }
            assert_eq!(total(&inv, &got), best);
        }
    }

    /// A group too large to search whole stops at [`NODE_BUDGET`] with an assignment no worse
    /// than the greedy floor, the same on every run (ISSUE-740-b).
    #[test]
    fn a_large_group_is_bounded_and_deterministic() {
        let mut seed = 11u64;
        let inv = random_inventories(&mut seed, 30, KEEP);
        let (a, capped) = assign(&inv);
        assert_eq!(a, assign(&inv).0);
        // The bound fired, and that is reported.
        assert_eq!(capped, vec![(0..inv.len()).collect::<Vec<_>>()]);
        // The greedy floor: each job the best given those before it.
        let mut greedy: Vec<usize> = Vec::new();
        for k in 0..inv.len() {
            let pick = (0..inv[k].len())
                .min_by_key(|&c| {
                    inv[k][c].score()
                        + (0..k)
                            .filter(|&j| conflict(&inv[k][c], &inv[j][greedy[j]]))
                            .count() as i64
                            * CROSSING
                })
                .unwrap();
            greedy.push(pick);
        }
        assert!(total(&inv, &a) <= total(&inv, &greedy));
    }

    #[test]
    fn lines_laid_along_each_other_meet() {
        // A leader up an extension line meets it; one beside it, or end to end, does not.
        assert!(lines_meet(
            [[0.0, 0.0], [0.0, 10.0]],
            [[0.1, 5.0], [0.1, 20.0]]
        ));
        assert!(!lines_meet(
            [[0.0, 0.0], [0.0, 10.0]],
            [[2.0, 5.0], [2.0, 20.0]]
        ));
        assert!(!lines_meet(
            [[0.0, 0.0], [0.0, 10.0]],
            [[0.0, 10.0], [0.0, 20.0]]
        ));
    }

    #[test]
    fn crossing_is_strict() {
        assert!(segments_cross(
            [[0.0, 0.0], [2.0, 2.0]],
            [[0.0, 2.0], [2.0, 0.0]]
        ));
        assert!(!segments_cross(
            [[0.0, 0.0], [1.0, 1.0]],
            [[1.0, 1.0], [2.0, 0.0]]
        ));
        assert!(!segments_cross(
            [[0.0, 0.0], [1.0, 0.0]],
            [[2.0, 0.0], [3.0, 0.0]]
        ));
    }
}
