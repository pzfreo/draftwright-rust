//! R5: coverage evidence can span sheets; geometry checks cannot.
use draftwright::{api, lint, model};
use std::path::Path;

#[test]
fn split_marks_are_covered_once_and_positions_stay_on_their_sheet() {
    let step = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step");
    let features =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recognition/plate.json");
    let (part, analysis) = model::analyse_with(&step, Some(&features)).unwrap();
    let p = model::plan_recovered(&analysis, &model::Spec::default()).unwrap();
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    let original = model::render(&part, &analysis, &p, &font).unwrap();
    let expected = lint::lint(&analysis, &original);
    assert_eq!(
        lint::lint_sheets(&analysis, std::slice::from_ref(&original)),
        expected
    );

    // Rebuild local text indices on both pages. The second page has a different offset;
    // simply concatenating annotations without rebasing their indices cannot pass.
    let mut drawings = vec![original.clone(), original.clone()];
    for (page, drawing) in drawings.iter_mut().enumerate() {
        drawing.text.clear();
        drawing.annotations.clear();
        for (index, annotation) in original.annotations.iter().enumerate() {
            if index % 2 != page {
                continue;
            }
            let mut annotation = annotation.clone();
            for run in &mut annotation.runs {
                let text = original.text[*run].clone();
                *run = drawing.text.len();
                drawing.text.push(text);
            }
            drawing.annotations.push(annotation);
        }
    }
    drawings[1].sheet.centre_marks.clear();
    assert_ne!(lint::lint(&analysis, &drawings[0]), expected);
    assert_ne!(lint::lint(&analysis, &drawings[1]), expected);
    assert_eq!(lint::lint_sheets(&analysis, &drawings), expected);
    assert_eq!(
        lint::lint_sheets(
            &analysis,
            &drawings.iter().rev().cloned().collect::<Vec<_>>()
        ),
        expected
    );

    // Identical boxes on separate pages do not collide; each page's own findings retain
    // their identity and severity, and the sheet-1 serialized shape stays unchanged.
    let per_page = lint::structural::check(&drawings[0]);
    let both = lint::structural::check_sheets(&[drawings[0].clone(), drawings[0].clone()]);
    assert_eq!(both.len(), per_page.len() * 2);
    assert_eq!(&both[..per_page.len()], per_page);
    assert!(both[per_page.len()..].iter().all(|i| i.sheet == Some(2)));
    assert!(
        serde_json::to_value(&per_page)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i.get("sheet").is_none())
    );
}

#[test]
fn an_address_must_reach_the_drawn_target_zone() {
    let step = std::path::PathBuf::from(std::env::var("DRAFTWRIGHT_CORPUS").unwrap())
        .join("nist/nist_ftc_06_asme1_rd.stp");
    let (part, analysis) = model::analyse_native(&step).unwrap();
    let s = serde_json::from_value(
        serde_json::json!({"sheets":{"max":2},"views":{"section_aa":{"sheet":2}}}),
    )
    .unwrap();
    let p = model::plan_recovered(&analysis, &s).unwrap();
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    let mut drawings = model::render_sheets(&part, &analysis, &p, &font).unwrap();
    assert!(
        !lint::structural::check_sheets(&drawings)
            .iter()
            .any(|i| i.code == "cross_reference_missing")
    );
    let mut lost = drawings.clone();
    for drawing in &mut lost {
        drawing.sheet.references.clear();
    }
    let lost_issues = lint::structural::check_sheets(&lost);
    assert_eq!(
        lost_issues
            .iter()
            .filter(|i| i.code == "cross_reference_missing")
            .count(),
        2
    );
    assert!(
        lost_issues
            .iter()
            .any(|i| i.code == "cross_reference_missing" && i.sheet == Some(2))
    );
    // A mutually consistent claim and text still cannot point to the wrong grid field.
    let reference = &mut drawings[0].sheet.references[0];
    reference.zone = "A1".into();
    let (run, text) = (
        reference.run,
        format!(
            "{} SHEET {}, ZONE {}",
            reference.prefix, reference.sheet, reference.zone
        ),
    );
    drawings[0].text[run].text = text;
    assert_eq!(
        lint::structural::check_sheets(&drawings)
            .iter()
            .filter(|i| i.code == "cross_reference_missing")
            .count(),
        1
    );
    assert!(
        !lint::structural::check(&drawings[0])
            .iter()
            .any(|i| i.code == "cross_reference_missing")
    );
}

#[test]
fn remote_hole_table_rows_cover_the_sheet_one_balloons() {
    let step =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/scattered_holes.step");
    let (part, analysis) = model::analyse_native(&step).unwrap();
    let mut p = model::plan(
        &analysis,
        &serde_json::from_value(serde_json::json!({"sheet":{"preferred_max_size":"A0"}})).unwrap(),
    )
    .unwrap();
    let id = p
        .dimensions
        .iter()
        .find(|d| {
            d.view == "v:plan"
                && d.placed.is_some()
                && d.kind == "linear"
                && d.measurements.iter().any(|m| {
                    m.feature.starts_with("f:hole:") && m.parameter.starts_with("location.length")
                })
        })
        .unwrap()
        .id
        .clone();
    // Exercise I1's existing escalation trigger without changing the allocator or the part.
    p.problems.push(model::Problem {
        id: format!("stand-in:{id}"),
        code: "annotation_off_sheet".into(),
        severity: "warning".into(),
        subjects: vec![id],
        ..Default::default()
    });
    assert_eq!(
        draftwright::hole_table::escalate(&analysis, &mut p, Some(2)),
        draftwright::hole_table::TableOutcome::Kept
    );
    let mut second = p.sheet.clone();
    second.number = 2;
    second.projection_symbol = None;
    let table = p.hole_table.as_mut().unwrap();
    table.blocks =
        draftwright::hole_table::place_on_sheet(table, &second, &Default::default()).unwrap();
    p.sheets.push(second);
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    let drawings = model::render_sheets(&part, &analysis, &p, &font).unwrap();
    assert_eq!(drawings.len(), 2);
    assert_eq!(p.hole_table.as_ref().unwrap().rows.len(), 18);
    assert_eq!(p.balloons.len(), 18);
    let first = lint::lint(&analysis, &drawings[0]);
    let together = lint::lint_sheets(&analysis, &drawings);
    let missing = |issues: &[lint::LintIssue]| {
        issues
            .iter()
            .filter(|i| i.code == "hole_requirement_missing")
            .count()
    };
    assert!(
        missing(&first) > 0,
        "the table's requirements must really be absent from sheet 1"
    );
    assert_eq!(
        missing(&together),
        0,
        "every physical hole row must be credited: {together:?}"
    );
}
