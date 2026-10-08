//! Holes and hole patterns from Rust quiddity (`bridge::native`) against the same parts' Python
//! quiddity recognition through the bridge, record by record.
//!
//! Every part with bridged Python recognition is compared: the fixtures in `tests/fixtures/parts/`
//! (set `parts`), and the sampled corpus parts with Python quiddity 0.4.0.dev0's recognition
//! (`recognition/corpus/`, set `corpus`) and 0.3.9's, the recogniser draftwright drew their sheets
//! with (`recognition/sheets/`, set `sheets`). sheets' ctc04 is not compared: the bridge refuses
//! its recognition (one face's area differs from OpenCascade's).
//!
//! Python's records are in its working frame and are mapped into the part's (the native ones
//! already are). Holes are paired by axis, location and diameter; patterns by their members.
//! For each pair the IDs, faces and parameters are compared, numbers to 1e-6 relative (absolute
//! below 1); for each pattern also the arrangement and its members. Every difference is listed
//! in `tests/fixtures/known_native.json` with the values on both sides, a verdict and the
//! evidence. The test fails on a difference not listed, a listed one that no longer occurs or
//! has changed, and a duplicate entry, and prints the entries it needs (`NATIVE_REPORT=1` prints
//! every difference).

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::bridge::{self, Feature, HolePattern, native};
use draftwright::haecceity::Part;
use draftwright::model;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    set: String,
    part: String,
    /// Python's ID of the hole or pattern, or `rust <ID>`, the native one, when Python has none.
    subject: String,
    /// `presence`, `id`, `faces`, `opening` (a through hole's location and axis), `arrangement`,
    /// `members` or `parameters.<path>`.
    field: String,
    python: String,
    rust: String,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    evidence: String,
}

type Key = (String, String, String, String);

#[derive(Debug, PartialEq)]
struct Difference {
    python: String,
    rust: String,
}

struct Case {
    set: &'static str,
    part: String,
    step: PathBuf,
    json: String,
}

fn gunzip(path: &Path) -> Option<String> {
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).ok()?)
        .read_to_string(&mut text)
        .unwrap();
    Some(text)
}

/// Every part with bridged Python recognition; the corpus sets only when the corpus is found.
fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for part in ["boss_circle", "plate", "prismatic", "turned", "two_grids"] {
        out.push(Case {
            set: "parts",
            part: part.into(),
            step: fixture(&format!("parts/{part}.step")),
            json: std::fs::read_to_string(fixture(&format!("recognition/{part}.json"))).unwrap(),
        });
    }
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED (corpus parts): the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return out;
    };
    for set in ["corpus", "sheets"] {
        let mut names: Vec<String> = std::fs::read_dir(fixture(&format!("recognition/{set}")))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter_map(|f| f.strip_suffix(".json.gz").map(String::from))
            .collect();
        names.sort();
        for part in names {
            let json = gunzip(&fixture(&format!("recognition/{set}/{part}.json.gz"))).unwrap();
            out.push(Case {
                set,
                step: dir.join(&facts::load(&part).binding.file),
                part,
                json,
            });
        }
    }
    out
}

/// A record value as the list shows it: compact JSON, numbers to 1e-6.
fn show(v: &Value) -> String {
    fn rounded(v: &Value) -> Value {
        match v {
            Value::Number(n) => {
                let x = (n.as_f64().unwrap() * 1e6).round() / 1e6;
                Value::from(if x == 0.0 { 0.0 } else { x })
            }
            Value::Array(a) => Value::Array(a.iter().map(rounded).collect()),
            Value::Object(o) => Value::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), rounded(v)))
                    .collect::<Map<_, _>>(),
            ),
            v => v.clone(),
        }
    }
    rounded(v).to_string()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}

/// Python's record mapped into the part's frame: points (`location`, `center`) and directions
/// (`axis`, `direction`), here and in nested records; empty `named_dimensions` dropped (the
/// native document never carries them, and an empty one says nothing).
fn to_part(frame: &[[f64; 3]; 4], record: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (k, v) in record {
        let mapped = match (k.as_str(), bridge::holes::point(Some(v))) {
            ("location" | "center", Some(p)) => {
                Value::from(bridge::holes::to_part(frame, p).to_vec())
            }
            ("axis" | "direction", Some(d)) => {
                Value::from(bridge::holes::dir_to_part(frame, d).to_vec())
            }
            ("named_dimensions", _) if v.as_object().is_some_and(Map::is_empty) => continue,
            (_, _) => match v {
                Value::Object(o) => Value::Object(to_part(frame, o)),
                v => v.clone(),
            },
        };
        out.insert(k.clone(), mapped);
    }
    out
}

/// The differences between two values under *path*, into *out*.
fn compare(
    path: &str,
    py: Option<&Value>,
    rs: Option<&Value>,
    out: &mut Vec<(String, Difference)>,
) {
    let differ = match (py, rs) {
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for k in keys {
                compare(&format!("{path}.{k}"), a.get(k), b.get(k), out);
            }
            false
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) if a.len() == b.len() => {
            let mut inner = Vec::new();
            for (x, y) in a.iter().zip(b) {
                compare(path, Some(x), Some(y), &mut inner);
            }
            !inner.is_empty()
        }
        (Some(Value::Number(a)), Some(Value::Number(b))) => {
            !close(a.as_f64().unwrap(), b.as_f64().unwrap())
        }
        (a, b) => a != b,
    };
    if differ {
        let s = |v: Option<&Value>| v.map_or("-".to_string(), show);
        out.push((
            path.to_string(),
            Difference {
                python: s(py),
                rust: s(rs),
            },
        ));
    }
}

fn location(p: &Map<String, Value>) -> [f64; 3] {
    bridge::holes::point(p.get("location")).unwrap()
}

/// A hole as the list names it when it has no counterpart: its ID, faces and record.
fn summary(f: &Feature, record: &Map<String, Value>) -> String {
    format!(
        "{} {:?} {}",
        f.id,
        f.faces,
        show(&Value::Object(record.clone()))
    )
}

/// The differences on one part, read by the bridge as *recognition* and *features*, by
/// (subject, field).
fn differences(
    part: &Part,
    case: &Case,
    recognition: &bridge::Recognition,
    features: &[Feature],
) -> BTreeMap<(String, String), Difference> {
    let frame = recognition.frame;
    let py: Vec<(&Feature, Map<String, Value>)> = features
        .iter()
        .filter(|f| f.kind == "hole")
        .map(|f| (f, to_part(&frame, &f.parameters)))
        .collect();
    let rs = native::recognise(part).unwrap_or_else(|e| panic!("{}: {e}", case.part));
    let mut out = BTreeMap::new();
    let mut put = |subject: &str, field: &str, d: Difference| {
        let old = out.insert((subject.to_string(), field.to_string()), d);
        assert!(old.is_none(), "{} {subject} {field} twice", case.part);
    };

    // Holes, paired by axis, location and diameter (each native hole at most once); a through
    // hole also with the one drilled from its other opening (its axis reversed, its location
    // the other end of its depth).
    let mut pair: BTreeMap<String, String> = BTreeMap::new();
    let mut taken = vec![false; rs.holes.len()];
    for (f, p) in &py {
        let found = rs.holes.iter().enumerate().position(|(i, h)| {
            let q = &h.parameters;
            !taken[i]
                && compare_none(p.get("diameter"), q.get("diameter"))
                && (same_opening(p, q) || other_opening(p, q))
        });
        let Some(i) = found else {
            put(
                &f.id,
                "presence",
                Difference {
                    python: summary(f, p),
                    rust: "-".into(),
                },
            );
            continue;
        };
        taken[i] = true;
        let h = &rs.holes[i];
        pair.insert(h.id.clone(), f.id.clone());
        if h.id != f.id {
            put(
                &f.id,
                "id",
                Difference {
                    python: f.id.clone(),
                    rust: h.id.clone(),
                },
            );
        }
        if h.faces != f.faces {
            put(
                &f.id,
                "faces",
                Difference {
                    python: format!("{:?}", f.faces),
                    rust: format!("{:?}", h.faces),
                },
            );
        }
        let (mut p, mut q) = (p.clone(), h.parameters.clone());
        if !same_opening(&p, &q) {
            let opening = |r: &mut Map<String, Value>| {
                let at = r.remove("location").unwrap();
                let axis = r.remove("axis").unwrap();
                format!("{} along {}", show(&at), show(&axis))
            };
            put(
                &f.id,
                "opening",
                Difference {
                    python: opening(&mut p),
                    rust: opening(&mut q),
                },
            );
        }
        let mut diffs = Vec::new();
        compare(
            "parameters",
            Some(&Value::Object(p)),
            Some(&Value::Object(q)),
            &mut diffs,
        );
        for (field, d) in diffs {
            put(&f.id, &field, d);
        }
    }
    for (i, h) in rs.holes.iter().enumerate() {
        if !taken[i] {
            put(
                &format!("rust {}", h.id),
                "presence",
                Difference {
                    python: "-".into(),
                    rust: summary(h, &h.parameters),
                },
            );
        }
    }

    // Patterns, paired by their members (native members named by their Python pair).
    let members = |p: &HolePattern, map: Option<&BTreeMap<String, String>>| -> BTreeSet<String> {
        p.members
            .iter()
            .map(|m| {
                map.map_or(m.clone(), |map| {
                    map.get(m).cloned().unwrap_or(format!("rust {m}"))
                })
            })
            .collect()
    };
    let mut taken = vec![false; rs.patterns.len()];
    for p in &recognition.patterns {
        let mine = members(p, None);
        let Some(i) = rs
            .patterns
            .iter()
            .enumerate()
            .position(|(i, q)| !taken[i] && members(q, Some(&pair)) == mine)
        else {
            put(
                &p.id,
                "presence",
                Difference {
                    python: format!("{} {:?}", p.arrangement, p.members),
                    rust: "-".into(),
                },
            );
            continue;
        };
        taken[i] = true;
        let q = &rs.patterns[i];
        let mut put_if = |field: &str, a: String, b: String| {
            if a != b {
                put(&p.id, field, Difference { python: a, rust: b });
            }
        };
        put_if("id", p.id.clone(), q.id.clone());
        put_if("arrangement", p.arrangement.clone(), q.arrangement.clone());
        put_if("faces", format!("{:?}", p.faces), format!("{:?}", q.faces));
        let order: Vec<String> = q.members.iter().map(|m| pair[m].clone()).collect();
        put_if("members", format!("{:?}", p.members), format!("{order:?}"));
        let mut diffs = Vec::new();
        compare(
            "parameters",
            Some(&Value::Object(to_part(&frame, &p.parameters))),
            Some(&Value::Object(q.parameters.clone())),
            &mut diffs,
        );
        for (field, d) in diffs {
            put(&p.id, &field, d);
        }
    }
    for (i, q) in rs.patterns.iter().enumerate() {
        if !taken[i] {
            put(
                &format!("rust {}", q.id),
                "presence",
                Difference {
                    python: "-".into(),
                    rust: format!("{} {:?}", q.arrangement, q.members),
                },
            );
        }
    }
    out
}

/// Whether two hole records open at the same point along the same axis.
fn same_opening(p: &Map<String, Value>, q: &Map<String, Value>) -> bool {
    (0..3).all(|k| close(location(p)[k], location(q)[k]))
        && compare_none(p.get("axis"), q.get("axis"))
}

/// Whether two through holes are one bore drilled from its two openings: axes reversed, each
/// location the other's moved by the depth (to its 0.01 rounding) along its axis.
fn other_opening(p: &Map<String, Value>, q: &Map<String, Value>) -> bool {
    let through =
        |r: &Map<String, Value>| r.get("bottom").and_then(Value::as_str) == Some("through");
    let (Some(a), Some(b)) = (
        bridge::holes::point(p.get("axis")),
        bridge::holes::point(q.get("axis")),
    ) else {
        return false;
    };
    let Some(depth) = p.get("depth").and_then(Value::as_f64) else {
        return false;
    };
    through(p)
        && through(q)
        && compare_none(p.get("depth"), q.get("depth"))
        && (0..3).all(|k| close(a[k], -b[k]))
        && {
            // The other end lies on the axis line, the depth along it; quiddity rounds the
            // depth to 0.01, so the ends are that far apart within 0.005.
            let d: [f64; 3] = std::array::from_fn(|k| location(q)[k] - location(p)[k]);
            let along: f64 = (0..3).map(|k| d[k] * a[k]).sum();
            (along - depth).abs() <= 0.005 + 1e-6 * depth
                && (0..3).all(|k| close(location(p)[k] + along * a[k], location(q)[k]))
        }
}

/// Whether two values agree as [`compare`] compares them.
fn compare_none(a: Option<&Value>, b: Option<&Value>) -> bool {
    let mut d = Vec::new();
    compare("", a, b, &mut d);
    d.is_empty()
}

#[test]
fn native_holes_and_patterns_match_the_bridge_or_are_listed() {
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known_native.json")).unwrap())
            .unwrap();
    let verdicts = [
        "rust-correct",
        "rust-wrong",
        "equivalent",
        "undetermined",
        "not-applicable",
    ];
    let mut listed: BTreeMap<Key, &Known> = BTreeMap::new();
    for k in &known {
        assert!(verdicts.contains(&k.verdict.as_str()), "{k:?}");
        assert!(!k.evidence.trim().is_empty(), "{k:?} has no evidence");
        let key = (
            k.set.clone(),
            k.part.clone(),
            k.subject.clone(),
            k.field.clone(),
        );
        assert!(listed.insert(key, k).is_none(), "listed twice: {k:?}");
    }
    let corpus_found = corpus::dir().is_some();
    let report = std::env::var_os("NATIVE_REPORT").is_some();
    let mut seen: BTreeSet<Key> = BTreeSet::new();
    let mut needed = Vec::new();
    let mut changed = Vec::new();
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let mut compared = 0;
    for case in cases() {
        let (part, a) = model::analyse(&case.step).unwrap();
        let (recognition, features) =
            match bridge::read(&part, &a.binding.source_sha256, &case.json) {
                Ok(read) => read,
                Err(e) => {
                    assert_eq!(
                        (case.set, case.part.as_str()),
                        ("sheets", "ctc04"),
                        "the bridge refuses {}/{}: {e}",
                        case.set,
                        case.part
                    );
                    continue;
                }
            };
        compared += 1;
        for ((subject, field), d) in differences(&part, &case, &recognition, &features) {
            let key = (case.set.to_string(), case.part.clone(), subject, field);
            if report {
                eprintln!("{key:?}: python {} rust {}", d.python, d.rust);
            }
            match listed.get(&key) {
                Some(k) if k.python == d.python && k.rust == d.rust => {
                    *tally.entry(k.verdict.clone()).or_default() += 1;
                }
                Some(k) => changed.push(format!(
                    "{key:?}: listed python {} rust {}, now python {} rust {}",
                    k.python, k.rust, d.python, d.rust
                )),
                None => needed.push(Known {
                    set: key.0.clone(),
                    part: key.1.clone(),
                    subject: key.2.clone(),
                    field: key.3.clone(),
                    python: d.python,
                    rust: d.rust,
                    verdict: "?".into(),
                    evidence: "?".into(),
                }),
            }
            seen.insert(key);
        }
    }
    let stale: Vec<_> = listed
        .keys()
        .filter(|k| !seen.contains(*k) && (corpus_found || k.0 == "parts"))
        .collect();
    eprintln!("compared {compared} parts; differences by verdict: {tally:?}");
    assert!(
        needed.is_empty() && changed.is_empty() && stale.is_empty(),
        "unlisted differences (add to tests/fixtures/known_native.json):\n{}\nchanged: {changed:#?}\nno longer occur: {stale:#?}",
        serde_json::to_string_pretty(&needed).unwrap()
    );
    if corpus_found {
        // 5 fixture parts, 12 corpus parts and the 12 sheets parts the bridge reads.
        assert_eq!(compared, 29);
    }
}

/// The native document of *name* (a fixture part), and the part.
fn document(name: &str) -> (Part, quiddity::recognition::RecognitionDocument) {
    let (part, _) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
    let r = quiddity::correspondence::recognise(&part);
    let doc = quiddity::recognition::document(&r, part.faces.len());
    (part, doc)
}

#[test]
fn plate_holes_and_grid_are_read_in_the_parts_frame() {
    let (part, doc) = document("plate");
    let n = native::read(&part, &doc).unwrap();
    let ids: Vec<_> = n.holes.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["f:hole:1", "f:hole:2", "f:hole:3", "f:hole:4"]);
    for (f, face) in n.holes.iter().zip(6..) {
        assert_eq!((f.kind.as_str(), &f.faces), ("hole", &vec![face]));
        assert_eq!(f.fingerprint.version, bridge::FINGERPRINT_VERSION);
        assert_eq!(f.fingerprint.axis, Some([0.0, 0.0, 1.0]));
        let p = f.fingerprint.pattern.as_ref().unwrap();
        assert_eq!((p.count, p.pitch), (4, 20.0));
    }
    assert_eq!(n.patterns.len(), 1);
    assert_eq!(n.patterns[0].id, "f:hole_pattern:1");
    assert_eq!(n.patterns[0].arrangement, "grid");
    assert_eq!(n.patterns[0].faces, [6, 7, 8, 9]);
    assert!(
        n.recogniser.starts_with("quiddity-rust "),
        "{}",
        n.recogniser
    );
    assert_eq!(native::recognise(&part).unwrap(), n);
}

#[test]
fn the_reader_refuses_what_it_does_not_know() {
    let (part, doc) = document("plate");
    let refused = |edit: &dyn Fn(&mut quiddity::recognition::RecognitionDocument)| {
        let mut d = doc.clone();
        edit(&mut d);
        native::read(&part, &d).unwrap_err()
    };
    let e = refused(&|d| d.schema_version = "quiddity-rust/recognition/2".into());
    assert!(e.contains("\"quiddity-rust/recognition/2\", not"), "{e}");
    let e = refused(&|d| d.coordinate_space = "working".into());
    assert!(e.contains("\"working\" coordinates"), "{e}");
    let e = refused(&|d| d.face_count += 1);
    assert!(e.contains("11 faces and the part 10"), "{e}");
    let e = refused(&|d| d.records[0].family = "warp_drives".into());
    assert!(
        e.contains("family \"warp_drives\" this reader does not know"),
        "{e}"
    );
    let e = refused(&|d| d.records[0].record_type = "WarpDrive".into());
    assert!(e.contains("WarpDrive (holes), a record type"), "{e}");
    // A record type known to another family is not known to this one.
    let e = refused(&|d| d.records[0].record_type = "BoltCircle".into());
    assert!(e.contains("BoltCircle (holes)"), "{e}");
    let e = refused(&|d| d.records[0].faces = vec![10]);
    assert!(e.contains("names face 10"), "{e}");
    let e = refused(&|d| {
        let faces = d.records[1].faces.clone();
        d.records[0].faces = faces;
    });
    assert!(e.contains("cannot be told apart"), "{e}");
    let e = refused(&|d| {
        let last = d.records.len() - 1;
        d.records[last].parameters.remove("holes");
    });
    assert!(e.contains("has no member holes"), "{e}");
}

/// The reader's schema is the provider's: its version and every family and record type.
#[test]
fn the_reader_knows_the_providers_schema() {
    assert_eq!(
        native::SCHEMA_VERSION,
        quiddity::recognition::SCHEMA_VERSION
    );
    assert_eq!(native::FAMILIES, quiddity::recognition::RECORD_TYPES);
}

/// quiddity and haecceity come from one commit of quiddity-rust: their `source` lines in
/// Cargo.lock agree.
#[test]
fn quiddity_and_haecceity_are_pinned_together() {
    let lock =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).unwrap();
    let source = |name: &str| {
        let block = lock
            .split("[[package]]")
            .find(|b| b.contains(&format!("name = \"{name}\"\n")))
            .unwrap_or_else(|| panic!("{name} is not in Cargo.lock"));
        block
            .lines()
            .find_map(|l| l.strip_prefix("source = "))
            .unwrap()
            .to_string()
    };
    assert!(source("haecceity").contains('#'), "{}", source("haecceity"));
    assert_eq!(source("quiddity"), source("haecceity"));
}
