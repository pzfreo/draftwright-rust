//! Symmetric V recesses from ordered provider supports. This is drafting evidence,
//! not a new recognition occurrence. Rounded roots use the theoretical sharp apex.
use super::profile_angles::{Corner, Source, Support};
use crate::model::Analysis;
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|k| a[k] - b[k])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot(a, a).sqrt();
    (n.is_finite() && n > 1e-10).then(|| a.map(|v| v * (1. / n)))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[derive(Clone, Debug)]
pub(crate) struct Notch {
    pub id: String,
    pub angle: Corner,
    pub mouth: [f64; 3],
    pub axis: usize,
    pub depth_axis: usize,
    pub width_axis: usize,
    pub faces: Vec<usize>,
    pub body: Vec<usize>,
}
impl Notch {
    pub fn depth(&self) -> f64 {
        (self.angle.vertex[self.depth_axis] - self.mouth[self.depth_axis]).abs()
    }
}
/// Source PMI angles on these exact flank faces may state the same normal-section
/// angle at another height. Face identity prevents credit across overlapping bodies.
pub(crate) fn pmi_angles(a: &Analysis, n: &Notch) -> Vec<String> {
    let Some(crate::pmi::Reading::Read(report)) = &a.pmi else {
        return Vec::new();
    };
    let ids = report.identities();
    report
        .records
        .iter()
        .filter_map(|r| {
            if r.category != crate::pmi::Category::Dimension
                || r.kind != "angular"
                || r.faces.len() != 2
                || !r.faces.iter().all(|f| n.faces.contains(f))
                || !r
                    .value
                    .is_some_and(|v| (v - n.angle.degrees()).abs() < 1e-6)
            {
                return None;
            }
            let g = r.geometry.as_ref()?;
            if !g.reasons.is_empty() {
                return None;
            }
            let p = g.angle.as_ref()?;
            let rays = [
                unit(sub(p.first, p.vertex))?,
                unit(sub(p.second, p.vertex))?,
            ];
            let normal = unit(cross(rays[0], rays[1]))?;
            if normal[n.axis].abs() < 1.0 - 1e-6
                || (0..3).any(|k| k != n.axis && (p.vertex[k] - n.angle.vertex[k]).abs() > 1e-6)
                || (dot(cross(rays[0], rays[1]), cross(rays[0], rays[1]))
                    .sqrt()
                    .atan2(dot(rays[0], rays[1]))
                    .to_degrees()
                    - n.angle.degrees())
                .abs()
                    > 1e-6
            {
                return None;
            }
            let own = [
                unit(sub(n.angle.first, n.angle.vertex))?,
                unit(sub(n.angle.second, n.angle.vertex))?,
            ];
            if !own
                .iter()
                .all(|u| rays.iter().any(|v| dot(*u, *v).abs() > 1.0 - 1e-6))
            {
                return None;
            }
            ids.id(&r.source_id).ok().map(str::to_owned)
        })
        .collect()
}

/// An existing unequal-bevel callout may already state the notch's depth as its
/// first support leg. Require its physical flank face, mouth-end virtual corner,
/// normal run, both true legs and half-angle. Provider legs/corners have three
/// decimal places and angles two; these bounds account only for that publication.
pub(crate) fn bevel_depths(a: &Analysis, n: &Notch) -> Vec<String> {
    let frame = crate::bridge::holes::frame(a);
    let principal = |v: [f64; 3], k: usize| unit(v).is_some_and(|u| u[k].abs() > 1.0 - 1e-6);
    a.features
        .iter()
        .filter_map(|f| {
            if !matches!(f.kind.as_str(), "chamfer" | "oriented_chamfer")
                || f.faces.is_empty()
                || !f.faces.iter().all(|i| n.faces.contains(i))
                || f.parameters
                    .get("turned")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            {
                return None;
            }
            let p = &f.parameters;
            let (l1, l2, angle) = (
                super::parameters::size(f, "leg1").ok()?,
                super::parameters::size(f, "leg2").ok()?,
                super::parameters::size(f, "angle").ok()?,
            );
            if ![l1, l2, angle].iter().all(|v| v.is_finite() && *v > 0.0) {
                return None;
            }
            if super::chamfers::equal_45(l1, l2, angle) {
                return None;
            }
            let oriented = f.kind == "oriented_chamfer";
            let run =
                super::parameters::direction(&frame, p.get(if oriented { "run" } else { "axis" }))?;
            if !principal(run, n.axis) {
                return None;
            }
            if oriented {
                super::oriented_chamfers::read(a, f).ok()?;
                if !principal(
                    super::parameters::direction(&frame, p.get("leg1_direction"))?,
                    n.depth_axis,
                ) || !principal(
                    super::parameters::direction(&frame, p.get("leg2_direction"))?,
                    n.width_axis,
                ) {
                    return None;
                }
            } else if ![n.depth_axis, n.width_axis]
                .iter()
                .all(|&k| frame[1..].iter().any(|&v| principal(v, k)))
            {
                return None;
            }
            let corner = super::parameters::place(&frame, p.get("corner"))?;
            let mouth = [n.angle.first, n.angle.second].into_iter().find(|c| {
                [n.depth_axis, n.width_axis]
                    .iter()
                    .all(|&k| (c[k] - corner[k]).abs() <= 0.00050001)
            })?;
            let other_leg = (n.angle.vertex[n.width_axis] - mouth[n.width_axis]).abs();
            ((l1 - n.depth()).abs() <= 0.00050001
                && (l2 - other_leg).abs() <= 0.00050001
                && (angle - n.angle.degrees() / 2.0).abs() <= 0.00500001)
                .then(|| f.id.clone())
        })
        .collect()
}

fn line(s: &Support) -> Option<[[f64; 3]; 2]> {
    if let Support::Line { start, end } = s {
        Some([*start, *end])
    } else {
        None
    }
}
pub(crate) fn from_profile(a: &Analysis, s: &Source) -> Vec<Notch> {
    let p = &s.profile;
    if ![1, 2].contains(&p.schema_version) || p.boundary_kind != "outer" {
        return vec![];
    }
    let Some(axis) = (0..3).find(|&k| p.normal[k].abs() > 1. - 1e-8) else {
        return vec![];
    };
    let facing = if axis == 1 { -1. } else { 1. };
    if p.normal[axis] * facing < 1. - 1e-8 {
        return vec![];
    }
    let n = p.supports.len();
    let mut out = Vec::new();
    for i in 0..n {
        let Some([a0, a1]) = line(&p.supports[i]) else {
            continue;
        };
        let next = (i + 1) % n;
        let rounded = matches!(p.supports[next], Support::Arc { .. });
        let j = (i + 1 + usize::from(rounded)) % n;
        let Some([b0, b1]) = line(&p.supports[j]) else {
            continue;
        };
        let (Some(u), Some(v)) = (unit(sub(a1, a0)), unit(sub(b1, b0))) else {
            continue;
        };
        let turn = dot(cross(u, v), p.normal);
        if turn >= -1e-8 || dot(u, v).abs() < 1e-8 {
            continue;
        }
        let distance = dot(cross(sub(b0, a1), v), p.normal) / turn;
        let vertex = [0, 1, 2].map(|k| a1[k] + distance * u[k]);
        let along = dot(sub(vertex, b0), v);
        let rebuilt = [0, 1, 2].map(|k| b0[k] + along * v[k]);
        if dot(sub(vertex, rebuilt), sub(vertex, rebuilt)) > 1e-10 {
            continue;
        }
        if rounded {
            let Support::Arc { center, sweep, .. } = p.supports[next] else {
                // Invariant: rounded is set only when this support matched Support::Arc.
                unreachable!()
            };
            if distance <= 1e-6
                || along >= -1e-6
                || sweep >= 0.
                || dot(u, sub(a1, center)).abs() > 1e-6
                || dot(v, sub(b0, center)).abs() > 1e-6
            {
                continue;
            }
        } else if dot(sub(a1, b0), sub(a1, b0)) > 1e-10 {
            continue;
        }
        let mouth = [0, 1, 2].map(|k| 0.5 * (a0[k] + b1[k]));
        // A single principal mouth, symmetric about a principal depth line.
        let Some(width_axis) = (0..3).find(|&k| {
            k != axis
                && (a0[k] - b1[k]).abs() > 1e-6
                && (0..3).all(|j| j == k || (a0[j] - b1[j]).abs() < 1e-6)
        }) else {
            continue;
        };
        // Invariant: axis and width_axis are distinct indices from 0..3, leaving one depth axis.
        let depth_axis = (0..3).find(|&k| k != axis && k != width_axis).unwrap();
        if (vertex[width_axis] - mouth[width_axis]).abs() > 1e-6
            || (vertex[depth_axis] - mouth[depth_axis]).abs() < 1e-6
        {
            continue;
        }
        // The flanking supports establish the mouth datum; arbitrary sloping pairs do not.
        let before = line(&p.supports[(i + n - 1) % n]);
        let after = line(&p.supports[(j + 1) % n]);
        if ![before, after].into_iter().all(|l| {
            l.is_some_and(|[x, y]| {
                (x[depth_axis] - mouth[depth_axis]).abs() < 1e-6
                    && (y[depth_axis] - mouth[depth_axis]).abs() < 1e-6
            })
        }) {
            continue;
        }
        let Some(normal) = unit(cross(sub(a0, vertex), sub(b1, vertex))) else {
            continue;
        };
        if (0..3).any(|k| k != axis && normal[k].abs() > 1e-6) {
            continue;
        }
        let edges = [s.edges[i], s.edges[j]];
        let id = format!("notch:f{}:e{}:e{}", s.face, edges[0], edges[1]);
        let angle = Corner {
            feature: id.clone(),
            axis,
            vertex,
            first: a0,
            second: b1,
            virtual_vertex: rounded,
            ends: [a0, b1],
        };
        let faces = a
            .outer_profiles
            .iter()
            .filter(|f| {
                f.face != s.face
                    && f.body_faces == s.body_faces
                    && f.edges.iter().any(|e| edges.contains(e))
            })
            .map(|f| f.face)
            .collect();
        out.push(Notch {
            id,
            angle,
            mouth,
            axis,
            depth_axis,
            width_axis,
            faces,
            body: s.body_faces.clone(),
        });
    }
    out
}
/// The profile V-notch, by ID, whose flanks include V channel *f*'s (#233): its angle and depth
/// state the channel.
pub(crate) fn channel_owner(notches: &[Notch], f: &crate::bridge::Feature) -> Option<String> {
    notches
        .iter()
        .find(|n| f.faces.iter().all(|i| n.faces.contains(i)))
        .map(|n| n.id.clone())
}

pub(crate) fn requirements(a: &Analysis) -> Vec<Notch> {
    let mut out: Vec<Notch> = Vec::new();
    for notch in a.outer_profiles.iter().flat_map(|s| from_profile(a, s)) {
        // A notch on two caps is one requirement. Keep the foremost physical profile.
        let same = out.iter().position(|n| {
            n.axis == notch.axis
                && n.body == notch.body
                && (0..3)
                    .filter(|&k| k != n.axis)
                    .all(|k| (n.angle.vertex[k] - notch.angle.vertex[k]).abs() < 1e-6)
                && (n.depth() - notch.depth()).abs() < 1e-6
                && (n.angle.degrees() - notch.angle.degrees()).abs() < 1e-6
        });
        if let Some(i) = same {
            let side = if notch.axis == 1 { -1. } else { 1. };
            if side * (notch.angle.vertex[notch.axis] - out[i].angle.vertex[notch.axis]) > 0. {
                out[i] = notch;
            }
        } else {
            out.push(notch);
        }
    }
    out
}
