//! LINT-angular-profile_angle_requirement (ISO 129-1, ASME Y14.5): an outer-profile corner
//! needs its geometry defined, not necessarily its angle printed. A corner the linear
//! dimensions fix needs no angle; one a single station short is completed linearly; any other
//! gets its angle; a closed polygon's equal corners get one counted angle; an angle with no
//! clear place is dropped and stays a lint warning; Spec overrides choose per corner and per
//! profile.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::api::{self, Features};
use draftwright::dims::PlannedDimension;
use draftwright::items::State;
use draftwright::lint::profile_angles::{self, Corner};
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::json;
use std::sync::OnceLock;

/// A synthetic part: a 40 × 20 × 10 block whose top face (face 1) has the outer profile
/// through *corners* (anticlockwise, seen from +z).
fn part(corners: &[[f64; 2]]) -> Analysis {
    let n = corners.len();
    let supports: Vec<_> = (0..n)
        .map(|k| {
            let (a, b) = (corners[k], corners[(k + 1) % n]);
            json!({"kind":"line","start":[a[0],a[1],10],"end":[b[0],b[1],10]})
        })
        .collect();
    let (lo, hi) = corners.iter().fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            )
        },
    );
    serde_json::from_value(json!({"schema":model::ANALYSIS_SCHEMA,
        "binding":{"source_sha256":"","part":"test","face_count":20,"loader":"test"},
        "envelope":[[lo[0],lo[1],0],[hi[0],hi[1],10]],"iso_bounds":[lo[0],lo[1],hi[0],hi[1]],
        "features":[],
        "outer_profiles":[{"face":1,"body_faces":(1..=12).collect::<Vec<_>>(),
            "edges":(1..=n).collect::<Vec<_>>(),"profile":{"origin":[0,0,10],"normal":[0,0,1],
            "schema_version":1,"boundary_kind":"outer","inner_loop_count":0,"supports":supports}}]}))
    .unwrap()
}

fn plan(a: &Analysis, spec: serde_json::Value) -> Plan {
    model::plan(a, &serde_json::from_value(spec).unwrap()).unwrap()
}

fn angles(p: &Plan) -> Vec<&PlannedDimension> {
    p.dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter == "profile_angle.angle")
        })
        .collect()
}

/// A corner's angle ID (`d:envelope/profile_angle.angle.f1_e1_e2`) from its requirement.
fn angle_id(feature: &str) -> String {
    format!(
        "d:envelope/profile_angle.angle.{}",
        feature
            .strip_prefix("outer_profile:")
            .unwrap()
            .replace(':', "_")
    )
}

fn stations(p: &Plan) -> Vec<&PlannedDimension> {
    p.dimensions
        .iter()
        .filter(|d| d.id.starts_with("d:envelope/profile_location_"))
        .collect()
}

/// A trapezoid: its sloping side runs from (40, 0) to (30, 20). Every station of both its
/// corners is on the part's box except x = 30.
const TRAPEZOID: [[f64; 2]; 4] = [[0., 0.], [40., 0.], [30., 20.], [0., 20.]];
/// A pentagon whose sloping side runs from (40, 10) to (25, 20): x = 25 and y = 10 are both
/// off the box.
const PENTAGON: [[f64; 2]; 5] = [[0., 0.], [40., 0.], [40., 10.], [25., 20.], [0., 20.]];

#[test]
fn a_linearly_fixed_corner_needs_no_angle() {
    let corner = Corner {
        feature: "outer_profile:f1:e1:e2".into(),
        axis: 2,
        vertex: [40., 0., 10.],
        first: [0., 0., 10.],
        second: [30., 20., 10.],
        virtual_vertex: false,
        ends: [[0., 0., 10.], [30., 20., 10.]],
    };
    let a = part(&TRAPEZOID);
    let box_only: Vec<(usize, f64, f64)> = Vec::new();
    let missing = profile_angles::undefined_stations(&corner, &|k, v| {
        profile_angles::rooted(&a, &box_only, k, v)
    });
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert_eq!(missing[0].0, 0);
    assert!((missing[0].1 - 30.).abs() < 1e-9);
    // A chain of linear dimensions from the box reaches x = 30: the corner is defined.
    let chain = vec![(0, 0., 12.), (0, 12., 30.)];
    assert!(
        profile_angles::undefined_stations(&corner, &|k, v| {
            profile_angles::rooted(&a, &chain, k, v)
        })
        .is_empty()
    );
    // An unrooted span proves nothing.
    let floating = vec![(0, 12., 30.)];
    assert_eq!(
        profile_angles::undefined_stations(&corner, &|k, v| {
            profile_angles::rooted(&a, &floating, k, v)
        })
        .len(),
        1
    );
}

/// One station short is still not fixed: by default the corner gets its angle. Completing it
/// linearly is the profile's `linear` choice (see the CTC-03 test), never automatic.
#[test]
fn a_nearly_fixed_corner_gets_its_angle_by_default() {
    let a = part(&TRAPEZOID);
    assert_eq!(profile_angles::requirements(&a).len(), 2);
    let p = plan(&a, json!({}));
    assert_eq!(angles(&p).len(), 2, "{:?}", angles(&p));
    assert!(stations(&p).is_empty(), "{:?}", stations(&p));
}

#[test]
fn an_under_defined_corner_gets_its_angle() {
    let a = part(&PENTAGON);
    let p = plan(&a, json!({}));
    let found = angles(&p);
    assert_eq!(found.len(), 2, "{found:?}");
    for d in &found {
        assert_eq!(d.view, "v:plan");
        assert!(
            !d.label.contains('×') && d.label.ends_with('°'),
            "{}",
            d.label
        );
        assert_eq!(d.measurements.len(), 1);
    }
    assert!(stations(&p).is_empty());
}

#[test]
fn a_regular_polygon_gets_one_counted_angle() {
    let hexagon: Vec<[f64; 2]> = (0..6)
        .map(|k| {
            let t = (60. * k as f64).to_radians();
            [20. + 20. * t.cos(), 20. + 20. * t.sin()]
        })
        .collect();
    let a = part(&hexagon);
    assert_eq!(profile_angles::requirements(&a).len(), 6);
    let p = plan(&a, json!({}));
    let found = angles(&p);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].id, "d:envelope/profile_angle.angle.f1");
    assert_eq!(found[0].label, "6× 120°");
    assert_eq!(found[0].measurements.len(), 6);
}

#[test]
fn a_shown_corner_keeps_its_angle_and_a_hidden_one_is_omitted() {
    let a = part(&TRAPEZOID);
    let id = profile_angles::requirements(&a)
        .iter()
        .map(|c| angle_id(&c.feature))
        .next()
        .unwrap();
    // Show an angle the defaults would draw anyway: it stays, now listed in the spec.
    let p = plan(&a, json!({"dimensions": {&id: {}}}));
    assert!(angles(&p).iter().any(|d| d.id == id), "{:?}", angles(&p));
    // Hide an angle the planner would draw.
    let b = part(&PENTAGON);
    let hidden = angle_id(&profile_angles::requirements(&b)[0].feature);
    let p = plan(&b, json!({"dimensions": {&hidden: {"omit": true}}}));
    let d = p.dimensions.iter().find(|d| d.id == hidden);
    assert!(d.is_none_or(|d| d.state == State::OmittedByUser), "{d:?}");
    assert!(
        p.omitted
            .iter()
            .any(|o| o.id == hidden && o.state == State::OmittedByUser)
    );
    assert_eq!(angles(&p).len(), 1);
}

fn ctc03() -> &'static std::path::PathBuf {
    static PATH: OnceLock<std::path::PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        corpus::dir()
            .expect("corpus")
            .join("nist/nist_ctc_03_asme1_rc.stp")
    })
}

fn review(path: &std::path::Path, spec: serde_json::Value) -> api::Review {
    let spec: Spec = serde_json::from_value(spec).unwrap();
    api::review_from(path, Features::Native, &spec).unwrap()
}

fn missing(r: &api::Review) -> Vec<String> {
    r.lint
        .iter()
        .filter(|i| i.code.starts_with("angular_requirement_"))
        .flat_map(|i| i.requirements.iter().map(|q| q.feature.clone()))
        .collect()
}

/// CTC-03's two 135° corners: by default each is drawn where clear, or dropped and still
/// reported; `faces/face:37/profile_corners: linear` locates their stations instead, and lint
/// reads those drawn dimensions as defining both.
#[test]
fn ctc03_corners_are_stated_by_angle_or_linearly_by_profile_choice() {
    if corpus::dir().is_none() {
        return;
    }
    let r = review(ctc03(), json!({}));
    let dropped: Vec<_> = r
        .problems
        .iter()
        .filter(|p| p.code == "angular_dimension_dropped")
        .flat_map(|p| p.subjects.clone())
        .collect();
    assert!(
        dropped.len() < 2,
        "at least one corner is drawn: {dropped:?}"
    );
    let missing_now = missing(&r);
    assert_eq!(missing_now.len(), dropped.len(), "{missing_now:?}");
    assert!(
        dropped
            .iter()
            .all(|id| missing_now.iter().any(|m| &angle_id(m) == id))
    );
    let linear = review(
        ctc03(),
        json!({"faces": {"face:37": {"profile_corners": "linear"}}}),
    );
    assert!(
        !linear
            .problems
            .iter()
            .any(|p| p.code == "angular_dimension_dropped"),
        "{:?}",
        linear.problems
    );
    assert!(missing(&linear).is_empty(), "{:?}", missing(&linear));
    let a = model::analyse_native(ctc03()).unwrap().1;
    let p = model::plan_recovered(
        &a,
        &serde_json::from_value(json!({"faces": {"face:37": {"profile_corners": "linear"}}}))
            .unwrap(),
    )
    .unwrap();
    assert!(angles(&p).is_empty());
    assert!(!stations(&p).is_empty());
    assert!(stations(&p).iter().all(|d| d.placed.is_some()));
}

/// mfcadpp/10049: a corner whose angle finds no clear place is dropped, never forced, and
/// lint still reports it missing.
#[test]
fn a_failed_placement_leaves_the_corner_a_lint_warning() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let r = review(&dir.join("mfcadpp/10049.step"), json!({}));
    let dropped: Vec<_> = r
        .problems
        .iter()
        .filter(|p| p.code == "angular_dimension_dropped")
        .flat_map(|p| p.subjects.clone())
        .collect();
    assert!(!dropped.is_empty());
    let missing = missing(&r);
    for id in &dropped {
        assert!(
            missing.iter().any(|m| &angle_id(m) == id),
            "{id}: {missing:?}"
        );
    }
}
