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
    analysed_bytes(
        name.strip_suffix(".gz").unwrap(),
        &gunzip(&std::fs::read(&gz).unwrap()),
    )
}

/// The STEP file *name* of *bytes*' part and Analysis, from its own scratch file.
fn analysed_bytes(name: &str, bytes: &[u8]) -> (draftwright::haecceity::Part, Analysis) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pmi_thread_knurl")
        .join(n.to_string());
    std::fs::create_dir_all(&scratch).unwrap();
    let path = scratch.join(name);
    std::fs::write(&path, bytes).unwrap();
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

/// The plan's item claiming *source* of *a* (by its item ID, `pmi:<identity>`), with its label.
fn claimed<'a>(plan: &'a Plan, a: &Analysis, source: &str) -> Vec<(&'a str, &'a str)> {
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("its PMI is read")
    };
    let feature = report.identities().ids[source].clone();
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
        .find(|p| p.code == "pmi_not_lowered" && p.refs.first().is_some_and(|s| s == source))
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
        claimed(&plan, &a, source),
        [("d:f:hole:2/bore.diameter", python.as_str())]
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
        ("manufacturing_requirement:#689", "d:f:hole:1/bore.diameter"),
        (
            "manufacturing_requirement:#722",
            "d:f:turned_step:4/step.diameter",
        ),
        (
            "manufacturing_requirement:#749",
            "d:f:turned_step:1/step.diameter",
        ),
    ] {
        let python = python_label(&drawing_py, source);
        assert_eq!(
            claimed(&plan, &a, source),
            [(id, python.as_str())],
            "{source}"
        );
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
    assert!(claimed(&plan, &b, source).is_empty());
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

/// The draftwright-editor's straight knurl (AP242 `turned_knurl`, pitch 1, 75 teeth) on the ⌀24
/// cylinder of draftwright.io's spindle sample, its text edited by *edit*. Beyond draftwright:
/// its Part 21 pass never reads a `turned_knurl`.
fn spindle(edit: impl Fn(String) -> String) -> (draftwright::haecceity::Part, Analysis) {
    let gz = root().join("tests/fixtures/turned_knurl/spindle_turned_knurl.step.gz");
    let text = String::from_utf8(gunzip(&std::fs::read(gz).unwrap())).unwrap();
    analysed_bytes("spindle_turned_knurl.step", edit(text).as_bytes())
}

#[test]
fn a_semantic_turned_knurl_prints_on_its_diameters_callout() {
    let (part, a) = spindle(|s| s);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let source = "manufacturing_requirement:#925";
    let labels: Vec<&str> = claimed(&plan, &a, source).iter().map(|c| c.1).collect();
    assert_eq!(labels, ["ø24 MAX AFTER KNURL; STRAIGHT KNURL P1"]);
    assert_eq!(refused(&plan, source), None);
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "pmi_not_extracted" && p.message.contains("turned_knurl")),
        "{:#?}",
        plan.problems
    );
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    assert!(
        pdf::svg_text(&drawing.svg())
            .join(" ")
            .contains("STRAIGHT KNURL P1")
    );
}

#[test]
fn a_turned_knurl_with_no_tooth_count_is_not_lowered() {
    // Nothing confirms that the diametral pitch is the circular pitch, so it is not printed.
    let (_, a) = spindle(|s| s.replace("#935,#937,#938", "#935,#938"));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let source = "manufacturing_requirement:#925";
    let why = refused(&plan, source).unwrap();
    assert!(
        why.ends_with("turned knurl has no tooth count to confirm its pitch."),
        "{why}"
    );
}

/// The draftwright-editor's diamond knurl (pitch 1, 75 teeth, helix angle 30°) on the same
/// spindle, its text edited by *edit*.
fn diamond(edit: impl Fn(String) -> String) -> (draftwright::haecceity::Part, Analysis) {
    let gz = root().join("tests/fixtures/turned_knurl/spindle_diamond_knurl.step.gz");
    let text = String::from_utf8(gunzip(&std::fs::read(gz).unwrap())).unwrap();
    analysed_bytes("spindle_diamond_knurl.step", edit(text).as_bytes())
}

#[test]
fn a_semantic_diamond_knurl_prints_on_its_diameters_callout() {
    let (_, a) = diamond(|s| s);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let source = "manufacturing_requirement:#925";
    let labels: Vec<&str> = claimed(&plan, &a, source).iter().map(|c| c.1).collect();
    assert_eq!(labels, ["ø24 MAX AFTER KNURL; DIAMOND KNURL P1"]);
    assert_eq!(refused(&plan, source), None);
}

#[test]
fn a_diagonal_turned_knurl_is_reported_not_drawn() {
    // A valid diagonal knurl (helix angle and hand stated) reaches the port, which has no
    // drawing for it: it stays a pmi_not_extracted error rather than a guessed callout.
    let (_, a) = diamond(|s| {
        s.replace("TURNED_KNURL('','diamond')", "TURNED_KNURL('','diagonal')")
            .replace(
                "#939,#942),#841);",
                "#939,#942,#99999),#841);\n#99999=DESCRIPTIVE_REPRESENTATION_ITEM('helix hand','right');",
            )
    });
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        plan.problems.iter().any(|p| p.code == "pmi_not_extracted"
            && p.message.contains("knurl:0:0 (turned_knurl)")
            && p.message.contains("not projected")),
        "{:#?}",
        plan.problems
    );
    assert!(plan.dimensions.iter().all(|d| !d.label.contains("KNURL")));
}

#[test]
fn a_turned_knurl_whose_pitch_and_teeth_disagree_is_not_lowered() {
    // π·24/1 is 75.4 teeth, not 60: the pitch is not the circular pitch the editor writes.
    let (_, a) = spindle(|s| s.replace("COUNT_MEASURE(75.)", "COUNT_MEASURE(60.)"));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let source = "manufacturing_requirement:#925";
    let why = refused(&plan, source).unwrap();
    assert!(
        why.ends_with("turned knurl pitch 1 and tooth count 60 disagree: π·24/1 is 75.4 teeth."),
        "{why}"
    );
    assert!(
        claimed(&plan, &a, source)
            .iter()
            .all(|c| !c.1.contains("KNURL"))
    );
}

#[test]
fn the_requirements_drawing_is_deterministic() {
    let (_, a) = analysed("thumbwheel_thread_knurl");
    let one = serde_json::to_string(&model::plan(&a, &Spec::default()).unwrap()).unwrap();
    let two = serde_json::to_string(&model::plan(&a, &Spec::default()).unwrap()).unwrap();
    assert_eq!(one, two);
}

/// The straight-knurl spindle with the draftwright-editor's external thread on its ⌀10 band,
/// hand-written as the editor's `pmi_writer.rs` writes one (AP242 `thread`, side external,
/// form M, fit class 6g, qualifier M10x1.5, and a 'thread manufacturing' attribute set with
/// pitch 1.5 and thread length 18, the band's length as specify-core states it).
fn external_thread() -> (draftwright::haecceity::Part, Analysis) {
    let gz = root().join("tests/fixtures/turned_knurl/spindle_external_thread.step.gz");
    analysed_bytes(
        "spindle_external_thread.step",
        &gunzip(&std::fs::read(gz).unwrap()),
    )
}

/// The lines of the drawing's thin layer (threads and knurls drawn over the outline), page mm.
fn thin_lines(svg: &str) -> Vec<[f64; 4]> {
    let Some(start) = svg.find("id=\"thin\"") else {
        return Vec::new();
    };
    let group = &svg[start..start + svg[start..].find("</g>").unwrap()];
    group
        .split("<line ")
        .skip(1)
        .map(|l| {
            ["x1", "y1", "x2", "y2"].map(|k| {
                let at = l.find(&format!("{k}=\"")).unwrap() + k.len() + 2;
                l[at..at + l[at..].find('"').unwrap()].parse().unwrap()
            })
        })
        .collect()
}

fn length(l: &[f64; 4]) -> f64 {
    (l[2] - l[0]).hypot(l[3] - l[1])
}

/// The scale of the plan's front view (the profile views share it).
fn front_scale(plan: &Plan) -> f64 {
    plan.views.iter().find(|v| v.id == "v:front").unwrap().scale
}

#[test]
fn a_semantic_external_thread_prints_its_designation_in_place_of_the_diameter() {
    let (part, a) = external_thread();
    let r = records(&a)
        .iter()
        .find(|r| r.kind == "external_thread")
        .expect("the AP242 external thread is projected");
    let source = r.source_id.clone();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let labels: Vec<&str> = claimed(&plan, &a, &source).iter().map(|c| c.1).collect();
    assert_eq!(labels, ["M10 x 1.5-6g RH, FULL AVAILABLE LENGTH"]);
    assert_eq!(refused(&plan, &source), None);
    // Neither the thread nor its manufacturing attribute set is reported not projected.
    assert!(
        !plan.problems.iter().any(|p| p.code == "pmi_not_extracted"
            && (p.message.contains("(thread)")
                || p.message.contains("(attribute set)")
                    && p.message.contains("general_property"))),
        "{:#?}",
        plan.problems
    );
    // The knurl still prints on its own diameter.
    let knurl = "manufacturing_requirement:#925";
    let labels: Vec<&str> = claimed(&plan, &a, knurl).iter().map(|c| c.1).collect();
    assert_eq!(labels, ["ø24 MAX AFTER KNURL; STRAIGHT KNURL P1"]);
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let svg = drawing.svg();
    assert!(pdf::svg_text(&svg).join(" ").contains("M10 x 1.5-6g RH"));
    let lint = draftwright::lint::lint(&a, &drawing);
    assert!(
        !lint.iter().any(|i| i.sources.contains(&source)),
        "{lint:#?}"
    );
}

#[test]
fn an_external_thread_is_drawn_at_its_minor_diameter_over_its_length() {
    let (part, a) = external_thread();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let s = front_scale(&plan);
    // ISO 6410: thin lines along the axis at d3 = 10 − 1.22687·1.5 over the 18 mm thread
    // length, one each side of the axis, in each profile view (front and side, not the
    // section).
    let minor = 10.0 - 1.226_87 * 1.5;
    let lines: Vec<[f64; 4]> = thin_lines(&drawing.svg())
        .into_iter()
        .filter(|l| (length(l) - 18.0 * s).abs() < 1e-6)
        .collect();
    let pairs = lines
        .iter()
        .enumerate()
        .flat_map(|(i, a)| lines[i + 1..].iter().map(move |b| (a, b)))
        .filter(|(a, b)| {
            (a[0] - a[2]).abs() < 1e-9
                && (b[0] - b[2]).abs() < 1e-9
                && (a[1] - b[1]).abs() < 1e-9
                && ((a[0] - b[0]).abs() - minor * s).abs() < 1e-3
        })
        .count();
    assert_eq!(pairs, 2, "{lines:?}");
}

#[test]
fn a_knurl_is_drawn_as_a_band_of_thin_lines_over_its_face() {
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    for (straight, (part, a)) in [(true, spindle(|s| s)), (false, diamond(|s| s))] {
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let s = front_scale(&plan);
        let drawing = model::render(&part, &a, &plan, &font).unwrap();
        let lines = thin_lines(&drawing.svg());
        assert!(lines.len() >= 6, "{lines:?}");
        // Over the middle half of the 12 mm knurled length, within the ⌀24 band's width.
        let (lo, hi) = lines.iter().fold((f64::MAX, f64::MIN), |(lo, hi), l| {
            (lo.min(l[1]).min(l[3]), hi.max(l[1]).max(l[3]))
        });
        assert!(hi - lo <= 6.0 * s + 1e-6, "{lo} {hi}");
        for l in &lines {
            let dx = (l[2] - l[0]).abs();
            let dy = (l[3] - l[1]).abs();
            if straight {
                // Along the (vertical) axis.
                assert!(dx < 1e-9, "{l:?}");
            } else {
                // Crossed at 30° to it.
                assert!((dx.atan2(dy).to_degrees() - 30.0).abs() < 1e-3, "{l:?}");
            }
            assert!(length(l) <= 6.0 * s / if straight { 1.0 } else { 0.75f64.sqrt() } + 1e-6);
        }
        if !straight {
            let rising = lines.iter().filter(|l| (l[2] - l[0]) * (l[3] - l[1]) > 0.0);
            assert!(rising.count() * 2 >= lines.len() - 2);
        }
    }
}

/// The editor's fragments: a thread's and knurl's thin lines belong to their view's fragment,
/// so they move with it, and none are left in the furniture; together the fragments still hold
/// every line of the sheet.
#[test]
fn thread_and_knurl_lines_travel_in_their_views_fragment() {
    let (part, a) = external_thread();
    let s = draftwright::session::Session::new(part, a, Spec::default()).unwrap();
    let full = s.drawing().svg();
    let fragments = s.svg_fragments(None);
    assert_eq!(
        full.matches("<line ").count(),
        fragments
            .values()
            .map(|f| f.matches("<line ").count())
            .sum::<usize>()
    );
    let total = thin_lines(&full).len();
    let front = thin_lines(&fragments["v:front"]).len();
    assert!(front > 0 && front < total, "{front} of {total}");
    assert_eq!(
        fragments
            .iter()
            .filter(|(id, _)| id.starts_with("v:"))
            .map(|(_, f)| thin_lines(f).len())
            .sum::<usize>(),
        total
    );
    assert!(thin_lines(&fragments["furniture"]).is_empty());
}

/// The thread's callout is led to the ⌀10 band's silhouette at an angle, not along it: a leader
/// lying on the part's edge is ambiguous (ISO 128-22). The spindle turns about z, which its
/// profile views draw upright, so the silhouette is vertical on the page; the default sheet and
/// a Spec-named A3 at 2:1 (where the leader once ran down the outline) are both checked.
#[test]
fn the_threads_leader_meets_the_silhouette_at_an_angle() {
    let (_, a) = external_thread();
    let a3: Spec = serde_json::from_value(serde_json::json!({
        "schema": "draftwright.spec/2",
        "sheet": { "size": "A3", "scale": "2:1" }
    }))
    .unwrap();
    for spec in [Spec::default(), a3] {
        let plan = model::plan(&a, &spec).unwrap();
        let d = plan
            .dimensions
            .iter()
            .find(|d| d.label.starts_with("M10 x 1.5-6g"))
            .expect("the thread's callout");
        let [tip, elbow] = d.placed.as_ref().expect("placed").lines[0];
        let (dx, dy) = ((elbow[0] - tip[0]).abs(), (elbow[1] - tip[1]).abs());
        assert!(
            dx.atan2(dy).to_degrees() >= 15.0 - 1e-9,
            "{}: {tip:?} {elbow:?}",
            plan.sheet.basis
        );
    }
}

/// A thread and a fit (J3, #30) on one part: the threaded diameter prints only its thread's
/// designation, which carries the thread's own tolerance class, and a fit or band given to it
/// is withheld and reported rather than spliced into the designation; a plain diameter keeps its
/// shaft fit with its ISO 286 limits.
#[test]
fn a_threaded_diameter_keeps_its_designation_and_a_plain_one_its_fit() {
    let gz = root().join("tests/fixtures/pmi/step/hw_thumbwheel_thread_knurl_intent.step.gz");
    let (_, a) = analysed_bytes(
        "hw_thumbwheel_thread_knurl_intent.step",
        &gunzip(&std::fs::read(gz).unwrap()),
    );
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let labels: Vec<&str> = plan.dimensions.iter().map(|d| d.label.as_str()).collect();
    assert!(labels.contains(&"ø10 f7 (-0.013/-0.028)"), "{labels:?}");
    assert!(
        labels.contains(&"M5 x 0.8-6g RH, FULL AVAILABLE LENGTH"),
        "{labels:?}"
    );

    let (_, a) = external_thread();
    let id = "d:f:turned_step:1/step.diameter";
    let spec: Spec = serde_json::from_value(serde_json::json!({
        "schema": "draftwright.spec/2",
        "dimensions": { id: { "tolerance": "+0 -0.1" } }
    }))
    .unwrap();
    let plan = model::plan(&a, &spec).unwrap();
    let d = plan.dimensions.iter().find(|d| d.id == id).unwrap();
    assert_eq!(d.label, "M10 x 1.5-6g RH, FULL AVAILABLE LENGTH");
    assert!(
        plan.problems
            .iter()
            .any(|p| p.code == "tolerance_withheld" && p.subjects == [id]),
        "{:?}",
        plan.problems
    );
}

/// A reference is only useful while its complete, source-bound row reaches the sheet.
#[test]
fn manufacturing_references_require_complete_source_matched_ink() {
    let (part, a) = analysed("thumbwheel_thread_knurl");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let original = model::render(&part, &a, &plan, &font).unwrap();
    let unresolved = |drawing: &draftwright::output::Drawing| {
        draftwright::lint::lint(&a, drawing)
            .into_iter()
            .filter(|i| i.code == "manufacturing_reference_unresolved")
            .collect::<Vec<_>>()
    };
    assert!(unresolved(&original).is_empty());
    for damage in ["header text", "header absent"] {
        let mut drawing = original.clone();
        let header = drawing
            .annotations
            .iter_mut()
            .find(|a| a.name == "manufacturing_requirements_header")
            .unwrap();
        if damage == "header text" {
            drawing.text[header.runs[0]].text = "WRONG HEADER".into();
        } else {
            header.runs.clear();
        }
        assert_eq!(unresolved(&drawing).len(), 3, "{damage}");
    }
    // Removing the whole table cannot leave the short leaders certified by provenance alone.
    let mut absent = original.clone();
    absent
        .annotations
        .retain(|a| !a.name.starts_with("manufacturing_requirements/"));
    absent
        .sheet
        .items
        .retain(|i| i.name != "manufacturing_requirements");
    assert_eq!(unresolved(&absent).len(), 3);
    let mut punctuated = absent.clone();
    for annotation in original.annotations.iter().filter(|a| {
        !a.name.starts_with("manufacturing_requirements/")
            && a.runs
                .iter()
                .any(|&i| original.text[i].text.contains("SEE MFG"))
    }) {
        let last = *annotation.runs.last().unwrap();
        punctuated.text[last].text.push('.');
    }
    assert_eq!(unresolved(&punctuated).len(), 3);
    let rows: Vec<_> = original
        .annotations
        .iter()
        .filter(|a| a.name.starts_with("manufacturing_requirements/"))
        .collect();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert!(
            row.claims.is_empty(),
            "a row is source provenance, not a second measurement owner"
        );
        assert_eq!(row.approved_claims.len(), 1);
        for damage in ["text", "empty", "source", "tag", "duplicate"] {
            let mut drawing = original.clone();
            let index = drawing
                .annotations
                .iter()
                .position(|a| a.name == row.name)
                .unwrap();
            match damage {
                "text" => {
                    drawing.text[row.runs[1]].text = "INCOMPLETE REQUIREMENT".into();
                }
                "empty" => drawing.annotations[index].runs.clear(),
                "source" => {
                    drawing.annotations[index].approved_claims[0].feature =
                        "pmi:wrong-source".into()
                }
                "tag" => drawing.text[row.runs[0]].text = "MFG 999".into(),
                "duplicate" => drawing.annotations.push(row.clone()),
                _ => unreachable!(),
            }
            let issues = unresolved(&drawing);
            assert_eq!(issues.len(), 1, "{} {damage}: {issues:?}", row.name);
            assert_eq!(issues[0].severity, draftwright::lint::Severity::Error);
            assert!(issues[0].sources.len() == 1 && issues[0].annotation.is_some());
        }
    }
}

#[test]
fn authored_manufacturing_rows_keep_the_compilers_source_bindings() {
    let (part, analysis) = analysed("thumbwheel_thread_knurl");
    let spec = model::edit(
        &Spec::default(),
        &serde_json::json!([{
            "op":"set", "path":"features/f:hole:1/thread",
            "value":{"designation":"M2x0.4-6H", "length":2.5}
        }]),
    )
    .unwrap();
    let plan = model::plan(&analysis, &spec).unwrap();
    assert!(
        plan.tables
            .iter()
            .any(|t| t.id == "manufacturing_requirements" && t.placed.is_some())
    );
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    // The renderer receives the original Analysis, exactly as the batch and session paths do.
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let effective = draftwright::spec_gdt::effective(&analysis, &spec).unwrap();
    let issues = draftwright::lint::lint(&effective, &drawing);
    assert!(
        !issues
            .iter()
            .any(|i| i.code == "manufacturing_reference_unresolved"),
        "{issues:?}"
    );
    let manufacturing = draftwright::pmi_lowering::manufacturing(&effective);
    let authored = manufacturing
        .owned
        .iter()
        .find(|o| o.source.starts_with("spec:"))
        .unwrap();
    assert!(drawing.annotations.iter().any(|a| {
        a.name.starts_with("manufacturing_requirements/")
            && a.approved_claims
                .iter()
                .any(|c| c.feature == authored.id && c.parameter == authored.aspect)
    }));
}
