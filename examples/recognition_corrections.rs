//! Reproducible before/after review sheets for issue #76 item 2.
//! The pocket is a real native omission; the boss shows an authored interpretation.
//! The STEP geometry is never changed. `cargo run --release --example recognition_corrections`.
use draftwright::{
    api,
    model::{self, Analysis, Spec},
    recognition_corrections::face_fingerprint,
};
use serde_json::{Value, json};
use std::path::Path;
fn spec(v: Value) -> Spec {
    serde_json::from_value(v).unwrap()
}
fn pair(name: &str, part: &draftwright::haecceity::Part, a: &Analysis, s: &Spec, out: &Path) {
    let font = draftwright::text::Font::new(api::MONO.to_vec()).unwrap();
    for (suffix, s) in [("before", &Spec::default()), ("after", s)] {
        let p = model::plan_recovered(a, s).unwrap();
        let d = model::render(part, a, &p, &font).unwrap();
        let stem = out.join(format!("{name}-{suffix}"));
        std::fs::write(stem.with_extension("png"), d.png(110.).unwrap()).unwrap();
        std::fs::write(stem.with_extension("svg"), d.svg()).unwrap();
        std::fs::write(
            stem.with_extension("plan.json"),
            serde_json::to_vec_pretty(&p).unwrap(),
        )
        .unwrap();
        std::fs::write(
            stem.with_extension("spec.json"),
            serde_json::to_vec_pretty(&p.spec).unwrap(),
        )
        .unwrap();
        let effective = draftwright::spec_gdt::effective(a, &p.spec).unwrap();
        std::fs::write(
            stem.with_extension("lint.json"),
            serde_json::to_vec_pretty(&draftwright::lint::lint(&effective, &d)).unwrap(),
        )
        .unwrap();
        println!(
            "{}: {} at {}, {} dimensions",
            stem.display(),
            p.sheet.size,
            p.sheet.scale,
            p.dimensions.len()
        );
    }
}
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("target/recognition-corrections/renders");
    std::fs::create_dir_all(&out).unwrap();
    let (part, a) =
        model::analyse_native(&root.join("tests/fixtures/parts/mounting_plate.step")).unwrap();
    let pattern = &a.recognition.as_ref().unwrap().patterns[0];
    pair(
        "split-grid",
        &part,
        &a,
        &spec(json!({"features":{&pattern.id:{"split":true}}})),
        &out,
    );
    let (part, a) =
        model::analyse_native(&root.join("tests/fixtures/recognition/issues/pocket_z.step"))
            .unwrap();
    let faces: Vec<_> = a
        .face_attachments
        .iter()
        .enumerate()
        .filter(|(i, f)| f.surface.as_deref() == Some("plane") && !a.stock_faces.contains(i))
        .map(|(i, _)| face_fingerprint(&a, i).unwrap())
        .collect();
    pair(
        "missed-pocket",
        &part,
        &a,
        &spec(
            json!({"feature_declarations":[{"id":"f:declared:recess","kind":"pocket","faces":faces}]}),
        ),
        &out,
    );
    let (part, a) =
        model::analyse_native(&root.join("tests/fixtures/parts/boss_circle.step")).unwrap();
    let boss = a
        .features
        .iter()
        .find(|f| f.kind == "boss")
        .unwrap()
        .clone();
    pair(
        "boss-band",
        &part,
        &a,
        &spec(json!({"features":{&boss.id:{"reclassify":"turned_step"}}})),
        &out,
    );
}
