//! Re-read notch support geometry; printed nominals, never tolerance digits, prove claims.
use super::{LintIssue, Mark, holes};
use crate::model::Analysis;
pub(super) fn coverage(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let mut out = Vec::new();
    for n in super::notch_evidence::requirements(a) {
        let values = [
            ("notch.angle".to_string(), n.angle.degrees()),
            ("notch_depth.length".to_string(), n.depth()),
            (
                format!("location.length.{}", ["x", "y", "z"][n.width_axis]),
                (n.mouth[n.width_axis] - a.envelope[0][n.width_axis]).abs(),
            ),
            (
                format!("location.length.{}", ["x", "y", "z"][n.depth_axis]),
                (n.mouth[n.depth_axis] - a.envelope[0][n.depth_axis]).abs(),
            ),
        ];
        for (parameter, value) in values {
            if value < 1e-6 {
                continue;
            }
            let claimed: Vec<_> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.feature == n.id && c.parameter == parameter)
                })
                .collect();
            if claimed.iter().any(|m| {
                let mut words = m.text.split_whitespace();
                let first = words.next().unwrap_or("");
                let nominal = if first.ends_with('×') {
                    words.next().unwrap_or("")
                } else {
                    first
                };
                (parameter != "notch.angle" || nominal.ends_with('°'))
                    && holes::states(nominal, value)
            }) {
                continue;
            }
            let code = if claimed.is_empty() {
                "notch_requirement_missing"
            } else {
                "notch_requirement_unverifiable"
            };
            let mut issue = super::uncovered(
                a,
                code,
                &n.id,
                &parameter,
                format!(
                    "V-notch {} requires {value} for {parameter}, measured from its ordered supports.",
                    n.id
                ),
            );
            issue.annotation = claimed.first().map(|m| m.name.clone());
            out.push(issue);
        }
    }
    out
}
