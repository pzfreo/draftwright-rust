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
    if let Some(measured) = basis.strip_prefix("whole-part measured fit ") {
        let pair = measured.split(';').next().unwrap();
        assert_eq!(pair, format!("{} at {}", plan.sheet.size, plan.sheet.scale));
        assert!(matches!(plan.sheet.size.as_str(), "A4" | "A3"));
        assert!(basis.contains(
            "complete principal and annotation blocks fit, zero readability failures, no new routes"
        ));
        assert!(plan.dimensions.iter().all(|d| d.placed.is_some()));
        assert!(!plan.problems.iter().any(|p| matches!(
            p.code.as_str(),
            "annotation_off_sheet" | "views_do_not_fit" | "page_fit_uncertain"
        )));
        if plan.sheet.size == "A3" {
            assert!(basis.contains(
                "A4: complete same-scale principal and annotation blocks do not fit cleanly"
            ));
        }
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    if let Some(seed_basis) = basis.strip_prefix("whole-part fitted seed ") {
        let pair = seed_basis.split(';').next().unwrap();
        assert_eq!(pair, format!("{} at {}", plan.sheet.size, plan.sheet.scale));
        assert!(
            basis.contains(
                "general readability: 0 text/arrow fit failures, 0 dimension-lane failures"
            )
        );
        assert!(basis.contains("physical bounds and measurements retained"));
        assert!(basis.contains("rejected search:"));
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    if let Some(header) = basis.strip_prefix("whole-part paper ") {
        let (pair, _) = header.split_once(';').unwrap();
        let (paper, scale) = pair.split_once(" at ").unwrap();
        let (ordinary, details) = basis
            .split_once("; whole-part trials: ")
            .unwrap()
            .1
            .split_once("; local-detail trials: ")
            .unwrap();
        let enlarged = basis.contains("; local readability recovered by enlarged detail;");
        let a3_exception =
            basis.contains("; local readability A3 exception after failed A4 details;");
        let mut base = plan.clone();
        base.sheet.size = paper.into();
        base.sheet.scale = scale.into();
        base.sheet.basis = ordinary.into();
        assert_follows(name, &base, seed);
        let partial = basis.contains(
            "local readability partially improved by enlarged detail; remaining needs refused",
        );
        if partial {
            assert_eq!(plan.sheet.size, paper);
            assert!(plan.sheet.scale_factor >= factor(scale));
            assert!(
                plan.problems
                    .iter()
                    .any(|q| q.code == "layout_recovery_exhausted" && q.severity == "warning")
            );
            let crops: Vec<_> = plan
                .views
                .iter()
                .filter(|v| v.detail.as_ref().is_some_and(|d| !d.authored))
                .collect();
            assert!(!crops.is_empty() && crops.len() <= 2);
            for crop in crops {
                assert!(
                    plan.dimensions
                        .iter()
                        .any(|d| d.view == crop.id && d.placed.is_some() && !d.reference)
                );
            }
        } else if enlarged || a3_exception {
            if a3_exception {
                assert_eq!(plan.sheet.size, "A3");
            } else {
                assert!(plan.views.iter().any(|v| {
                    v.detail
                        .as_ref()
                        .is_some_and(|d| !d.authored && d.scale > plan.sheet.scale_factor)
                }));
            }
            assert!(
                sheet_index(&plan.sheet.size) <= sheet_index(paper)
                    || (paper == "A4" && plan.sheet.size == "A3")
            );
            assert!(
                !plan
                    .problems
                    .iter()
                    .any(|q| q.code == "layout_recovery_exhausted")
            );
            assert!(details.contains("0 unreadable step spans, 0 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures"));
        } else {
            assert_eq!(
                (&plan.sheet.size, &plan.sheet.scale),
                (&paper.to_string(), &scale.to_string())
            );
            assert!(
                plan.problems
                    .iter()
                    .any(|q| q.code == "layout_recovery_exhausted")
            );
        }
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    if basis.starts_with("screen-first fitted-reference retention on ") {
        assert!(basis.contains("established clean readable drawing retained"));
        assert!(basis.contains("retained readability [0 unreadable step spans, 0 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures]"));
        assert!(
            !plan
                .problems
                .iter()
                .any(|p| p.code == "layout_recovery_exhausted" && p.severity == "warning")
        );
        if plan.spec.sheet.size.is_none() {
            for paper in draftwright::layout::PAGES
                .iter()
                .take(sheet_index(&plan.sheet.size))
            {
                assert!(
                    basis.contains(&format!("{} multi-sheet:", paper.0)),
                    "{name}: {basis}"
                );
            }
        }
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    if basis.starts_with("readability refusal on ") {
        assert!(basis.contains("established fitted geometry retained"));
        assert!(
            plan.problems
                .iter()
                .any(|p| p.code == "layout_recovery_exhausted" && p.severity == "warning")
        );
        assert!(
            tried(basis).iter().all(|t| !t.valid || t.over > 0),
            "{name}: a qualified candidate was refused: {basis}"
        );
        if plan.spec.sheet.size.is_none() {
            for paper in draftwright::layout::PAGES
                .iter()
                .take(sheet_index(&plan.sheet.size))
            {
                assert!(
                    basis.contains(&format!("{} multi-sheet:", paper.0)),
                    "{name}: {basis}"
                );
            }
        }
        return (plan.sheet.size.clone(), plan.sheet.scale.clone());
    }
    if basis.starts_with("screen-first scheduled-row recovery:") {
        let trials = basis.rsplit_once("; schedule trials: ").unwrap().1;
        let completed: Vec<_> = trials
            .split("; ")
            .filter(|t| t.contains(" scheduled ") && t.contains(", valid "))
            .collect();
        let valid = completed
            .iter()
            .find(|t| t.contains(", valid true,"))
            .unwrap();
        assert!(valid.contains("installed true"));
        assert!(valid.contains("hard: 0, dropped: 0"));
        assert!(valid.contains("retained true, bounds fit true, no new ink true"));
        assert!(valid.contains("readability [0 unreadable step spans, 0 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures]"));
        let pair = |t: &str| {
            let words: Vec<_> = t.split_whitespace().collect();
            (
                words[0].to_owned(),
                words[2].trim_end_matches(':').to_owned(),
            )
        };
        let chosen = pair(valid);
        assert_eq!(
            (&plan.sheet.size, &plan.sheet.scale),
            (&chosen.0, &chosen.1)
        );
        for earlier in completed
            .iter()
            .take_while(|t| !t.contains(", valid true,"))
        {
            let other = pair(earlier);
            assert!(
                sheet_index(&other.0) < sheet_index(&chosen.0)
                    || (other.0 == chosen.0 && factor(&other.1) > factor(&chosen.1))
            );
        }
        for paper in draftwright::layout::PAGES
            .iter()
            .take(sheet_index(&chosen.0))
        {
            assert!(basis.contains(&format!("{} multi-sheet:", paper.0)));
        }
        return chosen;
    }
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
