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
//! length (leader and shoulder, plus travel inside the solid bounds to prefer a near exit),
//! in micrometres so sub-micron noise
//! cannot reorder candidates (ISSUE-740-b). Its blockers are counted in two tiers:
//! - **hard** (`_hard_fixed_blockers`): ink off the drawing area; the label or a line in the
//!   title block; the label or shoulder over any view (the exterior region); the leader across
//!   another view, or back into its own view's box after leaving it; a leader longer than
//!   [`MAX_LEADER`];
//! - **crossings** (Policy B, ADR2-D6): one per other item met and kind of contact, as the
//!   plan's collision checks report one problem per pair and code: an item whose line the
//!   leader or shoulder crosses or runs along, and (again) one whose ink and the label meet (its
//!   line through the label, the label on its label, a leader line through its label);
//!   each other feature footprint crossed by the shaft and ink outside the reserved area also
//!   count in this tier. The original assignment is retained when the measured candidates and their pair
//!   contacts find more annotation defects in the new assignment;
//!   the dimensions are placed first and fixed (ARCH0014-3: principal dimensions claim their
//!   strips; the callouts yield);
//! - **text**: of the items met, those whose ink and the label meet, below the crossings:
//!   between as many crossings, a leader across a line is kept over text on a line (draftwright
//!   protects text both ways, `annotation_text_ink_clear`);
//!
//! The original assignment keeps its area tier below text; the footprint-aware assignment
//! counts it with crossings, because either contact is an annotation defect.
//!
//! The jobs are then assigned jointly (ISSUE-740-a, `_assign_by_view`): each takes one candidate,
//! minimising first the hard blockers, then the crossings (with the fixed ink and between the
//! chosen leaders, where one leader's ink on another's label counts two: [`conflict`]), then the
//! text touched, then the ink out of its area, then the total length.
//! The jobs split into independent groups that could conflict; each group is solved exactly by a
//! branch and bound whose work is bounded ([`NODE_BUDGET`]), from a greedy start that is its
//! floor when the budget runs out. Nothing is dropped: a job whose every candidate is blocked
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
//! draftwright's `--leader-region exterior`. Feature footprints from the Analysis count crossings inside the box, shared with the
//! plan collision checks. The incomplete principal outer wires use solid bounds for re-entry
//! and distance to an exit; projected material remains unavailable (ADR2-8). Arrowheads
//! are tested by their shafts (ISSUE-367-a).

use std::collections::BTreeMap;

use super::PlannedDimension;
use super::footprints::Footprints;
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
/// Each tier's weight exceeds the most the tier below it reaches on one candidate.
const HARD: i64 = 1_000_000_000_000_000;
const CROSSING: i64 = 1_000_000_000_000;
/// The weight of ink outside the view's reserved area (its box and bands): below a crossing,
/// above any length (a metre, in micrometres).
const AREA: i64 = 1_000_000;
/// Between as many crossings, the fewer through text (draftwright protects text both ways,
/// `annotation_text_ink_clear`, and keeps shaft crossings under Policy B): above the area.
/// A candidate meets text of fewer than a thousand items, so this stays below one crossing.
const TEXT_INK: i64 = 1_000 * AREA;
/// The clear-route measurement (ISSUE-1187-a, after draftwright's test helper `_clear_routes`,
/// which no placer uses): from each of a callout's [`SWEEP_SITES`] sites nearest its leader's
/// tip, the leader leaves radially in [`SWEEP_DIRECTIONS`] directions, its elbow at each of
/// [`SWEEP_REACH`] from the rim, mm.
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
    feature_crossings: u32,
    inside: i64,
    aware: bool,
    /// Of its crossings, those that touch text: a line through a label, or two labels together.
    text: u32,
    /// Contacts with the text pad beside fixed extension lines.
    contacts: u32,
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
            + i64::from(self.contacts.min(1)) * (AREA / 2)
            + if self.aware {
                i64::from(self.crossings + self.outside + self.feature_crossings) * CROSSING
                    + i64::from(self.text) * TEXT_INK
                    + self.cost
                    + self.inside
            } else {
                i64::from(self.crossings) * CROSSING
                    + i64::from(self.text) * TEXT_INK
                    + i64::from(self.outside) * AREA
                    + self.cost
            }
    }

    /// Whether it is clear: no hard blocker and nothing crossed.
    fn clear(&self) -> bool {
        self.hard == 0 && self.crossings == 0 && (!self.aware || self.feature_crossings == 0)
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
    extensions: Vec<bool>,
    segments: Vec<(usize, [P2; 2])>,
    labels: Vec<(usize, [f64; 4])>,
    /// The arrowheads' outlines as `dims::render` draws them ([`head`]), which must stay out of
    /// the label: a flare can reach into it beside a shaft that does not.
    heads: Vec<(usize, [P2; 2])>,
    /// Where each of them lies, so a candidate tests only those near it.
    grid: Grid,
}

impl Fixed {
    /// The fixed ink of the placed items *on* (each with its key).
    fn new<'p>(on: impl Iterator<Item = (usize, &'p Placed)> + Clone) -> Fixed {
        let extensions = on.clone().flat_map(|(_,p)| {
            let linear=p.lines.len()>=3 && p.arrows.len()==2;
            p.lines.iter().enumerate().map(move |(j,_)|linear && j<2)
                .chain(std::iter::repeat_n(false,p.arrows.len()))
        }).collect();
        let segments: Vec<(usize, [P2; 2])> = on
            .clone()
            .flat_map(|(i, p)| p.lines.iter().chain(&p.arrows).map(move |s| (i, *s)))
            .collect();
        let labels: Vec<(usize, [f64; 4])> = on.clone().map(|(i, p)| (i, p.label_box)).collect();
        let heads: Vec<(usize, [P2; 2])> = on
            .flat_map(|(i, p)| p.arrows.iter().flat_map(move |a| head(*a).map(|e| (i, e))))
            .collect();
        let line_box = |s: &(usize, [P2; 2])| {
            let [a, b] = s.1;
            [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[0].max(b[0]),
                a[1].max(b[1]),
            ]
        };
        let grid = Grid::new([
            segments.iter().map(line_box).collect(),
            labels.iter().map(|l| l.1).collect(),
            heads.iter().map(line_box).collect(),
        ]);
        Fixed {
            extensions,
            segments,
            labels,
            heads,
            grid,
        }
    }
}

/// A uniform grid over boxes of three kinds (the fixed segments, labels and heads): each cell
/// lists the boxes that meet it, within a hair ([`touches`]), so the boxes a query box meets are
/// among those its cells list (each given once, with some that do not meet it). A box off the
/// grid lies in its edge cells, as does a query. A box that is not finite is listed for every
/// query, and a query that is not finite is given every box, so nothing the tests could pass is
/// left out.
struct Grid {
    origin: P2,
    cell: f64,
    size: [usize; 2],
    cells: [Vec<Vec<u32>>; 3],
    always: [Vec<u32>; 3],
    count: [usize; 3],
    /// The last query that gave each box, so a box listed in several cells is given once.
    seen: [Vec<std::cell::Cell<u64>>; 3],
    query: std::cell::Cell<u64>,
}

fn finite(b: [f64; 4]) -> bool {
    b.iter().all(|v| v.is_finite())
}

/// The grid's cells along its longer side.
const GRID: f64 = 32.0;

impl Grid {
    fn new(boxes: [Vec<[f64; 4]>; 3]) -> Grid {
        let all = boxes.iter().flatten().filter(|b| finite(**b));
        let lo = all
            .clone()
            .fold([f64::INFINITY; 2], |m, b| [m[0].min(b[0]), m[1].min(b[1])]);
        let hi = all.fold([f64::NEG_INFINITY; 2], |m, b| {
            [m[0].max(b[2]), m[1].max(b[3])]
        });
        let (origin, extent) = if lo[0] <= hi[0] && lo[1] <= hi[1] {
            (lo, (hi[0] - lo[0]).max(hi[1] - lo[1]))
        } else {
            ([0.0; 2], 0.0)
        };
        let cell = (extent / GRID).max(1.0);
        let size = [
            ((hi[0] - origin[0]) / cell).floor().clamp(0.0, GRID) as usize + 1,
            ((hi[1] - origin[1]) / cell).floor().clamp(0.0, GRID) as usize + 1,
        ];
        let mut grid = Grid {
            origin,
            cell,
            size,
            cells: std::array::from_fn(|_| vec![Vec::new(); size[0] * size[1]]),
            always: Default::default(),
            count: std::array::from_fn(|k| boxes[k].len()),
            seen: std::array::from_fn(|k| vec![std::cell::Cell::new(0); boxes[k].len()]),
            query: std::cell::Cell::new(0),
        };
        for (kind, list) in boxes.iter().enumerate() {
            for (n, b) in list.iter().enumerate() {
                if !finite(*b) {
                    grid.always[kind].push(n as u32);
                    continue;
                }
                let (xs, ys) = grid.span(*b);
                for y in ys {
                    for x in xs.clone() {
                        grid.cells[kind][y * size[0] + x].push(n as u32);
                    }
                }
            }
        }
        grid
    }

    /// The columns and rows of the cells box *b* (grown by a hair) meets, clamped to the grid. A
    /// box given with its corners swapped is taken as the box they span, which holds every point
    /// the tests could find it meeting.
    fn span(
        &self,
        b: [f64; 4],
    ) -> (
        std::ops::RangeInclusive<usize>,
        std::ops::RangeInclusive<usize>,
    ) {
        const HAIR: f64 = 1e-6;
        let at = |v: f64, o: f64, n: usize| {
            ((v - o) / self.cell).floor().clamp(0.0, (n - 1) as f64) as usize
        };
        let (o, [nx, ny]) = (self.origin, self.size);
        let (x0, x1) = (b[0].min(b[2]), b[0].max(b[2]));
        let (y0, y1) = (b[1].min(b[3]), b[1].max(b[3]));
        (
            at(x0 - HAIR, o[0], nx)..=at(x1 + HAIR, o[0], nx),
            at(y0 - HAIR, o[1], ny)..=at(y1 + HAIR, o[1], ny),
        )
    }

    /// Calls *f* with the index of each box of *kind* listed in the cells *b* meets, once.
    fn near(&self, kind: usize, b: [f64; 4], mut f: impl FnMut(usize)) {
        if !finite(b) {
            (0..self.count[kind]).for_each(f);
            return;
        }
        let query = self.query.get() + 1;
        self.query.set(query);
        let seen = &self.seen[kind];
        self.always[kind].iter().for_each(|&n| f(n as usize));
        let (xs, ys) = self.span(b);
        for y in ys {
            for x in xs.clone() {
                for &n in &self.cells[kind][y * self.size[0] + x] {
                    if seen[n as usize].replace(query) != query {
                        f(n as usize);
                    }
                }
            }
        }
    }
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
    footprints: &'a Footprints,
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
#[allow(clippy::too_many_arguments)]
pub(super) fn place(
    analysis: &crate::model::Analysis,
    cache: &super::footprints::Cache,
    views: &[crate::model::PlannedView],
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    sides: &[Option<Side>],
    placed: &mut [Option<Placed>],
    page: &Page,
) -> Vec<Problem> {
    place_with_obstacles(
        analysis,
        cache,
        views,
        frames,
        items,
        views_of,
        sides,
        placed,
        page,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn place_with_obstacles(
    analysis: &crate::model::Analysis,
    cache: &super::footprints::Cache,
    views: &[crate::model::PlannedView],
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    sides: &[Option<Side>],
    placed: &mut [Option<Placed>],
    page: &Page,
    obstacles: &[PlannedDimension],
) -> Vec<Problem> {
    let on = || {
        placed
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Some((i, p.as_ref()?)))
    };
    let fixed = Fixed::new(
        on().chain(
            obstacles
                .iter()
                .enumerate()
                .filter_map(|(i, d)| Some((items.len() + i, d.placed.as_ref()?))),
        ),
    );
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
    let footprints: BTreeMap<_, _> = views
        .iter()
        .map(|v| (v.id.trim_start_matches("v:"), cache.get(analysis, v)))
        .collect();
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
                footprints: &footprints[view],
                outline: f.outline(),
                area: page.area(view),
                sites: page_sites(f, sites),
                ink_min: ink.min,
                ink_max: ink.max,
                beyond,
            },
        ));
    }
    let (mut inventories, original): (Vec<_>, Vec<_>) = jobs
        .iter()
        .map(|(_, job)| inventory(job, &fixed, page))
        .unzip();
    // Semantic PMI claims its strips after dimensions. Until those strips participate in the
    // joint solve, retain the established objective rather than planning PMI twice as a probe.
    let semantic_pmi = !crate::pmi_lowering::lower(analysis, &|name| frames.contains_key(name))
        .0
        .is_empty();
    let (old_choice, old_capped) = assign(&original);
    let (mut choice, mut capped) = if semantic_pmi {
        (old_choice.clone(), old_capped.clone())
    } else {
        assign(&inventories)
    };
    // Keep the annotation floor using the already measured candidates and their pair contacts.
    // No dimensions, feature projections or dependent PMI placements are rebuilt here.
    let defects = |inv: &[Vec<Candidate>], selected: &[usize]| {
        let mut count = 0;
        for (k, &c) in selected.iter().enumerate() {
            let a = &inv[k][c];
            count += a.hard + a.crossings + a.outside;
            for (j, &other) in selected.iter().enumerate().take(k) {
                count += conflict(a, &inv[j][other]);
            }
        }
        count
    };
    if semantic_pmi || defects(&inventories, &choice) > defects(&original, &old_choice) {
        inventories = original;
        choice = old_choice;
        capped = old_capped;
    }
    let mut problems = Vec::new();
    let named = |i: usize| format!("{} ({})", items[i].id, items[i].terms.join(" "));
    for ((i, job), (inv, c)) in jobs.iter().zip(inventories.iter().zip(choice)) {
        let Some(chosen) = inv.get(c) else { continue };
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
                ..Default::default()
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
                ..Default::default()
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
            ..Default::default()
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

/// An explicit shoulder keeps its station; attach radially to its nearest proven feature site.
pub(super) fn intent_tip(frame: &Frame, sites: &[(super::P3, f64)], elbow: P2) -> P2 {
    rim_tip(nearest(&page_sites(frame, sites), elbow), elbow)
}

/// Keep a prescribed strip and lane. Prefer its station; shift only to reduce feature
/// crossings without increasing contacts with other prescribed pins. Free ink yields later.
pub(super) fn route_intent(
    footprints: &Footprints,
    frame: &Frame,
    w: &Want,
    p: &mut Placed,
    side: crate::placement_intent::Side,
    obstacles: &[(&str, &Placed)],
) {
    let Shape::Callout { sites } = &w.shape else {
        return;
    };
    let Some(&[tip, elbow]) = p.lines.first() else {
        return;
    };
    let before = footprints.crossed([tip, elbow], &w.measurements).len();
    if before == 0 {
        return;
    }
    let k = usize::from(matches!(
        side,
        crate::placement_intent::Side::Left | crate::placement_intent::Side::Right
    ));
    let o = frame.outline();
    let sites = page_sites(frame, sites);
    let contacts = |tip: P2, next: P2| {
        let shift = [next[0] - elbow[0], next[1] - elbow[1]];
        let label = [
            p.label_box[0] + shift[0],
            p.label_box[1] + shift[1],
            p.label_box[2] + shift[0],
            p.label_box[3] + shift[1],
        ];
        let lines = [
            [tip, next],
            [
                next,
                [p.lines[1][1][0] + shift[0], p.lines[1][1][1] + shift[1]],
            ],
        ];
        obstacles
            .iter()
            .filter(|(id, _)| *id != w.id)
            .map(|(_, p)| *p)
            .map(|q| {
                usize::from(
                    boxes_overlap(label, q.label_box)
                        || lines.iter().any(|line| crosses(*line, q.label_box))
                        || q.lines
                            .iter()
                            .chain(&q.arrows)
                            .any(|line| crosses(*line, label)),
                ) + usize::from(lines.iter().any(|line| {
                    q.lines
                        .iter()
                        .chain(&q.arrows)
                        .any(|other| lines_meet(*line, *other))
                }))
            })
            .sum::<usize>()
    };
    let old_contacts = contacts(tip, elbow);
    let mut best = (before, 0.0, tip, elbow);
    for station in stations(elbow[k], o[k].min(elbow[k]), o[k + 2].max(elbow[k])) {
        let mut next = elbow;
        next[k] = station;
        for &site in &sites {
            let tip = rim_tip(site, next);
            let n = footprints.crossed([tip, next], &w.measurements).len();
            let shift = (station - elbow[k]).abs();
            if contacts(tip, next) > old_contacts {
                continue;
            }
            if n < best.0
                || (n == best.0 && (shift < best.1 || (shift == best.1 && station < best.3[k])))
            {
                best = (n, shift, tip, next);
            }
        }
    }
    if best.0 >= before {
        return;
    }
    let delta = [best.3[0] - elbow[0], best.3[1] - elbow[1]];
    for line in p.lines.iter_mut().skip(1) {
        for point in line {
            for k in 0..2 {
                point[k] += delta[k];
            }
        }
    }
    p.lines[0] = [best.2, best.3];
    p.arrows = vec![arrow(best.2, best.3)];
    for (k, shift) in delta.iter().enumerate() {
        p.label_at[k] += shift;
        p.label_box[k] += shift;
        p.label_box[k + 2] += shift;
    }
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
fn inventory(job: &Job, fixed: &Fixed, page: &Page) -> (Vec<Candidate>, Vec<Candidate>) {
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
    let mut measured: Vec<Candidate> = Vec::new();
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
            if let Some(c) = measure(job, tip, elbow, right, fixed, page) {
                measured.push(c);
            }
        }
    }
    let aware = keep(&measured);
    for c in &mut measured {
        c.aware = false;
    }
    (aware, keep(&measured))
}

fn keep(measured: &[Candidate]) -> Vec<Candidate> {
    // Ordered by their keys alone (each key distinct, by its generation order), not moving the
    // candidates themselves.
    let mut order: Vec<_> = measured
        .iter()
        .enumerate()
        .map(|(n, c)| (c.score(), n))
        .collect();
    order.sort_unstable();
    let mut kept: Vec<Candidate> = Vec::new();
    for (_, n) in order {
        let c = &measured[n];
        if kept.len() >= KEEP {
            break;
        }
        let same = kept.iter().any(|k| {
            (0..4).all(|j| (k.label_box[j] - c.label_box[j]).abs() < 0.25)
                && (k.tip[0] - c.tip[0]).abs() < 0.25
                && (k.tip[1] - c.tip[1]).abs() < 0.25
        });
        if !same {
            kept.push(c.clone());
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
    fixed: &Fixed,
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
    let mut contacts = 0;
    let lines = segs.map(Line::new);
    fixed.grid.near(0, near, |n| {
        let (i, f) = &fixed.segments[n];
        if !segment_near(*f, near) {
            return;
        }
        if lines.iter().any(|s| s.meets(*f)) {
            crossed.push(*i);
        }
        // Resolve witness-pad contacts after the existing crossing and area criteria.
        // This keeps dense sheets from trading feature clearance for a text pad.
        if fixed.extensions[n] && crosses(*f, padded) {
            contacts += 1;
        }
        if crosses(*f, lb) {
            text.push(*i);
        }
    });
    fixed.grid.near(2, near, |n| {
        let (i, e) = &fixed.heads[n];
        if segment_near(*e, near) && crosses(*e, lb) {
            text.push(*i);
        }
    });
    let own_head = head(arrow(tip, elbow));
    // A label clear of the padded label and of every line's box is met by neither: the tests
    // below need not run on it.
    let reach = segs.iter().chain(&own_head).flatten().fold(padded, |b, p| {
        [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ]
    });
    fixed.grid.near(1, reach, |n| {
        let (i, l) = &fixed.labels[n];
        if finite(reach) && finite(*l) && !touches(reach, *l) {
            return;
        }
        if boxes_overlap(padded, *l) || segs.iter().chain(&own_head).any(|s| crosses(*s, *l)) {
            text.push(*i);
        }
    });
    // An item both crossed and met by the label counts in each: the plan reports a crossing
    // and a text contact as two problems of the pair.
    for v in [&mut crossed, &mut text] {
        v.sort_unstable();
        v.dedup();
    }
    let crossings = (crossed.len() + text.len()) as u32;
    let feature_crossings = job.footprints.crossing_count(segs[0]) as u32;
    // The ink's extreme points (its arrowhead within a hair of its tip) against the view's area.
    let outside = job.area.is_some_and(|a| {
        [tip, elbow, end, [lb[0], lb[1]], [lb[2], lb[3]]]
            .iter()
            .any(|p| {
                p[0] < a[0] - 1e-6 || p[0] > a[2] + 1e-6 || p[1] < a[1] - 1e-6 || p[1] > a[3] + 1e-6
            })
    });
    let cost = ((shaft + SHOULDER) * 1000.0).round() as i64;
    let inside = (job.footprints.inside_length(segs[0]) * 1000.0).round() as i64;
    Some(Candidate {
        tip,
        elbow,
        end,
        label_at,
        label_box: lb,
        cost,
        hard,
        crossings,
        feature_crossings,
        inside,
        aware: true,
        text: text.len() as u32,
        contacts,
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

/// How many routes of the clear-route sweep (ISSUE-1187-a) the planned callout *id* has that are
/// clear of the page's furniture, the views and every other planned dimension's ink. A
/// measurement, as draftwright's `_clear_routes` is (no placer runs it): Policy B keeps a blocked
/// or crossing leader only when this is zero. None when *id* is not a placed callout of the plan.
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
    let (mut wants, ..) = super::wants(analysis);
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
    let footprints = Footprints::new(analysis, f, view);
    let job = Job {
        view,
        footprints: &footprints,
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
    let fixed = Fixed::new(rest());
    let tip = p.lines.first()?[0];
    Some(
        sweep(&job, tip)
            .into_iter()
            .filter_map(|(tip, elbow, right)| measure(&job, tip, elbow, right, &fixed, &page))
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

/// Whether boxes *a* and *b* meet or come within a hair of each other: a pre-check that every
/// overlap ([`boxes_overlap`]) and every segment through a box ([`crosses`]) within *a* passes.
fn touches(a: [f64; 4], b: [f64; 4]) -> bool {
    const HAIR: f64 = 1e-6;
    a[0] <= b[2] + HAIR && a[2] >= b[0] - HAIR && a[1] <= b[3] + HAIR && a[3] >= b[1] - HAIR
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
    Line::new(a).along(b)
}

/// A line to test against many others ([`lines_meet`]), its direction worked out once.
struct Line {
    a: [P2; 2],
    /// Its unit direction and length; none when it is shorter than a nanometre.
    dir: Option<(P2, f64)>,
}

impl Line {
    fn new(a: [P2; 2]) -> Line {
        let d = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
        let len = d[0].hypot(d[1]);
        let dir = if len < 1e-9 {
            None
        } else {
            Some(([d[0] / len, d[1] / len], len))
        };
        Line { a, dir }
    }

    /// [`lines_meet`] of this line and *b*.
    fn meets(&self, b: [P2; 2]) -> bool {
        segments_cross(self.a, b) || self.along(b)
    }

    /// [`segments_along`] of this line and *b*.
    fn along(&self, b: [P2; 2]) -> bool {
        let (a, Some((u, len))) = (self.a, self.dir) else {
            return false;
        };
        let off = |p: P2| (p[0] - a[0][0]) * u[1] - (p[1] - a[0][1]) * u[0];
        let at = |p: P2| (p[0] - a[0][0]) * u[0] + (p[1] - a[0][1]) * u[1];
        if off(b[0]).abs() > ALONG || off(b[1]).abs() > ALONG {
            return false;
        }
        let (t0, t1) = (at(b[0]).min(at(b[1])), at(b[0]).max(at(b[1])));
        t1.min(len) - t0.max(0.0) >= ALONG_RUN
    }
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

/// How badly two candidates' ink conflicts (`_candidate_conflict`), in crossings: two when it
/// touches text (labels within [`LABEL_PAD`] of each other, or a line of one through the other's
/// label: a hard finding, `annotation_ink_overlap`), one when only their lines cross (Policy B
/// keeps that, `annotation_leader_crossing`), none when they are clear.
fn conflict(a: &Candidate, b: &Candidate) -> u32 {
    let grow = |x: [f64; 4]| {
        [
            x[0] - LABEL_PAD,
            x[1] - LABEL_PAD,
            x[2] + LABEL_PAD,
            x[3] + LABEL_PAD,
        ]
    };
    if !boxes_overlap(grow(a.bounds), grow(b.bounds)) {
        return 0;
    }
    if boxes_overlap(grow(a.label_box), b.label_box) {
        return TEXT_CONFLICT;
    }
    let (sa, sb) = (a.segments(), b.segments());
    if sa.iter().any(|s| crosses(*s, b.label_box)) || sb.iter().any(|s| crosses(*s, a.label_box)) {
        TEXT_CONFLICT
    } else {
        u32::from(sa.iter().any(|s| sb.iter().any(|t| lines_meet(*s, *t))))
    }
}

/// The crossings a conflict that touches text counts (see [`conflict`]): two, so two leaders
/// cross rather than one runs through the other's label, as draftwright ranks them (a text
/// contact is in `_HARD_LAYOUT_CODES`, a leader crossing is Policy B info). Only between two
/// leaders, which can both move: weighing the fixed ink's text the same was measured and moves
/// more leaders across lines than it clears off labels (`docs/quality-2026-10-09.md`, "Accepted
/// regressions").
const TEXT_CONFLICT: u32 = 2;

/// The joint assignment: one candidate index per inventory (none for an empty one), minimising the
/// sum of the chosen candidates' scores plus [`CROSSING`] per crossing between the chosen
/// ([`conflict`]). Jobs are grouped by possible conflicts; each group starts from its greedy
/// assignment (jobs in order, each the best given those before it) and is then searched exactly,
/// best first, up to [`NODE_BUDGET`] nodes; the best found is kept, and the groups whose search
/// hit that bound are returned too (their members' job indices). Deterministic: ties keep the earlier job's and candidate's
/// order.
fn assign(inventories: &[Vec<Candidate>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let n = inventories.len();
    // Pairwise conflicts, per pair of jobs that have any.
    let mut pairs: BTreeMap<(usize, usize), Vec<Vec<u32>>> = BTreeMap::new();
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
            let m: Vec<Vec<u32>> = inventories[a]
                .iter()
                .map(|ca| inventories[b].iter().map(|cb| conflict(ca, cb)).collect())
                .collect();
            if m.iter().flatten().any(|x| *x > 0) {
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

/// The conflicts between a group's jobs, dense: for jobs *k* and *l* of the group (local
/// indices), `of[k * m + l][w][c]` holds, bit *d*, whether candidate *c* of *k* and candidate *d*
/// of *l* conflict by more than *w* crossings, one layer per crossing up to the pair's most;
/// none for a pair that never does. Inventories hold at most [`KEEP`] candidates, so one word
/// covers each.
struct Conflicts {
    m: usize,
    of: Vec<Option<Vec<Vec<u64>>>>,
}

const _: () = assert!(KEEP <= 64);

impl Conflicts {
    fn new(
        members: &[usize],
        sizes: &[usize],
        pairs: &BTreeMap<(usize, usize), Vec<Vec<u32>>>,
    ) -> Conflicts {
        let m = members.len();
        let mut of = vec![None; m * m];
        for k in 0..m {
            for l in (0..m).filter(|&l| l != k) {
                let (a, b) = (members[k], members[l]);
                let Some(matrix) = pairs.get(&(a.min(b), a.max(b))) else {
                    continue;
                };
                let met = |c: usize, d: usize| if a < b { matrix[c][d] } else { matrix[d][c] };
                let most = matrix.iter().flatten().copied().max().unwrap_or(0);
                of[k * m + l] = Some(
                    (0..most)
                        .map(|layer| {
                            (0..sizes[k])
                                .map(|c| {
                                    (0..sizes[l])
                                        .filter(|&d| met(c, d) > layer)
                                        .fold(0, |w, d| w | 1 << d)
                                })
                                .collect()
                        })
                        .collect(),
                );
            }
        }
        Conflicts { m, of }
    }

    fn any(&self, k: usize, l: usize) -> bool {
        self.of[k * self.m + l].is_some()
    }

    /// [`CROSSING`] per crossing by which candidate *c* of job *k* and candidate *d* of job *l*
    /// conflict.
    fn cost(&self, k: usize, c: usize, l: usize, d: usize) -> i64 {
        match &self.of[k * self.m + l] {
            Some(layers) => {
                layers.iter().filter(|rows| rows[c] >> d & 1 == 1).count() as i64 * CROSSING
            }
            None => 0,
        }
    }
}

fn solve_group(
    members: &[usize],
    inventories: &[Vec<Candidate>],
    pairs: &BTreeMap<(usize, usize), Vec<Vec<u32>>>,
) -> (Vec<usize>, bool) {
    let m = members.len();
    let scores: Vec<Vec<i64>> = members
        .iter()
        .map(|&i| inventories[i].iter().map(Candidate::score).collect())
        .collect();
    let sizes: Vec<usize> = scores.iter().map(Vec::len).collect();
    let conflicts = Conflicts::new(members, &sizes, pairs);
    let pair_cost = |k: usize, c: usize, l: usize, d: usize| conflicts.cost(k, c, l, d);
    // Greedy floor.
    let mut best: Vec<usize> = Vec::with_capacity(m);
    let mut best_total = 0i64;
    for (k, row) in scores.iter().enumerate() {
        let mut pick = (i64::MAX, 0usize);
        for (c, s) in row.iter().enumerate() {
            let t = s + (0..k).map(|j| pair_cost(k, c, j, best[j])).sum::<i64>();
            if t < pick.0 {
                pick = (t, c);
            }
        }
        best.push(pick.1);
        best_total += if row.is_empty() { 0 } else { pick.0 };
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
            .map(|j| pair_cost(k, c, j, best[j]))
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
                if !conflicts.any(k, l) {
                    continue;
                }
                // The others' part of each job's cost, which the pair's own choice leaves alone.
                let rest_k: Vec<i64> = (0..scores[k].len())
                    .map(|c| others(k, c, l, &best))
                    .collect();
                let rest_l: Vec<i64> = (0..scores[l].len())
                    .map(|d| others(l, d, k, &best))
                    .collect();
                let joint = |c: usize, d: usize| {
                    scores[k][c] + scores[l][d] + pair_cost(k, c, l, d) + rest_k[c] + rest_l[d]
                };
                let now = joint(best[k], best[l]);
                let mut pick = (now, best[k], best[l]);
                for c in 0..scores[k].len() {
                    for d in 0..scores[l].len() {
                        let t = joint(c, d);
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
    // Exact search, bounded.
    let mut search = Search::new(&scores, &conflicts, best, best_total);
    search.from(0, 0);
    (search.best, search.nodes >= NODE_BUDGET)
}

/// The joint assignment's exact search, best first and bounded by [`NODE_BUDGET`] nodes. At each
/// job its candidates are taken in order of their score plus [`CROSSING`] per crossing with the
/// candidates chosen before (ties by index), generated lazily: the candidates with the same
/// number of crossings are in their ranked order already, so the next is the least of each
/// count's first, and a job whose bound fails after a few never orders the rest.
struct Search {
    /// Per job, its candidates' indices in order of (score, index), and their scores so.
    ranked: Vec<Vec<usize>>,
    ranked_scores: Vec<Vec<i64>>,
    /// Per job, each earlier job it can conflict with, once per layer of [`Conflicts`], and per
    /// candidate of that job, the ranks of this job's candidates it conflicts with by more than
    /// that layer.
    before: Vec<Vec<(usize, Vec<u64>)>>,
    /// The remaining jobs' best scores, from each job on.
    rest: Vec<i64>,
    /// Per job, scratch: its ranks not yet taken, by their number of conflicts.
    rem: Vec<Vec<u64>>,
    current: Vec<usize>,
    best: Vec<usize>,
    best_total: i64,
    nodes: usize,
    /// A digest of the nodes tried, in order (the tests compare it).
    #[cfg(test)]
    trail: u64,
}

impl Search {
    /// The search of a group with these *scores* and *conflicts*, from the incumbent *best*.
    fn new(
        scores: &[Vec<i64>],
        conflicts: &Conflicts,
        best: Vec<usize>,
        best_total: i64,
    ) -> Search {
        let m = scores.len();
        let sizes: Vec<usize> = scores.iter().map(Vec::len).collect();
        // The remaining jobs' best scores bound what is left.
        let floor: Vec<i64> = scores
            .iter()
            .map(|s| s.iter().copied().min().unwrap_or(0))
            .collect();
        let mut rest = vec![0i64; m + 1];
        for k in (0..m).rev() {
            rest[k] = rest[k + 1] + floor[k];
        }
        // Each job's candidates ranked by (score, index), and each earlier conflicting job's
        // conflicts with them as words over those ranks.
        let ranked: Vec<Vec<usize>> = scores
            .iter()
            .map(|s| {
                let mut r: Vec<usize> = (0..s.len()).collect();
                r.sort_by_key(|&c| (s[c], c));
                r
            })
            .collect();
        let (sizes, ranked_ref) = (&sizes, &ranked);
        let before: Vec<Vec<(usize, Vec<u64>)>> = (0..m)
            .map(|k| {
                (0..k)
                    .filter_map(|j| conflicts.of[j * m + k].as_ref().map(|layers| (j, layers)))
                    .flat_map(|(j, layers)| {
                        layers.iter().map(move |rows| {
                            let words = (0..sizes[j])
                                .map(|cj| {
                                    (0..sizes[k])
                                        .filter(|&r| rows[cj] >> ranked_ref[k][r] & 1 == 1)
                                        .fold(0, |x, r| x | 1 << r)
                                })
                                .collect();
                            (j, words)
                        })
                    })
                    .collect()
            })
            .collect();
        Search {
            ranked_scores: (0..m)
                .map(|k| ranked[k].iter().map(|&c| scores[k][c]).collect())
                .collect(),
            rem: before.iter().map(|b| vec![0; b.len() + 2]).collect(),
            ranked,
            before,
            rest,
            current: Vec::with_capacity(m),
            best,
            best_total,
            nodes: 0,
            #[cfg(test)]
            trail: 0,
        }
    }

    fn from(&mut self, k: usize, total: i64) {
        let m = self.ranked.len();
        if k == m {
            if total < self.best_total {
                self.best_total = total;
                self.best.clone_from(&self.current);
            }
            return;
        }
        let size = self.ranked[k].len();
        if size == 0 {
            self.current.push(0);
            self.from(k + 1, total);
            self.current.pop();
            return;
        }
        // The ranks with at least n crossings, then with exactly n.
        let rem = &mut self.rem[k];
        rem.fill(0);
        rem[0] = u64::MAX >> (64 - size);
        for (i, (j, words)) in self.before[k].iter().enumerate() {
            let w = words[self.current[*j]];
            for n in (1..=i + 1).rev() {
                rem[n] |= rem[n - 1] & w;
            }
        }
        for n in 0..rem.len() - 1 {
            rem[n] &= !rem[n + 1];
        }
        loop {
            let mut next: Option<(i64, usize, usize, usize)> = None;
            for (n, &w) in self.rem[k].iter().enumerate() {
                if w == 0 {
                    continue;
                }
                let r = w.trailing_zeros() as usize;
                let (t, c) = (
                    self.ranked_scores[k][r] + n as i64 * CROSSING,
                    self.ranked[k][r],
                );
                if next.is_none_or(|(bt, bc, _, _)| (t, c) < (bt, bc)) {
                    next = Some((t, c, n, r));
                }
            }
            let Some((t, c, n, r)) = next else {
                break;
            };
            self.rem[k][n] &= !(1 << r);
            if self.nodes >= NODE_BUDGET {
                return;
            }
            self.nodes += 1;
            #[cfg(test)]
            {
                self.trail = self
                    .trail
                    .wrapping_mul(1_000_003)
                    .wrapping_add((k * 64 + c) as u64);
            }
            if total + t + self.rest[k + 1] >= self.best_total {
                break;
            }
            self.current.push(c);
            self.from(k + 1, total + t);
            self.current.pop();
        }
    }
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
            feature_crossings: 0,
            inside: 0,
            contacts: 0,
            aware: false,
            text: 0,
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
        assert_eq!(conflict(&a[0], &b[0]), 1);
        assert_eq!(conflict(&a[1], &b[0]), 0);
        assert_eq!(assign(&[a, b]), (vec![1, 0], vec![]));
    }

    /// A leader through another's label weighs two crossings, so the assignment takes a leader
    /// crossing a leader before one through a label, though it is longer.
    #[test]
    fn a_leader_through_a_label_weighs_two_crossings() {
        // Job 1's leader runs right, its label at x 18 to 27 on y 5.
        let b = vec![cand([0.0, 5.0], [15.0, 5.0], 20_000)];
        // Job 0's cheapest runs up through that label; its second crosses job 1's leader.
        let a = vec![
            cand([20.0, -5.0], [20.0, 15.0], 20_000),
            cand([10.0, -5.0], [10.0, 15.0], 21_000),
        ];
        assert_eq!(conflict(&a[0], &b[0]), TEXT_CONFLICT);
        assert_eq!(conflict(&a[1], &b[0]), 1);
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
                t += i64::from(conflict(
                    &inventories[a][choice[a]],
                    &inventories[b][choice[b]],
                )) * CROSSING;
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
                            .map(|j| i64::from(conflict(&inv[k][c], &inv[j][greedy[j]])))
                            .sum::<i64>()
                            * CROSSING
                })
                .unwrap();
            greedy.push(pick);
        }
        assert!(total(&inv, &a) <= total(&inv, &greedy));
    }

    /// The search as it was first written: each job's candidates fully sorted by score plus
    /// crossings with those chosen before, then tried in that order.
    #[allow(clippy::too_many_arguments)]
    fn reference_search(
        k: usize,
        total: i64,
        current: &mut Vec<usize>,
        best: &mut Vec<usize>,
        best_total: &mut i64,
        nodes: &mut usize,
        trail: &mut u64,
        scores: &[Vec<i64>],
        rest: &[i64],
        conflicts: &Conflicts,
    ) {
        if k == scores.len() {
            if total < *best_total {
                *best_total = total;
                *best = current.clone();
            }
            return;
        }
        let mut order: Vec<(i64, usize)> = (0..scores[k].len())
            .map(|c| {
                let p: i64 = (0..k).map(|j| conflicts.cost(k, c, j, current[j])).sum();
                (scores[k][c] + p, c)
            })
            .collect();
        order.sort();
        if order.is_empty() {
            current.push(0);
            reference_search(
                k + 1,
                total,
                current,
                best,
                best_total,
                nodes,
                trail,
                scores,
                rest,
                conflicts,
            );
            current.pop();
            return;
        }
        for (t, c) in order {
            if *nodes >= NODE_BUDGET {
                return;
            }
            *nodes += 1;
            *trail = trail
                .wrapping_mul(1_000_003)
                .wrapping_add((k * 64 + c) as u64);
            if total + t + rest[k + 1] >= *best_total {
                break;
            }
            current.push(c);
            reference_search(
                k + 1,
                total + t,
                current,
                best,
                best_total,
                nodes,
                trail,
                scores,
                rest,
                conflicts,
            );
            current.pop();
        }
    }

    /// The lazily ordered search visits the same nodes in the same order as the fully sorted one:
    /// the same assignment, total and node count, whether it finishes or stops at its bound, on
    /// random groups with ties, empty inventories and many conflicts.
    #[test]
    fn the_lazy_search_matches_the_sorted_one() {
        let mut seed = 5u64;
        let mut stopped = 0;
        for round in 0..24 {
            let m = 3 + round % 10;
            let scores: Vec<Vec<i64>> = (0..m)
                .map(|k| {
                    let n = if k == 1 && round % 4 == 0 {
                        0
                    } else {
                        1 + (lcg(&mut seed) * KEEP as f64) as usize
                    };
                    (0..n)
                        .map(|_| {
                            (lcg(&mut seed) * 4.0) as i64 * CROSSING / 2
                                + (lcg(&mut seed) * 6.0) as i64
                        })
                        .collect()
                })
                .collect();
            let sizes: Vec<usize> = scores.iter().map(Vec::len).collect();
            let density = 0.05 + 0.3 * lcg(&mut seed);
            let mut pairs = BTreeMap::new();
            for a in 0..m {
                for b in a + 1..m {
                    if lcg(&mut seed) < 0.3 {
                        continue;
                    }
                    // Conflicts of one crossing and of two (text touched).
                    let matrix: Vec<Vec<u32>> = (0..sizes[a])
                        .map(|_| {
                            (0..sizes[b])
                                .map(|_| {
                                    let x = lcg(&mut seed);
                                    u32::from(x < density) + u32::from(x < density / 3.0)
                                })
                                .collect()
                        })
                        .collect();
                    if matrix.iter().flatten().any(|x| *x > 0) {
                        pairs.insert((a, b), matrix);
                    }
                }
            }
            let members: Vec<usize> = (0..m).collect();
            let conflicts = Conflicts::new(&members, &sizes, &pairs);
            // A poor incumbent (each job's first candidate), so the search has work to do.
            let start: Vec<usize> = vec![0; m];
            let start_total = i64::MAX / 2;
            let mut search = Search::new(&scores, &conflicts, start.clone(), start_total);
            search.from(0, 0);
            let (mut best, mut best_total, mut nodes, mut trail) = (start, start_total, 0, 0);
            reference_search(
                0,
                0,
                &mut Vec::new(),
                &mut best,
                &mut best_total,
                &mut nodes,
                &mut trail,
                &scores,
                &search.rest,
                &conflicts,
            );
            stopped += usize::from(nodes >= NODE_BUDGET);
            assert_eq!(
                (search.best, search.best_total, search.nodes, search.trail),
                (best, best_total, nodes, trail),
                "round {round}"
            );
        }
        // Both ends: searches that finish and searches stopped at the bound.
        assert!(stopped > 0 && stopped < 24, "{stopped} of 24 stopped");
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
