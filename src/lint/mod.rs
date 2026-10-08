//! Lint: an independent check of a finished drawing against the part.
//!
//! Lint judges what the drawing *says* (its annotations' text and the measurements they claim)
//! against what the part *is* (the [`Analysis`]). It never reads the Plan or the Spec the drawing
//! was made from (ADR1-9, ADR5-2; maintainer decision 1 in ADR R4): the requirements it counts
//! come from the Analysis, and the evidence from the rendered [`Drawing`]. A guard test
//! (`guard::tests`) fails if any file in this module names the Plan or the Spec. So the same
//! checks can judge Python draftwright's captured sheets and the port's own.
//!
//! - [`lint`] runs every ported rule on a drawing;
//! - [`marks`] reads a drawing's annotations as lint sees them, and [`check`] runs the rules on
//!   marks from any source (the comparison with Python's sheets builds them from the captured
//!   facts);
//! - [`LintIssue`] is a finding, with the catalogue's fields (LINT-issues-lintissue_schema).
//!
//! Ported rules, in the order Python's lint runs them: boss heights ([`bosses`]); angled steps
//! ([`angled_steps`]); flat, groove, chamfer, fillet and blend coverage ([`flats`], [`grooves`],
//! [`chamfers`], [`fillets`], [`blends`]); paired-ramp and circular blind step coverage
//! ([`paired_ramp_steps`], [`circular_blind_steps`]); through-step and slot coverage ([`steps`],
//! [`slots`]); and hole coverage ([`holes`]). The families whose requirements are a feature's
//! own sizes share [`parameters`]. The envelope's and turned steps' completeness ([`envelope`],
//! [`turned`]) are ledgers, not findings: Python's lint reports nothing for them, so `check` does
//! not run them.

use serde::Serialize;

use crate::model::Analysis;
use crate::output::{Claim, Drawing};

pub mod angled_steps;
pub mod blends;
pub mod blind_slots;
pub mod bosses;
pub mod chamfers;
pub mod channels;
pub mod circular_blind_steps;
pub mod envelope;
pub mod fillets;
pub mod flats;
pub mod grooves;
pub mod holes;
pub mod paired_ramp_steps;
pub mod parameters;
pub mod pockets;
pub mod polygonal;
pub mod section_recesses;
pub mod slots;
pub mod steps;
pub mod turned;

#[cfg(test)]
mod guard;

/// How serious a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Where in the drawing's making a finding arose: a valid annotation that did not fit
/// (`placement`), or malformed input (`validation`). Coverage findings have none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Placement,
    Validation,
}

/// A physical requirement of a feature: the feature's Analysis ID and the parameter, in the role
/// vocabulary (`bore.diameter`, `bore.through`, `grouping.count`, …).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Requirement {
    pub feature: String,
    pub parameter: String,
}

/// One finding (LINT-issues-lintissue_schema: Python's `LintIssue`, with the same stable fields).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LintIssue {
    /// Stable within one drawing: `l:<code>:<subject>`.
    pub id: String,
    /// The stable machine-readable check, e.g. `hole_requirement_missing`.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    /// Where on the page, mm, when the finding has a place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<[f64; 2]>,
    /// The view it is in, `v:<name>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// The features it is about, by Analysis ID.
    pub subjects: Vec<String>,
    /// The physical requirements it is about (Python's `hole_requirement_ids`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<Requirement>,
    /// The measurements it implicates, as `<feature>/<parameter>` (Python's `measurement_ids`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub measurements: Vec<String>,
    /// External source records it implicates (AP242 PMI IDs).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<Stage>,
    /// The annotation it names, and any others taking part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    /// Why the evidence was not accepted, where some was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// What would put it right, in words a caller can act on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// An annotation as lint sees it: the text it prints and the measurements it claims.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub name: String,
    pub view: Option<String>,
    /// Its label's text runs, joined by spaces.
    pub text: String,
    pub claims: Vec<Claim>,
}

/// The drawing's annotations as marks: each label's text read back from the drawing's own text
/// runs, never from the item it was drawn from.
pub fn marks(drawing: &Drawing) -> Vec<Mark> {
    drawing
        .annotations
        .iter()
        .map(|a| Mark {
            name: a.name.clone(),
            view: a.view.clone(),
            text: a
                .runs
                .iter()
                .filter_map(|&i| drawing.text.get(i))
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            claims: a.claims.clone(),
        })
        .collect()
}

/// Every ported rule's findings on *drawing*, a drawing of the part *analysis* describes.
pub fn lint(analysis: &Analysis, drawing: &Drawing) -> Vec<LintIssue> {
    check(analysis, &marks(drawing))
}

/// Every ported rule's findings on a drawing whose annotations are *marks*.
pub fn check(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let mut issues = bosses::coverage(analysis, marks);
    issues.extend(polygonal::boss_coverage(analysis, marks));
    issues.extend(section_recesses::coverage(analysis, marks));
    issues.extend(angled_steps::coverage(analysis, marks));
    issues.extend(flats::coverage(analysis, marks));
    issues.extend(grooves::coverage(analysis, marks));
    issues.extend(chamfers::coverage(analysis, marks));
    issues.extend(fillets::coverage(analysis, marks));
    issues.extend(blends::coverage(analysis, marks));
    issues.extend(paired_ramp_steps::coverage(analysis, marks));
    issues.extend(circular_blind_steps::coverage(analysis, marks));
    issues.extend(blind_slots::coverage(analysis, marks));
    issues.extend(steps::coverage(analysis, marks));
    issues.extend(slots::coverage(analysis, marks));
    issues.extend(holes::coverage(analysis, marks));
    issues.extend(channels::coverage(analysis, marks));
    issues.extend(polygonal::stock_coverage(analysis, marks));
    issues.extend(pockets::coverage(analysis, marks));
    issues
}
