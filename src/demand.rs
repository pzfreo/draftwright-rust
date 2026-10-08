//! What the sheet must make room for, read from the Analysis before the views are placed:
//! draftwright's sheet-demand planning (`analysis._plan_sheet_demand`,
//! `compose._compose_anno_boxes`, `builder._automatic_turned_principals` and
//! `planner.plan_sections`) as far as the bridged features carry it.
//!
//! - **Turned parts** keep their profile and end views and drop the view that repeats the profile
//!   (draftwright's `redundant_radial_view_removed`), unless a feature needs that view end-on.
//! - **Step levels**: a turned part's shoulders along a vertical axis, or the part's horizontal
//!   face levels; they size the height ladder right of the front view.
//! - **Annotation bands** ([`Strips`]): the height ladder, the X-location tiers above the plan,
//!   the balloon ring, and the rows of Y-hole leaders below the front.
//! - **Section A–A** through the densest row of Z-axis holes whose profile no standard view shows.
//!
//! Not ported here: the hole-callout band (it is measured from the callout labels, which come with
//! the hole callouts), authored dimensions, pads, seat offsets, through-step legs and angular
//! reservations, the IR's ownership rules (a boss absorbed by a groove still counts here), the
//! rotational-OD fallback for a turning axis, and sections on internal-detail rows.

use serde_json::Value;

use crate::bridge::holes;
use crate::layout::{self, DIM_PAD, Pick, Request, Scaled, Strips};
use crate::model::Analysis;

/// A bridged hole in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Hole {
    /// Its location as quiddity reports it, in the part's frame.
    pub at: [f64; 3],
    /// Its axis, when it lies along x, y or z (0, 1, 2).
    pub axis: Option<usize>,
    pub diameter: f64,
    pub through: bool,
    pub cbore: bool,
    pub spotface: bool,
    /// Its machining requirement ([`holes::Hole::requirement`]: signed direction, sizes,
    /// bottom, counterbore, spotface, countersink): holes with the same requirement are one
    /// feature, called out once.
    pub spec: Vec<i64>,
    /// The recognised pattern it belongs to (`Analysis.recognition.patterns`), if any.
    pub pattern: Option<String>,
}

/// Where section A–A cuts, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// The plane y = `cut_y` (part frame), seen from the front.
    pub cut_y: f64,
    /// The holes on that row whose profile it shows.
    pub holes: usize,
}

/// What the sheet must make room for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Demand {
    /// The axis (0, 1, 2) of the one turned profile, if the part has exactly one.
    pub turning_axis: Option<usize>,
    /// The number of turned profiles.
    pub profiles: usize,
    /// The step levels (z, part frame) the height ladder may dimension.
    pub step_zs: Vec<f64>,
    pub holes: Vec<Hole>,
    /// Bosses along z with a height: each takes a slot on the height ladder.
    pub z_boss_heights: usize,
    /// A boss or turned step along y: its radial fan needs room beside the height slot.
    pub y_radial: bool,
    pub section: Option<Section>,
}

/// The part frame's point for quiddity's local *l*.
fn to_part(frame: &[[f64; 3]; 4], l: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|k| frame[0][k] + (0..3).map(|j| l[j] * frame[j + 1][k]).sum::<f64>())
}

/// The part frame's direction for quiddity's local *d*.
fn dir_to_part(frame: &[[f64; 3]; 4], d: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|k| (0..3).map(|j| d[j] * frame[j + 1][k]).sum::<f64>())
}

fn vec3(v: &Value) -> Option<[f64; 3]> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

/// A local axis given as a letter or a vector.
fn local_axis(v: &Value) -> Option<[f64; 3]> {
    match v.as_str() {
        Some("x") => Some([1.0, 0.0, 0.0]),
        Some("y") => Some([0.0, 1.0, 0.0]),
        Some("z") => Some([0.0, 0.0, 1.0]),
        Some(_) => None,
        None => vec3(v),
    }
}

/// The end-exclusion of a span (quiddity's `bounded_end_margin`, draftwright's 0.6 mm).
const END_MARGIN: f64 = 0.6;

/// Reads what the sheet must make room for from *analysis*.
pub fn read(analysis: &Analysis) -> Demand {
    let mut d = Demand::default();
    let [lo, hi] = analysis.envelope;
    let frame = analysis.recognition.as_ref().map(|r| r.frame);

    // Turned profiles: their steps' shoulders, and the profile's axis in the part's frame.
    let mut profiles: Vec<(String, Option<usize>, Vec<f64>)> = Vec::new();
    for f in analysis.features.iter().filter(|f| f.kind == "turned_step") {
        let (Some(frame), Some(axis), Some(lo_t), Some(hi_t)) = (
            frame,
            f.parameters.get("axis").and_then(local_axis),
            f.parameters.get("lo").and_then(Value::as_f64),
            f.parameters.get("hi").and_then(Value::as_f64),
        ) else {
            continue;
        };
        let profile = f.parameters.get("profile");
        let key = profile
            .and_then(|p| p.get("body_key"))
            .map_or_else(String::new, Value::to_string);
        let origin = profile
            .and_then(|p| p.get("axis_origin"))
            .and_then(vec3)
            .unwrap_or([0.0; 3]);
        let k = (0..3).find(|&k| axis[k] != 0.0).unwrap_or(0);
        let world = |t: f64| {
            let mut l = origin;
            l[k] = t;
            to_part(&frame, l)
        };
        let axis = holes::principal(dir_to_part(&frame, axis));
        let zs = [world(lo_t)[2], world(hi_t)[2]];
        match profiles.iter_mut().find(|p| p.0 == key) {
            Some(p) => p.2.extend(zs),
            None => profiles.push((key, axis, zs.to_vec())),
        }
    }
    d.profiles = profiles.len();
    if profiles.len() == 1 {
        d.turning_axis = profiles[0].1;
    }
    let inside = |z: f64| lo[2] + END_MARGIN < z && z < hi[2] - END_MARGIN;
    d.step_zs = if !profiles.is_empty() && profiles.iter().all(|p| p.1 == Some(2)) {
        profiles.iter().flat_map(|p| p.2.iter().copied()).collect()
    } else {
        analysis.levels.clone()
    };
    d.step_zs.retain(|z| inside(*z));
    d.step_zs.sort_by(f64::total_cmp);
    d.step_zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    // The holes that can be read (the dimensions report the others).
    let pattern_of = |id: &str| {
        let patterns = analysis.recognition.as_ref().map(|r| r.patterns.as_slice());
        patterns
            .unwrap_or_default()
            .iter()
            .find(|p| p.members.iter().any(|m| m == id))
            .map(|p| p.id.clone())
    };
    d.holes = holes::read(analysis)
        .iter()
        .filter_map(|(f, h)| h.as_ref().ok().map(|h| hole(h, pattern_of(&f.id))))
        .collect();
    // Polygonal stock's length takes the overall height's slot; other axial heights add one.
    let mut stock = false;
    for f in &analysis.features {
        match f.kind.as_str() {
            "boss" | "polygonal_boss" | "polygonal_stock" => {
                let axis = f.fingerprint.axis.and_then(holes::principal);
                let tall = ["height", "length", "axial_length"]
                    .iter()
                    .any(|k| f.fingerprint.sizes.contains_key(*k));
                if axis == Some(2) && tall {
                    d.z_boss_heights += 1;
                    stock |= f.kind == "polygonal_stock";
                }
                if axis == Some(1) && f.kind == "boss" {
                    d.y_radial = true;
                }
            }
            "turned_step" if f.fingerprint.axis.and_then(holes::principal) == Some(1) => {
                d.y_radial = true;
            }
            _ => {}
        }
    }
    d.z_boss_heights -= usize::from(stock);
    // A turned part's concentric through bores are dimensioned by its centre-line, so they drive
    // no section (draftwright's `feature_hole_keys`).
    let centre = [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])];
    let keyed: Vec<Hole> = d
        .holes
        .iter()
        .filter(|h| {
            !(d.turning_axis == Some(2)
                && h.axis == Some(2)
                && h.through
                && (h.at[0] - centre[0]).hypot(h.at[1] - centre[1]) <= 0.5)
        })
        .cloned()
        .collect();
    d.section = section(&keyed, centre[1]);
    d
}

fn hole(h: &holes::Hole, pattern: Option<String>) -> Hole {
    Hole {
        at: h.at,
        axis: h.principal.map(|(k, _)| k),
        diameter: h.diameter,
        through: h.through,
        cbore: h.counterbore.is_some(),
        spotface: h.spotface.is_some(),
        spec: h.requirement(),
        pattern,
    }
}

/// Draftwright's section rule (`plan_sections`): a Z-axis hole with a counterbore, a spotface or
/// a bottom has a profile no standard view shows; the plane cuts the densest row of them
/// (within 0.5 mm), the tie going to the row nearest the part's centre.
fn section(holes: &[Hole], centre_y: f64) -> Option<Section> {
    let ys: Vec<f64> = holes
        .iter()
        .filter(|h| h.axis == Some(2) && (h.cbore || h.spotface || !h.through))
        .map(|h| h.at[1])
        .collect();
    let mut rows: Vec<f64> = ys.iter().map(|y| (y * 10.0).round() / 10.0).collect();
    rows.sort_by(f64::total_cmp);
    rows.dedup();
    let count = |v: f64| ys.iter().filter(|y| (*y - v).abs() <= 0.5).count();
    rows.into_iter()
        .map(|v| (count(v), v))
        .max_by(|a, b| {
            a.0.cmp(&b.0)
                .then((b.1 - centre_y).abs().total_cmp(&(a.1 - centre_y).abs()))
                .then(b.1.total_cmp(&a.1))
        })
        .map(|(holes, cut_y)| Section { cut_y, holes })
}

/// The principal views a turned part keeps (draftwright's `_automatic_turned_principals`): the
/// profile and the end view for its axis. None when the part is not turned about one axis, or
/// when a hole needs the dropped view end-on (it would have nowhere to be called out: the
/// veto draftwright reaches by planning the reduced set). Returns the kept views, the dropped one
/// and the reason.
pub fn turned_views(d: &Demand) -> Option<([&'static str; 2], &'static str, String)> {
    let axis = d.turning_axis?;
    let (kept, dropped, end_on) = match axis {
        0 | 1 => (["front", "side"], "plan", 2),
        _ => (["front", "plan"], "side", 0),
    };
    let letter = ["x", "y", "z"][axis];
    if d.holes.iter().any(|h| h.axis == Some(end_on)) {
        return None;
    }
    Some((
        kept,
        dropped,
        format!(
            "the part is turned about {letter} (one profile), so it keeps its profile and end views and leaves out the {dropped} view, which would repeat the profile (no feature needs it end-on)"
        ),
    ))
}

/// The holes as draftwright's hole features: a recognised pattern is one feature, and loose
/// holes of one machining requirement another (draftwright's machining-spec groups). Returns,
/// per feature, its axis, its members' locations and whether it is a pattern.
fn hole_features(holes: &[Hole]) -> Vec<(Option<usize>, Vec<[f64; 3]>, bool)> {
    type Key = (Option<usize>, Vec<i64>, Option<String>);
    let mut grouped: Vec<(Key, Vec<[f64; 3]>)> = Vec::new();
    for h in holes {
        let key = (h.axis, h.spec.clone(), h.pattern.clone());
        match grouped.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(h.at),
            None => grouped.push((key, vec![h.at])),
        }
    }
    grouped
        .into_iter()
        .map(|(k, m)| (k.0, m, k.2.is_some()))
        .collect()
}

/// Whether a pattern's members lie on a circle about their centroid, and are not the corners
/// of a rectangle (four holes on a rectangle are a grid to quiddity, read here the same way).
fn bolt_circle(members: &[[f64; 3]], c: [f64; 2]) -> bool {
    let r: Vec<f64> = members
        .iter()
        .map(|m| (m[0] - c[0]).hypot(m[1] - c[1]))
        .collect();
    let concyclic = members.len() >= 3 && r.iter().all(|x| (x - r[0]).abs() < 1e-3 && *x > 1e-3);
    // Four concyclic points are a rectangle's corners when the diagonals both pass the centre.
    let rectangle = members.len() == 4
        && members.iter().all(|a| {
            members.iter().any(|b| {
                ((a[0] + b[0]) * 0.5 - c[0]).abs() < 1e-3
                    && ((a[1] + b[1]) * 0.5 - c[1]).abs() < 1e-3
                    && (a[0] - b[0]).hypot(a[1] - b[1]) > 1e-3
            })
        });
    concyclic && !rectangle
}

/// The X reference a Z-axis hole feature's location dimension tiers off: a bolt circle's centre,
/// a pattern's first member (here its lowest x: quiddity's member order is not bridged), or each
/// loose hole.
fn x_refs(holes: &[Hole]) -> Vec<f64> {
    let mut xs = Vec::new();
    for (axis, members, pattern) in hole_features(holes) {
        if axis != Some(2) {
            continue;
        }
        if !pattern {
            xs.extend(members.iter().map(|m| m[0]));
            continue;
        }
        let n = members.len() as f64;
        let c = [0, 1].map(|k| members.iter().map(|m| m[k]).sum::<f64>() / n);
        let circle = bolt_circle(&members, c);
        let first = members
            .iter()
            .min_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])))
            .expect("a pattern has members");
        xs.push(if circle { c[0] } else { first[0] });
    }
    xs
}

/// The bands the features need with *n_steps* steps on the height ladder (draftwright's
/// `_compose_anno_boxes` reduced to `StripDepths`), for the principal views *views*.
pub fn strips(d: &Demand, n_steps: usize, plan_shown: bool) -> Strips {
    let mut right = layout::right_strip_depth(n_steps, d.z_boss_heights);
    if d.y_radial {
        right = right.max(layout::right_strip_depth(0, 0) + layout::RADIAL_REACH);
    }
    let mut xs = x_refs(&d.holes);
    xs.sort_by(f64::total_cmp);
    let mut distinct: Vec<f64> = Vec::new();
    for x in xs {
        if distinct.last().is_none_or(|l| (x - l).abs() > 0.5) {
            distinct.push(x);
        }
    }
    let features = hole_features(&d.holes);
    let front_rows = features.iter().filter(|f| f.0 == Some(1)).count();
    // Draftwright tabulates and balloons a plan of at least 16 holes not mostly in patterns.
    let loose: usize = features
        .iter()
        .filter(|f| f.0 == Some(2) && !f.2)
        .map(|f| f.1.len())
        .sum();
    let covered: usize = features
        .iter()
        .filter(|f| f.0 == Some(2) && f.2)
        .map(|f| f.1.len())
        .sum();
    let total = loose + covered;
    let balloon = total >= 16 && (covered as f64) < 0.8 * total as f64;
    let plan_work = d.holes.iter().any(|h| h.axis == Some(2)) || d.turning_axis != Some(2);
    Strips {
        right,
        left: DIM_PAD,
        top: layout::plan_above_depth(distinct.len()),
        pv_halo: if balloon { layout::PLAN_HALO } else { 0.0 },
        front_hole_below: layout::front_hole_rows_depth(front_rows),
        plan_demand: plan_shown && plan_work,
        ..Strips::without_features()
    }
}

/// The steps legible at *scale* above the part's base *base*.
pub fn legible(d: &Demand, base: f64, scale: f64) -> usize {
    layout::legible_steps(&d.step_zs, base, scale).len()
}

/// The part's horizontal face levels (quiddity's `step_level_records`, read in the part's frame):
/// per solid, planar faces facing ±z clustered within 0.5 mm, each cluster with at least 1 % of
/// the solid's footprint in area, strictly inside the solid's height less its end margin.
pub fn face_levels(part: &haecceity::Part) -> Vec<f64> {
    use haecceity::geom::{Bounds, Surface};
    let scopes: Vec<Vec<usize>> = if part.solids.is_empty() {
        vec![(0..part.faces.len()).collect()]
    } else {
        part.solids.iter().map(|s| s.faces.clone()).collect()
    };
    let mut out = Vec::new();
    for faces in scopes {
        let mut b = Bounds::empty();
        for &f in &faces {
            b.merge(&part.face_bounds(f));
        }
        let span = b.max[2] - b.min[2];
        let margin = END_MARGIN.min(span.max(0.0) * 0.25);
        let threshold = 0.01 * (b.max[0] - b.min[0]) * (b.max[1] - b.min[1]);
        let mut levels: Vec<(f64, f64)> = faces
            .iter()
            .filter_map(|&f| match &part.faces[f].surface {
                Surface::Plane { frame } if frame.z[2].abs() > 0.99 => {
                    Some((frame.origin[2], part.face_mass(f).map_or(0.0, |m| m[0])))
                }
                _ => None,
            })
            .collect();
        levels.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut i = 0;
        while i < levels.len() {
            let z0 = levels[i].0;
            let mut area = 0.0;
            let mut j = i;
            while j < levels.len() && levels[j].0 - z0 <= 0.5 {
                area += levels[j].1;
                j += 1;
            }
            if area >= threshold * (1.0 + 1e-9) && b.min[2] + margin < z0 && z0 < b.max[2] - margin
            {
                out.push(z0);
            }
            i = j;
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

/// Chooses the scale, sheet and arrangement for *request* with the bands *d* needs, the height
/// ladder sized for the steps legible at the scale chosen ([`layout::converge_steps`]). *base*
/// is the part's lowest z. Returns the choice, the bands, and the number of steps they hold.
pub fn size(
    d: &Demand,
    request: &Request,
    scale: Option<f64>,
    page: Option<[f64; 2]>,
    base: f64,
) -> (Pick, Strips, usize) {
    let none = |_: f64| Scaled::default();
    let measure = |n: usize| strips(d, n, request.views.plan);
    let choose = |strips: &Strips| {
        let req = Request {
            strips: strips.clone(),
            ..request.clone()
        };
        layout::choose(
            &req,
            &none,
            scale,
            page,
            &layout::AUTOMATIC_ARRANGEMENTS,
            None,
        )
    };
    let count = |s: f64| legible(d, base, s);
    layout::converge_steps(d.step_zs.len(), &measure, &choose, &count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hole(axis: usize) -> Hole {
        Hole {
            at: [0.0; 3],
            axis: Some(axis),
            diameter: 5.0,
            through: true,
            cbore: false,
            spotface: false,
            spec: Vec::new(),
            pattern: None,
        }
    }

    /// A part turned about z keeps front and plan; about x or y, front and side. A hole that
    /// needs the dropped view end-on vetoes the reduction.
    #[test]
    fn a_turned_part_drops_the_repeated_view_unless_a_feature_needs_it() {
        let mut d = Demand {
            turning_axis: Some(2),
            profiles: 1,
            ..Demand::default()
        };
        let (kept, dropped, _) = turned_views(&d).unwrap();
        assert_eq!((kept, dropped), (["front", "plan"], "side"));
        d.holes.push(hole(2));
        assert!(
            turned_views(&d).is_some(),
            "an axial hole is seen end-on in the plan"
        );
        d.holes.push(hole(0));
        assert!(
            turned_views(&d).is_none(),
            "a cross hole along x needs the side view"
        );
        d.holes.clear();
        d.turning_axis = Some(0);
        assert_eq!(turned_views(&d).unwrap().1, "plan");
        d.holes.push(hole(2));
        assert!(turned_views(&d).is_none());
        d.turning_axis = None;
        assert!(turned_views(&Demand::default()).is_none());
    }

    /// The section cuts the densest row of z-axis holes with a hidden profile; through holes
    /// with nothing hidden do not count, and a tie goes to the row nearest the centre.
    #[test]
    fn the_section_cuts_the_densest_row_of_hidden_profiles() {
        let at = |y: f64, blind: bool, cbore: bool| Hole {
            at: [0.0, y, 0.0],
            through: !blind,
            cbore,
            ..hole(2)
        };
        assert_eq!(section(&[at(5.0, false, false)], 0.0), None);
        let holes = [
            at(10.0, true, false),
            at(10.3, false, true),
            at(-4.0, true, false),
            at(30.0, false, false),
            at(30.0, false, false),
            at(30.0, false, false),
        ];
        let s = section(&holes, 0.0).unwrap();
        assert_eq!((s.cut_y, s.holes), (10.0, 2));
        let tie = [at(10.0, true, false), at(-4.0, true, false)];
        assert_eq!(section(&tie, 0.0).unwrap().cut_y, -4.0);
    }

    #[test]
    fn steps_are_legible_by_page_height_and_separation() {
        // 12.4 mm above the base on the page, then 7 mm apart.
        let zs = [5.0, 7.0, 9.0, 13.0];
        assert_eq!(layout::legible_steps(&zs, 0.0, 1.0), vec![13.0]);
        assert_eq!(layout::legible_steps(&zs, 0.0, 2.0), vec![7.0, 13.0]);
        assert_eq!(
            layout::legible_steps(&zs, 0.0, 5.0),
            vec![5.0, 7.0, 9.0, 13.0]
        );
    }
}
