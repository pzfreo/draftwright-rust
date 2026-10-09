//! ISSUE-1247-a: a recognised angled step is reported as unsupported by the plan, one warning
//! per step (`angled_step_requirement_unsupported`, on `p:<code>:<feature id>`), and nothing is
//! dimensioned for it (maintainer decision, 2026-10-09). Lint's warning of the same code stays.
//!
//! The parts: draftwright's fixture for #1247 (`tests/fixtures/parts/issue_1247_angled_blind_step
//! .step`, copied from draftwright's `tests/fixtures/` at origin/main 4dfc63f) with Rust quiddity's
//! recognition, and corpus part m10245 with Python quiddity 0.3.9's
//! (`tests/fixtures/recognition/dims/m10245.json.gz`).

#[path = "common/corpus.rs"]
mod corpus;

use std::io::Read as _;
use std::path::Path;

use draftwright::lint;
use draftwright::model::{self, Analysis, Plan, Spec};

const CODE: &str = "angled_step_requirement_unsupported";

/// The plan for *a* states nothing of its angled steps and reports each once as unsupported, as
/// lint does; *steps* is how many *a* has.
fn check(name: &str, a: &Analysis, steps: usize) {
    let ids: Vec<&str> = a
        .features
        .iter()
        .filter(|f| f.kind == "angled_step")
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(ids.len(), steps, "{name}");
    let plan: Plan = model::plan(a, &Spec::default()).unwrap();
    let stated: Vec<&str> = plan
        .dimensions
        .iter()
        .flat_map(|d| d.measurements.iter().map(|m| m.feature.as_str()))
        .filter(|f| ids.contains(f))
        .collect();
    assert!(stated.is_empty(), "{name}: {stated:?}");
    for id in &ids {
        let about: Vec<_> = plan
            .problems
            .iter()
            .filter(|p| p.subjects.iter().any(|s| s == id))
            .collect();
        assert_eq!(about.len(), 1, "{name} {id}: {about:?}");
        let p = about[0];
        assert_eq!(p.id, format!("p:{CODE}:{id}"));
        assert_eq!(p.code, CODE);
        assert_eq!(p.severity, "warning");
        assert_eq!(p.subjects, vec![id.to_string()]);
        assert!(
            p.message.contains("outside automatic drawing approval"),
            "{}",
            p.message
        );
    }
    assert_eq!(
        plan.problems.iter().filter(|p| p.code == CODE).count(),
        steps
    );
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "dimensions_not_ported"
                && p.subjects.iter().any(|s| ids.contains(&s.as_str())))
    );
    let linted = lint::check(a, &[])
        .into_iter()
        .filter(|i| i.code == CODE)
        .count();
    assert_eq!(linted, steps, "{name}");
}

#[test]
fn draftwrights_angled_step_fixture_is_reported_unsupported() {
    let step = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts/issue_1247_angled_blind_step.step");
    let (_, a) = model::analyse_native(&step).unwrap();
    check("issue_1247_angled_blind_step", &a, 1);
}

#[test]
fn a_corpus_angled_step_is_reported_unsupported() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED (corpus part): the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/recognition/dims/m10245.json.gz");
    let mut json = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_string(&mut json)
        .unwrap();
    let (part, mut a) = model::analyse(&dir.join("mfcadpp/10245.step")).unwrap();
    model::recognise(&part, &mut a, &json).unwrap();
    check("m10245", &a, 1);
}
