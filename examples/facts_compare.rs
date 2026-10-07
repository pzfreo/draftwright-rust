//! The port's plan of each captured part against draftwright's sheet, fact by fact, for what the
//! plan holds today (the sheet, the scale, the views and where they sit):
//! `cargo run --release --example facts_compare -- <corpus> [name ...]`.
//!
//! A report, not a test: `tests/views.rs` holds the same comparison to its known differences.
//! A view's position is compared like for like, as where each side puts the centre of the part's
//! envelope on the page.

#[path = "../tests/common/facts.rs"]
mod facts;

use draftwright::model::{self, Spec};
use haecceity::hlr::View;

fn same(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(corpus) = args.first().map(std::path::PathBuf::from) else {
        eprintln!("usage: facts_compare <corpus> [name ...]");
        std::process::exit(2);
    };
    let names = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        facts::names()
    };
    let (mut agree, mut differ, mut failed) = (0, 0, 0);
    let mut tally = |ok: bool| {
        if ok {
            agree += 1
        } else {
            differ += 1
        }
        if ok { "same" } else { "DIFFERS" }
    };
    for name in &names {
        let f = facts::load(name);
        let path = corpus.join(&f.binding.file);
        let (analysis, plan) =
            match model::analyse(&path)
                .map_err(|e| e.to_string())
                .and_then(|(_, a)| {
                    model::plan(&a, &Spec::default())
                        .map(|p| (a, p))
                        .map_err(|e| e.to_string())
                }) {
                Ok(r) => r,
                Err(e) => {
                    println!("{name}: the port failed: {e}");
                    failed += 1;
                    continue;
                }
            };
        println!("{name} ({})", f.binding.file);
        if analysis.binding.source_sha256 != f.binding.sha256 {
            println!("  file: DIFFERS from the one captured (sha256); skipped");
            failed += 1;
            continue;
        }
        let ours = &plan.sheet;
        println!(
            "  sheet: {} {}x{} vs {} {}x{}: {}",
            f.sheet.format,
            f.sheet.page[0],
            f.sheet.page[1],
            ours.size,
            ours.page[0],
            ours.page[1],
            tally(f.sheet.format == ours.size && f.sheet.page == ours.page)
        );
        println!(
            "  scale: {} vs {}: {}",
            f.sheet.scale,
            ours.scale,
            tally(f.sheet.scale == ours.scale)
        );
        let theirs: Vec<&str> = f.views.iter().map(|v| v.name.as_str()).collect();
        let mine: Vec<&str> = plan
            .views
            .iter()
            .map(|v| v.id.trim_start_matches("v:"))
            .collect();
        println!(
            "  views: [{}] vs [{}]: {}",
            theirs.join(", "),
            mine.join(", "),
            tally(theirs == mine)
        );
        // The envelope's centre, which the port centres each view on.
        let [lo, hi] = analysis.envelope;
        let centre = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
        for v in &plan.views {
            let id = v.id.trim_start_matches("v:");
            let Some(t) = f.views.iter().find(|t| t.name == id) else {
                println!("  {id}: not on draftwright's sheet");
                continue;
            };
            let view = View::new(v.direction, v.up).expect("a planned view");
            let camera = match (t.direction, t.x_axis, t.up) {
                (Some(d), Some(x), Some(y)) => {
                    same(d, view.toward) && same(x, view.x) && same(y, view.y)
                }
                _ => false,
            };
            let at = t.page(centre);
            let (scale, factor) = (t.scale.unwrap_or(f64::NAN), v.scale);
            match at {
                Some(at) => {
                    let (dx, dy) = (v.at[0] - at[0], v.at[1] - at[1]);
                    println!(
                        "  {id}: camera {}; scale {scale} vs {factor}; centre ({:.1}, {:.1}) vs ({:.1}, {:.1}), off by ({dx:+.1}, {dy:+.1}): {}",
                        if camera { "same" } else { "DIFFERS" },
                        at[0],
                        at[1],
                        v.at[0],
                        v.at[1],
                        tally(camera && (scale - factor).abs() < 1e-9 && dx.hypot(dy) < 0.5)
                    );
                }
                None => println!("  {id}: draftwright recorded no camera"),
            }
        }
        for t in &f.views {
            if !plan
                .views
                .iter()
                .any(|v| v.id.trim_start_matches("v:") == t.name)
            {
                println!(
                    "  {}: {} view not planned by the port",
                    t.name,
                    t.kind.as_deref().unwrap_or("unknown")
                );
            }
        }
        let measuring = f
            .annotations
            .iter()
            .filter(|a| !a.measurements.is_empty())
            .count();
        println!(
            "  annotations: draftwright {} measuring, {} lint findings; the port plans none yet",
            measuring,
            f.lint.len()
        );
    }
    println!("{agree} facts the same, {differ} different, {failed} parts not compared");
}
