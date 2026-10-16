//! One owner per support-plane interval; a stated total leaves one chain link as reference.

use super::{Omitted, Shape, Want};
use crate::items::State;
use crate::lint::redundancy::{Span, TOL, path};
use crate::model::Analysis;

type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

/// The reference dimensions [`consolidate`] made, each by its ID with the IDs of the dimensions
/// that determine it, in the order its basis names them (#26: the plan's `determined_by`).
pub(super) type References = std::collections::BTreeMap<String, Vec<String>>;

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
) -> References {
    let mut references = References::new();
    if a.solid_count.is_some_and(|n| n > 1) {
        return references;
    }
    let mut gone = std::collections::BTreeSet::new();
    // #33: on a turned profile dimensioned as a baseline from its datum, a band's own length is
    // conveyed by the rung that locates its far shoulder (its `step_position.length`, or the
    // overall length for the last band); the rungs before it locate its near one. Unless the spec
    // tolerances that length, or omits the rung, it is not drawn again.
    for i in 0..kept.len() {
        let Some(band) = kept[i]
            .0
            .id
            .strip_prefix("d:")
            .and_then(|s| s.strip_suffix("/step.length"))
        else {
            continue;
        };
        if !given[i].is_empty() {
            continue;
        }
        let rung = (0..kept.len()).find(|&j| {
            j != i
                && kept[j]
                    .0
                    .measurements
                    .iter()
                    .any(|m| m.feature == band && m.parameter == "step_position.length")
        });
        if let Some(j) = rung {
            cover(
                i,
                j,
                kept,
                omitted,
                "a baseline from the profile's datum locates both shoulders of its band",
            );
            gone.insert(i);
        }
    }
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
            // A dimension the spec tolerances, or one that states the part's PMI, is controlling,
            // never reference (#33: reference is for redundant, untoleranced dimensions only).
            if !given[i].is_empty()
                || kept[i]
                    .0
                    .measurements
                    .iter()
                    .any(|m| m.feature.starts_with("pmi:"))
            {
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
                let by: Vec<String> = route.iter().map(|j| kept[*j].0.id.clone()).collect();
                let carriers = by.join(", ");
                references.insert(kept[i].0.id.clone(), by);
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
            let by = route.iter().map(|j| kept[*j].0.id.clone()).collect();
            references.insert(kept[i].0.id.clone(), by);
            let w = &mut kept[i].0;
            w.terms = vec![format!("({})", w.terms.join(" "))];
            w.basis +=
                "; reference: the stated overall and slot length determine this floor interval";
        }
    }
    shoulders_on_slot_walls(a, kept, given, &gone, &mut references);
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
    references
}
/// Ladder shoulders standing on a slot's walls close against the slot's stated width: with its
/// centre-line location also stated, every such shoulder is determined; otherwise, of two
/// shoulders on both walls, the farther is. A determined shoulder is shown as reference.
fn shoulders_on_slot_walls(
    a: &Analysis,
    kept: &mut [Kept],
    given: &[Vec<String>],
    gone: &std::collections::BTreeSet<usize>,
    references: &mut References,
) {
    use crate::lint::redundancy::slot_walls;
    let reference = |w: &Want| w.terms.len() == 1 && w.terms[0].starts_with('(');
    let stated = |kept: &[Kept], feature: &str, parameter: &str| {
        (0..kept.len()).find(|&j| {
            !gone.contains(&j)
                && !reference(&kept[j].0)
                && kept[j]
                    .0
                    .measurements
                    .iter()
                    .any(|m| m.feature == feature && m.parameter == parameter)
        })
    };
    for f in a.features.iter().filter(|f| f.kind == "slot") {
        let Some((walls, _)) = slot_walls(a, &f.id) else {
            continue;
        };
        let Some(width) = stated(kept, &f.id, "slot_width.length") else {
            continue;
        };
        let across = stated(kept, &f.id, "location_slot_across.length");
        // Every retained statement of a wall's station from the box: (index, station, a ladder item).
        let mut rungs: Vec<(usize, f64, bool)> = (0..kept.len())
            .filter(|&i| !gone.contains(&i) && !reference(&kept[i].0))
            .filter_map(|i| {
                let s = kept[i]
                    .0
                    .measurements
                    .iter()
                    .filter(|m| m.feature == "step_level")
                    .find_map(|m| crate::lint::redundancy::span(a, &m.feature, &m.parameter))?;
                let ladder = kept[i].0.id.starts_with("d:step_level/") && given[i].is_empty();
                (s.axis == walls.axis
                    && (s.lo - a.envelope[0][s.axis]).abs() <= TOL
                    && [walls.lo, walls.hi].iter().any(|w| (s.hi - w).abs() <= TOL))
                .then_some((i, s.hi, ladder))
            })
            .collect();
        if across.is_none() {
            let on = |w: f64| rungs.iter().any(|r| (r.1 - w).abs() <= TOL);
            if !(on(walls.lo) && on(walls.hi)) {
                continue;
            }
            // The farther wall's ladder item, else the nearer's.
            rungs.sort_by(|x, y| y.1.total_cmp(&x.1));
            rungs.retain(|r| r.2);
            rungs.truncate(1);
        } else {
            rungs.retain(|r| r.2);
        }
        let by: Vec<String> = std::iter::once(width)
            .chain(across)
            .map(|j| kept[j].0.id.clone())
            .collect();
        let carriers = by.join(", ");
        for (i, ..) in rungs {
            references.insert(kept[i].0.id.clone(), by.clone());
            let w = &mut kept[i].0;
            w.terms = vec![format!("({})", w.terms.join(" "))];
            w.basis += &format!(
                "; reference: the slot's stated width and location determine this shoulder on its wall ({carriers})"
            );
        }
    }
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
