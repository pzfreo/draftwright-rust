//! Paper-space readability measures, independently perturbed across their limits.
use draftwright::{model, sheet_fit};
use serde_json::json;
use std::path::Path;

fn plate() -> (model::Analysis, model::Plan) {
    let (_, a) = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step"),
    )
    .unwrap();
    let s = serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1"}})).unwrap();
    let p = model::plan_recovered(&a, &s).unwrap();
    (a, p)
}

#[test]
fn clean_outside_text_and_arrows_are_readable_but_straddling_witnesses_are_not() {
    let (a, mut p) = plate();
    assert!(sheet_fit::border_contacts(&a, &p).is_empty());
    let mut border = p.clone();
    border
        .dimensions
        .iter_mut()
        .find_map(|d| d.placed.as_mut())
        .unwrap()
        .lines[0][0][0] = 10.0;
    assert!(!sheet_fit::border_contacts(&a, &border).is_empty());
    assert_eq!(sheet_fit::readability(&a, &p).text_arrows, 0);
    let d = p
        .dimensions
        .iter_mut()
        .find(|d| d.kind == "linear" && d.placed.as_ref().is_some_and(|p| p.arrows.len() == 2))
        .unwrap();
    let ink = d.placed.as_mut().unwrap();
    let axis = usize::from((ink.arrows[1][0][1] - ink.arrows[0][0][1]).abs() > 0.1);
    ink.label_box[axis] = ink.arrows[0][0][axis].min(ink.arrows[1][0][axis]) - 1.0;
    ink.label_box[axis + 2] = ink.arrows[0][0][axis].max(ink.arrows[1][0][axis]) + 1.0;
    assert!(sheet_fit::readability(&a, &p).text_arrows > 0);
}

#[test]
fn dimensioned_openings_use_two_mm_at_their_owning_views_scale() {
    let (a, mut p) = plate();
    assert_eq!(sheet_fit::readability(&a, &p).openings, 0);
    for view in &mut p.views {
        view.scale = 0.1;
    }
    assert!(sheet_fit::readability(&a, &p).openings > 0);
    for view in &mut p.views {
        view.scale = 1.0;
    }
    assert_eq!(sheet_fit::readability(&a, &p).openings, 0);
}

#[test]
fn first_dimension_line_needs_ten_mm_from_its_witnesses() {
    let (a, mut p) = plate();
    let before = sheet_fit::readability(&a, &p).lanes;
    let ink = p
        .dimensions
        .iter_mut()
        .find(|d| d.kind == "linear")
        .unwrap()
        .placed
        .as_mut()
        .unwrap();
    let other = usize::from((ink.arrows[1][0][0] - ink.arrows[0][0][0]).abs() > 0.1);
    let row = ink.arrows[0][0][other];
    for line in &mut ink.lines[..2] {
        line[0][other] = row - 3.0;
    }
    assert!(sheet_fit::readability(&a, &p).lanes > before);
}

#[test]
fn overlapping_parallel_dimension_spans_need_seven_mm() {
    let (a, mut p) = plate();
    let before = sheet_fit::readability(&a, &p).lanes;
    let mut d = p
        .dimensions
        .iter()
        .find(|d| d.kind == "linear")
        .unwrap()
        .clone();
    let ink = d.placed.as_mut().unwrap();
    let other = usize::from((ink.arrows[1][0][0] - ink.arrows[0][0][0]).abs() > 0.1);
    for point in ink.lines.iter_mut().chain(&mut ink.arrows).flatten() {
        point[other] += 3.5;
    }
    ink.label_box[other] += 3.5;
    ink.label_box[other + 2] += 3.5;
    p.dimensions.push(d);
    assert!(sheet_fit::readability(&a, &p).lanes > before);
}

#[test]
fn modelled_repetitive_lines_need_point_seven_mm_white_space() {
    let (mut a, mut p) = plate();
    let ink = a.edge_ink.as_mut().unwrap();
    ink.curved = serde_json::from_value(json!([
        {"bounds":[[-10,-10,0],[10,10,4]],"cylinder":{"origin":[0,0,0],"axis":[0,0,1],"radius":10}},
        {"bounds":[[-10,-10,4],[10,10,8]],"cylinder":{"origin":[0,0,4],"axis":[0,0,1],"radius":10}},
        {"bounds":[[-10,-10,8],[10,10,12]],"cylinder":{"origin":[0,0,8],"axis":[0,0,1],"radius":10}},
        {"bounds":[[-10,-10,12],[10,10,16]],"cylinder":{"origin":[0,0,12],"axis":[0,0,1],"radius":10}},
        {"bounds":[[-12,-12,0],[12,12,16]],"cylinder":{"origin":[0,0,0],"axis":[0,0,1],"radius":12}}
    ])).unwrap();
    for v in &mut p.views {
        v.scale = 0.2;
    }
    assert!(sheet_fit::readability(&a, &p).depiction > 0);
    for v in &mut p.views {
        v.scale = 0.5;
    }
    assert_eq!(sheet_fit::readability(&a, &p).depiction, 0);
}

#[test]
fn prismatic_baseline_intervals_preserve_a_two_mm_wall() {
    let (a, mut p) = plate();
    let mut d = p
        .dimensions
        .iter()
        .find(|d| d.kind == "linear" && d.view == "v:front")
        .unwrap()
        .clone();
    d.measurements = serde_json::from_value(
        json!([{"feature":"step_level","parameter":"step_height.length.h18","value":18.0}]),
    )
    .unwrap();
    p.dimensions.push(d.clone());
    d.measurements = serde_json::from_value(
        json!([{"feature":"envelope","parameter":"height.length","value":20.0}]),
    )
    .unwrap();
    p.dimensions.push(d);
    assert_eq!(sheet_fit::readability(&a, &p).steps, 0);
    let n = p.dimensions.len();
    p.dimensions[n - 2].measurements[0].value = 18.5;
    assert!(sheet_fit::readability(&a, &p).steps > 0);
    p.views
        .iter_mut()
        .find(|v| v.id == "v:front")
        .unwrap()
        .scale = 2.0;
    assert_eq!(sheet_fit::readability(&a, &p).steps, 0);
}

#[test]
fn explicit_paper_and_scale_override_an_unrelated_recovery_seed() {
    let (a, _) = plate();
    let seed_spec = serde_json::from_value(json!({"sheet":{"size":"A3","scale":"2:1"}})).unwrap();
    let seed = model::plan_escalated(&a, &seed_spec).unwrap();
    let spec = serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1"}})).unwrap();
    let p = model::recover(&a, &spec, seed).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
}

#[test]
fn a_hole_schedule_keeps_the_opening_floor_in_the_balloon_view() {
    let (a, mut p) = plate();
    let hole = a.features.iter().find(|f| f.kind == "hole").unwrap();
    p.dimensions.clear();
    p.balloons = serde_json::from_value(json!([{"id":"hole_schedule/tag/A1","tag":"A1","view":"v:plan","features":[hole.id],"certifying":true,"at":[20,20],"radius":0,"state":{"kind":"shown"}}])).unwrap();
    assert_eq!(sheet_fit::readability(&a, &p).openings, 0);
    p.views.iter_mut().find(|v| v.id == "v:plan").unwrap().scale = 0.1;
    assert_eq!(sheet_fit::readability(&a, &p).openings, 1);
}

#[path = "common/corpus.rs"]
mod corpus;

#[test]
fn one_readable_a3_precedes_two_a4_sheets_when_fill_needs_enlargement() {
    // The tapped-post intent has a clean, proven one-sheet A3 counterpart.
    // 10252 cannot exercise this success path: its enlargement changes retained
    // leader associations, so its fitted drawing must report refusal instead.
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step/hw_string_post_tapped_intent.step.gz");
    let (_, a) = model::analyse_native(&source).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (
            p.sheet.size.as_str(),
            p.sheet.scale.as_str(),
            1 + p.sheets.len()
        ),
        ("A3", "5:1", 1),
        "{}",
        p.sheet.basis
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert!(p.spec.sheets.is_none());
    assert!(p.views.iter().all(|v| v.sheet == 1));
    let single_spec =
        serde_json::from_value(json!({"sheet":{"size":"A4"},"sheets":{"max":1}})).unwrap();
    let single = model::plan_recovered(&a, &single_spec).unwrap();
    assert!(single.sheet.fill < sheet_fit::FILL_BAND[0]);
    assert!(single.sheet.scale_factor < p.sheet.scale_factor);
    let a3_spec =
        serde_json::from_value(json!({"sheet":{"size":"A3","scale":"5:1"},"sheets":{"max":1}}))
            .unwrap();
    let a3 = model::plan_recovered(&a, &a3_spec).unwrap();
    assert!(a3.sheets.is_empty());
    assert_eq!(sheet_fit::readability(&a, &a3).failures(), 0);
}

#[test]
fn clear_joint_routes_keep_the_prismatic_part_on_a4() {
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&dir.join("mfcadpp/10103.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "2:1")
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert!(sheet_fit::border_contacts(&a, &p).is_empty());
    assert!(p.sheets.is_empty());
    let spec = serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1"}})).unwrap();
    let compressed = model::plan_recovered(&a, &spec).unwrap();
    assert!(sheet_fit::readability(&a, &compressed).steps > 0);
}

#[test]
fn aligned_dimensions_measure_text_and_lanes_in_their_rotated_frame() {
    let (a, mut p) = plate();
    let mut d = p
        .dimensions
        .iter()
        .find(|d| d.kind == "linear")
        .unwrap()
        .clone();
    let ink = d.placed.as_mut().unwrap();
    let h = 0.5_f64.sqrt();
    // A 20 mm aligned span, with outside text and 10 mm witness clearance.
    ink.arrows = vec![
        [[0.0, 0.0], [-2.7 * h, -2.7 * h]],
        [[20.0 * h, 20.0 * h], [22.7 * h, 22.7 * h]],
    ];
    ink.lines = vec![
        [[10.0 * h, -10.0 * h], [0.0, 0.0]],
        [[30.0 * h, 10.0 * h], [20.0 * h, 20.0 * h]],
        [[0.0, 0.0], [20.0 * h, 20.0 * h]],
    ];
    ink.label_at = [25.0 * h, 25.0 * h];
    ink.rotation = 45.0;
    p.dimensions = vec![d.clone()];
    let before = sheet_fit::readability(&a, &p);
    assert_eq!(before.text_arrows, 0);
    let ink = p.dimensions[0].placed.as_mut().unwrap();
    ink.label_at = [19.0 * h, 19.0 * h];
    assert!(sheet_fit::readability(&a, &p).text_arrows > before.text_arrows);
    p.dimensions = vec![d.clone()];
    let ink = d.placed.as_mut().unwrap();
    for point in ink.lines.iter_mut().chain(&mut ink.arrows).flatten() {
        point[0] -= 3.5 * h;
        point[1] += 3.5 * h;
    }
    p.dimensions.push(d);
    assert!(sheet_fit::readability(&a, &p).lanes > before.lanes);
    p.dimensions.truncate(1);
    let ink = p.dimensions[0].placed.as_mut().unwrap();
    ink.lines[0][0] = [3.0 * h, -3.0 * h];
    assert!(sheet_fit::readability(&a, &p).lanes > before.lanes);
}

#[test]
fn outside_text_does_not_excuse_colliding_inward_arrowheads() {
    let (a, mut p) = plate();
    let mut d = p
        .dimensions
        .iter()
        .find(|d| d.kind == "linear")
        .unwrap()
        .clone();
    let ink = d.placed.as_mut().unwrap();
    ink.arrows = vec![[[0.0, 0.0], [2.7, 0.0]], [[2.0, 0.0], [-0.7, 0.0]]];
    ink.label_box = [10.0, 0.0, 20.0, 3.5];
    p.dimensions = vec![d];
    assert_eq!(sheet_fit::readability(&a, &p).text_arrows, 1);
    let ink = p.dimensions[0].placed.as_mut().unwrap();
    ink.arrows[0][1] = [-2.7, 0.0];
    ink.arrows[1][1] = [4.7, 0.0];
    assert_eq!(sheet_fit::readability(&a, &p).text_arrows, 0);
}

#[test]
fn local_floors_never_count_as_whole_drawing_failures() {
    let local = sheet_fit::Readability {
        steps: 1,
        openings: 2,
        native_openings: 3,
        depiction: 4,
        text_arrows: 0,
        lanes: 0,
    };
    assert_eq!(local.failures(), 10);
    assert_eq!(local.whole_drawing_failures(), 0);
    assert_eq!(
        sheet_fit::Readability {
            text_arrows: 2,
            lanes: 3,
            ..local
        }
        .whole_drawing_failures(),
        5
    );
}

#[test]
fn rejected_local_floor_up_moves_are_bounded_to_practical_paper() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS").map(std::path::PathBuf::from) else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    for file in [
        "mfcadpp/10000.step",
        "mfcadpp/10167.step",
        "mfcadpp/1004.step",
        "mfcadpp/10245.step",
        "mfcadpp/10119.step",
        "mfcadpp_holdout/229.step",
        "mfcadpp_holdout/520.step",
        "mfcadpp_holdout/808.step",
        "mfcadpp_holdout/257.step",
    ] {
        let (_, a) = model::analyse_native(&corpus.join(file)).unwrap();
        let plan = model::plan_recovered(&a, &model::Spec::default()).unwrap();
        assert!(
            matches!(plan.sheet.size.as_str(), "A4" | "A3"),
            "{file}: {}",
            plan.sheet.basis
        );
        let r = sheet_fit::readability(&a, &plan);
        if file == "mfcadpp_holdout/229.step" {
            assert_eq!(plan.sheet.size, "A4");
            assert_eq!(
                r.failures(),
                4,
                "joint routes recover the openings and two intervals; four remaining intervals stay explicitly refused"
            );
            assert_eq!(r.steps, 4);
            assert_eq!((r.openings, r.native_openings), (0, 0));
            assert!(sheet_fit::border_contacts(&a, &plan).is_empty());
            assert!(plan.sheets.is_empty());
            assert!(
                plan.problems
                    .iter()
                    .any(|q| q.code == "layout_recovery_exhausted")
            );
        }
        if plan
            .sheet
            .basis
            .contains("local readability recovered by enlarged detail")
        {
            assert_eq!(r.failures(), 0, "{file}");
            assert!(
                plan.views
                    .iter()
                    .any(|v| v.detail.as_ref().is_some_and(|d| !d.authored))
            );
        } else if r.failures() > 0 {
            assert!(
                plan.problems
                    .iter()
                    .any(|p| p.code == "layout_recovery_exhausted"),
                "{file}"
            );
        }
    }
}

#[test]
fn parent_coordinates_keep_the_enlarged_openings_size_carrier() {
    let (a, _) = plate();
    let spec =
        serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1"},"hole_table":"never"}))
            .unwrap();
    let mut p = model::plan_recovered(&a, &spec).unwrap();
    for view in &mut p.views {
        view.scale = 0.1;
    }
    let index = p
        .dimensions
        .iter()
        .position(|d| {
            d.kind == "callout"
                && d.measurements.iter().any(|m| {
                    a.features
                        .iter()
                        .any(|f| f.id == m.feature && f.kind == "hole")
                })
        })
        .unwrap();
    let owned: std::collections::BTreeSet<_> = p.dimensions[index]
        .measurements
        .iter()
        .filter(|m| {
            a.features
                .iter()
                .any(|f| f.id == m.feature && f.kind == "hole")
        })
        .map(|m| m.feature.clone())
        .collect();
    // Give each actual bore a parent datum measurement. Only its carrier
    // scale matters to this perturbation; the opening's real size stays owned
    // by its existing callout in the enlarged view.
    let mut coordinate = p
        .dimensions
        .iter()
        .find(|d| d.kind == "linear")
        .unwrap()
        .clone();
    coordinate.id = "d:parent-coordinate-probe".into();
    coordinate.view = p.dimensions[index].view.clone();
    coordinate.measurements = owned
        .iter()
        .map(|id| draftwright::dims::Measurement {
            feature: id.clone(),
            parameter: "location.length.x".into(),
            value: a.features.iter().find(|f| f.id == *id).unwrap().parameters["location"][0]
                .as_f64()
                .unwrap(),
        })
        .collect();
    p.dimensions.push(coordinate);
    let before = sheet_fit::readability(&a, &p).openings;
    assert!(p.dimensions.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| owned.contains(&m.feature) && m.parameter.contains("location"))
    }));
    let mut detail = p
        .views
        .iter()
        .find(|v| v.id == p.dimensions[index].view)
        .unwrap()
        .clone();
    detail.id = "v:detail_a".into();
    detail.scale = 1.0;
    p.dimensions[index].view = detail.id.clone();
    p.views.push(detail);
    assert_eq!(
        sheet_fit::readability(&a, &p).openings,
        before - owned.len()
    );
    assert!(
        p.dimensions
            .iter()
            .filter(|d| d
                .measurements
                .iter()
                .any(|m| owned.contains(&m.feature) && m.parameter.contains("location")))
            .all(|d| d.view != "v:detail_a")
    );
    p.views.last_mut().unwrap().scale = 0.1;
    assert_eq!(sheet_fit::readability(&a, &p).openings, before);
}

#[test]
fn a_local_interval_reservation_encloses_its_repaired_text() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) =
        model::analyse_native(&Path::new(&corpus).join("mfcadpp_holdout/520.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert_eq!(p.views.iter().filter(|v| v.detail.is_some()).count(), 1);
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.reference && d.view.starts_with("v:detail_"))
    );
    assert!(
        p.dimensions
            .iter()
            .any(|d| !d.reference && d.view.starts_with("v:detail_"))
    );
    assert!(
        !p.problems
            .iter()
            .any(|q| q.code == "annotation_outside_area")
    );
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
}

#[test]
fn a_crowded_main_view_keeps_its_readable_a3_scale() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    for name in ["10007", "1017"] {
        let (_, a) =
            model::analyse_native(&Path::new(&corpus).join(format!("mfcadpp/{name}.step")))
                .unwrap();
        let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A3", "2:1"),
            "{name}: {}",
            p.sheet.basis
        );
        assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
        assert!(p.views.iter().all(|v| v.detail.is_none()));
        assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
        assert!(p.sheet.basis.contains("halving a crowded main view"));
    }
}

#[test]
fn ordinary_paper_keeps_readable_controlled_openings_with_the_switch_off() {
    let (_, a) = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/boss_circle.step"),
    )
    .unwrap();
    for automatic in [None, Some(false)] {
        let mut spec = model::Spec::default();
        spec.sheet.automatic_details = automatic;
        let p = model::plan_recovered(&a, &spec).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A3", "1:1")
        );
        assert!(p.views.iter().all(|v| v.detail.is_none()));
        assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
        assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    }
}

#[test]
fn readable_bores_keep_the_whole_profile_without_circle_only_crops() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) =
        model::analyse_native(&Path::new(&corpus).join("mfcadpp_holdout/419.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(p.sheet.size, "A3");
    assert!(p.sheet.scale_factor >= 1.0);
    let control = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:step_level/step_height.length.h8_23")
        .unwrap();
    assert!(control.placed.is_some());
    let carrier = p.views.iter().find(|v| v.id == control.view).unwrap();
    assert!(carrier.scale >= 1.0);
    assert!(control.value * carrier.scale >= sheet_fit::MIN_STEP_SPAN);
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
}

#[test]
fn small_native_part_uses_a4_with_the_controlling_profile_and_clear_borders() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) = model::analyse_native(&Path::new(&corpus).join("mfcadpp/10098.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    let crop = p.views.iter().find(|v| v.detail.is_some()).unwrap();
    assert_eq!(crop.scale, 2.0);
    assert!(sheet_fit::border_contacts(&a, &p).is_empty());
    assert!(p.sheets.is_empty());
    let controls: std::collections::BTreeSet<_> = p
        .dimensions
        .iter()
        .filter(|d| {
            matches!(
                d.id.as_str(),
                "d:step_level/step_height.length.h4_581"
                    | "d:step_level/step_height.length.h10_385"
            )
        })
        .map(|d| d.id.as_str())
        .collect();
    assert_eq!(
        controls,
        [
            "d:step_level/step_height.length.h4_581",
            "d:step_level/step_height.length.h10_385"
        ]
        .into_iter()
        .collect()
    );
    assert!(
        p.dimensions
            .iter()
            .filter(|d| controls.contains(d.id.as_str()))
            .all(|d| !d.reference && d.placed.is_some() && d.view == crop.id)
    );
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    let r = sheet_fit::readability(&a, &p);
    assert_eq!(
        (r.native_openings, r.failures(), r.whole_drawing_failures()),
        (1, 1, 0)
    );
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted")
    );
}

#[test]
fn an_automatic_crop_cannot_reduce_the_independent_main_view_scale() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) = model::analyse_native(&Path::new(&corpus).join("mfcadpp/10049.step")).unwrap();
    let estimate = model::plan(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (estimate.sheet.size.as_str(), estimate.sheet.scale.as_str()),
        ("A3", "2:1")
    );
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    let ordinary: model::Spec =
        serde_json::from_value(json!({"sheet":{"automatic_details":false}})).unwrap();
    let ordinary = model::plan_recovered(&a, &ordinary).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "2:1")
    );
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        (ordinary.sheet.size.as_str(), ordinary.sheet.scale.as_str())
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    assert!(p.sheets.is_empty());
}

#[test]
fn refusing_a_crop_keeps_the_ordinary_paper_even_if_plain_a3_is_readable() {
    let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) =
        model::analyse_native(&Path::new(&corpus).join("mfcadpp_holdout/274.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    let without: model::Spec =
        serde_json::from_value(json!({"sheet":{"automatic_details":false}})).unwrap();
    let ordinary = model::plan_recovered(&a, &without).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        (ordinary.sheet.size.as_str(), ordinary.sheet.scale.as_str())
    );
    assert!(p.sheets.is_empty());
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    let r = sheet_fit::readability(&a, &p);
    assert_eq!(r.whole_drawing_failures(), 0);
    assert!(r.failures() > 0);
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted")
    );
}
