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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scales: Vec<String>,
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
    "leader_on_extension",
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

pub(crate) fn layout_problem(problem: &model::Problem) -> bool {
    FAILING_CODES.contains(&problem.code.as_str())
        && (problem.code != "layout_recovery_exhausted" || problem.severity != "info")
}

fn failures(analysis: &Analysis, plan: &Plan, floor: f64) -> BTreeSet<(String, Vec<String>)> {
    let mut failures: BTreeSet<_> = plan
        .problems
        .iter()
        .filter(|p| layout_problem(p))
        .map(problem_key)
        .collect();
    // This quality finding is measured from final ink, rather than emitted as a
    // Problem. Use the placer's corridor predicate, including its 1 mm clearance,
    // and compare code + item pair just like the reported crossing findings.
    failures.extend(
        crate::dims::leader_on_extension(&plan.dimensions)
            .into_iter()
            .map(|subjects| ("leader_on_extension".into(), subjects)),
    );
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
    // Internal automatic pairs suppress the explicit-scale warning, but must
    // still meet the automatic planner's existing printed-step minimum.
    if sheet_fit::illegible_steps(analysis, plan) > 0 {
        failures.insert(("legibility_floor_breached".into(), vec!["sheet".into()]));
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

/// Baseline outline sizes and reserved paper-mm bands. Outlines change with scale;
/// annotation lanes, captions and other furniture do not. Optional details and the
/// freely resized isometric are omitted, making the necessary fit test optimistic.
struct Estimate {
    views: Vec<(String, [f64; 2], [f64; 2])>,
    blocks: Vec<[f64; 2]>,
    first_area: f64,
    first_blocks: usize,
    scale: f64,
    aligned: bool,
}

impl Estimate {
    fn new(analysis: &Analysis, baseline: &Plan) -> Self {
        let outlines = sheet_fit::footprints(analysis, baseline);
        let bands = crate::view_gutters::bands(analysis, baseline);
        let views = baseline
            .views
            .iter()
            .zip(outlines)
            .zip(bands)
            .filter(|((v, _), _)| v.id != "v:iso" && v.detail.is_none())
            .map(|((v, outline), band)| {
                let size = [outline[2] - outline[0], outline[3] - outline[1]];
                let fixed = std::array::from_fn(|k| {
                    (outline[k] - band[k]).max(0.0) + (band[k + 2] - outline[k + 2]).max(0.0)
                });
                (v.id.clone(), size, fixed)
            })
            .collect();
        let mut boxes: Vec<_> = baseline
            .notes_block
            .iter()
            .chain(&baseline.radius_notes_block)
            .map(|n| n.cells)
            .collect();
        boxes.extend(
            baseline
                .radius_table
                .iter()
                .filter_map(|t| t.table.placed.as_ref().map(|p| p.cells)),
        );
        // R5 moves the rescue hole_table and declared tables, but general
        // notes, radius furniture and principal-view hole schedules stay on 1.
        for t in &baseline.hole_tables {
            boxes.extend(
                t.blocks
                    .iter()
                    .filter_map(|b| b.placed.as_ref().map(|p| p.cells)),
            );
        }
        let first_area = boxes.iter().map(|b| (b[2] - b[0]) * (b[3] - b[1])).sum();
        let first_blocks = boxes.len();
        boxes.extend(
            baseline
                .tables
                .iter()
                .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
        );
        for t in baseline.hole_table.iter() {
            boxes.extend(
                t.blocks
                    .iter()
                    .filter_map(|b| b.placed.as_ref().map(|p| p.cells)),
            );
        }
        Self {
            views,
            first_area,
            first_blocks,
            blocks: boxes
                .into_iter()
                .map(|b| [b[2] - b[0], b[3] - b[1]])
                .collect(),
            scale: baseline.sheet.scale_factor,
            aligned: baseline.spec.views.values().all(|v| v.place.is_none()),
        }
    }

    fn fits(&self, size: &str, max: usize, scale: f64) -> bool {
        let (_, w, h) = layout::PAGES.iter().find(|p| p.0 == size).unwrap();
        let available = [w - 2.0 * layout::MARGIN, h - 2.0 * layout::MARGIN];
        let mut principal = std::collections::BTreeMap::new();
        let mut area = 0.0;
        let mut principal_area = 0.0;
        for (id, outline, band) in &self.views {
            let footprint =
                std::array::from_fn::<_, 2, _>(|k| outline[k] * scale / self.scale + band[k]);
            if (0..2).any(|k| footprint[k] > available[k]) {
                return false;
            }
            area += footprint[0] * footprint[1];
            if ["v:front", "v:plan", "v:side"].contains(&id.as_str()) {
                principal_area += footprint[0] * footprint[1];
                principal.insert(id.as_str(), footprint);
            }
        }
        // R5 keeps the aligned principal projections together on sheet 1.
        const GUTTER: f64 = 6.0;
        if self.aligned
            && let Some(front) = principal.get("v:front")
            && (principal.get("v:side").is_some_and(|side| {
                front[0] + side[0] + GUTTER - shared_band(&self.views, "v:front", "v:side", 0)
                    > available[0]
            }) || principal
                .get("v:plan")
                .is_some_and(|plan| front[1] + plan[1] + GUTTER > available[1]))
        {
            return false;
        }
        for (i, block) in self.blocks.iter().enumerate() {
            if (0..2).any(|k| block[k] > available[k]) {
                return false;
            }
            // A tall block that cannot stack vertically with either aligned
            // principal must fit beside both, allowing their shared corridor.
            // R5 may move declared furniture, but not the principal schedules.
            if self.aligned
                && (max == 1 || i < self.first_blocks)
                && let (Some(front), Some(side)) =
                    (principal.get("v:front"), principal.get("v:side"))
                && block[1] + front[1] > available[1]
                && block[1] + side[1] > available[1]
                && block[0] + front[0] + side[0] - shared_band(&self.views, "v:front", "v:side", 0)
                    > available[0]
            {
                return false;
            }
            area += block[0] * block[1];
        }
        let drawing_area = available[0] * available[1]
            - layout::title_block_width(*w) * crate::title_block::top([0.0; 4]);
        principal_area + self.first_area <= drawing_area && area <= drawing_area * max as f64
    }

    fn scale(&self, size: &str, max: usize, floor: f64) -> Option<f64> {
        layout::SCALES
            .into_iter()
            .filter(|s| *s + 1e-9 >= floor)
            .find(|s| self.fits(size, max, *s))
    }
}

// Opposing annotation bands can share the common base corridor. Keep the
// remaining dimension tiers and callout widths in paper mm.
fn shared_band(views: &[(String, [f64; 2], [f64; 2])], a: &str, b: &str, k: usize) -> f64 {
    let band = |id| views.iter().find(|v| v.0 == id).map_or(0.0, |v| v.2[k]);
    band(a).min(band(b)).min(layout::DIM_PAD)
}

// With no detail owner, the baseline's printed primary steps must remain
// readable at the candidate's common scale. Do not tighten an inherited
// illegibility finding, or assume an authored/enlarged owner has sheet scale.
fn candidate_floor(analysis: &Analysis, baseline: &Plan, floor: f64) -> f64 {
    if baseline.views.iter().any(|v| v.detail.is_some())
        || baseline.spec.views.values().any(|v| v.detail.is_some())
        || sheet_fit::illegible_steps(analysis, baseline) > 0
    {
        return floor;
    }
    let needed = sheet_fit::min_step_scale(analysis, baseline);
    let standard = layout::SCALES
        .into_iter()
        .rev()
        .find(|s| *s >= needed)
        .unwrap_or(needed);
    floor.max(standard)
}

fn requests(printer: &str) -> [(&str, usize); 3] {
    if printer == "A3" {
        [("A4", 1), ("A3", 1), ("A3", 9)]
    } else {
        [("A4", 1), ("A4", 2), ("A3", 1)]
    }
}

pub(crate) fn select(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    if automatic(spec) && crate::placement_intent::has_pins(spec) {
        return pinned(analysis, spec);
    }
    let original_binding = spec.binding.clone();
    let mut baseline = model::plan_automatic(analysis, spec)?;
    if !automatic(spec) {
        return Ok(baseline);
    }
    if baseline.sheet.size == "A4" && baseline.sheets.is_empty() {
        reuse_baseline(&mut baseline, Vec::new(), 1);
        return Ok(baseline);
    }
    let spec = &baseline.spec.clone();
    let effective = crate::spec_gdt::effective(analysis, spec)?;
    let fit_analysis = effective.as_ref();
    let printer = spec.sheet.preferred_printer.as_deref().unwrap_or("A4");
    let floor =
        sheet_fit::min_legible_scale(fit_analysis).max(crate::sheets::floor(spec).unwrap_or(0.0));
    let bound_floor = candidate_floor(fit_analysis, &baseline, floor);
    let mut baseline_failures = failures(fit_analysis, &baseline, floor);
    // Match every baseline problem by code + subject, including informational findings.
    baseline_failures.extend(baseline.problems.iter().map(problem_key));
    let mut estimate = None;
    let mut candidates = Vec::new();
    let mut single: Option<(String, Plan, String)> = None;
    for (size, max) in requests(printer) {
        // A baseline already as good in this preference order needs no forced replay.
        if baseline.sheet.size == size && baseline.sheets.len() < max {
            reuse_baseline(&mut baseline, candidates, max);
            return Ok(baseline);
        }
        // R5's existing max-only shortcut keeps a complete one-sheet result,
        // including a smaller retry refused for readability. Permission to
        // overflow must not restart that complete drawing at a larger scale.
        if max > 1
            && let Some((paper, plan, outcome)) = &single
            && paper == size
            && model::findings(plan).clean()
            && model::drawn_sheets_fit(fit_analysis, plan)
            && crate::sheets::max_only(spec)
        {
            candidates.push(Candidate {
                size: size.into(),
                max,
                outcome: format!("single-sheet R5 result reused: {outcome}"),
                scales: Vec::new(),
            });
            continue;
        }
        let mut scales = Vec::new();
        let mut outcome = format!(
            "reserved footprints cannot fit at {}",
            model::scale_name(bound_floor)
        );
        if let Some(scale) = estimate
            .get_or_insert_with(|| Estimate::new(fit_analysis, &baseline))
            .scale(size, max, bound_floor)
        {
            // Explicit scale bypasses the explicit-paper scale search. At most one
            // smaller standard scale is tried, using the ordinary planner each time.
            let smaller = layout::SCALES
                .into_iter()
                .find(|s| *s < scale && *s + 1e-9 >= floor);
            for scale in std::iter::once(scale).chain(smaller) {
                scales.push(model::scale_name(scale));
                let mut forced = spec.clone();
                // Binding an unbound result to this same Analysis does not change
                // intent. Keep the caller's binding so baseline candidate keys match.
                forced.binding = original_binding.clone();
                forced.sheet.size = Some(size.into());
                forced.sheet.scale = Some(model::scale_name(scale));
                if max > 1 {
                    forced.sheets.get_or_insert_with(Default::default).max = Some(max as u32);
                }
                omit_absent_details(fit_analysis, &mut forced, &baseline, scale);
                match crate::planning::fixed_pair(|| model::plan_automatic(analysis, &forced)) {
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
                                scales,
                            });
                            plan.spec = spec.clone();
                            plan.sheet.default = true;
                            for sheet in &mut plan.sheets {
                                sheet.default = true;
                            }
                            explain(&mut plan, candidates, &reason);
                            return Ok(plan);
                        }
                        let below_minimum = codes.contains("legibility_floor_breached");
                        let crossed_leaders = codes.contains("annotation_leader_crossing")
                            || codes.contains("leader_on_extension");
                        outcome = format!(
                            "layout/readability problems at {} ({})",
                            model::scale_name(scale),
                            codes.into_iter().collect::<Vec<_>>().join(", ")
                        );
                        if max == 1 {
                            single = Some((size.into(), plan.clone(), outcome.clone()));
                        }
                        // Smaller outlines cannot repair a printed-size minimum.
                        // With crossed leaders and no space drops, use R5/the next
                        // paper rather than repeating the same routing search.
                        // A stronger printed-step bound only seeds the first trial:
                        // keep R5's complete-single-sheet prefix at the ordinary floor.
                        if below_minimum
                            || ((model::findings(&plan).hard == 0
                                || (crossed_leaders && bound_floor <= floor + 1e-9))
                                && !plan
                                    .problems
                                    .iter()
                                    .any(|p| crate::hole_table::DROPS.contains(&p.code.as_str())))
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        outcome = format!("cannot plan: {}", error.message);
                        break;
                    }
                }
            }
        }
        candidates.push(Candidate {
            size: size.into(),
            max,
            outcome,
            scales,
        });
    }
    let reason = format!(
        "{}: automatic baseline retained at {}",
        baseline.sheet.size, baseline.sheet.scale
    );
    explain(&mut baseline, candidates, &reason);
    Ok(baseline)
}

fn automatic(spec: &Spec) -> bool {
    spec.sheet.size.is_none()
        && spec.sheet.scale.is_none()
        && spec.sheets.as_ref().and_then(|s| s.max).is_none()
}

fn pinned(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    // Offsets belong to the unpinned drawing's frame. A drag may clear a
    // rejected candidate's crossing, but must not change paper and its origin.
    // Select that canonical frame first, then replay the existing fixed-pair R5
    // machinery; there is no new search or placement repair for pins.
    let mut base = model::pin_base(spec);
    base.sheets = spec.sheets.clone();
    let canonical = select(analysis, &base)?;
    let count = 1 + canonical.sheets.len();
    let migrated = model::migrate(spec, analysis)?;
    let intent = crate::recognition_corrections::canonical(analysis, &migrated)?.into_owned();
    // The unpinned automatic baseline is already memoised by canonical selection.
    // Keep its ordinary pin replay when no forced paper was selected; a fixed-pair
    // replay can have a different nominal presentation even at the same scale.
    let raw = model::plan_automatic(analysis, &base)?;
    let mut plan = if raw.sheet.size == canonical.sheet.size
        && raw.sheet.scale == canonical.sheet.scale
        && raw.sheets.len() == canonical.sheets.len()
    {
        model::plan_automatic(analysis, spec)?
    } else {
        let mut forced = intent.clone();
        forced.binding = spec.binding.clone();
        forced.sheet.size = Some(canonical.sheet.size.clone());
        forced.sheet.scale = Some(canonical.sheet.scale.clone());
        if count > 1 {
            forced.sheets.get_or_insert_with(Default::default).max = Some(count as u32);
            for view in canonical.views.iter().filter(|v| v.sheet > 1) {
                forced
                    .views
                    .entry(view.id.trim_start_matches("v:").into())
                    .or_default()
                    .sheet = Some(view.sheet);
            }
            if let Some(table) = canonical.hole_table.as_ref().filter(|t| t.sheet > 1) {
                forced.sheets.as_mut().unwrap().hole_table = Some(table.sheet);
            }
            for table in &mut forced.tables {
                if let Some(placed) = canonical
                    .tables
                    .iter()
                    .find(|t| t.id == table.id && t.sheet > 1)
                {
                    table.sheet = Some(placed.sheet);
                }
            }
        }
        omit_absent_details(
            analysis,
            &mut forced,
            &canonical,
            canonical.sheet.scale_factor,
        );
        crate::planning::fixed_pair(|| model::plan_automatic(analysis, &forced))?
    };
    plan.spec = intent;
    plan.sheet.default = canonical.sheet.default;
    for sheet in &mut plan.sheets {
        sheet.default = canonical.sheet.default;
    }
    let reason = format!(
        "{}: canonical {count} sheet(s) at {} retained for placement pins",
        canonical.sheet.size, canonical.sheet.scale
    );
    explain(&mut plan, canonical.sheet_candidates, &reason);
    Ok(plan)
}

fn omit_absent_details(analysis: &Analysis, forced: &mut Spec, baseline: &Plan, scale: f64) {
    for (name, detail) in crate::detail::requests(analysis, forced, scale) {
        if !detail.authored
            && !baseline
                .views
                .iter()
                .any(|v| v.id == format!("v:{name}") && v.detail.is_some())
        {
            forced.views.entry(name).or_default().omit = true;
        }
    }
}

fn reuse_baseline(plan: &mut Plan, mut candidates: Vec<Candidate>, max: usize) {
    let size = plan.sheet.size.clone();
    let reason = format!(
        "{size}: automatic baseline at {} ({} sheet(s))",
        plan.sheet.scale,
        1 + plan.sheets.len()
    );
    candidates.push(Candidate {
        size,
        max,
        outcome: "automatic baseline reused".into(),
        scales: Vec::new(),
    });
    explain(plan, candidates, &reason);
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
    fn extension_clearance_uses_padding_and_baseline_item_pairs() {
        let a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        let mut linear = p.dimensions[0].clone();
        linear.id = "linear".into();
        linear.kind = "linear".into();
        let ink = linear.placed.as_mut().unwrap();
        ink.lines = vec![
            [[20., 20.], [20., 40.]],
            [[20., 20.], [20., 40.]],
            [[20., 40.], [30., 40.]],
        ];
        ink.arrows = vec![[[20., 40.], [22., 40.]], [[30., 40.], [28., 40.]]];
        let mut callout = linear.clone();
        callout.id = "callout".into();
        callout.kind = "callout".into();
        let ink = callout.placed.as_mut().unwrap();
        ink.lines = vec![[[21., 30.], [25., 30.]]];
        ink.arrows.truncate(1);
        ink.label_box = [22., 10., 25., 12.];
        p.dimensions = vec![linear, callout];
        let contacts = |p: &Plan| {
            failures(&a, p, 0.)
                .into_iter()
                .filter(|(code, _)| code == "leader_on_extension")
                .collect::<BTreeSet<_>>()
        };
        let inherited = contacts(&p);
        assert_eq!(
            inherited.len(),
            1,
            "inclusive 1 mm corridor; one pair per owner"
        );
        assert!(contacts(&p).difference(&inherited).next().is_none());
        p.dimensions[1].id = "another-callout".into();
        assert_eq!(contacts(&p).difference(&inherited).count(), 1);
        p.dimensions[1].placed.as_mut().unwrap().lines[0][0][0] = 21.001;
        assert!(contacts(&p).is_empty());
        p.dimensions[1].placed.as_mut().unwrap().label_box = [21., 25., 24., 27.];
        assert_eq!(contacts(&p).len(), 1, "padded text also counts");
        p.dimensions[1].placed = None;
        assert!(contacts(&p).is_empty());
    }

    #[test]
    fn smaller_sheets_do_not_gain_extension_contacts() {
        for file in [
            "gramel/GRM-03_thumbwheel_drive_screw.step",
            "ttt_inputs/sm-hanger.step",
        ] {
            let Some(a) = corpus(file) else { continue };
            let p = model::plan_recovered(&a, &Spec::default()).unwrap();
            assert!(
                crate::dims::leader_on_extension(&p.dimensions).is_empty(),
                "{file}"
            );
            assert!(
                p.sheet_candidates
                    .iter()
                    .any(|c| c.outcome.contains("leader_on_extension")),
                "{file}"
            );
        }
    }

    #[test]
    fn automatic_pins_keep_canonical_paper_scale_and_sheet_count() {
        let Some(a) = corpus("gramel/GRM-03_thumbwheel_drive_screw.step") else {
            return;
        };
        let tapped = model::analyse_native(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/pmi/step/string_post_tapped.step.gz"),
        )
        .unwrap()
        .1;
        for a in [&tapped, &a] {
            let canonical = model::plan_recovered(a, &Spec::default()).unwrap();
            let id = &canonical
                .dimensions
                .iter()
                .find(|d| d.kind == "callout" && d.placed.is_some())
                .unwrap()
                .id;
            let s: Spec = serde_json::from_value(json!({
                "dimensions": {id: {"place": {"pinned": true}}}
            }))
            .unwrap();
            let pinned = model::plan_recovered(a, &s).unwrap();
            let choice = |p: &Plan| (p.sheet.size.clone(), p.sheet.scale.clone(), p.sheets.len());
            assert_eq!(choice(&pinned), choice(&canonical));
            assert!(pinned.spec.sheet.size.is_none());
            assert!(pinned.spec.sheet.scale.is_none());
            assert!(pinned.spec.sheets.as_ref().and_then(|s| s.max).is_none());
        }
    }

    /// Opt-in survey of the *unrestricted* explicit-paper planner. Normal tests
    /// exercise the bound below; this audit is deliberately separate from timing.
    #[test]
    #[ignore = "set DRAFTWRIGHT_SHEET_BOUND_SURVEY to a JSON array of analysis/baseline/selected paths"]
    fn corpus_bound_survey() {
        let path = std::env::var("DRAFTWRIGHT_SHEET_BOUND_SURVEY").unwrap();
        let rows: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut results = Vec::new();
        let mut counterexamples = Vec::new();
        for row in rows {
            let read = |key: &str| std::fs::read(row[key].as_str().unwrap()).unwrap();
            let analysis: Analysis = serde_json::from_slice(&read("analysis")).unwrap();
            let baseline: Plan = serde_json::from_slice(&read("baseline")).unwrap();
            let selected: Plan = serde_json::from_slice(&read("selected")).unwrap();
            if !selected
                .sheet_candidates
                .iter()
                .any(|r| r.outcome.starts_with("reserved footprints cannot fit"))
            {
                continue;
            }
            // Match production's baseline preparation scope, including its
            // existing canonical and established-route policies.
            crate::planning::with(&analysis, || {
                let baseline = model::plan_automatic(&analysis, &baseline.spec).unwrap();
                let floor = sheet_fit::min_legible_scale(&analysis)
                    .max(crate::sheets::floor(&baseline.spec).unwrap_or(0.0));
                let bound_floor = candidate_floor(&analysis, &baseline, floor);
                let estimate = Estimate::new(&analysis, &baseline);
                let mut inherited = failures(&analysis, &baseline, floor);
                inherited.extend(baseline.problems.iter().map(problem_key));
                for request in &selected.sheet_candidates {
                    if !request
                        .outcome
                        .starts_with("reserved footprints cannot fit")
                    {
                        continue;
                    }
                    assert!(
                        estimate
                            .scale(&request.size, request.max, bound_floor)
                            .is_none()
                    );
                    let mut forced = baseline.spec.clone();
                    forced.sheet.size = Some(request.size.clone());
                    forced.sheet.scale = None;
                    if request.max > 1 {
                        forced.sheets.get_or_insert_with(Default::default).max =
                            Some(request.max as u32);
                    }
                    let full_path = format!(
                        "{path}.{}-{}-{}.full.json",
                        row["name"].as_str().unwrap(),
                        request.size,
                        request.max
                    );
                    let full = if let Ok(bytes) = std::fs::read(&full_path) {
                        Ok(serde_json::from_slice::<Plan>(&bytes).unwrap())
                    } else {
                        let result = model::plan_automatic(&analysis, &forced);
                        if let Ok(plan) = &result {
                            std::fs::write(&full_path, serde_json::to_vec(plan).unwrap()).unwrap();
                        }
                        result
                    };
                    let codes: BTreeSet<_> = match &full {
                        Ok(plan) => failures(&analysis, plan, floor)
                            .difference(&inherited)
                            .map(|(c, _)| c.clone())
                            .collect(),
                        Err(error) => BTreeSet::from([error.message.clone()]),
                    };
                    if codes.is_empty() {
                        counterexamples.push(format!(
                            "{} {} max={}",
                            row["name"], request.size, request.max
                        ));
                    }
                    let result = json!({"name":row["name"], "size":request.size, "max":request.max, "codes":codes,
                        "full_choice":full.as_ref().ok().map(|p| (&p.sheet.size, &p.sheet.scale, 1+p.sheets.len()))});
                    eprintln!("{}", result);
                    results.push(result);
                }
            });
        }
        std::fs::write(
            format!("{path}.results.json"),
            serde_json::to_vec_pretty(&results).unwrap(),
        )
        .unwrap();
        assert!(
            counterexamples.is_empty(),
            "false refusals: {counterexamples:?}"
        );
    }

    #[test]
    fn paper_bands_and_furniture_do_not_shrink_with_the_outline() {
        let estimate = Estimate {
            views: vec![("v:front".into(), [100.0, 80.0], [280.0, 15.0])],
            blocks: Vec::new(),
            first_area: 0.0,
            first_blocks: 0,
            scale: 1.0,
            aligned: true,
        };
        // An outline-only test would accept A4 at 1:10. A fixed-width
        // annotation reservation exceeds it even with additional sheets.
        assert_eq!(estimate.scale("A4", 1, 0.1), None);
        assert_eq!(estimate.scale("A4", 2, 0.1), None);
        assert_eq!(estimate.scale("A3", 1, 0.1), Some(1.0));
        let with_table = Estimate {
            blocks: vec![[401.0, 20.0]],
            ..estimate
        };
        assert_eq!(with_table.scale("A3", 9, 0.1), None);
    }

    #[test]
    fn tall_furniture_must_fit_beside_both_aligned_principals() {
        let estimate = Estimate {
            views: vec![
                ("v:front".into(), [200.0, 100.0], [0.0; 2]),
                ("v:side".into(), [70.0, 100.0], [0.0; 2]),
            ],
            blocks: vec![[150.0, 238.0]],
            first_area: 150.0 * 238.0,
            first_blocks: 1,
            scale: 1.0,
            aligned: true,
        };
        // Total area and every individual side fit A3. The tall schedule
        // forces both aligned projections beside it; their widths do not.
        assert!(!estimate.fits("A3", 1, 1.0));
        assert!(!estimate.fits("A3", 2, 1.0));
        assert!(estimate.fits("A3", 1, 0.5));
        let movable = Estimate {
            first_blocks: 0,
            first_area: 0.0,
            ..estimate
        };
        assert!(!movable.fits("A3", 1, 1.0));
        assert!(movable.fits("A3", 2, 1.0));
    }

    #[test]
    fn primary_step_readability_bounds_both_a4_requests() {
        let Some(a) = corpus("cadgenbench/threaded_connector_109.step") else {
            return;
        };
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A3", "1:5")
        );
        assert_eq!(p.sheet_candidates.len(), 3);
        for candidate in &p.sheet_candidates[..2] {
            assert_eq!(candidate.size, "A4");
            assert!(candidate.scales.is_empty());
            assert!(
                candidate
                    .outcome
                    .starts_with("reserved footprints cannot fit")
            );
        }
    }

    #[test]
    fn a_routing_refusal_without_space_drops_does_not_repeat_the_scale() {
        let Some(a) = corpus("mfcadpp/10255.step") else {
            return;
        };
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A3", "2:1")
        );
        assert!(p.sheet_candidates.iter().all(|c| c.scales.len() <= 1));
    }

    #[test]
    fn tapped_posts_keep_the_established_r5_choices() {
        for (file, printer, size, extra_sheets) in [
            ("string_post_tapped", "A4", "A4", 1),
            ("string_post_tapped", "A3", "A3", 0),
            ("hw_string_post_tapped_intent", "A4", "A3", 0),
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/pmi/step/{file}.step.gz"));
            let a = model::analyse_native(&path).unwrap().1;
            let s: Spec =
                serde_json::from_value(json!({"sheet":{"preferred_printer":printer}})).unwrap();
            let p = model::plan_recovered(&a, &s).unwrap();
            assert_eq!(
                (
                    p.sheet.size.as_str(),
                    p.sheet.scale.as_str(),
                    p.sheets.len()
                ),
                (size, "5:1", extra_sheets),
                "{file}: {}",
                p.sheet.basis
            );
            assert!(p.sheet_candidates.iter().all(|c| c.scales.len() <= 2));
        }
    }

    #[test]
    fn bounded_trials_keep_clean_reductions_and_reject_new_ink() {
        for (file, size, scale) in [
            ("mfcadpp/10020.step", "A4", "1:1"),
            // The former A4 reduction gains a leader/extension contact.
            ("mfcadpp/10033.step", "A3", "2:1"),
            ("mfcadpp/10163.step", "A4", "1:2"),
            ("mfcadpp/10278.step", "A0", "10:1"),
            ("cadgenbench_inputs/cgb245.step", "A3", "1:5"),
            ("nist/nist_ftc_07_asme1_rd.stp", "A3", "1:5"),
        ] {
            let Some(a) = corpus(file) else {
                return;
            };
            let p = model::plan_recovered(&a, &Spec::default()).unwrap();
            assert_eq!(
                (
                    p.sheet.size.as_str(),
                    p.sheet.scale.as_str(),
                    p.sheets.len()
                ),
                (size, scale, 0),
                "{file}: {}",
                p.sheet.basis
            );
            assert!(p.sheet_candidates.iter().all(|c| c.scales.len() <= 2));
        }
    }

    #[test]
    fn fixed_pair_scope_does_not_leak_into_explicit_requests() {
        assert!(!crate::planning::is_fixed_pair());
        crate::planning::fixed_pair(|| {
            assert!(crate::planning::is_fixed_pair());
            crate::nominal::with_comparison(|| assert!(!crate::planning::is_fixed_pair()));
            assert!(crate::planning::is_fixed_pair());
            let panicked = std::panic::catch_unwind(|| {
                crate::planning::fixed_pair(|| panic!("trial refused"))
            });
            assert!(panicked.is_err());
            assert!(crate::planning::is_fixed_pair());
        });
        assert!(!crate::planning::is_fixed_pair());
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
