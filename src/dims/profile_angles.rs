//! Outer-profile angles from immutable provider evidence, placed by the angular renderer.
use super::{Context, Measurement, Planner, Problem, Shape, Want, end_on, fmt, want_id};
pub(super) const PLANNER: Planner = Planner { kinds: &[], wants };
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let out = crate::lint::profile_angles::requirements(cx.analysis)
        .iter()
        .map(|c| {
            let value = c.degrees();
            let witness = |p: [f64; 3]| {
                let delta = [0, 1, 2].map(|i| p[i] - c.vertex[i]);
                let length = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
                if c.virtual_vertex {
                    p
                } else {
                    [0, 1, 2].map(|i| c.vertex[i] + delta[i] * length.min(1.) / length)
                }
            };
            Want {
                id: want_id(&c.feature, "profile_angle.angle"),
                views: vec![end_on(c.axis)],
                value,
                terms: vec![format!("{}°", fmt(value))],
                measurements: vec![Measurement {
                    feature: c.feature.clone(),
                    parameter: "profile_angle.angle".into(),
                    value,
                }],
                shape: Shape::Angular {
                    angle: super::angular::Angle {
                        vertex: c.vertex,
                        first: witness(c.first),
                        second: witness(c.second),
                    },
                },
                turning: None,
                basis: "a non-right convex outer-profile corner on its body's foremost camera-facing face, from the provider's ordered supports".into(),
            }
        })
        .collect();
    (out, Vec::new())
}
