//! Decoded graphical source evidence, independent of planning and authoring policy.
use super::Report;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Callout {
    pub id: u64,
    pub name: String,
    /// Source associations already established by the legacy manufacturing reader.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_sources: Vec<String>,
    pub links: Vec<Link>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub association: u64,
    pub definition: u64,
    pub entity: String,
    /// An exact definition refused by the reader can already have its own source finding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub part: usize,
    /// The reader's typed, part-local item ID, retained for inspection.
    pub item: String,
    /// Source IDs from semantic lowering, never inferred from annotation text or shared faces.
    pub sources: Vec<String>,
}

impl Callout {
    pub(crate) fn reference(&self) -> String {
        format!("graphical_callout:#{}", self.id)
    }

    pub(crate) fn description(&self) -> String {
        if self.name.is_empty() {
            format!("Graphical callout #{} (unnamed)", self.id)
        } else {
            format!("Graphical callout #{} ({:?})", self.id, self.name)
        }
    }

    pub(crate) fn sources(&self) -> BTreeSet<&str> {
        self.legacy_sources
            .iter()
            .chain(self.links.iter().flat_map(|l| {
                l.sources
                    .iter()
                    .chain(l.targets.iter().flat_map(|t| &t.sources))
            }))
            .map(String::as_str)
            .collect()
    }
}

/// The independent drawing lint attaches the same source evidence to its own semantic findings.
/// It reads annotation marks, never the planner's coverage decision.
pub(crate) fn attach_to_lint(
    report: &Report,
    on_sheet: &BTreeSet<&str>,
    issues: &mut [crate::lint::LintIssue],
) {
    for c in &report.reader.callouts {
        let missing: BTreeSet<_> = c.sources().difference(on_sheet).copied().collect();
        for i in issues.iter_mut() {
            if i.sources.iter().any(|s| missing.contains(s.as_str())) {
                // `sources` names semantic requirements: a graphical annotation is evidence
                // about one, not an additional requirement to count in coverage audits.
                let marker = c.reference();
                let evidence = i.evidence.get_or_insert_with(String::new);
                if !evidence
                    .lines()
                    .any(|line| line.starts_with(&format!("{marker};")))
                {
                    if !evidence.is_empty() {
                        evidence.push('\n');
                    }
                    evidence.push_str(&format!("{marker}; {}.", c.description()));
                    i.message.push_str(&format!(" {}.", c.description()));
                }
            }
        }
    }
}
