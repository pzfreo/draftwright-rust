//! Authored quantities use exact B-rep evidence and the ordinary editing and ink pipeline.
use draftwright::{
    api,
    items::Op,
    model::{self, Analysis, Spec, dimension_geometry::Reference},
    session::Session,
};
use serde_json::json;
use std::path::Path;

fn plate() -> (draftwright::haecceity::Part, Analysis) {
    model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step"),
    )
    .unwrap()
}
fn parse(v: serde_json::Value) -> Spec {
    serde_json::from_value(v).unwrap()
}
fn planes(a: &Analysis, k: usize) -> Vec<(String, f64)> {
    let mut refs: Vec<_> = a
        .dimension_references()
        .iter()
        .filter_map(|(id, rs)| match rs.as_slice() {
            [Reference::Plane { origin, normal, .. }] if normal[k].abs() > 1.0 - 1e-8 => {
                Some((id.clone(), origin[k]))
            }
            _ => None,
        })
        .collect();
    refs.sort_by(|p, q| p.1.total_cmp(&q.1));
    refs
}
fn width(a: &Analysis) -> serde_json::Value {
    let refs = planes(a, 0);
    json!({"between":[refs.first().unwrap().0,refs.last().unwrap().0],"along":"x","view":"front","tolerance":"±0.1","place":{"side":"below","lane":1}})
}
fn authored(a: &Analysis) -> Spec {
    parse(json!({"dimension_source":"authored","dimensions":{"d:authored:width":width(a)}}))
}

#[test]
fn authored_face_distance_is_measured_toleranced_placed_and_round_trips() {
    let (part, a) = plate();
    let s = authored(&a);
    let p = model::plan_recovered(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:width")
        .unwrap();
    assert_eq!(d.value, a.envelope[1][0] - a.envelope[0][0]);
    assert!(d.label.contains("±0.1"));
    assert!(d.placed.is_some());
    assert!(!d.default);
    assert!(d.from_spec.contains(&"between".into()));
    assert!(d.from_spec.contains(&"along".into()));
    assert!(d.from_spec.contains(&"tolerance".into()));
    assert!(
        d.measurements
            .iter()
            .any(|m| m.feature == "envelope" && m.parameter == "width.length")
    );
    assert!(p.omitted.iter().any(|o| o.id == "d:envelope/width.length"
        && matches!(&o.state,draftwright::items::State::Covered{by} if by=="d:authored:width")));
    let saved: Spec = serde_json::from_slice(&serde_json::to_vec(&p.spec).unwrap()).unwrap();
    let reloaded: Analysis = serde_json::from_slice(&serde_json::to_vec(&a).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(model::plan_recovered(&reloaded, &saved).unwrap()).unwrap()
    );
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    let drawing = model::render(&part, &a, &p, &font).unwrap();
    let mut marks = draftwright::lint::marks(&drawing);
    assert!(
        !draftwright::lint::check(&a, &marks)
            .iter()
            .any(|i| i.code == "authored_dimension_invalid")
    );
    marks
        .iter_mut()
        .find(|m| m.name == "d:authored:width")
        .unwrap()
        .text = "999 ±0.1".into();
    assert!(
        draftwright::lint::check(&a, &marks)
            .iter()
            .any(|i| i.code == "authored_dimension_invalid")
    );
}

#[test]
fn absent_override_keeps_the_default_and_legacy_analysis_works() {
    let (_, a) = plate();
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let mut legacy = a.clone();
    legacy.edge_ink.as_mut().unwrap().references.clear();
    let mut old = model::plan_recovered(&legacy, &Spec::default()).unwrap();
    // Provenance binds the entire Analysis. The new inventory must change its hash,
    // while every actual drawing decision stays exactly the same.
    assert_ne!(p.analysis_sha256, old.analysis_sha256);
    old.analysis_sha256 = p.analysis_sha256.clone();
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    assert!(
        model::plan_recovered(&legacy, &authored(&a))
            .unwrap_err()
            .message
            .contains("analyse the STEP again")
    );
}

#[test]
fn duplicates_are_refused_with_the_existing_owner_and_bad_references_are_actionable() {
    let (_, a) = plate();
    let mut s = authored(&a);
    s.dimension_source = model::DimensionSource::Auto;
    let e = model::plan_recovered(&a, &s).unwrap_err();
    assert!(e.message.contains("duplicates"), "{e:?}");
    assert!(!e.choices.is_empty());
    let mut s = authored(&a);
    s.dimensions
        .get_mut("d:authored:width")
        .unwrap()
        .between
        .as_mut()
        .unwrap()[0] = "face:v1:missing".into();
    let e = model::plan_recovered(&a, &s).unwrap_err();
    assert!(e.message.contains("Unknown reference"));
    assert!(e.message.contains("Analysis.edge_ink.references"));
    let mut s = authored(&a);
    s.dimensions.get_mut("d:authored:width").unwrap().along = Some("y".into());
    assert!(
        model::plan_recovered(&a, &s)
            .unwrap_err()
            .message
            .contains("no unique")
    );
}

#[test]
fn vertex_true_distance_edge_angle_and_face_angle_are_authored() {
    let (_, a) = plate();
    let points: Vec<_> = a
        .dimension_references()
        .iter()
        .filter_map(|(id, rs)| match rs.as_slice() {
            [Reference::Vertex { point }] => Some((id, point)),
            _ => None,
        })
        .collect();
    let (p, q) = points
        .iter()
        .flat_map(|p| points.iter().map(move |q| (p, q)))
        .find(|(p, q)| {
            (p.1[1] - q.1[1]).abs() < 1e-6
                && (p.1[0] - q.1[0]).abs() > 1e-3
                && (p.1[2] - q.1[2]).abs() > 1e-3
        })
        .unwrap();
    let value = ((p.1[0] - q.1[0]).powi(2) + (p.1[2] - q.1[2]).powi(2)).sqrt();
    let s = parse(
        json!({"dimensions":{"d:authored:diagonal":{"between":[p.0,q.0],"along":"true","view":"front"}}}),
    );
    let plan = model::plan_recovered(&a, &s).unwrap();
    let d = plan
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:diagonal")
        .unwrap();
    assert!((d.value - value).abs() < 1e-8);
    assert!(d.placed.is_some());
    let x = planes(&a, 0);
    let z = planes(&a, 2);
    let s = parse(
        json!({"dimensions":{"d:authored:angle":{"kind":"angle","between":[x[0].0,z[0].0],"view":"front"}}}),
    );
    let plan = model::plan_recovered(&a, &s).unwrap();
    let d = plan
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:angle")
        .unwrap();
    assert_eq!(d.kind, "angular");
    assert!((d.value - 90.0).abs() < 1e-8);
    assert!(d.placed.is_some());
    let edges: Vec<_> = a
        .dimension_references()
        .iter()
        .filter_map(|(id, rs)| match rs.as_slice() {
            [Reference::Edge { ends }] => Some((id, ends)),
            _ => None,
        })
        .collect();
    let (e, f) = edges
        .iter()
        .flat_map(|e| edges.iter().map(move |f| (e, f)))
        .find(|(e, f)| {
            e.0 != f.0 && e.1.iter().any(|p| f.1.contains(p)) && {
                let u = [0, 1, 2].map(|k| e.1[1][k] - e.1[0][k]);
                let v = [0, 1, 2].map(|k| f.1[1][k] - f.1[0][k]);
                (0..3).map(|k| u[k] * v[k]).sum::<f64>().abs() < 1e-8
            }
        })
        .unwrap();
    let s = parse(json!({"dimensions":{"d:authored:edges":{"kind":"angle","between":[e.0,f.0]}}}));
    assert!(
        model::plan_recovered(&a, &s)
            .unwrap()
            .dimensions
            .iter()
            .any(|d| d.id == "d:authored:edges" && d.placed.is_some())
    );
}

#[test]
fn session_preview_commit_undo_and_redo_create_and_remove_dimensions() {
    let (part, a) = plate();
    let value = width(&a);
    let mut session = Session::new(part, a, Spec::default()).unwrap();
    let ops = vec![
        Op::set("dimension_source", json!("authored")),
        Op::set("dimensions/d:authored:width", value),
    ];
    let before = serde_json::to_value(session.spec()).unwrap();
    session.apply(&ops, false).unwrap();
    assert_eq!(serde_json::to_value(session.spec()).unwrap(), before);
    session.apply(&ops, true).unwrap();
    assert!(
        session
            .item("d:authored:width")
            .unwrap()
            .anchors
            .iter()
            .any(|s| s.starts_with("face:v1:"))
    );
    let options = session.options("d:authored:width").unwrap();
    assert!(!options.placements.is_empty());
    assert!(
        options
            .settable
            .contains_key("dimensions/d:authored:width/between")
    );
    session.undo().unwrap();
    assert!(session.spec().dimensions.is_empty());
    session.redo().unwrap();
    assert!(session.spec().dimensions.contains_key("d:authored:width"));
    let last = serde_json::to_value(session.spec()).unwrap();
    assert!(
        session
            .apply(
                &[Op::set(
                    "dimensions/d:authored:width/between",
                    json!(["face:bad", "face:worse"])
                )],
                true
            )
            .is_err()
    );
    assert_eq!(serde_json::to_value(session.spec()).unwrap(), last);
}

#[test]
fn edit_rejects_incomplete_or_geometric_intent_and_refuses_ambiguity() {
    for entry in [
        json!({"kind":"radius","between":["a","b"]}),
        json!({"kind":"bogus","between":["a"]}),
        json!({"between":["a","b"],"along":"u"}),
        json!({"between":["a","b"],"value":10}),
    ] {
        assert!(
            model::edit(
                &Spec::default(),
                &json!([{"op":"set","path":"dimensions/d:authored:test","value":entry}])
            )
            .is_err()
        );
    }
    let (_, mut a) = plate();
    let s = authored(&a);
    let key = s.dimensions["d:authored:width"].between.as_ref().unwrap()[0].clone();
    let rs = a
        .edge_ink
        .as_mut()
        .unwrap()
        .references
        .get_mut(&key)
        .unwrap();
    rs.push(rs[0].clone());
    assert!(
        model::plan_recovered(&a, &s)
            .unwrap_err()
            .message
            .contains("ambiguous")
    );
}

#[path = "common/corpus.rs"]
mod corpus;

#[test]
fn cylinders_spheres_and_axis_offsets_are_measured_and_linted() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step");
    let (part, a) = model::analyse_native(&fixture).unwrap();
    let (cylinder, radius) = a
        .dimension_references()
        .iter()
        .find_map(|(id, rs)| match rs.as_slice() {
            [Reference::Cylinder { radius, .. }] => Some((id.clone(), *radius)),
            _ => None,
        })
        .unwrap();
    for (kind, value) in [("diameter", 2.0 * radius), ("radius", radius)] {
        let s = parse(
            json!({"dimension_source":"authored","dimensions":{"d:authored:size":{"kind":kind,"between":[cylinder],"tolerance":"±0.05"}}}),
        );
        let p = model::plan_recovered(&a, &s).unwrap();
        let d = p
            .dimensions
            .iter()
            .find(|d| d.id == "d:authored:size")
            .unwrap();
        assert!((d.value - value).abs() < 1e-8);
        assert!(d.placed.is_some());
        assert!(d.label.contains("±0.05"));
        let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
        let drawing = model::render(&part, &a, &p, &font).unwrap();
        assert!(
            !draftwright::lint::lint(&a, &drawing)
                .iter()
                .any(|i| i.code == "authored_dimension_invalid")
        );
        let mut bad = draftwright::lint::marks(&drawing);
        bad.iter_mut()
            .find(|m| m.name == "d:authored:size")
            .unwrap()
            .text = "R999 ±0.05".into();
        assert!(
            draftwright::lint::check(&a, &bad)
                .iter()
                .any(|i| i.code == "authored_dimension_invalid")
        );
        let mut duplicate = s.clone();
        duplicate.dimension_source = model::DimensionSource::Auto;
        assert!(
            model::plan_recovered(&a, &duplicate)
                .unwrap_err()
                .message
                .contains("duplicates")
        );
    }
    let x = planes(&a, 0);
    let s = parse(
        json!({"dimension_source":"authored","dimensions":{"d:authored:offset":{"between":[x[0].0,format!("axis:{cylinder}")],"along":"x"}}}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:authored:offset" && d.placed.is_some())
    );
    let Some(corpus) = corpus::dir() else { return };
    let (_, sphere) = model::analyse_native(&corpus.join("nist/nist_ctc_04_asme1_rd.stp")).unwrap();
    let (id, radius) = sphere
        .dimension_references()
        .iter()
        .find_map(|(id, rs)| match rs.as_slice() {
            [Reference::Sphere { radius, .. }] => Some((id, *radius)),
            _ => None,
        })
        .expect("ctc04 spherical face");
    for (kind, expected) in [("radius", radius), ("diameter", radius * 2.0)] {
        let s = parse(
            json!({"dimension_source":"authored","dimensions":{"d:authored:sphere":{"kind":kind,"between":[id]}}}),
        );
        let p = model::plan_recovered(&sphere, &s).unwrap();
        let d = p
            .dimensions
            .iter()
            .find(|d| d.id == "d:authored:sphere")
            .unwrap();
        assert_eq!(d.value, expected);
        assert!(d.placed.is_some());
    }
}

#[test]
fn authored_precision_pins_and_duplicate_authored_quantities() {
    let (_, a) = plate();
    let mut s = authored(&a);
    s.precision.insert("d:authored:width".into(), 3);
    s.dimensions
        .get_mut("d:authored:width")
        .unwrap()
        .place
        .as_mut()
        .unwrap()
        .pinned = true;
    let p = model::plan_recovered(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:width")
        .unwrap();
    assert!(
        d.label.starts_with(&format!("{:.3}", d.value)),
        "{}",
        d.label
    );
    assert!(d.placed.is_some());
    s.dimensions.insert(
        "d:authored:second".into(),
        s.dimensions["d:authored:width"].clone(),
    );
    assert!(
        model::plan_recovered(&a, &s)
            .unwrap_err()
            .message
            .contains("duplicates")
    );
}

#[test]
fn authored_linear_fit_is_class_only_and_requires_one_tolerance_owner() {
    let (_, a) = plate();
    let mut s = authored(&a);
    let d = s.dimensions.get_mut("d:authored:width").unwrap();
    d.tolerance = None;
    d.fit = Some("H7".into());
    let p = model::plan_recovered(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:width")
        .unwrap();
    assert!(d.label.ends_with(" H7"), "{}", d.label);
    assert!(d.from_spec.contains(&"fit".into()));
    s.dimensions.get_mut("d:authored:width").unwrap().tolerance = Some("±0.1".into());
    assert!(model::plan_recovered(&a, &s).is_err());
}

#[test]
fn saved_v1_reference_keys_keep_resolving() {
    let (_, a) = plate();
    let saved: Spec =
        serde_json::from_str(include_str!("fixtures/identity/authored.spec2.json")).unwrap();
    let p = model::plan_recovered(&a, &saved).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:width")
        .unwrap();
    assert_eq!(d.value, 60.0);
    assert_eq!(d.label, "60 ±0.1");
    assert!(d.placed.is_some());
}

#[test]
fn obtuse_face_and_edge_angles_keep_the_physical_included_sector() {
    let (_, mut a) = plate();
    let height = 5.0 * 3.0f64.sqrt();
    let records = a.edge_ink.as_mut().unwrap();
    records.references.insert(
        "face:test-horizontal".into(),
        vec![Reference::Plane {
            origin: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            bounds: [[0.0, -1.0, 0.0], [10.0, 1.0, 0.0]],
        }],
    );
    records.references.insert(
        "face:test-inclined".into(),
        vec![Reference::Plane {
            origin: [0.0; 3],
            normal: [3.0f64.sqrt() / 2.0, 0.0, 0.5],
            bounds: [[-5.0, -1.0, 0.0], [0.0, 1.0, height]],
        }],
    );
    records.references.insert(
        "edge:test-horizontal".into(),
        vec![Reference::Edge {
            ends: [[0.0; 3], [10.0, 0.0, 0.0]],
        }],
    );
    records.references.insert(
        "edge:test-inclined".into(),
        vec![Reference::Edge {
            ends: [[-5.0, 0.0, height], [0.0; 3]],
        }],
    );
    for prefix in ["face", "edge"] {
        let refs = vec![
            format!("{prefix}:test-horizontal"),
            format!("{prefix}:test-inclined"),
        ];
        assert!((a.measure_references(&refs, "angle", None).unwrap().value - 120.0).abs() < 1e-9);
        let s = parse(
            json!({"dimensions":{"d:authored:obtuse":{"kind":"angle","between":refs,"view":"front"}}}),
        );
        let p = model::plan_recovered(&a, &s).unwrap();
        let d = p
            .dimensions
            .iter()
            .find(|d| d.id == "d:authored:obtuse")
            .unwrap();
        assert!((d.value - 120.0).abs() < 1e-9);
        assert_eq!(d.label, "120°");
        assert!(d.placed.is_some());
    }
}

#[test]
fn angular_editor_options_offer_radial_placement_without_inapplicable_sides() {
    let (part, a) = plate();
    let x = planes(&a, 0);
    let z = planes(&a, 2);
    let s = parse(
        json!({"dimensions":{"d:authored:angle":{"kind":"angle","between":[x[0].0,z.last().unwrap().0],"view":"front"}}}),
    );
    let session = Session::new(part, a, s).unwrap();
    let options = session.options("d:authored:angle").unwrap();
    assert!(!options.placements.is_empty());
    assert!(
        options
            .placements
            .iter()
            .all(|p| p.side.is_none() && p.view.as_deref() == Some("front"))
    );
    assert_eq!(
        options.settable["dimensions/d:authored:angle/place/side"]["choices"],
        json!([])
    );
    assert_eq!(
        options.settable["dimensions/d:authored:angle/place/view"]["choices"],
        json!(["front"])
    );
}
