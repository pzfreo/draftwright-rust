//! Write each captured drawing (`tools/capture_output.py`) as the port writes it, beside
//! draftwright's, and compare the SVG and DXF layer by layer:
//! `cargo run --release --example output_compare -- [out-dir]`.

#[path = "../tests/common/output.rs"]
mod common;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or("target/output".into()));
    std::fs::create_dir_all(&out).unwrap();
    for name in ["m10127", "string_post", "ctc01"] {
        let drawing = common::drawing(&common::load(name));
        let (svg, dxf, pdf) = (drawing.svg(), drawing.dxf(), drawing.pdf(&common::fonts));
        std::fs::write(out.join(format!("{name}.svg")), &svg).unwrap();
        std::fs::write(out.join(format!("{name}.dxf")), &dxf).unwrap();
        std::fs::write(out.join(format!("{name}.pdf")), &pdf).unwrap();
        for (kind, theirs, mine) in [
            (
                "svg",
                common::read_svg(
                    &String::from_utf8(common::gunzip(&format!("{name}.svg.gz"))).unwrap(),
                ),
                common::read_svg(&svg),
            ),
            (
                "dxf",
                common::read_dxf(&common::gunzip(&format!("{name}.dxf.gz"))),
                common::read_dxf(&dxf),
            ),
        ] {
            for (layer, t) in &theirs {
                let Some(m) = mine.get(layer) else {
                    println!("{name} {kind} {layer}: missing ({} lines)", t.lines.len());
                    continue;
                };
                println!(
                    "{name} {kind} {layer}: theirs→ours {:.4} ours→theirs {:.4}; style {}",
                    common::covered(&t.lines, &m.lines),
                    common::covered(&m.lines, &t.lines),
                    if t.style == m.style {
                        "same".to_string()
                    } else {
                        format!("{:?} vs {:?}", t.style, m.style)
                    }
                );
            }
        }
        println!(
            "{name}: svg {} bytes, dxf {} bytes, pdf {} bytes",
            svg.len(),
            dxf.len(),
            pdf.len()
        );
    }
}
