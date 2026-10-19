//! An independent oracle for the maintainer's sheet rule (2026-10-12), read from the candidates
//! a plan's sheet basis records. It does not call the planner's selection code.
//!
//! 1. The smallest fit is the smallest sheet with a valid complete candidate (no hard finding or
//!    loss, no more annotations over another view than the first estimate), at its largest such
//!    scale. 2. Only that pair and the next larger sheet at a larger scale are allowed. 3. Against
//!    a valid first estimate, an allowed pair adding material crossings is set aside; if every one
//!    does, the first estimate is kept. 4. In the fill band the larger scale wins, then the
//!    smaller sheet; otherwise the fullest valid pair wins.

#![allow(dead_code)]

use draftwright::model::Plan;

#[derive(Clone, Debug, PartialEq)]
pub struct Tried {
    pub size: String,
    pub scale: String,
    pub fill: f64,
    pub valid: bool,
    pub over: usize,
    pub crossings: usize,
}

pub fn sheet_index(size: &str) -> usize {
    draftwright::layout::PAGES
        .iter()
        .position(|p| p.0 == size)
        .unwrap_or_else(|| panic!("{size} is not a sheet"))
}

pub fn factor(scale: &str) -> f64 {
    let (a, b) = scale.split_once(':').unwrap();
    a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap()
}

/// Every finished candidate the basis lists (pairs refused before planning have no fill).
pub fn tried(basis: &str) -> Vec<Tried> {
    let mut rest = basis
        .split_once("candidates: ")
        .unwrap_or_else(|| panic!("no candidates: {basis}"))
        .1;
    let mut out = Vec::new();
    while let Some(i) = rest.find(" fill ") {
        let head: Vec<&str> = rest[..i].rsplitn(3, ' ').collect();
        let (scale, size) = (head[0], head[1]);
        let tail = &rest[i + 6..];
        let pct = tail.find('%').unwrap();
        let fill = tail[..pct].parse::<f64>().unwrap() / 100.0;
        let open = tail.find('(').unwrap();
        let close = open + tail[open..].find(')').unwrap();
        let inside = &tail[open + 1..close];
        let count = |what: &str| {
            inside
                .split(", ")
                .find_map(|p| p.strip_suffix(what))
                .map(|n| n.trim().parse::<usize>().unwrap())
                .unwrap_or_else(|| panic!("{what} missing: {inside}"))
        };
        out.push(Tried {
            size: size.to_string(),
            scale: scale.to_string(),
            fill,
            valid: inside.starts_with("valid and complete"),
            over: count(" additional view conflicts"),
            crossings: count(" additional material crossings"),
        });
        rest = &tail[close..];
    }
    out
}

fn in_band(f: f64) -> bool {
    (draftwright::sheet_fit::FILL_BAND[0]..=draftwright::sheet_fit::FILL_BAND[1]).contains(&f)
}

/// Asserts *plan*'s automatic sheet follows the rule over its recorded candidates; *seed* is the
/// first estimate's (sheet, scale). Returns the smallest fit.
pub fn assert_follows(name: &str, plan: &Plan, seed: (&str, &str)) -> (String, String) {
    if plan
        .sheet_candidates
        .last()
        .is_some_and(|c| c.outcome.starts_with(&format!("{}: clean at ", c.size)))
    {
        let expected = if plan.spec.sheet.preferred_printer.as_deref() == Some("A3") {
            [("A4", 1), ("A3", 1), ("A3", 9)]
        } else {
            [("A4", 1), ("A4", 2), ("A3", 1)]
        };
        assert!(plan.sheet_candidates.len() <= expected.len());
        for (candidate, request) in plan.sheet_candidates.iter().zip(expected) {
            assert_eq!(
                (candidate.size.as_str(), candidate.max),
                request,
                "{name}: trial order"
            );
            assert!(
                candidate.scales.len() <= 2,
                "{name}: unbounded scale search"
            );
            if candidate.scales.len() == 2 {
                let indices: Vec<_> = candidate
                    .scales
                    .iter()
                    .map(|scale| {
                        let (numerator, denominator) = scale.split_once(':').unwrap();
                        let factor =
                            numerator.parse::<f64>().unwrap() / denominator.parse::<f64>().unwrap();
                        draftwright::layout::SCALES
                            .iter()
                            .position(|s| *s == factor)
                            .unwrap()
                    })
                    .collect();
                assert_eq!(
                    indices[1],
                    indices[0] + 1,
                    "{name}: retry must be the next smaller standard scale"
                );
            }
        }
        let winner = plan.sheet_candidates.last().unwrap();
        assert!(
            plan.sheet_candidates[..plan.sheet_candidates.len() - 1]
                .iter()
                .all(|c| !c.outcome.starts_with(&format!("{}: clean at ", c.size)))
        );
        assert_eq!(winner.size, plan.sheet.size, "{name}: paper report");
        assert_eq!(
            winner.scales.last(),
            Some(&plan.sheet.scale),
            "{name}: scale report"
        );
        assert!(plan.sheets.len() < winner.max, "{name}: sheet count");
        assert!(plan.sheet.default);
        assert!(
            plan.sheet
                .basis
                .contains(&format!("sheet choice: {}", winner.outcome))
        );
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    let basis = &plan.sheet.basis;
    let all = tried(basis);
    let estimate = format!("first estimate {} at {} (", seed.0, seed.1);
    let said = basis
        .split_once(&estimate)
        .unwrap_or_else(|| panic!("{name}: no {estimate}: {basis}"))
        .1;
    let seed_clean = said.starts_with("valid and complete)");
    let valid: Vec<&Tried> = all.iter().filter(|t| t.valid && t.over == 0).collect();
    assert!(!valid.is_empty(), "{name}: no valid candidate: {basis}");
    let smallest = valid
        .iter()
        .min_by(|a, b| {
            sheet_index(&a.size)
                .cmp(&sheet_index(&b.size))
                .then(factor(&b.scale).total_cmp(&factor(&a.scale)))
        })
        .unwrap();
    assert!(
        basis.starts_with(&format!(
            "smallest fit {} at {};",
            smallest.size, smallest.scale
        )),
        "{name}: {basis}"
    );
    let pool: Vec<&Tried> = valid
        .iter()
        .copied()
        .filter(|t| {
            (t.size == smallest.size && t.scale == smallest.scale)
                || (sheet_index(&t.size) == sheet_index(&smallest.size) + 1
                    && factor(&t.scale) > factor(&smallest.scale))
        })
        .filter(|t| !seed_clean || t.crossings == 0)
        .collect();
    if pool.is_empty() {
        assert!(seed_clean, "{name}: no allowed pair: {basis}");
        assert_eq!(
            (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
            seed,
            "{name}: the valid first estimate is kept: {basis}"
        );
        assert!(basis.contains("which is kept"), "{name}: {basis}");
        return (smallest.size.clone(), smallest.scale.clone());
    }
    let chosen = pool
        .iter()
        .find(|t| t.size == plan.sheet.size && t.scale == plan.sheet.scale)
        .unwrap_or_else(|| {
            panic!(
                "{name}: {} {} is not an allowed pair: {basis}",
                plan.sheet.size, plan.sheet.scale
            )
        });
    // Fill is recorded to 0.1%: compare with that resolution.
    let band: Vec<&&Tried> = pool.iter().filter(|t| in_band(t.fill)).collect();
    if band.is_empty() {
        let fullest = pool.iter().map(|t| t.fill).fold(0.0, f64::max);
        assert!(
            chosen.fill >= fullest - 1e-9,
            "{name}: below the band {} {} ({:.1}%) is not the fullest ({:.1}%): {basis}",
            chosen.size,
            chosen.scale,
            chosen.fill * 100.0,
            fullest * 100.0
        );
    } else {
        assert!(in_band(chosen.fill), "{name}: a band pair exists: {basis}");
        let best = band
            .iter()
            .map(|t| (factor(&t.scale), -(sheet_index(&t.size) as f64)))
            .fold((0.0, f64::NEG_INFINITY), |a, b| {
                if b.0 > a.0 || (b.0 == a.0 && b.1 > a.1) {
                    b
                } else {
                    a
                }
            });
        assert_eq!(
            (factor(&chosen.scale), -(sheet_index(&chosen.size) as f64)),
            best,
            "{name}: {basis}"
        );
    }
    (smallest.size.clone(), smallest.scale.clone())
}

/// The first estimate's (sheet, scale): from the `layout_replanned` finding, or the plan's own.
pub fn seed_of(plan: &Plan) -> (String, String) {
    plan.problems
        .iter()
        .find(|p| p.code == "layout_replanned")
        .map(|p| {
            let from = p.message.split_once("from ").unwrap().1;
            let words: Vec<&str> = from.split_whitespace().collect();
            (words[0].to_string(), words[2].to_string())
        })
        .unwrap_or_else(|| (plan.sheet.size.clone(), plan.sheet.scale.clone()))
}

/// [`assert_follows`] with the seed read from the plan.
pub fn assert_rule(name: &str, plan: &Plan) -> (String, String) {
    let seed = seed_of(plan);
    assert_follows(name, plan, (&seed.0, &seed.1))
}
