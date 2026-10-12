//! Bounded completeness: Analysis face claims and recognised features without drawn dimensions.
//! Neither count proves recognition recall or manufacturing readiness.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::items::State;
use crate::model::{Analysis, Plan, Problem};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Clear,
    #[default]
    NeedsAttention,
}

pub fn status(problems: &[Problem]) -> Status {
    if problems.iter().any(|p| p.severity != "info") {
        Status::NeedsAttention
    } else {
        Status::Clear
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Completeness {
    pub claimed_faces: usize,
    pub faces: usize,
    pub unplanned_features: usize,
}

/// Refresh after escalation, which can move a hole's measurements into a table.
pub fn report(analysis: &Analysis, plan: &mut Plan) {
    let severity = if analysis.solid_count.is_some_and(|n| n > 1) {
        "info"
    } else {
        "warning"
    };
    let claimed: BTreeSet<usize> = analysis
        .features
        .iter()
        .flat_map(|f| f.faces.iter().copied())
        .filter(|&i| i < analysis.binding.face_count)
        .collect();
    let mut stated: BTreeSet<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some() || matches!(d.state, State::Covered { .. }))
        .flat_map(|d| d.measurements.iter().map(|m| m.feature.as_str()))
        .collect();
    // Radius coverage may be carried by a general note or a located radius table.
    stated.extend(plan.notes.iter().filter(|n| n.placed.is_some())
        .flat_map(|n| &n.measurements)
        .filter(|m| matches!(m.parameter.as_str(), "blend.radius" | "fillet.radius"))
        .map(|m| m.feature.as_str()));
    stated.extend(plan.radius_table.iter().flat_map(|t| &t.rows).map(|r| r.feature.as_str()));
    if plan.notes.iter().any(|n| n.id == crate::lint::blends::RELIEF_NOTE && n.placed.is_some()) {
        stated.extend(analysis.features.iter().filter(|f| crate::lint::blends::is_relief(analysis, f)).map(|f| f.id.as_str()));
    }
    for p in analysis.recognition.iter().flat_map(|r| &r.patterns) {
        if stated.contains(p.id.as_str()) {
            stated.extend(p.members.iter().map(String::as_str));
        }
    }
    for pattern in analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot_pattern")
    {
        if stated.contains(pattern.id.as_str())
            && let Some(members) = pattern.parameters.get("slots").and_then(|v| v.as_array())
        {
            stated.extend(members.iter().filter_map(|m| m.as_str()));
        }
    }
    let bands = crate::lint::turned::bands(analysis);
    let unplanned: Vec<String> = analysis
        .features
        .iter()
        .filter(|f| {
            if stated.contains(f.id.as_str())
                || crate::dims::UNDRAFTED.contains(&f.fingerprint.subtype.as_str())
            {
                return false;
            }
            if f.kind == "boss"
                && crate::lint::bosses::groove_floor(analysis, f)
                && analysis.features.iter().any(|g| {
                    g.kind == "groove"
                        && stated.contains(g.id.as_str())
                        && g.faces.iter().any(|i| f.faces.contains(i))
                })
            {
                return false;
            }
            if f.kind == "turned_step"
                && bands
                    .as_ref()
                    .is_ok_and(|bs| !bs.iter().any(|b| b.id == f.id))
                && analysis.features.iter().any(|g| {
                    g.kind == "groove"
                        && stated.contains(g.id.as_str())
                        && g.faces.iter().any(|i| f.faces.contains(i))
                })
            {
                return false;
            }
            true
        })
        .map(|f| f.id.clone())
        .collect();
    plan.completeness = Completeness {
        claimed_faces: claimed.len(),
        faces: analysis.binding.face_count,
        unplanned_features: unplanned.len(),
    };
    plan.problems
        .retain(|p| p.code != "unclaimed_geometry" && !p.id.starts_with("p:feature_unplanned:"));
    for p in &mut plan.problems {
        if matches!(
            p.code.as_str(),
            "dimensions_not_ported" | "feature_not_dimensioned"
        ) {
            p.severity = severity.into();
        }
    }
    if claimed.len() < analysis.binding.face_count {
        plan.problems.push(Problem {
            id: "p:unclaimed_geometry:sheet".into(), code: "unclaimed_geometry".into(),
            severity: severity.into(),
            message: format!("Analysis features claim {} of {} faces; {} faces are unclaimed. Review the geometry: ordinary stock faces may also be unclaimed; this is not a count of missed features.", claimed.len(), analysis.binding.face_count, analysis.binding.face_count - claimed.len()),
            subjects: vec!["sheet".into()], ..Default::default()
        });
    }
    for id in unplanned {
        if !plan.problems.iter().any(|p| {
            p.subjects.contains(&id)
                && matches!(
                    p.code.as_str(),
                    "dimensions_not_ported"
                        | "feature_not_dimensioned"
                        | "angled_step_requirement_unsupported"
                )
        }) {
            plan.problems.push(Problem {
                id: format!("p:feature_unplanned:{id}"), code: "feature_not_dimensioned".into(),
                severity: severity.into(), message: format!("{id} has no dimension drawn or carried by the hole table; review its omitted or undrawable items."),
                subjects: vec![id], ..Default::default()
            });
        }
    }
    plan.status = status(&plan.problems);
}
