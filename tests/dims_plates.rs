//! Physical plate thicknesses are editable items and independently checked on the ink.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::{
    items::{Op, State, key},
    lint,
    model::{self, Spec},
    text::Font,
};
use serde_json::json;

fn thicknesses(name: &str) {
    let Some(dir) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&dir.join(format!("mfcadpp/{name}.step"))).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let plates: Vec<_> = a.features.iter().filter(|f| f.kind == "plate").collect();
    assert!(!plates.is_empty(), "{name}: recognised plates");
    let drawn: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter == "thickness.length")
        })
        .collect();
    assert!(
        !drawn.is_empty(),
        "{name}: no plate thickness items; plates: {plates:#?}"
    );
    for d in &drawn {
        assert_eq!(d.state, State::Shown, "{name}: {}", d.id);
        assert!(d.placed.is_some(), "{name}: {}", d.id);
        let ink = d.placed.as_ref().unwrap();
        let along = usize::from(ink.rotation % 180.0 != 0.0);
        let scale = plan.views.iter().find(|v| v.id == d.view).unwrap().scale;
        assert!(
            ((ink.arrows[0][0][along] - ink.arrows[1][0][along]).abs() - d.value * scale).abs()
                < 1e-6,
            "{} has its actual thickness span",
            d.id
        );
        for m in d
            .measurements
            .iter()
            .filter(|m| m.parameter == "thickness.length")
        {
            assert_eq!(
                drawn
                    .iter()
                    .filter(|other| other.measurements.contains(m))
                    .count(),
                1
            );
        }
    }
    let drawing = model::render(
        &part,
        &a,
        &plan,
        &Font::new(draftwright::api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    let findings = lint::lint(&a, &drawing);
    assert!(
        !findings
            .iter()
            .any(|i| i.code.starts_with("plate_requirement_")),
        "{name}: {findings:#?}"
    );
    if name == "10038" {
        assert!(
            drawn.iter().any(|d| (d.value - 10.09).abs() < 0.01),
            "the 10.09 wall is stated"
        );
    }
    let d = drawn[0];
    let hidden = model::edit(
        &Spec::default(),
        &json!([Op::set(
            format!("dimensions/{}/omit", key(&d.id)),
            json!(true)
        )]),
    )
    .unwrap();
    let hidden_plan = model::plan_recovered(&a, &hidden).unwrap();
    assert!(
        hidden_plan
            .omitted
            .iter()
            .any(|o| o.id == d.id && o.state == State::OmittedByUser)
    );
    let drawing = model::render(
        &part,
        &a,
        &hidden_plan,
        &Font::new(draftwright::api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    assert!(
        lint::lint(&a, &drawing)
            .iter()
            .any(|i| i.code == "plate_requirement_suppressed")
    );
    let pinned = model::edit(
        &Spec::default(),
        &json!([Op::set(
            format!("dimensions/{}/place", key(&d.id)),
            json!({"pinned":true})
        )]),
    )
    .unwrap();
    let pinned_plan = model::plan_recovered(&a, &pinned).unwrap();
    assert_eq!(
        pinned_plan
            .dimensions
            .iter()
            .find(|p| p.id == d.id)
            .unwrap()
            .placed,
        d.placed
    );
    let view = if d.measurements.iter().any(|m| {
        plates
            .iter()
            .any(|f| f.id == m.feature && f.parameters["axis"] != "z")
    }) {
        "plan"
    } else {
        "side"
    };
    let moved = model::edit(
        &Spec::default(),
        &json!([Op::set(
            format!("dimensions/{}/place", key(&d.id)),
            json!({"view":view,"pinned":true})
        )]),
    )
    .unwrap();
    let moved_plan = model::plan_recovered(&a, &moved).unwrap();
    let moved_item = moved_plan.dimensions.iter().find(|p| p.id == d.id).unwrap();
    assert_eq!(moved_item.view, format!("v:{view}"));
    assert_eq!(moved_item.value, d.value);
    assert!(moved_item.placed.is_some());
    let ink = moved_item.placed.as_ref().unwrap();
    let along = usize::from(ink.rotation % 180.0 != 0.0);
    let scale = moved_plan
        .views
        .iter()
        .find(|v| v.id == moved_item.view)
        .unwrap()
        .scale;
    assert!(
        ((ink.arrows[0][0][along] - ink.arrows[1][0][along]).abs() - d.value * scale).abs() < 1e-6
    );

    assert_eq!(
        serde_json::to_vec(&plan).unwrap(),
        serde_json::to_vec(&model::plan_recovered(&a, &Spec::default()).unwrap()).unwrap()
    );
}

#[test]
fn m10007_states_its_plate_thicknesses_once() {
    thicknesses("10007");
}

#[test]
fn m10038_states_its_ten_point_zero_nine_wall_once() {
    thicknesses("10038");
}

#[test]
fn m10049_states_its_plate_thicknesses_once() {
    thicknesses("10049");
}
