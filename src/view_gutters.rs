//! Spend spare paper on blank view gutters, only after sheet/scale recovery and allocation.
//! All packers (including relative placements, details and measured refits) meet here.
use crate::layout::{MARGIN, Rect};
use crate::model::{Analysis, Plan};
use crate::placement_intent::{move_box, move_point};

pub(crate) const PREFERRED: f64 = 12.0;

fn include(a: &mut Rect, b: Rect) {
    for k in 0..2 {
        a[k] = a[k].min(b[k]);
        a[k + 2] = a[k + 2].max(b[k + 2]);
    }
}

/// Reserved annotation bands, enlarged to contain settled ink and view furniture.
/// The iso's empty fitting region is not an annotation band: use its drawn bounds.
fn bands(analysis: &Analysis, plan: &Plan) -> Vec<Rect> {
    let mut bounds: Vec<_> = plan
        .views
        .iter()
        .map(|v| {
            if v.id == "v:iso" && plan.spec.views.get("iso").is_none_or(|v| v.place.is_none()) {
                let b = analysis.iso_bounds;
                [
                    v.at[0] + v.scale * b[0],
                    v.at[1] + v.scale * b[1],
                    v.at[0] + v.scale * b[2],
                    v.at[1] + v.scale * b[3],
                ]
            } else {
                v.area
            }
        })
        .collect();
    let mut own = |id: &str, b: Rect| {
        if let Some(i) = plan.views.iter().position(|v| v.id == id) {
            include(&mut bounds[i], b);
        }
    };
    for d in &plan.dimensions {
        if let Some(b) = crate::dims::ink_box(d) {
            own(&d.view, b);
        }
    }
    for g in &plan.gdt {
        if let Some(d) = crate::dims::gdt::proxy(g)
            && let Some(b) = crate::dims::ink_box(&d)
        {
            own(&g.view, b);
        }
    }
    for c in &plan.captions {
        own(&c.view, c.ink);
    }
    for b in &plan.balloons {
        let mut r = [
            b.at[0] - b.radius,
            b.at[1] - b.radius,
            b.at[0] + b.radius,
            b.at[1] + b.radius,
        ];
        if let Some(label) = b.label_box {
            include(&mut r, label);
        }
        for p in b.leader.iter().flatten() {
            include(&mut r, [p[0], p[1], p[0], p[1]]);
        }
        own(&b.view, r);
    }
    for v in &plan.views {
        for w in &v.line_work {
            for c in &w.curves {
                own(&v.id, c.bbox());
            }
        }
        // With one view, furniture_bounds returns the fixed sheet furniture first,
        // then its caption and (for a detail) the source marker. Resolve ownership.
        let sheet = std::iter::once(&plan.sheet)
            .chain(&plan.sheets)
            .find(|s| s.number == v.sheet)
            .expect("view sheet");
        let fixed = crate::sheet_fit::furniture_bounds(analysis, sheet, &[]).len();
        let furniture =
            crate::sheet_fit::furniture_bounds(analysis, sheet, std::slice::from_ref(v));
        for b in furniture.into_iter().skip(fixed) {
            own(&v.id, b);
        }
        if let Some(d) = &v.detail
            && let Some(source) = plan.views.iter().find(|s| s.id == d.source_view)
        {
            let (_, at) = crate::detail::marker(analysis, d, source, &plan.views, sheet.page, &[]);
            let text = crate::dims::font().text(&d.label, crate::section::FONT_SIZE);
            own(
                &source.id,
                [
                    at[0] + text.min[0],
                    at[1] + text.min[1],
                    at[0] + text.max[0],
                    at[1] + text.max[1],
                ],
            );
        }
    }
    bounds
}

// Difference constraints: translation[to] <= translation[from] + limit.
#[derive(Clone, Copy)]
struct Edge {
    from: usize,
    to: usize,
    limit: f64,
}

fn solve(n: usize, edges: &[Edge]) -> Option<Vec<f64>> {
    let mut d = vec![0.0; n];
    for pass in 0..n {
        let mut changed = false;
        for &Edge { from, to, limit } in edges {
            if d[to] > d[from] + limit {
                d[to] = d[from] + limit;
                changed = true;
            }
        }
        if !changed {
            let zero = d[n - 1];
            for x in &mut d {
                *x -= zero;
            }
            return Some(d);
        }
        if pass == n - 1 {
            return None;
        }
    }
    unreachable!()
}

/// Keep each pair on its established side. This preserves the drawing's composition
/// and turns a non-convex packing problem into two small difference-constraint systems.
fn separation(a: Rect, b: Rect) -> (usize, bool, f64) {
    let choices = [
        (0, true, b[0] - a[2]),
        (0, false, a[0] - b[2]),
        (1, true, b[1] - a[3]),
        (1, false, a[1] - b[3]),
    ];
    *choices.iter().max_by(|a, b| a.2.total_cmp(&b.2)).unwrap()
}

fn translations(
    analysis: &Analysis,
    plan: &Plan,
    bounds: &[Rect],
    preferred: f64,
    above_title: bool,
) -> Option<Vec<[f64; 2]>> {
    let n = bounds.len();
    let mut hard = [Vec::new(), Vec::new()];
    let mut soft = [Vec::new(), Vec::new()];
    let equality = |edges: &mut Vec<Edge>, i, j| {
        edges.extend([
            Edge {
                from: i,
                to: j,
                limit: 0.0,
            },
            Edge {
                from: j,
                to: i,
                limit: 0.0,
            },
        ]);
    };
    let index = |id: &str| plan.views.iter().position(|v| v.id == id);
    // Projection origins remain aligned even when the two annotation bands differ.
    for (id, k) in [("v:plan", 0), ("v:side", 1)] {
        if let (Some(i), Some(j)) = (index("v:front"), index(id))
            && plan.views[i].sheet == plan.views[j].sheet
        {
            equality(&mut hard[k], i, j);
        }
    }
    for (i, v) in plan.views.iter().enumerate() {
        // Explicit placement relations retain their exact gap and alignment, pinned
        // or otherwise. Their connected group may translate, but never deform.
        if let Some(p) = plan
            .spec
            .views
            .get(v.id.trim_start_matches("v:"))
            .and_then(|s| s.place.as_ref())
            && let Some(j) = index(&format!("v:{}", p.relative_to))
        {
            for edges in &mut hard {
                equality(edges, i, j);
            }
        }
        let pinned = plan.spec.dimensions.iter().any(|(id,d)| d.place.as_ref().is_some_and(|p| p.pinned)
                && plan.dimensions.iter().any(|d| d.id == *id && d.view == v.id))
            || plan.spec.annotations.iter().any(|(id,d)| d.place.as_ref().is_some_and(|p| p.pinned)
                && plan.gdt.iter().any(|g| g.id == *id && g.view == v.id))
            // Cross-sheet references encode both destination zones and local stations.
            || v.xref.is_some() || v.cut.as_ref().is_some_and(|c| c.xref.is_some());
        if pinned || crate::model::dimension_text_edited(&plan.spec) {
            for edges in &mut hard {
                equality(edges, i, n);
            }
        }
        let sheet = std::iter::once(&plan.sheet)
            .chain(&plan.sheets)
            .find(|s| s.number == v.sheet)
            .expect("view sheet");
        // Principal bands already contain blank annotation padding. The iso band
        // is measured ink, so keep its caption one mm inside the drawn frame.
        let inset = MARGIN + if v.id == "v:iso" { 1.0 } else { 0.0 };
        for k in 0..2 {
            hard[k].push(Edge {
                from: i,
                to: n,
                limit: (bounds[i][k] - inset - 1e-5).max(0.0),
            });
            hard[k].push(Edge {
                from: n,
                to: i,
                limit: (sheet.page[k] - inset - bounds[i][k + 2] - 1e-5).max(0.0),
            });
        }
        let mut obstacles = vec![sheet.title_block];
        obstacles.extend(crate::sheet_fit::furniture_bounds(analysis, sheet, &[]));
        if sheet.number == 1 {
            obstacles.extend(
                plan.notes_block
                    .iter()
                    .chain(&plan.radius_notes_block)
                    .map(|b| b.cells),
            );
            obstacles.extend(
                plan.requirements
                    .iter()
                    .filter_map(|r| r.placed.as_ref().map(|p| p.label_box)),
            );
        }
        obstacles.extend(
            plan.tables
                .iter()
                .filter(|t| t.sheet == sheet.number)
                .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
        );
        obstacles.extend(
            plan.hole_tables
                .iter()
                .filter(|t| t.sheet == sheet.number)
                .flat_map(|t| &t.blocks)
                .filter_map(|t| t.placed.as_ref().map(|p| p.cells)),
        );
        if let Some(t) = &plan.radius_table
            && sheet.number == t.table.sheet
            && let Some(p) = &t.table.placed
        {
            obstacles.push(p.cells);
        }
        for (obstacle, b) in obstacles.into_iter().enumerate() {
            // A row initially left of the title may have room to lift above it.
            // Try that composition too, before sacrificing a useful gutter.
            if above_title && obstacle == 0 && v.id != "v:iso" {
                hard[1].push(Edge {
                    from: i,
                    to: n,
                    limit: bounds[i][1] - b[3] - 1e-5,
                });
                continue;
            }
            let (k, forward, gap) = separation(bounds[i], b);
            // Never worsen an existing overlap; gutters cannot invalidate a drawing.
            hard[k].push(if forward {
                Edge {
                    from: n,
                    to: i,
                    limit: (gap - 1e-5).max(0.0),
                }
            } else {
                Edge {
                    from: i,
                    to: n,
                    limit: (gap - 1e-5).max(0.0),
                }
            });
        }
        for j in 0..i {
            if plan.views[j].sheet != v.sheet {
                continue;
            }
            let (k, forward, gap) = separation(bounds[j], bounds[i]);
            let (from, to) = if forward { (i, j) } else { (j, i) };
            // A rigid explicit relation can have a deliberately zero gap.
            let explicit = |a: usize, b: usize| {
                plan.spec
                    .views
                    .get(plan.views[a].id.trim_start_matches("v:"))
                    .and_then(|v| v.place.as_ref())
                    .is_some_and(|p| format!("v:{}", p.relative_to) == plan.views[b].id)
            };
            if explicit(i, j) || explicit(j, i) {
                continue;
            }
            soft[k].push((from, to, gap));
        }
    }
    let mut result = vec![[0.0; 2]; n];
    for k in 0..2 {
        let mut edges = hard[k].clone();
        // Start at the known feasible composition and spend slack on each pair.
        // A constrained pair never prevents another pair from getting room.
        for &(from, to, gap) in &soft[k] {
            edges.push(Edge {
                from,
                to,
                limit: gap - gap.min(0.0),
            });
        }
        solve(n + 1, &edges)?;
        // Share tight slack before improving individual pairs: no early pair may
        // consume all the paper and leave another automatic pair touching.
        let set_common = |edges: &mut [Edge], fraction: f64| {
            for (edge, &(_, _, gap)) in edges[hard[k].len()..].iter_mut().zip(&soft[k]) {
                let floor = gap.min(0.0);
                edge.limit = gap - (floor + fraction * (preferred - floor));
            }
        };
        set_common(&mut edges, 1.0);
        if solve(n + 1, &edges).is_none() {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..32 {
                let mid = (lo + hi) / 2.0;
                set_common(&mut edges, mid);
                if solve(n + 1, &edges).is_some() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            set_common(&mut edges, lo);
        }
        for slot in hard[k].len()..edges.len() {
            let gap = soft[k][slot - hard[k].len()].2;
            let target = gap - preferred;
            let old = edges[slot].limit;
            edges[slot].limit = target;
            if solve(n + 1, &edges).is_none() {
                let (mut lo, mut hi) = (target, old);
                for _ in 0..32 {
                    let mid = (lo + hi) / 2.0;
                    edges[slot].limit = mid;
                    if solve(n + 1, &edges).is_some() {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                edges[slot].limit = hi;
            }
        }
        let d = solve(n + 1, &edges)?;
        for i in 0..n {
            result[i][k] = d[i];
        }
    }
    Some(result)
}

pub(crate) fn apply(analysis: &Analysis, plan: &mut Plan) {
    let preferred = plan.spec.sheet.view_gutter.unwrap_or(PREFERRED);
    if preferred == 0.0 || plan.views.len() < 2 {
        return;
    }
    let bounds = bands(analysis, plan);
    let mut deltas = translations(analysis, plan, &bounds, preferred, false)
        .expect("the established composition remains feasible");
    if let Some(lifted) = translations(analysis, plan, &bounds, preferred, true) {
        let score = |d: &[[f64; 2]]| {
            let mut smallest = preferred;
            let mut total = 0.0;
            for i in 0..bounds.len() {
                for j in 0..i {
                    if plan.views[i].sheet != plan.views[j].sheet {
                        continue;
                    }
                    let mut a = bounds[i];
                    let mut b = bounds[j];
                    move_box(&mut a, d[i]);
                    move_box(&mut b, d[j]);
                    let gap = separation(a, b).2.min(preferred);
                    smallest = smallest.min(gap);
                    total += gap;
                }
            }
            (smallest, total)
        };
        let a = score(&deltas);
        let b = score(&lifted);
        if b.0 > a.0 + 1e-6 || ((b.0 - a.0).abs() < 1e-6 && b.1 > a.1 + 1e-6) {
            deltas = lifted;
        }
    }
    // Below the bisection resolution, preserve an already settled frame exactly.
    for d in deltas.iter_mut().flatten() {
        if d.abs() < 1e-8 {
            *d = 0.0;
        }
    }
    // Rebase editor tip metadata from the transported paper point. Keeping the old
    // relative offset can lose an ulp when added to a translated view origin, so a
    // no-op offset pin would otherwise change its reported tip.
    let tips: Vec<_> = plan
        .callout_tips
        .keys()
        .filter_map(|id| {
            let (view, mut tip) = plan.automatic_callout_tip(id)?;
            if plan
                .spec
                .dimensions
                .get(id)
                .is_none_or(|d| d.place.is_none())
                && let Some(p) = plan
                    .dimensions
                    .iter()
                    .find(|d| d.id == *id && d.view == view)
                    .and_then(|d| d.placed.as_ref())
                    .and_then(|p| p.lines.first())
            {
                tip = p[0];
            }
            let i = plan.views.iter().position(|v| v.id == view)?;
            move_point(&mut tip, deltas[i]);
            Some((id.clone(), i, tip))
        })
        .collect();
    for ((v, b), delta) in plan.views.iter_mut().zip(bounds).zip(&deltas) {
        move_point(&mut v.at, *delta);
        if v.id == "v:iso" && plan.spec.views.get("iso").is_none_or(|v| v.place.is_none()) {
            v.area = b;
        }
        move_box(&mut v.area, *delta);
        crate::centrelines::shift(v, *delta);
    }
    let delta = |id: &str| {
        plan.views
            .iter()
            .position(|v| v.id == id)
            .map_or([0.0; 2], |i| deltas[i])
    };
    for d in &mut plan.dimensions {
        if let Some(p) = &mut d.placed {
            let d = delta(&d.view);
            move_point(&mut p.label_at, d);
            move_box(&mut p.label_box, d);
            for q in p.lines.iter_mut().chain(&mut p.arrows).flatten() {
                move_point(q, d);
            }
        }
    }
    for g in &mut plan.gdt {
        if let Some(p) = &mut g.placed {
            let d = delta(&g.view);
            move_point(&mut p.tip, d);
            move_box(&mut p.label_box, d);
            for q in p
                .lines
                .iter_mut()
                .chain(&mut p.arrows)
                .chain(&mut p.strokes)
                .flatten()
                .chain(p.fills.iter_mut().flatten())
            {
                move_point(q, d);
            }
            for arc in &mut p.arcs {
                arc[0] += d[0];
                arc[1] += d[1];
            }
            for t in p.letters.iter_mut().chain(&mut p.runs) {
                move_point(&mut t.at, d);
            }
        }
    }
    for c in &mut plan.captions {
        let d = delta(&c.view);
        move_point(&mut c.at, d);
        move_box(&mut c.ink, d);
    }
    for b in &mut plan.balloons {
        let d = delta(&b.view);
        move_point(&mut b.at, d);
        if let Some(label) = &mut b.label_box {
            move_box(label, d);
        }
        for p in b.leader.iter_mut().flatten() {
            move_point(p, d);
        }
    }
    for (id, i, tip) in tips {
        let v = &plan.views[i];
        plan.callout_tips
            .insert(id, (v.id.clone(), [tip[0] - v.at[0], tip[1] - v.at[1]]));
    }
    plan.sheet.fill = crate::sheet_fit::fill(analysis, plan);
}
