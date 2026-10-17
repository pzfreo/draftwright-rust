//! Re-measure every printed scheme link and prove connectivity before granting derived roles.
//! No Spec or planner output is read: losing one link loses its coverage.
use super::{
    LintIssue, Mark, Severity, holes,
    redundancy::{Span, path},
};
use crate::{dimension_scheme::station, model::Analysis, output::Claim};
use std::collections::BTreeMap;

type Graph = (String, usize, Option<String>, String);

pub(crate) fn evidence(a: &Analysis, marks: &[Mark]) -> (Vec<Mark>, Vec<LintIssue>) {
    if !marks
        .iter()
        .any(|m| m.claims.iter().any(|c| c.feature.starts_with("scheme:")))
    {
        return (Vec::new(), Vec::new());
    }
    let mut graphs: BTreeMap<Graph, Vec<(Span, usize)>> = BTreeMap::new();
    let mut issues = Vec::new();
    for (i, m) in marks.iter().enumerate() {
        for c in &m.claims {
            let Some(key) = c.feature.strip_prefix("scheme:") else {
                continue;
            };
            let fields: Vec<_> = key.split('|').collect();
            let measured = (|| {
                let [name, datum, from, to] = fields.as_slice() else {
                    return None;
                };
                let (style, axis) = c.parameter.split_once('.')?;
                if !["chain", "baseline", "ordinate"].contains(&style) {
                    return None;
                }
                let k = ["x", "y", "z"].iter().position(|&a| a == axis)?;
                station(a, datum, k).ok()?;
                let p = station(a, from, k).ok()?;
                let q = station(a, to, k).ok()?;
                let value = if style == "ordinate" {
                    q[k] - p[k]
                } else {
                    (q[k] - p[k]).abs()
                };
                let nominal = m.text.split_whitespace().next()?;
                let decimals = nominal
                    .split_once('.')
                    .map_or(0, |(_, digits)| digits.len());
                let truthful = nominal.parse::<f64>().is_ok_and(|n| {
                    n.is_finite()
                        && (n - value).abs() <= 0.5 * 10f64.powi(-(decimals as i32)) + 1e-9
                });
                if !truthful || m.text.starts_with('(') {
                    return None;
                }
                Some((
                    name.to_string(),
                    datum.to_string(),
                    Span::new(k, p[k], q[k]),
                ))
            })();
            let Some((name, datum, span)) = measured else {
                issues.push(LintIssue {id:format!("l:dimension_scheme_invalid:{}",m.name), code:"dimension_scheme_invalid".into(),severity:Severity::Error,
                    message:format!("{} does not state the measured distance between its scheme references; restore its nominal or correct the references and regenerate.",m.name),
                    annotation:Some(m.name.clone()), subjects:vec![m.name.clone()], view:m.view.clone(), sheet:None, at:None, requirements:vec![], measurements:vec![], sources:vec![],stage:None,related:vec![],evidence:None,fix:Some("Correct the scheme references or regenerate its measured ink.".into()) });
                continue;
            };
            let edges = graphs
                .entry((name, span.axis, m.view.clone(), datum))
                .or_default();
            if let Some(route) = path(edges, span) {
                issues.push(LintIssue {id:format!("l:closed_dimension_chain:{}",m.name),code:"closed_dimension_chain".into(),severity:Severity::Warning,
                    message:"The printed scheme closes an already dimensioned station chain; remove its redundant link.".into(),annotation:Some(m.name.clone()),
                    subjects:vec![m.name.clone()],related:route.iter().map(|&j|marks[j].name.clone()).collect(),sheet:None, at:None,view:m.view.clone(),requirements:vec![],measurements:vec![],sources:vec![],stage:None,evidence:None,fix:Some("Remove the redundant scheme link.".into())});
            }
            edges.push((span, i));
        }
    }
    let mut extra = Vec::new();
    for ((_, k, view, datum), mut edges) in graphs {
        // A different chosen datum remains related to the minimum end by the ordinary
        // controlling overall, or another truthful support-plane interval on the sheet.
        for (i, m) in marks.iter().enumerate() {
            if m.text.starts_with('(') {
                continue;
            }
            for c in &m.claims {
                if let Some(s) = super::redundancy::span(a, &c.feature, &c.parameter)
                    && s.axis == k
                    && holes::states(m.text.split_whitespace().next().unwrap_or(""), s.hi - s.lo)
                {
                    edges.push((s, i));
                }
            }
        }
        let mut roles = Vec::new();
        let parameter = super::envelope::PARAMETERS[k];
        roles.push((
            "envelope".to_string(),
            parameter.to_string(),
            Span::new(k, a.envelope[0][k], a.envelope[1][k]),
        ));
        for f in &a.features {
            if f.kind == "turned_step" {
                if let Ok(b) = super::turned::band(&crate::bridge::holes::frame(a), f)
                    && b.axis == k
                {
                    roles.push((f.id.clone(), "step.length".into(), Span::new(k, b.lo, b.hi)));
                }
            } else if f.fingerprint.axis.is_some_and(|axis| axis[k].abs() < 1e-8)
                && ["hole", "hole_pattern", "boss", "polygonal_boss"].contains(&f.kind.as_str())
            {
                roles.push((
                    f.id.clone(),
                    format!("location.length.{}", ["x", "y", "z"][k]),
                    Span::new(k, a.envelope[0][k], f.fingerprint.position[k]),
                ));
            }
            for role in ["location_slot.length", "location_slot_across.length"] {
                if let Some(s) = super::redundancy::span(a, &f.id, role)
                    && s.axis == k
                {
                    roles.push((f.id.clone(), role.into(), s));
                }
            }
        }
        // Slot-pattern requirements retain their vocabulary, but their location and pitch
        // can be determined by independently verified physical-member coordinates.
        for f in a.features.iter().filter(|f| f.kind == "slot_pattern") {
            let members: Vec<_> = f
                .parameters
                .get("slots")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .filter_map(|id| station(a, id, k).ok())
                .collect();
            let origin = station(a, &datum, k).expect("verified datum");
            if members.len() < 2
                || !members
                    .iter()
                    .all(|p| path(&edges, Span::new(k, origin[k], p[k])).is_some())
            {
                continue;
            }
            if let Ok(wanted) = super::slots::pattern_wanted(a, f) {
                for w in wanted {
                    let applies = w.parameter
                        == format!("location_slot_pattern.length.{}", ["x", "y", "z"][k])
                        || w.parameter
                            == format!("grid_pitch.length.{}", if k == 0 { "col" } else { "row" })
                        || w.parameter == "pitch.length"
                            && members.windows(2).any(|p| (p[0][k] - p[1][k]).abs() > 1e-6);
                    if applies && let Ok(values) = w.values {
                        extra.push(Mark {
                            name: format!("derived:{}/{}", f.id, w.parameter),
                            view: view.clone(),
                            text: values
                                .into_iter()
                                .map(crate::dims::fmt)
                                .collect::<Vec<_>>()
                                .join(" "),
                            claims: vec![Claim {
                                feature: f.id.clone(),
                                parameter: w.parameter,
                            }],
                        });
                    }
                }
            }
        }
        for (feature, parameter, span) in roles {
            let target = if parameter.starts_with("location") {
                let origin = station(a, &datum, k).expect("verified datum");
                Span::new(k, origin[k], span.hi)
            } else {
                span
            };
            if let Some(route) = path(&edges, target)
                && !route.is_empty()
                && route.iter().any(|&i| {
                    marks[i]
                        .claims
                        .iter()
                        .any(|c| c.feature.starts_with("scheme:"))
                })
                && !extra.iter().any(|m: &Mark| {
                    m.claims
                        .iter()
                        .any(|c| c.feature == feature && c.parameter == parameter)
                })
            {
                extra.push(Mark {
                    name: format!("derived:{feature}/{parameter}"),
                    view: view.clone(),
                    text: crate::dims::fmt(span.hi - span.lo),
                    claims: vec![Claim { feature, parameter }],
                });
            }
        }
    }
    (extra, issues)
}
