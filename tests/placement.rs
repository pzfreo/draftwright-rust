//! Where the dimensions land, and what then goes wrong (review M3, ARCH0003-2).
//!
//! - **A ratchet** on the placement problems (`annotation_*`) of the port's own drawings of the
//!   captured corpus parts (the corpus's, with their features from
//!   `tests/fixtures/recognition/corpus/`, planned under the default spec): every part and code,
//!   with its count, is listed in `tests/fixtures/known_placement.json` with a verdict and the
//!   evidence. A new, changed or vanished entry fails, and the failure prints the entries to
//!   paste, each with the problems behind it, for a verdict. Set `PLACEMENT_REPORT=1` to print
//!   every placement problem.
//! - **Ink in its area:** every placed item's ink, as `dims::render` draws it (the dims layer of
//!   `model::render`), lies inside the area its view reserved, or the plan reports it
//!   (`annotation_outside_area`); and the side view's ink never runs past its bands outward.
//! - **Edit locality:** omitting one dimension, or adding one to an authored set, moves no other.
//!
//! Only the plan is drawn here: the dims layer is `dims::render` of the plan's dimensions alone,
//! so the part's hidden lines (minutes for connector109) add nothing to these checks.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use draftwright::dims::{self, PlannedDimension};
use draftwright::model::{self, Analysis, DimensionSource, DimensionSpec, Plan, Spec};
use draftwright::output::{Curve, Item};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    code: String,
    count: usize,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    reason: String,
}

const VERDICTS: [&str; 5] = [
    "rust-correct",
    "rust-wrong",
    "equivalent",
    "undetermined",
    "not-applicable",
];

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// The captured parts with corpus recognition, analysed: none when the corpus is missing.
fn corpus_parts() -> Vec<(String, Analysis)> {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED (corpus parts): the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return Vec::new();
    };
    let names: Vec<String> = facts::names()
        .into_iter()
        .filter(|n| corpus::recognition(n).is_some())
        .collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let dir = &dir;
                s.spawn(move || {
                    let f = facts::load(name);
                    (name.clone(), corpus::analyse(dir, &f.binding.file, name).1)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

/// The fixture parts with their recognition.
fn fixture_parts() -> Vec<(String, Analysis)> {
    ["plate", "turned", "prismatic", "two_grids", "boss_circle"]
        .iter()
        .map(|n| {
            let (part, mut a) = model::analyse(&fixture(&format!("parts/{n}.step"))).unwrap();
            let json = std::fs::read_to_string(fixture(&format!("recognition/{n}.json"))).unwrap();
            model::recognise(&part, &mut a, &json).unwrap();
            (n.to_string(), a)
        })
        .collect()
}

#[test]
fn placement_problems_are_known() {
    let parts = corpus_parts();
    if parts.is_empty() {
        return;
    }
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known_placement.json")).unwrap())
            .unwrap();
    let mut listed: BTreeMap<(String, String), &Known> = BTreeMap::new();
    for k in &known {
        assert!(
            VERDICTS.contains(&k.verdict.as_str()) && !k.reason.is_empty() && k.count > 0,
            "{k:?}"
        );
        assert!(
            listed.insert((k.part.clone(), k.code.clone()), k).is_none(),
            "{} {} is listed twice",
            k.part,
            k.code
        );
    }
    let report = std::env::var_os("PLACEMENT_REPORT").is_some();
    // Per part and code: the problems' messages.
    let mut found: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (name, analysis) in &parts {
        let plan = model::plan(analysis, &Spec::default()).unwrap();
        for p in plan
            .problems
            .iter()
            .filter(|p| p.code.starts_with("annotation_"))
        {
            if report {
                eprintln!("{name}: {} {}", p.code, p.message);
            }
            found
                .entry((name.clone(), p.code.clone()))
                .or_default()
                .push(p.message.clone());
        }
    }
    let mut wrong = Vec::new();
    for ((part, code), messages) in &found {
        match listed.get(&(part.clone(), code.clone())) {
            Some(k) if k.count == messages.len() => {}
            k => wrong.push(format!(
                "{}\n  (listed: {}; the problems: {})",
                serde_json::to_string_pretty(&Known {
                    part: part.clone(),
                    code: code.clone(),
                    count: messages.len(),
                    verdict: "VERDICT".into(),
                    reason: k.map_or_else(|| "EVIDENCE".into(), |k| k.reason.clone()),
                })
                .unwrap(),
                k.map_or_else(|| "no".into(), |k| format!("count {}", k.count)),
                messages.join(" | ")
            )),
        }
    }
    for (key, k) in &listed {
        if !found.contains_key(key) {
            wrong.push(format!(
                "{} {} (count {}) is listed but no longer found: take it off",
                k.part, k.code, k.count
            ));
        }
    }
    eprintln!(
        "{} placement problems on {} parts, in {} part and code pairs",
        found.values().map(Vec::len).sum::<usize>(),
        parts.len(),
        found.len()
    );
    assert!(
        wrong.is_empty(),
        "placement problems differ from known_placement.json; give each entry below a verdict \
         (one of {VERDICTS:?}) and the evidence, and replace its entry:\n{}",
        wrong.join("\n")
    );
}

/// Every point of *d*'s drawn ink: its lines' ends, its arrowheads' and symbols' corners, and its
/// glyphs' outline poles (a Bézier lies in its poles' hull).
fn ink(d: &PlannedDimension) -> Vec<[f64; 2]> {
    let mut items = Vec::new();
    dims::render(std::slice::from_ref(d), &mut items, &mut Vec::new());
    let mut points = Vec::new();
    let mut curve = |c: &Curve| match c {
        Curve::Line(a, b) => points.extend([*a, *b]),
        Curve::Beziers(pieces) => points.extend(pieces.iter().flatten().copied()),
        other => panic!("{}: a dimension draws no {other:?}", d.id),
    };
    for item in &items {
        match item {
            Item::Edge(c) => curve(c),
            Item::Face(wires) => wires.iter().flatten().for_each(&mut curve),
        }
    }
    points
}

/// How far *p* lies outside *area* (zero or less inside).
fn past(area: [f64; 4], p: [f64; 2]) -> f64 {
    (area[0] - p[0])
        .max(p[0] - area[2])
        .max(area[1] - p[1])
        .max(p[1] - area[3])
}

fn reported(plan: &Plan, code: &str, id: &str) -> bool {
    plan.problems
        .iter()
        .any(|p| p.code == code && p.subjects.first().is_some_and(|s| s == id))
}

/// M3: placed ink lies in its view's area (the view's box and its bands) or is reported, and a
/// report means it does not; in the side view, whose bands come from its lanes, no ink runs past
/// them outward (a horizontal item's ink stays within the area's height, a vertical one's within
/// its width; only along its lane, as a callout's label or a short span's outside arrows, may it
/// run on).
#[test]
fn placed_ink_lies_in_its_views_area_or_is_reported() {
    let mut parts = fixture_parts();
    parts.extend(corpus_parts());
    let (mut checked, mut outside) = (0, 0);
    for (name, analysis) in &parts {
        let plan = model::plan(analysis, &Spec::default()).unwrap();
        for d in plan.dimensions.iter().filter(|d| d.placed.is_some()) {
            let v = plan.views.iter().find(|v| v.id == d.view).unwrap();
            let points = ink(d);
            assert!(!points.is_empty(), "{name}: {} draws nothing", d.id);
            let worst = points
                .iter()
                .map(|p| past(v.area, *p))
                .fold(f64::NEG_INFINITY, f64::max);
            checked += 1;
            let is_reported = reported(&plan, "annotation_outside_area", &d.id);
            assert_eq!(
                worst > 1e-6,
                is_reported,
                "{name}: {} reaches {worst:.3} mm past {}'s area {:?}, and is {}reported",
                d.id,
                d.view,
                v.area,
                if is_reported { "" } else { "not " }
            );
            outside += usize::from(is_reported);
            if d.view == "v:side" {
                let p = d.placed.as_ref().unwrap();
                let k = if p.rotation == 0.0 { 1 } else { 0 };
                for q in &points {
                    assert!(
                        q[k] >= v.area[k] - 1e-6 && q[k] <= v.area[k + 2] + 1e-6,
                        "{name}: {} runs past the side view's band at {q:?} (area {:?})",
                        d.id,
                        v.area
                    );
                }
            }
        }
    }
    eprintln!("{checked} placed items; {outside} reported past their view's area");
}

fn ids(plan: &Plan) -> Vec<&str> {
    plan.dimensions.iter().map(|d| d.id.as_str()).collect()
}

/// The placed items by ID.
fn placed(plan: &Plan) -> BTreeMap<&str, &PlannedDimension> {
    plan.dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .map(|d| (d.id.as_str(), d))
        .collect()
}

/// ARCH0003-2 (edit locality, as R4 translates it: a small spec edit changes only the affected
/// plan items). Omitting any one default dimension leaves every other item, its view, sheet and
/// ink as they were; so does taking one out of an authored set. On plate.step this used to fail:
/// lanes were packed in order, so omitting the column pitch moved the x location and the bore
/// callout.
#[test]
fn omitting_one_dimension_moves_no_other() {
    let mut parts = fixture_parts();
    // Corpus parts with items in all three views, the side's among them.
    parts.extend(
        corpus_parts()
            .into_iter()
            .filter(|(n, _)| ["m10127", "hanger", "ftc06"].contains(&n.as_str())),
    );
    let mut edits = 0;
    for (name, analysis) in &parts {
        let full = model::plan(analysis, &Spec::default()).unwrap();
        let all = placed(&full);
        let authored = |listed: &[&str]| Spec {
            dimension_source: DimensionSource::Authored,
            dimensions: listed
                .iter()
                .map(|id| (id.to_string(), DimensionSpec::default()))
                .collect(),
            ..Spec::default()
        };
        let every = ids(&full);
        assert_eq!(
            placed(&model::plan(analysis, &authored(&every)).unwrap())
                .values()
                .map(|d| (&d.id, &d.placed))
                .collect::<Vec<_>>(),
            all.values().map(|d| (&d.id, &d.placed)).collect::<Vec<_>>(),
            "{name}: the authored complete set is placed as the defaults are"
        );
        for gone in &every {
            let mut spec = Spec::default();
            spec.dimensions.insert(
                gone.to_string(),
                DimensionSpec {
                    omit: true,
                    ..Default::default()
                },
            );
            let rest: Vec<&str> = every.iter().copied().filter(|i| i != gone).collect();
            for (how, spec) in [
                ("omitted", spec),
                ("left out of the authored set", authored(&rest)),
            ] {
                let plan = model::plan(analysis, &spec).unwrap();
                edits += 1;
                assert_eq!(
                    serde_json::to_value(&plan.sheet).unwrap(),
                    serde_json::to_value(&full.sheet).unwrap(),
                    "{name}: {gone} {how}"
                );
                assert_eq!(
                    plan.views
                        .iter()
                        .map(|v| (&v.id, v.at, v.area))
                        .collect::<Vec<_>>(),
                    full.views
                        .iter()
                        .map(|v| (&v.id, v.at, v.area))
                        .collect::<Vec<_>>(),
                    "{name}: {gone} {how}"
                );
                assert_eq!(ids(&plan), rest, "{name}: {gone} {how}");
                for d in &plan.dimensions {
                    let was = full.dimensions.iter().find(|e| e.id == d.id).unwrap();
                    assert_eq!(
                        d.placed, was.placed,
                        "{name}: {} moved when {gone} was {how}",
                        d.id
                    );
                }
            }
        }
    }
    eprintln!("{edits} single-dimension edits, none moving another item");
}
