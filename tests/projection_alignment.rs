//! #109: spacing projected geometry must not align asymmetric dimension bands.
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

fn plate() -> Analysis {
    model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/samples/editor-plate.step"),
    )
    .unwrap()
    .1
}

fn planned(a: &Analysis, value: Value) -> Plan {
    model::plan_recovered(a, &serde_json::from_value::<Spec>(value).unwrap()).unwrap()
}

fn rounded(value: Value) -> Value {
    match value {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => {
            json!(format!("{:.2}", n.as_f64().unwrap()))
        }
        Value::Array(items) => Value::Array(items.into_iter().map(rounded).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, rounded(v))).collect())
        }
        other => other,
    }
}

#[test]
fn legacy_alignments_plan_byte_identically() {
    let a = plate();
    let expected = include_str!("fixtures/projection_alignment/legacy.sha256");
    for line in expected.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let sheets = fields[0] == "true";
        let align = fields[1];
        let mut value = json!({"sheet":{"size":"A2","scale":"2:1"}});
        if sheets {
            value["sheets"] = json!({"max":1});
        }
        if align != "none" {
            value["views"] = json!({"plan":{"place":{"relative_to":"front","side":"above","gap":15,"align":align,"pinned":true}}});
        }
        // Floats are rounded to 0.01 mm and the analysis hash is dropped, so platform
        // floating-point differences (libm, analysis bytes) do not count.
        let mut plan = serde_json::to_value(planned(&a, value)).unwrap();
        plan.as_object_mut().unwrap().remove("analysis_sha256");
        let bytes = serde_json::to_vec(&rounded(plan)).unwrap();
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if hash != fields[2] {
            eprintln!("{}", String::from_utf8_lossy(&bytes));
        }
        assert_eq!(hash, fields[2], "sheets={sheets}, align={align}");
    }
}

// A pinned hole-pattern callout expands only the plan's right edge, recreating #109.
// For side views, a high depth-dimension lane expands the top band instead.
fn lopsided(view: &str, projection: &str, sheets: bool) -> Value {
    let mut value = json!({"sheet":{"size":"A3","scale":"1:1","projection":projection}});
    value["dimensions"] = if view == "plan" {
        json!({"d:f:hole_pattern:1/bore.diameter":{"place":{"view":"plan","side":"right","lane":4,"along":"middle","pinned":true}}})
    } else {
        json!({"d:envelope/depth.length":{"place":{"view":"side","side":"above","lane":4,"along":"middle","pinned":true}}})
    };
    if sheets {
        value["sheets"] = json!({"max":1});
    }
    value
}

fn projection_gaps(view: &str) {
    let a = plate();
    let axis = usize::from(view == "side");
    for projection in ["first", "third"] {
        for sheets in [false, true] {
            let value = lopsided(view, projection, sheets);
            let before = planned(&a, value.clone());
            let v = before
                .views
                .iter()
                .find(|v| v.id == format!("v:{view}"))
                .unwrap();
            assert!(
                (0.5 * (v.area[axis] + v.area[axis + 2]) - v.at[axis]).abs() > 1.,
                "{view} must have asymmetric bands: {:?}",
                v.area
            );
            for reverse in [false, true] {
                for gap in [0., 5., 15., 30.] {
                    let side = match (view, projection, reverse) {
                        ("plan", "third", false) | ("plan", "first", true) => "above",
                        ("plan", _, _) => "below",
                        (_, "third", false) | (_, "first", true) => "right",
                        _ => "left",
                    };
                    let (name, anchor) = if reverse {
                        ("front", view)
                    } else {
                        (view, "front")
                    };
                    let mut spec = value.clone();
                    spec["views"] = json!({name:{"place":{"relative_to":anchor,"side":side,"gap":gap,"align":"projection","pinned":true}}});
                    let p = planned(&a, spec);
                    let front = p.views.iter().find(|v| v.id == "v:front").unwrap();
                    let partner = p
                        .views
                        .iter()
                        .find(|v| v.id == format!("v:{view}"))
                        .unwrap();
                    assert!(
                        (partner.at[axis] - front.at[axis]).abs() < 1e-6,
                        "{view} {projection} reverse={reverse} gap={gap}: {:?} vs {:?}",
                        partner.at,
                        front.at
                    );
                    let measured = if projection == "third" {
                        partner.area[1 - axis] - front.area[3 - axis]
                    } else {
                        front.area[1 - axis] - partner.area[3 - axis]
                    };
                    assert!(
                        (measured - gap).abs() < 1e-6,
                        "{view} {projection} reverse={reverse}: gap {measured} != {gap}"
                    );
                    assert!(
                        !p.problems
                            .iter()
                            .any(|p| p.code == "projection_relation_infeasible"),
                        "{:?}",
                        p.problems
                    );
                    if view == "plan" && projection == "third" && !sheets && !reverse && gap == 15.
                    {
                        eprintln!(
                            "#109 plan at {:?}, area {:?}; front at {:?}, area {:?}; gap {measured}",
                            partner.at, partner.area, front.at, front.area
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn projection_gap_keeps_lopsided_plan_on_front_in_both_angles_and_directions() {
    projection_gaps("plan");
}

#[test]
fn projection_gap_keeps_lopsided_side_on_front_in_both_angles_and_directions() {
    projection_gaps("side");
}

#[test]
fn projection_is_not_an_annotation_slot() {
    assert!(
        serde_json::from_value::<Spec>(
            json!({"dimensions":{"d:envelope/width.length":{"place":{"along":"projection"}}}})
        )
        .is_err()
    );
}
