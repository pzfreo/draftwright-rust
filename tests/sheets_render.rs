//! R5 I2a: every planned sheet reaches SVG/PDF/PNG and the review response.

#[path = "common/output.rs"]
mod output;
#[path = "common/pdf.rs"]
mod pdf;

use draftwright::model::Spec;
use draftwright::{api, model, title_block};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

fn spec(value: Value) -> Spec {
    serde_json::from_value(value).unwrap()
}

fn plate(s: &Spec, formats: &[&str]) -> (model::Plan, api::Files) {
    api::render_at(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
        s,
        formats,
        24.,
    )
    .unwrap()
}

#[test]
fn every_sheet_has_its_title_block() {
    for size in ["A4", "A3", "A2", "A1", "A0"] {
        let tables: Vec<_> = (2..=9)
            .map(|n| json!({"id":format!("t:{n}"),"sheet":n,"rows":[["ISSUED"]]}))
            .collect();
        let s = spec(
            json!({"sheet":{"size":size,"scale":"1:1","title":{"title":"PLATE","number":"DW-1","sheet":"CUSTOM","tolerance":"UNSPECIFIED"}},"sheets":{"max":9},"tables":tables}),
        );
        let (p, files) = plate(&s, &["svg"]);
        assert_eq!(p.sheets.len(), 8);
        assert_eq!(files.len(), 9);
        for (i, sheet) in std::iter::once(&p.sheet).chain(&p.sheets).enumerate() {
            let cells = title_block::cells(sheet.title_block, &s.sheet.title, size);
            assert_eq!(
                sheet.title_cells.iter().map(|c| c.cell).collect::<Vec<_>>(),
                cells.iter().map(|c| c.cell).collect::<Vec<_>>()
            );
            assert_eq!(
                sheet
                    .title_cells
                    .iter()
                    .find(|c| c.field == "sheet")
                    .unwrap()
                    .value,
                Some(format!("{}/9", i + 1))
            );
            let svg = std::str::from_utf8(&files[i].1).unwrap();
            let text = pdf::svg_text(svg);
            for value in [
                "PLATE",
                "DW-1",
                "UNSPECIFIED",
                size,
                &format!("{}/9", i + 1),
            ] {
                assert!(text.iter().any(|t| t == value), "sheet {}: {value}", i + 1);
            }
            assert!(!text.iter().any(|t| t == "CUSTOM"));
        }
        assert_eq!(
            p.problems
                .iter()
                .filter(|q| q.code == "sheet_label_replaced")
                .count(),
            1
        );
        assert!(
            !p.problems
                .iter()
                .any(|q| q.code == "title_field_overflow" && q.refs == ["title_block/sheet"])
        );
    }
    let cells = title_block::cells(
        [167., 10., 287., 50.],
        &std::collections::BTreeMap::from([("sheet".into(), "100/120".into())]),
        "A4",
    );
    assert_eq!(
        cells
            .iter()
            .find(|c| c.field == "sheet")
            .unwrap()
            .value
            .as_deref(),
        Some("100/120")
    );
    assert!(
        title_block::problems(&cells, [297., 210.])
            .iter()
            .any(|q| q.code == "title_field_overflow" && q.refs == ["title_block/sheet"])
    );
}

#[test]
fn formats_and_written_sheet_files_round_trip_the_whole_spec() {
    let s = spec(
        json!({"sheet":{"size":"A4","scale":"1:1"},"sheets":{"max":2},"views":{"iso":{"sheet":2}}}),
    );
    let (p, files) = plate(&s, &["svg", "pdf", "png"]);
    assert_eq!(
        files
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["svg", "sheet2.svg", "pdf", "png", "sheet2.png"]
    );
    let objects = pdf::pdf_objects(&files.iter().find(|(n, _)| n == "pdf").unwrap().1);
    assert_eq!(
        objects
            .values()
            .filter(|(d, _)| d.contains("/Type /Page\n"))
            .count(),
        2
    );
    assert!(objects.values().any(|(d, _)| d.contains("/Count 2")));
    let contents: Vec<_> = objects
        .values()
        .filter(|(d, _)| d.contains("/Type /Page\n"))
        .map(|(d, _)| &objects[&pdf::reference(d, "/Contents")].1)
        .collect();
    assert_ne!(contents[0], contents[1], "distinct page ink/text streams");
    let bytes = |name: &str| &files.iter().find(|(n, _)| n == name).unwrap().1;
    assert_ne!(
        bytes("png"),
        bytes("sheet2.png"),
        "each sheet gets its own raster"
    );

    let dir = std::env::temp_dir().join(format!("draftwright-i2a-formats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = api::write(&dir.join("part.rev2.pdf"), &files).unwrap();
    for ((name, bytes), path) in files.iter().zip(&paths) {
        assert_eq!(
            Path::new(path).file_name().unwrap().to_str().unwrap(),
            format!("part.rev2.{name}")
        );
        assert_eq!(&std::fs::read(path).unwrap(), bytes);
        assert_eq!(api::spec(Path::new(path)).unwrap(), p.spec);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn review_exposes_later_sheets_and_keeps_sheet_one_furniture() {
    let s = spec(
        json!({"sheet":{"size":"A4","scale":"1:1"},"sheets":{"max":2},"views":{"iso":{"sheet":2}},"notes":[{"id":"n:1","text":"BREAK SHARP EDGES"}],"tables":[{"id":"t:1","sheet":2,"rows":[["ISSUED"]]}]}),
    );
    let review = api::review(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
        &s,
    )
    .unwrap();
    let review = serde_json::to_value(review).unwrap();
    let extra = review["sheets"].as_array().expect("later sheets in review");
    assert_eq!(extra.len(), 1);
    let first = pdf::svg_text(review["svg"].as_str().unwrap());
    let second = pdf::svg_text(extra[0].as_str().unwrap());
    assert!(first.join(" ").contains("BREAK SHARP EDGES"));
    assert!(!first.iter().any(|t| t == "ISSUED"));
    assert!(second.iter().any(|t| t == "ISSUED"));
    assert!(!second.join(" ").contains("BREAK SHARP EDGES"));
    let (_, a) = model::analyse_with(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
    )
    .unwrap();
    let single = api::review_analysed(&fixture("parts/plate.step"), &a, &Spec::default()).unwrap();
    assert!(
        serde_json::to_value(single)
            .unwrap()
            .get("sheets")
            .is_none()
    );
}

#[test]
fn remote_section_has_its_hatch_and_caption_on_sheet_two() {
    let corpus = std::env::var("DRAFTWRIGHT_CORPUS").unwrap();
    let path = Path::new(&corpus).join("nist/nist_ftc_06_asme1_rd.stp");
    let s = spec(json!({"sheets":{"max":2},"views":{"section_aa":{"sheet":2}}}));
    let (plan, files) = api::render_from(&path, api::Features::Native, &s, &["svg"], 24.).unwrap();
    assert_eq!(files.len(), 2);
    let text = |i: usize| pdf::svg_text(std::str::from_utf8(&files[i].1).unwrap());
    assert!(
        text(0)
            .iter()
            .any(|t| t.starts_with("SECTION A–A, SHEET 2, ZONE "))
    );
    // Row A is labelled on both borders of each page, in addition to the two cut letters.
    assert_eq!(text(0).iter().filter(|t| *t == "A").count(), 4);
    assert!(text(1).iter().any(|t| t == "SECTION A–A"));
    assert_eq!(text(1).iter().filter(|t| *t == "A").count(), 2);
    let section = plan.views.iter().find(|v| v.id == "v:section_aa").unwrap();
    let ink = output::read_svg(std::str::from_utf8(&files[1].1).unwrap());
    assert!(
        ink["dims"]
            .lines
            .iter()
            .any(|(a, b)| [a, b].iter().all(|p| {
                p[0] >= section.area[0]
                    && p[0] <= section.area[2]
                    && p[1] > section.at[1]
                    && p[1] <= section.area[3]
            })),
        "hatch strokes reach the upper half of the section, away from its caption/title block"
    );
}
