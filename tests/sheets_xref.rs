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
fn zone_labels_are_drawn_on_every_multi_sheet_border_only() {
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
