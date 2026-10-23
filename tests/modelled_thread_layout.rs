//! Default native thread drawings retain their designation and clear printed text.
#[path = "common/corpus.rs"]
mod corpus;

use std::io::Read;

use draftwright::{api, model, text::Font};

#[test]
fn modelled_thread_callouts_clear_labels_and_dimension_ink() {
    let Some(corpus) = corpus::dir() else {
        return;
    };
    let font = Font::new(api::MONO.to_vec()).unwrap();
    for (file, expected) in [
        (
            "cadgenbench_inputs/cgb243.step.gz",
            vec!["M14×1.5", "M62×1.5", "M62×1.5", "M75×1.5"],
        ),
        (
            "cadgenbench/threaded_connector_109.step",
            vec!["⌀110 × P6.35"],
        ),
    ] {
        let path = corpus.join(file);
        let mut bytes = std::fs::read(&path).unwrap();
        if file.ends_with(".gz") {
            let mut plain = Vec::new();
            flate2::read::GzDecoder::new(bytes.as_slice())
                .read_to_end(&mut plain)
                .unwrap();
            bytes = plain;
        }
        let (part, analysis) = model::analyse_native_step(&path, &bytes).unwrap();
        let plan = model::plan_recovered(&analysis, &model::Spec::default()).unwrap();
        let labels: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
            .map(|d| d.label.as_str())
            .collect();
        assert_eq!(labels, expected, "{file}");
        let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
        // cgb243's M14×1.5 and two M62×1.5 are shorter than the holes they are cut in, but a
        // modelled thread's span does not prove a depth: lint asks it for none (the modelled
        // half of LINT-hole_coverage-thread_and_profile is retired).
        let depths: Vec<_> = draftwright::lint::lint(&analysis, &drawing)
            .into_iter()
            .filter(|i| i.requirements.iter().any(|r| r.parameter == "thread.depth"))
            .map(|i| i.subjects.join(","))
            .collect();
        assert!(depths.is_empty(), "{file}: {depths:?}");
        for code in ["annotation_overlap", "annotation_ink_overlap"] {
            let problems: Vec<_> = plan
                .problems
                .iter()
                .chain(&drawing.problems)
                .filter(|p| p.code == code)
                .collect();
            assert!(problems.is_empty(), "{file}: {code}: {problems:?}");
        }
        // The fallback remains measured rather than being guessed as metric.
        assert_eq!(
            plan.problems
                .iter()
                .filter(|p| p.code == "thread_designation_measured")
                .count(),
            usize::from(file.contains("threaded_connector"))
        );
        assert!(
            plan.dimensions
                .iter()
                .filter(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
                .all(|d| {
                    !d.label.contains("LONG")
                        && !d.label.contains('\n')
                        && d.measurements
                            .iter()
                            .any(|m| m.parameter == "thread.length")
                })
        );
    }
}

/// A thread on cgb243's lone ø90 boss prints its designation in place of the diameter, and
/// lint counts that callout as stating the boss's diameter (it claims `boss.diameter`), so
/// there is neither a `pmi_not_lowered` refusal nor a false `feature_not_dimensioned`.
#[test]
fn a_thread_on_a_lone_boss_states_its_diameter() {
    let Some(corpus) = corpus::dir() else {
        return;
    };
    let path = corpus.join("cadgenbench_inputs/cgb243.step.gz");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::read(&path).unwrap().as_slice())
        .read_to_end(&mut bytes)
        .unwrap();
    let (_, analysis) = model::analyse_native_step(&path, &bytes).unwrap();
    let spec: model::Spec = serde_json::from_value(serde_json::json!({
        "features": {"f:boss:2": {"thread": {"designation": "M90x2"}}}
    }))
    .unwrap();
    let plan = model::plan_recovered(&analysis, &spec).unwrap();
    let boss = plan
        .dimensions
        .iter()
        .find(|d| d.id == "d:f:boss:2/boss.diameter")
        .expect("the boss's callout is planned");
    assert_eq!(boss.label, "M90 x 2");
    assert!(
        plan.problems.iter().all(|p| p.code != "pmi_not_lowered"),
        "{:?}",
        plan.problems
    );
    let marks: Vec<_> = plan
        .dimensions
        .iter()
        .map(|d| draftwright::lint::Mark {
            name: d.id.clone(),
            view: Some(d.view.clone()),
            text: d.label.replace('\n', " "),
            claims: d
                .measurements
                .iter()
                .map(|m| draftwright::output::Claim {
                    feature: m.feature.clone(),
                    parameter: m.parameter.clone(),
                })
                .collect(),
        })
        .collect();
    let unstated: Vec<_> = draftwright::lint::coverage::feature_coverage(&analysis, &marks)
        .into_iter()
        .filter(|i| i.subjects.iter().any(|s| s == "f:boss:2"))
        .collect();
    assert!(unstated.is_empty(), "{unstated:?}");
}
