//! The annotation bands the port reserves from a part's features (`src/demand.rs`), against the
//! bands draftwright reserved for the same part: its first placement's strip depths and step
//! count (the first `_layout_geometry` call recorded in `tests/fixtures/layout/<name>.json.gz`,
//! with all three principal views), and its step levels (the count its first scale choice is
//! seeded with). Every difference is listed, with a verdict and the reason,
//! in `tests/fixtures/known_bands.json`; a listed difference that has gone away must come off it.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::BTreeSet;
use std::io::Read as _;
use std::path::Path;

use draftwright::demand;
use draftwright::layout::{Principals, Request};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    /// A strip field, `step_levels`, `n_steps` or `section`.
    fact: String,
    /// `rust-correct`, `rust-wrong`, `equivalent` or `undetermined`.
    verdict: String,
    reason: String,
}

const FIELDS: [&str; 11] = [
    "right",
    "left",
    "top",
    "pv_halo",
    "fv_top",
    "fv_bottom",
    "front_hole_below",
    "pv_bottom",
    "sv_top",
    "sv_bottom",
    "sv_right",
];

fn calls(name: &str) -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/layout")
        .join(format!("{name}.json.gz"));
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    v["calls"].as_array().unwrap().clone()
}

#[test]
fn bands_match_draftwright_or_are_known() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let known: Vec<Known> = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/known_bands.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for k in &known {
        assert!(
            ["rust-correct", "rust-wrong", "equivalent", "undetermined"]
                .contains(&k.verdict.as_str())
                && !k.reason.is_empty(),
            "{k:?}"
        );
    }
    let listed: BTreeSet<(String, String)> = known
        .iter()
        .map(|k| (k.part.clone(), k.fact.clone()))
        .collect();
    assert_eq!(listed.len(), known.len(), "a difference is listed twice");

    let mut differ = BTreeSet::new();
    let mut agree = 0;
    for name in facts::names() {
        let f = facts::load(&name);
        let (_, analysis) = corpus::analyse(&dir, &f.binding.file, &name);
        let d = demand::read(&analysis);
        let [lo, hi] = analysis.envelope;
        let mut request = Request::new(
            [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]],
            Principals::ALL,
            true,
        );
        request.sections = usize::from(d.section.is_some());
        let (_, strips, n_steps) = demand::size(&d, &request, None, None, lo[2]);
        let calls = calls(&name);
        // Draftwright seeds its first scale choice with every step level, then places the
        // views with the count legible at the scale it settles on.
        let seed = calls[0]["n_steps"].as_u64().unwrap() as usize;
        let theirs = calls
            .iter()
            .find(|c| c["call"] == "layout_geometry" && c["views"].is_null())
            .unwrap_or_else(|| panic!("{name}: no first placement"));
        let ours = serde_json::json!({
            "right": strips.right, "left": strips.left, "top": strips.top,
            "pv_halo": strips.pv_halo, "fv_top": strips.fv_top, "fv_bottom": strips.fv_bottom,
            "front_hole_below": strips.front_hole_below, "pv_bottom": strips.pv_bottom,
            "sv_top": strips.sv_top, "sv_bottom": strips.sv_bottom, "sv_right": strips.sv_right,
        });
        let mut check = |fact: &str, same: bool, detail: String| {
            if same {
                agree += 1;
            } else {
                eprintln!("{name} {fact}: {detail}");
                differ.insert((name.clone(), fact.to_string()));
            }
        };
        for field in FIELDS {
            let (a, b) = (
                ours[field].as_f64().unwrap(),
                theirs["strips"][field].as_f64().unwrap(),
            );
            check(
                field,
                (a - b).abs() < 1e-6,
                format!("ours {a}, draftwright {b}"),
            );
        }
        check(
            "step_levels",
            d.step_zs.len() == seed,
            format!("ours {:?}, draftwright {seed} levels", d.step_zs),
        );
        let theirs_n = theirs["n_steps"].as_u64().unwrap() as usize;
        check(
            "n_steps",
            n_steps == theirs_n,
            format!("ours {n_steps} of {:?}, draftwright {theirs_n}", d.step_zs),
        );
        let theirs_s = theirs["section"].as_u64().unwrap() as usize;
        check(
            "section",
            request.sections == theirs_s,
            format!("ours {}, draftwright {theirs_s}", request.sections),
        );
    }
    let unlisted: Vec<_> = differ.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&differ).collect();
    eprintln!(
        "{agree} band facts agree, {} differ ({} known)",
        differ.len(),
        known.len()
    );
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "differences not in known_bands.json: {unlisted:#?}\nlisted but no longer differing: {stale:#?}"
    );
}
