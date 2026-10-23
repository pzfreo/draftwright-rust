//! Stream K4 (#43 item 2): a callout's leader tip against the physical edge it names, read
//! from the rendered drawing (the tip is the first point of the callout's leader,
//! [`Drawn::lines`]), its view's recorded transform ([`Sheet::frames`]) and the Analysis's
//! geometry of the features the callout claims, never the Plan's sites.
//!
//! - `diameter_leader_target_mismatch` (draftwright's `lint_hole_leader_targets`, ISSUE-1378): a
//!   callout claiming a hole's `bore.diameter` has its tip within [`TOLERANCE`] of a boundary
//!   circle of one of the holes it claims, as projected in its view: the bore's rim at its
//!   opening and, when its depth is known, at its far end; a counterbore's or spotface's rim
//!   at the opening and at its floor; a countersink's at the opening. A tip on another hole's
//!   rim, or in blank material, is a warning. draftwright measures the occurrence's defining
//!   edges; for a recognised round hole those are these circles (its seams lie on them when seen
//!   end-on, where the port calls a hole out). A double-D bore's boundary, seen end-on, is its
//!   trimmed profile: the major circle's two arcs between the flats, and the two flats.
//! - `diameter_leader_target_unverifiable`: a callout whose view has a recorded transform but
//!   none of whose claimed holes can be read (no axis, location or diameter; a double-D bore
//!   not seen end-on) says so.
//! - `radius_leader_target_mismatch`: a straight blend tip misses its named cylindrical
//!   trimmed profile. Exact face evidence is used where present; older straight-blend
//!   analyses use their rolling axis and quarter/half/three-quarter samples.
//! - `radius_leader_not_radial`: the drawn shaft (automatic or pinned) must meet the
//!   middle 25–75% of that profile, within 15 degrees of radial, from either side.
//! - `radius_leader_target_unverifiable`: a straight blend's target cannot be read.
//! - `step_diameter_leader_target_mismatch` (ISSUE-1374-d): a callout claiming a turned step's
//!   `step.diameter` has its tip on the band's own drawn cylinder: its outline each side of its
//!   axis and its end edges, trimmed by the flats that cut it ([`crate::lint::turned::cuts`]),
//!   or, seen along its axis, the arcs left between the flats. `…_unverifiable` (info) when the
//!   record cannot be read.
//! - `chamfer_leader_target_mismatch` (ISSUE-1374-a): a `chamfer.length` or
//!   `oriented_chamfer.length` tip lies on the bevel: a turned bevel's meridian, turned about its
//!   coaxial band onto the outline (or its ring, seen along the axis); a prismatic bevel's
//!   section seen along its edge; an oriented bevel's face. `…_unverifiable` (info) when the view
//!   cannot show it (a prismatic edge not seen end-on) or the record cannot be read.
//!   For both, a callout counting several members is a mismatch only when every member could
//!   be measured; a tip that misses the measurable ones while others cannot be verified is
//!   `…_unverifiable`, since it may sit on one of those.
//!
//! - `leader_crosses_silhouette` (draftwright's structural check, #796/#798): a callout's shaft,
//!   after leaving the material at its tip, cuts back into it in its own view, measured by
//!   [`crate::material::reentry`], the predicate the leader router prices
//!   (`dims::leaders`), on the material [`crate::material::solids`] builds from the Analysis
//!   and the view's recorded transform. Info, with the depth cut, as in draftwright.
//!
//! A fillet carrying equal blend claims is judged against its own fillet faces.
//! An annotation whose view has no recorded transform (a captured Python sheet) is not judged,
//! as in [`super::fidelity`].

use std::collections::BTreeMap;
use std::f64::consts::PI;

use super::{LintIssue, Severity};
use crate::bridge::Feature;
use crate::bridge::holes::{self, Hole, dir_to_part, point, to_part};
use crate::model::Analysis;
use crate::output::{Drawing, DrawnFrame, Kind, P};

/// How far a tip may lie from the edge it names, page mm (draftwright's `2e-3`).
pub const TOLERANCE: f64 = 2e-3;

type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: V3) -> Option<V3> {
    let n = dot(a, a).sqrt();
    (n > 1e-12 && n.is_finite()).then(|| a.map(|x| x / n))
}

/// A view as drawn: its projection, and where the part's centre lands, as
/// [`super::coverage::drawn_centre_marks`] projects.
struct Projection {
    view: haecceity::hlr::View,
    at: P,
    centre: [f64; 2],
    scale: f64,
}

impl Projection {
    fn new(analysis: &Analysis, f: &DrawnFrame) -> Option<Projection> {
        let view = haecceity::hlr::View::new(f.direction, f.up)?;
        let [lo, hi] = analysis.envelope;
        let centre = view.map([0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k])));
        (f.scale.is_finite() && f.scale > 0.0).then_some(Projection {
            view,
            at: f.at,
            centre,
            scale: f.scale,
        })
    }

    fn page(&self, p: V3) -> P {
        let q = self.view.map(p);
        [0, 1].map(|k| self.at[k] + self.scale * (q[k] - self.centre[k]))
    }

    /// Whether direction *n* is along the line of sight.
    fn end_on(&self, n: V3) -> bool {
        unit(n).is_some_and(|n| dot(n, self.view.toward).abs() > 1.0 - 1e-9)
    }

    /// The page distance from *tip* to the circle of radius *r* about *c* square to *n*.
    fn circle_gap(&self, c: V3, n: V3, r: f64, tip: P) -> f64 {
        let Some(n) = unit(n) else {
            return f64::INFINITY;
        };
        if self.end_on(n) {
            let p = self.page(c);
            return ((tip[0] - p[0]).hypot(tip[1] - p[1]) - self.scale * r).abs();
        }
        // An oblique or side-on circle projects to an ellipse or a segment: the nearest of
        // 720 points, refined by golden section between its neighbours.
        let a = if n[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let e1 = unit([
            a[1] * n[2] - a[2] * n[1],
            a[2] * n[0] - a[0] * n[2],
            a[0] * n[1] - a[1] * n[0],
        ])
        // Invariant: The circle normal is normalized and the chosen seed is not parallel to it.
        .unwrap();
        let e2 = [
            n[1] * e1[2] - n[2] * e1[1],
            n[2] * e1[0] - n[0] * e1[2],
            n[0] * e1[1] - n[1] * e1[0],
        ];
        let gap = |t: f64| {
            let (s, co) = t.sin_cos();
            let p = self.page([0, 1, 2].map(|k| c[k] + r * (co * e1[k] + s * e2[k])));
            (tip[0] - p[0]).hypot(tip[1] - p[1])
        };
        const N: usize = 720;
        let step = 2.0 * PI / N as f64;
        let k = (0..N)
            .min_by(|&i, &j| gap(i as f64 * step).total_cmp(&gap(j as f64 * step)))
            .unwrap_or(0);
        let (mut lo, mut hi) = ((k as f64 - 1.0) * step, (k as f64 + 1.0) * step);
        let g = 0.5 * (5f64.sqrt() - 1.0);
        for _ in 0..60 {
            let (x, y) = (hi - g * (hi - lo), lo + g * (hi - lo));
            if gap(x) < gap(y) {
                hi = y;
            } else {
                lo = x;
            }
        }
        gap(0.5 * (lo + hi)).min(gap(k as f64 * step))
    }
}

/// The boundary circles of hole *h*: centre, axis and radius.
fn rims(h: &Hole) -> Vec<(V3, V3, f64)> {
    let at = |d: f64| [0, 1, 2].map(|k| h.at[k] + d * h.direction[k]);
    let mut out = vec![(h.at, h.direction, 0.5 * h.diameter)];
    if let Some(d) = h.depth {
        out.push((at(d), h.direction, 0.5 * h.diameter));
    }
    for r in [h.counterbore, h.spotface].into_iter().flatten() {
        out.push((h.at, h.direction, 0.5 * r.diameter));
        if let Some(d) = r.depth {
            out.push((at(d), h.direction, 0.5 * r.diameter));
            out.push((at(d), h.direction, 0.5 * h.diameter));
        }
    }
    if let Some(c) = h.countersink {
        out.push((h.at, h.direction, 0.5 * c.diameter));
    }
    out.retain(|(c, _, r)| r.is_finite() && *r > 0.0 && c.iter().all(|x| x.is_finite()));
    out
}

/// The page distance from *tip* to double-D bore *f*'s trimmed profile, seen end-on in *p*: the
/// major circle's arcs between the flats, and the two flats. None when the bore is not seen
/// end-on; an error when its record cannot be read.
fn double_d_gap(
    analysis: &Analysis,
    p: &Projection,
    f: &Feature,
    tip: P,
) -> Result<Option<f64>, String> {
    let frame = holes::frame(analysis);
    let q = &f.parameters;
    let axis = point(q.get("axis")).ok_or("its record gives no axis")?;
    let at = point(q.get("location")).ok_or("its record gives no location")?;
    let flats = point(q.get("flat_direction")).ok_or("its record gives no flat direction")?;
    let size = |name: &str| {
        q.get(name)
            .and_then(serde_json::Value::as_f64)
            .filter(|x| x.is_finite() && *x > 0.0)
            .ok_or(format!("its record gives no {name}"))
    };
    let (major, across) = (size("major_diameter")?, size("across_flats")?);
    if !p.end_on(dir_to_part(&frame, axis)) {
        return Ok(None);
    }
    let at = to_part(&frame, at);
    let c = p.page(at);
    let n = unit(dir_to_part(&frame, flats)).ok_or("its flat direction is a zero vector")?;
    let e = p.page([0, 1, 2].map(|k| at[k] + n[k]));
    let (nx, ny) = ((e[0] - c[0]) / p.scale, (e[1] - c[1]) / p.scale);
    let (r, a) = (0.5 * p.scale * major, 0.5 * p.scale * across.min(major));
    let w = (r * r - a * a).max(0.0).sqrt();
    let v = [tip[0] - c[0], tip[1] - c[1]];
    let (along, across) = (v[0] * nx + v[1] * ny, v[1] * nx - v[0] * ny);
    let d = along.hypot(across);
    // The arcs: the nearest point of the circle, when it lies between the flats.
    let mut gap = if d > 1e-12 && (r * along / d).abs() <= a {
        (d - r).abs()
    } else {
        f64::INFINITY
    };
    // The flats, each from end to end (their ends are the arcs' ends).
    for side in [-a, a] {
        let t = across.clamp(-w, w);
        gap = gap.min((along - side).hypot(across - t));
    }
    Ok(Some(gap))
}

/// The page distance from *t* to segment *a*–*b*.
fn segment_gap(a: P, b: P, t: P) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let n = d[0] * d[0] + d[1] * d[1];
    let s = if n < 1e-24 {
        0.0
    } else {
        (((t[0] - a[0]) * d[0] + (t[1] - a[1]) * d[1]) / n).clamp(0.0, 1.0)
    };
    (t[0] - a[0] - s * d[0]).hypot(t[1] - a[1] - s * d[1])
}

/// The page distance from *t* to the convex quadrilateral *q* (zero inside it).
fn quad_gap(q: [P; 4], t: P) -> f64 {
    let side = |i: usize| {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        (b[0] - a[0]) * (t[1] - a[1]) - (b[1] - a[1]) * (t[0] - a[0])
    };
    let sides = [0, 1, 2, 3].map(side);
    if sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0) {
        return 0.0;
    }
    (0..4)
        .map(|i| segment_gap(q[i], q[(i + 1) % 4], t))
        .fold(f64::INFINITY, f64::min)
}

/// The page distance from *tip* to turned step *f*'s cylinder as *p* draws it: its outline on each
/// side of its axis, and its end edges, trimmed by the flats that cut it; or, seen along its axis,
/// its trimmed circle. Err when its record cannot be read.
fn step_gap(analysis: &Analysis, p: &Projection, f: &Feature, tip: P) -> Result<f64, String> {
    use super::turned as t;
    let b = t::band(&holes::frame(analysis), f)?;
    let cuts = t::cuts(analysis, &b);
    let mut a = [0.0; 3];
    a[b.axis] = 1.0;
    let mut stations = vec![b.lo, b.hi];
    stations.extend(cuts.iter().flat_map(|c| [c.lo, c.hi]));
    stations.sort_by(f64::total_cmp);
    if p.end_on(a) {
        let mut gap = f64::INFINITY;
        for w in stations.windows(2).filter(|w| w[1] - w[0] > 1e-9) {
            for arc in t::arcs(&b, &cuts, 0.5 * (w[0] + w[1])) {
                let (c, r, span) = arc.projected(|q| p.page(q));
                let d = (tip[0] - c[0]).hypot(tip[1] - c[1]);
                if span.contains(
                    (tip[1] - c[1]).atan2(tip[0] - c[0]),
                    TOLERANCE / r.max(1e-9),
                ) {
                    gap = gap.min((d - r).abs());
                }
            }
        }
        return Ok(gap);
    }
    let t3 = p.view.toward;
    let u = unit([
        t3[1] * a[2] - t3[2] * a[1],
        t3[2] * a[0] - t3[0] * a[2],
        t3[0] * a[1] - t3[1] * a[0],
    ])
    .ok_or("the view looks along its axis")?;
    let mut gap = f64::INFINITY;
    for d in [u, u.map(|x| -x)] {
        for [q0, q1] in t::outline(&b, &cuts, d) {
            gap = gap.min(segment_gap(p.page(q0), p.page(q1), tip));
        }
    }
    // The end edges, from one side's outline to the other's.
    for end in [b.lo, b.hi] {
        let inside = if end == b.lo {
            b.lo + 1e-6
        } else {
            b.hi - 1e-6
        };
        if let (Some(mut q0), Some(mut q1)) = (
            t::extreme(&b, &cuts, inside, u),
            t::extreme(&b, &cuts, inside, u.map(|x| -x)),
        ) {
            q0[b.axis] = end;
            q1[b.axis] = end;
            gap = gap.min(segment_gap(p.page(q0), p.page(q1), tip));
        }
    }
    Ok(gap)
}

/// The page distance from *tip* to chamfer *f*'s bevel as *p* draws it, or why the view cannot
/// show it. A turned bevel (a cone about its coaxial band) reads in profile as its meridian
/// through its record's point turned onto the outline, either way the bevel may slope; or, seen
/// along its axis, as the ring it covers. A prismatic bevel reads seen along its edge, as its
/// section through the record's point, its legs along the two axes square to the edge. An oriented
/// bevel is its face: the quadrilateral its legs and run span.
fn chamfer_gap(analysis: &Analysis, p: &Projection, f: &Feature, tip: P) -> Result<f64, String> {
    let frame = holes::frame(analysis);
    if f.kind == "oriented_chamfer" {
        let b = super::oriented_chamfers::read(analysis, f)?;
        let along = |q: V3| dot([0, 1, 2].map(|k| q[k] - b.corner[k]), b.run);
        let mid = along(b.at);
        let at = |leg: V3, l: f64, s: f64| {
            p.page(
                [0, 1, 2].map(|k| b.corner[k] + l * leg[k] + (mid + s * 0.5 * b.length) * b.run[k]),
            )
        };
        return Ok(quad_gap(
            [
                at(b.u, b.legs[0], -1.0),
                at(b.u, b.legs[0], 1.0),
                at(b.v, b.legs[1], 1.0),
                at(b.v, b.legs[1], -1.0),
            ],
            tip,
        ));
    }
    let q = &f.parameters;
    let at = point(q.get("at"))
        .map(|l| to_part(&frame, l))
        .ok_or("its record gives no point on it (at)")?;
    let k = q
        .get("axis")
        .and_then(serde_json::Value::as_str)
        .and_then(|a| ["x", "y", "z"].iter().position(|x| *x == a))
        .and_then(|i| {
            let mut d = [0.0; 3];
            d[i] = 1.0;
            holes::principal(dir_to_part(&frame, d))
        })
        .ok_or("its record gives no principal axis")?;
    let leg = |name: &str| {
        q.get(name)
            .and_then(serde_json::Value::as_f64)
            .filter(|x| x.is_finite() && *x > 0.0)
            .ok_or(format!("its record gives no {name}"))
    };
    let (l1, l2) = (leg("leg1")?, leg("leg2")?);
    let mut a = [0.0; 3];
    a[k] = 1.0;
    // The bevel's section through *centre*, along *along* and *across* (unit, square to each
    // other), either way it slopes and either way round its legs lie.
    let section = |centre: V3, along: V3, across: V3| {
        let mut gap = f64::INFINITY;
        for (la, lr) in [(l1, l2), (l2, l1)] {
            for s in [-1.0, 1.0] {
                let e = [0, 1, 2].map(|i| 0.5 * (la * along[i] + s * lr * across[i]));
                let q0 = p.page([0, 1, 2].map(|i| centre[i] + e[i]));
                let q1 = p.page([0, 1, 2].map(|i| centre[i] - e[i]));
                gap = gap.min(segment_gap(q0, q1, tip));
            }
        }
        gap
    };
    if q.get("turned").and_then(serde_json::Value::as_bool) == Some(true) {
        let bands = super::turned::all_bands(analysis);
        // Its coaxial band's axis, or the one axis line every band about *k* shares (a part
        // turned whole, as the planner turns it).
        let (j, m) = ((k + 1) % 3, (k + 2) % 3);
        let along: Vec<&super::turned::Band> = bands.iter().filter(|b| b.axis == k).collect();
        let on = super::turned::coaxial(&bands, k, at, l1.max(l2))
            .map(|b| b.on)
            .or_else(|| {
                let first = along.first()?;
                along
                    .iter()
                    .all(|b| {
                        (b.on[j] - first.on[j]).abs() <= 1e-3
                            && (b.on[m] - first.on[m]).abs() <= 1e-3
                    })
                    .then_some(first.on)
            })
            .ok_or("no coaxial turned band holds its station")?;
        let b = super::turned::Band {
            axis: k,
            on,
            lo: at[k],
            hi: at[k],
            diameter: 0.0,
        };
        let rho = (at[j] - b.on[j]).hypot(at[m] - b.on[m]);
        if p.end_on(a) {
            let c = p.page(b.on);
            let d = (tip[0] - c[0]).hypot(tip[1] - c[1]);
            return Ok(((d - p.scale * rho).abs() - p.scale * 0.5 * l1.max(l2)).max(0.0));
        }
        let t3 = p.view.toward;
        let u = unit([
            t3[1] * a[2] - t3[2] * a[1],
            t3[2] * a[0] - t3[0] * a[2],
            t3[0] * a[1] - t3[1] * a[0],
        ])
        .ok_or("the view looks along its axis")?;
        let mut gap = f64::INFINITY;
        for w in [u, u.map(|x| -x)] {
            let mut o = b.on;
            o[k] = at[k];
            let o = [0, 1, 2].map(|i| o[i] + rho * w[i]);
            gap = gap.min(section(o, a, w));
        }
        return Ok(gap);
    }
    if !p.end_on(a) {
        return Err("the view does not see its edge end-on, where its bevel shows edge-on".into());
    }
    let (j, m) = ((k + 1) % 3, (k + 2) % 3);
    let (mut ej, mut em) = ([0.0; 3], [0.0; 3]);
    ej[j] = 1.0;
    em[m] = 1.0;
    Ok(section(at, ej, em))
}

fn issue(code: &str, name: &str, view: &str, tip: P, message: String, why: &str) -> LintIssue {
    LintIssue {
        sheet: None,
        id: format!("l:{code}:{name}"),
        code: code.into(),
        severity: if matches!(
            code,
            "radius_leader_target_unverifiable"
                | "step_diameter_leader_target_unverifiable"
                | "chamfer_leader_target_unverifiable"
        ) {
            Severity::Info
        } else {
            Severity::Warning
        },
        message,
        at: Some(tip),
        view: Some(view.into()),
        subjects: Vec::new(),
        requirements: Vec::new(),
        measurements: Vec::new(),
        sources: Vec::new(),
        stage: None,
        annotation: Some(name.into()),
        related: Vec::new(),
        evidence: Some(why.into()),
        fix: None,
    }
}

/// Every finding of this module on *drawing*.
pub(super) fn check(analysis: &Analysis, drawing: &Drawing) -> Vec<LintIssue> {
    let read: BTreeMap<&str, Hole> = holes::read(analysis)
        .into_iter()
        .filter_map(|(f, h)| Some((f.id.as_str(), h.ok()?)))
        .collect();
    let tips_shafts: BTreeMap<&str, [P; 2]> = drawing
        .sheet
        .items
        .iter()
        .filter(|d| d.kind == Kind::Dimension && d.elbow.is_some())
        .filter_map(|d| Some((d.name.as_str(), *d.lines.first()?)))
        .collect();
    let mut issues = Vec::new();
    for a in &drawing.annotations {
        let (Some(view), Some(&[tip, elbow])) =
            (a.view.as_deref(), tips_shafts.get(a.name.as_str()))
        else {
            continue;
        };
        let Some(p) = drawing
            .sheet
            .frames
            .get(view.strip_prefix("v:").unwrap_or(view))
            .and_then(|f| Projection::new(analysis, f))
        else {
            continue;
        };
        let claimed = |parameter: &str| -> Vec<&str> {
            a.claims
                .iter()
                .filter(|c| c.parameter == parameter)
                .map(|c| c.feature.as_str())
                .collect()
        };
        let mut found =
            |code: &str, subjects: Vec<&str>, parameter: &str, message: String, why: &str| {
                let mut i = issue(code, &a.name, view, tip, message, why);
                i.measurements = subjects
                    .iter()
                    .map(|f| format!("{f}/{parameter}"))
                    .collect();
                i.subjects = subjects.into_iter().map(String::from).collect();
                issues.push(i);
            };
        let bores = claimed("bore.diameter");
        if !bores.is_empty() {
            let mut gaps: Vec<f64> = bores
                .iter()
                .filter_map(|f| read.get(f))
                .flat_map(rims)
                .map(|(c, n, r)| p.circle_gap(c, n, r, tip))
                .collect();
            for f in analysis
                .features
                .iter()
                .filter(|f| f.kind == "double_d_bore" && bores.contains(&f.id.as_str()))
            {
                if let Ok(Some(gap)) = double_d_gap(analysis, &p, f, tip) {
                    gaps.push(gap);
                }
            }
            let gap = gaps.iter().copied().fold(f64::INFINITY, f64::min);
            if gaps.is_empty() {
                found(
                    "diameter_leader_target_unverifiable",
                    bores,
                    "bore.diameter",
                    format!(
                        "{}: the diameter leader's physical boundary cannot be verified",
                        a.name
                    ),
                    "no hole the callout names has a readable boundary in this view",
                );
            } else if gap > TOLERANCE {
                found(
                    "diameter_leader_target_mismatch",
                    bores,
                    "bore.diameter",
                    format!(
                        "{}: the diameter leader tip does not touch its named physical boundary (it lies {gap:.3} mm from the nearest rim of the holes it names)",
                        a.name
                    ),
                    "drawn tip is outside the named physical boundary tolerance",
                );
            }
        }
        // Read the drawn shaft, so authored/pinned leaders receive the same check.
        // A consolidated fillet names its own faces, rather than borrowing a blend target.
        let fillets = claimed("fillet.radius");
        let is_fillet = !fillets.is_empty();
        let (radii, parameter) = if fillets.is_empty() {
            (claimed("blend.radius"), "blend.radius")
        } else {
            (fillets, "fillet.radius")
        };
        if !radii.is_empty() {
            let mut shown = Vec::new();
            let mut reasons = Vec::new();
            for id in &radii {
                match crate::radius::projected(analysis, id, p.view.toward, |q| p.page(q)) {
                    Ok(arcs) => shown.extend(arcs),
                    Err(why) => reasons.push(why),
                }
            }
            reasons.sort();
            reasons.dedup();
            if shown.is_empty() {
                found(
                    "radius_leader_target_unverifiable",
                    radii,
                    parameter,
                    format!(
                        "{}: the radius leader's trimmed profile arc cannot be verified",
                        a.name
                    ),
                    &reasons.join("; "),
                );
            } else if !shown.iter().any(|arc| arc.on_arc(tip)) {
                found(
                    if is_fillet {
                        "radius_leader_not_radial"
                    } else {
                        "radius_leader_target_mismatch"
                    },
                    radii,
                    parameter,
                    format!(
                        "{}: the radius leader tip does not touch its named trimmed profile arc",
                        a.name
                    ),
                    "drawn tip is outside the trimmed physical profile arc tolerance",
                );
            } else if !shown.iter().any(|arc| arc.accepts([tip, elbow])) {
                let mut why: Vec<_> = shown
                    .iter()
                    .filter(|arc| arc.on_arc(tip))
                    .filter_map(|arc| arc.violation([tip, elbow]))
                    .collect();
                why.sort();
                why.dedup();
                found(
                    "radius_leader_not_radial",
                    radii,
                    parameter,
                    format!(
                        "{}: radius leader must touch the middle 25–75% of the arc and approach within 15 degrees of radial from either side; automatic routing keeps the drawn route when no compliant candidate fits",
                        a.name
                    ),
                    &why.join("; "),
                );
            }
        }
        for parameter in ["circular_step_radius.radius", "slot_end_radius.radius"] {
            let steps = claimed(parameter);
            if !steps.is_empty() {
                let arcs: Vec<_> = steps
                    .iter()
                    .filter_map(|id| analysis.profile_arcs.get(*id))
                    .flatten()
                    .collect();
                let on = arcs.iter().any(|arc| {
                    let (c, r, span) = arc.projected(|q| p.page(q));
                    ((tip[0] - c[0]).hypot(tip[1] - c[1]) - r).abs() <= TOLERANCE
                        && span.contains(
                            (tip[1] - c[1]).atan2(tip[0] - c[0]),
                            TOLERANCE / r.max(1e-9),
                        )
                });
                if !on {
                    found(
                        if arcs.is_empty() {
                            "radius_leader_target_unverifiable"
                        } else {
                            "radius_leader_target_mismatch"
                        },
                        steps,
                        parameter,
                        format!(
                            "{}: the radius leader tip does not touch its named trimmed circular profile arc",
                            a.name
                        ),
                        "drawn tip must be on a recorded circular boundary edge, within its angular span",
                    );
                }
            }
        }
        // A turned step's diameter, and a chamfer's size, against the faces they name.
        type Gap = fn(&Analysis, &Projection, &Feature, P) -> Result<f64, String>;
        for (parameters, noun, leader, what, gap) in [
            (
                &["step.diameter"][..],
                "step_diameter",
                "diameter",
                "diameter leader tip does not touch its named turned surface",
                step_gap as Gap,
            ),
            (
                &["chamfer.length", "oriented_chamfer.length"][..],
                "chamfer",
                "chamfer",
                "chamfer leader tip does not touch its named bevel",
                chamfer_gap as Gap,
            ),
        ] {
            // One callout may count members of several of these families (principal and
            // oriented chamfers): its tip need only meet one member's face.
            let ids: Vec<&str> = parameters.iter().flat_map(|p| claimed(p)).collect();
            let Some(parameter) = parameters.iter().find(|p| !claimed(p).is_empty()) else {
                continue;
            };
            let (mut gaps, mut reasons) = (Vec::new(), Vec::new());
            for f in analysis
                .features
                .iter()
                .filter(|f| ids.contains(&f.id.as_str()))
            {
                match gap(analysis, &p, f, tip) {
                    Ok(g) => gaps.push(g),
                    Err(why) => reasons.push(why),
                }
            }
            reasons.sort();
            reasons.dedup();
            let best = gaps.iter().copied().fold(f64::INFINITY, f64::min);
            if best <= TOLERANCE {
                continue;
            }
            // A miss is proven only when every claimed member could be measured: the tip
            // may sit on a member this view cannot verify.
            if !reasons.is_empty() {
                // Literal codes, so the finding-impact source inventory sees each one.
                found(
                    match noun {
                        "chamfer" => "chamfer_leader_target_unverifiable",
                        _ => "step_diameter_leader_target_unverifiable",
                    },
                    ids,
                    parameter,
                    format!(
                        "{}: the {leader} leader's target cannot be verified in this view",
                        a.name
                    ),
                    &reasons.join("; "),
                );
            } else {
                found(
                    match noun {
                        "chamfer" => "chamfer_leader_target_mismatch",
                        _ => "step_diameter_leader_target_mismatch",
                    },
                    ids,
                    parameter,
                    format!("{}: the {what} (it lies {best:.3} mm from it)", a.name),
                    "drawn tip is off the named physical surface's projected outline",
                );
            }
        }
        // The shaft against the material it left at its tip.
        let material = crate::material::solids(analysis, |q| p.page(q));
        let shaft = tips_shafts[a.name.as_str()];
        let cut = crate::material::reentry(&material, shaft);
        if cut > 0.0 {
            let mut i = issue(
                "leader_crosses_silhouette",
                &a.name,
                view,
                shaft[1],
                format!(
                    "leader '{}' shaft cuts {cut:.1} mm back through view '{}' — route it clear of the body, or dimension the feature in a detail view",
                    a.name,
                    view.strip_prefix("v:").unwrap_or(view)
                ),
                "the shaft re-enters the part's material after leaving it at its tip",
            );
            i.severity = Severity::Info;
            i.id = format!("l:leader_crosses_silhouette:{}", a.name);
            issues.push(i);
        }
    }
    issues
}
