//! Radius leader evidence and geometry, independent of the router's chosen sites.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    arc::{ProfileArc, Span},
    bridge::Feature,
    model::Analysis,
};

type P = [f64; 2];

pub(crate) const TOLERANCE: f64 = 0.002;
pub(crate) const RADIAL_DEGREES: f64 = 15.0;

/// A cylinder's trimmed cross-section, with the material side read from its face normal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiusArc {
    pub arc: ProfileArc,
    pub axis: [f64; 3],
    pub centre_open: bool,
    /// Proven points in the middle half of the trimmed face, used by candidate generation.
    pub tips: Vec<[f64; 3]>,
}

pub fn evidence(part: &haecceity::Part, features: &[Feature]) -> BTreeMap<String, Vec<RadiusArc>> {
    let mut out = BTreeMap::new();
    for f in features
        .iter()
        .filter(|f| matches!(f.kind.as_str(), "blend" | "fillet"))
    {
        let Some(r) = f.parameters.get("radius").and_then(|v| v.as_f64()) else {
            continue;
        };
        let mut arcs = Vec::new();
        for &face in &f.faces {
            let Some(data) = part.faces.get(face) else {
                continue;
            };
            let haecceity::geom::Surface::Cylinder { frame, radius } = &data.surface else {
                continue;
            };
            if (radius - r).abs() > TOLERANCE {
                continue;
            }
            let (Some(domain), Some((u0, u1, v0, v1))) = (part.domain(face), part.uv_bounds(face))
            else {
                continue;
            };
            if ![u0, u1, v0, v1].iter().all(|x| x.is_finite()) || u1 - u0 <= 1e-9 {
                continue;
            }
            let mut tips = Vec::new();
            for i in 4..=12 {
                let u = u0 + (u1 - u0) * i as f64 / 16.0;
                for fraction in [0.5, 0.25, 0.75] {
                    let v = v0 + (v1 - v0) * fraction;
                    if domain.contains(u, v) {
                        tips.push(data.surface.value(u, v));
                    }
                }
            }
            if tips.is_empty() {
                continue;
            }
            arcs.push(RadiusArc {
                arc: ProfileArc {
                    centre: frame.origin,
                    radius: *radius,
                    u: frame.x,
                    v: frame.y,
                    start: u0,
                    sweep: u1 - u0,
                },
                axis: frame.z,
                centre_open: data.reversed,
                tips,
            });
        }
        if !arcs.is_empty() {
            out.insert(f.id.clone(), arcs);
        }
    }
    out
}

#[derive(Clone, Debug)]
pub(crate) struct Projected {
    pub centre: P,
    pub radius: f64,
    pub span: Span,
    pub centre_open: bool,
    pub tips: Vec<P>,
}

impl Projected {
    pub fn on_arc(&self, tip: P) -> bool {
        let d = [tip[0] - self.centre[0], tip[1] - self.centre[1]];
        (d[0].hypot(d[1]) - self.radius).abs() <= TOLERANCE
            && self
                .span
                .contains(d[1].atan2(d[0]), TOLERANCE / self.radius.max(1e-9))
    }

    pub fn middle(&self, tip: P) -> bool {
        Span {
            start: self.span.start + self.span.sweep * 0.25,
            sweep: self.span.sweep * 0.5,
        }
        .contains(
            (tip[1] - self.centre[1]).atan2(tip[0] - self.centre[0]),
            TOLERANCE / self.radius.max(1e-9),
        )
    }

    /// Tip-to-elbow must point towards the centre, not merely share the radial line.
    pub fn accepts(&self, [tip, elbow]: [P; 2]) -> bool {
        let u = [self.centre[0] - tip[0], self.centre[1] - tip[1]];
        let v = [elbow[0] - tip[0], elbow[1] - tip[1]];
        let norm = u[0].hypot(u[1]) * v[0].hypot(v[1]);
        self.centre_open
            && self.on_arc(tip)
            && self.middle(tip)
            && norm > 1e-12
            && u[0] * v[0] + u[1] * v[1] >= RADIAL_DEGREES.to_radians().cos() * norm - 1e-9
    }
}

/// Read only end-on cylinders. Toroidal paths have no single projected arc centre.
/// Older analyses can still prove a straight blend from its rolling axis and trimmed samples.
pub(crate) fn projected(
    a: &Analysis,
    id: &str,
    toward: [f64; 3],
    page: impl Fn([f64; 3]) -> P,
) -> Result<Vec<Projected>, String> {
    let end_on = |axis: [f64; 3]| {
        let n = axis.iter().map(|x| x * x).sum::<f64>().sqrt();
        n > 1e-12
            && (axis.iter().zip(toward).map(|(a, b)| a * b).sum::<f64>() / n).abs() > 1.0 - 1e-9
    };
    if let Some(arcs) = a.radius_arcs.get(id) {
        let shown: Vec<_> = arcs
            .iter()
            .filter(|arc| end_on(arc.axis))
            .map(|arc| {
                let (centre, radius, span) = arc.arc.projected(&page);
                let mut tips: Vec<P> = Vec::new();
                for p in arc.tips.iter().map(|q| page(*q)) {
                    if !tips.iter().any(|q| (p[0] - q[0]).hypot(p[1] - q[1]) < 1e-8) {
                        tips.push(p);
                    }
                }
                Projected {
                    centre,
                    radius,
                    span,
                    centre_open: arc.centre_open,
                    tips,
                }
            })
            .collect();
        return if shown.is_empty() {
            Err("the view does not show the cylindrical radius end-on".into())
        } else {
            Ok(shown)
        };
    }
    let f = a
        .features
        .iter()
        .find(|f| f.id == id)
        .ok_or("the radius feature is absent")?;
    let path = f
        .parameters
        .get("path")
        .ok_or("no cylindrical arc centre and trim evidence is recorded for this fillet")?;
    if path.get("normal").is_some() {
        return Err("a circular-path/toroidal blend has no single projected minor-arc centre; radial direction and open side cannot be verified from its sampled points".into());
    }
    let frame = crate::bridge::holes::frame(a);
    let axis = crate::lint::parameters::direction(&frame, path.get("direction"))
        .ok_or("the radius path has no direction")?;
    if !end_on(axis) {
        return Err("the view does not show the radius end-on".into());
    }
    let at = crate::lint::parameters::place(&frame, path.get("at"))
        .ok_or("the radius path has no centre")?;
    let r = f
        .parameters
        .get("radius")
        .and_then(|v| v.as_f64())
        .filter(|r| r.is_finite() && *r > 0.0)
        .ok_or("the radius is unreadable")?;
    let sites = a
        .blend_sites
        .get(id)
        .filter(|s| !s.is_empty())
        .ok_or("the radius has no proven trimmed-surface points")?;
    let centre = page(at);
    let tips: Vec<_> = sites.iter().map(|q| page(*q)).collect();
    let angle = |p: P| (p[1] - centre[1]).atan2(p[0] - centre[0]);
    let reference = angle(tips[0]);
    let (lo, hi) = tips
        .iter()
        .map(|q| crate::arc::turn(angle(*q), reference))
        .fold((0.0f64, 0.0f64), |(lo, hi), t| (lo.min(t), hi.max(t)));
    if hi - lo <= 1e-6 {
        return Err("a lone angular sample cannot prove the arc's tangent ends".into());
    }
    // Scale is isotropic; choose a part direction perpendicular to the line of sight.
    let seed = if axis[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let norm = axis.iter().map(|x| x * x).sum::<f64>();
    let along = seed.iter().zip(axis).map(|(x, y)| x * y).sum::<f64>() / norm;
    let u = [0, 1, 2].map(|k| seed[k] - along * axis[k]);
    let n = u.iter().map(|x| x * x).sum::<f64>().sqrt();
    let edge = page([0, 1, 2].map(|k| at[k] + r * u[k] / n));
    Ok(vec![Projected {
        centre,
        radius: (edge[0] - centre[0]).hypot(edge[1] - centre[1]),
        span: Span {
            start: reference + lo - (hi - lo) * 0.5,
            sweep: (hi - lo) * 2.0,
        },
        centre_open: f.parameters.get("side").and_then(|v| v.as_str()) == Some("concave"),
        tips,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn quarter() -> Projected {
        Projected {
            centre: [0.0, 0.0],
            radius: 3.0,
            span: Span {
                start: 0.0,
                sweep: PI / 2.0,
            },
            centre_open: true,
            tips: Vec::new(),
        }
    }

    fn shaft(degrees: f64, deviation: f64) -> [P; 2] {
        let angle = degrees.to_radians();
        let tip = [3.0 * angle.cos(), 3.0 * angle.sin()];
        let direction = angle + PI + deviation.to_radians();
        [
            tip,
            [
                tip[0] + 10.0 * direction.cos(),
                tip[1] + 10.0 * direction.sin(),
            ],
        ]
    }

    #[test]
    fn clean_open_side_radius_and_tolerance_boundaries() {
        let arc = quarter();
        for angle in [22.5, 45.0, 67.5] {
            for deviation in [-15.0, 0.0, 15.0] {
                assert!(arc.accepts(shaft(angle, deviation)));
            }
        }
    }

    #[test]
    fn tangent_ends_are_not_radius_targets() {
        for angle in [0.0, 10.0, 80.0, 90.0] {
            assert!(!quarter().accepts(shaft(angle, 0.0)));
        }
    }

    #[test]
    fn off_radial_shafts_are_rejected() {
        for deviation in [-16.0, 16.0, 32.0, 90.0] {
            assert!(!quarter().accepts(shaft(45.0, deviation)));
        }
    }

    #[test]
    fn material_side_and_zero_length_shafts_are_rejected() {
        let arc = quarter();
        assert!(!arc.accepts(shaft(45.0, 180.0)));
        let tip = shaft(45.0, 0.0)[0];
        assert!(!arc.accepts([tip, tip]));
        let mut convex = arc;
        convex.centre_open = false;
        assert!(!convex.accepts(shaft(45.0, 0.0)));
    }
}
