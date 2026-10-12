//! Shaft routing regressions from the Stage F draftsman review. Test the drawn geometry as
//! well as its problems: a missing warning must not make a bad route look clear.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::model::{self, Plan, Spec};
use serde_json::json;
use std::path::Path;

type P2 = [f64; 2];

fn shaft(plan: &Plan, id: &str) -> [P2; 2] {
    plan.dimensions
        .iter()
        .find(|d| d.id == id)
        .unwrap()
        .placed
        .as_ref()
        .unwrap()
        .lines[0]
}

fn circle_crossing([a, b]: [P2; 2], centre: P2, radius: f64) -> bool {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (((centre[0] - a[0]) * d[0] + (centre[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
        .clamp(0.0, 1.0);
    (a[0] + t * d[0] - centre[0]).hypot(a[1] + t * d[1] - centre[1]) < radius - 1e-6
}

fn project(plan: &Plan, a: &model::Analysis, view: &str, point: [f64; 3]) -> P2 {
    let v = plan.views.iter().find(|v| v.id == view).unwrap();
    let frame = draftwright::haecceity::hlr::View::new(v.direction, v.up).unwrap();
    let [lo, hi] = a.envelope;
    let corners: Vec<_> = (0..8)
        .map(|i| frame.map([0, 1, 2].map(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] })))
        .collect();
    let p = frame.map(point);
    [0, 1].map(|k| {
        let low = corners.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
        let high = corners
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max);
        v.at[k] + v.scale * (p[k] - 0.5 * (low + high))
    })
}

#[test]
fn right_pinned_plate_callouts_clear_other_holes_and_report_no_callout_problem() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (part, mut a) =
        model::analyse(&root.join("tests/fixtures/parts/mounting_plate.step")).unwrap();
    model::recognise(
        &part,
        &mut a,
        &std::fs::read_to_string(root.join("tools/mcp-eval/features/plate.features.json")).unwrap(),
    )
    .unwrap();
    let ids = [
        "d:f:hole_pattern:1/bore.diameter",
        "d:f:hole:4/bore.diameter",
        "d:f:hole:7/bore.diameter",
    ];
    let mut s = Spec::default();
    for (lane, id) in ids.iter().enumerate() {
        s=model::edit(&s,&json!([{"op":"set","path":format!("dimensions/{}/place",draftwright::items::key(id)),
            "value":{"view":"plan","side":"right","lane":lane,"along":(["start","middle","end"][lane]),"pinned":true}}])).unwrap();
    }
    let plan = model::plan_recovered(&a, &s).unwrap();
    let small = a.features.iter().find(|f| f.id == "f:hole:7").unwrap();
    let c = project(&plan, &a, "v:plan", small.fingerprint.position);
    let scale = plan.views.iter().find(|v| v.id == "v:plan").unwrap().scale;
    assert!(!circle_crossing(shaft(&plan, ids[1]), c, 2.1 * scale));
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.subjects.iter().any(|s| ids.contains(&s.as_str()))),
        "{:?}",
        plan.problems
    );
    let changed = model::edit(
        &s,
        &json!([{"op":"set","path":"dimensions/d:envelope~1width.length/omit","value":true}]),
    )
    .unwrap();
    assert_eq!(
        shaft(&plan, ids[1]),
        shaft(&model::plan_recovered(&a, &changed).unwrap(), ids[1])
    );
}

#[test]
fn notch_leader_exits_by_a_near_free_edge_instead_of_traversing_the_height() {
    let Some(c) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&c.join("mfcadpp/10020.step")).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let [tip, elbow] = shaft(&plan, "d:f:chamfer:1/chamfer.length");
    let v = plan.views.iter().find(|v| v.id == "v:front").unwrap();
    let width = (a.envelope[1][0] - a.envelope[0][0]) * v.scale;
    // Independently count the actual projected straight boundaries, including hidden features.
    // The former vertical route went to the station 3 paper mm above the view's envelope.
    let old = [
        tip,
        [
            tip[0],
            project(&plan, &a, "v:front", a.envelope[1])[1] + 3.0,
        ],
    ];
    let geometry =
        model::render_geometry(&part, &a, &plan, &mut model::Projections::default()).unwrap();
    let front = &geometry
        .views
        .iter()
        .find(|(id, _)| id == "v:front")
        .unwrap()
        .1;
    let crosses = |shaft: [P2; 2], edge: [P2; 2]| {
        let cross =
            |p: P2, q: P2, r: P2| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
        cross(shaft[0], shaft[1], edge[0]) * cross(shaft[0], shaft[1], edge[1]) < -1e-9
            && cross(edge[0], edge[1], shaft[0]) * cross(edge[0], edge[1], shaft[1]) < -1e-9
    };
    let count = |shaft| {
        front.visible.iter().chain(&front.hidden).filter(|item| {
        matches!(item,draftwright::output::Item::Edge(draftwright::output::Curve::Line(a,b)) if crosses(shaft,[*a,*b]))
    }).count()
    };
    eprintln!(
        "notch boundary crossings {} -> {}",
        count(old),
        count([tip, elbow])
    );
    assert!(count([tip, elbow]) < count(old));

    assert!(
        (elbow[0] - tip[0]).hypot(elbow[1] - tip[1]) < width,
        "{tip:?} -> {elbow:?}, width {width}"
    );
}

#[test]
fn hex_leader_avoids_the_blind_hole_band_it_previously_crossed() {
    let Some(c) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&c.join("nist/nist_ctc_01_asme1_rd.stp")).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let id = "d:f:polygonal_boss:1/polygon_across_flats.length";
    let [tip, elbow] = shaft(&plan, id);
    // The HEX is on the upper face. Its old downward shaft crossed the bottom outline and
    // the projected band of f:hole:1; leaving upwards removes that crossing.
    assert!(elbow[1] > tip[1], "{tip:?} -> {elbow:?}");
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "leader_through_feature" && p.subjects[0] == id),
        "{:?}",
        plan.problems
    );
}

#[test]
fn underside_cavity_containment_is_not_a_crossing_and_extensions_are_report_only() {
    let Some(c) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&c.join("nist/nist_ftc_08_asme1_rc.stp")).unwrap();
    let plan = model::plan_recovered(
        &a,
        &Spec {
            hole_table: draftwright::hole_table::Setting::Never,
            ..Spec::default()
        },
    )
    .unwrap();
    let callouts = [
        "d:f:hole_pattern:2/bore.diameter",
        "d:f:hole_pattern:3/bore.diameter",
        "d:f:hole:9/bore.diameter",
        "d:f:hole:15/bore.diameter",
        "d:f:hole:17/bore.diameter",
        "d:f:hole:19/bore.diameter",
    ];
    for id in callouts {
        assert!(
            plan.dimensions
                .iter()
                .any(|d| d.id == id && d.placed.is_some())
        );
        assert!(
            !plan
                .problems
                .iter()
                .any(|p| p.code == "leader_through_feature" && p.subjects == [id, "f:slot:1"]),
            "{id}: {:?}",
            plan.problems
        );
    }
    let extensions: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "extension_through_feature")
        .collect();
    assert!(!extensions.is_empty());
    assert!(extensions.iter().all(|p| p.fix.is_empty()));
}
