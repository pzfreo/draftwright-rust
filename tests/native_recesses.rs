//! Native default-path regressions for the recess-reader rules.
use draftwright::{
    lint::{
        self,
        section_recesses::{Grammar, grammar},
    },
    model::{self, Analysis, Spec},
    text::Font,
};
use serde_json::json;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn read(name: &str) -> (draftwright::haecceity::Part, Analysis) {
    let path = fixture(name);
    if path.extension().is_some_and(|e| e == "gz") {
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
            .read_to_end(&mut bytes)
            .unwrap();
        model::analyse_native_step(&path, &bytes).unwrap()
    } else {
        model::analyse_native(&path).unwrap()
    }
}
fn font() -> Font {
    Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
        .unwrap()
}

/// ISSUE-885-a: size, depth and both in-plane positions reach the native plan on all axes.
#[test]
fn native_side_pockets_state_size_depth_and_both_locations() {
    for (name, axis) in [("pocket_x", 0), ("pocket_y", 1), ("pocket_z", 2)] {
        let (part, a) = read(&format!("recognition/issues/{name}.step"));
        assert!(
            !a.unrecognised
                .iter()
                .any(|u| u.family == "section_recesses")
        );
        let f = a
            .features
            .iter()
            .find(|f| matches!(grammar(f), Ok(Grammar::Pocket(_))))
            .unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let own: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| d.measurements.iter().any(|m| m.feature == f.id))
            .collect();
        let callout = own.iter().find(|d| d.kind == "callout").unwrap();
        assert!(callout.label.contains("DEEP"));
        assert!(callout.placed.is_some());
        let locations: Vec<_> = own
            .iter()
            .flat_map(|d| &d.measurements)
            .filter(|m| m.parameter.starts_with("location_pocket."))
            .collect();
        assert_eq!(locations.len(), 2, "{name}: {own:#?}");
        let expected = ["side", "front", "plan"][axis];
        assert!(
            own.iter().all(|d| d.view == format!("v:{expected}")),
            "{name}"
        );
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        assert!(
            lint::lint(&a, &drawing)
                .iter()
                .all(|i| !i.code.starts_with("pocket_requirement_")),
            "{name}"
        );
    }
}

/// LINT-pocket_pattern_coverage-requirements, ISSUE-1485-a/c: retain one owner for
/// count, sizes, depth, pitches and absolute location, with no per-member size callouts.
#[test]
fn native_patterns_have_one_grouped_callout_pitches_and_absolute_location() {
    for (name, count, pitches) in [
        ("array.step.gz", 3, 1),
        ("grid.step.gz", 6, 2),
        ("curved-grid.step.gz", 6, 2),
        ("tuner.step", 5, 1),
    ] {
        let (part, a) = read(&format!("recesses/{name}"));
        let f = a
            .features
            .iter()
            .find(|f| f.kind == "pocket_pattern")
            .unwrap_or_else(|| panic!("{name}: no pattern"));
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let dims: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| d.measurements.iter().any(|m| m.feature == f.id))
            .collect();
        let callouts: Vec<_> = dims.iter().filter(|d| d.kind == "callout").collect();
        assert_eq!(callouts.len(), 1, "{name}");
        assert!(callouts[0].label.starts_with(&format!("{count}× ")));
        if name.starts_with("curved") {
            assert!(callouts[0].label.contains("MAX DEEP"));
        }
        assert_eq!(
            dims.iter()
                .filter(|d| d.id.contains("pitch.length"))
                .count(),
            pitches,
            "{name}"
        );
        assert_eq!(
            dims.iter()
                .filter(|d| d.id.contains("location_pocket_pattern"))
                .count(),
            2,
            "{name}"
        );
        assert!(dims.iter().all(|d| d.placed.is_some()), "{name}: {dims:#?}");
        assert!(
            !plan.dimensions.iter().any(|d| d.kind == "callout"
                && d.id.starts_with("d:f:section_recess:")
                && !d.id.contains("pocket_corner_radius")),
            "{name}"
        );
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        let issues = lint::lint(&a, &drawing);
        assert!(
            !issues
                .iter()
                .any(|i| i.code.starts_with("pocket_pattern_requirement_")
                    || i.code.starts_with("pocket_corner_requirement_")),
            "{name}: {issues:#?}"
        );
    }
}

#[test]
fn rounded_pocket_corner_radius_reaches_native_ink_and_lint() {
    let (part, a) = read("recesses/tuner.step");
    let p = a
        .features
        .iter()
        .find(|f| f.kind == "pocket_pattern")
        .unwrap();
    let group = lint::pocket_patterns::read(&a, p).unwrap();
    let radius = group.pocket.corner_radius.unwrap();
    assert!((radius - 3.94).abs() < 1e-6);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let d = plan
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter.starts_with("pocket_corner_radius.radius."))
        })
        .unwrap();
    assert!(
        d.label
            .contains(&format!("R{}", draftwright::dims::fmt(radius)))
    );
    let drawing = model::render(&part, &a, &plan, &font()).unwrap();
    let mut marks = lint::marks(&drawing);
    marks.retain(|m| {
        !m.claims
            .iter()
            .any(|c| c.parameter.starts_with("pocket_corner_radius.radius."))
    });
    let missing = lint::pockets::corner_coverage(&a, &marks);
    assert_eq!(missing.len(), 20);
    assert!(
        missing
            .iter()
            .all(|i| i.code == "pocket_corner_requirement_missing")
    );
    assert!(
        lint::lint(&a, &drawing)
            .iter()
            .all(|i| !(i.code.starts_with("pocket_pattern_requirement_")
                || i.code.starts_with("pocket_corner_requirement_")))
    );
}

/// LINT-code-pocket_pattern_requirement_state: missing, unverifiable and authored
/// suppression remain distinguishable and honest.
#[test]
fn pocket_pattern_requirement_states_are_checked_independently() {
    let (part, a) = read("recesses/array.step.gz");
    let issues = lint::pocket_patterns::coverage(&a, &[]);
    assert_eq!(issues.len(), 7);
    assert!(
        issues
            .iter()
            .all(|i| i.code == "pocket_pattern_requirement_missing")
    );
    let mut mark = lint::Mark {
        name: String::new(),
        view: None,
        text: String::new(),
        claims: Vec::new(),
    };
    mark.name = "bad".into();
    mark.text = "POCKETS".into();
    let id = a
        .features
        .iter()
        .find(|f| f.kind == "pocket_pattern")
        .unwrap()
        .id
        .clone();
    mark.claims.push(draftwright::output::Claim {
        feature: id.clone(),
        parameter: "pitch.length".into(),
    });
    let issues = lint::pocket_patterns::coverage(&a, &[mark.clone()]);
    assert!(
        issues
            .iter()
            .any(|i| i.code == "pocket_pattern_requirement_unverifiable")
    );
    mark.name = "suppressed:pitch".into();
    let issues = lint::pocket_patterns::coverage(&a, &[mark]);
    assert!(
        issues
            .iter()
            .any(|i| i.code == "pocket_pattern_requirement_suppressed")
    );
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let drawing = model::render(&part, &a, &plan, &font()).unwrap();
    let mut marks = lint::marks(&drawing);
    assert!(lint::prismatic::unlocated_pockets(&a, &marks).is_empty());
    marks.retain(|m| {
        !m.claims
            .iter()
            .any(|c| c.feature == id && c.parameter == "pitch.length")
    });
    assert_eq!(lint::prismatic::unlocated_pockets(&a, &marks).len(), 3);
    // The omission survives the rendered drawing boundary too.
    let spec: Spec = serde_json::from_value(
        json!({"dimensions":{format!("d:{id}/pitch.length"): {"omit":true}}}),
    )
    .unwrap();
    let plan = model::plan(&a, &spec).unwrap();
    let drawing = model::render(&part, &a, &plan, &font()).unwrap();
    assert!(
        lint::lint(&a, &drawing)
            .iter()
            .any(|i| i.code == "pocket_pattern_requirement_suppressed")
    );
}

/// ISSUE-1245-a, ISSUE-1246-a, ARCH0015-1 and ISSUE-917-a now run on the
/// recogniser the CLI actually uses: unsupported passages/pockets are reported,
/// channels own their width, and every accepted grammar can supply dimensions.
#[test]
fn native_recess_grammars_and_unsupported_dispositions_are_preserved() {
    for name in [
        "bounded_pocket",
        "u_channel",
        "u_channel_asymmetric",
        "curved_centered",
    ] {
        let (_, a) = read(&format!("recognition/issues/{name}.step"));
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert!(
            a.features.iter().any(|f| f.kind == "section_recess"),
            "{name}"
        );
        assert!(
            plan.dimensions.iter().any(|d| d
                .measurements
                .iter()
                .any(|m| m.feature.starts_with("f:section_recess:"))),
            "{name}"
        );
        if name.starts_with("u_channel") {
            assert_eq!(
                plan.dimensions
                    .iter()
                    .flat_map(|d| &d.measurements)
                    .filter(|m| m.parameter == "channel_width.length")
                    .count(),
                1
            );
        }
    }
}

#[path = "common/corpus.rs"]
mod corpus;

#[test]
fn native_passages_pockets_and_source_refusals_reach_lint() {
    let Some(root) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&root.join("mfcadpp/1000.step")).unwrap();
    let found = lint::section_recesses::coverage(&a, &[]);
    assert!(
        found
            .iter()
            .any(|i| i.code == "passage_requirement_unsupported")
    );
    assert!(
        a.recognition
            .as_ref()
            .unwrap()
            .refusals
            .iter()
            .any(|r| r.subtype == "SectionRecessRefusal")
    );
    assert!(
        found
            .iter()
            .any(|i| i.code == "section_recess_recognition_refused")
    );
    assert!(
        a.recognition
            .as_ref()
            .unwrap()
            .evidence
            .iter()
            .any(|r| r.family == "step_levels")
    );
    let (_, a) = model::analyse_native(&root.join("inventory_refusal/14052.step.gz")).unwrap();
    assert!(
        lint::section_recesses::coverage(&a, &[])
            .iter()
            .any(|i| i.code == "prismatic_pocket_requirement_unsupported")
    );
}

#[test]
fn native_reader_refuses_bad_recess_fields_local_coordinates_and_pattern_links() {
    use draftwright::bridge::native;
    let (part, _) = read("recesses/array.step.gz");
    let rec = quiddity::correspondence::recognise(&part);
    let doc = quiddity::recognition::document(&rec, part.faces.len());
    for field in ["index", "body", "geometry", "classification", "evidence"] {
        let mut bad = doc.clone();
        bad.records
            .iter_mut()
            .find(|r| r.record_type == "SectionRecess")
            .unwrap()
            .parameters
            .remove(field);
        assert!(native::read(&part, &bad).is_err(), "{field}");
    }
    let mut bad = doc.clone();
    bad.records
        .iter_mut()
        .find(|r| r.record_type == "SectionRecess")
        .unwrap()
        .local
        .push("geometry.frame".into());
    assert!(native::read(&part, &bad).is_err());
    for members in [json!([0, 0, 1]), json!([0, 1, 9999])] {
        let mut bad = doc.clone();
        bad.records
            .iter_mut()
            .find(|r| r.family == "section_recess_patterns")
            .unwrap()
            .parameters
            .insert("members".into(), members);
        assert!(native::read(&part, &bad).is_err());
    }
}

#[test]
fn python_and_native_tuner_patterns_keep_the_same_members_and_measurements() {
    let (part, native) = read("recesses/tuner.step");
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture("recesses/tuner.json.gz")).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    let mut bridge = native.clone();
    model::recognise(&part, &mut bridge, &text).unwrap();
    let pattern = |a: &Analysis| {
        a.features
            .iter()
            .find(|f| f.kind == "pocket_pattern")
            .unwrap()
            .clone()
    };
    let (p, q) = (pattern(&native), pattern(&bridge));
    let world_direction = |a: &Analysis, f: &draftwright::bridge::Feature| {
        draftwright::bridge::holes::dir_to_part(
            &draftwright::bridge::holes::frame(a),
            draftwright::bridge::holes::point(f.parameters.get("direction")).unwrap(),
        )
    };
    let (u, v) = (world_direction(&native, &p), world_direction(&bridge, &q));
    let mut expected = p.parameters["members"].as_array().unwrap().clone();
    if u.iter().zip(v).map(|(x, y)| x * y).sum::<f64>() < 0.0 {
        expected.reverse();
    }
    assert_eq!(json!(expected), q.parameters["members"]);
    assert_eq!(p.faces, q.faces);
    let (p, q) = (
        lint::pocket_patterns::read(&native, &p).unwrap(),
        lint::pocket_patterns::read(&bridge, &q).unwrap(),
    );
    assert_eq!(p.members.len(), 5);
    assert_eq!(q.members.len(), 5);
    for (x, y) in [
        (p.pocket.width, q.pocket.width),
        (p.pocket.length, q.pocket.length),
        (p.pocket.depth, q.pocket.depth),
    ] {
        assert!((x - y).abs() < 1e-3, "{x} != {y}");
    }
    for a in [&native, &bridge] {
        let plan = model::plan(a, &Spec::default()).unwrap();
        let group = plan
            .dimensions
            .iter()
            .find(|d| d.id.starts_with("d:f:pocket_pattern:") && d.kind == "callout")
            .unwrap();
        assert_eq!(group.label, "5× 7.9 × 13.6 × 19 DEEP");
    }
}

#[test]
fn a_recess_location_on_the_far_datum_uses_the_overall_span_once() {
    let Some(root) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&root.join("mfcadpp/1000.step")).unwrap();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let owners: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter == "seat_location.location.x")
        })
        .collect();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].id, "d:envelope/width.length");
    let covered = plan
        .omitted
        .iter()
        .find(|o| o.id.ends_with("/seat_location.location.x"))
        .unwrap();
    assert!(
        matches!(&covered.state,draftwright::items::State::Covered {by} if by=="d:envelope/width.length")
    );
    let mut spec = Spec::default();
    spec.dimensions.insert(
        "d:envelope/width.length".into(),
        model::DimensionSpec {
            omit: true,
            ..Default::default()
        },
    );
    let promoted = model::plan(&a, &spec).unwrap();
    assert!(
        promoted
            .dimensions
            .iter()
            .any(|d| d.id == covered.id && d.placed.is_some())
    );
}

#[test]
fn native_blind_slots_use_the_existing_approved_recess_callout_grammars() {
    let Some(root) = corpus::dir() else { return };
    for (file, labels) in [
        (
            "mfcadpp/10047.step",
            vec![
                "ROUND-BOTTOM OPEN SLOT 1 BOTTOM FLAT × R0.5 × 2.4 LONG",
                "ROUND-BOTTOM OPEN SLOT 1 BOTTOM FLAT × R4.4 × 4.7 LONG",
            ],
        ),
        ("mfcadpp/10245.step", vec!["OPEN SLOT 3.3 × 9.2 × 2.0 DEEP"]),
        (
            "mfcadpp_regressions/11512.step",
            vec!["OPEN SLOT 2.5 × 11.7 × 6.7 DEEP"],
        ),
    ] {
        let (_, a) = model::analyse_native(&root.join(file)).unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        for label in labels {
            let d = plan
                .dimensions
                .iter()
                .find(|d| d.label == label)
                .unwrap_or_else(|| panic!("{file}: {label}"));
            assert!(d.placed.is_some());
            assert!(
                d.measurements
                    .iter()
                    .all(|m| m.feature.starts_with("f:section_recess:"))
            );
        }
    }
}
