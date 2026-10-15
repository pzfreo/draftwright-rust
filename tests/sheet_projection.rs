//! #13: a pinned sheet and scale raise `page_fit_uncertain` only when the views do not fit, and
//! then carry the nearest pair that does as a fix; `sheet/projection` arranges the views first
//! or third angle (ISO 5456-2) and draws the matching symbol.

use std::path::Path;

use draftwright::items::Op;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::session::Session;
use serde_json::json;

fn analysed(path: &str) -> Analysis {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    model::analyse_native(&root.join(path)).unwrap().1
}

/// The editor's sample plate (draftwright-editor's public/samples/plate.step), which #13 names.
fn editor_plate() -> Analysis {
    analysed("tests/fixtures/samples/editor-plate.step")
}

fn plate() -> Analysis {
    analysed("tests/fixtures/parts/plate.step")
}

fn spec(value: serde_json::Value) -> Spec {
    serde_json::from_value(value).unwrap()
}

fn planned(analysis: &Analysis, value: serde_json::Value) -> Plan {
    model::plan_recovered(analysis, &spec(value)).unwrap()
}

fn fit_problem(plan: &Plan) -> Option<&model::Problem> {
    plan.problems
        .iter()
        .find(|p| matches!(p.code.as_str(), "page_fit_uncertain" | "views_do_not_fit"))
}

fn at(plan: &Plan, id: &str) -> [f64; 2] {
    plan.views.iter().find(|v| v.id == id).unwrap().at
}

#[test]
fn the_editor_plate_pinned_a3_at_2_to_1_raises_no_page_fit_uncertain() {
    let plan = planned(
        &editor_plate(),
        json!({"sheet": {"size": "A3", "scale": "2:1"}}),
    );
    assert_eq!(
        (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
        ("A3", "2:1")
    );
    assert!(fit_problem(&plan).is_none(), "{:?}", fit_problem(&plan));
}

/// Pinning the pair the engine chooses itself draws what it draws, and raises nothing.
#[test]
fn pinning_the_automatic_pair_draws_the_automatic_sheet() {
    let analysis = plate();
    let automatic = planned(&analysis, json!({}));
    let pinned = planned(
        &analysis,
        json!({"sheet": {"size": automatic.sheet.size, "scale": automatic.sheet.scale}}),
    );
    assert!(fit_problem(&pinned).is_none(), "{:?}", fit_problem(&pinned));
    for view in &automatic.views {
        assert_eq!(view.at, at(&pinned, &view.id), "{}", view.id);
        assert_eq!(
            view.area,
            pinned.views.iter().find(|v| v.id == view.id).unwrap().area
        );
    }
}

#[test]
fn a_pin_that_cannot_fit_carries_a_fix_that_fits() {
    for analysis in [editor_plate(), plate()] {
        let asked = json!({"sheet": {"size": "A4", "scale": "10:1"}});
        let plan = planned(&analysis, asked.clone());
        let problem = fit_problem(&plan).expect("A4 at 10:1 does not hold the plate");
        assert_eq!(problem.code, "page_fit_uncertain");
        assert!(!problem.fix.is_empty(), "{problem:?}");
        assert!(
            problem
                .fix
                .iter()
                .all(|op| op.op == "set"
                    && ["sheet/size", "sheet/scale"].contains(&op.path.as_str())),
            "{:?}",
            problem.fix
        );
        let fixed =
            model::edit(&spec(asked), &serde_json::to_value(&problem.fix).unwrap()).unwrap();
        let again = model::plan_recovered(&analysis, &fixed).unwrap();
        assert!(fit_problem(&again).is_none(), "{:?}", fit_problem(&again));
        assert!(
            problem.message.contains(&format!(
                "{} at {} fits",
                again.sheet.size, again.sheet.scale
            )),
            "{}",
            problem.message
        );
    }
}

#[test]
fn first_angle_swaps_the_views_and_the_symbol() {
    let analysis = plate();
    for size in ["A3", "A4"] {
        let pin = |projection: &str| {
            planned(
                &analysis,
                json!({"sheet": {"size": size, "scale": "1:1", "projection": projection}}),
            )
        };
        let (third, first) = (pin("third"), pin("first"));
        // Third angle: the plan above the front, the right side to its right.
        assert!(at(&third, "v:plan")[1] > at(&third, "v:front")[1]);
        assert!(at(&third, "v:side")[0] > at(&third, "v:front")[0]);
        // First angle: the plan below, the right side to the left; still projected in line.
        assert!(at(&first, "v:plan")[1] < at(&first, "v:front")[1], "{size}");
        assert!(at(&first, "v:side")[0] < at(&first, "v:front")[0], "{size}");
        assert_eq!(at(&first, "v:plan")[0], at(&first, "v:front")[0]);
        assert_eq!(at(&first, "v:side")[1], at(&first, "v:front")[1]);
        assert!(
            fit_problem(&first).is_none(),
            "{size}: {:?}",
            fit_problem(&first)
        );
        let symbol = |p: &Plan| p.sheet.projection_symbol.clone().unwrap();
        assert_eq!(symbol(&third).method, "third-angle");
        assert_eq!(symbol(&first).method, "first-angle");
        // The cone's two end lines: third angle's large end left, first angle's right.
        let ends = |p: &Plan| {
            let s = symbol(p);
            let len = |i: usize| match &s.curves[i] {
                draftwright::output::Curve::Line(a, b) => (a[1] - b[1]).abs(),
                _ => panic!("an end line"),
            };
            (len(0), len(1))
        };
        assert!(ends(&third).0 > ends(&third).1);
        assert!(ends(&first).0 < ends(&first).1);
    }
}

/// `third` is the default spelled out: the same plan apart from the spec it echoes.
#[test]
fn third_angle_is_the_default() {
    let analysis = plate();
    let default = planned(&analysis, json!({}));
    let mut third = planned(&analysis, json!({"sheet": {"projection": "third"}}));
    third.spec = default.spec.clone();
    assert_eq!(
        serde_json::to_value(&default).unwrap(),
        serde_json::to_value(&third).unwrap()
    );
    // Refused when the spec is read; a spec built in code is refused by the plan, with choices.
    assert!(serde_json::from_value::<Spec>(json!({"sheet": {"projection": "second"}})).is_err());
    let mut second = Spec::default();
    second.sheet.projection = Some("second".into());
    let refused = model::plan_recovered(&analysis, &second).unwrap_err();
    assert_eq!(refused.choices, ["first", "third"]);
}

/// Editable like sheet/size and sheet/scale through the session API.
#[test]
fn the_session_offers_and_applies_the_projection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (part, analysis) =
        model::analyse_native(&root.join("tests/fixtures/parts/plate.step")).unwrap();
    let mut session = Session::new(part, analysis, Spec::default()).unwrap();
    let options = session.options("sheet").unwrap();
    assert_eq!(
        options.settable["sheet/projection"]["choices"],
        json!(["first", "third"])
    );
    session
        .apply(&[Op::set("sheet/projection", json!("first"))], true)
        .unwrap();
    assert_eq!(
        serde_json::to_value(session.spec()).unwrap()["sheet"]["projection"],
        "first"
    );
    let plan = session.plan();
    assert!(at(plan, "v:plan")[1] < at(plan, "v:front")[1]);
    assert_eq!(
        plan.sheet.projection_symbol.as_ref().unwrap().method,
        "first-angle"
    );
}
