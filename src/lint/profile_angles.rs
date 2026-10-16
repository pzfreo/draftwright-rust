//! Non-right convex corners on the foremost camera-facing outer profile of each body.
//! Python's profile_angles.py: supplied ordered supports establish adjacency; inner wires,
//! back faces, concave and right corners never issue automatic requirements.
use super::{LintIssue, Mark, holes};
use crate::model::Analysis;
use serde::{Deserialize, Serialize};

type P3 = [f64; 3];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Support {
    Line {
        start: P3,
        end: P3,
    },
    Arc {
        start: P3,
        end: P3,
        center: P3,
        radius: f64,
        sweep: f64,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub origin: P3,
    pub normal: P3,
    pub supports: Vec<Support>,
    pub inner_loop_count: usize,
    pub schema_version: u8,
    pub boundary_kind: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub face: usize,
    pub body_faces: Vec<usize>,
    pub edges: Vec<usize>,
    pub profile: Profile,
}
/// Keep the provider's evidence as geometry, independent of recognition feature conversion.
pub fn evidence(part: &haecceity::Part) -> Vec<Source> {
    (0..part.faces.len())
        .filter_map(|face| {
            let source =
                quiddity::features::outer_profile::planar_outer_profile(part, face).ok()?;
            Some(Source {
                face,
                body_faces: source.body_faces,
                edges: source.edges,
                profile: serde_json::from_value(
                    serde_json::to_value(source.profile).expect("profile serializes"),
                )
                .expect("provider profile schema"),
            })
        })
        .collect()
}
fn sub(a: P3, b: P3) -> P3 {
    [0, 1, 2].map(|i| a[i] - b[i])
}
fn dot(a: P3, b: P3) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P3) -> P3 {
    let n = norm(a);
    a.map(|x| x / n)
}
#[derive(Clone, Debug)]
pub struct Corner {
    pub feature: String,
    pub axis: usize,
    pub vertex: P3,
    pub first: P3,
    pub second: P3,
    pub virtual_vertex: bool,
}
impl Corner {
    pub fn degrees(&self) -> f64 {
        let (u, v) = (
            unit(sub(self.first, self.vertex)),
            unit(sub(self.second, self.vertex)),
        );
        norm(cross(u, v)).atan2(dot(u, v)).to_degrees()
    }
}
/// Same-body feature definitions use exact source edge/face identity, never matching values.
fn bevel_edges(a: &Analysis, s: &Source) -> (bool, Vec<usize>) {
    let mut bevels = Vec::new();
    for f in &a.features {
        let polygon = matches!(f.kind.as_str(), "polygonal_stock" | "polygonal_boss");
        if !(polygon || f.kind == "chamfer")
            || f.faces.is_empty()
            || !f.faces.iter().all(|i| s.body_faces.contains(i))
        {
            continue;
        }
        if f.parameters.get("turned").and_then(|v| v.as_bool()) == Some(true) {
            continue;
        }
        let edges: Vec<_> = a
            .outer_profiles
            .iter()
            .filter(|p| f.faces.contains(&p.face))
            .flat_map(|p| p.edges.iter().copied())
            .collect();
        let all_lines = s
            .profile
            .supports
            .iter()
            .all(|s| matches!(s, Support::Line { .. }));
        let count = f.parameters.get("side_count").and_then(|v| v.as_u64());
        if polygon
            && all_lines
            && count == Some(s.edges.len() as u64)
            && (f.faces.contains(&s.face) || s.edges.iter().all(|e| edges.contains(e)))
        {
            return (true, Vec::new());
        }
        if !polygon {
            bevels.extend(edges);
        }
    }
    (false, bevels)
}
pub fn requirements(a: &Analysis) -> Vec<Corner> {
    let candidates: Vec<_> = a
        .outer_profiles
        .iter()
        .filter_map(|s| {
            let p = &s.profile;
            if ![1, 2].contains(&p.schema_version) || p.boundary_kind != "outer" {
                return None;
            }
            let axis = (0..3).max_by(|&i, &j| p.normal[i].abs().total_cmp(&p.normal[j].abs()))?;
            let side = if axis == 1 { -1. } else { 1. };
            (side * p.normal[axis] >= 1. - 1e-8
                && (0..3).all(|i| i == axis || p.normal[i].abs() <= 1e-8))
            .then_some((s, axis, side))
        })
        .collect();
    let mut out = Vec::new();
    for &(s, axis, side) in &candidates {
        if candidates.iter().any(|&(other, k, _)| {
            k == axis
                && other.body_faces == s.body_faces
                && side * (other.profile.origin[axis] - s.profile.origin[axis]) > 1e-6
        }) {
            continue;
        }
        let (polygon, bevels) = bevel_edges(a, s);
        if polygon {
            continue;
        }
        let p = &s.profile;
        let n = p.supports.len();
        for (i, first) in p.supports.iter().enumerate() {
            let Support::Line { start: a0, end: a1 } = first else {
                continue;
            };
            let mut j = (i + 1) % n;
            let rounded = matches!(p.supports[j], Support::Arc { .. });
            let transition = &p.supports[j];
            if rounded {
                j = (j + 1) % n;
            }
            let Support::Line { start: b0, end: b1 } = p.supports[j] else {
                continue;
            };
            let (u, v) = (unit(sub(*a1, *a0)), unit(sub(b1, b0)));
            let turn = dot(cross(u, v), p.normal);
            if turn <= 1e-8 || dot(u, v).abs() <= 1e-8 {
                continue;
            }
            let distance = dot(cross(sub(b0, *a1), v), p.normal) / turn;
            let vertex = [0, 1, 2].map(|k| a1[k] + distance * u[k]);
            let along = dot(sub(vertex, b0), v);
            let rebuilt = [0, 1, 2].map(|k| b0[k] + along * v[k]);
            if norm(sub(vertex, rebuilt)) > 1e-6 {
                continue;
            }
            let (first, second) = if let Support::Arc { center, .. } = transition {
                if distance <= 1e-6
                    || along >= -1e-6
                    || dot(u, sub(*a1, *center)).abs() > 1e-6
                    || dot(v, sub(b0, *center)).abs() > 1e-6
                {
                    continue;
                }
                (*a1, b0)
            } else {
                if norm(sub(vertex, *a1)) > 1e-6 || *a1 != b0 {
                    continue;
                }
                if [i, j]
                    .iter()
                    .any(|&i| s.edges.get(i).is_some_and(|e| bevels.contains(e)))
                {
                    continue;
                }
                (*a0, b1)
            };
            out.push(Corner {
                feature: format!("outer_profile:f{}:e{}:e{}", s.face, s.edges[i], s.edges[j]),
                axis,
                vertex,
                first,
                second,
                virtual_vertex: rounded,
            });
        }
    }
    out
}
/// Require live, role-bound, degree-bearing ink for every issued corner.
pub fn coverage(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    requirements(a)
        .iter()
        .filter_map(|c| {
            let claimed: Vec<_> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|cl| cl.feature == c.feature && cl.parameter == "profile_angle.angle")
                })
                .collect();
            let degrees = c.degrees();
            if claimed.iter().any(|m| {
                m.text
                    .strip_suffix('°')
                    .is_some_and(|t| holes::states(t, degrees))
            }) {
                return None;
            }
            let code = if claimed.is_empty() {
                "angular_requirement_missing"
            } else {
                "angular_requirement_unverifiable"
            };
            let mut finding = super::uncovered(
                a,
                code,
                &c.feature,
                "profile_angle.angle",
                format!(
                    "Outer-profile corner {} requires {:.1}° on the drawing.",
                    c.feature, degrees
                ),
            );
            finding.annotation = claimed.first().map(|m| m.name.clone());
            Some(finding)
        })
        .collect()
}
