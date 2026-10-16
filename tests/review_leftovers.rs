//! Regression cases from the review of the single-sheet drawings.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::model::{self, Analysis, Plan, Spec};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn saved_spec(analysis: &Analysis, prefix: &str) -> Spec {
    let saved: BTreeMap<String, Vec<Spec>> =
        serde_json::from_str(include_str!("fixtures/known_single_sheet_specs.json")).unwrap();
    saved[&analysis.binding.source_sha256]
        .iter()
        .find(|spec| {
            let hash: String = Sha256::digest(serde_json::to_vec(spec).unwrap())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            hash.starts_with(prefix)
        })
        .unwrap_or_else(|| panic!("missing spec {prefix}"))
        .clone()
}

fn plan(file: &str, prefix: &str) -> Option<Plan> {
    let dir = corpus::dir()?;
    let (_, analysis) = model::analyse_native(&dir.join(file)).unwrap();
    Some(model::plan_recovered(&analysis, &saved_spec(&analysis, prefix)).unwrap())
}

#[test]
fn adjacent_labels_have_readable_separation() {
    let Some(plan) = plan("mfcadpp/10127.step", "2f119c6b") else {
        return;
    };
    let labels: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| d.view == "v:side" && ["23.9", "32.3"].contains(&d.label.as_str()))
        .collect();
    assert_eq!(labels.len(), 2);
    let a = labels[0].placed.as_ref().unwrap().label_box;
    let b = labels[1].placed.as_ref().unwrap().label_box;
    let gap = (a[0] - b[2])
        .max(b[0] - a[2])
        .max(a[1] - b[3])
        .max(b[1] - a[3]);
    assert!(
        gap >= 1.0 - 1e-6,
        "labels run together: {a:?}, {b:?}, gap {gap}"
    );
}
