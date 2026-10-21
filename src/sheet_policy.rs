//! Smallest-first automatic paper and sheet-count preferences.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::{self, Analysis, Invalid, Plan, Spec};
use crate::{layout, sheet_fit};

/// Cheap evidence for one paper/count request; `max` is a bound, not an allocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub size: String,
    /// Preferred largest paper, including for refused candidates.
    #[serde(default)]
    pub preferred_max_size: String,
    #[serde(default)]
    pub preferred_max_sheets: usize,
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
    "cross_reference_missing",
    "datum_not_identified",
];

fn problem_key(p: &model::Problem) -> (String, Vec<String>) {
    let mut subjects = p.subjects.clone();
    subjects.sort();
    subjects.dedup();
    (p.code.clone(), subjects)
}

pub(crate) fn layout_problem(problem: &model::Problem) -> bool {
    (FAILING_CODES.contains(&problem.code.as_str())
        || problem.labels().impact == crate::finding::Impact::Readability)
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
    for sheet in std::iter::once(&plan.sheet).chain(&plan.sheets) {
        let dimensions: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| {
                plan.views
                    .iter()
                    .any(|v| v.id == d.view && v.sheet == sheet.number)
            })
            .cloned()
            .collect();
        failures.extend(
            crate::dims::leader_on_extension(&dimensions)
                .into_iter()
                .map(|subjects| ("leader_on_extension".into(), subjects)),
        );
    }
    failures.extend(crate::dims::sheet_route_limits(analysis, plan));
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
        // Invariant: Candidates select paper names from the fixed PAGES table.
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
    // Never newly compress a larger readable reference to the absolute 1:10
    // fallback just to retain preferred paper.
    let floor = if baseline.sheet.scale_factor > sheet_fit::ABSOLUTE_FLOOR {
        floor.max(
            layout::SCALES
                .into_iter()
                .rev()
                .find(|s| *s > sheet_fit::ABSOLUTE_FLOOR)
                .unwrap_or(sheet_fit::ABSOLUTE_FLOOR),
        )
    } else {
        floor
    };
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

fn cap(spec: &Spec) -> &str {
    spec.sheet.preferred_max_size.as_deref().unwrap_or("A3")
}

fn preferred_sheets(spec: &Spec) -> usize {
    spec.sheet.preferred_max_sheets.unwrap_or(2) as usize
}

fn requests(spec: &Spec) -> Vec<(&'static str, usize)> {
    let preferred = layout::PAGES
        .iter()
        .position(|p| p.0 == cap(spec))
        .unwrap_or(1);
    let hard = spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(9) as usize;
    let count = preferred_sheets(spec).min(hard);
    let mut out: Vec<_> = layout::PAGES[..=preferred]
        .iter()
        .map(|p| (p.0, 1))
        .collect();
    out.extend((2..=count).map(|n| (layout::PAGES[preferred].0, n)));
    for p in &layout::PAGES[preferred + 1..] {
        out.extend((1..=count).map(|n| (p.0, n)));
    }
    out.extend((count + 1..=hard).map(|n| ("A0", n)));
    out
}

fn rank(plan: &Plan) -> (usize, usize, f64, usize) {
    let f = model::findings(plan);
    (
        f.hard,
        f.dropped + model::undrawn_gdt(plan),
        -plan.sheet.scale_factor,
        1 + plan.sheets.len(),
    )
}

/// Readability uses the existing feature-opening/overall-size floor and
/// printed-step limits. Layout/readability counts and required losses must not
/// exceed the unrestricted larger-paper reference.
fn usable(analysis: &Analysis, plan: &Plan, reference: &Plan, floor: f64) -> bool {
    if !sheet_fit::inside_frames(analysis, plan)
        || plan.sheet.scale_factor + 1e-9 < floor
        || sheet_fit::illegible_steps(analysis, plan)
            > sheet_fit::illegible_steps(analysis, reference)
        || plan
            .views
            .iter()
            .any(|v| v.id != "v:iso" && v.scale + 1e-9 < floor)
        || !model::carries_required(reference, plan)
        || model::findings(plan).dropped > model::findings(reference).dropped
        || model::undrawn_gdt(plan) > model::undrawn_gdt(reference)
    {
        return false;
    }
    let counts = |p: &Plan| {
        let mut out = std::collections::BTreeMap::new();
        for (code, _) in failures(analysis, p, floor) {
            *out.entry(code).or_insert(0usize) += 1;
        }
        out
    };
    let reference = counts(reference);
    counts(plan)
        .into_iter()
        .all(|(code, n)| n <= *reference.get(&code).unwrap_or(&0))
}

fn finish_choice(mut plan: Plan, spec: &Spec, candidates: Vec<Candidate>, reason: &str) -> Plan {
    plan.spec = spec.clone();
    plan.sheet.default = true;
    for s in &mut plan.sheets {
        s.default = true;
    }
    let larger = layout::PAGES.iter().position(|p| p.0 == plan.sheet.size)
        > layout::PAGES.iter().position(|p| p.0 == cap(spec));
    let more = 1 + plan.sheets.len() > preferred_sheets(spec);
    if larger || more {
        let which = match (larger, more) {
            (true, true) => "paper size and sheet count",
            (true, false) => "paper size",
            _ => "sheet count",
        };
        plan.problems.push(model::Problem {
            id: "p:sheet_preference_exceeded:sheet".into(),
            code: "sheet_preference_exceeded".into(), severity: "info".into(),
            message: format!("Preferred {which} exceeded: using {} sheet(s) of {} at {}. No layout within preferred {} paper and {} sheet(s) kept all ink inside the frame at a readable scale without additional layout problems or required losses.",
                1+plan.sheets.len(),plan.sheet.size,plan.sheet.scale,cap(spec),preferred_sheets(spec)),
            subjects: vec!["sheet".into()], ..Default::default()
        });
    }
    plan.status = crate::completeness::status(&plan.problems);
    explain(&mut plan, candidates, reason);
    plan
}

pub(crate) fn select(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    if automatic(spec) && crate::placement_intent::has_pins(spec) {
        return pinned(analysis, spec);
    }
    let baseline = model::plan_automatic(analysis, spec)?;
    if !automatic(spec) {
        return Ok(baseline);
    }
    let intent = baseline.spec.clone();
    let effective = crate::spec_gdt::effective(analysis, &intent)?;
    let fit = effective.as_ref();
    let floor = candidate_floor(
        fit,
        &baseline,
        sheet_fit::min_geometry_scale(fit).max(crate::sheets::floor(spec).unwrap_or(0.)),
    );
    let estimate = Estimate::new(fit, &baseline);
    let mut candidates = Vec::new();
    let mut best = sheet_fit::inside_frames(fit, &baseline).then(|| baseline.clone());
    // Whole-view allocations are independent of a count bound once they fit.
    let mut separated = std::collections::BTreeMap::<(String, u64), Option<Plan>>::new();
    let mut scale_sources = std::collections::BTreeMap::<u64, Option<Plan>>::new();
    let mut unblocked = std::collections::BTreeMap::<(String, u64), Plan>::new();
    for (size, max) in requests(spec) {
        let mut scales = Vec::new();
        let mut outcome = "reserved footprints cannot fit at a readable scale".to_string();
        let mut trials = Vec::new();
        if baseline.sheet.size == size && baseline.sheets.len() < max {
            trials.push(baseline.clone());
            outcome = "automatic baseline reused".into();
        } else if let Some(scale) = estimate.scale(size, max, floor) {
            let reference_scale = (baseline.sheet.scale_factor < scale
                && baseline.sheet.scale_factor + 1e-9 >= floor)
                .then_some(baseline.sheet.scale_factor);
            for scale in std::iter::once(scale).chain(reference_scale) {
                scales.push(model::scale_name(scale));
                let key = (size.to_string(), scale.to_bits());
                let cached = (max > 1).then(|| unblocked.get(&key).cloned()).flatten();
                let mut forced = intent.clone();
                forced.binding = spec.binding.clone();
                forced.sheet.size = Some(size.into());
                forced.sheet.scale = Some(model::scale_name(scale));
                forced.sheets.get_or_insert_with(Default::default).max = Some(max as u32);
                omit_absent_details(fit, &mut forced, &baseline, scale);
                match cached.map(Ok).unwrap_or_else(|| {
                    crate::planning::fixed_pair(|| model::plan_automatic(analysis, &forced))
                }) {
                    Ok(plan) => {
                        if max > 1 && !plan.sheet.basis.contains("blocked by sheets.max") {
                            unblocked.insert(key, plan.clone());
                        }
                        let needs_split = !usable(fit, &plan, &baseline, floor);
                        trials.push(plan);
                        // A smaller readable scale may fit individual views even when
                        // the unrestricted reference's scale cannot. Plan its ink on
                        // roomy paper, then reuse whole-view allocation; never pack a
                        // cramped small-paper plan after it has already lost items.
                        if needs_split && scale < baseline.sheet.scale_factor {
                            let source =
                                scale_sources.entry(scale.to_bits()).or_insert_with(|| {
                                    let mut wide = forced.clone();
                                    wide.sheet.size = Some("A0".into());
                                    wide.sheets.get_or_insert_with(Default::default).max =
                                        Some(spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(9));
                                    crate::planning::fixed_pair(|| {
                                        model::plan_automatic(analysis, &wide)
                                    })
                                    .ok()
                                });
                            let split = separated
                                .entry((size.into(), scale.to_bits()))
                                .or_insert_with(|| {
                                    crate::sheets::separate_views(
                                        fit,
                                        source.as_ref()?,
                                        size,
                                        spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(9)
                                            as usize,
                                    )
                                });
                            if let Some(plan) = split.as_ref().filter(|p| p.sheets.len() < max) {
                                trials.push(plan.clone());
                            }
                        }
                    }
                    Err(e) => outcome = format!("cannot plan: {}", e.message),
                }
            }
        }
        {
            let split = separated
                .entry((size.into(), baseline.sheet.scale_factor.to_bits()))
                .or_insert_with(|| {
                    crate::sheets::separate_views(
                        fit,
                        &baseline,
                        size,
                        spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(9) as usize,
                    )
                });
            if let Some(plan) = split.as_ref().filter(|p| p.sheets.len() < max) {
                trials.push(plan.clone());
            }
        }
        for plan in trials {
            if !scales.contains(&plan.sheet.scale) {
                scales.push(plan.sheet.scale.clone());
            }
            if !sheet_fit::inside_frames(fit, &plan) {
                outcome = format!(
                    "hard rejected: view geometry or annotation ink leaves the drawable frame at {}",
                    plan.sheet.scale
                );
                continue;
            }
            if best.as_ref().is_none_or(|b| rank(&plan) < rank(b)) {
                best = Some(plan.clone());
            }
            if usable(fit, &plan, &baseline, floor) {
                let reason = format!(
                    "{size}: usable at {} ({} sheet(s)); no clipping, readable scale, no additional layout/readability failures or required losses",
                    plan.sheet.scale,
                    1 + plan.sheets.len()
                );
                candidates.push(Candidate {
                    size: size.into(),
                    preferred_max_size: String::new(),
                    preferred_max_sheets: 0,
                    max,
                    outcome: reason.clone(),
                    scales,
                });
                return Ok(finish_choice(plan, &intent, candidates, &reason));
            }
            outcome = format!(
                "refused at {}: additional layout/readability problems, required losses or unreadable scale ({} hard, {} drops; {})",
                plan.sheet.scale,
                model::findings(&plan).hard,
                model::findings(&plan).dropped,
                failures(fit, &plan, floor)
                    .into_iter()
                    .map(|(code, _)| code)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        candidates.push(Candidate {
            size: size.into(),
            preferred_max_size: String::new(),
            preferred_max_sheets: 0,
            max,
            outcome,
            scales,
        });
    }
    let mut best = best.ok_or_else(|| Invalid {
        message: "No unclipped automatic drawing fits on supported paper within sheets.max; increase sheets/max or state an explicit layout.".into(),
        choices: vec!["sheets/max".into(),"sheet/size".into()],
    })?;
    best.problems.push(model::Problem {
        id:"p:layout_recovery_exhausted:sheet-preferences".into(),
        code:"layout_recovery_exhausted".into(),severity:"warning".into(),
        message:"No usable automatic layout was found through A0 within sheets.max; the least-bad unclipped candidate is retained. Its remaining layout, readability and required-information limitations are reported.".into(),
        subjects:vec!["sheet".into()],..Default::default()
    });
    let reason = "least-bad unclipped candidate; supported-paper search exhausted";
    Ok(finish_choice(best, &intent, candidates, reason))
}

fn automatic(spec: &Spec) -> bool {
    spec.sheet.size.is_none() && spec.sheet.scale.is_none()
}

fn pinned(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    // Offsets belong to the unpinned drawing's frame. A drag may clear a
    // rejected candidate's crossing, but must not change paper and its origin.
    // Select that canonical frame first, then replay the existing fixed-pair R5
    // machinery; there is no new search or placement repair for pins.
    let mut base = model::pin_base(spec);
    base.sheets = spec.sheets.clone();
    let mut canonical = select(analysis, &base)?;
    if spec.sheet.references != Some(true)
        && spec.dimensions.iter().any(|(id, d)| {
            d.place.as_ref().is_some_and(|p| p.pinned)
                && canonical.omitted.iter().any(|o| {
                    &o.id == id
                        && o.basis
                            .contains("other stated dimensions determine this interval")
                })
        })
    {
        // A pinned reference retains the drawing it came from, including its
        // paper choice with reference ink enabled (as model's pin replay does).
        base.sheet.references = Some(true);
        canonical = select(analysis, &base)?;
    }
    let count = 1 + canonical.sheets.len();
    let migrated = model::migrate(spec, analysis)?;
    let intent = crate::recognition_corrections::canonical(analysis, &migrated)?.into_owned();
    // The unpinned automatic baseline is already memoised by canonical selection.
    // Keep its ordinary pin replay when no forced paper was selected; a fixed-pair
    // replay can have a different nominal presentation even at the same scale.
    let raw = model::plan_automatic(analysis, &base)?;
    let mut plan = if count > 1
        && crate::placement_intent::automatic_pins_only(spec)
        && spec.views.values().all(|v| v.place.is_none())
    {
        // A bare pin holds the selected finished ink, including peers on every
        // physical page. Re-solving sheet 1 would create a different pin origin.
        let mut held = canonical.clone();
        held.spec = intent.clone();
        for d in &mut held.dimensions {
            if spec
                .dimensions
                .get(&d.id)
                .is_some_and(|d| d.place.is_some())
            {
                d.default = false;
                if !d.from_spec.iter().any(|s| s == "place") {
                    d.from_spec.push("place".into());
                }
            }
        }
        crate::placement_intent::finish(&mut held, analysis);
        held
    } else if canonical
        .views
        .iter()
        .any(|v| ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()) && v.sheet > 1)
    {
        let source = if canonical.sheet.scale == raw.sheet.scale {
            model::plan_automatic(analysis, spec)?
        } else {
            let mut wide = spec.clone();
            wide.sheet.size = Some("A0".into());
            wide.sheet.scale = Some(canonical.sheet.scale.clone());
            wide.sheets.get_or_insert_with(Default::default).max =
                Some(spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(9));
            crate::planning::fixed_pair(|| model::plan_automatic(analysis, &wide))?
        };
        crate::sheets::replay_views(analysis, &source, &canonical).ok_or_else(|| Invalid {
            message: "Pinned ink cannot fit in the retained annotated view frames; change its placement or remove the pin.".into(),
            choices: vec!["dimensions/<id>/place".into()],
        })?
    } else if raw.sheet.size == canonical.sheet.size
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
                // Invariant: The forced multi-sheet Spec was initialized with Some sheets before this branch.
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
    for problem in canonical
        .problems
        .iter()
        .filter(|p| p.code == "sheet_preference_exceeded")
    {
        if !plan.problems.iter().any(|p| p.code == problem.code) {
            plan.problems.push(problem.clone());
        }
    }
    plan.status = crate::completeness::status(&plan.problems);
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

fn explain(plan: &mut Plan, mut candidates: Vec<Candidate>, reason: &str) {
    let maximum = cap(&plan.spec).to_string();
    for candidate in &mut candidates {
        candidate.preferred_max_size = maximum.clone();
        candidate.preferred_max_sheets = preferred_sheets(&plan.spec);
    }
    let attempts = candidates
        .iter()
        .map(|c| format!("{} up to {}: {}", c.size, c.max, c.outcome))
        .collect::<Vec<_>>()
        .join("; ");
    plan.sheet.basis = format!(
        "{}; sheet/preferred_max_size {maximum}, sheet/preferred_max_sheets {}; sheet choice: {reason}; {attempts}",
        plan.sheet.basis,
        preferred_sheets(&plan.spec)
    );
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
                    .any(|c| c.outcome.contains("leader_on_extension")
                        || c.outcome.contains("hard rejected")
                        || c.outcome.contains("reserved footprints cannot fit")
                        || c.outcome.contains("annotation_overlap")
                        || c.outcome.contains("page_fit_uncertain")),
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
        let ftc08 = corpus("nist/nist_ftc_08_asme1_rc.stp");
        for a in [&tapped, &a].into_iter().chain(ftc08.as_ref()) {
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
            for d in &canonical.dimensions {
                let replayed = pinned.dimensions.iter().find(|p| p.id == d.id).unwrap();
                assert_eq!(replayed.placed, d.placed, "bare pin moved {}", d.id);
            }
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
                let floor = sheet_fit::min_geometry_scale(&analysis)
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
        assert_eq!(p.sheet_candidates.len(), 2);
        for candidate in &p.sheet_candidates[..1] {
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
    fn tapped_posts_keep_readable_scale_and_prefer_complete_allocation() {
        for (file, maximum, size, scale, extra_sheets) in [
            ("string_post_tapped", "A4", "A4", "5:1", 1),
            ("string_post_tapped", "A3", "A3", "5:1", 0),
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/pmi/step/{file}.step.gz"));
            let a = model::analyse_native(&path).unwrap().1;
            let s: Spec =
                serde_json::from_value(json!({"sheet":{"preferred_max_size":maximum}})).unwrap();
            let p = model::plan_recovered(&a, &s).unwrap();
            assert_eq!(
                (
                    p.sheet.size.as_str(),
                    p.sheet.scale.as_str(),
                    p.sheets.len()
                ),
                (size, scale, extra_sheets),
                "{file}: {}",
                p.sheet.basis
            );
            assert!(model::findings(&p).clean(), "{}", p.sheet.basis);
            assert!(p.sheet_candidates.iter().all(|c| c.scales.len() <= 2));
        }
    }

    #[test]
    fn bounded_trials_keep_clean_reductions_and_reject_new_ink() {
        for (file, size, scale) in [
            ("mfcadpp/10020.step", "A4", "1:1"),
            // The former A4 reduction gains a leader/extension contact.
            ("mfcadpp/10033.step", "A3", "2:1"),
            // Whole-view packing clears the A4 frame at the original readable scale.
            ("mfcadpp/10163.step", "A4", "1:2"),
            ("cadgenbench_inputs/cgb245.step", "A2", "1:2"),
            // The reduction loses document notes; the readable larger reference keeps them.
            ("nist/nist_ftc_07_asme1_rd.stp", "A2", "1:2"),
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
        assert!(p.sheet_candidates[0].outcome.contains("usable at"));
    }

    fn inspection_spec(printer: &str) -> Spec {
        let mut rows = vec![json!(["INSPECTION SCHEDULE", "CHECKED"])];
        rows.extend((0..12).map(|i| json!([format!("FORM AND SIZE {}", "X".repeat(60)), i])));
        serde_json::from_value(json!({"sheet":{"preferred_max_size":printer}, "sheets":{"smallest_scale":"5:1"}, "hole_table":"never", "tables":[{"id":"t:1","rows":rows,"prefer":"tr"}]})).unwrap()
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
        assert!(model::carries_required(&q, &p));
        assert!(sheet_fit::inside_frames(&a, &p));
        assert!(model::findings(&p).clean());
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
        let s: Spec = serde_json::from_value(json!({"sheet":{"preferred_max_size":"A3"}})).unwrap();
        let p = model::plan_recovered(&a, &s).unwrap();
        assert_eq!(p.sheet.size, "A3", "{}", p.sheet.basis);
        assert_eq!(p.sheet_candidates.len(), 2);
        assert!(p.sheet_candidates[1].outcome.contains("usable at"));
    }

    #[test]
    fn default_preferences_never_keep_a_clipped_large_part() {
        let a = model::analyse_native(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/pmi/step/nist_stc_09_asme1_ap242-e4.stp.gz"),
        )
        .unwrap()
        .1;
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert!(sheet_fit::inside_frames(&a, &p), "{}", p.sheet.basis);
        assert!(p.sheet.scale_factor >= 0.2, "{}", p.sheet.basis);
        let exceeded = !["A4", "A3"].contains(&p.sheet.size.as_str()) || p.sheets.len() > 1;
        let notices: Vec<_> = p
            .problems
            .iter()
            .filter(|p| p.code == "sheet_preference_exceeded")
            .collect();
        assert_eq!(notices.len(), usize::from(exceeded));
        if let Some(n) = notices.first() {
            assert_eq!(n.severity, "info");
        }
        assert!(
            p.sheet_candidates
                .iter()
                .all(|c| c.preferred_max_size == "A3" && c.preferred_max_sheets == 2)
        );
        for d in &p.dimensions {
            if let crate::items::State::NotDrawable { reason_code } = &d.state {
                assert!(
                    p.problems
                        .iter()
                        .any(|q| q.code == *reason_code && q.subjects.contains(&d.id)),
                    "unreported {}",
                    d.id
                );
            }
        }
    }

    #[test]
    fn one_advisory_identifies_both_exceeded_preferences() {
        let a = fixture();
        let spec = Spec::default();
        let mut p = model::plan_automatic(&a, &spec).unwrap();
        p.sheet.size = "A2".into();
        p.sheets = vec![p.sheet.clone(), p.sheet.clone()];
        let p = finish_choice(p, &spec, Vec::new(), "test");
        let notices: Vec<_> = p
            .problems
            .iter()
            .filter(|p| p.code == "sheet_preference_exceeded")
            .collect();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].severity, "info");
        assert_eq!(notices[0].labels().impact, crate::finding::Impact::Advisory);
        assert!(notices[0].message.contains("paper size and sheet count"));
        assert!(notices[0].message.contains("3 sheet(s) of A2"));
        assert!(notices[0].fix.is_empty());
    }

    #[test]
    fn principal_blocks_keep_their_annotations_and_scale_on_separate_sheets() {
        let a = fixture();
        let spec: Spec = serde_json::from_value(
            json!({"sheet":{"size":"A1","scale":"5:1"},"views":{"iso":{"omit":true}}}),
        )
        .unwrap();
        let source = model::plan_automatic(&a, &spec).unwrap();
        let split = crate::sheets::separate_views(&a, &source, "A3", 9)
            .expect("individual annotated views fit A3");
        assert!(sheet_fit::inside_frames(&a, &split));
        assert!(!split.sheets.is_empty());
        assert!(split.views.iter().all(|v| v.scale == 5.));
        assert!(
            split.views.iter().any(|v| v.id == "v:front" && v.sheet > 1)
                || split.views.iter().any(|v| v.id == "v:plan" && v.sheet > 1)
        );
        assert_eq!(source.dimensions.len(), split.dimensions.len());
        for old in &source.dimensions {
            let new = split.dimensions.iter().find(|d| d.id == old.id).unwrap();
            assert_eq!(new.view, old.view);
            assert_eq!(new.state, old.state);
            assert_eq!(new.measurements, old.measurements);
            if let (Some(a), Some(b)) = (&old.placed, &new.placed) {
                let delta = crate::placement_intent::anchor_delta(&source, &split.views, &old.view)
                    .unwrap();
                for (k, shift) in delta.iter().enumerate() {
                    assert!((a.label_at[k] + shift - b.label_at[k]).abs() < 1e-9);
                }
                for (a, b) in a.lines.iter().zip(&b.lines) {
                    for j in 0..2 {
                        for k in 0..2 {
                            assert!((a[j][k] + delta[k] - b[j][k]).abs() < 1e-9);
                        }
                    }
                }
            } else {
                assert_eq!(old.placed.is_some(), new.placed.is_some());
            }
        }
    }

    #[test]
    fn clipping_rejects_a_candidate_even_without_any_reported_problems() {
        let a = fixture();
        let p = model::plan_automatic(&a, &Spec::default()).unwrap();
        let mut clipped = p.clone();
        clipped.problems.clear();
        clipped.views[0].at[0] += clipped.sheet.page[0];
        assert!(!sheet_fit::inside_frames(&a, &clipped));
        assert!(!usable(&a, &clipped, &p, 0.));
        let mut clipped = p.clone();
        clipped.problems.clear();
        let dimension = clipped
            .dimensions
            .iter_mut()
            .find_map(|d| d.placed.as_mut())
            .unwrap();
        dimension.label_box[2] += clipped.sheet.page[0];
        assert!(!sheet_fit::inside_frames(&a, &clipped));
        assert!(!usable(&a, &clipped, &p, 0.));
    }

    #[test]
    fn a_renderer_trimmed_detail_marker_does_not_pass_the_frame_gate() {
        let a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        assert!(sheet_fit::inside_frames(&a, &p));
        let mut detail = p.views.iter().find(|v| v.id == "v:front").unwrap().clone();
        detail.id = "v:detail_a".into();
        detail.detail = Some(crate::detail::Detail {
            label: "A".into(),
            axis: 0,
            source_view: "v:front".into(),
            source_cut: None,
            kind: "feature".into(),
            bounds: a.envelope,
            marked_bounds: [[-1000.; 3], [1000.; 3]],
            dimensions: vec![],
            scale: 1.,
            authored: false,
            partial_profile: false,
        });
        p.views.push(detail);
        p.problems.clear();
        assert!(!sheet_fit::inside_frames(&a, &p));
    }

    #[test]
    fn route_quality_measures_runs_on_their_carrying_sheet_only() {
        let mut a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        a.edge_ink = None;
        let mut linear = p
            .dimensions
            .iter()
            .find(|d| d.placed.is_some())
            .unwrap()
            .clone();
        linear.id = "linear".into();
        linear.view = "v:front".into();
        linear.kind = "linear".into();
        let ink = linear.placed.as_mut().unwrap();
        ink.lines = vec![
            [[20., 20.], [20., 50.]],
            [[30., 20.], [30., 50.]],
            [[20., 40.], [30., 40.]],
        ];
        ink.arrows = vec![[[20., 40.], [23., 40.]], [[30., 40.], [27., 40.]]];
        let mut callout = linear.clone();
        callout.id = "callout".into();
        callout.kind = "callout".into();
        callout.view = "v:plan".into();
        let ink = callout.placed.as_mut().unwrap();
        ink.lines = vec![[[20.1, 22.], [20.1, 35.]], [[20.1, 35.], [35., 35.]]];
        ink.arrows.clear();
        p.dimensions = vec![linear, callout];
        p.views.retain(|v| v.id == "v:front" || v.id == "v:plan");
        p.views.iter_mut().find(|v| v.id == "v:plan").unwrap().sheet = 2;
        assert!(crate::dims::sheet_route_limits(&a, &p).is_empty());
        p.views.iter_mut().find(|v| v.id == "v:plan").unwrap().sheet = 1;
        let measured = crate::dims::sheet_route_limits(&a, &p);
        assert!(measured.contains(&("leader_ink_run".into(), vec!["callout".into()])));
        p.dimensions[1].placed.as_mut().unwrap().lines =
            vec![[[20.1, 22.], [20.1, 23.]], [[20.1, 23.], [35., 23.]]];
        assert!(crate::dims::sheet_route_limits(&a, &p).is_empty());
    }

    #[test]
    fn a_larger_readable_reference_is_not_newly_reduced_to_the_absolute_floor() {
        let a = fixture();
        let mut p = model::plan_automatic(&a, &Spec::default()).unwrap();
        p.dimensions.clear();
        p.sheet.scale_factor = 0.5;
        assert_eq!(candidate_floor(&a, &p, sheet_fit::ABSOLUTE_FLOOR), 0.2);
        p.sheet.scale_factor = sheet_fit::ABSOLUTE_FLOOR;
        assert_eq!(
            candidate_floor(&a, &p, sheet_fit::ABSOLUTE_FLOOR),
            sheet_fit::ABSOLUTE_FLOOR
        );
    }

    #[test]
    fn requests_follow_both_soft_preferences_then_the_hard_limit() {
        assert_eq!(
            requests(&Spec::default()),
            vec![
                ("A4", 1),
                ("A3", 1),
                ("A3", 2),
                ("A2", 1),
                ("A2", 2),
                ("A1", 1),
                ("A1", 2),
                ("A0", 1),
                ("A0", 2),
                ("A0", 3),
                ("A0", 4),
                ("A0", 5),
                ("A0", 6),
                ("A0", 7),
                ("A0", 8),
                ("A0", 9)
            ]
        );
        let spec: Spec = serde_json::from_value(json!({"sheet":{"preferred_max_size":"A4","preferred_max_sheets":3},"sheets":{"max":2}})).unwrap();
        assert_eq!(
            requests(&spec),
            vec![
                ("A4", 1),
                ("A4", 2),
                ("A3", 1),
                ("A3", 2),
                ("A2", 1),
                ("A2", 2),
                ("A1", 1),
                ("A1", 2),
                ("A0", 1),
                ("A0", 2)
            ]
        );
        let spec: Spec = serde_json::from_value(json!({"sheet":{"preferred_max_size":"A2","preferred_max_sheets":1},"sheets":{"max":3}})).unwrap();
        assert_eq!(
            requests(&spec),
            vec![
                ("A4", 1),
                ("A3", 1),
                ("A2", 1),
                ("A1", 1),
                ("A0", 1),
                ("A0", 2),
                ("A0", 3)
            ]
        );
    }

    #[test]
    fn unreleased_cap_is_unknown_and_preferred_count_must_be_positive() {
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"max_size":"A3"}}))
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"preferred_max_sheets":0}})).is_err()
        );
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"preferred_max_sheets":-1}})).is_err()
        );
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"preferred_max_sheets":100}})).is_ok()
        );
    }

    #[test]
    fn removed_printer_is_refused_with_migration_guidance() {
        for value in ["A4", "A3", "A2"] {
            let error =
                serde_json::from_value::<Spec>(json!({"sheet":{"preferred_printer":value}}))
                    .unwrap_err()
                    .to_string();
            for message in [
                "unknown field",
                "preferred_printer",
                "sheet/preferred_max_size",
                "A4 printer",
                "A3 printer",
            ] {
                assert!(error.contains(message), "{error}");
            }
        }
        let error = model::edit(
            &Spec::default(),
            &json!([{"op":"set", "path":"sheet/preferred_printer", "value":"A4"}]),
        )
        .unwrap_err();
        assert!(error.message.contains("sheet/preferred_max_size"));
    }

    fn assert_passthrough(value: serde_json::Value) {
        let a = fixture();
        let s: Spec = serde_json::from_value(value).unwrap();
        let mut b = model::plan_automatic(&a, &s).unwrap();
        // Public planning attaches retained-layout constraints after paper selection.
        // Compare the complete plans after the same offer finalization, preserving
        // the passthrough check for every drawing field and all remedy metadata.
        crate::fixes::retain_local_layout(&mut b);
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
    fn kept_layout_wins_over_both_automatic_preferences_and_their_edits() {
        let a = fixture();
        let spec: Spec = serde_json::from_value(json!({
            "sheet":{"size":"A2","scale":"1:1","keep_layout":true},
            "sheets":{"max":1}
        }))
        .unwrap();
        for spec in [
            spec.clone(),
            model::edit(
                &spec,
                &json!([
                    {"op":"set","path":"sheet/preferred_max_size","value":"A4"},
                    {"op":"set","path":"sheet/preferred_max_sheets","value":1}
                ]),
            )
            .unwrap(),
        ] {
            assert_eq!(spec.sheet.keep_layout, Some(true));
            let plan = model::plan_recovered(&a, &spec).unwrap();
            assert_eq!(
                (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
                ("A2", "1:1")
            );
            assert!(plan.sheets.is_empty());
            assert!(plan.sheet_candidates.is_empty());
            assert!(
                !plan
                    .problems
                    .iter()
                    .any(|p| p.code == "sheet_preference_exceeded")
            );
        }
    }

    #[test]
    fn explicit_sheet_limit_stays_hard_with_soft_paper_preferences() {
        for max in [1, 2] {
            let spec: Spec = serde_json::from_value(
                json!({"sheet":{"preferred_max_size":"A4"}, "sheets":{"max":max}}),
            )
            .unwrap();
            let p = model::plan_recovered(&fixture(), &spec).unwrap();
            assert_eq!(p.sheet.size, "A4");
            assert!(p.sheets.len() < max as usize);
            assert!(!p.sheet_candidates.is_empty());
        }
    }

    #[test]
    fn retained_candidates_respect_a_lowered_sheet_limit() {
        let Some(a) = corpus("gramel/string_post.step") else {
            return;
        };
        let mut spec = inspection_spec("A4");
        spec.sheets.as_mut().unwrap().max = Some(3);
        crate::planning::with_retained(&a, || model::plan_recovered(&a, &spec)).unwrap();
        for max in [2, 1] {
            spec.sheets.as_mut().unwrap().max = Some(max);
            let warm =
                crate::planning::with_retained(&a, || model::plan_recovered(&a, &spec)).unwrap();
            let cold = crate::planning::isolated(&a, || model::plan_recovered(&a, &spec)).unwrap();
            assert!(warm.sheets.len() < max as usize);
            assert_eq!(
                serde_json::to_value(warm).unwrap(),
                serde_json::to_value(cold).unwrap()
            );
        }
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
        assert!(p.sheet_candidates[0].outcome.contains("usable at"));
    }

    #[test]
    fn validates_preferred_max_size_in_documents_and_typed_specs() {
        assert!(
            serde_json::from_value::<Spec>(json!({"sheet":{"preferred_max_size":"A5"}})).is_err()
        );
        let mut s = Spec::default();
        s.sheet.preferred_max_size = Some("A5".into());
        assert!(
            model::plan_recovered(&fixture(), &s)
                .unwrap_err()
                .message
                .contains("preferred_max_size")
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
