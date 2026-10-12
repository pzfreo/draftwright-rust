//! Blend ownership and exact-face attachment evidence, shared by planning and coverage.
use std::collections::BTreeMap;

use crate::bridge::Feature;
use crate::haecceity::Part;
use crate::haecceity::geom::Surface;

/// Proven surface points of each blend, in the part frame. The rolling path is not a surface
/// point. Sample only its own trimmed cylinder/torus, with a matching rolling radius.
pub fn evidence(part: &Part, features: &[Feature]) -> BTreeMap<String, Vec<[f64; 3]>> {
    let mut out = BTreeMap::new();
    for f in features.iter().filter(|f| f.kind == "blend") {
        let Some(r) = f.parameters.get("radius").and_then(|v| v.as_f64()) else {
            continue;
        };
        let mut sites = Vec::new();
        for &face in &f.faces {
            let Some(face_data) = part.faces.get(face) else {
                continue;
            };
            let radius = match &face_data.surface {
                Surface::Cylinder { radius, .. } => *radius,
                Surface::Torus { minor, .. } => *minor,
                _ => continue,
            };
            if (radius - r).abs() > 0.002 {
                continue;
            }
            let (Some(domain), Some((u0, u1, v0, v1))) = (part.domain(face), part.uv_bounds(face))
            else {
                continue;
            };
            if ![u0, u1, v0, v1].iter().all(|x| x.is_finite()) {
                continue;
            }
            for u in [0.5, 0.25, 0.75] {
                for v in [0.5, 0.25, 0.75] {
                    let (u, v) = (u0 + u * (u1 - u0), v0 + v * (v1 - v0));
                    if domain.contains(u, v) {
                        sites.push(face_data.surface.value(u, v));
                    }
                }
            }
        }
        if !sites.is_empty() {
            out.insert(f.id.clone(), sites);
        }
    }
    out
}

/// A grouped shallow relief, proved from topology rather than a feature or part name.
/// Its profiles are governed by the model, not by the drawing's default corner radius.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relief {
    pub carrier: usize,
    pub floors: Vec<usize>,
    pub faces: Vec<usize>,
    pub depth: f64,
}

/// Complex, tiny floors bounded by short walls ending on one broad, parallel carrier.
/// Require a group: a lone shallow pocket, a simple rounded rectangle, a deep recess or
/// a region with a large wall remains a manufacturing radius requirement.
pub fn reliefs(part: &Part) -> Vec<Relief> {
    use crate::haecceity::geom::{dot, sub};
    if part.solids.len() != 1 {
        return Vec::new();
    }
    use std::collections::BTreeSet;
    let bounds = part.bounds();
    let spans = [0, 1, 2].map(|k| bounds.max[k] - bounds.min[k]);
    let longest = spans.into_iter().fold(0.0_f64, f64::max);
    let area = (spans[0] * spans[1])
        .max(spans[0] * spans[2])
        .max(spans[1] * spans[2]);
    let tol = (longest * 1e-6).max(1e-6);
    let small = |face| {
        part.face_moments(face)
            .is_some_and(|m| m.area > 0.0 && m.area <= area * 0.01)
    };
    let mut candidates = BTreeMap::<usize, Vec<(usize, Vec<usize>, f64)>>::new();
    for (floor, f) in part.faces.iter().enumerate() {
        let Surface::Plane { frame } = &f.surface else {
            continue;
        };
        let walls: BTreeSet<_> = part
            .face_edges(floor)
            .iter()
            .flat_map(|&e| part.edge_faces()[e].iter().copied())
            .filter(|&n| n != floor)
            .collect();
        if walls.len() < 12 || !small(floor) || walls.iter().any(|&w| !small(w)) {
            continue;
        }
        let mut common: Option<BTreeSet<usize>> = None;
        for &wall in &walls {
            let adjacent: BTreeSet<_> = part
                .face_edges(wall)
                .iter()
                .flat_map(|&e| part.edge_faces()[e].iter().copied())
                .filter(|&n| n != floor && n != wall && !walls.contains(&n))
                .collect();
            common = Some(match common {
                None => adjacent,
                Some(previous) => previous.intersection(&adjacent).copied().collect(),
            });
        }
        let common = common.unwrap_or_default();
        if common.len() != 1 {
            continue;
        }
        let carrier = *common.first().unwrap();
        let Surface::Plane { frame: top } = &part.faces[carrier].surface else {
            continue;
        };
        if dot(frame.z, top.z).abs() < 1.0 - 1e-6 || small(carrier) {
            continue;
        }
        let bottom = dot(frame.origin, frame.z);
        let level = dot(top.origin, frame.z);
        let depth = dot(sub(top.origin, frame.origin), frame.z).abs();
        if depth <= tol || depth > longest * 0.01 {
            continue;
        }
        let walls: Vec<_> = walls.into_iter().collect();
        if walls.iter().any(|&w| {
            let (lo, hi) = part.extent_along(&[w], frame.z);
            (lo - bottom.min(level)).abs() > tol || (hi - bottom.max(level)).abs() > tol
        }) {
            continue;
        }
        candidates
            .entry(carrier)
            .or_default()
            .push((floor, walls, depth));
    }
    let mut out = Vec::new();
    for (carrier, floors) in candidates {
        // Equal depth on a common carrier proves one relief layer; widely separated or
        // large groups fail the small-region test instead of exempting scattered pockets.
        if floors.len() < 2 || floors.iter().any(|f| (f.2 - floors[0].2).abs() > tol) {
            continue;
        }
        let faces: BTreeSet<_> = floors
            .iter()
            .flat_map(|(floor, walls, _)| std::iter::once(*floor).chain(walls.iter().copied()))
            .collect();
        let mut region = crate::haecceity::geom::Bounds::empty();
        for &face in &faces {
            let b = part.face_bounds(face);
            region.add(b.min);
            region.add(b.max);
        }
        let sizes = [0, 1, 2].map(|k| region.max[k] - region.min[k]);
        let footprint = (sizes[0] * sizes[1])
            .max(sizes[0] * sizes[2])
            .max(sizes[1] * sizes[2]);
        if footprint > area * 0.02 {
            continue;
        }
        out.push(Relief {
            carrier,
            floors: floors.iter().map(|f| f.0).collect(),
            faces: faces.into_iter().collect(),
            depth: floors[0].2,
        });
    }
    out
}
