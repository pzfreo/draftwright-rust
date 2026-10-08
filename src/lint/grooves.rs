//! Groove coverage (Python's `linting/groove_coverage.py`): every turned groove states its axial
//! width and its floor diameter (LINT-groove_coverage-width_and_floor,
//! LINT-code-groove_requirement_state).
//!
//! Each of the Analysis's `groove` features (quiddity's `Groove`: one annular recess, its `axis`,
//! `at`, `width` and floor `diameter`) has two requirements, `groove.length` (the width) and
//! `groove.diameter`, each judged by [`parameters::coverage`]: an annotation that claims it must
//! print the value (`10 WIDE × ø122` states both). Python joins quiddity's record to its own IR
//! feature by exact geometry before following the measurements; here the Analysis feature is the
//! record, so there is no join to fail. A groove whose record lacks a size has that requirement
//! reported unverifiable.

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark};
use crate::bridge::Feature;
use crate::bridge::holes::frame;
use crate::model::Analysis;

/// The grooves' requirements: the width, then the floor diameter, of each.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let mut out = Vec::new();
    for f in analysis.features.iter().filter(|f| f.kind == "groove") {
        let width = parameters::size(f, "width");
        out.push(Wanted {
            feature: f,
            parameter: "groove.length".into(),
            callout: width
                .as_ref()
                .map_or("its width".into(), |w| format!("{} WIDE", fmt(*w))),
            values: width.map(|w| vec![w]),
        });
        let diameter = parameters::size(f, "diameter");
        out.push(Wanted {
            feature: f,
            parameter: "groove.diameter".into(),
            callout: diameter
                .as_ref()
                .map_or("its floor diameter".into(), |d| format!("ø{}", fmt(*d))),
            values: diameter.map(|d| vec![d]),
        });
    }
    out
}

/// The groove coverage findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let p = &f.parameters;
        let mut s = format!("groove {}", f.id);
        if let (Ok(w), Ok(d)) = (
            parameters::size(f, "width"),
            parameters::size(f, "diameter"),
        ) {
            s += &format!(" {} wide, ø{}", fmt(w), fmt(d));
        }
        if let Some(a) = parameters::direction(&frame, p.get("axis")) {
            s += &format!(", axis {}", fmt3(a));
        }
        if let Some(at) = parameters::place(&frame, p.get("at")) {
            s += &format!(", at {}", fmt3(at));
        }
        s
    };
    parameters::coverage(
        "groove",
        &wanted(analysis),
        marks,
        noun,
        "a groove callout (width and floor diameter), in a view that shows the turned profile",
    )
}

#[cfg(test)]
mod tests {
    use super::super::paired_ramp_steps::tests::part;
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    fn mark(text: &str, parameters: &[&str]) -> Mark {
        Mark {
            name: "m_groove_z0".into(),
            view: Some("v:front".into()),
            text: text.into(),
            claims: parameters
                .iter()
                .map(|p| Claim {
                    feature: "f:groove:1".into(),
                    parameter: p.to_string(),
                })
                .collect(),
        }
    }

    /// connector109's first groove (quiddity 0.3.9): `10 WIDE × ø122` states both requirements;
    /// a callout of the width alone leaves the diameter missing; a misprinted floor diameter is
    /// unverifiable; a record without its width cannot be checked.
    #[test]
    fn a_groove_states_its_width_and_floor_diameter() {
        let record = json!({ "at": [-116.39, 0.024, 0.003], "axis": "x", "diameter": 122.0,
                             "width": 10.0 });
        let a = part("groove", record.clone());
        let both = ["groove.length", "groove.diameter"];
        assert!(coverage(&a, &[mark("10 WIDE × ø122", &both)]).is_empty());
        let codes = |a: &Analysis, marks: &[Mark]| -> Vec<String> {
            coverage(a, marks)
                .into_iter()
                .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
                .collect()
        };
        assert_eq!(
            codes(&a, &[mark("10 WIDE", &["groove.length"])]),
            ["groove_requirement_missing groove.diameter"]
        );
        assert_eq!(
            codes(&a, &[mark("10 WIDE × ø120", &both)]),
            ["groove_requirement_unverifiable groove.diameter"]
        );
        let mut bad = record;
        bad.as_object_mut().unwrap().remove("width");
        assert_eq!(
            codes(&part("groove", bad), &[mark("10 WIDE × ø122", &both)]),
            ["groove_requirement_unverifiable groove.length"]
        );
        let issue = &coverage(&a, &[])[0];
        assert_eq!(
            issue.message,
            "groove f:groove:1 10 wide, ø122, axis (1, 0, 0), at (-116.39, 0.024, 0.003): \
             requirement groove.length (10 WIDE) is not stated on the sheet"
        );
    }
}
