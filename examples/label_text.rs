//! Reproduce the before/after label-text drafting review, including embedded saved Specs.
use draftwright::model::{self, Spec};
use serde_json::json;
use std::path::{Path, PathBuf};

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/label-text-renders"));
    std::fs::create_dir_all(&out).unwrap();
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    for name in ["plate", "turned", "prismatic"] {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/parts/{name}.step"));
        let (part, a) = model::analyse_native(&path).unwrap();
        let base = model::plan_recovered(&a, &Spec::default()).unwrap();
        let target = base
            .dimensions
            .iter()
            .find(|d| {
                d.placed.is_some()
                    && !d.reference
                    && if name == "turned" {
                        d.kind == "callout"
                    } else {
                        d.id == "d:envelope/width.length"
                    }
            })
            .unwrap();
        let target_id = target.id.clone();
        let edit = match name {
            "plate" => json!({"text":{"prefix":"2×","suffix":"TYP"},"style":"reference"}),
            "turned" => json!({"text":{"template":"{value} AFTER PLATING"}}),
            _ => json!({"text":{"suffix":"(NOM)"},"style":"basic"}),
        };
        let spec: Spec = serde_json::from_value(json!({"dimensions":{&target.id:edit}})).unwrap();
        let after = model::plan_recovered(&a, &spec).unwrap();
        for (version, plan) in [("before", base), ("after", after)] {
            let drawing = model::render(&part, &a, &plan, &font).unwrap();
            let stem = out.join(format!("{name}-{version}"));
            std::fs::write(stem.with_extension("png"), drawing.png(150.0).unwrap()).unwrap();
            std::fs::write(stem.with_extension("svg"), drawing.svg()).unwrap();
            std::fs::write(
                stem.with_extension("plan.json"),
                serde_json::to_vec_pretty(&plan).unwrap(),
            )
            .unwrap();
            std::fs::write(
                stem.with_extension("spec.json"),
                serde_json::to_vec_pretty(&plan.spec).unwrap(),
            )
            .unwrap();
            std::fs::write(
                stem.with_extension("lint.json"),
                serde_json::to_vec_pretty(&draftwright::lint::lint(&a, &drawing)).unwrap(),
            )
            .unwrap();
            let d = plan.dimensions.iter().find(|d| d.id == target_id);
            if let Some(d) = d.and_then(|d| d.placed.as_ref()) {
                let b = d.label_box;
                std::fs::write(
                    out.join(format!("{name}-{version}-label.png")),
                    drawing
                        .png_region(
                            250.0,
                            Some([
                                (b[0] - 6.0).max(0.0),
                                (b[1] - 6.0).max(0.0),
                                (b[2] + 6.0).min(drawing.page[0]),
                                (b[3] + 6.0).min(drawing.page[1]),
                            ]),
                        )
                        .unwrap(),
                )
                .unwrap();
            }
            println!("{}", stem.with_extension("png").display());
        }
    }
}
