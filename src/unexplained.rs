//! Recognisable shapes among the faces quiddity leaves unexplained (#213): a protrusion, a cut,
//! a chamfer on a hole or shaft edge, or a periodic tooth pattern. Read from the part's topology
//! when the Analysis is made; the plan warns of each one whose faces no feature claims
//! (`shape_not_recognised`), beside `unclaimed_geometry`'s count.

use std::collections::BTreeSet;

use haecceity::brep::{Arc, Part};
use haecceity::geom::{self, Surface, V3};
use serde::{Deserialize, Serialize};

use crate::model::{Analysis, FaceGroup, Problem};

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
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Protrusion => "protrusion",
            Kind::Cut => "cut",
            Kind::Chamfer => "chamfer",
            Kind::Teeth => "teeth",
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
        Surface::Cylinder { frame, .. } | Surface::Cone { frame, .. } => {
            Some((frame.origin, frame.z))
        }
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
        });
    }
    out.extend(teeth(part, &mut free, scale));
    // Cuts: faces joined by concave edges.
    let concave = |a: usize, b: usize| part.arc(a, b) == Some(Arc::Concave);
    for kind in [Kind::Cut, Kind::Protrusion] {
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
            faces.sort_unstable();
            for f in &faces {
                free.remove(f);
            }
            out.push(Shape {
                kind,
                bounds: bounds(part, &faces),
                faces,
                chamfer: None,
                teeth: None,
            });
        }
    }
    out.sort_by(|a, b| a.faces.cmp(&b.faces));
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
                bounds: [[0.0; 3]; 2],
                chamfer: None,
                teeth: Some(Teeth {
                    count,
                    centre: axis_point,
                    axis: d,
                }),
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

/// A warning for each of *analysis*'s shapes none of whose faces *claimed* holds.
pub fn problems(analysis: &Analysis, claimed: &BTreeSet<usize>) -> Vec<Problem> {
    analysis
        .unexplained_shapes
        .iter()
        .filter(|s| !s.faces.iter().any(|f| claimed.contains(f)))
        .map(|s| {
            let [lo, hi] = s.bounds;
            let at = format!(
                "({}, {}, {})",
                num((lo[0] + hi[0]) / 2.0),
                num((lo[1] + hi[1]) / 2.0),
                num((lo[2] + hi[2]) / 2.0)
            );
            let size = format!(
                "{} × {} × {} mm",
                num(hi[0] - lo[0]),
                num(hi[1] - lo[1]),
                num(hi[2] - lo[2])
            );
            let faces = faces_text(&s.faces);
            let what = match (s.kind, &s.chamfer, &s.teeth) {
                (Kind::Chamfer, Some(c), _) => format!(
                    "{faces} {} a {} × {}° chamfer on the edge of a ø{} cylinder at {at}, which no hole callout or chamfer dimension states",
                    if s.faces.len() == 1 { "is" } else { "are" },
                    num(c.length),
                    num(c.angle),
                    num(c.diameter)
                ),
                (Kind::Teeth, _, Some(t)) => format!(
                    "{faces} form a periodic pattern of {} teeth or notches about the axis through ({}, {}, {}) along ({}, {}, {}), {size}; no tooth form or count is drawn",
                    t.count,
                    num(t.centre[0]),
                    num(t.centre[1]),
                    num(t.centre[2]),
                    num(t.axis[0]),
                    num(t.axis[1]),
                    num(t.axis[2])
                ),
                (kind, _, _) => format!(
                    "{faces} form a {} of {size} at {at} that recognition does not give; it is neither sized nor located",
                    kind.name()
                ),
            };
            Problem {
                id: format!("p:shape_not_recognised:face:{}", s.faces[0]),
                code: "shape_not_recognised".into(),
                severity: "warning".into(),
                message: format!(
                    "{what}. If it matches a supported feature type, declare it in the Spec's feature_declarations using its face fingerprints; otherwise add its dimensions and review it manually."
                ),
                subjects: s.faces.iter().map(|f| format!("face:{f}")).collect(),
                refs: vec![format!("shape:{}", s.kind.name())],
                ..Default::default()
            }
        })
        .collect()
}
