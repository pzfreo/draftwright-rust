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
        .max(ABSOLUTE_FLOOR);
    crate::layout::SCALES
        .into_iter()
        .rev()
        .find(|s| *s >= needed)
        .unwrap_or(needed)
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
            let view = haecceity::hlr::View::new(v.direction, v.up).expect("planned view");
            let (size, _) = crate::model::extent(analysis, &view);
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

/// Bounded measured correction for a free orthographic row with no page-anchored annotations.
/// A failed estimate can leave an orientation view cramped below an otherwise clear row.
/// Lift the entire row together (preserving projection alignment and every dimension), then
/// fit the orientation view against the measured blocks, once. Never move pinned content.
pub(crate) fn correct_estimate(analysis: &Analysis, plan: &mut Plan) {
    if !plan.problems.iter().any(|p| p.code == "page_fit_uncertain")
        || crate::placement_intent::has_pins(&plan.spec)
        || plan.spec.iso_scale_factor.is_some()
        || plan.spec.views.contains_key("iso")
        || !plan.gdt.is_empty()
        || !plan.balloons.is_empty()
        || plan.hole_table.is_some()
        || !plan.notes.is_empty()
        || !plan.tables.is_empty()
        || plan.requirements.iter().any(|r| r.placed.is_some())
        || plan.views.iter().any(|v| v.cut.is_some())
        || plan
            .dimensions
            .iter()
            .any(|d| !matches!(d.kind.as_str(), "linear" | "callout"))
    {
        return;
    }
    let Some(iso_index) = plan.views.iter().position(|v| v.id == "v:iso") else {
        return;
    };
    let bounds = footprints(analysis, plan);
    let mut groups: Vec<Rect> = bounds[..plan.views.len()].to_vec();
    for d in plan.dimensions.iter().filter(|d| d.placed.is_some()) {
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
    let top = groups
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != iso_index)
        .map(|(_, b)| b[3])
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

/// Pack already settled GD&T view blocks onto a smaller page without re-routing witnesses or
/// leaders. The template is built at the same scale on A0; only owned paper geometry moves.
/// Page-anchored blocks and authored view relations stay with the existing planner.
pub(crate) fn pack_gdt(analysis: &Analysis, template: &Plan, recipient: &Plan) -> Option<Plan> {
    if template.spec.iso_scale_factor.is_some()
        || template.gdt.is_empty()
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
    let footprints = footprints(analysis, template);
    let count = template.views.len();
    let mut groups = footprints[..count].to_vec();
    let owners = template
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
    let common = [MARGIN + 1.0 - left, MARGIN + 1.0 - row_bottom];
    for i in &column {
        shifts[*i] = common;
    }
    if let Some(i) = column
        .iter()
        .copied()
        .find(|i| template.views[*i].id == "v:plan")
        && i != root
    {
        shifts[i][1] += groups[root][3] + 6.0 - groups[i][1];
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
        let scale = crate::layout::keep_sheet_scale(iso.scale.min(0.9 * fit), sheet_scale);
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

#[cfg(test)]
mod tests {
    use super::*;
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
