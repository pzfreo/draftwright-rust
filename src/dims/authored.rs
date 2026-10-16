//! Spec-authored quantities use the ordinary Want -> lanes -> placement -> ink pipeline.
use super::{Measurement, Shape, Want, angular, fmt};
use crate::model::dimension_geometry::{Reference, dot, sub, unit};
use crate::model::{Analysis, Invalid, Spec};

fn invalid(message: impl Into<String>) -> Invalid {
    Invalid {
        message: message.into(),
        choices: vec![],
    }
}

pub(crate) fn check(s: &Spec) -> Result<(), Invalid> {
    for (id, d) in &s.dimensions {
        let authored = d.between.is_some() || d.kind.is_some() || d.along.is_some();
        if !authored {
            if id.starts_with("d:authored:") {
                return Err(invalid(format!(
                    "{id} needs between and kind (linear, diameter, radius or angle); remove the entry to delete it."
                )));
            }
            continue;
        }
        if !id.strip_prefix("d:authored:").is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        }) {
            return Err(invalid(format!(
                "{id}: authored dimensions need a stable d:authored:<name> ID (letters, digits, _ or -)."
            )));
        }
        let kind = d.kind.as_deref().unwrap_or("linear");
        let count = match kind {
            "linear" | "angle" => 2,
            "diameter" | "radius" => 1,
            _ => {
                return Err(invalid(format!(
                    "{id}: kind must be linear, diameter, radius or angle."
                )));
            }
        };
        if !d
            .between
            .as_ref()
            .is_some_and(|refs| refs.len() == count && refs.iter().all(|r| !r.is_empty()))
        {
            return Err(invalid(format!(
                "{id}: {kind} needs {count} reference(s) in between; copy keys from Analysis.edge_ink.references."
            )));
        }
        if d.along
            .as_ref()
            .is_some_and(|a| kind != "linear" || !["x", "y", "z", "true"].contains(&a.as_str()))
        {
            return Err(invalid(format!(
                "{id}: along is x, y, z or true, and applies only to linear dimensions."
            )));
        }
        if d.fit.is_some() && !["linear", "diameter"].contains(&kind) {
            return Err(invalid(format!(
                "{id}: a fit qualifies a linear size or a diameter; give this radius or angle an explicit tolerance."
            )));
        }
    }
    Ok(())
}

pub(super) fn append(a: &Analysis, s: &Spec, wants: &mut Vec<Want>) -> Result<(), Invalid> {
    if !s.dimensions.keys().any(|id| id.starts_with("d:authored:"))
        && !s
            .dimensions
            .values()
            .any(|d| d.between.is_some() || d.kind.is_some() || d.along.is_some())
    {
        return Ok(());
    }
    check(s)?;
    for (id, d) in s.dimensions.iter().filter(|(_, d)| d.between.is_some()) {
        let refs = d.between.as_ref().unwrap();
        if s.binding
            .as_ref()
            .is_some_and(|b| b.source_sha256 != a.binding.source_sha256)
            && refs.iter().any(|r| r.starts_with("axis:f:"))
        {
            return Err(invalid(format!(
                "{id}: the Spec is bound to another STEP revision, so its feature-axis ID is unsafe; replace axis:f:<id> with the matching axis:face:v1:<fingerprint> from the current Analysis, or explicitly rebind the Spec."
            )));
        }
        let kind = d.kind.as_deref().unwrap_or("linear");
        let along = d.along.as_deref();
        let m = a
            .measure_references(refs, kind, along)
            .map_err(|why| invalid(format!("dimensions/{id}: {why}")))?;
        let (shape, views) = match kind {
            "linear" => {
                let [p, q] = m.witnesses.unwrap();
                if let Some(k) = ["x", "y", "z"].iter().position(|k| Some(*k) == along) {
                    (
                        Shape::Linear {
                            a: p,
                            b: q,
                            axis: k,
                            a_edge: false,
                            b_edge: false,
                            order: 0,
                        },
                        super::envelope::profile_views(k),
                    )
                } else {
                    let views = super::TOWARD
                        .iter()
                        .filter(|(_, t)| dot(sub(q, p), *t).abs() < 1e-6)
                        .map(|(v, _)| *v)
                        .collect();
                    (Shape::Aligned { a: p, b: q }, views)
                }
            }
            "angle" => {
                let [p, q] = m.witnesses.unwrap();
                let [u, v] = m.directions.unwrap();
                let ra = a.dimension_reference(&refs[0]).map_err(invalid)?;
                let mut angle = if matches!(ra, Reference::Plane { .. }) {
                    angular::from_planes([(p, u), (q, v)], [p, q]).map_err(invalid)?
                } else {
                    let c = dot(u, v);
                    let delta = sub(q, p);
                    let den = 1.0 - c * c;
                    if den < 1e-10 {
                        return Err(invalid(format!(
                            "{id}: parallel edges have no angular sector."
                        )));
                    }
                    let t = (dot(delta, u) - c * dot(delta, v)) / den;
                    let t2 = c * t - dot(delta, v);
                    let vertex = [0, 1, 2].map(|k| p[k] + t * u[k]);
                    if (0..3).any(|k| (vertex[k] - (q[k] + t2 * v[k])).abs() > 1e-6) {
                        return Err(invalid(format!(
                            "{id}: skew edges do not meet; choose coplanar edges."
                        )));
                    }
                    angular::Angle {
                        vertex,
                        first: [0, 1, 2].map(|k| vertex[k] + u[k]),
                        second: [0, 1, 2].map(|k| vertex[k] + v[k]),
                    }
                };
                if angle.degrees().is_some_and(|v| v > 90.0 + 1e-7) {
                    angle.second = [0, 1, 2].map(|k| 2.0 * angle.vertex[k] - angle.second[k]);
                }
                let views = angle
                    .principal()
                    .map(|k| vec![super::end_on(k)])
                    .unwrap_or_default();
                (Shape::Angular { angle }, views)
            }
            _ => {
                if m.sites.is_empty() {
                    return Err(invalid(format!(
                        "{id}: the face has no proven leader attachment; choose another face."
                    )));
                }
                // Exact surface sites, including oblique cylinders: a size leader need not be end-on.
                let views = match a.dimension_reference(&refs[0]).map_err(invalid)? {
                    Reference::Cylinder { axis, .. } => {
                        let k = (0..3).find(|&k| axis[k].abs() > 1.0 - 1e-8);
                        k.map(|k| vec![super::end_on(k)])
                            .unwrap_or_else(|| super::NAMES.to_vec())
                    }
                    _ => super::NAMES.to_vec(),
                };
                (
                    Shape::Callout {
                        sites: m.sites.iter().take(32).map(|&p| (p, 0.0)).collect(),
                    },
                    views,
                )
            }
        };
        if views.is_empty() {
            return Err(invalid(format!(
                "{id}: no principal view shows this quantity true; choose references lying in a principal plane, or along x, y or z."
            )));
        }
        let parameter = if kind == "linear" {
            format!("linear.{}", along.unwrap_or("true"))
        } else {
            kind.to_string()
        };
        let feature = format!("geometry:{}", refs.join("|"));
        let label = match kind {
            "diameter" => format!("⌀{}", fmt(m.value)),
            "radius" => format!("R{}", fmt(m.value)),
            "angle" => format!("{}°", fmt(m.value)),
            _ => fmt(m.value),
        };
        let w = Want {
            id: id.clone(),
            views,
            value: m.value,
            terms: vec![label],
            measurements: vec![Measurement {
                feature,
                parameter,
                value: m.value,
            }],
            basis: format!(
                "spec-authored: {kind} from {}; measured from exact model references",
                refs.join(" and ")
            ),
            shape,
            turning: None,
        };
        // Only compare physical supports, never scalar equality alone.
        for existing in wants.iter().filter(|w| {
            !d.omit
                && !s.dimensions.get(&w.id).is_some_and(|d| d.omit)
                && (s.dimension_source == crate::model::DimensionSource::Auto
                    || s.dimensions.contains_key(&w.id))
        }) {
            if duplicate(a, &w, existing, refs, kind) {
                return Err(Invalid {
                    message: format!(
                        "{id} duplicates the quantity owned by {}; remove {id} and set tolerance/view/place on the existing owner (K6: one quantity, one owner).",
                        existing.id
                    ),
                    choices: vec![existing.id.clone()],
                });
            }
        }
        wants.push(w);
    }
    Ok(())
}

fn duplicate(a: &Analysis, w: &Want, o: &Want, refs: &[String], kind: &str) -> bool {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-5;
    let span = |s: &Shape| match s {
        Shape::Linear { a, b, axis, .. } => {
            Some((*axis, a[*axis].min(b[*axis]), a[*axis].max(b[*axis])))
        }
        Shape::Aligned { a, b } => {
            let delta = sub(*b, *a);
            let k = (0..3).find(|&k| (0..3).all(|j| j == k || delta[j].abs() < 1e-6))?;
            Some((k, a[k].min(b[k]), a[k].max(b[k])))
        }
        _ => None,
    };
    if let (Some((k, p, q)), Some((j, r, s))) = (span(&w.shape), span(&o.shape)) {
        return k == j && close(p, r) && close(q, s);
    }
    if let (Shape::Aligned { a: p, b: q }, Shape::Aligned { a: r, b: s }) = (&w.shape, &o.shape) {
        let eq = |p: &[f64; 3], q: &[f64; 3]| (0..3).all(|k| close(p[k], q[k]));
        if (eq(p, r) && eq(q, s)) || (eq(p, s) && eq(q, r)) {
            return true;
        }
    }
    if o.measurements.iter().any(|m| {
        m.feature.starts_with("geometry:") && {
            let other: Vec<_> = m
                .feature
                .trim_start_matches("geometry:")
                .split('|')
                .collect();
            let mut r: Vec<_> = refs.iter().map(String::as_str).collect();
            r.sort();
            let mut other = other;
            other.sort();
            r == other
                && ((kind == "diameter" || kind == "radius")
                    && (m.parameter == "diameter" || m.parameter == "radius")
                    || m.parameter == w.measurements[0].parameter)
        }
    }) {
        return true;
    }
    if kind == "radius" || kind == "diameter" {
        let Ok(r) = a.dimension_reference(&refs[0]) else {
            return false;
        };
        let bounds = match r {
            Reference::Cylinder { bounds, .. } | Reference::Sphere { bounds, .. } => bounds,
            _ => return false,
        };
        let faces: Vec<_> = a
            .face_attachments
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                f.bounds
                    .filter(|b| {
                        (0..3).all(|k| close(bounds[0][k], b[k]) && close(bounds[1][k], b[k + 3]))
                    })
                    .map(|_| i)
            })
            .collect();
        return o.measurements.iter().any(|m| {
            (m.parameter.contains("diameter") || m.parameter.contains("radius"))
                && a.features
                    .iter()
                    .find(|f| f.id == m.feature)
                    .is_some_and(|f| f.faces.iter().any(|i| faces.contains(i)))
        });
    }
    if let (Shape::Angular { angle: p }, Shape::Angular { angle: q }) = (&w.shape, &o.shape) {
        return close(w.value, o.value)
            && (0..3).all(|k| close(p.vertex[k], q.vertex[k]))
            && p.rays().zip(q.rays()).is_some_and(|(p, q)| {
                p.iter().all(|p| {
                    q.iter().any(|q| {
                        unit(*q)
                            .zip(unit(*p))
                            .is_some_and(|(p, q)| dot(p, q).abs() > 1.0 - 1e-6)
                    })
                })
            });
    }
    false
}

/// Hand only the numerically stated roles to a new owner. Compound riders remain required.
#[derive(Default)]
pub(super) struct Ownership {
    pub claims: std::collections::BTreeMap<String, Vec<Measurement>>,
    pub covered: std::collections::BTreeMap<String, String>,
}

pub(super) fn ownership(a: &Analysis, s: &Spec, wants: &[Want]) -> Ownership {
    let mut out = Ownership::default();
    if s.dimension_source != crate::model::DimensionSource::Authored {
        return out;
    }
    for w in wants.iter().filter(|w| w.id.starts_with("d:authored:")) {
        let Some(d) = s.dimensions.get(&w.id) else {
            continue;
        };
        let Some(refs) = &d.between else { continue };
        let kind = d.kind.as_deref().unwrap_or("linear");
        for original in wants
            .iter()
            .filter(|o| !o.id.starts_with("d:authored:") && !s.dimensions.contains_key(&o.id))
        {
            if !duplicate(a, w, original, refs, kind) {
                continue;
            }
            let stated: Vec<_> = original
                .measurements
                .iter()
                .filter(|m| {
                    (m.value - w.value).abs() < 1e-6
                        && match kind {
                            "linear" => matches!(
                                original.shape,
                                Shape::Linear { .. } | Shape::Aligned { .. }
                            ),
                            "angle" => m.parameter.contains("angle"),
                            "diameter" => m.parameter.contains("diameter"),
                            "radius" => m.parameter.contains("radius"),
                            _ => false,
                        }
                })
                .cloned()
                .collect();
            if !stated.is_empty() && stated.len() == original.measurements.len() {
                out.covered.insert(original.id.clone(), w.id.clone());
            }
            let claims = out.claims.entry(w.id.clone()).or_default();
            for m in stated {
                if !claims
                    .iter()
                    .any(|c| c.feature == m.feature && c.parameter == m.parameter)
                {
                    claims.push(m);
                }
            }
        }
    }
    out
}
