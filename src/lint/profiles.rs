//! Principal profile coverage (Python's `lint_principal_profile_coverage` in
//! `linting/coverage.py`, LINT-code-unrecognised_defining_geometry-principal_profile): every
//! opening through a principal boundary face of the part is a profile draftwright's drawing
//! vocabulary can dimension, or a recognised feature accounts for it; and an outer boundary of
//! evenly spaced equal arcs on one circle (a toothed outline) is reported, since no feature
//! states it.
//!
//! Python scans the B-rep from lint, with its own denominator: no recognised feature can narrow
//! it. The port's lint never sees the part, so the same scan is made when the part is analysed
//! ([`evidence`], recorded as the Analysis's `principal_profiles`): for each solid of a part of
//! several (else the whole part), its box and each principal boundary face (Python's
//! `_principal_boundary_plane`: a plane face flat across exactly one axis, within the part's
//! tolerance of the solid's box there), with its inner loops' edges and its outer loop's arcs.
//! Lint judges that record ([`verdicts`], [`coverage`]):
//!
//! - an inner loop is supported when it is an open end of a section recess record (Python's
//!   `_recess_matches_principal_wire`: a closed, square-ended recess running along the face's
//!   axis whose boundary's vertices match the loop's), the mouth of a through double-D bore not
//!   claimed by another loop (`_double_d_bore_matches_principal_wire`: its arcs, flats and ends
//!   measured against the record), or a shape the vocabulary dimensions on its own
//!   (`_supported_inner_profile`: a circle, an axis-aligned rectangle, an obround with
//!   semicircular ends, or a rectangle with four equal quarter-circle corners);
//! - an outer loop with six or more arcs, not all of its edges, all on one circle with equal
//!   lengths and equally spaced, has a proven radial arc count (`_radial_outer_arc_count`).
//!
//! One `unrecognised_defining_geometry` for the unsupported openings and one for the largest
//! radial count, with Python's messages; info on a part of several solids (a general arrangement),
//! else a warning, as in Python. Python's tolerance is the part's: 1e-5 of its largest extent,
//! at least 1e-5 mm.
//!
//! The section recesses and double-D bores are the Analysis's features, in their records'
//! frame, carried into the part's. Python reads the same records from its recognition, in its
//! own order; the port takes the bores in the Analysis's order, which matters only when two
//! loops match one bore's mouth. An Analysis without the record (one made before it was kept)
//! has no sources, and nothing is reported.

use std::collections::BTreeSet;
use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::section_recesses::recesses;
use super::{LintIssue, Severity};
use crate::bridge::holes::{dir_to_part, frame, point, to_part};
use crate::model::Analysis;

type P3 = [f64; 3];

/// What an edge is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Line,
    Circle,
    Other,
}

/// An edge of a principal face's loop, in the part's frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub kind: Kind,
    pub start: P3,
    pub end: P3,
    /// The point halfway along it.
    pub mid: P3,
    pub length: f64,
    /// Its axis-aligned box: exact for lines and arcs.
    pub bounds: [P3; 2],
    /// A circle's centre and radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre: Option<P3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
}

/// An inner loop: its distinct edges, and its distinct vertices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub edges: Vec<Edge>,
    pub vertices: Vec<P3>,
}

/// A principal boundary face: the axis it is flat across and where, its inner loops, and its
/// outer loop's arcs and number of edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    pub axis: usize,
    pub at: f64,
    pub inner: Vec<Wire>,
    pub outer_edges: usize,
    pub outer_arcs: Vec<Edge>,
    /// Straight boundary edges for choosing exposed plate profiles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_lines: Vec<Edge>,
    /// Its loops could not be told apart (the kernel found no outer loop), so its openings are
    /// not read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unsplit: bool,
}

/// A curved face's measured bounds and its revolution axis, when analytic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurvedFace {
    pub bounds: [P3; 2],
    pub axis: Option<P3>,
}

/// One solid scanned (the whole part, when it has one): its box and principal faces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub bounds: [P3; 2],
    pub faces: Vec<Face>,
    /// Model-space boundary chords for annotation clearance, including internal face edges.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary_segments: Vec<[P3; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curved_faces: Vec<CurvedFace>,
}

/// Python's tolerance for a solid of box *bounds*.
fn tolerance(bounds: &[P3; 2]) -> f64 {
    let extent = (0..3)
        .map(|k| bounds[1][k] - bounds[0][k])
        .fold(f64::NEG_INFINITY, f64::max);
    (extent * 1e-5).max(1e-5)
}

fn edge(e: &haecceity::brep::Edge) -> Edge {
    use haecceity::geom::Curve;
    let (kind, centre, radius) = match &e.curve {
        Curve::Line { .. } => (Kind::Line, None, None),
        Curve::Circle { frame, radius } => (Kind::Circle, Some(frame.origin), Some(*radius)),
        _ => (Kind::Other, None, None),
    };
    let mut points = vec![e.start, e.end];
    let length = match &e.curve {
        Curve::Line { .. } => dist(e.start, e.end),
        Curve::Circle { frame, radius } => {
            let (a, b) = haecceity::sampling::edge_interval(
                &e.curve,
                e.start,
                e.end,
                e.same_sense,
                e.is_closed(),
            );
            let (lo, hi) = (a.min(b), a.max(b));
            // Where the arc may be extreme along each axis: the angles at which the circle's
            // tangent is square to it, each turn of them inside the edge's interval.
            for k in 0..3 {
                let t0 = frame.y[k].atan2(frame.x[k]);
                for base in [t0, t0 + PI] {
                    let mut t = base + ((lo - base) / TAU).ceil() * TAU;
                    while t <= hi {
                        points.push(e.curve.value(t));
                        t += TAU;
                    }
                }
            }
            radius * (hi - lo)
        }
        _ => {
            points.extend(&e.samples);
            e.samples.windows(2).map(|w| dist(w[0], w[1])).sum()
        }
    };
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in &points {
        for k in 0..3 {
            bounds[0][k] = bounds[0][k].min(p[k]);
            bounds[1][k] = bounds[1][k].max(p[k]);
        }
    }
    Edge {
        kind,
        start: e.start,
        end: e.end,
        mid: e.midpoint(),
        length,
        bounds,
        centre,
        radius,
    }
}

/// The wire of *part*'s edges *edges*: each once, with its distinct vertices.
fn wire(part: &haecceity::Part, edges: &[usize]) -> Wire {
    let mut seen = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    let mut out = Wire {
        edges: Vec::new(),
        vertices: Vec::new(),
    };
    for &i in edges {
        if !seen.insert(i) {
            continue;
        }
        let e = &part.edges[i];
        out.edges.push(edge(e));
        for (v, p) in [(e.vertices.0, e.start), (e.vertices.1, e.end)] {
            if vertices.insert(v) {
                out.vertices.push(p);
            }
        }
    }
    out
}

/// Analytic generator lines tangent to the three principal datum directions. These are
/// model-space surface facts, not hidden-line projections or paper-coordinate geometry.
fn principal_tangents(part: &haecceity::Part, faces: &[usize]) -> Vec<[P3; 2]> {
    use haecceity::geom::Surface;
    let mut out = Vec::new();
    for &f in faces {
        let surface = &part.faces[f].surface;
        if !matches!(surface, Surface::Cylinder { .. } | Surface::Cone { .. }) {
            continue;
        }
        let Some((u0, u1, v0, v1)) = part.uv_bounds(f) else {
            continue;
        };
        for axis in 0..3 {
            let g = |u| surface.normal(u, (v0 + v1) * 0.5).map_or(0.0, |n| n[axis]);
            let c = (g(0.0) + g(PI)) * 0.5;
            let a = g(0.0) - c;
            let b = g(PI * 0.5) - c;
            let r = a.hypot(b);
            if r < 1e-9 || c.abs() > r {
                continue;
            }
            let phase = b.atan2(a);
            let angle = (-c / r).clamp(-1.0, 1.0).acos();
            for base in [phase + angle, phase - angle] {
                let mut u = base + ((u0 - base) / TAU).ceil() * TAU;
                while u <= u1 + 1e-9 {
                    out.push([surface.value(u, v0), surface.value(u, v1)]);
                    u += TAU;
                }
            }
        }
    }
    out
}

/// The principal boundary faces of the solids of *part* (each solid of a part of several, else
/// the whole part), as Python's scan reads them: what the Analysis records.
pub fn evidence(part: &haecceity::Part) -> Vec<Source> {
    use haecceity::geom::Surface;
    let scopes: Vec<(haecceity::geom::Bounds, Vec<usize>)> = if part.solids.len() > 1 {
        (0..part.solids.len())
            .map(|s| (part.solid_bounds(s), part.solids[s].faces.clone()))
            .collect()
    } else {
        vec![(part.bounds(), (0..part.faces.len()).collect())]
    };
    scopes
        .into_iter()
        .map(|(b, faces)| {
            let bounds = [b.min, b.max];
            let tol = tolerance(&bounds);
            let curved_faces = faces
                .iter()
                .filter_map(|&f| {
                    let axis = match &part.faces[f].surface {
                        Surface::Plane { .. } | Surface::Cylinder { .. } | Surface::Cone { .. } => {
                            return None;
                        }
                        Surface::Torus { frame, .. } => Some(frame.z),
                        _ => None,
                    };
                    let b = part.face_bounds(f);
                    Some(CurvedFace {
                        bounds: [b.min, b.max],
                        axis,
                    })
                })
                .collect();
            let edges: BTreeSet<_> = faces
                .iter()
                .flat_map(|&f| &part.faces[f].loops)
                .flat_map(|l| &l.edges)
                .map(|&(i, _)| i)
                .collect();
            let mut boundary_segments: Vec<_> = edges
                .into_iter()
                .flat_map(|i| {
                    let e = &part.edges[i];
                    let samples = if e.samples.len() >= 2 {
                        e.samples.clone()
                    } else {
                        vec![e.start, e.end]
                    };
                    samples.windows(2).map(|p| [p[0], p[1]]).collect::<Vec<_>>()
                })
                .collect();
            boundary_segments.extend(principal_tangents(part, &faces));
            let faces = faces
                .into_iter()
                .filter(|&f| matches!(part.faces[f].surface, Surface::Plane { .. }))
                .filter_map(|f| {
                    let fb = part.face_bounds(f);
                    let flat: Vec<usize> =
                        (0..3).filter(|&k| fb.max[k] - fb.min[k] <= tol).collect();
                    let [axis] = flat[..] else {
                        return None;
                    };
                    let at = (fb.min[axis] + fb.max[axis]) / 2.0;
                    if (at - b.min[axis]).abs().min((at - b.max[axis]).abs()) > tol {
                        return None;
                    }
                    let loops = &part.faces[f].loops;
                    let outer = part.outer_loop(f);
                    let unsplit = outer.is_none() && loops.len() > 1;
                    let inner = quiddity::features::wire_seed::inner_wires(part, f)
                        .iter()
                        .map(|edges| wire(part, edges))
                        .collect();
                    let outer = wire(part, &part.outer_edges(f));
                    Some(Face {
                        axis,
                        at,
                        inner,
                        outer_edges: outer.edges.len(),
                        outer_lines: outer
                            .edges
                            .iter()
                            .filter(|e| e.kind == Kind::Line)
                            .cloned()
                            .collect(),
                        outer_arcs: outer
                            .edges
                            .into_iter()
                            .filter(|e| e.kind == Kind::Circle)
                            .collect(),
                        unsplit,
                    })
                })
                .collect();
            Source {
                bounds,
                faces,
                boundary_segments,
                curved_faces,
            }
        })
        .collect()
}

fn dist(a: P3, b: P3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The box of a wire's edges.
fn wire_bounds(w: &Wire) -> [P3; 2] {
    let lo = [0, 1, 2].map(|k| {
        w.edges
            .iter()
            .map(|e| e.bounds[0][k])
            .fold(f64::INFINITY, f64::min)
    });
    let hi = [0, 1, 2].map(|k| {
        w.edges
            .iter()
            .map(|e| e.bounds[1][k])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    [lo, hi]
}

fn size(b: &[P3; 2], k: usize) -> f64 {
    b[1][k] - b[0][k]
}

fn mid(b: &[P3; 2], k: usize) -> f64 {
    (b[0][k] + b[1][k]) / 2.0
}

/// The two axes across a face flat across *axis*, in order.
fn plane_axes(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

/// Whether *w*'s shape alone is one draftwright's vocabulary dimensions (Python's
/// `_supported_inner_profile` without a recess or a bore): a circle, an axis-aligned rectangle, a
/// true obround, or a rectangle with four equal quarter-circle corners.
pub fn vocabulary(w: &Wire, plane: [usize; 2], tol: f64) -> bool {
    let edges = &w.edges;
    if edges.is_empty() {
        return false;
    }
    let wbb = wire_bounds(w);
    let ext = |k: usize| size(&wbb, k);
    if edges.iter().all(|e| e.kind == Kind::Circle) {
        let (Some(radius), Some(centre)) = (edges[0].radius, edges[0].centre) else {
            return false;
        };
        let same_circle = edges[1..].iter().all(|e| match (e.radius, e.centre) {
            (Some(r), Some(c)) => {
                (r - radius).abs() <= tol && plane.iter().all(|&k| (c[k] - centre[k]).abs() <= tol)
            }
            _ => false,
        });
        return same_circle
            && plane
                .iter()
                .all(|&k| (ext(k) - 2.0 * radius).abs() <= (8.0 * tol).max(radius * 1e-3));
    }
    let perimeter = 2.0 * (ext(plane[0]) + ext(plane[1]));
    if edges.iter().all(|e| e.kind == Kind::Line) {
        // Split collinear edges are harmless, but every segment must lie on one of the four
        // sides of the box; the perimeter alone would also pass an L-shaped loop.
        let on_boundary = edges.iter().all(|e| {
            let varying: Vec<usize> = plane
                .iter()
                .copied()
                .filter(|&k| size(&e.bounds, k) > tol)
                .collect();
            let [v] = varying[..] else {
                return false;
            };
            let fixed = if plane[0] == v { plane[1] } else { plane[0] };
            let at = mid(&e.bounds, fixed);
            (at - wbb[0][fixed]).abs().min((at - wbb[1][fixed]).abs()) <= tol
        });
        let length: f64 = edges.iter().map(|e| e.length).sum();
        return on_boundary && (length - perimeter).abs() <= (8.0 * tol).max(perimeter * 1e-3);
    }
    if edges.iter().any(|e| e.kind == Kind::Other) {
        return false;
    }
    let lines: Vec<&Edge> = edges.iter().filter(|e| e.kind == Kind::Line).collect();
    let arcs: Vec<&Edge> = edges.iter().filter(|e| e.kind == Kind::Circle).collect();
    if !matches!(lines.len(), 2 | 4) || arcs.len() < 2 {
        return false;
    }
    // Python's stable sort by extent: a tie keeps the plane's order.
    let (short_axis, long_axis) = if ext(plane[1]) < ext(plane[0]) {
        (plane[1], plane[0])
    } else {
        (plane[0], plane[1])
    };
    let short = ext(short_axis);
    let radius_tol = (8.0 * tol).max(short * 1e-3);
    let Some(radius) = arcs[0].radius else {
        return false;
    };
    if radius <= tol
        || arcs
            .iter()
            .any(|e| e.radius.is_none_or(|r| (r - radius).abs() > radius_tol))
    {
        return false;
    }
    let profile_tol = radius_tol.max(ext(plane[0]).max(ext(plane[1])) * 1e-3);
    let other = |k: usize| if plane[0] == k { plane[1] } else { plane[0] };
    let line_matches = |e: &Edge, varying: usize, corner: f64| {
        let fixed = other(varying);
        if size(&e.bounds, varying) <= tol || size(&e.bounds, fixed) > tol {
            return false;
        }
        let at = mid(&e.bounds, fixed);
        if (at - wbb[0][fixed]).abs().min((at - wbb[1][fixed]).abs()) > profile_tol {
            return false;
        }
        (e.bounds[0][varying] - (wbb[0][varying] + corner)).abs() <= profile_tol
            && (e.bounds[1][varying] - (wbb[1][varying] - corner)).abs() <= profile_tol
    };
    let both_sides = |varying: usize| {
        let matched: Vec<&&Edge> = lines
            .iter()
            .filter(|e| line_matches(e, varying, radius))
            .collect();
        if matched.len() != 2 {
            return false;
        }
        let fixed = other(varying);
        let positions: Vec<f64> = matched.iter().map(|e| mid(&e.bounds, fixed)).collect();
        [wbb[0][fixed], wbb[1][fixed]]
            .iter()
            .all(|bound| positions.iter().any(|p| (p - bound).abs() <= profile_tol))
    };
    let arcs_match = |expected: &[[f64; 2]], expected_length: f64| {
        let mut lengths = vec![0.0; expected.len()];
        for e in &arcs {
            let Some(c) = e.centre else {
                return false;
            };
            let matches: Vec<usize> = (0..expected.len())
                .filter(|&i| {
                    plane
                        .iter()
                        .enumerate()
                        .all(|(j, &k)| (c[k] - expected[i][j]).abs() <= profile_tol)
                })
                .collect();
            let [i] = matches[..] else {
                return false;
            };
            lengths[i] += e.length;
        }
        lengths
            .iter()
            .all(|l| (l - expected_length).abs() <= profile_tol)
    };
    if lines.len() == 2 {
        // A true obround has two long sides, semicircular ends, and cap radius equal to half
        // the short overall extent. A double-D fails that last correspondence.
        if (radius - short / 2.0).abs() > radius_tol || !both_sides(long_axis) {
            return false;
        }
        let mid_short = mid(&wbb, short_axis);
        let long_lo = wbb[0][long_axis] + radius;
        let long_hi = wbb[1][long_axis] - radius;
        let expected = if plane[0] == long_axis {
            [[long_lo, mid_short], [long_hi, mid_short]]
        } else {
            [[mid_short, long_lo], [mid_short, long_hi]]
        };
        return arcs_match(&expected, PI * radius);
    }
    // Four side runs ending at four quarter-circle corner groups.
    if radius * 2.0 >= short - radius_tol {
        return false;
    }
    if !plane.iter().all(|&k| both_sides(k)) {
        return false;
    }
    let centres = |k: usize| [wbb[0][k] + radius, wbb[1][k] - radius];
    let mut expected = Vec::new();
    for a in centres(plane[0]) {
        for b in centres(plane[1]) {
            expected.push([a, b]);
        }
    }
    arcs_match(&expected, PI * radius / 2.0)
}

/// A section recess record's geometry as the match reads it, in the part's frame.
struct Recess {
    origin: P3,
    run: P3,
    u: P3,
    v: P3,
    interval: [f64; 2],
    open: [bool; 2],
    boundary: Vec<([f64; 2], f64)>,
}

impl Recess {
    /// A closed, square-ended recess record, or none.
    fn read(analysis: &Analysis, p: &serde_json::Map<String, Value>) -> Option<Recess> {
        let g = p.get("geometry")?;
        let profile = g.get("profile")?;
        if profile.get("closure")?.as_str()? != "closed" {
            return None;
        }
        let mut open = [false; 2];
        for (i, name) in ["low", "high"].iter().enumerate() {
            let end = g.get("ends")?.get(name)?;
            let s = end.get("surface")?;
            let square = s.get("type")?.as_str()? == "plane"
                && s.get("gradient")?
                    .as_array()
                    .is_some_and(|g| g.len() == 2 && g.iter().all(|x| x.as_f64() == Some(0.0)));
            if !square {
                return None;
            }
            open[i] = end.get("condition")?.as_str()? == "open";
        }
        let frame_of = frame(analysis);
        let f = g.get("frame")?;
        let interval = g.get("run_interval")?.as_array()?;
        let boundary = profile
            .get("boundary")?
            .as_array()?
            .iter()
            .map(|v| {
                let pt = v.get("point")?.as_array()?;
                Some((
                    [pt.first()?.as_f64()?, pt.get(1)?.as_f64()?],
                    v.get("bulge")?.as_f64()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Recess {
            origin: to_part(&frame_of, point(f.get("origin"))?),
            run: dir_to_part(&frame_of, point(f.get("run"))?),
            u: dir_to_part(&frame_of, point(f.get("u"))?),
            v: dir_to_part(&frame_of, point(f.get("v"))?),
            interval: [interval.first()?.as_f64()?, interval.get(1)?.as_f64()?],
            open,
            boundary,
        })
    }

    /// Whether *w*, on a face flat across *axis* at *at*, is one of this recess's open ends
    /// (Python's `_recess_matches_principal_wire`).
    fn matches(&self, w: &Wire, axis: usize, at: f64, tol: f64) -> bool {
        let plane = plane_axes(axis);
        let run = self.run;
        if (run[axis].abs() - 1.0).abs() > tol || (0..3).any(|k| k != axis && run[k].abs() > tol) {
            return false;
        }
        let near = (8.0 * tol).max(1e-3);
        let Some(station) = (0..2)
            .filter(|&i| self.open[i])
            .map(|i| self.interval[i])
            .find(|s| (self.origin[axis] + run[axis] * s - at).abs() <= near)
        else {
            return false;
        };
        if self.boundary.is_empty() || self.boundary.len() != w.vertices.len() {
            return false;
        }
        if w.edges.iter().any(|e| e.kind == Kind::Other) {
            return false;
        }
        let expected_arcs = self.boundary.iter().filter(|(_, b)| b.abs() > tol).count();
        let actual_arcs = w.edges.iter().filter(|e| e.kind == Kind::Circle).count();
        if expected_arcs != actual_arcs {
            return false;
        }
        let mut unmatched: Vec<[f64; 2]> = w
            .vertices
            .iter()
            .map(|p| [p[plane[0]], p[plane[1]]])
            .collect();
        for ([su, sv], _) in &self.boundary {
            let world: P3 = [0, 1, 2]
                .map(|i| self.origin[i] + run[i] * station + self.u[i] * su + self.v[i] * sv);
            let point = [world[plane[0]], world[plane[1]]];
            let Some(i) = unmatched
                .iter()
                .position(|c| (0..2).all(|j| (point[j] - c[j]).abs() <= near))
            else {
                return false;
            };
            unmatched.remove(i);
        }
        unmatched.is_empty()
    }
}

/// A double-D bore record as the match reads it, in the part's frame.
struct Bore {
    axis: P3,
    location: P3,
    depth: f64,
    major: f64,
    across: f64,
    flat: P3,
}

impl Bore {
    /// A through double-D bore record, or none.
    fn read(analysis: &Analysis, p: &serde_json::Map<String, Value>) -> Option<Bore> {
        if p.get("through")? != &Value::Bool(true) {
            return None;
        }
        let frame_of = frame(analysis);
        let finite = |name: &str| p.get(name)?.as_f64().filter(|x| x.is_finite());
        let bore = Bore {
            axis: dir_to_part(&frame_of, point(p.get("axis"))?),
            location: to_part(&frame_of, point(p.get("location"))?),
            depth: finite("depth")?,
            major: finite("major_diameter")?,
            across: finite("across_flats")?,
            flat: dir_to_part(&frame_of, point(p.get("flat_direction"))?),
        };
        bore.axis
            .iter()
            .chain(&bore.location)
            .chain(&bore.flat)
            .all(|x| x.is_finite())
            .then_some(bore)
    }

    /// Its two ends along *axis*: its location's, and the far one.
    fn ends(&self, axis: usize) -> [f64; 2] {
        [
            self.location[axis],
            self.location[axis] - self.axis[axis] * self.depth,
        ]
    }

    /// Whether *w*, on a face of the solid of box *solid* flat across *axis* at *at*, is one of
    /// this bore's two mouths (Python's `_double_d_bore_matches_principal_wire`).
    fn matches(&self, w: &Wire, axis: usize, at: f64, solid: &[P3; 2], tol: f64) -> bool {
        let plane = plane_axes(axis);
        let c = self.axis[axis];
        if (c.abs() - 1.0).abs() > 1e-6 || (0..3).any(|k| k != axis && self.axis[k].abs() > 1e-6) {
            return false;
        }
        if self.depth <= tol {
            return false;
        }
        let mut ends = self.ends(axis);
        ends.sort_by(f64::total_cmp);
        let span_tol = (8.0 * tol).max(self.depth * 1e-5).max(1e-4);
        if (ends[0] - solid[0][axis]).abs() > span_tol
            || (ends[1] - solid[1][axis]).abs() > span_tol
        {
            return false;
        }
        if ends.iter().filter(|e| (at - *e).abs() <= span_tol).count() != 1 {
            return false;
        }
        let radius = self.major / 2.0;
        let half = self.across / 2.0;
        if radius <= tol || half <= tol || half >= radius - tol {
            return false;
        }
        let flat = self.flat;
        if flat[axis].abs() > 1e-6 {
            return false;
        }
        let norm = plane.iter().map(|&k| flat[k] * flat[k]).sum::<f64>().sqrt();
        if (norm - 1.0).abs() > 1e-6 {
            return false;
        }
        let lines: Vec<&Edge> = w.edges.iter().filter(|e| e.kind == Kind::Line).collect();
        let arcs: Vec<&Edge> = w.edges.iter().filter(|e| e.kind == Kind::Circle).collect();
        if w.edges.len() != 4 || lines.len() != 2 || arcs.len() != 2 {
            return false;
        }
        let mut centre = self.location;
        centre[axis] = at;
        let wbb = wire_bounds(w);
        let scale = plane
            .iter()
            .map(|&k| size(&wbb, k))
            .fold(f64::NEG_INFINITY, f64::max);
        let metric = (8.0 * tol).max(scale * 1e-3).max(1e-4);
        for a in &arcs {
            let (Some(r), Some(ac)) = (a.radius, a.centre) else {
                return false;
            };
            if (r - radius).abs() > metric
                || plane.iter().any(|&k| (ac[k] - centre[k]).abs() > metric)
            {
                return false;
            }
        }
        let chord = 2.0 * (radius * radius - half * half).sqrt();
        let mut offsets = Vec::new();
        for l in &lines {
            if (l.length - chord).abs() > metric {
                return false;
            }
            let ends = [l.start, l.end];
            if ends.iter().any(|e| (e[axis] - at).abs() > metric)
                || ends
                    .iter()
                    .any(|e| (dist(*e, centre) - radius).abs() > metric)
            {
                return false;
            }
            let delta: P3 = [0, 1, 2].map(|i| l.end[i] - l.start[i]);
            let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
            if length <= tol {
                return false;
            }
            if (0..3)
                .map(|i| delta[i] / length * flat[i])
                .sum::<f64>()
                .abs()
                > 1e-4
            {
                return false;
            }
            let midpoint: P3 = [0, 1, 2].map(|i| (l.start[i] + l.end[i]) / 2.0);
            offsets.push(
                (0..3)
                    .map(|i| (midpoint[i] - centre[i]) * flat[i])
                    .sum::<f64>(),
            );
        }
        offsets.sort_by(f64::total_cmp);
        if (offsets[0] + half).abs() > metric || (offsets[1] - half).abs() > metric {
            return false;
        }
        let arc = 2.0 * radius * (half / radius).asin();
        arcs.iter().all(|a| (a.length - arc).abs() <= metric)
    }
}

/// How one inner loop of a principal face is accounted for, in Python's order of trial.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    /// The source (solid) it is on, the axis its face is flat across and where.
    pub source: usize,
    pub axis: usize,
    pub at: f64,
    /// Its box across the face (the plane's two axes, in order): `[a0, b0, a1, b1]`.
    pub bounds: [f64; 4],
    /// An open end of a section recess record.
    pub recess: bool,
    /// The mouth of a through double-D bore no earlier loop claimed.
    pub double_d: bool,
    /// Its shape alone is in the vocabulary.
    pub vocabulary: bool,
    pub supported: bool,
}

/// Every inner loop of every principal face, judged in order (a double-D bore's mouth claimed
/// once); a face whose loops could not be told apart is one unsupported verdict with no box.
pub fn verdicts(analysis: &Analysis) -> Vec<Verdict> {
    let recesses: Vec<Recess> = recesses(analysis)
        .into_iter()
        .filter_map(|f| Recess::read(analysis, &f.parameters))
        .collect();
    let bores: Vec<Option<Bore>> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "double_d_bore")
        .map(|f| Bore::read(analysis, &f.parameters))
        .collect();
    let mut claimed: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut out = Vec::new();
    for (s, source) in analysis.principal_profiles.iter().enumerate() {
        let tol = tolerance(&source.bounds);
        for face in &source.faces {
            let plane = plane_axes(face.axis);
            if face.unsplit {
                out.push(Verdict {
                    source: s,
                    axis: face.axis,
                    at: face.at,
                    bounds: [f64::NAN; 4],
                    recess: false,
                    double_d: false,
                    vocabulary: false,
                    supported: false,
                });
                continue;
            }
            for w in &face.inner {
                let recess = recesses
                    .iter()
                    .any(|r| r.matches(w, face.axis, face.at, tol));
                let double_d = !recess
                    && bores.iter().enumerate().any(|(i, b)| {
                        let Some(b) = b else {
                            return false;
                        };
                        if !b.matches(w, face.axis, face.at, &source.bounds, tol) {
                            return false;
                        }
                        let ends = b.ends(face.axis);
                        let mouth =
                            usize::from((face.at - ends[1]).abs() < (face.at - ends[0]).abs());
                        claimed.insert((i, mouth))
                    });
                let vocabulary = vocabulary(w, plane, tol);
                let b = wire_bounds(w);
                out.push(Verdict {
                    source: s,
                    axis: face.axis,
                    at: face.at,
                    bounds: [
                        b[0][plane[0]],
                        b[0][plane[1]],
                        b[1][plane[0]],
                        b[1][plane[1]],
                    ],
                    recess,
                    double_d,
                    vocabulary,
                    supported: recess || double_d || vocabulary,
                });
            }
        }
    }
    out
}

/// A source's proven radial arc count (Python's `_radial_outer_arc_count`): the most arcs, six
/// or more and not every edge, of a principal face's outer loop that share one circle, have
/// equal lengths and are equally spaced round it.
pub fn radial_count(source: &Source) -> Option<usize> {
    let tol = tolerance(&source.bounds);
    let mut best: Option<usize> = None;
    for face in &source.faces {
        let plane = plane_axes(face.axis);
        let arcs = &face.outer_arcs;
        if arcs.len() < 6 || arcs.len() == face.outer_edges {
            continue;
        }
        let (Some(radius), Some(centre)) = (arcs[0].radius, arcs[0].centre) else {
            continue;
        };
        let length = arcs[0].length;
        let metric = (8.0 * tol).max(radius * 1e-3);
        if arcs[1..].iter().any(|e| match (e.radius, e.centre) {
            (Some(r), Some(c)) => {
                (r - radius).abs() > metric
                    || (e.length - length).abs() > metric
                    || plane.iter().any(|&k| (c[k] - centre[k]).abs() > metric)
            }
            _ => true,
        }) {
            continue;
        }
        let mut angles: Vec<f64> = arcs
            .iter()
            .map(|e| {
                (e.mid[plane[1]] - centre[plane[1]])
                    .atan2(e.mid[plane[0]] - centre[plane[0]])
                    .rem_euclid(TAU)
            })
            .collect();
        angles.sort_by(f64::total_cmp);
        let n = angles.len();
        let expected = TAU / n as f64;
        if (0..n)
            .all(|i| ((angles[(i + 1) % n] - angles[i]).rem_euclid(TAU) - expected).abs() <= 1e-3)
        {
            best = Some(best.unwrap_or(0).max(n));
        }
    }
    best
}

fn issue(key: &str, severity: Severity, message: String, fix: &str) -> LintIssue {
    LintIssue {
        id: format!("l:unrecognised_defining_geometry:{key}"),
        code: "unrecognised_defining_geometry".into(),
        severity,
        message,
        at: None,
        view: None,
        subjects: Vec::new(),
        requirements: Vec::new(),
        measurements: Vec::new(),
        sources: Vec::new(),
        stage: None,
        annotation: None,
        related: Vec::new(),
        evidence: None,
        fix: Some(fix.into()),
    }
}

/// The principal profile findings: unsupported openings, then a toothed outer boundary.
pub fn coverage(analysis: &Analysis) -> Vec<LintIssue> {
    let sources = &analysis.principal_profiles;
    let severity = if sources.len() > 1 {
        Severity::Info
    } else {
        Severity::Warning
    };
    let mut issues = Vec::new();
    let unsupported: Vec<Verdict> = verdicts(analysis)
        .into_iter()
        .filter(|v| !v.supported)
        .collect();
    if !unsupported.is_empty() {
        let mut i = issue(
            "principal_profile",
            severity,
            "dimension-relevant source geometry is absent from recognised IR: unsupported \
             internal profile on a principal boundary face"
                .into(),
            "Define the opening's profile by hand: no feature of the drawing vocabulary states it.",
        );
        i.evidence = Some(format!(
            "{} opening(s) through principal faces are not a circle, rectangle, obround, rounded \
             rectangle, double-D bore mouth or section recess end",
            unsupported.len()
        ));
        issues.push(i);
    }
    if let Some(n) = sources.iter().filter_map(radial_count).max() {
        issues.push(issue(
            "outer_boundary",
            severity,
            format!(
                "dimension-relevant source geometry is absent from recognised IR: unsupported \
                 outer boundary on a principal face contains {n} evenly spaced common-circle arcs"
            ),
            "Define the outer boundary by hand (for a gear, its tooth data): no feature of the \
             drawing vocabulary states it.",
        ));
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: [f64; 2], b: [f64; 2]) -> Edge {
        let (s, e) = ([a[0], a[1], 0.0], [b[0], b[1], 0.0]);
        Edge {
            kind: Kind::Line,
            start: s,
            end: e,
            mid: [0, 1, 2].map(|k| (s[k] + e[k]) / 2.0),
            length: dist(s, e),
            bounds: [
                [0, 1, 2].map(|k| s[k].min(e[k])),
                [0, 1, 2].map(|k| s[k].max(e[k])),
            ],
            centre: None,
            radius: None,
        }
    }

    /// An arc about *c* of radius *r* from angle *a0* to *a1* (radians, counter-clockwise).
    fn arc(c: [f64; 2], r: f64, a0: f64, a1: f64) -> Edge {
        let at = |t: f64| [c[0] + r * t.cos(), c[1] + r * t.sin(), 0.0];
        let mut points = vec![at(a0), at(a1)];
        for k in 0..4 {
            let t = k as f64 * PI / 2.0;
            let t = t + ((a0 - t) / TAU).ceil() * TAU;
            if t <= a1 {
                points.push(at(t));
            }
        }
        let lo = [0, 1, 2].map(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
        let hi = [0, 1, 2].map(|k| {
            points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        Edge {
            kind: Kind::Circle,
            start: at(a0),
            end: at(a1),
            mid: at((a0 + a1) / 2.0),
            length: r * (a1 - a0),
            bounds: [lo, hi],
            centre: Some([c[0], c[1], 0.0]),
            radius: Some(r),
        }
    }

    fn wire(edges: Vec<Edge>) -> Wire {
        Wire {
            edges,
            vertices: Vec::new(),
        }
    }

    /// `_supported_inner_profile`'s vocabulary: a circle (one closed edge or two halves), an
    /// axis-aligned rectangle (sides split or not), a true obround and a rectangle with four equal
    /// quarter-circle corners are supported; an L-shaped loop with a rectangle's bounding box
    /// perimeter is not, nor is a double-D (its caps are not half the short extent), nor a
    /// rounded rectangle whose corners are not all one radius.
    #[test]
    fn the_vocabulary_is_circles_rectangles_obrounds_and_rounded_rectangles() {
        let tol = 1e-5;
        let plane = [0, 1];
        let ok = |edges: Vec<Edge>| vocabulary(&wire(edges), plane, tol);
        assert!(ok(vec![arc([0.0, 0.0], 3.0, 0.0, TAU)]));
        assert!(ok(vec![
            arc([0.0, 0.0], 3.0, 0.0, PI),
            arc([0.0, 0.0], 3.0, PI, TAU)
        ]));
        let rect = |pts: &[[f64; 2]]| {
            (0..pts.len())
                .map(|i| line(pts[i], pts[(i + 1) % pts.len()]))
                .collect::<Vec<_>>()
        };
        assert!(ok(rect(&[
            [0.0, 0.0],
            [20.0, 0.0],
            [20.0, 10.0],
            [0.0, 10.0]
        ])));
        assert!(ok(rect(&[
            [0.0, 0.0],
            [10.0, 0.0],
            [20.0, 0.0],
            [20.0, 10.0],
            [0.0, 10.0]
        ])));
        // An L: same box, same perimeter, one side off the box.
        assert!(!ok(rect(&[
            [0.0, 0.0],
            [20.0, 0.0],
            [20.0, 5.0],
            [10.0, 5.0],
            [10.0, 10.0],
            [0.0, 10.0]
        ])));
        // An obround 30 × 10: caps of radius 5 on centres 10 apart.
        let obround = vec![
            line([5.0, -5.0], [25.0, -5.0]),
            arc([25.0, 0.0], 5.0, -PI / 2.0, PI / 2.0),
            line([25.0, 5.0], [5.0, 5.0]),
            arc([5.0, 0.0], 5.0, PI / 2.0, 3.0 * PI / 2.0),
        ];
        assert!(ok(obround));
        // A double-D: Ø10 with flats 7.2 apart, caps of radius 5 on a 7.2 short extent.
        let h: f64 = 3.6;
        let a = (h / 5.0).asin();
        let x = (25.0 - h * h).sqrt();
        let double_d = vec![
            line([-x, -h], [x, -h]),
            arc([0.0, 0.0], 5.0, -a, a),
            line([x, h], [-x, h]),
            arc([0.0, 0.0], 5.0, PI - a, PI + a),
        ];
        assert!(!ok(double_d));
        // A 20 × 10 rectangle with R2 corners, and the same with one corner R3.
        let rounded = |r3: f64| {
            vec![
                line([2.0, 0.0], [18.0, 0.0]),
                arc([18.0, 2.0], 2.0, -PI / 2.0, 0.0),
                line([20.0, 2.0], [20.0, 8.0]),
                arc([18.0, 8.0], 2.0, 0.0, PI / 2.0),
                line([18.0, 10.0], [r3, 10.0]),
                arc([r3, 10.0 - r3], r3, PI / 2.0, PI),
                line([0.0, 10.0 - r3], [0.0, 2.0]),
                arc([2.0, 2.0], 2.0, PI, 3.0 * PI / 2.0),
            ]
        };
        assert!(ok(rounded(2.0)));
        assert!(!ok(rounded(3.0)));
    }

    /// `_radial_outer_arc_count`: six or more arcs of one circle, equal and evenly spaced, not
    /// every edge of the loop, are counted; uneven spacing, or a loop of arcs alone, is not.
    #[test]
    fn evenly_spaced_equal_arcs_on_one_circle_are_counted() {
        let teeth = |n: usize, skew: f64| -> Vec<Edge> {
            (0..n)
                .map(|i| {
                    let t = TAU * i as f64 / n as f64 + if i == 1 { skew } else { 0.0 };
                    arc([0.0, 0.0], 20.0, t, t + 0.2)
                })
                .collect()
        };
        let source = |arcs: Vec<Edge>, edges: usize| Source {
            boundary_segments: Vec::new(),
            curved_faces: Vec::new(),
            bounds: [[-20.0, -20.0, 0.0], [20.0, 20.0, 5.0]],
            faces: vec![Face {
                axis: 2,
                at: 5.0,
                inner: Vec::new(),
                outer_edges: edges,
                outer_arcs: arcs,
                outer_lines: Vec::new(),
                unsplit: false,
            }],
        };
        assert_eq!(radial_count(&source(teeth(12, 0.0), 36)), Some(12));
        assert_eq!(radial_count(&source(teeth(12, 0.05), 36)), None);
        assert_eq!(radial_count(&source(teeth(12, 0.0), 12)), None);
        assert_eq!(radial_count(&source(teeth(5, 0.0), 15)), None);
    }
}
