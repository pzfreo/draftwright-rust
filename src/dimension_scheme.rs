//! Model-space station references shared by scheme planning and independent ink verification.
use crate::model::{Analysis, Invalid, Spec, dimension_geometry::Reference};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scheme {
    pub scheme: String,
    pub axis: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

pub(crate) fn invalid(message: impl Into<String>) -> Invalid {
    Invalid {
        message: message.into(),
        choices: vec![],
    }
}

pub(crate) fn check(s: &Spec) -> Result<(), Invalid> {
    for (name, d) in &s.dimension_schemes {
        let fail = |why| {
            invalid(format!(
                "dimension_schemes/{name}: {why}; correct this entry or remove it."
            ))
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(fail("use a stable name containing letters, digits, _ or -"));
        }
        if !["chain", "baseline", "ordinate"].contains(&d.scheme.as_str()) {
            return Err(fail("scheme must be chain, baseline or ordinate"));
        }
        if !["x", "y", "z"].contains(&d.axis.as_str()) {
            return Err(fail("axis must be x, y or z in the part frame"));
        }
        if d.view
            .as_deref()
            .is_some_and(|v| !["front", "plan", "side"].contains(&v))
        {
            return Err(fail("view must be front, plan or side"));
        }
        if d.targets.is_empty() && d.view.is_none() {
            return Err(fail("name targets, or a view for its whole axis"));
        }
        if d.targets.iter().any(String::is_empty) || d.from.as_deref() == Some("") {
            return Err(fail(
                "references cannot be empty; use feature IDs or fingerprint keys from Analysis.edge_ink.references",
            ));
        }
    }
    Ok(())
}

/// A reference's station and physical witness. No Spec coordinate is accepted.
pub(crate) fn station(a: &Analysis, key: &str, k: usize) -> Result<[f64; 3], String> {
    if key == "min" || key == "max" {
        let mut p = a.envelope[0];
        p[k] = a.envelope[usize::from(key == "max")][k];
        return Ok(p);
    }
    if let Some((end, id)) = key.split_once(':')
        && ["start", "end"].contains(&end)
    {
        let f = a.features.iter().find(|f| f.id == id).ok_or_else(|| {
            format!("Unknown feature {id}; choose a current Analysis feature ID.")
        })?;
        let b = crate::lint::turned::band(&crate::bridge::holes::frame(a), f)?;
        if b.axis != k {
            return Err(format!(
                "{key} is not a shoulder on the chosen axis; choose the feature's turning axis."
            ));
        }
        let mut p = b.on;
        p[k] = if end == "start" { b.lo } else { b.hi };
        // Witnesses start on the outside of the profile, rather than its centre line.
        let radial = (0..3).find(|&j| j != k).unwrap();
        p[radial] += b.diameter / 2.;
        return Ok(p);
    }
    if let Some(f) = a.features.iter().find(|f| f.id == key) {
        if f.kind == "turned_step" {
            return Err(format!(
                "{key} has two shoulders; choose start:{key} or end:{key}."
            ));
        }
        if matches!(f.kind.as_str(), "slot" | "slot_pattern") {
            return Ok(f.fingerprint.position);
        }
        // A planar datum feature must prove one plane perpendicular to the measuring axis.
        let planes: Vec<_> = f
            .faces
            .iter()
            .filter_map(|&i| a.face_attachments.get(i))
            .filter_map(|f| f.bounds.zip(f.normal))
            .filter(|(_, n)| n[k].abs() > 1. - 1e-8)
            .map(|(b, _)| [0, 1, 2].map(|j| (b[j] + b[j + 3]) / 2.))
            .collect();
        if let Some(p) = planes.first() {
            if planes.iter().any(|q| (p[k] - q[k]).abs() > 1e-6) {
                return Err(format!(
                    "{key} has multiple datum planes; choose a face:v1:<fingerprint> instead."
                ));
            }
            return Ok(*p);
        }
        if f.fingerprint.axis.is_some_and(|axis| axis[k].abs() > 1e-8) {
            return Err(format!(
                "{key}'s axis does not define a station across {}; choose a perpendicular axis or planar face.",
                ["x", "y", "z"][k]
            ));
        }
        return Ok(f.fingerprint.position);
    }
    match a.dimension_reference(key)? {
        Reference::Plane {
            origin,
            normal,
            bounds,
        } if normal[k].abs() > 1. - 1e-8 => {
            let mut p = [0, 1, 2].map(|j| (bounds[0][j] + bounds[1][j]) / 2.);
            p[k] = origin[k];
            Ok(p)
        }
        Reference::Vertex { point } => Ok(point),
        Reference::Edge { ends } if (ends[0][k] - ends[1][k]).abs() < 1e-8 => {
            Ok([0, 1, 2].map(|j| (ends[0][j] + ends[1][j]) / 2.))
        }
        Reference::Cylinder {
            origin,
            axis,
            bounds,
            ..
        } if axis[k].abs() < 1e-8 => {
            let centre = [0, 1, 2].map(|j| (bounds[0][j] + bounds[1][j]) / 2.);
            let t: f64 = (0..3).map(|j| (centre[j] - origin[j]) * axis[j]).sum();
            Ok([0, 1, 2].map(|j| origin[j] + t * axis[j]))
        }
        _ => Err(format!(
            "{key} does not define one station on this axis; choose a perpendicular planar face, a transverse cylinder axis, vertex or constant-axis edge."
        )),
    }
}
