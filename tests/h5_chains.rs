#[path = "common/reference_geometry.rs"]
mod reference_geometry;

#[path = "common/corpus.rs"]
mod corpus;

use draftwright::lint::{self, Mark};
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::output::Claim;
use serde_json::json;

fn native(file: &str) -> Analysis {
    model::analyse_native(&corpus::dir().unwrap().join(file))
        .unwrap()
        .1
}
// Independent authored profiles retain chain semantics; a connected whole profile now
// defaults to a baseline. Keep the original reference/omission/pin invariants on this case.
fn chain_profile(file: &str) -> Analysis {
    let mut a = native(file);
    a.features.retain(|f| f.kind != "plate");
    for f in a.features.iter_mut().filter(|f| f.kind == "turned_step") {
        f.parameters["profile"]["body_key"] = json!([f.id]);
    }
    a
}
fn marks(p: &Plan) -> Vec<Mark> {
    p.dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .map(|d| Mark {
            name: d.id.clone(),
            view: Some(d.view.clone()),
            text: d.label.clone(),
            claims: d
                .measurements
                .iter()
                .map(|m| Claim {
                    feature: m.feature.clone(),
                    parameter: m.parameter.clone(),
                })
                .collect(),
        })
        .collect()
}
#[test]
fn grm03_has_an_overall_and_baseline_without_plate_thickness() {
    let a = native("gramel/GRM-03_thumbwheel_drive_screw.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:envelope/width.length" && d.label == "23.5")
    );
    let links: Vec<_> = p
        .dimensions
        .iter()
        .filter(|d| d.id.ends_with("/step.length"))
        .collect();
    assert_eq!(links.len(), 2);
    assert_eq!(
        p.dimensions
            .iter()
            .filter(|d| d.id.ends_with("/step_position.length"))
            .count(),
        2
    );
    assert_eq!(
        links
            .iter()
            .filter(|d| d.label.starts_with('(') && d.label.ends_with(')'))
            .count(),
        0
    );
    assert!(!p.dimensions.iter().any(|d| d.id.contains("f:plate:")));
    assert!(
        !lint::check(&a, &marks(&p))
            .iter()
            .any(|i| i.code == "closed_dimension_chain" || i.code == "plate_requirement_missing")
    );
}
#[test]
fn ctc05_states_its_od_and_across_flats_once() {
    let a = native("nist/nist_ctc_05_asme1_rd.stp");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        !p.dimensions
            .iter()
            .any(|d| d.id == "d:envelope/depth.length" || d.id == "d:envelope/width.length")
    );
    assert!(p.dimensions.iter().any(|d| d.label == "ø558.8"));
    assert!(p.dimensions.iter().any(|d| d.label == "527.05 A/F"));
}
#[test]
fn slot_chain_keeps_overall_and_functional_length_without_closure() {
    let a = native("mfcadpp/10047.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:envelope/height.length")
    );
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:f:slot:1/slot_length.length" && d.label == "19.1")
    );
    assert!(
        !p.dimensions
            .iter()
            .any(|d| d.id == "d:f:slot:1/location_slot.length" && !d.label.starts_with('('))
    );
    let floor = p
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.feature == "step_level" && m.parameter == "step_height.length.h5_133")
        })
        .unwrap();
    assert_eq!(floor.id, "d:f:slot:1/location_slot.length");
    assert_eq!(floor.label, "(5.1)");
    assert!(p.omitted.iter().any(|d| d.id == "d:step_level/step_height.length.h5_133"
        && matches!(&d.state, draftwright::items::State::Covered { by } if by == &floor.id)));
    let spec = model::edit(&Spec::default(), &json!([{"op":"set", "path":"dimensions/d:f:slot:1~1location_slot.length/omit", "value":true}])).unwrap();
    let p = model::plan(&a, &spec).unwrap();
    let floor = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:step_level/step_height.length.h5_133")
        .unwrap();
    assert_eq!(
        floor.label, "(5.1)",
        "restoring the floor must not close the chain again"
    );
}
#[test]
fn concentric_bolt_circles_have_axis_location_credit() {
    let a = native("cadgenbench/flanged_spool_132.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    let issues = lint::check(&a, &marks(&p));
    assert!(
        !issues
            .iter()
            .any(|i| i.code == "hole_pattern_requirement_missing"),
        "{issues:?}"
    );
}
#[test]
fn fully_dimensioned_box_is_clear() {
    let a: Analysis = serde_json::from_value(json!({"schema":model::ANALYSIS_SCHEMA,
        "binding":{"source_sha256":"","part":"box","face_count":6,"loader":"test"},
        "envelope":[[0,0,0],[40,30,10]],"solid_count":1,"iso_bounds":[-20,-20,20,20],
        "principal_profiles":[{"bounds":[[0,0,0],[40,30,10]],"faces":(0..3).flat_map(|axis| [0,1].map(move |end| json!({"axis":axis,"at":if end==0 {0} else {[40,30,10][axis]},"inner":[],"outer_edges":4,"outer_arcs":[],"outer_lines":[]}))).collect::<Vec<_>>()}]})).unwrap();
    let spec: Spec =
        serde_json::from_value(json!({"sheet":{"title":{"tolerance":"ISO 2768-m"}}})).unwrap();
    let p = model::plan(&a, &spec).unwrap();
    assert_eq!(serde_json::to_value(&p).unwrap()["status"], "clear");
    assert!(!p.problems.iter().any(|p| p.code == "unclaimed_geometry"));
    let less = model::edit(
        &spec,
        &json!([{"op":"set", "path":"dimensions/d:envelope~1width.length/omit", "value":true}]),
    )
    .unwrap();
    let p = model::plan(&a, &less).unwrap();
    assert_eq!(
        serde_json::to_value(&p).unwrap()["status"],
        "needs_attention"
    );
    assert_eq!(p.completeness.claimed_faces, 0);
    assert!(p.problems.iter().any(|p| p.code == "unclaimed_geometry"));
}
#[test]
fn lint_detects_duplicate_claims_and_closed_turned_chains() {
    let a = chain_profile("gramel/GRM-03_thumbwheel_drive_screw.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    let mut ink = marks(&p);
    for m in &mut ink {
        m.text = m.text.trim_matches(['(', ')']).to_string();
    }
    ink.push(Mark {
        name: "overall".into(),
        view: Some("v:front".into()),
        text: "23.5".into(),
        claims: vec![Claim {
            feature: "envelope".into(),
            parameter: "width.length".into(),
        }],
    });
    let mut duplicate = ink
        .iter()
        .find(|m| m.claims.iter().any(|c| c.parameter == "step.length"))
        .unwrap()
        .clone();
    duplicate.name = "duplicate".into();
    ink.push(duplicate);
    let codes: Vec<_> = lint::check(&a, &ink)
        .iter()
        .map(|i| i.code.clone())
        .collect();
    assert!(codes.contains(&"duplicate_measurement".into()));
    assert!(codes.contains(&"closed_dimension_chain".into()));
}
#[test]
fn wall_witness_avoids_the_hole_centreline() {
    let a = native("nist/nist_ftc_07_asme1_rd.stp");
    let p = model::plan(&a, &Spec::default()).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:f:plate:1/thickness.length")
        .unwrap();
    assert_eq!(d.label, "2× 9.3");
    assert_eq!(d.view, "v:plan");
}
#[test]
fn consolidated_flush_boss_height_is_not_reported_missing() {
    let a: Analysis=serde_json::from_value(json!({"schema":model::ANALYSIS_SCHEMA,
        "binding":{"source_sha256":"","part":"flush_hub","face_count":8,"loader":"test"},
        "envelope":[[0,0,0],[4.5,10,24]],"iso_bounds":[-20,-20,20,20],"solid_count":1,
        "features":[{"id":"f:boss:1","kind":"boss","faces":[6,7],
        "parameters":{"axis":[1,0,0],"location":[4.5,5,8],"diameter":8,"height":4.5},
        "fingerprint":{"version":"test","family":"boss","subtype":"BossRecord","sizes":{},"position":[2.25,5,8],"axis":[1,0,0],"neighbourhood":[]}}]})).unwrap();
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        p.omitted
            .iter()
            .any(|d| d.id == "d:f:boss:1/boss_height.length")
    );
    assert!(lint::bosses::coverage(&a, &marks(&p)).is_empty());
}

#[test]
fn cross_view_locations_keep_single_claim_owners_and_controlling_labels() {
    for file in [
        "nist/nist_ctc_02_asme1_rc.stp",
        "nist/nist_ftc_10_asme1_rb.stp",
        "cadgenbench_inputs/cgb245.step",
    ] {
        let a = native(file);
        let p = model::plan(&a, &Spec::default()).unwrap();
        let mut owners = std::collections::BTreeMap::new();
        for d in p.dimensions.iter().filter(|d| d.placed.is_some()) {
            for m in &d.measurements {
                assert!(
                    owners.insert((&m.feature, &m.parameter), &d.id).is_none(),
                    "{file}: duplicate claim {m:?} by {}",
                    d.id
                );
                if m.parameter.starts_with("location.length.") {
                    assert!(!d.label.starts_with('('), "{file}: {}", d.id);
                }
            }
        }
    }
}

#[test]
fn references_become_controlling_without_moving_other_ink() {
    for (file, total) in [
        ("mfcadpp/10047.step", "d:envelope/height.length"),
        (
            "gramel/GRM-03_thumbwheel_drive_screw.step",
            "d:envelope/width.length",
        ),
        (
            "gramel/GRM-05_depth_lock_bolt.step",
            "d:envelope/height.length",
        ),
        ("gramel/string_post.step", "d:envelope/height.length"),
    ] {
        let a = chain_profile(file);
        let full = model::plan(&a, &Spec::default()).unwrap();
        let reference = full
            .dimensions
            .iter()
            .find(|d| d.label.starts_with('('))
            .unwrap();
        let less = model::edit(&Spec::default(), &json!([{"op":"set", "path":format!("dimensions/{}/omit", draftwright::items::key(total)), "value":true}])).unwrap();
        let p = model::plan(&a, &less).unwrap();
        let controlling = p.dimensions.iter().find(|d| d.id == reference.id).unwrap();
        assert_eq!(
            reference.label,
            format!("({})", controlling.label),
            "{file}: {:?}",
            p.dimensions
                .iter()
                .map(|d| (&d.id, &d.label, &d.basis))
                .collect::<Vec<_>>()
        );
        for now in &p.dimensions {
            let was = full.dimensions.iter().find(|d| d.id == now.id).unwrap();
            reference_geometry::assert_local(was, now);
        }
    }
}

#[test]
fn a_pinned_reference_keeps_its_ink_and_reports_a_broken_controlling_chain() {
    let a = chain_profile("gramel/GRM-03_thumbwheel_drive_screw.step");
    // A pin holds the ink of the drawing it was taken from: the recovered one (H6 moves GRM-03
    // to A3 10:1), as the pinned plan below is recovered too.
    let full = model::plan_recovered(&a, &Spec::default()).unwrap();
    let reference = full.dimensions.iter().find(|d| d.label == "(0.5)").unwrap();
    let spec = model::edit(&Spec::default(), &json!([
        {"op":"set", "path":format!("dimensions/{}/place", draftwright::items::key(&reference.id)), "value":{"pinned":true}},
        {"op":"set", "path":"dimensions/d:envelope~1width.length/omit", "value":true}
    ])).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    let pinned = p.dimensions.iter().find(|d| d.id == reference.id).unwrap();
    assert_eq!(pinned.label, reference.label);
    assert_eq!(pinned.placed, reference.placed);
    let finding = p
        .problems
        .iter()
        .find(|p| p.code == "pinned_reference_chain_changed")
        .unwrap_or_else(|| {
            panic!(
                "{:?}",
                p.dimensions
                    .iter()
                    .map(|d| (&d.id, &d.label, &d.basis))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(finding.severity, "warning");
    assert_eq!(
        finding.subjects.as_slice(),
        std::slice::from_ref(&reference.id)
    );
    let released = model::edit(&spec, &serde_json::to_value(&finding.fix).unwrap()).unwrap();
    let p = model::plan_recovered(&a, &released).unwrap();
    assert!(
        !p.problems
            .iter()
            .any(|p| p.code == "pinned_reference_chain_changed")
    );
    assert_eq!(
        p.dimensions
            .iter()
            .find(|d| d.id == reference.id)
            .unwrap()
            .label,
        "0.5"
    );
}

fn label(p: &Plan, id: &str) -> String {
    p.dimensions
        .iter()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("{id} not planned"))
        .label
        .clone()
}

#[test]
fn shoulders_on_slot_walls_are_references_when_the_slot_is_stated() {
    // 10047: 18.6 + 4.5 = 23.1, and the slot's centre 20.9 fixes both walls.
    let a = native("mfcadpp/10047.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(label(&p, "d:f:slot:1/slot_width.length"), "4.5");
    assert_eq!(label(&p, "d:f:slot:1/location_slot_across.length"), "20.9");
    assert_eq!(
        label(&p, "d:step_level/step_position.length.y18_61"),
        "(18.6)"
    );
    assert_eq!(
        label(&p, "d:step_level/step_position.length.y23_144"),
        "(23.1)"
    );
    let issues = lint::check(&a, &marks(&p));
    assert!(
        !issues
            .iter()
            .any(|i| i.code == "closed_dimension_chain" || i.code == "duplicate_measurement"),
        "{issues:?}"
    );
    // Without the centre line only the farther shoulder closes the chain.
    let spec = model::edit(&Spec::default(), &json!([{"op":"set", "path":"dimensions/d:f:slot:1~1location_slot_across.length/omit", "value":true}])).unwrap();
    let p = model::plan(&a, &spec).unwrap();
    assert_eq!(
        label(&p, "d:step_level/step_position.length.y18_61"),
        "18.6"
    );
    assert_eq!(
        label(&p, "d:step_level/step_position.length.y23_144"),
        "(23.1)"
    );
    // The same rule on z ladders (FTC-09's slots, FTC-10's 12.5 + 50 = 62.5).
    let p = model::plan(&native("nist/nist_ftc_09_asme1_rd.stp"), &Spec::default()).unwrap();
    assert_eq!(
        label(&p, "d:step_level/step_height.length.h41_275"),
        "(41.3)"
    );
    let p = model::plan(&native("nist/nist_ftc_10_asme1_rb.stp"), &Spec::default()).unwrap();
    assert_eq!(label(&p, "d:step_level/step_height.length.h62_5"), "(62.5)");
}

#[test]
fn lint_reports_the_10047_slot_wall_closure_when_every_member_is_stated() {
    let a = native("mfcadpp/10047.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    let mut ink = marks(&p);
    for m in &mut ink {
        m.text = m.text.trim_matches(['(', ')']).to_string();
    }
    let closed: Vec<_> = lint::check(&a, &ink)
        .into_iter()
        .filter(|i| i.code == "closed_dimension_chain")
        .filter_map(|i| i.annotation)
        .collect();
    assert!(
        closed
            .iter()
            .any(|n| n.starts_with("d:step_level/step_position.length.y")),
        "{closed:?}"
    );
}

#[test]
fn a_plate_between_a_shoulder_and_the_box_is_not_stated_again() {
    // 10060: 33.6 - 19.3 = 14.3, the plate's thickness.
    let a = native("mfcadpp/10060.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        !p.dimensions
            .iter()
            .any(|d| d.id == "d:f:plate:1/thickness.length")
    );
    assert!(
        p.omitted
            .iter()
            .any(|d| d.id == "d:f:plate:1/thickness.length")
    );
    let issues = lint::check(&a, &marks(&p));
    assert!(
        !issues
            .iter()
            .any(|i| i.code == "closed_dimension_chain" || i.code == "plate_requirement_missing"),
        "{issues:?}"
    );
}

#[test]
fn lint_reports_an_overall_that_repeats_an_outside_diameter_or_across_flats() {
    // CTC-05 before this stream: linear 558.8 beside ø558.8, 527.0 beside 527.05 A/F.
    let a = native("nist/nist_ctc_05_asme1_rd.stp");
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        !lint::check(&a, &marks(&p))
            .iter()
            .any(|i| i.code == "duplicate_measurement" || i.code == "closed_dimension_chain")
    );
    let mut ink = marks(&p);
    for (name, parameter, text) in [
        ("overall x", "width.length", "527.0"),
        ("overall y", "depth.length", "558.8"),
    ] {
        ink.push(Mark {
            name: name.into(),
            view: Some("v:front".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: "envelope".into(),
                parameter: parameter.into(),
            }],
        });
    }
    let duplicates: Vec<_> = lint::check(&a, &ink)
        .into_iter()
        .filter(|i| i.code == "duplicate_measurement")
        .filter_map(|i| i.annotation)
        .collect();
    assert!(
        duplicates.contains(&"overall x".to_string()),
        "{duplicates:?}"
    );
    assert!(
        duplicates.contains(&"overall y".to_string()),
        "{duplicates:?}"
    );
}
