//! Final, bounded route repair against the complete printed inventory (#147).
//! Geometry producers remain in `leaders`; datum triangles never move here.
use std::collections::{BTreeMap, BTreeSet};

use super::{Placed, PlannedDimension, gdt, interior, leaders, placement};
use crate::model::{Analysis, Plan, Problem, Spec};

/// Counts every crossing kind independently, plus every other problem code.
/// Trading a leader/leader crossing for a leader through text is never an improvement.
fn counts(problems: &[Problem]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for p in problems {
        *out.entry(p.code.clone()).or_default() += 1;
        for kind in &p.crossing_kinds {
            *out.entry(format!("crossing:{kind}")).or_default() += 1;
        }
    }
    out
}

fn improves(before: &[Problem], after: &[Problem]) -> bool {
    let a = counts(before);
    let b = counts(after);
    b.iter().all(|(k, n)| *n <= a.get(k).copied().unwrap_or(0))
        && b.get("annotation_leader_crossing").copied().unwrap_or(0)
            < a.get("annotation_leader_crossing").copied().unwrap_or(0)
}

fn furniture(analysis: &Analysis, plan: &Plan) -> Vec<PlannedDimension> {
    let mut fixed = crate::centrelines::obstacles(&plan.views);
    fixed.extend(plan.tables.iter().filter_map(crate::tables::proxy));
    for block in [&plan.notes_block, &plan.radius_notes_block]
        .into_iter()
        .flatten()
    {
        fixed.push(crate::notes::proxy(block));
    }
    if let Some(table) = &plan.radius_table {
        fixed.extend(crate::tables::proxy(&table.table));
    }
    fixed.extend(plan.requirements.iter().filter_map(gdt::finish_proxy));
    fixed.extend(
        crate::iso_caption::plan(analysis, &plan.sheet, &plan.views, &plan.dimensions)
            .iter()
            .map(crate::iso_caption::proxy),
    );
    // The datum triangle is ink too, separate from its letter box.
    for g in &plan.gdt {
        if let Some(p) = &g.placed {
            for (i, fill) in p.fills.iter().enumerate() {
                let mut d = gdt::proxy(g).expect("placed datum");
                d.id = format!("{}:triangle:{i}", g.id);
                let b = fill.iter().fold(
                    [
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ],
                    |b, p| {
                        [
                            b[0].min(p[0]),
                            b[1].min(p[1]),
                            b[2].max(p[0]),
                            b[3].max(p[1]),
                        ]
                    },
                );
                let p = d.placed.as_mut().expect("proxy placement");
                p.label_box = b;
                p.lines = (0..fill.len())
                    .map(|i| [fill[i], fill[(i + 1) % fill.len()]])
                    .collect();
                p.arrows.clear();
                fixed.push(d);
            }
        }
    }
    fixed
}

fn rehang(g: &mut gdt::PlannedGdt, next: &Placed) {
    let p = g.placed.as_mut().expect("placed glyph");
    let delta = [
        next.label_box[0] - p.label_box[0],
        next.label_box[1] - p.label_box[1],
    ];
    let shift = |q: &mut [f64; 2]| {
        q[0] += delta[0];
        q[1] += delta[1];
    };
    // The glyph and text move as one rigid object: all compartment widths and 3 mm text stay.
    for line in &mut p.strokes {
        for q in line {
            shift(q);
        }
    }
    for arc in &mut p.arcs {
        arc[0] += delta[0];
        arc[1] += delta[1];
    }
    for text in p.letters.iter_mut().chain(&mut p.runs) {
        shift(&mut text.at);
    }
    if g.kind != "datum_feature" {
        let old_elbow = p.lines.first().map(|s| s[1]);
        let new_elbow = next.lines.first().map(|s| s[1]);
        // All-around/all-over rings belong at the bend, rather than on the frame.
        if let (Some(a), Some(b)) = (old_elbow, new_elbow) {
            let count = g
                .modifiers
                .iter()
                .map(|m| if m == "all_over" { 2 } else { 1 })
                .sum::<usize>();
            for arc in p.arcs.iter_mut().rev().take(count) {
                arc[0] += b[0] - a[0] - delta[0];
                arc[1] += b[1] - a[1] - delta[1];
            }
        }
        p.tip = next.lines[0][0];
    }
    p.lines.clone_from(&next.lines);
    p.arrows.clone_from(&next.arrows);
    p.label_box = next.label_box;
}

pub(crate) fn finish(analysis: &Analysis, spec: &Spec, plan: &mut Plan) {
    if crate::nominal::comparison()
        || crate::planning::lookup::<bool>(analysis, "canonical-circular-recipes", &[])
            .unwrap_or(false)
        || !plan
            .problems
            .iter()
            .any(|p| p.code == "annotation_leader_crossing")
    {
        return;
    }
    let cache = super::footprints::Cache::default();
    let proofs = interior::proofs(analysis, &plan.views);
    let mut wants = super::wants(analysis).0;
    let frames = plan
        .views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            (super::NAMES.contains(&name) || crate::detail::NAMES.contains(&name))
                .then(|| (name, placement::Frame::new(analysis, v)))
        })
        .collect();
    super::detail_routes(analysis, &mut wants, &plan.views);
    super::route(&mut wants, &frames, analysis);
    let fixed = furniture(analysis, plan);
    let mut all = plan.dimensions.clone();
    all.extend(plan.gdt.iter().filter_map(gdt::proxy));
    let dimension_count = plan.dimensions.len();
    let mut changed = BTreeSet::new();
    // Two deterministic sweeps let a later frame release an earlier callout's corridor.
    for _ in 0..2 {
        let mut moved = false;
        for i in 0..all.len() {
            let d = &all[i];
            if d.kind != "callout"
                || d.placed.is_none()
                || spec
                    .dimensions
                    .get(&d.id)
                    .is_some_and(|s| s.place.is_some())
                || spec
                    .annotations
                    .get(&d.id)
                    .is_some_and(|s| s.place.is_some())
            {
                continue;
            }
            let Some(view) = plan.views.iter().find(|v| v.id == d.view) else {
                continue;
            };
            let Some(sheet) = std::iter::once(&plan.sheet)
                .chain(&plan.sheets)
                .find(|s| s.number == view.sheet)
            else {
                continue;
            };
            let before =
                placement::collisions_for(analysis, &all, sheet, &plan.views, &cache, &d.id);
            if !before
                .iter()
                .any(|p| p.code == "annotation_leader_crossing")
            {
                continue;
            }
            let glyph = plan.gdt.iter().find(|g| g.id == d.id);
            // Finishes have a prescribed outward normal, independent of leadered labels.
            if glyph.is_some_and(|g| g.kind == "surface_finish") {
                continue;
            }
            let peers: Vec<_> = all
                .iter()
                .enumerate()
                .filter(|(j, q)| {
                    *j != i
                        && plan
                            .views
                            .iter()
                            .find(|v| v.id == q.view)
                            .is_none_or(|v| v.sheet == sheet.number)
                })
                .map(|(_, q)| q.clone())
                .chain(
                    fixed
                        .iter()
                        .filter(|q| !q.id.starts_with(&format!("{}:triangle:", d.id)))
                        .cloned(),
                )
                .collect();
            let candidates = if glyph.is_some_and(|g| g.kind == "datum_feature") {
                gdt::normal_routes(glyph.expect("datum"))
            } else {
                leaders::settled_routes(
                    analysis,
                    sheet,
                    &plan.views,
                    d,
                    wants.iter().find(|w| w.id == d.id),
                    &peers,
                    proofs.get(&d.view).map(|p| p.as_ref()),
                )
            };
            let id = d.id.clone();
            let old = d.placed.clone();
            for next in candidates {
                all[i].placed = Some(next.clone());
                let after =
                    placement::collisions_for(analysis, &all, sheet, &plan.views, &cache, &id);
                if improves(&before, &after) {
                    if i < dimension_count {
                        let d = &mut plan.dimensions[i];
                        let inside = proofs
                            .get(&d.view)
                            .is_some_and(|p| p.annotation_clear(view, &next, true));
                        d.region = inside.then_some(super::CandidateRegion::Interior);
                        if inside && !d.basis.contains(interior::BASIS) {
                            d.basis += interior::BASIS;
                        }
                        d.placed = Some(next.clone());
                        plan.callout_tips.insert(
                            id.clone(),
                            (
                                d.view.clone(),
                                [
                                    next.lines[0][0][0] - view.at[0],
                                    next.lines[0][0][1] - view.at[1],
                                ],
                            ),
                        );
                    } else if let Some(g) = plan.gdt.iter_mut().find(|g| g.id == id) {
                        rehang(g, &next);
                    }
                    changed.insert(id.clone());
                    moved = true;
                    break;
                }
                all[i].placed.clone_from(&old);
            }
        }
        if !moved {
            break;
        }
    }
    if changed.is_empty() {
        return;
    }
    let checked: Vec<_> = std::iter::once(&plan.sheet)
        .chain(&plan.sheets)
        .flat_map(|s| placement::collisions_cached(analysis, &all, s, &plan.views, &cache))
        .collect();
    // Replace only findings involving changed ink; untouched diagnostics retain their identity.
    plan.problems.retain(|p| {
        !p.subjects.iter().any(|s| changed.contains(s))
            || !(p.code.starts_with("annotation_") || p.code == "leader_through_feature")
    });
    plan.problems.extend(
        checked
            .into_iter()
            .filter(|p| p.subjects.iter().any(|s| changed.contains(s))),
    );
    plan.status = crate::completeness::status(&plan.problems);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossing(kinds: &[&str]) -> Problem {
        Problem { code: "annotation_leader_crossing".into(),
            crossing_kinds: kinds.iter().map(|s| (*s).into()).collect(), ..Default::default() }
    }

    #[test]
    fn a_route_must_improve_crossings_without_trading_kinds_or_text() {
        let before = vec![crossing(&["leader"]), crossing(&["extension_line"])];
        assert!(improves(&before, &[crossing(&["extension_line"])]));
        assert!(!improves(&before, &[crossing(&["arrow"])]));
        assert!(!improves(&before, &before), "ties retain the existing exterior route");
        assert!(!improves(&before, &[Problem { code: "annotation_ink_overlap".into(), ..Default::default() }]));
    }
}
