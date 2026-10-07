//! The port's view planning against draftwright's sheets (`tests/fixtures/sheets/`), fact by
//! fact: the sheet, the scale, the view set, and for each view both draw, its scale and where it
//! puts the centre of the part's box. Every difference must be listed, with a verdict and the
//! reason, in `tests/fixtures/known_views.json`; a listed difference that has gone away must be
//! taken off the list.
//!
//! The parts are read from the corpus (`DRAFTWRIGHT_CORPUS`, by default
//! `~/repos/quiddity/tests/corpus`).
//!
//! Tolerances: a view's centre within 0.01 mm on the page, its scale within 1e-6 relative. Both
//! sides centre each view on the part's tight box, and the boxes agree to well under that (the
//! test checks it); the facts are rounded to 1e-6.

#[path = "common/facts.rs"]
mod facts;

use std::collections::BTreeSet;
use std::path::PathBuf;

use draftwright::model::{self, Spec};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    /// `sheet`, `scale`, `views`, `centre:<view>` or `scale:<view>`.
    fact: String,
    /// `rust-correct`, `equivalent` or `undetermined`.
    verdict: String,
    reason: String,
}

fn corpus() -> Option<PathBuf> {
    let dir = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })?;
    dir.is_dir().then_some(dir)
}

#[test]
fn view_planning_matches_draftwright_or_is_known() {
    let Some(corpus) = corpus() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let known: Vec<Known> = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/known_views.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for k in &known {
        assert!(
            ["rust-correct", "equivalent", "undetermined"].contains(&k.verdict.as_str())
                && !k.reason.is_empty(),
            "{k:?}"
        );
    }
    let listed: BTreeSet<(String, String)> = known
        .iter()
        .map(|k| (k.part.clone(), k.fact.clone()))
        .collect();
    assert_eq!(listed.len(), known.len(), "a difference is listed twice");

    let mut differ: BTreeSet<(String, String)> = BTreeSet::new();
    let mut agree = 0;
    for name in facts::names() {
        let f = facts::load(&name);
        let (_, analysis) = model::analyse(&corpus.join(&f.binding.file)).unwrap();
        assert_eq!(analysis.binding.source_sha256, f.binding.sha256, "{name}");
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        let mut check = |fact: String, same: bool| {
            if same {
                agree += 1;
            } else {
                differ.insert((name.clone(), fact));
            }
        };
        check("sheet".into(), f.sheet.format == plan.sheet.size);
        check("scale".into(), f.sheet.scale == plan.sheet.scale);
        let theirs: Vec<&str> = f.views.iter().map(|v| v.name.as_str()).collect();
        let ours: Vec<&str> = plan
            .views
            .iter()
            .map(|v| v.id.trim_start_matches("v:"))
            .collect();
        check("views".into(), theirs == ours);

        let [lo, hi] = analysis.envelope;
        let centre = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
        for v in &plan.views {
            let id = v.id.trim_start_matches("v:");
            let Some(t) = f.views.iter().find(|t| t.name == id) else {
                continue;
            };
            // Both centre the view on the part's box.
            let look_at = t.look_at.unwrap();
            let off = (0..3)
                .map(|k| (look_at[k] - centre[k]).abs())
                .fold(0.0, f64::max);
            assert!(
                off < 1e-3,
                "{name} {id}: the boxes' centres differ by {off} mm"
            );
            let at = t.page(centre).unwrap();
            check(
                format!("centre:{id}"),
                (v.at[0] - at[0]).hypot(v.at[1] - at[1]) < 0.01,
            );
            let scale = t.scale.unwrap();
            check(
                format!("scale:{id}"),
                (v.scale - scale).abs() <= 1e-6 * scale,
            );
        }
    }
    let unlisted: Vec<_> = differ.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&differ).collect();
    eprintln!(
        "{agree} facts agree, {} differ ({} known)",
        differ.len(),
        known.len()
    );
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known_views.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}

/// Third-angle packing keeps the projection alignment: the plan above the front sharing its x,
/// the side to its right sharing its y, and the isometric in free space; the reserved areas are
/// disjoint and inside the border, clear of the title block.
#[test]
fn views_keep_projection_alignment_and_their_areas_apart() {
    for name in ["turned.step", "prismatic.step"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts")
            .join(name);
        let (_, analysis) = model::analyse(&path).unwrap();
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        let view = |id: &str| plan.views.iter().find(|v| v.id == id).unwrap();
        let (front, top, side, iso) = (
            view("v:front"),
            view("v:plan"),
            view("v:side"),
            view("v:iso"),
        );
        assert_eq!(top.at[0], front.at[0], "{name}");
        assert!(top.at[1] > front.at[1], "{name}");
        assert_eq!(side.at[1], front.at[1], "{name}");
        assert!(side.at[0] > front.at[0], "{name}");
        let [w, h] = plan.sheet.page;
        let mut areas: Vec<[f64; 4]> = plan.views.iter().map(|v| v.area).collect();
        areas.push(plan.sheet.title_block);
        for (i, a) in areas.iter().enumerate() {
            assert!(a[0] >= 10.0 - 0.5 && a[1] >= 10.0 - 0.5, "{name}: {a:?}");
            assert!(
                a[2] <= w - 10.0 + 0.5 && a[3] <= h - 10.0 + 0.5,
                "{name}: {a:?}"
            );
            for b in &areas[i + 1..] {
                let apart = a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1];
                assert!(apart, "{name}: {a:?} meets {b:?}");
            }
        }
        assert!(iso.scale > 0.0 && plan.sheet.scale_factor > 0.0);
    }
}
