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
fn grm03_has_an_overall_and_one_reference_link_without_plate_thickness() {
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
    assert_eq!(links.len(), 4);
    assert_eq!(
        links
            .iter()
            .filter(|d| d.label.starts_with('(') && d.label.ends_with(')'))
            .count(),
        1
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
}
#[test]
fn lint_detects_duplicate_claims_and_closed_turned_chains() {
    let a = native("gramel/GRM-03_thumbwheel_drive_screw.step");
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
fn radius_leader_and_label_clear_thickness_extensions_including_pad() {
    let a = native("mfcadpp/10033.step");
    let p = model::plan(&a, &Spec::default()).unwrap();
    let thickness = p
        .dimensions
        .iter()
        .find(|d| d.id.ends_with("/thickness.length"))
        .unwrap()
        .placed
        .as_ref()
        .unwrap();
    let radius = p
        .dimensions
        .iter()
        .find(|d| d.label == "R6.1")
        .unwrap()
        .placed
        .as_ref()
        .unwrap();
    let b = radius.label_box;
    let pad = [b[0] - 1., b[1] - 1., b[2] + 1., b[3] + 1.];
    for e in thickness.lines.iter().take(2) {
        assert!(!crosses(*e, pad), "extension within label pad: {radius:?}");
        // Axis-aligned witnesses: a horizontal shoulder must not cross or touch one.
        for l in &radius.lines {
            let vertical = (e[0][0] - e[1][0]).abs() < 1e-8;
            let horizontal = (l[0][1] - l[1][1]).abs() < 1e-8;
            if vertical && horizontal {
                assert!(
                    !(e[0][0] >= l[0][0].min(l[1][0]) - 1e-6
                        && e[0][0] <= l[0][0].max(l[1][0]) + 1e-6
                        && l[0][1] >= e[0][1].min(e[1][1]) - 1e-6
                        && l[0][1] <= e[0][1].max(e[1][1]) + 1e-6)
                );
            }
        }
    }
}

fn crosses(line: [[f64; 2]; 2], b: [f64; 4]) -> bool {
    let (mut lo, mut hi) = (0_f64, 1_f64);
    for k in 0..2 {
        let d = line[1][k] - line[0][k];
        if d.abs() < 1e-9 {
            if line[0][k] < b[k] || line[0][k] > b[k + 2] {
                return false;
            }
        } else {
            let (a, c) = ((b[k] - line[0][k]) / d, (b[k + 2] - line[0][k]) / d);
            lo = lo.max(a.min(c));
            hi = hi.min(a.max(c));
        }
    }
    lo <= hi
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
