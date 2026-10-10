//! AP242 thread and knurl requirements on the drawing (PMI stage 2c,
//! `pmi_lowering::manufacturing`): each is given to the hole or turned step its cylinder proves
//! and printed on that feature's callout, its suffix or, with two or more and one too long,
//! `SEE MFG n` and the manufacturing requirements table, as draftwright's
//! `lower_ap242_manufacturing_requirements` and `manufacturing_schedule` do; what no one feature
//! owns is reported with draftwright's reason, never dropped.
//!
//! draftwright's callouts and table rows are in `tests/fixtures/pmi_drawings/<stem>.json.gz`
//! (`tools/capture_pmi_drawings.py`: the owner's callout that prints each requirement, and
//! `manufacturing_table`).

#[path = "common/pdf.rs"]
mod pdf;

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::pmi::{Field, Reading, Record};
use serde_json::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn gunzip(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(raw)
        .read_to_end(&mut out)
        .unwrap();
    out
}

/// The fixture *stem*'s part and Analysis (Rust quiddity's features), from its own scratch file.
fn analysed(stem: &str) -> (draftwright::haecceity::Part, Analysis) {
    let dir = root().join("tests/fixtures/pmi/step");
    let gz = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("{stem}."))
        })
        .unwrap();
    let name = gz.file_name().unwrap().to_string_lossy().into_owned();
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pmi_thread_knurl")
        .join(n.to_string());
    std::fs::create_dir_all(&scratch).unwrap();
    let path = scratch.join(name.strip_suffix(".gz").unwrap());
    std::fs::write(&path, gunzip(&std::fs::read(&gz).unwrap())).unwrap();
    let out = model::analyse_native(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

/// draftwright's drawing of *stem* (`tools/capture_pmi_drawings.py`).
fn captured(stem: &str) -> Value {
    let path = root().join(format!("tests/fixtures/pmi_drawings/{stem}.json.gz"));
    serde_json::from_slice(&gunzip(&std::fs::read(path).unwrap())).unwrap()
}

/// draftwright's label of the callout that prints *source*, `ø` read as the port's `⌀` on a hole
/// (the port prints a hole's `⌀` and a turned diameter's `ø`, as draftwright does).
fn python_label(drawing: &Value, source: &str) -> String {
    let item = drawing["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["source_id"] == source)
        .unwrap();
    let drawn = item["drawn"].as_array().unwrap();
    assert_eq!(drawn.len(), 1, "{source}: {drawn:?}");
    drawn[0]["text"].as_str().unwrap().to_string()
}

fn records(a: &Analysis) -> &[Record] {
    match &a.pmi {
        Some(Reading::Read(r)) => &r.records,
        other => panic!("not read: {other:?}"),
    }
}

fn edit(a: &Analysis, source: &str, f: impl Fn(&mut Record)) -> Analysis {
    let mut b = a.clone();
    let Some(Reading::Read(r)) = &mut b.pmi else {
        unreachable!()
    };
    r.records
        .iter_mut()
        .filter(|r| r.source_id == source)
        .for_each(f);
    b
}

/// The plan's item claiming *source*, with its label.
fn claimed<'a>(plan: &'a Plan, source: &str) -> Vec<(&'a str, &'a str)> {
    let feature = format!("pmi:{source}");
    plan.dimensions
        .iter()
        .filter(|d| d.measurements.iter().any(|m| m.feature == feature))
        .map(|d| (d.id.as_str(), d.label.as_str()))
        .collect()
}

/// The plan's refusal of *source*, if any: its message.
fn refused(plan: &Plan, source: &str) -> Option<String> {
    plan.problems
        .iter()
        .find(|p| p.code == "pmi_not_lowered" && p.subjects.first().is_some_and(|s| s == source))
        .map(|p| p.message.clone())
}

#[test]
fn a_single_thread_prints_its_suffix_on_its_holes_callout() {
    let (part, a) = analysed("string_post_tapped");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let source = "manufacturing_requirement:#3157";
    let python = python_label(&captured("string_post_tapped"), source);
    assert_eq!(python, "⌀1.6 ↧ 4 M2 x 0.4-6H RH; 2.5 MIN FULL THREAD");
    assert_eq!(
        claimed(&plan, source),
        [("d:f:hole:2/bore.diameter@plan", python.as_str())]
    );
    assert_eq!(refused(&plan, source), None);
    // One requirement: no table, in draftwright's drawing or the port's.
    assert!(captured("string_post_tapped")["manufacturing_table"].is_null());
    assert!(plan.tables.is_empty());
    // Drawn, searchable, and credited by the coverage lint.
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let svg = pdf::svg_text(&drawing.svg()).join(" ");
    assert!(svg.contains("MIN FULL THREAD"), "{svg}");
    let lint = draftwright::lint::lint(&a, &drawing);
    assert!(
        !lint.iter().any(|i| i.sources.iter().any(|s| s == source)),
        "{lint:#?}"
    );
}

#[test]
fn several_long_requirements_refer_to_the_manufacturing_requirements_table() {
    let (part, a) = analysed("thumbwheel_thread_knurl");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let drawing_py = captured("thumbwheel_thread_knurl");
    for (source, id) in [
        (
            "manufacturing_requirement:#689",
            "d:f:hole:1/bore.diameter@side",
        ),
        (
            "manufacturing_requirement:#722",
            "d:f:turned_step:4/step.diameter@front",
        ),
        (
            "manufacturing_requirement:#749",
            "d:f:turned_step:1/step.diameter@front",
        ),
    ] {
        let python = python_label(&drawing_py, source);
        assert_eq!(claimed(&plan, source), [(id, python.as_str())], "{source}");
        assert_eq!(refused(&plan, source), None);
    }
    let rows: Vec<Vec<String>> =
        serde_json::from_value(drawing_py["manufacturing_table"].clone()).unwrap();
    assert_eq!(rows[2], ["MFG 2", "ø6 MAX AFTER KNURL; DIAMOND KNURL P1;"]);
    let table = plan
        .tables
        .iter()
        .find(|t| t.id == "manufacturing_requirements")
        .unwrap();
    assert_eq!(table.rows, rows);
    assert!(table.default);
    let p = table.placed.as_ref().expect("the table has a place");
    // The requirement column is left-aligned at the padding, as `_left_align_cols=(1,)`.
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    for (row, at) in table.rows.iter().zip(&p.cells_at) {
        let laid = font.baseline(
            &draftwright::tables::ink(&row[1]),
            draftwright::tables::TEXT,
        );
        let left = at[1].unwrap()[0] - 0.5 * (laid.max[0] - laid.min[0]);
        assert!(
            (left - (p.columns[1] + draftwright::tables::PAD)).abs() < 1e-9,
            "{row:?}"
        );
    }
    // The table does not cover the general notes, the views or the dimensions.
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.subjects.iter().any(|s| s == "manufacturing_requirements")),
        "{:#?}",
        plan.problems
    );
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let text = pdf::svg_text(&drawing.svg());
    for cell in rows.iter().flatten().filter(|c| !c.is_empty()) {
        assert!(text.contains(cell), "{cell}");
    }
    let lint = draftwright::lint::lint(&a, &drawing);
    for source in ["#689", "#722", "#749"] {
        let source = format!("manufacturing_requirement:{source}");
        assert!(
            !lint.iter().any(|i| i.sources.contains(&source)),
            "{source}"
        );
    }
}

#[test]
fn a_requirement_no_one_feature_owns_is_reported_with_draftwrights_reason() {
    let (part, a) = analysed("thumbwheel_thread_knurl");
    let lint_of = |b: &Analysis| {
        let plan = model::plan(b, &Spec::default()).unwrap();
        let font =
            draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT))
                .unwrap();
        let drawing = model::render(&part, b, &plan, &font).unwrap();
        (plan.clone(), draftwright::lint::lint(b, &drawing))
    };
    // No owner: the tapped hole is not recognised.
    let mut b = a.clone();
    b.features.retain(|f| f.id != "f:hole:1");
    let source = "manufacturing_requirement:#689";
    let (plan, lint) = lint_of(&b);
    let why = refused(&plan, source).unwrap();
    assert!(
        why.ends_with(
            "is not drawn: unmatched manufacturing requirement: no canonical feature matches source topology."
        ),
        "{why}"
    );
    assert!(claimed(&plan, source).is_empty());
    assert!(lint.iter().any(|i| i.code == "pmi_not_lowered"
        && i.sources == [source]
        && i.message.contains("no canonical feature matches")));
    // The two left still go to the table (the knurl's suffix is 55 characters), renumbered.
    let labels: Vec<&str> = plan.dimensions.iter().map(|d| d.label.as_str()).collect();
    assert!(labels.contains(&"ø6 SEE MFG 1"), "{labels:?}");
    assert!(labels.contains(&"ø5 SEE MFG 2"), "{labels:?}");
    // Two owners: a second turned step on the knurl's cylinder.
    let mut b = a.clone();
    let mut twin = b
        .features
        .iter()
        .find(|f| f.id == "f:turned_step:4")
        .unwrap()
        .clone();
    twin.id = "f:turned_step:9".into();
    b.features.push(twin);
    let plan = model::plan(&b, &Spec::default()).unwrap();
    let why = refused(&plan, "manufacturing_requirement:#722").unwrap();
    assert!(
        why.contains(
            "ambiguous manufacturing requirement: source topology matches 2 canonical features"
        ),
        "{why}"
    );
    // Text that disagrees with its cylinder: the knurl's maximum diameter, in words and fields.
    let b = edit(&a, "manufacturing_requirement:#722", |r| {
        r.label = r.label.replace("DIA 6 mm", "DIA 7 mm");
        for (name, v) in &mut r.fields {
            if name == "major diameter" {
                *v = Field::Length(7.0);
            }
        }
    });
    let plan = model::plan(&b, &Spec::default()).unwrap();
    let why = refused(&plan, "manufacturing_requirement:#722").unwrap();
    assert!(
        why.ends_with("manufacturing requirement text disagrees with source cylinder."),
        "{why}"
    );
    // Words and fields that disagree.
    let b = edit(&a, "manufacturing_requirement:#722", |r| {
        r.label = r.label.replace("DIA 6 mm", "DIA 7 mm");
    });
    let plan = model::plan(&b, &Spec::default()).unwrap();
    let why = refused(&plan, "manufacturing_requirement:#722").unwrap();
    assert!(
        why.ends_with("structured knurl values disagree with prose."),
        "{why}"
    );
    // Words outside draftwright's grammar, paired with a structured statement.
    let b = edit(&a, "manufacturing_requirement:#749", |r| {
        r.label = "M5 thread".into();
    });
    let plan = model::plan(&b, &Spec::default()).unwrap();
    let why = refused(&plan, "manufacturing_requirement:#749").unwrap();
    assert!(
        why.ends_with("structured thread cannot reconcile with prose."),
        "{why}"
    );
    assert!(records(&a).iter().any(|r| r.source_id == source));
}

#[test]
fn the_requirements_drawing_is_deterministic() {
    let (_, a) = analysed("thumbwheel_thread_knurl");
    let one = serde_json::to_string(&model::plan(&a, &Spec::default()).unwrap()).unwrap();
    let two = serde_json::to_string(&model::plan(&a, &Spec::default()).unwrap()).unwrap();
    assert_eq!(one, two);
}
