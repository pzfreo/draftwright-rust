//! The drafting rules of the issue regressions (`docs/rules/issues.yaml`, class `drafting`) that
//! the port can exercise: what the sheet must state, show or avoid. Each test names the rules it
//! checks; a part (and its recognition) and a spec go in, a property of the plan, its problems,
//! lint's findings on the drawing or the output comes out. A rule the port breaks is checked by an
//! ignored test that says why (`rust-wrong`), and stays pending in the catalogue.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use draftwright::bridge::holes;
use draftwright::dims::{PlannedDimension, fmt};
use draftwright::lint::{self, Mark};
use draftwright::model::{self, Analysis, DimensionSource, DimensionSpec, Plan, PlannedView, Spec};
use draftwright::output::{Claim, Drawing};
use draftwright::text::Font;
use draftwright::title_block;
use serde_json::json;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

fn font() -> Font {
    Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
        .unwrap()
}

/// The fixture part *name* with its recognition.
fn analysed(name: &str) -> (draftwright::haecceity::Part, Analysis) {
    let (part, mut a) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
    let json = std::fs::read_to_string(fixture(&format!("recognition/{name}.json"))).unwrap();
    model::recognise(&part, &mut a, &json).unwrap();
    (part, a)
}

const FIXTURES: [&str; 5] = ["plate", "turned", "prismatic", "two_grids", "boss_circle"];

/// The fixtures, read once for every test here.
fn fixtures() -> &'static [(String, Analysis)] {
    static FIXED: OnceLock<Vec<(String, Analysis)>> = OnceLock::new();
    FIXED.get_or_init(|| {
        FIXTURES
            .iter()
            .map(|n| (n.to_string(), analysed(n).1))
            .collect()
    })
}

/// The sampled sheets' corpus parts (`tests/fixtures/recognition/sheets/`), each with the
/// features draftwright's own recogniser found for it, checked to be the file the sheet was
/// captured from; read once for every test here, and none when the corpus is missing. ctc04 is
/// left out: the bridge refuses its recognition.
fn sampled() -> &'static [(String, Analysis)] {
    static SAMPLED: OnceLock<Vec<(String, Analysis)>> = OnceLock::new();
    SAMPLED.get_or_init(|| {
        let Some(dir) = corpus::dir() else {
            eprintln!("SKIPPED (corpus part): the corpus is not found; set DRAFTWRIGHT_CORPUS");
            return Vec::new();
        };
        let mut names: Vec<String> = std::fs::read_dir(fixture("recognition/sheets"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter_map(|f| f.strip_suffix(".json.gz").map(String::from))
            .filter(|n| n != "ctc04")
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|name| {
                let f = facts::load(&name);
                let (part, mut a) = model::analyse(&dir.join(&f.binding.file)).unwrap();
                assert_eq!(a.binding.source_sha256, f.binding.sha256, "{name}");
                let mut json = String::new();
                flate2::read::GzDecoder::new(
                    std::fs::File::open(fixture(&format!("recognition/sheets/{name}.json.gz")))
                        .unwrap(),
                )
                .read_to_string(&mut json)
                .unwrap();
                model::recognise(&part, &mut a, &json).unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, a)
            })
            .collect()
    })
}

/// Every part here, with its default plan.
fn planned() -> &'static [(String, Analysis, Plan)] {
    static PLANNED: OnceLock<Vec<(String, Analysis, Plan)>> = OnceLock::new();
    PLANNED.get_or_init(|| {
        fixtures()
            .iter()
            .chain(sampled())
            .map(|(n, a)| {
                (
                    n.clone(),
                    a.clone(),
                    model::plan(a, &Spec::default()).unwrap(),
                )
            })
            .collect()
    })
}

fn spec(value: serde_json::Value) -> Spec {
    serde_json::from_value(value).unwrap()
}

fn omitting(omit: &[&str]) -> Spec {
    let mut spec = Spec::default();
    for id in omit {
        spec.dimensions.insert(
            id.to_string(),
            DimensionSpec {
                omit: true,
                ..Default::default()
            },
        );
    }
    spec
}

fn drawn(name: &str, plan: &Plan) -> (Analysis, Drawing) {
    let (part, analysis) = analysed(name);
    let drawing = model::render(&part, &analysis, plan, &font()).unwrap();
    (analysis, drawing)
}

fn codes(plan: &Plan) -> BTreeSet<&str> {
    plan.problems.iter().map(|p| p.code.as_str()).collect()
}

fn hole_callout(d: &PlannedDimension) -> bool {
    d.kind == "callout"
        && d.measurements
            .iter()
            .any(|m| m.parameter == "bore.diameter")
}

/// Where world point *p* lands on the page in view *v* (the centre of the part's box lands on
/// `at`; the page's x is `up × direction`).
fn page_of(v: &PlannedView, analysis: &Analysis, p: [f64; 3]) -> [f64; 2] {
    let [lo, hi] = analysis.envelope;
    let (t, u) = (v.direction, v.up);
    let right = [
        u[1] * t[2] - u[2] * t[1],
        u[2] * t[0] - u[0] * t[2],
        u[0] * t[1] - u[1] * t[0],
    ];
    let q = [0, 1, 2].map(|k| p[k] - 0.5 * (lo[k] + hi[k]));
    let dot = |a: [f64; 3]| a[0] * q[0] + a[1] * q[1] + a[2] * q[2];
    [v.at[0] + v.scale * dot(right), v.at[1] + v.scale * dot(u)]
}

fn inside(b: [f64; 4], outer: [f64; 4]) -> bool {
    b[0] >= outer[0] - 1e-9
        && b[1] >= outer[1] - 1e-9
        && b[2] <= outer[2] + 1e-9
        && b[3] <= outer[3] + 1e-9
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] - 1e-9 && a[2] > b[0] + 1e-9 && a[1] < b[3] - 1e-9 && a[3] > b[1] + 1e-9
}

/// The numbers a label prints as measurements: each term's number, less a count (`4×`), a
/// grid's shape (`(2×2)`) and a face's ordinal (`FACE 1 OF 3`).
fn printed(label: &str) -> Vec<String> {
    let terms: Vec<&str> = label.split(' ').collect();
    let mut out = Vec::new();
    for (i, t) in terms.iter().enumerate() {
        let before = i.checked_sub(1).map(|j| terms[j]);
        if t.ends_with('×') || t.starts_with('(') || matches!(before, Some("FACE" | "OF")) {
            continue;
        }
        let n = t
            .trim_start_matches(['⌀', 'ø', 'R', 'C'])
            .trim_end_matches('°');
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.') {
            out.push(n.to_string());
        }
    }
    out
}

/// ISSUE-1142-a: a hole callout's claims cover everything its compound text prints. Every number
/// it prints (the bore, its depth, the recesses, the countersink, a bolt circle) is the value of
/// a measurement it claims, and every measurement it claims is printed; the drawn annotation
/// carries the plan's claims and prints the plan's label as one run (the port does not wrap).
#[test]
fn a_hole_callout_claims_everything_it_prints() {
    let mut seen = 0;
    for (name, _, plan) in planned() {
        for d in plan.dimensions.iter().filter(|d| hole_callout(d)) {
            seen += 1;
            let numbers: BTreeSet<String> = printed(&d.label).into_iter().collect();
            let claimed: BTreeSet<String> = d.measurements.iter().map(|m| fmt(m.value)).collect();
            assert_eq!(numbers, claimed, "{name}: {} prints {:?}", d.id, d.label);
            assert_eq!(d.label, d.terms.join(" "), "{name}: {}", d.id);
        }
    }
    assert!(seen > 10, "{seen} hole callouts");
    for name in ["plate", "boss_circle"] {
        let plan = &planned().iter().find(|p| p.0 == name).unwrap().2;
        let (_, drawing) = drawn(name, plan);
        for d in plan.dimensions.iter().filter(|d| hole_callout(d)) {
            let a = drawing.annotations.iter().find(|a| a.name == d.id).unwrap();
            let claims: Vec<Claim> = d
                .measurements
                .iter()
                .map(|m| Claim {
                    feature: m.feature.clone(),
                    parameter: m.parameter.clone(),
                })
                .collect();
            assert_eq!(a.claims, claims, "{name}: {}", d.id);
            assert_eq!(a.runs.len(), 1, "{name}: {}", d.id);
            assert_eq!(drawing.text[a.runs[0]].text, d.label, "{name}: {}", d.id);
        }
    }
}

/// ISSUE-883-a, in part: each measured axis of a location is its own measurement. Every location
/// dimension measures one axis; no measurement (feature and parameter) is stated by two
/// dimensions; a pattern's centre is measured on the pattern, never on a member; and omitting one
/// axis's location leaves the other axis, and the other members, as they were.
#[test]
fn each_axis_of_a_location_is_its_own_measurement() {
    for (name, analysis, plan) in planned() {
        let patterns: BTreeSet<&str> = analysis
            .recognition
            .iter()
            .flat_map(|r| r.patterns.iter().map(|p| p.id.as_str()))
            .collect();
        let mut stated = BTreeMap::new();
        for d in &plan.dimensions {
            for m in &d.measurements {
                let key = (m.feature.as_str(), m.parameter.as_str());
                if let Some(other) = stated.insert(key, d.id.as_str()) {
                    assert_eq!(other, d.id, "{name}: {key:?} is stated twice");
                }
            }
            let axes: BTreeSet<&str> = d
                .measurements
                .iter()
                .filter_map(|m| m.parameter.strip_prefix("location"))
                .collect();
            assert!(axes.len() <= 1, "{name}: {} measures {axes:?}", d.id);
        }
        for d in plan
            .dimensions
            .iter()
            .filter(|d| d.id.starts_with("d:f:hole_pattern") && d.id.contains("/location"))
        {
            assert!(
                d.measurements
                    .iter()
                    .all(|m| patterns.contains(m.feature.as_str())),
                "{name}: {} locates a member",
                d.id
            );
        }
    }

    let analysis = &fixtures().iter().find(|p| p.0 == "boss_circle").unwrap().1;
    let ids = |plan: &Plan| -> BTreeSet<String> {
        plan.dimensions.iter().map(|d| d.id.clone()).collect()
    };
    let full = ids(&model::plan(analysis, &Spec::default()).unwrap());
    for gone in [
        "d:f:hole:5/location.length.x@plan",
        "d:f:hole_pattern:1/location.length.y@plan",
    ] {
        let left = ids(&model::plan(analysis, &omitting(&[gone])).unwrap());
        let mut expected = full.clone();
        assert!(expected.remove(gone), "{gone}");
        assert_eq!(left, expected, "omitting {gone}");
    }
}

/// ISSUE-1146-c: each principal projection fits inside the physical page, and no two views'
/// reserved areas overlap. On every part the views the plan places lie on the page unless the
/// plan says they do not fit; and a sheet and scale pinned so that they cannot hold the views
/// are reported, never silently overrun.
#[test]
fn every_view_lies_on_the_page_or_the_plan_says_it_does_not() {
    for (name, _, plan) in planned() {
        let [w, h] = plan.sheet.page;
        let unfit = codes(plan).contains("views_do_not_fit");
        for v in &plan.views {
            assert!(
                unfit || inside(v.area, [0.0, 0.0, w, h]),
                "{name}: {} at {:?} on {w}×{h}",
                v.id,
                v.area
            );
        }
        // The principal views, each with its annotation bands, never overlap. (The isometric's
        // area is only the space they leave, which it is fitted into; a section's area is not
        // checked here, see the rule's rust_note.)
        let ortho: Vec<&PlannedView> = plan
            .views
            .iter()
            .filter(|v| ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()))
            .collect();
        for (i, a) in ortho.iter().enumerate() {
            for b in &ortho[i + 1..] {
                assert!(
                    unfit || !overlap(a.area, b.area),
                    "{name}: {} and {} overlap",
                    a.id,
                    b.id
                );
            }
        }
    }
    let plate = &fixtures()[0].1;
    let pinned = model::plan(
        plate,
        &spec(json!({ "sheet": { "size": "A4", "scale": "10:1" } })),
    )
    .unwrap();
    assert_eq!(
        (pinned.sheet.size.as_str(), pinned.sheet.scale.as_str()),
        ("A4", "10:1")
    );
    assert!(
        codes(&pinned).contains("page_fit_uncertain"),
        "{:?}",
        codes(&pinned)
    );
}

/// ISSUE-1146-a, in part: a scale never reduces the sheet silently. A pinned scale is kept as
/// asked and its sheet chosen to hold it; when none does, or the pinned sheet cannot, the plan
/// says so; and the automatic choice states its reason. Not ported: completeness is not weighed
/// in the choice (no outcome the scale loses is counted) and there is no scale policy.
#[test]
fn a_scale_that_does_not_fit_is_reported() {
    for (name, analysis, plan) in planned() {
        assert!(
            plan.sheet.basis.contains(&plan.sheet.scale),
            "{name}: {}",
            plan.sheet.basis
        );
        let fallback = ["scale_fallback_applied", "views_do_not_fit"]
            .iter()
            .any(|c| codes(plan).contains(c));
        let standard = draftwright::layout::SCALES
            .iter()
            .any(|s| (s - plan.sheet.scale_factor).abs() < 1e-12);
        assert!(
            standard || fallback,
            "{name}: {} unreported",
            plan.sheet.scale
        );
        for (size, scale, code) in [
            (None, "10:1", Some("views_do_not_fit")),
            (Some("A4"), "10:1", Some("page_fit_uncertain")),
            (Some("A0"), "1:10000", None),
        ] {
            let mut asked = json!({ "sheet": { "scale": scale } });
            if let Some(s) = size {
                asked["sheet"]["size"] = s.into();
            }
            let pinned = model::plan(analysis, &spec(asked)).unwrap();
            assert_eq!(pinned.sheet.scale, scale, "{name}");
            let fits = !["views_do_not_fit", "page_fit_uncertain"]
                .iter()
                .any(|c| codes(&pinned).contains(c));
            match code {
                // 10:1 fits some small parts; it never fits silently when it does not.
                Some(c) => assert!(fits || codes(&pinned).contains(c), "{name}: {scale}"),
                None => assert!(fits, "{name}: {scale} on {size:?}"),
            }
        }
    }
    let ctc01 = sampled().iter().find(|p| p.0 == "ctc01");
    if let Some((_, analysis)) = ctc01 {
        let big = model::plan(analysis, &spec(json!({ "sheet": { "scale": "10:1" } }))).unwrap();
        assert_eq!(big.sheet.size, "A0");
        assert!(codes(&big).contains("views_do_not_fit"));
    }
}

/// ISSUE-1260-a, in part: what the spec asks of the views is kept, or the plan says why not. A
/// pinned scale is never relaxed (`a_scale_that_does_not_fit_is_reported`); a section is drawn
/// only with the plan view its cutting plane is marked on, and is reported when that view is
/// omitted; and an omitted view is planned out before anything is placed in it, its dimensions
/// carried by the views left or kept unplaced and reported. Not ported:
/// per-view scales and pinned positions (the spec has neither).
#[test]
fn what_the_spec_asks_of_the_views_is_kept_or_reported() {
    for (name, analysis, plan) in planned() {
        for omit in ["front", "plan", "side", "iso"] {
            if !plan.views.iter().any(|v| v.id == format!("v:{omit}")) {
                continue;
            }
            let Ok(less) = model::plan(
                analysis,
                &spec(json!({ "views": { omit: { "omit": true } } })),
            ) else {
                continue;
            };
            let gone = format!("v:{omit}");
            assert!(less.views.iter().all(|v| v.id != gone), "{name}: {omit}");
            // Each dimension it carried moves to a view left, or stays unplaced and reported.
            for d in less.dimensions.iter().filter(|d| d.view == gone) {
                assert!(d.placed.is_none(), "{name}: {} is drawn in {gone}", d.id);
                assert!(
                    less.problems
                        .iter()
                        .any(|p| p.code == "view_plan_incomplete" && p.subjects.contains(&d.id)),
                    "{name}: {} is not reported",
                    d.id
                );
            }
            let section = less.views.iter().any(|v| v.cut.is_some());
            if omit == "plan" && plan.views.iter().any(|v| v.cut.is_some()) {
                assert!(!section, "{name}");
                assert!(codes(&less).contains("section_not_shown"), "{name}");
            }
            for v in less.views.iter().filter_map(|v| v.cut.as_ref()) {
                assert!(less.views.iter().any(|w| w.id == v.marked_in), "{name}");
            }
        }
    }
    let string_post = sampled().iter().find(|p| p.0 == "string_post");
    if let Some((_, analysis)) = string_post {
        let plan = model::plan(analysis, &Spec::default()).unwrap();
        assert!(
            plan.views.iter().any(|v| v.cut.is_some()),
            "string_post has a section"
        );
    }
}

/// ISSUE-1299-a, in part: a sheet the spec names is never escalated. Pinned alone it takes the
/// largest standard scale at which the views fit it; pinned with a scale too large for it, both
/// are kept and the misfit reported. Not ported: draftwright's recovery, which re-plans on its
/// finished drawing's findings (a larger scale on the same page before a larger sheet); the
/// port makes the first choice only.
#[test]
fn a_sheet_the_spec_names_is_never_escalated() {
    for (name, analysis, _) in planned() {
        for size in ["A4", "A3"] {
            let pinned =
                model::plan(analysis, &spec(json!({ "sheet": { "size": size } }))).unwrap();
            assert_eq!(pinned.sheet.size, size, "{name}");
            let both = model::plan(
                analysis,
                &spec(json!({ "sheet": { "size": size, "scale": "10:1" } })),
            )
            .unwrap();
            assert_eq!(
                (both.sheet.size.as_str(), both.sheet.scale.as_str()),
                (size, "10:1")
            );
        }
    }
}

/// ISSUE-1378-a, in part: a hole callout's leader tip touches the rim of a hole it names. On
/// every part, each placed hole callout's arrow tip lies on the projected bore circle of one of
/// the members it claims, in its view. Not ported: lint's check of a drawn tip (a tip in blank
/// material lowering fidelity); double-D holes (not recognised).
#[test]
fn a_hole_callouts_tip_is_on_the_rim_of_a_hole_it_names() {
    let mut seen = 0;
    for (name, analysis, plan) in planned() {
        let read: BTreeMap<&str, holes::Hole> = holes::read(analysis)
            .into_iter()
            .filter_map(|(f, h)| Some((f.id.as_str(), h.ok()?)))
            .collect();
        for d in plan.dimensions.iter().filter(|d| hole_callout(d)) {
            let (Some(p), Some(v)) = (&d.placed, plan.views.iter().find(|v| v.id == d.view)) else {
                continue;
            };
            seen += 1;
            let tip = p.arrows[0][0];
            let on_rim = d
                .measurements
                .iter()
                .filter(|m| m.parameter == "bore.diameter")
                .filter_map(|m| read.get(m.feature.as_str()))
                .any(|h| {
                    let c = page_of(v, analysis, h.at);
                    let r = (tip[0] - c[0]).hypot(tip[1] - c[1]);
                    (r - 0.5 * v.scale * h.diameter).abs() < 1e-3
                });
            assert!(on_rim, "{name}: {} tip at {tip:?}", d.id);
        }
    }
    assert!(seen > 10, "{seen}");
}

/// The features the plan states *parameter* of.
fn stated<'a>(plan: &'a Plan, parameter: &str) -> BTreeSet<&'a str> {
    plan.dimensions
        .iter()
        .flat_map(|d| d.measurements.iter())
        .filter(|m| m.parameter == parameter)
        .map(|m| m.feature.as_str())
        .collect()
}

/// ISSUE-1450-a, in part: no face is dimensioned both as a boss's diameter and as a fillet's
/// radius. On every part, the faces of the bosses whose diameter the plan states and of the
/// fillets whose radius it states are disjoint. Not ported: draftwright's absorption decision
/// (the port takes each recognised feature as quiddity gives it, and has no predicate to fail
/// closed); GRM-04, the reported part, has no recognition here.
#[test]
fn no_face_is_both_a_boss_diameter_and_a_fillet_radius() {
    for (name, analysis, plan) in planned() {
        let faces = |ids: BTreeSet<&str>| -> BTreeSet<usize> {
            analysis
                .features
                .iter()
                .filter(|f| ids.contains(f.id.as_str()))
                .flat_map(|f| f.faces.iter().copied())
                .collect()
        };
        let bosses = faces(stated(plan, "boss.diameter"));
        let fillets = faces(stated(plan, "fillet.radius"));
        assert!(bosses.is_disjoint(&fillets), "{name}");
    }
}

/// ISSUE-1505-a, in part: equal diameters on different features are stated and credited
/// separately. No dimension states both a hole's bore and an outside diameter (a boss's, or the
/// part's overall), and lint's turned-step ledger lets the overall diameter stand for a band's
/// only when that band is the unique largest of its profile and alone on its axis line with its
/// diameter. Not ported: the plan does not dimension turned steps' own diameters yet
/// (src/dims/turned.rs), so a bore equal to a shoulder has no shoulder dimension to keep apart.
#[test]
fn equal_diameters_on_different_features_are_stated_apart() {
    for (name, _, plan) in planned() {
        for d in &plan.dimensions {
            let ps: BTreeSet<&str> = d
                .measurements
                .iter()
                .map(|m| m.parameter.as_str())
                .collect();
            let outside = ps.contains("boss.diameter") || ps.contains("od.diameter");
            assert!(
                !(outside && ps.contains("bore.diameter")),
                "{name}: {}",
                d.id
            );
        }
    }
    let plan = &planned().iter().find(|p| p.0 == "turned").unwrap().2;
    let (analysis, drawing) = drawn("turned", plan);
    let marks = lint::marks(&drawing);
    let ledger = |a: &Analysis| -> Vec<String> {
        lint::turned::outcomes(a, &marks)
            .into_iter()
            .filter(|o| o.parameter == "step.diameter")
            .map(|o| format!("{} {:?}", o.feature, o.state))
            .collect()
    };
    assert_eq!(
        ledger(&analysis),
        ["f:turned_step:1 Placed", "f:turned_step:2 Missing"]
    );
    // Two bands of the largest diameter: the overall diameter stands for neither.
    let mut twin = analysis.clone();
    let d = twin
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .map(|f| f.parameters["diameter"].as_f64().unwrap())
        .fold(0.0, f64::max);
    let d = serde_json::Value::from(d);
    for f in twin.features.iter_mut().filter(|f| f.kind == "turned_step") {
        f.parameters.insert("diameter".into(), d.clone());
    }
    assert_eq!(
        ledger(&twin),
        ["f:turned_step:1 Missing", "f:turned_step:2 Missing"]
    );
}

/// ISSUE-1518-a, in part: a rear view is never added automatically, and one asked for is
/// refused with the views there are, before anything is planned. Not ported: the explicit rear
/// view itself (the port draws front, plan, side, the isometric and section A–A).
#[test]
fn a_rear_view_is_never_added_and_one_asked_for_is_refused() {
    for (name, analysis, plan) in planned() {
        let names: BTreeSet<&str> = plan.views.iter().map(|v| v.id.as_str()).collect();
        assert!(
            names.is_subset(&BTreeSet::from([
                "v:front",
                "v:plan",
                "v:side",
                "v:iso",
                "v:section_aa"
            ])),
            "{name}: {names:?}"
        );
        let e = model::plan(analysis, &spec(json!({ "views": { "rear": {} } }))).unwrap_err();
        assert!(e.message.contains("rear"), "{e}");
        assert!(e.choices.iter().any(|c| c == "front"), "{e}");
    }
}

/// ISSUE-1530-a, in part: a section's ink describes its cut. The cutting plane lies inside the
/// part's extent along its normal and is marked on a view the plan draws; the drawing hatches
/// the cut, captions it `SECTION A–A` and letters both ends of the plane on the marking view.
/// Not ported: an authored cut (the spec cannot place one, so none outside the part can be
/// asked for), and the tangent, gap and coplanar-boundary cases have no fixture here.
#[test]
fn a_sections_ink_describes_its_cut() {
    let mut seen = 0;
    for (name, analysis, plan) in planned() {
        let Some(v) = plan.views.iter().find(|v| v.cut.is_some()) else {
            continue;
        };
        seen += 1;
        let cut = v.cut.as_ref().unwrap();
        let [lo, hi] = analysis.envelope;
        let along = |p: [f64; 3]| (0..3).map(|k| p[k] * cut.normal[k]).sum::<f64>();
        let (a, b) = (along(lo).min(along(hi)), along(lo).max(along(hi)));
        assert!(a < along(cut.origin) && along(cut.origin) < b, "{name}");
        assert!(plan.views.iter().any(|w| w.id == cut.marked_in), "{name}");
        if !FIXTURES.contains(&name.as_str()) {
            let dir = corpus::dir().unwrap();
            let (part, _) = model::analyse(&dir.join(facts::load(name).binding.file)).unwrap();
            let drawing = model::render(&part, analysis, plan, &font()).unwrap();
            let texts: Vec<&str> = drawing.text.iter().map(|t| t.text.as_str()).collect();
            assert!(texts.contains(&"SECTION A\u{2013}A"), "{name}: {texts:?}");
            assert_eq!(texts.iter().filter(|t| **t == "A").count(), 2, "{name}");
        }
    }
    assert!(seen > 0);
}

/// The plan of fixture *name* with sheet title fields *title*.
fn titled(name: &str, title: serde_json::Value) -> (Plan, Drawing) {
    let analysis = &fixtures().iter().find(|p| p.0 == name).unwrap().1;
    let plan = model::plan(analysis, &spec(json!({ "sheet": { "title": title } }))).unwrap();
    let (_, drawing) = drawn(name, &plan);
    (plan, drawing)
}

/// ISSUE-1536-a: a title-block value wider than its cell is reported, and kept whole. An
/// 86-character title on plate is wider than its cell: the plan carries one
/// `title_field_overflow` warning for the title's cell, with the full value, and the drawing
/// carries the value whole. (Every field and multi-line heights: `tests/title_block.rs`.)
#[test]
fn a_title_block_value_that_overflows_is_reported() {
    let long =
        "FRAME FOR THE ENGINEERING REVIEW GROUP, PRELIMINARY MACHINING ISSUE, SHEET ONE OF ONE";
    let (plan, drawing) = titled("plate", json!({ "title": long }));
    let cell = plan.sheet.title_cells.iter().find(|c| c.field == "title");
    let [x0, _, x1, _] = cell.unwrap().cell;
    let [width, _] = title_block::measure(long);
    assert!(
        width > x1 - x0,
        "the fixture's title must overflow its cell"
    );
    let overflow: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "title_field_overflow")
        .collect();
    assert_eq!(overflow.len(), 1, "{:?}", codes(&plan));
    assert_eq!(overflow[0].subjects, ["title_block/title"]);
    assert_eq!(overflow[0].severity, "warning");
    assert!(overflow[0].message.contains(long));
    assert_eq!(drawing.text.iter().filter(|t| t.text == long).count(), 1);
}

/// ISSUE-1585-a: a supplied date reaches the title block as its own field: recorded in the
/// plan's date cell (not the revision's), drawn as a searchable run at that cell's centre with
/// its ink inside the cell, and absent when not supplied. (Each of draftwright's clauses:
/// `tests/title_block.rs`; the PDF text layer: `tests/output.rs`.)
#[test]
fn a_supplied_date_reaches_the_title_block() {
    let (plan, drawing) = titled("plate", json!({ "title": "BRACKET", "date": "2026-09-11" }));
    let cell = |plan: &Plan, field: &str| {
        plan.sheet
            .title_cells
            .iter()
            .find(|c| c.field == field)
            .unwrap()
            .clone()
    };
    let date = cell(&plan, "date");
    assert_eq!(date.value.as_deref(), Some("2026-09-11"));
    assert_ne!(date.cell, cell(&plan, "revision").cell);
    let [x0, y0, x1, y1] = date.cell;
    let runs: Vec<_> = drawing
        .text
        .iter()
        .filter(|t| t.text == "2026-09-11")
        .collect();
    assert_eq!(runs.len(), 1, "the date is drawn once");
    assert_eq!(runs[0].at, [0.5 * (x0 + x1), 0.5 * (y0 + y1)]);
    let [w, h] = title_block::measure("2026-09-11");
    assert!(w < x1 - x0 && h < y1 - y0);
    assert!(!codes(&plan).contains("title_field_overflow"));
    let (undated, drawing) = titled("plate", json!({ "title": "BRACKET" }));
    assert_eq!(cell(&undated, "date").value, None);
    assert!(!drawing.text.iter().any(|t| t.text == "2026-09-11"));
}

/// ISSUE-1593-a, in part: nothing is drawn into the title block unreported. On every part, a
/// placed dimension whose label, lines or arrowheads reach into the title block is the subject
/// of an `annotation_over_title_block` problem. Not ported: the keep-out itself (the port places
/// by lanes and reports what collides, as a warning; draftwright relocates the dimension to the
/// opposite strip, or withholds it as an error and replans onto a sheet that fits).
#[test]
fn a_dimension_in_the_title_block_is_reported() {
    for (name, _, plan) in planned() {
        let tb = plan.sheet.title_block;
        let within = |p: [f64; 2]| tb[0] < p[0] && p[0] < tb[2] && tb[1] < p[1] && p[1] < tb[3];
        for d in &plan.dimensions {
            let Some(p) = &d.placed else { continue };
            let ink = overlap(p.label_box, tb)
                || p.lines
                    .iter()
                    .chain(&p.arrows)
                    .flatten()
                    .any(|q| within(*q));
            let reported = plan
                .problems
                .iter()
                .any(|q| q.code == "annotation_over_title_block" && q.subjects.contains(&d.id));
            assert!(!ink || reported, "{name}: {} is in the title block", d.id);
        }
    }
}

/// ISSUE-1177-a, in part: an angle is never drawn as a length. On every part no linear
/// dimension states an angle, and every angle the plan states (a countersink's, a chamfer's)
/// is printed in degrees in a callout. Not ported: angular dimensions (the port plans none, so
/// none can fall back to a linear one) and their refusal when they cannot be drawn.
#[test]
fn an_angle_is_never_drawn_as_a_length() {
    for (name, _, plan) in planned() {
        for d in &plan.dimensions {
            let angles: Vec<f64> = d
                .measurements
                .iter()
                .filter(|m| m.parameter.ends_with("angle"))
                .map(|m| m.value)
                .collect();
            if d.kind == "linear" {
                assert!(
                    angles.is_empty() && !d.label.contains('°'),
                    "{name}: {}",
                    d.id
                );
            }
            for a in angles {
                assert!(
                    d.label.contains(&format!("{}°", fmt(a))),
                    "{name}: {}",
                    d.id
                );
            }
        }
    }
}

/// ISSUE-1433-a, for fillets: fillets share a callout only when their radii are equal, each
/// occurrence is credited by it, and without it every occurrence is reported. Not ported: blends
/// (quiddity's `Blend`, the rule's subject) are not dimensioned yet: each is reported
/// `dimensions_not_ported` (src/dims/edges.rs plans fillets and chamfers only).
#[test]
fn fillets_share_a_callout_only_when_their_radii_are_equal() {
    for (name, analysis, plan) in planned() {
        let radius = |id: &str| {
            analysis
                .features
                .iter()
                .find(|f| f.id == id)
                .unwrap()
                .parameters["radius"]
                .as_f64()
                .unwrap()
        };
        for d in plan
            .dimensions
            .iter()
            .filter(|d| d.id.contains("/fillet.radius"))
        {
            let printed = d.label.rsplit('R').next().unwrap();
            for m in &d.measurements {
                assert_eq!(fmt(radius(&m.feature)), printed, "{name}: {}", d.id);
            }
            let count = d.measurements.len();
            assert!(
                (count == 1 && !d.label.contains('×'))
                    || d.label.starts_with(&format!("{count}× ")),
                "{name}: {}",
                d.id
            );
        }
    }
    let plan = &planned().iter().find(|p| p.0 == "prismatic").unwrap().2;
    let callout = plan
        .dimensions
        .iter()
        .find(|d| d.id.contains("/fillet.radius"))
        .unwrap();
    assert_eq!(callout.label, "4× R2");
    let (analysis, drawing) = drawn("prismatic", plan);
    let fillet_findings = |marks: &[Mark]| -> BTreeSet<String> {
        lint::check(&analysis, marks)
            .into_iter()
            .filter(|i| i.code.starts_with("fillet_"))
            .flat_map(|i| i.subjects)
            .collect()
    };
    let marks = lint::marks(&drawing);
    assert!(fillet_findings(&marks).is_empty());
    let without: Vec<Mark> = marks.into_iter().filter(|m| m.name != callout.id).collect();
    let every: BTreeSet<String> = callout
        .measurements
        .iter()
        .map(|m| m.feature.clone())
        .collect();
    assert_eq!(fillet_findings(&without), every);
}

/// ISSUE-1374-c, in part: each turned outside-diameter band carries two requirements, its
/// diameter and its length, and a band a groove owns is the groove's, not a second step. On every
/// part, lint's turned-step ledger has exactly those two outcomes for each band it reads, and
/// none for a groove's band. Not ported: the plan does not dimension turned steps yet
/// (src/dims/turned.rs), so the grouped length label (exact count, no skipped equal neighbour)
/// has nothing to check.
#[test]
fn each_turned_band_carries_its_diameter_and_length() {
    let mut grooved = 0;
    for (name, analysis, _) in planned() {
        let Ok(bands) = lint::turned::bands(analysis) else {
            continue;
        };
        let steps = analysis
            .features
            .iter()
            .filter(|f| f.kind == "turned_step")
            .count();
        let ledger = lint::turned::outcomes(analysis, &[]);
        let mut by_band: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for o in &ledger {
            by_band.entry(&o.feature).or_default().push(&o.parameter);
        }
        assert_eq!(by_band.len(), bands.len(), "{name}");
        for b in &bands {
            assert_eq!(
                by_band[b.id.as_str()],
                ["step.length", "step.diameter"],
                "{name}"
            );
        }
        if bands.len() < steps {
            grooved += 1;
        }
    }
    if !sampled().is_empty() {
        assert!(grooved > 0, "no sampled part has a band a groove owns");
    }
}

/// ISSUE-1560-a, in part: the part's extents never leave the drawing silently. On every part,
/// each non-zero axis of the box has an outcome in lint's envelope ledger (placed on the default
/// drawing); an extent the spec omits is listed in the plan's `omitted` with why, and stays in
/// the ledger, missing; an empty authored set leaves all three in the ledger. Not ported: a local
/// turned profile or overlapping steps standing for an axis (no sampled part has one), and
/// Python's other carriers of an end-to-end span (`lint::envelope`'s module note).
#[test]
fn the_parts_extents_never_leave_the_drawing_silently() {
    let (part, analysis) = analysed("plate");
    let ledger = |plan: &Plan| -> Vec<String> {
        let drawing = model::render(&part, &analysis, plan, &font()).unwrap();
        lint::envelope::outcomes(&analysis, &lint::marks(&drawing))
            .into_iter()
            .map(|o| format!("{} {:?}", o.parameter, o.state))
            .collect()
    };
    let full = model::plan(&analysis, &Spec::default()).unwrap();
    assert_eq!(
        ledger(&full),
        [
            "width.length Placed",
            "depth.length Placed",
            "height.length Placed"
        ]
    );
    let width = "d:envelope/width.length@front";
    let less = model::plan(&analysis, &omitting(&[width])).unwrap();
    assert!(
        less.omitted
            .iter()
            .any(|o| o.id == width && !o.basis.is_empty())
    );
    assert_eq!(
        ledger(&less),
        [
            "width.length Missing",
            "depth.length Placed",
            "height.length Placed"
        ]
    );
    let none = Spec {
        dimension_source: DimensionSource::Authored,
        ..Spec::default()
    };
    assert_eq!(
        ledger(&model::plan(&analysis, &none).unwrap()),
        [
            "width.length Missing",
            "depth.length Missing",
            "height.length Missing"
        ]
    );
    for (name, analysis, plan) in planned() {
        let [lo, hi] = analysis.envelope;
        let axes = (0..3).filter(|&k| hi[k] - lo[k] > 1e-9).count();
        let stated: Vec<Mark> = plan
            .dimensions
            .iter()
            .filter(|d| d.id.starts_with("d:envelope/"))
            .map(|d| Mark {
                name: d.id.clone(),
                view: Some(d.view.clone()),
                text: d.label.clone(),
                claims: d
                    .measurements
                    .iter()
                    .map(|m| Claim {
                        feature: m.feature.clone(),
                        parameter: m.parameter.clone(),
                    })
                    .collect(),
            })
            .collect();
        let outcomes = lint::envelope::outcomes(analysis, &stated);
        assert_eq!(outcomes.len(), axes, "{name}");
        for o in &outcomes {
            assert_eq!(
                format!("{:?}", o.state),
                "Placed",
                "{name}: {}",
                o.parameter
            );
        }
    }
}

/// ISSUE-1357-b, in part: eccentric parallel cylinders are not one turned profile. The turned
/// fixture's two bands share one axis line, and the plan states the part's overall diameter and
/// length; with its smaller band's axis moved 2 mm off that line, the plan no longer reads the
/// part as turned whole (it states the three extents instead) and reports each band on its own,
/// none merged or silently chosen. Not ported: turned steps' own dimensions (each is reported
/// `dimensions_not_ported`) and several bodies in one compound (no fixture has one).
#[test]
fn eccentric_cylinders_are_not_one_turned_profile() {
    let analysis = &fixtures().iter().find(|p| p.0 == "turned").unwrap().1;
    let envelope = |plan: &Plan| -> Vec<String> {
        plan.dimensions
            .iter()
            .filter(|d| d.id.starts_with("d:envelope/"))
            .map(|d| d.measurements[0].parameter.clone())
            .collect()
    };
    let reported = |plan: &Plan| -> BTreeSet<String> {
        plan.problems
            .iter()
            .filter(|p| p.code == "dimensions_not_ported")
            .flat_map(|p| p.subjects.clone())
            .collect()
    };
    let steps: BTreeSet<String> = ["f:turned_step:1", "f:turned_step:2"]
        .map(String::from)
        .into();
    let plan = model::plan(analysis, &Spec::default()).unwrap();
    assert_eq!(envelope(&plan), ["od.diameter", "height.length"]);
    assert_eq!(reported(&plan), steps);

    let mut eccentric = analysis.clone();
    let step = eccentric
        .features
        .iter_mut()
        .find(|f| f.id == "f:turned_step:2")
        .unwrap();
    step.parameters["profile"]["axis_origin"] = json!([0.0, 2.0, 0.0]);
    let plan = model::plan(&eccentric, &Spec::default()).unwrap();
    assert_eq!(
        envelope(&plan),
        ["width.length", "depth.length", "height.length"]
    );
    assert_eq!(reported(&plan), steps);
}
