//! Corner-block tables (`src/tables.rs`): the spec's `tables` read, refused when malformed,
//! planned as items with IDs and bases, each placed towards its corner clear of the views, their
//! dimensions, the title block, the general notes and the tables before it, drawn ruled and
//! searchable, and compared with draftwright's `Sheet.table` tables (`tests/fixtures/tables.json`,
//! written by `tools/capture_tables.py`).

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/output.rs"]
mod output;

use std::path::Path;
use std::sync::OnceLock;

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::tables::{self, PlannedTable};
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

fn captured() -> Value {
    serde_json::from_str(&std::fs::read_to_string(fixture("tables.json")).unwrap()).unwrap()
}

/// draftwright's revision block and BOM, as `tools/capture_tables.py` declares them.
fn two_tables() -> Spec {
    let tables: Vec<Value> = captured()["tables"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "id": format!("t:{}", i + 1), "rows": t["rows"], "prefer": t["prefer"] }))
        .collect();
    spec(json!({ "tables": tables }))
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn inside(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] >= b[0] && a[1] >= b[1] && a[2] <= b[2] && a[3] <= b[3]
}

fn seg_box(l: &[[f64; 2]; 2]) -> [f64; 4] {
    [
        l[0][0].min(l[1][0]),
        l[0][1].min(l[1][1]),
        l[0][0].max(l[1][0]),
        l[0][1].max(l[1][1]),
    ]
}

/// Every placed table lies on the drawing area, clear of the title block, every view's area
/// (the isometric's drawn box), every dimension's label and lines, the notes block and the
/// other tables, and holds its cells' ink centres.
fn assert_clear(analysis: &Analysis, plan: &Plan) {
    let [w, h] = plan.sheet.page;
    let placed: Vec<(&str, [f64; 4])> = plan
        .tables
        .iter()
        .filter_map(|t| t.placed.as_ref().map(|p| (t.id.as_str(), p.cells)))
        .collect();
    for (k, &(id, b)) in placed.iter().enumerate() {
        assert!(inside(b, [10.0, 10.0, w - 10.0, h - 10.0]), "{id} {b:?}");
        assert!(!overlaps(b, plan.sheet.title_block), "{id}");
        for v in &plan.views {
            let vb = if v.id == "v:iso" {
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
            assert!(!overlaps(b, vb), "{id} {} {vb:?} {b:?}", v.id);
        }
        for d in &plan.dimensions {
            let Some(p) = &d.placed else { continue };
            assert!(!overlaps(b, p.label_box), "{id} {}", d.id);
            for l in &p.lines {
                assert!(!overlaps(b, seg_box(l)), "{id} {}", d.id);
            }
        }
        if let Some(n) = &plan.notes_block {
            assert!(!overlaps(b, n.cells), "{id} and the notes");
        }
        for &(other, ob) in &placed[..k] {
            assert!(!overlaps(b, ob), "{id} and {other}");
        }
    }
    for t in &plan.tables {
        let Some(p) = &t.placed else { continue };
        for at in p.cells_at.iter().flatten().flatten() {
            assert!(inside([at[0], at[1], at[0], at[1]], p.cells), "{}", t.id);
        }
    }
}

#[test]
fn tables_round_trip_and_are_absent_when_empty() {
    let value = json!({
        "schema": "draftwright.spec/1",
        "sheet": {},
        "tables": [
            { "id": "t:1", "rows": [["REV", "DESCRIPTION"], ["A", "INITIAL RELEASE"]] },
            { "id": "t:3", "rows": [["ITEM", "QTY"], [1, 4], [2, 0.5]], "prefer": "bl" }
        ]
    });
    let s = spec(value.clone());
    assert_eq!(s.tables.len(), 2);
    assert_eq!(serde_json::to_value(&s).unwrap(), value);
    let none = serde_json::to_value(Spec::default()).unwrap();
    assert!(none.get("tables").is_none(), "{none}");
    assert!(
        serde_json::from_value::<Spec>(
            json!({ "tables": [ { "id": "t:1", "rows": [["A"]], "name": "x" } ] })
        )
        .is_err()
    );
    // The plan carries the spec, tables included; numbers are cells as JSON writes them.
    let (_, a) = plate();
    let plan = model::plan(a, &s).unwrap();
    assert_eq!(plan.spec.tables, s.tables);
    assert_eq!(
        plan.tables[1].rows,
        [["ITEM", "QTY"], ["1", "4"], ["2", "0.5"]]
    );
    let bare = serde_json::to_value(model::plan(a, &Spec::default()).unwrap()).unwrap();
    assert!(bare.get("tables").is_none());
}

#[test]
fn malformed_tables_are_refused() {
    let (_, a) = plate();
    let rows = json!([["A", "B"], ["1", "2"]]);
    for bad in ["t:0", "t:01", "T:1", "t:", "t:x", "1", "t:1 ", "n:1"] {
        let s = spec(
            json!({ "tables": [ { "id": "t:2", "rows": rows }, { "id": bad, "rows": rows } ] }),
        );
        let e = model::plan(a, &s).unwrap_err();
        assert!(
            e.message.contains("t:<k>") && e.message.contains("tables/1"),
            "{bad}: {e}"
        );
        assert_eq!(e.choices, ["t:3"], "{bad}");
    }
    let s = spec(json!({ "tables": [
        { "id": "t:1", "rows": rows }, { "id": "t:4", "rows": rows }, { "id": "t:1", "rows": rows } ] }));
    let e = model::plan(a, &s).unwrap_err();
    assert!(
        e.message.contains("\"t:1\"") && e.message.contains("tables/2"),
        "{e}"
    );
    assert_eq!(e.choices, ["t:5"]);
    // draftwright's `Sheet.table` refusals, and the port's on cells and corners.
    for (rows, says) in [
        (json!([]), "no rows"),
        (json!(["REV", "DESCRIPTION"]), "use notes"),
        (json!([["A"], 7]), "not a list"),
        (json!([[]]), "at least one"),
        (json!([["A", "B"], ["1"]]), "row 1 has 1"),
        (json!([["A", "B"], ["1", "2", "3"]]), "same number"),
        (json!([["A", null]]), "neither text nor a number"),
        (json!([["A", true]]), "rows/0/1"),
        (json!([["A"], [["B"]]]), "rows/1/0"),
    ] {
        let s = spec(json!({ "tables": [ { "id": "t:1", "rows": rows } ] }));
        let e = model::plan(a, &s).unwrap_err();
        assert!(
            e.message.contains(says) && e.message.contains("t:1"),
            "{rows}: {e}"
        );
    }
    let s = spec(json!({ "tables": [ { "id": "t:1", "rows": rows, "prefer": "top" } ] }));
    let e = model::plan(a, &s).unwrap_err();
    assert!(e.message.contains("\"top\""), "{e}");
    assert_eq!(e.choices, ["tr", "tl", "br", "bl"]);
    // edit refuses the same, so a spec it returns always plans.
    let e = model::edit(
        &Spec::default(),
        &json!([{ "op": "add", "path": "tables", "value": { "rows": ["A", "B"] } }]),
    )
    .unwrap_err();
    assert!(e.message.contains("use notes"), "{e}");
}

#[test]
fn edit_adds_tables_taking_the_next_free_id() {
    let s = model::edit(
        &Spec::default(),
        &json!([
            { "op": "add", "path": "tables", "value": { "rows": [["REV"], ["A"]] } },
            { "op": "add", "path": "tables", "value": { "id": "t:5", "rows": [["ITEM"], ["1"]], "prefer": "bl" } },
            { "op": "add", "path": "tables", "value": { "rows": [["X"]] } },
            { "op": "add", "path": "notes", "value": { "text": "DEBURR" } },
        ]),
    )
    .unwrap();
    let ids: Vec<&str> = s.tables.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["t:1", "t:5", "t:6"]);
    assert_eq!(s.notes[0].id, "n:1");
    assert_eq!(s.tables[1].prefer.as_deref(), Some("bl"));
    let e = model::edit(
        &s,
        &json!([{ "op": "add", "path": "tables", "value": { "id": "t:5", "rows": [["X"]] } }]),
    )
    .unwrap_err();
    assert!(e.message.contains("\"t:5\""), "{e}");
}

#[test]
fn tables_are_plan_items_placed_towards_their_corners() {
    let (_, a) = plate();
    let s = two_tables();
    let plan = model::plan(a, &s).unwrap();
    let ids: Vec<&str> = plan.tables.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["t:1", "t:2"]);
    assert!(plan.problems.iter().all(|p| p.code != "table_not_placed"));
    for (t, given) in plan
        .tables
        .iter()
        .zip(captured()["tables"].as_array().unwrap())
    {
        assert_eq!(json!(t.rows), given["rows"]);
        assert!(!t.default);
        assert!(
            t.basis.starts_with("authored: the spec's table") && t.basis.contains(&t.id),
            "{}",
            t.basis
        );
        let p = t.placed.as_ref().unwrap();
        assert_eq!(p.cells_at.len(), t.rows.len());
        assert_eq!(p.columns.len(), t.rows[0].len() + 1);
        assert!((p.cells[3] - p.cells[1] - tables::ROW * t.rows.len() as f64).abs() < 1e-9);
        assert_eq!(p.columns[0], p.cells[0]);
        assert!((p.columns.last().unwrap() - p.cells[2]).abs() < 1e-9);
        // An empty cell draws nothing and has no ink centre; every other is centred in its cell.
        for (r, row) in t.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let at = p.cells_at[r][c];
                assert_eq!(at.is_none(), cell.is_empty(), "{} {r} {c}", t.id);
                if let Some(at) = at {
                    assert!((at[0] - 0.5 * (p.columns[c] + p.columns[c + 1])).abs() < 1e-9);
                    let mid = p.cells[3] - (r as f64 + 0.5) * tables::ROW;
                    assert!((at[1] - mid).abs() < 1e-9);
                }
            }
        }
    }
    assert_clear(a, &plan);
    // The revision block in the drawing area's top-right corner, the BOM in its bottom left.
    let [w, h] = plan.sheet.page;
    let rev = plan.tables[0].placed.as_ref().unwrap().cells;
    assert!((rev[2] - (w - 10.0)).abs() < 1e-9 && (rev[3] - (h - 10.0)).abs() < 1e-9);
    let bom = plan.tables[1].placed.as_ref().unwrap().cells;
    assert!((bom[0] - 10.0).abs() < 1e-9 && (bom[1] - 10.0).abs() < 1e-9);
}

#[test]
fn tables_keep_clear_of_the_notes_and_of_each_other() {
    // Both towards the top right, after the general notes, which take the corner first.
    let (_, a) = plate();
    let s = spec(json!({
        "notes": [ { "id": "n:1", "text": "BREAK ALL SHARP EDGES" } ],
        "tables": [
            { "id": "t:1", "rows": [["REV", "DESCRIPTION"], ["A", "INITIAL RELEASE"]] },
            { "id": "t:2", "rows": [["ITEM", "QTY"], ["1", "4"]] }
        ]
    }));
    let plan = model::plan(a, &s).unwrap();
    assert!(
        plan.problems
            .iter()
            .all(|p| p.code != "table_not_placed" && p.code != "notes_not_placed"),
        "{:?}",
        plan.problems
    );
    let [w, h] = plan.sheet.page;
    let notes = plan.notes_block.as_ref().unwrap().cells;
    assert!((notes[2] - (w - 10.0)).abs() < 1e-9 && (notes[3] - (h - 10.0)).abs() < 1e-9);
    assert!(plan.tables.iter().all(|t| t.placed.is_some()));
    assert_clear(a, &plan);
}

#[test]
fn a_table_that_fits_nowhere_is_reported_not_dropped() {
    let (part, a) = plate();
    // A cell wider than an A4 sheet.
    let wide = "X".repeat(200);
    let s = spec(json!({ "sheet": { "size": "A4" }, "tables": [
        { "id": "t:1", "rows": [["REV"], ["A"]] },
        { "id": "t:2", "rows": [["DESCRIPTION"], [wide]] } ] }));
    let plan = model::plan(a, &s).unwrap();
    let p: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "table_not_placed")
        .collect();
    assert_eq!(p.len(), 1, "{:?}", plan.problems);
    assert_eq!(p[0].id, "p:table_not_placed:t:2");
    assert_eq!(p[0].severity, "error");
    assert_eq!(p[0].subjects, ["t:2"]);
    assert!(p[0].message.contains("sheet/size"), "{}", p[0].message);
    // It stays in the plan, unplaced; the other is placed; the views keep their layout.
    assert_eq!(plan.tables.len(), 2);
    assert!(plan.tables[0].placed.is_some());
    assert!(plan.tables[1].placed.is_none());
    let bare = model::plan(a, &spec(json!({ "sheet": { "size": "A4" } }))).unwrap();
    assert_eq!(plan.sheet.scale, bare.sheet.scale);
    assert_eq!(
        serde_json::to_value(&plan.views).unwrap(),
        serde_json::to_value(&bare.views).unwrap()
    );
    // Nothing of it is drawn.
    let drawing = model::render(part, a, &plan, &font()).unwrap();
    assert!(
        drawing
            .text
            .iter()
            .all(|t| t.text != wide && t.text != "DESCRIPTION")
    );
    assert!(drawing.text.iter().any(|t| t.text == "REV"));
}

#[test]
fn a_long_table_moves_the_sheet_on_rather_than_be_dropped() {
    let (_, a) = plate();
    let bare = model::plan(a, &Spec::default()).unwrap();
    let mut rows = vec![json!(["ITEM", "PART NO.", "DESCRIPTION", "QTY"])];
    rows.extend(
        (1..=30).map(|k| json!([k, format!("DW-{k:04}"), "A PART WITH A LONG DESCRIPTION", 2])),
    );
    let plan = model::plan(
        a,
        &spec(json!({ "tables": [ { "id": "t:1", "rows": rows } ] })),
    )
    .unwrap();
    assert!(
        plan.problems.iter().all(|p| p.code != "table_not_placed"),
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
fn tables_are_drawn_ruled_and_searchable_and_deterministic() {
    let (part, a) = plate();
    let s = two_tables();
    let plan = model::plan(a, &s).unwrap();
    let again = model::plan(a, &s).unwrap();
    assert_eq!(
        serde_json::to_string(&plan).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
    let font = font();
    let drawing = model::render(part, a, &plan, &font).unwrap();
    let svg = drawing.svg();
    for t in &plan.tables {
        let p = t.placed.as_ref().unwrap();
        for (cell, at) in tables::texts(t) {
            // The cell's run, on its row and within its column.
            let c = p.columns.partition_point(|&x| x < at[0]);
            let runs: Vec<_> = drawing
                .text
                .iter()
                .filter(|r| {
                    r.text == cell
                        && r.at[0] > p.columns[c - 1]
                        && r.at[0] < p.columns[c]
                        && (r.at[1] - at[1]).abs() < 0.5 * tables::ROW
                })
                .collect();
            assert_eq!(runs.len(), 1, "{cell}");
            assert_eq!(runs[0].font, draftwright::dims::FONT);
            let escaped = cell.replace('&', "&amp;").replace('<', "&lt;");
            assert!(svg.contains(&format!(">{escaped}</text>")), "{cell}");
        }
        // A rule at each column edge and at the top and bottom of each row.
        let rules = tables::ruling(p.cells, &p.columns);
        assert_eq!(rules.len(), (t.rows[0].len() + 1) + (t.rows.len() + 1));
    }
    // `⌀` is searchable as written.
    assert!(drawing.text.iter().any(|r| r.text == "⌀6 HOLES REAMED H7"));
    let pdf = drawing.pdf(&output::fonts);
    let again = model::render(part, a, &again, &font).unwrap();
    assert_eq!(pdf, again.pdf(&output::fonts));
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

#[test]
fn tables_match_draftwrights_on_corpus_parts() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let captured = captured();
    let s = two_tables();
    let mut sheets_compared = 0;
    for (name, py) in captured["parts"].as_object().unwrap() {
        let (_, a) = corpus::analyse(&dir, py["file"].as_str().unwrap(), name);
        let plan = model::plan(&a, &s).unwrap();
        // Where the port chooses draftwright's sheet and scale without tables, it keeps them
        // with the tables, as draftwright does on these parts. (m10127's sheet differs without
        // them: tests/fixtures/known_views.json.)
        let bare = model::plan(&a, &Spec::default()).unwrap();
        let same = |p: &Plan, page: &Value, scale: &Value| {
            json!(p.sheet.page) == *page
                && (p.sheet.scale_factor - scale.as_f64().unwrap()).abs() < 1e-9
        };
        assert_eq!(
            same(&plan, &py["page"], &py["scale"]),
            same(&bare, &py["bare"]["page"], &py["bare"]["scale"]),
            "{name}"
        );
        let comparable = same(&bare, &py["bare"]["page"], &py["bare"]["scale"]);
        sheets_compared += usize::from(comparable);
        let [pw, ph] = [
            py["page"][0].as_f64().unwrap(),
            py["page"][1].as_f64().unwrap(),
        ];
        let [w, h] = plan.sheet.page;
        for (t, pt) in plan.tables.iter().zip(py["tables"].as_array().unwrap()) {
            let p: &tables::PlacedTable = t
                .placed
                .as_ref()
                .unwrap_or_else(|| panic!("{name} {}: {:?}", t.id, plan.problems));
            assert_eq!(json!(t.rows), pt["rows"], "{name} {}", t.id);
            let pb = floats(&pt["box"]);
            let pc = floats(&pt["columns"]);
            // Same rows and height; each column's width the widest cell's ink and the padding,
            // the ink measured from the same font's outlines (equal to 1e-4 mm on these tables;
            // 1e-3 allowed for outline flattening).
            assert!(
                (p.cells[3] - p.cells[1] - (pb[3] - pb[1])).abs() < 1e-6,
                "{name} {}",
                t.id
            );
            for c in 0..p.columns.len() - 1 {
                let (ours, theirs) = (p.columns[c + 1] - p.columns[c], pc[c + 1] - pc[c]);
                assert!(
                    (ours - theirs).abs() < 1e-3,
                    "{name} {} column {c}: {ours} vs {theirs}",
                    t.id
                );
            }
            // On the same sheet, the same corner of the drawing area, flush with it. (On
            // m10127's smaller sheet the BOM's corner is taken by the front view's area, so it
            // goes to the next free place: not comparable.)
            let corner = pt["prefer"].as_str().unwrap();
            let (right, top) = (corner.ends_with('r'), corner.starts_with('t'));
            let (x, px) = if right {
                (w - p.cells[2], pw - pb[2])
            } else {
                (p.cells[0], pb[0])
            };
            let (y, py_) = if top {
                (h - p.cells[3], ph - pb[3])
            } else {
                (p.cells[1], pb[1])
            };
            if comparable {
                assert!(
                    (x - px).abs() < 1e-6 && (y - py_).abs() < 1e-6,
                    "{name} {} {:?} {pb:?}",
                    t.id,
                    p.cells
                );
            }
            // Each cell's ink centred where draftwright centres it, measured from the table's
            // left and top edges; an empty cell draws nothing in both.
            for (r, row) in pt["ink"].as_array().unwrap().iter().enumerate() {
                for (c, ink) in row.as_array().unwrap().iter().enumerate() {
                    let ours = p.cells_at[r][c];
                    if ink.is_null() {
                        assert!(ours.is_none(), "{name} {} {r} {c}", t.id);
                        continue;
                    }
                    let b = floats(ink);
                    let theirs = [0.5 * (b[0] + b[2]) - pb[0], pb[3] - 0.5 * (b[1] + b[3])];
                    let ours = ours.unwrap();
                    let ours = [ours[0] - p.cells[0], p.cells[3] - ours[1]];
                    assert!(
                        (ours[0] - theirs[0]).abs() < 1e-3 && (ours[1] - theirs[1]).abs() < 1e-3,
                        "{name} {} {r} {c}: {ours:?} vs {theirs:?}",
                        t.id
                    );
                }
            }
        }
        assert!(
            plan.tables
                .iter()
                .all(|t: &PlannedTable| t.placed.is_some())
        );
        assert_clear(&a, &plan);
    }
    assert!(sheets_compared > 0);
}
