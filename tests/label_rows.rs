//! E1: the editor reads the same exterior callout rows that resolve a pin.
use draftwright::model::Spec;
use draftwright::{Session, api, dims};
use serde_json::json;
use std::path::Path;

#[test]
fn plate_rows_follow_automatic_frames_widths_and_explicit_pins() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step");
    let mut session = Session::open(&path, api::Features::Native, Spec::default()).unwrap();
    let id = "d:f:hole_pattern:1/bore.diameter";
    let original = session
        .plan()
        .dimensions
        .iter()
        .find(|d| d.id == id)
        .unwrap()
        .placed
        .clone();
    let guides = session.label_rows(id).unwrap();
    let plan = session.plan();
    let view = plan.views.iter().find(|v| v.id == "v:plan").unwrap();
    let guide = &guides["v:plan"];
    assert_eq!(
        guide.outline,
        [
            view.at[0] - 50.0 * view.scale,
            view.at[1] - 30.0 * view.scale,
            view.at[0] + 50.0 * view.scale,
            view.at[1] + 30.0 * view.scale
        ]
    );
    let width = plan
        .dimensions
        .iter()
        .filter(|d| d.kind == "callout" && d.view == "v:plan")
        .filter_map(|d| d.placed.as_ref())
        .map(|p| p.label_box[2] - p.label_box[0])
        .fold(0.0, f64::max);
    assert!((guide.row_spacing["left"] - width - 10.0).abs() < 1e-6);
    assert_eq!(guide.row_spacing["above"], dims::LANE);
    assert_eq!(guide.row_spacing["below"], dims::LANE);
    assert_eq!(guide.first_row["above"], guide.outline[3] + dims::FIRST);
    assert_eq!(guide.first_row["below"], guide.outline[1] - dims::FIRST);
    let stats = session.stats();
    assert_eq!(guides, session.label_rows(id).unwrap());
    assert_eq!(
        stats,
        session.stats(),
        "snap guides must not plan or project"
    );
    let key = draftwright::items::key(id);
    session
        .apply(
            &[draftwright::items::Op::set(
                format!("dimensions/{key}/place"),
                json!({"pinned":true}),
            )],
            true,
        )
        .unwrap();
    assert_eq!(
        session
            .plan()
            .dimensions
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .placed,
        original
    );
    assert_eq!(session.label_rows(id).unwrap(), guides);
    for side in ["above", "below", "left", "right"] {
        for lane in [0, 1, 2] {
            session
                .apply(
                    &[draftwright::items::Op::set(
                        format!("dimensions/{key}/place"),
                        json!({"view":"plan","side":side,"lane":lane,"along":0.5,"pinned":true}),
                    )],
                    true,
                )
                .unwrap();
            let ink = session
                .plan()
                .dimensions
                .iter()
                .find(|d| d.id == id)
                .unwrap()
                .placed
                .as_ref()
                .unwrap();
            let axis = usize::from(side == "above" || side == "below");
            let sign = if side == "above" || side == "right" {
                1.0
            } else {
                -1.0
            };
            let expected = guide.first_row[side] + sign * f64::from(lane) * guide.row_spacing[side];
            assert!(
                (ink.lines[0][1][axis] - expected).abs() < 1e-6,
                "{side} lane {lane}: {ink:?}"
            );
            assert_eq!(session.label_rows(id).unwrap(), guides);
        }
    }
    // Another callout's longer text must not change this pin's column grid.
    session
        .apply(
            &[draftwright::items::Op::set(
                "dimensions/d:f:hole:7~1bore.diameter/tolerance",
                json!("±1234.5678"),
            )],
            true,
        )
        .unwrap();
    assert_eq!(session.label_rows(id).unwrap(), guides);
    assert_eq!(
        session.label_rows(&format!("sheet:1/{id}")).unwrap(),
        guides
    );
    assert!(session.label_rows(&format!("sheet:2/{id}")).is_err());
    assert!(session.label_rows("sheet").is_err());
    assert!(session.label_rows("d:missing/diameter").is_err());
}
