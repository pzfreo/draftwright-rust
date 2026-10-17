//! Symmetric V recesses from ordered provider supports. This is drafting evidence,
//! not a new recognition occurrence. Rounded roots use the theoretical sharp apex.
use crate::model::dimension_geometry::{dot, sub, unit};
use crate::{
    dims::angular::Angle,
    lint::profile_angles::{Source, Support},
    model::Analysis,
};

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
    pub angle: Angle,
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
fn line(s: &Support) -> Option<[[f64; 3]; 2]> {
    if let Support::Line { start, end } = s {
        Some([*start, *end])
    } else {
        None
    }
}
fn from_profile(a: &Analysis, s: &Source) -> Vec<Notch> {
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
        let angle = Angle {
            vertex,
            first: a0,
            second: b1,
        };
        if angle.principal() != Some(axis) {
            continue;
        }
        let edges = [s.edges[i], s.edges[j]];
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
            id: format!("notch:f{}:e{}:e{}", s.face, edges[0], edges[1]),
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
pub(crate) fn requirements(a: &Analysis) -> Vec<Notch> {
    crate::planning::memo(a, "v_notches", Vec::new(), || requirements_uncached(a))
}

fn requirements_uncached(a: &Analysis) -> Vec<Notch> {
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
                && (n.angle.degrees().unwrap() - notch.angle.degrees().unwrap()).abs() < 1e-6
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
