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
    fn model_point(&self, v: &PlannedView, p: P2) -> P2 {
        [
            (p[0] - v.at[0]) / v.scale + self.centre[0],
            (p[1] - v.at[1]) / v.scale + self.centre[1],
        ]
    }

    fn touches_visible(&self, v: &PlannedView, tip: P2) -> bool {
        let q = self.model_point(v, tip);
        let b = padded([q[0], q[1], q[0], q[1]], 0.05 / v.scale);
        self.ink.visible.iter().any(|c| !curve_clear(c, b, 0))
    }

    fn segment_clear(&self, v: &PlannedView, line: [P2; 2], attachment: bool) -> bool {
        let [a, b] = line;
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        let start = if attachment {
            (0.6 / length.max(1e-12)).min(1.0)
        } else {
            0.0
        };
        let steps = (length * (1.0 - start) / 0.2).ceil().max(1.0) as usize;
        let model_line = [self.model_point(v, a), self.model_point(v, b)];
        let nearby: Vec<_> = self
            .bounds
            .iter()
            .filter(|(_, cb)| crosses(model_line, padded(*cb, 0.2 / v.scale)))
            .collect();
        (0..steps).all(|i| {
            let point =
                |t: f64| self.model_point(v, [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            let q = point(start + (1.0 - start) * i as f64 / steps as f64);
            let z = point(start + (1.0 - start) * (i + 1) as f64 / steps as f64);
            let box_ = padded(
                [
                    q[0].min(z[0]),
                    q[1].min(z[1]),
                    q[0].max(z[0]),
                    q[1].max(z[1]),
                ],
                0.2 / v.scale,
            );
            nearby.iter().all(|(c, cb)| {
                cb[2] < box_[0]
                    || cb[0] > box_[2]
                    || cb[3] < box_[1]
                    || cb[1] > box_[3]
                    || curve_clear(c, box_, 0)
            })
        })
    }

    fn arrow_clear(&self, v: &PlannedView, [tip, tail]: [P2; 2], attachment: bool) -> bool {
        let d = [tail[0] - tip[0], tail[1] - tip[1]];
        let length = d[0].hypot(d[1]).max(1e-12);
        let t = if attachment {
            (0.6 / length).min(1.0)
        } else {
            0.0
        };
        let normal = [
            -super::ARROW_HALF * d[1] / length,
            super::ARROW_HALF * d[0] / length,
        ];
        let start = [tip[0] + t * d[0], tip[1] + t * d[1]];
        let points = [
            [start[0] + t * normal[0], start[1] + t * normal[1]],
            [start[0] - t * normal[0], start[1] - t * normal[1]],
            [tail[0] + normal[0], tail[1] + normal[1]],
            [tail[0] - normal[0], tail[1] - normal[1]],
        ];
        let b = points.into_iter().map(|p| self.model_point(v, p)).fold(
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
        let b = padded(b, 0.2 / v.scale);
        self.bounds.iter().all(|(c, cb)| {
            cb[2] < b[0] || cb[0] > b[2] || cb[3] < b[1] || cb[1] > b[3] || curve_clear(c, b, 0)
        })
    }

    pub(super) fn annotation_clear(&self, v: &PlannedView, p: &Placed, callout: bool) -> bool {
        self.blank(v, p.label_box)
            && (!callout || self.touches_visible(v, p.lines[0][0]))
            && p.lines
                .iter()
                .enumerate()
                .all(|(i, s)| self.segment_clear(v, *s, callout && i == 0))
            && p.arrows.iter().all(|s| self.arrow_clear(v, *s, callout))
            && (!callout || {
                let q = p.lines[0][1];
                self.blank(v, [q[0], q[1], q[0], q[1]])
            })
    }

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
        let clear = self.bounds.iter().all(|(c, cb)| {
            cb[2] < b[0] || cb[0] > b[2] || cb[3] < b[1] || cb[1] > b[3] || curve_clear(c, b, 0)
        });
        clear
            && self.ink.faces.iter().any(|face| {
                !face.iter().any(|s| crosses(*s, b))
                    && face
                        .iter()
                        .filter(|[a, z]| {
                            (a[1] > middle[1]) != (z[1] > middle[1])
                                && middle[0]
                                    < a[0] + (middle[1] - a[1]) * (z[0] - a[0]) / (z[1] - a[1])
                        })
                        .count()
                        % 2
                        == 1
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
    let radial = w
        .measurements
        .iter()
        .any(|m| m.parameter == "bore.diameter");
    let ink = label_ink(&w.terms, font());
    let ray = old
        .lines
        .first()
        .map(|[a, b]| [b[0] - a[0], b[1] - a[1]])
        .unwrap_or([1.0, 0.0]);
    let n = ray[0].hypot(ray[1]).max(1e-12);
    let u = [ray[0] / n, ray[1] / n];
    let page_point = |q: P2| {
        [
            v.at[0] + v.scale * (q[0] - proof.centre[0]),
            v.at[1] + v.scale * (q[1] - proof.centre[1]),
        ]
    };
    let mut attachments = vec![old.lines[0][0]];
    if !radial {
        // Physical arc anchors, rather than the absent half of a construction circle.
        for (centre, radius) in sites {
            let q = frame.page(*centre);
            for c in &proof.ink.visible {
                if let Curve::Conic { centre, radii, .. } = c {
                    let at = page_point(*centre);
                    if (at[0] - q[0]).hypot(at[1] - q[1]) <= 0.05
                        && radii
                            .iter()
                            .all(|r| (v.scale * r - frame.scale() * radius).abs() <= 0.05)
                    {
                        for station in [0.25, 0.5, 0.75] {
                            let tip = page_point(c.at(station));
                            if !attachments
                                .iter()
                                .any(|p| (p[0] - tip[0]).hypot(p[1] - tip[1]) <= 0.05)
                            {
                                attachments.push(tip);
                            }
                            if attachments.len() == 64 {
                                break;
                            }
                        }
                    }
                }
                if attachments.len() == 64 {
                    break;
                }
            }
            if attachments.len() == 64 {
                break;
            }
        }
    }
    let mut out = Vec::new();
    // Share Python's 64 alternatives round-robin over anchors; grouping cannot multiply work.
    for (rotation, lane) in [0.0_f64, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0, 180.0]
        .into_iter()
        .flat_map(|a| (1..=8).map(move |n| (a, n)))
    {
        let (s, c) = rotation.to_radians().sin_cos();
        let d = [u[0] * c - u[1] * s, u[0] * s + u[1] * c];
        let tips: Vec<_> = if radial {
            sites
                .iter()
                .take(64)
                .map(|(centre, radius)| {
                    let q = frame.page(*centre);
                    let r = radius * frame.scale();
                    [q[0] + r * d[0], q[1] + r * d[1]]
                })
                .collect()
        } else {
            attachments.clone()
        };
        for tip in tips {
            let outline = frame.outline();
            let exit = (0..2)
                .filter_map(|k| {
                    if d[k] > 0.0 {
                        Some((outline[k + 2] - tip[k]) / d[k])
                    } else if d[k] < 0.0 {
                        Some((outline[k] - tip[k]) / d[k])
                    } else {
                        None
                    }
                })
                .fold(f64::INFINITY, f64::min)
                .max(0.0);
            if lane as f64 * LANE + 0.5 >= exit {
                continue;
            }
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
            if proof.annotation_clear(v, &p, true) {
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
        let f = cache.get(analysis, v);
        let candidates =
            match &items[i].shape {
                Shape::Callout { .. } => callouts(frame, &items[i], old, proof, v),
                Shape::Linear { .. }
                    if !page.fits(old, name)
                        || old.lines.iter().take(2).any(|s| {
                            !f.crossed_extension(*s, &items[i].measurements).is_empty()
                        })
                        || obstacles
                            .iter()
                            .filter_map(|d| d.placed.as_ref())
                            .any(|p| conflict(old, p))
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
        let candidate = candidates
            .into_iter()
            .filter_map(|mut p| {
                if matches!(items[i].shape, Shape::Linear { .. })
                    && !p
                        .lines
                        .iter_mut()
                        .take(2)
                        .all(|s| f.interior_witness(s, &items[i].measurements))
                {
                    return None;
                }
                Some(p)
            })
            .filter(|p| {
                proof.annotation_clear(v, p, matches!(items[i].shape, Shape::Callout { .. }))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> PlannedView {
        serde_json::from_value(serde_json::json!({
            "id": "v:plan", "direction": [0,0,1], "up": [0,1,0],
            "at": [0,0], "scale": 1, "area": [-50,-50,50,50],
            "state": {"kind": "shown"}, "default": true, "basis": "test"
        }))
        .unwrap()
    }

    fn ink(curves: Vec<Curve>) -> crate::model::projected_ink::ProjectedInk {
        crate::model::projected_ink::ProjectedInk {
            visible: curves,
            hidden: vec![],
            faces: vec![vec![
                [[-50., -50.], [50., -50.]],
                [[50., -50.], [50., 50.]],
                [[50., 50.], [-50., 50.]],
                [[-50., 50.], [-50., -50.]],
            ]],
        }
    }

    fn proof(ink: &crate::model::projected_ink::ProjectedInk) -> Proof<'_> {
        Proof {
            ink,
            bounds: ink
                .visible
                .iter()
                .chain(&ink.hidden)
                .map(|c| (c, c.bbox()))
                .collect(),
            centre: [0., 0.],
        }
    }

    fn dimension() -> Placed {
        Placed {
            lines: vec![
                [[-10., -10.], [-10., 0.]],
                [[10., -10.], [10., 0.]],
                [[-10., 0.], [10., 0.]],
            ],
            arrows: vec![[[-10., 0.], [-8., 0.]], [[10., 0.], [8., 0.]]],
            label_box: [-3., 2., 3., 5.],
            label_at: [-3., 2.],
            rotation: 0.,
        }
    }

    #[test]
    fn blank_label_does_not_excuse_dimension_line_or_arrow_on_ink() {
        let v = view();
        let p = dimension();
        assert!(proof(&ink(vec![])).annotation_clear(&v, &p, false));
        for line in [
            [[-20., 0.], [20., 0.]],
            [[-9., -1.], [-9., 1.]],
            [[0., -1.], [0., 1.]],
        ] {
            let mut ink = ink(vec![]);
            ink.hidden.push(Curve::Line(line[0], line[1]));
            let proof = proof(&ink);
            assert!(proof.blank(&v, p.label_box));
            assert!(!proof.annotation_clear(&v, &p, false));
        }
    }

    #[test]
    fn partial_arc_tip_and_complete_leader_must_clear_visible_ink() {
        let v = view();
        let mut ink = ink(vec![Curve::Conic {
            centre: [0., 0.],
            radii: [5., 5.],
            x_axis: [1., 0.],
            y_axis: [0., 1.],
            t0: 0.,
            t1: std::f64::consts::PI,
        }]);
        let p = Placed {
            lines: vec![[[0., 5.], [0., 12.]], [[0., 12.], [5., 12.]]],
            arrows: vec![[[0., 5.], [0., 7.]]],
            label_box: [6., 11., 12., 14.],
            label_at: [6., 11.],
            rotation: 0.,
        };
        assert!(proof(&ink).annotation_clear(&v, &p, true));
        assert!(!proof(&ink).touches_visible(&v, [0., -5.]));
        for line in [[[-2., 9.], [2., 9.]], [[3., 10.], [3., 15.]]] {
            ink.hidden = vec![Curve::Line(line[0], line[1])];
            assert!(proof(&ink).blank(&v, p.label_box));
            assert!(!proof(&ink).annotation_clear(&v, &p, true));
        }
        ink.hidden.clear();
        let mut void = p;
        void.lines[0][1] = [0., 60.];
        assert!(!proof(&ink).annotation_clear(&v, &void, true));
    }
    #[test]
    fn failed_exterior_corridors_retry_the_whole_linear_dimension_inside() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/mounting_plate.step");
        let mut a = crate::model::analyse_native(&file).unwrap().1;
        let sheet = crate::model::plan(&a, &Spec::default()).unwrap().sheet;
        a.envelope = [[-50., -50., -1.], [50., 50., 1.]];
        a.features.clear();
        a.principal_profiles.clear();
        a.projected_views.clear();
        a.projected_views.insert("v:plan".into(), ink(vec![]));
        let mut v = view();
        v.at = [150., 150.];
        v.area = [80., 80., 220., 220.];
        let views = [v];
        let frame = Frame::new(&a, &views[0]);
        let frames = BTreeMap::from([("plan", frame)]);
        let w = Want {
            id: "d:test/location.length.x".into(),
            views: vec!["plan"],
            value: 20.,
            terms: vec!["20".into()],
            measurements: vec![],
            basis: "test".into(),
            turning: None,
            shape: Shape::Linear {
                a: [-10., -20., 0.],
                b: [10., -20., 0.],
                axis: 0,
                a_edge: false,
                b_edge: false,
                order: 1,
            },
        };
        let mut old = dimension();
        old.label_box = [10000., 10000., 10006., 10003.];
        let page = Page::new(&a, &sheet, &views);
        assert!(!page.fits(&old, "plan"));
        let mut placed = [Some(old)];
        let moved = place(
            &a,
            &Spec::default(),
            &views,
            &frames,
            &[w],
            &[Some("plan")],
            &[Some(Side::Above)],
            &mut placed,
            &page,
            &[],
            &super::super::footprints::Cache::default(),
        );
        assert!(moved.contains("d:test/location.length.x"));
        let p = placed[0].as_ref().unwrap();
        assert_eq!(p.lines.len(), 3);
        assert_eq!(p.arrows.len(), 2);
        assert!(page.fits_interior(p, "plan"));
        assert!(proofs(&a, &views)["v:plan"].annotation_clear(&views[0], p, false));
    }
    #[test]
    fn radius_candidates_use_physical_arc_anchors_and_never_the_missing_semicircle() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/mounting_plate.step");
        let mut a = crate::model::analyse_native(&file).unwrap().1;
        a.envelope = [[-50., -50., -1.], [50., 50., 1.]];
        let v = view();
        let frame = Frame::new(&a, &v);
        let ink = ink(vec![Curve::Conic {
            centre: [0., 0.],
            radii: [5., 5.],
            x_axis: [1., 0.],
            y_axis: [0., 1.],
            t0: 0.,
            t1: std::f64::consts::PI,
        }]);
        let proof = proof(&ink);
        let w = Want {
            id: "d:test/slot_end_radius.radius".into(),
            views: vec!["plan"],
            value: 5.,
            terms: vec!["R5".into()],
            measurements: vec![],
            basis: "test".into(),
            turning: None,
            shape: Shape::Callout {
                sites: vec![([0., 0., 0.], 5.)],
            },
        };
        let mut old = dimension();
        old.lines = vec![[[0., -5.], [0., -12.]], [[0., -12.], [3., -12.]]];
        let candidates = callouts(&frame, &w, &old, &proof, &v);
        assert!(!candidates.is_empty());
        for p in candidates {
            assert!(p.lines[0][0][1] >= 0.0);
            assert!(proof.annotation_clear(&v, &p, true));
        }
    }
}
