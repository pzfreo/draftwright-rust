//! Python's bounded interior rays and corridor retry, against proven blank face and settled ink.
use super::placement::{Frame, Page, Side, arrow, boxes_overlap, crosses, label_box};
use super::{CAP, LANE, P2, Placed, PlannedDimension, Shape, Want, font, label_ink};
use crate::model::{Analysis, PlannedView, Spec};
use crate::output::Curve;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const BASIS: &str = "; proven blank interior of its carrying view";

fn padded(b: [f64; 4], pad: f64) -> [f64; 4] {
    [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]
}
fn contains(b: [f64; 4], p: P2) -> bool {
    p[0] >= b[0] && p[0] <= b[2] && p[1] >= b[1] && p[1] <= b[3]
}

/// Subdivision rejects uncertain curve intersections rather than sampling past thin ink.
fn curve_clear(c: &Curve, b: [f64; 4], depth: usize) -> bool {
    let cb = c.bbox();
    if cb[2] < b[0] || cb[0] > b[2] || cb[3] < b[1] || cb[1] > b[3] {
        return true;
    }
    if let Curve::Line(a, z) = c {
        return !crosses([*a, *z], b);
    }
    if depth == 16 || contains(b, c.start()) || contains(b, c.end()) {
        return false;
    }
    curve_clear(&c.trim(0.0, 0.5), b, depth + 1) && curve_clear(&c.trim(0.5, 1.0), b, depth + 1)
}

/// Prepared once per view and planning pass; candidate count never multiplies curve bounds.
pub(super) struct Proof<'a> {
    ink: &'a crate::model::projected_ink::ProjectedInk,
    bounds: Vec<(&'a Curve, [f64; 4])>,
    centre: P2,
}

pub(super) fn proofs<'a>(
    analysis: &'a Analysis,
    views: &[PlannedView],
) -> BTreeMap<String, Proof<'a>> {
    views
        .iter()
        .filter_map(|v| {
            if v.cut.is_some() || !crate::model::projected_ink::matches(v) {
                return None;
            }
            let ink = analysis.projected_views.get(&v.id)?;
            if ink.faces.is_empty() {
                return None;
            }
            let view = haecceity::hlr::View::new(v.direction, v.up).expect("planned view");
            Some((
                v.id.clone(),
                Proof {
                    ink,
                    bounds: ink
                        .visible
                        .iter()
                        .chain(&ink.hidden)
                        .map(|c| (c, c.bbox()))
                        .collect(),
                    centre: crate::model::extent(analysis, &view).1,
                },
            ))
        })
        .collect()
}

impl Proof<'_> {
    /// Whole-label material containment plus visible/hidden projected-ink clearance.
    pub(super) fn blank(&self, v: &PlannedView, label: [f64; 4]) -> bool {
        let unplace = |p: P2| {
            [
                (p[0] - v.at[0]) / v.scale + self.centre[0],
                (p[1] - v.at[1]) / v.scale + self.centre[1],
            ]
        };
        let lo = unplace([label[0], label[1]]);
        let hi = unplace([label[2], label[3]]);
        // Sampled boundaries are accurate to 0.0002 model mm. The padded label stays
        // farther away at every scale; visible stroke width is included in the paper gap.
        let b = padded([lo[0], lo[1], hi[0], hi[1]], (0.5 / v.scale).max(0.0005));
        let middle = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
        let material = self.ink.faces.iter().any(|face| {
            !face.iter().any(|s| crosses(*s, b))
                && face
                    .iter()
                    .filter(|[a, z]| {
                        (a[1] > middle[1]) != (z[1] > middle[1])
                            && middle[0] < a[0] + (middle[1] - a[1]) * (z[0] - a[0]) / (z[1] - a[1])
                    })
                    .count()
                    % 2
                    == 1
        });
        material
            && self.bounds.iter().all(|(c, cb)| {
                cb[2] < b[0] || cb[0] > b[2] || cb[3] < b[1] || cb[1] > b[3] || curve_clear(c, b, 0)
            })
    }
}

fn text_conflict(a: &Placed, b: &Placed) -> bool {
    boxes_overlap(padded(a.label_box, 0.5), b.label_box)
        || a.lines
            .iter()
            .chain(&a.arrows)
            .any(|s| crosses(*s, padded(b.label_box, 0.5)))
        || b.lines
            .iter()
            .chain(&b.arrows)
            .any(|s| crosses(*s, padded(a.label_box, 0.5)))
}

fn conflict(a: &Placed, b: &Placed) -> bool {
    text_conflict(a, b)
        || a.lines.iter().chain(&a.arrows).any(|s| {
            b.lines
                .iter()
                .chain(&b.arrows)
                .any(|t| super::leaders::lines_meet(*s, *t))
        })
}

fn callouts(
    frame: &Frame,
    w: &Want,
    old: &Placed,
    proof: &Proof<'_>,
    v: &PlannedView,
) -> Vec<Placed> {
    let Shape::Callout { sites } = &w.shape else {
        return Vec::new();
    };
    let ink = label_ink(&w.terms, font());
    let ray = old
        .lines
        .first()
        .map(|[a, b]| [b[0] - a[0], b[1] - a[1]])
        .unwrap_or([1.0, 0.0]);
    let n = ray[0].hypot(ray[1]).max(1e-12);
    let u = [ray[0] / n, ray[1] / n];
    let mut out = Vec::new();
    // Share Python's 64 alternatives round-robin over anchors; grouping cannot multiply work.
    for (rotation, lane) in [0.0_f64, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0, 180.0]
        .into_iter()
        .flat_map(|a| (1..=8).map(move |n| (a, n)))
    {
        let (s, c) = rotation.to_radians().sin_cos();
        let d = [u[0] * c - u[1] * s, u[0] * s + u[1] * c];
        for (centre, radius) in sites.iter().take(64) {
            let q = frame.page(*centre);
            let r = radius * frame.scale();
            let tip = [q[0] + r * d[0], q[1] + r * d[1]];
            let elbow = [
                tip[0] + lane as f64 * LANE * d[0],
                tip[1] + lane as f64 * LANE * d[1],
            ];
            let right = d[0] >= -1e-9;
            let end = [
                elbow[0]
                    + if right {
                        super::SHOULDER
                    } else {
                        -super::SHOULDER
                    },
                elbow[1],
            ];
            let label_at = [
                if right {
                    end[0] + 1.0 - ink.min[0]
                } else {
                    end[0] - 1.0 - ink.max[0]
                },
                elbow[1] - 0.5 * CAP,
            ];
            let p = Placed {
                lines: vec![[tip, elbow], [elbow, end]],
                arrows: vec![arrow(tip, elbow)],
                label_at,
                rotation: 0.0,
                label_box: label_box(&ink, label_at, 0.0),
            };
            if proof.blank(v, p.label_box) {
                out.push(p);
            }
            if out.len() == 64 {
                return out;
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn place(
    analysis: &Analysis,
    spec: &Spec,
    views: &[PlannedView],
    frames: &BTreeMap<&str, Frame>,
    items: &[Want],
    chosen: &[Option<&str>],
    sides: &[Option<Side>],
    placed: &mut [Option<Placed>],
    page: &Page,
    obstacles: &[PlannedDimension],
    cache: &super::footprints::Cache,
) -> BTreeSet<String> {
    let mut moved = BTreeSet::new();
    let proofs = proofs(analysis, views);
    for i in 0..items.len() {
        if spec
            .dimensions
            .get(&items[i].id)
            .is_some_and(|d| d.place.is_some())
        {
            continue;
        }
        let (Some(name), Some(old)) = (chosen[i], placed[i].as_ref()) else {
            continue;
        };
        let Some(v) = views.iter().find(|v| v.id.trim_start_matches("v:") == name) else {
            continue;
        };
        // A cutting-plane rider is fixed rendered ink. Until it is a plan item, refuse all
        // interior candidates on its carrying view; section hatching likewise has no evidence.
        if views.iter().any(|q| {
            q.cut
                .as_ref()
                .is_some_and(|c| c.marked_in == v.id || c.marked_in == name)
        }) {
            continue;
        }
        let Some(proof) = proofs.get(&v.id) else {
            continue;
        };
        let frame = &frames[name];
        let candidates = match &items[i].shape {
            Shape::Callout { .. } => callouts(frame, &items[i], old, proof, v),
            Shape::Linear { .. }
                if !page.fits(old, name)
                    || placed.iter().enumerate().any(|(j, p)| {
                        j != i && p.as_ref().is_some_and(|p| text_conflict(old, p))
                    }) =>
            {
                sides[i]
                    .map(|side| super::placement::interior_lanes(frame, &items[i], side))
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let f = cache.get(analysis, v);
        let candidate = candidates
            .into_iter()
            .filter(|p| {
                proof.blank(v, p.label_box)
                    && page.fits_interior(p, name)
                    && !placed
                        .iter()
                        .enumerate()
                        .any(|(j, q)| j != i && q.as_ref().is_some_and(|q| conflict(p, q)))
                    && !obstacles
                        .iter()
                        .filter_map(|d| d.placed.as_ref())
                        .any(|q| conflict(p, q))
                    && p.lines
                        .iter()
                        .take(if matches!(items[i].shape, Shape::Callout { .. }) {
                            1
                        } else {
                            2
                        })
                        .all(|s| {
                            if matches!(items[i].shape, Shape::Callout { .. }) {
                                f.crossed(*s, &items[i].measurements).is_empty()
                            } else {
                                f.crossed_extension(*s, &items[i].measurements).is_empty()
                            }
                        })
            })
            .min_by(|a, b| {
                let length = |p: &Placed| {
                    p.lines
                        .iter()
                        .map(|[a, b]| (a[0] - b[0]).hypot(a[1] - b[1]))
                        .sum::<f64>()
                };
                length(a).total_cmp(&length(b))
            });
        if let Some(p) = candidate {
            placed[i] = Some(p);
            moved.insert(items[i].id.clone());
        }
    }
    moved
}
