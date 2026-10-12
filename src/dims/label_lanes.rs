//! Bounded label alternatives on settled ink. GD&T and callout routes are immutable obstacles;
//! moving a dimension may remove contact, but may never add it (ISSUE-1334-a).
use std::collections::BTreeSet;

use super::placement::{Page, boxes_overlap, crosses, ink_points};
use super::{ARROW, LANE, Placed, PlannedDimension};
use crate::model::{Analysis, PlannedSheet, PlannedView, Spec};

const CLEAR: f64 = 0.5;
const OFF_SPAN: f64 = ARROW + 3.0;

fn axis(p: &Placed) -> usize {
    usize::from(p.rotation != 0.0)
}

fn tips(p: &Placed) -> (f64, f64) {
    let k = axis(p);
    let (a, b) = (p.arrows[0][0][k], p.arrows[1][0][k]);
    (a.min(b), a.max(b))
}

pub(super) fn off_span<'a>(p: &Placed, others: impl Iterator<Item = &'a Placed>) -> bool {
    if p.arrows.len() != 2 {
        return false;
    }
    let k = axis(p);
    let (lo, hi) = tips(p);
    let b = p.label_box;
    if b[k] >= lo - 1e-6 && b[k + 2] <= hi + 1e-6 {
        return false;
    }
    if (lo - b[k + 2]).max(b[k] - hi) >= OFF_SPAN - 1e-6 {
        return true;
    }
    let t = p.arrows[0][0][1 - k];
    let normal = [t.min(b[1 - k]), t.max(b[3 - k])];
    others.filter(|q| q.arrows.len() == 2).any(|q| {
        q.lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .any(|(_, [a, z])| {
                let between = (b[k] + 1e-6 < a[k] && a[k] < lo - 1e-6)
                    || (hi + 1e-6 < a[k] && a[k] < b[k + 2] - 1e-6);
                (a[k] - z[k]).abs() <= 1e-6
                    && between
                    && a[1 - k].min(z[1 - k]) <= normal[1]
                    && a[1 - k].max(z[1 - k]) >= normal[0]
            })
    })
}

/// Finished label/leader predicates, plus the measured-span guard for sharing a lane.
fn contacts(p: &Placed, q: &PlannedDimension) -> BTreeSet<u8> {
    let Some(qp) = &q.placed else {
        return BTreeSet::new();
    };
    let mut hits = BTreeSet::new();
    // Crossing extension lines is normal; coincident dimension lines over overlapping
    // measured spans are ambiguous even when their text boxes happen to be separate.
    if q.kind == "linear"
        && p.arrows.len() == 2
        && qp.arrows.len() == 2
        && p.rotation == qp.rotation
    {
        let k = axis(p);
        let (a, b) = tips(p);
        let (c, d) = tips(qp);
        if (p.arrows[0][0][1 - k] - qp.arrows[0][0][1 - k]).abs() < 1e-6
            && b.min(d) - a.max(c) > 1e-6
        {
            hits.insert(4);
        }
    }

    if boxes_overlap(p.label_box, qp.label_box) {
        hits.insert(0);
    }
    if p.lines.iter().any(|l| crosses(*l, qp.label_box)) {
        hits.insert(1);
    }
    if qp.lines.iter().any(|l| crosses(*l, p.label_box)) {
        hits.insert(2);
    }
    if q.kind == "callout"
        && qp.lines.iter().any(|l| {
            p.lines
                .iter()
                .chain(&p.arrows)
                .any(|t| super::leaders::lines_meet(*l, *t))
        })
    {
        hits.insert(3);
    }
    hits
}

fn outside(p: &Placed, area: [f64; 4]) -> bool {
    ink_points(p).any(|q| {
        q[0] < area[0] - 1e-6
            || q[1] < area[1] - 1e-6
            || q[0] > area[2] + 1e-6
            || q[1] > area[3] + 1e-6
    })
}

fn label_at(p: &Placed, at: f64) -> Placed {
    let k = axis(p);
    let mut c = p.clone();
    let d = at - p.label_box[k];
    c.label_at[k] += d;
    c.label_box[k] += d;
    c.label_box[k + 2] += d;
    let (lo, hi) = tips(p);
    // Reset the old run-on: the dimension line ends beside its own text.
    let outside = p
        .arrows
        .iter()
        .any(|[_, tail]| tail[k] < lo || tail[k] > hi);
    let reach = if outside { ARROW + 2.0 } else { 0.0 };
    c.lines[2][0][k] = lo - reach;
    c.lines[2][1][k] = hi + reach;
    if at < lo {
        c.lines[2][0][k] = c.lines[2][0][k].min(at + c.label_box[k + 2] - c.label_box[k] + CLEAR);
    }
    if at > hi {
        c.lines[2][1][k] = c.lines[2][1][k].max(at - CLEAR);
    }
    c
}

fn alternatives(p: &Placed, drawable: [f64; 4]) -> Vec<Placed> {
    let k = axis(p);
    let (lo, hi) = tips(p);
    let width = p.label_box[k + 2] - p.label_box[k];
    let outside_arrows = p
        .arrows
        .iter()
        .any(|[_, tail]| tail[k] < lo || tail[k] > hi);
    let clear = if outside_arrows { CLEAR } else { ARROW + CLEAR };
    let low = (lo + clear).max(drawable[k]);
    let high = (hi - clear - width).min(drawable[k + 2] - width);
    let mut positions = Vec::new();
    if low <= high {
        let centre = (0.5 * (lo + hi - width)).clamp(low, high);
        positions.extend([centre, low, high]);
        let steps = ((high - low) / CLEAR).ceil() as usize;
        positions.extend((0..=steps).map(|n| (low + n as f64 * CLEAR).min(high)));
    } else {
        let gap = if outside_arrows {
            ARROW + 2.0 * CLEAR
        } else {
            2.0 * CLEAR
        };
        positions.extend([hi + gap, lo - gap - width]);
    }
    positions.sort_by(|a, b| {
        (a - p.label_box[k])
            .abs()
            .total_cmp(&(b - p.label_box[k]).abs())
            .then(a.total_cmp(b))
    });
    positions.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    positions
        .into_iter()
        .filter(|at| *at >= drawable[k] && at + width <= drawable[k + 2])
        .map(|at| label_at(p, at))
        .collect()
}

fn promote(p: &Placed, amount: f64) -> Placed {
    let k = 1 - axis(p);
    let line = p.arrows[0][0][k];
    let out = (line - p.lines[0][0][k]).signum();
    let d = out * amount;
    let mut c = p.clone();
    for l in c.lines.iter_mut().take(2) {
        l[1][k] += d;
    }
    for l in c.lines.iter_mut().skip(2).chain(&mut c.arrows) {
        for q in l {
            q[k] += d;
        }
    }
    c.label_at[k] += d;
    c.label_box[k] += d;
    c.label_box[k + 2] += d;
    c
}

pub(crate) fn settle_inventory(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    inventory: &mut [PlannedDimension],
    fixed: &[PlannedDimension],
) {
    let page = Page::new(analysis, sheet, views);
    // A strictly decreasing (contact, off-span) objective bounds retries without a repair pass.
    for _ in 0..2 * inventory.len() {
        let mut changed = false;
        for i in 0..inventory.len() {
            let d = &inventory[i];
            if d.kind != "linear"
                || spec
                    .dimensions
                    .get(&d.id)
                    .and_then(|e| e.place.as_ref())
                    .is_some_and(|p| p.pinned)
            {
                continue;
            }
            let Some(p) = &d.placed else { continue };
            if p.arrows.len() != 2 || p.lines.len() < 3 {
                continue;
            }
            let view = d.view.strip_prefix("v:").unwrap_or(&d.view);
            let area = page.area(view).unwrap_or(page.drawable());
            let others: Vec<_> = inventory
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| q)
                .chain(fixed)
                .collect();
            // Promotion uses room already occupied by this view's dimensions or reserved by
            // its bands; it never grows a dense tower past that footprint.
            let region = inventory
                .iter()
                .filter(|q| q.view == d.view)
                .filter_map(|q| q.placed.as_ref())
                .flat_map(ink_points)
                .fold(area, |b, q| {
                    [
                        b[0].min(q[0]),
                        b[1].min(q[1]),
                        b[2].max(q[0]),
                        b[3].max(q[1]),
                    ]
                });
            let old: Vec<_> = others.iter().map(|q| contacts(p, q)).collect();
            let count: usize = old.iter().map(BTreeSet::len).sum();
            let span_bad = |c: &Placed| {
                off_span(
                    c,
                    others
                        .iter()
                        .filter(|q| q.kind == "linear" && q.view == d.view)
                        .filter_map(|q| q.placed.as_ref()),
                )
            };
            if count == 0 && !span_bad(p) {
                continue;
            }
            let allowed = |c: &Placed| {
                page.fits(c, view)
                    && !outside(c, region)
                    && (!outside(c, area) || outside(p, area))
                    && others.iter().zip(&old).all(|(q, before)| {
                        let contact = contacts(c, q);
                        !contact.contains(&4) && contact.is_subset(before)
                    })
                    && others
                        .iter()
                        .filter(|q| q.kind == "linear" && q.view == d.view)
                        .filter_map(|q| q.placed.as_ref())
                        .all(|q| {
                            !off_span(q, std::iter::once(c)) || off_span(q, std::iter::once(p))
                        })
            };
            let better = |c: &Placed| {
                let hits: usize = others.iter().map(|q| contacts(c, q).len()).sum();
                (hits, span_bad(c)) < (count, span_bad(p))
            };
            // Try the same lane first. Promotion stops at the sheet edge and has a fixed cap.
            let found = (0..=64)
                .take_while(|n| !outside(&promote(p, *n as f64 * LANE), region))
                .find_map(|n| {
                    let lane = promote(p, n as f64 * LANE);
                    alternatives(&lane, page.drawable())
                        .into_iter()
                        .find(|c| allowed(c) && better(c) && !span_bad(c))
                });
            if let Some(c) = found {
                inventory[i].placed = Some(c);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model;
    use serde_json::json;

    fn dimension(id: &str, kind: &str, value: f64, p: Placed) -> PlannedDimension {
        serde_json::from_value(json!({
            "id": id, "kind": kind, "view": "v:front", "value": value,
            "label": value.to_string(), "terms": [value.to_string()],
            "measurements": [{"feature": id, "parameter": "length", "value": value}],
            "default": true, "from_spec": [], "basis": "synthetic ink probe",
            "state": {"kind": "shown"}, "placed": p
        }))
        .unwrap()
    }

    #[test]
    fn promotion_skips_an_occupied_lane_preserves_pins_and_is_idempotent() {
        let analysis = model::analyse(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/plate.step"),
        )
        .unwrap()
        .1;
        let sheet: PlannedSheet = serde_json::from_value(json!({
            "size": "A4", "page": [297, 210], "scale": "1:1", "scale_factor": 1,
            "arrangement": "columns", "title_block": [200, 10, 287, 40],
            "default": true, "basis": "", "state": {"kind": "shown"}, "title": {}
        }))
        .unwrap();
        let mut size = dimension(
            "d:size/length",
            "linear",
            20.0,
            Placed {
                lines: vec![
                    [[20.0, 10.0], [20.0, 32.0]],
                    [[40.0, 10.0], [40.0, 32.0]],
                    [[20.0, 30.0], [40.0, 30.0]],
                ],
                arrows: vec![[[20.0, 30.0], [22.7, 30.0]], [[40.0, 30.0], [37.3, 30.0]]],
                label_at: [25.0, 31.0],
                label_box: [25.0, 31.0, 35.0, 33.0],
                rotation: 0.0,
            },
        );
        size.label = "20 ±1".into();
        size.terms = vec!["20".into(), "±1".into()];
        let peer = dimension(
            "d:peer/length",
            "linear",
            40.0,
            Placed {
                lines: vec![
                    [[15.0, 10.0], [15.0, 46.0]],
                    [[55.0, 10.0], [55.0, 46.0]],
                    [[15.0, 44.0], [55.0, 44.0]],
                ],
                arrows: vec![[[15.0, 44.0], [17.7, 44.0]], [[55.0, 44.0], [52.3, 44.0]]],
                label_at: [43.0, 45.0],
                label_box: [43.0, 45.0, 49.0, 47.0],
                rotation: 0.0,
            },
        );
        let mut datum = dimension(
            "pmi:datum:test",
            "callout",
            0.0,
            Placed {
                lines: vec![[[30.0, 10.0], [30.0, 35.0]]],
                arrows: vec![],
                label_at: [27.0, 35.0],
                label_box: [27.0, 35.0, 33.0, 41.0],
                rotation: 0.0,
            },
        );
        datum.label = "D".into();
        datum.terms = vec!["D".into()];
        datum.measurements.clear();
        let original = vec![size, peer];
        let fixed = vec![datum];
        let spec: Spec = serde_json::from_value(json!({
            "dimensions": {"d:peer/length": {"place": {"pinned": true}}}
        }))
        .unwrap();
        let p = original[0].placed.as_ref().unwrap();
        assert!(crosses(
            fixed[0].placed.as_ref().unwrap().lines[0],
            p.label_box
        ));
        assert!(
            alternatives(p, [10.0, 10.0, 287.0, 200.0])
                .iter()
                .all(|c| !contacts(c, &fixed[0]).is_empty()),
            "no along-span alternative is clear"
        );
        let mut items = original.clone();
        settle_inventory(&analysis, &spec, &sheet, &[], &mut items, &fixed);
        let c = items[0].placed.as_ref().unwrap();
        // Lane one meets the datum box; lane two is occupied by the peer's span.
        assert_eq!(c.arrows[0][0][1], 51.0);
        assert_eq!(
            c.arrows.iter().map(|a| a[0][0]).collect::<Vec<_>>(),
            vec![20.0, 40.0]
        );
        assert_eq!(
            c.lines[..2].iter().map(|l| l[0]).collect::<Vec<_>>(),
            p.lines[..2].iter().map(|l| l[0]).collect::<Vec<_>>()
        );
        assert_eq!(items[0].measurements, original[0].measurements);
        assert_eq!(items[1], original[1]);
        assert!(contacts(c, &fixed[0]).is_empty() && contacts(c, &items[1]).is_empty());
        let settled = items.clone();
        settle_inventory(&analysis, &spec, &sheet, &[], &mut items, &fixed);
        assert_eq!(items, settled);
        let mut pinned_spec = spec;
        pinned_spec.dimensions.insert(
            "d:size/length".into(),
            serde_json::from_value(json!({"place": {"pinned": true}})).unwrap(),
        );
        let mut pinned = original.clone();
        settle_inventory(&analysis, &pinned_spec, &sheet, &[], &mut pinned, &fixed);
        assert_eq!(pinned, original, "an infeasible pin remains visible");
    }
}
