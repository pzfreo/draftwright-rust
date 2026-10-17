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
fn two_readable_a4_sheets_precede_one_a3_when_fill_needs_enlargement() {
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&dir.join("mfcadpp/10252.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (
            p.sheet.size.as_str(),
            p.sheet.scale.as_str(),
            1 + p.sheets.len()
        ),
        ("A4", "2:1", 2),
        "{}",
        p.sheet.basis
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert!(p.spec.sheets.is_none());
    assert!(
        p.views
            .iter()
            .filter(|v| ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()))
            .all(|v| v.sheet == 1)
    );
    let single_spec =
        serde_json::from_value(json!({"sheet":{"size":"A4"},"sheets":{"max":1}})).unwrap();
    let single = model::plan_recovered(&a, &single_spec).unwrap();
    assert!(single.sheet.fill < sheet_fit::FILL_BAND[0]);
    assert!(single.sheet.scale_factor < p.sheet.scale_factor);
    let a3_spec =
        serde_json::from_value(json!({"sheet":{"size":"A3","scale":"2:1"},"sheets":{"max":1}}))
            .unwrap();
    let a3 = model::plan_recovered(&a, &a3_spec).unwrap();
    assert!(a3.sheets.is_empty());
    assert_eq!(sheet_fit::readability(&a, &a3).failures(), 0);
}

#[test]
fn a3_explains_the_a4_prismatic_readability_refusal() {
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&dir.join("mfcadpp/10103.step")).unwrap();
    let p = model::plan_recovered(&a, &model::Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "5:1")
    );
    assert_eq!(sheet_fit::readability(&a, &p).failures(), 0);
    assert!(p.sheet.basis.contains("A4 multi-sheet:"));
    let spec = serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1"}})).unwrap();
    let compressed = model::plan_recovered(&a, &spec).unwrap();
    assert!(sheet_fit::readability(&a, &compressed).steps > 0);
}
