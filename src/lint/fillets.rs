//! Fillet coverage (Python's `linting/fillet_coverage.py`): every recognised fillet states its
//! radius (LINT-fillet_coverage-radius, LINT-code-fillet_requirement_state).
//!
//! Each of the Analysis's `fillet` features (quiddity's `Fillet`: one rounded edge, its `radius`,
//! `axis` and `at`) has one requirement, `fillet.radius`, judged by [`parameters::coverage`]: an
//! annotation that claims it must print the radius. Python joins quiddity's record to its own IR
//! feature by exact geometry before following the measurement; here the Analysis feature is the
//! record, so there is no join to fail.

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark};
use crate::bridge::holes::frame;
use crate::model::Analysis;

/// The fillets' requirements.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    analysis
        .features
        .iter()
        .filter(|f| {
            matches!(f.kind.as_str(), "fillet" | "cylindrical_profile")
                && !super::blends::is_relief(analysis, f)
        })
        .map(|f| {
            let radius = parameters::size(f, "radius");
            Wanted {
                feature: f,
                parameter: if f.kind == "fillet" {
                    "fillet.radius"
                } else {
                    "cylindrical_profile.radius"
                }
                .into(),
                callout: radius
                    .as_ref()
                    .map_or("R?".into(), |r| format!("R{}", fmt(*r))),
                values: radius.map(|r| vec![r]),
            }
        })
        .collect()
}

/// The fillet coverage findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &crate::bridge::Feature| {
        let p = &f.parameters;
        let mut s = format!("fillet {}", f.id);
        if let Some(r) = p.get("radius").and_then(|v| v.as_f64()) {
            s += &format!(" R{}", fmt(r));
        }
        if let Some(a) = parameters::direction(&frame, p.get("axis")) {
            s += &format!(" along {}", fmt3(a));
        }
        if let Some(at) = parameters::place(&frame, p.get("at")) {
            s += &format!(" at {}", fmt3(at));
        }
        s
    };
    parameters::coverage(
        "fillet",
        &wanted(analysis),
        marks,
        noun,
        "a radius callout, in a view that sees the fillet's arc",
    )
}
