//! #76 item 3: model-space camera intent and auxiliary/partial view authoring.
#[path = "common/mcp_schema.rs"]
mod mcp_schema;
use draftwright::{
    Session, api,
    items::Op,
    lint,
    model::{self, Analysis, Plan, Spec},
    text::Font,
};
use serde_json::{Value, json};
use std::path::Path;

fn fixture(name: &str) -> (draftwright::haecceity::Part, Analysis) {
    model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}")),
    )
    .unwrap()
}
fn plate() -> (draftwright::haecceity::Part, Analysis) {
    fixture("parts/mounting_plate.step")
}
fn spec(v: Value) -> Spec {
    serde_json::from_value(v).unwrap()
}
fn font() -> Font {
    Font::new(api::MONO.to_vec()).unwrap()
}
fn view<'a>(p: &'a Plan, name: &str) -> &'a model::PlannedView {
    p.views
        .iter()
        .find(|v| v.id == format!("v:{name}"))
        .unwrap()
}
fn face(a: &Analysis, axis: Option<usize>) -> String {
    let i = a
        .face_attachments
        .iter()
        .position(|f| f.surface.as_deref() == Some("plane") && f.axis == axis)
        .unwrap();
    draftwright::recognition_corrections::face_fingerprint(a, i).unwrap()
}
fn fixed() -> Value {
    json!({"sheet":{"size":"A3","scale":"1:1"},"views":{"front":{},"plan":{},"side":{}}})
}
fn aux(a: &Analysis) -> Spec {
    let mut s = fixed();
    s["views"]["detail_b"] = json!({"auxiliary":{"face":face(a,None),"from":"front"}});
    spec(s)
}

#[test]
fn front_direction_rotation_and_explicit_principals_are_applied_without_changing_measurements() {
    let (part, a) = plate();
    let auto = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(view(&auto, "front").direction, [0., -1., 0.]);
    assert_eq!(view(&auto, "front").up, [0., 0., 1.]);
    assert!(auto.views.iter().all(|v| v.from_spec.is_empty()));
    for direction in ["x", "-x", "y", "-y", "z", "-z"] {
        for rotation in [0., 90., 30.] {
            let mut s = fixed();
            s["views"]["front"] = json!({"orientation":{"from":direction,"rotation":rotation},"principal_views":["front","plan","side"]});
            let p = model::plan(&a, &spec(s)).unwrap();
            for v in &p.views {
                for c in v.direction.iter().chain(&v.up) {
                    assert!(
                        *c != 0. || !c.is_sign_negative(),
                        "{} camera must serialize zero identically in native and browser JSON",
                        v.id
                    );
                }
            }
            assert!(
                view(&p, "front")
                    .from_spec
                    .contains(&"views/front/orientation".into())
            );
            assert!(
                view(&p, "side")
                    .from_spec
                    .contains(&"views/front/principal_views".into())
            );
            let drawing = model::render(&part, &a, &p, &font()).unwrap();
            let issues = lint::structural::check(&drawing);
            assert!(
                !issues.iter().any(|i| i.code == "label_vs_measured"),
                "{direction} {rotation}: {issues:?}"
            );
            for d in &p.dimensions {
                if let Some(old) = auto.dimensions.iter().find(|v| v.id == d.id) {
                    assert_eq!(d.value, old.value);
                }
            }
        }
    }
    let restored = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        serde_json::to_value(auto).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
}

#[test]
fn feature_axis_and_both_face_fingerprint_vocabularies_choose_front() {
    let (_, a) = plate();
    let mut refs = vec!["f:hole:1".to_string(), face(&a, Some(2))];
    refs.push(a.edge_ink.as_ref().unwrap().references.iter().find(|(_,rs)|rs.len()==1 && matches!(&rs[0],model::dimension_geometry::Reference::Plane{normal,..} if normal[2].abs()>0.999)).unwrap().0.clone());
    for from in refs {
        let mut s = fixed();
        s["views"]["front"]["orientation"] = json!({"from":from});
        let p = model::plan(&a, &spec(s)).unwrap();
        assert!(view(&p, "front").direction[2].abs() > 0.999);
    }
}

#[test]
fn explicit_set_can_restore_a_reduced_view_and_exclude_other_principals() {
    let (_, a) = fixture("parts/turned.step");
    let auto = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        auto.views
            .iter()
            .filter(|v| ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()))
            .count()
            < 3
    );
    let p = model::plan(
        &a,
        &spec(json!({"views":{"front":{"principal_views":["front","plan","side"]}}})),
    )
    .unwrap();
    for n in ["front", "plan", "side"] {
        assert!(!view(&p, n).default);
    }
    let p = model::plan(
        &a,
        &spec(json!({"views":{"front":{"principal_views":["front"]}}})),
    )
    .unwrap();
    assert!(
        p.views
            .iter()
            .all(|v| !["v:side", "v:plan"].contains(&v.id.as_str()))
    );
    assert!(!p.problems.is_empty(), "missing coverage remains visible");
}

#[test]
fn auxiliary_is_true_shape_from_an_edge_view_and_partial_can_use_its_camera() {
    let (part, a) = fixture("recognition/steps/paired_ramp_step.step");
    let mut s = aux(&a);
    let normal = a
        .face_attachments
        .iter()
        .find(|f| f.surface.as_deref() == Some("plane") && f.axis.is_none())
        .unwrap()
        .normal
        .unwrap();
    let p = model::plan(&a, &s).unwrap();
    assert_eq!(view(&p, "detail_b").direction, normal);
    let source = view(&p, "front");
    let target = view(&p, "detail_b");
    let [d, n] = [source.direction, target.direction];
    let edge = [
        d[1] * n[2] - d[2] * n[1],
        d[2] * n[0] - d[0] * n[2],
        d[0] * n[1] - d[1] * n[0],
    ];
    let before = haecceity::hlr::View::new(source.direction, source.up)
        .unwrap()
        .map(edge);
    let after = haecceity::hlr::View::new(target.direction, target.up)
        .unwrap()
        .map(edge);
    assert!((before[0] - after[0]).abs() < 1e-6 && (before[1] - after[1]).abs() < 1e-6);

    assert_eq!(
        view(&p, "detail_b").detail.as_ref().unwrap().source_view,
        "v:front"
    );
    let target = s.views["detail_b"].auxiliary.as_ref().unwrap().face.clone();
    s.views.insert(
        "detail_c".into(),
        serde_json::from_value(
            json!({"partial":{"from":"detail_b","around":target,"scale_factor":2},"hidden":false}),
        )
        .unwrap(),
    );
    let p = model::plan(&a, &s).unwrap();
    let b = view(&p, "detail_b");
    let c = view(&p, "detail_c");
    assert_eq!(c.direction, b.direction);
    assert_eq!(c.up, b.up);
    assert_eq!(c.scale, 2. * b.scale);
    assert_eq!(c.line_style.hidden, Some(false));
    assert!(c.from_spec.contains(&"views/detail_c/partial".into()));
    assert!(c.from_spec.contains(&"views/detail_c/hidden".into()));
    let d = model::render(&part, &a, &p, &font()).unwrap();
    assert!(d.text.iter().any(|t| t.text == "AUXILIARY B (1:1)"));
    assert!(d.text.iter().any(|t| t.text == "PARTIAL C (2:1)"));
    assert!(
        !lint::structural::check(&d)
            .iter()
            .any(|i| i.code == "derived_view_identifier_reused")
    );
    let mut cache = model::Projections::default();
    model::render_geometry(&part, &a, &p, &mut cache).unwrap();
    assert_eq!(
        cache.stats().projections,
        p.views
            .iter()
            .filter(
                |v| v.detail.is_none() || v.detail.as_ref().is_some_and(|d| d.kind == "auxiliary")
            )
            .count()
    );
}

#[test]
fn bad_references_sources_conflicts_and_degenerate_regions_are_refused_with_remedies() {
    let (_, a) = plate();
    for (field, value) in [
        ("orientation", json!({"from":"face:sha256:missing"})),
        ("orientation", json!({"from":"face:999"})),
        ("orientation", json!({"from":"f:hole:999"})),
        ("orientation", json!({"from":"-y","rotation":361})),
        ("principal_views", json!([])),
        ("principal_views", json!(["front", "front"])),
        ("principal_views", json!(["iso"])),
    ] {
        let mut s = fixed();
        s["views"]["front"][field] = value;
        let error = model::plan(&a, &spec(s)).unwrap_err();
        assert!(
            error.message.contains("remove") || error.message.contains("reattach"),
            "{error:?}"
        );
    }
    for value in [
        json!({"partial":{"from":"plan","around":"f:hole:999"}}),
        json!({"partial":{"from":"iso","around":"f:hole:1"}}),
        json!({"partial":{"from":"front","around":face(&a,Some(2))}}),
        json!({"partial":{"from":"plan","around":[]}}),
        json!({"auxiliary":{"from":"plan","face":face(&a,Some(2))}}),
        json!({"auxiliary":{"from":"missing","face":face(&a,Some(2))}}),
        json!({"partial":{"from":"plan","around":"f:hole:1","scale_factor":0}}),
        json!({"partial":{"from":"plan","around":"f:hole:1"},"detail":{"around":"f:hole:1"}}),
    ] {
        let mut s = fixed();
        s["views"]["detail_b"] = value;
        let authored = spec(s);
        let e = model::plan(&a, &authored).unwrap_err();
        assert!(
            e.message.contains("remove") || e.message.contains("reattach"),
            "{e:?}"
        );
        for choice in &e.choices {
            if let Some(path) = choice.strip_prefix("remove ") {
                let repaired =
                    model::edit(&authored, &json!([{"op":"remove","path":path}])).unwrap();
                model::plan(&a, &repaired).expect("the suggested removal leaves a valid Spec");
            }
        }
    }
    let mut s = fixed();
    s["views"]["front"]["principal_views"] = json!(["front", "plan"]);
    s["views"]["plan"]["omit"] = json!(true);
    assert!(
        model::plan(&a, &spec(s))
            .unwrap_err()
            .message
            .contains("both requested and omitted")
    );
}

#[test]
fn saved_spec_round_trips_and_feature_anchors_survive_reordering() {
    let (part, a) = plate();
    let mut s = fixed();
    s["views"]["front"]["orientation"] = json!({"from":"f:hole:1"});
    s["views"]["detail_b"] =
        json!({"partial":{"from":"front","around":"f:hole:1","scale_factor":2}});
    let p = model::plan(&a, &spec(s)).unwrap();
    assert!(p.spec.features["f:hole:1"].anchor.is_some());
    let saved: Spec = serde_json::from_slice(&serde_json::to_vec(&p.spec).unwrap()).unwrap();
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert_eq!(
        model::embedded_spec(drawing.svg().as_bytes())
            .unwrap()
            .unwrap(),
        saved
    );
    assert_eq!(
        model::embedded_spec(&drawing.png(72.).unwrap())
            .unwrap()
            .unwrap(),
        saved
    );
    let mut changed = a.clone();
    for f in &mut changed.features {
        if f.id == "f:hole:1" {
            f.id = "f:hole:99".into();
        }
    }
    let rebound = model::plan(&changed, &saved).unwrap();
    assert_eq!(
        rebound.spec.views["front"]
            .orientation
            .as_ref()
            .unwrap()
            .from,
        "f:hole:99"
    );
    assert_eq!(
        rebound.spec.views["detail_b"]
            .partial
            .as_ref()
            .unwrap()
            .around,
        draftwright::detail::Around::One("f:hole:99".into())
    );
    assert_eq!(
        view(&p, "front").direction,
        view(&rebound, "front").direction
    );
}

#[test]
fn session_edits_preview_replay_history_and_refusal_are_transactional() {
    let (part, a) = plate();
    let mut session = Session::new(part, a, Spec::default()).unwrap();
    let initial = session.drawing().svg();
    let mut state = session.state().clone();
    let ops = [
        Op::set("views/front/orientation", json!({"from":"z","rotation":90})),
        Op::set(
            "views/front/principal_views",
            json!(["front", "plan", "side"]),
        ),
    ];
    session.apply(&ops, false).unwrap();
    assert_eq!(session.drawing().svg(), initial);
    let delta = session.apply(&ops, true).unwrap();
    delta.apply_to(&mut state).unwrap();
    assert_eq!(
        serde_json::to_value(state).unwrap(),
        serde_json::to_value(session.state()).unwrap()
    );
    assert!(
        session
            .options("sheet")
            .unwrap()
            .settable
            .contains_key("views/detail_<a-h>/auxiliary")
    );
    assert!(
        session
            .options("v:front")
            .unwrap()
            .settable
            .contains_key("views/front/orientation")
    );
    let changed = session.drawing().svg();
    assert!(
        session
            .apply(
                &[Op::set("views/front/orientation", json!({"from":"bogus"}))],
                true
            )
            .is_err()
    );
    assert_eq!(session.drawing().svg(), changed);
    session.undo().unwrap();
    assert_eq!(session.drawing().svg(), initial);
    session.redo().unwrap();
    assert_eq!(session.drawing().svg(), changed);
}

#[test]
fn lint_still_checks_authored_output_and_missing_coverage() {
    let (part, a) = plate();
    let mut s = fixed();
    s["views"]["front"]["orientation"] = json!({"from":"-y","rotation":90});
    let p = model::plan(&a, &spec(s)).unwrap();
    let mut d = model::render(&part, &a, &p, &font()).unwrap();
    let id = "d:envelope/width.length";
    let index = d.annotations.iter().find(|a| a.name == id).unwrap().runs[0];
    assert!(
        !lint::structural::check(&d)
            .iter()
            .any(|i| i.code == "label_vs_measured")
    );
    d.text[index].text = "999".into();
    assert!(
        lint::structural::check(&d)
            .iter()
            .any(|i| i.code == "label_vs_measured" && i.annotation.as_deref() == Some(id))
    );
    // An authored set of views cannot suppress independent completeness findings.
    let p = model::plan(
        &a,
        &spec(json!({"views":{"front":{"principal_views":["front"]}}})),
    )
    .unwrap();
    assert!(
        p.problems
            .iter()
            .any(|p| p.code.contains("view") || p.code.contains("dimension"))
    );
}

#[test]
fn auxiliary_and_partial_views_use_the_same_multisheet_allocator_as_details() {
    let (_, a) = fixture("recognition/steps/paired_ramp_step.step");
    let mut s = aux(&a);
    s.sheets = Some(serde_json::from_value(json!({"max":2})).unwrap());
    s.views.get_mut("detail_b").unwrap().sheet = Some(2);
    let p = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(view(&p, "detail_b").sheet, 2);
    assert_eq!(view(&p, "detail_b").scale, p.sheet.scale_factor);
}

#[test]
fn full_spec_schema_describes_setup_without_expanding_tools_list() {
    let schema: Value = serde_json::from_str(include_str!("../schemas/spec.json")).unwrap();
    let defs = &schema["$defs"];
    for field in ["orientation", "principal_views", "auxiliary", "partial"] {
        assert!(!defs["view"]["properties"][field].is_null());
    }
    assert_eq!(
        defs["orientation"]["properties"]["rotation"]["maximum"],
        360
    );
    assert_eq!(
        defs["auxiliary"]["properties"]["scale_factor"]["exclusiveMinimum"],
        0
    );
    let schema = mcp_schema::with_full_spec("get_spec", false);
    let envelope = |v: Value| json!({"part_ref":"part","spec":{"views":{"front":v}}});
    assert!(
        draftwright::mcp::validate(
            &schema,
            &envelope(
                json!({"orientation":{"from":"z","rotation":45},"principal_views":["front","plan"]})
            )
        )
        .is_ok()
    );
    assert!(
        draftwright::mcp::validate(
            &schema,
            &envelope(json!({"orientation":{"from":"z","rotation":361}}))
        )
        .is_err()
    );
    assert!(
        draftwright::mcp::validate(&schema, &envelope(json!({"principal_views":["iso"]}))).is_err()
    );
    assert!(draftwright::mcp::validate(&schema,&json!({"part_ref":"part","spec":{"views":{"detail_b":{"auxiliary":{"face":"fingerprint","from":"plan","scale_factor":0}}}}})).is_err());
}

#[test]
fn automatic_sheet_search_skips_infeasible_auxiliary_scale_candidates() {
    let (_, a) = fixture("recognition/steps/paired_ramp_step.step");
    let mut s = aux(&a);
    s.sheet = Default::default();
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(view(&p, "detail_b").detail.as_ref().unwrap().authored);
    assert!(
        view(&p, "detail_b")
            .from_spec
            .contains(&"views/detail_b/auxiliary".into())
    );
}

#[test]
fn auxiliary_accepts_explicit_true_size_dimensions_and_preserves_one_owner() {
    let (part, a) = fixture("recognition/steps/paired_ramp_step.step");
    let mut s = aux(&a);
    s.dimensions.insert(
        "d:envelope/depth.length".into(),
        serde_json::from_value(json!({"view":"detail_b"})).unwrap(),
    );
    let p = model::plan(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:envelope/depth.length")
        .unwrap();
    assert_eq!(d.view, "v:detail_b");
    assert!(d.placed.is_some());
    assert_eq!(
        p.dimensions
            .iter()
            .filter(|d| d.id == "d:envelope/depth.length")
            .count(),
        1
    );
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(
        !lint::structural::check(&drawing)
            .iter()
            .any(|i| i.code == "label_vs_measured")
    );
}

#[test]
fn remote_partial_keeps_its_parent_marker_and_scale_on_separate_pages() {
    let (part, a) = plate();
    let mut s = fixed();
    s["sheets"] = json!({"max":2});
    s["views"]["detail_b"] =
        json!({"partial":{"from":"plan","around":"f:hole:1","scale_factor":2},"sheet":2});
    let p = model::plan_recovered(&a, &spec(s)).unwrap();
    assert_eq!(view(&p, "detail_b").sheet, 2);
    let drawings = model::render_sheets(&part, &a, &p, &font()).unwrap();
    assert_eq!(drawings.len(), 2);
    assert!(drawings[0].text.iter().any(|t| t.text == "B (SHEET 2)"));
    assert!(drawings[1].text.iter().any(|t| t.text == "PARTIAL B (2:1)"));
    assert!(
        drawings[0]
            .text
            .iter()
            .all(|t| !t.text.starts_with("PARTIAL "))
    );
}

#[test]
fn physical_sections_follow_a_rotated_marking_view_and_keep_their_cut_plane() {
    let (part, a) = fixture("recognition/issues/counterbored_block.step");
    let auto = model::plan(&a, &spec(fixed())).unwrap();
    let cut = auto
        .views
        .iter()
        .find_map(|v| v.cut.as_ref())
        .expect("counterbore section");
    let mut s = fixed();
    s["views"]["front"]["orientation"] = json!({"from":"x","rotation":0});
    let p = model::plan(&a, &spec(s)).unwrap();
    let section = p.views.iter().find(|v| v.cut.is_some()).unwrap();
    assert_eq!(section.cut.as_ref().unwrap().origin, cut.origin);
    assert_eq!(section.cut.as_ref().unwrap().normal, cut.normal);
    let marker = view(&p, "plan")
        .line_work
        .iter()
        .find(|w| w.kind == "cutting_plane")
        .unwrap();
    let b = marker.curves[0].bbox();
    assert!(
        (b[2] - b[0]).abs() < 1e-6,
        "cutting line rotates into the plan"
    );
    let drawing = model::render(&part, &a, &p, &font()).unwrap();
    assert!(drawing.text.iter().any(|t| t.text.starts_with("SECTION ")));
    assert!(
        drawing
            .sheet
            .items
            .iter()
            .any(|i| i.name == "section_arrow_left")
    );
}

#[test]
fn invalid_feature_references_name_the_view_edit_that_can_be_removed() {
    let (_, a) = plate();
    let s = spec(json!({"views":{"front":{"orientation":{"from":"f:hole:999"}}}}));
    let e = model::plan(&a, &s).unwrap_err();
    assert!(e.message.contains("remove views/front/orientation"));
    assert!(
        e.choices
            .iter()
            .any(|c| c == "remove views/front/orientation")
    );
    let mut duplicate = a.clone();
    let i = duplicate
        .face_attachments
        .iter()
        .position(|f| f.surface.as_deref() == Some("plane"))
        .unwrap();
    let key = draftwright::recognition_corrections::face_fingerprint(&duplicate, i).unwrap();
    duplicate
        .face_attachments
        .push(duplicate.face_attachments[i].clone());
    let e = model::plan(
        &duplicate,
        &spec(json!({"views":{"front":{"orientation":{"from":key}}}})),
    )
    .unwrap_err();
    assert!(
        e.message.contains("ambiguous") && e.message.contains("remove views/front/orientation")
    );
}

#[test]
fn suggestions_restore_an_explicitly_excluded_principal_in_the_set() {
    let (_, a) = fixture("recognition/issues/counterbored_block.step");
    let s = spec(json!({"views":{"front":{"principal_views":["front","side"]}}}));
    let p = model::plan(&a, &s).unwrap();
    let problem = p
        .problems
        .iter()
        .find(|p| p.code == "section_not_shown")
        .unwrap();
    assert!(problem.message.contains("explicit principal set"));
    let repaired = model::edit(&s, &serde_json::to_value(&problem.fix).unwrap()).unwrap();
    assert!(
        repaired.views["front"]
            .principal_views
            .as_ref()
            .unwrap()
            .contains(&"plan".into())
    );
    assert!(
        model::plan(&a, &repaired)
            .unwrap()
            .views
            .iter()
            .any(|v| v.cut.is_some())
    );
}

#[test]
fn pinning_a_dimension_keeps_an_authored_partial_on_its_assigned_sheet() {
    let (_, a) = plate();
    let mut s = fixed();
    s["sheets"] = json!({"max":2});
    s["views"]["detail_b"] =
        json!({"partial":{"from":"plan","around":"f:hole:1","scale_factor":2},"sheet":2});
    s["dimensions"] = json!({"d:envelope/width.length":{"place":{"pinned":true}}});
    let p = model::plan_recovered(&a, &spec(s)).unwrap();
    assert_eq!(view(&p, "detail_b").sheet, 2);
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:envelope/width.length" && d.placed.is_some())
    );
}
