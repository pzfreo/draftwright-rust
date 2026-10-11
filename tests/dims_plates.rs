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
    if name == "1000" {
        assert_eq!(
            drawn[0].view, "v:plan",
            "the wall beside the U-slot has solid edges in plan"
        );
    }
    if name == "10038" {
        assert_eq!(
            drawn
                .iter()
                .find(|d| (d.value - 4.448).abs() < 0.001)
                .unwrap()
                .view,
            "v:side",
            "the flange is exposed in the L profile"
        );
        let underside = "d:step_level/step_height.length.h7_168";
        assert!(
            !plan.dimensions.iter().any(|d| d.id == underside),
            "no 7.2 + 4.4 = 11.6 chain"
        );
        assert!(
            plan.omitted
                .iter()
                .any(|o| o.id == underside && matches!(o.state, State::Covered { .. }))
        );
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
    if name == "10038" && (d.value - 4.448).abs() < 0.001 {
        assert!(
            hidden_plan
                .dimensions
                .iter()
                .any(|d| d.id == "d:step_level/step_height.length.h7_168")
        );
    }
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
    let Some(dir) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&dir.join("mfcadpp/10007.step")).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let drawing = model::render(
        &part,
        &a,
        &plan,
        &Font::new(draftwright::api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    assert!(
        !lint::lint(&a, &drawing)
            .iter()
            .any(|i| i.code.starts_with("plate_requirement_"))
    );
    for f in a.features.iter().filter(|f| f.kind == "plate") {
        let id = format!("d:{}/thickness.length", f.id);
        let covered = plan.omitted.iter().find(|o| o.id == id).unwrap();
        assert!(matches!(covered.state, State::Covered { .. }));
        assert!(!plan.dimensions.iter().any(|d| d.id == id));
        let owners = covered.basis.split("; dependencies: ").nth(1).unwrap();
        for owner in owners.split(", ") {
            assert!(
                plan.dimensions
                    .iter()
                    .any(|d| d.id == owner && d.placed.is_some())
            );
            let spec = model::edit(
                &Spec::default(),
                &json!([Op::set(
                    format!("dimensions/{}/omit", key(owner)),
                    json!(true)
                )]),
            )
            .unwrap();
            let restored = model::plan_recovered(&a, &spec).unwrap();
            assert!(
                restored
                    .dimensions
                    .iter()
                    .any(|d| d.id == id && d.placed.is_some()),
                "omitting {owner} restores {id}"
            );
        }
    }
}

#[test]
fn m10038_states_its_ten_point_zero_nine_wall_once() {
    thicknesses("10038");
}

#[test]
fn m10049_states_its_plate_thicknesses_once() {
    thicknesses("10049");
}

#[test]
fn m1000_wall_uses_the_visible_plan_profile() {
    thicknesses("1000");
}

#[test]
fn captured_m1000_seat_callout_stays_inside_its_front_band() {
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = corpus::analyse(&dir, "mfcadpp/1000.step", "m1000");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let overruns: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "annotation_outside_area")
        .collect();
    assert_eq!(overruns.len(), 3, "{overruns:#?}");
    assert!(
        overruns
            .iter()
            .all(|p| !p.message.contains("seat_diameter"))
    );
}
