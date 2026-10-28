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
const DATUM_F: &str = "pmi:datum.datum:f244,245:t=F";
const STC09: &str = "nist_stc_09_asme1_ap242-e4.stp.gz";
const SPLIT_POSITION: &str =
    "pmi:geometric_tolerance.position:f42,43,44,45,46,47,54,55:ref=A,B,C:mod=diameter_zone";

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

/// FTC-10's datum F stands on its hole table tag. (Its plan-view cylindricity frame on the ⌀7
/// holes attached beneath their callout until the schedule rows replaced the front holes' PMI
/// callouts, 2026-10-28: the freed front band now holds the position frames above the front
/// view, the plan view sits lower, and the ⌀7 callout has no clear paper beneath it, so that
/// frame keeps its lowered leader. The frame attachment is checked on STC-09 below.)
#[test]
fn ftc10_hole_datums_attach_to_their_marks() {
    let a = nist(FTC10);
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
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
    let own: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code.starts_with("annotation_") && p.subjects.iter().any(|s| s == DATUM_F))
        .collect();
    assert!(own.is_empty(), "{DATUM_F}: {own:#?}");
}

/// A frame on holes whose bores are split in halves at an angle (so neither half spans the
/// diameter across a principal axis) still finds its holes' size callout: seen end-on, with no
/// leader to blank material, beneath its callout and left-aligned with it, and with no finding
/// of its own.
#[test]
fn a_frame_on_split_bores_attaches_to_their_callout() {
    let a = nist(STC09);
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let frame = plan.gdt.iter().find(|g| g.id == SPLIT_POSITION).unwrap();
    assert!(frame.basis.contains("size callout"), "{}", frame.basis);
    let p = frame.placed.as_ref().unwrap();
    assert!(p.lines.is_empty() && p.arrows.is_empty(), "{p:?}");
    let callout = mark_box(&plan, p.label_box, 1.0).expect("a callout beside the frame");
    assert!((callout[0] - p.label_box[0]).abs() < 1e-6);
    let own: Vec<_> = plan
        .problems
        .iter()
        .filter(|q| {
            q.code.starts_with("annotation_") && q.subjects.iter().any(|s| s == SPLIT_POSITION)
        })
        .collect();
    assert!(own.is_empty(), "{own:#?}");
}

/// The Spec's placement intent is the override: a frame placed on a side keeps its leader, a
/// datum placed in a view its surface.
#[test]
fn a_placed_hole_frame_or_datum_keeps_its_lowered_attachment() {
    let a = nist(STC09);
    let spec: Spec = serde_json::from_value(serde_json::json!({"annotations": {
        SPLIT_POSITION: {"place": {"side": "below"}},
    }}))
    .unwrap();
    let plan = model::plan_recovered(&a, &spec).unwrap();
    let frame = plan.gdt.iter().find(|g| g.id == SPLIT_POSITION).unwrap();
    assert!(!frame.placed.as_ref().unwrap().lines.is_empty());
    assert!(!frame.basis.contains("size callout"));
    let a = nist(FTC10);
    let spec: Spec = serde_json::from_value(serde_json::json!({"annotations": {
        DATUM_F: {"place": {"view": "side"}},
    }}))
    .unwrap();
    let plan = model::plan_recovered(&a, &spec).unwrap();
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

const STC06: &str = "nist_stc_06_asme1_ap242-e3.stp.gz";

/// The identity of *letter*'s datum on *a*.
fn datum_id(a: &Analysis, letter: &str) -> String {
    let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi else {
        panic!("PMI read");
    };
    let ids = report.identities();
    let r = report
        .records
        .iter()
        .find(|r| r.category == draftwright::pmi::Category::Datum && r.label == letter)
        .unwrap();
    ids.id(&r.source_id).unwrap().to_string()
}

/// A datum on a width's two opposite planes is the width's median plane: STC-06's datum B, which
/// twelve frames cite, stands on an extension line of its 304.8 size dimension, in line with the
/// dimension line (ISO 5459), not on either face; no frame then lacks it, and the refusal that
/// left it undrawn is gone. `annotations/<id>/omit` leaves it out.
#[test]
fn a_width_datum_stands_on_its_size_dimension() {
    let a = nist(STC06);
    let b = datum_id(&a, "B");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let datum = plan.gdt.iter().find(|g| g.id == b).expect("datum B drawn");
    assert_eq!(datum.state, draftwright::items::State::Shown);
    assert!(datum.basis.contains("median plane"), "{}", datum.basis);
    let p = datum.placed.as_ref().unwrap();
    let size = plan
        .dimensions
        .iter()
        .find(|d| d.view == datum.view && d.label.starts_with("304.8"))
        .expect("its size dimension in the datum's view");
    let s = size.placed.as_ref().unwrap();
    // The triangle's base is on one of the dimension's arrow tips (where its dimension line meets
    // its extension line), and the symbol's line runs along the dimension line, outward.
    let tips: Vec<[f64; 2]> = s.arrows.iter().map(|a| a[0]).collect();
    let tip = tips
        .iter()
        .find(|t| (t[0] - p.tip[0]).abs() < 1e-6 && (t[1] - p.tip[1]).abs() < 1e-6)
        .expect("the triangle on an arrow tip");
    let other = tips.iter().find(|t| *t != tip).unwrap();
    for line in &p.lines {
        for q in line {
            assert!(
                (q[1] - tip[1]).abs() < 1e-6,
                "in line with the dimension line"
            );
            assert!(
                (q[0] - tip[0]) * (tip[0] - other[0]) >= -1e-6,
                "outward: {line:?}"
            );
        }
    }
    let own: Vec<_> = plan
        .problems
        .iter()
        .filter(|q| q.code.starts_with("annotation_") && q.subjects.contains(&b))
        .collect();
    assert!(own.is_empty(), "{own:#?}");
    assert!(
        !plan
            .problems
            .iter()
            .any(|q| q.code == "frame_datum_not_drawn" && q.refs.contains(&"datum B".into()))
    );
    assert!(
        !plan
            .problems
            .iter()
            .any(|q| q.code == "pmi_not_lowered" && q.refs.first() == Some(&datum.source))
    );

    let mut spec = Spec::default();
    spec.annotations.insert(
        b.clone(),
        serde_json::from_value(serde_json::json!({"omit": true})).unwrap(),
    );
    let omitted = model::plan_recovered(&a, &spec).unwrap();
    assert!(
        !omitted
            .gdt
            .iter()
            .any(|g| g.id == b && g.state == draftwright::items::State::Shown)
    );
}

/// A datum feature that names no face of its own, which the reader reads as its datum's usage's
/// faces (a nonconformance it reads through), is drawn on those faces, the reading in its basis:
/// CTC-02's datum D.
#[test]
fn a_datum_read_through_a_nonconformance_is_drawn() {
    let a = nist("nist_ctc_02_asme1_ap242-e2.stp.gz");
    let d = datum_id(&a, "D");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let datum = plan
        .gdt
        .iter()
        .find(|g| g.id == d)
        .expect("datum D lowered");
    assert_eq!(datum.state, draftwright::items::State::Shown);
    assert!(datum.basis.contains("read as"), "{}", datum.basis);
    assert!(
        !plan
            .problems
            .iter()
            .any(|q| q.code == "frame_datum_not_drawn" && q.refs.contains(&"datum D".into()))
    );
}
