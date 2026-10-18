//! Surface finish, threads, knurls and notes authored in the Spec ([`draftwright::spec_notes`],
//! draftwright-rust #43 item 1, stream K1b): lowered into the records the AP242 PMI path reads,
//! so each is drawn by the path that draws the file's own (the ISO 1302 symbol, the owner's
//! callout, the leadered label, the default finish beside the title block); the Spec wins over
//! the file's PMI with a warning; what cannot be read or attached is refused or reported.

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

/// A gzipped sample, from its own scratch file.
fn sample(name: &str) -> (Part, Analysis) {
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::read(
            root()
                .join("tests/fixtures/samples")
                .join(format!("{name}.step.gz")),
        )
        .unwrap()
        .as_slice(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    // nextest runs cases in separate processes, each starting at ThreadId(1).
    // Isolate their atomic staging files as well as ordinary harness threads.
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spec_notes")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&out).unwrap();
    let path = out.join(format!("{name}.step"));
    let part = out.join(format!("{name}.{:?}.part", std::thread::current().id()));
    std::fs::write(&part, &bytes).unwrap();
    std::fs::rename(&part, &path).unwrap();
    model::analyse_native(&path).unwrap()
}

/// The editor's spindle without PMI: ø10, ø15 and ø24 turned steps, a ⌀4.2 blind hole.
fn spindle() -> (Part, Analysis) {
    sample("specify-sample-spindle")
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

fn dimension<'a>(plan: &'a Plan, id: &str) -> &'a draftwright::dims::PlannedDimension {
    plan.dimensions
        .iter()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("{id} is planned"))
}

/// The drawing's PMI lint findings, against *a* with the spec it embeds merged in, as
/// `api::review` lints.
fn pmi_lint(a: &Analysis, d: &draftwright::output::Drawing) -> Vec<String> {
    let s: Spec = serde_json::from_str(d.spec.as_deref().expect("the drawing embeds its spec"))
        .expect("the embedded spec reads");
    let merged = draftwright::spec_gdt::effective(a, &s).expect("the plan accepted the spec");
    draftwright::lint::lint(&merged, d)
        .into_iter()
        .filter(|i| i.code.starts_with("pmi_"))
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

fn problems<'a>(plan: &'a Plan, code: &str) -> Vec<&'a model::Problem> {
    plan.problems.iter().filter(|p| p.code == code).collect()
}

fn no_pmi_problems(plan: &Plan) {
    assert!(
        !plan.problems.iter().any(|p| p.code.starts_with("pmi_")),
        "{:?}",
        plan.problems
    );
}

#[test]
fn ra_0_8_on_a_plate_face_draws_its_iso_1302_symbol() {
    let (part, a) = plate();
    assert!(a.pmi.is_none(), "plate.step carries no PMI");
    let s = spec(json!({"faces": {"top": {"finish": {"ra": 0.8}}}}));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let finishes = gdt(&plan, "surface_finish");
    assert_eq!(finishes.len(), 1);
    let f = finishes[0];
    assert_eq!(f.cells, ["Ra 0.8"]);
    assert_eq!(f.source, "spec:faces/top/finish");
    assert_eq!((f.view.as_str(), f.side.as_str()), ("v:front", "above"));
    assert!(f.basis.starts_with("spec-authored"), "{}", f.basis);
    assert!(!f.default);
    assert!(f.placed.is_some());
    assert_eq!(f.faces.len(), 1, "the top face alone");
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert_eq!(runs(&d, &f.id), ["Ra 0.8"]);
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
    // On a feature, its cylinders carry it; a removal condition is drawn as ISO 1302's bar.
    let (part, a) = spindle();
    let s = spec(
        json!({"features": {"f:turned_step:2": {"finish": {"ra": 1.6, "removal": "required"}}}}),
    );
    let plan = model::plan_recovered(&a, &s).unwrap();
    let f = gdt(&plan, "surface_finish")[0];
    assert_eq!(f.cells, ["Ra 1.6"]);
    let step = a
        .features
        .iter()
        .find(|f| f.id == "f:turned_step:2")
        .unwrap();
    assert_eq!(f.faces, step.faces);
    assert!(f.placed.is_some());
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
}

#[test]
fn a_default_finish_is_drawn_beside_the_title_block() {
    let (part, a) = plate();
    let s = spec(json!({"sheet": {"default_finish": {"ra": 3.2, "removal": "prohibited"}}}));
    let plan = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(plan.requirements.len(), 1);
    let r = &plan.requirements[0];
    assert_eq!(
        (r.kind.as_str(), r.text.as_str(), r.carrier.as_str()),
        ("default_surface_finish", "Ra 3.2", "sheet")
    );
    assert_eq!(r.source, "spec:sheet/default_finish");
    assert!(r.basis.starts_with("spec-authored"), "{}", r.basis);
    // No material removal: ISO 1302's circle.
    let placed = r.placed.as_ref().expect("the symbol is placed");
    assert!(!placed.arcs.is_empty(), "{placed:?}");
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert!(d.svg().contains("Ra 3.2"));
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
}

#[test]
fn m6_on_the_plate_hole_pattern_prints_on_its_callout() {
    let (part, a) = plate();
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let id = "d:f:hole_pattern:1/bore.diameter";
    assert_eq!(dimension(&base, id).label, "4× ⌀6 THRU (2×2)");
    let s = spec(json!({"features": {"f:hole_pattern:1": {"thread": {"designation": "M6"}}}}));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let callout = dimension(&plan, id);
    assert_eq!(callout.label, "4× M6 THRU\n⌀6 THRU (2×2)");
    let claim = callout
        .measurements
        .iter()
        .find(|m| m.parameter == "thread")
        .expect("the callout claims the thread");
    assert!(
        claim
            .feature
            .starts_with("pmi:manufacturing_requirement.internal_thread:f"),
        "{}",
        claim.feature
    );
    assert!(
        callout.basis.contains("the spec's thread"),
        "{}",
        callout.basis
    );
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert!(d.svg().contains("M6"));
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
    // A depth prints as the minimum full thread; a member of the pattern is pointed at it; a tap
    // too small for its hole is reported, not drawn.
    let plan = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:hole_pattern:1": {"thread": {"designation": "M6x1-6H", "length": 4}}}})),
    )
    .unwrap();
    assert_eq!(dimension(&plan, id).label, "4× M6-6H ↧ 4\n⌀6 THRU (2×2)");
    let plan = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:hole:1": {"thread": {"designation": "M6"}}}})),
    )
    .unwrap();
    let refused = problems(&plan, "pmi_not_lowered");
    assert_eq!(refused.len(), 1, "{:?}", plan.problems);
    assert_eq!(refused[0].severity, "error");
    assert_eq!(refused[0].refs, ["spec:features/f:hole:1/thread"]);
    assert!(
        refused[0]
            .message
            .contains("features/f:hole_pattern:1/thread"),
        "{}",
        refused[0].message
    );
    assert_eq!(dimension(&plan, id).label, "4× ⌀6 THRU (2×2)");
    let plan = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:hole_pattern:1": {"thread": {"designation": "M3"}}}})),
    )
    .unwrap();
    let refused = problems(&plan, "pmi_not_lowered");
    assert_eq!(refused.len(), 1, "{:?}", plan.problems);
    assert!(
        refused[0].message.contains("cannot be cut"),
        "{}",
        refused[0].message
    );
}

#[test]
fn m10_and_a_knurl_on_the_spindle_print_after_their_diameters() {
    let (part, a) = spindle();
    assert!(a.pmi.is_none(), "the sample spindle carries no PMI");
    let s = spec(json!({"features": {
        "f:turned_step:1": {"thread": {"designation": "M10", "length": 15}},
        "f:turned_step:3": {"knurl": {"pattern": "diamond", "pitch": 0.8}}
    }}));
    let plan = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        dimension(&plan, "d:f:turned_step:1/step.diameter").label,
        "M10; 15 MIN FULL THREAD"
    );
    // The ø24 step's diameter is the overall diameter's, which prints its knurl.
    let od = dimension(&plan, "d:envelope/od.diameter");
    assert_eq!(od.label, "ø24\nDIAMOND KNURL P0.8");
    assert!(od.measurements.iter().any(|m| m.parameter == "knurl"));
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert!(d.svg().contains("M10"));
    assert!(d.svg().contains("KNURL"));
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
    // A thread whose nominal size is not its diameter, or longer than its step, is reported.
    let plan = model::plan_recovered(
        &a,
        &spec(json!({"features": {
            "f:turned_step:2": {"thread": {"designation": "M16"}},
            "f:turned_step:1": {"thread": {"designation": "M10", "length": 40}},
            "f:hole:1": {"thread": {"designation": "M5", "length": 12}}
        }})),
    )
    .unwrap();
    let refused = problems(&plan, "pmi_not_lowered");
    assert_eq!(refused.len(), 3, "{:?}", plan.problems);
    // A tap deeper than its blind hole (ISSUE-1360-a).
    assert!(
        refused
            .iter()
            .any(|p| p.message.contains("exceeds its hole's depth 9.2"))
    );
    assert!(
        refused
            .iter()
            .any(|p| p.message.contains("not the nominal size"))
    );
    assert!(
        refused
            .iter()
            .any(|p| p.message.contains("exceeds its length 18"))
    );
}

#[test]
fn a_feature_note_and_a_sheet_note_are_drawn() {
    let (part, a) = spindle();
    let s = spec(json!({
        "features": {"f:hole:1": {"note": "DRILL AND TAP AFTER HARDENING"}},
        "notes": [{"id": "n:1", "text": "BREAK ALL SHARP EDGES"}]
    }));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let labels = gdt(&plan, "label");
    assert_eq!(labels.len(), 1);
    let note = labels[0];
    assert_eq!(note.cells, ["DRILL AND TAP AFTER HARDENING"]);
    assert_eq!(note.source, "spec:features/f:hole:1/note");
    assert!(note.basis.starts_with("spec-authored"), "{}", note.basis);
    assert!(note.placed.is_some());
    let hole = a.features.iter().find(|f| f.id == "f:hole:1").unwrap();
    assert_eq!(note.faces, hole.faces);
    assert_eq!(plan.notes.len(), 1);
    assert_eq!(plan.notes[0].rows, ["1  BREAK ALL SHARP EDGES"]);
    no_pmi_problems(&plan);
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    assert_eq!(runs(&d, &note.id), ["DRILL AND TAP AFTER", "HARDENING"]);
    assert!(d.svg().contains("BREAK ALL SHARP EDGES"));
    assert_eq!(pmi_lint(&a, &d), Vec::<String>::new());
    // A face takes a note too.
    let (_, plate) = plate();
    let plan = model::plan_recovered(&plate, &spec(json!({"faces": {"top": {"note": "DEBURR"}}})))
        .unwrap();
    assert_eq!(gdt(&plan, "label")[0].cells, ["DEBURR"]);
}

#[test]
fn spec_finish_threads_and_knurls_win_over_step_pmi_with_a_problem() {
    let (part, a) = sample("cnc-review-spindle");
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(
        dimension(&base, "d:envelope/od.diameter").label,
        "ø24 MAX AFTER KNURL\nDIAMOND KNURL P1"
    );
    assert_eq!(
        base.requirements
            .iter()
            .filter(|r| r.text == "Ra 3.2")
            .count(),
        1
    );
    let s = spec(json!({
        "features": {
            "f:turned_step:3": {"knurl": {"pattern": "straight", "pitch": 1.0}},
            "f:hole:1": {"thread": {"designation": "M5x0.8-6H", "length": 8}}
        },
        "sheet": {"default_finish": {"ra": 1.6, "removal": "required"}}
    }));
    let plan = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        dimension(&plan, "d:envelope/od.diameter").label,
        "ø24\nSTRAIGHT KNURL P1"
    );
    assert_eq!(
        dimension(&plan, "d:f:hole:1/bore.diameter").label,
        "M5-6H ↧ 8\n⌀4.2 ↧ 9.2"
    );
    let finishes: Vec<_> = plan
        .requirements
        .iter()
        .filter(|r| r.kind == "default_surface_finish")
        .collect();
    assert_eq!(finishes.len(), 1);
    assert_eq!(finishes[0].text, "Ra 1.6");
    let mut overridden: Vec<(&str, &str)> = problems(&plan, "spec_overrides_pmi")
        .iter()
        .map(|p| (p.refs[0].as_str(), p.refs[1].as_str()))
        .collect();
    overridden.sort();
    assert_eq!(
        overridden,
        [
            (
                "manufacturing_requirement:#1007",
                "spec:sheet/default_finish"
            ),
            (
                "manufacturing_requirement:#925",
                "spec:features/f:hole:1/thread"
            ),
            (
                "manufacturing_requirement:#947",
                "spec:features/f:turned_step:3/knurl"
            ),
        ]
    );
    assert!(
        problems(&plan, "spec_overrides_pmi")
            .iter()
            .all(|p| p.severity == "warning")
    );
    assert!(
        problems(&plan, "pmi_not_lowered").is_empty(),
        "{:?}",
        plan.problems
    );
    // Lint judges the drawing against the merged reading: nothing lost, nothing unknown.
    let d = model::render(&part, &a, &plan, &font()).unwrap();
    let base_d = model::render(&part, &a, &base, &font()).unwrap();
    assert_eq!(pmi_lint(&a, &d), pmi_lint(&a, &base_d));
}

#[test]
fn unknown_ids_and_unreadable_entries_are_refused() {
    let (_, a) = plate();
    let e = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:hole_patern:1": {"thread": {"designation": "M6"}}}})),
    )
    .unwrap_err();
    assert!(
        e.message
            .starts_with("There is no feature \"f:hole_patern:1\" on this part"),
        "{}",
        e.message
    );
    assert_eq!(e.choices[0], "f:hole_pattern:1");
    let e = model::plan_recovered(
        &a,
        &spec(json!({"faces": {"underside": {"finish": {"ra": 0.8}}}})),
    )
    .unwrap_err();
    assert!(
        e.message.contains("\"underside\" names no face"),
        "{}",
        e.message
    );
    let e = model::plan_recovered(
        &a,
        &spec(json!({"features": {"f:hole:1": {"knurl": {"pattern": "straight", "pitch": 0.8}}}})),
    )
    .unwrap_err();
    assert!(e.message.contains("takes no knurl"), "{}", e.message);
    let refused = |ops: serde_json::Value| model::edit(&Spec::default(), &ops).unwrap_err();
    let e = refused(json!([{"op": "set", "path": "faces/top/finish", "value": {"ra": 0}}]));
    assert!(e.message.contains("positive"), "{}", e.message);
    let e = refused(
        json!([{"op": "set", "path": "faces/top/finish", "value": {"ra": 0.8, "removal": "maybe"}}]),
    );
    assert_eq!(e.choices, ["any", "required", "prohibited"]);
    let e = refused(
        json!([{"op": "set", "path": "features/f:hole:1/thread", "value": {"designation": "1/4-20 UNC"}}]),
    );
    assert!(
        e.message.contains("not an ISO metric thread"),
        "{}",
        e.message
    );
    let e = refused(
        json!([{"op": "set", "path": "features/f:hole:1/thread", "value": {"designation": "M13"}}]),
    );
    assert!(
        e.message.contains("no ISO 261 coarse pitch"),
        "{}",
        e.message
    );
    let e = refused(
        json!([{"op": "set", "path": "features/f:turned_step:1/knurl", "value": {"pattern": "spiral", "pitch": 1}}]),
    );
    assert_eq!(e.choices, ["straight", "diamond"]);
    let e = refused(json!([{"op": "set", "path": "features/f:hole:1/note", "value": "  "}]));
    assert!(e.message.contains("empty"), "{}", e.message);
    // What reads is kept, as written.
    let s = model::edit(
        &Spec::default(),
        &json!([
            {"op": "set", "path": "faces/top/finish", "value": {"ra": 0.8}},
            {"op": "set", "path": "features/f:hole:1/thread", "value": {"designation": "M5 LH", "length": 8}},
            {"op": "set", "path": "sheet/default_finish", "value": {"ra": 3.2}}
        ]),
    )
    .unwrap();
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["faces"]["top"]["finish"]["ra"], 0.8);
    assert_eq!(v["features"]["f:hole:1"]["thread"]["designation"], "M5 LH");
    assert_eq!(v["sheet"]["default_finish"]["ra"], 3.2);
}

#[test]
fn the_session_offers_finish_thread_knurl_and_note_paths() {
    let (part, a) = spindle();
    let session = draftwright::Session::new(part, a, Spec::default()).unwrap();
    let options =
        serde_json::to_value(session.options("d:f:turned_step:1/step.diameter").unwrap()).unwrap();
    let settable = &options["settable"];
    let base = "features/f:turned_step:1";
    for path in ["finish", "thread", "knurl", "note"] {
        assert!(
            settable.get(format!("{base}/{path}")).is_some(),
            "{path}: {settable}"
        );
    }
    let options =
        serde_json::to_value(session.options("d:f:hole:1/bore.diameter").unwrap()).unwrap();
    assert!(
        options["settable"]
            .get("features/f:hole:1/thread")
            .is_some()
    );
    assert!(options["settable"].get("features/f:hole:1/knurl").is_none());
}

#[test]
fn structured_feature_notes_cover_only_explicit_roles_of_their_placed_owner() {
    let (part, a) = plate();
    let owner = a
        .features
        .iter()
        .find(|f| f.kind == "hole")
        .unwrap()
        .id
        .clone();
    let make = |claims: bool| {
        spec(json!({
            "dimension_source":"authored",
            "features":{&owner:{"note":"DIAMETER PER PROCESS SHEET", "note_satisfies":if claims {vec!["bore.diameter"]} else {vec![]}}}
        }))
    };
    let draw = |s: &Spec| {
        let plan = model::plan(&a, s).unwrap();
        assert!(plan.dimensions.is_empty());
        let merged = draftwright::spec_gdt::effective(&a, s)
            .unwrap()
            .into_owned();
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        (merged, drawing)
    };
    let missing = |a: &Analysis, d: &draftwright::output::Drawing| {
        draftwright::lint::lint(a, d)
            .into_iter()
            .filter(|i| {
                i.code == "hole_requirement_missing"
                    && i.requirements
                        .iter()
                        .any(|r| r.parameter == "bore.diameter")
            })
            .collect::<Vec<_>>()
    };
    let (merged, prose) = draw(&make(false));
    let before = missing(&merged, &prose);
    assert!(!before.is_empty());
    let (merged, mut structured) = draw(&make(true));
    let annotation = structured
        .annotations
        .iter()
        .find(|a| !a.satisfies.is_empty())
        .unwrap()
        .clone();
    assert!(annotation.claims.is_empty());
    assert_eq!(annotation.satisfies.len(), 1);
    // The plate has identical siblings. Its grouped missing requirement must remain until
    // every physical identity has authority; one note must not close the group.
    let after = missing(&merged, &structured);
    assert!(!after.is_empty());
    assert!(
        after
            .iter()
            .all(|i| i.requirements.iter().any(|r| r.feature != owner))
    );
    assert!(
        after
            .iter()
            .flat_map(|i| &i.requirements)
            .all(|r| r.feature != owner)
    );
    structured.annotations.push(annotation.clone());
    assert_eq!(missing(&merged, &structured), after);
    structured.annotations.retain(|a| a.name != annotation.name);
    assert_eq!(missing(&merged, &structured), before);

    // With an explicit note for every physical hole, the diameter requirement closes,
    // independently of through/count roles. Omitting the note's view grants no authority.
    let mut all = make(true);
    for f in a.features.iter().filter(|f| f.kind == "hole") {
        all.features.insert(
            f.id.clone(),
            serde_json::from_value(
                json!({"note":"SIZE PER PROCESS", "note_satisfies":["bore.diameter"]}),
            )
            .unwrap(),
        );
    }
    let (merged, mut complete) = draw(&all);
    assert!(missing(&merged, &complete).is_empty());
    assert!(
        draftwright::lint::lint(&merged, &complete)
            .iter()
            .any(|i| i.requirements.iter().any(|r| r.parameter == "bore.through"))
    );
    for a in &mut complete.annotations {
        a.view = None;
    }
    assert!(!missing(&merged, &complete).is_empty());
    let bad = spec(json!({"features":{&owner:{"note":"TEST", "note_satisfies":["made.up"]}}}));
    assert!(model::plan(&a, &bad).is_err());
}
