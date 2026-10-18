//! Gutters spend slack after sheet/scale selection; authored gaps remain distances.
use draftwright::model::{self, Plan, Spec};
use serde_json::json;
use std::sync::OnceLock;

fn plate() -> &'static model::Analysis {
    static A: OnceLock<model::Analysis> = OnceLock::new();
    A.get_or_init(|| {
        model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/samples/editor-plate.step"),
        )
        .unwrap()
        .1
    })
}
fn plan(value: serde_json::Value) -> Plan {
    model::plan_recovered(plate(), &serde_json::from_value::<Spec>(value).unwrap()).unwrap()
}
fn gap(a: [f64; 4], b: [f64; 4]) -> f64 {
    (a[0] - b[2])
        .max(b[0] - a[2])
        .max(a[1] - b[3])
        .max(b[1] - a[3])
}
fn minimum(p: &Plan) -> f64 {
    p.views
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            p.views[..i]
                .iter()
                .filter(move |b| b.sheet == a.sheet)
                .map(move |b| gap(a.area, b.area))
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn roomy_sheet_has_preferred_gutters_and_projection_origins() {
    for projection in ["first", "third"] {
        let p = plan(json!({"sheet":{"size":"A2","scale":"1:1","projection":projection}}));
        assert!(minimum(&p) >= 25.0 - 1e-6, "{}", minimum(&p));
        let front = p.views.iter().find(|v| v.id == "v:front").unwrap();
        for (id, k) in [("v:plan", 0), ("v:side", 1)] {
            if let Some(v) = p.views.iter().find(|v| v.id == id) {
                assert!((v.at[k] - front.at[k]).abs() < 1e-6);
            }
        }
    }
}

#[test]
fn gutter_never_changes_automatic_sheet_scale_or_view_scales() {
    let bare = plan(json!({"sheet":{"view_gutter":0}}));
    for gutter in [6, 12, 1000] {
        let p = plan(json!({"sheet":{"view_gutter":gutter}}));
        assert_eq!(
            (&p.sheet.size, &p.sheet.scale),
            (&bare.sheet.size, &bare.sheet.scale)
        );
        for (v, b) in p.views.iter().zip(&bare.views) {
            assert_eq!((&v.id, v.scale), (&b.id, b.scale));
        }
        for v in &p.views {
            assert!(v.area[0] >= 10.0 - 1e-6 && v.area[1] >= 10.0 - 1e-6);
            assert!(
                v.area[2] <= p.sheet.page[0] - 10.0 + 1e-6
                    && v.area[3] <= p.sheet.page[1] - 10.0 + 1e-6
            );
            assert!(gap(v.area, p.sheet.title_block) >= -1e-6);
        }
    }
}

#[test]
fn explicit_projection_gap_is_kept_including_zero() {
    for gap in [0, 5, 30] {
        let p = plan(json!({"sheet":{"size":"A2","scale":"1:1"},"views":{"plan":{
            "place":{"relative_to":"front","side":"above","gap":gap,"align":"projection","pinned":true}}}}));
        let front = p.views.iter().find(|v| v.id == "v:front").unwrap();
        let top = p.views.iter().find(|v| v.id == "v:plan").unwrap();
        assert!((top.area[1] - front.area[3] - gap as f64).abs() < 1e-6);
        assert!((top.at[0] - front.at[0]).abs() < 1e-6);
    }
}

#[test]
fn gutter_override_validates_and_round_trips() {
    let p = plan(json!({"sheet":{"size":"A2","scale":"1:1","view_gutter":20}}));
    assert!(minimum(&p) >= 20.0 - 1e-6);
    assert_eq!(
        serde_json::to_value(&p.spec).unwrap()["sheet"]["view_gutter"],
        20.0
    );
    assert!(serde_json::from_value::<Spec>(json!({"sheet":{"view_gutter":-1}})).is_err());
    let mut invalid = Spec::default();
    invalid.sheet.view_gutter = Some(-1.0);
    assert!(model::plan_recovered(plate(), &invalid).is_err());
}

#[test]
fn settled_routes_move_rigidly_with_their_view() {
    let bare = plan(json!({"sheet":{"size":"A2","scale":"1:1","view_gutter":0}}));
    let spaced = plan(json!({"sheet":{"size":"A2","scale":"1:1"}}));
    let mut moved = false;
    for d in &spaced.dimensions {
        let Some(p) = &d.placed else { continue };
        let before = bare
            .dimensions
            .iter()
            .find(|b| b.id == d.id)
            .unwrap()
            .placed
            .as_ref()
            .unwrap();
        let v = spaced.views.iter().find(|v| v.id == d.view).unwrap();
        let b = bare.views.iter().find(|v| v.id == d.view).unwrap();
        let delta = [v.at[0] - b.at[0], v.at[1] - b.at[1]];
        moved |= delta[0].abs() + delta[1].abs() > 1e-6;
        for (point, old) in p
            .lines
            .iter()
            .chain(&p.arrows)
            .flatten()
            .chain(std::iter::once(&p.label_at))
            .zip(
                before
                    .lines
                    .iter()
                    .chain(&before.arrows)
                    .flatten()
                    .chain(std::iter::once(&before.label_at)),
            )
        {
            for k in 0..2 {
                assert!((point[k] - old[k] - delta[k]).abs() < 1e-6, "{}", d.id);
            }
        }
    }
    assert!(moved);
}
