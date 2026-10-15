//! Datums and feature control frames authored in the Spec ([`draftwright::spec_gdt`],
//! draftwright-rust #43 item 1): lowered into the records the AP242 PMI path reads, so they are
//! drawn, placed and linted as the file's own are; the Spec wins over the file's PMI with a
//! warning; what cannot be attached is refused, naming the entry.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::json;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn plate() -> (Part, Analysis) {
    model::analyse_native(&root().join("tests/fixtures/parts/plate.step")).unwrap()
}

/// The editor's spindle with the reviewer's AP242 PMI (datums A and B, ⌖ ⌀0.25 and ⌀0.05).
fn spindle() -> (Part, Analysis) {
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::read(root().join("tests/fixtures/samples/cnc-review-spindle.step.gz"))
            .unwrap()
            .as_slice(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spec_gdt");
    std::fs::create_dir_all(&out).unwrap();
    let path = out.join("cnc-review-spindle.step");
    let part = out.join(format!("{:?}.part", std::thread::current().id()));
    std::fs::write(&part, &bytes).unwrap();
    std::fs::rename(&part, &path).unwrap();
    model::analyse_native(&path).unwrap()
}

fn spec(v: serde_json::Value) -> Spec {
    serde_json::from_value(v).expect("the spec reads")
}

fn font() -> draftwright::text::Font {
    draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap()
}

fn gdt<'a>(plan: &'a Plan, kind: &str) -> Vec<&'a draftwright::dims::gdt::PlannedGdt> {
    plan.gdt.iter().filter(|g| g.kind == kind).collect()
}

/// The drawing's PMI lint findings, against *a* with the datums and controls of the spec the
/// drawing embeds merged in, as `api::review` lints (lint itself reads no spec).
fn pmi_lint(a: &Analysis, d: &draftwright::output::Drawing) -> Vec<String> {
    let s: Spec = serde_json::from_str(d.spec.as_deref().expect("the drawing embeds its spec"))
        .expect("the embedded spec reads");
    let merged = draftwright::spec_gdt::effective(a, &s).expect("the plan accepted the spec");
    draftwright::lint::lint(&merged, d)
        .into_iter()
        .filter(|i| i.code.starts_with("pmi_") || i.code.starts_with("datum"))
        .map(|i| format!("{}: {}", i.code, i.message))
        .collect()
}

/// The runs of the drawing's annotation *name*.
fn runs<'a>(d: &'a draftwright::output::Drawing, name: &str) -> Vec<&'a str> {
    let a = d
        .annotations
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("{name} is drawn"));
    a.runs.iter().map(|&i| d.text[i].text.as_str()).collect()
}

#[test]
fn a_datum_on_the_bottom_face_and_a_position_on_the_hole_pattern_are_drawn() {
    let (part, a) = plate();
    assert!(a.pmi.is_none(), "plate.step carries no PMI");
    let s = spec(json!({
        "faces": {"bottom": {"datum": "A"}},
        "features": {"f:hole_pattern:1": {"controls": [
            {"characteristic": "position", "tolerance": 0.1, "datums": ["A"]}
        ]}}
    }));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let datums = gdt(&plan, "datum_feature");
    assert_eq!(datums.len(), 1);
    let datum = datums[0];
    assert_eq!(datum.cells, ["A"]);
    assert_eq!(
        (datum.view.as_str(), datum.side.as_str()),
        ("v:front", "below")
    );
    assert!(datum.basis.starts_with("spec-authored"), "{}", datum.basis);
    assert_eq!(datum.source, "spec:faces/bottom/datum");
    assert!(!datum.default);
    let frames = gdt(&plan, "control_frame");
    assert_eq!(frames.len(), 1);
    let frame = frames[0];
    assert_eq!(frame.label, "⌖ | ⌀0.1 | A");
    // Seen end-on, where the holes' common axis is: the plan view.
    assert_eq!(frame.view, "v:plan");
    let mut holes: Vec<usize> = a
        .features
        .iter()
        .filter(|f| f.kind == "hole")
        .flat_map(|f| f.faces.clone())
        .collect();
    holes.sort();
    assert_eq!(frame.faces, holes);
    assert_eq!(
        frame.id,
        format!(
            "pmi:geometric_tolerance.position:f{}:ref=A:mod=diameter_zone",
            holes
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    );
    for g in [datum, frame] {
        assert!(g.placed.is_some(), "{} is placed", g.id);
        assert_eq!(serde_json::to_value(&g.state).unwrap()["kind"], "shown");
    }
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code.starts_with("pmi_") || p.code == "datum_undeclared"),
        "{:?}",
        plan.problems
    );
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert_eq!(runs(&d, &datum.id), ["A"]);
    assert_eq!(runs(&d, &frame.id), ["⌖", "⌀0.1", "A"]);
    assert!(d.svg().contains("⌀0.1"));
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
    // Deterministic.
    let again = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        serde_json::to_string(&again).unwrap(),
        serde_json::to_string(&plan).unwrap()
    );
}

#[test]
fn a_flatness_control_on_a_face_draws_its_frame() {
    let (part, a) = plate();
    let s = spec(json!({
        "faces": {"top": {"controls": [{"characteristic": "flatness", "tolerance": 0.05}]}}
    }));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let frames = gdt(&plan, "control_frame");
    assert_eq!(frames.len(), 1);
    let frame = frames[0];
    assert_eq!(frame.label, "⏥ | 0.05");
    assert_eq!(frame.source, "spec:faces/top/controls/0");
    assert!(frame.placed.is_some());
    // The top face, alone: z at the box's top.
    assert_eq!(frame.faces.len(), 1);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert_eq!(runs(&d, &frame.id), ["⏥", "0.05"]);
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
}

#[test]
fn a_spec_control_that_restates_step_pmi_wins_with_a_problem() {
    let (part, a) = spindle();
    let s = spec(json!({
        "features": {"f:hole:2": {"controls": [
            {"characteristic": "position", "tolerance": 0.02, "datums": ["A", "B"]}
        ]}}
    }));
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let plan = model::plan_recovered(&a, &s).unwrap();
    let labels = |p: &Plan| -> Vec<String> {
        gdt(p, "control_frame")
            .iter()
            .map(|g| g.label.clone())
            .collect()
    };
    assert!(labels(&base).contains(&"⌖ | ⌀0.05 | A | B".to_string()));
    let after = labels(&plan);
    assert!(
        after.contains(&"⌖ | ⌀0.02 | A | B".to_string()),
        "{after:?}"
    );
    assert!(!after.iter().any(|l| l.contains("0.05")), "{after:?}");
    assert!(
        after.contains(&"⌖ | ⌀0.25 | A | B".to_string()),
        "{after:?}"
    );
    // The same requirement on the same faces: the same item, so its pins carry over.
    let id = |p: &Plan, label: &str| {
        gdt(p, "control_frame")
            .iter()
            .find(|g| g.label.contains(label))
            .map(|g| g.id.clone())
            .unwrap()
    };
    assert_eq!(id(&base, "0.05"), id(&plan, "0.02"));
    let overrides: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "spec_overrides_pmi")
        .collect();
    assert_eq!(overrides.len(), 1, "{:?}", plan.problems);
    assert_eq!(overrides[0].severity, "warning");
    assert_eq!(
        overrides[0].refs,
        [
            "geometric_tolerance:#918",
            "spec:features/f:hole:2/controls/0"
        ]
    );
    assert!(overrides[0].message.contains("0.05"));
    // A datum the spec restates replaces the file's of the same letter.
    let datum = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:turned_step:2": {"datum": "B"}}})),
    )
    .unwrap();
    let b: Vec<_> = gdt(&datum, "datum_feature")
        .into_iter()
        .filter(|g| g.cells == ["B"])
        .collect();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].source, "spec:features/f:turned_step:2/datum");
    assert!(
        datum
            .problems
            .iter()
            .any(|p| p.code == "spec_overrides_pmi" && p.refs[0] == "datum:#859")
    );
    // Lint judges the drawing against the merged reading: nothing lost, nothing unknown.
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    let base_d = model::render(&part, &a, &base, &font()).unwrap();
    assert_eq!(pmi_lint(&a, &d), pmi_lint(&a, &base_d));
}

#[test]
fn an_unknown_feature_or_face_is_a_clear_error() {
    let (_, a) = plate();
    let s = spec(json!({
        "features": {"f:hole_patern:1": {"controls": [
            {"characteristic": "position", "tolerance": 0.1}
        ]}}
    }));
    let e = model::plan_recovered(&a, &s).unwrap_err();
    assert!(
        e.message
            .starts_with("There is no feature \"f:hole_patern:1\" on this part"),
        "{}",
        e.message
    );
    assert!(
        e.message.contains("features/f:hole_patern:1"),
        "{}",
        e.message
    );
    assert_eq!(e.choices[0], "f:hole_pattern:1");
    let e = model::plan_recovered(&a, &spec(json!({"faces": {"underside": {"datum": "A"}}})))
        .unwrap_err();
    assert!(
        e.message.contains("\"underside\" names no face"),
        "{}",
        e.message
    );
    assert!(e.choices.contains(&"bottom".to_string()));
    let e = model::plan_recovered(&a, &spec(json!({"faces": {"face:99": {"datum": "A"}}})))
        .unwrap_err();
    assert!(
        e.message.contains("There is no face \"face:99\""),
        "{}",
        e.message
    );
}

#[test]
fn edits_author_datums_and_controls_and_refuse_what_cannot_be_drawn() {
    let ops = json!([
        {"op": "set", "path": "faces/bottom/datum", "value": "A"},
        {"op": "add", "path": "features/f:hole_pattern:1/controls",
         "value": {"characteristic": "position", "tolerance": 0.1, "datums": ["A"]}}
    ]);
    let s = model::edit(&Spec::default(), &ops).unwrap();
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["faces"]["bottom"]["datum"], "A");
    assert_eq!(
        v["features"]["f:hole_pattern:1"]["controls"][0]["characteristic"],
        "position"
    );
    let refused = |value: serde_json::Value| {
        model::edit(
            &Spec::default(),
            &json!([{"op": "add", "path": "faces/top/controls", "value": value}]),
        )
        .unwrap_err()
    };
    let e = refused(json!({"characteristic": "flatnes", "tolerance": 0.1}));
    assert!(
        e.message.contains("is not a characteristic"),
        "{}",
        e.message
    );
    assert_eq!(e.choices[0], "flatness");
    let e = refused(json!({"characteristic": "flatness", "tolerance": 0.1, "datums": ["A"]}));
    assert!(e.message.contains("refers to no datum"), "{}", e.message);
    let e = refused(json!({"characteristic": "position", "tolerance": 0.0}));
    assert!(e.message.contains("positive"), "{}", e.message);
    let e = refused(json!({"characteristic": "position", "tolerance": 0.1, "modifier": "P"}));
    assert!(e.message.contains("projected"), "{}", e.message);
    let e = model::edit(
        &Spec::default(),
        &json!([{"op": "set", "path": "faces/top/datum", "value": "a"}]),
    )
    .unwrap_err();
    assert_eq!(e.choices, ["A"]);
    // A control referring to a datum nothing declares is drawn, with a warning.
    let (_, a) = plate();
    let plan = model::plan_recovered(
        &a,
        &spec(json!({"faces": {"top": {"controls": [
            {"characteristic": "parallelism", "tolerance": 0.05, "datums": ["B"]}
        ]}}})),
    )
    .unwrap();
    assert_eq!(gdt(&plan, "control_frame").len(), 1);
    assert!(
        plan.problems
            .iter()
            .any(|p| p.code == "datum_undeclared" && p.severity == "warning")
    );
}

#[test]
fn the_session_offers_datum_and_control_paths_on_a_feature() {
    let (part, a) = plate();
    let session = draftwright::Session::new(part, a, Spec::default()).unwrap();
    let options =
        serde_json::to_value(session.options("d:f:hole_pattern:1/bore.diameter").unwrap()).unwrap();
    let settable = &options["settable"];
    assert_eq!(
        settable["features/f:hole_pattern:1/datum"]["forms"],
        json!(["A"])
    );
    let controls = &settable["features/f:hole_pattern:1/controls"];
    assert!(
        controls["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("characteristic"))
    );
}
