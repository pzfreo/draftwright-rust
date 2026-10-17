//! Finished-item footprints for sheet choice; reserved layout bands are never fill.
use crate::layout::{MARGIN, Rect};
use crate::model::{Analysis, Plan};

/// Paper changes must preserve the established callout quality in paper mm.
/// Use the same 2 mm length allowance, 1 mm label/extension pad and 0.3 mm
/// parallel-ink strip as the rendered leader acceptance. Projected edges measure
/// ambiguous parallel travel; exact annotation/feature association findings are
/// guarded independently. Each page is independent.
pub(crate) fn routes_no_worse(analysis: &Analysis, before: &Plan, after: &Plan) -> bool {
    routes_no_worse_for(analysis, before, after, None)
}

fn routes_no_worse_for(
    analysis: &Analysis,
    before: &Plan,
    after: &Plan,
    only: Option<&str>,
) -> bool {
    route_refusal_for(analysis, before, after, only).is_none()
}

pub(crate) fn route_refusal(
    analysis: &Analysis,
    before: &Plan,
    after: &Plan,
) -> Option<&'static str> {
    route_refusal_for(analysis, before, after, None)
}

fn route_refusal_for(
    analysis: &Analysis,
    before: &Plan,
    after: &Plan,
    only: Option<&str>,
) -> Option<&'static str> {
    let length =
        |p: &crate::dims::Placed| p.lines.iter().map(|s| distance(s[0], s[1])).sum::<f64>();
    let global_floor_recovery = std::cell::OnceCell::new();
    let mut changed = Vec::new();
    for d in after
        .dimensions
        .iter()
        .filter(|d| d.kind == "callout" && only.is_none_or(|id| id == d.id))
    {
        let Some(p) = &d.placed else { continue };
        let Some(old) = before.dimensions.iter().find(|o| o.id == d.id) else {
            continue;
        };
        let Some(q) = &old.placed else { continue };
        if length(p) > length(q) + 2.0 + 1e-6
            && !*global_floor_recovery.get_or_init(|| {
                // The global feature/depiction floor requires the whole model
                // to grow: a detail cannot open repetitive ink in every principal
                // view. Clear travel to the enlarged exterior can then grow too.
                // Every contact/crossing/run gate remains independent.
                let floor = min_legible_scale(analysis)
                    .max(crate::sheets::floor(&after.spec).unwrap_or(0.0));
                let old = readability(analysis, before);
                let new = readability(analysis, after);
                before.sheet.scale_factor + 1e-6 < floor
                    && after.sheet.scale_factor + 1e-6 >= floor
                    && new.failures() < old.failures()
                    && new.depiction <= old.depiction
                    && new.steps <= old.steps
                    && new.openings <= old.openings
                    && new.text_arrows <= old.text_arrows
                    && new.lanes <= old.lanes
            })
        {
            return Some("leader length exceeds retained route by more than 2 mm");
        }
        let (Some(v), Some(w)) = (
            after.views.iter().find(|v| v.id == d.view),
            before.views.iter().find(|v| v.id == old.view),
        ) else {
            return Some("callout has no carrying view");
        };
        let same = p.lines.len() == q.lines.len()
            && p.lines
                .iter()
                .flatten()
                .zip(q.lines.iter().flatten())
                .all(|(a, b)| (0..2).all(|k| ((a[k] - v.at[k]) - (b[k] - w.at[k])).abs() < 1e-6))
            && (0..4).all(|k| {
                ((p.label_box[k] - v.at[k % 2]) - (q.label_box[k] - w.at[k % 2])).abs() < 1e-6
            });
        changed.push((p, v.sheet, q, w.sheet, same));
    }
    if changed.is_empty() {
        return None;
    }
    let extensions = |plan: &Plan, sheet| {
        plan.dimensions
            .iter()
            .filter(|d| {
                plan.views
                    .iter()
                    .any(|v| v.id == d.view && v.sheet == sheet)
            })
            .filter_map(|d| d.placed.as_ref().map(|p| (d.id.as_str(), p)))
            .filter(|(_, p)| p.arrows.len() == 2 && p.lines.len() >= 3)
            .flat_map(|(id, p)| p.lines[..2].iter().map(move |s| (id.to_string(), *s)))
            .collect::<Vec<_>>()
    };
    let mut current_ink = std::collections::BTreeMap::new();
    let mut reference_ink = std::collections::BTreeMap::new();
    for (p, sheet, q, old_sheet, same) in changed {
        let a = extensions(after, sheet);
        let b = extensions(before, old_sheet);
        let contacts = |p: &crate::dims::Placed, ext: &[(String, [[f64; 2]; 2])]| {
            let pad = grow_route_box(p.label_box, 1.0);
            let mut all = std::collections::BTreeSet::new();
            let mut text = std::collections::BTreeSet::new();
            for (id, s) in ext {
                if route_touches(*s, pad) {
                    text.insert(id);
                    all.insert(id);
                }
                if p.lines
                    .iter()
                    .any(|l| route_touches(*l, grow_route_box(route_bounds(*s), 1.0)))
                {
                    all.insert(id);
                }
            }
            (all.len(), text.len())
        };
        let (total, text) = contacts(p, &a);
        let (old_total, old_text) = contacts(q, &b);
        if total > old_total || (!same && text > old_text) {
            return Some("leader/text gains a 1 mm padded extension-line contact");
        }
        // Match rendered acceptance: movement is measured in paper coordinates,
        // independent of scale. An unchanged pre-existing run is retained only
        // when its measured length does not grow.
        {
            let ink = current_ink
                .entry(sheet)
                .or_insert_with(|| indexed_route_ink(analysis, after, sheet, &a));
            let old_ink = reference_ink
                .entry(old_sheet)
                .or_insert_with(|| indexed_route_ink(analysis, before, old_sheet, &b));
            if p.lines.iter().enumerate().any(|(i, s)| {
                let run = ink.run(*s);
                run > 2.0 + 1e-6 && (!same || run > old_ink.run(q.lines[i]) + 1e-6)
            }) {
                return Some(
                    "moved leader runs for more than 2 mm within 0.3 mm of model/extension ink",
                );
            }
        }
    }
    None
}

/// Preserve a short proven paper-space leader when its feature grows. The new
/// proven attachment maps into the new ordinary view; the old elbow/label offset
/// is transported in paper millimetres. This is a bounded alternative, accepted only after all ordinary
/// placement, information and route gates are repeated.
pub(crate) fn retain_callouts(analysis: &Analysis, reference: &Plan, plan: &mut Plan) {
    retain_callouts_with(analysis, reference, plan, true);
    if !routes_no_worse(analysis, reference, plan) {
        retain_callouts_with(analysis, reference, plan, false);
    }
}

fn retain_callouts_with(
    analysis: &Analysis,
    reference: &Plan,
    plan: &mut Plan,
    shorten_current: bool,
) {
    if routes_no_worse(analysis, reference, plan) {
        return;
    }
    let worse: std::collections::BTreeSet<_> = plan
        .dimensions
        .iter()
        .filter(|d| d.kind == "callout")
        .filter(|d| !routes_no_worse_for(analysis, reference, plan, Some(&d.id)))
        .map(|d| d.id.clone())
        .collect();
    let mut trial = plan.clone();
    let mut changed = false;
    for d in &mut trial.dimensions {
        if !worse.contains(&d.id)
            || trial
                .spec
                .dimensions
                .get(&d.id)
                .is_some_and(|e| e.place.is_some())
        {
            continue;
        }
        let Some(old) = reference
            .dimensions
            .iter()
            .find(|old| old.id == d.id && old.terms == d.terms && old.view == d.view)
        else {
            continue;
        };
        let (Some(p), Some(q)) = (&mut d.placed, &old.placed) else {
            continue;
        };
        let (Some(line), Some(old_line)) = (p.lines.first(), q.lines.first()) else {
            continue;
        };
        if shorten_current && p.lines.len() == 2 && q.lines.len() == 2 {
            let length =
                |p: &crate::dims::Placed| p.lines.iter().map(|s| distance(s[0], s[1])).sum::<f64>();
            let shaft = distance(p.lines[0][0], p.lines[0][1]);
            let wanted = length(q) + 2.0 - distance(p.lines[1][0], p.lines[1][1]);
            if wanted >= 2.7 && wanted < shaft - 1e-6 {
                let delta =
                    [0, 1].map(|k| (p.lines[0][1][k] - p.lines[0][0][k]) * (wanted / shaft - 1.0));
                for k in 0..2 {
                    p.lines[0][1][k] += delta[k];
                    p.label_at[k] += delta[k];
                    p.label_box[k] += delta[k];
                    p.label_box[k + 2] += delta[k];
                }
                for at in &mut p.lines[1] {
                    for k in 0..2 {
                        at[k] += delta[k];
                    }
                }
                changed = true;
                continue;
            }
        }
        let (Some(view), Some(old_view)) = (
            trial.views.iter().find(|v| v.id == d.view),
            reference.views.iter().find(|v| v.id == old.view),
        ) else {
            continue;
        };
        if view.direction != old_view.direction || view.up != old_view.up {
            continue;
        }
        // Retain the proven attachment itself. A new router can select the
        // opposite rim; translating the old outgoing ray from that other rim
        // would take the shaft through the part. Ordinary view centres share
        // the same model projection, so this affine map preserves the site.
        let tip = if view.detail.is_none()
            && old_view.detail.is_none()
            && view.cut.is_none()
            && old_view.cut.is_none()
        {
            [0, 1].map(|k| {
                view.at[k] + (old_line[0][k] - old_view.at[k]) * view.scale / old_view.scale
            })
        } else {
            line[0]
        };
        let delta = [0, 1].map(|k| tip[k] - old_line[0][k]);
        *p = q.clone();
        for at in p.lines.iter_mut().chain(&mut p.arrows).flatten() {
            for k in 0..2 {
                at[k] += delta[k];
            }
        }
        for k in 0..2 {
            p.label_at[k] += delta[k];
            p.label_box[k] += delta[k];
            p.label_box[k + 2] += delta[k];
        }
        changed = true;
    }
    if !changed || !routes_no_worse(analysis, reference, &trial) {
        return;
    }
    let checked = crate::dims::recheck(analysis, &trial);
    let codes: std::collections::BTreeSet<_> = crate::dims::recheck(analysis, plan)
        .into_iter()
        .chain(checked.iter().cloned())
        .map(|q| q.code)
        .collect();
    trial.problems.retain(|q| !codes.contains(&q.code));
    trial.problems.extend(checked);
    enclose_free_ink(analysis, &mut trial);
    if routes_no_worse(analysis, reference, &trial)
        && crate::model::no_new_drawing_issues(reference, &trial)
        && crate::model::no_new_drawing_issues(plan, &trial)
        && crate::model::drawn_sheets_fit(analysis, &trial)
        && crate::model::carries_required(plan, &trial)
        && readability(analysis, &trial).failures() <= readability(analysis, plan).failures()
    {
        trial.status = crate::completeness::status(&trial.problems);
        trial.sheet.fill = fill(analysis, &trial);
        *plan = trial;
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn grow_route_box(b: Rect, pad: f64) -> Rect {
    [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]
}
fn route_bounds(s: [[f64; 2]; 2]) -> Rect {
    [
        s[0][0].min(s[1][0]),
        s[0][1].min(s[1][1]),
        s[0][0].max(s[1][0]),
        s[0][1].max(s[1][1]),
    ]
}
fn route_touches(s: [[f64; 2]; 2], b: Rect) -> bool {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        let d = s[1][k] - s[0][k];
        if d.abs() < 1e-12 {
            if s[0][k] < b[k] || s[0][k] > b[k + 2] {
                return false;
            }
        } else {
            let a = (b[k] - s[0][k]) / d;
            let z = (b[k + 2] - s[0][k]) / d;
            lo = lo.max(a.min(z));
            hi = hi.min(a.max(z));
        }
    }
    lo <= hi
}
// Index only accelerates the existing finite-strip calculation. Long lines and
// wide query boxes use the exhaustive path, bounding memory without omitting ink.
struct RouteInk {
    segments: Vec<[[f64; 2]; 2]>,
    cells: std::collections::BTreeMap<[i32; 2], Vec<usize>>,
    long: Vec<usize>,
}
impl RouteInk {
    const CELL: f64 = 10.0;
    const MAX_CELLS: i64 = 256;

    fn cells(b: [f64; 4]) -> ([i32; 2], [i32; 2], i64) {
        let lo = [0, 1].map(|k| (b[k] / Self::CELL).floor() as i32);
        let hi = [0, 1].map(|k| (b[k + 2] / Self::CELL).floor() as i32);
        let count = (i64::from(hi[0]) - i64::from(lo[0]) + 1)
            .saturating_mul(i64::from(hi[1]) - i64::from(lo[1]) + 1);
        (lo, hi, count)
    }

    fn new(segments: Vec<[[f64; 2]; 2]>) -> Self {
        let mut ink = Self {
            segments,
            cells: Default::default(),
            long: Vec::new(),
        };
        for (i, s) in ink.segments.iter().enumerate() {
            let (lo, hi, count) = Self::cells(route_bounds(*s));
            if count > Self::MAX_CELLS {
                ink.long.push(i);
                continue;
            }
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    ink.cells.entry([x, y]).or_default().push(i);
                }
            }
        }
        ink
    }

    fn run(&self, route: [[f64; 2]; 2]) -> f64 {
        let (lo, hi, count) = Self::cells(grow_route_box(route_bounds(route), 0.3));
        if count > Self::MAX_CELLS {
            return route_run(route, &self.segments);
        }
        let mut indices: std::collections::BTreeSet<_> = self.long.iter().copied().collect();
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                if let Some(cell) = self.cells.get(&[x, y]) {
                    indices.extend(cell);
                }
            }
        }
        let nearby: Vec<_> = indices.into_iter().map(|i| self.segments[i]).collect();
        route_run(route, &nearby)
    }
}

fn indexed_route_ink(
    analysis: &Analysis,
    plan: &Plan,
    sheet: u32,
    extensions: &[(String, [[f64; 2]; 2])],
) -> std::sync::Arc<RouteInk> {
    // The Analysis owns all model evidence. Only these view transforms/crops and
    // actual extension strokes can change this sheet's projected route obstacle.
    let views: Vec<_> = plan
        .views
        .iter()
        .filter(|v| v.sheet == sheet)
        .map(|v| {
            (
                &v.id,
                v.sheet,
                v.direction,
                v.up,
                v.at,
                v.scale,
                &v.cut,
                &v.detail,
            )
        })
        .collect();
    let key = serde_json::to_vec(&(sheet, views, extensions)).expect("route ink inputs serialize");
    crate::planning::memo(analysis, "indexed-sheet-route-ink", key, || {
        let mut segments = route_ink(analysis, plan, sheet);
        segments.extend(extensions.iter().map(|(_, s)| *s));
        std::sync::Arc::new(RouteInk::new(segments))
    })
}

fn route_ink(analysis: &Analysis, plan: &Plan, sheet: u32) -> Vec<[[f64; 2]; 2]> {
    let Some(ink) = &analysis.edge_ink else {
        return Vec::new();
    };
    plan.views
        .iter()
        .filter(|v| {
            v.sheet == sheet
                && v.id != "v:iso"
                && v.cut.is_none()
                && v.detail.as_ref().is_none_or(|d| d.source_cut.is_none())
        })
        .flat_map(|v| {
            let frame = haecceity::hlr::View::new(v.direction, v.up).expect("planned view");
            let direction_norm = v.direction.iter().map(|x| x * x).sum::<f64>().sqrt();
            let toward = v.direction.map(|x| x / direction_norm);
            let (size, centre) = if v.detail.is_some() {
                crate::detail::crop_extent(analysis, v)
            } else {
                crate::model::extent(analysis, &frame)
            };
            let page = |p| {
                let q = frame.map(p);
                [0, 1].map(|k| v.at[k] + v.scale * (q[k] - centre[k]))
            };
            let mut lines: Vec<_> = ink
                .edges
                .iter()
                .flat_map(|e| e.windows(2).map(|s| [page(s[0]), page(s[1])]))
                .collect();
            // Principal-face evidence already includes tangent generators for
            // cones and other analytic curved faces. They are not B-rep edges;
            // omitting them can approve a leader running along a chamfer.
            lines.extend(analysis.principal_profiles.iter().flat_map(|source| {
                source
                    .boundary_segments
                    .iter()
                    .map(|s| [page(s[0]), page(s[1])])
            }));
            // A cylinder's side generators are silhouettes, not B-rep edges. Use
            // the same square-on evidence as the existing callout route refiner.
            for face in &ink.curved {
                let Some(c) = &face.cylinder else { continue };
                let norm = c.axis.iter().map(|x| x * x).sum::<f64>().sqrt();
                if norm < 1e-12 {
                    continue;
                }
                let axis = c.axis.map(|x| x / norm);
                let along: f64 = (0..3).map(|k| axis[k] * toward[k]).sum();
                if along.abs() >= 1e-9 {
                    continue;
                }
                let [lo, hi] = face.bounds;
                let heights = (0..8)
                    .map(|i| {
                        (0..3)
                            .map(|k| {
                                ((if i >> k & 1 == 0 { lo[k] } else { hi[k] }) - c.origin[k])
                                    * axis[k]
                            })
                            .sum::<f64>()
                    })
                    .collect::<Vec<_>>();
                let h0 = heights.iter().copied().fold(f64::INFINITY, f64::min);
                let h1 = heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let normal = [
                    axis[1] * toward[2] - axis[2] * toward[1],
                    axis[2] * toward[0] - axis[0] * toward[2],
                    axis[0] * toward[1] - axis[1] * toward[0],
                ];
                for sign in [-1.0, 1.0] {
                    let at = |h| {
                        [0, 1, 2].map(|k| c.origin[k] + h * axis[k] + sign * c.radius * normal[k])
                    };
                    let a = at(h0);
                    let b = at(h1);
                    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
                    // Trimmed cylinders need not include both generators. Clip
                    // to their evidence box, allowing only its rounding grid.
                    for k in 0..3 {
                        let delta = b[k] - a[k];
                        let min = lo[k] - crate::model::projected_ink::GRID;
                        let max = hi[k] + crate::model::projected_ink::GRID;
                        if delta.abs() < 1e-12 {
                            if a[k] < min || a[k] > max {
                                t1 = -1.0;
                                break;
                            }
                        } else {
                            let x = (min - a[k]) / delta;
                            let y = (max - a[k]) / delta;
                            t0 = t0.max(x.min(y));
                            t1 = t1.min(x.max(y));
                        }
                    }
                    if t0 <= t1 {
                        let point = |t| page([0, 1, 2].map(|k| a[k] + t * (b[k] - a[k])));
                        lines.push([point(t0), point(t1)]);
                    }
                }
            }
            if v.detail.is_some() {
                let rect = [
                    v.at[0] - size[0] * v.scale / 2.0,
                    v.at[1] - size[1] * v.scale / 2.0,
                    v.at[0] + size[0] * v.scale / 2.0,
                    v.at[1] + size[1] * v.scale / 2.0,
                ];
                lines = lines
                    .into_iter()
                    .flat_map(|s| crate::detail::clip(crate::output::Curve::Line(s[0], s[1]), rect))
                    .filter_map(|c| match c {
                        crate::output::Curve::Line(a, b) => Some([a, b]),
                        _ => None,
                    })
                    .collect();
            }
            lines
        })
        .collect()
}
fn route_run(a: [[f64; 2]; 2], ink: &[[[f64; 2]; 2]]) -> f64 {
    let d = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
    let mut intervals = Vec::new();
    let near = grow_route_box(route_bounds(a), 0.3);
    for b in ink {
        if (0..2).any(|k| b[0][k].max(b[1][k]) < near[k] || b[0][k].min(b[1][k]) > near[k + 2]) {
            continue;
        }
        let w = [b[1][0] - b[0][0], b[1][1] - b[0][1]];
        let n = w[0] * w[0] + w[1] * w[1];
        if n < 1e-18 || (d[0] * w[1] - d[1] * w[0]).abs() > 0.3 * distance(a[0], a[1]) * n.sqrt() {
            continue;
        }
        let q = [a[0][0] - b[0][0], a[0][1] - b[0][1]];
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for (at, slope, min, max) in [
            (q[0] * w[0] + q[1] * w[1], d[0] * w[0] + d[1] * w[1], 0.0, n),
            (
                q[0] * w[1] - q[1] * w[0],
                d[0] * w[1] - d[1] * w[0],
                -0.3 * n.sqrt(),
                0.3 * n.sqrt(),
            ),
        ] {
            if slope.abs() < 1e-15 {
                if at < min || at > max {
                    hi = -1.0;
                    break;
                }
            } else {
                let x = (min - at) / slope;
                let y = (max - at) / slope;
                lo = lo.max(x.min(y));
                hi = hi.min(x.max(y));
            }
        }
        if hi > lo {
            intervals.push((lo, hi));
        }
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut start, mut stop, mut longest) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (lo, hi) in intervals {
        if lo > stop {
            start = lo;
        }
        stop = stop.max(hi);
        longest = longest.max(stop - start);
    }
    longest * distance(a[0], a[1])
}

/// A primary interval cannot become readable by repacking annotations. Defer
/// this preflight whenever an authored/automatic crop may change its owner.
/// The finished drawing remains authoritative; this only rejects a proved floor
/// failure before the expensive layout solves.
pub(crate) fn unavoidable_steps_at_scale(
    analysis: &Analysis,
    spec: &crate::model::Spec,
    plan: &Plan,
    scale: f64,
    primary_only: bool,
) -> usize {
    let requests = crate::detail::requests(analysis, spec, scale);
    if !requests.is_empty() {
        // Keep the historical reference and authored crops unchanged. An
        // automatic crop whose bare model ink exceeds this paper cannot own a
        // step on any auxiliary page either; packing cannot make it smaller.
        let paper = spec
            .sheet
            .size
            .as_ref()
            .and_then(|name| crate::layout::PAGES.iter().find(|p| p.0 == name));
        if primary_only
            || requests.iter().any(|(_, d)| d.authored)
            || paper.is_none_or(|(_, w, h)| {
                requests.iter().any(|(_, d)| {
                    let [x, y] = crate::detail::size(d);
                    x <= w - 2.0 * MARGIN && y <= h - 2.0 * MARGIN
                })
            })
        {
            return 0;
        }
    }
    let mut probe = plan.clone();
    for view in probe.views.iter_mut().filter(|v| v.detail.is_none()) {
        view.scale = scale;
    }
    if primary_only {
        primary_illegible_steps(analysis, &probe)
    } else {
        readability(analysis, &probe).steps
    }
}

/// Usable-paper fill target. The lower bound triggers enlargement on the same
/// paper (including allocation); it never buys a larger sheet (2026-10-17).
pub const FILL_BAND: [f64; 2] = [0.60, 0.85];
/// Clearance around each drawn item, page mm.
pub const FILL_CLEARANCE: f64 = 1.0;
/// Minimum visible opening across a hole or slot, page mm. Small edge treatments do
/// not set the whole-sheet scale; their sizes are stated by callouts.
pub const MIN_OPENING: f64 = 2.0;
/// The smallest scale the automatic choice offers, whatever the part (1:10).
pub const ABSOLUTE_FLOOR: f64 = 0.1;

pub fn min_legible_scale(analysis: &Analysis) -> f64 {
    scale_floor(analysis, MIN_OPENING, modelled_line_scale(analysis))
}

// Preserve the established frame preparation at explicit and refused scales.
// This historical layout heuristic never qualifies a drawing as readable.
pub(crate) fn preparation_scale(analysis: &Analysis) -> f64 {
    scale_floor(analysis, 0.8, 0.0)
}

fn scale_floor(analysis: &Analysis, opening_floor: f64, depiction_floor: f64) -> f64 {
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
    let needed = (opening_floor / opening)
        .max(20.0 / longest.max(1e-12))
        .max(depiction_floor)
        .max(ABSOLUTE_FLOOR);
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
    illegible_spans(analysis, plan, false)
}

pub(crate) fn primary_illegible_steps(analysis: &Analysis, plan: &Plan) -> usize {
    illegible_spans(analysis, plan, true)
}

fn illegible_spans(analysis: &Analysis, plan: &Plan, primary_only: bool) -> usize {
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
                if primary_only
                    && !matches!(m.parameter.as_str(), "step.length" | "step_position.length")
                {
                    return None;
                }
                let length = if m.parameter.starts_with("through_step_leg.length")
                    || m.parameter == "circular_step_depth.length"
                    || m.parameter.starts_with("step_height.length")
                {
                    m.value.abs()
                } else {
                    if !matches!(m.parameter.as_str(), "step.length" | "step_position.length") {
                        return None;
                    }
                    let f = analysis
                        .features
                        .iter()
                        .find(|f| f.id == m.feature && f.kind == "turned_step")?;
                    f.parameters.get("hi")?.as_f64()? - f.parameters.get("lo")?.as_f64()?
                };
                let floor = if m.parameter == "step.length" || m.parameter == "step_position.length"
                {
                    MIN_STEP_SPAN
                } else {
                    MIN_OPENING
                };
                (length.is_finite() && length > 0.0 && length * scale + 1e-6 < floor)
                    .then_some(m.feature.as_str())
            })
        })
        .filter(|feature| checked.insert(*feature))
        .count()
}

/// Thin depiction strokes need visible white between them on an ordinary printer.
/// This also applies to modelled repetitive ink, without inventing a thread callout.
pub const MIN_DEPICTION_GAP: f64 = 0.7;

/// Repeated coaxial cylindrical strips are evidence of modelled repetitive ink.
/// Require four distinct starts to avoid treating ordinary shoulders as threads.
/// Use the axial repeat and root/crest separation, not tessellation chord lengths.
fn modelled_line_scale(analysis: &Analysis) -> f64 {
    crate::planning::memo(analysis, "sheet-depiction-scale", vec![], || {
        modelled_line_scale_uncached(analysis)
    })
}

fn modelled_line_scale_uncached(analysis: &Analysis) -> f64 {
    let Some(ink) = &analysis.edge_ink else {
        return 0.0;
    };
    let mut groups = std::collections::BTreeMap::<[i64; 4], Vec<[f64; 3]>>::new();
    for face in &ink.curved {
        let Some(c) = &face.cylinder else { continue };
        let Some(axis) = (0..3).find(|&k| c.axis[k].abs() > 1.0 - 1e-6) else {
            continue;
        };
        let mut key = c.origin.map(|x| (x * 1000.0).round() as i64);
        key[axis] = 0;
        groups
            .entry([axis as i64, key[0], key[1], key[2]])
            .or_default()
            .push([face.bounds[0][axis], face.bounds[1][axis], c.radius]);
    }
    let mut minimum = f64::INFINITY;
    for (key, faces) in &groups {
        let mut radii: Vec<_> = faces.iter().map(|f| f[2]).collect();
        radii.sort_by(f64::total_cmp);
        radii.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        for radius in radii {
            let mut starts: Vec<_> = faces
                .iter()
                .filter(|f| (f[2] - radius).abs() < 0.01)
                .map(|f| f[0])
                .collect();
            starts.sort_by(f64::total_cmp);
            starts.dedup_by(|a, b| (*a - *b).abs() < 0.01);
            if starts.len() < 4 {
                continue;
            }
            // Consecutive repeats must agree, so an arbitrary stack of shoulders is not ink density.
            for four in starts.windows(4) {
                let pitch = (four[3] - four[0]) / 3.0;
                if pitch <= 0.01
                    || !four
                        .windows(2)
                        .all(|w| ((w[1] - w[0]) - pitch).abs() < 0.02 * pitch)
                {
                    continue;
                }
                minimum = minimum.min(pitch);
                // A modelled helical crest need not itself be a cylinder. Its
                // curved-face bounds still prove the radial white-space limit.
                let axis = key[0] as usize;
                let crest = ink
                    .curved
                    .iter()
                    .filter(|f| f.bounds[0][axis] <= four[3] && f.bounds[1][axis] >= four[0])
                    .map(|face| {
                        (0..3)
                            .filter(|&k| k != axis)
                            .map(|k| {
                                let centre = key[k + 1] as f64 / 1000.0;
                                (face.bounds[0][k] - centre)
                                    .abs()
                                    .max((face.bounds[1][k] - centre).abs())
                            })
                            .fold(0.0, f64::max)
                    })
                    .fold(radius, f64::max);
                if crest > radius + 0.01 {
                    minimum = minimum.min(crest - radius);
                }
                for face in faces
                    .iter()
                    .filter(|f| f[0] <= four[3] && f[1] >= four[0] && f[2] > radius + 0.01)
                {
                    minimum = minimum.min(face[2] - radius);
                }
            }
        }
    }
    MIN_DEPICTION_GAP / minimum
}

/// Measured failures in the owning views, in paper millimetres. Counts stay
/// separate so a larger-paper decision can name its actual readability failure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Readability {
    pub steps: usize,
    pub openings: usize,
    pub text_arrows: usize,
    pub depiction: usize,
    pub lanes: usize,
}
impl Readability {
    pub fn failures(self) -> usize {
        self.steps + self.openings + self.text_arrows + self.depiction + self.lanes
    }
}
impl std::fmt::Display for Readability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} unreadable step spans, {} openings below 2 mm, {} text/arrow fit failures, {} depiction gaps below 0.7 mm, {} dimension-lane failures",
            self.steps, self.openings, self.text_arrows, self.depiction, self.lanes
        )
    }
}

/// Test settled ink rather than nominal text length: outside arrows and outside
/// labels are valid when they clear their own extension lines. Existing hard ink
/// checks supply clearance from other annotations, borders and foreign views.
pub fn readability(analysis: &Analysis, plan: &Plan) -> Readability {
    let mut result = Readability {
        steps: illegible_steps(analysis, plan),
        ..Default::default()
    };
    let mut openings = std::collections::BTreeSet::new();
    // A schedule's hole sizes still owe readable geometry in the balloon's
    // carrier view; moving their numbers into a table does not enlarge the holes.
    for balloon in &plan.balloons {
        let scale = plan
            .views
            .iter()
            .find(|v| v.id == balloon.view)
            .map_or(0.0, |v| v.scale);
        for id in &balloon.features {
            if let Some(feature) = analysis
                .features
                .iter()
                .find(|f| f.id == *id && f.kind == "hole")
                && let Some(size) = feature.parameters.get("diameter").and_then(|v| v.as_f64())
                && size > 0.0
                && size * scale + 1e-6 < MIN_OPENING
            {
                openings.insert(feature.id.as_str());
            }
        }
    }
    let mut rows = Vec::new();
    let mut levels = std::collections::BTreeMap::<&str, Vec<f64>>::new();
    let mut locations = std::collections::BTreeMap::<(&str, &str), Vec<f64>>::new();
    let modelled = modelled_line_scale(analysis);
    let manufacturing = crate::planning::memo(analysis, "sheet-depictions", vec![], || {
        crate::pmi_lowering::manufacturing(analysis)
    });
    for v in plan.views.iter().filter(|v| v.id != "v:iso") {
        if v.detail.is_none() && v.scale + 1e-6 < modelled {
            result.depiction += 1;
        }
        for depiction in manufacturing
            .owned
            .iter()
            .filter_map(|o| o.depiction.as_ref())
        {
            if v.direction[depiction.axis].abs() > 1e-6 || v.cut.is_some() {
                continue;
            }
            if let crate::pmi_lowering::DepictionKind::Thread { minor } = depiction.kind
                && v.line_style.thread_depiction != Some(false)
                && (depiction.diameter - minor) * 0.5 * v.scale + 1e-6 < MIN_DEPICTION_GAP
            {
                result.depiction += 1;
            }
            // Cosmetic knurls already enforce 3 mm paper spacing (cosmetic::ink).
        }
    }
    for d in &plan.dimensions {
        let Some(p) = &d.placed else { continue };
        let Some(v) = plan.views.iter().find(|v| v.id == d.view) else {
            continue;
        };
        // The existing ordinate legibility measure applies in the owning view,
        // including enlarged details; schedules remove the local ordinate stack.
        if d.kind == "linear"
            && let Some(m) = d.measurements.first()
            && matches!(
                m.parameter.as_str(),
                "location.length.x" | "location.length.y"
            )
            && !d
                .measurements
                .iter()
                .any(|m| m.feature.starts_with("f:hole_pattern:"))
        {
            locations
                .entry((v.id.as_str(), m.parameter.as_str()))
                .or_default()
                .push(d.value);
        }
        for m in &d.measurements {
            if m.feature == "step_level" && m.parameter.starts_with("step_height.length") {
                levels
                    .entry(v.id.as_str())
                    .or_default()
                    .push(m.value.abs() * v.scale);
            }
            if m.feature == "envelope" && m.parameter == "height.length" {
                levels
                    .entry(v.id.as_str())
                    .or_default()
                    .push(m.value.abs() * v.scale);
            }
            let Some(feature) = analysis.features.iter().find(|f| f.id == m.feature) else {
                continue;
            };
            let parameter = match feature.kind.as_str() {
                "hole" => "diameter",
                "slot" => "width",
                _ => continue,
            };
            if let Some(size) = feature.parameters.get(parameter).and_then(|n| n.as_f64())
                && size > 0.0
                && size * v.scale + 1e-6 < MIN_OPENING
            {
                openings.insert(feature.id.as_str());
            }
        }
        if p.lines.len() < 3 || p.arrows.len() != 2 || d.kind != "linear" {
            continue;
        }
        let a = p.arrows[0][0];
        let b = p.arrows[1][0];
        let delta = [b[0] - a[0], b[1] - a[1]];
        let span = delta[0].hypot(delta[1]);
        if span < 1e-6 {
            result.text_arrows += 1;
            continue;
        }
        // Project into the dimension's frame, including authored aligned spans.
        let sign = if delta[0] < -1e-6 || (delta[0].abs() < 1e-6 && delta[1] < 0.0) {
            -1.0
        } else {
            1.0
        };
        let u = [sign * delta[0] / span, sign * delta[1] / span];
        let n = [-u[1], u[0]];
        let along = |p: [f64; 2]| p[0] * u[0] + p[1] * u[1];
        let across = |p: [f64; 2]| p[0] * n[0] + p[1] * n[1];
        let lo = along(a).min(along(b));
        let hi = along(a).max(along(b));
        let text = if u[0].abs() < 1e-6 || u[1].abs() < 1e-6 {
            let axis = usize::from(u[1].abs() > u[0].abs());
            [p.label_box[axis], p.label_box[axis + 2]]
        } else {
            // An axis-aligned bounding box overestimates rotated text. Project
            // the actual label's four corners through its placement transform.
            let ink = crate::dims::label_ink(&d.terms, crate::dims::font());
            let (sin, cos) = p.rotation.to_radians().sin_cos();
            let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
            for x in [ink.min[0], ink.max[0]] {
                for y in [ink.min[1], ink.max[1]] {
                    let t = along([
                        p.label_at[0] + cos * x - sin * y,
                        p.label_at[1] + sin * x + cos * y,
                    ]);
                    bounds[0] = bounds[0].min(t);
                    bounds[1] = bounds[1].max(t);
                }
            }
            bounds
        };
        let inward = p.arrows.iter().all(|[tip, tail]| {
            (along(*tail) - along(*tip)) * ((lo + hi) * 0.5 - along(*tip)) > 0.0
        });
        let inset = if inward { crate::dims::ARROW } else { 0.0 };
        let inside = text[0] >= lo + inset - 0.1 && text[1] <= hi - inset + 0.1;
        let outside =
            text[1] <= lo - crate::dims::ARROW + 0.1 || text[0] >= hi + crate::dims::ARROW - 0.1;
        if (inward && hi - lo + 0.1 < 2.0 * crate::dims::ARROW) || (!inside && !outside) {
            result.text_arrows += 1;
        }
        // Interior dimensions deliberately use internal material space. Exterior
        // lanes must be achievable within the settled view reservation.
        if d.region != Some(crate::dims::CandidateRegion::Interior) {
            let row = across(a);
            if p.lines[..2]
                .iter()
                .any(|line| (row - across(line[0])).abs() + 1.0 + 0.1 < crate::dims::FIRST)
            {
                result.lanes += 1;
            }
            rows.push((d.view.as_str(), u, row, lo, hi));
        }
    }
    for values in levels.values_mut().filter(|v| v.len() > 1) {
        values.push(0.0);
        values.sort_by(f64::total_cmp);
        values.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        result.steps += values
            .windows(2)
            .filter(|w| w[1] - w[0] + 1e-6 < MIN_OPENING)
            .count();
    }
    for ((view, _), values) in locations {
        let scale = plan
            .views
            .iter()
            .find(|v| v.id == view)
            .map_or(0.0, |v| v.scale);
        result.lanes += crate::layout::crowded_locations(&values, scale);
    }
    result.openings = openings.len();
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            if a.0 == b.0
                && (a.1[0] * b.1[1] - a.1[1] * b.1[0]).abs() < 1e-6
                && a.3 < b.4
                && b.3 < a.4
            {
                let gap = (a.2 - b.2).abs();
                if gap + 0.1 < crate::dims::LANE {
                    result.lanes += 1;
                }
            }
        }
    }
    result
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
    if plan.sheet.number == 1 && plan.sheets.is_empty() && plan.views.iter().all(|v| v.sheet == 1) {
        return fill_local(analysis, plan);
    }
    // Page coordinates are independent. Supplementary geometry or a moved
    // table must not inflate the main page's fill by sharing its coordinates.
    let mut local = plan.clone();
    let number = plan.sheet.number;
    local.sheets.clear();
    local.views.retain(|v| v.sheet == number);
    let carries = |id: &str| local.views.iter().any(|v| v.id == id);
    local.dimensions.retain(|d| carries(&d.view));
    local.gdt.retain(|g| carries(&g.view));
    local.balloons.retain(|b| carries(&b.view));
    local.tables.retain(|t| t.sheet == number);
    local.hole_table = local.hole_table.filter(|t| t.sheet == number);
    local.hole_tables.retain(|t| t.sheet == number);
    if number > 1 {
        local.notes.clear();
        local.notes_block = None;
        local.radius_notes_block = None;
        local.radius_table = None;
        local.requirements.clear();
    }
    fill_local(analysis, &local)
}

fn fill_local(analysis: &Analysis, plan: &Plan) -> f64 {
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
        *plan = candidate;
    }
}

/// Reuse settled-block packing when an ordinary drawing's reserved bands
/// underestimate its annotation ink. Move whole carriers, never reroute a leader.
pub(crate) fn enclose_free_ink(analysis: &Analysis, plan: &mut Plan) {
    if !plan.sheets.is_empty()
        || plan.views.iter().any(|v| v.detail.is_some())
        || !plan
            .problems
            .iter()
            .any(|q| q.code == "annotation_outside_area")
    {
        return;
    }
    if let Some(candidate) = pack_settled_views(analysis, plan, plan)
        && crate::model::drawn_sheets_fit(analysis, &candidate)
        && crate::model::carries_required(plan, &candidate)
        && crate::model::no_new_drawing_issues(plan, &candidate)
        && crate::model::ink_findings(&candidate) < crate::model::ink_findings(plan)
        && readability(analysis, &candidate).failures() <= readability(analysis, plan).failures()
    {
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

/// Pack already settled annotated view blocks onto a page without re-routing witnesses or
/// leaders. The template is built at the same scale on A0; only owned paper geometry moves.
/// Page-anchored blocks and authored view relations stay with the existing planner.
pub(crate) fn pack_settled_views(
    analysis: &Analysis,
    template: &Plan,
    recipient: &Plan,
) -> Option<Plan> {
    if template.spec.iso_scale_factor.is_some()
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
        v.basis += "; settled annotations packed as one measured view block";
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

/// Paper is a real cost. Fill triggers a same-paper search, never a
/// comparison in which a marginal fill gain purchases a larger sheet.
pub(crate) fn rank(a: (f64, f64, f64), b: (f64, f64, f64)) -> std::cmp::Ordering {
    a.1.total_cmp(&b.1).then_with(|| b.0.total_cmp(&a.0))
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

/// Smallest clean readable paper, then largest standard scale. The caller tries
/// same-paper allocation before opening the next paper size. Material crossings
/// retain the existing quality ratchet against a valid seed.
pub(crate) fn choose(
    candidates: &[Candidate],
    _named_sheet: bool,
    seed_clean: bool,
) -> Option<(Option<usize>, (usize, f64))> {
    let smallest = candidates
        .iter()
        .map(|c| (c.sheet, c.scale))
        .min_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)))?;
    let chosen = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| !seed_clean || c.extra_crossings == 0)
        .min_by(|a, b| {
            rank(
                (a.1.scale, a.1.width, a.1.fill),
                (b.1.scale, b.1.width, b.1.fill),
            )
        })
        .map(|(i, _)| i);
    Some((chosen, smallest))
}

// Compatibility choice for a reported refusal ONLY. Qualified drawings always
// use the screen-first comparator above. Keeping the established fitted geometry
// when the stronger floor cannot be met prevents a readability change from
// replacing a published drawing with an incomplete or less readable raw seed.
fn retained_refusal_rank(a: (f64, f64, f64), b: (f64, f64, f64)) -> std::cmp::Ordering {
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

pub(crate) fn retained_refusal_choice(
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
                || (c.sheet == smallest.0 + 1 && c.scale > smallest.1)
        })
        .filter(|(_, c)| !seed_clean || c.extra_crossings == 0)
        .min_by(|a, b| retained_refusal_rank(key(a.1), key(b.1)))
        .map(|(i, _)| i);
    Some((chosen, smallest))
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
        && (plan.gdt.is_empty() || plan.sheet.scale_factor >= preparation_scale(analysis))
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
    fn indexed_route_strips_equal_exhaustive_runs() {
        let mut segments = Vec::new();
        for i in -40..=40 {
            let x = f64::from(i) * 0.5;
            segments.push([[x, -20.0], [x, 20.0]]);
            segments.push([[-20.0, x], [20.0, x]]);
            segments.push([[x, -20.0], [x + 20.0, 20.0]]);
        }
        // Wide geometry and wide queries must retain the exhaustive fallback.
        segments.push([[-10000.0, -10000.0], [10000.0, 10000.0]]);
        let ink = RouteInk::new(segments);
        for route in [
            [[-10.3, -12.0], [-10.3, 15.0]],
            [[-10.30001, -12.0], [-10.30001, 15.0]],
            [[-20.0, -20.0], [20.0, 20.0]],
            [[-1000.0, -1000.0], [1000.0, 1000.0]],
            [[-20.0, -0.3], [20.0, -0.3]],
            [[10.0, 10.0], [10.0, 10.0]],
        ] {
            assert!((ink.run(route) - route_run(route, &ink.segments)).abs() < 1e-9);
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn cylindrical_silhouettes_also_guard_parallel_leaders() {
        let (_, mut a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/mounting_plate.step"),
        )
        .unwrap();
        let mut p = crate::model::plan(&a, &crate::model::Spec::default()).unwrap();
        a.envelope = [[-1.0, -1.0, 0.0], [1.0, 1.0, 10.0]];
        a.principal_profiles.clear();
        a.edge_ink = Some(crate::model::projected_ink::EdgeInk {
            references: Default::default(),
            edges: Vec::new(),
            planes: Vec::new(),
            curved: vec![crate::model::projected_ink::CurvedFace {
                bounds: a.envelope,
                cylinder: Some(crate::model::projected_ink::Cylinder {
                    origin: [0.0; 3],
                    axis: [0.0, 0.0, 1.0],
                    radius: 1.0,
                }),
            }],
        });
        p.views.retain(|v| v.id == "v:front");
        let v = &mut p.views[0];
        v.direction = [0.0, -1.0, 0.0];
        v.up = [0.0, 0.0, 1.0];
        v.scale = 1.0;
        v.at = [50.0, 50.0];
        let ink = route_ink(&a, &p, 1);
        assert!((route_run([[49.2, 46.0], [49.2, 49.0]], &ink) - 3.0).abs() < 1e-6);
        assert_eq!(route_run([[49.31, 46.0], [49.31, 49.0]], &ink), 0.0);
        p.views[0].direction = [0.0, -2.0, 0.0];
        assert!(
            (route_run([[49.2, 46.0], [49.2, 49.0]], &route_ink(&a, &p, 1)) - 3.0).abs() < 1e-6
        );
        // A face which never reaches x = -radius has no left generator.
        a.edge_ink.as_mut().unwrap().curved[0].bounds[0][0] = 0.0;
        assert_eq!(
            route_run([[49.2, 46.0], [49.2, 49.0]], &route_ink(&a, &p, 1)),
            0.0
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn principal_tangents_guard_non_cylindrical_silhouettes() {
        let (_, mut a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/mounting_plate.step"),
        )
        .unwrap();
        let mut p = crate::model::plan(&a, &crate::model::Spec::default()).unwrap();
        a.envelope = [[-4.0, -4.0, 0.0], [4.0, 4.0, 10.0]];
        // A conical tangent has no corresponding B-rep edge or cylinder.
        a.edge_ink.as_mut().unwrap().edges.clear();
        a.edge_ink.as_mut().unwrap().curved.clear();
        a.principal_profiles = vec![crate::lint::profiles::Source {
            bounds: a.envelope,
            faces: Vec::new(),
            boundary_segments: vec![[[-1.0, 0.0, 1.0], [-4.0, 0.0, 4.0]]],
            curved_faces: Vec::new(),
        }];
        p.views.retain(|v| v.id == "v:front");
        let v = &mut p.views[0];
        v.direction = [0.0, -1.0, 0.0];
        v.up = [0.0, 0.0, 1.0];
        v.scale = 1.0;
        v.at = [50.0, 50.0];
        let ink = route_ink(&a, &p, 1);
        assert!(route_run([[49.1, 46.0], [46.1, 49.0]], &ink) > 4.0);
        assert_eq!(route_run([[49.5, 46.0], [46.5, 49.0]], &ink), 0.0);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn smaller_paper_is_not_its_own_route_reference() {
        let Some(corpus) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
            return;
        };
        let (_, a) = crate::model::analyse_native(
            &std::path::PathBuf::from(corpus).join("mfcadpp/10033.step"),
        )
        .unwrap();
        let p = crate::model::plan_recovered(&a, &crate::model::Spec::default()).unwrap();
        if p.sheet.size != "A4" {
            assert_eq!(
                (p.sheet.size.as_str(), p.sheet.scale.as_str()),
                ("A3", "2:1")
            );
            assert!(
                p.sheet
                    .basis
                    .contains("leader length exceeds retained route"),
                "{}",
                p.sheet.basis
            );
        }
        let id = "d:f:chamfer:2/chamfer.length";
        let q = p
            .dimensions
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .placed
            .as_ref()
            .unwrap();
        let baseline: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/leader_routes.json")).unwrap();
        let old_length = baseline["parts"]["10033"][id]["length"].as_f64().unwrap();
        assert!(
            q.lines.iter().map(|s| distance(s[0], s[1])).sum::<f64>() <= old_length + 2.0 + 1e-6
        );
    }

    #[test]
    fn leader_runs_join_adjacent_strokes_but_not_transverse_contacts() {
        let route = [[0.0, 0.0], [10.0, 0.0]];
        assert!(
            (route_run(route, &[[[1.0, 0.2], [2.0, 0.2]], [[2.0, 0.2], [4.0, 0.2]]]) - 3.0).abs()
                < 1e-6
        );
        assert_eq!(route_run(route, &[[[1.0, -2.0], [1.0, 2.0]]]), 0.0);
        assert_eq!(route_run(route, &[[[1.0, 0.31], [4.0, 0.31]]]), 0.0);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn paper_enlargement_cannot_buy_longer_or_obstructed_callouts() {
        let (_, a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/mounting_plate.step"),
        )
        .unwrap();
        let first = crate::model::plan(&a, &crate::model::Spec::default()).unwrap();
        assert!(routes_no_worse(&a, &first, &first));
        let mut longer = first.clone();
        let p = longer
            .dimensions
            .iter_mut()
            .find(|d| d.kind == "callout" && d.placed.is_some())
            .unwrap()
            .placed
            .as_mut()
            .unwrap();
        p.lines[1][1][0] += 50.0;
        assert!(!routes_no_worse(&a, &first, &longer));
        let mut obstructed = first.clone();
        let extension = first
            .dimensions
            .iter()
            .filter_map(|d| d.placed.as_ref())
            .find(|p| p.arrows.len() == 2 && p.lines.len() >= 3)
            .unwrap()
            .lines[0];
        let p = obstructed
            .dimensions
            .iter_mut()
            .find(|d| d.kind == "callout" && d.placed.is_some())
            .unwrap()
            .placed
            .as_mut()
            .unwrap();
        p.label_box = grow_route_box(route_bounds(extension), 0.1);
        assert!(!routes_no_worse(&a, &first, &obstructed));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn settled_band_repacking_preserves_every_owned_route_and_measurement() {
        let (_, a) = crate::model::analyse_native(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/mounting_plate.step"),
        )
        .unwrap();
        let spec = serde_json::from_value(serde_json::json!({
            "sheet":{"size":"A4","scale":"1:1"},
            "views":{"plan":{"omit":true},"side":{"omit":true},"iso":{"omit":true}}
        }))
        .unwrap();
        let mut p = crate::model::plan_recovered(&a, &spec).unwrap();
        assert_eq!(p.views.len(), 1);
        let b = p
            .dimensions
            .iter()
            .filter_map(crate::dims::ink_box)
            .max_by(|a, b| a[3].total_cmp(&b[3]))
            .unwrap();
        p.views[0].area[3] = b[3] - 0.25;
        p.problems.retain(|q| q.code != "annotation_outside_area");
        p.problems.extend(
            crate::dims::recheck_cached(&a, &p, &crate::dims::footprints::Cache::default())
                .into_iter()
                .filter(|q| q.code == "annotation_outside_area"),
        );
        assert!(
            p.problems
                .iter()
                .any(|q| q.code == "annotation_outside_area")
        );
        let relative = |p: &Plan| {
            p.dimensions
                .iter()
                .filter(|d| d.placed.is_some())
                .map(|d| {
                    let v = p.views.iter().find(|v| v.id == d.view).unwrap();
                    let mut d = d.clone();
                    if let Some(q) = &mut d.placed {
                        for pt in q
                            .lines
                            .iter_mut()
                            .chain(&mut q.arrows)
                            .flatten()
                            .chain(std::iter::once(&mut q.label_at))
                        {
                            pt[0] -= v.at[0];
                            pt[1] -= v.at[1];
                        }
                        for (k, x) in q.label_box.iter_mut().enumerate() {
                            *x -= v.at[k % 2];
                        }
                    }
                    d
                })
                .collect::<Vec<_>>()
        };
        let before = relative(&p);
        enclose_free_ink(&a, &mut p);
        let after = relative(&p);
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.measurements, b.measurements);
            let (a, b) = (a.placed.as_ref().unwrap(), b.placed.as_ref().unwrap());
            assert!(
                a.label_box
                    .iter()
                    .zip(b.label_box)
                    .all(|(a, b)| (a - b).abs() < 1e-6)
            );
            for (a, b) in a
                .lines
                .iter()
                .chain(&a.arrows)
                .flatten()
                .chain(std::iter::once(&a.label_at))
                .zip(
                    b.lines
                        .iter()
                        .chain(&b.arrows)
                        .flatten()
                        .chain(std::iter::once(&b.label_at)),
                )
            {
                assert!((a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6);
            }
        }
        assert!(
            !p.problems
                .iter()
                .any(|q| q.code == "annotation_outside_area")
        );
        assert!(crate::model::drawn_sheets_fit(&a, &p));
        assert_eq!(readability(&a, &p).failures(), 0);
    }

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
        let main_fill = fill(&a, &p);
        for v in &mut p.views {
            v.scale *= 10.0;
        }
        assert_eq!(
            fill(&a, &p),
            main_fill,
            "foreign geometry cannot fill this page"
        );
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
        let a4 = candidate(1.0, 297.0, 0.52);
        assert!(rank(a4, candidate(2.0, 420.0, 0.55)).is_lt());
        assert!(rank(a4, candidate(10.0, 420.0, 0.80)).is_lt());
        assert!(rank(candidate(2.0, 297.0, 0.40), a4).is_lt());
    }
    #[test]
    fn smallest_readable_paper_then_largest_scale_independent_of_fill() {
        let set = [
            c(0, 1.0, 0.52),
            c(0, 0.5, 0.60),
            c(1, 2.0, 0.55),
            c(2, 5.0, 0.75),
        ];
        assert_eq!(choose(&set, false, true), Some((Some(0), (0, 1.0))));
        assert_eq!(choose(&set[2..], false, true), Some((Some(0), (1, 2.0))));
    }
    #[test]
    fn material_crossing_ratchet_is_preserved() {
        let set = [x(0, 1.0, 0.52, 1), x(1, 2.0, 0.55, 0)];
        assert_eq!(choose(&set, false, true), Some((Some(1), (0, 1.0))));
        assert_eq!(choose(&set, false, false), Some((Some(0), (0, 1.0))));
        assert_eq!(choose(&set[..1], false, true), Some((None, (0, 1.0))));
    }
    #[test]
    fn named_paper_chooses_its_largest_readable_scale() {
        let set = [c(2, 1.0, 0.30), c(2, 0.5, 0.60)];
        assert_eq!(choose(&set, true, true).unwrap().0, Some(0));
        assert_eq!(choose(&[], true, true), None);
    }
}
