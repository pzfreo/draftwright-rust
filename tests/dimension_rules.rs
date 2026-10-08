//! The drafting rules the dimension planner ports, as scenarios: a part and its recognition in,
//! a property of the plan (or its problems, or the drawing) out. Each test names the catalogue
//! rules it checks (`docs/rules/`).

use std::path::{Path, PathBuf};

use draftwright::dims::{self, PlannedDimension};
use draftwright::model::{self, Analysis, DimensionSource, DimensionSpec, Plan, Spec};
use draftwright::text::Font;
use serde_json::{Value, json};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

fn recognition(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(fixture(&format!("recognition/{name}.json"))).unwrap(),
    )
    .unwrap()
}

/// The part *name* with *json* as its recognition.
fn analysed_with(name: &str, json: &Value) -> (haecceity::Part, Analysis) {
    let (part, mut a) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
    let (r, features) =
        draftwright::bridge::read(&part, &a.binding.source_sha256, &json.to_string()).unwrap();
    a.recognition = Some(r);
    a.features = features;
    (part, a)
}

fn planned(name: &str, json: &Value, spec: &Spec) -> Plan {
    model::plan(&analysed_with(name, json).1, spec).unwrap()
}

fn find<'a>(plan: &'a Plan, id: &str) -> &'a PlannedDimension {
    plan.dimensions
        .iter()
        .find(|d| d.id == id)
        .unwrap_or_else(|| {
            panic!(
                "no {id}; the plan has {:?}",
                plan.dimensions.iter().map(|d| &d.id).collect::<Vec<_>>()
            )
        })
}

fn labels(plan: &Plan, kind: &str) -> Vec<String> {
    let mut l: Vec<String> = plan
        .dimensions
        .iter()
        .filter(|d| d.kind == kind)
        .map(|d| d.label.clone())
        .collect();
    l.sort();
    l
}

/// draftwright's `_fmt`, case for case (Python's own results).
#[test]
fn numbers_print_as_draftwright_prints_them() {
    for (v, s) in [
        (12.9898, "13.0"),
        (5.0, "5"),
        (0.25, "0.2"),
        (4.45, "4.5"),
        (72.39, "72.4"),
        (-0.0, "0"),
        (-0.04, "-0.0"),
        (276.25, "276.2"),
        (247.649_999_999_9, "247.7"),
        (5.000_000_1, "5"),
    ] {
        assert_eq!(dims::fmt(v), s, "{v}");
    }
}

/// ADR1-6: a hole's view follows from its axis by one rule.
#[test]
fn a_holes_view_follows_from_its_axis() {
    assert_eq!(
        [dims::end_on(0), dims::end_on(1), dims::end_on(2)],
        ["side", "front", "plan"]
    );
    let plan = planned("plate", &recognition("plate"), &Spec::default());
    for d in plan
        .dimensions
        .iter()
        .filter(|d| !d.id.starts_with("d:envelope"))
    {
        assert_eq!(d.view, "v:plan", "{}: the plate's holes run along z", d.id);
    }
}

/// ARCH0008-2 (identical features are counted), ISSUE-1143-c (a grid has both pitches and is
/// located), ADR4-7 (THRU is the feature's fact; a through hole states no depth),
/// LINT-envelope_coverage-overall_extents and ARCH0016-5 (each dimension in exactly one view).
#[test]
fn a_grid_of_holes_is_called_out_once_located_and_spaced() {
    let plan = planned("plate", &recognition("plate"), &Spec::default());
    let c = find(&plan, "d:f:hole_pattern:1/bore.diameter@plan");
    assert_eq!(c.label, "4× ⌀6 THRU (2×2)");
    assert_eq!(c.measurements.len(), 4, "one bore diameter per hole");
    assert!(
        c.measurements
            .iter()
            .all(|m| m.parameter == "bore.diameter")
    );
    assert_eq!(
        find(&plan, "d:f:hole_pattern:1/grid_pitch.length.col@plan").label,
        "40"
    );
    assert_eq!(
        find(&plan, "d:f:hole_pattern:1/grid_pitch.length.row@plan").label,
        "20"
    );
    // The grid's centre is the part's centre: 30 and 20 from the left and front faces.
    assert_eq!(
        find(&plan, "d:f:hole_pattern:1/location.length.x@plan").label,
        "30"
    );
    assert_eq!(
        find(&plan, "d:f:hole_pattern:1/location.length.y@plan").label,
        "20"
    );
    for (id, label) in [
        ("d:envelope/width.length@front", "60"),
        ("d:envelope/depth.length@side", "40"),
        ("d:envelope/height.length@front", "5"),
    ] {
        assert_eq!(find(&plan, id).label, label);
    }
    let mut ids: Vec<&str> = plan.dimensions.iter().map(|d| d.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), plan.dimensions.len(), "IDs are unique");
    for d in &plan.dimensions {
        assert!(d.id.ends_with(&format!("@{}", &d.view[2..])), "{}", d.id);
        assert!(d.default && !d.basis.is_empty(), "{}", d.id);
        assert!(d.placed.is_some(), "{}", d.id);
    }
}

/// ADR4-7, ISSUE-1229-a and ISSUE-1517-a: a blind hole states its depth and never reads THRU;
/// holes group only when every machining term matches, so a through hole is not counted with
/// blind ones.
#[test]
fn blind_and_through_holes_are_separate_requirements() {
    let mut json = recognition("plate");
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        f["record"]["bottom"] = "flat".into();
    }
    let plan = planned("plate", &json, &Spec::default());
    assert_eq!(labels(&plan, "callout"), ["4× ⌀6 ↧ 5 (2×2)"]);
    let c = &plan
        .dimensions
        .iter()
        .find(|d| d.kind == "callout")
        .unwrap();
    assert_eq!(
        c.measurements
            .iter()
            .filter(|m| m.parameter == "bore.depth")
            .count(),
        4
    );

    // An unknown bottom is not a through hole either (string_post's cross hole is one).
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        f["record"]["bottom"] = "unknown".into();
    }
    let plan = planned("plate", &json, &Spec::default());
    assert_eq!(labels(&plan, "callout"), ["4× ⌀6 ↧ 5 (2×2)"]);

    json["quiddity"]["features"][0]["record"]["bottom"] = "through".into();
    let plan = planned("plate", &json, &Spec::default());
    assert_eq!(labels(&plan, "callout"), ["3× ⌀6 ↧ 5", "⌀6 THRU"]);
    // Dimensioned singly, every hole is located in both directions (LINT-hole_coverage-location).
    let located: Vec<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("/location."))
        .flat_map(|d| d.measurements.iter().map(|m| m.feature.as_str()))
        .collect();
    for h in 1..=4 {
        assert_eq!(
            located
                .iter()
                .filter(|f| **f == format!("f:hole:{h}"))
                .count(),
            2,
            "f:hole:{h}"
        );
    }
}

/// ISSUE-1596-a: a bolt circle of fewer than five holes is no evidence of a circle; its holes are
/// still drawn, counted and located.
#[test]
fn a_four_hole_bolt_circle_is_not_trusted() {
    let mut json = recognition("plate");
    let p = &mut json["quiddity"]["derived"]["hole_patterns"][0];
    let o = p.as_object_mut().unwrap();
    o.remove("width");
    o.remove("height");
    o.insert("diameter".into(), json!(44.721));
    let (_, a) = analysed_with("plate", &json);
    assert_eq!(
        a.recognition.as_ref().unwrap().patterns[0].arrangement,
        "bolt_circle"
    );
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(labels(&plan, "callout"), ["4× ⌀6 THRU"]);
    assert!(!plan.dimensions.iter().any(|d| d.label.contains("BC")));
    let c = plan
        .dimensions
        .iter()
        .find(|d| d.kind == "callout")
        .unwrap();
    assert!(c.basis.contains("fewer than five"), "{}", c.basis);
    assert_eq!(
        plan.dimensions
            .iter()
            .filter(|d| d.id.contains("/location."))
            .count(),
        4,
        "x at 10 and 50, y at 10 and 30"
    );
}

/// R1 invariant 3 (ADR4-9, ADR5-7, ADR2-12): an approved item that cannot be drawn stays in the
/// plan and is reported against its ID, naming the view that would carry it.
#[test]
fn nothing_approved_vanishes_when_its_view_is_omitted() {
    let mut spec = Spec::default();
    spec.views.insert("plan".into(), Default::default());
    spec.views.get_mut("plan").unwrap().omit = true;
    let plan = planned("plate", &recognition("plate"), &spec);
    let unplaced: Vec<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_none())
        .map(|d| d.id.as_str())
        .collect();
    assert!(unplaced.contains(&"d:f:hole_pattern:1/bore.diameter@plan"));
    for id in &unplaced {
        assert!(
            plan.problems
                .iter()
                .any(|p| p.code == "view_plan_incomplete" && p.subjects.iter().any(|s| s == id)),
            "{id} is reported"
        );
    }
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "view_plan_incomplete")
        .unwrap();
    assert_eq!(p.severity, "error");
    assert!(p.message.contains("plan view"), "{}", p.message);
}

/// ADR4-5: both sources of dimension intent. `auto` draws the defaults less what the spec omits;
/// `authored` draws exactly what it lists; each omission says it was the author's (ADR4-8); a
/// dimension the part does not have is refused with the nearest IDs (R1 invariant 9).
#[test]
fn both_dimension_sources_and_their_validation() {
    let json = recognition("plate");
    let callout = "d:f:hole_pattern:1/bore.diameter@plan";
    let width = "d:envelope/width.length@front";

    // auto, through an edit: the ID's `/` is written `~1`.
    let spec = model::edit(
        &Spec::default(),
        &json!([{"op": "set", "path": format!("dimensions/{}/omit", callout.replace('/', "~1")), "value": true}]),
    )
    .unwrap();
    assert!(spec.dimensions[callout].omit);
    let plan = planned("plate", &json, &spec);
    assert!(!plan.dimensions.iter().any(|d| d.id == callout));
    assert_eq!(plan.omitted.len(), 1);
    assert!(plan.omitted[0].basis.starts_with("authored"));

    // authored: exactly the listed ones.
    let mut spec = Spec {
        dimension_source: DimensionSource::Authored,
        ..Spec::default()
    };
    spec.dimensions
        .insert(width.into(), DimensionSpec::default());
    let plan = planned("plate", &json, &spec);
    assert_eq!(plan.dimensions.len(), 1);
    assert_eq!(plan.dimensions[0].id, width);
    assert!(!plan.dimensions[0].default);
    assert!(plan.omitted.iter().any(|o| o.id == callout));
    let round: Spec = serde_json::from_value(serde_json::to_value(&spec).unwrap()).unwrap();
    assert_eq!(round, spec);
    assert!(
        serde_json::to_string(&spec)
            .unwrap()
            .contains("\"dimension_source\":\"authored\"")
    );

    // An omission inside an authored set is refused: leaving it out is how.
    spec.dimensions.get_mut(width).unwrap().omit = true;
    let (_, a) = analysed_with("plate", &json);
    let e = model::plan(&a, &spec).unwrap_err();
    assert!(e.message.contains("authored set"), "{}", e.message);

    // An unknown ID: the nearest are offered.
    let mut spec = Spec::default();
    spec.dimensions.insert(
        "d:envelope/width.length@plan".into(),
        DimensionSpec::default(),
    );
    let e = model::plan(&a, &spec).unwrap_err();
    assert!(e.choices.iter().any(|c| c == width), "{:?}", e.choices);

    // A source that does not exist is refused by the schema, naming the two.
    let e = serde_json::from_value::<Spec>(json!({"dimension_source": "both"})).unwrap_err();
    assert!(
        e.to_string().contains("auto") && e.to_string().contains("authored"),
        "{e}"
    );
}

/// A part turned whole states its diameter (with ø) and its length, not a width and depth that
/// are both its diameter.
#[test]
fn a_turned_part_states_its_diameter_and_length() {
    let plan = planned("turned", &recognition("turned"), &Spec::default());
    let env: Vec<(&str, &str)> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.starts_with("d:envelope"))
        .map(|d| (d.id.as_str(), d.label.as_str()))
        .collect();
    assert_eq!(
        env,
        [
            ("d:envelope/od.diameter@front", "ø30"),
            ("d:envelope/height.length@front", "70")
        ]
    );
}

/// When the preferred view is omitted, a turned part's diameter moves to the other profile view
/// and is drawn across the radial axis that view shows; the plan never says it is drawable and
/// undrawable at once (R1 invariant 3).
#[test]
fn a_turned_parts_diameter_follows_its_view() {
    let spec: Spec = serde_json::from_value(json!({"views": {"front": {"omit": true}}})).unwrap();
    let plan = planned("turned", &recognition("turned"), &spec);
    for id in [
        "d:envelope/od.diameter@side",
        "d:envelope/height.length@side",
    ] {
        let d = find(&plan, id);
        assert!(d.placed.is_some(), "{id} is drawn");
        assert_eq!(d.view, "v:side");
        assert!(d.basis.contains("the front view is omitted"), "{}", d.basis);
    }
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "view_plan_incomplete"),
        "{:?}",
        plan.problems
    );
    // With both profile views omitted, the message names both and why.
    let spec: Spec =
        serde_json::from_value(json!({"views": {"front": {"omit": true}, "side": {"omit": true}}}))
            .unwrap();
    let plan = planned("turned", &recognition("turned"), &spec);
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "view_plan_incomplete")
        .expect("reported");
    assert!(
        p.subjects
            .iter()
            .any(|s| s == "d:envelope/od.diameter@front"),
        "{:?}",
        p.subjects
    );
    assert!(
        p.message.contains("front or side view") && p.message.contains("omitted"),
        "{}",
        p.message
    );
}

/// Review M8, and ARCH0017-1 in part: a recognised feature of a kind no planner dimensions yet is
/// reported once, as info, in the Analysis's feature order; a record type draftwright itself
/// leaves undimensioned (here a freeform surface) is no gap in the port and is not reported, and
/// the holes the port plans are not reported either.
#[test]
fn features_with_no_ported_dimensions_are_reported() {
    let not_ported = |plan: &Plan| -> Vec<String> {
        plan.problems
            .iter()
            .filter(|p| p.code == "dimensions_not_ported")
            .map(|p| {
                assert_eq!(p.severity, "info");
                assert_eq!(p.id, format!("p:dimensions_not_ported:{}", p.subjects[0]));
                p.subjects.concat()
            })
            .collect()
    };
    let (_, mut a) = analysed_with("turned", &recognition("turned"));
    let kinds: Vec<&str> = a.features.iter().map(|f| f.kind.as_str()).collect();
    assert!(kinds.contains(&"fillet") && kinds.contains(&"turned_step"));
    assert!(!kinds.contains(&"hole"));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let ids: Vec<String> = a.features.iter().map(|f| f.id.clone()).collect();
    assert_eq!(not_ported(&plan), ids);
    let step = plan
        .problems
        .iter()
        .find(|p| p.subjects[0].starts_with("f:turned_step:"))
        .unwrap();
    assert!(
        step.message
            .ends_with("no dimensions for turned_step are ported yet."),
        "{}",
        step.message
    );

    a.features[0].fingerprint.subtype = "FreeformSurface".into();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(not_ported(&plan), ids[1..]);

    let plate = planned("plate", &recognition("plate"), &Spec::default());
    assert!(not_ported(&plate).is_empty(), "{:?}", plate.problems);
}

/// An ID names the view that carries its dimension, so omitting that view renames it; a spec
/// still keyed by the old ID is refused, and the refusal names the new ID first.
#[test]
fn a_dimension_renamed_by_an_omitted_view_is_named_in_the_refusal() {
    let spec: Spec = serde_json::from_value(json!({
        "views": {"front": {"omit": true}},
        "dimensions": {"d:envelope/width.length@front": {"omit": true}}
    }))
    .unwrap();
    let (_, a) = analysed_with("plate", &recognition("plate"));
    let e = model::plan(&a, &spec).unwrap_err();
    assert_eq!(
        e.choices[0], "d:envelope/width.length@plan",
        "{:?}",
        e.choices
    );
    assert!(
        e.message.contains("d:envelope/width.length@plan")
            && e.message.contains("front view is omitted"),
        "{}",
        e.message
    );
}

/// A countersunk hole's callout states its countersink as quiddity records it (`major_diameter`,
/// `included_angle`: `countersinks.py`), in draftwright's grammar `⌵ ⌀D × A°`, and claims both
/// sizes for every hole.
#[test]
fn a_countersunk_hole_states_its_countersink() {
    let mut json = recognition("plate");
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        let r = &mut f["record"];
        r["csink"] = json!({
            "axis": r["axis"].clone(), "location": r["location"].clone(),
            "major_diameter": 12.0, "drill_diameter": 6.0, "included_angle": 90.0, "depth": 3.0,
        });
    }
    let plan = planned("plate", &json, &Spec::default());
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "feature_not_dimensioned"),
        "{:?}",
        plan.problems
    );
    let c = find(&plan, "d:f:hole_pattern:1/bore.diameter@plan");
    assert_eq!(c.label, "4× ⌀6 THRU ⌵ ⌀12 × 90° (2×2)");
    for (parameter, value) in [("countersink.diameter", 12.0), ("countersink.angle", 90.0)] {
        let m: Vec<f64> = c
            .measurements
            .iter()
            .filter(|m| m.parameter == parameter)
            .map(|m| m.value)
            .collect();
        assert_eq!(m, [value; 4], "{parameter}");
    }
}

/// A hole with both a counterbore and a spotface (quiddity finds both when a shallow facing cut
/// and a deeper step lie between the opening and the bore) states both, the counterbore first as
/// in draftwright's segment order, and claims all four sizes; lint then finds nothing missing.
/// draftwright itself prints only the counterbore (its one recess slot), which its own lint
/// would report: the port departs from it here (accuracy over parity). No captured part has
/// such a hole, so the plate's holes are given both.
#[test]
fn a_hole_with_a_counterbore_and_a_spotface_states_both() {
    let mut json = recognition("plate");
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        f["record"]["spotface"] = json!({ "diameter": 20.0, "depth": 1.0 });
        f["record"]["cbore"] = json!({ "diameter": 11.0, "depth": 6.4 });
    }
    let (part, analysis) = analysed_with("plate", &json);
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    let c = find(&plan, "d:f:hole_pattern:1/bore.diameter@plan");
    assert_eq!(c.label, "4× ⌀6 THRU ⌴ ⌀11 ↧ 6.4 ⌴ ⌀20 ↧ 1 (2×2)");
    for (parameter, value) in [
        ("counterbore.diameter", 11.0),
        ("counterbore.depth", 6.4),
        ("spotface.diameter", 20.0),
        ("spotface.depth", 1.0),
    ] {
        let m: Vec<f64> = c
            .measurements
            .iter()
            .filter(|m| m.parameter == parameter)
            .map(|m| m.value)
            .collect();
        assert_eq!(m, [value; 4], "{parameter}");
    }
    let font =
        Font::new(std::fs::read(fixture("../../fonts/IBMPlexMono-Regular.ttf")).unwrap()).unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let found: Vec<String> = draftwright::lint::lint(&analysis, &drawing)
        .iter()
        .map(|i| format!("{} {:?}", i.code, i.requirements))
        .collect();
    assert!(found.is_empty(), "{found:#?}");
}

/// ADR4-7: a through hole's depth is not part of its callout, so a through hole whose record
/// gives none is still called out; a blind hole without a depth cannot be, and says why.
#[test]
fn only_a_blind_hole_needs_its_depth() {
    let mut json = recognition("plate");
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        f["record"].as_object_mut().unwrap().remove("depth");
    }
    let plan = planned("plate", &json, &Spec::default());
    assert_eq!(labels(&plan, "callout"), ["4× ⌀6 THRU (2×2)"]);
    json["quiddity"]["features"][0]["record"]["bottom"] = "flat".into();
    let plan = planned("plate", &json, &Spec::default());
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "feature_not_dimensioned")
        .unwrap_or_else(|| panic!("reported: {:?}", plan.problems));
    assert_eq!(p.subjects, ["f:hole:1"]);
    assert!(
        p.message.contains("blind and has no depth"),
        "{}",
        p.message
    );
    assert_eq!(labels(&plan, "callout"), ["3× ⌀6 THRU"]);
}

/// A hole whose bottom recognition did not establish is called out blind to its depth, as
/// draftwright does, and says so (accuracy over parity: no silent claim).
#[test]
fn an_unestablished_hole_bottom_is_reported() {
    let mut json = recognition("plate");
    json["quiddity"]["features"][0]["record"]["bottom"] = "unknown".into();
    let plan = planned("plate", &json, &Spec::default());
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "hole_bottom_unknown")
        .unwrap_or_else(|| panic!("reported: {:?}", plan.problems));
    assert_eq!(p.severity, "info");
    assert!(
        !planned("plate", &recognition("plate"), &Spec::default())
            .problems
            .iter()
            .any(|p| p.code == "hole_bottom_unknown")
    );
}

/// R1 invariant 2 (ADR4-4) and ARCH0019-1: the renderer draws the plan's own label text and
/// composes none; every dimension run it writes is a plan item's terms, joined by spaces.
#[test]
fn renderers_draw_only_from_the_plan() {
    let (part, a) = analysed_with("plate", &recognition("plate"));
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let mut plan = model::plan(&a, &Spec::default()).unwrap();
    let runs = |plan: &Plan| -> Vec<String> {
        model::render(&part, &a, plan, &font)
            .unwrap()
            .text
            .iter()
            .filter(|r| r.font == dims::FONT && r.size == dims::TEXT)
            .map(|r| r.text.clone())
            .collect()
    };
    let labels: Vec<String> = plan.dimensions.iter().map(|d| d.terms.join(" ")).collect();
    let before = runs(&plan);
    assert!(!before.is_empty());
    for r in &before {
        assert!(labels.contains(r), "{r:?} is not in the plan");
    }
    // Change the plan's text: the drawing follows it, with no number of its own.
    let d = plan
        .dimensions
        .iter_mut()
        .find(|d| d.id == "d:envelope/width.length@front")
        .unwrap();
    d.terms = vec!["WIDTH-FROM-PLAN".into()];
    let after = runs(&plan);
    assert!(after.contains(&"WIDTH-FROM-PLAN".to_string()));
    assert!(!after.contains(&"60".to_string()));
}

/// R1 invariant 7: the same documents give the same plan.
#[test]
fn dimension_planning_is_deterministic() {
    let json = recognition("plate");
    let a = serde_json::to_string(&planned("plate", &json, &Spec::default())).unwrap();
    let b = serde_json::to_string(&planned("plate", &json, &Spec::default())).unwrap();
    assert_eq!(a, b);
}
