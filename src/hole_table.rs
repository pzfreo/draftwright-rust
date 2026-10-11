//! Drop and escalate (the maintainer's decision of 2026-10-09: draftwright's
//! `_prepare_hole_table_run`): a hole callout or hole location the placement could not put on the
//! sheet is dropped and reported, and the drop escalates to a plan-view hole table with balloons.
//!
//! The port's placement keeps every item and reports what collides, so "could not be placed" is
//! read from the placed plan: a hole callout (its bore diameter, of holes or a hole pattern) or a
//! hole location (`location.length.*` of holes or hole patterns only) that placement reports
//! `annotation_off_sheet` has no place on the sheet. Its ink is withdrawn (it stays in the plan,
//! unplaced, as an item no shown view carries does) and a problem says so, with draftwright's
//! code: `callout_dropped`, `location_ref_dropped` for a location in the plan view, or
//! `off_axis_location_dropped` (info, as draftwright's) for a location of a hole the plan does
//! not see end-on. Every drop is a required loss the layout recovery reads
//! ([`crate::model::recover`]), as every `*_dropped` is a blocker of draftwright's
//! `_is_required_scale_drop`.
//!
//! Then draftwright's escalation, with its trigger and gates (`orchestrator.py`
//! `_maybe_tabulate_holes_impl`):
//!
//! - It runs only when a callout or a plan-view location was dropped; an off-axis location drop
//!   never tabulates (draftwright's distinct code, `hole_locations.py` `_off_axis_drop`).
//! - **The table** is warranted only for a dense plan view: when the plan view's holes outside
//!   any recognised pattern (each member of each group of identical holes, in the plan's order)
//!   number at least 16 (`_TABULATE_MIN_HOLES`). It then has one row per such hole, tagged A, B,
//!   …, Z, AA, … (`_tag_sequence`), `TAG | ⌀ | DEPTH | X | Y`: the diameter as the hole's callout
//!   states it, its tolerance included; THRU or the depth; the X and Y locations from the part's
//!   box, as the plan's location dimensions state them (blank when none does). It replaces the
//!   plan-view callouts of plain holes (no counterbore, spotface or countersink, no authored
//!   tolerance or fit: `_hole_table_replaceable_feature`) and the plan-view location dimensions
//!   that locate only tabulated holes; a compound callout keeps its leader beside its row. The
//!   table goes towards the sheet's top right clear of the views, the remaining dimensions, the
//!   title block, the notes block and the spec's tables, widened into up to four side-by-side
//!   blocks until it fits (`_wrap_rows`, `_build_table`'s `block_cols`).
//! - **Each row has a balloon**, a ring of 4.5 mm radius with its tag, leadered from the hole's
//!   rim: draftwright's `render_balloons` bands left, right, above and below the plan view, each
//!   beyond the deepest annotation placed there and along the view's reserved area, and its strip
//!   spacing. The table is kept only if every row's balloon lands; otherwise it is withdrawn
//!   whole, every replaced item restored and the drops left reported (`table_dropped`,
//!   `balloon_dropped`), as draftwright rolls its transaction back.
//! - **A dropped pattern callout** in the plan view gets one balloon `n×TAG` on its first member,
//!   tagged after the table's rows. It does not state the pattern's sizes, so its
//!   `callout_dropped` stays (draftwright's non-certifying marker).
//!
//! A table that replaces a dropped item clears its drop: the row states what it would have.
//! Lint never reads the plan: the drawing records each row as an annotation claiming the row's
//! hole's bore diameter (and depth) and the locations its cells state, with the cells' text as
//! its own ([`crate::model::render`]), so hole coverage credits the rows as it credits a callout,
//! as draftwright registers the table's measurements and requirements.
//!
//! Departures from draftwright, each with its verdict (`docs/rules`, `tests/hole_tables.rs`):
//! - "Could not be placed" is an item placed off the sheet. draftwright also drops an item that
//!   collides or whose ladder is illegibly crowded; the port places those and reports them
//!   (rust-correct for now: the collisions are another stream's, and an illegible ladder is read
//!   by the recovery from the placed plan).
//! - The balloons are placed more simply: no halo is reserved by the layout (draftwright's
//!   `_est_plan_halo`; the port's layout reserves none); each hole goes out of the side of the view
//!   nearest it that has a band, its band's members spread in order near their holes (draftwright
//!   assigns bands by min-cost max-flow with a perimeter preference); a ring that then overlaps
//!   another or a leader that crosses a label, another view, the title block, a table or another
//!   leader is tried again on every band, half a strip step at a time either way of its hole, the
//!   nearest clear place taken (draftwright's guarded inventory searches its own way). Leaders
//!   crossing other items' lines are not checked (undetermined: on NIST CTC-02, the only corpus
//!   part with a table attempt in both, both withdraw it for want of balloon room).
//! - The rows come from the plan's callouts: a hole whose callout the spec omits has no row
//!   (draftwright tabulates every z hole of its model, with blank cells) (undetermined).

use std::collections::{BTreeMap, BTreeSet};

use haecceity::hlr::View;
use serde::{Deserialize, Serialize};

use crate::bridge::holes;
use crate::dims::{Measurement, PlannedDimension};
use crate::layout::{Corner, Rect};
use crate::model::{Analysis, Plan, PlannedView, Problem};
use crate::tables::{self, PlannedTable, ROW};

/// The table's name, draftwright's.
pub const ID: &str = "hole_table_plan";
/// The fewest holes outside a pattern that warrant a table (draftwright's `_TABULATE_MIN_HOLES`).
pub const MIN_HOLES: usize = 16;
/// The table's header.
pub const HEADER: [&str; 5] = ["TAG", "⌀", "DEPTH", "X", "Y"];
/// A balloon's radius (draftwright's `_balloon_radius`: 1.5 text heights) and its ring's stroke.
pub const RADIUS: f64 = 1.5 * crate::dims::TEXT;
pub const RING: f64 = 0.35;
/// The clearance between the view's outline and what is placed beside it, and between strip
/// neighbours (draftwright's `_STRIP_GAP`, `_STRIP_SPACING`).
const STRIP_GAP: f64 = 10.0;
const STRIP_SPACING: f64 = 2.5;
/// The whitespace between side-by-side blocks of a wrapped table (`_table_metrics`' `3 * pad`).
const BLOCK_GAP: f64 = 3.0 * tables::PAD;
/// How many half strip steps a balloon is tried either way of its hole along a band.
const STEPS: usize = 40;

/// The drop codes: each a required loss to the layout recovery.
pub const DROPS: [&str; 3] = [
    "callout_dropped",
    "location_ref_dropped",
    "off_axis_location_dropped",
];

/// The plan-view hole table: one row per hole, drawn as blocks side by side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoleTable {
    pub id: String,
    /// One row per hole, in the plan's order.
    pub rows: Vec<HoleRow>,
    pub default: bool,
    pub basis: String,
    /// Always shown: a table withdrawn is not in the plan.
    pub state: crate::items::State,
    /// The blocks drawn, left to right: each the header and its share of the rows (the last
    /// padded with blank rows), placed.
    pub blocks: Vec<PlannedTable>,
}

/// A hole's row: its tag, its hole, the cells it prints and the measurements they state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoleRow {
    pub tag: String,
    pub feature: String,
    /// `TAG`, `⌀`, `DEPTH`, `X`, `Y`.
    pub cells: Vec<String>,
    pub measurements: Vec<Measurement>,
}

/// A balloon: a ring with its tag, leadered from a hole's rim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balloon {
    /// `balloon_plan_<tag>_0`, draftwright's name.
    pub id: String,
    pub tag: String,
    pub view: String,
    /// The holes it marks: one hole (a table row's), or a pattern's members.
    pub features: Vec<String>,
    /// Whether a table row states what it marks (a pattern's balloon states nothing).
    pub certifying: bool,
    /// The ring's centre, mm on the page.
    pub at: [f64; 2],
    pub radius: f64,
    /// The leader from the hole's rim (its arrowhead's tip) to the ring; none when the ring
    /// could not clear the hole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader: Option<[[f64; 2]; 2]>,
    /// Always shown: a balloon with no place is not in the plan.
    pub state: crate::items::State,
}

/// What a dropped item was.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Dropped {
    /// A callout; of a recognised pattern or not.
    Callout { pattern: bool },
    /// A location; in the plan view or another.
    Location { plan: bool },
}

/// A hole callout or hole location, by what it measures.
fn hole_item(d: &PlannedDimension) -> Option<Dropped> {
    let hole = |f: &str| f.starts_with("f:hole:") || f.starts_with("f:hole_pattern:");
    let ms = &d.measurements;
    if d.kind == "callout"
        && ms.iter().any(|m| m.parameter == "bore.diameter")
        && ms.iter().all(|m| hole(&m.feature))
    {
        return Some(Dropped::Callout {
            pattern: d.id.starts_with("d:f:hole_pattern:"),
        });
    }
    if d.kind == "linear"
        && !ms.is_empty()
        && ms
            .iter()
            .all(|m| m.parameter.starts_with("location.length.") && hole(&m.feature))
    {
        return Some(Dropped::Location {
            plan: d.view == "v:plan",
        });
    }
    None
}

/// draftwright's tags for *n* rows: `A, B, …, Z, AA, AB, …` (`_tag_sequence`).
pub fn tags(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            let mut s = Vec::new();
            let mut k = i as i64;
            loop {
                s.push(char::from(b'A' + (k % 26) as u8));
                k = k / 26 - 1;
                if k < 0 {
                    break;
                }
            }
            s.iter().rev().collect()
        })
        .collect()
}

/// The bore features *d* calls out, in its order.
fn bores(d: &PlannedDimension) -> Vec<&str> {
    d.measurements
        .iter()
        .filter(|m| m.parameter == "bore.diameter")
        .map(|m| m.feature.as_str())
        .collect()
}

/// A plan with a problem for *d*'s drop, its ink withdrawn and the placement problems about it
/// gone with it.
fn drop_item(plan: &mut Plan, i: usize, kind: Dropped) {
    let d = &mut plan.dimensions[i];
    d.placed = None;
    let id = d.id.clone();
    let (code, severity, what) = match kind {
        Dropped::Callout { .. } => ("callout_dropped", "warning", "hole callout"),
        Dropped::Location { plan: true } => ("location_ref_dropped", "warning", "hole location"),
        Dropped::Location { plan: false } => ("off_axis_location_dropped", "info", "hole location"),
    };
    let off_axis = if kind == (Dropped::Location { plan: false }) {
        "; a hole not seen in the plan has no hole-table row (draftwright's off-axis drop)"
    } else {
        ""
    };
    d.state = crate::items::State::not_drawable(code);
    let message = format!(
        "The {what} {id} ({}) is dropped from the {} view: placed, its ink ran off the sheet's drawing area, so it is not drawn{off_axis}. A larger sheet or a smaller scale may give it room.",
        d.label,
        d.view.trim_start_matches("v:")
    );
    let mut subjects = vec![id.clone()];
    for m in &d.measurements {
        if !subjects.contains(&m.feature) {
            subjects.push(m.feature.clone());
        }
    }
    plan.problems
        .retain(|p| !(p.code.starts_with("annotation_") && p.subjects.contains(&id)));
    plan.problems.push(Problem {
        id: format!("p:{code}:{id}"),
        code: code.into(),
        severity: severity.into(),
        message,
        subjects,
        ..Default::default()
    });
}

/// A view's world-to-page map and its outline on the page.
struct Frame {
    view: View,
    at: [f64; 2],
    s: f64,
    centre: [f64; 2],
    outline: Rect,
}

impl Frame {
    fn new(analysis: &Analysis, v: &PlannedView) -> Frame {
        let view = View::new(v.direction, v.up).expect("a planned view");
        let (size, centre) = crate::model::extent(analysis, &view);
        let (at, s) = (v.at, v.scale);
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

    fn page(&self, p: [f64; 3]) -> [f64; 2] {
        let q = self.view.map(p);
        [
            self.at[0] + self.s * (q[0] - self.centre[0]),
            self.at[1] + self.s * (q[1] - self.centre[1]),
        ]
    }
}

/// Drops what has no place and escalates as draftwright does (the module's doc). Applied to
/// every plan the recovery judges.
pub fn escalate(analysis: &Analysis, plan: &mut Plan) {
    let off: BTreeSet<String> = plan
        .problems
        .iter()
        .filter(|p| p.code == "annotation_off_sheet" && p.severity != "info")
        .filter_map(|p| p.subjects.first().cloned())
        .collect();
    let drops: Vec<(usize, Dropped)> = plan
        .dimensions
        .iter()
        .enumerate()
        .filter(|(_, d)| d.placed.is_some() && off.contains(&d.id))
        .filter(|(_, d)| {
            !plan
                .spec
                .dimensions
                .get(&d.id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.pinned)
        })
        .filter_map(|(i, d)| Some((i, hole_item(d)?)))
        .collect();
    for &(i, kind) in &drops {
        drop_item(plan, i, kind);
    }
    let escalating: Vec<usize> = drops
        .iter()
        .filter(|(_, k)| !matches!(k, Dropped::Location { plan: false }))
        .map(|(i, _)| *i)
        .collect();
    if escalating.is_empty() {
        return;
    }
    let Some(view) = plan.views.iter().find(|v| v.id == "v:plan").cloned() else {
        return;
    };
    let frame = Frame::new(analysis, &view);
    let sites: BTreeMap<&str, ([f64; 3], f64)> = holes::read(analysis)
        .into_iter()
        .filter_map(|(f, h)| {
            let h = h.ok()?;
            Some((f.id.as_str(), (h.at, h.diameter)))
        })
        .collect();
    let hole_site = |f: &str| {
        sites
            .get(f)
            .map(|(at, d)| (frame.page(*at), 0.5 * d * frame.s))
    };

    // The dropped pattern callouts of the plan view, each a balloon of its own.
    let patterns: Vec<usize> = drops
        .iter()
        .filter(|(i, k)| {
            *k == Dropped::Callout { pattern: true } && plan.dimensions[*i].view == "v:plan"
        })
        .map(|(i, _)| *i)
        .collect();
    // The plan view's holes outside any pattern: each member of each group's callout.
    let loose = loose(plan);
    let tabulate = loose.len() >= MIN_HOLES;
    if !tabulate && patterns.is_empty() {
        let why = format!(
            " No hole table: the plan view has {} hole{} outside a pattern, fewer than the {MIN_HOLES} that warrant one (draftwright's), and no pattern's callout was dropped there.",
            loose.len(),
            if loose.len() == 1 { "" } else { "s" }
        );
        for &i in &escalating {
            say(plan, i, &why);
        }
        return;
    }
    let n_rows = if tabulate { loose.len() } else { 0 };
    let all_tags = tags(n_rows + patterns.len());
    let (row_tags, pattern_tags) = all_tags.split_at(n_rows);

    let mut attempt = plan.dimensions.clone();
    let mut table = None;
    let mut replaced: Vec<usize> = Vec::new();
    let mut table_failure = None;
    if tabulate {
        let (rows, replace) = rows(analysis, plan, &loose, row_tags);
        if replace.iter().any(|&i| {
            plan.spec
                .dimensions
                .get(&plan.dimensions[i].id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.pinned)
        }) {
            for &(i, _) in &drops {
                say(
                    plan,
                    i,
                    " No hole table: it would replace a pinned hole item.",
                );
            }
            return;
        }
        for &i in &replace {
            attempt[i].placed = None;
            attempt[i].state = crate::items::State::Covered { by: ID.into() };
        }
        match place_table(analysis, plan, &attempt, &rows) {
            Some(blocks) => {
                table = Some(HoleTable {
                    id: ID.into(),
                    rows,
                    default: true,
                    state: crate::items::State::Shown,
                    basis: format!(
                        "draftwright's drop and escalate: a hole callout or location had no place on the sheet, and the plan view has {} holes outside a pattern (at least {MIN_HOLES}), so each is a row, tagged and ballooned, and the table replaces the plan view's plain callouts and the locations of its holes; towards the sheet's top right in {} block{}",
                        loose.len(),
                        blocks.len(),
                        if blocks.len() == 1 { "" } else { "s" }
                    ),
                    blocks,
                });
                replaced = replace;
            }
            None => {
                attempt = plan.dimensions.clone();
                table_failure = Some(
                    "it has no place on the sheet clear of the views, the dimensions, the title block, the notes and the spec's tables, even widened into four blocks"
                        .to_string(),
                );
            }
        }
    }

    // The balloons: the rows' first (they decide the table), then the patterns'.
    let mut specs: Vec<Wanted> = Vec::new();
    if let Some(t) = &table {
        for r in &t.rows {
            if let Some((hole, radius)) = hole_site(&r.feature) {
                specs.push(Wanted {
                    tag: r.tag.clone(),
                    features: vec![r.feature.clone()],
                    hole,
                    radius,
                    certifying: true,
                });
            }
        }
    }
    for (&i, tag) in patterns.iter().zip(pattern_tags) {
        let d = &plan.dimensions[i];
        let members: Vec<String> = bores(d).into_iter().map(String::from).collect();
        if let Some((hole, radius)) = members.first().and_then(|f| hole_site(f)) {
            specs.push(Wanted {
                tag: format!("{}×{tag}", members.len()),
                features: members,
                hole,
                radius,
                certifying: false,
            });
        }
    }
    let fixed = fixed_obstacles(analysis, plan, &view, table.as_ref());
    let mut landed = place_balloons(&specs, &frame, plan, &attempt, &fixed, table.is_some());
    let rows_landed = table.as_ref().is_none_or(|t| {
        t.rows.len() == specs.iter().filter(|s| s.certifying).count()
            && landed
                .iter()
                .filter(|b| b.as_ref().is_some_and(|b| b.certifying))
                .count()
                == t.rows.len()
    });
    if !rows_landed {
        // The table without every row's balloon is withdrawn whole.
        table = None;
        replaced.clear();
        attempt = plan.dimensions.clone();
        let short: Vec<&str> = specs
            .iter()
            .zip(&landed)
            .filter(|(s, b)| s.certifying && b.is_none())
            .map(|(s, _)| s.tag.as_str())
            .collect();
        table_failure = Some(format!(
            "the balloons of {} of its rows ({}) have no place beside the plan view whose ring clears the other rings and whose leader crosses no label, other view, title block, table or other leader",
            short.len(),
            short.join(", ")
        ));
        specs.retain(|s| !s.certifying);
        let fixed = fixed_obstacles(analysis, plan, &view, None);
        landed = place_balloons(&specs, &frame, plan, &attempt, &fixed, false);
    }

    // Commit.
    if let Some(why) = table_failure {
        plan.problems.push(Problem {
            id: format!("p:table_dropped:{ID}"),
            code: "table_dropped".into(),
            severity: "warning".into(),
            message: format!(
                "The hole table ({} rows) that would replace the dropped hole items is not drawn: {why}. The drops stay reported.",
                loose.len()
            ),
            // The drops it would have stated; the table itself is not in the plan.
            subjects: escalating
                .iter()
                .map(|&i| plan.dimensions[i].id.clone())
                .collect(),
            refs: vec![ID.into()],
            ..Default::default()
        });
    }
    let unplaced: Vec<&Wanted> = specs
        .iter()
        .zip(&landed)
        .filter(|(_, b)| b.is_none())
        .map(|(s, _)| s)
        .collect();
    if !unplaced.is_empty() {
        let names: Vec<String> = unplaced.iter().map(|s| s.tag.clone()).collect();
        plan.problems.push(Problem {
            id: "p:balloon_dropped:v:plan".into(),
            code: "balloon_dropped".into(),
            severity: "warning".into(),
            message: format!(
                "{} balloon{} ({}) could not be placed beside the plan view clear of its labels, the other views and the title block.",
                names.len(),
                if names.len() == 1 { "" } else { "s" },
                names.join(", ")
            ),
            subjects: unplaced.iter().flat_map(|s| s.features.clone()).collect(),
            ..Default::default()
        });
    }
    plan.balloons = landed.into_iter().flatten().collect();
    if let Some(t) = table {
        let resolved: BTreeSet<&str> = replaced
            .iter()
            .map(|&i| plan.dimensions[i].id.as_str())
            .collect::<BTreeSet<_>>();
        let resolved: BTreeSet<String> = resolved.into_iter().map(String::from).collect();
        for &i in &replaced {
            let d = &mut attempt[i];
            d.basis += &format!(
                "; stated instead by the hole table's rows ({ID}), which replaces the plan view's plain callouts and hole locations"
            );
        }
        plan.dimensions = attempt;
        let cleared: Vec<String> = plan
            .problems
            .iter()
            .filter(|p| {
                matches!(p.code.as_str(), "callout_dropped" | "location_ref_dropped")
                    && p.subjects.first().is_some_and(|s| resolved.contains(s))
            })
            .map(|p| p.subjects[0].clone())
            .collect();
        plan.problems.retain(|p| {
            !(matches!(p.code.as_str(), "callout_dropped" | "location_ref_dropped")
                && p.subjects.first().is_some_and(|s| resolved.contains(s)))
        });
        plan.problems.push(Problem {
            id: format!("p:hole_table_escalated:{ID}"),
            code: "hole_table_escalated".into(),
            severity: "info".into(),
            message: format!(
                "Hole items had no place on the sheet, so the plan view's {} holes outside a pattern are tabulated ({ID}) and ballooned; the table replaces {} callouts and locations{}.",
                t.rows.len(),
                resolved.len(),
                if cleared.is_empty() {
                    String::new()
                } else {
                    format!(" and states the dropped {}", cleared.join(", "))
                }
            ),
            subjects: std::iter::once(ID.to_string())
                .chain(resolved.iter().cloned())
                .collect(),
            ..Default::default()
        });
        plan.hole_table = Some(t);
    }
    let marked: Vec<(usize, String)> = plan
        .balloons
        .iter()
        .filter(|b| !b.certifying)
        .filter_map(|b| {
            let i = patterns.iter().copied().find(|&i| {
                bores(&plan.dimensions[i]).first().copied()
                    == b.features.first().map(String::as_str)
            })?;
            Some((i, b.tag.clone()))
        })
        .collect();
    for (i, tag) in marked {
        let why = format!(
            " Balloon {tag} marks the pattern beside the plan view; it does not state the pattern's sizes, so the callout stays missing."
        );
        say(plan, i, &why);
    }
}

/// The plan view's holes outside any recognised pattern, each member of each group's callout in
/// the plan's order, with its callout's index.
fn loose(plan: &Plan) -> Vec<(usize, &str)> {
    plan.dimensions
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "callout" && d.view == "v:plan" && d.id.starts_with("d:f:hole:"))
        .flat_map(|(i, d)| bores(d).into_iter().map(move |f| (i, f)))
        .collect()
}

/// The rows a hole table of *plan*'s plan view has, whether or not its escalation runs: one per
/// hole outside a pattern, tagged in order.
pub fn table_rows(analysis: &Analysis, plan: &Plan) -> Vec<HoleRow> {
    let loose = loose(plan);
    rows(analysis, plan, &loose, &tags(loose.len())).0
}

/// Adds *why* to the drop problem of dimension *i*.
fn say(plan: &mut Plan, i: usize, why: &str) {
    let id = plan.dimensions[i].id.clone();
    if let Some(p) = plan
        .problems
        .iter_mut()
        .find(|p| DROPS.contains(&p.code.as_str()) && p.subjects.first() == Some(&id))
    {
        p.message += why;
    }
}

/// The table's rows for *loose* (each a callout's index and a hole), tagged *tags*, and the
/// dimensions the table replaces.
fn rows(
    analysis: &Analysis,
    plan: &Plan,
    loose: &[(usize, &str)],
    tags: &[String],
) -> (Vec<HoleRow>, Vec<usize>) {
    let read: BTreeMap<&str, holes::Hole> = holes::read(analysis)
        .into_iter()
        .filter_map(|(f, h)| Some((f.id.as_str(), h.ok()?)))
        .collect();
    let plain = |f: &str| {
        read.get(f).is_some_and(|h| {
            h.counterbore.is_none() && h.spotface.is_none() && h.countersink.is_none()
        })
    };
    let tabulated: BTreeSet<&str> = loose.iter().map(|(_, f)| *f).collect();
    let locations: Vec<(usize, &PlannedDimension)> = plan
        .dimensions
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            d.kind == "linear"
                && d.view == "v:plan"
                && !d.measurements.is_empty()
                && d.measurements.iter().all(|m| {
                    matches!(
                        m.parameter.as_str(),
                        "location.length.x" | "location.length.y"
                    )
                })
        })
        .collect();
    let location = |f: &str, axis: &str| {
        let parameter = format!("location.length.{axis}");
        locations.iter().find_map(|(_, d)| {
            d.measurements
                .iter()
                .find(|m| m.feature == f && m.parameter == parameter)
                .map(|m| (d.label.clone(), m.clone()))
        })
    };
    let mut out = Vec::new();
    for ((i, f), tag) in loose.iter().zip(tags) {
        let d = &plan.dimensions[*i];
        let at = d.terms.iter().position(|t| t.starts_with('⌀'));
        let tolerance = !d.from_spec.is_empty();
        let diameter = at.map_or(String::new(), |k| {
            let mut s = format!("ø{}", &d.terms[k]['⌀'.len_utf8()..]);
            if tolerance
                && let Some(t) = d
                    .terms
                    .get(k + 1)
                    .filter(|t| !matches!(t.as_str(), "THRU" | "↧"))
            {
                s += &format!(" {t}");
            }
            s
        });
        let depth = if d.terms.iter().any(|t| t == "THRU") {
            "THRU".to_string()
        } else {
            d.terms
                .iter()
                .position(|t| t == "↧")
                .and_then(|k| d.terms.get(k + 1))
                .cloned()
                .unwrap_or_default()
        };
        let mut measurements: Vec<Measurement> = d
            .measurements
            .iter()
            .filter(|m| {
                m.feature == *f && matches!(m.parameter.as_str(), "bore.diameter" | "bore.depth")
            })
            .cloned()
            .collect();
        let mut cells = vec![tag.clone(), diameter, depth];
        for axis in ["x", "y"] {
            match location(f, axis) {
                Some((label, m)) => {
                    cells.push(label);
                    measurements.push(m);
                }
                None => cells.push(String::new()),
            }
        }
        out.push(HoleRow {
            tag: tag.clone(),
            feature: (*f).into(),
            cells,
            measurements,
        });
    }
    let mut replace: Vec<usize> = Vec::new();
    for (i, d) in plan.dimensions.iter().enumerate() {
        if d.kind != "callout" || d.view != "v:plan" || !d.id.starts_with("d:f:hole:") {
            continue;
        }
        let members = bores(d);
        if !members.is_empty()
            && members.iter().all(|f| tabulated.contains(f) && plain(f))
            && d.from_spec.is_empty()
            && d.measurements
                .iter()
                .all(|m| matches!(m.parameter.as_str(), "bore.diameter" | "bore.depth"))
        {
            replace.push(i);
        }
    }
    for (i, d) in &locations {
        if d.measurements
            .iter()
            .all(|m| tabulated.contains(m.feature.as_str()))
        {
            replace.push(*i);
        }
    }
    replace.sort();
    (out, replace)
}

/// The obstacles every block keeps clear of: the views' areas, *dims*' ink and the PMI's, the
/// title block, the notes block and the spec's tables.
fn table_obstacles(analysis: &Analysis, plan: &Plan, dims: &[PlannedDimension]) -> Vec<Rect> {
    let mut ink: Vec<PlannedDimension> = dims.to_vec();
    ink.extend(plan.gdt.iter().filter_map(crate::dims::gdt::proxy));
    let mut out = tables::obstacles(analysis, &plan.sheet, &plan.views, &ink);
    out.extend(plan.notes_block.as_ref().map(|b| b.cells));
    out.extend(
        plan.tables
            .iter()
            .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
    );
    out
}

/// The table's blocks, placed towards the sheet's top right, in the fewest blocks (up to four)
/// that fit; none when no wrapping fits.
fn place_table(
    analysis: &Analysis,
    plan: &Plan,
    dims: &[PlannedDimension],
    rows: &[HoleRow],
) -> Option<Vec<PlannedTable>> {
    let obstacles = table_obstacles(analysis, plan, dims);
    let header: Vec<String> = HEADER.iter().map(|s| s.to_string()).collect();
    for ncols in 1..=4usize {
        let per = rows.len().div_ceil(ncols);
        let blocks: Vec<Vec<Vec<String>>> = (0..ncols)
            .map(|c| {
                let mut b = vec![header.clone()];
                for r in 0..per {
                    b.push(
                        rows.get(c * per + r)
                            .map_or_else(|| vec![String::new(); HEADER.len()], |r| r.cells.clone()),
                    );
                }
                b
            })
            .collect();
        let widths: Vec<Vec<f64>> = blocks.iter().map(|b| tables::widths(b)).collect();
        let w = widths.iter().flatten().sum::<f64>() + BLOCK_GAP * (ncols - 1) as f64;
        let h = ROW * (per + 1) as f64;
        let Some([x0, y0]) = tables::place([w, h], &plan.sheet, &obstacles, Corner::TopRight)
        else {
            continue;
        };
        let mut x = x0;
        let mut out = Vec::new();
        for (k, (b, ws)) in blocks.iter().zip(&widths).enumerate() {
            let placed = tables::lay(b, ws, [x, y0]);
            x = placed.cells[2] + BLOCK_GAP;
            out.push(PlannedTable {
                id: if ncols == 1 {
                    ID.into()
                } else {
                    format!("{ID}/{}", k + 1)
                },
                rows: b.clone(),
                default: true,
                basis: format!("block {} of {ncols} of the hole table", k + 1),
                state: crate::items::State::Shown,
                placed: Some(placed),
            });
        }
        return Some(out);
    }
    None
}

/// What a balloon's ring and leader keep clear of besides the labels: the other views' areas,
/// the title block, the notes block, the spec's tables and the hole table.
fn fixed_obstacles(
    analysis: &Analysis,
    plan: &Plan,
    own: &PlannedView,
    table: Option<&HoleTable>,
) -> Vec<Rect> {
    let mut out: Vec<Rect> = plan
        .views
        .iter()
        .filter(|v| v.id != own.id)
        .map(|v| {
            if v.id == "v:iso" {
                let ib = analysis.iso_bounds;
                [
                    v.at[0] + v.scale * ib[0],
                    v.at[1] + v.scale * ib[1],
                    v.at[0] + v.scale * ib[2],
                    v.at[1] + v.scale * ib[3],
                ]
            } else {
                v.area
            }
        })
        .collect();
    out.push(plan.sheet.title_block);
    out.extend(plan.notes_block.as_ref().map(|b| b.cells));
    out.extend(
        plan.tables
            .iter()
            .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
    );
    if let Some(t) = table {
        out.extend(
            t.blocks
                .iter()
                .filter_map(|b| b.placed.as_ref().map(|p| p.cells)),
        );
    }
    out
}

/// A line from one point to another, mm on the page.
type Segment = [[f64; 2]; 2];

/// A balloon wanted: its tag, what it marks, its hole on the page and the hole's radius there.
struct Wanted {
    tag: String,
    features: Vec<String>,
    hole: [f64; 2],
    radius: f64,
    certifying: bool,
}

/// One band beside the view: its free axis (0 along x, 1 along y), the fixed coordinate of its
/// line, and the free stretches of that line a ring's centre may take.
struct Band {
    axis: usize,
    line: f64,
    free: Vec<[f64; 2]>,
}

/// Whether boxes *a* and *b* overlap.
fn overlap(a: Rect, b: Rect) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// Whether segment *s* runs through box *b* (Liang–Barsky).
fn crosses(s: [[f64; 2]; 2], b: Rect) -> bool {
    let [p, q] = s;
    let d = [q[0] - p[0], q[1] - p[1]];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for k in 0..2 {
        for (num, den) in [(p[k] - b[k], -d[k]), (b[k + 2] - p[k], d[k])] {
            if den.abs() < 1e-12 {
                if num < 0.0 {
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
    }
    t0 < t1
}

/// The free stretches of [lo, hi] once each of *blocked* is taken out.
fn carve(lo: f64, hi: f64, mut blocked: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    blocked.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut out = Vec::new();
    let mut at = lo;
    for [a, b] in blocked {
        if a > at {
            out.push([at, a.min(hi)]);
        }
        at = at.max(b);
        if at >= hi {
            break;
        }
    }
    if at < hi {
        out.push([at, hi]);
    }
    out.retain(|s| s[1] >= s[0]);
    out
}

/// Positions near *naturals* (ascending), at least *gap* apart, within [lo, hi], nearest in the
/// least-squares sense (isotonic regression, clipped to the bounds).
fn spread(naturals: &[f64], gap: f64, lo: f64, hi: f64) -> Vec<f64> {
    let n = naturals.len();
    // Pool adjacent violators over the naturals less their rank's gaps: blocks of (mean, count).
    let mut blocks: Vec<(f64, usize)> = Vec::new();
    for (i, v) in naturals.iter().enumerate() {
        blocks.push((v - i as f64 * gap, 1));
        while blocks.len() > 1 && blocks[blocks.len() - 2].0 > blocks[blocks.len() - 1].0 {
            let (m2, c2) = blocks.pop().expect("a block");
            let (m1, c1) = blocks.pop().expect("a block");
            let c = c1 + c2;
            blocks.push(((m1 * c1 as f64 + m2 * c2 as f64) / c as f64, c));
        }
    }
    let top = (hi - n.saturating_sub(1) as f64 * gap).max(lo);
    blocks
        .iter()
        .flat_map(|&(m, c)| std::iter::repeat_n(m.clamp(lo, top), c))
        .enumerate()
        .map(|(i, y)| y + i as f64 * gap)
        .collect()
}

/// Each wanted balloon placed beside the plan view, or none: draftwright's bands left, right,
/// above and below the view, each just beyond the deepest annotation placed there, the members
/// spread along them at the strip spacing and kept off what is placed (the module's doc).
fn place_balloons(
    wanted: &[Wanted],
    frame: &Frame,
    plan: &Plan,
    dims: &[PlannedDimension],
    fixed: &[Rect],
    perimeter: bool,
) -> Vec<Option<Balloon>> {
    if wanted.is_empty() {
        return Vec::new();
    }
    let r = RADIUS;
    let [pw, ph] = plan.sheet.page;
    let m = crate::layout::MARGIN;
    let drawable = [m, m, pw - m, ph - m];
    // The halo draftwright reserves for a table's ring, else the nearest standoff.
    let extent = if perimeter {
        STRIP_GAP + 2.0 * r + STRIP_SPACING
    } else {
        STRIP_GAP + 2.0 * r
    };
    let offset = extent - r;
    let mut ink: Vec<PlannedDimension> = dims.to_vec();
    ink.extend(plan.gdt.iter().filter_map(crate::dims::gdt::proxy));
    let mut labels: Vec<Rect> = Vec::new();
    let mut occupants: Vec<Rect> = Vec::new();
    for p in ink.iter().filter_map(|d| d.placed.as_ref()) {
        labels.push(p.label_box);
        occupants.push(p.label_box);
        for l in p.lines.iter().chain(&p.arrows) {
            occupants.push([
                l[0][0].min(l[1][0]),
                l[0][1].min(l[1][1]),
                l[0][0].max(l[1][0]),
                l[0][1].max(l[1][1]),
            ]);
        }
    }
    let [pl, pb, pr, pt] = frame.outline;
    // How deep the placed annotations in the view's reserved area reach into each band
    // (draftwright measures each band up to its strip's limit).
    let area = plan
        .views
        .iter()
        .find(|v| v.id == "v:plan")
        .map_or(drawable, |v| v.area);
    let (mut left, mut right, mut top, mut bottom) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for b in occupants.iter().filter(|b| overlap(**b, area)) {
        if b[2] > pl && b[0] < pr {
            if b[3] > pt {
                top = top.max(b[3] - pt);
            }
            if b[1] < pb {
                bottom = bottom.max(pb - b[1]);
            }
        }
        if b[3] > pb && b[1] < pt {
            if b[0] < pl {
                left = left.max(pl - b[0]);
            }
            if b[2] > pr {
                right = right.max(b[2] - pr);
            }
        }
    }
    let glyph_blockers: Vec<Rect> = occupants
        .iter()
        .chain(fixed)
        .copied()
        .chain(std::iter::once(frame.outline))
        .collect();
    let pad = r + STRIP_SPACING;
    let band = |axis: usize, line: f64| -> Band {
        let cross = 1 - axis;
        let fits = line - r >= drawable[cross] && line + r <= drawable[cross + 2];
        // Along the view's reserved area, so a ring stays beside its own view.
        let (lo, hi) = (
            drawable[axis].max(area[axis]) + r,
            drawable[axis + 2].min(area[axis + 2]) - r,
        );
        let blocked = glyph_blockers
            .iter()
            .filter(|b| b[cross] < line + pad && b[cross + 2] > line - pad)
            .map(|b| [b[axis] - pad, b[axis + 2] + pad])
            .collect();
        Band {
            axis,
            line,
            free: if fits {
                carve(lo, hi, blocked)
            } else {
                Vec::new()
            },
        }
    };
    let bands = [
        band(1, pl - left - offset),
        band(1, pr + right + offset),
        band(0, pt + top + offset),
        band(0, pb - bottom - offset),
    ];
    let font = crate::dims::font();
    let gap = wanted
        .iter()
        .map(|w| {
            let laid = font.baseline(&w.tag, crate::dims::TEXT);
            (laid.max[0] - laid.min[0]).max(2.0 * r)
        })
        .fold(2.0 * r, f64::max)
        + 2.0 * STRIP_SPACING;
    // A balloon's place on band *b* at *along*: its centre, and its leader from the hole's rim.
    let at_on = |w: &Wanted, b: &Band, along: f64| {
        let at = if b.axis == 1 {
            [b.line, along]
        } else {
            [along, b.line]
        };
        let d = [at[0] - w.hole[0], at[1] - w.hole[1]];
        let dist = d[0].hypot(d[1]);
        let leader = (dist > w.radius + r).then(|| {
            let u = [d[0] / dist, d[1] / dist];
            [
                [w.hole[0] + u[0] * w.radius, w.hole[1] + u[1] * w.radius],
                [at[0] - u[0] * r, at[1] - u[1] * r],
            ]
        });
        (at, dist, leader)
    };
    let mut rings: Vec<Rect> = Vec::new();
    let mut leaders: Vec<[[f64; 2]; 2]> = Vec::new();
    // Whether a ring at *at* with *leader* clears the labels, the fixed obstacles and the balloons
    // placed so far.
    let clear =
        |at: [f64; 2], leader: Option<[[f64; 2]; 2]>, rings: &[Rect], leaders: &[[[f64; 2]; 2]]| {
            let ring = [at[0] - r, at[1] - r, at[0] + r, at[1] + r];
            let spaced = |q: &Rect| {
                overlap(
                    [
                        q[0] - STRIP_SPACING,
                        q[1] - STRIP_SPACING,
                        q[2] + STRIP_SPACING,
                        q[3] + STRIP_SPACING,
                    ],
                    ring,
                )
            };
            !rings.iter().any(spaced)
                && leader.is_none_or(|l| {
                    !labels
                        .iter()
                        .chain(fixed)
                        .chain(rings)
                        .any(|q| crosses(l, *q))
                        && !leaders.iter().any(|m| segments_cross(l, *m))
                })
        };
    // First, draftwright's strips: each member goes out of the side of the view nearest it that
    // has a band, the members of a band in order along it, spread at the strip spacing near their
    // holes, so their leaders keep their order.
    let usable: Vec<usize> = (0..bands.len())
        .filter(|&k| !bands[k].free.is_empty())
        .collect();
    let side = |w: &Wanted| {
        let edge = [
            w.hole[0] - pl,
            pr - w.hole[0],
            pt - w.hole[1],
            w.hole[1] - pb,
        ];
        usable
            .iter()
            .copied()
            .min_by(|&a, &b| edge[a].total_cmp(&edge[b]).then(a.cmp(&b)))
    };
    let mut planned: Vec<Option<(usize, f64)>> = vec![None; wanted.len()];
    for (k, b) in bands.iter().enumerate() {
        let mut members: Vec<usize> = (0..wanted.len())
            .filter(|&i| side(&wanted[i]) == Some(k))
            .collect();
        let axis = b.axis;
        members.sort_by(|&x, &y| {
            wanted[x].hole[axis]
                .total_cmp(&wanted[y].hole[axis])
                .then(x.cmp(&y))
        });
        let mut by_stretch: Vec<Vec<usize>> = vec![Vec::new(); b.free.len()];
        let (mut s, mut prev) = (0usize, f64::NEG_INFINITY);
        for &i in &members {
            let natural = wanted[i].hole[axis];
            while s < b.free.len() {
                let at = natural.max(prev + gap).max(b.free[s][0]);
                if at <= b.free[s][1] + 1e-9 {
                    by_stretch[s].push(i);
                    prev = at;
                    break;
                }
                s += 1;
                prev = f64::NEG_INFINITY;
            }
        }
        for (stretch, ids) in b.free.iter().zip(&by_stretch) {
            let naturals: Vec<f64> = ids.iter().map(|&i| wanted[i].hole[axis]).collect();
            for (&i, c) in ids
                .iter()
                .zip(spread(&naturals, gap, stretch[0], stretch[1]))
            {
                planned[i] = Some((k, c));
            }
        }
    }
    let mut out: Vec<Option<Balloon>> = vec![None; wanted.len()];
    let mut place = |i: usize,
                     at: [f64; 2],
                     leader: Option<[[f64; 2]; 2]>,
                     rings: &mut Vec<Rect>,
                     leaders: &mut Vec<[[f64; 2]; 2]>| {
        let w = &wanted[i];
        rings.push([at[0] - r, at[1] - r, at[0] + r, at[1] + r]);
        leaders.extend(leader);
        out[i] = Some(Balloon {
            id: format!("balloon_plan_{}_0", w.tag),
            tag: w.tag.clone(),
            view: "v:plan".into(),
            features: w.features.clone(),
            certifying: w.certifying,
            at,
            radius: r,
            leader,
            state: crate::items::State::Shown,
        });
    };
    let mut left_over = Vec::new();
    for (i, p) in planned.iter().enumerate() {
        let fits = p.and_then(|(k, along)| {
            let (at, _, leader) = at_on(&wanted[i], &bands[k], along);
            clear(at, leader, &rings, &leaders).then_some((at, leader))
        });
        match fits {
            Some((at, leader)) => place(i, at, leader, &mut rings, &mut leaders),
            None => left_over.push(i),
        }
    }
    // Then each member left over, nearest a band first, to the clear place on any band nearest its
    // hole: tried at the hole's own coordinate along the band, then half a strip step at a time
    // either way.
    let reach = |w: &Wanted, b: &Band| (w.hole[1 - b.axis] - b.line).abs();
    let nearest = |w: &Wanted| {
        usable
            .iter()
            .map(|&k| reach(w, &bands[k]))
            .fold(f64::INFINITY, f64::min)
    };
    left_over.sort_by(|&a, &b| {
        nearest(&wanted[a])
            .total_cmp(&nearest(&wanted[b]))
            .then(a.cmp(&b))
    });
    let step = 0.5 * gap;
    for i in left_over {
        let w = &wanted[i];
        let mut best: Option<(f64, [f64; 2], Option<Segment>)> = None;
        for &k in &usable {
            let b = &bands[k];
            for s in &b.free {
                for j in 0..=2 * STEPS {
                    let sign = if j % 2 == 1 { 1.0 } else { -1.0 };
                    let along =
                        w.hole[b.axis].clamp(s[0], s[1]) + sign * j.div_ceil(2) as f64 * step;
                    if along < s[0] - 1e-9 || along > s[1] + 1e-9 {
                        continue;
                    }
                    let (at, dist, leader) = at_on(w, b, along);
                    if best.as_ref().is_some_and(|(c, ..)| dist >= *c) {
                        continue;
                    }
                    if clear(at, leader, &rings, &leaders) {
                        best = Some((dist, at, leader));
                    }
                }
            }
        }
        if let Some((_, at, leader)) = best {
            place(i, at, leader, &mut rings, &mut leaders);
        }
    }
    out
}

/// Whether segments *a* and *b* cross.
fn segments_cross(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    let side = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let (d1, d2) = (side(b[0], b[1], a[0]), side(b[0], b[1], a[1]));
    let (d3, d4) = (side(a[0], a[1], b[0]), side(a[0], a[1], b[1]));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_run_as_draftwrights() {
        let t = tags(28);
        assert_eq!(&t[..3], ["A", "B", "C"]);
        assert_eq!(t[25], "Z");
        assert_eq!(&t[26..], ["AA", "AB"]);
        assert_eq!(tags(26 + 26 * 26 + 1).last().unwrap(), "AAA");
    }

    #[test]
    fn spread_keeps_the_gap_and_the_bounds() {
        let s = spread(&[5.0, 5.0, 6.0], 4.0, 0.0, 100.0);
        assert!((s[1] - s[0] - 4.0).abs() < 1e-9 && (s[2] - s[1] - 4.0).abs() < 1e-9);
        assert!((s[1] - 5.333_333_333).abs() < 1e-6);
        assert_eq!(spread(&[0.0, 1.0], 4.0, 10.0, 100.0), [10.0, 14.0]);
        assert_eq!(spread(&[99.0, 100.0], 4.0, 0.0, 100.0), [96.0, 100.0]);
    }

    #[test]
    fn carving_leaves_the_free_stretches() {
        assert_eq!(
            carve(0.0, 10.0, vec![[2.0, 3.0], [-1.0, 1.0], [2.5, 4.0]]),
            [[1.0, 2.0], [4.0, 10.0]]
        );
        assert_eq!(carve(0.0, 10.0, vec![[-1.0, 11.0]]), Vec::<[f64; 2]>::new());
    }

    #[test]
    fn a_segment_crosses_a_box_it_passes_through() {
        let b = [0.0, 0.0, 1.0, 1.0];
        assert!(crosses([[-1.0, 0.5], [2.0, 0.5]], b));
        assert!(!crosses([[-1.0, 2.0], [2.0, 2.0]], b));
        assert!(crosses([[0.5, 0.5], [0.6, 0.6]], b));
        assert!(!crosses([[-1.0, -1.0], [-0.5, 3.0]], b));
    }
}
