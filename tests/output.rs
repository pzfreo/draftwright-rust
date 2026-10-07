//! The port's SVG and DXF against draftwright's, written from the same drawings
//! (`tools/capture_output.py`): every layer must carry the same style and the same lines, each
//! side's within `common::TOL` of the other's. (The PDF, rendered and searched, is compared by
//! `tools/compare_pdf.py`, which needs a PDF renderer.)

#[path = "common/output.rs"]
mod common;

#[test]
fn svg_and_dxf_match_draftwright() {
    let mut problems = Vec::new();
    for name in ["m10127", "string_post", "ctc01"] {
        let drawing = common::drawing(&common::load(name));
        for (kind, theirs, mine) in [
            (
                "svg",
                common::read_svg(
                    &String::from_utf8(common::gunzip(&format!("{name}.svg.gz"))).unwrap(),
                ),
                common::read_svg(&drawing.svg()),
            ),
            (
                "dxf",
                common::read_dxf(&common::gunzip(&format!("{name}.dxf.gz"))),
                common::read_dxf(&drawing.dxf()),
            ),
        ] {
            let layers: std::collections::BTreeSet<&String> =
                theirs.keys().chain(mine.keys()).collect();
            for layer in layers {
                let (Some(t), Some(m)) = (theirs.get(layer), mine.get(layer)) else {
                    problems.push(format!("{name} {kind} {layer}: on one side only"));
                    continue;
                };
                let (a, b) = (
                    common::covered(&t.lines, &m.lines),
                    common::covered(&m.lines, &t.lines),
                );
                if a < 0.9999 || b < 0.9999 || t.style != m.style {
                    problems.push(format!(
                        "{name} {kind} {layer}: {a:.4} {b:.4}, style {:?} vs {:?}",
                        t.style, m.style
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
