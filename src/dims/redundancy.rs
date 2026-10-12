//! One owner per support-plane interval; a stated total leaves one chain link as reference.

use super::{Omitted, Shape, Want};
use crate::items::State;
use crate::lint::redundancy::{Span, TOL, path};
use crate::model::Analysis;

type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

fn interval(analysis: &Analysis, w: &Want) -> Option<Span> {
    if w.id.starts_with("d:pmi:") {
        return None;
    }
    let Shape::Linear { a, b, axis, .. } = w.shape else {
        return None;
    };
    // Point locations use their own view and physical witness contracts; they
    // are not support planes. Counted sizes and diameters are not one axial interval.
    if w.measurements
        .iter()
        .any(|m| m.parameter.starts_with("location.length.") || m.feature == "step_level")
    {
        return None;
    }
    if w.terms.iter().any(|t| t.contains('×') || t.contains('ø'))
        || w.measurements
            .iter()
            .any(|m| m.parameter.contains("diameter"))
        || ((b[axis] - a[axis]).abs() - w.value).abs() > TOL
    {
        return None;
    }
    let drawn = Span::new(axis, a[axis], b[axis]);
    w.measurements.iter().find_map(|m| {
        crate::lint::redundancy::span(analysis, &m.feature, &m.parameter).filter(|s| s.same(drawn))
    })
}

pub(super) fn consolidate(
    a: &Analysis,
    kept: &mut Vec<Kept>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
) {
    if a.solid_count.is_some_and(|n| n > 1) {
        return;
    }
    let mut gone = std::collections::BTreeSet::new();
    // A functional outside diameter or A/F statement owns an equal overall span only
    // when its actual support planes reach both ends of that axis.
    for i in 0..kept.len() {
        if !given[i].is_empty() || !kept[i].0.id.starts_with("d:envelope/") {
            continue;
        }
        let Some(s) = interval(a, &kept[i].0) else {
            continue;
        };
        let owner = (0..kept.len()).find(|&j| {
            j != i
                && !gone.contains(&j)
                && kept[j].0.measurements.iter().any(|m| {
                    crate::lint::redundancy::external_span(a, &m.feature, &m.parameter, s.axis)
                        .is_some_and(|p| p.same(s))
                })
        });
        if let Some(j) = owner {
            let claims = kept[i].0.measurements.clone();
            kept[j].0.measurements.extend(claims);
            cover(
                i,
                j,
                kept,
                omitted,
                "the functional outside size spans the same support planes",
            );
            gone.insert(i);
        }
    }
    let mut order: Vec<_> = (0..kept.len())
        .filter(|i| !gone.contains(i) && interval(a, &kept[*i].0).is_some())
        .collect();
    order.sort_by(|&i, &j| {
        let rank = |i: usize| {
            let w = &kept[i].0;
            if w.id.starts_with("d:envelope/") {
                0
            } else if w.id.contains("step_level") || w.id.contains("/location") {
                3
            } else {
                2
            }
        };
        rank(i)
            .cmp(&rank(j))
            .then(
                ((kept[j].0.value * 1e6).round() as i64)
                    .cmp(&((kept[i].0.value * 1e6).round() as i64)),
            )
            .then(kept[i].0.id.cmp(&kept[j].0.id))
    });
    let mut edges = Vec::new();
    for i in order {
        let s = interval(a, &kept[i].0).unwrap();
        if let Some(route) = path(&edges, s) {
            if !given[i].is_empty() {
                edges.push((s, i));
                continue;
            }
            if route.len() == 1 && given[i].is_empty() && given[route[0]].is_empty() {
                let j = route[0];
                if kept[i]
                    .0
                    .measurements
                    .iter()
                    .any(|m| m.parameter == "thickness.length")
                    && kept[j]
                        .0
                        .measurements
                        .iter()
                        .any(|m| m.parameter == "thickness.length")
                {
                    let body = |w: &Want| {
                        w.measurements
                            .first()
                            .and_then(|m| a.features.iter().find(|f| f.id == m.feature))
                            .and_then(|f| f.parameters.get("body_key"))
                            .filter(|key| !key.is_null())
                    };
                    if body(&kept[i].0).is_none() || body(&kept[i].0) != body(&kept[j].0) {
                        edges.push((s, i));
                        continue;
                    }
                }
                if kept[i].2 != kept[j].2
                    && kept[i].0.measurements.iter().all(|m| {
                        m.feature.starts_with("f:hole:") || m.feature.starts_with("f:hole_pattern:")
                    })
                    && kept[j].0.measurements.iter().all(|m| {
                        m.feature.starts_with("f:hole:") || m.feature.starts_with("f:hole_pattern:")
                    })
                {
                    edges.push((s, i));
                    continue;
                }
                let claims = kept[i].0.measurements.clone();
                kept[j].0.measurements.extend(claims);
                cover(
                    i,
                    j,
                    kept,
                    omitted,
                    "the same physical support-plane interval is already stated",
                );
                gone.insert(i);
            } else {
                let carriers = route
                    .iter()
                    .map(|j| kept[*j].0.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let w = &mut kept[i].0;
                w.terms = vec![format!("({})", w.terms.join(" "))];
                w.basis += &format!(
                    "; reference: the stated total and other chain members determine this interval ({carriers})"
                );
            }
        } else {
            edges.push((s, i));
        }
    }
    // A ladder's datum stays controlling unless a supported feature interval
    // states those exact planes. In particular, a slot's reference must own its
    // coincident floor rung too, rather than leaving the closed chain on another view.
    for i in 0..kept.len() {
        if gone.contains(&i)
            || !kept[i].0.id.starts_with("d:step_level/")
            || !given[i].is_empty()
            || kept[i]
                .0
                .measurements
                .iter()
                .any(|m| m.feature.starts_with("pmi:"))
        {
            continue;
        }
        let Some(s) = kept[i]
            .0
            .measurements
            .iter()
            .filter(|m| m.feature == "step_level")
            .find_map(|m| crate::lint::redundancy::level_span(a, &m.parameter))
        else {
            continue;
        };
        let owner = (0..kept.len()).find(|&j| {
            j != i
                && !gone.contains(&j)
                && kept[j].2.is_some()
                && interval(a, &kept[j].0).is_some_and(|p| p.same(s))
        });
        if let Some(j) = owner {
            let claims = kept[i].0.measurements.clone();
            kept[j].0.measurements.extend(claims);
            cover(
                i,
                j,
                kept,
                omitted,
                "the feature interval states the same support planes as this ladder datum",
            );
            gone.insert(i);
        } else if a.features.iter().filter(|f| f.kind == "slot").any(|f| {
            crate::lint::redundancy::span(a, &f.id, "location_slot.length")
                .is_some_and(|p| p.same(s))
        }) && let Some(route) = path(&edges, s)
            && route.len() > 1
            && route.iter().all(|j| kept[*j].2.is_some())
        {
            let w = &mut kept[i].0;
            w.terms = vec![format!("({})", w.terms.join(" "))];
            w.basis +=
                "; reference: the stated overall and slot length determine this floor interval";
        }
    }
    let mut i = 0;
    given.retain(|_| {
        let keep = !gone.contains(&i);
        i += 1;
        keep
    });
    let mut i = 0;
    kept.retain(|_| {
        let keep = !gone.contains(&i);
        i += 1;
        keep
    });
}
fn cover(i: usize, j: usize, kept: &[Kept], omitted: &mut Vec<Omitted>, why: &str) {
    let (w, owner) = (&kept[i].0, &kept[j].0);
    omitted.push(Omitted {
        id: w.id.clone(),
        label: w.terms.join(" "),
        basis: format!("conveyed by {}: {why}", owner.id),
        state: State::Covered {
            by: owner.id.clone(),
        },
    });
}
