//! #224: frames and datum symbols on holes attach to the holes' size callout or table tag where
//! the holes are seen end-on, and a drawn frame citing an undrawn datum is said.
use std::path::Path;

use draftwright::model::{self, Analysis, Plan, Spec};

fn nist(name: &str) -> Analysis {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step")
        .join(name);
    model::analyse_native(&path).unwrap().1
}

const FTC10: &str = "nist_ftc_10_asme1_ap242-e2.stp.gz";
const CYLINDRICITY: &str = "pmi:geometric_tolerance.cylindricity:f101,102,104,105,107,108,110,111";
const DATUM_F: &str = "pmi:datum.datum:f244,245:t=F";

/// The size callout or tag box a moved glyph sits against, within *gap* paper mm across and
/// overlapping it along.
fn mark_box(plan: &Plan, b: [f64; 4], gap: f64) -> Option<[f64; 4]> {
    let beside = |m: [f64; 4]| {
        let along = b[0] < m[2] && b[2] > m[0];
        let vertical = (b[3] - m[1]).abs() <= gap || (m[3] - b[1]).abs() <= gap;
        along && vertical
    };
    plan.dimensions
        .iter()
        .filter(|d| d.kind == "callout")
        .filter_map(|d| d.placed.as_ref().map(|p| p.label_box))
        .chain(plan.balloons.iter().filter_map(|b| b.label_box))
        .find(|m| beside(*m))
}

#[test]
fn ftc10_hole_frames_and_datums_attach_to_their_marks() {
    let a = nist(FTC10);
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let frame = plan.gdt.iter().find(|g| g.id == CYLINDRICITY).unwrap();
    let p = frame.placed.as_ref().unwrap();
    // Seen end-on (the plan looks down z), with no leader to blank material, beneath its callout
    // and left-aligned with it.
    assert_eq!(frame.view, "v:plan");
    assert!(p.lines.is_empty() && p.arrows.is_empty(), "{p:?}");
    let callout = mark_box(&plan, p.label_box, 1.0).expect("a callout above the frame");
    assert!((callout[0] - p.label_box[0]).abs() < 1e-6);
    assert!(frame.basis.contains("size callout"), "{}", frame.basis);

    // The datum's triangle stands on its tag in the view that sees its hole end-on.
    let datum = plan.gdt.iter().find(|g| g.id == DATUM_F).unwrap();
    assert_eq!(datum.view, "v:front");
    let d = datum.placed.as_ref().unwrap();
    let triangle = d.fills[0].iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |t, q| {
            [
                t[0].min(q[0]),
                t[1].min(q[1]),
                t[2].max(q[0]),
                t[3].max(q[1]),
            ]
        },
    );
    assert!(
        mark_box(&plan, triangle, 0.5 + 1e-6).is_some(),
        "the triangle's base stands on its mark: {triangle:?}"
    );
    // Nothing moved has a finding of its own.
    for id in [CYLINDRICITY, DATUM_F] {
        let own: Vec<_> = plan
            .problems
            .iter()
            .filter(|p| p.code.starts_with("annotation_") && p.subjects.iter().any(|s| s == id))
            .collect();
        assert!(own.is_empty(), "{id}: {own:#?}");
    }
}

/// A frame on holes whose bores are split in halves at an angle (so neither half spans the
/// diameter across a principal axis) still finds its holes' size callout.
#[test]
fn a_frame_on_split_bores_attaches_to_their_callout() {
    let a = nist("nist_stc_09_asme1_ap242-e4.stp.gz");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let frame = plan
        .gdt
        .iter()
        .find(|g| {
            g.id == "pmi:geometric_tolerance.position:f42,43,44,45,46,47,54,55:ref=A,B,C:mod=diameter_zone"
        })
        .unwrap();
    assert!(frame.basis.contains("size callout"), "{}", frame.basis);
    let p = frame.placed.as_ref().unwrap();
    assert!(p.lines.is_empty(), "{p:?}");
    assert!(mark_box(&plan, p.label_box, 1.0).is_some());
}

/// The Spec's placement intent is the override: a frame placed on a side keeps its leader, a
/// datum placed in a view its surface.
#[test]
fn a_placed_hole_frame_or_datum_keeps_its_lowered_attachment() {
    let a = nist(FTC10);
    let spec: Spec = serde_json::from_value(serde_json::json!({"annotations": {
        CYLINDRICITY: {"place": {"side": "below"}},
        DATUM_F: {"place": {"view": "side"}},
    }}))
    .unwrap();
    let plan = model::plan_recovered(&a, &spec).unwrap();
    let frame = plan.gdt.iter().find(|g| g.id == CYLINDRICITY).unwrap();
    assert!(!frame.placed.as_ref().unwrap().lines.is_empty());
    assert!(!frame.basis.contains("size callout"));
    let datum = plan.gdt.iter().find(|g| g.id == DATUM_F).unwrap();
    assert_eq!(datum.view, "v:side");
    assert!(!datum.basis.contains("table tag"));
}

/// A bare pin on an attached datum holds it on its hole's size callout, not back on the edge-on
/// silhouette it was lowered onto.
#[test]
fn a_bare_pin_holds_an_attached_datum() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pmi/step/spool_fits.step.gz");
    let a = model::analyse_native(&path).unwrap().1;
    const DATUM_B: &str = "pmi:datum.datum:f55:t=B";
    let free = model::plan_recovered(&a, &Spec::default()).unwrap();
    let spec: Spec = serde_json::from_value(serde_json::json!({"annotations": {
        DATUM_B: {"place": {"pinned": true}},
    }}))
    .unwrap();
    let pinned = model::plan_recovered(&a, &spec).unwrap();
    let datum = |p: &Plan| p.gdt.iter().find(|g| g.id == DATUM_B).unwrap().clone();
    let (free, pinned) = (datum(&free), datum(&pinned));
    assert_eq!(free.view, "v:plan");
    assert!(free.basis.contains("size callout"), "{}", free.basis);
    assert_eq!(pinned.view, free.view);
    let (f, p) = (free.placed.unwrap(), pinned.placed.unwrap());
    assert!(
        (0..2).all(|k| (f.tip[k] - p.tip[k]).abs() < 1e-6),
        "{:?} vs {:?}",
        f.tip,
        p.tip
    );
}

/// Every shown frame citing a datum with no shown symbol is said once, naming what it lacks,
/// and no other frame is.
#[test]
fn frames_citing_undrawn_datums_are_said() {
    let a = nist("nist_stc_10_asme1_ap242-e2.stp.gz");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let drawn: Vec<&str> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature" && g.state == draftwright::items::State::Shown)
        .map(|g| g.cells[0].as_str())
        .collect();
    let mut said = 0;
    for g in plan
        .gdt
        .iter()
        .filter(|g| g.kind == "control_frame" && g.state == draftwright::items::State::Shown)
    {
        let missing: Vec<&str> = g.cells[2..]
            .iter()
            .map(String::as_str)
            .filter(|d| !drawn.contains(d))
            .collect();
        let found: Vec<_> = plan
            .problems
            .iter()
            .filter(|p| p.code == "frame_datum_not_drawn" && p.subjects == [g.id.clone()])
            .collect();
        if missing.is_empty() {
            assert!(found.is_empty(), "{}: {found:#?}", g.id);
        } else {
            assert_eq!(found.len(), 1, "{}", g.id);
            assert_eq!(found[0].severity, "warning");
            for d in missing {
                assert!(found[0].refs.contains(&format!("datum {d}")));
            }
            said += 1;
        }
    }
    // stc-10's symmetry frame alone cites an undrawn datum, J, which the file never lowers.
    assert_eq!(said, 1);
    let lacking: Vec<&String> = plan
        .problems
        .iter()
        .filter(|p| p.code == "frame_datum_not_drawn")
        .flat_map(|p| &p.refs)
        .collect();
    assert_eq!(lacking, ["datum J"]);
}

/// Where a datum's place on its hole's mark runs past the view's reserved area into vacant
/// paper, that area grows to hold it: both of CTC-01's hole datums and CTC-02's datum F attach in
/// the plan view, each with no finding of its own, and the grown area stays clear of every
/// other view's.
#[test]
fn hole_datums_attach_where_their_view_area_can_grow() {
    for (part, datums) in [
        (
            "nist_ctc_01_asme1_ap242-e1.stp.gz",
            &["pmi:datum.datum:f75,76:t=C", "pmi:datum.datum:f89,90:t=B"][..],
        ),
        (
            "nist_ctc_02_asme1_ap242-e2.stp.gz",
            &["pmi:datum.datum:f620,621:t=F"][..],
        ),
    ] {
        let a = nist(part);
        let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
        let view = plan.views.iter().find(|v| v.id == "v:plan").unwrap();
        for id in datums {
            let datum = plan.gdt.iter().find(|g| g.id == *id).unwrap();
            assert_eq!(datum.view, "v:plan", "{part} {id}");
            assert!(
                datum.basis.contains("end-on"),
                "{part} {id}: {}",
                datum.basis
            );
            let p = datum.placed.as_ref().unwrap();
            let a = view.area;
            for q in p.lines.iter().flatten().chain(p.fills.iter().flatten()) {
                assert!(
                    a[0] <= q[0] && q[0] <= a[2] && a[1] <= q[1] && q[1] <= a[3],
                    "{part} {id}: {q:?} outside {a:?}"
                );
            }
            let own: Vec<_> = plan
                .problems
                .iter()
                .filter(|p| p.subjects.iter().any(|s| s == id))
                .filter(|p| p.code.starts_with("annotation_") || p.code.contains("through"))
                .collect();
            assert!(own.is_empty(), "{part} {id}: {own:#?}");
        }
        for other in plan
            .views
            .iter()
            .filter(|v| v.id != view.id && v.sheet == view.sheet)
        {
            let (a, b) = (view.area, other.area);
            assert!(
                !(a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]),
                "{part}: the plan's area {a:?} overlaps {}'s {b:?}",
                other.id
            );
        }
    }
}
