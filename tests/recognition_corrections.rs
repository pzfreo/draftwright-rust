//! Issue #76 item 2: author intent changes the inventory, never the measuring/rendering path.
use std::borrow::Cow;
use std::path::Path;

use draftwright::items::Op;
use draftwright::model::{self, Analysis, Spec};
use draftwright::recognition_corrections::{self as corrections, face_fingerprint};
use draftwright::{Session, api, haecceity::Part, lint};
use serde_json::{Value, json};

fn load(name: &str) -> (Part, Analysis) {
    model::analyse_native(&Path::new(env!("CARGO_MANIFEST_DIR")).join(name)).unwrap()
}
fn plate() -> (Part, Analysis) {
    load("tests/fixtures/parts/mounting_plate.step")
}
fn spec(value: Value) -> Spec {
    serde_json::from_value(value).unwrap()
}
fn font() -> draftwright::text::Font {
    draftwright::text::Font::new(api::MONO.to_vec()).unwrap()
}
fn saved(s: &Spec) -> Spec {
    serde_json::from_slice(&serde_json::to_vec(s).unwrap()).unwrap()
}

#[test]
fn absent_corrections_borrow_the_inventory_and_keep_default_bytes() {
    let (_, a) = plate();
    let default = Spec::default();
    assert!(matches!(
        corrections::effective(&a, &default).unwrap(),
        Cow::Borrowed(_)
    ));
    let p = model::plan_recovered(&a, &default).unwrap();
    let json = serde_json::to_value(&p).unwrap();
    assert!(json.get("recognition_corrections").is_none());
    assert!(
        !serde_json::to_string(&default)
            .unwrap()
            .contains("feature_declarations")
    );
    let explicit = spec(
        json!({"feature_declarations":[],"features":{"f:hole:1":{"suppress":false,"split":false}}}),
    );
    let mut explicit = model::plan_recovered(&a, &explicit).unwrap();
    explicit.spec = p.spec.clone();
    assert_eq!(serde_json::to_value(explicit).unwrap(), json);
}

#[test]
fn suppression_removes_dimensions_and_lint_requirements_but_does_not_hide_other_findings() {
    let (part, a) = plate();
    let hole = a.features.iter().find(|f| f.kind == "hole").unwrap();
    let s = spec(json!({"features":{&hole.id:{"suppress":true}}}));
    let p = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        p.recognition_corrections[0].from_spec,
        [format!("features/{}/suppress", hole.id)]
    );
    assert!(p.spec.features[&hole.id].anchor.is_some());
    assert!(
        p.dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .all(|m| m.feature != hole.id)
    );
    let effective = corrections::effective(&a, &p.spec).unwrap();
    assert!(effective.features.iter().all(|f| f.id != hole.id));
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(
        lint::lint(&effective, &drawing)
            .iter()
            .all(|i| !i.subjects.contains(&hole.id))
    );
    assert!(
        !lint::check(&effective, &[]).is_empty(),
        "unsuppressed requirements still judged"
    );
    let replay = model::plan_recovered(&a, &saved(&p.spec)).unwrap();
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
}

#[test]
fn declare_a_missed_bore_from_faces_then_lint_its_actual_printed_value() {
    let (part, mut a) = plate();
    let hole = a
        .features
        .iter()
        .find(|f| f.kind == "hole")
        .unwrap()
        .clone();
    let faces: Vec<_> = hole
        .faces
        .iter()
        .map(|&i| face_fingerprint(&a, i).unwrap())
        .collect();
    a.features.retain(|f| f.id != hole.id);
    a.recognition.as_mut().unwrap().patterns.clear();
    let s = spec(
        json!({"feature_declarations":[{"id":"f:declared:pilot","kind":"hole","faces":faces}]}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.feature == "f:declared:pilot" && m.parameter == "bore.diameter")
        })
        .unwrap();
    assert!(!d.default);
    assert!(
        d.from_spec
            .contains(&"feature_declarations/f:declared:pilot".into())
    );
    let effective = corrections::effective(&a, &s).unwrap();
    let declared = effective
        .features
        .iter()
        .find(|f| f.id == "f:declared:pilot")
        .unwrap();
    assert!(
        (declared.parameters["diameter"].as_f64().unwrap()
            - hole.parameters["diameter"].as_f64().unwrap())
        .abs()
            < 1e-5
    );
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    let mut marks = lint::marks(&drawing);
    assert!(!lint::check(&effective, &[]).is_empty());
    let mark = marks
        .iter_mut()
        .find(|m| {
            m.claims
                .iter()
                .any(|c| c.feature == "f:declared:pilot" && c.parameter == "bore.diameter")
        })
        .unwrap();
    mark.text = "ø999 THRU".into();
    assert!(
        lint::check(&effective, &marks)
            .iter()
            .any(|i| i.subjects.contains(&"f:declared:pilot".into())),
        "changed text is still checked"
    );
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(model::plan_recovered(&a, &saved(&p.spec)).unwrap()).unwrap()
    );
    a.face_attachments[hole.faces[0]].bounds = None;
    let e = model::plan_recovered(&a, &s).unwrap_err();
    assert!(
        e.message.contains("fingerprint")
            && e.message.contains("reattach")
            && e.message.contains("remove"),
        "{}",
        e.message
    );
}

#[test]
fn split_and_regroup_use_the_ordinary_grid_planner_and_saved_ids() {
    let (part, a) = plate();
    let pattern = a.recognition.as_ref().unwrap().patterns.first().unwrap();
    let split = spec(json!({"features":{&pattern.id:{"split":true}}}));
    let p = model::plan_recovered(&a, &split).unwrap();
    assert!(
        corrections::effective(&a, &p.spec)
            .unwrap()
            .recognition
            .as_ref()
            .unwrap()
            .patterns
            .is_empty()
    );
    assert!(p.dimensions.iter().all(|d| !d.id.contains("grid_pitch")));
    let grouped = spec(
        json!({"features":{"f:hole_pattern:mounting":{"pattern":{"arrangement":"grid","members":pattern.members}}}}),
    );
    let p = model::plan_recovered(&a, &grouped).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id.starts_with("d:f:hole_pattern:mounting/") && d.id.contains("grid_pitch"))
    );
    assert!(
        pattern
            .members
            .iter()
            .all(|id| p.spec.features[id].anchor.is_some())
    );
    let effective = corrections::effective(&a, &p.spec).unwrap();
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(!lint::check(&effective, &[]).is_empty());
    let findings = lint::lint(&effective, &drawing);
    assert!(
        !findings
            .iter()
            .any(|i| i.code == "hole_pattern_requirement_missing"),
        "{findings:?}"
    );
    let mut again = a.clone();
    again.features.reverse();
    let replay = model::plan_recovered(&again, &saved(&p.spec)).unwrap();
    assert!(
        replay
            .dimensions
            .iter()
            .any(|d| d.id.starts_with("d:f:hole_pattern:mounting/"))
    );
}

#[test]
fn bad_references_conflicting_actions_and_false_patterns_are_refused() {
    let (_, a) = plate();
    for value in [
        json!({"features":{"f:hole:missing":{"suppress":true}}}),
        json!({"features":{"f:hole:1":{"suppress":true,"reclassify":"boss"}}}),
        json!({"features":{"f:hole:1":{"split":true}}}),
        json!({"features":{"f:hole_pattern:bad":{"pattern":{"arrangement":"linear","members":["f:hole:1","f:hole:1"]}}}}),
        json!({"features":{"f:hole_pattern:bad":{"pattern":{"arrangement":"linear","members":["f:hole:1","f:hole:999"]}}}}),
    ] {
        let e = model::plan_recovered(&a, &spec(value)).unwrap_err();
        assert!(
            e.message.contains("remove") && e.message.contains("reattach"),
            "{}",
            e.message
        );
    }
    let pattern = &a.recognition.as_ref().unwrap().patterns[0];
    let bad = spec(
        json!({"features":{"f:hole_pattern:bad":{"pattern":{"arrangement":"linear","members":pattern.members}}}}),
    );
    assert!(
        model::plan_recovered(&a, &bad)
            .unwrap_err()
            .message
            .contains("principal axis")
    );
    assert!(
        serde_json::from_value::<Spec>(json!({"features":{"f:hole:1":{"reclassify":"dragon"}}}))
            .is_err()
    );
    assert!(model::edit(&Spec::default(),&json!([{"op":"add","path":"feature_declarations","value":{"id":"bad","kind":"hole","faces":[]}}])).is_err());
}

#[test]
fn boss_to_turned_step_reads_the_faces_and_preserves_the_feature_identity() {
    let (part, mut a) = load("tests/fixtures/parts/boss_circle.step");
    let boss = a
        .features
        .iter()
        .find(|f| f.kind == "boss")
        .unwrap()
        .clone();
    a.features
        .retain(|f| f.id == boss.id || f.faces != boss.faces);
    let s = spec(json!({"features":{&boss.id:{"reclassify":"turned_step"}}}));
    let p = model::plan_recovered(&a, &s).unwrap();
    let effective = corrections::effective(&a, &p.spec).unwrap();
    assert_eq!(
        effective
            .features
            .iter()
            .find(|f| f.id == boss.id)
            .unwrap()
            .kind,
        "turned_step"
    );
    let d = p
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.feature == boss.id && m.parameter == "step.length")
        })
        .unwrap();
    assert!(!d.default);
    assert!(
        d.from_spec
            .contains(&format!("features/{}/reclassify", boss.id))
    );
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(!lint::check(&effective, &[]).is_empty());
    let _ = lint::lint(&effective, &drawing);
    let wrong = spec(json!({"features":{&boss.id:{"reclassify":"pocket"}}}));
    assert!(
        model::plan_recovered(&a, &wrong)
            .unwrap_err()
            .message
            .contains("reinterpretations")
    );
}

#[test]
fn declarations_use_rectangular_wall_geometry_and_refuse_unproved_kind_changes() {
    let (_, a) = load("tests/fixtures/recognition/issues/pocket_z.step");
    let faces: Vec<_> = a
        .face_attachments
        .iter()
        .enumerate()
        .filter(|(i, f)| f.surface.as_deref() == Some("plane") && !a.stock_faces.contains(i))
        .map(|(i, _)| face_fingerprint(&a, i).unwrap())
        .collect();
    let s = spec(
        json!({"feature_declarations":[{"id":"f:declared:recess","kind":"pocket","faces":faces}]}),
    );
    let effective = corrections::effective(&a, &s).unwrap();
    let f = effective
        .features
        .iter()
        .find(|f| f.id == "f:declared:recess")
        .unwrap();
    let draftwright::lint::section_recesses::Grammar::Pocket(pocket) =
        draftwright::lint::section_recesses::grammar(f).unwrap()
    else {
        panic!("declared pocket grammar");
    };
    assert_eq!((pocket.width, pocket.length, pocket.depth), (14., 22., 10.));
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(p.dimensions.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| m.feature == "f:declared:recess")
            && !d.default
    }));
    assert!(
        lint::check(&effective, &[])
            .iter()
            .any(|i| i.subjects.contains(&"f:declared:recess".into()))
    );
    let mut wrong = s.clone();
    wrong.feature_declarations[0].kind = corrections::Kind::Slot;
    assert!(
        model::plan_recovered(&a, &wrong)
            .unwrap_err()
            .message
            .contains("floor")
    );
}

#[test]
fn session_preview_apply_history_and_declaration_ops_share_validation() {
    let (part, a) = plate();
    let id = a
        .features
        .iter()
        .find(|f| f.kind == "hole")
        .unwrap()
        .id
        .clone();
    let mut session = Session::new(part, a, Spec::default()).unwrap();
    let before = serde_json::to_value(session.state()).unwrap();
    let ops = [Op::set(format!("features/{id}/suppress"), json!(true))];
    session.apply(&ops, false).unwrap();
    assert_eq!(serde_json::to_value(session.state()).unwrap(), before);
    session.apply(&ops, true).unwrap();
    assert!(session.spec().features[&id].anchor.is_some());
    let after = serde_json::to_value(session.state()).unwrap();
    session.undo().unwrap();
    assert_eq!(serde_json::to_value(session.state()).unwrap(), before);
    session.redo().unwrap();
    assert_eq!(serde_json::to_value(session.state()).unwrap(), after);
    assert!(
        session
            .apply(&[Op::set("features/f:missing/suppress", json!(true))], true)
            .is_err()
    );
    assert_eq!(serde_json::to_value(session.state()).unwrap(), after);
    let faces = session
        .analysis()
        .features
        .iter()
        .find(|f| f.id == id)
        .unwrap()
        .faces
        .iter()
        .map(|&i| face_fingerprint(session.analysis(), i).unwrap())
        .collect::<Vec<_>>();
    session
        .apply(
            &[Op::add(
                "feature_declarations",
                json!({"id":"f:declared:replacement","kind":"hole","faces":faces}),
            )],
            true,
        )
        .unwrap();
    assert!(
        session
            .plan()
            .dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:declared:replacement")
    );
}

#[test]
fn exact_saved_face_anchors_resolve_after_reanalysis_and_refuse_vanished_faces() {
    let (_, a) = plate();
    let id = a
        .features
        .iter()
        .find(|f| f.kind == "hole")
        .unwrap()
        .id
        .clone();
    let p = model::plan_recovered(&a, &spec(json!({"features":{&id:{"suppress":true}}}))).unwrap();
    let (_, mut again) = plate();
    again.binding.source_sha256 = "a new source containing the same faces".into();
    let replay = model::plan_recovered(&again, &saved(&p.spec)).unwrap();
    assert_eq!(replay.recognition_corrections[0].id, id);
    assert_eq!(
        replay.spec.binding.as_ref().unwrap().source_sha256,
        again.binding.source_sha256
    );
    assert!(
        replay
            .dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .all(|m| m.feature != id)
    );
    let face = again.features.iter().find(|f| f.id == id).unwrap().faces[0];
    again.face_attachments[face].bounds = None;
    let e = model::plan_recovered(&again, &p.spec).unwrap_err();
    assert!(
        e.message.contains("fingerprint") && e.message.contains("reattach"),
        "{}",
        e.message
    );
}

#[test]
fn declaration_faces_can_be_reindexed_without_changing_the_authored_id() {
    let (_, mut a) = plate();
    let hole = a
        .features
        .iter()
        .find(|f| f.kind == "hole")
        .unwrap()
        .clone();
    let faces = hole
        .faces
        .iter()
        .map(|&i| face_fingerprint(&a, i).unwrap())
        .collect::<Vec<_>>();
    a.features.clear();
    a.recognition.as_mut().unwrap().patterns.clear();
    let s = spec(
        json!({"feature_declarations":[{"id":"f:declared:mount","kind":"hole","faces":faces}]}),
    );
    let first = corrections::effective(&a, &s).unwrap();
    let expected = first
        .features
        .iter()
        .find(|f| f.id == "f:declared:mount")
        .unwrap()
        .parameters
        .clone();
    a.face_attachments.reverse();
    let again = corrections::effective(&a, &saved(&s)).unwrap();
    let f = again
        .features
        .iter()
        .find(|f| f.id == "f:declared:mount")
        .unwrap();
    assert_eq!(f.parameters, expected);
    assert_ne!(f.faces, hole.faces);
}

#[test]
fn an_authored_linear_pair_replaces_its_automatic_grid_membership() {
    let (_, a) = plate();
    let holes: Vec<_> = draftwright::bridge::holes::read(&a)
        .into_iter()
        .map(|(_, h)| h.unwrap())
        .collect();
    let first = &holes[0];
    let k = first.principal.unwrap().0;
    let second = holes
        .iter()
        .skip(1)
        .find(|h| {
            (0..3)
                .filter(|&i| i != k)
                .any(|i| (h.at[i] - first.at[i]).abs() < 1e-5)
        })
        .unwrap();
    let s = spec(
        json!({"features":{"f:hole_pattern:pair":{"pattern":{"arrangement":"linear","members":[first.feature.id,second.feature.id]}}}}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:f:hole_pattern:pair/pitch.length" && !d.default)
    );
    let effective = corrections::effective(&a, &p.spec).unwrap();
    let patterns = &effective.recognition.as_ref().unwrap().patterns;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].id, "f:hole_pattern:pair");
}

#[test]
fn a_bolt_circle_derives_its_pcd_and_lint_still_requires_its_position() {
    let (part, a) = model::analyse_native_bytes(
        "bolt-circle.step",
        include_bytes!("fixtures/recognition_corrections/bolt_circle.step.gz"),
    )
    .unwrap();
    let members: Vec<_> = a
        .features
        .iter()
        .filter(|f| {
            f.kind == "hole"
                && f.parameters["diameter"]
                    .as_f64()
                    .is_some_and(|d| (d - 4.).abs() < 1e-5)
        })
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(members.len(), 4);
    let s = spec(
        json!({"features":{"f:hole_pattern:bolts":{"pattern":{"arrangement":"bolt_circle","members":members}}}}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    let effective = corrections::effective(&a, &p.spec).unwrap();
    let circle = effective
        .recognition
        .as_ref()
        .unwrap()
        .patterns
        .iter()
        .find(|p| p.id == "f:hole_pattern:bolts")
        .unwrap();
    assert!((circle.parameters["diameter"].as_f64().unwrap() - 40.).abs() < 1e-5);
    assert!(p.dimensions.iter().any(|d| {
        d.label.contains("BC")
            && d.from_spec
                .contains(&"features/f:hole_pattern:bolts/pattern".into())
    }));
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(
        !lint::lint(&effective, &drawing)
            .iter()
            .any(|i| i.code == "hole_pattern_requirement_missing")
    );
    assert!(
        lint::check(&effective, &[])
            .iter()
            .any(|i| i.code == "hole_pattern_requirement_missing")
    );
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(model::plan_recovered(&a, &saved(&p.spec)).unwrap()).unwrap()
    );
}

#[test]
fn published_mcp_schema_and_common_model_accept_corrections_and_refuse_bad_shapes() {
    let input = draftwright::mcp::schema("inspect_part", false).unwrap();
    assert!(
        draftwright::mcp::validate(&input, &json!({"part_ref":"part","part_ref\n":"bad"})).is_err()
    );
    let s = json!({"feature_declarations":[{"id":"f:declared:bore","kind":"hole","faces":["face:sha256:0000000000000000000000000000000000000000000000000000000000000000"]}]});
    assert!(draftwright::mcp::validate(&input, &json!({"part_ref":"part","spec":s})).is_ok());
    for bad in [
        json!({"id":"f:declared:x","kind":"hole","faces":"face:bad"}),
        json!({"id":"f:declared:x","kind":"hole","faces":[],"diameter":20}),
    ] {
        assert!(
            draftwright::mcp::validate(
                &input,
                &json!({"part_ref":"part","spec":{"feature_declarations":[bad]}})
            )
            .is_err()
        );
    }
}
