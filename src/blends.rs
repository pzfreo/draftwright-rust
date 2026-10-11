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
