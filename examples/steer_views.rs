//! Reproduce the before/after drafting review: cargo run --release --example steer_views -- DIR
use draftwright::{
    api, lint,
    model::{self, Spec},
    text::Font,
};
use serde_json::{Value, json};
use std::path::Path;
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/view-setup-review".into());
    let out = Path::new(&out);
    std::fs::create_dir_all(out).unwrap();
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, file) in [
        ("front-frame", "parts/mounting_plate.step"),
        ("auxiliary", "recognition/steps/paired_ramp_step.step"),
        ("partial", "parts/mounting_plate.step"),
    ] {
        let (part, a) =
            model::analyse_native(&root.join(format!("tests/fixtures/{file}"))).unwrap();
        let mut changed = json!({"views":{}});
        match name {
            "front-frame" => {
                changed["views"]["front"] = json!({"orientation":{"from":"z","rotation":90},"principal_views":["front","plan","side"]})
            }
            "auxiliary" => {
                let (i, f) = a
                    .face_attachments
                    .iter()
                    .enumerate()
                    .find(|(_, f)| f.surface.as_deref() == Some("plane") && f.axis.is_none())
                    .unwrap();
                let key = draftwright::recognition_corrections::face_fingerprint(&a, i).unwrap();
                let normal = f.normal.unwrap();
                let source = [("front", 1), ("plan", 2), ("side", 0)]
                    .into_iter()
                    .find(|(_, axis)| normal[*axis].abs() < 1e-6)
                    .unwrap()
                    .0;
                changed["views"][source] = json!({});
                changed["views"]["detail_b"] = json!({"auxiliary":{"face":key,"from":source}});
            }
            _ => {
                changed["views"]["detail_b"] =
                    json!({"partial":{"from":"plan","around":"f:hole:1","scale_factor":2}})
            }
        }
        for (which, value) in [("before", json!({})), ("after", changed)] {
            let s: Spec = serde_json::from_value(value).unwrap();
            let p = model::plan_recovered(&a, &s).unwrap();
            let d = model::render(&part, &a, &p, &font).unwrap();
            let stem = out.join(format!("{name}-{which}"));
            std::fs::write(stem.with_extension("png"), d.png(120.).unwrap()).unwrap();
            std::fs::write(stem.with_extension("svg"), d.svg()).unwrap();
            std::fs::write(
                stem.with_extension("spec.json"),
                serde_json::to_vec_pretty(&p.spec).unwrap(),
            )
            .unwrap();
            let review: Value = json!({"sheet":p.sheet,"views":p.views,"problems":p.problems,"lint":lint::lint(&a,&d)});
            std::fs::write(
                stem.with_extension("review.json"),
                serde_json::to_vec_pretty(&review).unwrap(),
            )
            .unwrap();
            println!("{}", stem.with_extension("png").display());
        }
    }
}
