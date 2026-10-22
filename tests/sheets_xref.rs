//! R5 I2b: border zones and reciprocal, searchable section addresses.
#[path = "common/pdf.rs"]
mod pdf;

use draftwright::{api, model, sheets};
use serde_json::json;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn zone_labels_are_drawn_on_every_multi_sheet_border() {
    for size in ["A4", "A3", "A2", "A1", "A0"] {
        let s = serde_json::from_value(json!({"sheet":{"size":size,"scale":"1:1"},"sheets":{"max":2},"tables":[{"id":"t:1","sheet":2,"rows":[["ISSUED"]]}]})).unwrap();
        let (part, analysis) = model::analyse_with(
            &fixture("parts/plate.step"),
            Some(&fixture("recognition/plate.json")),
        )
        .unwrap();
        let p = model::plan_recovered(&analysis, &s).unwrap();
        let drawings = model::render_sheets(
            &part,
            &analysis,
            &p,
            &draftwright::text::Font::new(api::MONO.to_vec()).unwrap(),
        )
        .unwrap();
        let grid = sheets::zones(p.sheet.page);
        for drawing in drawings {
            let text: Vec<_> = drawing
                .text
                .iter()
                .filter(|t| {
                    t.at[0] < 10.
                        || t.at[1] < 10.
                        || t.at[0] > drawing.page[0] - 10.
                        || t.at[1] > drawing.page[1] - 10.
                })
                .map(|t| t.text.clone())
                .collect();
            for n in 1..=grid.columns {
                assert_eq!(
                    text.iter().filter(|t| **t == n.to_string()).count(),
                    2,
                    "{size}, column {n}"
                );
            }
            for &letter in &b"ABCDEFGHJKLMNPQRSTUVWXYZ"[..grid.rows as usize] {
                assert_eq!(
                    text.iter()
                        .filter(|t| **t == (letter as char).to_string())
                        .count(),
                    2,
                    "{size}, row {letter}"
                );
            }
        }
    }
}

fn section() -> (model::Plan, api::Files) {
    let path = PathBuf::from(std::env::var("DRAFTWRIGHT_CORPUS").unwrap())
        .join("nist/nist_ftc_06_asme1_rd.stp");
    let s = serde_json::from_value(json!({"sheets":{"max":2},"views":{"section_aa":{"sheet":2}}}))
        .unwrap();
    api::render_from(&path, api::Features::Native, &s, &["svg", "pdf"], 24.).unwrap()
}

#[test]
fn remote_section_prints_both_planned_addresses() {
    let (p, files) = section();
    let v = p.views.iter().find(|v| v.id == "v:section_aa").unwrap();
    let from = v.xref.as_ref().unwrap();
    let to = v.cut.as_ref().unwrap().xref.as_ref().unwrap();
    let expected = [
        format!("SECTION A–A, SHEET {}, ZONE {}", to.sheet, to.zone),
        format!("FROM SHEET {}, ZONE {}", from.sheet, from.zone),
    ];
    for (i, wanted) in expected.iter().enumerate() {
        let text = pdf::svg_text(std::str::from_utf8(&files[i].1).unwrap());
        assert!(text.contains(wanted), "sheet {} lacks {wanted}", i + 1);
        assert!(!text.contains(&expected[1 - i]));
    }
    // Read each page's text through its own ToUnicode font maps.
    let objects = pdf::pdf_objects(&files[2].1);
    assert_eq!(
        objects
            .values()
            .filter(|(d, _)| d.contains("/Type /Page\n"))
            .count(),
        2
    );
    let text = pdf::pdf_text_pages(&files[2].1);
    for (page, wanted) in text.iter().zip(expected) {
        assert!(page.contains(&wanted), "PDF lacks {wanted}: {page:?}");
    }
}

#[test]
fn review_checks_structure_on_later_sheets() {
    let s = serde_json::from_value(json!({"sheet":{"size":"A4","scale":"1:1","title":{"title":"X".repeat(200)}},"sheets":{"max":2},"tables":[{"id":"t:1","sheet":2,"rows":[["ISSUED"]]}]})).unwrap();
    let r = api::review(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
        &s,
    )
    .unwrap();
    let issues = serde_json::to_value(r.structural).unwrap();
    assert!(
        issues.as_array().unwrap().iter().any(|i| i["sheet"] == 2),
        "later sheet findings are absent: {issues}"
    );
}

#[test]
fn missing_or_wrong_printed_reference_is_a_structural_error() {
    let path = PathBuf::from(std::env::var("DRAFTWRIGHT_CORPUS").unwrap())
        .join("nist/nist_ftc_06_asme1_rd.stp");
    let (part, analysis) = model::analyse_native(&path).unwrap();
    let (p, _) = section();
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    let mut drawings = model::render_sheets(&part, &analysis, &p, &font).unwrap();
    for (index, drawing) in drawings.iter_mut().enumerate() {
        let clean = draftwright::lint::structural::check(drawing);
        assert!(!clean.iter().any(|i| i.code == "cross_reference_missing"));
        assert!(
            !clean.iter().any(|i| matches!(
                i.code.as_str(),
                "annotation_out_of_bounds"
                    | "view_annotation_overlap"
                    | "annotation_overlap"
                    | "annotation_ink_overlap"
            ) && i
                .annotation
                .as_ref()
                .is_some_and(|name| name.ends_with("/xref"))),
            "new reference must clear the border and existing ink: {clean:?}"
        );
        let prefix = if index == 0 {
            "SECTION A–A, SHEET "
        } else {
            "FROM SHEET "
        };
        // Keep the drawn inventory and change only the searchable printed text.
        for run in &mut drawing.text {
            if run.text.starts_with(prefix) {
                run.text = "WRONG ADDRESS".into();
            }
        }
        let issues = draftwright::lint::structural::check(drawing);
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.code == "cross_reference_missing")
                .count(),
            1
        );
    }
}

/// Check addresses against the geometry on the rendered destination pages, including a
/// section whose caption crosses into the next zone. Copying the planned address is insufficient.
#[test]
fn small_part_remote_zones_match_drawn_view_and_cutting_letter() {
    let root = PathBuf::from(std::env::var("DRAFTWRIGHT_CORPUS").unwrap());
    for name in ["10224", "10255"] {
        let (part, analysis) =
            model::analyse_native(&root.join(format!("mfcadpp/{name}.step"))).unwrap();
        let spec = serde_json::from_value(json!({"sheet":{"size":"A3","scale":"2:1"},"sheets":{"max":2},"views":{"iso":{"sheet":2},"section_aa":{"sheet":2}}})).unwrap();
        let p = model::plan_recovered(&analysis, &spec).unwrap();
        let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
        let drawings = model::render_sheets(&part, &analysis, &p, &font).unwrap();
        assert_eq!(drawings.len(), 2);
        let centre = |b: [f64; 4]| [(b[0] + b[2]) / 2., (b[1] + b[3]) / 2.];
        let section = drawings[1]
            .sheet
            .views
            .iter()
            .find(|v| v.name == "v:section_aa")
            .unwrap();
        let letter = drawings[0]
            .sheet
            .items
            .iter()
            .find(|i| i.name == "section_a_left")
            .unwrap();
        let expected = [
            sheets::zone(drawings[1].page, centre(section.bbox)),
            sheets::zone(drawings[0].page, centre(letter.label.unwrap())),
        ];
        for (i, drawing) in drawings.iter().enumerate() {
            let r = &drawing.sheet.references[0];
            assert_eq!(
                r.zone,
                expected[i],
                "{name}: wrong destination on sheet {}",
                i + 1
            );
            assert!(
                drawing.text[r.run]
                    .text
                    .ends_with(&format!("ZONE {}", expected[i]))
            );
        }
        assert!(
            !draftwright::lint::structural::check_sheets(&drawings)
                .iter()
                .any(|i| i.code == "cross_reference_missing")
        );
        // A caption crossing a grid boundary does not move the view it describes.
        let mut changed = drawings.clone();
        let caption = changed[1]
            .sheet
            .items
            .iter_mut()
            .find(|i| i.name == "section_caption")
            .unwrap();
        caption.ink[1] -= 50.;
        caption.ink[3] -= 50.;
        caption.label = Some(caption.ink);
        assert_ne!(
            expected[0],
            sheets::zone(
                changed[1].page,
                centre(
                    changed[1]
                        .sheet
                        .items
                        .iter()
                        .find(|i| i.name == "section_caption")
                        .unwrap()
                        .ink
                )
            )
        );
        assert!(
            !draftwright::lint::structural::check_sheets(&changed)
                .iter()
                .any(|i| i.code == "cross_reference_missing")
        );
        // Moving the target ink without updating its reference must be detected, both ways.
        for page in 0..2 {
            let mut changed = drawings.clone();
            if page == 1 {
                let view = changed[page]
                    .sheet
                    .views
                    .iter_mut()
                    .find(|v| v.name == "v:section_aa")
                    .unwrap();
                view.bbox[1] -= 60.;
                view.bbox[3] -= 60.;
            } else {
                let letter = changed[page]
                    .sheet
                    .items
                    .iter_mut()
                    .find(|i| i.name == "section_a_left")
                    .unwrap();
                let b = letter.label.as_mut().unwrap();
                b[1] -= 60.;
                b[3] -= 60.;
            }
            assert!(
                draftwright::lint::structural::check_sheets(&changed)
                    .iter()
                    .any(|i| i.code == "cross_reference_missing"
                        && i.annotation.as_deref()
                            == Some(if page == 1 {
                                "v:section_aa/cut/xref"
                            } else {
                                "v:section_aa/xref"
                            }))
            );
        }
        // The auxiliary page uses its width and leaves comfortable frame clearance.
        let views = &drawings[1].sheet.views;
        for v in views {
            assert!(v.bbox[0] > 20. && v.bbox[1] > 20. && v.bbox[2] < 400. && v.bbox[3] < 277.);
        }
        assert!((centre(views[0].bbox)[0] - centre(views[1].bbox)[0]).abs() > 150.);
    }
}

/// ISO 7200: the title block's right and bottom edges are the frame's, on every sheet, drawn
/// once (the frame's line), and no view lies on it.
#[test]
fn title_block_is_flush_with_the_frame_on_every_sheet() {
    use draftwright::layout::MARGIN;
    use draftwright::output::{Curve, Item};
    let (part, analysis) = model::analyse_with(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
    )
    .unwrap();
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    for spec in [
        json!({}),
        json!({"sheet":{"size":"A4","scale":"1:1"},"sheets":{"max":2},"tables":[{"id":"t:1","sheet":2,"rows":[["ISSUED"]]}]}),
    ] {
        let p = model::plan_recovered(&analysis, &serde_json::from_value(spec).unwrap()).unwrap();
        let sheets: Vec<_> = std::iter::once(&p.sheet).chain(&p.sheets).collect();
        let drawings = model::render_sheets(&part, &analysis, &p, &font).unwrap();
        assert_eq!(drawings.len(), sheets.len());
        for (sheet, drawing) in sheets.iter().zip(&drawings) {
            let [w, _] = sheet.page;
            let tb = sheet.title_block;
            assert!(
                (tb[2] - (w - MARGIN)).abs() < 1e-9,
                "sheet {}: {tb:?}",
                sheet.number
            );
            assert!(
                (tb[1] - MARGIN).abs() < 1e-9,
                "sheet {}: {tb:?}",
                sheet.number
            );
            for v in p.views.iter().filter(|v| v.sheet == sheet.number) {
                let a = v.area;
                assert!(
                    a[2] <= tb[0] || a[0] >= tb[2] || a[3] <= tb[1] || a[1] >= tb[3],
                    "sheet {}: {} {a:?} on the title block {tb:?}",
                    sheet.number,
                    v.id
                );
            }
            // One line along each shared side: the frame's.
            let on = |k: usize, at: f64| {
                drawing
                    .layers
                    .iter()
                    .flat_map(|l| &l.items)
                    .filter(|i| match i {
                        Item::Edge(Curve::Line(a, b)) => {
                            (a[k] - at).abs() < 1e-9
                                && (b[k] - at).abs() < 1e-9
                                && (a[1 - k] - b[1 - k]).abs() > 1e-9
                        }
                        _ => false,
                    })
                    .count()
            };
            assert_eq!(on(0, w - MARGIN), 1, "sheet {}", sheet.number);
            assert_eq!(on(1, MARGIN), 1, "sheet {}", sheet.number);
        }
    }
}

/// The four centring marks drawn on a sheet, or none.
fn centring_marks(drawing: &draftwright::output::Drawing) -> Vec<[[f64; 2]; 2]> {
    drawing
        .layers
        .iter()
        .filter(|l| l.name == "centring_marks")
        .flat_map(|l| &l.items)
        .map(|i| match i {
            draftwright::output::Item::Edge(draftwright::output::Curve::Line(a, b)) => [*a, *b],
            other => panic!("a centring mark is a line: {other:?}"),
        })
        .collect()
}

fn margin_labels(drawing: &draftwright::output::Drawing) -> usize {
    drawing
        .text
        .iter()
        .filter(|t| {
            t.at[0] < 10.
                || t.at[1] < 10.
                || t.at[0] > drawing.page[0] - 10.
                || t.at[1] > drawing.page[1] - 10.
        })
        .count()
}

/// ISO 5457: each mark runs from the trimmed edge at a side's midpoint across the margin, to
/// the frame or at most 5 mm inside it, at least 0.7 mm wide.
fn assert_centring_marks(drawing: &draftwright::output::Drawing) {
    let [w, h] = drawing.page;
    let marks = centring_marks(drawing);
    assert_eq!(marks.len(), 4, "{marks:?}");
    let layer = drawing
        .layers
        .iter()
        .find(|l| l.name == "centring_marks")
        .unwrap();
    assert!(layer.style.width >= 0.7);
    let starts = [[w / 2., 0.], [w / 2., h], [0., h / 2.], [w, h / 2.]];
    for ([a, b], start) in marks.iter().zip(starts) {
        assert_eq!(*a, start);
        let depth = (b[0] - a[0]).abs() + (b[1] - a[1]).abs();
        assert!((10.0..=15.0 + 1e-9).contains(&depth), "{a:?} → {b:?}");
        assert!(
            b[0] == a[0] || b[1] == a[1],
            "a mark is perpendicular to its side"
        );
    }
}

#[test]
fn every_sheet_has_zones_and_centring_marks_by_default_and_none_when_off() {
    let (part, analysis) = model::analyse_with(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
    )
    .unwrap();
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    for (spec, sheets) in [
        (json!({}), 1),
        (json!({"sheet":{"size":"A2","scale":"1:1"}}), 1),
        (
            json!({"sheet":{"size":"A4","scale":"1:1"},"sheets":{"max":2},"tables":[{"id":"t:1","sheet":2,"rows":[["ISSUED"]]}]}),
            2,
        ),
    ] {
        let on: model::Spec = serde_json::from_value(spec.clone()).unwrap();
        let mut off_json = spec.clone();
        off_json["sheet"]["zones"] = json!(false);
        let off: model::Spec = serde_json::from_value(off_json).unwrap();
        let shown = model::plan_recovered(&analysis, &on).unwrap();
        let hidden = model::plan_recovered(&analysis, &off).unwrap();
        assert_eq!(shown.sheets.len() + 1, sheets, "{spec}");
        for sheet in std::iter::once(&shown.sheet).chain(&shown.sheets) {
            assert_eq!(sheet.zones, Some(sheets::zones(sheet.page)), "{spec}");
        }
        for sheet in std::iter::once(&hidden.sheet).chain(&hidden.sheets) {
            assert_eq!(sheet.zones, None, "{spec}");
        }
        assert!(!hidden.problems.iter().any(|p| p.code == "sheet_zones_kept"));
        // The furniture is in the margin: the setting changes no view, scale or annotation.
        let layout = |p: &model::Plan| {
            let mut v = serde_json::to_value(p).unwrap();
            v["spec"] = json!(null);
            v["sheet"]["zones"] = json!(null);
            for s in v["sheets"].as_array_mut().into_iter().flatten() {
                s["zones"] = json!(null);
            }
            v
        };
        assert_eq!(layout(&shown), layout(&hidden), "{spec}");
        let drawn = model::render_sheets(&part, &analysis, &shown, &font).unwrap();
        let bare = model::render_sheets(&part, &analysis, &hidden, &font).unwrap();
        for (d, b) in drawn.iter().zip(&bare) {
            assert_centring_marks(d);
            assert!(margin_labels(d) > 0);
            assert!(centring_marks(b).is_empty());
            assert_eq!(margin_labels(b), 0);
            assert_eq!(d.sheet.views.len(), b.sheet.views.len());
            for (x, y) in d.sheet.views.iter().zip(&b.sheet.views) {
                assert_eq!((&x.name, x.bbox), (&y.name, y.bbox));
            }
        }
    }
}

#[test]
fn cited_zones_stay_when_switched_off_and_are_reported_once() {
    let root = PathBuf::from(std::env::var("DRAFTWRIGHT_CORPUS").unwrap());
    let (part, analysis) = model::analyse_native(&root.join("mfcadpp/10224.step")).unwrap();
    let spec = serde_json::from_value(json!({"sheet":{"size":"A3","scale":"2:1","zones":false},"sheets":{"max":2},"views":{"iso":{"sheet":2},"section_aa":{"sheet":2}}})).unwrap();
    let p = model::plan_recovered(&analysis, &spec).unwrap();
    assert!(p.views.iter().any(|v| v.xref.is_some()));
    assert!(p.sheet.zones.is_some() && p.sheets.iter().all(|s| s.zones.is_some()));
    let kept: Vec<_> = p
        .problems
        .iter()
        .filter(|q| q.code == "sheet_zones_kept")
        .collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].severity, "info");
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    for d in model::render_sheets(&part, &analysis, &p, &font).unwrap() {
        assert_centring_marks(&d);
    }
}
