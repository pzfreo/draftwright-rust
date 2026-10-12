//! Native regressions for text association and collision-driven lane promotion.
use draftwright::model::{self, Spec};
#[path = "common/corpus.rs"]
mod corpus;

fn plan(name: &str) -> Option<model::Plan> {
    let a = model::analyse_native(&corpus::dir()?.join(format!("mfcadpp/{name}.step")))
        .unwrap()
        .1;
    Some(model::plan_recovered(&a, &Spec::default()).unwrap())
}

fn adjacent(d: &draftwright::dims::PlannedDimension) -> bool {
    let p = d.placed.as_ref().unwrap();
    let k = usize::from(p.rotation != 0.0);
    let lo = p.arrows[0][0][k].min(p.arrows[1][0][k]);
    let hi = p.arrows[0][0][k].max(p.arrows[1][0][k]);
    let b = p.label_box;
    (b[k] >= lo - 1e-6 && b[k + 2] <= hi + 1e-6)
        || (b[k + 2] <= lo && lo - b[k + 2] < 5.7 - 1e-6)
        || (b[k] >= hi && b[k] - hi < 5.7 - 1e-6)
}

#[test]
fn short_native_dimensions_keep_their_text_on_their_span() {
    for (name, labels) in [
        ("1000", vec!["6.9", "8.9", "10.9"]),
        ("10020", vec!["4.5", "1.2"]),
    ] {
        let Some(p) = plan(name) else {
            eprintln!("SKIPPED: the corpus is not found");
            return;
        };
        for label in labels {
            let ds: Vec<_> = p
                .dimensions
                .iter()
                .filter(|d| d.kind == "linear" && d.label == label && d.placed.is_some())
                .collect();
            assert!(!ds.is_empty(), "{name}: missing {label}");
            assert!(
                ds.iter().all(|d| adjacent(d)),
                "{name}: {label} away from its own span"
            );
        }
    }
}
