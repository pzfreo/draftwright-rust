//! Recognisable shapes among the faces quiddity leaves unexplained (#213): a protrusion, a cut,
//! a chamfer on a hole or shaft edge, a turned step or land, or a periodic tooth pattern. Read
//! from the part's topology when the Analysis is made; the plan warns of each one the Spec does
//! not state (`shape_not_recognised`), beside `unclaimed_geometry`'s count, congruent shapes in
//! one warning.

use std::collections::BTreeSet;

use haecceity::brep::{Arc, Part};
use haecceity::geom::{self, Surface, V3};
use serde::{Deserialize, Serialize};

use crate::model::{Analysis, FaceGroup, Problem, Spec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Faces standing out of the part: met by the rest along concave edges only.
    Protrusion,
    /// Faces of a recess: their own faces meet along concave edges.
    Cut,
    /// A cone on the edge of a coaxial cylinder, which no hole or turned feature states.
    Chamfer,
    /// Six or more congruent curved faces evenly spaced about one axis.
    Teeth,
    /// A shoulder plane with coaxial curved faces about its centre: a turned step or land.
    Step,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Protrusion => "protrusion",
            Kind::Cut => "cut",
            Kind::Chamfer => "chamfer",
            Kind::Teeth => "teeth",
            Kind::Step => "step",
        }
    }
}

/// A chamfer's axial length and angle to the axis (degrees), on a cylinder of *diameter*.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chamfer {
    pub diameter: f64,
    pub length: f64,
    pub angle: f64,
}

/// A periodic pattern's count and axis, in the part's frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Teeth {
    pub count: usize,
    pub centre: [f64; 3],
    pub axis: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub kind: Kind,
    /// Ascending file face indices.
    pub faces: Vec<usize>,
    /// The faces' box, mm, in the part's frame.
    pub bounds: [[f64; 3]; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chamfer: Option<Chamfer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teeth: Option<Teeth>,
    /// A step's shoulder diameters, the outer edge of each of its planes, ascending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shoulders: Vec<f64>,
}

const TOL: f64 = 1e-3;

fn parallel(a: V3, b: V3) -> bool {
    geom::norm(geom::cross(a, b)) <= TOL
}

/// The distance of *p* from the line through *o* along unit *d*.
fn off_axis(p: V3, o: V3, d: V3) -> f64 {
    let v = geom::sub(p, o);
    geom::norm(geom::sub(v, geom::scale(d, geom::dot(v, d))))
}

fn axis_of(s: &Surface) -> Option<(V3, V3)> {
    match s {
        Surface::Cylinder { frame, .. }
        | Surface::Cone { frame, .. }
        | Surface::Torus { frame, .. } => Some((frame.origin, frame.z)),
        _ => None,
    }
}

fn bounds(part: &Part, faces: &[usize]) -> [[f64; 3]; 2] {
    let mut b = geom::Bounds::empty();
    for &f in faces {
        b.merge(&part.face_bounds(f));
    }
    [b.min, b.max]
}

/// The shapes among *accounting*'s unexplained faces of *part*. Planar faces on the part's box
/// are its stock, never a shape's.
pub fn shapes(part: &Part, accounting: &[FaceGroup]) -> Vec<Shape> {
    let n = part.faces.len().min(accounting.len());
    let whole = part.bounds();
    let scale = (0..3).fold(1.0f64, |m, k| m.max(whole.max[k] - whole.min[k]));
    // On a side of the part's box, and spanning half of it one way: a tongue's end face on
    // the box is not stock.
    let stock = |i: usize| {
        matches!(part.faces[i].surface, Surface::Plane { .. }) && {
            let b = part.face_bounds(i);
            (0..3).any(|k| {
                b.max[k] - b.min[k] <= TOL
                    && ((b.min[k] - whole.min[k]).abs() <= TOL
                        || (b.max[k] - whole.max[k]).abs() <= TOL)
                    && (0..3).any(|j| {
                        j != k && b.max[j] - b.min[j] >= 0.5 * (whole.max[j] - whole.min[j])
                    })
            })
        }
    };
    let mut free: BTreeSet<usize> = (0..n)
        .filter(|&i| accounting[i] == FaceGroup::Unexplained && !stock(i))
        .collect();
    if free.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    // Chamfers: a cone meeting a coaxial cylinder at a convex edge, with the cone's other
    // coaxial halves.
    for &i in &free.clone() {
        let Surface::Cone { semi_angle, .. } = part.faces[i].surface else {
            continue;
        };
        if !free.contains(&i) {
            continue;
        }
        let (o, d) = axis_of(&part.faces[i].surface).expect("a cone has an axis");
        let coaxial = |j: usize| {
            axis_of(&part.faces[j].surface)
                .is_some_and(|(p, e)| parallel(d, e) && off_axis(p, o, d) <= TOL * scale)
        };
        let Some(radius) =
            part.neighbours(i)
                .into_iter()
                .find_map(|j| match part.faces[j].surface {
                    Surface::Cylinder { radius, .. }
                        if coaxial(j) && part.arc(i, j) == Some(Arc::Convex) =>
                    {
                        Some(radius)
                    }
                    _ => None,
                })
        else {
            continue;
        };
        let mut faces = vec![i];
        let mut k = 0;
        while k < faces.len() {
            for j in part.neighbours(faces[k]) {
                if free.contains(&j)
                    && !faces.contains(&j)
                    && matches!(part.faces[j].surface, Surface::Cone { semi_angle: s, .. } if (s - semi_angle).abs() <= TOL)
                    && coaxial(j)
                {
                    faces.push(j);
                }
            }
            k += 1;
        }
        let (lo, hi) = part.extent_along(&faces, d);
        let length = hi - lo;
        if !(length > TOL && length <= 2.0 * radius) {
            continue;
        }
        for f in &faces {
            free.remove(f);
        }
        faces.sort_unstable();
        out.push(Shape {
            kind: Kind::Chamfer,
            bounds: bounds(part, &faces),
            faces,
            chamfer: Some(Chamfer {
                diameter: 2.0 * radius,
                length,
                angle: semi_angle.abs().to_degrees(),
            }),
            teeth: None,
            shoulders: Vec::new(),
        });
    }
    out.extend(teeth(part, &mut free, scale));
    // Cuts: faces joined by concave edges.
    let concave = |a: usize, b: usize| part.arc(a, b) == Some(Arc::Concave);
    let convex = |a: usize, b: usize| part.arc(a, b) == Some(Arc::Convex);
    let mut protrusions: Vec<Vec<usize>> = Vec::new();
    for kind in [Kind::Cut, Kind::Protrusion] {
        if kind == Kind::Protrusion {
            out.extend(steps(part, &mut free, scale));
        }
        let mut seen = BTreeSet::new();
        for &seed in &free.clone() {
            if seen.contains(&seed) || !free.contains(&seed) {
                continue;
            }
            let mut faces = vec![seed];
            seen.insert(seed);
            let mut k = 0;
            while k < faces.len() {
                let a = faces[k];
                for b in part.neighbours(a) {
                    if free.contains(&b)
                        && !seen.contains(&b)
                        && (kind == Kind::Protrusion || concave(a, b))
                    {
                        seen.insert(b);
                        faces.push(b);
                    }
                }
                k += 1;
            }
            // Spanning most of the part two ways, it is the part's body, not a shape on it.
            let [lo, hi] = bounds(part, &faces);
            let spans = (0..3)
                .filter(|&k| hi[k] - lo[k] >= 0.75 * (whole.max[k] - whole.min[k]))
                .count();
            if faces.len() < 2 || spans >= 2 {
                continue;
            }
            // A protrusion stands on the rest of the part, met by it at a concave edge; its own
            // faces meet at none, as concave pairs are cuts by now.
            if kind == Kind::Protrusion
                && !faces.iter().any(|&a| {
                    part.neighbours(a)
                        .into_iter()
                        .any(|b| !faces.contains(&b) && concave(a, b))
                })
            {
                continue;
            }
            for f in &faces {
                free.remove(f);
            }
            if kind == Kind::Protrusion {
                protrusions.push(faces);
                continue;
            }
            faces.sort_unstable();
            out.push(Shape {
                kind,
                bounds: bounds(part, &faces),
                faces,
                chamfer: None,
                teeth: None,
                shoulders: Vec::new(),
            });
        }
    }
    // Protrusions that meet one claimed face at convex edges are one: a tab's two sides, either
    // side of its recognised wall. The face a protrusion stands on meets it at concave edges.
    let walls = |faces: &[usize]| -> BTreeSet<usize> {
        faces
            .iter()
            .flat_map(|&a| {
                part.neighbours(a).into_iter().filter(move |&c| {
                    accounting.get(c) == Some(&FaceGroup::Feature) && convex(a, c)
                })
            })
            .collect()
    };
    let mut k = 0;
    while k < protrusions.len() {
        let mine = walls(&protrusions[k]);
        match (k + 1..protrusions.len()).find(|&j| !mine.is_disjoint(&walls(&protrusions[j]))) {
            Some(j) => {
                let other = protrusions.remove(j);
                protrusions[k].extend(other);
            }
            None => k += 1,
        }
    }
    for mut faces in protrusions {
        faces.sort_unstable();
        out.push(Shape {
            kind: Kind::Protrusion,
            bounds: bounds(part, &faces),
            faces,
            chamfer: None,
            teeth: None,
            shoulders: Vec::new(),
        });
    }
    out.sort_by(|a, b| a.faces.cmp(&b.faces));
    out
}

/// Turned steps and lands: a free round shoulder plane about an axis, with the free coaxial
/// cylinders, cones and tori joined to it, whatever their edges. Read after the cuts and before
/// the protrusions, as a land's faces meet along tangent edges and may span the whole part.
fn steps(part: &Part, free: &mut BTreeSet<usize>, scale: f64) -> Vec<Shape> {
    let mut out = Vec::new();
    for &seed in &free.clone() {
        if !free.contains(&seed) {
            continue;
        }
        let Some((o, d)) = axis_of(&part.faces[seed].surface) else {
            continue;
        };
        let Some(d) = geom::unit(d) else { continue };
        let centre = |f: usize| {
            let b = part.face_bounds(f);
            geom::scale(geom::add(b.min, b.max), 0.5)
        };
        // A round plane's box is square across the axis, like the circle bounding it.
        let round = |j: usize| {
            let b = part.face_bounds(j);
            let w: Vec<f64> = (0..3)
                .filter(|&k| d[k].abs() < 1.0 - TOL)
                .map(|k| b.max[k] - b.min[k])
                .collect();
            w.len() < 2 || (w[0] - w[1]).abs() <= 0.01 * w[0].max(w[1])
        };
        let member = |j: usize| match &part.faces[j].surface {
            Surface::Plane { frame } => {
                parallel(frame.z, d) && off_axis(centre(j), o, d) <= TOL * scale && round(j)
            }
            s => {
                axis_of(s).is_some_and(|(p, e)| parallel(e, d) && off_axis(p, o, d) <= TOL * scale)
            }
        };
        let mut faces = vec![seed];
        let mut k = 0;
        while k < faces.len() {
            for j in part.neighbours(faces[k]) {
                if free.contains(&j) && !faces.contains(&j) && member(j) {
                    faces.push(j);
                }
            }
            k += 1;
        }
        // The coordinate direction most across the axis, to read a plane's outer radius.
        let across = (0..3)
            .min_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
            .unwrap_or(0);
        let mut shoulders: Vec<f64> = faces
            .iter()
            .filter(|&&f| matches!(part.faces[f].surface, Surface::Plane { .. }))
            .map(|&f| {
                let b = part.face_bounds(f);
                b.max[across] - b.min[across]
            })
            .collect();
        if faces.len() < 2 || shoulders.is_empty() {
            continue;
        }
        shoulders.sort_by(f64::total_cmp);
        for f in &faces {
            free.remove(f);
        }
        faces.sort_unstable();
        out.push(Shape {
            kind: Kind::Step,
            bounds: bounds(part, &faces),
            faces,
            chamfer: None,
            teeth: None,
            shoulders,
        });
    }
    out
}

/// *d* in one sense: its largest component positive.
fn sense(d: V3) -> V3 {
    let k = (0..3)
        .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
        .unwrap_or(2);
    if d[k] < 0.0 { geom::scale(d, -1.0) } else { d }
}

/// Classes of six or more congruent unexplained cylinders evenly spaced about one axis, each
/// with the free faces that touch them: a gear, sprocket or pulley's teeth.
fn teeth(part: &Part, free: &mut BTreeSet<usize>, scale: f64) -> Vec<Shape> {
    let key = |x: f64, step: f64| (x / step).round() as i64;
    let mut classes = std::collections::BTreeMap::<_, Vec<usize>>::new();
    for &i in free.iter() {
        let Surface::Cylinder { frame, radius } = &part.faces[i].surface else {
            continue;
        };
        let d = sense(frame.z);
        let (lo, hi) = part.extent_along(&[i], d);
        classes
            .entry((
                key(*radius, 0.01),
                d.map(|c| key(c, 0.01)),
                key(hi - lo, 0.05),
            ))
            .or_default()
            .push(i);
    }
    let mut out: Vec<Shape> = Vec::new();
    for ((_, _, _), faces) in classes {
        if faces.len() < 6 || faces.iter().any(|f| !free.contains(f)) {
            continue;
        }
        let Surface::Cylinder { frame, .. } = &part.faces[faces[0]].surface else {
            continue;
        };
        let Some(d) = geom::unit(sense(frame.z)) else {
            continue;
        };
        let flat = |p: V3| geom::sub(p, geom::scale(d, geom::dot(p, d)));
        let points: Vec<V3> = faces
            .iter()
            .map(|&f| match &part.faces[f].surface {
                Surface::Cylinder { frame, .. } => flat(frame.origin),
                _ => unreachable!("the class holds cylinders"),
            })
            .collect();
        let m = points.len() as f64;
        let centre = points
            .iter()
            .fold([0.0; 3], |c, p| geom::add(c, geom::scale(*p, 1.0 / m)));
        let radii: Vec<f64> = points
            .iter()
            .map(|p| geom::norm(geom::sub(*p, centre)))
            .collect();
        let r = radii.iter().sum::<f64>() / m;
        if r <= TOL * scale || radii.iter().any(|x| (x - r).abs() > 0.02 * r) {
            continue;
        }
        let (u, v) = {
            let a = geom::scale(geom::sub(points[0], centre), 1.0 / radii[0]);
            (a, geom::cross(d, a))
        };
        let mut angles: Vec<f64> = points
            .iter()
            .map(|p| {
                let q = geom::sub(*p, centre);
                geom::dot(q, v).atan2(geom::dot(q, u))
            })
            .collect();
        angles.sort_by(f64::total_cmp);
        let gaps: Vec<f64> = (0..angles.len())
            .map(|i| {
                let next = angles
                    .get(i + 1)
                    .copied()
                    .unwrap_or(angles[0] + std::f64::consts::TAU);
                next - angles[i]
            })
            .collect();
        let even = |step: usize| {
            (0..gaps.len()).all(|i| (gaps[i] - gaps[(i + step) % gaps.len()]).abs() <= 0.02)
        };
        let count = if even(1) {
            faces.len()
        } else if faces.len() % 2 == 0 && even(2) {
            faces.len() / 2
        } else {
            continue;
        };
        // The same axis as a pattern already found: its teeth's other faces.
        let axis_point = geom::add(centre, geom::scale(d, geom::dot(frame.origin, d)));
        if let Some(s) = out.iter_mut().find(|s| {
            s.teeth.as_ref().is_some_and(|t| {
                parallel(t.axis, d) && off_axis(axis_point, t.centre, t.axis) <= TOL * scale
            })
        }) {
            s.faces.extend(&faces);
            if let Some(t) = &mut s.teeth {
                t.count = t.count.max(count);
            }
        } else {
            out.push(Shape {
                kind: Kind::Teeth,
                faces,
                // Read below, once the pattern's faces are all gathered.
                bounds: [[0.0; 3]; 2],
                chamfer: None,
                teeth: Some(Teeth {
                    count,
                    centre: axis_point,
                    axis: d,
                }),
                shoulders: Vec::new(),
            });
        }
    }
    // The teeth's other free faces: those touching them, out from the axis as far as their
    // cylinders' centres nearly are (never a web or face across the axis).
    let centre_off = |f: usize, t: &Teeth| {
        let b = part.face_bounds(f);
        off_axis(geom::scale(geom::add(b.min, b.max), 0.5), t.centre, t.axis)
    };
    for s in &mut out {
        for f in &s.faces {
            free.remove(f);
        }
        let Some(t) = s.teeth.clone() else { continue };
        let root = s
            .faces
            .iter()
            .map(|&f| centre_off(f, &t))
            .fold(f64::INFINITY, f64::min);
        let mut k = 0;
        while k < s.faces.len() {
            for j in part.neighbours(s.faces[k]) {
                if free.contains(&j) && centre_off(j, &t) >= 0.8 * root {
                    free.remove(&j);
                    s.faces.push(j);
                }
            }
            k += 1;
        }
        s.faces.sort_unstable();
        s.bounds = bounds(part, &s.faces);
    }
    out
}

fn num(x: f64) -> String {
    let s = format!("{:.2}", x + 0.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// The faces by index, the first dozen of a longer list (the subjects name them all).
fn faces_text(faces: &[usize]) -> String {
    let list = faces
        .iter()
        .take(12)
        .map(|f| f.to_string())
        .collect::<Vec<_>>();
    match faces.len() {
        1 => format!("Face {}", list[0]),
        n if n > 12 => format!("{n} faces ({}, …)", list.join(", ")),
        _ => format!("Faces {}", list.join(", ")),
    }
}

const UNSIZED: [&str; 2] = [
    "recognition does not give it, so it is neither sized nor located",
    "recognition does not give them, so they are neither sized nor located",
];

/// What a shape is, without its faces or place, and what its warning says is missing, of one
/// shape and of several. Congruent shapes describe alike.
fn describe(s: &Shape) -> (String, [&'static str; 2]) {
    let [lo, hi] = s.bounds;
    let size = format!(
        "{} × {} × {} mm",
        num(hi[0] - lo[0]),
        num(hi[1] - lo[1]),
        num(hi[2] - lo[2])
    );
    match (s.kind, &s.chamfer, &s.teeth) {
        (Kind::Chamfer, Some(c), _) => (
            format!(
                "a {} × {}° chamfer on the edge of a ø{} cylinder",
                num(c.length),
                num(c.angle),
                num(c.diameter)
            ),
            [
                "no hole callout or chamfer dimension states it",
                "no hole callout or chamfer dimension states them",
            ],
        ),
        (Kind::Teeth, _, Some(t)) => (
            format!(
                "a periodic pattern of {} teeth or notches about the axis through ({}, {}, {}) along ({}, {}, {}), {size}",
                t.count,
                num(t.centre[0]),
                num(t.centre[1]),
                num(t.centre[2]),
                num(t.axis[0]),
                num(t.axis[1]),
                num(t.axis[2])
            ),
            ["no tooth form or count is drawn"; 2],
        ),
        (Kind::Step, _, _) => (
            format!(
                "a turned step or land, its {} ø{}, {size}",
                if s.shoulders.len() == 1 {
                    "shoulder"
                } else {
                    "shoulders"
                },
                s.shoulders
                    .iter()
                    .map(|d| num(*d))
                    .collect::<Vec<_>>()
                    .join(", ø")
            ),
            UNSIZED,
        ),
        (kind, _, _) => (format!("a {} of {size}", kind.name()), UNSIZED),
    }
}

/// A warning for each class of congruent shapes of *analysis* that *spec* does not state: none
/// of a shape's faces is *claimed* by a feature (a feature declaration's included) or carries
/// the Spec's leader note or feature control frame (`faces`), and no authored gear lies on a
/// tooth pattern's axis.
pub fn problems(analysis: &Analysis, spec: &Spec, claimed: &BTreeSet<usize>) -> Vec<Problem> {
    let mut stated = claimed.clone();
    for (key, f) in &spec.faces {
        if f.note.is_some() || !f.controls.is_empty() {
            stated.extend(crate::spec_gdt::named_faces(analysis, key).unwrap_or_default());
        }
    }
    let geared = |t: &Teeth| {
        spec.gears.iter().any(|g| {
            let k = ["x", "y", "z"].iter().position(|a| *a == g.frame.axis);
            k.is_some_and(|k| {
                let mut d = [0.0; 3];
                d[k] = 1.0;
                parallel(d, t.axis) && off_axis(t.centre, g.frame.origin, d) <= 0.01
            })
        })
    };
    let mut classes: Vec<((Kind, String), Vec<&Shape>)> = Vec::new();
    for s in analysis.unexplained_shapes.iter().filter(|s| {
        !s.faces.iter().any(|f| stated.contains(f)) && !s.teeth.as_ref().is_some_and(geared)
    }) {
        let key = (s.kind, describe(s).0);
        match classes.iter_mut().find(|(k, _)| *k == key) {
            Some((_, members)) => members.push(s),
            None => classes.push((key, vec![s])),
        }
    }
    classes
        .into_iter()
        .map(|((kind, what), members)| {
            let first = members[0];
            let [lo, hi] = first.bounds;
            let at = format!(
                "({}, {}, {})",
                num((lo[0] + hi[0]) / 2.0),
                num((lo[1] + hi[1]) / 2.0),
                num((lo[2] + hi[2]) / 2.0)
            );
            let all: Vec<usize> = members.iter().flat_map(|s| s.faces.iter().copied()).collect();
            let faces = faces_text(&all);
            let (_, [one, many]) = describe(first);
            let shape = if members.len() == 1 {
                let verb = match (kind, all.len()) {
                    (Kind::Chamfer, 1) => "is",
                    (Kind::Chamfer, _) => "are",
                    _ => "form",
                };
                format!("{faces} {verb} {what} at {at}; {one}")
            } else {
                format!(
                    "{faces} form {} congruent shapes, each {what}, the first at {at}; {many}",
                    members.len()
                )
            };
            let remedy = match kind {
                Kind::Teeth => {
                    "State the gear in the Spec's gears, which withdraws this warning for a gear on this axis, or add a note on one of its faces in the Spec's faces."
                }
                Kind::Chamfer => {
                    "State it with a note on one of its faces in the Spec's faces (its size, as a chamfer callout), which withdraws this warning; otherwise review it manually."
                }
                _ => {
                    "If it is a hole, boss, turned step, slot or pocket, declare it in the Spec's feature_declarations using its face fingerprints; otherwise state it with a note or feature control frame on one of its faces in the Spec's faces, which withdraws this warning."
                }
            };
            Problem {
                id: format!("p:shape_not_recognised:face:{}", first.faces[0]),
                code: "shape_not_recognised".into(),
                severity: "warning".into(),
                message: format!("{shape}. {remedy}"),
                subjects: all.iter().map(|f| format!("face:{f}")).collect(),
                refs: vec![format!("shape:{}", kind.name())],
                ..Default::default()
            }
        })
        .collect()
}
