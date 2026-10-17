//! Reproduce #76 item 4's before/after drawings.
//! cargo run --release --example dimension_schemes -- target/dimension-schemes
use draftwright::{
    api,
    model::{self, Spec},
    text::Font,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "target/dimension-schemes".into());
    fs::create_dir_all(&out)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts");
    let font = Font::new(api::MONO.to_vec()).unwrap();
    for (name, file) in [
        ("holes-chain", "mounting_plate"),
        ("turned-baseline", "turned"),
        ("pattern-ordinate", "two_grids"),
    ] {
        let (part, a) = model::analyse_native(&root.join(format!("{file}.step")))
            .map_err(|e| format!("{e:?}"))?;
        let intent = match file {
            "turned" => {
                let band = draftwright::lint::turned::band(
                    &draftwright::bridge::holes::frame(&a),
                    a.features.iter().find(|f| f.kind == "turned_step").unwrap(),
                )?;
                json!({"scheme":"baseline","axis":(["x","y","z"][band.axis]),"view":if band.axis==1 {"side"} else {"front"},"from":"max"})
            }
            "two_grids" => {
                json!({"scheme":"ordinate","axis":"x","view":"plan","targets":[a.recognition.as_ref().unwrap().patterns[0].id]})
            }
            _ => json!({"scheme":"chain","axis":"x","view":"plan"}),
        };
        let s: Spec = serde_json::from_value(json!({"dimension_schemes":{"inspection":intent}}))?;
        for (state, spec) in [("before", Spec::default()), ("after", s)] {
            let p = model::plan_recovered(&a, &spec).map_err(|e| format!("{name}: {e:?}"))?;
            let drawings = model::render_sheets(&part, &a, &p, &font)?;
            let stem = out.join(format!("{name}-{state}"));
            for (i, drawing) in drawings.iter().enumerate() {
                let path = if i == 0 {
                    stem.clone()
                } else {
                    out.join(format!("{name}-{state}-sheet{}", i + 1))
                };
                fs::write(path.with_extension("png"), drawing.png(120.)?)?;
                fs::write(path.with_extension("svg"), drawing.svg())?;
            }
            fs::write(
                stem.with_extension("plan.json"),
                serde_json::to_vec_pretty(&p)?,
            )?;
            fs::write(
                stem.with_extension("spec.json"),
                serde_json::to_vec_pretty(&p.spec)?,
            )?;
            let mut issues = draftwright::lint::lint_sheets(&a, &drawings);
            for drawing in &drawings {
                issues.extend(draftwright::lint::structural::check(drawing));
            }
            fs::write(
                stem.with_extension("lint.json"),
                serde_json::to_vec_pretty(&issues)?,
            )?;
            println!(
                "{}: {} at {}, {} sheets, {} findings",
                stem.display(),
                p.sheet.size,
                p.sheet.scale,
                drawings.len(),
                issues.len()
            );
        }
    }
    Ok(())
}
