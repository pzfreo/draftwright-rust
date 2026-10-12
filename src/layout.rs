//! Sheet, scale and view placement, ported from draftwright's layout authority
//! (`compose.choose_scale` and `compose._layout_geometry`, ADR 2): each principal view is a block,
//! its projected box plus the annotation bands reserved on each side, and the blocks are packed
//! by box arithmetic (compose-then-pack). The isometric takes the largest empty rectangle the
//! packed blocks and the title block leave.
//!
//! The arithmetic is held to draftwright's own calls by `tests/layout.rs`, which replays the
//! layout calls recorded from Python builds (`tools/capture_layout.py`).
//!
//! Not ported: the first-angle convention and the rear view (draftwright composes both through
//! relative projection origins), and the automatic `auto` arrangement (not a production choice).

/// Page mm. The sheet edge margin, the title block's inset and height, the annotation text
/// height and its padding.
pub const MARGIN: f64 = 10.0;
pub const TB_CLEAR: f64 = MARGIN + 1.0;
pub const TB_H: f64 = 40.0;
const FONT_SIZE: f64 = 3.0;
const PAD: f64 = 2.0;
/// The view outline to the first dimension line, and between successive dimension lines.
const STRIP_GAP: f64 = 10.0;
const STRIP_SPACING: f64 = 2.5;
const SLOT_DIM_WIDTH: f64 = 2.0 * FONT_SIZE + PAD;
const SLOT_DIM_HEIGHT: f64 = 2.0 * FONT_SIZE + 2.0 * PAD;
const SLOT_DIM_STEP: f64 = 4.0 * FONT_SIZE + PAD;
/// The base corridor between views: one first-line gap and one dimension tier (20 mm).
pub const DIM_PAD: f64 = STRIP_GAP + SLOT_DIM_HEIGHT;
/// The isometric's estimated width as a fraction of the part's largest extent, and the fraction
/// of that its rectangle must hold for a candidate to fit.
const ISO_WIDTH_BUDGET: f64 = 0.7;
const ISO_MIN_FIT_FRAC: f64 = 0.6;
/// The isometric grows at most this much to fill its rectangle.
const ISO_MAX_GROW: f64 = 1.3;
/// Blank paper between neighbouring views' reserved bands: the floor, and what spare room buys.
const VIEW_GUTTER: f64 = 6.0;
const VIEW_GUTTER_PREFERRED: f64 = 12.0;
/// Breathing room a table is first offered beyond its hard clearance.
const PREFERRED_BLOCK_GUTTER: f64 = 6.0;
const TOL: f64 = 0.5;
/// Blank paper between the side view's band and a section view, which is all the band a section
/// has on its left (draftwright's row budget for a section, `10 + 2 hw + DIM_PAD`).
const SECTION_GAP: f64 = 10.0;

/// ISO 216 sheets, smallest first: name, width, height (mm, landscape).
pub const PAGES: [(&str, f64, f64); 5] = [
    ("A4", 297.0, 210.0),
    ("A3", 420.0, 297.0),
    ("A2", 594.0, 420.0),
    ("A1", 841.0, 594.0),
    ("A0", 1189.0, 841.0),
];

/// ISO 5455's scales, largest first, as draftwright tries them.
pub const SCALES: [f64; 16] = [
    10.0, 5.0, 2.0, 1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002, 0.001, 0.0005, 0.0002,
    0.0001,
];

/// The automatic scale and sheet preference ladder, first fit: every enlargement on the smallest
/// sheet before the next sheet, then the reductions, least reduction first.
const LADDER: [(f64, &str); 26] = [
    (10.0, "A4"),
    (5.0, "A4"),
    (2.0, "A4"),
    (1.0, "A4"),
    (5.0, "A3"),
    (2.0, "A3"),
    (1.0, "A3"),
    (2.0, "A2"),
    (1.0, "A2"),
    (1.0, "A1"),
    (0.5, "A2"),
    (0.2, "A3"),
    (0.2, "A2"),
    (0.5, "A1"),
    (0.2, "A1"),
    (0.5, "A0"),
    (0.2, "A0"),
    (0.1, "A0"),
    (0.05, "A0"),
    (0.02, "A0"),
    (0.01, "A0"),
    (0.005, "A0"),
    (0.002, "A0"),
    (0.001, "A0"),
    (0.0005, "A0"),
    (0.0002, "A0"),
];

/// The title block's width for a sheet: 120 mm on A4, 150 mm on A3 and larger.
pub fn title_block_width(page_w: f64) -> f64 {
    if page_w <= 297.0 { 120.0 } else { 150.0 }
}

fn sheet(name: &str) -> [f64; 2] {
    let (_, w, h) = PAGES.iter().find(|p| p.0 == name).expect("a ladder sheet");
    [*w, *h]
}

/// `[x0, y0, x1, y1]`, page mm.
pub type Rect = [f64; 4];

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// Where the isometric goes relative to the orthographic row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    /// `[front][side][iso][title block]`: the iso has a column of its own.
    Columns,
    /// The iso shares the title block's column; the orthographic views pack left.
    StackedIso,
    /// As stacked, with the orthographic row lifted clear of the title block.
    StaggeredSide,
}

impl Arrangement {
    pub fn name(self) -> &'static str {
        match self {
            Arrangement::Columns => "columns",
            Arrangement::StackedIso => "stacked-iso",
            Arrangement::StaggeredSide => "staggered-side",
        }
    }
}

/// The arrangements an automatic choice tries, in preference order.
pub const AUTOMATIC_ARRANGEMENTS: [Arrangement; 2] =
    [Arrangement::Columns, Arrangement::StackedIso];

/// A table's preferred corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// Distances from the sheet edges, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margins {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Margins {
    pub fn uniform(m: f64) -> Self {
        Margins {
            left: m,
            right: m,
            top: m,
            bottom: m,
        }
    }
}

/// The annotation bands reserved around the principal views, page mm (draftwright's
/// `StripDepths`). They come from the features to be dimensioned.
#[derive(Clone, Debug, PartialEq)]
pub struct Strips {
    /// Right and left of the front/plan column.
    pub right: f64,
    pub left: f64,
    /// Above the plan: tiered X-location dimensions.
    pub top: f64,
    /// The balloon standoff ring around the plan.
    pub pv_halo: f64,
    pub fv_top: f64,
    pub fv_bottom: f64,
    /// Rows of Y-hole leaders below the front, spent only beside a derived view.
    pub front_hole_below: f64,
    pub pv_authored_top: f64,
    pub pv_bottom: f64,
    pub sv_top: f64,
    pub sv_bottom: f64,
    pub sv_right: f64,
    /// Left of the side view, between it and the front/plan column's right band: draftwright
    /// reserves none; only the port's GD&T reservation ([`Strips::reserve_gdt`]) sets it.
    pub sv_left: f64,
    /// Side-above dimensions that are not feature leaders (each takes an outward tier when the
    /// side view is staggered).
    pub side_ladder: usize,
    /// Whether any annotation is planned on the plan, and whether a Y step chain sits above the
    /// side: together they keep a derived view off the side's step row.
    pub plan_demand: bool,
    pub side_step_y: bool,
}

impl Strips {
    /// The bands of a part with no features to dimension: the overall height to the right of the
    /// front and the base corridor on the left (draftwright's estimate with no steps).
    pub fn without_features() -> Self {
        Strips {
            right: STRIP_GAP + SLOT_DIM_HEIGHT,
            left: DIM_PAD,
            top: 0.0,
            pv_halo: 0.0,
            fv_top: 0.0,
            fv_bottom: 0.0,
            front_hole_below: 0.0,
            pv_authored_top: 0.0,
            pv_bottom: 0.0,
            sv_top: 0.0,
            sv_bottom: 0.0,
            sv_right: 0.0,
            sv_left: 0.0,
            side_ladder: 0,
            plan_demand: false,
            side_step_y: false,
        }
    }
}

/// The lanes the port's own dimensions stack in beside each view (`dims::lanes`), by the strip
/// they take: horizontal dimensions below the front and the side and above the plan, vertical
/// ones right of the front and the side and left of the plan, callouts below the front and above
/// the plan and the side.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lanes {
    pub front_below: usize,
    pub front_right: usize,
    pub plate_front_left: usize,
    pub plate_front_top: f64,
    pub plate_front_below: usize,
    pub plate_side_left: f64,
    pub plate_side_top: f64,
    pub plate_plan_above: usize,
    /// Plate thicknesses whose witnesses meet the silhouette on a side the fixed map never uses.
    pub plate_front_above: usize,
    pub plate_plan_right: usize,
    pub plate_plan_below: usize,
    pub plate_side_left_lanes: usize,
    pub plan_above: usize,
    pub plan_left: usize,
    pub side_below: usize,
    pub side_above: usize,
    pub side_right: usize,
}

/// The distance between successive dimension lines in the port's lanes (`dims::LANE`), and how
/// far a lane's ink reaches past its line: the label's baseline lift and one text height, which
/// also covers the extension lines' 2 mm overshoot.
const LANE: f64 = 7.0;
const LANE_REACH: f64 = 1.0 + FONT_SIZE;

/// The band *lanes* stacked lanes need: the first line's gap, the lanes, and the outermost one's
/// reach; none for none.
pub fn lane_band(lanes: usize) -> f64 {
    if lanes == 0 {
        0.0
    } else {
        STRIP_GAP + (lanes - 1) as f64 * LANE + LANE_REACH
    }
}

impl Strips {
    /// Reserves the bands between neighbouring views for the port's own dimension lanes (review
    /// M3): each is at least [`lane_band`] of the lanes stacked in it, so no ladder runs past its
    /// band across the next view.
    ///
    /// - The side's bands: draftwright reserves them only for its own side-view schemes (axial
    ///   boss heights, seat offsets, through-step legs), none of which the port plans, so without
    ///   this the side had no band at all.
    /// - The band right of the front and the plan (their column's one right band, `geometry`, as
    ///   in draftwright), between them and the side: the front's height ladder ran on past
    ///   draftwright's estimate across the side view (ftc09, ftc10, cgb245; drawing quality,
    ///   2026-10-09). Its lanes hold linear dimensions alone, as callouts never stack right.
    ///
    /// The bands that face open paper stay draftwright's estimates: below the front (beyond the
    /// side's lanes, which it shares), above and left of the plan. Reserving the port's lanes
    /// there too moves most sheet and scale choices away from draftwright's
    /// (`docs/quality-2026-10-09.md`, "After the three quality merges and lane reservation":
    /// cgb245 from 1:2 to 1:5, ftc08 from 1:2 to 1:5, ctc01 to 1:10), because the port's lanes
    /// count callouts, which take no lane, and every rung, and stack outward 7 mm apart where
    /// draftwright's solver runs on to the sheet's margin; so it waits for the maintainer, and
    /// their ink past their bands is reported (`annotation_outside_area`).
    ///
    /// The front and the side share a row whose height is set from the front's lower band
    /// (`geometry`, as in draftwright), so the front's lower band is also at least the side's: a
    /// deeper band below the side alone would hang below the row.
    pub fn reserve(&mut self, lanes: &Lanes) {
        let widen = |band: &mut f64, n: usize| *band = band.max(lane_band(n));
        widen(&mut self.right, lanes.front_right);
        widen(&mut self.left, lanes.plate_front_left);
        // Short thickness spans use outside arrows and text. Reserve their ink
        // past the adjacent corner as well as the carrying strip's lanes.
        self.fv_top = self.fv_top.max(lanes.plate_front_top);
        if lanes.plate_front_below > 0 {
            widen(&mut self.fv_bottom, lanes.plate_front_below);
        }
        self.sv_left = self.sv_left.max(lanes.plate_side_left);
        self.sv_top = self.sv_top.max(lanes.plate_side_top);
        // An outside thickness label extends above the aligned front/side row.
        self.fv_top = self.fv_top.max(lanes.plate_side_top);
        if lanes.plate_plan_above > 0 {
            self.pv_authored_top = self.pv_authored_top.max(lane_band(lanes.plate_plan_above));
        }
        widen(&mut self.fv_top, lanes.plate_front_above);
        widen(&mut self.right, lanes.plate_plan_right);
        widen(&mut self.pv_bottom, lanes.plate_plan_below);
        widen(&mut self.sv_left, lanes.plate_side_left_lanes);
        widen(&mut self.fv_bottom, lanes.side_below);
        widen(&mut self.sv_bottom, lanes.side_below);
        widen(&mut self.sv_top, lanes.side_above);
        widen(&mut self.sv_right, lanes.side_right);
    }

    /// Reserves room at *scale* for the feature control frames, datum feature symbols and labels
    /// *items* (`dims::gdt::demand`): each strip that holds any is at least the room its
    /// dimensions take (below; *base* is the bands before any GD&T: draftwright's estimates,
    /// widened by [`Strips::reserve`] where the port reserves its dimension lanes) and the tiers
    /// its items stack in beyond it, as `dims::gdt` places them. A strip's items are packed in placement's order (above and below
    /// a view, the rightmost leader first; left and right, the lowest first), each into the
    /// nearest tier where its extent across the strip, leader included, is clear of the items
    /// already there.
    ///
    /// draftwright reserves no band for GD&T: its frames and symbols compete for the strips the
    /// dimensions' estimates leave (`annotations/_gdt.py`, `_GDT_SUBCHAIN`) and are dropped
    /// when none holds them (`pmi_dropped`); its typed annotation scheme
    /// (`layout_scheme.plan_annotation_scheme`) counts their lanes the same way, but only to
    /// observe, or to cap an opt-in profile's bands, never to widen them. The port places and
    /// reports every item (the maintainer's escalate-never-lose directive), so it reserves the
    /// room they take. A part with none reserves nothing.
    ///
    /// The tiers start beyond the lesser of *base*'s band and the band of the strip's dimension
    /// lanes ([`lane_band`]), or, with *beyond_lanes*, beyond the lanes' band whatever *base* is.
    /// The lane count is an upper bound: it counts callouts, which take no lane, and every item
    /// in a lane of its own, where placement shares lanes; so [`Strips::reserve`] keeps the bands
    /// that face open paper (below the front, above and left of the plan) at draftwright's
    /// estimates, and the GD&T's room starts within them too (nist_stc_09 e4: 45 lanes, 322 mm,
    /// below a front whose dimensions take 173 mm at 1:1 on A1; tiers beyond the count drove its
    /// scale down the ladder to 1:5). Where the dimensions then run past that start and leave an
    /// item no place on the sheet off the other views, the plan is made again with
    /// *beyond_lanes* (`model::plan_escalated`).
    ///
    /// The bands are those `geometry` reads: the front's above and below, the column's left and
    /// right (shared by the front and the plan), the plan's above and below, the side's four. An
    /// item that runs past the view's end along its strip (a frame hung right of a leader near
    /// the right end) widens the band beyond that end as far. As in [`Strips::reserve`], the
    /// front's lower band is then at least the side's. Bands only widen.
    pub fn reserve_gdt(
        &mut self,
        base: &Strips,
        lanes: &Lanes,
        beyond_lanes: bool,
        items: &[Gdt],
        scale: f64,
    ) {
        let mut base = base.clone();
        let strips: [(&str, &str, usize); 12] = [
            ("front", "below", lanes.front_below),
            ("front", "above", 0),
            ("front", "left", 0),
            ("front", "right", lanes.front_right),
            ("plan", "above", lanes.plan_above),
            ("plan", "below", 0),
            ("plan", "left", lanes.plan_left),
            ("plan", "right", 0),
            ("side", "above", lanes.side_above),
            ("side", "below", lanes.side_below),
            ("side", "left", 0),
            ("side", "right", lanes.side_right),
        ];
        for (view, side, n) in strips {
            let mine: Vec<&Gdt> = items
                .iter()
                .filter(|g| g.view == view && g.side == side)
                .collect();
            if mine.is_empty() {
                continue;
            }
            let start = if beyond_lanes {
                lane_band(n)
            } else {
                base.band(view, side).min(lane_band(n))
            };
            let depth = gdt_band(&mine, start, scale);
            let band = self.band(view, side);
            *band = band.max(depth);
            // A frame hung beside its leader near the view's end runs past the end: the band
            // beyond that end holds it.
            let ends = if matches!(side, "above" | "below") {
                ["left", "right"]
            } else {
                ["below", "above"]
            };
            for g in mine {
                let past = [
                    g.ends[0] * scale - (g.at * scale + g.span[0]),
                    g.at * scale + g.span[1] - g.ends[1] * scale,
                ];
                for (end, past) in ends.into_iter().zip(past) {
                    let band = self.band(view, end);
                    *band = band.max(past);
                }
            }
        }
        if !items.is_empty() {
            self.fv_bottom = self.fv_bottom.max(self.sv_bottom);
        }
    }

    /// The band `geometry` reads for strip *side* (`above`, `below`, `left`, `right`) of *view*.
    fn band(&mut self, view: &str, side: &str) -> &mut f64 {
        match (view, side) {
            ("front", "below") => &mut self.fv_bottom,
            ("front", "above") => &mut self.fv_top,
            ("front" | "plan", "left") => &mut self.left,
            ("front" | "plan", "right") => &mut self.right,
            ("plan", "above") => &mut self.pv_authored_top,
            ("plan", "below") => &mut self.pv_bottom,
            (_, "above") => &mut self.sv_top,
            (_, "below") => &mut self.sv_bottom,
            (_, "left") => &mut self.sv_left,
            _ => &mut self.sv_right,
        }
    }
}

/// The clearance `dims::gdt` keeps beyond a strip's dimensions and between its tiers, mm.
pub const GDT_CLEAR: f64 = 2.0;

/// A feature control frame, datum feature symbol or label as the layout reserves for it
/// ([`Strips::reserve_gdt`]), from `dims::gdt::demand`.
#[derive(Clone, Debug, PartialEq)]
pub struct Gdt {
    /// Its view (`front`, `plan` or `side`) and strip (`above`, `below`, `left`, `right`).
    pub view: &'static str,
    pub side: &'static str,
    /// Its site across the strip (page x above and below, page y left and right), world mm from
    /// the view's centre.
    pub at: f64,
    /// Its extent across the strip, page mm from its site, leader included.
    pub span: [f64; 2],
    /// The view's outline across the strip, world mm from its centre.
    pub ends: [f64; 2],
    /// Its extent outward, page mm: the frame's height above or below, its shelf and width
    /// left or right; a datum's frame.
    pub depth: f64,
    /// The least distance from the view's outline to its near edge outward: room for the
    /// leader's arrowhead or the datum's triangle.
    pub lead: f64,
}

/// The band, page mm from the outline, that *items* of one strip need beyond *base* (the
/// strip's dimensions' band) at *scale*: their tiers packed first fit.
fn gdt_band(items: &[&Gdt], base: f64, scale: f64) -> f64 {
    let horizontal = matches!(items[0].side, "above" | "below");
    let mut order: Vec<&Gdt> = items.to_vec();
    order.sort_by(|a, b| {
        if horizontal {
            b.at.total_cmp(&a.at)
        } else {
            a.at.total_cmp(&b.at)
        }
    });
    // Each tier's extents across the strip and its depth.
    let mut tiers: Vec<(Vec<[f64; 2]>, f64)> = Vec::new();
    for g in order {
        let x = [g.at * scale + g.span[0], g.at * scale + g.span[1]];
        // The frames' grown boxes must not meet (`dims::gdt`'s 0.5 mm each way).
        let clear = |t: &[[f64; 2]]| t.iter().all(|y| x[1] + 1.0 <= y[0] || y[1] + 1.0 <= x[0]);
        match tiers.iter_mut().find(|t| clear(&t.0)) {
            Some(t) => {
                t.0.push(x);
                t.1 = t.1.max(g.depth);
            }
            None => tiers.push((vec![x], g.depth)),
        }
    }
    let start = items
        .iter()
        .map(|g| g.lead)
        .fold(base + GDT_CLEAR, f64::max);
    start + tiers.iter().map(|t| t.1).sum::<f64>() + (tiers.len() - 1) as f64 * GDT_CLEAR
}

/// The band right of the front view for its height ladder (draftwright's
/// `_est_right_strip_depth`): the overall height's slot, then one slot per step level and per
/// other occupant (an axial boss height).
pub fn right_strip_depth(n_steps: usize, n_extra: usize) -> f64 {
    let n = (1 + n_steps + n_extra) as f64;
    STRIP_GAP + SLOT_DIM_HEIGHT + (n - 1.0) * (STRIP_SPACING + SLOT_DIM_STEP)
}

/// The tiers above the plan for X-location dimensions, one per distinct reference and one spare
/// (draftwright's `_est_pv_above_depth`); none without references.
pub fn plan_above_depth(distinct_x_refs: usize) -> f64 {
    if distinct_x_refs == 0 {
        0.0
    } else {
        (distinct_x_refs + 1) as f64 * (FONT_SIZE + 2.0 * PAD)
    }
}

/// The rows of Y-axis hole leaders below the front (draftwright's `front_hole_below` band).
pub fn front_hole_rows_depth(rows: usize) -> f64 {
    if rows == 0 {
        0.0
    } else {
        let row = FONT_SIZE + 2.0 * PAD;
        STRIP_GAP + rows as f64 * row + (rows - 1) as f64 * STRIP_SPACING
    }
}

/// The balloon standoff ring around a plan whose holes are tabulated (draftwright's
/// `_balloon_halo`): the first-line gap, one balloon (diameter three text heights) and a spacing.
pub const PLAN_HALO: f64 = STRIP_GAP + 3.0 * FONT_SIZE + STRIP_SPACING;

/// The end-on radial fan's reach beside the overall-height slot, for a Y-axis boss or step.
pub const RADIAL_REACH: f64 = FONT_SIZE + 6.0 * PAD;

/// The least page height a step must stand above the part's base to carry a dimension (text,
/// two arrows, two text gaps), and the least page distance between two dimensioned steps.
pub const MIN_STEP_DIM: f64 = FONT_SIZE + 2.0 * 2.7 + 2.0 * PAD;
pub const MIN_STEP_SEP: f64 = FONT_SIZE + 2.0 * PAD;

/// The step levels worth dimensioning at *scale* (draftwright's `_legible_steps`): each at least
/// [`MIN_STEP_DIM`] above the base on the page and [`MIN_STEP_SEP`] above the last one kept.
pub fn legible_steps(step_zs: &[f64], base: f64, scale: f64) -> Vec<f64> {
    let mut zs = step_zs.to_vec();
    zs.sort_by(f64::total_cmp);
    let mut kept: Vec<f64> = Vec::new();
    for z in zs {
        if (z - base) * scale < MIN_STEP_DIM {
            continue;
        }
        if kept
            .last()
            .is_some_and(|last| (z - last) * scale < MIN_STEP_SEP)
        {
            continue;
        }
        kept.push(z);
    }
    kept
}

/// Chooses the scale with a step band that matches the steps legible at it (draftwright's
/// `_converge_step_sizing`): seeded with every step, re-gated at each chosen scale until the count
/// repeats; a cycle reserves the largest count tried. *measure* gives the bands for a step count,
/// *pick* the choice for them, *count* the legible steps at a scale.
pub fn converge_steps(
    initial: usize,
    measure: &dyn Fn(usize) -> Strips,
    pick: &dyn Fn(&Strips) -> Pick,
    count: &dyn Fn(f64) -> usize,
) -> (Pick, Strips, usize) {
    let mut n = initial;
    let mut seen = std::collections::BTreeSet::new();
    let mut attempted = Vec::new();
    for _ in 0..4.max(initial + 2) {
        if !seen.insert(n) {
            break;
        }
        attempted.push(n);
        let strips = measure(n);
        let chosen = pick(&strips);
        let next = count(chosen.scale);
        if next == n {
            return (chosen, strips, n);
        }
        if seen.contains(&next) {
            n = next;
            break;
        }
        n = next;
    }
    let n = attempted
        .iter()
        .copied()
        .chain([n])
        .max()
        .unwrap_or(initial);
    let strips = measure(n);
    (pick(&strips), strips, n)
}

/// What depends on the scale: angular dimensions' footprints about their view's centre, and the
/// derived (detail) views' boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scaled {
    /// The view (`front`, `plan` or `side`) and the footprint relative to its centre.
    pub angular: Vec<(String, Rect)>,
    /// An identity, a width and a height.
    pub derived: Vec<(String, f64, f64)>,
}

/// Which principal views the sheet carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Principals {
    pub front: bool,
    pub plan: bool,
    pub side: bool,
}

impl Principals {
    pub const ALL: Principals = Principals {
        front: true,
        plan: true,
        side: true,
    };
}

/// Everything about the drawing the layout reads, apart from scale, sheet and arrangement.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    /// The part's extents along x, y and z, mm.
    pub size: [f64; 3],
    pub strips: Strips,
    /// The number of section views beside the side view.
    pub sections: usize,
    /// Optional tables (one of them must fit, top right) and required ones.
    pub table_sizes: Vec<[f64; 2]>,
    pub required_tables: Vec<([f64; 2], Corner)>,
    pub margins: Margins,
    /// Where the title block is pinned, if not draftwright's default inset.
    pub title_block_margins: Option<Margins>,
    pub views: Principals,
    pub include_iso: bool,
    /// The iso's exact scale relative to the sheet's, when one is set.
    pub iso_scale_factor: Option<f64>,
    /// Blank gutters between the views' bands (draftwright's protected-gutter layout profiles).
    pub view_gutters: bool,
}

impl Request {
    /// The request for a part of this size with draftwright's defaults.
    pub fn new(size: [f64; 3], views: Principals, include_iso: bool) -> Self {
        Request {
            size,
            strips: Strips::without_features(),
            sections: 0,
            table_sizes: Vec::new(),
            required_tables: Vec::new(),
            margins: Margins::uniform(MARGIN),
            title_block_margins: None,
            views,
            include_iso,
            iso_scale_factor: None,
            view_gutters: false,
        }
    }

    fn furniture(&self) -> Margins {
        self.title_block_margins.unwrap_or(Margins {
            right: TB_CLEAR,
            bottom: TB_CLEAR,
            ..Margins::uniform(MARGIN)
        })
    }
}

/// A view's footprint: the half-extents of its projected box and the band reserved on each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub hw: f64,
    pub hh: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Block {
    fn bare(hw: f64, hh: f64) -> Self {
        Block {
            hw,
            hh,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    /// The block's outer box, centred at (cx, cy).
    pub fn footprint(&self, cx: f64, cy: f64) -> Rect {
        [
            cx - self.hw - self.left,
            cy - self.hh - self.bottom,
            cx + self.hw + self.right,
            cy + self.hh + self.top,
        ]
    }
}

/// The centre nearest *centre*, never below it, at which a block's near edge (*edge_of* the
/// centre) is not short of *edge*: neighbouring blocks abut exactly, whatever the rounding.
fn clear_of(edge: f64, centre: f64, edge_of: impl Fn(f64) -> f64) -> f64 {
    let mut c = centre;
    for _ in 0..8 {
        if edge_of(c) >= edge {
            break;
        }
        c = c.next_up();
    }
    c
}

/// Measured blocks of an annotated drawing, for a repack: they may only widen the estimate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    pub front: Option<Block>,
    pub plan: Option<Block>,
    pub side: Option<Block>,
}

/// Where everything sits, and whether it fits.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub arrangement: Arrangement,
    /// The views' centres: where the centre of the part's box lands, mm.
    pub front: [f64; 2],
    pub plan: [f64; 2],
    pub side: [f64; 2],
    pub section: [f64; 2],
    /// The blocks as placed.
    pub blocks: [Block; 3],
    /// The isometric's rectangle and its centre.
    pub iso: Rect,
    pub iso_centre: [f64; 2],
    pub iso_valid: bool,
    pub iso_fits: bool,
    pub title_block: Rect,
    /// The section views' reserved blocks, left to right (the first centred on `section`).
    pub section_boxes: Vec<Rect>,
    pub derived: Vec<(String, Rect)>,
    pub vertical_gutter: f64,
    pub side_gutter: f64,
    pub x_offset: f64,
    /// The packed footprints fit the sheet (draftwright's verdict for a pinned scale or sheet).
    pub fits: bool,
    /// The row fits (the verdict for an automatic choice).
    pub auto_fits: bool,
}

/// Lays the views out at *scale* on a *page_w* × *page_h* sheet with a *tb_w* title block.
pub fn geometry(
    req: &Request,
    scaled: &Scaled,
    scale: f64,
    [page_w, page_h]: [f64; 2],
    tb_w: f64,
    arrangement: Arrangement,
    measured: Option<&Measured>,
) -> Geometry {
    geometry_with(
        req,
        scaled,
        scale,
        [page_w, page_h],
        tb_w,
        arrangement,
        measured,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn geometry_with(
    req: &Request,
    scaled: &Scaled,
    scale: f64,
    [page_w, page_h]: [f64; 2],
    tb_w: f64,
    arrangement: Arrangement,
    measured: Option<&Measured>,
    prefer_gutters: bool,
) -> Geometry {
    let [x_size, y_size, z_size] = req.size;
    let mut strips = req.strips.clone();
    // The Y-hole leader band is spent only when a derived view could claim it.
    if !scaled.derived.is_empty() && strips.front_hole_below > strips.fv_bottom {
        strips.fv_bottom = strips.front_hole_below;
    }
    let m = req.margins;
    let (left, right, top, bottom) = (m.left, m.right, m.top, m.bottom);
    let furniture = req.furniture();
    let (tb_right, tb_bottom) = (furniture.right, furniture.bottom);
    let reserved_tb_bottom = if req.title_block_margins.is_none() {
        bottom
    } else {
        tb_bottom
    };
    let bbox_max = x_size.max(y_size).max(z_size);
    let fv_hw = x_size * scale / 2.0;
    let fv_hh = z_size * scale / 2.0;
    let pv_hh = y_size * scale / 2.0;
    let sv_hw = y_size * scale / 2.0;

    // The estimated blocks.
    let halo = strips.pv_halo;
    let gap_fv_sv = DIM_PAD.max(strips.right).max(halo);
    let gap_left = DIM_PAD.max(strips.left).max(halo);
    let pv_below = STRIP_GAP + SLOT_DIM_WIDTH;
    let pv_top = if halo > 0.0 {
        DIM_PAD.max(strips.top) + halo
    } else {
        DIM_PAD
    }
    .max(strips.pv_authored_top);
    let sv_right_band =
        DIM_PAD
            .max(strips.sv_right)
            .max(if req.sections > 0 { strips.right } else { 0.0 });
    let mut fv = Block {
        top: (DIM_PAD - pv_below).max(strips.fv_top),
        right: gap_fv_sv,
        bottom: DIM_PAD.max(strips.fv_bottom),
        left: gap_left,
        ..Block::bare(fv_hw, fv_hh)
    };
    let mut pv = Block {
        top: pv_top,
        right: gap_fv_sv,
        bottom: pv_below.max(halo).max(strips.pv_bottom),
        left: gap_left,
        ..Block::bare(fv_hw, pv_hh)
    };
    let mut sv = Block {
        top: strips.sv_top,
        right: sv_right_band,
        bottom: strips.sv_bottom,
        left: strips.sv_left,
        ..Block::bare(sv_hw, fv_hh)
    };
    for (view, [x0, y0, x1, y1]) in &scaled.angular {
        let block = match view.as_str() {
            "front" => &mut fv,
            "plan" => &mut pv,
            "side" => &mut sv,
            _ => continue, // the rear view is not ported
        };
        block.left = block.left.max(-x0 - block.hw);
        block.right = block.right.max(x1 - block.hw);
        block.bottom = block.bottom.max(-y0 - block.hh);
        block.top = block.top.max(y1 - block.hh);
    }
    if let Some(measured) = measured {
        // A repack packs the measured footprints, never narrower than the estimate.
        let merge = |est: &mut Block, meas: Option<Block>| {
            if let Some(meas) = meas {
                est.top = est.top.max(meas.top);
                est.right = est.right.max(meas.right);
                est.bottom = est.bottom.max(meas.bottom);
                est.left = est.left.max(meas.left);
            }
        };
        merge(&mut fv, measured.front);
        merge(&mut pv, measured.plan);
        merge(&mut sv, measured.side);
    }

    let Principals {
        front: has_front,
        plan: has_plan,
        side: has_side,
    } = req.views;
    let has_column = has_front || has_plan;
    let gutters = req.view_gutters;
    let mut vertical_gutter = if gutters && has_front && has_plan {
        VIEW_GUTTER
    } else {
        0.0
    };
    let mut side_gutter = if gutters && has_column && has_side {
        VIEW_GUTTER
    } else {
        0.0
    };
    let present: Vec<&Block> = [(&fv, has_front), (&pv, has_plan)]
        .into_iter()
        .filter(|p| p.1)
        .map(|p| p.0)
        .collect();
    let col_left = present.iter().map(|b| b.left).fold(0.0, f64::max);
    let col_right = present.iter().map(|b| b.right).fold(0.0, f64::max);

    // Spare room buys calmer separation, never a larger sheet.
    if gutters && prefer_gutters && has_front && has_plan {
        let stack = fv.bottom + 2.0 * fv.hh + fv.top + pv.bottom + 2.0 * pv.hh + pv.top;
        let slack = page_h - top - bottom - stack - vertical_gutter;
        let mut extra = (VIEW_GUTTER_PREFERRED - VIEW_GUTTER).min((slack - 2.0).max(0.0));
        // Keep whether the row clears the title block as the minimum gutter left it.
        let min_column_h = stack + vertical_gutter;
        let side_h = if has_side {
            sv.bottom + 2.0 * sv.hh + sv.top
        } else {
            0.0
        };
        let min_y_offset = ((page_h - top - bottom - min_column_h.max(side_h)) / 2.0).max(0.0);
        let title_top = reserved_tb_bottom + TB_H;
        if min_y_offset + bottom + DIM_PAD >= title_top {
            let max_column_h = page_h - top - bottom - 2.0 * (title_top - bottom - DIM_PAD);
            extra = extra.min((max_column_h - min_column_h).max(0.0));
        }
        vertical_gutter += extra;
    }
    if gutters && prefer_gutters && has_column && has_side {
        let iso_budget = if req.include_iso && arrangement != Arrangement::StackedIso {
            bbox_max
                * scale
                * match req.iso_scale_factor {
                    Some(f) => 2f64.sqrt() * f,
                    None => ISO_WIDTH_BUDGET,
                }
        } else {
            0.0
        };
        let row = col_left
            + x_size * scale
            + col_right
            + sv.left
            + y_size * scale
            + (2.0 * DIM_PAD).max(sv.right + DIM_PAD)
            + iso_budget;
        let slack = page_w - left - right - tb_w - row - side_gutter;
        side_gutter += (VIEW_GUTTER_PREFERRED - VIEW_GUTTER).min((slack - 2.0).max(0.0));
    }

    // Place the orthographic views.
    let base_gap = fv.top + vertical_gutter + pv.bottom;
    let column_h = match (has_front, has_plan) {
        (true, true) => fv.bottom + 2.0 * fv.hh + base_gap + 2.0 * pv.hh + pv.top,
        (true, false) => fv.bottom + 2.0 * fv.hh + fv.top,
        (false, true) => pv.bottom + 2.0 * pv.hh + pv.top,
        (false, false) => 0.0,
    };
    let side_h = if has_side {
        sv.bottom + 2.0 * sv.hh + sv.top
    } else {
        0.0
    };
    let total_h = bottom + top + column_h.max(side_h);
    let y_offset = ((page_h - total_h) / 2.0).max(0.0);
    let sections = req.sections as f64;
    let section_hw = fv_hw.max(12.0);
    let section_hh = fv_hh;
    let section_right_band = if req.sections > 0 {
        sv.right + sections * (10.0 + 2.0 * section_hw + DIM_PAD)
    } else {
        0.0
    };
    let ortho_row_w = col_left
        + col_right
        + if has_column { x_size * scale } else { 0.0 }
        + if has_side { y_size * scale } else { 0.0 }
        + if gutters && has_side && has_column {
            sv.left
        } else {
            0.0
        }
        + side_gutter
        + (2.0 * DIM_PAD)
            .max(if has_side { sv.right + DIM_PAD } else { 0.0 })
            .max(section_right_band);
    let iso_exact = req.iso_scale_factor.is_some();
    let iso_natural = bbox_max
        * scale
        * match req.iso_scale_factor {
            Some(f) => 2f64.sqrt() * f,
            None => ISO_WIDTH_BUDGET,
        };
    let iso_row_budget = if req.include_iso { iso_natural } else { 0.0 };
    let (total_content_w, x_offset) = if arrangement == Arrangement::Columns {
        let total = ortho_row_w + iso_row_budget;
        (
            total,
            ((page_w - (left + right) - tb_w - total) / 2.0).max(0.0),
        )
    } else {
        // The iso shares the title block's column; the views pack left to leave it.
        (ortho_row_w, 0.0)
    };
    if arrangement == Arrangement::StaggeredSide && has_side && strips.side_ladder > 0 {
        // Reserve the side view's whole nested ladder of location dimensions.
        let n = strips.side_ladder.max(6) as f64;
        let tier = FONT_SIZE + 2.0 * 2.0;
        sv.top = sv
            .top
            .max(STRIP_GAP + n * tier + (n - 1.0).max(0.0) * STRIP_SPACING);
    }
    let fv_x = left + x_offset + col_left + fv.hw;
    let mut fv_y = y_offset + bottom + fv.bottom + fv.hh;
    let pv_x = fv_x;
    let mut pv_y = fv_y + fv.hh + (fv.top + vertical_gutter + pv.bottom) + pv.hh;
    let mut sv_x = fv_x + fv.hw + col_right + side_gutter + sv.left + sv.hw;
    let mut sv_y = fv_y;
    if arrangement == Arrangement::StaggeredSide && has_side {
        // Lift the whole orthographic row clear of the pinned title block.
        let title_top = tb_bottom + TB_H + 4.0;
        let aligned_y = title_top + (fv.bottom + fv.hh).max(sv.bottom + sv.hh);
        let lift = (aligned_y - fv_y).max(0.0);
        fv_y += lift;
        sv_y = fv_y;
        if has_plan {
            pv_y += lift;
        }
    }
    let mut section_x = sv_x + sv.hw + sv.right + SECTION_GAP + section_hw;
    let mut section_y = fv_y;
    let section_block = Block {
        top: DIM_PAD,
        right: DIM_PAD,
        bottom: DIM_PAD + FONT_SIZE + 4.0,
        left: SECTION_GAP,
        ..Block::bare(section_hw, section_hh)
    };
    // Neighbouring blocks abut exactly: rounding never makes their bands overlap.
    if has_front && has_plan {
        let edge = fv.footprint(fv_x, fv_y)[3];
        pv_y = clear_of(edge, pv_y, |y| pv.footprint(pv_x, y)[1]);
    }
    if has_column && has_side {
        let edge = [(fv, fv_y, has_front), (pv, pv_y, has_plan)]
            .iter()
            .filter(|v| v.2)
            .map(|(b, y, _)| b.footprint(fv_x, *y)[2])
            .fold(f64::NEG_INFINITY, f64::max);
        sv_x = clear_of(edge, sv_x, |x| sv.footprint(x, sv_y)[0]);
    }
    if req.sections > 0 {
        let edge = sv.footprint(sv_x, sv_y)[2];
        section_x = clear_of(edge, section_x, |x| {
            section_block.footprint(x, section_y)[0]
        });
    }
    // draftwright's stride between sections (`2 hw + DIM_PAD`): the plan draws only the first.
    let section_at = |i: usize, x: f64, y: f64| {
        section_block.footprint(x + i as f64 * (2.0 * section_hw + DIM_PAD), y)
    };
    // No view's bands may lie on the title block (draftwright's packed verdict, `fits`, keeps
    // them off it; its row verdict assumes a DIM_PAD band below the views, which a deeper band
    // breaks). When they would, the row is lifted clear if the sheet has the height; else the
    // layout does not fit.
    let tb_rect: Rect = [
        page_w - tb_right - tb_w,
        tb_bottom,
        page_w - tb_right,
        tb_bottom + TB_H,
    ];
    let placed_boxes = |fv_y: f64, pv_y: f64, sv_y: f64, section_y: f64| -> Vec<Rect> {
        [
            (fv, fv_x, fv_y, has_front),
            (pv, pv_x, pv_y, has_plan),
            (sv, sv_x, sv_y, has_side),
        ]
        .iter()
        .filter(|v| v.3)
        .map(|(b, x, y, _)| b.footprint(*x, *y))
        .chain((0..req.sections).map(|i| section_at(i, section_x, section_y)))
        .collect()
    };
    let on_tb = |boxes: &[Rect]| boxes.iter().any(|b| overlaps(b, &tb_rect));
    let boxes = placed_boxes(fv_y, pv_y, sv_y, section_y);
    let mut tb_clear = !on_tb(&boxes);
    let mut lifted_by = 0.0;
    if !tb_clear {
        let mut lift = boxes
            .iter()
            .filter(|b| overlaps(b, &tb_rect))
            .map(|b| tb_rect[3] - b[1])
            .fold(0.0, f64::max);
        let lifted =
            |lift: f64| placed_boxes(fv_y + lift, pv_y + lift, sv_y + lift, section_y + lift);
        for _ in 0..8 {
            if !on_tb(&lifted(lift)) {
                break;
            }
            lift = lift.next_up();
        }
        let top_edge = boxes.iter().map(|b| b[3]).fold(f64::NEG_INFINITY, f64::max);
        if !on_tb(&lifted(lift)) && top_edge + lift <= page_h - top + TOL {
            fv_y += lift;
            pv_y += lift;
            sv_y += lift;
            section_y += lift;
            lifted_by = lift;
            tb_clear = true;
        }
    }
    let drawable: Rect = [left, bottom, page_w - right, page_h - top];

    // The pinned title block, and the isometric in the largest gap left.
    let title_block = Block {
        top: DIM_PAD,
        right: DIM_PAD,
        bottom: tb_bottom - bottom,
        left: DIM_PAD,
        ..Block::bare(tb_w / 2.0, TB_H / 2.0)
    };
    let (tb_cx, tb_cy) = (page_w - tb_right - tb_w / 2.0, tb_bottom + TB_H / 2.0);
    let tb_box = title_block.footprint(tb_cx, tb_cy);
    let mut obstacles: Vec<Rect> = Vec::new();
    let views = [
        (fv, fv_x, fv_y, has_front),
        (pv, pv_x, pv_y, has_plan),
        (sv, sv_x, sv_y, has_side),
    ];
    for (b, x, y, present) in views {
        if present {
            // A repack packs around the measured blocks. The estimate keeps draftwright's padded
            // silhouettes and, as its repack would (#121), the full bands too: the port's plan is
            // the estimate, with no measured repack to move the isometric off a deeper band.
            obstacles.push(match measured {
                Some(_) => b.footprint(x, y),
                None => Block {
                    top: b.top.max(DIM_PAD),
                    right: b.right.max(DIM_PAD),
                    bottom: b.bottom.max(DIM_PAD),
                    left: b.left.max(DIM_PAD),
                    ..b
                }
                .footprint(x, y),
            });
        }
    }
    obstacles.push(tb_box);
    let section_boxes: Vec<Rect> = (0..req.sections)
        .map(|i| section_at(i, section_x, section_y))
        .collect();
    obstacles.extend(section_boxes.iter().copied());
    let view_footprints: Vec<Rect> = views
        .iter()
        .filter(|v| v.3)
        .map(|(b, x, y, _)| b.footprint(*x, *y))
        .collect();

    // Derived views take the full planned blocks as obstacles, and keep off a Y step row above
    // the side view.
    let side_step_band =
        if has_plan && !scaled.derived.is_empty() && strips.plan_demand && strips.side_step_y {
            STRIP_GAP + SLOT_DIM_STEP
        } else {
            0.0
        };
    let derived_side = Block {
        top: sv.top.max(side_step_band),
        ..sv
    };
    let mut derived_obstacles: Vec<Rect> = Vec::new();
    if !scaled.derived.is_empty() {
        for (b, x, y, present) in [
            (fv, fv_x, fv_y, has_front),
            (pv, pv_x, pv_y, has_plan),
            (derived_side, sv_x, sv_y, has_side),
        ] {
            if present {
                derived_obstacles.push(b.footprint(x, y));
            }
        }
        derived_obstacles.push(tb_box);
        derived_obstacles.extend(section_boxes.iter().copied());
    }
    let mut derived = Vec::new();
    let mut derived_fit = true;
    for (id, w, h) in &scaled.derived {
        let r = largest_empty_rect(drawable, &derived_obstacles, [*w, *h]);
        if r[2] - r[0] < *w || r[3] - r[1] < *h || derived_obstacles.iter().any(|o| overlaps(&r, o))
        {
            derived_fit = false;
            break;
        }
        let (cx, cy) = ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0);
        let b = [cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0];
        derived.push((id.clone(), b));
        derived_obstacles.push(b);
        obstacles.push(b);
    }
    if !derived_fit && prefer_gutters && gutters {
        // Recompose with the safety gutters alone before giving up a derived view's room.
        return geometry_with(
            req,
            scaled,
            scale,
            [page_w, page_h],
            tb_w,
            arrangement,
            measured,
            false,
        );
    }
    let mut required_fit = true;
    let mut required_boxes = Vec::new();
    for (size, corner) in &req.required_tables {
        let Some([x0, y0]) = fit_auxiliary_box(*size, drawable, &obstacles, *corner, 2.0) else {
            required_fit = false;
            break;
        };
        let b = [x0, y0, x0 + size[0], y0 + size[1]];
        required_boxes.push(b);
        obstacles.push(b);
    }
    let iso = largest_empty_rect(drawable, &obstacles, [1.0, 1.0]);
    let iso_valid = !obstacles.iter().any(|o| overlaps(&iso, o));

    // Does the packed layout fit?
    let mut placed = view_footprints.clone();
    placed.extend(section_boxes.iter().copied());
    let cx0 = placed.iter().map(|b| b[0]).fold(f64::INFINITY, f64::min);
    let cy0 = placed.iter().map(|b| b[1]).fold(f64::INFINITY, f64::min);
    let cx1 = placed
        .iter()
        .map(|b| b[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let cy1 = placed
        .iter()
        .map(|b| b[3])
        .fold(f64::NEG_INFINITY, f64::max);
    // Judged on the row as draftwright packs it: the lift above only keeps bands off the title
    // block, and never lets a row past the block's left edge that draftwright would refuse.
    let clears_tb = cy0 - lifted_by >= tb_bottom + TB_H;
    let right_limit = if clears_tb {
        page_w - right
    } else {
        page_w - tb_w - right
    };
    let auto_clears_tb = y_offset + bottom + DIM_PAD >= reserved_tb_bottom + TB_H;
    let auto_row_w = total_content_w + (left + right) + if auto_clears_tb { 0.0 } else { tb_w };
    let auto_row_fits = auto_row_w <= page_w + TOL;
    let iso_fit = (iso[2] - iso[0]).min(iso[3] - iso[1]);
    let iso_min_fit = if iso_exact {
        iso_natural
    } else {
        ISO_MIN_FIT_FRAC * iso_natural
    };
    let iso_fits = !req.include_iso || (iso_valid && iso_fit >= iso_min_fit);
    let mut table_obstacles = view_footprints;
    table_obstacles.push(tb_box);
    table_obstacles.extend(section_boxes.iter().copied());
    table_obstacles.extend(derived.iter().map(|d| d.1));
    if req.include_iso && iso_valid {
        table_obstacles.push(iso);
    }
    table_obstacles.extend(&required_boxes);
    let table_fits = req.table_sizes.is_empty()
        || req
            .table_sizes
            .iter()
            .any(|s| fit_box(*s, drawable, &table_obstacles, Corner::TopRight, 0.0).is_some());
    let inside = cy0 >= bottom - TOL && cy1 <= page_h - top + TOL && cx0 >= left - TOL;
    let fits = iso_fits
        && derived_fit
        && table_fits
        && required_fit
        && inside
        && tb_clear
        && cx1 <= right_limit + TOL;
    let auto_fits =
        auto_row_fits && derived_fit && table_fits && required_fit && inside && tb_clear;
    Geometry {
        arrangement,
        front: [fv_x, fv_y],
        plan: [pv_x, pv_y],
        side: [sv_x, sv_y],
        section: [section_x, section_y],
        blocks: [fv, pv, sv],
        iso,
        iso_centre: [(iso[0] + iso[2]) / 2.0, (iso[1] + iso[3]) / 2.0],
        iso_valid,
        iso_fits,
        title_block: [
            tb_cx - tb_w / 2.0,
            tb_bottom,
            tb_cx + tb_w / 2.0,
            tb_bottom + TB_H,
        ],
        section_boxes,
        derived,
        vertical_gutter,
        side_gutter,
        x_offset,
        fits,
        auto_fits,
    }
}

/// How the scale and sheet were reached when no standard choice fits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fallback {
    /// The largest scale that fits the largest sheet, found by bisection (not a standard scale).
    Computed,
    /// Nothing fits; the last candidate is used as it is.
    Overflow,
    /// A pinned scale and sheet that may not fit.
    Uncertain,
}

/// A scale, a sheet, its title-block width and the arrangement it was chosen under.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub scale: f64,
    pub page: [f64; 2],
    pub tb_w: f64,
    pub arrangement: Arrangement,
    pub fallback: Option<Fallback>,
}

/// Chooses the scale, sheet and arrangement as draftwright does (`compose.choose_scale`):
/// - neither pinned: the first fit on the preference ladder, judged by the row (`auto_fits`);
/// - a pinned sheet: the largest standard scale whose packed footprints fit it (`fits`);
/// - a pinned scale: the smallest sheet that holds it;
/// - both: taken as they are.
///
/// The scale is chosen under the first arrangement alone; the others may then take a smaller
/// sheet at that scale. *scaled* gives what depends on the scale.
pub fn choose(
    req: &Request,
    scaled: &dyn Fn(f64) -> Scaled,
    scale: Option<f64>,
    page: Option<[f64; 2]>,
    arrangements: &[Arrangement],
    tb_width: Option<f64>,
) -> Pick {
    let preferred = arrangements[0];
    let tb_of = |w: f64| tb_width.unwrap_or_else(|| title_block_width(w));
    let pick = |s: f64, p: [f64; 2], a: Arrangement, fallback| Pick {
        scale: s,
        page: p,
        tb_w: tb_of(p[0]),
        arrangement: a,
        fallback,
    };
    let fit = |s: f64, p: [f64; 2], a: Arrangement, packed: bool| {
        let g = geometry(req, &scaled(s), s, p, tb_of(p[0]), a, None);
        if packed { g.fits } else { g.auto_fits }
    };
    if let (Some(s), Some(p)) = (scale, page) {
        let fallback = (!fit(s, p, preferred, true)).then_some(Fallback::Uncertain);
        return pick(s, p, preferred, fallback);
    }
    let (candidates, packed): (Vec<(f64, [f64; 2])>, bool) = match (scale, page) {
        (None, Some(p)) => (SCALES.iter().map(|s| (*s, p)).collect(), true),
        (Some(s), None) => (PAGES.iter().map(|(_, w, h)| (s, [*w, *h])).collect(), true),
        _ => (LADDER.iter().map(|(s, n)| (*s, sheet(n))).collect(), false),
    };
    if let Some(i) = candidates
        .iter()
        .position(|(s, p)| fit(*s, *p, preferred, packed))
    {
        let (s, p) = candidates[i];
        // A smaller sheet at the same scale under another arrangement.
        for (cs, cp) in &candidates[..i] {
            if *cs != s {
                continue;
            }
            if let Some(a) = arrangements[1..]
                .iter()
                .find(|a| fit(*cs, *cp, **a, packed))
            {
                return pick(*cs, *cp, *a, None);
            }
        }
        return pick(s, p, preferred, None);
    }
    let (_, last) = *candidates.last().expect("candidates");
    if scale.is_none() {
        // Past the ladder: the largest scale that fits the largest sheet. draftwright bisects
        // with the three principals in columns, whatever was asked.
        let all = Request {
            views: Principals::ALL,
            ..req.clone()
        };
        let (mut lo, mut hi) = (0.0, SCALES[SCALES.len() - 1]);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            let g = geometry(
                &all,
                &scaled(mid),
                mid,
                last,
                tb_of(last[0]),
                Arrangement::Columns,
                None,
            );
            if if packed { g.fits } else { g.auto_fits } {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo > 0.0 {
            return pick(lo, last, Arrangement::Columns, Some(Fallback::Computed));
        }
    }
    let (s, p) = *candidates.last().expect("candidates");
    pick(s, p, Arrangement::Columns, Some(Fallback::Overflow))
}

/// A re-plan the automatic layout recovery tries (ISSUE-1299-a): a standard scale on the sheet
/// already chosen, or a larger standard sheet at the scale its fixed-sheet rule picks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Retry {
    Scale(f64),
    Sheet(&'static str),
}

/// What a finished plan's findings say of it, for the recovery: its hard layout failures (ink
/// off the sheet, labels overlapping, a line through a label, an overflowing title field, views
/// that do not fit) and the required items it lost for want of room.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Findings {
    pub hard: usize,
    pub dropped: usize,
}

impl Findings {
    /// Valid (no hard failure) and complete (nothing lost).
    pub fn clean(&self) -> bool {
        self.hard == 0 && self.dropped == 0
    }
}

/// One re-plan tried: its findings (`None`: it could not be planned) and whether it was taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attempt {
    pub retry: Retry,
    pub findings: Option<Findings>,
    pub taken: bool,
}

/// The larger scales a required loss tries on the chosen sheet (draftwright's
/// `_AUTOMATIC_UPSCALE_TRIAL_LIMIT`), and the scales each way a hard layout failure tries
/// (`_AUTOMATIC_VALIDITY_SCALE_TRIAL_LIMIT`): each try is a whole plan.
pub const UPSCALE_TRIALS: usize = 2;
pub const VALIDITY_TRIALS: usize = 1;

/// draftwright's bounded, ordered recovery on a finished drawing (`builder._AutomaticResolution`,
/// `recover_hard_layout` and `recover_required_no_iso`), for a plan at *scale* on *page* whose
/// findings are *first*:
/// - a hard layout failure comes first, whatever was lost: the nearest smaller then the nearest
///   larger standard scale on the same sheet, then each larger standard sheet;
/// - else a required loss: the next larger scales on the same sheet, then each larger sheet;
/// - else nothing: a valid, complete plan never spends a larger sheet.
///
/// A sheet the spec names (*page_pinned*) is never left. The first clean try is taken (*plan*
/// gives each try's findings); the tries are returned in order, the taken one last. None taken:
/// the first plan stands.
pub fn recover(
    scale: f64,
    page: [f64; 2],
    page_pinned: bool,
    first: Findings,
    plan: &mut dyn FnMut(Retry) -> Option<Findings>,
) -> Vec<Attempt> {
    let smaller = SCALES.iter().copied().filter(|s| *s < scale);
    let mut larger: Vec<f64> = SCALES.iter().copied().filter(|s| *s > scale).collect();
    larger.reverse();
    let scales: Vec<f64> = if first.hard > 0 {
        smaller
            .take(VALIDITY_TRIALS)
            .chain(larger.into_iter().take(VALIDITY_TRIALS))
            .collect()
    } else if first.dropped > 0 {
        larger.into_iter().take(UPSCALE_TRIALS).collect()
    } else {
        return Vec::new();
    };
    let sheets = PAGES
        .iter()
        .position(|(_, w, h)| [*w, *h] == page)
        .filter(|_| !page_pinned)
        .map_or(&[][..], |i| &PAGES[i + 1..]);
    let retries = scales
        .into_iter()
        .map(Retry::Scale)
        .chain(sheets.iter().map(|(n, ..)| Retry::Sheet(n)));
    let mut attempts = Vec::new();
    for retry in retries {
        let findings = plan(retry);
        let taken = findings.is_some_and(|f| f.clean());
        attempts.push(Attempt {
            retry,
            findings,
            taken,
        });
        if taken {
            break;
        }
    }
    attempts
}

/// The least page distance between two located ordinates on one axis (draftwright's
/// `_MIN_LOC_SEP_MM`: an arrow and a pad), and the least page span of a location.
pub const MIN_LOC_SEP: f64 = 2.7 + PAD;
pub const MIN_LOC_SPAN: f64 = 1.0;

/// How many of one view axis's located *ordinates* (distances from the datum) are illegible at
/// *scale* by draftwright's measure (`_discard_short_refs`, `_legible_locations`): a nonzero
/// span under [`MIN_LOC_SPAN`] on the page, or one under [`MIN_LOC_SEP`] past the last legible
/// one. draftwright drops those (`location_ref_dropped`, a required loss); the port keeps every
/// location (LINT-hole_coverage-location), so for it this counts witness lines too close to read.
pub fn crowded_locations(ordinates: &[f64], scale: f64) -> usize {
    let mut at: Vec<f64> = ordinates.iter().map(|o| o.abs()).collect();
    at.sort_by(f64::total_cmp);
    let mut crowded = 0;
    let mut last: Option<f64> = None;
    for o in at {
        if (1e-6 < o && o * scale < MIN_LOC_SPAN)
            || last.is_some_and(|l| (o - l) * scale < MIN_LOC_SEP)
        {
            crowded += 1;
            continue;
        }
        last = Some(o);
    }
    crowded
}

/// How much the isometric is scaled relative to the sheet to fill its rectangle (draftwright's
/// `_fit_iso_view` with no annotation obstacles), given the projected iso's box relative to the
/// view's centre at the sheet scale. One when it is left at sheet scale.
pub fn iso_factor(rect: Rect, centre: [f64; 2], bounds: Rect) -> f64 {
    let ratios: Vec<f64> = [
        (-bounds[0], centre[0] - rect[0]),
        (bounds[2], rect[2] - centre[0]),
        (-bounds[1], centre[1] - rect[1]),
        (bounds[3], rect[3] - centre[1]),
    ]
    .iter()
    .filter(|(extent, _)| *extent > 0.0)
    .map(|(extent, avail)| avail / extent)
    .collect();
    let needed = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    let needed = if needed.is_finite() { needed } else { 1.0 };
    let factor = if needed >= 1.0 {
        // Grow to 90 % of the rectangle, keeping the orientation view subordinate.
        ((needed * 0.90 * 10000.0).floor() / 10000.0).clamp(1.0, ISO_MAX_GROW)
    } else {
        // Shrink to fit, with a 2 % margin.
        (needed * 0.98 * 10000.0).floor() / 10000.0
    };
    if factor.is_nan() || factor <= 0.0 || (factor - 1.0).abs() < 0.05 {
        1.0
    } else {
        factor
    }
}

/// The free length along a sorted coordinate list, as cells: a zero-length cell at each
/// coordinate and the interval to the next. A segment tree of cover counts gives the longest
/// free run and the first run of a given length (draftwright's `_FreeRunIndex`).
struct FreeRuns<'a> {
    coordinates: &'a [f64],
    n: usize,
    cover: Vec<u32>,
    total: Vec<f64>,
    prefix: Vec<f64>,
    suffix: Vec<f64>,
    longest: Vec<f64>,
    all_free: Vec<bool>,
}

impl<'a> FreeRuns<'a> {
    fn new(coordinates: &'a [f64]) -> Self {
        let mut lengths = Vec::new();
        for (i, c) in coordinates.iter().enumerate() {
            lengths.push(0.0);
            if i + 1 < coordinates.len() {
                lengths.push(coordinates[i + 1] - c);
            }
        }
        let n = lengths.len();
        let mut runs = FreeRuns {
            coordinates,
            n,
            cover: vec![0; 4 * n],
            total: vec![0.0; 4 * n],
            prefix: vec![0.0; 4 * n],
            suffix: vec![0.0; 4 * n],
            longest: vec![0.0; 4 * n],
            all_free: vec![true; 4 * n],
        };
        runs.build(1, 0, n, &lengths);
        runs
    }

    fn build(&mut self, node: usize, lo: usize, hi: usize, lengths: &[f64]) {
        if hi - lo == 1 {
            let l = lengths[lo];
            self.total[node] = l;
            self.prefix[node] = l;
            self.suffix[node] = l;
            self.longest[node] = l;
            return;
        }
        let mid = (lo + hi) / 2;
        self.build(node * 2, lo, mid, lengths);
        self.build(node * 2 + 1, mid, hi, lengths);
        self.pull(node);
    }

    fn pull(&mut self, node: usize) {
        if self.cover[node] > 0 {
            self.all_free[node] = false;
            self.prefix[node] = 0.0;
            self.suffix[node] = 0.0;
            self.longest[node] = 0.0;
            return;
        }
        let (l, r) = (node * 2, node * 2 + 1);
        self.total[node] = self.total[l] + self.total[r];
        self.all_free[node] = self.all_free[l] && self.all_free[r];
        self.prefix[node] = self.prefix[l];
        if self.all_free[l] {
            self.prefix[node] += self.prefix[r];
        }
        self.suffix[node] = self.suffix[r];
        if self.all_free[r] {
            self.suffix[node] += self.suffix[l];
        }
        self.longest[node] = self.longest[l]
            .max(self.suffix[l] + self.prefix[r])
            .max(self.longest[r]);
    }

    fn longest(&self) -> f64 {
        self.longest[1]
    }

    /// Blocks the open span (lo, hi); touching an obstacle is allowed.
    fn block(&mut self, lo: f64, hi: f64) {
        let first = self.coordinates.partition_point(|c| *c < lo);
        let last = self.coordinates.partition_point(|c| *c < hi);
        let (a, b) = if lo == hi {
            (2 * first, 2 * first + 1)
        } else {
            (2 * first + 1, 2 * last)
        };
        self.block_range(1, 0, self.n, a, b);
    }

    fn block_range(&mut self, node: usize, lo: usize, hi: usize, a: usize, b: usize) {
        if a <= lo && hi <= b {
            self.cover[node] += 1;
            self.pull(node);
            return;
        }
        let mid = (lo + hi) / 2;
        if a < mid {
            self.block_range(node * 2, lo, mid, a, b);
        }
        if mid < b {
            self.block_range(node * 2 + 1, mid, hi, a, b);
        }
        self.pull(node);
    }

    /// The lowest start of a free run at least *minimum* long.
    fn first_run(&self, minimum: f64) -> f64 {
        self.first_run_in(1, 0, self.n, self.coordinates[0], minimum)
    }

    fn first_run_in(&self, node: usize, lo: usize, hi: usize, origin: f64, minimum: f64) -> f64 {
        if hi - lo == 1 {
            return origin;
        }
        let (l, r) = (node * 2, node * 2 + 1);
        let mid = (lo + hi) / 2;
        if self.longest[l] >= minimum {
            return self.first_run_in(l, lo, mid, origin, minimum);
        }
        if self.suffix[l] + self.prefix[r] >= minimum {
            return origin + self.total[l] - self.suffix[l];
        }
        self.first_run_in(r, mid, hi, origin + self.total[l], minimum)
    }
}

/// The largest empty rectangle of *drawable* that overlaps no obstacle, maximising the uniform
/// scale of *target* (`[1, 1]`: the largest square it holds). The whole drawable when there is
/// none. Draftwright's `_largest_empty_rect`, with its deterministic winners.
pub fn largest_empty_rect(drawable: Rect, obstacles: &[Rect], target: [f64; 2]) -> Rect {
    let [tw, th] = target;
    let [dx0, dy0, dx1, dy1] = drawable;
    let sorted = |a: f64, b: f64, pick: &dyn Fn(&Rect) -> [f64; 2]| {
        let mut v = vec![a, b];
        for o in obstacles {
            for c in pick(o) {
                if a < c && c < b {
                    v.push(c);
                }
            }
        }
        v.sort_by(f64::total_cmp);
        v.dedup();
        v
    };
    let xs = sorted(dx0, dx1, &|o| [o[0], o[2]]);
    let ys = sorted(dy0, dy1, &|o| [o[1], o[3]]);
    let mut events: Vec<[f64; 4]> = obstacles
        .iter()
        .filter_map(|o| {
            let (lo, hi) = (dy0.max(o[1]), dy1.min(o[3]));
            (!(hi < dy0 || lo > dy1)).then_some([o[0], o[2], lo, hi])
        })
        .collect();
    events.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut best: Option<Rect> = None;
    let mut best_score = 0.0;
    let nx = xs.len();
    for i in 0..nx - 1 {
        let rx0 = xs[i];
        if xs[nx - 1] - rx0 <= best_score * tw {
            break;
        }
        let mut free = FreeRuns::new(&ys);
        let mut e = 0;
        let start = xs.partition_point(|x| *x <= rx0 + best_score * tw);
        for &rx1 in &xs[start..] {
            let width = rx1 - rx0;
            while e < events.len() && events[e][0] < rx1 {
                let [_, ox1, lo, hi] = events[e];
                if rx0 < ox1 {
                    free.block(lo, hi);
                }
                e += 1;
            }
            let height = free.longest();
            let score = (width / tw).min(height / th);
            if score <= best_score {
                continue;
            }
            let needed = (width * th / tw).min(height);
            let ry0 = free.first_run(needed);
            let edge = ys.partition_point(|y| *y < ry0 + needed);
            let ry1 = ys[edge.min(ys.len() - 1)];
            best_score = score;
            best = Some([rx0, ry0, rx1, ry1]);
        }
    }
    best.unwrap_or(drawable)
}

/// Places a `w × h` box in *region* clear of *obstacles* (each inflated by *clearance*), as
/// near the *prefer* corner as it can go: its lower-left corner, or none. Draftwright's
/// `fit_box`: candidates are flush with a region or obstacle edge, tried nearest first.
pub fn fit_box(
    [w, h]: [f64; 2],
    region: Rect,
    obstacles: &[Rect],
    prefer: Corner,
    clearance: f64,
) -> Option<[f64; 2]> {
    let [rx0, ry0, rx1, ry1] = region;
    if w > rx1 - rx0 || h > ry1 - ry0 {
        return None;
    }
    let obs: Vec<Rect> = obstacles
        .iter()
        .map(|b| {
            [
                b[0] - clearance,
                b[1] - clearance,
                b[2] + clearance,
                b[3] + clearance,
            ]
        })
        .filter(|b| overlaps(b, &region))
        .collect();
    let candidates = |lo: f64, hi: f64, size: f64, edges: Vec<f64>| {
        let mut v: Vec<f64> = edges
            .into_iter()
            .flat_map(|e| [e, e - size])
            .filter(|x| lo <= *x && *x <= hi - size)
            .collect();
        v.sort_by(f64::total_cmp);
        v.dedup();
        v
    };
    let mut x_edges = vec![rx0, rx1];
    let mut y_edges = vec![ry0, ry1];
    for o in &obs {
        x_edges.extend([o[0], o[2]]);
        y_edges.extend([o[1], o[3]]);
    }
    let xs = candidates(rx0, rx1, w, x_edges);
    let ys = candidates(ry0, ry1, h, y_edges);
    let right = matches!(prefer, Corner::BottomRight | Corner::TopRight);
    let top = matches!(prefer, Corner::TopLeft | Corner::TopRight);
    let (cx, cy) = (if right { rx1 } else { rx0 }, if top { ry1 } else { ry0 });
    let mut ranked: Vec<(f64, f64, f64)> = Vec::new();
    for &bx in &xs {
        let sx = ((if right { bx + w } else { bx }) - cx).powi(2);
        for &by in &ys {
            let sy = ((if top { by + h } else { by }) - cy).powi(2);
            ranked.push((sx + sy, bx, by));
        }
    }
    ranked.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
    });
    ranked
        .into_iter()
        .find(|(_, bx, by)| {
            let b = [*bx, *by, bx + w, by + h];
            !obs.iter().any(|o| overlaps(&b, o))
        })
        .map(|(_, bx, by)| [bx, by])
}

/// A table's place: first with breathing room, then at the hard clearance.
pub(crate) fn fit_auxiliary_box(
    size: [f64; 2],
    region: Rect,
    obstacles: &[Rect],
    prefer: Corner,
    clearance: f64,
) -> Option<[f64; 2]> {
    let preferred = clearance.max(PREFERRED_BLOCK_GUTTER);
    if preferred > clearance
        && let Some(p) = fit_box(size, region, obstacles, prefer, preferred)
    {
        return Some(p);
    }
    fit_box(size, region, obstacles, prefer, clearance)
}
