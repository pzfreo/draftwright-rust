//! Modelled threads that quiddity refuses but whose faces still measure as one constant screw
//! thread. quiddity-rust 0.2.1 requires every flank face to span the thread depth, so a flank
//! split where a bore chamfer trims it (dw-examples t8-flange-nut, a 4-start Tr8 nut) refuses
//! the whole thread as `inconsistent_diameters`, and its raw helix is drawn with no size.
//!
//! The refused faces are measured here with the same invariants and nothing looser: every
//! helical edge has one lead; every face of constant radius is at the least or the greatest
//! radius of the faces (minor and major), and both occur; the crest and root edges repeat under
//! a shift of the lead / starts; depth and pitch keep quiddity's proportions. A refusal that
//! fails any of these stays refused. A completed refusal becomes a `ModelledThread` record with
//! the refusal's reason as `refused`, so it is read, identified and depicted like any other.
use std::collections::BTreeSet;
use std::f64::consts::{PI, TAU};

use haecceity::Part;
use haecceity::geom::{self, Curve, Surface, V3};
use quiddity::recognition::{FaceAccounting, RecognitionDocument};
use serde_json::{Map, Value, json};

/// Replaces each [`measure`]d `ModelledThreadRefusal` in *doc* by a `ModelledThread` record on
/// the same faces, claims those faces for it in the face accounting, and drops the freeform
/// surface records lying wholly on them, as quiddity reports none on a thread it recognises.
pub(crate) fn complete(part: &Part, doc: &mut RecognitionDocument) {
    let mut next = doc
        .records
        .iter()
        .filter(|r| r.family == "modelled_threads")
        .count();
    let mut claimed = BTreeSet::new();
    for r in &mut doc.records {
        if r.record_type != "ModelledThreadRefusal"
            || r.faces.iter().any(|&f| f >= part.faces.len())
        {
            continue;
        }
        let point = |v: &Value| -> Option<V3> {
            let a = v.as_array()?;
            (a.len() == 3).then_some(())?;
            Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
        };
        let axis = &r.parameters["axis"];
        let (Some(on), Some(direction)) = (
            point(&axis["point"]),
            point(&axis["direction"]).and_then(geom::unit),
        ) else {
            continue;
        };
        let Some(mut parameters) = measure(part, on, direction, &r.faces) else {
            continue;
        };
        parameters.insert("refused".into(), r.parameters["reason"].clone());
        r.id = format!("modelled_threads/{next}");
        next += 1;
        r.family = "modelled_threads".into();
        r.record_type = "ModelledThread".into();
        r.parameters = parameters;
        if let Some(groups) = &mut doc.face_accounting {
            for &f in &r.faces {
                if let Some(g) = groups.get_mut(f) {
                    *g = FaceAccounting::Feature {
                        record_ids: vec![r.id.clone()],
                    };
                }
            }
        }
        claimed.extend(r.faces.iter().copied());
    }
    doc.records.retain(|r| {
        r.family != "freeform_surfaces" || !r.faces.iter().all(|f| claimed.contains(f))
    });
}

/// Radial measurement tolerance, quiddity's: 0.001 mm or 0.1% of the radius.
fn tolerance(size: f64) -> f64 {
    1e-3_f64.max(size.abs() * 1e-3)
}

/// Axial residual, quiddity's, allowing the interpolation error of STEP helices.
fn axial_tolerance(lead: f64) -> f64 {
    0.003_f64.max(lead.abs() * 0.005)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let m = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[m - 1] + values[m]) * 0.5
    } else {
        values[m]
    }
}

/// The thread record quiddity would write for *faces* about the axis through *on* along
/// *direction*, or none when they are not one constant screw thread.
fn measure(part: &Part, on: V3, direction: V3, faces: &[usize]) -> Option<Map<String, Value>> {
    let x = geom::default_x_axis(direction);
    let y = geom::cross(direction, x);
    let polar = |p: V3| {
        let q = geom::sub(p, on);
        let (u, v) = (geom::dot(q, x), geom::dot(q, y));
        (u.hypot(v), v.atan2(u), geom::dot(q, direction))
    };
    // Boundary samples and a 17 × 17 grid inside each face's trimmed domain, with normals.
    let mut samples = Vec::new();
    for &f in faces {
        let mut points: Vec<V3> = part
            .face_edges(f)
            .iter()
            .flat_map(|&e| part.edges[e].samples.iter().copied())
            .collect();
        let mut normals = Vec::new();
        if let Some((u0, u1, v0, v1)) = part.uv_bounds(f) {
            for i in 0..=16 {
                for j in 0..=16 {
                    let (u, v) = (
                        u0 + (u1 - u0) * f64::from(i) / 16.,
                        v0 + (v1 - v0) * f64::from(j) / 16.,
                    );
                    if !part.domain(f).is_some_and(|d| d.contains(u, v)) {
                        continue;
                    }
                    let p = part.faces[f].surface.value(u, v);
                    points.push(p);
                    if let Some(n) = part.face_normal(f, u, v) {
                        normals.push((p, n));
                    }
                }
            }
        }
        if points.is_empty() {
            return None;
        }
        let radii: Vec<f64> = points.iter().map(|&p| polar(p).0).collect();
        let axial: Vec<f64> = points.iter().map(|&p| polar(p).2).collect();
        samples.push((f, radii, axial, normals));
    }
    let lo = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let r0 = samples
        .iter()
        .map(|s| lo(&s.1))
        .fold(f64::INFINITY, f64::min);
    let r1 = samples
        .iter()
        .map(|s| hi(&s.1))
        .fold(f64::NEG_INFINITY, f64::max);
    let depth = r1 - r0;
    let tol = 2. * tolerance(r1);
    if depth < 2. * tol {
        return None;
    }
    let (mut low, mut high, mut flanks, mut runouts) = (vec![], vec![], vec![], vec![]);
    let (mut low_radii, mut high_radii) = (vec![], vec![]);
    let (mut start, mut end) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut side = None;
    for (f, radii, axial, normals) in &samples {
        if hi(radii) - lo(radii) <= tol {
            let mut rs = radii.clone();
            let r = median(&mut rs);
            if (r - r0).abs() <= tol {
                low.push(*f);
                low_radii.extend(radii);
            } else if (r - r1).abs() <= tol {
                high.push(*f);
                high_radii.extend(radii);
            } else {
                return None;
            }
            // Material outside the surface (internal) faces its normal towards the axis.
            for &(p, n) in normals {
                let q = geom::sub(p, on);
                let radial = geom::unit(geom::sub(
                    q,
                    geom::scale(direction, geom::dot(q, direction)),
                ))?;
                let internal = geom::dot(n, radial) < 0.;
                if *side.get_or_insert(internal) != internal {
                    return None;
                }
            }
        } else {
            if matches!(part.faces[*f].surface, Surface::Freeform { .. }) {
                flanks.push(*f);
            } else {
                runouts.push(*f);
            }
            start = start.min(lo(axial));
            end = end.max(hi(axial));
        }
    }
    let internal = side?;
    if low.is_empty() || high.is_empty() || flanks.len() < 2 {
        return None;
    }
    let minor = 2. * median(&mut low_radii);
    let major = 2. * median(&mut high_radii);

    // Helical edges: constant radius, monotone angle, axial advance linear in the angle.
    let edges: BTreeSet<usize> = faces.iter().flat_map(|&f| part.face_edges(f)).collect();
    let mut helices = Vec::new();
    for e in edges {
        let edge = &part.edges[e];
        if edge.samples.len() < 9 || !matches!(edge.curve, Curve::Nurbs(_)) {
            continue;
        }
        let mut unwrapped = Vec::with_capacity(edge.samples.len());
        let mut previous: Option<f64> = None;
        for &p in &edge.samples {
            let (r, a, z) = polar(p);
            let a = previous.map_or(a, |prev| geom::nearest_turn(a, prev));
            previous = Some(a);
            unwrapped.push((r, a, z));
        }
        let n = unwrapped.len() as f64;
        let radius = unwrapped.iter().map(|p| p.0).sum::<f64>() / n;
        let (first, last) = (unwrapped[0], unwrapped[unwrapped.len() - 1]);
        let da = last.1 - first.1;
        let lead = (last.2 - first.2) * TAU / da;
        if unwrapped
            .iter()
            .any(|p| (p.0 - radius).abs() > tolerance(radius))
            || da.abs() < PI * 0.5
            || !lead.is_finite()
            || lead.abs() < 1e-3
            || unwrapped.windows(2).any(|w| (w[1].1 - w[0].1) * da < -1e-8)
        {
            continue;
        }
        let a_mean = unwrapped.iter().map(|p| p.1).sum::<f64>() / n;
        let z_mean = unwrapped.iter().map(|p| p.2).sum::<f64>() / n;
        if unwrapped
            .iter()
            .any(|p| (p.2 - z_mean - lead / TAU * (p.1 - a_mean)).abs() > axial_tolerance(lead))
        {
            continue;
        }
        helices.push((radius, lead, z_mean - lead * a_mean / TAU, (da / TAU).abs()));
    }
    let mut leads: Vec<f64> = helices
        .iter()
        .filter(|h| h.3 >= 0.45)
        .map(|h| h.1)
        .collect();
    if leads.len() < 2 {
        return None;
    }
    let lead = median(&mut leads);
    let axial = axial_tolerance(lead);
    if helices.iter().any(|h| (h.1 - lead).abs() > axial) {
        return None;
    }
    // Starts: the largest n under which the crest and the root edges' phases each repeat
    // with a period of lead / n.
    let phases = |radius: f64| -> Vec<f64> {
        helices
            .iter()
            .filter(|h| (h.0 - radius).abs() <= tol)
            .map(|h| h.2.rem_euclid(lead.abs()))
            .collect()
    };
    let (inner, outer) = (phases(minor / 2.), phases(major / 2.));
    if inner.is_empty() || outer.is_empty() {
        return None;
    }
    let near = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(lead.abs());
        d.min(lead.abs() - d) <= axial
    };
    let repeats =
        |set: &[f64], shift: f64| set.iter().all(|&p| set.iter().any(|&q| near(p + shift, q)));
    let starts = (1..=12u64)
        .rev()
        .find(|&n| {
            let shift = lead.abs() / n as f64;
            repeats(&inner, shift) && repeats(&outer, shift)
        })
        .unwrap_or(1);
    let pitch = lead.abs() / starts as f64;
    if end - start < 2. * pitch || depth / pitch < 0.2 || depth / pitch > 0.9 {
        return None;
    }
    let (crest, root) = if internal { (low, high) } else { (high, low) };
    json!({
        "axis": {"point": on, "direction": direction},
        "side": if internal { "internal" } else { "external" },
        "major_diameter": major,
        "minor_diameter": minor,
        "pitch": pitch,
        "hand": if lead > 0. { "right" } else { "left" },
        "starts": starts,
        "start": start,
        "end": end,
        "length": end - start,
        "tolerances": {
            "diameter": 4. * tolerance(major / 2.),
            "pitch": axial / starts as f64,
            "axial": axial,
        },
        "flank_faces": flanks,
        "crest_faces": crest,
        "root_faces": root,
        "runout_faces": runouts,
    })
    .as_object()
    .cloned()
}
