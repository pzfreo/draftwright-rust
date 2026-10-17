//! Independent check of the screen-first paper rule (2026-10-17).
//! Read the actual candidate history; never call the selection comparator.

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
        let Some(pct) = tail.find('%') else { break };
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

/// Smallest clean readable paper then largest standard scale, with the
/// existing material-crossing ratchet. Multi-sheet choices must record R5 and
/// precede larger paper; their individual sheets are checked by sheets tests.
pub fn assert_follows(name: &str, plan: &Plan, seed: (&str, &str)) -> (String, String) {
    let basis = &plan.sheet.basis;
    if basis.starts_with("screen-first paper policy:") {
        assert!(
            basis.contains("R5 allocation before larger paper"),
            "{name}: {basis}"
        );
        assert!(basis.contains("readability [0 unreadable step spans, 0 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures]"), "{name}: {basis}");
        for paper in draftwright::layout::PAGES
            .iter()
            .take(sheet_index(&plan.sheet.size))
        {
            assert!(
                basis.contains(&format!("{} multi-sheet:", paper.0)),
                "{name}: {basis}"
            );
        }
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    let all = tried(basis);
    let estimate = format!("first estimate {} at {} (", seed.0, seed.1);
    let said = basis
        .split_once(&estimate)
        .unwrap_or_else(|| panic!("{name}: no {estimate}: {basis}"))
        .1;
    let seed_clean = said.starts_with("valid and complete)");
    let valid: Vec<_> = all.iter().filter(|t| t.valid && t.over == 0).collect();
    let smallest = valid
        .iter()
        .min_by(|a, b| {
            sheet_index(&a.size)
                .cmp(&sheet_index(&b.size))
                .then(factor(&b.scale).total_cmp(&factor(&a.scale)))
        })
        .expect("valid candidate");
    assert!(
        basis.starts_with(&format!(
            "smallest fit {} at {};",
            smallest.size, smallest.scale
        )),
        "{name}: {basis}"
    );
    let best = valid
        .iter()
        .filter(|t| !seed_clean || t.crossings == 0)
        .min_by(|a, b| {
            sheet_index(&a.size)
                .cmp(&sheet_index(&b.size))
                .then(factor(&b.scale).total_cmp(&factor(&a.scale)))
        });
    if let Some(best) = best {
        assert_eq!(
            (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
            (best.size.as_str(), best.scale.as_str()),
            "{name}: {basis}"
        );
    } else {
        assert_eq!(
            (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
            seed,
            "{name}: crossing ratchet: {basis}"
        );
    }
    if plan.spec.sheet.size.is_none() {
        for paper in draftwright::layout::PAGES
            .iter()
            .take(sheet_index(&plan.sheet.size))
        {
            assert!(
                basis.contains(&format!("{} multi-sheet:", paper.0))
                    || plan.spec.sheets.as_ref().and_then(|s| s.max) == Some(1),
                "{name}: smaller paper must exhaust allocation: {basis}"
            );
        }
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
