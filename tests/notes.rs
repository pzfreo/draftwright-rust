//! General notes (`src/notes.rs`): the spec's `notes` read, refused when malformed, planned as
//! items with IDs and bases, placed in one block clear of the views, their dimensions and the
//! title block, drawn and searchable, and compared with draftwright's `general_notes` table
//! (`tests/fixtures/notes.json`, written by `tools/capture_notes.py`). The PDF text layer is read
//! back in `tests/output.rs`, which has the PDF reader.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/output.rs"]
mod output;

use std::path::Path;
use std::sync::OnceLock;

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::notes::{self, PlannedNote};
use draftwright::text::Font;
use serde_json::{Value, json};

fn fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// The plate fixture with its recognition, read once.
fn plate() -> &'static (Part, Analysis) {
    static PLATE: OnceLock<(Part, Analysis)> = OnceLock::new();
    PLATE.get_or_init(|| {
        let (part, mut a) = model::analyse(&fixture("parts/plate.step")).unwrap();
        let json = std::fs::read_to_string(fixture("recognition/plate.json")).unwrap();
        model::recognise(&part, &mut a, &json).unwrap();
        (part, a)
    })
}

fn font() -> Font {
    Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
        .unwrap()
}

fn spec(value: Value) -> Spec {
    serde_json::from_value(value).unwrap()
}

/// draftwright's two notes, as `tools/capture_notes.py` adds them.
fn captured() -> Value {
    serde_json::from_str(&std::fs::read_to_string(fixture("notes.json")).unwrap()).unwrap()
}

fn two_notes() -> Spec {
    let texts = captured()["notes"].clone();
    spec(json!({ "notes": [
        { "id": "n:1", "text": texts[0] },
        { "id": "n:7", "text": texts[1] },
    ] }))
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn inside(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] >= b[0] && a[1] >= b[1] && a[2] <= b[2] && a[3] <= b[3]
}

/// The box of a segment.
fn seg_box(l: &[[f64; 2]; 2]) -> [f64; 4] {
    [
        l[0][0].min(l[1][0]),
        l[0][1].min(l[1][1]),
        l[0][0].max(l[1][0]),
        l[0][1].max(l[1][1]),
    ]
}

/// The block lies on the sheet's drawing area and clear of the title block, every view's area
/// (the isometric's drawn box) and every dimension's label and lines.
fn assert_clear(analysis: &Analysis, plan: &Plan) {
    let block = plan
        .notes_block
        .as_ref()
        .expect("the block is placed")
        .cells;
    let [w, h] = plan.sheet.page;
    assert!(inside(block, [10.0, 10.0, w - 10.0, h - 10.0]), "{block:?}");
    assert!(!overlaps(block, plan.sheet.title_block));
    for v in &plan.views {
        let b = if v.id == "v:iso" {
            let ib = analysis.iso_bounds;
            [
                v.at[0] + v.scale * ib[0],
                v.at[1] + v.scale * ib[1],
                v.at[0] + v.scale * ib[2],
                v.at[1] + v.scale * ib[3],
            ]
        } else {
            v.area
        };
        assert!(!overlaps(block, b), "{} {b:?} {block:?}", v.id);
    }
    for d in &plan.dimensions {
        let Some(p) = &d.placed else { continue };
        assert!(!overlaps(block, p.label_box), "{}", d.id);
        for l in &p.lines {
            assert!(!overlaps(block, seg_box(l)), "{}", d.id);
        }
    }
    for n in &plan.notes {
        if n.id.starts_with("n:f:blend:") {
            if let Some(p) = &n.placed {
                assert!(inside(
                    p.cells,
                    plan.radius_notes_block.as_ref().unwrap().cells
                ));
                assert!(!overlaps(p.cells, block));
            } else {
                assert!(
                    plan.problems
                        .iter()
                        .any(|p| p.code == "notes_not_placed" && p.subjects.contains(&n.id))
                );
            }
            continue;
        }
        let cells = n.placed.as_ref().unwrap().cells;
        assert!(inside(cells, block), "{}", n.id);
    }
}

#[test]
fn notes_round_trip_and_are_absent_when_empty() {
    let value = json!({
        "schema": "draftwright.spec/1",
        "sheet": {},
        "notes": [ { "id": "n:1", "text": "BREAK ALL SHARP EDGES" }, { "id": "n:3", "text": "DEBURR" } ]
    });
    let s = spec(value.clone());
    assert_eq!(s.notes.len(), 2);
    assert_eq!(serde_json::to_value(&s).unwrap(), value);
    let none = serde_json::to_value(Spec::default()).unwrap();
    assert!(none.get("notes").is_none(), "{none}");
    // An unknown field inside a note is refused, as anywhere in the spec.
    assert!(
        serde_json::from_value::<Spec>(
            json!({ "notes": [ { "id": "n:1", "text": "A", "at": 1 } ] })
        )
        .is_err()
    );
    // The plan carries the spec, notes included, and nothing about notes when there are none.
    let (_, a) = plate();
    let plan = model::plan(a, &s).unwrap();
    assert_eq!(plan.spec.notes, s.notes);
    let bare = serde_json::to_value(model::plan(a, &Spec::default()).unwrap()).unwrap();
    assert!(bare.get("notes").is_none() && bare.get("notes_block").is_none());
}

#[test]
fn malformed_notes_are_refused_with_the_next_free_id() {
    let (_, a) = plate();
    for bad in ["n:0", "n:01", "N:1", "n:", "n:x", "1", "n:1 ", "n:-1"] {
        let s =
            spec(json!({ "notes": [ { "id": "n:2", "text": "A" }, { "id": bad, "text": "B" } ] }));
        let e = model::plan(a, &s).unwrap_err();
        assert!(
            e.message.contains("n:<k>") && e.message.contains("notes/1"),
            "{bad}: {e}"
        );
        assert_eq!(e.choices, ["n:3"], "{bad}");
    }
    let s = spec(json!({ "notes": [
        { "id": "n:1", "text": "A" }, { "id": "n:4", "text": "B" }, { "id": "n:1", "text": "C" } ] }));
    let e = model::plan(a, &s).unwrap_err();
    assert!(
        e.message.contains("\"n:1\"") && e.message.contains("notes/2"),
        "{e}"
    );
    assert_eq!(e.choices, ["n:5"]);
    for empty in ["", "   ", "\t\n"] {
        let s = spec(json!({ "notes": [ { "id": "n:1", "text": empty } ] }));
        let e = model::plan(a, &s).unwrap_err();
        assert!(
            e.message.contains("no text") && e.message.contains("n:1"),
            "{e}"
        );
    }
    // edit refuses the same, so a spec it returns always plans.
    let e = model::edit(
        &Spec::default(),
        &json!([{ "op": "set", "path": "notes", "value": [ { "id": "n:1", "text": "" } ] }]),
    )
    .unwrap_err();
    assert!(e.message.contains("no text"), "{e}");
}

#[test]
fn edit_adds_notes_taking_the_next_free_id() {
    let s = model::edit(
        &Spec::default(),
        &json!([
            { "op": "add", "path": "notes", "value": { "text": "DEBURR" } },
            { "op": "add", "path": "notes", "value": { "id": "n:5", "text": "BREAK ALL SHARP EDGES" } },
            { "op": "add", "path": "notes", "value": { "text": "MASK THREADS BEFORE PLATING" } },
        ]),
    )
    .unwrap();
    let ids: Vec<&str> = s.notes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["n:1", "n:5", "n:6"]);
    assert_eq!(s.notes[0].text, "DEBURR");
    // A duplicate given explicitly is refused; adding to what is not a list is refused.
    let e = model::edit(
        &s,
        &json!([{ "op": "add", "path": "notes", "value": { "id": "n:5", "text": "X" } }]),
    )
    .unwrap_err();
    assert!(e.message.contains("\"n:5\""), "{e}");
    let e = model::edit(
        &s,
        &json!([{ "op": "add", "path": "sheet/size", "value": "A3" }]),
    );
    assert!(e.is_err());
    let e = model::edit(
        &Spec::default(),
        &json!([{ "op": "add", "path": "sheet", "value": {} }]),
    )
    .unwrap_err();
    assert!(e.message.contains("not a list"), "{e}");
}

#[test]
fn notes_are_plan_items_numbered_in_order() {
    let (_, a) = plate();
    let plan = model::plan(a, &two_notes()).unwrap();
    let ids: Vec<&str> = plan.notes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["n:1", "n:7"]);
    let expected = &captured()["parts"]["m10127"]["rows"];
    let rows: Vec<&str> = std::iter::once(notes::TITLE)
        .chain(
            plan.notes
                .iter()
                .flat_map(|n| n.rows.iter().map(String::as_str)),
        )
        .collect();
    assert_eq!(json!(rows), *expected);
    for (i, n) in plan.notes.iter().enumerate() {
        assert_eq!(n.number, i + 1);
        assert!(!n.default);
        assert!(
            n.basis.starts_with("authored: the spec's note") && n.basis.contains(&n.id),
            "{}",
            n.basis
        );
        assert_eq!(n.placed.as_ref().unwrap().rows_at.len(), n.rows.len());
    }
    assert!(plan.problems.iter().all(|p| p.code != "notes_not_placed"));
    assert_clear(a, &plan);
    // The block sits at the top right of the drawing area, its rows from the top down.
    let block = plan.notes_block.as_ref().unwrap();
    let [w, h] = plan.sheet.page;
    assert!(
        (block.cells[2] - (w - 10.0)).abs() < 1e-9 && (block.cells[3] - (h - 10.0)).abs() < 1e-9
    );
    let ys: Vec<f64> = std::iter::once(block.title_at[1])
        .chain(
            plan.notes
                .iter()
                .flat_map(|n| n.placed.as_ref().unwrap().rows_at.iter().map(|p| p[1])),
        )
        .collect();
    assert!(
        ys.windows(2)
            .all(|p| (p[0] - p[1] - notes::ROW).abs() < 1e-9),
        "{ys:?}"
    );
    assert!((block.cells[3] - block.cells[1] - 5.0 * notes::ROW).abs() < 1e-9);
}

#[test]
fn rows_wrap_as_draftwright_wraps_them() {
    for case in captured()["wraps"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        // draftwright's rows are font-safe (`ø`); the port keeps `⌀` in the row and draws `ø`.
        let rows: Vec<String> = notes::rows(1, text).iter().map(|r| notes::ink(r)).collect();
        assert_eq!(json!(rows), case["rows"], "{text:?}");
    }
}

#[test]
fn a_continuation_row_keeps_its_indent() {
    // draftwright aligns each row's ink at the cell's padding, so its continuation rows lose
    // their three-space indent; the port aligns the rows on their pens.
    let (_, a) = plate();
    let plan = model::plan(a, &two_notes()).unwrap();
    let long = &plan.notes[1];
    let font = font();
    let left = |row: &str, at: [f64; 2]| {
        let laid = font.text(&notes::ink(row), notes::TEXT);
        at[0] + laid.min[0]
    };
    let p = long.placed.as_ref().unwrap();
    let number = left(&long.rows[0], p.rows_at[0]);
    let text = number + font.advance("2  ", notes::TEXT);
    for (row, at) in long.rows.iter().zip(&p.rows_at).skip(1) {
        let start = left(row, *at);
        // The continuation's first letter starts under the note's first letter (`R`), give or
        // take the two letters' side bearings.
        assert!((start - text).abs() < 0.5, "{row}: {start} vs {text}");
        assert!(start > number + 4.0, "{row}");
    }
}

#[test]
fn a_note_that_fits_nowhere_is_reported_not_dropped() {
    let (part, a) = plate();
    // One word of 200 characters, which is never broken, is wider than an A4 sheet.
    let word = "X".repeat(200);
    let s = spec(json!({ "sheet": { "size": "A4" }, "notes": [
        { "id": "n:1", "text": "DEBURR" }, { "id": "n:2", "text": word } ] }));
    let plan = model::plan(a, &s).unwrap();
    let p: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "notes_not_placed")
        .collect();
    assert_eq!(p.len(), 1, "{:?}", plan.problems);
    assert_eq!(p[0].severity, "error");
    assert_eq!(p[0].subjects, ["n:1", "n:2"]);
    assert!(p[0].message.contains("sheet/size"), "{}", p[0].message);
    // The notes stay in the plan, unplaced; the views keep the layout they have without them.
    assert_eq!(plan.notes.len(), 2);
    assert!(plan.notes.iter().all(|n| n.placed.is_none()));
    assert!(plan.notes_block.is_none());
    let bare = model::plan(a, &spec(json!({ "sheet": { "size": "A4" } }))).unwrap();
    assert_eq!(plan.sheet.scale, bare.sheet.scale);
    assert_eq!(
        serde_json::to_value(&plan.views).unwrap(),
        serde_json::to_value(&bare.views).unwrap()
    );
    assert!(plan.problems.iter().all(|p| p.code != "views_do_not_fit"));
    // Nothing of the notes is drawn.
    let drawing = model::render(part, a, &plan, &font()).unwrap();
    assert!(drawing.text.iter().all(|t| !t.text.contains("DEBURR")));
}

#[test]
fn a_long_note_moves_the_sheet_on_rather_than_be_dropped() {
    // A block too big for the first sheet is reserved, so the ladder takes the next one, as
    // draftwright's `layout_required_tables` does.
    let (_, a) = plate();
    let bare = model::plan(
        a,
        &serde_json::from_value(serde_json::json!({"sheet":{"preferred_max_size":"A0"}})).unwrap(),
    )
    .unwrap();
    let many: Vec<Value> = (1..=30)
        .map(|k| json!({ "id": format!("n:{k}"), "text": format!("NOTE {k}: A REQUIREMENT LONG ENOUGH TO WRAP ONTO A SECOND ROW OF THE BLOCK") }))
        .collect();
    let plan = model::plan(
        a,
        &spec(json!({ "sheet":{"preferred_max_size":"A0"}, "notes": many })),
    )
    .unwrap();
    assert!(
        plan.problems.iter().all(|p| p.code != "notes_not_placed"),
        "{:?}",
        plan.problems
    );
    assert_ne!(
        (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
        (bare.sheet.size.as_str(), bare.sheet.scale.as_str())
    );
    assert_clear(a, &plan);
}

#[test]
fn notes_are_drawn_and_searchable_and_deterministic() {
    let (part, a) = plate();
    let s = two_notes();
    let plan = model::plan(a, &s).unwrap();
    let again = model::plan(a, &s).unwrap();
    assert_eq!(
        serde_json::to_string(&plan).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
    let font = font();
    let drawing = model::render(part, a, &plan, &font).unwrap();
    let block = plan.notes_block.as_ref().unwrap();
    for n in std::iter::once(notes::TITLE.to_string()).chain(
        plan.notes
            .iter()
            .flat_map(|n: &PlannedNote| n.rows.iter().map(|r| r.trim_start().to_string())),
    ) {
        let runs: Vec<_> = drawing.text.iter().filter(|t| t.text == n).collect();
        assert_eq!(runs.len(), 1, "{n}");
        assert_eq!(runs[0].font, draftwright::dims::FONT);
        assert!(inside(
            [runs[0].at[0], runs[0].at[1], runs[0].at[0], runs[0].at[1]],
            block.cells
        ));
    }
    // The block's rules: its two sides and a line at the top of each row and the bottom.
    assert_eq!(notes::rules(block).len(), 2 + 6);
    let pdf = drawing.pdf(&output::fonts);
    let again = model::render(part, a, &again, &font).unwrap();
    assert_eq!(pdf, again.pdf(&output::fonts));
}

#[test]
fn the_block_matches_draftwrights_on_corpus_parts() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let captured = captured();
    let mut sheets_compared = 0;
    for (name, py) in captured["parts"].as_object().unwrap() {
        let (_, a) = corpus::analyse(&dir, py["file"].as_str().unwrap(), name);
        let plan = model::plan(&a, &two_notes()).unwrap();
        // Where the port chooses draftwright's sheet and scale without notes, it keeps them
        // with the notes, as draftwright does on these parts. (m10127's sheet differs without
        // notes: tests/fixtures/known_views.json.)
        let bare = model::plan(&a, &Spec::default()).unwrap();
        let same = |p: &Plan| {
            json!(p.sheet.page) == py["page"]
                && (p.sheet.scale_factor - py["scale"].as_f64().unwrap()).abs() < 1e-9
        };
        assert_eq!(same(&plan), same(&bare), "{name}");
        sheets_compared += usize::from(same(&bare));
        let block = plan
            .notes_block
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: {:?}", plan.problems))
            .cells;
        let pb: Vec<f64> = py["box"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let size: Vec<f64> = py["size"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        // Same height, rows and top-right corner; wider by the continuation rows' indent, give
        // or take the side bearings of the letters that start the rows (rust-correct:
        // draftwright drops the indent), so the left edge is further left by that much.
        assert!((block[3] - block[1] - size[1]).abs() < 1e-9, "{name}");
        let [w, h] = plan.sheet.page;
        let [pw, ph] = [
            py["page"][0].as_f64().unwrap(),
            py["page"][1].as_f64().unwrap(),
        ];
        if plan.views.iter().any(|v| v.detail.is_some()) {
            // Details reserve furniture space before the notes. A different corner is
            // justified only by actual occupied ink at Python's established corner.
            let corner = [
                w - (pw - pb[2]) - (block[2] - block[0]),
                h - (ph - pb[3]) - size[1],
                w - (pw - pb[2]),
                h - (ph - pb[3]),
            ];
            assert!(
                ((w - block[2] - (pw - pb[2])).abs() < 1e-6
                    && (h - block[3] - (ph - pb[3])).abs() < 1e-6)
                    || plan.views.iter().any(|v| overlaps(
                        corner,
                        [
                            v.area[0] - 2.0,
                            v.area[1] - 2.0,
                            v.area[2] + 2.0,
                            v.area[3] + 2.0
                        ]
                    ))
                    || plan
                        .dimensions
                        .iter()
                        .filter_map(|d| d.placed.as_ref())
                        .any(|p| overlaps(corner, p.label_box)
                            || p.lines.iter().any(|l| overlaps(corner, seg_box(l)))),
                "{name}: an unobstructed top-right corner must stay preferred"
            );
        } else {
            assert!(
                (w - block[2] - (pw - pb[2])).abs() < 1e-6
                    && (h - block[3] - (ph - pb[3])).abs() < 1e-6,
                "{name} {block:?} {pb:?}"
            );
        }
        let indent = font().advance("   ", notes::TEXT);
        let wider = (block[2] - block[0]) - size[0];
        assert!((wider - indent).abs() < 0.2, "{name}: {wider} vs {indent}");
        assert_clear(&a, &plan);
    }
    assert!(sheets_compared > 0);
}

/// Patch the existing plate with hand-built semantic text; geometry is never duplicated.
fn part_notes_step(fragment: &str) -> String {
    let source = std::fs::read_to_string(fixture("parts/plate.step")).unwrap();
    let source = source.replace(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{fragment}\nENDSEC;\nEND-ISO-10303-21;"),
    );
    assert!(source.contains("#50002="));
    source
}

fn with_part_notes(fragment: &str) -> (Part, Analysis) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let source = part_notes_step(fragment);
    let path = std::env::temp_dir().join(format!(
        "draftwright-part-notes-{}-{}.step",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, source).unwrap();
    let out = model::analyse(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    out
}

fn part_notes() -> &'static (Part, Analysis) {
    static NOTES: OnceLock<(Part, Analysis)> = OnceLock::new();
    NOTES.get_or_init(|| with_part_notes(include_str!("fixtures/pmi/part_notes.inc")))
}

fn drawing_notes() -> Spec {
    spec(
        json!({"notes": [{"id": "n:1", "text": "DO NOT SCALE"}, {"id": "n:2", "text": "DIMENSIONS IN MM"}]}),
    )
}

#[test]
fn part_semantic_text_notes_are_read_and_drawn_before_spec_notes() {
    use draftwright::pmi::{Category, Reading};
    let (part, a) = part_notes();
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("PMI not read")
    };
    let records: Vec<_> = report
        .records
        .iter()
        .filter(|r| r.category == Category::ManufacturingRequirement)
        .collect();
    assert_eq!(
        records.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>(),
        ["general_note", "surface_treatment", "heat_treatment"]
    );
    assert!(
        records
            .iter()
            .all(|r| r.blockers.is_empty() && r.faces.is_empty())
    );
    let s = drawing_notes();
    let plan = model::plan(a, &s).unwrap();
    assert_eq!(plan.spec.notes, s.notes);
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>(),
        [
            "ETCH PART NUMBER",
            "ANODIZE BLACK",
            "HARDEN TO 40 HRC",
            "DO NOT SCALE",
            "DIMENSIONS IN MM"
        ]
    );
    assert_eq!(
        plan.notes.iter().map(|n| n.number).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert!(plan.notes.iter().all(|n| n.placed.is_some()));
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "pmi_not_lowered" || p.code == "pmi_not_extracted")
    );
    let drawing = model::render(part, a, &plan, &font()).unwrap();
    for n in &plan.notes {
        assert_eq!(
            drawing.text.iter().filter(|t| t.text == n.rows[0]).count(),
            1
        );
    }
    let again = model::render(part, a, &model::plan(a, &s).unwrap(), &font()).unwrap();
    assert_eq!(drawing.pdf(&output::fonts), again.pdf(&output::fonts));
    let issues = draftwright::lint::lint(a, &drawing);
    assert!(
        !issues.iter().any(|i| i.code.starts_with("pmi_")),
        "{issues:?}"
    );
    // The independent text audit rejects a changed requirement on its correct source item.
    let mut altered = drawing.clone();
    let annotation = altered
        .annotations
        .iter()
        .find(|a| a.name == plan.notes[1].id)
        .unwrap();
    altered.text[annotation.runs[0]].text = "2  PAINT RED".into();
    assert!(
        draftwright::lint::lint(a, &altered)
            .iter()
            .any(|i| i.code == "pmi_source_text_mismatch")
    );
}

#[test]
fn specify_general_notes_are_drawn_in_one_based_note_index_order() {
    use draftwright::haecceity::{p21, pmi, step};
    let fragment = include_str!("fixtures/pmi/part_notes_indexed.inc");
    let bytes = part_notes_step(fragment).into_bytes();
    let defs = step::read_part_definitions(&bytes).unwrap();
    let doc = p21::Document::parse(bytes).unwrap();
    let read = pmi::read(&doc, &defs).unwrap();
    // Check the dependency's actual reading, not only draftwright's projected fields.
    let indices: Vec<_> = read.parts[0]
        .attributes
        .iter()
        .flat_map(|a| &a.items)
        .filter(|(name, _)| name == "note index")
        .map(|(_, value)| match value {
            pmi::AttributeValue::Integer(n) => *n,
            other => panic!("note index was not surfaced as an integer: {other:?}"),
        })
        .collect();
    assert_eq!(indices, [3, 1, 2]);
    let (part, a) = with_part_notes(fragment);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| (n.number, n.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (1, "Inspect bore"),
            (2, "DEBURR ALL EDGES"),
            (3, "ETCH PART NUMBER")
        ]
    );
    assert!(plan.notes.iter().all(|n| n.placed.is_some()));
    let drawing = model::render(&part, &a, &plan, &font()).unwrap();
    for note in &plan.notes {
        assert_eq!(
            drawing
                .text
                .iter()
                .filter(|t| t.text == note.rows[0])
                .count(),
            1
        );
    }
    assert_clear(&a, &plan);
    assert!(
        !draftwright::lint::lint(&a, &drawing)
            .iter()
            .any(|p| p.code.starts_with("pmi_"))
    );
}

#[test]
fn unindexed_part_notes_follow_indexed_notes_in_file_order() {
    let fragment = include_str!("fixtures/pmi/part_notes_indexed.inc")
        .replace("(#50000,#50001)", "(#50000)")
        .replace("(#50020,#50021)", "(#50020)")
        // Larger than the file instance IDs: an index is never compared with a file ID.
        .replace("'note index',1", "'note index',100000");
    let (_, a) = with_part_notes(&fragment);
    let plan = model::plan(&a, &drawing_notes()).unwrap();
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Inspect bore",
            "ETCH PART NUMBER",
            "DEBURR ALL EDGES",
            "DO NOT SCALE",
            "DIMENSIONS IN MM"
        ]
    );
    let (_, a) = with_part_notes(&fragment.replace("(#50010,#50011)", "(#50010)"));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>(),
        ["ETCH PART NUMBER", "Inspect bore", "DEBURR ALL EDGES"]
    );
}

#[test]
fn part_notes_honour_explicit_order_and_index() {
    let fragment = include_str!("fixtures/pmi/part_notes.inc")
        .replace("(#50000)", "(#50000,#50030)")
        .replace("(#50010)", "(#50010,#50031)")
        .replace("(#50020)", "(#50020,#50032)");
    let fragment = format!(
        "{fragment}\n#50030=DESCRIPTIVE_REPRESENTATION_ITEM('order','2');\n#50031=INTEGER_REPRESENTATION_ITEM('index',3);\n#50032=DESCRIPTIVE_REPRESENTATION_ITEM('index','1');"
    );
    let (_, a) = with_part_notes(&fragment);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>(),
        ["HARDEN TO 40 HRC", "ETCH PART NUMBER", "ANODIZE BLACK"]
    );
}

#[test]
fn part_note_omit_uses_the_annotation_item_id_and_keeps_an_omitted_record() {
    use draftwright::items::State;
    let (_, a) = part_notes();
    let before = model::plan(a, &drawing_notes()).unwrap();
    let note = &before.notes[1];
    let s = model::edit(&drawing_notes(), &json!([{"op": "set", "path": format!("annotations/{}/omit", draftwright::items::key(&note.id)), "value": true}])).unwrap();
    let plan = model::plan(a, &s).unwrap();
    let omitted = plan.notes.iter().find(|n| n.id == note.id).unwrap();
    assert_eq!(omitted.state, State::OmittedByUser);
    assert!(omitted.placed.is_none());
    assert_eq!(omitted.number, 0);
    assert_eq!(
        plan.notes
            .iter()
            .filter(|n| n.placed.is_some())
            .map(|n| (n.number, n.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (1, "ETCH PART NUMBER"),
            (2, "HARDEN TO 40 HRC"),
            (3, "DO NOT SCALE"),
            (4, "DIMENSIONS IN MM")
        ]
    );
    let item = draftwright::items::items(&plan)
        .into_iter()
        .find(|i| i.id == note.id)
        .unwrap();
    assert_eq!(item.state, State::OmittedByUser);
    assert_eq!(
        plan.requirements
            .iter()
            .find(|r| r.id == note.id)
            .unwrap()
            .state,
        State::OmittedByUser
    );
    assert_eq!(
        before.notes[1].text,
        model::plan(a, &drawing_notes()).unwrap().notes[1].text
    );
    let (part, analysis) = with_part_notes(include_str!("fixtures/pmi/part_notes.inc"));
    let session = draftwright::session::Session::new(part, analysis, s).unwrap();
    let options = session.options(&note.id).unwrap();
    assert_eq!(options.operations, ["omit"]);
    let path = format!("annotations/{}/omit", draftwright::items::key(&note.id));
    assert_eq!(options.settable[&path]["choices"], json!([false, true]));
    assert!(!options.settable.keys().any(|key| key.starts_with(&format!(
        "annotations/{}/place",
        draftwright::items::key(&note.id)
    ))));
}

#[test]
fn duplicate_spec_note_is_covered_once_and_its_offered_fix_removes_only_the_spec_copy() {
    use draftwright::items::State;
    let (part, a) = part_notes();
    let mut s = drawing_notes();
    s.notes.push(notes::NoteSpec {
        id: "n:8".into(),
        text: "  etch\tPART\n number  ".into(),
    });
    let plan = model::plan(a, &s).unwrap();
    assert_eq!(plan.spec.notes, s.notes);
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "note_duplicated_in_spec")
        .unwrap();
    assert_eq!(p.severity, "info");
    assert_eq!(p.subjects, [plan.notes[0].id.clone(), "n:8".into()]);
    assert!(p.refs.contains(&"manufacturing_requirement:#50002".into()));
    assert!(p.refs.contains(&"notes/2".into()));
    let covered = plan.notes.iter().find(|n| n.id == "n:8").unwrap();
    assert_eq!(
        covered.state,
        State::Covered {
            by: plan.notes[0].id.clone()
        }
    );
    assert!(covered.placed.is_none());
    let drawing = model::render(part, a, &plan, &font()).unwrap();
    assert_eq!(
        drawing
            .text
            .iter()
            .filter(|t| t.text.contains("ETCH PART NUMBER"))
            .count(),
        1
    );
    let fixed = model::edit(&s, &serde_json::to_value(&p.fix).unwrap()).unwrap();
    assert_eq!(fixed.notes, drawing_notes().notes);
    let after = model::plan(a, &fixed).unwrap();
    assert!(
        !after
            .problems
            .iter()
            .any(|p| p.code == "note_duplicated_in_spec")
    );
    assert_eq!(
        plan.notes
            .iter()
            .filter(|n| n.placed.is_some())
            .collect::<Vec<_>>(),
        after.notes.iter().collect::<Vec<_>>()
    );
    // Omitting the model copy keeps an existing drawing copy visible until its fix is applied.
    let omitted = model::edit(&s, &json!([{"op": "set", "path": format!("annotations/{}/omit", draftwright::items::key(&plan.notes[0].id)), "value": true}])).unwrap();
    let omitted = model::plan(a, &omitted).unwrap();
    assert!(
        omitted
            .notes
            .iter()
            .find(|n| n.id == "n:8")
            .unwrap()
            .placed
            .is_some()
    );
}

#[test]
fn parts_without_part_pmi_draw_only_spec_notes() {
    // Part-note reading adds nothing to a part that carries no part PMI: no notes without Spec
    // notes, and exactly the Spec notes, in order, when it has them.
    for file in ["plate", "mounting_plate"] {
        let (part, a) = model::analyse(&fixture(&format!("parts/{file}.step"))).unwrap();
        let plain = model::plan(&a, &Spec::default()).unwrap();
        assert!(plain.notes.is_empty(), "{file}: {:?}", plain.notes);
        let drawing = model::render(&part, &a, &plain, &font()).unwrap();
        assert!(
            !drawing.text.iter().any(|t| t.text.contains("NOTES")),
            "{file}: a notes block was drawn"
        );
        let plan = model::plan(&a, &drawing_notes()).unwrap();
        assert_eq!(
            plan.notes
                .iter()
                .map(|n| (n.number, n.text.as_str()))
                .collect::<Vec<_>>(),
            [(1, "DO NOT SCALE"), (2, "DIMENSIONS IN MM")],
            "{file}"
        );
        let noted = model::render(&part, &a, &plan, &font()).unwrap();
        assert!(
            noted.text.iter().any(|t| t.text.contains("NOTES")),
            "{file}"
        );
    }
}

#[test]
fn semantic_text_edges_and_default_finish_keep_their_carriers() {
    let fragment = format!(
        "{}\n{}",
        include_str!("fixtures/pmi/part_notes.inc"),
        r#"
#50040=DESCRIPTIVE_REPRESENTATION_ITEM('edge condition','BREAK SHARP EDGES');
#50041=GENERAL_PROPERTY('','semantic text',$);
#50042=PROPERTY_DEFINITION('semantic text','user defined attribute',#4);
#50043=REPRESENTATION('',(#50040),#713);
#50044=PROPERTY_DEFINITION_REPRESENTATION(#50042,#50043);
#50045=GENERAL_PROPERTY_ASSOCIATION('',$,#50041,#50042);
#50050=DESCRIPTIVE_REPRESENTATION_ITEM('material removal condition','any process allowed');
#50051=PROPERTY_DEFINITION('surface texture','',#4);
#50052=REPRESENTATION('surface texture',(#50050),#713);
#50053=PROPERTY_DEFINITION_REPRESENTATION(#50051,#50052);
#50054=DESCRIPTIVE_REPRESENTATION_ITEM('measuring method','Ra');
#50055=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MICRO.,.METRE.));
#50056=(LENGTH_MEASURE_WITH_UNIT()MEASURE_REPRESENTATION_ITEM()MEASURE_WITH_UNIT(LENGTH_MEASURE(1.6),#50055)REPRESENTATION_ITEM('characteristic value'));
#50057=PROPERTY_DEFINITION('surface_condition','',#4);
#50058=SURFACE_TEXTURE_REPRESENTATION('surface texture parameter',(#50054,#50056),#713);
#50059=PROPERTY_DEFINITION_REPRESENTATION(#50057,#50058);
#50060=PROPERTY_DEFINITION_RELATIONSHIP('surface texture parameter','',#50051,#50057);
#50061=GENERAL_PROPERTY('','surface_condition',$);
#50062=GENERAL_PROPERTY_ASSOCIATION('',$,#50061,#50057);
"#
    );
    let (_, a) = with_part_notes(&fragment);
    let plan = model::plan(&a, &drawing_notes()).unwrap();
    assert_eq!(plan.notes[3].text, "BREAK SHARP EDGES");
    assert_eq!(plan.notes[4].text, "DO NOT SCALE");
    let finish = plan
        .requirements
        .iter()
        .find(|r| r.kind == "default_surface_finish")
        .unwrap();
    assert_eq!(finish.text, "Ra 1.6");
    assert_eq!(finish.carrier, "sheet");
    assert!(finish.placed.is_some());
    assert!(!plan.notes.iter().any(|n| n.text.contains("Ra")));
}

#[test]
fn part_note_no_requirement_values_are_read_as_info_without_ink_or_problems() {
    use draftwright::{
        items::State,
        pmi::{Outcome, Reading},
    };
    for value in [
        "None", " nOnE ", "N/A", " n/a ", "NA", "na", "-", "", " \t\n ",
    ] {
        let fragment = include_str!("fixtures/pmi/part_notes.inc")
            .replace("ETCH PART NUMBER", value)
            .replace("ANODIZE BLACK", value)
            .replace("HARDEN TO 40 HRC", value);
        let (part, a) = with_part_notes(&fragment);
        let Some(Reading::Read(report)) = &a.pmi else {
            panic!("not read")
        };
        assert_eq!(report.records.len(), 3);
        assert!(
            report
                .records
                .iter()
                .all(|r| r.label.trim() == value.trim() && r.blockers.is_empty())
        );
        assert!(
            report
                .sources
                .iter()
                .all(|s| s.outcome == Outcome::Extracted
                    && s.reason.starts_with("info: no requirement stated")),
            "{value:?}: {:?}",
            report.sources
        );
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert!(
            plan.notes.is_empty() && plan.notes_block.is_none(),
            "{value:?}"
        );
        assert_eq!(plan.requirements.len(), 3);
        let omitted = model::edit(&Spec::default(), &json!([{"op":"set", "path":format!("annotations/{}/omit", draftwright::items::key(&plan.requirements[0].id)), "value":true}])).unwrap();
        assert!(model::plan(&a, &omitted).unwrap().notes.is_empty());
        assert!(plan.requirements.iter().all(|r| r.kind == "no_requirement"
            && r.carrier == "none"
            && r.state == State::NotAuthored
            && r.placed.is_none()
            && r.basis.starts_with("info:")));
        assert!(
            !plan
                .problems
                .iter()
                .any(|p| p.code.starts_with("pmi_") || p.code == "note_duplicated_in_spec"),
            "{value:?}: {:?}",
            plan.problems
        );
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        assert!(
            !drawing
                .text
                .iter()
                .any(|t| t.text == format!("1  {}", value.trim()))
        );
        assert!(
            !draftwright::lint::lint(&a, &drawing)
                .iter()
                .any(|i| i.code.starts_with("pmi_"))
        );
    }
}

#[test]
fn coating_and_edges_aliases_use_canonical_part_note_records() {
    let fragment = include_str!("fixtures/pmi/part_notes.inc")
        .replace("'surface treatment'", "'coating'")
        .replace("'general note'", "'edges'")
        .replace("ETCH PART NUMBER", "BREAK SHARP EDGES");
    let (_, a) = with_part_notes(&fragment);
    let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi else {
        panic!("not read")
    };
    assert_eq!(
        report
            .records
            .iter()
            .map(|r| r.kind.as_str())
            .collect::<Vec<_>>(),
        ["edge_condition", "surface_treatment", "heat_treatment"]
    );
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        plan.notes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>(),
        ["BREAK SHARP EDGES", "ANODIZE BLACK", "HARDEN TO 40 HRC"]
    );
}

#[test]
fn bare_process_values_get_a_kind_but_process_prose_keeps_its_words() {
    for (kind, value, expected) in [
        ("surface treatment", "Black", "Coating: Black"),
        ("coating", "Platinum", "Coating: Platinum"),
        ("heat treatment", "HRC 40", "Heat treatment: HRC 40"),
        ("surface treatment", "Anodise black", "Anodise black"),
        ("coating", "ANODIZE BLACK", "ANODIZE BLACK"),
        ("coating", "Zinc plated", "Zinc plated"),
        ("coating", "Plate with zinc", "Plate with zinc"),
        ("coating", "Paint black", "Paint black"),
        ("coating", "Coat black", "Coat black"),
        ("coating", "Coating: Black", "Coating: Black"),
        ("heat treatment", "Harden to HRC 40", "Harden to HRC 40"),
        ("heat treatment", "Temper at 200 C", "Temper at 200 C"),
        ("heat treatment", "Nitrided", "Nitrided"),
        ("heat treatment", "Treat to HRC 40", "Treat to HRC 40"),
        (
            "heat treatment",
            "Heat treatment: HRC 40",
            "Heat treatment: HRC 40",
        ),
        ("general note", "Black", "Black"),
    ] {
        let fragment = include_str!("fixtures/pmi/part_notes.inc")
            .replace("'surface treatment'", &format!("'{kind}'"))
            .replace("ANODIZE BLACK", value);
        let (part, a) = with_part_notes(&fragment);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert_eq!(plan.notes[1].text, expected);
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        assert!(
            !draftwright::lint::lint(&a, &drawing)
                .iter()
                .any(|i| i.code == "pmi_source_text_mismatch"),
            "{kind}: {value}"
        );
        if expected != value {
            let s = spec(json!({"notes":[{"id":"n:1","text":value}]}));
            let duplicate = model::plan(&a, &s).unwrap();
            assert!(
                duplicate
                    .problems
                    .iter()
                    .any(|p| p.code == "note_duplicated_in_spec")
            );
            assert!(
                duplicate
                    .notes
                    .iter()
                    .find(|n| n.id == "n:1")
                    .unwrap()
                    .placed
                    .is_none()
            );
        }
    }
}

#[test]
fn a_non_text_part_note_is_not_mistaken_for_an_empty_requirement() {
    let fragment = include_str!("fixtures/pmi/part_notes.inc").replace(
        "#50010=DESCRIPTIVE_REPRESENTATION_ITEM('surface treatment','ANODIZE BLACK');",
        "#50010=INTEGER_REPRESENTATION_ITEM('surface treatment',42);",
    );
    let (_, a) = with_part_notes(&fragment);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert!(plan.problems.iter().any(|p| p.code == "pmi_not_lowered"
        && p.refs.contains(&"manufacturing_requirement:#50012".into())));
    assert!(
        !plan
            .requirements
            .iter()
            .any(|r| r.source == "manufacturing_requirement:#50012")
    );
}
