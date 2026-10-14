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
