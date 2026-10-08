//! The port's SVG and DXF against draftwright's, written from the same drawings
//! (`tools/capture_output.py`): every layer must carry the same style and the same lines, each
//! side's within `common::TOL` of the other's. (The PDF, rendered and searched, is compared by
//! `tools/compare_pdf.py`, which needs a PDF renderer.)

#[path = "common/output.rs"]
mod common;
#[path = "common/corpus.rs"]
mod corpus;

use std::io::Read as _;

use draftwright::model::{self, Spec};

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

/// cgb242's front view had a hidden stretch of no span (an exact cubic from t = 0 to t = 0, at a
/// vertex several edges share), which the port left out and reported (`edges_not_drawn`, still
/// tested on its own in `output`). haecceity now passes over such stretches at source, so every
/// stretch is drawn, nothing is reported, and the sheet is drawn and reviewed.
#[test]
fn cgb242_is_drawn_whole() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(dir.join("cadgenbench_inputs/cgb242.step.gz")).unwrap(),
    )
    .read_to_end(&mut step)
    .unwrap();
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("cgb242.step");
    std::fs::write(&path, step).unwrap();

    let (part, analysis) = model::analyse(&path).unwrap();
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"),
        )
        .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let ids: Vec<&str> = drawing.problems.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.is_empty(), "{ids:?}");
    assert!(drawing.svg().contains("<path d="));
    assert!(!drawing.dxf().is_empty());
    assert!(!drawing.pdf(&common::fonts).is_empty());

    let review = draftwright::api::review(&path, None, &Spec::default()).unwrap();
    assert!(review.svg.contains("id=\"hidden\""));
}
