#[path = "common/corpus.rs"]
mod corpus;
use draftwright::{
    api,
    items::State,
    lint,
    model::{self, Analysis, Plan, Spec},
};
use serde_json::json;
use std::sync::OnceLock;
fn ctc() -> &'static (draftwright::haecceity::Part, Analysis) {
    static PART: OnceLock<(draftwright::haecceity::Part, Analysis)> = OnceLock::new();
    PART.get_or_init(|| {
        model::analyse_native(
            &corpus::dir()
                .expect("CTC corpus")
                .join("nist/nist_ctc_01_asme1_rd.stp"),
        )
        .unwrap()
    })
}
fn parse(v: serde_json::Value) -> Spec {
    serde_json::from_value(v).unwrap()
}
fn plan(s: &Spec) -> Plan {
    model::plan_recovered(&ctc().1, s).unwrap()
}
#[test]
fn ctc01_consumes_bevel_faces_and_sizes_both_notches_without_duplicate_c50() {
    let a = &ctc().1;
    assert_eq!(
        a.features
            .iter()
            .filter(|f| f.kind == "oriented_chamfer")
            .count(),
        4
    );
    assert!(
        !a.unrecognised
            .iter()
            .any(|r| r.family == "oriented_chamfers")
    );
    let p = plan(&Spec::default());
    assert!(p.completeness.claimed_faces > 77, "{:?}", p.completeness);
    let angles: Vec<_> = p
        .dimensions
        .iter()
        .filter(|d| d.measurements.iter().any(|m| m.parameter == "notch.angle"))
        .collect();
    assert_eq!(angles.len(), 2);
    for d in angles {
        assert_eq!(d.view, "v:plan");
        assert!((d.value - 60.).abs() < 1e-6);
        assert!(d.placed.is_some());
    }
    let depths: Vec<_> = p
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter == "notch_depth.length")
        })
        .collect();
    // Coincident equal depth spans may share a dimension, retaining both measurements.
    assert_eq!(
        depths
            .iter()
            .flat_map(|d| &d.measurements)
            .filter(|m| m.parameter == "notch_depth.length")
            .count(),
        2
    );
    assert!(
        depths
            .iter()
            .all(|d| (d.value - 50.).abs() < 1e-6 && d.placed.is_some())
    );
    assert_eq!(p.dimensions.iter().filter(|d| d.label == "C50").count(), 1);
    assert_eq!(
        p.dimensions.iter().filter(|d| d.label == "6× C5").count(),
        1
    );
    let mut seen = std::collections::BTreeSet::new();
    for d in &p.dimensions {
        for m in &d.measurements {
            assert!(seen.insert((&m.feature, &m.parameter)), "duplicate {m:?}");
        }
    }
    let mut without = a.clone();
    without.features.retain(|f| f.kind != "oriented_chamfer");
    assert!(
        p.completeness.claimed_faces
            >= model::plan(&without, &Spec::default())
                .unwrap()
                .completeness
                .claimed_faces
                + 4
    );
}
#[test]
fn notch_dimensions_accept_omit_tolerance_place_and_label_text() {
    let p = plan(&Spec::default());
    for parameter in [
        "notch.angle",
        "notch_depth.length",
        "location.length.x",
        "oriented_chamfer.length",
    ] {
        let original = p
            .dimensions
            .iter()
            .find(|d| {
                d.measurements.iter().any(|m| {
                    m.parameter == parameter
                        && (parameter != "location.length.x" || m.feature.starts_with("notch:"))
                })
            })
            .unwrap();
        let id = &original.id;
        let omitted = plan(&parse(json!({"dimensions":{id:{"omit":true}}})));
        assert!(!omitted.dimensions.iter().any(|d| &d.id == id));
        assert!(omitted.omitted.iter().any(|d| &d.id == id));
        let tolerance = plan(&parse(json!({"dimensions":{id:{"tolerance":"±0.5"}}})));
        assert!(
            tolerance
                .dimensions
                .iter()
                .find(|d| &d.id == id)
                .unwrap()
                .label
                .contains("±0.5")
        );
        let placed = plan(&parse(
            json!({"dimensions":{id:{"place":{"view":original.view.trim_start_matches("v:"),"lane":3,"pinned":true}}}}),
        ));
        let changed = placed.dimensions.iter().find(|d| &d.id == id).unwrap();
        assert!(changed.placed.is_some());
        assert_ne!(
            changed.placed,
            p.dimensions.iter().find(|d| &d.id == id).unwrap().placed
        );
        let labelled = plan(&parse(
            json!({"dimensions":{id:{"text":{"suffix":"INCLUDED"}}}}),
        ));
        assert_eq!(
            labelled
                .dimensions
                .iter()
                .find(|d| &d.id == id)
                .unwrap()
                .label,
            format!("{} INCLUDED", original.label)
        );
    }
}
#[test]
fn lint_remeasures_new_dimensions_and_rejects_wrong_nominals_and_tolerance_digits() {
    let (part, a) = ctc();
    let p = plan(&Spec::default());
    let drawing = model::render(
        part,
        a,
        &p,
        &draftwright::text::Font::new(api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    let mut marks = lint::marks(&drawing);
    assert!(
        !lint::check(a, &marks)
            .iter()
            .any(|i| i.code.starts_with("notch_requirement_")),
        "{:?}",
        lint::check(a, &marks)
    );
    for parameter in [
        "notch.angle",
        "notch_depth.length",
        "oriented_chamfer.length",
    ] {
        let i = marks
            .iter()
            .position(|m| m.claims.iter().any(|c| c.parameter == parameter))
            .unwrap();
        let old = marks[i].text.clone();
        marks[i].text = if parameter == "notch.angle" {
            "99° ±60"
        } else {
            "999 ±50"
        }
        .into();
        let issues = lint::check(a, &marks);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code.ends_with("requirement_unverifiable")
                    && issue.annotation.as_deref() == Some(&marks[i].name)),
            "{parameter}: {issues:?}"
        );
        marks[i].text = old;
    }
}
#[test]
fn datum_scheme_takes_the_notch_station_once() {
    let p = plan(&Spec::default());
    let id = p
        .dimensions
        .iter()
        .flat_map(|d| &d.measurements)
        .find(|m| m.parameter == "notch.angle")
        .unwrap()
        .feature
        .clone();
    let s = parse(
        json!({"dimension_schemes":{"notch":{"scheme":"baseline","axis":"x","view":"plan","targets":[id],"from":"max"}}}),
    );
    let q = plan(&s);
    assert!(
        q.dimensions
            .iter()
            .any(|d| d.id.starts_with("d:scheme:notch/"))
    );
    assert!(q.omitted.iter().any(|d| d.id.contains(&id)
        && d.id.ends_with("location.length.x")
        && matches!(d.state, State::Covered { .. })));
    let drawing = model::render(
        &ctc().0,
        &ctc().1,
        &q,
        &draftwright::text::Font::new(api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    let mut marks = lint::marks(&drawing);
    assert!(
        !lint::check(&ctc().1, &marks)
            .iter()
            .any(|i| i.code.starts_with("notch_requirement_"))
    );
    let link = marks
        .iter_mut()
        .find(|m| m.name.starts_with("d:scheme:notch/"))
        .unwrap();
    link.text = "999 ±290".into();
    let issues = lint::check(&ctc().1, &marks);
    assert!(issues.iter().any(|i| i.code == "dimension_scheme_invalid"));
    assert!(issues.iter().any(|i| i.code == "notch_requirement_missing"));
}

#[test]
fn authored_notch_angle_transfers_ownership_and_auto_mode_names_the_existing_owner() {
    use draftwright::model::dimension_geometry::Reference;
    let a = &ctc().1;
    let planes:Vec<_>=a.dimension_references().iter().filter(|(_,rs)| matches!(rs.as_slice(),[Reference::Plane{normal,bounds,..}] if normal[2].abs()<1e-8 && bounds[0][0]< -80. && bounds[1][0]>-140. && bounds[1][1]< -70.)).map(|(k,_)|k.clone()).collect();
    let refs = planes
        .iter()
        .find_map(|x| {
            planes.iter().find_map(|y| {
                let refs = vec![x.clone(), y.clone()];
                let m = a.measure_references(&refs, "angle", None).ok()?;
                let angle = m.angle?;
                ((m.value - 60.).abs() < 1e-5
                    && (angle.vertex[0] + 110.).abs() < 1e-5
                    && (angle.vertex[1] + 75.).abs() < 1e-5)
                    .then_some(refs)
            })
        })
        .expect("the two notch support planes");
    let s = parse(json!({"dimensions":{"d:authored:notch":{"kind":"angle","between":refs}}}));
    let error = model::plan(a, &s).unwrap_err();
    assert!(error.message.contains("duplicates"));
    assert!(error.choices.iter().any(|id| id.ends_with("/notch.angle")));
    let mut s = s;
    s.dimension_source = model::DimensionSource::Authored;
    let q = plan(&s);
    let owner = q
        .dimensions
        .iter()
        .find(|d| d.id == "d:authored:notch")
        .unwrap();
    assert!(
        owner
            .measurements
            .iter()
            .any(|m| m.parameter == "notch.angle")
    );
    assert!(q.omitted.iter().any(|d| d.id.ends_with("/notch.angle")
        && matches!(&d.state,State::Covered{by} if by=="d:authored:notch")));
}

#[test]
fn native_oriented_records_are_checked_and_non45_callouts_state_both_values() {
    let (part, a) = ctc();
    let r = quiddity::correspondence::recognise(part);
    let mut doc = quiddity::recognition::document(&r, part.faces.len());
    let i = doc
        .records
        .iter()
        .position(|r| r.family == "oriented_chamfers")
        .unwrap();
    for field in [
        "corner",
        "run",
        "leg1_direction",
        "leg2_direction",
        "support_spans",
        "length",
        "angle",
    ] {
        let mut bad = doc.clone();
        bad.records[i].parameters.remove(field);
        let mut unchanged = a.clone();
        let before = serde_json::to_value(&unchanged).unwrap();
        assert!(
            model::recognise_document(part, &mut unchanged, &bad)
                .unwrap_err()
                .contains(field)
        );
        assert_eq!(serde_json::to_value(unchanged).unwrap(), before);
    }
    doc.records[i].parameters.insert("leg1".into(), json!(10.));
    doc.records[i]
        .parameters
        .insert("angle".into(), json!(26.57));
    let mut a = a.clone();
    model::recognise_document(part, &mut a, &doc).unwrap();
    let q = model::plan_recovered(&a, &Spec::default()).unwrap();
    let d = q
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter == "oriented_chamfer.angle")
        })
        .unwrap();
    assert_eq!(d.label, "10 × 26.6°");
    assert!(d.placed.is_some());
    let drawing = model::render(
        part,
        &a,
        &q,
        &draftwright::text::Font::new(api::MONO.to_vec()).unwrap(),
    )
    .unwrap();
    let mut marks = lint::marks(&drawing);
    let m = marks.iter_mut().find(|m| m.name == d.id).unwrap();
    m.text = "999 × 99° ±10 ±26.57".into();
    assert!(
        lint::check(&a, &marks)
            .iter()
            .any(|i| i.code == "chamfer_requirement_unverifiable"
                && i.annotation.as_deref() == Some(&d.id))
    );
}

#[test]
fn bevel_run_midpoints_do_not_invent_locations_or_force_sheet_enlargement() {
    for (file, count, label) in [
        ("mfcadpp/1007.step", 1, "C0.1"),
        ("nist/nist_ctc_04_asme1_rd.stp", 4, "12× C5"),
    ] {
        let (_, a) = model::analyse_native(&corpus::dir().unwrap().join(file)).unwrap();
        assert_eq!(
            a.features
                .iter()
                .filter(|f| f.kind == "oriented_chamfer")
                .count(),
            count
        );
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(p.dimensions.iter().filter(|d| d.label == label).count(), 1);
        assert!(
            !p.dimensions
                .iter()
                .flat_map(|d| &d.measurements)
                .any(|m| m.feature.starts_with("f:oriented_chamfer:")
                    && m.parameter.starts_with("location."))
        );
        if count == 1 {
            assert_eq!(p.sheet.size, "A4");
            assert_eq!(p.sheet.scale, "1:1");
        }
    }
}

#[test]
fn bounded_notches_reserve_acute_arcs_and_leave_dense_general_profiles_alone() {
    for file in [
        "mfcadpp/10020.step",
        "mfcadpp/10046.step",
        "nist/nist_ftc_08_asme1_rc.stp",
    ] {
        let (_, a) = model::analyse_native(&corpus::dir().unwrap().join(file)).unwrap();
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        if file.contains("10020") {
            assert_eq!(
                p.dimensions
                    .iter()
                    .filter(|d| d.id.ends_with("/notch.angle"))
                    .count(),
                1
            );
            assert!(
                !p.problems
                    .iter()
                    .any(|p| p.code == "annotation_outside_area")
            );
            assert_eq!(p.sheet.size, "A4");
        } else {
            assert!(!p.dimensions.iter().any(|d| d.id.starts_with("d:notch:")));
        }
    }
}
