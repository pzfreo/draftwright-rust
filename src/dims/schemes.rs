//! Replace automatic location ownership with an authored, acyclic station network.
use super::{AXES, Measurement, Shape, Want, fmt};
use crate::{
    dimension_scheme::{invalid, station},
    model::{Analysis, Invalid, Spec},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Covered = BTreeMap<String, String>;

fn eligible(w: &Want, k: usize) -> bool {
    matches!(w.shape, Shape::Linear { axis, order: 1, .. } if axis == k)
        || (matches!(w.shape, Shape::Linear { axis, .. } if axis == k)
            && w.measurements
                .iter()
                .any(|m| m.parameter == "step.length" || m.parameter.contains("pitch.length")))
}
fn anchor(a: &Analysis, w: &Want, p: [f64; 3], k: usize) -> Option<String> {
    let mut keys = Vec::new();
    for m in &w.measurements {
        let mut members = BTreeSet::new();
        if expand(a, &m.feature, &mut members).is_ok() {
            keys.extend(members);
        }
        keys.extend([
            m.feature.clone(),
            format!("start:{}", m.feature),
            format!("end:{}", m.feature),
        ]);
    }
    keys.extend(["min".into(), "max".into()]);
    keys.extend(a.dimension_references().keys().cloned());
    keys.into_iter()
        .find(|r| station(a, r, k).is_ok_and(|q| (p[k] - q[k]).abs() < 1e-6))
}
fn expand(a: &Analysis, r: &str, out: &mut BTreeSet<String>) -> Result<(), Invalid> {
    if let Some(p) = a
        .recognition
        .as_ref()
        .and_then(|r0| r0.patterns.iter().find(|p| p.id == r))
    {
        if p.arrangement == "bolt_circle" {
            return Err(invalid(format!(
                "{r} is circular; retain its PCD scheme or select individual member IDs for Cartesian locations."
            )));
        }
        out.extend(p.members.iter().cloned());
    } else if let Some(f) = a
        .features
        .iter()
        .find(|f| f.id == r && f.kind == "slot_pattern")
    {
        let members = f
            .parameters
            .get("slots")
            .and_then(|v| v.as_array())
            .ok_or_else(|| invalid(format!("{r} has no member evidence; analyse again.")))?;
        out.extend(members.iter().filter_map(|v| v.as_str().map(String::from)));
    } else if a
        .features
        .iter()
        .any(|f| f.id == r && f.kind == "turned_step")
    {
        out.insert(format!("start:{r}"));
        out.insert(format!("end:{r}"));
    } else {
        out.insert(r.into());
    }
    Ok(())
}

pub(super) fn apply(a: &Analysis, s: &Spec, wants: &mut Vec<Want>) -> Result<Covered, Invalid> {
    let mut covered = Covered::new();
    if s.dimension_schemes.is_empty() {
        return Ok(covered);
    }
    crate::dimension_scheme::check(s)?;
    let old = wants.clone();
    let mut occupied: Vec<(usize, &'static str, f64, String)> = Vec::new();
    for (name, d) in &s.dimension_schemes {
        let fail = |why: String| {
            invalid(format!(
                "dimension_schemes/{name}: {why} Correct the reference or remove dimension_schemes/{name}."
            ))
        };
        let k = AXES.iter().position(|&k| k == d.axis).unwrap();
        if s.binding
            .as_ref()
            .is_some_and(|b| b.source_sha256 != a.binding.source_sha256)
            && d.targets.iter().chain(&d.from).any(|r| r.contains("f:"))
        {
            return Err(fail("Feature IDs belong to another STEP revision; replace them with current fingerprint references or explicitly rebind the Spec.".into()));
        }
        if a.solid_count.is_some_and(|n| n > 1) {
            return Err(fail("A scheme needs one carrying solid; select a single part before authoring its station network.".into()));
        }
        let mut refs = BTreeSet::new();
        for r in &d.targets {
            expand(a, r, &mut refs)?;
        }
        let selected: Vec<_> = old
            .iter()
            .filter(|w| {
                eligible(w, k)
                    && d.view.as_deref().is_none_or(|v| w.views.contains(&v))
                    && (d.targets.is_empty()
                        || w.measurements.iter().any(|m| {
                            d.targets.contains(&m.feature)
                                || refs.contains(&m.feature)
                                || refs.contains(&format!("start:{}", m.feature))
                        }))
            })
            .collect();
        if d.targets.is_empty() {
            for w in &selected {
                for m in &w.measurements {
                    if a.recognition
                        .as_ref()
                        .is_some_and(|r| r.patterns.iter().any(|p| p.id == m.feature))
                        || a.features
                            .iter()
                            .any(|f| f.id == m.feature && f.kind == "slot_pattern")
                    {
                        // A counted pitch's drawn span is only one interval. Replacing it
                        // must still locate every physical member, including the far end.
                        let mut members = BTreeSet::new();
                        if expand(a, &m.feature, &mut members).is_ok() {
                            refs.extend(members);
                        }
                    }
                }
                if let Shape::Linear { a: p, b: q, .. } = w.shape {
                    let ends: &[[f64; 3]] = if w.measurements.iter().any(|m| {
                        m.parameter == "step.length" || m.parameter.contains("pitch.length")
                    }) {
                        &[p, q]
                    } else {
                        &[q]
                    };
                    for &p in ends {
                        refs.insert(anchor(a,w,p,k).ok_or_else(||fail(format!("{} has no stable station reference; analyse again or name explicit targets.", w.id)))?);
                    }
                }
            }
        }
        if refs.is_empty() {
            return Err(fail("This view axis has no location dimensions; name feature or fingerprint targets instead.".into()));
        }
        let datum = d.from.as_deref().unwrap_or("min");
        let origin = station(a, datum, k).map_err(fail)?;
        let view = d
            .view
            .as_deref()
            .or_else(|| selected.first().and_then(|w| w.views.first().copied()))
            .unwrap_or(if k == 1 { "plan" } else { "front" });
        let view = *super::NAMES.iter().find(|&&n| n == view).unwrap();
        if super::TOWARD.iter().find(|(v, _)| *v == view).unwrap().1[k].abs() > 0.5 {
            return Err(fail(format!(
                "The {view} view looks along {}; choose a principal view showing that axis across the page.",
                d.axis
            )));
        }
        let mut sites: Vec<_> = refs
            .iter()
            .map(|r| station(a, r, k).map(|p| (r.clone(), p)).map_err(fail))
            .collect::<Result<_, _>>()?;
        sites.push((datum.into(), origin));
        // Coincident stations share one witness: use the member nearest the ordinary
        // exterior strip, so an extension does not traverse another row of the pattern.
        let (cross, high) = match (view, k) {
            ("plan", 0) => (1, true),
            ("plan", _) => (0, false),
            ("front", 0) | ("side", 1) => (2, false),
            ("front", _) => (0, true),
            _ => (1, true),
        };
        sites.sort_by(|a, b| {
            a.1[k]
                .total_cmp(&b.1[k])
                .then_with(|| {
                    if high {
                        b.1[cross].total_cmp(&a.1[cross])
                    } else {
                        a.1[cross].total_cmp(&b.1[cross])
                    }
                })
                .then(a.0.cmp(&b.0))
        });
        sites.dedup_by(|a, b| (a.1[k] - b.1[k]).abs() < 1e-6);
        if sites.len() < 2 {
            return Err(fail(
                "All targets coincide with the datum; choose at least one distinct station.".into(),
            ));
        }
        for (_, p) in &sites {
            if let Some((_, _, _, other)) = occupied.iter().find(|(j, v, x, _)| {
                *j == k && *v == view && (*x - p[k]).abs() < 1e-6 && (*x - origin[k]).abs() > 1e-6
            }) {
                return Err(fail(format!(
                    "This location is already owned by scheme {other}; combine the entries or remove the overlap."
                )));
            }
            occupied.push((k, view, p[k], name.clone()));
        }
        let mut pairs = Vec::new();
        if d.scheme == "chain" {
            pairs.extend(sites.windows(2).map(|p| (p[0].clone(), p[1].clone())));
        } else {
            pairs.extend(
                sites
                    .iter()
                    .filter(|(_, p)| (p[k] - origin[k]).abs() > 1e-6)
                    .map(|p| ((datum.to_string(), origin), p.clone())),
            );
        }
        let mut generated = Vec::new();
        for ((from, p), (to, q)) in pairs {
            let hash = Sha256::digest(to.as_bytes());
            let token: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
            let id = format!("d:scheme:{name}/{token}");
            let value = if d.scheme == "ordinate" {
                q[k] - p[k]
            } else {
                (q[k] - p[k]).abs()
            };
            let claim = format!("scheme:{name}|{datum}|{from}|{to}");
            let mut measurements = vec![Measurement {
                feature: claim,
                parameter: format!("{}.{}", d.scheme, d.axis),
                value,
            }];
            // Attachment provenance uses existing feature ownership for witness routing.
            // It carries no size/location role and grants no manufacturing coverage.
            for reference in [&from, &to] {
                let feature = reference
                    .strip_prefix("start:")
                    .or_else(|| reference.strip_prefix("end:"))
                    .or_else(|| reference.strip_prefix("axis:"))
                    .unwrap_or(reference);
                if a.features.iter().any(|f| f.id == feature) {
                    measurements.push(Measurement {
                        feature: feature.into(),
                        parameter: format!("scheme_witness.{name}.{token}"),
                        value,
                    });
                }
            }
            generated.push(Want {
                id, views:vec![view], value, terms:vec![fmt(value)], measurements,
                basis:format!("spec dimension_schemes/{name}: {} along {} from {from} to {to}; one station network owns these locations",d.scheme,d.axis),
                shape:Shape::Linear {a:p,b:q,axis:k,a_edge:from=="min"||from=="max",b_edge:to=="min"||to=="max",order:1},
                turning:None
            });
        }
        let on = |x: f64| sites.iter().any(|(_, p)| (p[k] - x).abs() < 1e-6);
        // Any existing interval determined by this network yields its ownership, including
        // coincident locations outside the named set and the overall that would close it.
        for w in &old {
            let Shape::Linear {
                a: p, b: q, axis, ..
            } = w.shape
            else {
                continue;
            };
            // An internal feature datum still needs its own one location to the part;
            // otherwise the entire network could float within the envelope.
            if eligible(w, k) && (q[k] - origin[k]).abs() < 1e-6 && !on(p[k]) {
                continue;
            }
            let pattern_centre = w.measurements.iter().any(|m| {
                (m.parameter.starts_with("location.length.")
                    || m.parameter.starts_with("location_slot_pattern.length."))
                    && d.targets.contains(&m.feature)
                    && (a
                        .recognition
                        .as_ref()
                        .is_some_and(|r| r.patterns.iter().any(|p| p.id == m.feature))
                        || a.features
                            .iter()
                            .any(|f| f.id == m.feature && f.kind == "slot_pattern"))
            });
            if (!eligible(w, k) && !w.id.starts_with("d:envelope/") && !w.id.starts_with("d:pmi:"))
                || axis != k
                || !w.views.contains(&view)
                || !(on(p[k]) && on(q[k])
                    || eligible(w, k) && (p[k] - a.envelope[0][k]).abs() < 1e-6 && on(q[k])
                    || pattern_centre)
            {
                continue;
            }
            if w.turning.is_some()
                || w.measurements
                    .iter()
                    .any(|m| m.parameter.contains("diameter"))
            {
                continue;
            }
            if w.id.starts_with("d:authored:") || w.id.starts_with("d:pmi:") {
                return Err(fail(format!(
                    "{} already owns an interval in this network; retain that authored/PMI owner and choose disjoint targets.",
                    w.id
                )));
            }
            if s.precision.contains_key(&w.id) {
                return Err(fail(format!(
                    "{} has explicit precision; remove that policy before replacing its owner, then apply precision to the generated ID.",
                    w.id
                )));
            }
            if let Some(e) = s.dimensions.get(&w.id)
                && (e.tolerance.is_some()
                    || e.fit.is_some()
                    || e.place.is_some()
                    || e.view.is_some()
                    || e.omit)
            {
                return Err(fail(format!(
                    "{} has explicit dimension intent; remove that edit before replacing its ownership, then apply tolerance/place to the generated owner.",
                    w.id
                )));
            }
            if covered.contains_key(&w.id) {
                return Err(fail(format!(
                    "{} is already owned by another scheme; combine the schemes.",
                    w.id
                )));
            }
            covered.insert(w.id.clone(), generated.last().unwrap().id.clone());
        }
        wants.extend(generated);
    }
    Ok(covered)
}

pub(super) fn from_spec(w: &Want) -> Option<String> {
    w.measurements.iter().find_map(|m| {
        m.feature
            .strip_prefix("scheme:")?
            .split_once('|')
            .map(|(n, _)| format!("dimension_schemes/{n}"))
    })
}

/// Running ordinates use an origin circle and one endpoint arrow. Placement reserves the
/// conservative two-arrow envelope first, so changing the ink cannot steal a neighbour's room.
pub(super) fn ink(w: &Want, mut p: super::Placed) -> super::Placed {
    if !w
        .measurements
        .iter()
        .any(|m| m.feature.starts_with("scheme:") && m.parameter.starts_with("ordinate."))
        || p.arrows.len() != 2
        || p.lines.is_empty()
    {
        return p;
    }
    let near = p.lines[0][1];
    let distance = |q: [f64; 2]| (q[0] - near[0]).powi(2) + (q[1] - near[1]).powi(2);
    let i = usize::from(distance(p.arrows[1][0]) < distance(p.arrows[0][0]));
    let origin = p.arrows.remove(i)[0];
    for n in 0..16 {
        let at = |j: usize| {
            let t = j as f64 * std::f64::consts::TAU / 16.;
            [origin[0] + 0.7 * t.cos(), origin[1] + 0.7 * t.sin()]
        };
        p.lines.push([at(n), at(n + 1)]);
    }
    p
}

/// #78 dimensions may not close an authored network through a different pair of endpoints.
pub(super) fn check_authored(s: &Spec, wants: &[Want]) -> Result<(), Invalid> {
    if s.dimension_schemes.is_empty() {
        return Ok(());
    }
    for w in wants.iter().filter(|w| {
        w.id.starts_with("d:authored:") && !s.dimensions.get(&w.id).is_some_and(|d| d.omit)
    }) {
        let interval = |shape: &Shape| match shape {
            Shape::Linear { a, b, axis, .. } => Some((*axis, a[*axis], b[*axis])),
            Shape::Aligned { a, b } => (0..3)
                .find(|&k| (0..3).all(|j| j == k || (a[j] - b[j]).abs() < 1e-8))
                .map(|k| (k, a[k], b[k])),
            _ => None,
        };
        let Some((k, lo, hi)) = interval(&w.shape) else {
            continue;
        };
        for (name, d) in &s.dimension_schemes {
            let edges: Vec<_> = wants
                .iter()
                .enumerate()
                .filter(|(_, o)| o.id.starts_with(&format!("d:scheme:{name}/")))
                .filter_map(|(i, o)| {
                    let (axis, a, b) = interval(&o.shape)?;
                    (axis == k).then_some((crate::lint::redundancy::Span::new(k, a, b), i))
                })
                .collect();
            if crate::lint::redundancy::path(&edges, crate::lint::redundancy::Span::new(k, lo, hi))
                .is_some()
            {
                return Err(invalid(format!(
                    "{} closes the station network dimension_schemes/{name} ({}); remove that authored dimension or choose disjoint scheme targets.",
                    w.id, d.scheme
                )));
            }
        }
    }
    Ok(())
}
