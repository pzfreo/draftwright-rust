//! Consumer repair for quiddity's per-face Flat occurrences. Keep the provider untouched.
use super::Feature;
use haecceity::{Part, geom::Surface};
use serde_json::{Value, json};

fn plane(part: &Part, f: &Feature) -> Option<([f64; 3], [f64; 3])> {
    let Surface::Plane { frame } = &part.faces[*f.faces.first()?].surface else {
        return None;
    };
    let n = frame.z;
    let length = dot(n, n).sqrt();
    (length > 0.0).then(|| (frame.origin, n.map(|v| v / length)))
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn delta(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
type Stock = (usize, [i64; 3], [i64; 2], [i64; 2]);
fn stock(f: &Feature) -> Option<Stock> {
    let p = &f.parameters;
    let k = ["x", "y", "z"]
        .iter()
        .position(|a| Some(*a) == p.get("axis").and_then(Value::as_str))?;
    let mut d = [0.; 3];
    d[k] = 1.;
    if p.contains_key("axis_direction") {
        d = super::holes::point(p.get("axis_direction"))?;
    }
    let length = dot(d, d).sqrt();
    if length < 1e-12 || d[k].abs() < 1e-12 {
        return None;
    }
    let sign = d[k].signum();
    let quantize = |v: f64| (v * 1e6).round() as i64;
    let pair = |name| -> Option<[f64; 2]> {
        let v = p.get(name)?.as_array()?;
        Some([v.first()?.as_f64()?, v.get(1)?.as_f64()?])
    };
    let span = pair("stock_span")?;
    let span = if sign < 0. {
        [-span[1], -span[0]]
    } else {
        span
    };
    Some((
        k,
        d.map(|v| quantize(v * sign / length)),
        pair("axis_line")?.map(quantize),
        span.map(quantize),
    ))
}
fn same_stock(a: &Feature, b: &Feature) -> bool {
    stock(a).is_some_and(|key| Some(key) == stock(b))
}
fn axis_point(f: &Feature, frame: &[[f64; 3]; 4]) -> Option<[f64; 3]> {
    let p = &f.parameters;
    let axis = p.get("axis")?.as_str()?;
    let k = ["x", "y", "z"].iter().position(|a| *a == axis)?;
    let line = p.get("axis_line")?.as_array()?;
    let mut q = [0.0; 3];
    let other: Vec<_> = (0..3).filter(|j| *j != k).collect();
    for i in 0..2 {
        q[other[i]] = line.get(i)?.as_f64()?;
    }
    let direction = super::holes::point(p.get("axis_direction"))?;
    if direction[k].abs() < 1e-9 {
        return None;
    }
    q[k] = -dot(q, direction) / direction[k];
    Some(std::array::from_fn(|i| {
        frame[0][i] + (0..3).map(|j| q[j] * frame[j + 1][i]).sum::<f64>()
    }))
}

pub(super) fn merge(part: &Part, frame: &[[f64; 3]; 4], features: &mut Vec<Feature>) {
    let mut i = 0;
    while i < features.len() {
        if features[i].kind != "flat" {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < features.len() {
            let coplanar = plane(part, &features[i])
                .zip(plane(part, &features[j]))
                .is_some_and(|((a, n), (b, m))| {
                    dot(n, m).abs() > 1.0 - 1e-9 && dot(delta(a, b), n).abs() <= 1e-6
                });
            if features[j].kind == "flat" && same_stock(&features[i], &features[j]) && coplanar {
                let removed = features.remove(j);
                let f = &mut features[i];
                f.faces.extend(removed.faces);
                f.faces.sort_unstable();
                f.faces.dedup();
                f.fingerprint.position = super::centre(part, &f.faces).map(super::round);
                f.fingerprint.neighbourhood = super::neighbourhood(part, &f.faces);
            } else {
                j += 1
            }
        }
        i += 1;
    }
    for i in 0..features.len() {
        if features[i].kind != "flat" {
            continue;
        }
        let Some((origin, normal)) = plane(part, &features[i]) else {
            continue;
        };
        let opposite = axis_point(&features[i], frame).is_some_and(|axis| {
            features.iter().enumerate().any(|(j, f)| {
                i != j
                    && f.kind == "flat"
                    && same_stock(&features[i], f)
                    && plane(part, f).is_some_and(|(b, m)| {
                        dot(normal, m).abs() > 1.0 - 1e-9
                            && dot(delta(origin, axis), normal) * dot(delta(b, axis), normal)
                                < -1e-12
                    })
            })
        });
        if let Some(axis) = axis_point(&features[i], frame) {
            features[i].parameters.insert(
                "axis_distance".into(),
                json!(dot(delta(origin, axis), normal).abs()),
            );
        }
        features[i]
            .parameters
            .insert("opposed".into(), Value::Bool(opposite));
        // Parameters share the provider's working frame, including this geometric proof.
        let at: [f64; 3] = std::array::from_fn(|k| dot(delta(origin, frame[0]), frame[k + 1]));
        let normal: [f64; 3] = std::array::from_fn(|k| dot(normal, frame[k + 1]));
        features[i]
            .parameters
            .insert("flat_plane".into(), json!({"at":at,"normal":normal}));
    }
    // Merging can remove a flat ID. Other families' member references remain unchanged.
    let mut order: Vec<_> = (0..features.len())
        .filter(|i| features[*i].kind == "flat")
        .collect();
    order.sort_by_key(|i| features[*i].faces.clone());
    for (n, i) in order.into_iter().enumerate() {
        features[i].id = format!("f:flat:{}", n + 1);
    }
}
