//! Stream K4 (#43 item 2): the Python leader-correctness rules, each checked on a sheet built
//! through the public types (an Analysis, and a drawing with one view and its recorded
//! transform), sound and then broken one way, through `lint::lint`:
//!
//! - ISSUE-1378-a, LINT-code-diameter_leader_target_*: a diameter leader's tip touches the rim
//!   of a hole it names (a double-D's trimmed profile, flats included), never blank material or
//!   another hole's rim; a callout whose holes cannot be read says so.
//! - ISSUE-1479-a, LINT-code-radius_leader_target_*: a radius leader's tip is on the named
//!   blend's trimmed profile arc, not a centre bore or the untrimmed circle's extension, and the
//!   check follows the geometry wherever it is; a view that cannot show the arc says so.
//! - ADR2-8, ISSUE-798-b/d, LINT-code-leader_crosses_silhouette: a shaft cutting back into the
//!   material after leaving it at its tip is reported with its measured depth; leaving its own
//!   body is not, and a second body is. The router prices the same predicate
//!   (`draftwright::material::reentry`), and the plan's report of a leader cutting back across
//!   its view and lint's finding name the same callouts with the same depth: on a pin's leader
//!   pinned to cut across the plate-and-pin assembly, and on the port's own drawings.
//!
//! The automatic drawings of the fixture parts (and string_post's blend) are linted too: every
//! hole and blend callout the planner places passes, so ISSUE-1378-a and ISSUE-1479-a are met
//! by the planner, not only checked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use draftwright::items::{Op, key};
use draftwright::lint::{self, LintIssue, Severity};
use draftwright::model::{self, Analysis, Spec};
use draftwright::output::{Annotation, Claim, Drawing, Drawn, DrawnFrame, DrawnView, Kind, Sheet};
use draftwright::{Session, api};
use serde_json::{Value, json};

type P = [f64; 2];

/// The view's frame: looking along +z with y up, so the part's (x, y) is the page's, at scale 2
/// about [100, 100] (the envelope's centre is the origin).
const AT: P = [100.0, 100.0];
const SCALE: f64 = 2.0;

fn page(x: f64, y: f64) -> P {
    [AT[0] + SCALE * x, AT[1] + SCALE * y]
}

fn feature(id: &str, kind: &str, parameters: Value) -> Value {
    json!({
        "id": id, "kind": kind, "faces": [0], "parameters": parameters,
        "fingerprint": { "version": "test", "family": "test", "subtype": "Test", "sizes": {},
                         "axis": [0.0, 0.0, 1.0], "position": [0.0, 0.0, 0.0],
                         "neighbourhood": [] },
    })
}

/// A 20 mm cube about the origin with *features*, *blend_sites* and *solids* (each a
/// principal profile's box; none for the envelope alone).
fn analysis(features: Vec<Value>, blend_sites: Value, solids: &[[[f64; 3]; 2]]) -> Analysis {
    let profiles: Vec<Value> = solids
        .iter()
        .map(|b| json!({ "bounds": b, "faces": [] }))
        .collect();
    serde_json::from_value(json!({
        "schema": model::ANALYSIS_SCHEMA,
        "binding": { "source_sha256": "test", "part": "test", "face_count": 1, "loader": "test" },
        "envelope": [[-10.0, -10.0, -10.0], [10.0, 10.0, 10.0]],
        "iso_bounds": [0.0, 0.0, 0.0, 0.0],
        "features": features,
        "blend_sites": blend_sites,
        "principal_profiles": profiles,
    }))
    .unwrap()
}

fn hole(id: &str, x: f64, y: f64, diameter: f64) -> Value {
    feature(
        id,
        "hole",
        json!({ "axis": [0.0, 0.0, 1.0], "location": [x, y, 10.0], "diameter": diameter,
                "depth": 6.0, "bottom": "flat" }),
    )
}

/// A callout *name* in the view, claiming *claims*, its leader from *tip* to *elbow* and a
/// shoulder 10 mm on.
fn callout(name: &str, claims: &[(&str, &str)], tip: P, elbow: P) -> (Annotation, Drawn) {
    let end = [elbow[0] + 10.0, elbow[1]];
    let label = [
        elbow[0] + 1.0,
        elbow[1] + 0.5,
        elbow[0] + 9.0,
        elbow[1] + 3.0,
    ];
    (
        Annotation {
            name: name.into(),
            view: Some("v:plan".into()),
            claims: claims
                .iter()
                .map(|(f, p)| Claim {
                    feature: f.to_string(),
                    parameter: p.to_string(),
                })
                .collect(),
            ..Annotation::default()
        },
        Drawn {
            heads: Vec::new(),
            name: name.into(),
            kind: Kind::Dimension,
            ink: [
                tip[0].min(elbow[0]),
                tip[1].min(elbow[1]),
                end[0].max(tip[0]),
                label[3].max(tip[1]),
            ],
            label: Some(label),
            lines: vec![[tip, elbow], [elbow, end]],
            elbow: Some(elbow),
            interior: false,
        },
    )
}

/// A sheet with the plan view drawn looking along *direction* (up +y, or +z when looking
/// along y), holding *items*.
fn drawing(direction: [f64; 3], items: Vec<(Annotation, Drawn)>) -> Drawing {
    let (annotations, drawn) = items.into_iter().unzip();
    let up = if direction[1] != 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    Drawing {
        page: [420.0, 297.0],
        layers: Vec::new(),
        link: None,
        text: Vec::new(),
        annotations,
        sheet: Sheet {
            references: Vec::new(),
            drawable: Some([10.0, 10.0, 410.0, 287.0]),
            views: vec![DrawnView {
                name: "v:plan".into(),
                bbox: [80.0, 80.0, 120.0, 120.0],
                edges: Vec::new(),
            }],
            items: drawn,
            fields: Vec::new(),
            centre_marks: BTreeMap::new(),
            frames: BTreeMap::from([(
                "plan".to_string(),
                DrawnFrame {
                    direction,
                    up,
                    at: AT,
                    scale: SCALE,
                },
            )]),
        },
        problems: Vec::new(),
        spec: None,
    }
}

const ALONG_Z: [f64; 3] = [0.0, 0.0, 1.0];

/// The findings on *drawing* whose code starts with *prefix*, as (code, annotation).
fn found(a: &Analysis, drawing: &Drawing, prefix: &str) -> Vec<(String, String)> {
    lint::lint(a, drawing)
        .into_iter()
        .filter(|i| i.code.starts_with(prefix))
        .map(|i| (i.code, i.annotation.unwrap_or_default()))
        .collect()
}

fn one(code: &str, name: &str) -> Vec<(String, String)> {
    vec![(code.to_string(), name.to_string())]
}

/// ISSUE-1378-a, LINT-code-diameter_leader_target_mismatch and _unverifiable: a hole callout's
/// tip on its own hole's rim (opening, or far end, counterbore none here) passes; in blank
/// material, or on the rim of a hole the callout does not name, it is a warning naming the
/// callout and its measurement; a hole whose record cannot be read is unverifiable. A
/// double-D bore's boundary is its trimmed profile: a flat and an arc pass, the major circle
/// where the flats cut it away does not.
#[test]
fn a_diameter_tip_touches_the_rim_of_the_hole_it_names() {
    let a = analysis(
        vec![
            hole("f:hole:1", 2.0, 0.0, 4.0),
            hole("f:hole:2", -6.0, 5.0, 2.0),
            feature(
                "f:double_d_bore:1",
                "double_d_bore",
                json!({ "axis": [0.0, 0.0, 1.0], "location": [-5.0, -5.0, 10.0],
                        "major_diameter": 6.0, "across_flats": 4.0,
                        "flat_direction": [1.0, 0.0, 0.0], "through": true }),
            ),
            feature("f:hole:3", "hole", json!({ "location": [0.0, 0.0, 0.0] })),
        ],
        json!({}),
        &[],
    );
    let bore = [("f:hole:1", "bore.diameter")];
    let check = |tip: P, claims: &[(&str, &str)]| {
        found(
            &a,
            &drawing(ALONG_Z, vec![callout("d:c", claims, tip, [130.0, 110.0])]),
            "diameter_leader_target",
        )
    };
    // On the rim, either side.
    assert!(check(page(4.0, 0.0), &bore).is_empty());
    assert!(check(page(2.0, -2.0), &bore).is_empty());
    // In blank material, 0.5 mm off the rim.
    assert_eq!(
        check(page(4.25, 0.0), &bore),
        one("diameter_leader_target_mismatch", "d:c")
    );
    // On another hole's rim, one the callout does not name.
    assert_eq!(
        check(page(-5.0, 5.0), &bore),
        one("diameter_leader_target_mismatch", "d:c")
    );
    // ... and fine when it names that hole too (a counted callout tips one member).
    assert!(check(page(-5.0, 5.0), &[bore[0], ("f:hole:2", "bore.diameter")]).is_empty());
    // A double-D: a flat (x = −3, |y + 5| ≤ √5), an arc (the top of the major circle).
    let dd = [("f:double_d_bore:1", "bore.diameter")];
    assert!(check(page(-3.0, -4.0), &dd).is_empty());
    assert!(check(page(-5.0, -2.0), &dd).is_empty());
    // The major circle where the flat cut it away is blank material.
    assert_eq!(
        check(page(-2.0, -5.0), &dd),
        one("diameter_leader_target_mismatch", "d:c")
    );
    // A hole without an axis or diameter cannot be checked.
    assert_eq!(
        check(page(0.0, 0.0), &[("f:hole:3", "bore.diameter")]),
        one("diameter_leader_target_unverifiable", "d:c")
    );
    // The finding carries the callout's measurement and where the tip is.
    let issues = lint::lint(
        &a,
        &drawing(
            ALONG_Z,
            vec![callout("d:c", &bore, page(4.25, 0.0), [130.0, 110.0])],
        ),
    );
    let issue: &LintIssue = issues
        .iter()
        .find(|i| i.code == "diameter_leader_target_mismatch")
        .unwrap();
    assert_eq!(issue.measurements, ["f:hole:1/bore.diameter"]);
    assert_eq!(issue.at, Some(page(4.25, 0.0)));
    assert_eq!(issue.severity, Severity::Warning);
}

/// A blend of radius 3 about the z axis through (*x*, *y*), its trimmed arc the quarter from
/// +x to +y (proven points at a quarter, a half and three quarters of it).
fn quarter_blend(x: f64, y: f64) -> (Value, Value) {
    let sites: Vec<[f64; 3]> = [22.5f64, 45.0, 67.5]
        .iter()
        .map(|d| {
            let t = d.to_radians();
            [x + 3.0 * t.cos(), y + 3.0 * t.sin(), 10.0]
        })
        .collect();
    (
        feature(
            "f:blend:1",
            "blend",
            json!({ "radius": 3.0, "side": "concave",
                    "path": { "at": [x, y, 0.0], "direction": [0.0, 0.0, 1.0] } }),
        ),
        json!({ "f:blend:1": sites }),
    )
}

/// ISSUE-1479-a, LINT-code-radius_leader_target_mismatch and _unverifiable: a radius tip on
/// the blend's trimmed arc passes, at its middle or near an end; on a centre bore (a smaller
/// circle about the same axis), off the circle, or on the untrimmed circle's extension it is a
/// warning; moved with the geometry it still passes; in a view that does not show the arc
/// end-on it cannot be checked.
#[test]
fn a_radius_tip_lands_on_the_trimmed_profile_arc() {
    let check = |at: P, tip_deg: f64, r: f64, direction: [f64; 3]| {
        let (f, sites) = quarter_blend(at[0], at[1]);
        let a = analysis(vec![f], sites, &[]);
        let t = tip_deg.to_radians();
        let tip = page(at[0] + r * t.cos(), at[1] + r * t.sin());
        found(
            &a,
            &drawing(
                direction,
                vec![callout(
                    "d:r",
                    &[("f:blend:1", "blend.radius")],
                    tip,
                    [130.0, 110.0],
                )],
            ),
            "radius_leader_target",
        )
    };
    let mismatch = one("radius_leader_target_mismatch", "d:r");
    for at in [[0.0, 0.0], [-4.5, 3.25]] {
        assert!(check(at, 45.0, 3.0, ALONG_Z).is_empty(), "{at:?}");
        assert!(check(at, 2.0, 3.0, ALONG_Z).is_empty(), "{at:?}");
        assert!(check(at, 88.0, 3.0, ALONG_Z).is_empty(), "{at:?}");
        // A centre bore of radius 1 on the same axis.
        assert_eq!(check(at, 45.0, 1.0, ALONG_Z), mismatch, "{at:?}");
        // Off the circle by 0.1 mm.
        assert_eq!(check(at, 45.0, 3.1, ALONG_Z), mismatch, "{at:?}");
        // On the circle, past the trimmed arc.
        assert_eq!(check(at, 180.0, 3.0, ALONG_Z), mismatch, "{at:?}");
        assert_eq!(check(at, -20.0, 3.0, ALONG_Z), mismatch, "{at:?}");
    }
    // A view along y cannot show the arc as an arc.
    assert_eq!(
        check([0.0, 0.0], 45.0, 3.0, [0.0, -1.0, 0.0]),
        one("radius_leader_target_unverifiable", "d:r")
    );
}

/// ADR2-8, ISSUE-798-b/d, LINT-code-leader_crosses_silhouette: a shaft leaving its own body
/// (whatever it passes over inside it, a bore included) is not reported; a shaft that goes on
/// into a second body is, as info, with the depth it cuts; a deeper cut reads deeper. The
/// predicate is `material::reentry`, which the router prices (its own tests are in
/// `src/material.rs`).
#[test]
fn a_shaft_cutting_back_into_the_material_is_reported_with_its_depth() {
    // Two bodies side by side: x −10..0 and 5..10 (page x 80..100 and 110..120).
    let a = analysis(
        vec![hole("f:hole:1", -5.0, 0.0, 4.0)],
        json!({}),
        &[
            [[-10.0, -10.0, -10.0], [0.0, 10.0, 10.0]],
            [[5.0, -10.0, -10.0], [10.0, 10.0, 10.0]],
        ],
    );
    let bore = [("f:hole:1", "bore.diameter")];
    let lint_of = |tip: P, elbow: P| -> Vec<LintIssue> {
        lint::lint(
            &a,
            &drawing(ALONG_Z, vec![callout("d:c", &bore, tip, elbow)]),
        )
        .into_iter()
        .filter(|i| i.code == "leader_crosses_silhouette")
        .collect()
    };
    // Out of its own body to the left, across the bore: one traversal.
    assert!(lint_of(page(-7.0, 0.0), [60.0, 100.0]).is_empty());
    // Out to the right across the gap and through the second body: a 10 mm cut.
    let cut = lint_of(page(-3.0, 0.0), [130.0, 100.0]);
    assert_eq!(cut.len(), 1, "{cut:#?}");
    assert_eq!(cut[0].severity, Severity::Info);
    assert_eq!(cut[0].annotation.as_deref(), Some("d:c"));
    assert!(
        cut[0]
            .message
            .contains("cuts 10.0 mm back through view 'plan'"),
        "{}",
        cut[0].message
    );
    // A shallower cut (stopping inside the second body) reads less.
    let shallow = lint_of(page(-3.0, 0.0), [113.0, 100.0]);
    assert!(
        shallow[0].message.contains("cuts 3.0 mm"),
        "{}",
        shallow[0].message
    );
}

// ----------------------------------------------------------------------------------------
// The port's own drawings
// ----------------------------------------------------------------------------------------

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

fn font() -> draftwright::text::Font {
    draftwright::text::Font::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"),
    )
    .unwrap()
}

/// Each fixture part's automatic plan and drawing, and string_post's from the corpus (a
/// blend radius callout) when the corpus is present.
fn drawings() -> Vec<(String, Analysis, model::Plan, Drawing)> {
    let mut out = Vec::new();
    for name in ["plate", "turned", "prismatic", "two_grids", "boss_circle"] {
        let (part, mut a) = model::analyse(&fixture(&format!("parts/{name}.step"))).unwrap();
        let json = std::fs::read_to_string(fixture(&format!("recognition/{name}.json"))).unwrap();
        model::recognise(&part, &mut a, &json).unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let drawing = model::render(&part, &a, &plan, &font()).unwrap();
        out.push((name.to_string(), a, plan, drawing));
    }
    if let Ok(corpus) = std::env::var("DRAFTWRIGHT_CORPUS") {
        let path = Path::new(&corpus).join("gramel/string_post.step");
        if path.is_file() {
            let (part, a) = model::analyse_native(&path).unwrap();
            let plan = model::plan(&a, &Spec::default()).unwrap();
            let drawing = model::render(&part, &a, &plan, &font()).unwrap();
            out.push(("string_post".into(), a, plan, drawing));
        } else {
            assert!(
                std::env::var("DRAFTWRIGHT_CORPUS_REQUIRED").is_err(),
                "string_post is missing from the corpus"
            );
        }
    }
    out
}

fn claims(d: &Drawing, parameter: &str) -> usize {
    d.annotations
        .iter()
        .filter(|a| a.claims.iter().any(|c| c.parameter == parameter))
        .filter(|a| {
            d.sheet
                .items
                .iter()
                .any(|i| i.name == a.name && i.elbow.is_some())
        })
        .count()
}

/// ISSUE-1378-a and ISSUE-1479-a met by the planner, and the router and lint agreeing
/// (ISSUE-798-d): on the port's automatic drawings no hole or blend callout's tip misses its
/// target, and the callouts the plan reports cutting back across their view are exactly those
/// lint reports crossing the silhouette (none on these drawings: the router finds every leader
/// a route that does not cut back; `the_plan_and_lint_agree_on_a_leader_cutting_back_into_the_
/// material` is the case where both report one).
#[test]
fn the_ports_own_callouts_touch_their_targets() {
    let (mut holes, mut blends) = (0, 0);
    for (name, a, plan, drawing) in drawings() {
        let issues = lint::lint(&a, &drawing);
        let target: Vec<&LintIssue> = issues
            .iter()
            .filter(|i| i.code.contains("_leader_target_"))
            .collect();
        assert!(target.is_empty(), "{name}: {target:#?}");
        holes += claims(&drawing, "bore.diameter");
        blends += claims(&drawing, "blend.radius");
        let planned: BTreeSet<&str> = plan
            .problems
            .iter()
            .filter(|p| p.message.contains("runs back across its own view"))
            .map(|p| p.subjects[0].as_str())
            .collect();
        let linted: BTreeSet<&str> = issues
            .iter()
            .filter(|i| i.code == "leader_crosses_silhouette")
            .filter_map(|i| i.annotation.as_deref())
            .collect();
        assert_eq!(planned, linted, "{name}");
    }
    // Nine across the fixtures and string_post (a counted callout names several holes).
    assert!(holes >= 9, "{holes} hole callouts checked");
    if std::env::var("DRAFTWRIGHT_CORPUS").is_ok() {
        assert!(blends >= 1, "{blends} blend callouts checked");
    }
}

/// ISSUE-798-d on a leader that does cut back into the material: the plate-and-pin assembly's
/// pin callout pinned with its bend out past the plate's edge, so its shaft leaves the pin and
/// crosses the plate. The plan (the router's measure, reported for a pinned leader it did not
/// route) and lint name the same callout with the same depth; pinned the other way, clear of
/// the plate, neither reports it.
#[test]
fn the_plan_and_lint_agree_on_a_leader_cutting_back_into_the_material() {
    use std::io::Read as _;
    const ID: &str = "d:f:turned_step:1/step.diameter";
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(fixture("pmi/step/assembly_plate_pin.step.gz")).unwrap(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("leader_rules");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("assembly_plate_pin.step");
    std::fs::write(&path, bytes).unwrap();
    let reports = |offset: [f64; 2]| -> (Vec<model::Problem>, Vec<LintIssue>) {
        let mut s = Session::open(&path, api::Features::Native, Spec::default()).unwrap();
        s.apply(
            &[Op::set(
                format!("dimensions/{}/place", key(ID)),
                json!({"offset": offset, "pinned": true}),
            )],
            true,
        )
        .unwrap();
        let planned = s
            .plan()
            .problems
            .iter()
            .filter(|p| p.code == "annotation_line_over_view")
            .filter(|p| p.subjects.first().map(String::as_str) == Some(ID))
            .cloned()
            .collect();
        let linted = lint::lint(s.analysis(), s.drawing())
            .into_iter()
            .filter(|i| i.code == "leader_crosses_silhouette")
            .collect();
        (planned, linted)
    };
    let depth = |text: &str, before: &str| -> String {
        let at = text.find(before).unwrap_or_else(|| panic!("{text}")) + before.len();
        text[at..].split(' ').next().unwrap().to_string()
    };
    let (planned, linted) = reports([15.0, 0.0]);
    assert_eq!(planned.len(), 1, "{planned:#?}");
    assert_eq!(linted.len(), 1, "{linted:#?}");
    assert_eq!(linted[0].annotation.as_deref(), Some(ID));
    let (p, l) = (
        depth(&planned[0].message, "cutting "),
        depth(&linted[0].message, "cuts "),
    );
    assert_eq!(p, l, "{} / {}", planned[0].message, linted[0].message);
    assert!(p.parse::<f64>().unwrap() > 1.0, "{p}");
    // The pin is the author's: the fix offered is to release it.
    assert_eq!(planned[0].fix.len(), 1, "{:?}", planned[0].fix);
    let (planned, linted) = reports([-15.0, 0.0]);
    assert!(
        planned.is_empty() && linted.is_empty(),
        "{planned:#?} {linted:#?}"
    );
}
