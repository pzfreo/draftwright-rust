//! Detail ownership and the default turned inspection baseline, including saved spec IDs.
#[path = "common/corpus.rs"]
mod corpus;
use draftwright::model::{self, Analysis, Spec};
use serde_json::json;
use std::path::Path;

fn string_post() -> Option<Analysis> {
    let dir = corpus::dir()?;
    Some(
        model::analyse_native(&dir.join("gramel/string_post.step"))
            .unwrap()
            .1,
    )
}

#[test]
fn string_post_uses_a4_with_an_enlarged_head_and_sheet_scale_iso() {
    let Some(a) = string_post() else { return };
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(plan.sheet.size, "A4");
    assert_eq!(plan.sheet.scale, "2:1");
    let v = plan
        .views
        .iter()
        .find(|v| v.detail.is_some())
        .expect("head detail");
    let d = v.detail.as_ref().unwrap();
    assert_eq!(d.kind, "turned-head");
    assert_eq!(v.scale, 10.0);
    assert!(d.bounds[0][2] <= 13.8 && d.bounds[1][2] >= 14.8);
    assert!(
        plan.dimensions.iter().any(|d| d.view == v.id
            && d.id == "d:f:turned_step:3/step.length"
            && d.placed.is_some())
    );
    assert_eq!(
        plan.views.iter().find(|v| v.id == "v:iso").unwrap().scale,
        plan.sheet.scale_factor
    );
    let dir = corpus::dir().unwrap();
    let (part, _) = model::analyse_native(&dir.join("gramel/string_post.step")).unwrap();
    let font = draftwright::text::Font::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fonts")
            .join(draftwright::dims::FONT),
    )
    .unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    assert!(drawing.text.iter().any(|t| t.text == "DETAIL B (10:1)"));
    assert!(
        drawing
            .sheet
            .items
            .iter()
            .any(|i| i.name == "detail_marker_B")
    );
    assert!(
        draftwright::lint::structural::check(&drawing)
            .iter()
            .all(|i| !matches!(
                i.code.as_str(),
                "derived_view_identifier_reused" | "detail_caption_clearance"
            ))
    );
}

fn crosses(line: [[f64; 2]; 2], b: [f64; 4]) -> bool {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        let d = line[1][k] - line[0][k];
        if d.abs() < 1e-9 {
            if line[0][k] <= b[k] + 1e-6 || line[0][k] >= b[k + 2] - 1e-6 {
                return false;
            }
        } else {
            let p = (b[k] - line[0][k]) / d;
            let q = (b[k + 2] - line[0][k]) / d;
            lo = lo.max(p.min(q));
            hi = hi.min(p.max(q));
        }
    }
    hi > lo + 1e-6
}

#[test]
fn string_post_heights_share_one_origin_and_extension_lines_clear_text() {
    let Some(a) = string_post() else { return };
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let heights: Vec<_> = p
        .dimensions
        .iter()
        .filter(|d| d.view == "v:front" && d.kind == "linear")
        .filter_map(|d| Some((d, d.placed.as_ref()?)))
        .filter(|(_, p)| p.rotation.abs() == 90.0)
        .collect();
    assert!(heights.len() >= 3);
    let base = heights[0]
        .1
        .arrows
        .iter()
        .map(|a| a[0][1])
        .find(|y| {
            heights
                .iter()
                .all(|(_, q)| q.arrows.iter().any(|a| (a[0][1] - y).abs() < 1e-6))
        })
        .expect("one common origin");
    for (d, q) in &heights {
        assert!(
            q.arrows.iter().any(|a| (a[0][1] - base).abs() < 1e-6),
            "{} has another origin",
            d.id
        );
        assert!(
            !d.label.starts_with('('),
            "{} is spuriously reference",
            d.id
        );
        for (other, r) in &heights {
            if d.id != other.id {
                for line in q.lines.iter().take(2) {
                    assert!(
                        !crosses(*line, r.label_box),
                        "{} crosses {}",
                        d.id,
                        other.id
                    );
                }
            }
        }
    }
    assert!(
        heights
            .iter()
            .any(|(d, _)| d.id.contains("location.length.z") && (d.value - 3.75).abs() < 1e-6)
    );
}

#[test]
fn an_authored_detail_is_honoured_and_keeps_its_exact_scale() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (_, a) = model::analyse_native(&root.join("tests/fixtures/parts/turned.step")).unwrap();
    let feature = a.features.iter().find(|f| f.kind == "turned_step").unwrap();
    let spec:Spec=serde_json::from_value(json!({"views":{"detail_c":{"detail":{"around":feature.id,"scale_factor":3.0}}},"sheet":{"size":"A3","scale":"1:2"}})).unwrap();
    let p = model::plan(&a, &spec).unwrap();
    let v = p.views.iter().find(|v| v.id == "v:detail_c").unwrap();
    assert_eq!(v.scale, 1.5);
    assert!(v.detail.as_ref().unwrap().authored);
}

#[test]
fn a_plain_part_has_no_stray_detail() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (_, a) =
        model::analyse_native(&root.join("tests/fixtures/parts/mounting_plate.step")).unwrap();
    assert!(
        model::plan_recovered(&a, &Spec::default())
            .unwrap()
            .views
            .iter()
            .all(|v| v.detail.is_none())
    );
    let Some(a) = string_post() else { return };
    assert!(
        model::plan_recovered(&a, &Spec::default())
            .unwrap()
            .views
            .iter()
            .any(|v| v.detail.is_some())
    );
}

#[test]
fn saved_chain_ids_and_tolerances_still_resolve() {
    let Some(a) = string_post() else { return };
    let spec:Spec=serde_json::from_value(json!({"dimensions":{"d:f:turned_step:1/step.length":{"tolerance":"±0.05"},"d:f:turned_step:3/step.length":{"tolerance":"±0.02"}}})).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:f:turned_step:3/step.length" && d.view.starts_with("v:detail_"))
    );
    for (id, tol) in [
        ("d:f:turned_step:1/step.length", "±0.05"),
        ("d:f:turned_step:3/step.length", "±0.02"),
    ] {
        assert!(
            p.dimensions
                .iter()
                .any(|d| d.id == id && d.label.contains(tol) && d.placed.is_some())
        );
    }
}

#[test]
fn the_saved_string_post_spec_keeps_every_tolerance_without_crossed_text() {
    let Some(a) = string_post() else { return };
    let saved: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/known_single_sheet_specs.json")).unwrap();
    // string_post.step:spec:1c29f39db68e8b009a719d83ec1389d96a1e350a2d6b4bc142069453c99e3569
    let spec: Spec = serde_json::from_value(saved[&a.binding.source_sha256][1].clone()).unwrap();
    assert_eq!(spec.dimensions.len(), 14);
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "2:1")
    );
    assert!(
        p.views
            .iter()
            .any(|v| v.detail.is_some() && v.scale == 10.0)
    );
    for (id, spec) in &spec.dimensions {
        let tol = spec.tolerance.as_ref().unwrap();
        assert!(
            p.dimensions
                .iter()
                .any(|d| d.id == *id && d.placed.is_some() && d.label.contains(tol)),
            "{id}: {tol}"
        );
    }
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_overlap" | "annotation_ink_overlap" | "annotation_off_sheet"
    )));
}

#[test]
fn crowded_prismatic_rungs_and_y_chains_use_their_own_camera_and_scale() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, kind, parent) in [
        ("crowded_prism", "prismatic-steps", "v:front"),
        ("unequal_prism", "prismatic-steps", "v:front"),
        ("y_chain", "y-turned-chain", "v:side"),
    ] {
        let (_, a) =
            model::analyse_native(&root.join(format!("tests/fixtures/details/{name}.step.gz")))
                .unwrap();
        let spec: Spec =
            serde_json::from_value(json!({"sheet":{"size":"A3","scale":"1:1"}})).unwrap();
        let p = model::plan(&a, &spec).unwrap();
        let v = p
            .views
            .iter()
            .find(|v| v.detail.as_ref().is_some_and(|d| d.kind == kind))
            .unwrap_or_else(|| panic!("{name}: no {kind}: {:?}", p.problems));
        let d = v.detail.as_ref().unwrap();
        assert_eq!(d.source_view, parent);
        assert!(v.scale > p.sheet.scale_factor);
        assert!(!d.dimensions.is_empty());
        for id in &d.dimensions {
            assert!(
                p.dimensions
                    .iter()
                    .any(|dim| dim.id == *id && dim.view == v.id)
            );
        }
    }
}

#[test]
fn an_authored_a_reserves_its_letter_before_the_automatic_section() {
    let Some(a) = string_post() else { return };
    let s:Spec=serde_json::from_value(json!({"sheet":{"size":"A3","scale":"2:1"},"views":{"detail_a":{"detail":{"around":"f:turned_step:3","scale_factor":5}}}})).unwrap();
    let p = model::plan(&a, &s).unwrap();
    assert!(p.views.iter().any(|v| v.id == "v:detail_a"));
    assert!(
        p.views
            .iter()
            .any(|v| v.id == "v:section_bb" && v.cut.as_ref().is_some_and(|c| c.label == "B"))
    );
}

#[test]
fn omitting_an_automatic_detail_does_not_rename_it() {
    let Some(a) = string_post() else { return };
    let s: Spec = serde_json::from_value(json!({"views":{"detail_b":{"omit":true}}})).unwrap();
    let p = model::plan(&a, &s).unwrap();
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    assert!(
        p.dimensions
            .iter()
            .any(|d| d.id == "d:f:turned_step:3/step.length" && d.view == "v:front")
    );
}
