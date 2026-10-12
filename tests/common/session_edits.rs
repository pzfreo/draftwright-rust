//! S0 bench/edits from draftwright-editor origin/thin-slice, mapped to relative Spec ops.
use draftwright::items::{Op, key};
use draftwright::session::Session;
use serde_json::json;

pub fn edits(s: &Session) -> Vec<Vec<Op>> {
    let d = s
        .plan()
        .dimensions
        .iter()
        .find(|d| d.kind == "linear" && d.placed.is_some())
        .unwrap();
    let base = format!("dimensions/{}", key(&d.id));
    let view = s.plan().views.iter().find(|v| v.id != "v:front").unwrap();
    let scale = if s.plan().sheet.scale == "1:2" {
        "1:5"
    } else {
        "1:2"
    };
    let mut sequence = vec![
        vec![Op::set(
            format!("views/{}/place", view.id.trim_start_matches("v:")),
            json!({"relative_to":"front","side":"above","gap":15,"align":"middle"}),
        )],
        vec![Op::set(format!("{base}/place/along"), json!(0.75))],
        vec![Op::set(
            format!("{base}/place/side"),
            json!(if d.placed.as_ref().unwrap().rotation == 90.0 {
                "left"
            } else {
                "below"
            }),
        )],
        vec![Op::set(format!("{base}/omit"), json!(true))],
        vec![Op::set(format!("{base}/omit"), json!(false))],
        vec![Op::set(format!("{base}/tolerance"), json!("±0.1"))],
        vec![Op::set("sheet/scale", json!(scale))],
    ];
    // S0 deliberately leaves the turned profile's axial carrier in its profile view.
    if !s
        .analysis()
        .features
        .iter()
        .any(|f| f.kind == "turned_step")
        && s.plan()
            .dimensions
            .iter()
            .any(|d| d.id == "d:envelope/width.length")
    {
        sequence.push(vec![Op::set(
            "dimensions/d:envelope~1width.length/place/view",
            json!("plan"),
        )]);
    }
    sequence.push(vec![Op::set("views/side/omit", json!(true))]);
    sequence
}
