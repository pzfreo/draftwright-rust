//! The port's PMI lowering ([`draftwright::pmi_lowering`]) and its feature control frames and
//! datum feature symbols ([`draftwright::dims::gdt`]) against draftwright's, item by item, and
//! their fail-closed paths, text layer and determinism.
//!
//! draftwright's lowering is in `tests/fixtures/pmi/lowering/<stem>.json.gz`
//! (`tools/capture_pmi_lowering.py`): `build_pmi_features` on each AP242 file of
//! `tests/fixtures/pmi/step/`, one item per geometric-tolerance or datum record, and what its
//! `pmi="annotate"` drawing placed and dropped. A draftwright record is paired with the port's
//! as `tests/pmi.rs` pairs them (by the Part 21 instance draftwright names, else by the faces,
//! else a datum by its letter), from draftwright's reading in `tests/fixtures/pmi/<stem>.json.gz`.
//! A paired item's outcome (`control_frame`, `datum_ref`, or `raw`: not lowered), view, strip,
//! what it states, scope, surface, site (within 1e-3 mm) and whether it is drawn are compared;
//! an item either side has no pair for is a difference of field `record`.
//!
//! Every difference must be listed, with a verdict and evidence, in
//! `tests/fixtures/known_pmi_lowering.json`; a listed difference that has gone away must be
//! taken off.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/pdf.rs"]
mod pdf;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::dims::gdt::PlannedGdt;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::pmi::{Category, Reading, Record};
use draftwright::pmi_lowering::{Glyph, Lowered, lower};
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

/// The part and its Analysis, without features, from *bytes* (written to a scratch file of its
/// own, as `analyse` reads a path and the tests run in parallel).
fn analysed(file: &str, bytes: &[u8]) -> (draftwright::haecceity::Part, Analysis) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pmi_lowering")
        .join(n.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file);
    std::fs::write(&path, bytes).unwrap();
    let out = model::analyse(&path).unwrap();
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

fn category(c: &str) -> Option<Category> {
    match c {
        "geometric_tolerance" => Some(Category::GeometricTolerance),
        "datum" => Some(Category::Datum),
        _ => None,
    }
}

/// draftwright's faces for *rec*: XCAF's, and for a datum its Part 21 support too.
fn python_faces(rec: &Value) -> BTreeSet<u64> {
    let mut out: BTreeSet<u64> = rec["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    if rec["source_category"] == "datum" {
        out.extend(
            rec["reference_item_ids"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| number(v.as_str().unwrap())),
        );
    }
    out
}

fn close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
        _ => false,
    }
}

/// draftwright's geometric-tolerance and datum records of *py* (its reading) paired with the
/// port's *rust* records, as `tests/pmi.rs` pairs them: draftwright's `source_id` to the
/// port's record index.
fn pairs(py: &Value, rust: &[Record]) -> BTreeMap<String, usize> {
    let records: Vec<&Value> = py["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| category(r["source_category"].as_str().unwrap()).is_some())
        .collect();
    let mut taken = vec![false; rust.len()];
    let mut out = BTreeMap::new();
    let id = |r: &Value| r["source_id"].as_str().unwrap().to_string();
    for rec in &records {
        let c = category(rec["source_category"].as_str().unwrap()).unwrap();
        if let Some(key) = number(rec["key"].as_str().unwrap())
            && let Some(j) = (0..rust.len())
                .find(|&j| !taken[j] && rust[j].category == c && rust[j].part21_id == key)
        {
            taken[j] = true;
            out.insert(id(rec), j);
        }
    }
    for rec in &records {
        if out.contains_key(&id(rec)) {
            continue;
        }
        let c = category(rec["source_category"].as_str().unwrap()).unwrap();
        let faces = python_faces(rec);
        if faces.is_empty() {
            continue;
        }
        let candidates: Vec<usize> = (0..rust.len())
            .filter(|&j| {
                !taken[j]
                    && rust[j].category == c
                    && rust[j].face_ids.iter().copied().collect::<BTreeSet<_>>() == faces
            })
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
    for rec in &records {
        if out.contains_key(&id(rec)) {
            continue;
        }
        let c = category(rec["source_category"].as_str().unwrap()).unwrap();
        let faces = python_faces(rec);
        if faces.is_empty() {
            continue;
        }
        if let Some(j) = (0..rust.len()).find(|&j| {
            !taken[j]
                && rust[j].category == c
                && rust[j].kind == rec["kind"].as_str().unwrap()
                && close(rust[j].value, rec["value"].as_f64())
                && faces.iter().all(|f| rust[j].face_ids.contains(f))
        }) {
            taken[j] = true;
            out.insert(id(rec), j);
        }
    }
    for rec in &records {
        if out.contains_key(&id(rec)) || rec["source_category"] != "datum" {
            continue;
        }
        if let Some(j) = (0..rust.len()).find(|&j| {
            !taken[j]
                && rust[j].category == Category::Datum
                && rust[j].label == rec["label"].as_str().unwrap()
        }) {
            taken[j] = true;
            out.insert(id(rec), j);
        }
    }
    out
}

/// What a frame or symbol states, in one line: `position ⌀0.75Ⓜ A B C`, or a datum's letter.
fn python_text(item: &Value) -> Value {
    if item["lowered"] == "datum_ref" {
        return item["letter"].clone();
    }
    let zone = if item["diameter"] == true {
        "⌀"
    } else if item["spherical_diameter"] == true {
        "S⌀"
    } else {
        ""
    };
    let material = match item["modifier"].as_str() {
        Some("M") => "Ⓜ",
        Some("L") => "Ⓛ",
        _ => "",
    };
    let mut s = format!(
        "{} {zone}{}{material}",
        item["characteristic"].as_str().unwrap(),
        item["tolerance"].as_str().unwrap()
    );
    for d in item["datums"].as_array().unwrap() {
        s += &format!(" {}", d.as_str().unwrap());
    }
    json!(s)
}

fn rust_text(l: &Lowered) -> Value {
    match &l.glyph {
        Glyph::Datum { letter, .. } => json!(letter),
        Glyph::ControlFrame {
            characteristic,
            tolerance,
            zone,
            material,
            datums,
            ..
        } => {
            let zone = match zone {
                Some("diameter") => "⌀",
                Some(_) => "S⌀",
                None => "",
            };
            let material = match material {
                Some("M") => "Ⓜ",
                Some(_) => "Ⓛ",
                None => "",
            };
            let mut s = format!("{characteristic} {zone}{tolerance}{material}");
            for d in datums {
                s += &format!(" {d}");
            }
            json!(s)
        }
    }
}

fn python_scope(item: &Value) -> Value {
    json!({"all_around": item["all_around"], "all_over": item["all_over"]})
}

fn rust_scope(l: &Lowered) -> Value {
    match &l.glyph {
        Glyph::ControlFrame {
            all_around,
            all_over,
            ..
        } => json!({"all_around": all_around, "all_over": all_over}),
        Glyph::Datum { .. } => Value::Null,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    file: String,
    /// draftwright's `source_id`, or the port's when draftwright has no such item.
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

/// The views a default plan of *a* shows, as lowering asks.
fn shown_in(plan: &Plan) -> impl Fn(&str) -> bool + '_ {
    move |name: &str| plan.views.iter().any(|v| v.id == format!("v:{name}"))
}

fn compare(file: &str, py: &Value, reading: &Value, a: &Analysis, plan: &Plan) -> Vec<Diff> {
    let rust = records(a);
    let paired = pairs(reading, rust);
    let (lowered, problems) = lower(a, &shown_in(plan));
    let by_source: BTreeMap<&str, &Lowered> =
        lowered.iter().map(|l| (l.source.as_str(), l)).collect();
    let planned: BTreeMap<&str, &PlannedGdt> =
        plan.gdt.iter().map(|g| (g.source.as_str(), g)).collect();
    let mut diffs = Vec::new();
    // draftwright could not draw the part at all: one difference for the file.
    let failed = py["drawn"]["error"].as_str();
    if let Some(e) = failed {
        diffs.push(Diff {
            file: file.into(),
            record: "file".into(),
            port: Value::Null,
            field: "drawing".into(),
            python: json!(e),
            rust: json!(format!(
                "{} items drawn",
                plan.gdt.iter().filter(|g| g.placed.is_some()).count()
            )),
        });
    }
    let dropped: BTreeSet<&str> = py["drawn"]["dropped"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut seen = BTreeSet::new();
    for item in py["items"].as_array().unwrap() {
        let id = item["source_id"].as_str().unwrap().to_string();
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
                json!({"category": item["category"], "kind": item["kind"], "lowered": item["lowered"]}),
                Value::Null,
            );
            continue;
        };
        seen.insert(r.source_id.clone());
        let l = by_source.get(r.source_id.as_str()).copied();
        let outcome = match l.map(|l| &l.glyph) {
            Some(Glyph::ControlFrame { .. }) => "control_frame",
            Some(Glyph::Datum { .. }) => "datum_ref",
            None => "raw",
        };
        if item["lowered"] != outcome {
            let why = problems
                .iter()
                .find(|p| p.subjects.contains(&r.source_id))
                .map_or(Value::Null, |p| json!(p.code));
            diff(
                "lowered",
                item["lowered"].clone(),
                json!({"lowered": outcome, "problem": why}),
            );
            continue;
        }
        let Some(l) = l else { continue };
        if item["view"] != l.view {
            diff("view", item["view"].clone(), json!(l.view));
        }
        if item["side"] != l.side {
            diff("side", item["side"].clone(), json!(l.side));
        }
        let (pt, rt) = (python_text(item), rust_text(l));
        if pt != rt {
            diff("text", pt, rt);
        }
        if item["lowered"] == "control_frame" {
            let (ps, rs) = (python_scope(item), rust_scope(l));
            if ps != rs {
                diff("scope", ps, rs);
            }
        } else if let Glyph::Datum { surface, .. } = &l.glyph
            && item["surface"] != surface.as_str()
        {
            diff("surface", item["surface"].clone(), json!(surface));
        }
        let site: Vec<f64> = item["site"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        if (0..3).any(|k| (site[k] - l.site[k]).abs() > 1e-3) {
            let round = |v: f64| (v * 1e4).round() / 1e4;
            diff(
                "site",
                json!(site.iter().map(|v| round(*v)).collect::<Vec<_>>()),
                json!(l.site.map(round)),
            );
        }
        let drawn = planned
            .get(r.source_id.as_str())
            .is_some_and(|g| g.placed.is_some());
        let python_drawn = !dropped.contains(id.as_str());
        if failed.is_none() && drawn != python_drawn {
            diff("drawn", json!(python_drawn), json!(drawn));
        }
    }
    for l in &lowered {
        if !seen.contains(&l.source) {
            diffs.push(Diff {
                file: file.into(),
                record: l.source.clone(),
                port: json!(l.source),
                field: "record".into(),
                python: Value::Null,
                rust: json!({"lowered": l.id, "text": rust_text(l)}),
            });
        }
    }
    diffs
}

#[test]
fn pmi_lowering_matches_draftwright_or_is_known() {
    let known: Vec<Known> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/known_pmi_lowering.json")).unwrap(),
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
    let mut items = 0;
    for (file, stem, bytes) in inputs() {
        let py = json_gz(&fixtures().join(format!("lowering/{stem}.json.gz")));
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
        items += py["items"].as_array().unwrap().len();
        eprintln!(
            "{file}: {} draftwright items, {} port items, {} differences",
            py["items"].as_array().unwrap().len(),
            plan.gdt.len(),
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
        "{items} draftwright items; {} differences ({} known)",
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
        "differences not in known_pmi_lowering.json:\n{}\nlisted but no longer differing:\n{}",
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

/// The planted fixtures' parts and default plans.
fn planned(stem: &str) -> (draftwright::haecceity::Part, Analysis, Plan) {
    let (file, bytes) = fixture(stem);
    let (part, a) = analysed(&file, &bytes);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    (part, a, plan)
}

#[test]
fn every_record_and_source_is_drawn_or_reported() {
    for (file, _, bytes) in inputs() {
        let (_, a) = analysed(&file, &bytes);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let Some(Reading::Read(report)) = &a.pmi else {
            panic!("{file}: its PMI is read");
        };
        let named = |code: &str, subject: &str| {
            plan.problems
                .iter()
                .any(|p| p.code == code && p.subjects.iter().any(|s| s == subject))
        };
        for r in &report.records {
            let drawn = plan.gdt.iter().any(|g| g.source == r.source_id);
            match r.category {
                // A dimension is stated by a planned dimension or said not drawn, never both
                // (stage 2b, `tests/pmi_dimensions.rs`); never a frame or symbol.
                Category::Dimension => {
                    let feature = format!("pmi:{}", r.source_id);
                    let stated = plan
                        .dimensions
                        .iter()
                        .any(|d| d.measurements.iter().any(|m| m.feature == feature));
                    let reported = [
                        "pmi_not_lowered",
                        "pmi_dimensions_not_ported",
                        "pmi_not_drawn",
                    ]
                    .iter()
                    .any(|c| named(c, &r.source_id));
                    assert!(
                        stated != reported && !drawn,
                        "{file}: {} is stated ({stated}) or reported ({reported})",
                        r.source_id
                    );
                }
                Category::GeometricTolerance | Category::Datum => {
                    let reported = named("pmi_not_lowered", &r.source_id);
                    assert!(
                        drawn != reported,
                        "{file}: {} is drawn ({drawn}) or reported ({reported}), not both",
                        r.source_id
                    );
                    // A record whose reading left blockers is never drawn.
                    if !r.blockers.is_empty() {
                        assert!(reported, "{file}: {} has blockers", r.source_id);
                    }
                }
                _ => {}
            }
        }
        for s in report
            .sources
            .iter()
            .filter(|s| s.outcome == draftwright::pmi::Outcome::NotExtracted)
        {
            assert!(
                named("pmi_not_extracted", &s.id),
                "{file}: {} reported",
                s.id
            );
        }
        // Each item's ID is unique and placed (Policy B: kept and logged, never dropped).
        let ids: BTreeSet<&str> = plan.gdt.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids.len(), plan.gdt.len(), "{file}");
        for g in &plan.gdt {
            assert!(g.placed.is_some(), "{file}: {} placed", g.id);
            assert!(g.basis.starts_with("source-authored"), "{}", g.basis);
        }
    }
}

#[test]
fn what_lowering_cannot_prove_is_reported_not_guessed() {
    let (file, bytes) = fixture("nist_ctc_01_asme1_ap242-e1");
    let (_, a) = analysed(&file, &bytes);
    let shown = |_: &str| true;
    let (items, _) = lower(&a, &shown);
    let frame = items
        .iter()
        .find(|l| matches!(l.glyph, Glyph::ControlFrame { .. }))
        .unwrap()
        .source
        .clone();
    let datum = items
        .iter()
        .find(|l| matches!(&l.glyph, Glyph::Datum { surface, .. } if surface == "plane"))
        .unwrap()
        .clone();
    let edit = |f: &dyn Fn(&mut Record)| {
        let mut b = a.clone();
        let Some(Reading::Read(r)) = &mut b.pmi else {
            unreachable!()
        };
        r.records.iter_mut().for_each(f);
        b
    };
    let reason = |b: &Analysis, source: &str| {
        let (items, problems) = lower(b, &shown);
        assert!(
            items.iter().all(|l| l.source != source),
            "{source} not drawn"
        );
        problems
            .into_iter()
            .find(|p| p.code == "pmi_not_lowered" && p.subjects == [source.to_string()])
            .map(|p| p.message)
            .unwrap_or_else(|| panic!("{source} reported"))
    };
    // A modifier draftwright has no drawing for.
    let b = edit(&|r| {
        if r.source_id == frame {
            r.gtol_modifiers.push("free_state".into());
        }
    });
    assert!(reason(&b, &frame).contains("'free_state' is not supported"));
    // A blocker from the reading (a datum target, a composite relation, …).
    let b = edit(&|r| {
        if r.source_id == frame {
            r.blockers
                .push("the projected tolerance zone is not carried".into());
        }
    });
    assert!(reason(&b, &frame).contains("projected tolerance zone"));
    // An attachment the faces do not prove.
    let b = edit(&|r| {
        if r.source_id == datum.source
            && let Some(g) = &mut r.geometry
        {
            g.reasons
                .push("datum reference faces are not coplanar".into());
        }
    });
    assert!(reason(&b, &datum.source).contains("not coplanar"));
    // A plane's leader follows its outward normal: flipped, the symbol moves to the other side.
    let b = edit(&|r| {
        if r.source_id == datum.source
            && let Some(n) = r.geometry.as_mut().and_then(|g| g.normal.as_mut())
        {
            *n = n.map(|x| -x);
        }
    });
    let (items, _) = lower(&b, &shown);
    let flipped = items.iter().find(|l| l.source == datum.source).unwrap();
    assert_ne!(flipped.side, datum.side);
    // A normal the edge-on view cannot show along its axis.
    let b = edit(&|r| {
        if r.source_id == datum.source
            && let Some(n) = r.geometry.as_mut().and_then(|g| g.normal.as_mut())
        {
            *n = [0.6, 0.0, 0.8];
        }
    });
    assert!(reason(&b, &datum.source).contains("cannot be shown by an axial leader"));
    // A y-normal plane whose side view is not drawn goes to the plan.
    let b = edit(&|r| {
        if r.source_id == datum.source
            && let Some(g) = &mut r.geometry
        {
            g.axis = Some("Y".into());
            g.normal = Some([0.0, -1.0, 0.0]);
        }
    });
    let (items, _) = lower(&b, &|v: &str| v != "side");
    let moved = items.iter().find(|l| l.source == datum.source).unwrap();
    assert_eq!((moved.view, moved.side), ("plan", "below"));
    // A file whose PMI cannot be read is said, and nothing is drawn.
    let mut b = a.clone();
    b.pmi = Some(Reading::Refused("cannot read Part 21: test".into()));
    let (items, problems) = lower(&b, &shown);
    assert!(items.is_empty());
    assert!(
        problems
            .iter()
            .any(|p| p.code == "pmi_not_read" && p.severity == "error")
    );
}

#[test]
fn a_part_placed_twice_is_not_annotated_on_one_placement_by_guess() {
    let (_, a, plan) = planned("assembly_plate_pin");
    let twice: Vec<&Record> = records(&a)
        .iter()
        .filter(|r| r.faces.len() > r.face_ids.len())
        .collect();
    assert!(!twice.is_empty(), "the pin is placed twice");
    for r in twice {
        if matches!(r.category, Category::GeometricTolerance | Category::Datum) {
            assert!(plan.gdt.iter().all(|g| g.source != r.source_id));
            assert!(plan.problems.iter().any(|p| p.code == "pmi_not_lowered"
                && p.subjects == [r.source_id.clone()]
                && p.message.contains("placements of the part")));
        }
    }
}

#[test]
fn frames_and_symbols_are_searchable_and_drawn_from_the_font() {
    let (part, a, plan) = planned("nist_ftc_07_asme1_ap242-e2");
    assert!(plan.gdt.iter().any(|g| g.kind == "control_frame"));
    assert!(plan.gdt.iter().any(|g| g.kind == "datum_feature"));
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let mono = std::fs::read(root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let face = ttf_parser::Face::parse(&mono, 0).unwrap();
    for g in &plan.gdt {
        let p = g.placed.as_ref().unwrap();
        // Every letter and digit drawn is a glyph of the font, never its missing-glyph box.
        for l in &p.letters {
            for ch in l.text.chars() {
                assert!(
                    face.glyph_index(ch).is_some(),
                    "{ch:?} of {} in the font",
                    g.id
                );
            }
        }
        // Its text layer is its compartments, each a run of the drawing, under its name.
        let runs: Vec<String> = p.runs.iter().map(|r| r.text.clone()).collect();
        assert_eq!(runs, g.cells, "{}", g.id);
        let a = drawing.annotations.iter().find(|x| x.name == g.id).unwrap();
        let text: Vec<&str> = a
            .runs
            .iter()
            .map(|&i| drawing.text[i].text.as_str())
            .collect();
        assert_eq!(text, g.cells, "{}", g.id);
        assert!(a.claims.is_empty());
    }
    let written: Vec<String> = drawing.text.iter().map(|r| r.text.clone()).collect();
    assert_eq!(pdf::svg_text(&drawing.svg()), written, "the SVG's text");
    let pdf_bytes = drawing.pdf(&|name: &str| std::fs::read(root().join("fonts").join(name)).ok());
    assert_eq!(pdf::pdf_text(&pdf_bytes), written, "the PDF's text");
    let objects = pdf::pdf_objects(&pdf_bytes);
    let font_dict = objects
        .values()
        .map(|(d, _)| d)
        .find(|d| d.contains("/Subtype /Type0") && d.contains("IBMPlexMono-Regular"))
        .expect("the mono font");
    let unicode = pdf::to_unicode(&objects[&pdf::reference(font_dict, "/ToUnicode")].1);
    for symbol in plan
        .gdt
        .iter()
        .map(|g| g.cells[0].clone())
        .filter(|c| c.chars().count() == 1)
    {
        if symbol.chars().all(|c| face.glyph_index(c).is_none()) {
            assert!(
                unicode.values().any(|t| *t == symbol),
                "{symbol} is searchable through the ToUnicode map"
            );
        }
    }
    // The DXF and PNG draw the same ink.
    assert!(!drawing.dxf().is_empty());
    assert!(!drawing.png(100.0).unwrap().is_empty());
}

#[test]
fn lint_reads_a_frame_as_no_measurement() {
    let (part, a, plan) = planned("nist_ctc_01_asme1_ap242-e1");
    assert!(!plan.gdt.is_empty());
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let with = model::render(&part, &a, &plan, &font).unwrap();
    let mut bare = plan.clone();
    bare.gdt.clear();
    let without = model::render(&part, &a, &bare, &font).unwrap();
    // The PMI coverage lint judges the frames themselves (`tests/pmi_coverage.rs`); no other
    // rule may count them.
    let codes = |d: &draftwright::output::Drawing| {
        draftwright::lint::lint(&a, d)
            .into_iter()
            .filter(|i| !i.code.starts_with("pmi_"))
            .map(|i| (i.code, i.subjects))
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(&with), codes(&without));
}

#[test]
fn the_same_part_gives_the_same_plan_and_bytes() {
    for stem in ["nist_ctc_01_asme1_ap242-e1", "nist_ftc_07_asme1_ap242-e2"] {
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
        // The Analysis round-trips through JSON with its PMI, and plans the same.
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

#[test]
fn a_part_without_pmi_and_an_older_analysis_have_none() {
    let (_, a) = model::analyse(&root().join("tests/fixtures/parts/plate.step")).unwrap();
    assert!(a.pmi.is_none());
    assert!(!serde_json::to_string(&a).unwrap().contains("\"pmi\""));
    let (_, mut b, _) = planned("nist_ctc_01_asme1_ap242-e1");
    let mut older = serde_json::to_value(&b).unwrap();
    older["schema"] = "draftwright.analysis/5".into();
    older.as_object_mut().unwrap().remove("pmi");
    b = serde_json::from_value(older).unwrap();
    let plan = model::plan(&b, &Spec::default()).unwrap();
    assert!(plan.gdt.is_empty());
    assert!(plan.problems.iter().all(|p| !p.code.starts_with("pmi_")));
}

#[test]
fn the_corpus_nist_parts_carry_presentation_only() {
    let Some(dir) = corpus::dir() else {
        eprintln!(
            "SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS (DRAFTWRIGHT_CORPUS_REQUIRED=1 makes this a failure)"
        );
        return;
    };
    let mut nist: Vec<PathBuf> = std::fs::read_dir(dir.join("nist"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "stp"))
        .collect();
    nist.sort();
    assert_eq!(nist.len(), 10, "the corpus's NIST parts");
    for p in nist {
        let (_, a) = model::analyse(&p).unwrap();
        let Some(Reading::Read(r)) = &a.pmi else {
            panic!("{}: its presentation is counted", p.display());
        };
        assert!(r.records.is_empty(), "{}", p.display());
        assert!(r.accounting.presentation > 0, "{}", p.display());
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert!(plan.gdt.is_empty());
        assert!(
            plan.problems
                .iter()
                .any(|p| p.code == "pmi_presentation_not_read")
        );
    }
}

#[test]
fn the_same_bytes_in_every_time_zone() {
    let (file, bytes) = fixture("nist_ctc_01_asme1_ap242-e1");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pmi_timezone");
    std::fs::create_dir_all(&dir).unwrap();
    let step = dir.join(&file);
    std::fs::write(&step, &bytes).unwrap();
    let render = |tz: &str| {
        let out_dir = dir.join(tz.replace('/', "_"));
        std::fs::create_dir_all(&out_dir).unwrap();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_draftwright"))
            .env("TZ", tz)
            .arg("render")
            .arg(&step)
            .arg("--no-features")
            .arg("-o")
            .arg(out_dir.join("ctc01"))
            .args(["--formats", "svg,dxf,pdf"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{tz}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        ["svg", "dxf", "pdf"].map(|f| std::fs::read(out_dir.join(format!("ctc01.{f}"))).unwrap())
    };
    let utc = render("UTC");
    assert!(String::from_utf8_lossy(&utc[0]).contains("0.75"));
    assert_eq!(render("Asia/Kolkata"), utc);
}
