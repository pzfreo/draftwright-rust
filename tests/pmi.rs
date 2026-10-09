//! The port's PMI reading ([`draftwright::pmi`]) against draftwright's `extract_pmi_report`,
//! file by file and record by record, and its fail-closed paths.
//!
//! draftwright's reports are in `tests/fixtures/pmi/<stem>.json.gz` (`tools/capture_pmi.py`):
//! the AP242 files in `tests/fixtures/pmi/step/` (`tests/fixtures/pmi/README.md`) and the
//! corpus's NIST parts (`nist/*.stp`, geometry only, read from `DRAFTWRIGHT_CORPUS`).
//!
//! A draftwright record is paired with the port's by the Part 21 instance draftwright names for
//! it (`key`, the port's `part21_id`), else by the same category and the same `ADVANCED_FACE`
//! instances (a datum's: its XCAF faces and its Part 21 `reference_item_ids`), else, for a datum,
//! by its letter. A paired record's `kind`, `type_code`, `value` (within 1e-9, relative),
//! deviations, limits, `label`, `datum_refs`, `gtol_modifiers`, `basic`, structured fields and
//! faces are compared; the port's lower deviation is compared negated, draftwright's being an
//! unsigned magnitude below nominal. A manufacturing requirement's, surface label's and common
//! label's faces are its Part 21 `reference_item_ids` (XCAF has no shapes for them), compared
//! with the port's faces and edges; their `value` is draftwright's placeholder 0, not compared.
//! A record either side has no pair for is a difference of field `record`.
//!
//! Every difference must be listed, with a verdict and evidence, in
//! `tests/fixtures/known_pmi.json`; a listed difference that has gone away must be taken off.

#[path = "common/corpus.rs"]
mod corpus;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::{self, p21, pmi as hpmi, step};
use draftwright::pmi::{self, Category, Outcome, Record, Report};
use serde::Deserialize;
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pmi")
}

fn gunzip(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(raw)
        .read_to_end(&mut out)
        .unwrap();
    out
}

/// The STEP files compared: the committed AP242 fixtures, and the corpus's NIST parts when the
/// corpus is found. Each is `(name as captured, its bytes)`.
fn inputs() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut fixtures: Vec<PathBuf> = std::fs::read_dir(fixtures().join("step"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "gz"))
        .collect();
    fixtures.sort();
    for p in fixtures {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        out.push((name, gunzip(&std::fs::read(&p).unwrap())));
    }
    match corpus::dir() {
        Some(dir) => {
            let mut nist: Vec<PathBuf> = std::fs::read_dir(dir.join("nist"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|x| x == "stp"))
                .collect();
            nist.sort();
            assert_eq!(nist.len(), 10, "the corpus's NIST parts");
            for p in nist {
                let name = format!("nist/{}", p.file_name().unwrap().to_string_lossy());
                out.push((name, std::fs::read(&p).unwrap()));
            }
        }
        None => eprintln!(
            "SKIPPED the corpus's NIST parts: the corpus is not found; set DRAFTWRIGHT_CORPUS \
             (DRAFTWRIGHT_CORPUS_REQUIRED=1 makes this a failure)"
        ),
    }
    out
}

fn stem(name: &str) -> String {
    let base = name.rsplit('/').next().unwrap();
    let base = base.strip_suffix(".gz").unwrap_or(base);
    base.rsplit_once('.').unwrap().0.to_string()
}

fn python(name: &str) -> Value {
    let raw = std::fs::read(fixtures().join(format!("{}.json.gz", stem(name)))).unwrap();
    serde_json::from_slice(&gunzip(&raw)).unwrap()
}

fn read(bytes: &[u8]) -> (haecceity::Part, Report) {
    let part = haecceity::read_step(bytes).unwrap();
    let report = pmi::read(bytes, &part).unwrap();
    (part, report)
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    file: String,
    /// draftwright's `source_id`, or the port's when draftwright has no such record.
    record: String,
    /// The port's record paired with draftwright's (`source_id`), or null.
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

fn category(c: &str) -> Option<Category> {
    Some(match c {
        "dimension" => Category::Dimension,
        "geometric_tolerance" => Category::GeometricTolerance,
        "datum" => Category::Datum,
        "manufacturing_requirement" => Category::ManufacturingRequirement,
        "surface_label" => Category::SurfaceLabel,
        _ => return None,
    })
}

/// The instance number of `#N`.
fn number(s: &str) -> Option<u64> {
    s.strip_prefix('#')?.parse().ok()
}

/// Whether *rec* is read from Part 21 alone, with no XCAF shapes: a manufacturing requirement,
/// a surface label or a common label. Its value is a placeholder.
fn part21_only(rec: &Value) -> bool {
    rec["kind"] == "common_label"
        || ["manufacturing_requirement", "surface_label"]
            .contains(&rec["source_category"].as_str().unwrap())
}

/// draftwright's faces for *rec*: XCAF's, and for a datum or a Part 21 record its Part 21
/// support too.
fn python_faces(rec: &Value) -> BTreeSet<u64> {
    let mut out: BTreeSet<u64> = rec["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    if rec["source_category"] == "datum" || part21_only(rec) {
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

fn summary_python(rec: &Value) -> Value {
    json!({"category": rec["source_category"], "kind": rec["kind"], "label": rec["label"]})
}

fn summary_rust(rec: &Record) -> Value {
    json!({"category": rec.category, "kind": rec.kind, "label": rec.label})
}

fn close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
        _ => false,
    }
}

/// The port's faces for *r*, as `python_faces` takes draftwright's: its `ADVANCED_FACE`
/// instances, and for a Part 21 record (*part21*) its edges' instances too.
fn rust_faces(part: &haecceity::Part, r: &Record, part21: bool) -> BTreeSet<u64> {
    let mut out: BTreeSet<u64> = r.face_ids.iter().copied().collect();
    if part21 {
        out.extend(
            r.edges
                .iter()
                .filter_map(|&e| part.edge_source(e).map(|s| s.entity)),
        );
    }
    out
}

/// The differences between draftwright's report *py* and the port's *rust* of file *file*,
/// whose drawing's part is *part*.
fn compare(file: &str, py: &Value, part: &haecceity::Part, rust: &Report) -> (Vec<Diff>, usize) {
    let mut diffs = Vec::new();
    let mut paired: Vec<Option<usize>> = Vec::new();
    let mut taken = vec![false; rust.records.len()];
    let records = py["records"].as_array().unwrap();
    let projected = |rec: &Value| category(rec["source_category"].as_str().unwrap()).is_some();
    paired.resize(records.len(), None);
    // By the instance draftwright names.
    for (i, rec) in records.iter().enumerate() {
        let (Some(c), Some(key)) = (
            category(rec["source_category"].as_str().unwrap()),
            number(rec["key"].as_str().unwrap()),
        ) else {
            continue;
        };
        if !projected(rec) {
            continue;
        }
        if let Some(j) = (0..rust.records.len()).find(|&j| {
            !taken[j] && rust.records[j].category == c && rust.records[j].part21_id == key
        }) {
            taken[j] = true;
            paired[i] = Some(j);
        }
    }
    // By the faces, preferring the same kind and value.
    for (i, rec) in records.iter().enumerate() {
        if paired[i].is_some() || !projected(rec) {
            continue;
        }
        let c = category(rec["source_category"].as_str().unwrap()).unwrap();
        let faces = python_faces(rec);
        if faces.is_empty() {
            continue;
        }
        let candidates: Vec<usize> = (0..rust.records.len())
            .filter(|&j| {
                let r = &rust.records[j];
                !taken[j] && r.category == c && rust_faces(part, r, part21_only(rec)) == faces
            })
            .collect();
        let best = candidates
            .iter()
            .copied()
            .find(|&j| {
                let r = &rust.records[j];
                r.kind == rec["kind"].as_str().unwrap() && close(r.value, rec["value"].as_f64())
            })
            .or_else(|| candidates.first().copied());
        if let Some(j) = best {
            taken[j] = true;
            paired[i] = Some(j);
        }
    }
    // By the kind and value, draftwright's faces among the port's (XCAF names one member of a
    // pattern, for instance).
    for (i, rec) in records.iter().enumerate() {
        if paired[i].is_some() || !projected(rec) {
            continue;
        }
        let c = category(rec["source_category"].as_str().unwrap()).unwrap();
        let faces = python_faces(rec);
        if faces.is_empty() {
            continue;
        }
        if let Some(j) = (0..rust.records.len()).find(|&j| {
            let r = &rust.records[j];
            !taken[j]
                && r.category == c
                && r.kind == rec["kind"].as_str().unwrap()
                && close(r.value, rec["value"].as_f64())
                && faces.is_subset(&rust_faces(part, r, part21_only(rec)))
        }) {
            taken[j] = true;
            paired[i] = Some(j);
        }
    }
    // A datum by its letter.
    for (i, rec) in records.iter().enumerate() {
        if paired[i].is_some() || rec["source_category"] != "datum" {
            continue;
        }
        if let Some(j) = (0..rust.records.len()).find(|&j| {
            !taken[j]
                && rust.records[j].category == Category::Datum
                && rust.records[j].label == rec["label"].as_str().unwrap()
        }) {
            taken[j] = true;
            paired[i] = Some(j);
        }
    }
    let mut agree = 0;
    for (i, rec) in records.iter().enumerate() {
        let id = rec["source_id"].as_str().unwrap().to_string();
        let port = paired[i].map_or(Value::Null, |j| json!(rust.records[j].source_id));
        let mut diff = |field: &str, python: Value, rust: Value| {
            diffs.push(Diff {
                file: file.into(),
                record: id.clone(),
                port: port.clone(),
                field: field.into(),
                python,
                rust,
            });
        };
        let Some(j) = paired[i] else {
            diff("record", summary_python(rec), Value::Null);
            continue;
        };
        let r = &rust.records[j];
        let mut same = true;
        let mut check = |field: &str, python: Value, rust: Value, equal: bool| {
            if !equal {
                same = false;
                diff(field, python, rust);
            }
        };
        let f = |v: &Value| v.as_f64();
        check(
            "kind",
            rec["kind"].clone(),
            json!(r.kind),
            rec["kind"] == r.kind.as_str(),
        );
        check(
            "type_code",
            rec["type_code"].clone(),
            json!(r.type_code),
            rec["type_code"] == json!(r.type_code),
        );
        let fields = [
            ("value", f(&rec["value"]), r.value),
            ("upper_tol", f(&rec["upper_tol"]), r.upper_tol),
            ("lower_tol", f(&rec["lower_tol"]), r.lower_tol.map(|v| -v)),
            ("lower_bound", f(&rec["lower_bound"]), r.lower_bound),
            ("upper_bound", f(&rec["upper_bound"]), r.upper_bound),
        ];
        for (field, p, q) in fields {
            // A datum, a requirement and a label have no value: draftwright's is a placeholder 0.
            if field == "value" && (r.category == Category::Datum || part21_only(rec)) {
                continue;
            }
            check(field, json!(p), json!(q), close(p, q));
        }
        check(
            "label",
            rec["label"].clone(),
            json!(r.label),
            rec["label"] == r.label.as_str(),
        );
        check(
            "datum_refs",
            rec["datum_refs"].clone(),
            json!(r.datum_refs),
            rec["datum_refs"] == json!(r.datum_refs),
        );
        check(
            "gtol_modifiers",
            rec["gtol_modifiers"].clone(),
            json!(r.gtol_modifiers),
            rec["gtol_modifiers"] == json!(r.gtol_modifiers),
        );
        check(
            "basic",
            rec["basic"].clone(),
            json!(r.basic),
            rec["basic"] == r.basic,
        );
        let fields = rec.get("structured_fields").cloned().unwrap_or(json!([]));
        check(
            "fields",
            fields.clone(),
            json!(r.fields),
            fields == json!(r.fields),
        );
        let faces = python_faces(rec);
        let ours = rust_faces(part, r, part21_only(rec));
        check("faces", json!(faces), json!(ours), faces == ours);
        if same {
            agree += 1;
        }
    }
    for (j, r) in rust.records.iter().enumerate() {
        if !taken[j] {
            diffs.push(Diff {
                file: file.into(),
                record: r.source_id.clone(),
                port: Value::Null,
                field: "record".into(),
                python: Value::Null,
                rust: summary_rust(r),
            });
        }
    }
    (diffs, agree)
}

#[test]
fn pmi_records_match_draftwright_or_are_known() {
    let known: Vec<Known> = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/known_pmi.json"),
        )
        .unwrap(),
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
    let mut files = BTreeSet::new();
    let mut agree = 0;
    let mut paired_total = 0;
    for (name, bytes) in inputs() {
        let py = python(&name);
        assert_eq!(py["file"], name.as_str());
        assert_eq!(
            py["sha256"],
            sha256(&bytes).as_str(),
            "{name}: not the captured file"
        );
        assert!(
            py["error"].is_null(),
            "{name}: draftwright failed: {}",
            py["error"]
        );
        let (part, rust) = read(&bytes);
        let (d, a) = compare(&name, &py, &part, &rust);
        agree += a;
        paired_total += py["records"].as_array().unwrap().len();
        eprintln!(
            "{name}: {} draftwright records, {} port records, {a} agree, {} differences",
            py["records"].as_array().unwrap().len(),
            rust.records.len(),
            d.len()
        );
        diffs.extend(d);
        files.insert(name);
    }
    let listed: Vec<Diff> = known
        .iter()
        .filter(|k| files.contains(&k.file))
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
        "{agree} of {paired_total} draftwright records agree in every field; {} differences ({} known)",
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
        "differences not in known_pmi.json:\n{}\nlisted but no longer differing:\n{}",
        as_json(&unlisted),
        as_json(&stale)
    );
}

/// haecceity's reading of a fixture, and the drawing's part, for the planted tests.
fn reading(
    name: &str,
) -> (
    p21::Document,
    Vec<step::PartDefinition>,
    haecceity::Part,
    hpmi::PmiRead,
) {
    let bytes = gunzip(&std::fs::read(fixtures().join("step").join(name)).unwrap());
    let part = haecceity::read_step(&bytes).unwrap();
    let defs = step::read_part_definitions(&bytes).unwrap();
    let doc = p21::Document::parse(bytes).unwrap();
    let read = hpmi::read(&doc, &defs).unwrap();
    (doc, defs, part, read)
}

const CTC01: &str = "nist_ctc_01_asme1_ap242-e1.stp.gz";

#[test]
fn a_face_the_drawing_does_not_have_is_not_read() {
    let (doc, defs, part, mut read) = reading(CTC01);
    let before = pmi::report(&doc, &defs, &part, &read).unwrap();
    let pmi0 = &mut read.parts[0];
    let d = pmi0
        .dimensions
        .iter()
        .position(|d| matches!(d.kind, hpmi::DimensionKind::Size { .. }))
        .unwrap();
    pmi0.features
        .push(hpmi::Feature::Items(vec![hpmi::Anchor::Face(
            hpmi::FaceIndex(99_999),
        )]));
    let planted = hpmi::FeatureId(pmi0.features.len() - 1);
    if let hpmi::DimensionKind::Size { feature, .. } = &mut pmi0.dimensions[d].kind {
        *feature = planted;
    }
    let after = pmi::report(&doc, &defs, &part, &read).unwrap();
    assert_eq!(after.records.len() + 1, before.records.len());
    let gone: Vec<&Record> = before
        .records
        .iter()
        .filter(|r| !after.records.iter().any(|a| a.source_id == r.source_id))
        .collect();
    assert_eq!(gone.len(), 1);
    let source = after
        .sources
        .iter()
        .find(|s| s.id == gone[0].source_id)
        .unwrap();
    assert_eq!(source.outcome, Outcome::NotExtracted);
    assert!(
        source
            .reason
            .starts_with("unresolved: part 0 has no face 99999"),
        "{}",
        source.reason
    );
}

#[test]
fn a_face_of_another_file_is_not_read_and_a_mismatched_part_is_refused() {
    let (doc, defs, part, read) = reading(CTC01);
    // The same faces, but the drawing's part read from another file: refused outright.
    let other = haecceity::read_step(&gunzip(
        &std::fs::read(fixtures().join("step/spool_fits.step.gz")).unwrap(),
    ))
    .unwrap();
    let err = pmi::report(&doc, &defs, &other, &read).unwrap_err();
    assert!(err.contains("not read from the same file"), "{err}");
    // A part definition whose face is an instance the drawing's part does not hold.
    let mut defs2 = defs.clone();
    let face = read.parts[0]
        .datums
        .iter()
        .find_map(|d| match &read.parts[0].features[d.feature().unwrap().0] {
            hpmi::Feature::Items(a) => a.iter().find_map(|x| match x {
                hpmi::Anchor::Face(f) => Some(f.0),
                _ => None,
            }),
            _ => None,
        })
        .unwrap();
    defs2[0].faces[face] = u64::MAX;
    let r = pmi::report(&doc, &defs2, &part, &read).unwrap();
    let source = r
        .sources
        .iter()
        .find(|s| {
            s.reason
                .contains(&format!("(#{}) is none of the drawing's faces", u64::MAX))
        })
        .expect("the datum on that face is not read");
    assert_eq!(source.outcome, Outcome::NotExtracted);
    assert!(r.records.iter().all(|x| !x.face_ids.contains(&u64::MAX)));
}

#[test]
fn haecceity_findings_are_sources_or_record_blockers() {
    let (doc, defs, part, mut read) = reading(CTC01);
    let base = pmi::report(&doc, &defs, &part, &read).unwrap();
    let on = base
        .records
        .iter()
        .find(|r| r.category == Category::GeometricTolerance && r.blockers.is_empty())
        .unwrap()
        .clone();
    read.findings.push(hpmi::Finding {
        kind: hpmi::FindingKind::Unsupported,
        part: Some(hpmi::PartId(0)),
        ids: vec![99_999_999],
        entity: "planted_entity".into(),
        detail: "a construct the reader does not read".into(),
    });
    read.findings.push(hpmi::Finding {
        kind: hpmi::FindingKind::NotModelled,
        part: Some(hpmi::PartId(0)),
        ids: vec![on.part21_id],
        entity: "position_tolerance".into(),
        detail: "planted on a read tolerance".into(),
    });
    let r = pmi::report(&doc, &defs, &part, &read).unwrap();
    let s = r
        .sources
        .iter()
        .find(|s| s.id == "finding:#99999999")
        .unwrap();
    assert_eq!(s.outcome, Outcome::NotExtracted);
    assert_eq!(s.category, Category::Other);
    assert_eq!(
        s.reason,
        "haecceity unsupported: a construct the reader does not read"
    );
    let rec = r
        .records
        .iter()
        .find(|x| x.source_id == on.source_id)
        .unwrap();
    assert!(
        rec.blockers
            .iter()
            .any(|b| b.contains("planted on a read tolerance"))
    );
    let source = r.sources.iter().find(|x| x.id == on.source_id).unwrap();
    assert_eq!(source.outcome, Outcome::PartiallyExtracted);
}

#[test]
fn requirements_are_records_and_what_is_not_projected_is_a_source() {
    let (doc, defs, part, read) = reading("spool_fits.step.gz");
    let r = pmi::report(&doc, &defs, &part, &read).unwrap();
    let requirements: Vec<(&str, &str)> = r
        .records
        .iter()
        .filter(|x| x.category == Category::ManufacturingRequirement)
        .map(|x| (x.kind.as_str(), x.label.as_str()))
        .collect();
    assert_eq!(
        requirements,
        [
            ("surface_texture", "Ra 3.2 µm unless otherwise specified"),
            ("edge_condition", "Break sharp edges 0.2 max"),
            ("general_tolerances", "ISO 2768-m"),
        ]
    );
    // The general tolerance class is the requirement's structured statement, read with it.
    let general = &r.records[r.records.len() - 1];
    assert_eq!(
        general.fields,
        [(
            "tolerance class".to_string(),
            pmi::Field::Text("ISO 2768-m".into())
        )]
    );
    assert!(
        read.provenance.parts[0].general[0]
            .iter()
            .all(|id| general.instances.contains(id))
    );
    let not_read: BTreeMap<&str, usize> = r
        .sources
        .iter()
        .filter(|s| s.outcome == Outcome::NotExtracted)
        .fold(BTreeMap::new(), |mut m, s| {
            *m.entry(s.id.split(':').next().unwrap()).or_default() += 1;
            m
        });
    assert_eq!(not_read.get("note"), None);
    assert_eq!(not_read.get("general_tolerance"), None);
    assert_eq!(
        not_read.get("material").copied().unwrap_or(0),
        usize::from(read.parts[0].material.is_some())
    );
}

const BOLT: &str = "bolt_thread_knurl.step.gz";

#[test]
fn a_thread_takes_its_structured_statement_and_without_one_has_no_faces() {
    let (doc, defs, part, mut read) = reading(BOLT);
    let r = pmi::report(&doc, &defs, &part, &read).unwrap();
    let thread = r
        .records
        .iter()
        .find(|x| x.kind == "external_thread")
        .unwrap();
    assert_eq!(thread.part21_id, 444);
    assert_eq!(
        thread.label,
        "M10 x 1.5-6g RH, full available length on nominal DIA 10 region"
    );
    assert_eq!(thread.face_ids, [17]);
    assert!(thread.blockers.is_empty(), "{:?}", thread.blockers);
    assert!(thread.instances.contains(&468), "the structured property");
    assert_eq!(
        thread.fields[..2],
        [
            (
                "thread side".to_string(),
                pmi::Field::Text("external".into())
            ),
            (
                "designation".to_string(),
                pmi::Field::Text("M10x1.5".into())
            )
        ]
    );
    assert!(
        thread
            .fields
            .contains(&("pitch".to_string(), pmi::Field::Length(1.5)))
    );
    // Its structured statement gone, the requirement in words has no faces, and says so.
    let a = read.parts[0]
        .attributes
        .iter()
        .position(|a| a.name == "external thread")
        .unwrap();
    read.parts[0].attributes.remove(a);
    read.provenance.parts[0].attributes.remove(a);
    let r = pmi::report(&doc, &defs, &part, &read).unwrap();
    let thread = r
        .records
        .iter()
        .find(|x| x.kind == "external_thread")
        .unwrap();
    assert!(thread.faces.is_empty() && thread.fields.is_empty());
    assert!(
        thread
            .blockers
            .iter()
            .any(|b| b.starts_with("the requirement's faces are not read")),
        "{:?}",
        thread.blockers
    );
    let source = r.sources.iter().find(|s| s.id == thread.source_id).unwrap();
    assert_eq!(source.outcome, Outcome::PartiallyExtracted);
}

#[test]
fn a_requirement_or_label_record_is_reported_not_drawn() {
    for (name, severities) in [
        (
            BOLT,
            [
                ("external_thread", "warning"),
                ("general_tolerances", "info"),
            ],
        ),
        (
            CTC01,
            [("surface_label", "warning"), ("surface_label", "warning")],
        ),
    ] {
        let bytes = gunzip(&std::fs::read(fixtures().join("step").join(name)).unwrap());
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pmi_not_drawn");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name.strip_suffix(".gz").unwrap());
        std::fs::write(&path, &bytes).unwrap();
        let (_, a) = draftwright::model::analyse(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let Some(pmi::Reading::Read(report)) = &a.pmi else {
            panic!("{name}: its PMI is read");
        };
        let (_, problems) = draftwright::pmi_lowering::lower(&a, &|_| true);
        for r in report.records.iter().filter(|r| {
            matches!(
                r.category,
                Category::ManufacturingRequirement | Category::SurfaceLabel
            )
        }) {
            let said: Vec<_> = problems
                .iter()
                .filter(|p| p.subjects == [r.source_id.clone()])
                .collect();
            assert_eq!(said.len(), 1, "{name}: {} is said once", r.source_id);
            assert_eq!(said[0].code, "pmi_not_drawn");
            if let Some((_, s)) = severities.iter().find(|(k, _)| *k == r.kind) {
                assert_eq!(said[0].severity, *s, "{name}: {}", r.source_id);
            }
        }
    }
}

#[test]
fn every_item_and_finding_is_accounted_for_and_the_output_is_deterministic() {
    for (name, bytes) in inputs() {
        let part = haecceity::read_step(&bytes).unwrap();
        let defs = step::read_part_definitions(&bytes).unwrap();
        let doc = p21::Document::parse(bytes.clone()).unwrap();
        let read = hpmi::read(&doc, &defs).unwrap();
        let r = pmi::report(&doc, &defs, &part, &read).unwrap();
        assert_eq!(
            r.accounting.consumed + r.accounting.reported,
            r.accounting.semantic,
            "{name}"
        );
        // Every dimension, tolerance and datum is a record or a source not read.
        let items: usize = read
            .parts
            .iter()
            .map(|p| p.dimensions.len() + p.tolerances.len() + p.datums.len())
            .sum();
        let unread_records = r
            .sources
            .iter()
            .filter(|s| {
                s.outcome == Outcome::NotExtracted
                    && ["dimension:#", "geometric_tolerance:#", "datum:#"]
                        .iter()
                        .any(|p| s.id.starts_with(p))
            })
            .count();
        let item_records = r
            .records
            .iter()
            .filter(|x| {
                matches!(
                    x.category,
                    Category::Dimension | Category::GeometricTolerance | Category::Datum
                ) && x.kind != "common_label"
            })
            .count();
        assert_eq!(item_records + unread_records, items, "{name}");
        // Every requirement in words, general tolerance and attribute set is read into a record
        // or is a source not read.
        for (p, prov) in read.provenance.parts.iter().enumerate() {
            for (item, all) in [
                ("note", &prov.notes),
                ("general_tolerance", &prov.general),
                ("attribute_set", &prov.attributes),
            ] {
                for (i, ids) in all.iter().enumerate() {
                    let in_record = r.records.iter().any(|x| {
                        x.part == p && ids.iter().all(|id| x.instances.binary_search(id).is_ok())
                    });
                    let source = r.sources.iter().any(|s| {
                        s.outcome == Outcome::NotExtracted
                            && (s.id == format!("{item}:{p}:{i}") || s.instances == *ids)
                    });
                    assert!(in_record || source, "{name}: {item} {p}:{i} lost");
                }
            }
        }
        // Every finding is a blocker on a record or a source of its own.
        for f in &read.findings {
            let blocker = format!("haecceity {} on #{}", f.kind.as_str(), f.ids[0]);
            let attached = r
                .records
                .iter()
                .any(|x| x.blockers.iter().any(|b| b.starts_with(&blocker)));
            let own = r
                .sources
                .iter()
                .any(|s| s.id == format!("finding:#{}", f.ids[0]) && s.reason.ends_with(&f.detail));
            assert!(attached || own, "{name}: finding {f:?} lost");
        }
        // Every record is a source with its outcome.
        for x in &r.records {
            let s = r.sources.iter().find(|s| s.id == x.source_id).unwrap();
            assert_eq!(
                s.outcome == Outcome::Extracted,
                x.blockers.is_empty(),
                "{name}"
            );
        }
        let distinct_records: BTreeSet<&str> =
            r.records.iter().map(|x| x.source_id.as_str()).collect();
        assert_eq!(
            distinct_records.len(),
            r.records.len(),
            "{name}: a record id twice"
        );
        let (_, again) = self::read(&bytes);
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            serde_json::to_string(&again).unwrap(),
            "{name}: not deterministic"
        );
    }
}
