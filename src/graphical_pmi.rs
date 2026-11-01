//! Graphical annotations are evidence of source requirements, not extra dimensions to place.
//! Triage once on the finished plan; it must not influence layout recovery or paper selection.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::items::State;
use crate::model::{Analysis, Plan, Problem};
use crate::pmi::{Reading, Report};

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
    fn reference(&self) -> String {
        format!("graphical_callout:#{}", self.id)
    }

    fn description(&self) -> String {
        if self.name.is_empty() {
            format!("Graphical callout #{} (unnamed)", self.id)
        } else {
            format!("Graphical callout #{} ({:?})", self.id, self.name)
        }
    }

    fn sources(&self) -> BTreeSet<&str> {
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

/// Annotate a semantic refusal without changing its severity, impact or finding count.
fn attach(c: &Callout, refs: &mut Vec<String>, message: &mut String) {
    let id = c.reference();
    if !refs.contains(&id) {
        refs.push(id);
        if !c.name.is_empty() {
            refs.push(format!("graphical_text:{:?}", c.name));
        }
        message.push_str(&format!(" {}.", c.description()));
    }
}

/// Actual carriers, including engine callouts, covered items and manufacturing table claims.
/// A source linked to a dropped frame is not credited merely for having lowered successfully.
fn drawn_sources<'a>(report: &'a Report, plan: &Plan) -> BTreeSet<&'a str> {
    let items = crate::items::items(plan);
    let states: BTreeMap<_, _> = items.iter().map(|i| (i.id.as_str(), &i.state)).collect();
    let shown = |id: &str| {
        let mut state = states.get(id);
        for _ in 0..=states.len() {
            match state {
                Some(State::Shown) => return true,
                Some(State::Covered { by }) => state = states.get(by.as_str()),
                _ => return false,
            }
        }
        false // A corrupt cycle never proves ink.
    };
    let ids = report.identities();
    let mut carriers: BTreeSet<&str> = items
        .iter()
        .filter(|i| shown(&i.id))
        .map(|i| i.id.as_str())
        .collect();
    for d in plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some() && shown(&d.id))
    {
        carriers.extend(d.measurements.iter().map(|m| m.feature.as_str()));
    }
    for n in plan
        .notes
        .iter()
        .filter(|n| n.placed.is_some() && shown(&n.id))
    {
        carriers.extend(n.measurements.iter().map(|m| m.feature.as_str()));
    }
    for t in plan
        .tables
        .iter()
        .chain(
            plan.hole_table
                .iter()
                .chain(&plan.hole_tables)
                .flat_map(|t| &t.blocks),
        )
        .filter(|t| t.placed.is_some() && t.state == State::Shown)
    {
        carriers.extend(
            t.row_claims
                .iter()
                .flat_map(|r| &r.claims)
                .map(|c| c.feature.as_str()),
        );
    }
    for t in plan
        .hole_table
        .iter()
        .chain(&plan.hole_tables)
        .filter(|t| t.state == State::Shown && !t.blocks.is_empty())
    {
        carriers.extend(t.covered.iter().map(|m| m.feature.as_str()));
    }
    let mut out: BTreeSet<_> = report
        .records
        .iter()
        .filter(|r| {
            crate::pmi_lowering::no_part_requirement(r)
                || ids.id(&r.source_id).is_ok_and(|id| {
                    carriers.contains(id)
                        || shown(id)
                        || (crate::pmi::specify_general(report, r)
                            && (carriers.contains(
                                crate::dims::pmi_dimensions::general_feature(id).as_str(),
                            ) || plan.dimensions.iter().any(|d| {
                                d.placed.is_some()
                                    && shown(&d.id)
                                    && crate::dims::pmi_dimensions::conveys(&d.basis, &r.source_id)
                            })))
                })
        })
        .map(|r| r.source_id.as_str())
        .collect();
    out.extend(
        plan.gdt
            .iter()
            .filter(|g| g.placed.is_some() && shown(&g.id))
            .filter_map(|g| {
                report
                    .sources
                    .iter()
                    .find(|s| s.id == g.source)
                    .map(|s| s.id.as_str())
            }),
    );
    out.extend(
        plan.requirements
            .iter()
            .filter(|r| shown(&r.id))
            .filter_map(|r| {
                report
                    .sources
                    .iter()
                    .find(|s| s.id == r.source)
                    .map(|s| s.id.as_str())
            }),
    );
    for m in &report.materials {
        if report.owns(m)
            && !m.designation.trim().is_empty()
            && plan.sheet.title.get("material") == Some(&m.designation)
        {
            out.insert(m.source_id.as_str());
        }
    }
    out
}

pub(crate) fn apply(analysis: &Analysis, plan: &mut Plan) {
    let Some(Reading::Read(report)) = &analysis.pmi else {
        return;
    };
    if report.reader.callouts.is_empty() {
        return;
    }
    let drawn = drawn_sources(report, plan);
    let ids = report.identities();
    for c in &report.reader.callouts {
        let sources = c.sources();
        if sources.is_empty() {
            let id = format!("p:pmi_graphical_not_drawn:{}", c.id);
            if !plan.problems.iter().any(|p| p.id == id) {
                plan.problems.push(Problem {
                    id, code: "pmi_graphical_not_drawn".into(), severity: "warning".into(),
                    message: format!("{} is not drawn: no resolved semantic PMI link covers this annotation. Review its requirement manually.", c.description()),
                    refs: vec![c.reference()], ..Default::default()
                });
            }
            continue;
        }
        let missing: BTreeSet<_> = sources.difference(&drawn).copied().collect();
        if missing.is_empty() {
            continue;
        }
        let item_ids: BTreeSet<_> = missing.iter().filter_map(|s| ids.id(s).ok()).collect();
        for p in &mut plan.problems {
            if p.refs.iter().any(|r| missing.contains(r.as_str()))
                || p.subjects.iter().any(|s| item_ids.contains(s.as_str()))
                || p.code == "pmi_ignored"
            {
                attach(c, &mut p.refs, &mut p.message);
            }
        }
    }
    plan.status = crate::completeness::status(&plan.problems);
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
                attach(c, &mut i.sources, &mut i.message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_thread_callout_uses_the_readers_existing_requirement_association() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pmi/step/string_post_tapped.step.gz");
        let (_, analysis) = crate::model::analyse_native(&path).unwrap();
        let plan = crate::model::plan_recovered(&analysis, &crate::model::Spec::default()).unwrap();
        let Some(Reading::Read(report)) = &analysis.pmi else {
            panic!("PMI read")
        };
        let callout = report
            .reader
            .callouts
            .iter()
            .find(|c| c.id == 3165)
            .unwrap();
        assert_eq!(callout.legacy_sources, ["manufacturing_requirement:#3157"]);
        assert!(callout.sources().is_subset(&drawn_sources(report, &plan)));
        assert!(
            plan.problems
                .iter()
                .all(|p| !p.refs.contains(&callout.reference()))
        );
    }

    #[test]
    fn all_thirteen_spindle_callouts_are_covered_by_drawn_semantic_carriers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pmi/spindle-coplanar-groups.step.gz");
        let (_, analysis) = crate::model::analyse_native(&path).unwrap();
        let plan = crate::model::plan_recovered(&analysis, &crate::model::Spec::default()).unwrap();
        let Some(Reading::Read(report)) = &analysis.pmi else {
            panic!("PMI read")
        };
        assert_eq!(report.reader.callouts.len(), 13);
        let drawn = drawn_sources(report, &plan);
        for c in &report.reader.callouts {
            let sources = c.sources();
            assert!(!sources.is_empty(), "{}", c.description());
            assert!(
                sources.is_subset(&drawn),
                "{}: {:?}",
                c.description(),
                sources.difference(&drawn).collect::<Vec<_>>()
            );
        }
        assert!(plan.problems.iter().all(|p| !matches!(
            p.code.as_str(),
            "pmi_graphical_not_drawn" | "pmi_presentation_not_read"
        )));
        let spec = crate::model::Spec {
            pmi: crate::model::PmiMode::Ignore,
            ..Default::default()
        };
        let ignored = crate::model::plan(&analysis, &spec).unwrap();
        let finding = ignored
            .problems
            .iter()
            .find(|p| p.code == "pmi_ignored")
            .unwrap();
        for c in &report.reader.callouts {
            assert!(finding.refs.contains(&c.reference()));
        }
    }
}
