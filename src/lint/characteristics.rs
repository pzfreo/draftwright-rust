//! Compare the numbered source inventory with actual rendered carriers and rings.
use super::{LintIssue, Severity};
use crate::output::Drawing;
pub(super) fn check(d: &Drawing) -> Vec<LintIssue> {
    if !d.sheet.inspection_enabled {
        return Vec::new();
    }
    let mut out = Vec::new();
    for c in &d.sheet.characteristics {
        let drawn = c.drawn_item.as_ref().is_some_and(|id| {
            d.sheet.items.iter().any(|i| &i.name == id)
                || d.annotations
                    .iter()
                    .any(|a| &a.name == id || a.name.starts_with(&format!("{id}/")))
        });
        let (code, why) = if !drawn {
            ("characteristic_not_drawn", "has no rendered carrier")
        } else if c.balloon_ids.is_empty()
            || c.balloon_ids
                .iter()
                .any(|id| !d.sheet.items.iter().any(|i| &i.name == id))
        {
            (
                "characteristic_balloon_missing",
                "has no rendered inspection balloon",
            )
        } else {
            continue;
        };
        out.push(LintIssue {
            sheet: c.sheet,
            id: format!("l:{code}:{}", c.source),
            code: code.into(),
            severity: Severity::Warning,
            message: format!("Inspection characteristic {} {why}.", c.number),
            at: None,
            view: c.view.clone(),
            subjects: c.item.iter().cloned().collect(),
            requirements: Vec::new(),
            measurements: Vec::new(),
            sources: vec![c.source.clone()],
            stage: None,
            annotation: None,
            related: Vec::new(),
            evidence: None,
            fix: None,
        });
    }
    out
}
