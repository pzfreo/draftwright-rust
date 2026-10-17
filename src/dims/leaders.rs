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
//! Each retained candidate is measured analytically (`leader_callout_geometry`): its cost is its ink's
//! length (leader and shoulder, plus travel inside the solid bounds to prefer a near exit),
//! in micrometres so sub-micron noise
//! cannot reorder candidates (ISSUE-740-b). Its blockers are counted in two tiers:
//! - **hard** (`_hard_fixed_blockers`): ink off the drawing area; the label or a line in the
//!   title block; the label or shoulder over any view (the exterior region); the leader across
//!   another view, or back into the part's material after leaving it at its tip (each solid's
//!   box, measured by [`crate::material::reentry`], the predicate lint's
//!   `leader_crosses_silhouette` shares, ADR2-8; its depth is also added to the length); a leader
//!   longer than [`MAX_LEADER`];
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
//! and distance to an exit; a filled projected material field is unavailable (ADR2-8). A
//! label's contacts include the arrowheads' flares, its own and the fixed ink's (ISSUE-367-a);
//! line contacts are tested by the shafts.

use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

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
/// The least angle a leader to a turned step's silhouette makes with it, degrees: closer to
/// parallel, it reads as lying on the part's edge (ISO 128-22 has leaders meet the outline at an
/// angle; it states no figure).
const GRAZING: f64 = 15.0;
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
/// A route escaping fixed ink also tries its tip turned by one lead direction either way
/// ([`ROTATIONS`]) and, per shoulder fraction ([`SHOULDER_STEPS`]), a shorter shelf on its side
/// and on the other: those and the incumbent join the job's [`KEEP`].
const ROTATIONS: [f64; 2] = [-1.0, 1.0];
const SHOULDER_STEPS: [f64; 3] = [0.25, 0.5, 0.75];
/// The joint assignment's work bound per group of conflicting jobs, in search nodes.
pub(super) const NODE_BUDGET: usize = 200_000;
/// The descent passes (single jobs, then conflicting pairs) that improve the greedy start before
/// the search.
const DESCENT: usize = 50;
/// The clearance a callout's label keeps from other labels, mm (draftwright stacks callout rows
/// a text height and two `pad_around_text` apart; half a pad keeps them visibly apart here).
const LABEL_PAD: f64 = 1.0;
/// Extra readable paper for compound fit/tolerance labels beside strokes and arrow flares.
const STROKE_PAD: f64 = 2.0;

/// The first side shelf: full ink, elbow, shoulder, text gap and paper to the border.
pub(super) fn shelf_width(ink: &super::LabelInk) -> f64 {
    ink.max[0] - ink.min[0] + CLEAR + SHOULDER + 1.0 + STROKE_PAD
}

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
/// A leader running along or close beside a centre stroke reads as part of it
/// ([`crate::centrelines::runs_beside`]): avoided at any cost short of one crossing.
const BESIDE: i64 = 100 * TEXT_INK;
/// A leader within [`GRAZING`] of the turned silhouette it points at reads as lying on the
/// part's edge. When a callout's best route grazes, every route but an angled one crossing no
/// more model line-work or features ([`Candidate::astray`]) costs this, as much as 20 mm more
/// ink (micrometres): worth a longer leader, never a crossing, and not worth sending the
/// neighbouring callouts far round to make room.
const GRAZE: i64 = 20_000;
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
    furniture: u32,
    furniture_hard: u32,
    furniture_text: u32,
    /// Near strokes within the desired text clearance, after actual ink contacts.
    clearance: u32,
    /// Of the centre strokes it meets, those it runs along or close beside ([`BESIDE`]).
    beside: u32,
    /// Whether its shaft grazes the turned silhouette it points at (1) or not (0).
    grazing: u32,
    /// For a callout steered off its silhouette ([`inventory`]): 1 unless it does not graze
    /// and crosses no more of its view's model line-work, and no more features, than the
    /// grazing route ([`GRAZE`]); always 0 otherwise.
    astray: u32,
    feature_crossings: u32,
    recess: bool,
    /// Score the actual directed callout contacts in a shared recess solve.
    directed: bool,
    inside: i64,
    features_measured: bool,
    fixed_measured: bool,
    ignore_furniture: bool,
    aware: bool,
    /// Of its crossings, those that touch text: a line through a label, or two labels together.
    text: u32,
    /// Whether its ink leaves its view's reserved area (1) or not (0).
    outside: u32,
    /// The box of all its ink.
    bounds: [f64; 4],
    // Geometry is immutable; scores differ between the assignment variants.
    conflict_ink: OnceCell<Box<ConflictInk>>,
}

#[derive(Clone, Debug)]
struct ConflictInk {
    bounds: [f64; 4],
    label: [f64; 4],
    segments: [[P2; 2]; 2],
    lines: [Line; 2],
}

fn padded(b: [f64; 4]) -> [f64; 4] {
    [
        b[0] - LABEL_PAD,
        b[1] - LABEL_PAD,
        b[2] + LABEL_PAD,
        b[3] + LABEL_PAD,
    ]
}

impl Candidate {
    fn conflict_ink(&self) -> &ConflictInk {
        self.conflict_ink.get_or_init(|| {
            let segments = self.segments();
            Box::new(ConflictInk {
                bounds: padded(self.bounds),
                label: padded(self.label_box),
                segments,
                lines: segments.map(Line::new),
            })
        })
    }

    fn measure_features(&mut self, job: &Job) {
        if !self.features_measured {
            let shaft = [self.tip, self.elbow];
            self.feature_crossings = job.footprints.crossing_count(shaft) as u32;
            self.features_measured = true;
        }
    }

    fn without_furniture(&mut self) {
        self.hard -= self.furniture_hard;
        self.crossings -= self.furniture_text;
        self.text -= self.furniture_text;
        self.furniture = 0;
        self.furniture_hard = 0;
        self.furniture_text = 0;
        self.beside = 0;
    }

    fn segments(&self) -> [[P2; 2]; 2] {
        [[self.tip, self.elbow], [self.elbow, self.end]]
    }

    fn score(&self) -> i64 {
        i64::from(self.hard) * HARD
            + if self.aware {
                i64::from(
                    self.crossings
                        + self.outside
                        + self.feature_crossings * if self.recess { 2 } else { 1 },
                ) * CROSSING
                    + i64::from(self.text) * TEXT_INK
                    + self.cost
                    + self.inside
            } else {
                i64::from(self.crossings) * CROSSING
                    + i64::from(self.text) * TEXT_INK
                    + i64::from(self.outside) * AREA
                    + self.cost
            }
            + i64::from(self.clearance) * (TEXT_INK / 10)
            + i64::from(self.furniture) * 100_000
            + i64::from(self.beside) * BESIDE
            + i64::from(self.astray) * GRAZE
    }

    /// Whether it is clear: no hard blocker and nothing crossed.
    fn clear(&self) -> bool {
        self.hard == 0
            && self.crossings == 0
            && self.furniture == 0
            && self.clearance == 0
            && (!self.aware || self.feature_crossings == 0)
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
    extensions: Vec<(usize, [P2; 2])>,
    extension_index: ExtensionIndex,
    /// The arrowheads' outlines as `dims::render` draws them ([`head`]), which must stay out of
    /// the label: a flare can reach into it beside a shaft that does not.
    heads: Vec<(usize, [P2; 2])>,
    /// Where each of them lies, so a candidate tests only those near it.
    grid: Grid,
    furniture: BTreeMap<usize, bool>,
    /// Of the furniture, the centre marks' crosses.
    crosses: std::collections::BTreeSet<usize>,
}

impl Fixed {
    /// The fixed ink of the placed items *on* (each with its key).
    fn new<'p>(on: impl Iterator<Item = (usize, &'p Placed)> + Clone) -> Fixed {
        let segments: Vec<(usize, [P2; 2])> = on
            .clone()
            .flat_map(|(i, p)| p.lines.iter().chain(&p.arrows).map(move |s| (i, *s)))
            .collect();
        let labels: Vec<(usize, [f64; 4])> = on.clone().map(|(i, p)| (i, p.label_box)).collect();
        let extensions: Vec<_> = on
            .clone()
            .filter(|(_, p)| p.arrows.len() == 2 && p.lines.len() >= 3)
            .flat_map(|(i, p)| p.lines[..2].iter().map(move |s| (i, *s)))
            .collect();
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
        let extension_index = ExtensionIndex::new(extensions.clone());
        Fixed {
            segments,
            labels,
            extensions,
            extension_index,
            heads,
            grid,
            furniture: BTreeMap::new(),
            crosses: Default::default(),
        }
    }
}

// An owner may have two witnesses. Dense inventories usually fit in one
// integer; unusual larger owner IDs retain the general set semantics.
#[derive(Default)]
struct ContactOwners {
    small: u128,
    large: std::collections::BTreeSet<usize>,
}
impl ContactOwners {
    fn insert(&mut self, owner: usize) {
        if owner < 128 {
            self.small |= 1u128 << owner;
        } else {
            self.large.insert(owner);
        }
    }
    fn len(&self) -> usize {
        self.small.count_ones() as usize + self.large.len()
    }
}

struct ExtensionIndex {
    ink: Vec<(usize, [P2; 2])>,
    bounds: Vec<[f64; 4]>,
    grid: Option<Grid>,
}
impl ExtensionIndex {
    fn new(ink: Vec<(usize, [P2; 2])>) -> Self {
        let bounds: Vec<_> = ink
            .iter()
            .map(|(_, e)| grow(segment_bounds(*e), LABEL_PAD))
            .collect();
        let grid = (ink.len() > 64).then(|| Grid::new([bounds.clone(), Vec::new(), Vec::new()]));
        Self { ink, bounds, grid }
    }
    fn contacts(&self, lines: &[[P2; 2]], label: [f64; 4]) -> (usize, usize) {
        let pad = grow(label, LABEL_PAD);
        let reach = lines.iter().flatten().fold(pad, |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        });
        let (mut pairs, mut text) = (ContactOwners::default(), ContactOwners::default());
        let mut measure = |n: usize| {
            let (owner, e) = self.ink[n];
            let on_text = touches_box(e, pad);
            if on_text {
                text.insert(owner);
            }
            if on_text || lines.iter().any(|s| touches_box(*s, self.bounds[n])) {
                pairs.insert(owner);
            }
        };
        if let Some(grid) = &self.grid {
            grid.near(0, reach, measure);
        } else {
            for (n, b) in self.bounds.iter().enumerate() {
                if boxes_meet(*b, reach) {
                    measure(n);
                }
            }
        }
        (pairs.len(), text.len())
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
pub(super) fn head(arrow: [P2; 2]) -> [[P2; 2]; 3] {
    let [tip, tail] = arrow;
    let n = [tail[1] - tip[1], tip[0] - tail[0]];
    let k = super::ARROW_HALF / n[0].hypot(n[1]).max(1e-12);
    let (a, b) = (
        [tail[0] + k * n[0], tail[1] + k * n[1]],
        [tail[0] - k * n[0], tail[1] - k * n[1]],
    );
    [[tip, a], [a, b], [b, tip]]
}

fn boss_profile(w: &Want) -> bool {
    w.views.len() > 1
        && !w.measurements.is_empty()
        && w.measurements
            .iter()
            .all(|m| m.parameter == "boss.diameter")
        && matches!(&w.shape, Shape::Callout { sites } if !sites.is_empty() && sites.iter().all(|(_, r)| *r == 0.0))
}

/// One callout's job: its view's outline, its sites on the page (centre, radius), its label's ink
/// box about its baseline origin, and how far past each edge the strip's dimensions reach.
struct Job<'a> {
    side: Option<crate::placement_intent::Side>,
    view: &'a str,
    footprints: &'a Footprints,
    outline: [f64; 4],
    /// The view's reserved area (box and bands), when the sheet has it.
    area: Option<[f64; 4]>,
    sites: Vec<(P2, f64)>,
    /// For a turned step's diameter, whose sites lie on its silhouette: the silhouette's
    /// direction on the page ([`grazing`]).
    silhouette: Option<P2>,
    /// For such a callout, its view's model line-work ([`view_ink`]): a route across it is
    /// what an angle off the silhouette may not buy ([`Candidate::astray`]).
    model: Option<&'a SegmentIndex>,
    corridors: bool,
    directed: bool,
    short_profile: bool,
    profile_proof: Option<(&'a crate::model::PlannedView, &'a super::interior::Proof)>,
    /// Once [`inventory`] steers the callout off its silhouette: the most model lines and
    /// features an angled route may cross, the grazing route's ([`astray`]).
    steer: std::cell::Cell<Option<(usize, usize)>>,
    ink_min: P2,
    ink_max: P2,
    stroke_pad: f64,
    /// Below, above, left, right: the distance past that edge clear of the strip's dimensions.
    beyond: [f64; 4],
}

/// Compound requirements need paper around their extra signs and smaller stacked figures.
/// Ordinary labels retain their established corridors; all labels still test actual strokes.
fn stroke_clearance(terms: &[String]) -> f64 {
    let qualified = terms
        .iter()
        .flat_map(|t| t.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')')))
        .any(|word| {
            word == "-"
                || word
                    .strip_prefix('±')
                    .is_some_and(|n| n.parse::<f64>().is_ok())
                || (word.starts_with(['+', '-']) && word.parse::<f64>().is_ok())
                || super::tolerance::fit(word).is_ok()
        });
    if qualified { STROKE_PAD } else { 0.0 }
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
    spec: &crate::model::Spec,
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
        spec,
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
    spec: &crate::model::Spec,
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
    if !items.iter().enumerate().any(|(i, w)| {
        matches!(w.shape, Shape::Callout { .. }) && views_of[i].is_some() && placed[i].is_none()
    }) {
        return Vec::new();
    }
    // Candidate retries can reach the same settled inventory. Retain the exact
    // solve only within this immutable Analysis, with every routing input in
    // the key. Frames are derived from these planned views and the Analysis.
    let route_policy =
        crate::planning::lookup::<bool>(analysis, "established-route-policy", &[]).unwrap_or(false);
    let frame_geometry: Vec<_> = frames
        .iter()
        .map(|(name, f)| (name, f.memo_geometry()))
        .collect();
    let canonical = crate::planning::lookup::<bool>(analysis, "canonical-circular-recipes", &[])
        .unwrap_or(false);
    let key = serde_json::to_vec(&(
        &spec.dimensions,
        views,
        frame_geometry,
        canonical,
        items,
        views_of,
        sides,
        &*placed,
        page,
        obstacles,
        route_policy,
    ))
    .expect("leader solve inputs serialize");
    let (next, problems) = crate::planning::memo(analysis, "settled-leader-solve", key, || {
        let mut next = placed.to_vec();
        let problems = place_with_obstacles_uncached(
            analysis, spec, cache, views, frames, items, views_of, sides, &mut next, page,
            obstacles,
        );
        (next, problems)
    });
    placed.clone_from_slice(&next);
    problems
}

#[allow(clippy::too_many_arguments)]
fn place_with_obstacles_uncached(
    analysis: &crate::model::Analysis,
    spec: &crate::model::Spec,
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
    let mut fixed = Fixed::new(
        on().chain(
            obstacles
                .iter()
                .enumerate()
                .filter_map(|(i, d)| Some((items.len() + i, d.placed.as_ref()?))),
        ),
    );
    fixed.furniture = obstacles
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "ink")
        .map(|(i, d)| (items.len() + i, d.basis == "global_axis"))
        .collect();
    fixed.crosses = obstacles
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "ink" && d.basis == "cross")
        .map(|(i, _)| items.len() + i)
        .collect();
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
    // The model line-work of each view with a turned step's diameter callout to place.
    let recess_context = items.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    let models: BTreeMap<_, _> = views
        .iter()
        .filter(|v| {
            let id = v.id.trim_start_matches("v:");
            items.iter().enumerate().any(|(i, w)| {
                views_of[i] == Some(id)
                    && placed[i].is_none()
                    && (frames.get(id).is_some_and(|f| grazing(f, w).is_some())
                        || w.measurements
                            .iter()
                            .any(|m| m.feature.starts_with("f:section_recess:")))
            })
        })
        .filter_map(|v| Some((v.id.trim_start_matches("v:"), cache.model(analysis, v)?)))
        .collect();
    // Semantic PMI uses the legacy floor; its unused footprint-aware baseline need not be
    // sorted. The furniture-aware inventory and the legacy survivors stay fully measured.
    let semantic_pmi = !crate::pmi_lowering::lower(analysis, &|name| frames.contains_key(name))
        .0
        .is_empty();
    let profile_proofs = if items.iter().any(boss_profile) {
        super::interior::proofs(analysis, views)
    } else {
        BTreeMap::new()
    };
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
                side: spec
                    .dimensions
                    .get(&w.id)
                    .and_then(|d| d.place.as_ref())
                    .and_then(crate::placement_intent::side_hint),
                view,
                footprints: &footprints[view],
                outline: f.outline(),
                area: page.area(view),
                sites: page_sites(f, sites),
                silhouette: grazing(f, w),
                model: models.get(view).map(|m| &m.as_ref().0),
                short_profile: boss_profile(w),
                profile_proof: boss_profile(w)
                    .then(|| {
                        let v = views
                            .iter()
                            .find(|v| v.id.trim_start_matches("v:") == view)?;
                        Some((v, profile_proofs.get(&v.id)?.as_ref()))
                    })
                    .flatten(),
                directed: recess_context
                    && !semantic_pmi
                    && !crate::planning::lookup::<bool>(analysis, "established-route-policy", &[])
                        .unwrap_or(false),
                corridors: boss_profile(w)
                    || w.measurements
                        .iter()
                        .any(|m| m.feature.starts_with("f:section_recess:")),
                steer: std::cell::Cell::new(None),
                ink_min: ink.min,
                ink_max: ink.max,
                stroke_pad: stroke_clearance(&w.terms),
                beyond,
            },
        ));
    }
    let measured: Vec<_> = jobs
        .iter()
        .map(|(_, job)| inventory(job, &fixed, page, semantic_pmi))
        .collect();
    let mut inventories: Vec<_> = measured.iter().map(|v| v.0.clone()).collect();
    let mut baseline: Vec<_> = measured.iter().map(|v| v.1.clone()).collect();
    let baseline_original: Vec<_> = measured.iter().map(|v| v.2.clone()).collect();
    // Semantic PMI claims its strips after dimensions. Until those strips participate in the
    // joint solve, retain the established objective rather than planning PMI twice as a probe.
    let defects = |inv: &[Vec<Candidate>], selected: &[usize]| {
        let mut count = 0;
        for (k, &c) in selected.iter().enumerate() {
            let a = &inv[k][c];
            count += a.hard - a.furniture_hard + a.crossings - a.furniture_text + a.outside;
            for (j, &other) in selected.iter().enumerate().take(k) {
                count += conflict(a, &inv[j][other]);
            }
        }
        count
    };
    let (legacy_choice, legacy_capped) = assign(&baseline_original);
    let (mut baseline_choice, mut baseline_capped) = if semantic_pmi {
        (legacy_choice.clone(), legacy_capped.clone())
    } else {
        assign(&baseline)
    };
    if semantic_pmi
        || defects(&baseline, &baseline_choice) > defects(&baseline_original, &legacy_choice)
    {
        baseline = baseline_original;
        baseline_choice = legacy_choice;
        baseline_capped = legacy_capped;
    }
    // Furniture must not buy clearance by sending a leader through another feature.
    // Retain the established route as a candidate, and constrain each job by its prior floor.
    for (k, (_, job)) in jobs.iter().enumerate() {
        let b = &baseline[k][baseline_choice[k]];
        let ceiling = b.hard + b.crossings + b.outside;
        inventories[k].retain(|c| {
            c.feature_crossings <= b.feature_crossings
                && c.hard - c.furniture_hard + c.crossings - c.furniture_text + c.outside <= ceiling
        });
        let right = b.end[0] > b.elbow[0];
        let mut prior = measure(job, b.tip, b.elbow, right, &fixed, page).expect("existing route");
        // Keep the text anchored when escaping fixed ink: nearby rim points, shorter shelves,
        // or the opposite outward side, all measured against the same feature evidence.
        if prior.furniture_hard > 0 {
            let site = nearest(&job.sites, b.elbow);
            if site.1 > 1e-9 {
                let radial = rim_tip(site, b.elbow);
                let u = [radial[0] - site.0[0], radial[1] - site.0[1]];
                for turn in ROTATIONS {
                    // New recess footprints can move a radius leader; keep those retries
                    // radial while preserving the canonical routes on unaffected drawings.
                    if recess_context
                        && items[jobs[k].0]
                            .measurements
                            .iter()
                            .any(|m| m.parameter == "slot_end_radius.radius")
                    {
                        continue;
                    }
                    let (sin, cos) = (turn * std::f64::consts::TAU / DIRECTIONS as f64).sin_cos();
                    let tip = [
                        site.0[0] + cos * u[0] - sin * u[1],
                        site.0[1] + sin * u[0] + cos * u[1],
                    ];
                    if let Some(mut c) = measure_at(job, tip, b.elbow, b.end, right, &fixed, page)
                        && c.feature_crossings <= b.feature_crossings
                    {
                        c.aware = !semantic_pmi;
                        inventories[k].push(c);
                    }
                }
            }
            for offset in SHOULDER_STEPS.map(|f| f * SHOULDER) {
                let elbow = [
                    b.elbow[0] + if right { offset } else { -offset },
                    b.elbow[1],
                ];
                let tip = rim_tip(nearest(&job.sites, elbow), elbow);
                let across = [
                    b.elbow[0] + if right { -offset } else { offset },
                    b.elbow[1],
                ];
                let other_tip = rim_tip(nearest(&job.sites, across), across);
                if let Some(mut c) = measure(job, other_tip, across, !right, &fixed, page)
                    && c.feature_crossings <= b.feature_crossings
                {
                    c.aware = !semantic_pmi;
                    inventories[k].push(c);
                }
                if let Some(mut c) = measure_at(job, tip, elbow, b.end, right, &fixed, page) {
                    if c.feature_crossings > b.feature_crossings {
                        continue;
                    }

                    c.aware = !semantic_pmi;
                    inventories[k].push(c);
                }
            }
        }
        prior.aware = !semantic_pmi;
        inventories[k].push(prior);
    }
    if semantic_pmi {
        for inv in &mut inventories {
            for c in inv {
                c.aware = false;
            }
        }
    }
    let (mut choice, mut capped) = assign(&inventories);
    if defects(&inventories, &choice) > defects(&baseline, &baseline_choice) {
        // Restore the incumbent when the bounded joint solve spends its ordinary ink floor.
        choice = inventories.iter().map(|inv| inv.len() - 1).collect();
        capped = baseline_capped;
    }
    let floor = defects(&baseline, &baseline_choice);
    for _ in 0..DESCENT {
        let before = choice.clone();
        for k in 0..inventories.len() {
            if inventories[k][choice[k]].furniture_hard == 0 {
                continue;
            }
            let ordinary = |c: &Candidate| {
                c.hard - c.furniture_hard + c.crossings - c.furniture_text + c.outside
            };
            let local = |c: &Candidate| {
                ordinary(c)
                    + choice
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != k)
                        .map(|(j, &n)| conflict(c, &inventories[j][n]))
                        .sum::<u32>()
            };
            let ceiling = local(&inventories[k][choice[k]])
                .saturating_add(floor)
                .saturating_sub(defects(&inventories, &choice));
            choice[k] = inventories[k]
                .iter()
                .enumerate()
                .filter(|(_, c)| local(c) <= ceiling)
                .min_by_key(|(n, c)| (c.score() + i64::from(local(c) - ordinary(c)) * CROSSING, *n))
                .map(|(n, _)| n)
                // A remeasured incumbent can exceed the legacy floor after a new
                // corridor is installed. Keep it until a safe improvement exists.
                .unwrap_or(choice[k]);
        }
        if choice == before {
            break;
        }
    }
    let mut problems = Vec::new();
    for ((i, job), (inv, c)) in jobs.iter().zip(inventories.iter().zip(choice)) {
        let Some(chosen) = inv.get(c) else { continue };
        placed[*i] = Some(chosen.placed());
        problems.extend(route_reports(
            &items[*i],
            job.view,
            job.footprints.solids(),
            [chosen.tip, chosen.elbow],
        ));
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

/// Geometry findings are rebuilt when a refinement replaces their route.
fn route_reports(
    item: &Want,
    view_name: &str,
    solids: &[[f64; 4]],
    shaft: [P2; 2],
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let named = super::wording_of(item);
    let view = format!("v:{}", view_name);
    let cut = crate::material::reentry(solids, shaft);
    if cut > 0.0 {
        let subjects = vec![item.id.clone(), view.clone()];
        problems.push(Problem {
                id: format!("p:annotation_line_over_view:{}", subjects.join("|")),
                code: "annotation_line_over_view".into(),
                severity: "warning".into(),
                message: format!(
                    "The leader of {} runs back across its own view, {}, cutting {cut:.1} mm of it: every place it had was blocked.",
                    named,
                    crate::wording::view(&view)
                ),
                subjects,
                ..Default::default()
            });
    }
    let length = (shaft[1][0] - shaft[0][0]).hypot(shaft[1][1] - shaft[0][1]);
    if length > MAX_LEADER {
        let subjects = vec![item.id.clone()];
        problems.push(Problem {
                id: format!("p:annotation_leader_long:{}", subjects.join("|")),
                code: "annotation_leader_long".into(),
                severity: "warning".into(),
                message: format!(
                    "The leader of {} is {length:.1} mm long, over the {MAX_LEADER} mm a leader may run: every shorter place it had was blocked.",
                    named
                ),
                subjects,
                ..Default::default()
            });
    }
    problems
}

/// Retry free callouts hit by furniture placed after the initial joint solve. Only an
/// actual text contact triggers the full side/row/tier search; unaffected routes stay fixed.
pub(crate) fn settle_after_furniture(
    analysis: &crate::model::Analysis,
    spec: &crate::model::Spec,
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
    items: &mut [PlannedDimension],
    obstacles: &[PlannedDimension],
    cache: &super::footprints::Cache,
) -> BTreeMap<String, Vec<Problem>> {
    let contact = |a: &Placed, b: &Placed| {
        boxes_overlap(a.label_box, b.label_box)
            || super::placement::ink_through(a, b.label_box)
            || super::placement::ink_through(b, a.label_box)
    };
    let mut updated = BTreeMap::new();
    let mut wants = None;
    let page = Page::new(analysis, sheet, views);
    for i in 0..items.len() {
        let d = &items[i];
        if d.kind != "callout"
            || spec
                .dimensions
                .get(&d.id)
                .is_some_and(|e| e.place.is_some())
        {
            continue;
        }
        let Some(old) = &d.placed else { continue };
        if !obstacles
            .iter()
            .filter_map(|d| d.placed.as_ref())
            .any(|p| contact(old, p))
        {
            continue;
        }
        let Some(w) = wants
            .get_or_insert_with(|| super::wants(analysis).0)
            .iter()
            .find(|w| w.id == d.id)
        else {
            continue;
        };
        let (Shape::Callout { sites }, Some(v)) = (&w.shape, views.iter().find(|v| v.id == d.view))
        else {
            continue;
        };
        let frame = Frame::new(analysis, v);
        let view = v.id.trim_start_matches("v:");
        let ink = label_ink(&d.terms, font());
        let footprint = cache.get(analysis, v);
        let model = if grazing(&frame, w).is_some() {
            cache.model(analysis, v)
        } else {
            None
        };
        let job = Job {
            side: spec
                .dimensions
                .get(&d.id)
                .and_then(|e| e.place.as_ref())
                .and_then(crate::placement_intent::side_hint),
            view,
            footprints: &footprint,
            outline: frame.outline(),
            area: page.area(view),
            sites: page_sites(&frame, sites),
            directed: false,
            corridors: false,
            short_profile: boss_profile(w),
            profile_proof: None,
            silhouette: grazing(&frame, w),
            model: model.as_ref().map(|m| &m.as_ref().0),
            steer: std::cell::Cell::new(None),
            ink_min: ink.min,
            ink_max: ink.max,
            stroke_pad: stroke_clearance(&d.terms),
            beyond: [CLEAR; 4],
        };
        let others: Vec<_> = items
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, d)| d)
            .chain(obstacles)
            .collect();
        let fixed = Fixed::new(
            others
                .iter()
                .enumerate()
                .filter_map(|(j, d)| Some((j, d.placed.as_ref()?))),
        );
        let [tip, elbow] = old.lines[0];
        let end = old.lines[1][1];
        let Some(before) = measure_at(&job, tip, elbow, end, end[0] >= elbow[0], &fixed, &page)
        else {
            continue;
        };
        let (candidates, ..) = inventory(&job, &fixed, &page, false);
        let Some(next) = candidates
            .into_iter()
            .filter(|c| {
                c.text < before.text
                    && c.hard <= before.hard
                    && c.crossings <= before.crossings
                    && c.feature_crossings <= before.feature_crossings
                    && c.furniture <= before.furniture
                    && page.fits(&c.placed(), view)
            })
            .min_by_key(Candidate::score)
        else {
            continue;
        };
        let mut printed = w.clone();
        printed.terms.clone_from(&d.terms);
        updated.insert(
            d.id.clone(),
            route_reports(&printed, view, footprint.solids(), [next.tip, next.elbow]),
        );
        items[i].placed = Some(next.placed());
    }
    updated
}

/// Local callout refinement after interior placement, with the rest of the ink held fixed
/// (maintainer decision (iii): leaders and their text keep off extension lines). A callout in
/// contact moves to the nearest station of its row with fewer contacts only when nothing a
/// draftsman sees is worse: no longer by over 2 mm, no new crossing of model, hidden-edge or
/// cutting-plane ink, no 2 mm run beside a model or extension line, text and arrowhead on no
/// new extension line, elbow and text off the outline, and no worse on every placement tier.
fn text_conflict(
    i: usize,
    p: &Placed,
    placed: &[Option<Placed>],
    obstacles: &[PlannedDimension],
) -> bool {
    placed
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .filter_map(|(_, p)| p.as_ref())
        .chain(obstacles.iter().filter_map(|d| d.placed.as_ref()))
        .any(|q| {
            boxes_overlap(p.label_box, q.label_box)
                || super::placement::ink_through(p, q.label_box)
                || super::placement::ink_through(q, p.label_box)
        })
}

/// Claim an established exterior route before the new witnesses settle. These
/// are trial preferences, not pins: the complete layout is still adjudicated,
/// interior placement and refinement may repair them, and the other schedules
/// can win if these corridors are unsuitable for the expanded inventory.
#[allow(clippy::too_many_arguments)]
pub(super) fn seed_established(
    analysis: &crate::model::Analysis,
    spec: &crate::model::Spec,
    views: &[crate::model::PlannedView],
    items: &[Want],
    chosen: &[Option<&str>],
    placed: &mut [Option<Placed>],
    page: &Page,
) {
    let key = serde_json::to_vec(&(spec.hole_table, &spec.sheet.projection))
        .expect("presentation inputs serialize");
    let Some(reference) = crate::planning::lookup::<
        Result<crate::model::Plan, crate::model::Invalid>,
    >(analysis, "established-drawing", &key)
    .and_then(Result::ok) else {
        return;
    };
    for (i, w) in items.iter().enumerate() {
        if placed[i].is_some() || !matches!(w.shape, Shape::Callout { .. }) {
            continue;
        }
        let Some(d) = reference
            .dimensions
            .iter()
            .find(|d| d.id == w.id && d.kind == "callout")
        else {
            continue;
        };
        if reference.problems.iter().any(|p| {
            p.subjects.contains(&d.id)
                && matches!(
                    p.code.as_str(),
                    "annotation_line_over_view" | "annotation_leader_long"
                )
        }) {
            continue;
        }
        let Some(p) = translated(&reference, d, views, w) else {
            continue;
        };
        if chosen[i].is_some_and(|v| d.view == format!("v:{v}") && page.fits(&p, v)) {
            placed[i] = Some(p);
        }
    }
}

fn translated(
    reference: &crate::model::Plan,
    d: &PlannedDimension,
    views: &[crate::model::PlannedView],
    w: &Want,
) -> Option<Placed> {
    let before = reference.views.iter().find(|v| v.id == d.view)?;
    let now = views.iter().find(|v| {
        v.id == d.view
            && v.scale == before.scale
            && v.direction == before.direction
            && v.up == before.up
    })?;
    let mut p = super::placement::relabel(d.placed.as_ref()?, &d.terms, w);
    let delta = [now.at[0] - before.at[0], now.at[1] - before.at[1]];
    for q in p.lines.iter_mut().chain(&mut p.arrows).flatten() {
        q[0] += delta[0];
        q[1] += delta[1];
    }
    for (k, shift) in delta.into_iter().enumerate() {
        p.label_at[k] += shift;
        p.label_box[k] += shift;
        p.label_box[k + 2] += shift;
    }
    Some(p)
}

// Exact refinement rank, movement and immutable route geometry.
type RouteStation = ((usize, usize), f64, P2, P2, P2, bool, [f64; 4]);

#[allow(clippy::too_many_arguments)]
pub(super) fn refine(
    analysis: &crate::model::Analysis,
    spec: &crate::model::Spec,
    cache: &super::footprints::Cache,
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&str>],
    placed: &mut [Option<Placed>],
    page: &Page,
    obstacles: &[PlannedDimension],
    proofs: &BTreeMap<String, std::rc::Rc<super::interior::Proof>>,
) -> BTreeMap<String, Vec<Problem>> {
    if analysis.edge_ink.is_none() {
        return BTreeMap::new();
    }
    let reference_key = serde_json::to_vec(&(spec.hole_table, &spec.sheet.projection))
        .expect("presentation inputs serialize");
    // The reference is immutable for this key within the Analysis scope.
    let has_reference =
        crate::planning::lookup::<Result<crate::model::Plan, crate::model::Invalid>>(
            analysis,
            "established-drawing",
            &reference_key,
        )
        .is_some();
    let frame_geometry: Vec<_> = frames
        .iter()
        .map(|(name, f)| (name, f.memo_geometry()))
        .collect();
    let canonical = crate::planning::lookup::<bool>(analysis, "canonical-circular-recipes", &[])
        .unwrap_or(false);
    let proof_views: Vec<_> = proofs.keys().collect();
    let key = serde_json::to_vec(&(
        &spec.dimensions,
        proof_views,
        reference_key,
        has_reference,
        sheet,
        views,
        frame_geometry,
        canonical,
        items,
        views_of,
        &*placed,
        page,
        obstacles,
    ))
    .expect("leader refinement inputs serialize");
    let (next, reports) = crate::planning::memo(analysis, "settled-leader-refinement", key, || {
        let mut next = placed.to_vec();
        let reports = refine_uncached(
            analysis, spec, cache, sheet, views, frames, items, views_of, &mut next, page,
            obstacles, proofs,
        );
        (next, reports)
    });
    placed.clone_from_slice(&next);
    reports
}

#[allow(clippy::too_many_arguments)]
fn refine_uncached(
    analysis: &crate::model::Analysis,
    spec: &crate::model::Spec,
    cache: &super::footprints::Cache,
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&str>],
    placed: &mut [Option<Placed>],
    page: &Page,
    obstacles: &[PlannedDimension],
    proofs: &BTreeMap<String, std::rc::Rc<super::interior::Proof>>,
) -> BTreeMap<String, Vec<Problem>> {
    let mut updated = BTreeMap::new();
    let Some(ink) = analysis.edge_ink.as_ref() else {
        return updated;
    };
    // The linear dimensions' extension lines, each with its owner. Refinement moves only
    // callouts, so these stay fixed throughout.
    let extensions: Vec<(usize, [P2; 2])> = placed
        .iter()
        .enumerate()
        .filter_map(|(j, p)| p.as_ref().map(|p| (j, p)))
        .filter(|(_, p)| p.arrows.len() == 2 && p.lines.len() >= 3)
        .flat_map(|(j, p)| p.lines[..2].iter().map(move |s| (j, *s)))
        .collect();
    let extension_index = ExtensionIndex::new(extensions.clone());
    // Runs beside newly recognised edges need the same bounded repair as extension
    // contacts. Gather each projection once, then test only routes that need repair.
    let repair_models: BTreeMap<_, _> = if items.iter().any(|w| {
        w.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    }) {
        views
            .iter()
            .filter_map(|v| Some((v.id.trim_start_matches("v:"), cache.model(analysis, v)?)))
            .collect()
    } else {
        BTreeMap::new()
    };
    let extension_ink: Vec<_> = extensions.iter().map(|(_, e)| *e).collect();
    let bad_run = |p: &Placed, view: &str| {
        repair_models.get(view).is_some_and(|model| {
            p.lines
                .iter()
                .any(|s| model.as_ref().0.runs_beside(*s) || runs_beside(*s, &extension_ink))
        })
    };
    let centre_lines = |lines: &[[P2; 2]]| {
        lines.iter().any(|line| {
            obstacles
                .iter()
                .filter(|d| d.kind == "ink")
                .filter_map(|d| d.placed.as_ref())
                .flat_map(|p| &p.lines)
                .any(|fixed| crate::centrelines::runs_beside(*line, *fixed))
        })
    };
    let centre_run = |p: &Placed| centre_lines(&p.lines);
    let reference_key = serde_json::to_vec(&(spec.hole_table, &spec.sheet.projection))
        .expect("presentation inputs serialize");
    let established_reference = crate::planning::lookup::<
        Result<crate::model::Plan, crate::model::Invalid>,
    >(analysis, "established-drawing", &reference_key)
    .and_then(Result::ok);
    let bare = established_reference.is_none();
    // Bare API plans do not run sheet recovery. Give their final callout repair
    // an established same-scale recipe without changing the lane solve or its
    // inventory. The final drawing always uses the full original Analysis.
    let reference = established_reference.or_else(|| {
        if !items.iter().any(|w| {
            w.measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
        }) || obstacles.iter().any(|d| d.kind == "callout")
        {
            return None;
        }
        let key = serde_json::to_vec(&(&sheet.size, &sheet.scale, &spec.sheet.projection))
            .expect("bare route inputs serialize");
        crate::planning::memo(analysis, "bare-leader-reference", key, || {
            let mut base = analysis.clone();
            base.features.retain(|f| f.kind != "section_recess");
            let mut recipe = crate::model::Spec::default();
            recipe.sheet.size = Some(sheet.size.clone());
            recipe.sheet.scale = Some(sheet.scale.clone());
            recipe.sheet.projection = spec.sheet.projection.clone();
            crate::model::plan(&base, &recipe).ok()
        })
    });
    let legacy = |i: usize| {
        reference.as_ref().and_then(|p| {
            p.dimensions
                .iter()
                .find(|d| d.id == items[i].id && d.kind == "callout")
        })
    };
    let longer_than_established = |i: usize, p: &Placed| {
        legacy(i)
            .and_then(|d| d.placed.as_ref())
            .is_some_and(|old| length(&p.lines) > length(&old.lines) + 2.0 + 1e-6)
    };
    // A joint assignment can shorten an old leader while sending it through
    // different model edges. Length and annotation contacts alone do not catch
    // that regression; compare the complete physical route with its translated
    // established recipe before deciding whether refinement is necessary.
    let added_material_crossing = |i: usize, p: &Placed| {
        legacy(i)
            .and_then(|d| translated(reference.as_ref()?, d, views, &items[i]))
            .zip(views_of[i].and_then(|view| repair_models.get(view)))
            .is_some_and(|(old, model)| model.as_ref().0.new_crossing(&p.lines, &old.lines))
    };
    let callout_crossing = |i: usize, p: &Placed, placed: &[Option<Placed>]| {
        bare && !repair_models.is_empty()
            && obstacles.iter().all(|d| d.kind != "callout")
            && placed.iter().enumerate().any(|(j, q)| {
                i != j
                    && q.as_ref().is_some_and(|q| {
                        q.arrows.len() == 1
                            && p.lines.iter().any(|s| {
                                q.lines.iter().chain(&q.arrows).any(|t| lines_meet(*s, *t))
                            })
                    })
            })
    };
    // Only a free callout already in contact is refined; most sheets have none, and then the
    // view ink below is never gathered.
    let todo: Vec<usize> = (0..items.len())
        .filter(|&i| {
            matches!(items[i].shape, Shape::Callout { .. })
                && views_of[i].is_some()
                && !spec
                    .dimensions
                    .get(&items[i].id)
                    .is_some_and(|d| d.place.is_some())
                && placed[i].as_ref().is_some_and(|p| {
                    extension_index.contacts(&p.lines, p.label_box).0 > 0
                        || callout_crossing(i, p, placed)
                        || longer_than_established(i, p)
                        || added_material_crossing(i, p)
                        || bad_run(p, views_of[i].unwrap())
                        || (items[i]
                            .measurements
                            .iter()
                            .any(|m| m.feature.starts_with("f:section_recess:"))
                            && (centre_run(p) || text_conflict(i, p, placed, obstacles)))
                })
        })
        .collect();
    if todo.is_empty() {
        return updated;
    }
    // Each view's line-work for the route gates, gathered only once a candidate reaches it.
    let view_ink = |v: &crate::model::PlannedView| view_ink(analysis, ink, v);
    let view_reach: Vec<[f64; 4]> = views
        .iter()
        .map(|v| {
            let b = if v.id == "v:iso" {
                let b = analysis.iso_bounds;
                [
                    v.at[0] + v.scale * b[0],
                    v.at[1] + v.scale * b[1],
                    v.at[0] + v.scale * b[2],
                    v.at[1] + v.scale * b[3],
                ]
            } else {
                Frame::new(analysis, v).outline()
            };
            grow(b, 1.0)
        })
        .collect();
    let gathered: Vec<std::cell::OnceCell<ViewInk>> =
        views.iter().map(|_| std::cell::OnceCell::new()).collect();
    let cuts: Vec<_> = views
        .iter()
        .flat_map(|v| &v.line_work)
        .filter(|w| w.kind == "cutting_plane")
        .flat_map(crate::centrelines::segments)
        .collect();
    let footprints: BTreeMap<_, _> = views
        .iter()
        .map(|v| (v.id.trim_start_matches("v:"), cache.get(analysis, v)))
        .collect();
    for i in todo {
        let w = &items[i];
        let (Shape::Callout { sites }, Some(view), Some(old)) =
            (&w.shape, views_of[i], placed[i].clone())
        else {
            continue;
        };
        let [tip, elbow] = old.lines[0];
        let end = old.lines[1][1];
        let right = end[0] >= elbow[0];
        let frame = &frames[view];
        let page_sites = page_sites(frame, sites);
        let old_length = length(&old.lines);
        let new_recess = w
            .measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"));
        let ceiling = legacy(i)
            .and_then(|d| d.placed.as_ref())
            .map(|p| length(&p.lines) + 2.0);
        let max_length = ceiling.unwrap_or(if new_recess {
            MAX_LEADER
        } else {
            old_length + 2.0
        });
        let established = legacy(i).and_then(|d| translated(reference.as_ref()?, d, views, w));
        let repair_text = new_recess && text_conflict(i, &old, placed, obstacles);
        let clear_new_text = repair_text
            && !w.measurements.is_empty()
            && w.measurements
                .iter()
                .all(|m| m.feature.starts_with("f:section_recess:"));
        let repair_cross = callout_crossing(i, &old, placed);
        let repair_run = repair_cross
            || repair_text
            || longer_than_established(i, &old)
            || added_material_crossing(i, &old)
            || bad_run(&old, view)
            || (w
                .measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))
                && centre_run(&old));
        let label = label_ink(&w.terms, font());
        let before = extension_index.contacts(&old.lines, old.label_box);
        // Every station of the local row, shoulder-only changes first, kept only while it is
        // no longer than the old route by 2 mm and lowers the contacts without the text gaining
        // one: text on an extension line is worse than a leader across it.
        let job = Job {
            side: spec
                .dimensions
                .get(&w.id)
                .and_then(|e| e.place.as_ref())
                .and_then(crate::placement_intent::side_hint),
            stroke_pad: stroke_clearance(&w.terms),
            view,
            footprints: &footprints[view],
            outline: frame.outline(),
            area: page.area(view),
            sites: page_sites.clone(),
            silhouette: grazing(frame, w),
            model: None,
            short_profile: boss_profile(w),
            profile_proof: boss_profile(w)
                .then(|| {
                    let v = views
                        .iter()
                        .find(|v| v.id.trim_start_matches("v:") == view)?;
                    Some((v, proofs.get(&v.id)?.as_ref()))
                })
                .flatten(),
            directed: !repair_models.is_empty()
                && crate::pmi_lowering::lower(analysis, &|name| frames.contains_key(name))
                    .0
                    .is_empty(),
            corridors: boss_profile(w)
                || w.measurements
                    .iter()
                    .any(|m| m.feature.starts_with("f:section_recess:")),
            steer: std::cell::Cell::new(None),
            ink_min: label.min,
            ink_max: label.max,
            beyond: [CLEAR; 4],
        };
        let mut candidates = Vec::new();
        let reach = ROW as i32;
        let shifts = std::iter::once([0, 0]).chain((-reach..=reach).flat_map(|x| {
            (-reach..=reach)
                .filter(move |y| x != 0 || *y != 0)
                .map(move |y| [x, y])
        }));
        let shelf = (end[0] - elbow[0]).abs();
        for shift in shifts {
            let q = [0, 1].map(|k| elbow[k] + f64::from(shift[k]) * STEP);
            let t = if shift == [0, 0] {
                tip
            } else {
                let site = nearest(&page_sites, q);
                if (q[0] - site.0[0]).hypot(q[1] - site.0[1]) <= site.1 {
                    continue;
                }
                rim_tip(site, q)
            };
            if (q[0] - t[0]).hypot(q[1] - t[1]) < MIN_LEADER {
                continue;
            }
            for right in [right, !right] {
                // A shoulder never shrinks below the natural one: a stub reads as part of
                // whatever line the elbow sits near.
                for shoulder in [shelf, shelf + STEP, shelf + 2.0 * STEP] {
                    let e = [q[0] + if right { shoulder } else { -shoulder }, q[1]];
                    if (q[0] - t[0]) * (e[0] - q[0]) < -1e-9 {
                        continue;
                    }
                    let lines = [[t, q], [q, e]];
                    if length(&lines) > max_length + 1e-6 {
                        continue;
                    }
                    let (_, lb) = label_geometry(q, e, right, label.min, label.max);
                    let now = extension_index.contacts(&lines, lb);
                    if now.0 > before.0 || (!repair_run && now.0 == before.0) || now.1 > before.1 {
                        continue;
                    }
                    let moved = (q[0] - elbow[0]).hypot(q[1] - elbow[1])
                        + (e[0] - end[0]).hypot(e[1] - end[1]);
                    candidates.push((now, moved, t, q, e, right, lb));
                }
            }
        }
        if let Some(p) = established
            .as_ref()
            .filter(|p| p.lines.len() == 2 && length(&p.lines) <= max_length + 1e-6)
        {
            let [t, q] = p.lines[0];
            let e = p.lines[1][1];
            let right = e[0] >= q[0];
            let (_, lb) = label_geometry(q, e, right, label.min, label.max);
            let now = extension_index.contacts(&p.lines, lb);
            // Extend or trim the established shaft along its exact supporting
            // line, then keep the standard horizontal shelf. Only ink outside
            // the uncertain material can be new; the route gate proves this.
            if repair_run {
                let dy = q[1] - t[1];
                if dy.abs() > 1e-9 {
                    for step in -28..=28 {
                        let y = q[1] + f64::from(step) * 0.25;
                        let q = [t[0] + (q[0] - t[0]) * (y - t[1]) / dy, y];
                        for shoulder in [0.75, 1.5, SHOULDER] {
                            let e = [q[0] + if right { shoulder } else { -shoulder }, y];
                            let lines = [[t, q], [q, e]];
                            if self::length(&lines) > max_length + 1e-6
                                || (q[0] - t[0]) * (e[0] - q[0]) < -1e-9
                            {
                                continue;
                            }
                            let (_, lb) = label_geometry(q, e, right, label.min, label.max);
                            let contacts = extension_index.contacts(&lines, lb);
                            if contacts.0 <= before.0 && contacts.1 <= before.1 {
                                let moved = (q[0] - elbow[0]).hypot(q[1] - elbow[1])
                                    + (e[0] - end[0]).hypot(e[1] - end[1]);
                                candidates.push((contacts, moved, t, q, e, right, lb));
                            }
                        }
                    }
                }
            }
            if now.0 <= before.0 && now.1 <= before.1 && (repair_run || now.0 < before.0) {
                let moved =
                    (q[0] - elbow[0]).hypot(q[1] - elbow[1]) + (e[0] - end[0]).hypot(e[1] - end[1]);
                candidates.push((now, moved, t, q, e, right, lb));
                // An established shaft may already be the clearest route, with
                // its glyph only a fraction short of the outline's text pad.
                // Keep the shaft and shelf exact; try a small glyph lift with
                // real baseline and bounding-box movement, never a fake box.
                for lift in [0.25, -0.25, 0.5, -0.5] {
                    let lifted = [lb[0], lb[1] + lift, lb[2], lb[3] + lift];
                    let contacts = extension_index.contacts(&p.lines, lifted);
                    if contacts.0 <= before.0 && contacts.1 <= before.1 {
                        candidates.push((contacts, moved + lift.abs(), t, q, e, right, lifted));
                    }
                }
            }
        }
        // Local row moves cannot reach the opposite wall of a small pocket
        // or the opposite rim of a hole. Retry the same bounded radial sweep
        // used by the initial solver, retaining the route-length and ink gates.
        for (t, q, right) in (!repair_models.is_empty())
            .then(|| sweep(&job, tip))
            .into_iter()
            .flatten()
        {
            let e = [q[0] + if right { SHOULDER } else { -SHOULDER }, q[1]];
            let lines = [[t, q], [q, e]];
            if length(&lines) > max_length + 1e-6 {
                continue;
            }
            let (_, lb) = label_geometry(q, e, right, label.min, label.max);
            let now = extension_index.contacts(&lines, lb);
            if now.0 > before.0 || (!repair_run && now.0 == before.0) || now.1 > before.1 {
                continue;
            }
            let moved =
                (q[0] - elbow[0]).hypot(q[1] - elbow[1]) + (e[0] - end[0]).hypot(e[1] - end[1]);
            candidates.push((now, moved, t, q, e, right, lb));
        }
        if clear_new_text {
            // A dense datum tower can leave only a narrow clear row. The radial
            // sweep's discrete reaches may skip it, so also use the initial
            // solver's bounded one-millimetre stations beside the view.
            let o = job.outline;
            let dr = page.drawable();
            for right in [true, false] {
                for off in ESCAPES {
                    let x = if right { o[2] + off } else { o[0] - off };
                    for y in stations(tip[1], o[1].max(dr[1]), o[3].min(dr[3])) {
                        let q = [x, y];
                        let t = rim_tip(nearest(&page_sites, q), q);
                        let e = [x + if right { SHOULDER } else { -SHOULDER }, y];
                        let lines = [[t, q], [q, e]];
                        if length(&lines) > max_length + 1e-6
                            || (q[0] - t[0]).hypot(q[1] - t[1]) < MIN_LEADER
                        {
                            continue;
                        }
                        let (_, lb) = label_geometry(q, e, right, label.min, label.max);
                        let now = extension_index.contacts(&lines, lb);
                        if now.0 > before.0 || now.1 > before.1 {
                            continue;
                        }
                        let moved = (q[0] - elbow[0]).hypot(q[1] - elbow[1])
                            + (e[0] - end[0]).hypot(e[1] - end[1]);
                        candidates.push((now, moved, t, q, e, right, lb));
                    }
                }
            }
        }
        if repair_cross {
            // Concentric diameter callouts can need a fan on opposite sides of
            // their common axis. Reflect the elbow about its real circular site;
            // the route keeps its established length bound and all ink gates.
            let site = nearest(&page_sites, elbow);
            if site.1 > 0.0 {
                for shift in -(ROW as i32)..=ROW as i32 {
                    let q = [2.0 * site.0[0] - elbow[0] + f64::from(shift), elbow[1]];
                    let t = rim_tip(site, q);
                    for right in [true, false] {
                        let shoulder = (end[0] - elbow[0]).abs();
                        let e = [q[0] + if right { shoulder } else { -shoulder }, q[1]];
                        let lines = [[t, q], [q, e]];
                        let (_, lb) = label_geometry(q, e, right, label.min, label.max);
                        let now = extension_index.contacts(&lines, lb);
                        if length(&lines) <= max_length + 1e-6
                            && now.0 <= before.0
                            && now.1 <= before.1
                        {
                            let moved = (q[0] - elbow[0]).hypot(q[1] - elbow[1])
                                + (e[0] - end[0]).hypot(e[1] - end[1]);
                            candidates.push((now, moved, t, q, e, right, lb));
                        }
                    }
                }
            }
        }
        if candidates.is_empty() {
            continue;
        }
        // The line-work near the candidates, so each one is tested against only what it can meet.
        let mut region = old.label_box;
        for (_, _, t, q, e, _, lb) in &candidates {
            for p in [*t, *q, *e, [lb[0], lb[1]], [lb[2], lb[3]]] {
                region = [
                    region[0].min(p[0]),
                    region[1].min(p[1]),
                    region[2].max(p[0]),
                    region[3].max(p[1]),
                ];
            }
        }
        for p in old.lines.iter().flatten() {
            region = [
                region[0].min(p[0]),
                region[1].min(p[1]),
                region[2].max(p[0]),
                region[3].max(p[1]),
            ];
        }
        let region = [
            region[0] - 2.0 * LABEL_PAD,
            region[1] - 2.0 * LABEL_PAD,
            region[2] + 2.0 * LABEL_PAD,
            region[3] + 2.0 * LABEL_PAD,
        ];
        let mut near_model: Vec<[P2; 2]> = Vec::new();
        let mut uncertain: Vec<[f64; 4]> = Vec::new();
        for (k, v) in views.iter().enumerate() {
            if !boxes_meet(view_reach[k], region) {
                continue;
            }
            let (m, u) = gathered[k].get_or_init(|| view_ink(v));
            near_model.extend(m.iter().filter(|s| touches_box(**s, region)));
            uncertain.extend(u);
        }
        let near_cuts: Vec<[P2; 2]> = cuts
            .iter()
            .filter(|s| touches_box(**s, region))
            .copied()
            .collect();
        let near_lines: Vec<[P2; 2]> = near_model
            .iter()
            .copied()
            .chain(
                extensions
                    .iter()
                    .map(|(_, s)| *s)
                    .filter(|s| touches_box(*s, region)),
            )
            .collect();
        let model_index = SegmentIndex::new(near_model);
        let line_index = SegmentIndex::new(near_lines);
        let crossed = |lines: &[[P2; 2]], e: [P2; 2]| lines.iter().any(|s| segments_cross(*s, e));
        // The text and the elbow keep off the outline: a label or an elbow within the label
        // pad of a model line reads as butting the part.
        let on_model = |q: P2, lb: [f64; 4]| {
            let pad = grow(lb, LABEL_PAD);
            let at = grow([q[0], q[1], q[0], q[1]], LABEL_PAD);
            (
                model_index
                    .near(pad)
                    .into_iter()
                    .filter(|s| touches_box(**s, pad))
                    .count(),
                model_index
                    .near(at)
                    .into_iter()
                    .any(|s| touches_box(*s, at)),
            )
        };
        let (old_text_model, old_elbow_model) = on_model(elbow, old.label_box);
        // The arrowhead keeps out of the extension lines it was clear of: a flare over the
        // start of one reads as a contact whatever the shaft does.
        let head_on = |t: P2, q: P2| {
            let flare = head(arrow(t, q));
            extensions
                .iter()
                .filter(|(_, e)| {
                    let b = grow(
                        [
                            e[0][0].min(e[1][0]),
                            e[0][1].min(e[1][1]),
                            e[0][0].max(e[1][0]),
                            e[0][1].max(e[1][1]),
                        ],
                        0.2,
                    );
                    flare.iter().any(|s| touches_box(*s, b))
                })
                .count()
        };
        let old_head = head_on(tip, elbow);
        let historical = established
            .as_ref()
            .map_or(old.lines.as_slice(), |p| p.lines.as_slice());
        let route_safe = |lines: &[[P2; 2]]| {
            (clear_new_text
                || !uncertain
                    .iter()
                    .any(|b| lines.iter().any(|s| adds_unknown_path(*s, *b, historical))))
                && !lines.iter().any(|s| line_index.runs_beside(*s))
                && (clear_new_text || !model_index.new_crossing(lines, historical))
                && !near_cuts
                    .iter()
                    .any(|s| crossed(lines, *s) && !crossed(historical, *s))
        };

        let route_allowed = |(_, _, t, q, e, right, lb): &RouteStation| {
            if clear_new_text {
                let (label_at, _) = label_geometry(*q, *e, *right, label.min, label.max);
                let p = Placed {
                    lines: vec![[*t, *q], [*q, *e]],
                    arrows: vec![arrow(*t, *q)],
                    label_at,
                    label_box: *lb,
                    rotation: 0.0,
                };
                if !page.fits(&p, view)
                    || text_conflict(i, &p, placed, obstacles)
                    || centre_lines(&p.lines)
                {
                    return false;
                }
            }
            let (text_model, elbow_model) = on_model(*q, *lb);
            (!centre_run(&old)
                || !w
                    .measurements
                    .iter()
                    .any(|m| m.feature.starts_with("f:section_recess:"))
                || !centre_lines(&[[*t, *q], [*q, *e]]))
                && text_model <= old_text_model
                && (!elbow_model || old_elbow_model)
                && head_on(*t, *q) <= old_head
                && route_safe(&[[*t, *q], [*q, *e]])
        };
        if clear_new_text {
            candidates.retain(route_allowed);
        }
        if candidates.is_empty() {
            continue;
        }
        // Fewest contacts first, then the least movement: the nearest acceptable route wins.
        if clear_new_text {
            let mut graded: Vec<_> = candidates
                .into_iter()
                .map(|c| (job.footprints.crossing_count([c.2, c.3]), c))
                .collect();
            graded.sort_by(|(na, a), (nb, b)| {
                a.0.cmp(&b.0).then(na.cmp(nb)).then(a.1.total_cmp(&b.1))
            });
            candidates = graded.into_iter().map(|(_, c)| c).collect();
        } else {
            candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        }
        let mut fixed = Fixed::new(
            placed
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .filter_map(|(j, p)| Some((j, p.as_ref()?)))
                .chain(
                    obstacles
                        .iter()
                        .enumerate()
                        .filter_map(|(j, d)| Some((items.len() + j, d.placed.as_ref()?))),
                ),
        );
        fixed.furniture = obstacles
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind == "ink")
            .map(|(j, d)| (items.len() + j, d.basis == "global_axis"))
            .collect();
        fixed.crosses = obstacles
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind == "ink" && d.basis == "cross")
            .map(|(j, _)| items.len() + j)
            .collect();
        let Some(prior) = measure_at(&job, tip, elbow, end, right, &fixed, page) else {
            continue;
        };
        let interior = views
            .iter()
            .find(|v| v.id.trim_start_matches("v:") == view)
            .and_then(|v| {
                proofs
                    .get(&v.id)
                    .filter(|proof| proof.annotation_clear(v, &old, true))
                    .map(|proof| (v, proof))
            });
        let directed_contacts = |p: &Placed| {
            placed
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .filter_map(|(_, q)| q.as_ref())
                .map(|q| {
                    usize::from(
                        p.lines
                            .iter()
                            .any(|s| q.lines.iter().chain(&q.arrows).any(|t| lines_meet(*s, *t))),
                    ) + usize::from(
                        q.arrows.len() == 1
                            && q.lines.iter().any(|s| {
                                p.lines.iter().chain(&p.arrows).any(|t| lines_meet(*s, *t))
                            }),
                    )
                })
                .sum::<usize>()
        };
        let prior_contacts = directed_contacts(&old);
        let acceptable = |c: &Candidate| {
            // The initial search penalises finite extension-corridor contacts
            // in its hard tier. Here they retain their separate no-increase
            // gate above. A new callout may clear real text blockers while
            // retaining fewer such crossings; every physical hard blocker
            // (sheet, view, material re-entry, length and furniture) stays zero.
            let corridors = extension_index
                .contacts(&[[c.tip, c.elbow], [c.elbow, c.end]], c.label_box)
                .0 as u32;
            if c.hard > prior.hard
                || (clear_new_text && c.hard > corridors)
                || c.crossings > prior.crossings
                || (repair_cross && directed_contacts(&c.placed()) >= prior_contacts)
                || (!clear_new_text && c.feature_crossings > prior.feature_crossings)
                || (!clear_new_text && c.outside > prior.outside)
                || c.text > prior.text
                || (!clear_new_text && c.furniture > prior.furniture)
                || c.beside > prior.beside
                || c.grazing > prior.grazing
            {
                return None;
            }
            let p = c.placed();
            if clear_new_text && (!page.fits(&p, view) || text_conflict(i, &p, placed, obstacles)) {
                return None;
            }
            if interior.is_some_and(|(v, proof)| !proof.annotation_clear(v, &p, true))
                && !(new_recess && page.fits(&p, view))
            {
                return None;
            }
            Some(p)
        };
        // Measuring a station against the placed ink is the expensive tier, so it runs last.
        // Filtering after this stable sort retains the original selected route.
        let chosen = candidates
            .iter()
            .filter(|c| clear_new_text || route_allowed(c))
            .find_map(|&(_, _, t, q, e, right, lb)| {
                let mut c = measure_at_bounds(&job, t, q, e, right, page)?;
                let shift = [lb[0] - c.label_box[0], lb[1] - c.label_box[1]];
                if shift != [0.0; 2] {
                    c.label_at[0] += shift[0];
                    c.label_at[1] += shift[1];
                    c.label_box = lb;
                    c.bounds = [
                        c.bounds[0].min(lb[0]),
                        c.bounds[1].min(lb[1]),
                        c.bounds[2].max(lb[2]),
                        c.bounds[3].max(lb[3]),
                    ];
                    if !page.fits(&c.placed(), view) {
                        return None;
                    }
                }
                measure_fixed(&mut c, &job, &fixed);
                c.measure_features(&job);
                acceptable(&c)
            });
        if let Some(p) = chosen {
            updated.insert(
                w.id.clone(),
                route_reports(w, view, job.footprints.solids(), p.lines[0]),
            );
            placed[i] = Some(p);
        }
    }
    updated
}

/// A view's straight line-work and the boxes of its line-work that is not known exactly.
type ViewInk = (Vec<[P2; 2]>, Vec<[f64; 4]>);

/// A view's model line-work on the page (edges and square-on cylinder silhouettes), and the
/// boxes whose ink is not known as lines (the isometric view, section views, other curved faces).
pub(super) fn view_ink(
    analysis: &crate::model::Analysis,
    ink: &crate::model::projected_ink::EdgeInk,
    v: &crate::model::PlannedView,
) -> ViewInk {
    let mut model: Vec<[P2; 2]> = Vec::new();
    let mut uncertain: Vec<[f64; 4]> = Vec::new();
    let frame = Frame::new(analysis, v);
    if v.id == "v:iso" {
        let b = analysis.iso_bounds;
        uncertain.push([
            v.at[0] + v.scale * b[0],
            v.at[1] + v.scale * b[1],
            v.at[0] + v.scale * b[2],
            v.at[1] + v.scale * b[3],
        ]);
        return (model, uncertain);
    }
    if v.cut.is_some() {
        // Section ink needs a cut B-rep: new routes stay outside its envelope.
        uncertain.push(frame.outline());
        return (model, uncertain);
    }
    model.extend(
        ink.edges
            .iter()
            .flat_map(|e| e.windows(2).map(|s| [frame.page(s[0]), frame.page(s[1])])),
    );
    for f in &ink.curved {
        let [lo, hi] = f.bounds;
        let corners: Vec<_> = (0..8)
            .map(|i| [0, 1, 2].map(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] }))
            .collect();
        if let Some(c) = &f.cylinder {
            let norm = c.axis.iter().map(|x| x * x).sum::<f64>().sqrt();
            let axis = c.axis.map(|x| x / norm.max(1e-12));
            let along: f64 = (0..3).map(|k| axis[k] * v.direction[k]).sum();
            if along.abs() > 1.0 - 1e-9 {
                continue;
            }
            if along.abs() < 1e-9 {
                let h: Vec<f64> = corners
                    .iter()
                    .map(|p| (0..3).map(|k| (p[k] - c.origin[k]) * axis[k]).sum())
                    .collect();
                let h0 = h.iter().copied().fold(f64::INFINITY, f64::min);
                let h1 = h.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let w = [
                    axis[1] * v.direction[2] - axis[2] * v.direction[1],
                    axis[2] * v.direction[0] - axis[0] * v.direction[2],
                    axis[0] * v.direction[1] - axis[1] * v.direction[0],
                ];
                for sign in [-1.0, 1.0] {
                    let at = |t| {
                        frame.page(
                            [0, 1, 2].map(|k| c.origin[k] + t * axis[k] + sign * c.radius * w[k]),
                        )
                    };
                    model.push([at(h0), at(h1)]);
                }
                continue;
            }
        }
        let b = corners.into_iter().map(|p| frame.page(p)).fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| {
                [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ]
            },
        );
        uncertain.push(b);
    }
    (model, uncertain)
}

fn length(lines: &[[P2; 2]]) -> f64 {
    lines
        .iter()
        .map(|[a, b]| (b[0] - a[0]).hypot(b[1] - a[1]))
        .sum()
}

fn boxes_meet(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn grow(b: [f64; 4], d: f64) -> [f64; 4] {
    [b[0] - d, b[1] - d, b[2] + d, b[3] + d]
}

fn segment_bounds(s: [P2; 2]) -> [f64; 4] {
    [
        s[0][0].min(s[1][0]),
        s[0][1].min(s[1][1]),
        s[0][0].max(s[1][0]),
        s[0][1].max(s[1][1]),
    ]
}

/// A conservative broad phase for the final route checks. Dense models have
/// thousands of tessellated edge segments; a candidate still receives the exact
/// same predicates, but only against segments whose closed bounds can meet it.
pub(super) struct SegmentIndex {
    bounds: [f64; 4],
    ink: Vec<[P2; 2]>,
    children: Option<Box<[SegmentIndex; 2]>>,
    counts: std::cell::RefCell<std::collections::HashMap<[u64; 4], usize>>,
    besides: std::cell::RefCell<std::collections::HashMap<[u64; 4], bool>>,
    routes: std::cell::RefCell<std::collections::HashMap<[u64; 8], usize>>,
}

impl SegmentIndex {
    pub(super) fn new(mut ink: Vec<[P2; 2]>) -> Self {
        let bounds = ink.iter().map(|s| segment_bounds(*s)).fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            },
        );
        let children = if ink.len() > 16 {
            let axis = usize::from(bounds[3] - bounds[1] > bounds[2] - bounds[0]);
            ink.sort_unstable_by(|a, b| {
                (a[0][axis] + a[1][axis]).total_cmp(&(b[0][axis] + b[1][axis]))
            });
            let other = ink.split_off(ink.len() / 2);
            Some(Box::new([
                Self::new(std::mem::take(&mut ink)),
                Self::new(other),
            ]))
        } else {
            None
        };
        Self {
            bounds,
            ink,
            children,
            counts: std::cell::RefCell::default(),
            besides: std::cell::RefCell::default(),
            routes: std::cell::RefCell::default(),
        }
    }

    fn near(&self, bounds: [f64; 4]) -> Vec<&[P2; 2]> {
        let mut found = Vec::new();
        self.collect(bounds, &mut found);
        found
    }

    pub(super) fn crossings(&self, line: [P2; 2]) -> usize {
        let key = [
            line[0][0].to_bits(),
            line[0][1].to_bits(),
            line[1][0].to_bits(),
            line[1][1].to_bits(),
        ];
        if let Some(&count) = self.counts.borrow().get(&key) {
            return count;
        }
        let count = self.crossings_uncached(line);
        let mut saved = self.counts.borrow_mut();
        if saved.len() >= 8192 {
            saved.clear();
        }
        saved.insert(key, count);
        count
    }

    fn meets_line(&self, line: [P2; 2]) -> bool {
        self.meets_padded_line(line, 1e-9)
    }

    fn meets_padded_line(&self, line: [P2; 2], pad: f64) -> bool {
        // A diagonal witness's bounding rectangle can cover most of a dense
        // view. Clip the actual shaft against each tree node instead; keep
        // closed, slightly widened bounds so even degenerate leaves survive.
        // The final segment crossing predicate is unchanged.
        let bounds = grow(self.bounds, pad);
        let mut interval = [0.0_f64, 1.0_f64];
        for k in 0..2 {
            let d = line[1][k] - line[0][k];
            if d == 0.0 {
                if line[0][k] < bounds[k] || line[0][k] > bounds[k + 2] {
                    return false;
                }
            } else {
                let a = (bounds[k] - line[0][k]) / d;
                let b = (bounds[k + 2] - line[0][k]) / d;
                interval[0] = interval[0].max(a.min(b));
                interval[1] = interval[1].min(a.max(b));
                if interval[0] > interval[1] {
                    return false;
                }
            }
        }
        true
    }

    fn crossings_uncached(&self, line: [P2; 2]) -> usize {
        if !self.meets_line(line) {
            return 0;
        }
        if let Some(children) = &self.children {
            children
                .iter()
                .map(|child| child.crossings_uncached(line))
                .sum()
        } else {
            self.ink
                .iter()
                .filter(|s| segments_cross(line, **s))
                .count()
        }
    }

    // Count each projected edge once, even when both legs cross it. This is
    // the router's original full-scan predicate, queried through the same tree.
    fn route_crossings(&self, lines: [[P2; 2]; 2]) -> usize {
        let key = std::array::from_fn(|i| lines[i / 4][i / 2 % 2][i % 2].to_bits());
        if let Some(&n) = self.routes.borrow().get(&key) {
            return n;
        }
        let n = self.route_crossings_uncached(lines);
        let mut saved = self.routes.borrow_mut();
        if saved.len() >= 8192 {
            saved.clear();
        }
        saved.insert(key, n);
        n
    }

    fn route_crossings_uncached(&self, lines: [[P2; 2]; 2]) -> usize {
        if !lines.iter().any(|line| self.meets_line(*line)) {
            return 0;
        }
        if let Some(children) = &self.children {
            children
                .iter()
                .map(|child| child.route_crossings_uncached(lines))
                .sum()
        } else {
            self.ink
                .iter()
                .filter(|edge| lines.iter().any(|line| segments_cross(*line, **edge)))
                .count()
        }
    }

    // A refinement must not buy a shorter route by crossing a different
    // original edge. Preserve edge identity and the exact leaf predicates.
    fn new_crossing(&self, next: &[[P2; 2]], old: &[[P2; 2]]) -> bool {
        if !next.iter().any(|line| self.meets_line(*line)) {
            return false;
        }
        if let Some(children) = &self.children {
            children.iter().any(|child| child.new_crossing(next, old))
        } else {
            self.ink.iter().any(|edge| {
                next.iter().any(|line| segments_cross(*line, *edge))
                    && !old.iter().any(|line| segments_cross(*line, *edge))
            })
        }
    }

    pub(super) fn crosses_box(&self, bounds: [f64; 4]) -> bool {
        if !boxes_meet(self.bounds, bounds) {
            return false;
        }
        if let Some(children) = &self.children {
            children.iter().any(|child| child.crosses_box(bounds))
        } else {
            self.ink
                .iter()
                .any(|line| crate::dims::placement::crosses(*line, bounds))
        }
    }

    fn collect<'a>(&'a self, bounds: [f64; 4], found: &mut Vec<&'a [P2; 2]>) {
        if !boxes_meet(self.bounds, bounds) {
            return;
        }
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect(bounds, found);
            }
        } else {
            found.extend(
                self.ink
                    .iter()
                    .filter(|s| boxes_meet(segment_bounds(**s), bounds)),
            );
        }
    }

    fn collect_beside(&self, line: [P2; 2], found: &mut Vec<[P2; 2]>) {
        // Any interval within the 0.3 mm perpendicular band must meet
        // these expanded closed bounds, including a nearly diagonal run.
        if !self.meets_padded_line(line, 0.3 + 1e-9) {
            return;
        }
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_beside(line, found);
            }
        } else {
            found.extend(&self.ink);
        }
    }

    fn runs_beside(&self, line: [P2; 2]) -> bool {
        let key = [
            line[0][0].to_bits(),
            line[0][1].to_bits(),
            line[1][0].to_bits(),
            line[1][1].to_bits(),
        ];
        if let Some(&result) = self.besides.borrow().get(&key) {
            return result;
        }
        let mut ink = Vec::new();
        self.collect_beside(line, &mut ink);
        let result = runs_beside(line, &ink);
        let mut saved = self.besides.borrow_mut();
        if saved.len() >= 8192 {
            saved.clear();
        }
        saved.insert(key, result);
        result
    }
}

// The boolean form needs no distinct-owner bookkeeping. A positive owner
// count is exactly one of these unchanged finite-corridor predicates passing.
pub(super) fn meets_extensions(extensions: &[[P2; 2]], lines: &[[P2; 2]], label: [f64; 4]) -> bool {
    let label = grow(label, LABEL_PAD);
    extensions.iter().any(|e| {
        touches_box(*e, label)
            || lines
                .iter()
                .any(|s| touches_box(*s, grow(segment_bounds(*e), LABEL_PAD)))
    })
}

/// A callout's contacts with the extension lines of linear dimensions (decision (iii)):
/// the dimensions whose extension line the leader or the padded label touches, and of those,
/// the ones the padded label touches.
pub(super) fn extension_contacts(
    extensions: &[(usize, [P2; 2])],
    lines: &[[P2; 2]],
    label: [f64; 4],
) -> (usize, usize) {
    let pad = grow(label, LABEL_PAD);
    let mut pairs = std::collections::BTreeSet::new();
    let mut text = std::collections::BTreeSet::new();
    for (j, e) in extensions {
        let b = grow(
            [
                e[0][0].min(e[1][0]),
                e[0][1].min(e[1][1]),
                e[0][0].max(e[1][0]),
                e[0][1].max(e[1][1]),
            ],
            LABEL_PAD,
        );
        let on_text = touches_box(*e, pad);
        if on_text {
            text.insert(*j);
        }
        if on_text || lines.iter().any(|s| touches_box(*s, b)) {
            pairs.insert(*j);
        }
    }
    (pairs.len(), text.len())
}

/// Whether a leader segment runs within 0.3 mm of the given line-work for over 2 mm.
pub(super) fn runs_beside(s: [P2; 2], ink: &[[P2; 2]]) -> bool {
    let mut intervals: Vec<_> = ink.iter().filter_map(|e| along_interval(s, *e)).collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let length = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
    let (mut start, mut stop) = (0.0_f64, 0.0_f64);
    for (lo, hi) in intervals {
        if lo > stop + 1e-9 {
            start = lo;
        }
        stop = stop.max(hi);
        if (stop - start) * length > 2.0 + 1e-6 {
            return true;
        }
    }
    false
}

pub(super) fn touches_box(s: [P2; 2], b: [f64; 4]) -> bool {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        let d = s[1][k] - s[0][k];
        if d.abs() < 1e-12 {
            if s[0][k] < b[k] || s[0][k] > b[k + 2] {
                return false;
            }
        } else {
            let a = (b[k] - s[0][k]) / d;
            let z = (b[k + 2] - s[0][k]) / d;
            lo = lo.max(a.min(z));
            hi = hi.min(a.max(z));
        }
    }
    lo <= hi
}

/// Whether the candidate introduces any path inside an uncertain material box.
/// Collinear extension in empty paper is safe only when its entire clipped
/// material segment was already covered by one established straight segment.
fn adds_unknown_path(s: [P2; 2], b: [f64; 4], old: &[[P2; 2]]) -> bool {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        let d = s[1][k] - s[0][k];
        if d.abs() < 1e-12 {
            if s[0][k] < b[k] || s[0][k] > b[k + 2] {
                return false;
            }
        } else {
            let a = (b[k] - s[0][k]) / d;
            let z = (b[k + 2] - s[0][k]) / d;
            lo = lo.max(a.min(z));
            hi = hi.min(a.max(z));
        }
    }
    if lo > hi {
        return false;
    }
    let inside = [lo, hi].map(|t| [0, 1].map(|k| s[0][k] + t * (s[1][k] - s[0][k])));
    !old.iter().any(|[a, z]| {
        let d = [z[0] - a[0], z[1] - a[1]];
        let n = d[0] * d[0] + d[1] * d[1];
        n > 1e-18
            && inside.iter().all(|p| {
                let w = [p[0] - a[0], p[1] - a[1]];
                let cross = w[0] * d[1] - w[1] * d[0];
                let t = (w[0] * d[0] + w[1] * d[1]) / n;
                cross.abs() <= 1e-9 * n.sqrt() && (-1e-9..=1.0 + 1e-9).contains(&t)
            })
    })
}

/// The interval along a leader within 0.3 paper mm of another finite segment.
/// Clip both its overlap and its perpendicular distance, including near-parallel runs.
fn along_interval(a: [P2; 2], b: [P2; 2]) -> Option<(f64, f64)> {
    if (0..2).any(|k| {
        a[0][k].max(a[1][k]) < b[0][k].min(b[1][k]) - 0.3
            || a[0][k].min(a[1][k]) > b[0][k].max(b[1][k]) + 0.3
    }) {
        return None;
    }
    let d = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
    let w = [b[1][0] - b[0][0], b[1][1] - b[0][1]];
    let n = w[0] * w[0] + w[1] * w[1];
    if n < 1e-18 {
        return None;
    }
    // A transverse 0.6 mm strip cannot hold 2 mm of travel. It must not join
    // neighbouring transverse contacts into an apparent run along a line.
    let length = d[0].hypot(d[1]);
    if (d[0] * w[1] - d[1] * w[0]).abs() > (2.0 * 0.3 / 2.0) * length * n.sqrt() {
        return None;
    }
    let q = [a[0][0] - b[0][0], a[0][1] - b[0][1]];
    let band = 0.3 * n.sqrt();
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for (at, slope, min, max) in [
        (q[0] * w[0] + q[1] * w[1], d[0] * w[0] + d[1] * w[1], 0.0, n),
        (
            q[0] * w[1] - q[1] * w[0],
            d[0] * w[1] - d[1] * w[0],
            -band,
            band,
        ),
    ] {
        if slope.abs() < 1e-15 {
            if at < min || at > max {
                return None;
            }
        } else {
            let a = (min - at) / slope;
            let b = (max - at) / slope;
            lo = lo.max(a.min(b));
            hi = hi.min(a.max(b));
        }
    }
    (hi > lo).then_some((lo, hi))
}

/// The page direction of a turned step's silhouette, for its diameter's callout (*w* with a
/// turning axis), unless the view looks along the axis.
fn grazing(f: &Frame, w: &Want) -> Option<P2> {
    let t = w.turning?;
    let mut e = [0.0; 3];
    e[t] = 1.0;
    let (a, b) = (f.page([0.0; 3]), f.page(e));
    let d = [b[0] - a[0], b[1] - a[1]];
    let n = d[0].hypot(d[1]);
    (n > 1e-9 * f.scale()).then(|| [d[0] / n, d[1] / n])
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

/// A pinned callout whose bend is `offset` (paper mm) from its automatic leader tip, `held`'s
/// first point (#12). The bend may sit inside the view. The leader still attaches radially to the
/// site that tip lies on, so a bend dragged round the feature keeps a radial shaft; the label
/// reads on the side the leader runs to, as an automatic callout's does. At the automatic bend
/// it is the automatic ink itself, whatever that leader's direction. Without an automatic placement in this view the offset is
/// taken from the first site's centre and the leader goes to the nearest site. The pin is never
/// slid: crossings and contacts are reported against it.
pub(super) fn offset_intent(
    frame: &Frame,
    w: &Want,
    offset: P2,
    held: Option<(&Placed, &[String])>,
) -> Option<Placed> {
    let Shape::Callout { sites } = &w.shape else {
        return None;
    };
    let sites = page_sites(frame, sites);
    let held = held.and_then(|(p, terms)| Some((p, *p.lines.first()?, terms)));
    let anchor = match held {
        Some((_, [tip, _], _)) => tip,
        None => sites.first()?.0,
    };
    let elbow = [anchor[0] + offset[0], anchor[1] + offset[1]];
    let near = |a: P2, b: P2| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    if let Some((p, [_, e], terms)) = held
        && near(e, elbow)
    {
        return Some(super::placement::relabel(p, terms, w));
    }
    let tip = match held {
        Some((_, [tip, _], _)) => {
            let on = |s: &(P2, f64)| ((s.0[0] - tip[0]).hypot(s.0[1] - tip[1]) - s.1).abs();
            let site = *sites.iter().min_by(|a, b| on(a).total_cmp(&on(b)))?;
            rim_tip(site, elbow)
        }
        None => rim_tip(nearest(&sites, elbow), elbow),
    };
    let right = if (elbow[0] - tip[0]).abs() > 1e-9 {
        elbow[0] > tip[0]
    } else {
        // A vertical leader keeps the automatic label's side, else reads to the right.
        held.is_none_or(|(p, _, _)| p.lines.get(1).is_none_or(|s| s[1][0] >= s[0][0]))
    };
    let end = [
        elbow[0] + if right { SHOULDER } else { -SHOULDER },
        elbow[1],
    ];
    let ink = label_ink(&w.terms, font());
    let (label_at, label_box) = label_geometry(elbow, end, right, ink.min, ink.max);
    Some(Placed {
        lines: vec![[tip, elbow], [elbow, end]],
        arrows: vec![arrow(tip, elbow)],
        label_at,
        rotation: 0.0,
        label_box,
    })
}

/// Keep a prescribed strip and lane. Prefer its station; shift only to reduce feature
/// crossings without increasing contacts with other prescribed pins. A pin keeps its ink;
/// the finished-plan audit reports its feature crossings. Free ink yields later.
pub(super) fn route_intent(
    footprints: &Footprints,
    frame: &Frame,
    w: &Want,
    p: &mut Placed,
    side: crate::placement_intent::Side,
    obstacles: &[(&str, &Placed)],
    pinned: bool,
) {
    if pinned {
        return;
    }
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
    sites
        .iter()
        .map(|s| (s, (s.0[0] - elbow[0]).hypot(s.0[1] - elbow[1]) - s.1))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(s, _)| *s)
        .expect("a callout has a site")
}

/// The best [`KEEP`] candidates of *job* as if all were fully measured (fewest hard blockers,
/// then crossings, length and generation order), near-duplicates dropped.
type Inventories = (Vec<Candidate>, Vec<Candidate>, Vec<Candidate>);

fn inventory(job: &Job, fixed: &Fixed, page: &Page, semantic_pmi: bool) -> Inventories {
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
    if job.short_profile {
        // A hidden end rim may be represented from its visible profile. Give
        // its real end stations the same short bounded rays as interior jobs,
        // instead of forcing a long column outside the entire view envelope.
        for &site in &job.sites {
            for j in 0..DIRECTIONS {
                let a = std::f64::consts::TAU * j as f64 / DIRECTIONS as f64;
                let u = [a.cos(), a.sin()];
                let tip = [site.0[0] + site.1 * u[0], site.0[1] + site.1 * u[1]];
                for &reach in SWEEP_REACH.iter().take(3) {
                    raw.push((tip, [tip[0] + reach * u[0], tip[1] + reach * u[1]], None));
                }
            }
        }
    }
    if let Some(side) = job.side {
        use crate::placement_intent::Side;
        raw.retain(|(_, elbow, _)| match side {
            Side::Left => elbow[0] < o[0],
            Side::Right => elbow[0] > o[2],
            Side::Above => elbow[1] > o[3],
            Side::Below => elbow[1] < o[1],
        });
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
            if let Some(c) = measure_bounds(job, tip, elbow, right, page) {
                measured.push(c);
            }
        }
    }
    let mut lists = kept(&mut measured, job, fixed, semantic_pmi);
    if job.corridors
        && lists.0.first().is_none_or(|c| {
            c.hard > c.furniture_hard || c.feature_crossings > 0 || c.crossings > c.furniture_text
        })
    {
        measured.extend(
            sweep(job, job.sites[0].0)
                .into_iter()
                .filter_map(|(tip, elbow, right)| measure_bounds(job, tip, elbow, right, page)),
        );
        lists = kept(&mut measured, job, fixed, semantic_pmi);
    }
    // A turned step's diameter whose best route grazes its silhouette takes an angled route
    // instead, but only one that crosses no more of the part's line-work, and passes through
    // no more features, than the grazing one; with none, the grazing route stands.
    if job.model.is_none() {
        return lists;
    }
    let grazing: Vec<&Candidate> = [&lists.0, &lists.1, &lists.2]
        .iter()
        .filter_map(|l| l.first().filter(|c| c.grazing > 0))
        .collect();
    let (Some(edges), Some(features)) = (
        grazing.iter().map(|c| model_crossings(job, c)).min(),
        grazing.iter().map(|c| feature_count(job, c)).min(),
    ) else {
        return lists;
    };
    job.steer.set(Some((edges, features)));
    for c in &mut measured {
        c.astray = astray(job, c);
    }
    if measured.iter().all(|c| c.astray > 0) {
        job.steer.set(None);
        for c in &mut measured {
            c.astray = 0;
        }
        return lists;
    }
    kept(&mut measured, job, fixed, semantic_pmi)
}

/// How many of its view's model lines a route crosses ([`Job::model`]).
fn model_crossings(job: &Job, c: &Candidate) -> usize {
    job.model
        .map_or(0, |model| model.route_crossings(c.segments()))
}

/// How many features a route's shaft passes through.
fn feature_count(job: &Job, c: &Candidate) -> usize {
    job.footprints.crossing_count([c.tip, c.elbow])
}

/// [`Candidate::astray`] for a route of *job*, as steered ([`Job::steer`]).
fn astray(job: &Job, c: &Candidate) -> u32 {
    job.steer.get().map_or(0, |(edges, features)| {
        u32::from(
            c.grazing > 0 || model_crossings(job, c) > edges || feature_count(job, c) > features,
        )
    })
}

/// The furniture-aware, baseline-aware and legacy inventories kept from *measured*.
fn kept(measured: &mut [Candidate], job: &Job, fixed: &Fixed, semantic_pmi: bool) -> Inventories {
    let aware = keep(measured, job, fixed);
    let mut baseline = measured.to_vec();
    for c in &mut baseline {
        c.without_furniture();
        c.ignore_furniture = true;
    }
    let baseline_aware = if semantic_pmi {
        Vec::new()
    } else {
        keep(&mut baseline, job, fixed)
    };
    for c in &mut baseline {
        c.aware = false;
    }
    (aware, baseline_aware, keep(&mut baseline, job, fixed))
}

fn keep(measured: &mut [Candidate], job: &Job, fixed: &Fixed) -> Vec<Candidate> {
    // Merge sorted geometric lower bounds with refined scores. Fixed ink and feature
    // crossings only add to them; generation order still breaks every tie.
    // A fully measured inventory takes the original fast sorted-prefix path.
    let mut order: Vec<_> = measured
        .iter()
        .enumerate()
        .map(|(n, c)| (c.score(), n))
        .collect();
    order.sort_unstable();
    let mut pending: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    let mut next = 0;
    let mut kept: Vec<Candidate> = Vec::new();
    while kept.len() < KEEP {
        let (bound, n) = if next < order.len() && pending.peek().is_none_or(|p| order[next] <= p.0)
        {
            let key = order[next];
            next += 1;
            key
        } else if let Some(Reverse(key)) = pending.pop() {
            key
        } else {
            break;
        };
        let c = &mut measured[n];
        if !c.fixed_measured {
            measure_fixed(c, job, fixed);
            if c.score() != bound {
                pending.push(Reverse((c.score(), n)));
                continue;
            }
        }
        if c.aware && !c.features_measured {
            c.measure_features(job);
            if c.score() != bound {
                pending.push(Reverse((c.score(), n)));
                continue;
            }
        }
        let same = kept.iter().any(|k| {
            (0..4).all(|j| (k.label_box[j] - c.label_box[j]).abs() < 0.25)
                && (k.tip[0] - c.tip[0]).abs() < 0.25
                && (k.tip[1] - c.tip[1]).abs() < 0.25
        });
        if !same {
            // The legacy objective ignores features, but its survivors still supply the
            // crossing ceiling for the furniture-aware solve.
            c.measure_features(job);
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
    let mut c = measure_bounds(job, tip, elbow, right, page)?;
    measure_fixed(&mut c, job, fixed);
    c.measure_features(job);
    Some(c)
}

fn measure_bounds(job: &Job, tip: P2, elbow: P2, right: bool, page: &Page) -> Option<Candidate> {
    let end = [
        elbow[0] + if right { SHOULDER } else { -SHOULDER },
        elbow[1],
    ];
    measure_at_bounds(job, tip, elbow, end, right, page)
}

fn label_geometry(_elbow: P2, end: P2, right: bool, ink_min: P2, ink_max: P2) -> (P2, [f64; 4]) {
    let at_x = if right {
        end[0] + 1.0 - ink_min[0]
    } else {
        end[0] - 1.0 - ink_max[0]
    };
    let label_at = [at_x, end[1] - 0.5 * CAP];
    let ink = super::LabelInk {
        glyphs: Vec::new(),
        strokes: Vec::new(),
        min: ink_min,
        max: ink_max,
    };
    let lb = label_box(&ink, label_at, 0.0);
    (label_at, lb)
}

fn measure_at(
    job: &Job,
    tip: P2,
    elbow: P2,
    end: P2,
    right: bool,
    fixed: &Fixed,
    page: &Page,
) -> Option<Candidate> {
    let mut c = measure_at_bounds(job, tip, elbow, end, right, page)?;
    measure_fixed(&mut c, job, fixed);
    c.measure_features(job);
    Some(c)
}

fn measure_at_bounds(
    job: &Job,
    tip: P2,
    elbow: P2,
    end: P2,
    right: bool,
    page: &Page,
) -> Option<Candidate> {
    let shaft = (elbow[0] - tip[0]).hypot(elbow[1] - tip[1]);
    if shaft < MIN_LEADER {
        return None;
    }
    let (label_at, lb) = label_geometry(elbow, end, right, job.ink_min, job.ink_max);
    let segs = [[tip, elbow], [elbow, end]];
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
    // Material re-entry (ADR2-8): measured by the predicate lint's `leader_crosses_silhouette`
    // shares, it is a hard blocker, and its depth (in micrometres) is added to the length so a
    // shallower cut is preferred when every candidate re-enters.
    let cut = crate::material::reentry(job.footprints.solids(), segs[0]);
    if cut > 0.0 {
        hard += 1;
    }
    if shaft > MAX_LEADER {
        hard += 1;
    }
    // A leader along the silhouette it points at lies on the part's edge: ISO 128-22's leader
    // meets the outline at an angle.
    let grazing = job.silhouette.is_some_and(|u| {
        let d = [elbow[0] - tip[0], elbow[1] - tip[1]];
        (u[0] * d[1] - u[1] * d[0]).abs() < GRAZING.to_radians().sin() * shaft
    });
    let bounds = [
        tip[0].min(elbow[0]).min(end[0]).min(lb[0]),
        tip[1].min(elbow[1]).min(lb[1]),
        tip[0].max(elbow[0]).max(end[0]).max(lb[2]),
        tip[1].max(elbow[1]).max(lb[3]),
    ];
    // The ink's extreme points (its arrowhead within a hair of its tip) against the view's area.
    let outside = job.area.is_some_and(|a| {
        [tip, elbow, end, [lb[0], lb[1]], [lb[2], lb[3]]]
            .iter()
            .any(|p| {
                p[0] < a[0] - 1e-6 || p[0] > a[2] + 1e-6 || p[1] < a[1] - 1e-6 || p[1] > a[3] + 1e-6
            })
    });
    let cost =
        ((shaft + (end[0] - elbow[0]).hypot(end[1] - elbow[1]) + cut) * 1000.0).round() as i64;
    // The material's boxes are cheap; resolving this length now keeps the score bound tight.
    let inside = (job.footprints.inside_length(segs[0]) * 1000.0).round() as i64;
    let mut c = Candidate {
        tip,
        elbow,
        end,
        label_at,
        label_box: lb,
        cost,
        hard,
        crossings: 0,
        furniture: 0,
        furniture_hard: 0,
        furniture_text: 0,
        clearance: 0,
        beside: 0,
        grazing: u32::from(grazing),
        astray: 0,
        feature_crossings: 0,
        recess: job.corridors && !job.short_profile,
        directed: job.directed,
        inside,
        features_measured: false,
        fixed_measured: false,
        ignore_furniture: false,
        aware: true,
        text: 0,
        outside: u32::from(outside),
        bounds,
        conflict_ink: OnceCell::new(),
    };
    c.astray = astray(job, &c);
    if let Some((view, proof)) = job.profile_proof {
        // An invented or hidden station is never a diameter attachment, even
        // when every available real route has another reported obstruction.
        if !proof.touches(view, tip) {
            return None;
        }
        c.hard += u32::from(!proof.profile_clear(view, &c.placed()));
    }
    Some(c)
}

// Annotation contacts are the expensive tier. Every retained candidate is fully measured;
// lower bounds allow candidates that cannot enter the sorted prefix to skip these tests.
fn measure_fixed(c: &mut Candidate, job: &Job, fixed: &Fixed) {
    let (tip, elbow, end, lb, bounds) = (c.tip, c.elbow, c.end, c.label_box, c.bounds);
    let segs = [[tip, elbow], [elbow, end]];
    let mut hard = 0;
    let padded = [
        lb[0] - LABEL_PAD,
        lb[1] - LABEL_PAD,
        lb[2] + LABEL_PAD,
        lb[3] + LABEL_PAD,
    ];
    let stroke_box = grow(lb, job.stroke_pad);
    let near = grow(bounds, job.stroke_pad.max(LABEL_PAD));
    // The items whose lines the leader crosses, and those whose ink and the label's text meet
    // (a line through either label, or the labels together): each pair one problem, as the
    // plan's collision checks report them.
    let mut crossed: Vec<usize> = Vec::new();
    let mut text: Vec<usize> = Vec::new();
    let mut clearance = Vec::new();
    let mut furniture = Vec::new();
    let mut furniture_hard = 0;
    let mut furniture_text = Vec::new();
    let mut besides = Vec::new();
    let lines = segs.map(Line::new);
    fixed.grid.near(0, near, |n| {
        let (i, f) = &fixed.segments[n];
        if !segment_near(*f, near) {
            return;
        }
        // Exact collinear travel stays hard; a shaft's near-parallel travel is weighted
        // ([`BESIDE`]).
        let mut beside = false;
        let mut collinear = false;
        let meets = if let Some(&global) = fixed.furniture.get(i) {
            // The shaft only: a short shelf beside a stroke's end is not a run along it. A
            // radial leader leaving its own hole along that hole's centre mark is its
            // attachment, not travel along another feature's line.
            beside = crate::centrelines::runs_beside(segs[0], *f)
                && !(fixed.crosses.contains(i)
                    && !job.corridors
                    && through(*f, nearest(&job.sites, tip).0));
            collinear = lines.iter().any(|s| s.collinear(*f));
            beside
                || collinear
                || segs
                    .iter()
                    .enumerate()
                    .any(|(k, s)| crate::centrelines::leader_conflict(*s, *f, global, k == 0))
        } else {
            lines.iter().any(|s| s.meets(*f))
        };
        if meets {
            if fixed.furniture.contains_key(i) {
                furniture.push(*i);
                if beside {
                    besides.push(*i);
                }
                if collinear {
                    hard += 1;
                    furniture_hard += 1;
                }
            } else {
                crossed.push(*i);
            }
        }
        let through_text = crosses(*f, lb);
        if job.stroke_pad > 0.0
            && !through_text
            && !fixed.furniture.contains_key(i)
            && crosses(*f, stroke_box)
        {
            clearance.push(*i);
        }
        if through_text {
            text.push(*i);
            if fixed.furniture.contains_key(i) {
                hard += 1;
                furniture_hard += 1;
                furniture_text.push(*i);
            }
        }
    });
    fixed.grid.near(2, near, |n| {
        let (i, e) = &fixed.heads[n];
        if segment_near(*e, near) {
            if crosses(*e, lb) {
                text.push(*i);
            } else if job.stroke_pad > 0.0 && crosses(*e, stroke_box) {
                clearance.push(*i);
            }
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
            if fixed.furniture.contains_key(i) {
                hard += 1;
                furniture_hard += 1;
                furniture_text.push(*i);
            }
        }
    });
    // An item both crossed and met by the label counts in each: the plan reports a crossing
    // and a text contact as two problems of the pair.
    for v in [
        &mut crossed,
        &mut text,
        &mut clearance,
        &mut furniture,
        &mut furniture_text,
        &mut besides,
    ] {
        v.sort_unstable();
        v.dedup();
    }
    let crossings = (crossed.len() + text.len()) as u32;
    if job.corridors {
        // The same finite 1 mm extension corridor used by the route acceptance
        // check participates in the initial solve, before a route becomes fixed.
        hard += fixed.extension_index.contacts(&segs, lb).0 as u32;
        if job.short_profile {
            hard += fixed
                .extensions
                .iter()
                .filter(|(_, e)| {
                    let bounds = grow(segment_bounds(*e), 0.2);
                    own_head.iter().any(|h| touches_box(*h, bounds))
                })
                .count() as u32;
        }
        if job
            .model
            .is_some_and(|model| segs.iter().any(|s| model.runs_beside(*s)))
        {
            hard += 1;
        }
    }
    c.hard += hard;
    c.crossings = crossings;
    c.furniture = furniture.len() as u32;
    c.furniture_hard = furniture_hard;
    c.furniture_text = furniture_text.len() as u32;
    c.beside = besides.len() as u32;
    c.text = text.len() as u32;
    c.clearance = clearance.len() as u32;
    c.fixed_measured = true;
    if c.ignore_furniture {
        c.without_furniture();
    }
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
        side: None,
        view,
        footprints: &footprints,
        outline: f.outline(),
        area: page.area(view),
        sites: page_sites(f, sites),
        silhouette: grazing(f, w),
        model: None,
        short_profile: false,
        profile_proof: None,
        directed: false,
        corridors: false,
        steer: std::cell::Cell::new(None),
        ink_min: ink.min,
        ink_max: ink.max,
        stroke_pad: stroke_clearance(&d.terms),
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
#[derive(Clone, Debug)]
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

    fn collinear(&self, b: [P2; 2]) -> bool {
        let Some((u, _)) = self.dir else {
            return false;
        };
        let v = [b[1][0] - b[0][0], b[1][1] - b[0][1]];
        (u[0] * v[1] - u[1] * v[0]).abs() <= 1e-9 * v[0].hypot(v[1]) && self.along(b)
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

/// Whether the line of segment *s* passes within a micrometre of *p*.
fn through(s: [P2; 2], p: P2) -> bool {
    let u = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
    let n = u[0].hypot(u[1]);
    n > 1e-9 && ((p[0] - s[0][0]) * u[1] - (p[1] - s[0][1]) * u[0]).abs() / n < 1e-3
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
    let directed = a.directed || b.directed;
    let text = TEXT_CONFLICT + u32::from(directed);
    let (ia, ib) = (a.conflict_ink(), b.conflict_ink());
    if !boxes_overlap(ia.bounds, ib.bounds) {
        return 0;
    }
    if boxes_overlap(ia.label, b.label_box) {
        return text;
    }
    if ia.segments.iter().any(|s| crosses(*s, b.label_box))
        || ib.segments.iter().any(|s| crosses(*s, a.label_box))
    {
        text
    } else if directed {
        let ab = ia.segments.iter().any(|s| {
            ib.segments
                .iter()
                .chain(std::iter::once(&arrow(b.tip, b.elbow)))
                .any(|t| lines_meet(*s, *t))
        });
        let ba = ib.segments.iter().any(|s| {
            ia.segments
                .iter()
                .chain(std::iter::once(&arrow(a.tip, a.elbow)))
                .any(|t| lines_meet(*s, *t))
        });
        u32::from(ab) + u32::from(ba)
    } else {
        u32::from(
            ia.lines
                .iter()
                .any(|s| ib.segments.iter().any(|t| s.meets(*t))),
        )
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
/// none for a pair that never does. The shortlist, incumbent and furniture-only variants
/// fit in one word.
struct Conflicts {
    m: usize,
    of: Vec<Option<Vec<Vec<u64>>>>,
}

const _: () = assert!(KEEP + ROTATIONS.len() + 2 * SHOULDER_STEPS.len() < 64); // and the incumbent

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
    fn only_the_established_path_may_touch_uncertain_material() {
        let b = [0.0, 0.0, 10.0, 10.0];
        let old = [[[5.0, 5.0], [5.0, 12.0]]];
        assert!(!adds_unknown_path([[5.0, 5.0], [5.0, 15.0]], b, &old));
        assert!(!adds_unknown_path([[5.0, 5.0], [5.0, 11.0]], b, &old));
        assert!(adds_unknown_path([[5.0, 4.0], [5.0, 15.0]], b, &old));
        assert!(adds_unknown_path([[5.0, 5.0], [6.0, 15.0]], b, &old));
        assert!(adds_unknown_path([[4.0, 5.0], [4.0, 15.0]], b, &old));
    }

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

    #[test]
    fn a_part_without_recesses_keeps_its_published_radius_route() {
        let corpus = std::env::var_os("DRAFTWRIGHT_CORPUS")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| std::path::PathBuf::from(h).join("repos/quiddity/tests/corpus"))
            });
        let Some(corpus) = corpus.filter(|p| p.is_dir()) else {
            assert_ne!(
                std::env::var("DRAFTWRIGHT_CORPUS_REQUIRED").as_deref(),
                Ok("1")
            );
            return;
        };
        let a = crate::model::analyse_native(&corpus.join("cadgenbench_inputs/cgb245.step"))
            .unwrap()
            .1;
        assert!(a.features.iter().all(|f| f.kind != "section_recess"));
        let plan = crate::model::plan_recovered(&a, &crate::model::Spec::default()).unwrap();
        let id = "d:f:slot:1/slot_end_radius.radius";
        let d = plan.dimensions.iter().find(|d| d.id == id).unwrap();
        let p = d.placed.as_ref().unwrap();
        let v = plan.views.iter().find(|v| v.id == d.view).unwrap();
        let baseline: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/leader_routes.json")).unwrap();
        let route = &baseline["parts"]["cgb245"][id]["route"];
        let points: Vec<_> = p.lines.iter().flatten().collect();
        assert_eq!(points.len(), route.as_array().unwrap().len());
        for (i, point) in points.iter().enumerate() {
            for (k, coordinate) in point.iter().enumerate() {
                assert!(
                    (coordinate - v.at[k] - route[i][k].as_f64().unwrap()).abs() < 1e-6,
                    "{id}: route changed at {i}/{k}"
                );
            }
        }
    }

    #[test]
    fn refined_slot_radius_leaders_point_towards_the_arc_centre() {
        let corpus = std::env::var_os("DRAFTWRIGHT_CORPUS")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| std::path::PathBuf::from(h).join("repos/quiddity/tests/corpus"))
            });
        let Some(corpus) = corpus.filter(|p| p.is_dir()) else {
            assert_ne!(
                std::env::var("DRAFTWRIGHT_CORPUS_REQUIRED").as_deref(),
                Ok("1")
            );
            return;
        };
        let a = crate::model::analyse_native(&corpus.join("nist/nist_ftc_08_asme1_rc.stp"))
            .unwrap()
            .1;
        let plan = crate::model::plan_recovered(&a, &crate::model::Spec::default()).unwrap();
        let wants = super::super::wants(&a).0;
        let baseline: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/leader_routes.json")).unwrap();
        let before = &baseline["parts"]["nist_ftc_08_asme1_rc"];
        let mut checked = 0;
        for d in &plan.dimensions {
            if !d
                .measurements
                .iter()
                .any(|m| m.parameter == "slot_end_radius.radius")
            {
                continue;
            }
            let Some(p) = &d.placed else { continue };
            let w = wants.iter().find(|w| w.id == d.id).unwrap();
            let Shape::Callout { sites } = &w.shape else {
                panic!("radius needs arc sites")
            };
            let v = plan.views.iter().find(|v| v.id == d.view).unwrap();
            let sites = page_sites(&Frame::new(&a, v), sites);
            let [tip, elbow] = p.lines[0];
            let (centre, _) = nearest(&sites, tip);
            let r = [tip[0] - centre[0], tip[1] - centre[1]];
            let shaft = [elbow[0] - tip[0], elbow[1] - tip[1]];
            let unchanged = [tip, elbow].iter().enumerate().all(|(i, p)| {
                (0..2).all(|k| {
                    (p[k] - v.at[k] - before[&d.id]["route"][i][k].as_f64().unwrap()).abs() < 1e-6
                })
            });
            if !unchanged {
                let sine = (r[0] * shaft[1] - r[1] * shaft[0]).abs()
                    / (r[0].hypot(r[1]) * shaft[0].hypot(shaft[1]));
                assert!(sine < 1e-9, "{}: moved radius leader must be radial", d.id);
                assert!(r[0] * shaft[0] + r[1] * shaft[1] > 0.0);
            }
            checked += 1;
        }
        assert!(checked > 0);
    }

    /// string_post_tapped's turned-step callouts start with their labels on extension lines:
    /// `ø6` touches three dimensions' extension lines, two with its text, and `ø5` two, both with
    /// its text. Refinement takes both labels off: (3, 2) -> (1, 0) and (2, 2) -> (0, 0).
    #[test]
    fn refinement_takes_callout_text_off_extension_lines() {
        use std::io::Read as _;
        let gz = include_bytes!("../../tests/fixtures/pmi/step/string_post_tapped.step.gz");
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(&gz[..])
            .read_to_end(&mut bytes)
            .unwrap();
        let a = crate::model::analyse_native_step(
            std::path::Path::new("string_post_tapped.step"),
            &bytes,
        )
        .unwrap()
        .1;
        let plan = crate::model::plan_recovered(&a, &crate::model::Spec::default()).unwrap();
        let extensions: Vec<(usize, [P2; 2])> = plan
            .dimensions
            .iter()
            .enumerate()
            .filter_map(|(j, d)| Some((j, d.placed.as_ref()?)))
            .filter(|(_, p)| p.arrows.len() == 2 && p.lines.len() >= 3)
            .flat_map(|(j, p)| p.lines[..2].iter().map(move |s| (j, *s)))
            .collect();
        for (id, contacts) in [
            ("d:f:turned_step:1/step.diameter", (1, 0)),
            ("d:f:turned_step:2/step.diameter", (0, 0)),
        ] {
            let callout = plan
                .dimensions
                .iter()
                .find(|d| d.id == id)
                .and_then(|d| d.placed.as_ref())
                .unwrap();
            assert_eq!(
                extension_contacts(&extensions, &callout.lines, callout.label_box),
                contacts,
                "{id}"
            );
        }
    }

    #[test]
    fn indexed_extension_contacts_match_full_scans_and_count_distinct_owners() {
        let mut seed = 7919;
        let ink: Vec<_> = (0..300)
            .map(|n| {
                (
                    if n % 3 == 0 { n + 150 } else { n / 2 },
                    std::array::from_fn(|_| std::array::from_fn(|_| 200. * lcg(&mut seed) - 100.)),
                )
            })
            .collect();
        let indexed = ExtensionIndex::new(ink.clone());
        for _ in 0..500 {
            let route: [[P2; 2]; 2] = std::array::from_fn(|_| {
                std::array::from_fn(|_| std::array::from_fn(|_| 200. * lcg(&mut seed) - 100.))
            });
            let at = [200. * lcg(&mut seed) - 100., 200. * lcg(&mut seed) - 100.];
            let label = [at[0], at[1], at[0] + 12., at[1] + 4.];
            assert_eq!(
                indexed.contacts(&route, label),
                extension_contacts(&ink, &route, label)
            );
            assert_eq!(
                meets_extensions(
                    &ink.iter().map(|(_, e)| *e).collect::<Vec<_>>(),
                    &route,
                    label
                ),
                extension_contacts(&ink, &route, label).0 > 0
            );
            assert_eq!(
                indexed.contacts(&[], label),
                extension_contacts(&ink, &[], label)
            );
        }
        let ink = vec![(4, [[0., 0.], [0., 20.]]), (4, [[2., 0.], [2., 20.]])];
        let index = ExtensionIndex::new(ink.clone());
        assert_eq!(index.contacts(&[], [0., 5., 2., 10.]), (1, 1));
    }

    #[test]
    fn indexed_route_checks_match_full_edge_scans() {
        let mut seed = 7919;
        let mut ink: Vec<_> = (0..300)
            .map(|_| {
                std::array::from_fn(|_| std::array::from_fn(|_| 200.0 * lcg(&mut seed) - 100.0))
            })
            .collect();
        // A run assembled from short strokes must not disappear at leaf boundaries.
        ink.extend((0..40).map(|n| [[n as f64, 0.0], [n as f64 + 1.0, 0.0]]));
        let index = SegmentIndex::new(ink.clone());
        for n in 0..200 {
            let line = if n == 0 {
                [[0.0, 0.3], [40.0, 0.3]]
            } else {
                std::array::from_fn(|_| std::array::from_fn(|_| 200.0 * lcg(&mut seed) - 100.0))
            };
            assert_eq!(index.runs_beside(line), runs_beside(line, &ink));
            assert_eq!(index.runs_beside(line), runs_beside(line, &ink));
            let bounds = grow(segment_bounds(line), 1.0);
            assert_eq!(
                index.crosses_box(bounds),
                ink.iter()
                    .any(|line| crate::dims::placement::crosses(*line, bounds))
            );
            assert_eq!(
                index
                    .near(bounds)
                    .into_iter()
                    .filter(|s| touches_box(**s, bounds))
                    .count(),
                ink.iter().filter(|s| touches_box(**s, bounds)).count()
            );
            assert_eq!(
                index.crossings(line),
                ink.iter().filter(|s| segments_cross(line, **s)).count()
            );
        }
    }

    #[test]
    fn diagonal_crossing_queries_retain_thin_and_boundary_leaves() {
        let ink: Vec<_> = (0..2048)
            .map(|n| {
                let x = (n % 64) as f64;
                let y = (n / 64) as f64;
                [[x, y], [x, y + 0.75]]
            })
            .collect();
        let index = SegmentIndex::new(ink.clone());
        for line in [
            [[-1., -1.], [65., 33.]],
            [[0., 0.], [63., 31.]],
            [[0., 0.375], [63., 0.375]],
            [[63., -1.], [63., 33.]],
            [[-1., 31.375], [65., 31.375]],
        ] {
            for _ in 0..2 {
                assert_eq!(
                    index.crossings(line),
                    ink.iter().filter(|s| segments_cross(line, **s)).count()
                );
            }
        }
    }

    #[test]
    fn indexed_leader_routes_match_full_scans_and_count_double_crossings_once() {
        let edge = [[-1., 0.5], [3., 0.5]];
        let route = [[[0., 0.], [1., 1.]], [[1., 1.], [2., 0.]]];
        let index = SegmentIndex::new(vec![edge]);
        assert!(route.iter().all(|s| segments_cross(*s, edge)));
        assert_eq!(index.route_crossings(route), 1);
        assert_eq!(index.route_crossings(route), 1);
        let mut seed = 7919;
        let ink: Vec<_> = (0..300)
            .map(|_| std::array::from_fn(|_| std::array::from_fn(|_| 200. * lcg(&mut seed) - 100.)))
            .collect();
        let index = SegmentIndex::new(ink.clone());
        for _ in 0..200 {
            let points: [P2; 3] =
                std::array::from_fn(|_| std::array::from_fn(|_| 200. * lcg(&mut seed) - 100.));
            let route = [[points[0], points[1]], [points[1], points[2]]];
            let full = ink
                .iter()
                .filter(|edge| route.iter().any(|line| segments_cross(*line, **edge)))
                .count();
            assert_eq!(index.route_crossings(route), full);
            assert_eq!(index.route_crossings(route), full);
            let old = [[[0., 0.], [25., -30.]], [[25., -30.], [-20., 15.]]];
            assert_eq!(
                index.new_crossing(&route, &old),
                ink.iter().any(|edge| {
                    route.iter().any(|line| segments_cross(*line, *edge))
                        && !old.iter().any(|line| segments_cross(*line, *edge))
                })
            );
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
            furniture: 0,
            furniture_hard: 0,
            furniture_text: 0,
            clearance: 0,
            beside: 0,
            grazing: 0,
            astray: 0,
            feature_crossings: 0,
            recess: false,
            directed: false,
            inside: 0,
            features_measured: true,
            fixed_measured: true,
            ignore_furniture: false,
            aware: false,
            text: 0,
            outside: 0,
            conflict_ink: OnceCell::new(),
            bounds: [
                tip[0].min(elbow[0]),
                tip[1].min(elbow[1]).min(label_box[1]),
                label_box[2],
                tip[1].max(elbow[1]).max(label_box[3]),
            ],
        }
    }

    #[test]
    fn a_shared_recess_solve_counts_both_directions_of_a_shaft_crossing() {
        let mut a = cand([0., 0.], [10., 10.], 14_000);
        let b = cand([0., 10.], [10., 0.], 14_000);
        assert_eq!(conflict(&a, &b), 1);
        a.directed = true;
        assert_eq!(conflict(&a, &b), 2);
        assert_eq!(conflict(&b, &a), 2);
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
