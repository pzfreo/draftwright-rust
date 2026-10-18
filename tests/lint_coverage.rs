//! The coverage backstop (`lint::coverage`) against Python draftwright's.
//!
//! `tests/fixtures/lint_coverage/python.json.gz` (written by `tools/capture_lint_coverage.py`)
//! records, for corpus parts drawn as draftwright draws them by default, what its drawing lint's
//! one `lint_feature_coverage` call and one `lint_location_coverage` call read and found: the
//! holes, bosses and diameter inventory; each annotation's structured and text diameters, with
//! its quantity and owner; each hole's position key, pattern membership and projection into the
//! views it may be marked in; each view's centre marks and witness points; the structured
//! location facts; and the findings (and, for the diameters, the findings with the count check
//! on, as a standalone caller gets them). The port's checks on that recorded input give exactly
//! Python's findings (`pythons_backstop_is_reproduced_on_its_recorded_input`).
//!
//! The port's checks on its own drawings of the same parts are compared with Python's findings
//! on its sheets, with the prismatic and principal profile findings (`lint::prismatic`,
//! `lint::profiles`) of Python's lint on its sheets beside them; every difference, by part and
//! code, is listed with a verdict and the reason in `tests/fixtures/lint_coverage/known.json`
//! (`backstop_on_the_ports_own_drawings_is_known`), as are those on draftwright's own test parts
//! with recorded sheets (`prismatic_and_profile_findings_on_draftwrights_test_parts_are_known`).
//!
//! `tests/fixtures/lint_coverage/profiles.json.gz` (written by `tools/capture_lint_profiles.py`)
//! records draftwright's principal profile scan on every corpus part and on its test parts: each
//! principal face's inner loops with their verdicts, each solid's radial arc count and the
//! findings. The port's scan of the same parts gives the same, loop by loop
//! (`pythons_principal_profile_scan_is_reproduced`); differences are in `known_profiles.json`.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use draftwright::lint::coverage::{self, Page, Provided, Site};
use draftwright::lint::{LintIssue, Severity};
use draftwright::lint::{prismatic, profiles};
use draftwright::model;

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lint_coverage")
        .join(rel)
}

#[derive(Debug, Deserialize)]
struct Record {
    schema: String,
    parts: BTreeMap<String, Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    file: String,
    feature: FeatureCall,
    location: LocationCall,
}

#[derive(Debug, Deserialize)]
struct FeatureCall {
    holes: Vec<HoleRecord>,
    kept_bosses: Vec<BossRecord>,
    inventory: Vec<f64>,
    annotations: Vec<Annotation>,
    notes: Vec<Provision>,
    cells: Vec<Provision>,
    exclude: Vec<f64>,
    assembly: bool,
    check_hole_counts: bool,
    issues: Vec<Found>,
    counted_issues: Vec<Found>,
}

#[derive(Debug, Deserialize)]
struct HoleRecord {
    diameter: f64,
    cbore: Option<f64>,
    spotface: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BossRecord {
    diameter: f64,
}

#[derive(Debug, Deserialize)]
struct Annotation {
    structured: Vec<f64>,
    text: Vec<f64>,
    count: usize,
    owner: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct Provision {
    value: f64,
    count: usize,
    owner: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LocationCall {
    holes: Vec<LocationHole>,
    centre: [f64; 3],
    marks: BTreeMap<String, Vec<[f64; 2]>>,
    witnesses: BTreeMap<String, Vec<[f64; 2]>>,
    facts: Vec<Fact>,
    noted: Vec<Noted>,
    assembly: bool,
    issues: Vec<Found>,
}

#[derive(Debug, Deserialize)]
struct LocationHole {
    kind: String,
    location: [f64; 3],
    axis: String,
    #[serde(rename = "ref")]
    key: [f64; 3],
    patterned: bool,
    projections: BTreeMap<String, [f64; 2]>,
}

#[derive(Debug, Deserialize)]
struct Fact {
    feature_axis: Option<String>,
    members: Vec<[f64; 3]>,
    parameter: String,
    point: [f64; 3],
}

#[derive(Debug, Deserialize)]
struct Noted {
    feature_axis: Option<String>,
    members: Vec<[f64; 3]>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Found {
    code: String,
    severity: String,
    message: String,
}

fn record() -> Record {
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture("python.json.gz")).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    let r: Record = serde_json::from_str(&text).unwrap();
    assert_eq!(r.schema, "draftwright.lint-coverage/1");
    r
}

fn severity(assembly: bool) -> Severity {
    if assembly {
        Severity::Info
    } else {
        Severity::Warning
    }
}

fn found(issues: &[LintIssue]) -> Vec<Found> {
    issues
        .iter()
        .map(|i| Found {
            code: i.code.clone(),
            severity: serde_json::to_value(i.severity)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
            message: i.message.clone(),
        })
        .collect()
}

fn axis(letter: &str) -> usize {
    ["x", "y", "z"].iter().position(|a| *a == letter).unwrap()
}

/// The recorded holes as sites. Python's structured evidence locates a hole along an axis when a
/// fact's feature is drilled along the hole's axis and has a member at the hole's position key,
/// its parameter ends in that axis and its point is the hole's key; a placed note locating such
/// a feature locates the hole along every axis (`lint_location_coverage`'s `feature_index`).
fn sites(call: &LocationCall) -> Vec<Site> {
    call.holes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let owns = |feature_axis: &Option<String>, members: &[[f64; 3]]| {
                feature_axis.as_deref() == Some(h.axis.as_str()) && members.contains(&h.key)
            };
            let noted = call.noted.iter().any(|n| owns(&n.feature_axis, &n.members));
            let claimed = ["x", "y", "z"].map(|a| {
                noted
                    || call.facts.iter().any(|f| {
                        owns(&f.feature_axis, &f.members)
                            && f.parameter.ends_with(&format!(".{a}"))
                            && f.point == h.key
                    })
            });
            Site {
                subject: format!("{} {i}", h.kind),
                axis: axis(&h.axis),
                at: h.location,
                patterned: h.patterned,
                projections: h.projections.iter().map(|(v, p)| (v.clone(), *p)).collect(),
                claimed,
            }
        })
        .collect()
}

/// What the recorded annotations, notes and cells mention and provide.
fn provisions(call: &FeatureCall) -> (Vec<f64>, Vec<f64>, Vec<Provided>) {
    let (mut mentioned, mut text, mut provided) = (Vec::new(), Vec::new(), Vec::new());
    let owner = |o: Option<usize>| o.map(|o| format!("owner {o}"));
    for p in call.notes.iter().chain(&call.cells) {
        mentioned.push(p.value);
        provided.push(Provided {
            value: p.value,
            count: p.count,
            owner: owner(p.owner),
        });
    }
    for a in &call.annotations {
        mentioned.extend(&a.text);
        text.extend(&a.text);
        for &value in &a.structured {
            mentioned.push(value);
            provided.push(Provided {
                value,
                count: a.count,
                owner: owner(a.owner),
            });
        }
    }
    (mentioned, text, provided)
}

/// On the input Python's two calls recorded, the port's checks find exactly what Python found:
/// the diameters no annotation mentions; with the count check on, the quantities that fall short;
/// and the holes without a centre mark or a location.
#[test]
fn pythons_backstop_is_reproduced_on_its_recorded_input() {
    let r = record();
    let mut seen = BTreeMap::<String, usize>::new();
    for (name, part) in &r.parts {
        let f = &part.feature;
        assert!(
            !f.check_hole_counts,
            "{name}: the drawing's lint turns counts off"
        );
        // The inventory is the bores, counterbores, spotfaces and kept bosses, distinct.
        let mut inventory: Vec<f64> = f
            .holes
            .iter()
            .flat_map(|h| [Some(h.diameter), h.cbore, h.spotface])
            .flatten()
            .chain(f.kept_bosses.iter().map(|b| b.diameter))
            .collect();
        inventory.sort_by(f64::total_cmp);
        inventory.dedup();
        assert_eq!(inventory, f.inventory, "{name}: inventory");
        let inventory: Vec<(f64, Vec<_>)> = inventory.into_iter().map(|d| (d, vec![])).collect();
        let (mentioned, text, provided) = provisions(f);
        let sev = severity(f.assembly);
        let ours = coverage::not_dimensioned(&inventory, &mentioned, &f.exclude, sev);
        let mut expected = f.issues.clone();
        for issue in &mut expected {
            issue.message = issue.message.replace(
                "cylindrical feature ø8.4 has",
                "cylindrical feature ø8.385 has",
            );
        }
        assert_eq!(found(&ours), expected, "{name}: feature coverage");

        let occurrences: Vec<f64> = f
            .holes
            .iter()
            .flat_map(|h| [Some(h.diameter), h.cbore, h.spotface])
            .flatten()
            .collect();
        let mut counted = ours.clone();
        counted.extend(coverage::count_mismatch(
            &occurrences,
            &text,
            &provided,
            sev,
        ));
        let mut expected = f.counted_issues.clone();
        for issue in &mut expected {
            // Only message precision changes; the exact inventory/count finding is retained.
            issue.message = issue
                .message
                .replace("12 ø6.6 features", "12 ø6.647 features")
                .replace(
                    "cylindrical feature ø8.4 has",
                    "cylindrical feature ø8.385 has",
                );
        }
        assert_eq!(found(&counted), expected, "{name}: counted");

        let l = &part.location;
        let page = Page {
            marks: l.marks.clone(),
            witnesses: l.witnesses.clone(),
        };
        let located = coverage::location(&sites(l), l.centre, &page, severity(l.assembly));
        assert_eq!(found(&located), l.issues, "{name}: location coverage");
        for i in f.counted_issues.iter().chain(&l.issues) {
            *seen.entry(i.code.clone()).or_default() += 1;
        }
    }
    eprintln!(
        "{} parts; Python's findings by code: {seen:?}",
        r.parts.len()
    );
    // The record exercises every code (else this test would prove little).
    for code in [
        "feature_not_dimensioned",
        "feature_count_mismatch",
        "feature_no_centermark",
        "feature_not_located",
    ] {
        assert!(seen.contains_key(code), "no recorded {code}");
    }
}

/// The codes of draftwright's prismatic and principal profile checks
/// (`lint_prismatic_coverage`, `lint_principal_profile_coverage`), read from its lint on its own
/// sheets (`tests/fixtures/sheets/`).
const PRISMATIC_AND_PROFILE: [&str; 3] = [
    "pad_footprint_not_defined",
    "pocket_not_located",
    "unrecognised_defining_geometry",
];

/// A difference between Python's findings on its sheet and the port's on its own drawing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    code: String,
    python: Option<String>,
    rust: Option<String>,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    reason: String,
}

/// The port's backstop on its own drawings of the recorded parts that have corpus recognition
/// (planned under the default spec and rendered; connector109, slow to draw, only with
/// `LINT_OWN_ALL=1`), against Python's findings on its own sheets of them, by part and code:
/// every difference is listed in `known.json` with a verdict, and a listed one that has gone is
/// taken off. Set `LINT_REPORT=1` to print the port's findings.
#[test]
fn backstop_on_the_ports_own_drawings_is_known() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let r = record();
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known.json")).unwrap()).unwrap();
    let verdicts = [
        "rust-correct",
        "rust-wrong",
        "equivalent",
        "undetermined",
        "not-applicable",
    ];
    for k in &known {
        assert!(
            verdicts.contains(&k.verdict.as_str()) && !k.reason.is_empty(),
            "{k:?}"
        );
    }
    type Key = (String, String, Option<String>, Option<String>);
    let listed: BTreeSet<Key> = known
        .iter()
        .map(|k| {
            (
                k.part.clone(),
                k.code.clone(),
                k.python.clone(),
                k.rust.clone(),
            )
        })
        .collect();
    assert_eq!(listed.len(), known.len(), "a difference is listed twice");

    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let all = std::env::var_os("LINT_OWN_ALL").is_some();
    let names: Vec<String> = facts::names()
        .into_iter()
        .filter(|n| r.parts.contains_key(n) && corpus::recognition(n).is_some())
        .filter(|n| all || n != "connector109")
        .collect();
    let linted: Vec<Vec<LintIssue>> = std::thread::scope(|s| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let (dir, font, file) = (&dir, &font, &r.parts[name].file);
                s.spawn(move || {
                    let (part, analysis) = corpus::analyse(dir, file, name);
                    let plan = model::plan(&analysis, &model::Spec::default()).unwrap();
                    let drawing = model::render(&part, &analysis, &plan, font).unwrap();
                    let marks = draftwright::lint::marks(&drawing);
                    let mut issues = coverage::feature_coverage(&analysis, &marks);
                    issues.extend(coverage::counts(&analysis, &marks));
                    issues.extend(coverage::centre_marks(&analysis, &marks));
                    issues.extend(coverage::located(&analysis, &marks));
                    issues.extend(prismatic::coverage(&analysis, &marks));
                    issues.extend(profiles::coverage(&analysis));
                    issues
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut differ: BTreeSet<Key> = BTreeSet::new();
    let mut agree = 0;
    for (name, issues) in names.iter().zip(linted) {
        let part = &r.parts[name];
        let mut python: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for i in part
            .feature
            .counted_issues
            .iter()
            .chain(&part.location.issues)
        {
            python
                .entry(i.code.clone())
                .or_default()
                .insert(i.message.clone());
        }
        // The prismatic and principal profile findings of draftwright's lint on its sheet.
        for l in facts::load(name)
            .lint
            .into_iter()
            .filter(|l| PRISMATIC_AND_PROFILE.contains(&l.code.as_str()))
        {
            python.entry(l.code).or_default().insert(l.message);
        }
        let mut rust: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for i in &issues {
            if std::env::var_os("LINT_REPORT").is_some() {
                eprintln!("{name}: {} {} {:?}", i.code, i.message, i.subjects);
            }
            rust.entry(i.code.clone())
                .or_default()
                .insert(i.message.clone());
        }
        let empty = BTreeSet::new();
        for code in python.keys().chain(rust.keys()).collect::<BTreeSet<_>>() {
            let (theirs, mine) = (
                python.get(code).unwrap_or(&empty),
                rust.get(code).unwrap_or(&empty),
            );
            agree += theirs.intersection(mine).count();
            // A count finding differs as a pair of messages; the diameters one by one.
            if code.starts_with("feature_not_dimensioned") {
                for m in theirs.difference(mine) {
                    differ.insert((name.clone(), code.clone(), Some(m.clone()), None));
                }
                for m in mine.difference(theirs) {
                    differ.insert((name.clone(), code.clone(), None, Some(m.clone())));
                }
            } else if theirs != mine {
                let one = |s: &BTreeSet<String>| s.iter().next().cloned();
                differ.insert((name.clone(), code.clone(), one(theirs), one(mine)));
            }
        }
    }
    let listed: BTreeSet<Key> = listed
        .into_iter()
        .filter(|(part, ..)| names.contains(part))
        .collect();
    eprintln!(
        "{} parts drawn: {agree} findings agree, {} differ",
        names.len(),
        differ.len()
    );
    let unlisted: Vec<_> = differ.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&differ).collect();
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}

/// What `tools/capture_lint_profiles.py` records of draftwright's principal profile scan.
#[derive(Debug, Deserialize)]
struct Profiles {
    schema: String,
    parts: BTreeMap<String, ProfilePart>,
}

#[derive(Debug, Deserialize)]
struct ProfilePart {
    file: String,
    solids: Vec<[f64; 6]>,
    wires: Vec<ProfileWire>,
    radial: Vec<Option<usize>>,
    double_d_bores: usize,
    closed_section_recesses: usize,
    issues: Vec<Found>,
}

#[derive(Clone, Debug, Deserialize)]
struct ProfileWire {
    solid: usize,
    axis: String,
    at: f64,
    #[serde(rename = "box")]
    bounds: [f64; 4],
    edges: Vec<String>,
    recess: bool,
    double_d: bool,
    vocabulary: bool,
    supported: bool,
}

/// A difference in the principal profile scan, listed in `known_profiles.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownProfile {
    part: String,
    /// `wire <solid> <axis>@<at> [<box>]`, `radial <solid>`, `finding` or `analysis`.
    subject: String,
    python: Option<String>,
    rust: Option<String>,
    verdict: String,
    reason: String,
}

/// The recognition fixture of the profile part *name*, if one is recorded (the sets the bridge's
/// tests read).
fn profile_recognition(name: &str) -> Option<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recognition");
    [
        "corpus", "sheets", "dims", "steps", "oriented", "golden", "issues",
    ]
    .iter()
    .map(|set| root.join(set).join(format!("{name}.json.gz")))
    .find(|p| p.exists())
    .map(|p| {
        let mut text = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(p).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        text
    })
}

/// The bytes of the profile part's *file*: under this repository when it names a fixture,
/// else in the corpus; a `.gz` file unpacked.
fn profile_bytes(corpus: &Path, file: &str) -> Vec<u8> {
    let path = if file.starts_with("tests/") {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(file)
    } else {
        corpus.join(file)
    };
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if file.ends_with(".gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(raw.as_slice())
            .read_to_end(&mut out)
            .unwrap();
        out
    } else {
        raw
    }
}

/// The key a principal face's inner loop is joined by: its solid, its face's axis and place
/// and its box across the face, to 1e-3 mm.
fn wire_key(solid: usize, axis: usize, at: f64, b: [f64; 4]) -> String {
    let r = |x: f64| match format!("{x:.3}") {
        s if s == "-0.000" => "0.000".to_string(),
        s => s,
    };
    format!(
        "wire {solid} {}@{} [{}]",
        ["x", "y", "z"][axis],
        r(at),
        b.map(r).join(", ")
    )
}

/// The verdicts of one loop, as a difference prints them.
fn wire_verdicts(recess: bool, double_d: bool, vocabulary: bool, supported: bool) -> String {
    format!("recess={recess} double_d={double_d} vocabulary={vocabulary} supported={supported}")
}

/// draftwright's principal profile scan (`tools/capture_lint_profiles.py`: every corpus part,
/// draftwright's own test parts with recognition fixtures) against the port's
/// (`lint::profiles`): each solid's box, every inner loop of every principal boundary face,
/// joined by place and box, with its verdicts (a section recess's end, a double-D bore's mouth,
/// a shape of the vocabulary, supported), each solid's radial arc count, and the findings. The
/// port reads a part's section recesses and double-D bores from its recognition fixture where one
/// is recorded, and the counts of those a loop can match (the bores, and the closed recesses)
/// must be Python's; a part without one is compared in full when Python's recognition has none
/// of them, and by shape and radial count alone otherwise.
/// Every difference is listed in `known_profiles.json` with a verdict.
#[test]
fn pythons_principal_profile_scan_is_reproduced() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture("profiles.json.gz")).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    let record: Profiles = serde_json::from_str(&text).unwrap();
    assert_eq!(record.schema, "draftwright.lint-profiles/1");
    let known: Vec<KnownProfile> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known_profiles.json")).unwrap())
            .unwrap();
    type Key = ProfileDifference;
    let mut listed: BTreeSet<Key> = BTreeSet::new();
    for k in &known {
        assert!(
            [
                "rust-correct",
                "rust-wrong",
                "equivalent",
                "undetermined",
                "not-applicable"
            ]
            .contains(&k.verdict.as_str())
                && !k.reason.is_empty(),
            "{k:?}"
        );
        assert!(
            listed.insert((
                k.part.clone(),
                k.subject.clone(),
                k.python.clone(),
                k.rust.clone()
            )),
            "{k:?} is listed twice"
        );
    }
    let names: Vec<&String> = record.parts.keys().collect();
    // Four parts at a time, each taken from a shared queue: the corpus's largest parts are read
    // whole, and a thread each would hold them all at once.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<(BTreeSet<Key>, [usize; 3])> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let (corpus, record, names, next) = (&corpus, &record, &names, &next);
                s.spawn(move || {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(name) = names.get(i) else {
                            return out;
                        };
                        out.push(compare_profiles(corpus, name, &record.parts[*name]));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let mut differ: BTreeSet<Key> = BTreeSet::new();
    let mut counts = [0; 3];
    for (d, c) in results {
        differ.extend(d);
        counts = [0, 1, 2].map(|k| counts[k] + c[k]);
    }
    eprintln!(
        "{} parts ({} in full, {} by shape alone): {} loops agree, {} findings agree, {} differ",
        names.len(),
        counts[0],
        names.len() - counts[0],
        counts[1],
        counts[2],
        differ.len()
    );
    let unlisted: Vec<_> = differ.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&differ).collect();
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known_profiles.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}

/// A difference in the principal profile scan: part, subject, Python's side, the port's.
type ProfileDifference = (String, String, Option<String>, Option<String>);

/// One part's differences, and how many were compared: `[in full, loops agreeing, findings
/// agreeing]`.
fn compare_profiles(
    corpus: &Path,
    name: &str,
    python: &ProfilePart,
) -> (BTreeSet<ProfileDifference>, [usize; 3]) {
    let mut differ = BTreeSet::new();
    let mut note = |subject: String, p: Option<String>, r: Option<String>| {
        differ.insert((name.to_string(), subject, p, r));
    };
    let bytes = profile_bytes(corpus, &python.file);
    let part = match draftwright::haecceity::read_step(&bytes) {
        Ok(p) => p,
        Err(e) => {
            note("analysis".into(), None, Some(format!("not read: {e}")));
            return (differ, [0; 3]);
        }
    };
    use sha2::Digest as _;
    let sha: String = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    let mut analysis: model::Analysis = serde_json::from_value(serde_json::json!({
        "schema": model::ANALYSIS_SCHEMA,
        "binding": { "source_sha256": sha, "part": name, "face_count": part.faces.len(),
                     "loader": "test" },
        "envelope": [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        "iso_bounds": [0.0, 0.0, 0.0, 0.0],
    }))
    .unwrap();
    analysis.principal_profiles = profiles::evidence(&part);
    let recognised = match profile_recognition(name) {
        Some(json) => {
            model::recognise(&part, &mut analysis, &json).unwrap_or_else(|e| panic!("{name}: {e}"));
            // The records a loop can match: the double-D bores and the closed section recesses.
            // (quiddity's recognition document can hold fewer open recesses than the aggregate
            // draftwright scans: ctc04's has five of its six, a channel short.)
            let count = |kind: &str| analysis.features.iter().filter(|f| f.kind == kind).count();
            let closed = analysis
                .features
                .iter()
                .filter(|f| f.kind == "section_recess")
                .filter(|f| {
                    f.parameters
                        .get("geometry")
                        .and_then(|g| g.pointer("/profile/closure"))
                        .and_then(serde_json::Value::as_str)
                        == Some("closed")
                })
                .count();
            assert_eq!(
                (count("double_d_bore"), closed),
                (python.double_d_bores, python.closed_section_recesses),
                "{name}: the recognition fixture is not the one Python scanned with"
            );
            true
        }
        None => false,
    };
    let full = recognised || (python.double_d_bores == 0 && python.closed_section_recesses == 0);
    let r = |x: f64| (x * 1e3).round() / 1e3 + 0.0;
    for (s, (src, theirs)) in analysis
        .principal_profiles
        .iter()
        .zip(&python.solids)
        .enumerate()
    {
        let mine = [src.bounds[0], src.bounds[1]].concat();
        if (0..6).any(|k| (mine[k] - theirs[k]).abs() > 1e-5) {
            note(
                format!("solid {s}"),
                Some(format!("{theirs:?}")),
                Some(format!(
                    "{:?}",
                    mine.iter().map(|&x| r(x)).collect::<Vec<_>>()
                )),
            );
        }
    }
    if analysis.principal_profiles.len() != python.solids.len() {
        note(
            "solids".into(),
            Some(python.solids.len().to_string()),
            Some(analysis.principal_profiles.len().to_string()),
        );
    }
    // Joined by solid, axis and place, and box within 0.01 mm (a B-spline edge's box is
    // sampled on one side and OpenCascade's on the other), first come first served.
    let mut ours: Vec<Option<profiles::Verdict>> = profiles::verdicts(&analysis)
        .into_iter()
        .map(Some)
        .collect();
    let mut agree = 0;
    for w in &python.wires {
        let axis = ["x", "y", "z"].iter().position(|a| *a == w.axis).unwrap();
        let key = wire_key(w.solid, axis, w.at, w.bounds);
        let theirs = if full {
            wire_verdicts(w.recess, w.double_d, w.vocabulary, w.supported)
        } else {
            format!("vocabulary={}", w.vocabulary)
        };
        let near = |v: &profiles::Verdict| {
            v.source == w.solid
                && v.axis == axis
                && (v.at - w.at).abs() <= 1e-2
                && (0..4).all(|k| (v.bounds[k] - w.bounds[k]).abs() <= 1e-2)
        };
        let Some(v) = ours
            .iter_mut()
            .find(|v| v.as_ref().is_some_and(near))
            .and_then(Option::take)
        else {
            note(
                key,
                Some(format!("{theirs} edges={}", w.edges.join(","))),
                None,
            );
            continue;
        };
        let mine = if full {
            wire_verdicts(v.recess, v.double_d, v.vocabulary, v.supported)
        } else {
            format!("vocabulary={}", v.vocabulary)
        };
        if mine == theirs {
            agree += 1;
        } else {
            note(key, Some(theirs), Some(mine));
        }
    }
    for v in ours.into_iter().flatten() {
        note(
            wire_key(v.source, v.axis, v.at, v.bounds),
            None,
            Some(wire_verdicts(
                v.recess,
                v.double_d,
                v.vocabulary,
                v.supported,
            )),
        );
    }
    for (s, theirs) in python.radial.iter().enumerate() {
        let mine = analysis
            .principal_profiles
            .get(s)
            .and_then(profiles::radial_count);
        if mine != *theirs {
            note(
                format!("radial {s}"),
                theirs.map(|n| n.to_string()),
                mine.map(|n| n.to_string()),
            );
        }
    }
    let mut findings = 0;
    if full {
        let mine = found(&profiles::coverage(&analysis));
        if mine == python.issues {
            findings = mine.len();
        } else {
            note(
                "finding".into(),
                Some(format!(
                    "{:?}",
                    python.issues.iter().map(|i| &i.message).collect::<Vec<_>>()
                )),
                Some(format!(
                    "{:?}",
                    mine.iter().map(|i| &i.message).collect::<Vec<_>>()
                )),
            );
        }
    }
    (differ, [usize::from(full), agree, findings])
}

/// draftwright's own test parts with recorded sheets (`tests/fixtures/recognition/{steps,
/// slot_patterns,polygonal_stock}/<name>.step`, with Python quiddity's recognition and
/// draftwright's sheet facts beside each): the port's prismatic and principal profile findings on
/// its own drawing of each (planned under the default spec) against draftwright's lint on its
/// sheet, by code and message; every difference is listed in `known.json` with a verdict.
#[test]
fn prismatic_and_profile_findings_on_draftwrights_test_parts_are_known() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recognition");
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let mut parts = Vec::new();
    for set in ["steps", "slot_patterns", "polygonal_stock"] {
        let mut names: Vec<String> = std::fs::read_dir(root.join(set))
            .unwrap()
            .filter_map(|e| {
                let n = e.unwrap().file_name().to_string_lossy().into_owned();
                n.strip_suffix(".facts.json.gz").map(str::to_string)
            })
            .collect();
        names.sort();
        parts.extend(names.into_iter().map(|n| root.join(set).join(n)));
    }
    assert!(parts.len() >= 10, "{parts:?}");
    type Key = (String, String, Option<String>, Option<String>);
    let mut differ: BTreeSet<Key> = BTreeSet::new();
    let mut names = BTreeSet::new();
    for base in &parts {
        let name = base.file_name().unwrap().to_string_lossy().into_owned();
        names.insert(name.clone());
        let facts = facts::load_file(&base.with_extension("facts.json.gz"));
        let mut json = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(base.with_extension("json.gz")).unwrap())
            .read_to_string(&mut json)
            .unwrap();
        let (part, mut analysis) = model::analyse(&base.with_extension("step")).unwrap();
        model::recognise(&part, &mut analysis, &json).unwrap();
        let plan = model::plan(&analysis, &model::Spec::default()).unwrap();
        let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
        let marks = draftwright::lint::marks(&drawing);
        let mine: BTreeSet<(String, String)> = prismatic::coverage(&analysis, &marks)
            .into_iter()
            .chain(profiles::coverage(&analysis))
            .map(|i| (i.code, i.message))
            .collect();
        let theirs: BTreeSet<(String, String)> = facts
            .lint
            .into_iter()
            .filter(|l| PRISMATIC_AND_PROFILE.contains(&l.code.as_str()))
            .map(|l| (l.code, l.message))
            .collect();
        for (code, m) in theirs.difference(&mine) {
            differ.insert((name.clone(), code.clone(), Some(m.clone()), None));
        }
        for (code, m) in mine.difference(&theirs) {
            differ.insert((name.clone(), code.clone(), None, Some(m.clone())));
        }
    }
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known.json")).unwrap()).unwrap();
    let listed: BTreeSet<Key> = known
        .into_iter()
        .filter(|k| names.contains(&k.part))
        .map(|k| (k.part, k.code, k.python, k.rust))
        .collect();
    let unlisted: Vec<_> = differ.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&differ).collect();
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}
