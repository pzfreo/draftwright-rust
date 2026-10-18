//! Finished-item footprints for sheet choice; reserved layout bands are never fill.
use crate::layout::{MARGIN, Rect};
use crate::model::{Analysis, Plan};

/// Maintainer's initial target (2026-10-12), as a share of usable paper.
pub const FILL_BAND: [f64; 2] = [0.60, 0.85];
/// Clearance around each drawn item, page mm.
pub const FILL_CLEARANCE: f64 = 1.0;
/// Minimum visible opening across a hole or slot, page mm. Small edge treatments do
/// not set the whole-sheet scale; their sizes are stated by callouts.
pub const MIN_OPENING: f64 = 0.8;
/// The smallest scale the automatic choice offers, whatever the part (1:10).
pub const ABSOLUTE_FLOOR: f64 = 0.1;

pub fn min_legible_scale(analysis: &Analysis) -> f64 {
    let opening = analysis
        .features
        .iter()
        .filter_map(|f| {
            let key = match f.kind.as_str() {
                "hole" => "diameter",
                "slot" => "width",
                _ => return None,
            };
            f.parameters
                .get(key)?
                .as_f64()
                .filter(|v| v.is_finite() && *v > 0.0)
        })
        .fold(f64::INFINITY, f64::min);
    let longest = (0..3)
        .map(|k| analysis.envelope[1][k] - analysis.envelope[0][k])
        .fold(0.0, f64::max);
    let needed = (MIN_OPENING / opening)
        .max(20.0 / longest.max(1e-12))
        .max(ABSOLUTE_FLOOR)
        .max(crate::dims::recess_chain_scale(analysis));
    crate::layout::SCALES
        .into_iter()
        .rev()
        .find(|s| *s >= needed)
        .unwrap_or(needed)
}

/// Minimum primary step span: the dimension font's printed capital height, mm.
/// The 3 mm em includes unused font space; a 2.5 mm flange can still exceed its ink.
pub const MIN_STEP_SPAN: f64 = crate::dims::CAP;

/// Primary axial step spans need at least one capital height on paper. A collision-free
/// label alone does not make a compressed profile readable. Small treatments
/// stated by notes (grooves, chamfers and radii) do not set this floor. Baseline
/// stations still owe the physical interval they replace; enlarged detail owners
/// use their own scale, never the parent sheet's.
pub fn illegible_steps(analysis: &Analysis, plan: &Plan) -> usize {
    let mut checked = std::collections::BTreeSet::new();
    plan.dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .flat_map(|d| {
            let scale = plan
                .views
                .iter()
                .find(|v| v.id == d.view)
                .map_or(0.0, |v| v.scale);
            d.measurements.iter().filter_map(move |m| {
                if !matches!(m.parameter.as_str(), "step.length" | "step_position.length") {
                    return None;
                }
                let f = analysis
                    .features
                    .iter()
                    .find(|f| f.id == m.feature && f.kind == "turned_step")?;
                let length =
                    f.parameters.get("hi")?.as_f64()? - f.parameters.get("lo")?.as_f64()?;
                (length.is_finite() && length > 0.0 && length * scale + 1e-6 < MIN_STEP_SPAN)
                    .then_some(m.feature.as_str())
            })
        })
        .filter(|feature| checked.insert(*feature))
        .count()
}

fn include(b: &mut Rect, p: [f64; 2]) {
    b[0] = b[0].min(p[0]);
    b[1] = b[1].min(p[1]);
    b[2] = b[2].max(p[0]);
    b[3] = b[3].max(p[1]);
}

fn include_arrow(b: &mut Rect, [tip, tail]: [[f64; 2]; 2]) {
    let normal = [tail[1] - tip[1], tip[0] - tail[0]];
    let k = crate::dims::ARROW_HALF / normal[0].hypot(normal[1]).max(1e-12);
    include(b, tip);
    include(b, [tail[0] + k * normal[0], tail[1] + k * normal[1]]);
    include(b, [tail[0] - k * normal[0], tail[1] - k * normal[1]]);
}

/// Union area of axis-aligned item footprints, without double-counting their intersections.
pub fn union_area(boxes: &[Rect]) -> f64 {
    let mut xs: Vec<f64> = boxes.iter().flat_map(|b| [b[0], b[2]]).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    xs.windows(2)
        .map(|x| {
            let mut ys: Vec<[f64; 2]> = boxes
                .iter()
                .filter(|b| b[0] < x[1] && b[2] > x[0])
                .map(|b| [b[1], b[3]])
                .collect();
            ys.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let mut end = f64::NEG_INFINITY;
            let mut height = 0.0;
            for y in ys {
                height += (y[1] - y[0].max(end)).max(0.0);
                end = end.max(y[1]);
            }
            (x[1] - x[0]) * height
        })
        .sum()
}

/// Furniture outside the title block: scale, projection symbol and section marks/captions.
/// These use the same resolved paper geometry and font metrics as the renderer.
pub fn furniture_footprints(analysis: &Analysis, plan: &Plan) -> Vec<Rect> {
    furniture_bounds(analysis, &plan.sheet, &plan.views)
}

pub(crate) fn furniture_bounds(
    analysis: &Analysis,
    sheet: &crate::model::PlannedSheet,
    views: &[crate::model::PlannedView],
) -> Vec<Rect> {
    let mut out = Vec::new();
    let text_box = |text: &str, size: f64, at: [f64; 2]| {
        let laid = crate::title_block::lay(text, size);
        [
            at[0] + laid.min[0],
            at[1] + laid.min[1],
            at[0] + laid.max[0],
            at[1] + laid.max[1],
        ]
    };
    let scale = format!("SCALE {}", sheet.scale);
    out.push(text_box(
        &scale,
        crate::title_block::TEXT,
        crate::title_block::scale_note(sheet.title_block, &scale),
    ));
    if let Some(symbol) = &sheet.projection_symbol {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for c in &symbol.curves {
            let r = c.bbox();
            include(&mut b, [r[0], r[1]]);
            include(&mut b, [r[2], r[3]]);
        }
        if b.iter().all(|x| x.is_finite()) {
            out.push(b);
        }
    }
    for v in views.iter().filter(|v| v.sheet == sheet.number) {
        if let Some(d) = &v.detail {
            out.push(text_box(
                &crate::detail::caption(d),
                crate::section::FONT_SIZE,
                crate::detail::caption_at(v),
            ));
            if let Some(source) = views.iter().find(|s| s.id == d.source_view) {
                let (_, at) = crate::detail::marker(analysis, d, source, views, sheet.page, &[]);
                out.push(text_box(&d.label, crate::section::FONT_SIZE, at));
            }
        }
    }
    for v in views
        .iter()
        .filter(|v| v.sheet == sheet.number && v.cut.is_some())
    {
        let cut = v.cut.as_ref().expect("section cut");
        let view = haecceity::hlr::View::new(v.direction, v.up).expect("planned view");
        let half = crate::model::extent(analysis, &view).0[1] * v.scale / 2.0;
        out.push(text_box(
            &format!("SECTION {}–{}", cut.label, cut.label),
            crate::section::FONT_SIZE,
            [v.at[0], v.at[1] - half - 7.0],
        ));
        if let Some(m) = views.iter().find(|m| m.id == cut.marked_in) {
            let view = haecceity::hlr::View::new(m.direction, m.up).expect("planned view");
            let centre = crate::model::extent(analysis, &view).1;
            let at = |p| {
                let q = view.map(p);
                [
                    m.at[0] + m.scale * (q[0] - centre[0]),
                    m.at[1] + m.scale * (q[1] - centre[1]),
                ]
            };
            let [lo, hi] = analysis.envelope;
            let a = at([lo[0], cut.origin[1], lo[2]]);
            let b = at([hi[0], cut.origin[1], hi[2]]);
            let (ink, letters) = crate::section::cutting_plane(
                a[0].min(b[0]) - crate::section::OVERRUN,
                a[0].max(b[0]) + crate::section::OVERRUN,
                at(cut.origin)[1],
            );
            if let Some(b) = crate::output::items_bbox(&ink) {
                out.push(b);
            }
            out.extend(letters.map(|at| text_box(&cut.label, crate::section::FONT_SIZE, at)));
        }
    }
    out
}

/// Drawn bounds per item, including witnesses and leaders, excluding border and title block.
/// Principal views use the projected part envelope; the isometric uses its measured bounds.
pub fn footprints(analysis: &Analysis, plan: &Plan) -> Vec<Rect> {
    let mut out = Vec::new();
    for v in &plan.views {
        let b = if v.id == "v:iso" {
            let b = analysis.iso_bounds;
            [
                v.at[0] + v.scale * b[0],
                v.at[1] + v.scale * b[1],
                v.at[0] + v.scale * b[2],
                v.at[1] + v.scale * b[3],
            ]
        } else {
            let (size, _) = crate::detail::crop_extent(analysis, v);
            [
                v.at[0] - v.scale * size[0] / 2.0,
                v.at[1] - v.scale * size[1] / 2.0,
                v.at[0] + v.scale * size[0] / 2.0,
                v.at[1] + v.scale * size[1] / 2.0,
            ]
        };
        out.push(b);
    }
    for p in plan.dimensions.iter().filter_map(|d| d.placed.as_ref()) {
        let mut b = p.label_box;
        for q in p.lines.iter().flatten() {
            include(&mut b, *q);
        }
        for a in &p.arrows {
            include_arrow(&mut b, *a);
        }
        out.push(b);
    }
    for p in plan.gdt.iter().filter_map(|g| g.placed.as_ref()) {
        let mut b = p.label_box;
        for q in p.lines.iter().chain(&p.strokes).chain(&p.arrows).flatten() {
            include(&mut b, *q);
        }
        for q in p.fills.iter().flatten() {
            include(&mut b, *q);
        }
        for a in &p.arrows {
            include_arrow(&mut b, *a);
        }
        out.push(b);
    }
    out.extend(plan.notes_block.iter().map(|n| n.cells));
    out.extend(
        plan.tables
            .iter()
            .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
    );
    if let Some(t) = &plan.hole_table {
        out.extend(
            t.blocks
                .iter()
                .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
        );
    }
    for b in &plan.balloons {
        let mut r = [
            b.at[0] - b.radius,
            b.at[1] - b.radius,
            b.at[0] + b.radius,
            b.at[1] + b.radius,
        ];
        for q in b.leader.iter().flatten() {
            include(&mut r, *q);
        }
        out.push(r);
    }
    for p in plan.requirements.iter().filter_map(|r| r.placed.as_ref()) {
        let mut b = p.label_box;
        include(&mut b, [p.glyph[0], p.glyph[1]]);
        include(&mut b, [p.glyph[2], p.glyph[3]]);
        out.push(b);
    }
    out.extend(furniture_footprints(analysis, plan));
    out
}

/// Usable-area share. Clip at the border and subtract title-block intersections from the union.
pub fn fill(analysis: &Analysis, plan: &Plan) -> f64 {
    let [w, h] = plan.sheet.page;
    let mut tb = plan.sheet.title_block;
    tb[3] = crate::title_block::top(tb);
    let boxes: Vec<Rect> = footprints(analysis, plan)
        .into_iter()
        .map(|b| {
            [
                (b[0] - FILL_CLEARANCE).max(MARGIN),
                (b[1] - FILL_CLEARANCE).max(MARGIN),
                (b[2] + FILL_CLEARANCE).min(w - MARGIN),
                (b[3] + FILL_CLEARANCE).min(h - MARGIN),
            ]
        })
        .filter(|b| b[0] < b[2] && b[1] < b[3])
        .collect();
    let in_title: Vec<Rect> = boxes
        .iter()
        .map(|b| {
            [
                b[0].max(tb[0]),
                b[1].max(tb[1]),
                b[2].min(tb[2]),
                b[3].min(tb[3]),
            ]
        })
        .filter(|b| b[0] < b[2] && b[1] < b[3])
        .collect();
    (union_area(&boxes) - union_area(&in_title))
        / ((w - 2.0 * MARGIN) * (h - 2.0 * MARGIN) - (tb[2] - tb[0]) * (tb[3] - tb[1]))
}

/// Bounded measured correction: trim only empty off-paper reservation, then adjust a free row.
/// A failed estimate can leave an orientation view cramped below an otherwise clear row.
/// Lift the entire row together (preserving projection alignment and every dimension), then
/// fit the orientation view against the measured blocks, once. Never move pinned content.
/// Reconcile conservative section bands with settled ink before spilling onto another page.
/// Principal projections and their dimensions stay aligned; free sections may use another gap.
fn correct_sections(analysis: &Analysis, plan: &mut Plan) {
    if !plan.views.iter().any(|v| v.cut.is_some())
        || plan.spec.views.values().any(|v| v.place.is_some())
    {
        return;
    }
    let bounds = footprints(analysis, plan);
    let mut groups = bounds[..plan.views.len()].to_vec();
    for d in &plan.dimensions {
        let Some(p) = &d.placed else { continue };
        let Some(i) = plan.views.iter().position(|v| v.id == d.view) else {
            return;
        };
        include(&mut groups[i], [p.label_box[0], p.label_box[1]]);
        include(&mut groups[i], [p.label_box[2], p.label_box[3]]);
        for q in p.lines.iter().flatten() {
            include(&mut groups[i], *q);
        }
        for arrow in &p.arrows {
            include_arrow(&mut groups[i], *arrow);
        }
    }
    for (i, v) in plan.views.iter().enumerate() {
        for curve in v.line_work.iter().flat_map(|w| &w.curves) {
            let b = curve.bbox();
            include(&mut groups[i], [b[0], b[1]]);
            include(&mut groups[i], [b[2], b[3]]);
        }
        if let Some(detail) = &v.detail {
            let laid = crate::dims::font()
                .text(&crate::detail::caption(detail), crate::section::FONT_SIZE);
            let at = crate::detail::caption_at(v);
            include(&mut groups[i], [at[0] + laid.min[0], at[1] + laid.min[1]]);
            include(&mut groups[i], [at[0] + laid.max[0], at[1] + laid.max[1]]);
            if let Some(j) = plan.views.iter().position(|p| p.id == detail.source_view) {
                let (circle, at) = crate::detail::marker(
                    analysis,
                    detail,
                    &plan.views[j],
                    &plan.views,
                    plan.sheet.page,
                    &[],
                );
                let b = circle.bbox();
                include(&mut groups[j], [b[0], b[1]]);
                include(&mut groups[j], [b[2], b[3]]);
                let laid = crate::dims::font().text(&detail.label, crate::section::FONT_SIZE);
                include(&mut groups[j], [at[0] + laid.min[0], at[1] + laid.min[1]]);
                include(&mut groups[j], [at[0] + laid.max[0], at[1] + laid.max[1]]);
            }
        }
        if let Some(cut) = &v.cut {
            let projection = haecceity::hlr::View::new(v.direction, v.up).expect("section view");
            let half = crate::model::extent(analysis, &projection).0[1] * v.scale / 2.;
            let laid = crate::dims::font().text(
                &format!("SECTION {}–{}", cut.label, cut.label),
                crate::section::FONT_SIZE,
            );
            let at = [v.at[0], v.at[1] - half - 7.];
            include(&mut groups[i], [at[0] + laid.min[0], at[1] + laid.min[1]]);
            include(&mut groups[i], [at[0] + laid.max[0], at[1] + laid.max[1]]);
            if let Some(j) = plan.views.iter().position(|w| w.id == cut.marked_in) {
                let parent = &plan.views[j];
                let projection =
                    haecceity::hlr::View::new(parent.direction, parent.up).expect("parent view");
                let (x0, x1, y) = crate::model::cutting_line(analysis, parent, &projection, cut);
                let (ink, letters) = crate::section::cutting_plane(x0, x1, y);
                if let Some(b) = crate::output::items_bbox(&ink) {
                    include(&mut groups[j], [b[0], b[1]]);
                    include(&mut groups[j], [b[2], b[3]]);
                }
                let laid = crate::dims::font().text(&cut.label, crate::section::FONT_SIZE);
                for at in letters {
                    include(&mut groups[j], [at[0] + laid.min[0], at[1] + laid.min[1]]);
                    include(&mut groups[j], [at[0] + laid.max[0], at[1] + laid.max[1]]);
                }
            }
        }
    }

    let [w, h] = plan.sheet.page;
    let drawable = [MARGIN + 2., MARGIN + 2., w - MARGIN - 2., h - MARGIN - 2.];
    let mut obstacles = vec![plan.sheet.title_block];
    let mut candidate = plan.clone();
    for (i, v) in candidate
        .views
        .iter_mut()
        .enumerate()
        .filter(|(_, v)| v.id != "v:iso")
    {
        let b = groups[i];
        v.area = [b[0] - 1., b[1] - 1., b[2] + 1., b[3] + 1.];
        if v.cut.is_none() {
            obstacles.push(v.area);
        }
    }
    for v in candidate.views.iter_mut().filter(|v| v.cut.is_some()) {
        let b = v.area;
        let size = [b[2] - b[0], b[3] - b[1]];
        let inside = b[0] >= drawable[0]
            && b[1] >= drawable[1]
            && b[2] <= drawable[2]
            && b[3] <= drawable[3];
        if !inside || obstacles.iter().any(|o| crate::sheets::overlap(b, *o)) {
            let r = crate::layout::largest_empty_rect(drawable, &obstacles, size);
            if r[2] - r[0] < size[0] || r[3] - r[1] < size[1] {
                return;
            }
            let delta = [
                (r[0] + r[2] - b[0] - b[2]) / 2.,
                (r[1] + r[3] - b[1] - b[3]) / 2.,
            ];
            v.at = [v.at[0] + delta[0], v.at[1] + delta[1]];
            crate::centrelines::shift(v, delta);
            v.area = [
                b[0] + delta[0],
                b[1] + delta[1],
                b[2] + delta[0],
                b[3] + delta[1],
            ];
            for d in candidate.dimensions.iter_mut().filter(|d| d.view == v.id) {
                if let Some(p) = &mut d.placed {
                    for q in p
                        .lines
                        .iter_mut()
                        .chain(&mut p.arrows)
                        .flatten()
                        .chain(std::iter::once(&mut p.label_at))
                    {
                        q[0] += delta[0];
                        q[1] += delta[1];
                    }
                    for (k, x) in p.label_box.iter_mut().enumerate() {
                        *x += delta[k % 2];
                    }
                }
            }
        }
        v.basis += "; section placed using settled ink and caption bounds";
        obstacles.push(v.area);
    }
    for b in &mut obstacles {
        b[0] -= 2.;
        b[1] -= 2.;
        b[2] += 2.;
        b[3] += 2.;
    }
    if let Some(iso) = candidate.views.iter_mut().find(|v| v.id == "v:iso") {
        let b = analysis.iso_bounds;
        let size = [b[2] - b[0], b[3] - b[1]];
        if size.iter().any(|s| *s <= 0.) {
            return;
        }
        let r = crate::layout::largest_empty_rect(drawable, &obstacles, size);
        let fit = ((r[2] - r[0]) / size[0]).min((r[3] - r[1]) / size[1]);
        let scale = crate::layout::keep_sheet_scale(
            (0.9 * fit).min(1.3 * plan.sheet.scale_factor),
            plan.sheet.scale_factor,
        );
        if scale <= 0. || obstacles.iter().any(|o| crate::sheets::overlap(r, *o)) {
            return;
        }
        iso.scale = scale;
        iso.at = [
            (r[0] + r[2] - scale * (b[0] + b[2])) / 2.,
            (r[1] + r[3] - scale * (b[1] + b[3])) / 2.,
        ];
        iso.area = r;
        iso.basis = "orientation view fitted around settled projection and section blocks".into();
    }
    if crate::model::drawn_sheets_fit(analysis, &candidate) {
        // Packing moved the section and its centre lines together. Replace
        // their old page-bound findings with checks of the actual final ink.
        candidate
            .problems
            .retain(|p| !p.id.starts_with("p:annotation_off_sheet:ink:"));
        candidate.problems.extend(crate::model::line_work_problems(
            &candidate.sheet,
            &candidate.views,
        ));
        *plan = candidate;
    }
}

pub(crate) fn correct_estimate(analysis: &Analysis, plan: &mut Plan) {
    // Wider label shelves can push an unused edge of the final section's generic band
    // past the border. The complete rendered footprints prove whether it is empty paper;
    // never trim an estimate through any model, label, arrow or piece of furniture.
    if plan.problems.iter().any(|p| p.code == "page_fit_uncertain")
        && !crate::placement_intent::has_pins(&plan.spec)
        && plan.gdt.is_empty()
    {
        let [w, h] = plan.sheet.page;
        let inside =
            |b: &Rect| b[0] >= MARGIN && b[1] >= MARGIN && b[2] <= w - MARGIN && b[3] <= h - MARGIN;
        if footprints(analysis, plan).iter().all(inside) {
            for v in plan.views.iter_mut().filter(|v| v.cut.is_some()) {
                let b = [
                    v.area[0].max(MARGIN),
                    v.area[1].max(MARGIN),
                    v.area[2].min(w - MARGIN),
                    v.area[3].min(h - MARGIN),
                ];
                if b[0] < b[2] && b[1] < b[3] {
                    v.area = b;
                }
            }
        }
    }
    if !plan.problems.iter().any(|p| p.code == "page_fit_uncertain")
        || crate::placement_intent::has_pins(&plan.spec)
        || crate::model::dimension_text_edited(&plan.spec)
        || plan.spec.iso_scale_factor.is_some()
        || plan.spec.views.contains_key("iso")
        || !plan.gdt.is_empty()
        || !plan.balloons.is_empty()
        || plan.hole_table.is_some()
        || !plan.notes.is_empty()
        || !plan.tables.is_empty()
        || plan.requirements.iter().any(|r| r.placed.is_some())
        || plan
            .dimensions
            .iter()
            .any(|d| !matches!(d.kind.as_str(), "linear" | "callout" | "angular"))
    {
        return;
    }
    if plan.views.iter().any(|v| v.cut.is_some()) {
        if !crate::model::drawn_sheets_fit(analysis, plan) {
            correct_sections(analysis, plan);
        }
        return;
    }
    if plan.views.iter().any(|v| v.detail.is_some()) {
        return;
    }
    let Some(iso_index) = plan.views.iter().position(|v| v.id == "v:iso") else {
        return;
    };
    let bounds = footprints(analysis, plan);
    let mut groups: Vec<Rect> = bounds[..plan.views.len()].to_vec();
    // Corner arcs are independent packing obstacles. Expanding their whole
    // carrying view into an arc's empty quadrant needlessly overlaps its neighbour.
    let mut angular = Vec::new();
    for d in plan.dimensions.iter().filter(|d| d.placed.is_some()) {
        if d.kind == "angular" {
            if let Some(b) = crate::dims::ink_box(d) {
                angular.push(b);
            }
            continue;
        }
        let Some(i) = plan.views.iter().position(|v| v.id == d.view) else {
            return;
        };
        let p = d.placed.as_ref().expect("placed dimension");
        include(&mut groups[i], [p.label_box[0], p.label_box[1]]);
        include(&mut groups[i], [p.label_box[2], p.label_box[3]]);
        for q in p.lines.iter().chain(&p.arrows).flatten() {
            include(&mut groups[i], *q);
        }
    }
    // The projections share one column reservation, including its unused paper.
    // Keep their area edges aligned so a zero-gap relative pin is a true no-op.
    if plan.dimensions.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    }) && let (Some(front), Some(plan_view)) = (
        plan.views.iter().position(|v| v.id == "v:front"),
        plan.views.iter().position(|v| v.id == "v:plan"),
    ) {
        let left = groups[front][0].min(groups[plan_view][0]);
        let right = groups[front][2].max(groups[plan_view][2]);
        for i in [front, plan_view] {
            groups[i][0] = left;
            groups[i][2] = right;
        }
        if groups[plan_view][1] - groups[front][3] >= 2.0 {
            let boundary = 0.5 * (groups[front][3] + groups[plan_view][1]);
            groups[front][3] = boundary - 1.0;
            groups[plan_view][1] = boundary + 1.0;
        }
    }
    let top = groups
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != iso_index)
        .map(|(_, b)| b[3])
        .chain(angular.iter().map(|b| b[3]))
        .fold(f64::NEG_INFINITY, f64::max);
    let lift = plan.sheet.page[1] - MARGIN - 2.0 - top;
    if lift <= 0.0 {
        return;
    }
    for (i, v) in plan
        .views
        .iter_mut()
        .enumerate()
        .filter(|(i, _)| *i != iso_index)
    {
        v.at[1] += lift;
        crate::centrelines::shift(v, [0.0, lift]);
        let b = groups[i];
        v.area = [b[0] - 1.0, b[1] + lift - 1.0, b[2] + 1.0, b[3] + lift + 1.0];
        v.basis += "; free projection row lifted together using measured annotation bounds";
        groups[i] = v.area;
    }
    for p in plan.dimensions.iter_mut().filter_map(|d| d.placed.as_mut()) {
        for q in p.lines.iter_mut().chain(&mut p.arrows).flatten() {
            q[1] += lift;
        }
        p.label_at[1] += lift;
        p.label_box[1] += lift;
        p.label_box[3] += lift;
    }
    groups.extend(
        angular
            .into_iter()
            .map(|b| [b[0], b[1] + lift, b[2], b[3] + lift]),
    );
    groups.remove(iso_index);
    let mut tb = plan.sheet.title_block;
    tb[3] = tb[3].max(crate::title_block::top(tb) + 4.0);
    groups.push(tb);
    for b in &mut groups {
        b[0] -= 2.0;
        b[1] -= 2.0;
        b[2] += 2.0;
        b[3] += 2.0;
    }
    let b = analysis.iso_bounds;
    let size = [b[2] - b[0], b[3] - b[1]];
    if size.iter().any(|s| *s <= 0.0) {
        return;
    }
    let [w, h] = plan.sheet.page;
    let rect =
        crate::layout::largest_empty_rect([MARGIN, MARGIN, w - MARGIN, h - MARGIN], &groups, size);
    let fit = ((rect[2] - rect[0]) / size[0]).min((rect[3] - rect[1]) / size[1]);
    let sheet_scale = plan.sheet.scale_factor;
    let scale = crate::layout::keep_sheet_scale((0.9 * fit).min(1.3 * sheet_scale), sheet_scale);
    if scale <= 0.0 {
        return;
    }
    let iso = &mut plan.views[iso_index];
    iso.scale = scale;
    iso.at = [
        (rect[0] + rect[2] - scale * (b[0] + b[2])) / 2.0,
        (rect[1] + rect[3] - scale * (b[1] + b[3])) / 2.0,
    ];
    iso.area = rect;
    iso.basis = format!(
        "the automatic orientation view, fitted once around measured row blocks at {:.4} page mm per model mm{}",
        iso.scale,
        if scale == sheet_scale {
            " (the sheet's scale)"
        } else {
            " (not to scale)"
        }
    );
}

/// Pack settled annotated view blocks without re-routing their witnesses or leaders. The template is built at the same scale on A0; only owned paper geometry moves.
/// Page-anchored blocks and authored view relations stay with the existing planner.
pub(crate) fn pack_gdt(analysis: &Analysis, template: &Plan, recipient: &Plan) -> Option<Plan> {
    pack_reserved(analysis, template, recipient, template)
}

/// Omission edits reserve the complete inventory, keeping unaffected items fixed.
pub(crate) fn pack_inventory(
    analysis: &Analysis,
    plan: &Plan,
    recipient: &Plan,
    inventory: &[crate::dims::PlannedDimension],
) -> Option<Plan> {
    let mut reservation = plan.clone();
    reservation.dimensions = inventory.to_vec();
    pack_reserved(analysis, plan, recipient, &reservation)
}

fn pack_reserved(
    analysis: &Analysis,
    template: &Plan,
    recipient: &Plan,
    reservation: &Plan,
) -> Option<Plan> {
    if template.spec.iso_scale_factor.is_some()
        || template.spec.sheet.projection.as_deref() == Some("first")
        || template.hole_table.is_some()
        || !template.balloons.is_empty()
        || !template.notes.is_empty()
        || !template.tables.is_empty()
        || template.requirements.iter().any(|r| r.placed.is_some())
        || template.views.iter().any(|v| v.cut.is_some())
        || template.spec.views.values().any(|v| v.place.is_some())
        || crate::placement_intent::has_pins(&template.spec)
        || template.views.iter().map(|v| &v.id).collect::<Vec<_>>()
            != recipient.views.iter().map(|v| &v.id).collect::<Vec<_>>()
    {
        return None;
    }
    let mut p = template.clone();
    p.sheet = recipient.sheet.clone();
    let footprints = footprints(analysis, reservation);
    let count = template.views.len();
    let mut groups = footprints[..count].to_vec();
    let owners = reservation
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .map(|d| d.view.as_str())
        .chain(
            template
                .gdt
                .iter()
                .filter(|g| g.placed.is_some())
                .map(|g| g.view.as_str()),
        );
    for (owner, b) in owners.zip(&footprints[count..]) {
        let i = template.views.iter().position(|v| v.id == owner)?;
        include(&mut groups[i], [b[0], b[1]]);
        include(&mut groups[i], [b[2], b[3]]);
    }
    let mut shifts = vec![[0.0; 2]; count];
    let column: Vec<usize> = template
        .views
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v.id.as_str(), "v:front" | "v:plan"))
        .map(|(i, _)| i)
        .collect();
    let left = column
        .iter()
        .map(|i| groups[*i][0])
        .fold(f64::INFINITY, f64::min);
    let right = column
        .iter()
        .map(|i| groups[*i][2])
        .fold(f64::NEG_INFINITY, f64::max);
    // Keep the aligned projection column's reservation aligned too. Relative
    // view pins then describe the same zero-gap relationship as ordinary packing.
    for i in &column {
        groups[*i][0] = left;
        groups[*i][2] = right;
    }
    let root = column
        .iter()
        .copied()
        .find(|i| template.views[*i].id == "v:front")
        .or_else(|| column.first().copied())?;
    let row_bottom = template
        .views
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v.id.as_str(), "v:front" | "v:side"))
        .map(|(i, _)| groups[i][1])
        .fold(groups[root][1], f64::min);
    // Packing settled blocks retains the same readable paper to the border as a shelf.
    let common = [MARGIN + 2.0 - left, MARGIN + 2.0 - row_bottom];
    for i in &column {
        shifts[*i] = common;
    }
    if let Some(i) = column
        .iter()
        .copied()
        .find(|i| template.views[*i].id == "v:plan")
        && i != root
    {
        // Each measured block already receives 1 mm of padding below.
        shifts[i][1] += groups[root][3] + 2.0 - groups[i][1];
    }
    if let Some(i) = template.views.iter().position(|v| v.id == "v:side") {
        let right = column
            .iter()
            .map(|j| groups[*j][2] + shifts[*j][0])
            .fold(f64::NEG_INFINITY, f64::max);
        shifts[i] = [right + 6.0 - groups[i][0], common[1]];
    }
    let moved = |b: Rect, d: [f64; 2]| {
        [
            b[0] + d[0] - 1.0,
            b[1] + d[1] - 1.0,
            b[2] + d[0] + 1.0,
            b[3] + d[1] + 1.0,
        ]
    };
    let overlap = |a: Rect, b: Rect| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
    let mut boxes: Vec<_> = template
        .views
        .iter()
        .enumerate()
        .filter(|(_, v)| v.id != "v:iso")
        .map(|(i, _)| moved(groups[i], shifts[i]))
        .collect();
    let tb = p.sheet.title_block;
    let lift = boxes
        .iter()
        .filter(|b| overlap(**b, tb))
        .map(|b| tb[3] + 2.0 - b[1])
        .fold(0.0, f64::max);
    for d in &mut shifts {
        d[1] += lift;
    }
    for b in &mut boxes {
        b[1] += lift;
        b[3] += lift;
    }
    let [w, h] = p.sheet.page;
    if boxes.iter().any(|b| {
        b[0] < MARGIN - 1e-6
            || b[1] < MARGIN - 1e-6
            || b[2] > w - MARGIN + 1e-6
            || b[3] > h - MARGIN + 1e-6
    }) {
        return None;
    }
    for (i, a) in boxes.iter().enumerate() {
        if boxes[i + 1..].iter().any(|b| overlap(*a, *b)) {
            return None;
        }
    }
    let delta = |owner: &str| {
        template
            .views
            .iter()
            .position(|v| v.id == owner)
            .map(|i| shifts[i])
    };
    let move_point = |q: &mut [f64; 2], d: [f64; 2]| {
        q[0] += d[0];
        q[1] += d[1];
    };
    let move_box = |b: &mut Rect, d: [f64; 2]| {
        b[0] += d[0];
        b[1] += d[1];
        b[2] += d[0];
        b[3] += d[1];
    };
    for (i, v) in p
        .views
        .iter_mut()
        .enumerate()
        .filter(|(_, v)| v.id != "v:iso")
    {
        move_point(&mut v.at, shifts[i]);
        crate::centrelines::shift(v, shifts[i]);
        v.area = moved(groups[i], shifts[i]);
        v.basis += "; settled annotations and GD&T packed as one measured view block";
    }
    for d in &mut p.dimensions {
        if let Some(q) = &mut d.placed {
            let shift = delta(&d.view)?;
            for pt in q.lines.iter_mut().chain(&mut q.arrows).flatten() {
                move_point(pt, shift);
            }
            move_point(&mut q.label_at, shift);
            move_box(&mut q.label_box, shift);
        }
    }
    for g in &mut p.gdt {
        if let Some(q) = &mut g.placed {
            let shift = delta(&g.view)?;
            move_point(&mut q.tip, shift);
            move_box(&mut q.label_box, shift);
            for pt in q
                .lines
                .iter_mut()
                .chain(&mut q.arrows)
                .chain(&mut q.strokes)
                .flatten()
            {
                move_point(pt, shift);
            }
            for pt in q.fills.iter_mut().flatten() {
                move_point(pt, shift);
            }
            for arc in &mut q.arcs {
                arc[0] += shift[0];
                arc[1] += shift[1];
            }
            for text in q.letters.iter_mut().chain(&mut q.runs) {
                move_point(&mut text.at, shift);
            }
        }
    }
    let sheet_scale = p.sheet.scale_factor;
    let has_detail = p.views.iter().any(|v| v.detail.is_some());
    if let Some(iso) = p.views.iter_mut().find(|v| v.id == "v:iso") {
        boxes.push(tb);
        for b in &mut boxes {
            b[0] -= 2.0;
            b[1] -= 2.0;
            b[2] += 2.0;
            b[3] += 2.0;
        }
        let b = analysis.iso_bounds;
        let size = [b[2] - b[0], b[3] - b[1]];
        if size.iter().any(|s| *s <= 0.0) {
            return None;
        }
        let rect = crate::layout::largest_empty_rect(
            [MARGIN, MARGIN, w - MARGIN, h - MARGIN],
            &boxes,
            size,
        );
        let fit = ((rect[2] - rect[0]) / size[0]).min((rect[3] - rect[1]) / size[1]);
        let desired = if has_detail {
            iso.scale.min(sheet_scale)
        } else {
            iso.scale
        };
        let scale = crate::layout::keep_sheet_scale(desired.min(0.9 * fit), sheet_scale);
        if scale <= 0.0 {
            return None;
        }
        iso.scale = scale;
        iso.area = rect;
        iso.at = [
            (rect[0] + rect[2] - scale * (b[0] + b[2])) / 2.0,
            (rect[1] + rect[3] - scale * (b[1] + b[3])) / 2.0,
        ];
        iso.basis = format!(
            "the orientation view, fitted around the measured annotated blocks at {:.4} page mm per model mm{}",
            iso.scale,
            if scale == sheet_scale {
                " (the sheet's scale)"
            } else {
                " (not to scale)"
            }
        );
    }
    // The isometric was fitted again on the recipient sheet. Its scale
    // disclaimer must follow the settled view before grading the packed plan.
    crate::iso_caption::refresh(analysis, &mut p);
    let checked = crate::dims::recheck(analysis, &p);
    let checked_codes: std::collections::BTreeSet<_> = crate::dims::recheck(analysis, template)
        .into_iter()
        .chain(checked.iter().cloned())
        .map(|q| q.code)
        .collect();
    p.problems.retain(|q| !checked_codes.contains(&q.code));
    p.problems.extend(checked);
    p.problems.retain(|q| {
        !matches!(
            q.code.as_str(),
            "title_field_overflow" | "general_tolerance_missing"
        ) && !q.refs.iter().any(|r| r.starts_with("title_block/"))
    });
    p.problems.extend(crate::title_block::problems(
        &p.sheet.title_cells,
        p.sheet.page,
    ));
    p.problems.retain(|q| {
        !matches!(
            q.code.as_str(),
            "unclaimed_geometry" | "feature_not_dimensioned"
        )
    });
    crate::completeness::report(analysis, &mut p);
    p.status = crate::completeness::status(&p.problems);
    p.sheet.fill = fill(analysis, &p);
    Some(p)
}

/// A neighbour may consume more paper only when the dimensioned views enlarge.
pub(crate) fn allowed(smallest: (usize, f64), sheet: usize, scale: f64) -> bool {
    sheet == smallest.0 || (sheet == smallest.0 + 1 && scale > smallest.1)
}

pub(crate) fn rank(a: (f64, f64, f64), b: (f64, f64, f64)) -> std::cmp::Ordering {
    let band = |p: (f64, f64, f64)| (FILL_BAND[0]..=FILL_BAND[1]).contains(&p.2);
    band(b).cmp(&band(a)).then_with(|| {
        if band(a) && band(b) {
            b.0.total_cmp(&a.0).then_with(|| a.1.total_cmp(&b.1))
        } else {
            b.2.total_cmp(&a.2)
                .then_with(|| b.0.total_cmp(&a.0))
                .then_with(|| a.1.total_cmp(&b.1))
        }
    })
}

/// A finished valid, complete pair as the sheet choice sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Candidate {
    /// Index into [`crate::layout::PAGES`].
    pub sheet: usize,
    pub scale: f64,
    /// Page width, mm: the paper preference.
    pub width: f64,
    pub fill: f64,
    /// Leaders and witnesses through material beyond the first estimate's.
    pub extra_crossings: usize,
}

/// The maintainer's rule (2026-10-12) over valid, complete candidates only. The smallest fit is
/// the smallest sheet with a valid complete drawing, at its largest such scale; unless the Spec
/// names the sheet, only that pair and its next larger sheet at a larger scale compete, and
/// [`rank`] decides between them by fill. Against a valid first estimate (*seed_clean*), a pair
/// that adds material crossings is set aside (the quality ratchet); it still defines the smallest
/// fit. Returns the smallest fit, and the chosen index, or `None` when every allowed pair adds
/// crossings to the valid first estimate, which is then kept. `None` overall: no candidate.
pub(crate) fn choose(
    candidates: &[Candidate],
    named_sheet: bool,
    seed_clean: bool,
) -> Option<(Option<usize>, (usize, f64))> {
    let smallest = candidates
        .iter()
        .map(|c| (c.sheet, c.scale))
        .min_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)))?;
    let key = |c: &Candidate| (c.scale, c.width, c.fill);
    let chosen = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            named_sheet
                || (c.sheet == smallest.0 && c.scale == smallest.1)
                || (c.sheet == smallest.0 + 1 && allowed(smallest, c.sheet, c.scale))
        })
        .filter(|(_, c)| !seed_clean || c.extra_crossings == 0)
        .min_by(|a, b| rank(key(a.1), key(b.1)))
        .map(|(i, _)| i);
    Some((chosen, smallest))
}

/// Claim unused paper for final annotation bounds, transferring only vacant neighbouring
/// reservation while protecting every view and its ink, the title block and drawing border.
/// The full inventory includes
/// omitted dimensions, so withdrawing one item does not move its neighbours' bands.
pub(crate) fn reserve_annotation_bounds(
    analysis: &Analysis,
    sheet: &crate::model::PlannedSheet,
    views: &mut [crate::model::PlannedView],
    inventory: &[crate::dims::PlannedDimension],
    gdt: &[crate::dims::gdt::PlannedGdt],
) {
    if !inventory.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    }) {
        return;
    }
    reserve_bounds(analysis, sheet, views, inventory, gdt, None);
}

/// Longer default nominals need their actual paper reservation, including the
/// run-on of a short linear dimension. Reclaim only vacant neighbouring bands;
/// every view, annotation, border and title-block gate remains the same.
pub(crate) fn reserve_nominal_bounds(
    analysis: &Analysis,
    sheet: &crate::model::PlannedSheet,
    views: &mut [crate::model::PlannedView],
    inventory: &[crate::dims::PlannedDimension],
    gdt: &[crate::dims::gdt::PlannedGdt],
) {
    let grown: std::collections::BTreeSet<_> = inventory
        .iter()
        .filter(|d| d.kind == "linear")
        .filter(|d| {
            d.measurements.iter().any(|m| {
                let number = crate::nominal::format(m.value);
                number
                    .split_once('.')
                    .is_some_and(|(_, fraction)| fraction.len() > 1)
                    && d.label.contains(&number)
            })
        })
        .map(|d| d.id.clone())
        .collect();
    if !grown.is_empty() {
        reserve_bounds(analysis, sheet, views, inventory, gdt, Some(&grown));
    }
}

fn reserve_bounds(
    analysis: &Analysis,
    sheet: &crate::model::PlannedSheet,
    views: &mut [crate::model::PlannedView],
    inventory: &[crate::dims::PlannedDimension],
    gdt: &[crate::dims::gdt::PlannedGdt],
    selected: Option<&std::collections::BTreeSet<String>>,
) {
    let mut bounds: Vec<(&str, Rect, bool)> = inventory
        .iter()
        .filter_map(|d| Some((d.view.as_str(), crate::dims::ink_box(d)?, true)))
        .collect();
    let mut protected: Vec<Rect> = views
        .iter()
        .map(|v| {
            let mut b = if v.id == "v:iso" {
                [
                    v.at[0] + v.scale * analysis.iso_bounds[0],
                    v.at[1] + v.scale * analysis.iso_bounds[1],
                    v.at[0] + v.scale * analysis.iso_bounds[2],
                    v.at[1] + v.scale * analysis.iso_bounds[3],
                ]
            } else {
                let size = if v.detail.is_some() {
                    // A crop owns only its enlarged local outline. Reserving the
                    // whole part at the detail scale strands neighbouring bands.
                    crate::detail::crop_extent(analysis, v).0
                } else {
                    let camera =
                        haecceity::hlr::View::new(v.direction, v.up).expect("planned view");
                    crate::model::extent(analysis, &camera).0
                };
                [
                    v.at[0] - v.scale * size[0] / 2.0,
                    v.at[1] - v.scale * size[1] / 2.0,
                    v.at[0] + v.scale * size[0] / 2.0,
                    v.at[1] + v.scale * size[1] / 2.0,
                ]
            };
            for c in v.line_work.iter().flat_map(|w| &w.curves) {
                let r = c.bbox();
                include(&mut b, [r[0], r[1]]);
                include(&mut b, [r[2], r[3]]);
            }
            b
        })
        .collect();
    // Caption paper belongs to the crop too; a neighbouring annotation may
    // reclaim empty detail bands, but never its caption or readable gap.
    for caption in crate::detail::caption_obstacles(views) {
        if let (Some(i), Some(p)) = (
            views.iter().position(|v| v.id == caption.view),
            caption.placed.as_ref(),
        ) {
            include(&mut protected[i], [p.label_box[0], p.label_box[1]]);
            include(&mut protected[i], [p.label_box[2], p.label_box[3]]);
        }
    }
    for (view, b, _) in &bounds {
        if let Some(i) = views.iter().position(|v| v.id == *view) {
            include(&mut protected[i], [b[0], b[1]]);
            include(&mut protected[i], [b[2], b[3]]);
        }
    }
    for g in gdt {
        if let (Some(i), Some(p)) = (views.iter().position(|v| v.id == g.view), g.placed.as_ref()) {
            include(&mut protected[i], [p.label_box[0], p.label_box[1]]);
            include(&mut protected[i], [p.label_box[2], p.label_box[3]]);
            for q in p
                .lines
                .iter()
                .chain(&p.strokes)
                .chain(&p.arrows)
                .flatten()
                .chain(p.fills.iter().flatten())
            {
                include(&mut protected[i], *q);
            }
        }
    }
    if let Some(ids) = selected {
        bounds = inventory
            .iter()
            .filter(|d| ids.contains(&d.id))
            .filter_map(|d| Some((d.view.as_str(), crate::dims::ink_box(d)?, true)))
            .collect();
    }
    bounds.extend(
        gdt.iter()
            .filter(|g| selected.is_none() && g.kind != "control_frame")
            .filter_map(|g| {
                let p = g.placed.as_ref()?;
                let mut b = p.label_box;
                for q in p
                    .lines
                    .iter()
                    .chain(&p.strokes)
                    .chain(&p.arrows)
                    .flatten()
                    .chain(p.fills.iter().flatten())
                {
                    include(&mut b, *q);
                }
                Some((g.view.as_str(), b, true))
            }),
    );
    let overlap = |a: Rect, b: Rect| {
        a[0] < b[2] - 1e-9 && b[0] < a[2] - 1e-9 && a[1] < b[3] - 1e-9 && b[1] < a[3] - 1e-9
    };
    for (view, b, reclaim) in bounds {
        let Some(i) = views.iter().position(|v| v.id == view) else {
            continue;
        };
        let area = views[i].area;
        let mut candidate = [
            area[0].min(b[0] - 0.5),
            area[1].min(b[1] - 0.5),
            area[2].max(b[2] + 0.5),
            area[3].max(b[3] + 0.5),
        ];
        if b[0] < MARGIN - 1e-6
            || b[1] < MARGIN - 1e-6
            || b[2] > sheet.page[0] - MARGIN + 1e-6
            || b[3] > sheet.page[1] - MARGIN + 1e-6
            || overlap(candidate, sheet.title_block)
        {
            continue;
        }
        candidate[0] = candidate[0].max(MARGIN);
        candidate[1] = candidate[1].max(MARGIN);
        candidate[2] = candidate[2].min(sheet.page[0] - MARGIN);
        candidate[3] = candidate[3].min(sheet.page[1] - MARGIN);
        let mut areas: Vec<_> = views.iter().map(|v| v.area).collect();
        let mut clear = true;
        for j in 0..views.len() {
            if i == j || !overlap(candidate, areas[j]) {
                continue;
            }
            if !reclaim || views[j].id == "v:iso" {
                clear = false;
                break;
            }
            // Transfer only unused reservation. Every neighbouring view, mark,
            // leader, witness and glyph remains within its original paper.
            let old = areas[j];
            let ink = protected[j];
            let mut choices = Vec::new();
            if ink[2] <= candidate[0] && old[0] < candidate[0] {
                choices.push([old[0], old[1], candidate[0], old[3]]);
            }
            if ink[0] >= candidate[2] && candidate[2] < old[2] {
                choices.push([candidate[2], old[1], old[2], old[3]]);
            }
            if ink[3] <= candidate[1] && old[1] < candidate[1] {
                choices.push([old[0], old[1], old[2], candidate[1]]);
            }
            if ink[1] >= candidate[3] && candidate[3] < old[3] {
                choices.push([old[0], candidate[3], old[2], old[3]]);
            }
            let size = |b: Rect| (b[2] - b[0]) * (b[3] - b[1]);
            if let Some(b) = choices
                .into_iter()
                .max_by(|a, b| size(*a).total_cmp(&size(*b)))
            {
                areas[j] = b;
            } else {
                clear = false;
                break;
            }
        }
        if clear {
            areas[i] = candidate;
            for (v, area) in views.iter_mut().zip(areas) {
                v.area = area;
            }
        }
    }
}

/// Final compose-then-pack pass for settled linear ink (ISSUE-563-b). Use the
/// complete placement inventory so omitting a dimension does not change a pin's
/// frame. Moving a view block transports its already settled routes unchanged.
pub(crate) fn reserve_linear_corridors(
    analysis: &Analysis,
    plan: &mut Plan,
    inventory: &[crate::dims::PlannedDimension],
    automatic_pair: bool,
    cache: &crate::dims::footprints::Cache,
) {
    let old: Vec<_> = plan.views.iter().map(|v| (v.at, v.area, v.scale)).collect();
    let compact = (!plan.gdt.is_empty()
        || plan.spec.dimensions.values().any(|d| d.tolerance.is_some())
        || plan.spec.features.values().any(|f| f.fit.is_some()))
        // An unreadable seed keeps its established control-frame bands.
        && (plan.gdt.is_empty() || plan.sheet.scale_factor >= min_legible_scale(analysis))
        && !crate::placement_intent::has_pins(&plan.spec)
        && !crate::model::dimension_text_edited(&plan.spec)
        && !plan.spec.views.values().any(|v| v.place.is_some())
        && (automatic_pair || plan.spec.sheet.size.is_none() || plan.spec.sheet.scale.is_none());
    if compact {
        // Settled control frames and edited labels use their real ink, as pack_gdt does.
        // Estimated empty lanes are not additional physical drafting demands.
        for v in plan.views.iter_mut().filter(|v| v.id != "v:iso") {
            let size = crate::detail::crop_extent(analysis, v).0;
            v.area = [
                v.at[0] - v.scale * size[0] / 2.0,
                v.at[1] - v.scale * size[1] / 2.0,
                v.at[0] + v.scale * size[0] / 2.0,
                v.at[1] + v.scale * size[1] / 2.0,
            ];
        }
        let mut owned = plan.dimensions.clone();
        owned.extend(plan.gdt.iter().filter_map(crate::dims::gdt::proxy));
        for d in owned.iter().filter(|d| d.placed.is_some()) {
            if let Some(b) = crate::dims::ink_box(d)
                && let Some(v) = plan.views.iter_mut().find(|v| v.id == d.view)
            {
                include(&mut v.area, [b[0], b[1]]);
                include(&mut v.area, [b[2], b[3]]);
            }
        }
    }
    for d in inventory
        .iter()
        .chain(&plan.dimensions)
        .filter(|d| d.kind == "linear")
    {
        let Some(b) = crate::dims::ink_box(d) else {
            continue;
        };
        if let Some(v) = plan.views.iter_mut().find(|v| v.id == d.view) {
            include(&mut v.area, [b[0], b[1]]);
            include(&mut v.area, [b[2], b[3]]);
        }
    }
    if plan
        .views
        .iter()
        .zip(&old)
        .all(|(v, (_, a, _))| v.area == *a)
    {
        return;
    }
    // Placement has already checked this settled ink. Keep its collision
    // identities for replacement instead of repeating the same geometric check
    // before the carrier move. Unplaced-item reasons and the router's own-body
    // material re-entry findings are not replaced by foreign-view collision checks.
    let placed: std::collections::BTreeMap<_, _> = plan
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .map(|d| (d.id.as_str(), d.view.as_str()))
        .chain(
            plan.gdt
                .iter()
                .filter(|g| g.placed.is_some())
                .map(|g| (g.id.as_str(), g.view.as_str())),
        )
        .collect();
    let previously_checked: std::collections::BTreeSet<_> = plan
        .problems
        .iter()
        .filter(|p| {
            let Some(owner) = p.subjects.first().and_then(|id| placed.get(id.as_str())) else {
                return false;
            };
            match p.code.as_str() {
                "annotation_outside_area"
                | "dimension_text_stray"
                | "annotation_off_sheet"
                | "annotation_over_title_block" => true,
                "annotation_over_view" => p
                    .subjects
                    .get(1)
                    .is_some_and(|id| plan.views.iter().any(|v| v.id == *id)),
                "annotation_line_over_view" => p
                    .subjects
                    .get(1)
                    .is_some_and(|id| id != *owner && plan.views.iter().any(|v| v.id == *id)),
                "dimension_line_shared"
                | "annotation_overlap"
                | "annotation_ink_overlap"
                | "annotation_leader_crossing" => {
                    p.subjects.iter().all(|id| placed.contains_key(id.as_str()))
                }
                "leader_through_feature" | "extension_through_feature" => p.id.contains(":line:"),
                _ => false,
            }
        })
        .map(|p| p.id.clone())
        .collect();
    let mut shifts = vec![[0.0; 2]; plan.views.len()];
    let index = |id: &str| plan.views.iter().position(|v| v.id == id);
    let free = !crate::placement_intent::has_pins(&plan.spec)
        && !crate::model::dimension_text_edited(&plan.spec)
        && !plan.spec.views.values().any(|v| v.place.is_some())
        // A named sheet and scale retain their established projection frame,
        // including section marks. Reserve the full ink without silently moving
        // that frame; automatic sheet candidates may compose their free blocks.
        && (automatic_pair || plan.spec.sheet.size.is_none() || plan.spec.sheet.scale.is_none());
    if free {
        let first = plan.spec.sheet.projection.as_deref() == Some("first");
        let column: Vec<usize> = plan
            .views
            .iter()
            .enumerate()
            .filter(|(_, v)| matches!(v.id.as_str(), "v:front" | "v:plan"))
            .map(|(i, _)| i)
            .collect();
        if let (Some(front), Some(top)) = (index("v:front"), index("v:plan")) {
            let (below, above) = if first { (top, front) } else { (front, top) };
            let gap = (old[above].1[1] - old[below].1[3]).max(0.0);
            let extra = (plan.views[below].area[3] + gap - plan.views[above].area[1]).max(0.0);
            shifts[top][1] += if first { -extra } else { extra };
        }
        if let Some(side) = index("v:side") {
            let edge = column
                .iter()
                .map(|&i| {
                    if first {
                        plan.views[i].area[0]
                    } else {
                        plan.views[i].area[2]
                    }
                })
                .reduce(|a, b| if first { a.min(b) } else { a.max(b) })
                .unwrap_or(if first {
                    plan.views[side].area[2]
                } else {
                    plan.views[side].area[0]
                });
            let previous = column
                .iter()
                .map(|&i| if first { old[i].1[0] } else { old[i].1[2] })
                .reduce(|a, b| if first { a.min(b) } else { a.max(b) })
                .unwrap_or(edge);
            let gap = if first {
                (previous - old[side].1[2]).max(0.0)
            } else {
                (old[side].1[0] - previous).max(0.0)
            };
            shifts[side][0] = if first {
                (edge - gap - plan.views[side].area[2]).min(0.0)
            } else {
                (edge + gap - plan.views[side].area[0]).max(0.0)
            };
            for (i, _) in plan
                .views
                .iter()
                .enumerate()
                .filter(|(_, v)| v.cut.is_some())
            {
                shifts[i][0] = shifts[side][0];
                // A measured side corridor may reach its section partner. Carry
                // the section beyond the final side edge, preserving the old gap;
                // an actual positive gap also avoids rounded touching rectangles.
                if old[i].1[0] >= old[side].1[0] {
                    let gap = (old[i].1[0] - old[side].1[2]).max(1e-6);
                    shifts[i][0] = shifts[i][0].max(
                        plan.views[side].area[2] + shifts[side][0] + gap - plan.views[i].area[0],
                    );
                }
            }
        }
        // Keep the assembled row inside the drawing area; source annotations and
        // every projection partner travel with it. Page-anchored furniture stays.
        let group: Vec<usize> = plan
            .views
            .iter()
            .enumerate()
            .filter(|(_, v)| v.id != "v:iso")
            .map(|(i, _)| i)
            .collect();
        let lower = [0, 1].map(|k| {
            group
                .iter()
                .map(|&i| plan.views[i].area[k] + shifts[i][k])
                .fold(f64::INFINITY, f64::min)
        });
        let lift = lower.map(|v| (MARGIN - v).max(0.0));
        for &i in &group {
            for k in 0..2 {
                shifts[i][k] += lift[k];
            }
        }
        // A whole projection assembly can clear page furniture by a rigid lift.
        // Its settled dimensions and control frames keep their relative positions.
        let title = plan.sheet.title_block;
        let raise = group
            .iter()
            .filter_map(|&i| {
                let a = plan.views[i].area;
                let d = shifts[i];
                (a[0] + d[0] < title[2]
                    && title[0] < a[2] + d[0]
                    && a[1] + d[1] < title[3]
                    && title[1] < a[3] + d[1])
                    .then_some(title[3] + 1.0 - a[1] - d[1])
            })
            .fold(0.0, f64::max);
        if raise > 0.0
            && group.iter().all(|&i| {
                plan.views[i].area[3] + shifts[i][1] + raise <= plan.sheet.page[1] - MARGIN
            })
        {
            for &i in &group {
                shifts[i][1] += raise;
            }
        }
    }
    for (i, v) in plan.views.iter_mut().enumerate() {
        let delta = shifts[i];
        for (k, offset) in delta.into_iter().enumerate() {
            v.at[k] += offset;
            v.area[k] += offset;
            v.area[k + 2] += offset;
        }
        crate::centrelines::shift(v, delta);
    }
    if plan.spec.views.values().any(|v| v.place.is_some()) {
        // A relative constraint uses the final footprints. Re-resolve its gap,
        // retaining the held annotation carriers, then transport their free peers.
        let template = plan.clone();
        let mut more = Vec::new();
        crate::placement_intent::views(
            &plan.spec,
            &mut plan.views,
            Some(&template),
            &plan.sheet,
            &[],
            &mut more,
        );
        for (i, v) in plan.views.iter_mut().enumerate() {
            if v.id == "v:iso"
                && plan
                    .spec
                    .views
                    .get("iso")
                    .is_some_and(|v| v.place.as_ref().is_some_and(|p| p.pinned))
            {
                v.at = old[i].0;
                v.area = old[i].1;
                v.scale = old[i].2;
            }
            let extra = [
                v.at[0] - template.views[i].at[0],
                v.at[1] - template.views[i].at[1],
            ];
            crate::centrelines::shift(v, extra);
            shifts[i] = [v.at[0] - old[i].0[0], v.at[1] - old[i].0[1]];
        }
        for p in more {
            if !plan.problems.iter().any(|q| q.id == p.id) {
                plan.problems.push(p);
            }
        }
    }
    let deltas: std::collections::BTreeMap<_, _> = plan
        .views
        .iter()
        .enumerate()
        .map(|(i, v)| (v.id.clone(), shifts[i]))
        .collect();
    for d in &mut plan.dimensions {
        let Some(p) = &mut d.placed else {
            continue;
        };
        let delta = deltas.get(&d.view).copied().unwrap_or([0.0; 2]);
        for q in p.lines.iter_mut().chain(&mut p.arrows).flatten() {
            for (k, offset) in delta.into_iter().enumerate() {
                q[k] += offset;
            }
        }
        for (k, offset) in delta.into_iter().enumerate() {
            p.label_at[k] += offset;
            p.label_box[k] += offset;
            p.label_box[k + 2] += offset;
        }
    }
    for g in &mut plan.gdt {
        let delta = deltas.get(&g.view).copied().unwrap_or([0.0; 2]);
        if delta == [0.0; 2] {
            continue;
        }
        if let Some(q) = &mut g.placed {
            for (k, offset) in delta.into_iter().enumerate() {
                q.tip[k] += offset;
                q.label_box[k] += offset;
                q.label_box[k + 2] += offset;
            }
            for pt in q
                .lines
                .iter_mut()
                .chain(&mut q.arrows)
                .chain(&mut q.strokes)
                .flatten()
            {
                for (k, offset) in delta.into_iter().enumerate() {
                    pt[k] += offset;
                }
            }
            for pt in q.fills.iter_mut().flatten() {
                for (k, offset) in delta.into_iter().enumerate() {
                    pt[k] += offset;
                }
            }
            for arc in &mut q.arcs {
                arc[0] += delta[0];
                arc[1] += delta[1];
            }
            for text in q.letters.iter_mut().chain(&mut q.runs) {
                for (k, offset) in delta.into_iter().enumerate() {
                    text.at[k] += offset;
                }
            }
        }
    }
    // Free isometrics have no measurement ownership. First retain their exact ink
    // and caption if that footprint remains clear; otherwise fit the orientation
    // view once against the composed blocks and the page-anchored furniture.
    if let Some(i) = plan.views.iter().position(|v| v.id == "v:iso") {
        let iso = &plan.views[i];
        let mut ink = [
            iso.at[0] + iso.scale * analysis.iso_bounds[0],
            iso.at[1] + iso.scale * analysis.iso_bounds[1],
            iso.at[0] + iso.scale * analysis.iso_bounds[2],
            iso.at[1] + iso.scale * analysis.iso_bounds[3],
        ];
        for c in plan.captions.iter().filter(|c| c.view == "v:iso") {
            include(&mut ink, [c.ink[0], c.ink[1]]);
            include(&mut ink, [c.ink[2], c.ink[3]]);
        }
        let overlap = |a: Rect, b: Rect| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
        let mut obstacles: Vec<_> = plan
            .views
            .iter()
            .filter(|v| v.id != "v:iso")
            .map(|v| v.area)
            .collect();
        obstacles.push(plan.sheet.title_block);
        obstacles.extend(
            plan.tables
                .iter()
                .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
        );
        if let Some(n) = &plan.notes_block {
            obstacles.push(n.cells);
        }
        let iso_pinned = plan
            .spec
            .views
            .get("iso")
            .is_some_and(|v| v.place.as_ref().is_some_and(|p| p.pinned));
        // A held region is part of the pin's canonical frame.
        if !iso_pinned && !obstacles.iter().any(|b| overlap(ink, *b)) {
            plan.views[i].area = ink;
        } else if !iso_pinned
            && free
            && plan.spec.iso_scale_factor.is_none()
            && !plan.spec.views.contains_key("iso")
        {
            let b = analysis.iso_bounds;
            let size = [b[2] - b[0], b[3] - b[1]];
            if size.iter().all(|v| *v > 0.0) {
                let [w, h] = plan.sheet.page;
                let rect = crate::layout::largest_empty_rect(
                    [MARGIN, MARGIN, w - MARGIN, h - MARGIN],
                    &obstacles,
                    size,
                );
                let fit = ((rect[2] - rect[0]) / size[0]).min((rect[3] - rect[1]) / size[1]);
                let scale = crate::layout::keep_sheet_scale(
                    plan.views[i].scale.min(0.9 * fit),
                    plan.sheet.scale_factor,
                );
                if scale > 0.0 {
                    let iso = &mut plan.views[i];
                    iso.scale = scale;
                    iso.area = rect;
                    iso.at = [
                        (rect[0] + rect[2] - scale * (b[0] + b[2])) / 2.0,
                        (rect[1] + rect[3] - scale * (b[1] + b[3])) / 2.0,
                    ];
                    iso.basis += "; orientation fitted around measured linear corridors";
                }
            }
        }
    }
    let [w, h] = plan.sheet.page;
    let too_large = plan.views.iter().filter(|v| v.id != "v:iso").any(|v| {
        v.area[0] < MARGIN - 1e-6
            || v.area[1] < MARGIN - 1e-6
            || v.area[2] > w - MARGIN + 1e-6
            || v.area[3] > h - MARGIN + 1e-6
    });
    if too_large && !plan.problems.iter().any(|p| p.code == "page_fit_uncertain") {
        plan.problems.push(crate::model::Problem {id:"p:page_fit_uncertain:linear_corridors".into(),code:"page_fit_uncertain".into(),severity:"warning".into(),message:"The composed linear corridors do not fit the drawing area; choose a larger sheet or a smaller scale.".into(),subjects:vec!["sheet".into()],..Default::default()});
    }
    let checked = crate::dims::recheck_cached(analysis, plan, cache);
    let ids: std::collections::BTreeSet<_> = checked
        .iter()
        .map(|p| p.id.clone())
        .chain(previously_checked)
        .collect();
    // Material re-entry diagnostics share a code with foreign-view collisions,
    // but have their own identities and must survive a rigid carrier move.
    plan.problems
        .retain(|p| p.code != "annotation_outside_area" && !ids.contains(&p.id));
    plan.problems.extend(checked);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_sheet_details_do_not_reserve_local_furniture() {
        let (_, a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/mounting_plate.step"),
        )
        .unwrap();
        let spec = serde_json::from_value(serde_json::json!({"sheet":{"size":"A2","scale":"1:2"},"views":{"detail_c":{"detail":{"around":"f:hole:1","from":"plan","scale_factor":2}}}})).unwrap();
        let mut p = crate::model::plan(&a, &spec).unwrap();

        for v in &mut p.views {
            v.sheet = 2;
        }
        let with = furniture_bounds(&a, &p.sheet, &p.views);
        p.views.retain(|v| v.detail.is_none());
        assert_eq!(with, furniture_bounds(&a, &p.sheet, &p.views));
    }

    fn candidate(scale: f64, width: f64, fill: f64) -> (f64, f64, f64) {
        (scale, width, fill)
    }
    fn c(sheet: usize, scale: f64, fill: f64) -> Candidate {
        x(sheet, scale, fill, 0)
    }
    fn x(sheet: usize, scale: f64, fill: f64, extra_crossings: usize) -> Candidate {
        Candidate {
            sheet,
            scale,
            width: crate::layout::PAGES[sheet].1,
            fill,
            extra_crossings,
        }
    }
    #[test]
    fn band_prefers_scale_then_smaller_paper_otherwise_fullest() {
        let best = candidate(2.0, 297.0, 0.60);
        assert!(rank(best, candidate(1.0, 297.0, 0.85)).is_lt());
        assert!(rank(best, candidate(2.0, 420.0, 0.80)).is_lt());
        assert!(rank(best, candidate(5.0, 420.0, 0.90)).is_lt());
        assert!(rank(candidate(1.0, 297.0, 0.59), candidate(2.0, 420.0, 0.40)).is_lt());
    }
    #[test]
    fn neighbour_requires_enlargement_and_two_steps_are_forbidden() {
        assert!(allowed((0, 1.0), 0, 0.5));
        assert!(!allowed((0, 1.0), 1, 1.0));
        assert!(allowed((0, 1.0), 1, 2.0));
        assert!(!allowed((0, 1.0), 2, 5.0));
    }
    /// 10060 as measured: A4 1:1 17.0% and A3 2:1 14.9%, both valid and below the band.
    #[test]
    fn below_the_band_the_fullest_valid_pair_wins_on_less_paper() {
        let set = [c(0, 1.0, 0.170), c(1, 2.0, 0.149)];
        assert_eq!(choose(&set, false, true), Some((Some(0), (0, 1.0))));
    }
    #[test]
    fn only_the_smallest_fits_largest_scale_and_one_enlarged_step_compete() {
        // A4 1:1 fits. A4 1:2, A3 1:1 and every A2 pair are not allowed, however full.
        let set = [
            c(0, 1.0, 0.30),
            c(0, 0.5, 0.31),
            c(1, 1.0, 0.70),
            c(1, 2.0, 0.40),
            c(2, 5.0, 0.75),
        ];
        assert_eq!(choose(&set, false, true), Some((Some(3), (0, 1.0))));
        // In the band, the larger scale wins, then the smaller sheet.
        let set = [c(0, 1.0, 0.65), c(1, 2.0, 0.62)];
        assert_eq!(choose(&set, false, true).unwrap().0, Some(1));
        let set = [c(0, 2.0, 0.65), c(1, 2.0, 0.70)];
        assert_eq!(choose(&set, false, true).unwrap().0, Some(0));
    }
    /// assembly_plate_pin as measured: a valid A3 1:2 adding two crossings still defines the
    /// smallest fit, so A1 is never reached; A2 1:1, its allowed enlargement, is chosen.
    /// spool_fits: every allowed pair adds crossings, so the valid first estimate is kept.
    #[test]
    fn a_material_veto_never_moves_the_smallest_fit() {
        let set = [
            x(1, 0.5, 0.194, 2),
            x(2, 1.0, 0.245, 0),
            x(2, 0.5, 0.096, 1),
            x(3, 2.0, 0.251, 0),
        ];
        assert_eq!(choose(&set, false, true), Some((Some(1), (1, 0.5))));
        // An invalid first estimate's crossing count vetoes nothing.
        assert_eq!(choose(&set, false, false), Some((Some(1), (1, 0.5))));
        let spool = [
            x(0, 0.2, 0.226, 4),
            x(1, 0.5, 0.311, 2),
            x(2, 1.0, 0.475, 0),
        ];
        assert_eq!(choose(&spool, false, true), Some((None, (0, 0.2))));
        assert_eq!(choose(&spool, false, false), Some((Some(1), (0, 0.2))));
    }
    #[test]
    fn a_named_sheet_chooses_its_scale_by_fill() {
        let set = [c(2, 1.0, 0.30), c(2, 0.5, 0.12)];
        assert_eq!(choose(&set, true, true).unwrap().0, Some(0));
        assert_eq!(choose(&[], true, true), None);
    }
}
