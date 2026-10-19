//! A4-first selection over the established planner. No candidate-specific recovery.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::{self, Analysis, Invalid, Plan, Spec};
use crate::{layout, sheet_fit};

/// Cheap evidence for one paper/count request; `max` is a bound, not an allocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub size: String,
    pub max: usize,
    pub outcome: String,
}

// Layout/readability failures only: overlaps and crossings, ink outside its frame/bands,
// unreadable text/scale, and content left undrawn for space. Recognition and engineering
// findings do not choose paper. Compare code + sorted subjects against the automatic baseline
// so findings shared by every paper do not penalise a smaller sheet. Spatial codes count even
// at info severity; only the informational recovery summary is not a spatial/readability failure.
const FAILING_CODES: &[&str] = &[
    "annotation_overlap",
    "annotation_ink_overlap",
    "datum_symbol_overlap",
    "annotation_leader_crossing",
    "leader_through_feature",
    "extension_through_feature",
    "annotation_off_sheet",
    "annotation_outside_area",
    "annotation_over_title_block",
    "annotation_over_view",
    "annotation_line_over_view",
    "title_field_overflow",
    "page_fit_uncertain",
    "views_do_not_fit",
    "views_overlap",
    "view_on_title_block",
    "turned_profile_superimposed",
    "turned_chain_no_clear_lane",
    "dimension_line_shared",
    "dimension_text_stray",
    "legibility_floor_breached",
    "layout_recovery_exhausted",
    "sheet_below_smallest_scale",
    "notes_not_placed",
    "table_not_placed",
    "table_dropped",
    "balloon_dropped",
    "callout_dropped",
    "location_ref_dropped",
    "off_axis_location_dropped",
    "section_dropped",
    "detail_unplaceable",
    "pmi_not_drawn",
    "view_plan_incomplete",
];

fn problem_key(p: &model::Problem) -> (String, Vec<String>) {
    let mut subjects = p.subjects.clone();
    subjects.sort();
    subjects.dedup();
    (p.code.clone(), subjects)
}

fn failures(analysis: &Analysis, plan: &Plan, floor: f64) -> BTreeSet<(String, Vec<String>)> {
    let mut failures: BTreeSet<_> = plan
        .problems
        .iter()
        .filter(|p| FAILING_CODES.contains(&p.code.as_str()))
        .filter(|p| p.code != "layout_recovery_exhausted" || p.severity != "info")
        .map(problem_key)
        .collect();
    // The outline box has no stroke width. At the frame its visible stroke is already
    // outside the drawing area. Use the renderer's line width, and the existing off-sheet
    // code/subject comparison; do not move or recover any candidate's geometry.
    let stroke = crate::output::layer_styles()[0].1.width;
    for (bounds, view) in sheet_fit::footprints(analysis, plan)
        .into_iter()
        .zip(&plan.views)
    {
        let page = plan
            .sheets
            .iter()
            .find(|s| s.number == view.sheet)
            .unwrap_or(&plan.sheet)
            .page;
        if bounds[0] < layout::MARGIN + stroke
            || bounds[1] < layout::MARGIN + stroke
            || bounds[2] > page[0] - layout::MARGIN - stroke
            || bounds[3] > page[1] - layout::MARGIN - stroke
        {
            failures.insert(("annotation_off_sheet".into(), vec![view.id.clone()]));
        }
    }
    if plan.sheet.scale_factor + 1e-9 < floor {
        failures.insert(("legibility_floor_breached".into(), vec!["sheet".into()]));
    }
    for view in plan.views.iter().filter(|v| v.id != "v:iso") {
        if view.scale + 1e-9 < floor {
            failures.insert(("legibility_floor_breached".into(), vec![view.id.clone()]));
        }
    }
    failures
}

/// Necessary outline-only fit, deliberately optimistic: no annotation bands or furniture
/// clearance. Principals share sheet 1; auxiliary views may move under R5. Ignore optional
/// details and the freely resized isometric. A refusal therefore cannot discard a real fit.
fn outlines_fit(analysis: &Analysis, baseline: &Plan, size: &str, max: usize, floor: f64) -> bool {
    let (_, w, h) = layout::PAGES.iter().find(|p| p.0 == size).unwrap();
    let available = [w - 2.0 * layout::MARGIN, h - 2.0 * layout::MARGIN];
    let mut principal = std::collections::BTreeMap::new();
    let mut area = 0.0;
    for v in &baseline.views {
        if v.id == "v:iso" || v.detail.is_some() {
            continue;
        }
        let view = crate::haecceity::hlr::View::new(v.direction, v.up).unwrap();
        let (outline, _) = model::extent(analysis, &view);
        let footprint = outline.map(|extent| extent * floor);
        if (0..2).any(|k| footprint[k] > available[k]) {
            return false;
        }
        area += footprint[0] * footprint[1];
        if ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()) {
            principal.insert(v.id.as_str(), footprint);
        }
    }
    // The existing minimum blank gutter, without the surrounding annotation corridors.
    const GUTTER: f64 = 6.0;
    if baseline.spec.views.values().all(|v| v.place.is_none())
        && let Some(front) = principal.get("v:front")
        && (principal
            .get("v:side")
            .is_some_and(|side| front[0] + side[0] + GUTTER > available[0])
            || principal
                .get("v:plan")
                .is_some_and(|plan| front[1] + plan[1] + GUTTER > available[1]))
    {
        return false;
    }
    let drawing_area = available[0] * available[1]
        - layout::title_block_width(*w) * crate::title_block::top([0.0; 4]);
    area <= drawing_area * max as f64
}

fn requests(printer: &str) -> [(&str, usize); 3] {
    if printer == "A3" {
        [("A4", 1), ("A3", 1), ("A3", 9)]
    } else {
        [("A4", 1), ("A4", 2), ("A3", 1)]
    }
}

pub(crate) fn select(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    let mut baseline = model::plan_automatic(analysis, spec)?;
    if spec.sheet.size.is_some()
        || spec.sheet.scale.is_some()
        || spec.sheets.as_ref().and_then(|s| s.max).is_some()
    {
        return Ok(baseline);
    }
    let spec = &baseline.spec.clone();
    let effective = crate::spec_gdt::effective(analysis, spec)?;
    let fit_analysis = effective.as_ref();
    let printer = spec.sheet.preferred_printer.as_deref().unwrap_or("A4");
    let floor =
        sheet_fit::min_legible_scale(fit_analysis).max(crate::sheets::floor(spec).unwrap_or(0.0));
    let mut baseline_failures = failures(fit_analysis, &baseline, floor);
    // Match every baseline problem by code + subject, including informational findings.
    baseline_failures.extend(baseline.problems.iter().map(problem_key));
    let mut candidates = Vec::new();
    for (size, max) in requests(printer) {
        // A baseline already as good in this preference order needs no forced replay.
        if baseline.sheet.size == size && baseline.sheets.len() < max {
            let reason = format!(
                "{size}: automatic baseline at {} ({} sheet(s))",
                baseline.sheet.scale,
                1 + baseline.sheets.len()
            );
            candidates.push(Candidate {
                size: size.into(),
                max,
                outcome: "automatic baseline reused".into(),
            });
            explain(&mut baseline, candidates, &reason);
            return Ok(baseline);
        }
        let outcome;
        if !outlines_fit(fit_analysis, &baseline, size, max, floor) {
            outcome = format!(
                "outline footprints cannot fit at {}",
                model::scale_name(floor)
            );
        } else {
            let mut forced = spec.clone();
            forced.sheet.size = Some(size.into());
            if max > 1 {
                forced.sheets.get_or_insert_with(Default::default).max = Some(max as u32);
            }
            match model::plan_automatic(analysis, &forced) {
                Ok(mut plan) => {
                    let codes: BTreeSet<_> = failures(fit_analysis, &plan, floor)
                        .difference(&baseline_failures)
                        .map(|(code, _)| code.clone())
                        .collect();
                    if codes.is_empty() {
                        let reason = format!(
                            "{size}: clean at {} ({} sheet(s))",
                            plan.sheet.scale,
                            1 + plan.sheets.len()
                        );
                        candidates.push(Candidate {
                            size: size.into(),
                            max,
                            outcome: reason.clone(),
                        });
                        plan.spec = spec.clone();
                        plan.sheet.default = true;
                        for sheet in &mut plan.sheets {
                            sheet.default = true;
                        }
                        explain(&mut plan, candidates, &reason);
                        return Ok(plan);
                    }
                    outcome = format!(
                        "layout/readability problems ({})",
                        codes.into_iter().collect::<Vec<_>>().join(", ")
                    );
                }
                Err(error) => outcome = format!("cannot plan: {}", error.message),
            }
        }
        candidates.push(Candidate {
            size: size.into(),
            max,
            outcome,
        });
    }
    let reason = format!(
        "{}: automatic baseline retained at {}",
        baseline.sheet.size, baseline.sheet.scale
    );
    explain(&mut baseline, candidates, &reason);
    Ok(baseline)
}

fn explain(plan: &mut Plan, candidates: Vec<Candidate>, reason: &str) {
    let attempts = candidates
        .iter()
        .map(|c| format!("{} up to {}: {}", c.size, c.max, c.outcome))
        .collect::<Vec<_>>()
        .join("; ");
    plan.sheet.basis = format!("{}; sheet choice: {reason}; {attempts}", plan.sheet.basis);
    plan.sheet_candidates = candidates;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn fixture() -> Analysis {
        model::analyse_native(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/prismatic.step"),
        )
        .unwrap()
        .1
    }

    fn corpus(file: &str) -> Option<Analysis> {
        let root = std::env::var_os("DRAFTWRIGHT_CORPUS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
                    .join("repos/quiddity/tests/corpus")
            });
        if !root.is_dir() {
            assert_ne!(
                std::env::var("DRAFTWRIGHT_CORPUS_REQUIRED").as_deref(),
                Ok("1")
            );
            return None;
        }
        Some(model::analyse_native(&root.join(file)).unwrap().1)
    }

    #[test]
    fn fits_a4_reuses_the_only_baseline() {
        let a = fixture();
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(p.sheet.size, "A4");
        assert_eq!(p.sheet_candidates.len(), 1);
        assert_eq!(p.sheet_candidates[0].outcome, "automatic baseline reused");
    }

    fn inspection_spec(printer: &str) -> Spec {
        let mut rows = vec![json!(["INSPECTION SCHEDULE", "CHECKED"])];
        rows.extend((0..6).map(|i| json!([format!("FORM AND SIZE {}", "X".repeat(30)), i])));
        serde_json::from_value(json!({"sheet":{"preferred_printer":printer}, "sheets":{"smallest_scale":"5:1"}, "hole_table":"never", "tables":[{"id":"t:1","rows":rows,"prefer":"tr"}]})).unwrap()
    }

    #[test]
    fn a4_printer_uses_two_a4_sheets() {
        let Some(a) = corpus("gramel/string_post.step") else {
            return;
        };
        let s = inspection_spec("A4");
        let p = model::plan_recovered(&a, &s).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheets.len()),
            ("A4", 1),
            "{}",
            p.sheet.basis
        );
        assert_eq!(p.sheet.scale, "5:1");
        assert_eq!(p.sheet_candidates.len(), 2);
        assert_eq!(p.spec.sheets.as_ref().unwrap().max, None);
        let mut forced = s.clone();
        forced.sheet.size = Some("A4".into());
        forced.sheets.as_mut().unwrap().max = Some(2);
        let q = model::plan_automatic(&a, &forced).unwrap();
        assert_eq!(
            serde_json::to_value(&p.views).unwrap(),
            serde_json::to_value(&q.views).unwrap()
        );
    }

    #[test]
    fn a3_printer_uses_one_a3_for_the_same_drawing() {
        let Some(a) = corpus("gramel/string_post.step") else {
            return;
        };
        let p = model::plan_recovered(&a, &inspection_spec("A3")).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheets.len()),
            ("A3", 0),
            "{}",
            p.sheet.basis
        );
        assert_eq!(p.sheet.scale, "5:1");
        assert_eq!(p.sheet_candidates.len(), 2);
    }

    #[test]
    fn needs_a3_and_stops_before_replaying_the_baseline() {
        let Some(a) = corpus("cadgenbench/threaded_connector_109.step") else {
            return;
        };
        let s: Spec = serde_json::from_value(json!({"sheet":{"preferred_printer":"A3"}})).unwrap();
        let p = model::plan_recovered(&a, &s).unwrap();
        assert_eq!(p.sheet.size, "A3", "{}", p.sheet.basis);
        assert_eq!(p.sheet_candidates.len(), 2);
        assert_eq!(p.sheet_candidates[1].outcome, "automatic baseline reused");
    }

    #[test]
    fn falls_back_to_automatic() {
        let Some(a) = corpus("nist/nist_ftc_09_asme1_rd.stp") else {
            return;
        };
        let b = model::plan_automatic(&a, &Spec::default()).unwrap();
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size, p.sheet.scale, p.sheets.len()),
            (b.sheet.size, b.sheet.scale, b.sheets.len())
        );
        assert_eq!(p.sheet_candidates.len(), 3);
        assert!(p.sheet.basis.contains("automatic baseline retained"));
    }

    fn assert_passthrough(value: serde_json::Value) {
        let a = fixture();
        let s: Spec = serde_json::from_value(value).unwrap();
        let b = model::plan_automatic(&a, &s).unwrap();
        let p = model::plan_recovered(&a, &s).unwrap();
        assert!(p.sheet_candidates.is_empty());
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }

    #[test]
    fn explicit_size_keeps_the_original_path() {
        assert_passthrough(json!({"sheet":{"size":"A2"}}));
    }

    #[test]
    fn explicit_scale_is_passed_through() {
        assert_passthrough(json!({"sheet":{"scale":"1:2"}}));
    }

    #[test]
    fn explicit_max_keeps_the_original_path() {
        assert_passthrough(json!({"sheets":{"max":1}}));
        assert_passthrough(json!({"sheets":{"max":2}}));
    }

    #[test]
    fn automatic_baseline_references_keep_the_established_planner() {
        let Some(a) = corpus("mfcadpp/10146.step") else {
            return;
        };
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (
                p.sheet.size.as_str(),
                p.sheet.scale.as_str(),
                p.sheets.len()
            ),
            ("A4", "1:1", 0)
        );
        assert_eq!(p.sheet_candidates.len(), 1);
        assert_eq!(p.sheet_candidates[0].outcome, "automatic baseline reused");
    }

    #[test]
    fn validates_preferred_printer_in_documents_and_typed_specs() {
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"preferred_printer":"A2"}})).is_err()
        );
        let mut s = Spec::default();
        s.sheet.preferred_printer = Some("A2".into());
        assert!(
            model::plan_recovered(&fixture(), &s)
                .unwrap_err()
                .message
                .contains("preferred_printer")
        );
    }

    #[test]
    fn only_new_code_and_subject_pairs_fail() {
        let a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        p.problems = vec![model::Problem {
            code: "annotation_overlap".into(),
            subjects: vec!["b".into(), "a".into()],
            ..Default::default()
        }];
        let b = failures(&a, &p, 0.1);
        p.problems[0].subjects.reverse();
        assert!(failures(&a, &p, 0.1).difference(&b).next().is_none());
        p.problems[0].subjects.push("c".into());
        assert_eq!(failures(&a, &p, 0.1).difference(&b).count(), 1);
        p.problems[0].code = "features_not_recognised".into();
        assert!(failures(&a, &p, 0.1).is_empty());
        p.problems[0].code = "layout_recovery_exhausted".into();
        p.problems[0].severity = "info".into();
        assert!(failures(&a, &p, 0.1).is_empty());
        p.problems[0].severity = "warning".into();
        assert!(!failures(&a, &p, 0.1).is_empty());

        assert!(
            failures(&a, &p, p.sheet.scale_factor * 2.0)
                .iter()
                .any(|(code, _)| code == "legibility_floor_breached")
        );
    }
    #[test]
    fn view_strokes_must_clear_the_frame() {
        let a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        let original = failures(&a, &p, 0.1);
        let first = sheet_fit::footprints(&a, &p)[0];
        p.views[0].at[0] += layout::MARGIN - first[0];
        let changed = failures(&a, &p, 0.1);
        assert!(
            changed
                .difference(&original)
                .any(|(code, subjects)| code == "annotation_off_sheet"
                    && subjects == std::slice::from_ref(&p.views[0].id))
        );
    }
}
