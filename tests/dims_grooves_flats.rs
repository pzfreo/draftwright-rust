//! Groove and flat callouts (`dims::grooves`, `dims::flats`), as scenarios: a corpus part and its
//! recognition (sometimes with records edited) in, the plan's callouts and problems, or the
//! drawing's lint, out.
//!
//! The sampled sheets with grooves or flats are connector109 (three grooves and a flat) and
//! string_post (a double-D); no other sheet or recognition fixture has either.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use draftwright::lint;
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::Value;

/// Sampled part *name* analysed with its recognition, each record (by its index) rewritten by
/// *edit*; none when the corpus is missing.
fn analysed(
    name: &str,
    edit: impl Fn(usize, &mut Value),
) -> Option<(draftwright::haecceity::Part, Analysis)> {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return None;
    };
    let f = facts::load(name);
    let (part, mut a) = model::analyse(&dir.join(&f.binding.file)).unwrap();
    let mut json: Value = serde_json::from_str(&corpus::recognition(name).unwrap()).unwrap();
    for (i, r) in json["quiddity"]["features"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        edit(i, r);
    }
    model::recognise(&part, &mut a, &json.to_string()).unwrap();
    Some((part, a))
}

fn planned(name: &str, edit: impl Fn(usize, &mut Value)) -> Option<Plan> {
    let (_, a) = analysed(name, edit)?;
    Some(model::plan(&a, &Spec::default()).unwrap())
}

/// The plan's groove and flat items: label, view, and each measurement's parameter and value.
type Item = (String, String, Vec<(String, f64)>);

fn items(plan: &Plan) -> Vec<Item> {
    let mut out: Vec<Item> = plan
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter.starts_with("groove.") || m.parameter == "flat.length")
        })
        .map(|d| {
            (
                d.label.clone(),
                d.view.trim_start_matches("v:").to_string(),
                d.measurements
                    .iter()
                    .map(|m| (m.parameter.clone(), m.value))
                    .collect(),
            )
        })
        .collect();
    out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    out
}

fn codes(plan: &Plan, code: &str) -> Vec<String> {
    plan.problems
        .iter()
        .filter(|p| p.code == code)
        .flat_map(|p| p.subjects.clone())
        .collect()
}

fn set(r: &mut Value, field: &str, v: Value) {
    r["record"][field] = v;
}

/// Every groove and flat callout on draftwright's sheets (`10 WIDE × ø122` and `6 WIDE × ø122`
/// twice in connector109's front view, `52.6 A/F` in its plan; string_post's double-D as one
/// `2.4 A/F` in the plan stating both faces) is planned with the same label, view, parameters
/// and values, each a default with a stable ID, and nothing is reported unplanned.
#[test]
fn grooves_and_flats_match_draftwrights_sheets() {
    for name in ["connector109", "string_post"] {
        let Some(plan) = planned(name, |_, _| {}) else {
            return;
        };
        let f = facts::load(name);
        let mut python: Vec<Item> = f
            .annotations
            .iter()
            .filter(|a| {
                a.measurements
                    .iter()
                    .any(|m| m.parameter.starts_with("groove.") || m.parameter == "flat.length")
            })
            .map(|a| {
                (
                    a.label.clone().unwrap(),
                    a.view.clone().unwrap(),
                    a.measurements
                        .iter()
                        .map(|m| (m.parameter.clone(), m.value.unwrap()))
                        .collect(),
                )
            })
            .collect();
        python.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        assert_eq!(items(&plan), python, "{name}");
        let ours: Vec<&str> = plan
            .dimensions
            .iter()
            .filter(|d| d.id.contains("groove") || d.id.contains("flat"))
            .map(|d| {
                assert!(d.default && d.kind == "callout", "{}", d.id);
                d.id.as_str()
            })
            .collect();
        let want: &[&str] = if name == "connector109" {
            &[
                "d:f:groove:1/groove.length",
                "d:f:groove:2/groove.length",
                "d:f:groove:3/groove.length",
                "d:f:flat:1/flat.length",
            ]
        } else {
            &["d:f:flat:1/flat.length"]
        };
        assert_eq!(ours, want, "{name}");
        for code in ["dimensions_not_ported", "feature_not_dimensioned"] {
            assert!(
                !codes(&plan, code)
                    .iter()
                    .any(|s| s.contains("groove") || s.contains("flat")),
                "{name}: {code}"
            );
        }
    }
}

/// Lint reads the callout's claims off the drawing: on string_post's own drawing the double-D's
/// requirement is stated (LINT-flat_coverage-across_flats). connector109's grooves are checked
/// the same way by `tests/lint.rs` with `LINT_OWN_ALL=1`.
#[test]
fn lint_finds_the_flats_stated_on_the_ports_drawing() {
    let Some((part, a)) = analysed("string_post", |_, _| {}) else {
        return;
    };
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"),
        )
        .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let flats: Vec<String> = lint::lint(&a, &drawing)
        .into_iter()
        .filter(|i| i.code.starts_with("flat_"))
        .map(|i| i.code)
        .collect();
    assert!(flats.is_empty(), "{flats:?}");
}

/// string_post's flats are records 2 and 3. Two faces on different stock (another span) are two
/// definitions of one size, called out once and counted; a reversed face whose span runs along
/// its own direction is the same stock once canonicalised, so the double-D stays one definition.
#[test]
fn equal_flats_on_other_stock_are_counted_and_a_double_d_is_one() {
    let Some(plan) = planned("string_post", |i, r| {
        if i == 3 {
            set(r, "stock_span", serde_json::json!([20.0, 30.0]));
        }
    }) else {
        return;
    };
    let got = items(&plan);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "2× 2.4 A/F");
    assert_eq!(got[0].2.len(), 2);

    let plan = planned("string_post", |i, r| {
        if i == 3 {
            set(r, "axis_direction", serde_json::json!([-1.0, 0.0, 0.0]));
            set(r, "stock_span", serde_json::json!([-10.004, -2.804]));
        }
    })
    .unwrap();
    let got = items(&plan);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "2.4 A/F");
    assert_eq!(got[0].2.len(), 2);
}

/// An x/z slant that ties once rounded to six decimals (as draftwright's canonical direction is)
/// is drawn in the x end-on view, draftwright's `presentation_axis`; a slant that does not tie
/// keeps its named axis's view.
#[test]
fn an_x_z_slant_that_ties_to_six_decimals_is_drawn_end_on_x() {
    let Some(base) = planned("string_post", |_, _| {}) else {
        return;
    };
    let base = items(&base);
    let slant = |d: [f64; 3]| {
        let plan = planned("string_post", move |i, r| {
            if i == 2 || i == 3 {
                set(r, "axis", "z".into());
                set(r, "axis_direction", serde_json::json!(d));
            }
        })
        .unwrap();
        items(&plan)
    };
    let tie = slant([1.0, 0.0, 1.000_000_1]);
    assert_eq!(tie.len(), 1, "{tie:?}");
    assert_eq!(tie[0].1, base[0].1);
    let no_tie = slant([1.0, 0.0, 1.01]);
    assert_eq!(no_tie.len(), 1, "{no_tie:?}");
    assert_ne!(no_tie[0].1, base[0].1);
}

/// A flat whose record gives no stock identity (`axis_line`, `stock_span`; quiddity always
/// emits both) is a definition of its own, as `lint::flats` keys it, rather than draftwright's
/// `(0, 0)` default, which would merge flats on unrelated stock.
#[test]
fn a_flat_without_stock_identity_is_its_own_definition() {
    let Some(plan) = planned("string_post", |i, r| {
        if i == 2 || i == 3 {
            r["record"].as_object_mut().unwrap().remove("stock_span");
        }
    }) else {
        return;
    };
    let got = items(&plan);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "2× 2.4 A/F");
}

/// Every nominal uses its own shortest faithful decimal, intentionally replacing Python's
/// shared `flat_auto_decimals` policy. Hundredths remain distinct without padding integers.
#[test]
fn flats_of_different_sizes_print_decimals_that_tell_them_apart() {
    let Some(plan) = planned("string_post", |i, r| {
        if i == 3 {
            set(r, "across", 2.45.into());
        }
    }) else {
        return;
    };
    let labels: Vec<String> = items(&plan).into_iter().map(|i| i.0).collect();
    assert_eq!(labels, ["2.4 A/F", "2.45 A/F"]);
    let plan = planned("string_post", |i, r| {
        if i == 3 {
            set(r, "across", 3.0.into());
        }
    })
    .unwrap();
    let labels: Vec<String> = items(&plan).into_iter().map(|i| i.0).collect();
    assert_eq!(labels, ["2.4 A/F", "3 A/F"]);
}

/// A groove or flat whose record cannot be stated is reported (`feature_not_dimensioned`), never
/// dropped, and the others are still called out. (The bridge numbers string_post's records 3
/// and 2 as `f:flat:1` and `f:flat:2`.)
#[test]
fn a_record_that_cannot_be_stated_is_reported() {
    let Some(plan) = planned("string_post", |i, r| {
        if i == 2 {
            r["record"].as_object_mut().unwrap().remove("across");
        }
        if i == 3 {
            set(r, "axis_direction", serde_json::json!([0.0, 1.0, 0.0]));
        }
    }) else {
        return;
    };
    assert!(items(&plan).is_empty());
    assert_eq!(
        codes(&plan, "feature_not_dimensioned"),
        ["f:flat:1", "f:flat:2"]
    );
    let messages: Vec<&str> = plan
        .problems
        .iter()
        .filter(|p| p.code == "feature_not_dimensioned")
        .map(|p| p.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "The 2.4 flat is not dimensioned: its axis_direction does not lie along its axis x.",
            "The flat is not dimensioned: its record gives no positive across.",
        ]
    );

    // connector109's grooves are records 6, 7 and 8; the bridge numbers record 7 `f:groove:3`.
    let plan = planned("connector109", |i, r| {
        if i == 7 {
            r["record"].as_object_mut().unwrap().remove("width");
        }
    })
    .unwrap();
    let labels: Vec<String> = items(&plan)
        .into_iter()
        .filter(|i| i.0.contains("WIDE"))
        .map(|i| i.0)
        .collect();
    assert_eq!(labels, ["10 WIDE × ø122", "6 WIDE × ø122"]);
    // Without the groove's width no band's ownership can be decided, so every turned step is
    // reported too (src/dims/turned.rs), none guessed.
    let mut reported: Vec<String> = (1..=7).map(|i| format!("f:turned_step:{i}")).collect();
    reported.push("f:groove:3".into());
    assert_eq!(codes(&plan, "feature_not_dimensioned"), reported);
}
