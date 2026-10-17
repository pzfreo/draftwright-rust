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
    let bare = model::plan(a, &Spec::default()).unwrap();
    let many: Vec<Value> = (1..=30)
        .map(|k| json!({ "id": format!("n:{k}"), "text": format!("NOTE {k}: A REQUIREMENT LONG ENOUGH TO WRAP ONTO A SECOND ROW OF THE BLOCK") }))
        .collect();
    let plan = model::plan(a, &spec(json!({ "notes": many }))).unwrap();
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
