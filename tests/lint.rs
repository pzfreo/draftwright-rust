//! Lint on Python draftwright's captured sheets (`tests/fixtures/sheets/`), against the findings
//! Python's own lint made on them.
//!
//! The port's lint judges a drawing from what it prints and claims, against the Analysis. Here
//! the drawing is Python's: each captured annotation becomes a mark (its PDF text runs, and the
//! measurements its registry says it draws), and the Analysis is the part read with the same
//! Python quiddity recognition draftwright drew from (`tests/fixtures/recognition/sheets/`,
//! written by `tools/recognise.py` with draftwright's quiddity 0.3.9).
//!
//! Python names a measured feature by its geometry (`hole@(x,y,z)/axis[sizes]`, or
//! `pattern@(centre)/axis`), so each is joined to the Analysis exactly, and only when exactly one
//! hole (or one hole group) fits: the same axis, the opening on its axis line within 0.01 mm and
//! within its length, the same diameter. A claim that fits none or several is dropped and counted
//! (LINT-_coverage_common-exact_correspondence: never guess).
//!
//! Every difference between the two lints' hole findings must be listed, with a verdict and the
//! reason, in `tests/fixtures/known_lint.json`; a listed difference that has gone away must be
//! taken off the list. The parts are read from the corpus (`DRAFTWRIGHT_CORPUS`, by default
//! `~/repos/quiddity/tests/corpus`).

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use draftwright::lint::holes::{HoleGroup, groups};
use draftwright::lint::{self, Mark};
use draftwright::model::{self, Analysis};
use draftwright::output::Claim;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    /// The hole group's first member, its description, and the requirement (`*` for all).
    subject: String,
    group: String,
    parameter: String,
    /// Each side's finding code, or none.
    python: Option<String>,
    rust: Option<String>,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    reason: String,
}

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// The part with its features, from the recorded recognition, or why the bridge refuses it.
fn analysis(corpus: &Path, name: &str, file: &str) -> Result<Analysis, String> {
    let step = corpus.join(file);
    let (part, mut analysis) = model::analyse(&step).unwrap();
    let mut json = String::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(fixture(&format!("recognition/sheets/{name}.json.gz"))).unwrap(),
    )
    .read_to_string(&mut json)
    .unwrap();
    let (recognition, features) =
        draftwright::bridge::read(&part, &analysis.binding.source_sha256, &json)?;
    analysis.recognition = Some(recognition);
    analysis.features = features;
    Ok(analysis)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// How far *p* is from the line through *c* along the unit *axis*, across it and along it.
fn offsets(p: [f64; 3], c: [f64; 3], axis: [f64; 3]) -> (f64, f64) {
    let d = sub(p, c);
    let along = dot(d, axis);
    let across = (dot(d, d) - along * along).max(0.0).sqrt();
    (across, along.abs())
}

fn letter(axis: &str) -> Option<[f64; 3]> {
    match axis {
        "x" => Some([1.0, 0.0, 0.0]),
        "y" => Some([0.0, 1.0, 0.0]),
        "z" => Some([0.0, 0.0, 1.0]),
        _ => None,
    }
}

/// A hole's length along its axis: the bore and any counterbore, spotface or countersink (a
/// drill point's cone is not in the depth; [`reaches`] allows for it).
fn length(f: &draftwright::bridge::Feature) -> f64 {
    let p = &f.parameters;
    let get = |k: &str, j: &str| {
        p.get(k)
            .and_then(|v| v.get(j))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    };
    p.get("depth").and_then(|v| v.as_f64()).unwrap_or(0.0)
        + get("cbore", "depth")
        + get("spotface", "depth")
        + get("csink", "depth")
}

/// How far a hole's opening can be from its centre (the centre of its faces' box) along its
/// axis: half its length, and half its diameter for a drill point's cone.
fn reaches(f: &draftwright::bridge::Feature) -> f64 {
    let d = f
        .parameters
        .get("diameter")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    0.5 * (length(f) + d) + 0.01
}

/// The Analysis holes a Python feature names, when exactly one hole (or one group) fits.
fn join(
    analysis: &Analysis,
    groups: &[HoleGroup],
    feature: &facts::Feature,
    value: Option<f64>,
) -> Option<Vec<String>> {
    let axis = letter(feature.axis.as_deref()?)?;
    let origin = feature.origin?;
    let parallel = |a: Option<[f64; 3]>| a.is_some_and(|a| dot(a, axis).abs() > 1.0 - 1e-6);
    match feature.kind.as_str() {
        "hole" => {
            let key = feature.key.as_deref()?;
            let diameter: f64 = key
                .split("diameter=")
                .nth(1)?
                .split([',', ']'])
                .next()?
                .parse()
                .ok()?;
            let fits: Vec<&draftwright::bridge::Feature> = analysis
                .features
                .iter()
                .filter(|f| f.kind == "hole" && parallel(f.fingerprint.axis))
                .filter(|f| {
                    let d = f.parameters.get("diameter").and_then(|v| v.as_f64());
                    d.is_some_and(|d| (d - diameter).abs() <= 1e-3)
                })
                .filter(|f| {
                    let (across, along) = offsets(origin, f.fingerprint.position, axis);
                    across <= 0.01 && along <= reaches(f)
                })
                .collect();
            (fits.len() == 1).then(|| vec![fits[0].id.clone()])
        }
        "pattern" => {
            let fits: Vec<&HoleGroup> = groups
                .iter()
                .filter(|g| g.members.len() > 1 && parallel(g.axis))
                .filter(|g| value.is_none_or(|v| (v - g.sizes.diameter).abs() <= 1e-3))
                .filter(|g| {
                    let (across, along) = offsets(origin, g.centre(), axis);
                    let half = g
                        .members
                        .iter()
                        .filter_map(|m| analysis.features.iter().find(|f| &f.id == m))
                        .map(reaches)
                        .fold(0.0, f64::max);
                    across <= 0.01 && along <= half + 0.01
                })
                .collect();
            (fits.len() == 1).then(|| fits[0].members.clone())
        }
        _ => None,
    }
}

/// The parameters hole coverage reads.
fn sized(parameter: &str) -> bool {
    ["bore.", "counterbore.", "spotface.", "countersink."]
        .iter()
        .any(|p| parameter.starts_with(p))
}

/// Python's sheet as marks, and how many of its hole size claims could not be joined.
fn marks(analysis: &Analysis, groups: &[HoleGroup], f: &facts::Facts) -> (Vec<Mark>, usize) {
    let mut unjoined = 0;
    let marks = f
        .annotations
        .iter()
        .map(|a| {
            let mut claims = Vec::new();
            for m in &a.measurements {
                let Some(feature) = &m.feature else { continue };
                if !["hole", "pattern"].contains(&feature.kind.as_str()) || !sized(&m.parameter) {
                    continue;
                }
                let value = (m.parameter == "bore.diameter")
                    .then_some(m.value)
                    .flatten();
                match join(analysis, groups, feature, value) {
                    Some(ids) => claims.extend(ids.into_iter().map(|id| Claim {
                        feature: id,
                        parameter: m.parameter.clone(),
                    })),
                    None => unjoined += 1,
                }
            }
            Mark {
                name: a.name.clone(),
                view: a.view.clone().map(|v| format!("v:{v}")),
                text: a.text.join(" "),
                claims,
            }
        })
        .collect();
    (marks, unjoined)
}

/// The group a Python hole finding is about: its message names the quantity, the diameter and
/// a point on the holes' axis (or the pattern's), in the part's frame.
fn python_subject(analysis: &Analysis, groups: &[HoleGroup], message: &str) -> (usize, String) {
    let n: usize = message.split('×').next().unwrap().trim().parse().unwrap();
    let diameter: f64 = message
        .split('ø')
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let at: Vec<f64> = message
        .split(" at (")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap()
        .split(", ")
        .map(|c| c.parse().unwrap())
        .collect();
    let at = [at[0], at[1], at[2]];
    let parameter = match message.split(" requirement ").nth(1) {
        Some(rest) => rest.split(' ').next().unwrap().to_string(),
        None => "*".to_string(),
    };
    let fits: Vec<usize> = groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.members.len() == n && (g.sizes.diameter - diameter).abs() < 0.05)
        .filter(|(_, g)| {
            let axis = g.axis.unwrap();
            let reach = g
                .members
                .iter()
                .filter_map(|m| analysis.features.iter().find(|f| &f.id == m))
                .map(reaches)
                .fold(0.0, f64::max);
            // Python zeroes a loose through hole's coordinate along its axis.
            let anywhere = g.sizes.through && !g.pattern;
            let on = |c: [f64; 3]| {
                let (across, along) = offsets(at, c, axis);
                across <= 0.01 && (anywhere || along <= reach)
            };
            on(g.centre()) || g.centres.iter().any(|c| on(*c))
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(fits.len(), 1, "{message}: {fits:?}");
    (fits[0], parameter)
}

#[test]
fn hole_coverage_on_pythons_sheets_matches_pythons_lint_or_is_known() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known_lint.json")).unwrap())
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
            verdicts.contains(&k.verdict.as_str()) && !k.reason.is_empty(),
            "{k:?}"
        );
    }
    type Key = (String, String, String, Option<String>, Option<String>);
    let listed: BTreeSet<Key> = known
        .iter()
        .map(|k| {
            (
                k.part.clone(),
                k.subject.clone(),
                k.parameter.clone(),
                k.python.clone(),
                k.rust.clone(),
            )
        })
        .collect();
    assert_eq!(listed.len(), known.len(), "a difference is listed twice");

    let mut differ: BTreeMap<Key, String> = BTreeMap::new();
    let (mut agree, mut joined, mut unjoined_total, mut judged) = (0, 0, 0, 0);
    for name in facts::names() {
        let f = facts::load(&name);
        let analysis = match analysis(&corpus, &name, &f.binding.file) {
            Ok(a) => a,
            Err(e) => {
                // The whole sheet is one difference: the port cannot read the part's features.
                differ.insert(
                    (name.clone(), "analysis".into(), "*".into(), None, None),
                    format!("the bridge refuses the recognition: {e}"),
                );
                continue;
            }
        };
        assert_eq!(analysis.binding.source_sha256, f.binding.sha256, "{name}");
        let groups = groups(&analysis);
        let (marks, unjoined) = marks(&analysis, &groups, &f);
        unjoined_total += unjoined;
        joined += marks.iter().map(|m| m.claims.len()).sum::<usize>();
        let ours = lint::check(&analysis, &marks);
        judged += groups.iter().map(|g| g.requirements().len()).sum::<usize>();

        // Each side's findings by (group, requirement).
        let mut python: BTreeMap<(usize, String), String> = BTreeMap::new();
        for finding in f
            .lint
            .iter()
            .filter(|l| l.code.starts_with("hole_requirement_"))
        {
            python.insert(
                python_subject(&analysis, &groups, &finding.message),
                finding.code.clone(),
            );
        }
        let mut rust: BTreeMap<(usize, String), String> = BTreeMap::new();
        for issue in &ours {
            let g = groups
                .iter()
                .position(|g| g.members == issue.subjects)
                .unwrap();
            rust.insert(
                (g, issue.requirements[0].parameter.clone()),
                issue.code.clone(),
            );
        }
        let r = |x: f64| match format!("{x:.3}") {
            s if s == "-0.000" => "0.000".to_string(),
            s => s,
        };
        let describe = |g: usize| {
            let h = &groups[g];
            let c = h.centre();
            format!(
                "{}× ø{} {} at ({}, {}, {})",
                h.members.len(),
                h.sizes.diameter,
                if h.sizes.through { "THRU" } else { "blind" },
                r(c[0]),
                r(c[1]),
                r(c[2])
            )
        };
        let mut seen = BTreeSet::new();
        for ((g, parameter), code) in &python {
            seen.insert((*g, parameter.clone()));
            // Python's "all requirements" finding stands for each of the group's requirements
            // (those lint has ported: its locations are not yet).
            let theirs: Vec<(&str, Option<&String>)> = if parameter == "*" {
                groups[*g]
                    .requirements()
                    .iter()
                    .map(|(p, _)| (*p, rust.get(&(*g, p.to_string()))))
                    .collect()
            } else {
                vec![(parameter.as_str(), rust.get(&(*g, parameter.clone())))]
            };
            for (p, r) in theirs {
                seen.insert((*g, p.to_string()));
                if r == Some(code) {
                    agree += 1;
                } else {
                    differ.insert(
                        (
                            name.clone(),
                            groups[*g].members[0].clone(),
                            p.to_string(),
                            Some(code.clone()),
                            r.cloned(),
                        ),
                        describe(*g),
                    );
                }
            }
        }
        for ((g, parameter), code) in &rust {
            if !seen.contains(&(*g, parameter.clone())) {
                differ.insert(
                    (
                        name.clone(),
                        groups[*g].members[0].clone(),
                        parameter.clone(),
                        None,
                        Some(code.clone()),
                    ),
                    describe(*g),
                );
            }
        }
        if std::env::var_os("LINT_REPORT").is_some() {
            eprintln!(
                "== {name}: {} groups, {} marks claim holes, {unjoined} claims unjoined",
                groups.len(),
                marks.iter().filter(|m| !m.claims.is_empty()).count()
            );
            for (i, g) in groups.iter().enumerate() {
                eprintln!("   group {i}: {} {:?}", describe(i), g.members);
            }
            for i in &ours {
                eprintln!("   rust {} {}", i.code, i.message);
            }
            for l in f
                .lint
                .iter()
                .filter(|l| l.code.starts_with("hole_requirement_"))
            {
                eprintln!("   python {} {}", l.code, l.message);
            }
        }
    }
    let found: BTreeSet<Key> = differ.keys().cloned().collect();
    let unlisted: Vec<_> = differ.iter().filter(|(k, _)| !listed.contains(k)).collect();
    for k in &known {
        let key = (
            k.part.clone(),
            k.subject.clone(),
            k.parameter.clone(),
            k.python.clone(),
            k.rust.clone(),
        );
        if let Some(group) = differ.get(&key) {
            assert_eq!(
                &k.group, group,
                "{k:?}: the group's description has changed"
            );
        }
    }
    let stale: Vec<_> = listed.difference(&found).collect();
    eprintln!(
        "{judged} hole requirements judged: {agree} findings agree, {} differ ({} known), the \
         rest neither side reports; {joined} claims joined, {unjoined_total} not",
        differ.len(),
        known.len()
    );
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known_lint.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}

/// The part *name* (`tests/fixtures/parts/`) with *json* as its recognition, planned under
/// *spec*, rendered, and linted: each finding's code, first subject and requirement.
fn reviewed(name: &str, json: &Value, spec: &model::Spec) -> (model::Plan, Vec<String>) {
    let (part, mut analysis) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
    model::recognise(&part, &mut analysis, &json.to_string()).unwrap();
    let plan = model::plan(&analysis, spec).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let found = lint::lint(&analysis, &drawing)
        .into_iter()
        .map(|i| {
            format!(
                "{} {} {}",
                i.code, i.subjects[0], i.requirements[0].parameter
            )
        })
        .collect();
    (plan, found)
}

fn recognition(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(fixture(&format!("recognition/{name}.json"))).unwrap(),
    )
    .unwrap()
}

/// The port's own callout states the plate's four ø6 holes (`4× ⌀6 THRU (2×2)`), so review finds
/// nothing missing; with that callout omitted from the spec, each of their requirements is
/// missing again, and the fix says how to state them. Lint reads the drawing's annotations
/// (their text runs and claims), never the plan or the spec.
#[test]
fn review_finds_the_plates_holes_stated_by_its_own_callout() {
    let step = fixture("parts/plate.step");
    let features = fixture("recognition/plate.json");
    let review = draftwright::api::review(&step, Some(&features), &model::Spec::default()).unwrap();
    assert!(review.lint.is_empty(), "{:#?}", review.lint);

    let callout = "d:f:hole_pattern:1/bore.diameter@plan";
    let mut spec = model::Spec::default();
    spec.dimensions.entry(callout.into()).or_default().omit = true;
    let review = draftwright::api::review(&step, Some(&features), &spec).unwrap();
    let found: Vec<(&str, &str)> = review
        .lint
        .iter()
        .map(|i| (i.code.as_str(), i.requirements[0].parameter.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("hole_requirement_missing", "bore.diameter"),
            ("hole_requirement_missing", "bore.through"),
            ("hole_requirement_missing", "grouping.count"),
        ]
    );
    for issue in &review.lint {
        assert_eq!(issue.subjects.len(), 4, "{issue:?}");
        assert!(
            issue.fix.as_ref().unwrap().contains("\"4× ⌀6 THRU\""),
            "{issue:?}"
        );
    }
    // Without features there is nothing to require.
    let bare = draftwright::api::review(&step, None, &model::Spec::default()).unwrap();
    assert!(bare.lint.is_empty());
}

/// LINT-hole_coverage-csink_diameter_angle on the port's own drawing: the plate's holes
/// countersunk (quiddity's record: `major_diameter`, `included_angle`) are called out with their
/// countersink, which lint reads back as stated; a countersink whose angle quiddity did not give
/// is called out without it, and lint reports the angle unverifiable rather than guessing.
#[test]
fn a_countersunk_hole_is_called_out_and_its_callout_satisfies_lint() {
    let mut json = recognition("plate");
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        let r = &mut f["record"];
        r["csink"] = json!({
            "axis": r["axis"].clone(), "location": r["location"].clone(),
            "major_diameter": 12.0, "drill_diameter": 6.0, "included_angle": 90.0, "depth": 3.0,
        });
    }
    let (plan, found) = reviewed("plate", &json, &model::Spec::default());
    let callout = plan
        .dimensions
        .iter()
        .find(|d| d.kind == "callout")
        .unwrap();
    assert_eq!(callout.label, "4× ⌀6 THRU ⌵ ⌀12 × 90° (2×2)");
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "feature_not_dimensioned")
    );
    assert_eq!(found, Vec::<String>::new());

    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        f["record"]["csink"]
            .as_object_mut()
            .unwrap()
            .remove("included_angle");
    }
    let (plan, found) = reviewed("plate", &json, &model::Spec::default());
    assert!(
        plan.dimensions
            .iter()
            .any(|d| d.label == "4× ⌀6 THRU ⌵ ⌀12 (2×2)")
    );
    assert_eq!(
        found,
        ["hole_requirement_unverifiable f:hole:1 countersink.angle"]
    );
}

/// Two identical grids of 2×3 ø6 through holes 80 mm apart (`tests/fixtures/parts/two_grids.step`,
/// made with build123d: a 140×40×5 box less `Cylinder(3, 5)` at x = ±40 + {-10, 0, 10},
/// y = ±8), which quiddity 0.3.9 recognises as four identical linear patterns of three. Lint
/// groups the holes by the recognised patterns, as the dimensions call them out, so the port's
/// four `3× ⌀6 THRU` callouts state every requirement; its old fingerprint heuristic (count,
/// pitch, sizes, direction, level) could not tell the four apart and demanded `12×`. Python
/// draftwright on the same part prints the same four `3× ⌀6 THRU` callouts and its lint finds
/// nothing: the port now agrees (equivalent).
#[test]
fn identical_patterns_are_grouped_as_recognised() {
    let json = recognition("two_grids");
    let (part, mut analysis) = model::analyse(&fixture("parts/two_grids.step")).unwrap();
    model::recognise(&part, &mut analysis, &json.to_string()).unwrap();
    let g = groups(&analysis);
    assert_eq!(g.len(), 4);
    for (g, p) in g
        .iter()
        .zip(&analysis.recognition.as_ref().unwrap().patterns)
    {
        assert!(g.pattern);
        let mut members = p.members.clone();
        members.sort_by_key(|m| analysis.features.iter().position(|f| &f.id == m));
        assert_eq!(g.members, members);
    }
    let (plan, found) = reviewed("two_grids", &json, &model::Spec::default());
    let callouts: Vec<&str> = plan
        .dimensions
        .iter()
        .filter(|d| d.kind == "callout")
        .map(|d| d.label.as_str())
        .collect();
    assert_eq!(callouts, ["3× ⌀6 THRU"; 4]);
    assert_eq!(found, Vec::<String>::new());
    // Without the recognition record, the fingerprints alone cannot tell the patterns apart.
    analysis.recognition = None;
    let g = groups(&analysis);
    assert_eq!((g.len(), g[0].members.len(), g[0].pattern), (1, 12, false));
}

/// Lint reads a rendered drawing's own annotations: a callout that claims the plate's holes and
/// prints their sizes satisfies them; one that prints a wrong size, or is not there, does not.
#[test]
fn lint_reads_the_drawings_annotations() {
    use draftwright::output::{Annotation, HAlign, TextRun, VAlign};
    let (part, analysis) = model::analyse_with(
        &fixture("parts/plate.step"),
        Some(&fixture("recognition/plate.json")),
    )
    .unwrap();
    let plan = model::plan(&analysis, &model::Spec::default()).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let holes = &groups(&analysis)[0].members;
    let with = |label: &str| {
        // The drawing without the port's own callout, and one added by hand.
        let mut drawing = model::render(&part, &analysis, &plan, &font).unwrap();
        drawing.annotations.clear();
        drawing.text.push(TextRun {
            text: label.into(),
            at: [100.0, 100.0],
            size: 3.5,
            rotation: 0.0,
            font: "IBMPlexMono-Regular.ttf".into(),
            h_align: HAlign::Left,
            v_align: VAlign::Baseline,
        });
        drawing.annotations.push(Annotation {
            name: "d:f:hole:1/bore.diameter@v:plan".into(),
            view: Some("v:plan".into()),
            runs: vec![drawing.text.len() - 1],
            claims: holes
                .iter()
                .map(|h| Claim {
                    feature: h.clone(),
                    parameter: "bore.diameter".into(),
                })
                .collect(),
        });
        lint::lint(&analysis, &drawing)
            .into_iter()
            .map(|i| (i.code, i.requirements[0].parameter.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(with("4× ⌀6 THRU"), []);
    let s = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(
        with("4× ⌀6.5 THRU"),
        [s("hole_requirement_unverifiable", "bore.diameter")]
    );
    assert_eq!(
        with("⌀6"),
        [
            s("hole_requirement_missing", "bore.through"),
            s("hole_requirement_missing", "grouping.count")
        ]
    );
}

/// On a captured sheet, taking a callout away or misprinting it is found: lint is judging the
/// sheet, not echoing it.
#[test]
fn lint_finds_what_is_taken_off_a_captured_sheet() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let f = facts::load("m10127");
    let analysis = analysis(&corpus, "m10127", &f.binding.file).unwrap();
    let groups = groups(&analysis);
    let (marks, unjoined) = marks(&analysis, &groups, &f);
    assert_eq!(unjoined, 0);
    assert!(lint::check(&analysis, &marks).is_empty());
    // hc_side0 prints "⌀8.1 THRU" for the through hole.
    let codes = |marks: &[Mark]| {
        lint::check(&analysis, marks)
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
            .collect::<Vec<_>>()
    };
    let without: Vec<Mark> = marks
        .iter()
        .filter(|m| m.name != "hc_side0")
        .cloned()
        .collect();
    assert_eq!(
        codes(&without),
        [
            "hole_requirement_missing bore.diameter",
            "hole_requirement_missing bore.through"
        ]
    );
    let misprinted: Vec<Mark> = marks
        .iter()
        .cloned()
        .map(|mut m| {
            if m.name == "hc_side0" {
                m.text = m.text.replace("8.1", "8.4").replace("THRU", "");
            }
            m
        })
        .collect();
    assert_eq!(
        codes(&misprinted),
        [
            "hole_requirement_unverifiable bore.diameter",
            "hole_requirement_missing bore.through"
        ]
    );
}
