//! The bridge from Python quiddity's recognition JSON into the Analysis's features.
//!
//! The fixtures in `tests/fixtures/recognition/` were written by `tools/recognise.py` (quiddity
//! 0.4.0.dev0) from the parts beside them. `plate.step` is a 60 × 40 × 5 plate with four Ø6 holes
//! on a 40 × 20 grid, made in build123d (`Box(60, 40, 5)`, `GridLocations(40, 20, 2, 2)`,
//! `Hole(3)`).
//!
//! That quiddity's face order (OpenCascade's) is haecceity's was checked on eleven parts, 1,137
//! faces: the fixtures, five MFCAD++ and two Gramel parts, sm-hanger, and NIST CTC 01–03. Every
//! face agreed in surface type, and in area within 1e-4 (analytic) or 1e-3 (B-spline) relative.
//! The bridge checks the same on every read.

use std::path::{Path, PathBuf};
use std::process::Command;

use draftwright::bridge;
use draftwright::model::{self, Analysis};
use serde_json::Value;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

fn analysed(name: &str) -> Analysis {
    let step = fixture(&format!("parts/{name}.step"));
    let json = fixture(&format!("recognition/{name}.json"));
    model::analyse_with(&step, Some(&json)).unwrap().1
}

fn recognition(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(fixture(&format!("recognition/{name}.json"))).unwrap(),
    )
    .unwrap()
}

/// The features read from *json* (edited) for the part *name*.
fn read(name: &str, json: &Value) -> Result<Vec<bridge::Feature>, String> {
    let (part, a) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
    bridge::read(&part, &a.binding.source_sha256, &json.to_string()).map(|r| r.1)
}

#[test]
fn holes_carry_their_faces_sizes_axis_position_and_pattern() {
    let a = analysed("plate");
    let ids: Vec<_> = a.features.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["f:hole:1", "f:hole:2", "f:hole:3", "f:hole:4"]);
    let mut at = Vec::new();
    for (f, face) in a.features.iter().zip(6..) {
        assert_eq!(f.kind, "hole");
        assert_eq!(f.faces, [face]);
        let p = &f.fingerprint;
        assert_eq!(p.family, "holes");
        assert_eq!(p.sizes["diameter"], 6.0);
        assert_eq!(p.sizes["depth"], 5.0);
        // quiddity's working frame is not the part's: its axis is x there, z here.
        assert_eq!(f.parameters["axis"], serde_json::json!([-1.0, 0.0, 0.0]));
        assert_eq!(p.axis, Some([0.0, 0.0, 1.0]));
        let pattern = p.pattern.as_ref().unwrap();
        assert_eq!((pattern.count, pattern.pitch), (4, 20.0));
        assert_eq!(p.position[2], 0.0);
        at.push([p.position[0], p.position[1]]);
        assert!(!f.parameters.contains_key("dependents"));
    }
    at.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        at,
        [[-20.0, -10.0], [-20.0, 10.0], [20.0, -10.0], [20.0, 10.0]]
    );
    let r = a.recognition.unwrap();
    assert!(r.recogniser.starts_with("quiddity "), "{}", r.recogniser);
}

#[test]
fn turned_features_are_in_the_parts_frame() {
    let a = analysed("turned");
    let summary: Vec<_> = a
        .features
        .iter()
        .map(|f| (f.id.as_str(), f.faces.clone(), f.fingerprint.position[2]))
        .collect();
    assert_eq!(
        summary,
        [
            ("f:fillet:1", vec![1], 0.4),
            ("f:fillet:2", vec![2], 39.6),
            ("f:fillet:3", vec![5], 40.4),
            ("f:fillet:4", vec![7], 69.6),
            ("f:turned_step:1", vec![0], 20.0),
            ("f:turned_step:2", vec![6], 55.0),
        ]
    );
    for f in &a.features {
        assert_eq!(f.fingerprint.axis, Some([0.0, 0.0, 1.0]), "{}", f.id);
        assert!(f.fingerprint.pattern.is_none());
    }
    let step = &a.features[4].fingerprint.sizes;
    assert_eq!((step["diameter"], step["axial_length"]), (30.0, 40.0));
    assert_eq!(analysed("prismatic").features.len(), 4);
}

#[test]
fn ids_are_unique_stable_and_independent_of_discovery_order() {
    for name in ["plate", "turned", "prismatic"] {
        let a = serde_json::to_string(&analysed(name)).unwrap();
        assert_eq!(a, serde_json::to_string(&analysed(name)).unwrap(), "{name}");
        let features = analysed(name).features;
        let mut ids: Vec<_> = features.iter().map(|f| &f.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), features.len(), "{name}");

        let mut json = recognition(name);
        json["quiddity"]["features"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert_eq!(read(name, &json).unwrap(), features, "{name}");
    }
}

#[test]
fn quiddity_face_indices_are_mapped_to_the_parts() {
    // Relabel quiddity's working faces 6 and 7 as each other's file faces: the hole quiddity
    // found on working face 6 is on the part's face 7.
    let mut json = recognition("plate");
    let first = json["quiddity"]["features"][0].clone();
    assert_eq!(first["constituent_faces"], serde_json::json!([6]));
    let faces = json["quiddity"]["faces"].as_array_mut().unwrap();
    faces[6]["caller_index"] = 7.into();
    faces[7]["caller_index"] = 6.into();
    let features = read("plate", &json).unwrap();
    let moved = features
        .iter()
        .find(|f| Value::from(f.parameters.clone())["location"] == first["record"]["location"])
        .unwrap();
    assert_eq!(moved.faces, [7]);
}

#[test]
fn recognition_of_another_part_or_shape_is_refused() {
    let plate = fixture("parts/plate.step");
    let turned = fixture("recognition/turned.json");
    let e = model::analyse_with(&plate, Some(&turned)).unwrap_err();
    assert!(e.contains("different file"), "{e}");

    let mut json = recognition("plate");
    json["faces"][3]["area"] = 301.0.into();
    let e = read("plate", &json).unwrap_err();
    assert!(e.contains("face 3 has area"), "{e}");

    let mut json = recognition("plate");
    json["faces"][6]["surface"] = "plane".into();
    let e = read("plate", &json).unwrap_err();
    assert!(e.contains("face 6 is a cylinder here"), "{e}");

    let mut json = recognition("plate");
    json["faces"].as_array_mut().unwrap().pop();
    assert!(
        read("plate", &json)
            .unwrap_err()
            .contains("9 faces and the part 10")
    );

    let mut json = recognition("plate");
    let twin = json["quiddity"]["features"][1].clone();
    json["quiddity"]["features"][0]["constituent_faces"] = twin["constituent_faces"].clone();
    let e = read("plate", &json).unwrap_err();
    assert!(e.contains("cannot be told apart"), "{e}");

    let quiddity_alone = recognition("plate")["quiddity"].clone();
    assert!(
        read("plate", &quiddity_alone)
            .unwrap_err()
            .contains("tools/recognise.py")
    );
}

/// The analysis of plate.json (edited).
fn read_all(json: &Value) -> Result<(bridge::Recognition, Vec<bridge::Feature>), String> {
    let (part, a) = model::analyse(&fixture("parts/plate.step")).unwrap();
    bridge::read(&part, &a.binding.source_sha256, &json.to_string())
}

#[test]
fn refusals_and_evidence_are_not_features() {
    // quiddity's records as the corpus has them: a section recess it refused, and a face level.
    let mut json = recognition("plate");
    let features = json["quiddity"]["features"].as_array_mut().unwrap();
    features[0]["family"] = "section_recesses".into();
    features[0]["record_type"] = "SectionRecessRefusal".into();
    features[0]["record"] = serde_json::json!({"reason": "unsupported_support_geometry"});
    features[1]["family"] = "step_levels".into();
    features[1]["record_type"] = "FaceLevel".into();
    features[1]["record"] = serde_json::json!({"axis": "x", "level": 2.5});
    // The plate's hole pattern named those two holes; with them gone it no longer describes
    // the document (a pattern naming a hole that is not there is refused, below).
    json["quiddity"]["derived"]["hole_patterns"] = serde_json::json!([]);
    let (r, features) = read_all(&json).unwrap();
    let ids: Vec<_> = features.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["f:hole:1", "f:hole:2"]);
    assert_eq!(features[0].faces, [8]);
    assert_eq!(r.refusals.len(), 1);
    assert_eq!(r.refusals[0].subtype, "SectionRecessRefusal");
    assert_eq!(r.refusals[0].faces, [6]);
    assert_eq!(
        r.refusals[0].parameters["reason"],
        "unsupported_support_geometry"
    );
    assert_eq!(r.evidence.len(), 1);
    assert_eq!(
        (r.evidence[0].family.as_str(), &r.evidence[0].faces),
        ("step_levels", &vec![7])
    );

    // A hole pattern naming a member that is not a recognised hole is refused.
    let mut json = recognition("plate");
    json["quiddity"]["features"][0]["record_type"] = "FaceLevel".into();
    json["quiddity"]["features"][0]["family"] = "step_levels".into();
    let e = read_all(&json).unwrap_err();
    assert!(e.contains("not a recognised hole"), "{e}");

    // A record type with no home is refused, not guessed at.
    let mut json = recognition("plate");
    json["quiddity"]["features"][0]["record_type"] = "WarpDrive".into();
    let e = read_all(&json).unwrap_err();
    assert!(
        e.contains("WarpDrive (holes), which the bridge has no home for"),
        "{e}"
    );
}

#[test]
fn circular_face_patterns_count_and_pitch_their_instances() {
    // A circular face pattern whose instances are the plate's four holes, one face each.
    let mut json = recognition("plate");
    let instances: Vec<_> = (6..10)
        .map(|f| serde_json::json!({"face_indices": [f]}))
        .collect();
    json["quiddity"]["features"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "family": "circular_face_patterns",
            "record_type": "CircularFacePattern",
            "record": {"count": 4},
            "constituent_faces": [6, 7, 8, 9],
            "instances": instances,
        }));
    let (_, features) = read_all(&json).unwrap();
    let p = features
        .iter()
        .find(|f| f.kind == "circular_face_pattern")
        .unwrap();
    assert_eq!(p.id, "f:circular_face_pattern:1");
    assert_eq!(p.faces, [6, 7, 8, 9]);
    let pattern = p.fingerprint.pattern.as_ref().unwrap();
    assert_eq!((pattern.count, pattern.pitch), (4, 20.0));
}

#[test]
fn unreadable_fingerprint_parts_are_refused() {
    let mut json = recognition("plate");
    json["quiddity"]["features"][0]["record"]["axis"] = "w".into();
    let e = read_all(&json).unwrap_err();
    assert!(e.contains("axis \"w\" is not an axis"), "{e}");

    let mut json = recognition("plate");
    let holes = &mut json["quiddity"]["derived"]["hole_patterns"][0]["holes"];
    holes[3]["location"] = Value::Null;
    let e = read_all(&json).unwrap_err();
    assert!(e.contains("member with no location"), "{e}");
}

#[test]
fn the_command_line_takes_features() {
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_draftwright"))
            .args(args)
            .output()
            .unwrap();
        let json: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code(), json)
    };
    let step = fixture("parts/plate.step");
    let features = fixture("recognition/plate.json");
    let (step, features) = (step.to_str().unwrap(), features.to_str().unwrap());
    let (code, a) = run(&["analyse", step, "--features", features]);
    assert_eq!(code, Some(0));
    assert_eq!(a["features"].as_array().unwrap().len(), 4);
    let (code, _) = run(&["plan", step, "--features", features]);
    assert_eq!(code, Some(0));
    let wrong = fixture("recognition/turned.json");
    let (code, e) = run(&["analyse", step, "--features", wrong.to_str().unwrap()]);
    assert_eq!(code, Some(2));
    assert_eq!(e["error"]["code"], "invalid");
}

/// The AAG neighbourhood: repeats of one feature in like surroundings share it (position tells
/// them apart), and features in different surroundings do not.
#[test]
fn neighbourhoods_describe_how_a_feature_meets_the_part() {
    let plate = analysed("plate");
    let through = ["cylinder>out:plane:convex", "cylinder>out:plane:convex"];
    for f in &plate.features {
        assert_eq!(f.fingerprint.neighbourhood, through, "{}", f.id);
    }
    let turned = analysed("turned");
    let of = |id: &str| {
        &turned
            .features
            .iter()
            .find(|f| f.id == id)
            .unwrap()
            .fingerprint
            .neighbourhood
    };
    assert_eq!(
        of("f:fillet:1"),
        &["torus>out:cylinder:smooth", "torus>out:plane:smooth"]
    );
    assert_eq!(
        of("f:turned_step:1"),
        &["cylinder>out:torus:smooth", "cylinder>out:torus:smooth"]
    );
    assert_ne!(of("f:fillet:1"), of("f:turned_step:1"));
}

/// quiddity reports a rectangle of four as width and height, and an N×M lattice (RectGrid) as
/// rows, columns and their pitches; both are a grid (nist_ftc_10 has a RectGrid).
#[test]
fn a_rect_grid_is_a_grid() {
    let mut json = recognition("plate");
    let p = json["quiddity"]["derived"]["hole_patterns"][0]
        .as_object_mut()
        .unwrap();
    let (w, h) = (p.remove("width").unwrap(), p.remove("height").unwrap());
    for (k, v) in [
        ("rows", 2.into()),
        ("cols", 2.into()),
        ("row_pitch", h),
        ("col_pitch", w),
    ] {
        p.insert(k.into(), v);
    }
    let (part, a) = model::analyse(&fixture("parts/plate.step")).unwrap();
    let (recognition, _) =
        bridge::read(&part, &a.binding.source_sha256, &json.to_string()).unwrap();
    assert_eq!(recognition.patterns.len(), 1);
    assert_eq!(recognition.patterns[0].arrangement, "grid");
    assert_eq!(recognition.patterns[0].members.len(), 4);
}
