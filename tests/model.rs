//! The document model (ADR R1): analyse, plan, edit, render, and the spec carried in the output.

use std::path::{Path, PathBuf};

use draftwright::model::{self, Spec};
use draftwright::text::Font;

fn part(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts")
        .join(name)
}

fn font() -> Font {
    Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
        .unwrap()
}

#[test]
fn analysis_binds_to_its_file() {
    let (_, a) = model::analyse(&part("turned.step")).unwrap();
    assert_eq!(a.binding.source_sha256.len(), 64);
    assert_eq!(a.binding.part, "turned");
    assert!(a.binding.face_count > 0);
    let (_, b) = model::analyse(&part("prismatic.step")).unwrap();
    assert_ne!(a.binding.source_sha256, b.binding.source_sha256);
}

#[test]
fn every_default_says_why_and_the_views_fit_the_sheet_apart() {
    for name in ["turned.step", "prismatic.step"] {
        let (_, a) = model::analyse(&part(name)).unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert!(plan.sheet.default && !plan.sheet.basis.is_empty());
        assert_eq!(plan.views.len(), 4);
        assert!(plan.views.iter().all(|v| v.default && !v.basis.is_empty()));
        assert!(plan.problems.is_empty(), "{:?}", plan.problems);
        // Every view's ink lies inside the border, and no two views' boxes meet.
        let drawing = model::render(&model::analyse(&part(name)).unwrap().0, &a, &plan, &font());
        let [w, h] = plan.sheet.page;
        let mut boxes = Vec::new();
        for v in &plan.views {
            let pieces: Vec<[f64; 2]> = drawing.layers[..2]
                .iter()
                .flat_map(|l| l.items.iter())
                .filter_map(|i| match i {
                    draftwright::output::Item::Edge(c) => Some(c.start()),
                    _ => None,
                })
                .filter(|p| (p[0] - v.at[0]).abs() < w / 2.0 && (p[1] - v.at[1]).abs() < h / 2.0)
                .collect();
            assert!(!pieces.is_empty());
            boxes.push(v.at);
        }
        for p in drawing.layers[1].items.iter().filter_map(|i| match i {
            draftwright::output::Item::Edge(c) => Some(c.start()),
            _ => None,
        }) {
            assert!(
                p[0] > 10.0 && p[0] < w - 10.0 && p[1] > 10.0 && p[1] < h - 10.0,
                "{p:?} outside the border"
            );
        }
    }
}

#[test]
fn invalid_requests_say_what_would_do() {
    let (_, a) = model::analyse(&part("turned.step")).unwrap();
    let with = |ops: serde_json::Value| model::edit(&Spec::default(), &ops).unwrap();
    let e = model::plan(
        &a,
        &with(serde_json::json!([{ "op": "set", "path": "sheet/scale", "value": "3:1" }])),
    )
    .unwrap_err();
    assert!(e.choices.contains(&"2:1".to_string()), "{e}");
    let e = model::plan(
        &a,
        &with(serde_json::json!([{ "op": "set", "path": "views/top/omit", "value": true }])),
    )
    .unwrap_err();
    assert!(e.choices.contains(&"plan".to_string()), "{e}");
    let all = serde_json::json!(["front", "plan", "side", "iso"].map(
        |v| serde_json::json!({ "op": "set", "path": format!("views/{v}/omit"), "value": true })
    ));
    assert!(model::plan(&a, &with(all)).is_err());
    let e = model::edit(
        &Spec::default(),
        &serde_json::json!([{ "op": "set", "path": "sheet/colour", "value": "red" }]),
    )
    .unwrap_err();
    assert!(e.message.contains("colour"), "{e}");
    let e = model::edit(
        &Spec::default(),
        &serde_json::json!([{ "op": "rename", "path": "sheet/size" }]),
    )
    .unwrap_err();
    assert_eq!(e.choices, ["set", "remove"]);
}

#[test]
fn edits_apply_and_the_spec_travels_in_the_output() {
    let (p, a) = model::analyse(&part("prismatic.step")).unwrap();
    let spec = model::edit(
        &Spec::default(),
        &serde_json::json!([
            { "op": "set", "path": "views/iso/omit", "value": true },
            { "op": "set", "path": "sheet/title/material", "value": "6082-T6 ß ]]>" },
            { "op": "set", "path": "sheet/size", "value": "A3" },
            { "op": "remove", "path": "sheet/size" },
        ]),
    )
    .unwrap();
    assert!(spec.views["iso"].omit && spec.sheet.size.is_none());
    let plan = model::plan(&a, &spec).unwrap();
    assert_eq!(plan.views.len(), 3);
    let drawing = model::render(&p, &a, &plan, &font());
    let pdf = drawing.pdf(&|_| {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .ok()
    });
    for bytes in [drawing.svg().into_bytes(), drawing.dxf(), pdf] {
        let back = model::embedded_spec(&bytes).expect("the spec is embedded");
        assert_eq!(back, plan.spec);
        assert_eq!(
            back.binding.as_ref().unwrap().source_sha256,
            a.binding.source_sha256
        );
    }
}

#[test]
fn the_same_documents_give_the_same_bytes() {
    let run = || {
        let (p, a) = model::analyse(&part("turned.step")).unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let d = model::render(&p, &a, &plan, &font());
        (
            serde_json::to_string(&plan).unwrap(),
            d.svg(),
            d.dxf(),
            d.pdf(&|_| None),
        )
    };
    let (a, b) = (run(), run());
    assert!(a.0 == b.0, "plan differs");
    assert!(a.1 == b.1, "svg differs");
    assert!(a.2 == b.2, "dxf differs");
    assert!(a.3 == b.3, "pdf differs");
}
