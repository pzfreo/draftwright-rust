//! Compare draftwright's text with build123d's (`tools/capture_text.py`), every score printed:
//! `cargo run --release --example text_compare -- tests/fixtures/text.json.gz`.

#[path = "../tests/common/text.rs"]
mod common;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("tests/fixtures/text.json.gz".into());
    for case in common::load(std::path::Path::new(&path))["cases"]
        .as_array()
        .unwrap()
    {
        let s = common::compare(case);
        println!(
            "{} {:?} {}: box {:.2e}, outline occ→rust {:.4} rust→occ {:.4}, area occ {:.5} rust {:.5} ({:+.1e}); faces {} contours {}",
            case["font"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
            case["size"],
            s.box_error,
            s.outline.0,
            s.outline.1,
            s.area.0,
            s.area.1,
            (s.area.1 - s.area.0) / s.area.0,
            s.faces,
            s.contours
        );
    }
}
