//! Read the provider's true normal section, never XYZ-projected legs.
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, point, to_part};
use crate::model::Analysis;

pub(crate) struct Bevel {
    pub view_axis: usize,
    pub corner: [f64; 3],
}

pub(crate) fn read(a: &Analysis, f: &Feature) -> Result<Bevel, String> {
    let q = frame(a);
    let p = &f.parameters;
    let vector = |name| {
        point(p.get(name))
            .filter(|d| d.iter().all(|v| v.is_finite()) && dot(*d, *d).is_finite())
            .and_then(unit)
            .map(|d| dir_to_part(&q, d))
            .ok_or_else(|| format!("its record gives no finite {name} direction"))
    };
    let run = vector("run")?;
    let u = vector("leg1_direction")?;
    let v = vector("leg2_direction")?;
    if dot(u, v).abs() > 1e-5 || dot(run, u).abs() > 1e-5 || dot(run, v).abs() > 1e-5 {
        return Err("its run and support legs do not form an orthogonal normal section".into());
    }
    let size = |name| {
        p.get(name)
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.)
            .ok_or_else(|| format!("its record gives no positive {name}"))
    };
    let (l1, l2, angle) = (size("leg1")?, size("leg2")?, size("angle")?);
    size("length")?;
    // Legs are published to 0.001 mm; the angle is published to 0.01 degrees.
    let low = (l2 - 0.0005).max(0.).atan2(l1 + 0.0005).to_degrees() - 0.01;
    let high = (l2 + 0.0005).atan2((l1 - 0.0005).max(0.)).to_degrees() + 0.01;
    if angle < low || angle > high {
        return Err("its section angle disagrees with its support legs".into());
    }
    let corner = point(p.get("corner"))
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .map(|p| to_part(&q, p))
        .ok_or("its record gives no finite virtual corner")?;
    let at = point(p.get("at"))
        .map(|p| to_part(&q, p))
        .ok_or("its record gives no bevel site")?;
    // The bevel's normal is the gradient of x/leg1 + y/leg2; its largest
    // component gives the most readable face for a leader (not a projected angle).
    let normal = [0, 1, 2].map(|k| u[k] / l1 + v[k] / l2);
    let view_axis = (0..3)
        .max_by(|&i, &j| normal[i].abs().total_cmp(&normal[j].abs()))
        .unwrap();
    if !dot(sub(at, corner), run).is_finite() {
        return Err("its bevel site is not finite".into());
    }
    Ok(Bevel { view_axis, corner })
}

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
