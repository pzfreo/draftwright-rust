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
//! - `radius_leader_target_mismatch` (`blend_coverage`, ISSUE-1479): a callout claiming a
//!   straight blend's `blend.radius` has its tip on that blend's trimmed profile arc, seen
//!   end-on: on the circle of its radius about its rolling axis, and within the arc's sector.
//!   The sector is read from the blend's proven trimmed-surface points (`Analysis::blend_sites`,
//!   sampled at a quarter, a half and three quarters of its face's parameter range, so the arc
//!   runs half their spread again beyond them on each side). A tip on a centre bore, on another
//!   radius or on the untrimmed circle's extension is a warning. A circular-path (toroidal)
//!   blend is not checked, as in draftwright.
//! - `radius_leader_target_unverifiable`: a straight blend whose axis, radius or trimmed points
//!   cannot be read, or whose callout is in a view that does not show its arc end-on.
//!
//! - `leader_crosses_silhouette` (draftwright's structural check, #796/#798): a callout's shaft,
//!   after leaving the material at its tip, cuts back into it in its own view, measured by
//!   [`crate::material::reentry`], the predicate the leader router prices
//!   (`dims::leaders`), on the material [`crate::material::solids`] builds from the Analysis
//!   and the view's recorded transform. Info, with the depth cut, as in draftwright.
//!
//! A fillet callout carrying its equal blends' claims is not judged (its target is the fillet).
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

/// A straight blend's profile arc: its rolling axis (a point and direction), its radius and
/// its proven trimmed-surface points.
struct Arc {
    at: V3,
    axis: V3,
    radius: f64,
    sites: Vec<V3>,
}

fn arc(analysis: &Analysis, id: &str) -> Result<Option<Arc>, String> {
    let f = analysis
        .features
        .iter()
        .find(|f| f.id == id)
        .ok_or("the blend is not in the Analysis")?;
    let path = f.parameters.get("path").ok_or("the blend has no path")?;
    if path.get("normal").is_some() {
        // A circular-path blend: a different target contract, not checked (draftwright).
        return Ok(None);
    }
    let frame = holes::frame(analysis);
    let axis = super::parameters::direction(&frame, path.get("direction"))
        .and_then(unit)
        .ok_or("the blend's path has no direction")?;
    let at =
        super::parameters::place(&frame, path.get("at")).ok_or("the blend's path has no point")?;
    let radius = super::parameters::size(f, "radius").map_err(|_| "the blend has no radius")?;
    let sites = analysis
        .blend_sites
        .get(id)
        .filter(|s| !s.is_empty())
        .ok_or("the blend has no proven trimmed-surface point")?
        .clone();
    Ok(Some(Arc {
        at,
        axis,
        radius,
        sites,
    }))
}

/// *a* − *b* wrapped into (−π, π].
fn turn(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(2.0 * PI);
    if d > PI { d - 2.0 * PI } else { d }
}

/// Whether *tip* lies on *arc* as seen end-on in *p*, within [`TOLERANCE`].
fn on_arc(p: &Projection, arc: &Arc, tip: P) -> bool {
    let c = p.page(arc.at);
    let r = p.scale * arc.radius;
    let angle = |q: P| (q[1] - c[1]).atan2(q[0] - c[0]);
    if ((tip[0] - c[0]).hypot(tip[1] - c[1]) - r).abs() > TOLERANCE {
        return false;
    }
    let reference = angle(p.page(arc.sites[0]));
    let (lo, hi) = arc
        .sites
        .iter()
        .map(|s| turn(angle(p.page(*s)), reference))
        .fold((0.0f64, 0.0f64), |(lo, hi), d| (lo.min(d), hi.max(d)));
    let spread = hi - lo;
    // A lone point gives no spread: a blend's arc is at most a half turn, so a quarter turn
    // each way is taken (an overturnable judgement, recorded in docs/next-session.md).
    let (lo, hi) = if spread > 1e-6 {
        (lo - 0.5 * spread, hi + 0.5 * spread)
    } else {
        (-0.5 * PI, 0.5 * PI)
    };
    let slack = TOLERANCE / r.max(1e-9);
    let t = turn(angle(tip), reference);
    t >= lo - slack && t <= hi + slack
}

fn issue(code: &str, name: &str, view: &str, tip: P, message: String, why: &str) -> LintIssue {
    LintIssue {
        sheet: None,
        id: format!("l:{code}:{name}"),
        code: code.into(),
        severity: Severity::Warning,
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
        let (Some(view), Some(&[tip, _])) = (a.view.as_deref(), tips_shafts.get(a.name.as_str()))
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
        let mut found = |code: &str, subjects: Vec<&str>, parameter: &str, message, why| {
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
        // A fillet's leader that also carries its equal blends' claims (`dims::blends::
        // consolidate`) points at the fillet, whose record gives a point on its surface, not its
        // arc: it is judged as a fillet callout, which this module does not check.
        let blends = claimed("blend.radius");
        if !blends.is_empty() && claimed("fillet.radius").is_empty() {
            let mut arcs = Vec::new();
            let mut unreadable = None;
            for id in &blends {
                match arc(analysis, id) {
                    Ok(Some(arc)) => arcs.push(arc),
                    Ok(None) => {}
                    Err(e) => unreadable = unreadable.or(Some(e)),
                }
            }
            let shown: Vec<&Arc> = arcs.iter().filter(|c| p.end_on(c.axis)).collect();
            if !arcs.is_empty() && shown.is_empty() {
                unreadable = Some("the callout's view does not show the blend's arc end-on".into());
            }
            if shown.is_empty() {
                if let Some(why) = unreadable {
                    found(
                        "radius_leader_target_unverifiable",
                        blends,
                        "blend.radius",
                        format!(
                            "{}: the radius leader's trimmed profile arc cannot be verified",
                            a.name
                        ),
                        &why,
                    );
                }
            } else if !shown.iter().any(|c| on_arc(&p, c, tip)) {
                found(
                    "radius_leader_target_mismatch",
                    blends,
                    "blend.radius",
                    format!(
                        "{}: the radius leader tip does not touch its named trimmed profile arc",
                        a.name
                    ),
                    "drawn tip is outside the trimmed physical profile arc tolerance",
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
