//! The part's AP242 dimensions on the sheet (`draftwright::dims`' PMI dimensions, stage 2b)
//! against draftwright's, record by record, and their fail-closed paths, spec interplay, ink and
//! determinism.
//!
//! draftwright's outcome is in `tests/fixtures/pmi/dimensions/<stem>.json.gz`
//! (`tools/capture_pmi_dimensions.py`): for each dimension record of each AP242 file of
//! `tests/fixtures/pmi/step/`, what its `pmi="annotate"` drawing made of it: a placed term that
//! cites it (`Drawing.dimension_sources`), its own (`standalone`) or a recognised feature's
//! (`absorbed`, with the parameter it states), or the issues naming it (`refused`, or `dropped`
//! when placement found it no room). A draftwright record is paired with the port's as
//! `tests/pmi.rs` pairs them (by the Part 21 instance draftwright names, else by the faces), from
//! draftwright's reading in `tests/fixtures/pmi/<stem>.json.gz`. The outcome, the view and the
//! label (draftwright's `ø` read as the port's `⌀`) are compared; a record either side has no
//! pair for is a difference of field `record`. A common label, read as a dimension, is drawn as a
//! label and compared by `tests/pmi_drawings.rs`.
//!
//! Every difference must be listed, with a verdict and evidence, in
//! `tests/fixtures/known_pmi_dimensions.json`; a listed difference that has gone away must be
//! taken off.

#[path = "common/pdf.rs"]
mod pdf;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::dims::PlannedDimension;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::pmi::{Category, Reading, Record};
use serde::Deserialize;
use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> PathBuf {
    root().join("tests/fixtures/pmi")
}

fn gunzip(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(raw)
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The AP242 fixtures: `(file name as captured, stem, bytes)`.
fn inputs() -> Vec<(String, String, Vec<u8>)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures().join("step"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "gz"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let file = name.strip_suffix(".gz").unwrap().to_string();
            let stem = file.split('.').next().unwrap().to_string();
            (file, stem, gunzip(&std::fs::read(&p).unwrap()))
        })
        .collect()
}

fn json_gz(path: &Path) -> Value {
    serde_json::from_slice(&gunzip(&std::fs::read(path).unwrap())).unwrap()
}

/// The part and its Analysis, with Rust quiddity's features (the CLI's default, and what
/// draftwright's drawing correlates its PMI with), from *bytes* (written to a scratch file of its
/// own, as `analyse` reads a path and the tests run in parallel).
fn analysed(file: &str, bytes: &[u8]) -> (draftwright::haecceity::Part, Analysis) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pmi_dimensions")
        .join(n.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file);
    std::fs::write(&path, bytes).unwrap();
    let out = model::analyse_native(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

fn records(a: &Analysis) -> &[Record] {
    match &a.pmi {
        Some(Reading::Read(r)) => &r.records,
        other => panic!("not read: {other:?}"),
    }
}

/// The instance number of `#N`.
fn number(s: &str) -> Option<u64> {
    s.strip_prefix('#')?.parse().ok()
}

fn close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
        _ => false,
    }
}

/// draftwright's dimension records of *reading* paired with the port's *rust* records, as
/// `tests/pmi.rs` pairs them: draftwright's `source_id` to the port's record index.
fn pairs(reading: &Value, rust: &[Record]) -> BTreeMap<String, usize> {
    let records: Vec<&Value> = reading["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source_category"] == "dimension")
        .collect();
    let mut taken = vec![false; rust.len()];
    let mut out = BTreeMap::new();
    let id = |r: &Value| r["source_id"].as_str().unwrap().to_string();
    let dimension = |j: usize| rust[j].category == Category::Dimension;
    for rec in &records {
        if let Some(key) = number(rec["key"].as_str().unwrap_or(""))
            && let Some(j) =
                (0..rust.len()).find(|&j| !taken[j] && dimension(j) && rust[j].part21_id == key)
        {
            taken[j] = true;
            out.insert(id(rec), j);
        }
    }
    for rec in &records {
        if out.contains_key(&id(rec)) {
            continue;
        }
        let faces: BTreeSet<u64> = rec["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        if faces.is_empty() {
            continue;
        }
        let same = |j: usize| rust[j].face_ids.iter().copied().collect::<BTreeSet<_>>() == faces;
        let candidates: Vec<usize> = (0..rust.len())
            .filter(|&j| !taken[j] && dimension(j) && same(j))
            .collect();
        let best = candidates
            .iter()
            .copied()
            .find(|&j| {
                rust[j].kind == rec["kind"].as_str().unwrap()
                    && close(rust[j].value, rec["value"].as_f64())
            })
            .or_else(|| candidates.first().copied());
        if let Some(j) = best {
            taken[j] = true;
            out.insert(id(rec), j);
        }
    }
    out
}

/// draftwright's label as the port writes its symbols.
fn norm(label: &str) -> String {
    label.replace(['ø', 'Ø'], "⌀")
}

/// draftwright's outcome for a captured record: `standalone`, `absorbed`, `dropped` (placement
/// found it no room) or `refused`, with the view and label it was drawn with.
fn python_outcome(rec: &Value) -> (String, Value, Value) {
    let drawn = rec["drawn"].as_array().unwrap();
    if let Some(d) = drawn.first() {
        let outcome = if d["parameter"].is_null() {
            "standalone"
        } else {
            "absorbed"
        };
        let label = d["label"].as_str().map_or(Value::Null, |l| json!(norm(l)));
        return (outcome.into(), d["view"].clone(), label);
    }
    let codes: BTreeSet<&str> = rec["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap())
        .collect();
    if codes.contains("pmi_dropped") || codes.contains("callout_dropped") {
        return ("dropped".into(), Value::Null, Value::Null);
    }
    (
        "refused".into(),
        Value::Null,
        json!(codes.into_iter().collect::<Vec<_>>()),
    )
}

/// The item ID of the record *r* of *a* (`pmi:<identity>`), if it has one.
fn item_id(a: &Analysis, r: &Record) -> Option<String> {
    match &a.pmi {
        Some(Reading::Read(report)) => report.identities().ids.get(&r.source_id).cloned(),
        _ => None,
    }
}

/// The planned item that states the port's record, if any: the one claiming its item ID.
fn stating<'a>(plan: &'a Plan, a: &Analysis, r: &Record) -> Option<&'a PlannedDimension> {
    let feature = item_id(a, r)?;
    plan.dimensions
        .iter()
        .find(|d| d.measurements.iter().any(|m| m.feature == feature))
}

/// The port's outcome for its record, in draftwright's terms.
fn rust_outcome(plan: &Plan, a: &Analysis, r: &Record) -> (String, Value, Value) {
    if let Some(d) = stating(plan, a, r) {
        let own = item_id(a, r).is_some_and(|id| d.id.starts_with(&format!("d:{id}/")));
        if d.placed.is_none() {
            return ("dropped".into(), Value::Null, Value::Null);
        }
        let outcome = if own { "standalone" } else { "absorbed" };
        let view = d.view.strip_prefix("v:").unwrap_or(&d.view);
        return (outcome.into(), json!(view), json!(norm(&d.label)));
    }
    let codes: BTreeSet<&str> = plan
        .problems
        .iter()
        .filter(|p| p.refs.contains(&r.source_id))
        .map(|p| p.code.as_str())
        .collect();
    (
        "refused".into(),
        Value::Null,
        json!(codes.into_iter().collect::<Vec<_>>()),
    )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    file: String,
    /// draftwright's `source_id`, or the port's when draftwright has no such record.
    record: String,
    /// The port's record paired with draftwright's, or null.
    port: Value,
    field: String,
    python: Value,
    rust: Value,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    evidence: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Diff {
    file: String,
    record: String,
    port: Value,
    field: String,
    python: Value,
    rust: Value,
}

fn compare(file: &str, py: &Value, reading: &Value, a: &Analysis, plan: &Plan) -> Vec<Diff> {
    let rust = records(a);
    let paired = pairs(reading, rust);
    let mut diffs = Vec::new();
    let failed = py["error"].as_str();
    if let Some(e) = failed {
        let drawn = rust
            .iter()
            .filter(|r| r.category == Category::Dimension)
            .filter(|r| stating(plan, a, r).is_some_and(|d| d.placed.is_some()))
            .count();
        diffs.push(Diff {
            file: file.into(),
            record: "file".into(),
            port: Value::Null,
            field: "drawing".into(),
            python: json!(e),
            rust: json!(format!("{drawn} dimensions drawn")),
        });
    }
    let mut seen = BTreeSet::new();
    // A common label (read as a dimension) is drawn as a label, compared with draftwright's by
    // `tests/pmi_drawings.rs`.
    for rec in py["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|rec| rec["kind"] != "common_label")
    {
        let id = rec["source_id"].as_str().unwrap().to_string();
        let port = paired.get(&id).map(|&j| &rust[j]);
        let port_id = port.map_or(Value::Null, |r| json!(r.source_id));
        let mut diff = |field: &str, python: Value, rust: Value| {
            diffs.push(Diff {
                file: file.into(),
                record: id.clone(),
                port: port_id.clone(),
                field: field.into(),
                python,
                rust,
            })
        };
        let Some(r) = port else {
            diff(
                "record",
                json!({"kind": rec["kind"], "label": rec["label"]}),
                Value::Null,
            );
            continue;
        };
        seen.insert(r.source_id.clone());
        if failed.is_some() {
            continue;
        }
        let (po, pv, pl) = python_outcome(rec);
        let (ro, rv, rl) = rust_outcome(plan, a, r);
        if po != ro {
            diff(
                "outcome",
                json!({"outcome": po, "detail": pl}),
                json!({"outcome": ro, "detail": rl}),
            );
            continue;
        }
        if pv != rv {
            diff("view", pv, rv);
        }
        if po != "refused" && pl != rl {
            diff("label", pl, rl);
        }
    }
    for r in rust
        .iter()
        .filter(|r| r.category == Category::Dimension && r.kind != "common_label")
    {
        if !seen.contains(&r.source_id) {
            let (ro, _, rl) = rust_outcome(plan, a, r);
            diffs.push(Diff {
                file: file.into(),
                record: r.source_id.clone(),
                port: json!(r.source_id),
                field: "record".into(),
                python: Value::Null,
                rust: json!({"outcome": ro, "detail": rl}),
            });
        }
    }
    diffs
}

#[test]
fn pmi_dimensions_match_draftwright_or_are_known() {
    let known: Vec<Known> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/known_pmi_dimensions.json")).unwrap(),
    )
    .unwrap();
    let verdicts = [
        "rust-correct",
        "rust-wrong",
        "equivalent",
        "undetermined",
        "not-applicable",
    ];
    for k in &known {
        assert!(
            verdicts.contains(&k.verdict.as_str()) && !k.evidence.trim().is_empty(),
            "{k:?}"
        );
    }
    let mut diffs = Vec::new();
    let mut records = 0;
    for (file, stem, bytes) in inputs() {
        let py = json_gz(&fixtures().join(format!("dimensions/{stem}.json.gz")));
        let reading = json_gz(&fixtures().join(format!("{stem}.json.gz")));
        assert_eq!(py["file"], file.as_str());
        assert_eq!(
            py["sha256"],
            sha256(&bytes).as_str(),
            "{file}: not the captured file"
        );
        let (_, a) = analysed(&file, &bytes);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let d = compare(&file, &py, &reading, &a, &plan);
        records += py["records"].as_array().unwrap().len();
        eprintln!(
            "{file}: {} draftwright records, {} differences",
            py["records"].as_array().unwrap().len(),
            d.len()
        );
        diffs.extend(d);
    }
    let listed: Vec<Diff> = known
        .iter()
        .map(|k| Diff {
            file: k.file.clone(),
            record: k.record.clone(),
            port: k.port.clone(),
            field: k.field.clone(),
            python: k.python.clone(),
            rust: k.rust.clone(),
        })
        .collect();
    let keys: BTreeSet<(String, String, String)> = listed
        .iter()
        .map(|d| (d.file.clone(), d.record.clone(), d.field.clone()))
        .collect();
    assert_eq!(keys.len(), listed.len(), "a difference is listed twice");
    let unlisted: Vec<&Diff> = diffs.iter().filter(|d| !listed.contains(d)).collect();
    let stale: Vec<&Diff> = listed.iter().filter(|d| !diffs.contains(d)).collect();
    eprintln!(
        "{records} draftwright records; {} differences ({} known)",
        diffs.len(),
        listed.len()
    );
    let as_json = |v: &[&Diff]| {
        v.iter()
            .map(|d| {
                json!({"file": d.file, "record": d.record, "port": d.port, "field": d.field, "python": d.python, "rust": d.rust})
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known_pmi_dimensions.json:\n{}\nlisted but no longer differing:\n{}",
        as_json(&unlisted),
        as_json(&stale)
    );
}

/// A fixture by stem: its file name and bytes.
fn fixture(stem: &str) -> (String, Vec<u8>) {
    inputs()
        .into_iter()
        .find(|(_, s, _)| s == stem)
        .map(|(f, _, b)| (f, b))
        .unwrap()
}

fn planned(stem: &str) -> (draftwright::haecceity::Part, Analysis, Plan) {
    let (file, bytes) = fixture(stem);
    let (part, a) = analysed(&file, &bytes);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    (part, a, plan)
}

/// The codes that say a PMI dimension is not drawn, and why.
const REFUSALS: [&str; 3] = [
    "pmi_not_lowered",
    "pmi_dimensions_not_ported",
    "pmi_not_drawn",
];

#[test]
fn every_dimension_is_drawn_or_reported_never_both() {
    for (file, _, bytes) in inputs() {
        let (_, a) = analysed(&file, &bytes);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        for r in records(&a)
            .iter()
            .filter(|r| r.category == Category::Dimension && r.kind != "common_label")
        {
            let stated = stating(&plan, &a, r);
            // #33: what states the part's PMI is controlling, never a reference dimension.
            if let Some(d) = stated {
                assert!(!d.reference, "{file}: {} is drawn as reference", d.id);
            }
            let reported = plan
                .problems
                .iter()
                .any(|p| REFUSALS.contains(&p.code.as_str()) && p.refs.contains(&r.source_id));
            assert!(
                stated.is_some() != reported,
                "{file}: {} is stated ({}) or reported ({reported}), exactly one",
                r.source_id,
                stated.is_some()
            );
            // A record whose reading left blockers, or whose faces prove no witness, is never
            // drawn.
            let unproven = r.geometry.as_ref().is_some_and(|g| !g.reasons.is_empty());
            if !r.blockers.is_empty() || unproven {
                assert!(reported, "{file}: {} has blockers", r.source_id);
            }
            // An item stated in an undrawn place is said (view_plan_incomplete), not lost.
            if let Some(d) = stated
                && d.placed.is_none()
            {
                assert!(
                    plan.problems.iter().any(|p| p.subjects.contains(&d.id)),
                    "{file}: {} unplaced and unreported",
                    d.id
                );
            }
        }
        // A standalone item's basis says where it came from.
        for d in plan
            .dimensions
            .iter()
            .filter(|d| d.id.starts_with("d:pmi:"))
        {
            assert!(d.basis.starts_with("source-authored"), "{}", d.basis);
        }
    }
}

#[test]
fn the_open_questions_stay_refused() {
    // stc_10's ⌀3 holes are dimensioned on supplemental geometry; ftc_10's ⌀3.5 G6 states its
    // class without limits. Both are open maintainer questions.
    for (stem, needle) in [
        ("nist_stc_10_asme1_ap242-e2", "supplemental geometry"),
        ("nist_ftc_10_asme1_ap242-e2", "stated without limits"),
    ] {
        let (_, a, plan) = planned(stem);
        let open: Vec<&Record> = records(&a)
            .iter()
            .filter(|r| r.category == Category::Dimension)
            .filter(|r| r.blockers.iter().any(|b| b.contains(needle)))
            .collect();
        assert!(!open.is_empty(), "{stem}: {needle}");
        for r in open {
            assert!(stating(&plan, &a, r).is_none(), "{}", r.source_id);
            let p = plan
                .problems
                .iter()
                .find(|p| p.code == "pmi_not_lowered" && p.refs == [r.source_id.clone()])
                .unwrap();
            assert!(
                p.message.contains("open question for the maintainer"),
                "{}",
                p.message
            );
        }
    }
}

#[test]
fn a_toleranced_bore_annotates_its_callout_and_a_spec_tolerance_wins() {
    let (_, a, plan) = planned("spool_fits");
    let callout = plan
        .dimensions
        .iter()
        .find(|d| d.id == "d:f:hole_pattern:1/bore.diameter")
        .unwrap();
    assert_eq!(callout.label, "6× ⌀10 +0.09 -0.0 THRU EQ SP ON ø90 BC");
    assert!(callout.basis.contains("lower_ap242_hole_tolerances"));
    let feature = callout
        .measurements
        .iter()
        .find(|m| m.feature.starts_with("pmi:"))
        .unwrap()
        .feature
        .clone();
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("its PMI is read")
    };
    let source = report.identities().source(&feature).unwrap().to_string();
    // Stated once: no item of its own.
    assert!(
        !plan
            .dimensions
            .iter()
            .any(|d| d.id.starts_with(&format!("d:{feature}/")))
    );
    // The spec's own tolerance on that size wins, and the PMI one is said not drawn.
    let mut spec = Spec::default();
    spec.dimensions.insert(
        callout.id.clone(),
        serde_json::from_value(json!({"tolerance": "±0.05"})).unwrap(),
    );
    let edited = model::plan(&a, &spec).unwrap();
    let c = edited
        .dimensions
        .iter()
        .find(|d| d.id == callout.id)
        .unwrap();
    assert_eq!(c.label, "6× ⌀10 ±0.05 THRU EQ SP ON ø90 BC");
    assert!(
        c.measurements
            .iter()
            .all(|m| !m.feature.starts_with("pmi:"))
    );
    assert!(
        !c.basis.contains("lower_ap242_hole_tolerances"),
        "{}",
        c.basis
    );
    assert!(edited.problems.iter().any(|p| p.code == "pmi_not_lowered"
        && p.refs.contains(&source)
        && p.message.contains("wins over an imported one")));

    // A fit on every hole wins the same way, and only the fit is printed.
    let holes: Vec<String> = callout
        .measurements
        .iter()
        .filter(|m| m.parameter == "bore.diameter")
        .map(|m| m.feature.clone())
        .collect();
    assert_eq!(holes.len(), 6, "{holes:?}");
    let fitted = |on: &[String]| {
        let mut spec = Spec::default();
        for h in on {
            spec.features.insert(
                h.clone(),
                serde_json::from_value(json!({"fit": "H7"})).unwrap(),
            );
        }
        let edited = model::plan(&a, &spec).unwrap();
        let c = edited
            .dimensions
            .iter()
            .find(|d| d.id == callout.id)
            .unwrap()
            .clone();
        (c, edited.problems)
    };
    let (c, problems) = fitted(&holes);
    assert_eq!(c.label, "6× ⌀10 H7 (+0.015/0) THRU EQ SP ON ø90 BC");
    assert!(problems.iter().any(|p| p.code == "pmi_not_lowered"
        && p.refs.contains(&source)
        && p.message.contains("its own fit")
        && !p.message.contains("withheld")));
    // A fit on some of them is withheld: the label states neither, and the report says so.
    let (c, problems) = fitted(&holes[..1]);
    assert_eq!(c.label, "6× ⌀10 THRU EQ SP ON ø90 BC");
    assert!(problems.iter().any(|p| p.code == "tolerance_withheld"));
    assert!(problems.iter().any(|p| p.code == "pmi_not_lowered"
        && p.refs.contains(&source)
        && p.message.contains("withheld")));
}

#[test]
fn holes_of_different_requirements_are_called_out_apart() {
    let (_, _, plan) = planned("nist_ctc_01_asme1_ap242-e1");
    let labels: Vec<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.ends_with("/bore.diameter") && d.view == "v:front")
        .map(|d| d.label.as_str())
        .collect();
    assert!(labels.contains(&"⌀20 +0.1 -0.05 ↧ 45"), "{labels:?}");
    assert!(labels.contains(&"⌀20 +0.05 -0.1 ↧ 45"), "{labels:?}");
    // The 2×2 grid of ⌀35 whose holes carry three requirements is split too, its arrangement
    // dropped from each callout (its pitches still locate the holes).
    let plan_labels: Vec<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.ends_with("/bore.diameter") && d.view == "v:plan")
        .map(|d| d.label.as_str())
        .collect();
    for want in [
        "⌀35 +0.0 -0.2 THRU",
        "⌀35 +0.2 -0.0 THRU",
        "2× ⌀35 ±0.2 THRU",
    ] {
        assert!(plan_labels.contains(&want), "{want}: {plan_labels:?}");
    }
    assert!(
        plan.dimensions
            .iter()
            .any(|d| d.id.contains("hole_pattern:2/grid_pitch"))
    );
}

#[test]
fn an_omitted_standalone_dimension_is_said_not_drawn() {
    let (_, a, plan) = planned("string_post_tapped");
    let own = plan
        .dimensions
        .iter()
        .find(|d| d.id.starts_with("d:pmi:"))
        .unwrap()
        .clone();
    let mut spec = Spec::default();
    spec.dimensions.insert(
        own.id.clone(),
        serde_json::from_value(json!({"omit": true})).unwrap(),
    );
    let edited = model::plan(&a, &spec).unwrap();
    assert!(edited.dimensions.iter().all(|d| d.id != own.id));
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("its PMI is read")
    };
    let ids = report.identities();
    let source = ids.source(&own.measurements[0].feature).unwrap();
    assert!(
        edited
            .problems
            .iter()
            .any(|p| p.code == "pmi_not_drawn" && p.refs.iter().any(|s| s == source))
    );
}

#[test]
fn a_basic_dimension_is_boxed_in_every_format_and_searchable() {
    let (part, a, plan) = planned("string_post_tapped");
    let basic = plan
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter.ends_with(".basic"))
        })
        .unwrap();
    assert_eq!(basic.label, "1.2");
    let p = basic.placed.as_ref().unwrap();
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    // The box: the four sides of its label box, on the dims layer.
    let [x0, y0, x1, y1] = p.label_box;
    let svg = drawing.svg();
    let mut bare = plan.clone();
    for d in &mut bare.dimensions {
        d.measurements.iter_mut().for_each(|m| {
            if let Some(kind) = m.parameter.strip_suffix(".basic") {
                m.parameter = kind.to_string();
            }
        });
    }
    let unboxed = model::render(&part, &a, &bare, &font).unwrap();
    assert!(svg.len() > unboxed.svg().len(), "the box adds ink");
    assert!(x1 - x0 > 0.0 && y1 - y0 > 0.0);
    let written: Vec<String> = drawing.text.iter().map(|r| r.text.clone()).collect();
    assert!(written.iter().any(|t| t == "1.2"));
    assert_eq!(pdf::svg_text(&svg), written, "the SVG's text");
    let pdf_bytes = drawing.pdf(&|name: &str| std::fs::read(root().join("fonts").join(name)).ok());
    assert_eq!(pdf::pdf_text(&pdf_bytes), written, "the PDF's text");
    assert!(
        drawing.dxf().len() > unboxed.dxf().len(),
        "the DXF draws the box"
    );
    assert!(!drawing.png(100.0).unwrap().is_empty());
}

/// #34: the editor's spindle with a reviewer's PMI (`samples/cnc-review-spindle.step.gz`): the
/// ⌀4 cross hole's 23 from datum A (the end face) is basic. A z plane to an x bore's axis was not
/// proven (draftwright proves an x or y plane to a z bore only), so the default location stood in,
/// drawn plain. The PMI dimension now draws it, boxed, in the side view that sees the hole end on,
/// and no unboxed 23 is drawn.
#[test]
fn a_basic_location_of_a_turned_parts_cross_hole_is_boxed() {
    let raw =
        std::fs::read(root().join("tests/fixtures/samples/cnc-review-spindle.step.gz")).unwrap();
    let (part, a) = analysed("cnc-review-spindle.step", &gunzip(&raw));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let basic = |d: &PlannedDimension| {
        d.measurements
            .iter()
            .any(|m| m.feature.starts_with("pmi:") && m.parameter.ends_with(".basic"))
    };
    let drawn: Vec<&PlannedDimension> = plan
        .dimensions
        .iter()
        .filter(|d| d.label == "23" && d.placed.is_some())
        .collect();
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    let d = drawn[0];
    assert!(basic(d), "{d:?}");
    assert_eq!(d.view, "v:side");
    // It claims the default location it conveys.
    assert!(
        d.measurements
            .iter()
            .any(|m| m.feature == "f:hole:2" && m.parameter == "location.length.z"),
        "{d:?}"
    );
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| REFUSALS.contains(&p.code.as_str()) && p.message.contains("(23)")),
        "{:?}",
        plan.problems
    );
    // The box is drawn: the plan's ink is more than the same plan with the basic status taken off.
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let mut bare = plan.clone();
    for d in &mut bare.dimensions {
        d.measurements.iter_mut().for_each(|m| {
            if let Some(kind) = m.parameter.strip_suffix(".basic") {
                m.parameter = kind.to_string();
            }
        });
    }
    let boxed = model::render(&part, &a, &plan, &font).unwrap();
    let unboxed = model::render(&part, &a, &bare, &font).unwrap();
    assert!(boxed.svg().len() > unboxed.svg().len(), "the box adds ink");
}

/// The drawn marks of *plan*, as lint reads them back from the sheet.
fn marks(plan: &Plan) -> Vec<draftwright::lint::Mark> {
    plan.dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .map(|d| draftwright::lint::Mark {
            name: d.id.clone(),
            view: Some(d.view.clone()),
            text: d.label.clone(),
            claims: d
                .measurements
                .iter()
                .map(|m| draftwright::output::Claim {
                    feature: m.feature.clone(),
                    parameter: m.parameter.clone(),
                })
                .collect(),
        })
        .collect()
}

/// The drawn labels of *plan*'s dimensions along z in the front view (the spindle's axis).
fn front_z_labels(plan: &Plan) -> Vec<String> {
    plan.dimensions
        .iter()
        .filter(|d| d.placed.is_some() && d.view == "v:front" && d.kind == "linear")
        .map(|d| d.label.clone())
        .collect()
}

/// #33: the editor's spindle names its bottom end face datum A, and its inspection list measures
/// the shoulders from A. Its step lengths were a chain (22, 18) with the 12 as reference `(12)`;
/// they are now a baseline from A: 12, 34 and the overall 52, none in brackets, and lint finds
/// every band's length determined.
#[test]
fn a_turned_profile_is_dimensioned_from_its_datum_end() {
    let raw =
        std::fs::read(root().join("tests/fixtures/samples/cnc-review-spindle.step.gz")).unwrap();
    let (_, a) = analysed("cnc-review-spindle.step", &gunzip(&raw));
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let labels = front_z_labels(&plan);
    for want in ["12", "34", "52"] {
        let d = plan
            .dimensions
            .iter()
            .find(|d| d.placed.is_some() && d.view == "v:front" && d.label == want)
            .unwrap_or_else(|| panic!("{want} is not drawn: {labels:?}"));
        assert!(d.basis.contains("datum A"), "{}: {}", d.id, d.basis);
        assert!(!d.reference, "{}", d.id);
    }
    for gone in ["22", "18", "(12)"] {
        assert!(!labels.iter().any(|l| l == gone), "{gone}: {labels:?}");
    }
    assert!(
        !plan
            .dimensions
            .iter()
            .any(|d| d.reference || d.label.starts_with('(')),
        "{labels:?}"
    );
    let issues = draftwright::lint::check(&a, &marks(&plan));
    assert!(
        !issues.iter().any(|i| matches!(
            i.code.as_str(),
            "axial_length_missing"
                | "turned_step_requirement_missing"
                | "turned_step_requirement_unverifiable"
                | "closed_dimension_chain"
                | "duplicate_measurement"
        )),
        "{issues:?}"
    );
    // The band lengths the baseline conveys keep their IDs: the editor's saved spec, which places
    // the ⌀15 band's length, still plans, and the plan says what conveys it.
    let spec: Spec = serde_json::from_value(json!({
        "schema": "draftwright.spec/2",
        "dimensions": {"d:f:turned_step:2/step.length": {"place": {
            "along": "outside", "lane": 0, "side": "left", "view": "front"}}}
    }))
    .unwrap();
    let placed = model::plan(&a, &spec).unwrap();
    assert_eq!(front_z_labels(&placed), labels);
    let covered = plan
        .omitted
        .iter()
        .find(|o| o.id == "d:f:turned_step:2/step.length")
        .unwrap();
    assert!(
        covered
            .basis
            .contains("d:f:turned_step:2/step_position.length"),
        "{}",
        covered.basis
    );
}

/// #33, the case the issue describes: the PMI states the spindle's heights from datum A (12 ±0.2,
/// 34 ±0.3, 52 ±0.3 to its shoulders and end, 42 ±0.3 to the tapped hole's floor). They are drawn
/// as they are stated, from A, every one controlling, and no chain restates the steps between
/// them. With datum A taken away the lengths alone establish the baseline (draftwright's
/// `_authored_step_baseline`).
#[test]
fn pmi_heights_from_a_datum_are_drawn_as_a_baseline_not_a_chain() {
    let raw =
        std::fs::read(root().join("tests/fixtures/samples/cnc-review-spindle.step.gz")).unwrap();
    let (_, base) = analysed("cnc-review-spindle.step", &gunzip(&raw));
    let template = records(&base)
        .iter()
        .find(|r| r.category == Category::Dimension && r.kind == "linear")
        .unwrap()
        .clone();
    for keep_datum in [true, false] {
        let mut a = base.clone();
        let Some(Reading::Read(report)) = &mut a.pmi else {
            unreachable!()
        };
        if !keep_datum {
            report.records.retain(|r| r.category != Category::Datum);
        }
        for (n, (to, tol)) in [(12.0, 0.2), (34.0, 0.3), (42.0, 0.3), (52.0, 0.3)]
            .into_iter()
            .enumerate()
        {
            let mut r = template.clone();
            let id = 9001 + n as u64;
            r.source_id = format!("dimension:#{id}");
            r.part21_id = id;
            r.instances = vec![id];
            r.value = Some(to);
            r.upper_tol = Some(tol);
            r.lower_tol = Some(-tol);
            r.basic = false;
            r.label = format!("{to} ±{tol}");
            // Faces of their own, so each record has an identity of its own.
            r.faces = vec![10, 100 + n];
            r.face_groups = vec![vec![10], vec![100 + n]];
            r.ref_bbox = Some([-2.0, -2.0, 0.0, 2.0, 2.0, to]);
            let g = r.geometry.as_mut().unwrap();
            g.reasons.clear();
            g.view = None;
            g.axis = Some("Z".into());
            g.stations = vec![[0.0, 0.0, 0.0], [0.0, 0.0, to]];
            report.records.push(r);
        }
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let labels = front_z_labels(&plan);
        for want in ["12 ±0.2", "34 ±0.3", "42 ±0.3", "52 ±0.3"] {
            let d = plan
                .dimensions
                .iter()
                .find(|d| d.placed.is_some() && d.label == want)
                .unwrap_or_else(|| panic!("datum {keep_datum}: {want} not drawn: {labels:?}"));
            assert!(!d.reference, "{}", d.id);
        }
        for gone in ["22", "18", "(12)", "12", "34", "52"] {
            assert!(
                !labels.iter().any(|l| l == gone),
                "datum {keep_datum}: {gone}: {labels:?}"
            );
        }
        assert!(
            !plan
                .dimensions
                .iter()
                .any(|d| d.reference || d.label.starts_with('(')),
            "datum {keep_datum}: {labels:?}"
        );
    }
}

#[test]
fn what_the_faces_do_not_prove_is_reported_not_guessed() {
    let (_, a, _) = planned("nist_stc_10_asme1_ap242-e2");
    let source = records(&a)
        .iter()
        .find(|r| r.category == Category::Dimension && r.kind == "linear" && r.blockers.is_empty())
        .unwrap()
        .source_id
        .clone();
    let edit = |f: &dyn Fn(&mut Record)| {
        let mut b = a.clone();
        let Some(Reading::Read(r)) = &mut b.pmi else {
            unreachable!()
        };
        r.records
            .iter_mut()
            .filter(|r| r.source_id == source)
            .for_each(f);
        model::plan(&b, &Spec::default()).unwrap()
    };
    let reason = |plan: &Plan| {
        plan.problems
            .iter()
            .find(|p| p.refs == [source.clone()])
            .map(|p| (p.code.clone(), p.message.clone()))
            .unwrap()
    };
    let plan = edit(&|r| {
        r.geometry
            .as_mut()
            .unwrap()
            .reasons
            .push("linear reference groups occupy the same station".into())
    });
    assert_eq!(reason(&plan).0, "pmi_not_lowered");
    let plan = edit(&|r| r.geometry.as_mut().unwrap().axis = None);
    assert_eq!(reason(&plan).0, "pmi_dimensions_not_ported");
    assert!(reason(&plan).1.contains("oblique"));
    let plan = edit(&|r| r.geometry = None);
    assert!(reason(&plan).1.contains("analyse the part again"));
    let plan = edit(&|r| r.kind = "angular".into());
    assert!(reason(&plan).1.contains("angle"));
}

#[test]
fn an_external_diameter_annotates_the_turned_size_with_its_signs() {
    // spool_fits' two ø130 flanges (a shaft band wholly below its nominal) and its ø70 step.
    let (_, _, plan) = planned("spool_fits");
    let label = |id: &str| {
        plan.dimensions
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.label.clone())
            .unwrap()
    };
    assert_eq!(label("d:envelope/od.diameter"), "2× ø130 -0.014 -0.039");
    assert_eq!(
        label("d:f:turned_step:3/step.diameter"),
        "ø70 -0.03 -0.06 OD"
    );
    // bolt_thread_knurl's shank: the turned step's ø6 takes its band, and nothing restates it.
    let (_, _, plan) = planned("bolt_thread_knurl");
    let d = plan
        .dimensions
        .iter()
        .find(|d| d.id == "d:f:turned_step:3/step.diameter")
        .unwrap();
    assert_eq!(d.label, "ø6 +0.0 -0.008");
    assert!(d.basis.contains("lower_ap242_external_diameter_tolerances"));
    assert!(!plan.dimensions.iter().any(|d| d.id.starts_with("d:pmi:")));
}

#[test]
fn a_shaft_fit_is_printed_on_its_turned_diameter_with_its_iso_286_limits() {
    // #30: the editor's spindle states its ø15 journal as LIMITS_AND_FITS('g','shaft','6')
    // with no limits; the turned step prints the class and its ISO 286-2 limits, as a hole fit
    // prints on its bore's callout (`ø 4 H7 (+0.012/0) THRU` on the same part).
    let bytes = gunzip(
        &std::fs::read(root().join("tests/fixtures/samples/spindle_shaft_fit.step.gz")).unwrap(),
    );
    let (_, a) = analysed("spindle_shaft_fit.step", &bytes);
    let fit = records(&a)
        .iter()
        .find(|r| r.fit.as_deref() == Some("g6"))
        .unwrap();
    assert_eq!(fit.blockers, Vec::<String>::new());
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let d = plan
        .dimensions
        .iter()
        .find(|d| d.label.starts_with("ø15"))
        .unwrap();
    assert_eq!(d.label, "ø15 g6 (-0.006/-0.017)");
    assert!(d.placed.is_some());
    assert!(d.basis.contains("lower_ap242_external_diameter_tolerances"));
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| REFUSALS.contains(&p.code.as_str()) && p.refs.contains(&fit.source_id)),
        "the g6 fit is drawn, not refused"
    );
    assert!(
        plan.dimensions
            .iter()
            .any(|d| d.label.contains("H7 (+0.012/0)")),
        "the hole fit still prints"
    );
    // `sheet/fit_limits: false` drops the limits and keeps the class, as for a hole fit.
    let mut quiet = Spec::default();
    quiet.sheet.fit_limits = Some(false);
    let plan = model::plan(&a, &quiet).unwrap();
    assert!(plan.dimensions.iter().any(|d| d.label == "ø15 g6"));
}

#[test]
fn the_same_part_gives_the_same_plan_and_bytes() {
    for stem in ["assembly_plate_pin", "nist_ftc_10_asme1_ap242-e2"] {
        let (part, a, plan) = planned(stem);
        let again = model::plan(&a, &Spec::default()).unwrap();
        assert_eq!(
            serde_json::to_string(&plan).unwrap(),
            serde_json::to_string(&again).unwrap(),
            "{stem}"
        );
        let font =
            draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT))
                .unwrap();
        let one = model::render(&part, &a, &plan, &font).unwrap();
        let two = model::render(&part, &a, &again, &font).unwrap();
        assert_eq!(one.svg(), two.svg(), "{stem}");
        assert_eq!(one.dxf(), two.dxf(), "{stem}");
        // The Analysis round-trips through JSON with its dimensions' geometry, and plans the
        // same.
        let text = serde_json::to_string(&a).unwrap();
        let back: Analysis = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), text, "{stem}");
        let replanned = model::plan(&back, &Spec::default()).unwrap();
        assert_eq!(
            serde_json::to_string(&replanned).unwrap(),
            serde_json::to_string(&plan).unwrap(),
            "{stem}"
        );
    }
}

/// One span, one dimension for PMI (`share_pmi`): the plate states each grid hole's basic
/// location, so a row or column states the same span once per hole, and the default hole
/// locations measure spans the PMI states. The plan view draws each span once: no two of its
/// placed PMI or location dimensions share a label and value (every one of them here runs from
/// the part's left or front face), and every basic record is claimed by a planned dimension.
#[test]
fn a_span_the_pmi_states_is_drawn_once() {
    let (_, a, plan) = planned("hw_assembly_plate_pin_intent");
    let mut seen = BTreeSet::new();
    for d in plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some() && d.view == "v:plan")
    {
        // The PMI dimensions and the default locations; a pitch of the same value spans
        // something else.
        if !(d.id.contains("/linear") || d.id.contains("/location.")) {
            continue;
        }
        assert!(
            seen.insert((d.label.clone(), d.value.to_bits())),
            "{} repeats {:?} in the plan view",
            d.id,
            d.label
        );
    }
    let conveyed: Vec<_> = plan
        .omitted
        .iter()
        .filter(|o| o.basis.contains("the part's PMI"))
        .collect();
    assert!(conveyed.len() >= 8, "{conveyed:?}");
    for r in records(&a).iter().filter(|r| r.kind == "linear" && r.basic) {
        assert!(
            stating(&plan, &a, r).is_some(),
            "{} is claimed by a planned dimension",
            r.source_id
        );
    }
}

/// Geometry and AP242 values can differ below a micrometre. The PMI states the requirement;
/// one line carries it and the default measurement, even when one is basic or in inches.
#[test]
fn a_basic_pmi_span_covers_defaults_despite_geometric_roundoff() {
    let (_, a, plan) = planned("bracket_positions");
    for feature in ["f:hole:2", "f:hole:4"] {
        let d = plan
            .dimensions
            .iter()
            .find(|d| {
                d.measurements
                    .iter()
                    .any(|m| m.feature == feature && m.parameter == "location.length.x")
            })
            .unwrap();
        assert!(d.id.starts_with("d:pmi:"), "{d:?}");
        assert!(d.measurements.iter().any(|m| m.parameter == "linear.basic"));
        assert!(
            !plan
                .dimensions
                .iter()
                .any(|other| other.id == format!("d:{feature}/location.length.x"))
        );
    }
    for r in records(&a).iter().filter(|r| {
        r.kind == "linear"
            && r.basic
            && [[234, 262], [235, 262], [236, 262]]
                .iter()
                .any(|faces| r.face_ids == *faces)
    }) {
        assert!(stating(&plan, &a, r).is_some(), "{}", r.source_id);
    }
}

#[test]
fn an_inch_pmi_span_covers_the_default_extent_despite_geometric_roundoff() {
    let (_, a, plan) = planned("nist_stc_06_asme1_ap242-e3");
    let d = plan
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.feature == "envelope" && m.parameter == "depth.length")
        })
        .unwrap();
    assert!(d.id.starts_with("d:pmi:"), "{d:?}");
    assert_eq!(d.label, "12.00 ±0.01 INCH");
    let mut spec = Spec::default();
    spec.dimensions.insert(
        "d:envelope/depth.length".into(),
        serde_json::from_value(json!({"tolerance": "±0.1"})).unwrap(),
    );
    let separate = model::plan(&a, &spec).unwrap();
    assert!(
        separate
            .dimensions
            .iter()
            .any(|d| d.id == "d:envelope/depth.length" && d.label == "304.8 ±0.1")
    );
    assert!(
        separate
            .dimensions
            .iter()
            .any(|d| d.label == "12.00 ±0.01 INCH")
    );
}
