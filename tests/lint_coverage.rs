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
//! on its sheets; every difference, by part and code, is listed with a verdict and the reason in
//! `tests/fixtures/lint_coverage/known.json` (`backstop_on_the_ports_own_drawings_is_known`).

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
        assert_eq!(found(&ours), f.issues, "{name}: feature coverage");

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
        assert_eq!(found(&counted), f.counted_issues, "{name}: counted");

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
