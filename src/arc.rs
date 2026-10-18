//! Exact trimmed circular profiles shared by callout placement and leader lint.
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use crate::bridge::Feature;
use haecceity::{
    Part,
    geom::{Curve, Surface},
};

/// *a* − *b* wrapped into (−π, π], shared with the blend sector check.
pub(crate) fn turn(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    if d > PI { d - TAU } else { d }
}

/// A closed angular interval, counterclockwise from `start`, including wrapped arcs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Span {
    pub start: f64,
    pub sweep: f64,
}

impl Span {
    pub fn clamp(self, angle: f64) -> f64 {
        let t = (angle - self.start).rem_euclid(TAU);
        if t <= self.sweep {
            angle
        } else if t - self.sweep < TAU - t {
            self.start + self.sweep
        } else {
            self.start
        }
    }

    pub fn contains(self, angle: f64, slack: f64) -> bool {
        let d = turn(angle, self.clamp(angle));
        d.abs() <= slack
    }
}

/// A feature's actual circular boundary edge, in the part frame. The unit `u`/`v`
/// vectors retain the edge's angular orientation through any drawing projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileArc {
    pub centre: [f64; 3],
    pub radius: f64,
    pub u: [f64; 3],
    pub v: [f64; 3],
    pub start: f64,
    pub sweep: f64,
}

impl ProfileArc {
    pub fn point(&self, angle: f64) -> [f64; 3] {
        [0, 1, 2].map(|k| {
            self.centre[k] + self.radius * (self.u[k] * angle.cos() + self.v[k] * angle.sin())
        })
    }

    pub(crate) fn projected(&self, page: impl Fn([f64; 3]) -> [f64; 2]) -> ([f64; 2], f64, Span) {
        let c = page(self.centre);
        let a = page(self.point(self.start));
        let b = page(self.point(self.start + self.sweep));
        let m = page(self.point(self.start + self.sweep / 2.0));
        let angle = |p: [f64; 2]| (p[1] - c[1]).atan2(p[0] - c[0]);
        let start = angle(a);
        let end = angle(b);
        let forward = (end - start).rem_euclid(TAU);
        let span = if self.sweep >= TAU - 1e-6 {
            Span { start, sweep: TAU }
        } else if (angle(m) - start).rem_euclid(TAU) <= forward + 1e-8 {
            Span {
                start,
                sweep: forward,
            }
        } else {
            Span {
                start: end,
                sweep: TAU - forward,
            }
        };
        (c, (a[0] - c[0]).hypot(a[1] - c[1]), span)
    }
}

/// Preserve the actual circle edges of radius-bearing partial profiles. Straight
/// boundaries and cylindrical seams cannot enlarge an arc into a full circle.
pub fn evidence(part: &Part, features: &[Feature]) -> BTreeMap<String, Vec<ProfileArc>> {
    let mut out = BTreeMap::new();
    for f in features
        .iter()
        .filter(|f| matches!(f.kind.as_str(), "circular_blind_step" | "slot"))
    {
        let Some(radius) = f
            .parameters
            .get(if f.kind == "slot" {
                "end_radius"
            } else {
                "radius"
            })
            .and_then(|v| v.as_f64())
        else {
            continue;
        };
        // Feature dimensions can be quantised (for example R3.17 for an
        // actual R3.175 cylinder). Select the carrying cylinders by nearest
        // recorded radius, then match their boundary edges to the exact surface,
        // never to that rounded dimension. Retain a planar-only legacy record.
        let mut faces: Vec<_> = f
            .faces
            .iter()
            .filter_map(|&i| {
                if let Surface::Cylinder { radius, .. } = part.faces[i].surface {
                    Some((i, radius))
                } else {
                    None
                }
            })
            .collect();
        if let Some(error) = faces
            .iter()
            .map(|(_, r)| (r - radius).abs())
            .reduce(f64::min)
        {
            faces.retain(|(_, r)| (r - radius).abs() <= error + 1e-8);
        } else {
            faces.extend(f.faces.iter().map(|&i| (i, radius)));
        }
        let mut arcs = Vec::new();
        for (face, surface_radius) in faces {
            for edge in part.face_edges(face) {
                let e = &part.edges[edge];
                let Curve::Circle { frame, radius: r } = &e.curve else {
                    continue;
                };
                if (surface_radius - r).abs() > 0.002 {
                    continue;
                }
                let (a, b) = haecceity::sampling::edge_interval(
                    &e.curve,
                    e.start,
                    e.end,
                    e.same_sense,
                    e.is_closed(),
                );
                let arc = ProfileArc {
                    centre: frame.origin,
                    radius: *r,
                    u: frame.x,
                    v: frame.y,
                    start: a.min(b),
                    sweep: (b - a).abs(),
                };
                if !arcs.contains(&arc) {
                    arcs.push(arc);
                }
            }
        }
        if !arcs.is_empty() {
            out.insert(f.id.clone(), arcs);
        }
    }
    for f in features.iter().filter(|f| f.kind == "slot_pattern") {
        let arcs: Vec<_> = f
            .parameters
            .get("slots")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|id| id.as_str().and_then(|id| out.get(id)))
            .flatten()
            .cloned()
            .collect();
        if !arcs.is_empty() {
            out.insert(f.id.clone(), arcs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_clamps_to_trimmed_arc_including_wrap_and_reflection() {
        let span = Span {
            start: PI,
            sweep: PI / 2.0,
        };
        assert!((span.clamp(3.0 * PI / 4.0) - PI).abs() < 1e-12);
        assert!(span.contains(5.0 * PI / 4.0, 0.0));
        assert!(!span.contains(3.0 * PI / 4.0, 0.002));
        let wrap = Span {
            start: 7.0 * PI / 4.0,
            sweep: PI / 2.0,
        };
        assert!(wrap.contains(0.0, 1e-12));
        assert!(!wrap.contains(PI, 0.002));
        let arc = ProfileArc {
            centre: [0.0; 3],
            radius: 2.0,
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
            start: PI,
            sweep: PI / 2.0,
        };
        let (_, _, reflected) = arc.projected(|p| [p[0], -p[1]]);
        assert!(reflected.contains(3.0 * PI / 4.0, 1e-12));
        assert!(!reflected.contains(5.0 * PI / 4.0, 0.002));
    }
}
