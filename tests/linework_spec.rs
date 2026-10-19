//! Issue #76 item 6: author line-work intent without weakening independent drawing checks.
#[path = "common/mcp_schema.rs"]
mod mcp_schema;

use draftwright::{
    Session, api,
    items::Op,
    lint, mcp,
    model::{self, Spec},
    text::Font,
};
use serde_json::{Value, json};
use std::path::Path;

fn plate() -> (draftwright::haecceity::Part, model::Analysis) {
    model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step"),
    )
    .unwrap()
}
fn spec(v: Value) -> Spec {
    serde_json::from_value(v).unwrap()
}
fn font() -> Font {
    Font::new(api::MONO.to_vec()).unwrap()
}
fn view<'a>(p: &'a model::Plan, name: &str) -> &'a model::PlannedView {
    p.views
        .iter()
        .find(|v| v.id == format!("v:{name}"))
        .unwrap()
}
fn geometry(
    part: &draftwright::haecceity::Part,
    a: &model::Analysis,
    p: &model::Plan,
    cache: &mut model::Projections,
) -> model::GeometryLayer {
    model::render_geometry(part, a, p, cache).unwrap()
}

#[test]
fn hidden_precedence_defaults_and_cache_reuse() {
    let (part, a) = plate();
    let auto = model::plan(&a, &Spec::default()).unwrap();
    let mut cache = model::Projections::default();
    let original = geometry(&part, &a, &auto, &mut cache);
    assert!(original.views.iter().any(|(_, g)| !g.hidden.is_empty()));
    assert!(
        auto.views
            .iter()
            .all(|v| v.from_spec.is_empty() && v.line_style.is_empty())
    );
    let s = spec(json!({"sheet":{"hidden":false},"views":{"front":{"hidden":true}}}));
    let p = model::plan(&a, &s).unwrap();
    let changed = geometry(&part, &a, &p, &mut cache);
    for (name, g) in &changed.views {
        let before = &original.views.iter().find(|(id, _)| id == name).unwrap().1;
        assert_eq!(
            serde_json::to_value(&g.visible).unwrap(),
            serde_json::to_value(&before.visible).unwrap()
        );
        if name == "v:front" {
            assert_eq!(g.hidden.len(), before.hidden.len());
            assert_eq!(view(&p, "front").from_spec, ["views/front/hidden"]);
        } else {
            assert!(g.hidden.is_empty(), "{name}");
            assert_eq!(
                p.views.iter().find(|v| &v.id == name).unwrap().from_spec,
                ["sheet/hidden"]
            );
        }
    }
    let restored = geometry(&part, &a, &auto, &mut cache);
    assert_eq!(
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
    // Iso uses visible precedence too, while explicit view switches still win
    // over sheet defaults and invalidate the same projection cache.
    let original_iso = &original
        .views
        .iter()
        .find(|(id, _)| id == "v:iso")
        .unwrap()
        .1;
    // The orientation view has no hidden edges by default; the Spec can ask for them.
    assert!(original_iso.hidden.is_empty());
    let mut requested: Option<serde_json::Value> = None;
    for (sheet_hidden, iso_hidden) in [(false, true), (true, false), (false, true)] {
        let p = model::plan(
            &a,
            &spec(json!({
                "sheet": {"hidden": sheet_hidden},
                "views": {"iso": {"hidden": iso_hidden}}
            })),
        )
        .unwrap();
        let changed = geometry(&part, &a, &p, &mut cache);
        let iso = &changed
            .views
            .iter()
            .find(|(id, _)| id == "v:iso")
            .unwrap()
            .1;
        assert_eq!(view(&p, "iso").from_spec, ["views/iso/hidden"]);
        assert_eq!(
            serde_json::to_value(&iso.visible).unwrap(),
            serde_json::to_value(&original_iso.visible).unwrap()
        );
        if iso_hidden {
            assert!(!iso.hidden.is_empty());
            let drawn = serde_json::to_value(&iso.hidden).unwrap();
            assert_eq!(requested.get_or_insert_with(|| drawn.clone()), &drawn);
        } else {
            assert!(iso.hidden.is_empty());
        }
    }
}

#[test]
fn centres_are_independent_switches_and_lint_still_judges_output() {
    let (part, a) = plate();
    let auto = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        view(&auto, "plan")
            .line_work
            .iter()
            .any(|w| w.kind == "cross")
    );
    assert!(
        view(&auto, "front")
            .line_work
            .iter()
            .any(|w| w.kind == "axis")
    );
    let s = spec(
        json!({"sheet":{"centre_marks":false,"centre_lines":false},"views":{"front":{"centre_lines":true}}}),
    );
    let p = model::plan(&a, &s).unwrap();
    assert!(
        view(&p, "plan")
            .line_work
            .iter()
            .all(|w| w.kind != "cross" && w.kind != "axis" && w.kind != "pitch_circle")
    );
    assert!(view(&p, "front").line_work.iter().any(|w| w.kind == "axis"));
    let d = model::render(&part, &a, &p, &font()).unwrap();
    assert!(
        lint::lint(&a, &d)
            .iter()
            .any(|i| i.code == "feature_no_centermark")
    );
    let restored = model::plan(
        &a,
        &spec(json!({"sheet":{"centre_marks":false},"views":{"plan":{"centre_marks":true}}})),
    )
    .unwrap();
    assert!(
        view(&restored, "plan")
            .line_work
            .iter()
            .any(|w| w.kind == "cross")
    );
    // The isometric retains its automatic absence of centre furniture when unspecified.
    assert!(view(&auto, "iso").line_work.is_empty());
}

#[test]
fn sheet_default_reaches_later_sheets_and_view_still_wins() {
    let (_, a) = plate();
    let s = spec(
        json!({"sheets":{"max":2},"sheet":{"hidden":false,"centre_marks":false,"thread_depiction":false},"views":{"iso":{"sheet":2,"centre_lines":true}}}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    let iso = view(&p, "iso");
    assert_eq!(iso.sheet, 2);
    assert_eq!(iso.line_style.hidden, Some(false));
    assert_eq!(iso.line_style.centre_marks, Some(false));
    assert_eq!(iso.line_style.centre_lines, Some(true));
    assert_eq!(iso.line_style.thread_depiction, Some(false));
    assert!(iso.from_spec.contains(&"sheet/hidden".into()));
    assert!(iso.from_spec.contains(&"views/iso/centre_lines".into()));
    assert!(iso.line_work.iter().any(|w| w.kind == "axis"));
}

#[test]
fn bad_references_and_values_are_refused_with_a_fix() {
    let (_, a) = plate();
    let err = model::plan(&a, &spec(json!({"views":{"sid":{"hidden":false}}}))).unwrap_err();
    assert!(err.message.contains("sid") && err.message.contains("remove"));
    assert!(err.choices.contains(&"side".into()));
    for bad in [
        json!({"views":{"side":{"hidden":"false"}}}),
        json!({"sheet":{"centre_marks":3}}),
        json!({"views":{"side":{"x":20}}}),
        json!({"sheet":{"centre_mark":true}}),
    ] {
        assert!(serde_json::from_value::<Spec>(bad).is_err());
    }
    let s = spec(
        json!({"sheet":{"hidden":false,"centre_marks":true,"centre_lines":false,"thread_depiction":false,"knurl_depiction":true},"views":{"side":{"hidden":true}}}),
    );
    let saved = serde_json::to_vec(&s).unwrap();
    let restored: Spec = serde_json::from_slice(&saved).unwrap();
    assert_eq!(s, restored);
    assert_eq!(
        serde_json::to_value(model::plan(&a, &s).unwrap()).unwrap(),
        serde_json::to_value(model::plan(&a, &restored).unwrap()).unwrap()
    );
}

#[test]
fn session_ops_preview_replay_undo_and_options_follow_the_same_path() {
    let (part, a) = plate();
    let mut session = Session::new(part, a, Spec::default()).unwrap();
    let initial = session.drawing().svg();
    let mut state = session.state().clone();
    let ops = [
        Op::set("sheet/hidden", json!(false)),
        Op::set("views/plan/centre_marks", json!(false)),
    ];
    session.apply(&ops, false).unwrap();
    assert_eq!(session.drawing().svg(), initial);
    let delta = session.apply(&ops, true).unwrap();
    assert!(!delta.geometry.views.is_empty());
    assert!(
        session
            .svg_fragments(Some(&delta))
            .contains_key("furniture")
    );
    delta.apply_to(&mut state).unwrap();
    assert_eq!(
        serde_json::to_value(state).unwrap(),
        serde_json::to_value(session.state()).unwrap()
    );
    for (id, base) in [("v:plan", "views/plan"), ("sheet", "sheet")] {
        let options = session.options(id).unwrap();
        for field in [
            "hidden",
            "centre_marks",
            "centre_lines",
            "thread_depiction",
            "knurl_depiction",
        ] {
            assert_eq!(
                options.settable[&format!("{base}/{field}")]["choices"],
                json!([false, true])
            );
        }
    }
    let good = session.drawing().svg();
    assert!(
        session
            .apply(&[Op::set("views/no_such_view/hidden", json!(false))], true)
            .is_err()
    );
    assert_eq!(session.drawing().svg(), good);
    session.undo().unwrap();
    assert_eq!(session.drawing().svg(), initial);
    session.redo().unwrap();
    assert_eq!(session.drawing().svg(), good);
    session
        .apply(
            &[
                Op::remove("sheet/hidden"),
                Op::remove("views/plan/centre_marks"),
            ],
            true,
        )
        .unwrap();
    assert!(!session.drawing().layers[1].items.is_empty());
}

#[test]
fn mcp_switch_shapes_remain_strict_after_schema_compaction() {
    for scope in ["sheet", "views"] {
        for field in [
            "hidden",
            "centre_marks",
            "centre_lines",
            "thread_depiction",
            "knurl_depiction",
        ] {
            let value = |v: Value| {
                if scope == "sheet" {
                    json!({"part_ref":"part","spec":{"sheet":{field:v}}})
                } else {
                    json!({"part_ref":"part","spec":{"views":{"side":{field:v}}}})
                }
            };
            let schema = mcp_schema::with_full_spec("get_spec", false);
            assert!(mcp::validate(&schema, &value(json!(true))).is_ok());
            assert!(mcp::validate(&schema, &value(json!("true"))).is_err());
        }
    }
    let schema = mcp_schema::with_full_spec("get_spec", false);
    assert!(mcp::validate(&schema, &json!({"part_ref":3})).is_err());
    assert!(mcp::validate(&schema, &json!({"part_ref":"part","bogus":true})).is_err());
    assert!(
        mcp::validate(
            &schema,
            &json!({"part_ref":"part","spec":{"sheet":{"bogus":true}}})
        )
        .is_err()
    );
}
