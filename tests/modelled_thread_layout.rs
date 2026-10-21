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
        let plan = model::plan_recovered(
            &analysis,
            &serde_json::from_value(serde_json::json!({"sheet":{"max_size":"A0"}})).unwrap(),
        )
        .unwrap();
        let labels: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
            .map(|d| d.label.as_str())
            .collect();
        assert_eq!(labels, expected, "{file}");
        let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
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
