//! A measurement table's compiler-approved cell text and actual rendered runs cross the
//! boundary separately. Only matching, nonempty ink can carry its claims or physical riders.
//! This verifies presentation (including precision and tolerances); Analysis still supplies
//! the physical requirements and independently checks the nominal values and roles.
use super::{LintIssue, Severity};
use crate::output::{Annotation, Drawing};

pub fn text(drawing: &Drawing, annotation: &Annotation) -> String {
    annotation
        .runs
        .iter()
        .filter_map(|&i| drawing.text.get(i))
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn verified(annotation: &Annotation, text: &str) -> bool {
    let scheduled = annotation.name.starts_with("hole_schedule_")
        || annotation.name.starts_with("hole_table_plan/")
        || annotation.name.starts_with("t:radius-centres/");
    let Some(expected) = &annotation.approved_text else {
        return !scheduled;
    };
    let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    !text.trim().is_empty()
        && words(text) == words(expected)
        && annotation.claims == annotation.approved_claims
}

pub fn check(drawing: &Drawing) -> Vec<LintIssue> {
    drawing.annotations.iter().filter_map(|a| {
        let actual = text(drawing, a);
        if verified(a, &actual) { return None; }
        Some(LintIssue {
            id: format!("l:claimed_value_absent:{}", a.name), code: "claimed_value_absent".into(),
            severity: Severity::Warning,
            message: format!("{} does not render its compiler-approved schedule cell; its measurement claims are not credited", a.name),
            sheet: None, at: None, view: a.view.clone(),
            subjects: a.claims.iter().map(|c| c.feature.clone()).collect(),
            requirements: Vec::new(), measurements: a.claims.iter().map(|c| format!("{}/{}", c.feature, c.parameter)).collect(),
            sources: Vec::new(), stage: None, annotation: Some(a.name.clone()), related: Vec::new(),
            evidence: Some(format!("expected {:?} for {:?}, rendered {actual:?} claiming {:?}", a.approved_text, a.approved_claims, a.claims)),
            fix: Some("Restore the complete approved cell, including its precision, tolerance and wording.".into()),
        })
    }).collect()
}
