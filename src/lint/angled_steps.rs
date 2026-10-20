//! Angled step disposition (Python's `linting/angled_step_coverage.py`): every recognised angled
//! blind step is reported as not represented by the drawing, so its angle, legs and run are
//! reviewed outside automatic approval (LINT-code-angled_step_requirement_unsupported).
//!
//! Each of the Analysis's `angled_step` features (quiddity's `AngledStep`: `axis`, `leg1`,
//! `leg2`, `angle`, the run `length` and `at`) gives one `angled_step_requirement_unsupported`
//! warning, whatever the marks say: draftwright has no grammar to dimension one, so no
//! annotation can be judged to state it. As in Python, the finding names no requirement.

use super::parameters::{self, fmt, fmt3};
use super::{LintIssue, Mark, Severity};
use crate::bridge::holes::frame;
use crate::model::Analysis;

/// The angled step findings, one per step ("k of N"), in Analysis order.
pub fn coverage(analysis: &Analysis, _marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let steps: Vec<_> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "angled_step")
        .collect();
    let total = steps.len();
    steps
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let p = &f.parameters;
            let number = |name: &str| parameters::size(f, name).map_or("?".into(), fmt);
            let mut message = format!(
                "angled step {}: recognised angled blind step at {} degrees with {} mm run \
                 ({} of {total})",
                f.id,
                number("angle"),
                number("length"),
                i + 1
            );
            if let Some(at) = parameters::place(&frame, p.get("at")) {
                message += &format!(", at {}", fmt3(at));
            }
            message += " is not represented by Draftwright dimensions; review and define its \
                        manufacturing geometry outside automatic drawing approval";
            let code = "angled_step_requirement_unsupported";
            LintIssue {
                sheet: None,
                id: format!("l:{code}:{}", f.id),
                code: code.into(),
                severity: Severity::Warning,
                message,
                at: None,
                view: None,
                subjects: vec![f.id.clone()],
                requirements: Vec::new(),
                measurements: Vec::new(),
                sources: Vec::new(),
                stage: None,
                annotation: None,
                related: Vec::new(),
                evidence: None,
                fix: Some(format!(
                    "Review and dimension the angle, legs and run of {} by hand.",
                    crate::wording::feature(f)
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::paired_ramp_steps::tests::part;
    use super::*;
    use serde_json::json;

    /// An angled step is always reported, claimed or not: its geometry has no drafting grammar.
    #[test]
    fn an_angled_step_is_reported_unsupported() {
        let a = part(
            "angled_step",
            json!({ "axis": "y", "leg1": 3.0, "leg2": 2.0, "angle": 33.69, "length": 15.0,
                    "at": [1.0, 7.5, 9.0] }),
        );
        let issues = coverage(&a, &[]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "angled_step_requirement_unsupported");
        assert_eq!(
            issues[0].message,
            "angled step f:angled_step:1: recognised angled blind step at 33.69 degrees with 15 \
             mm run (1 of 1), at (1, 7.5, 9) is not represented by Draftwright dimensions; \
             review and define its manufacturing geometry outside automatic drawing approval"
        );
        assert!(issues[0].requirements.is_empty());
    }
}
