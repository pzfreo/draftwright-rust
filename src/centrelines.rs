//! ISO 128 chain-line furniture, resolved in page mm before annotation placement.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bridge::holes;
use crate::dims::{Placed, PlannedDimension};
use crate::model::{Analysis, PlannedView, extent};
use crate::output::{Curve, P};
use haecceity::hlr::View;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineWork {
    pub id: String,
    /// `cross`, `axis`, `pitch_circle` or `cutting_plane`.
    pub kind: String,
    pub curves: Vec<Curve>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre: Option<P>,
    #[serde(default)]
    pub radial: bool,
    #[serde(default)]
    pub global_axis: bool,
}

pub fn overrun(radius: f64) -> f64 {
    radius.clamp(1.0, 3.0).max(radius * 0.05)
}

fn page(a: &Analysis, v: &PlannedView, p: [f64; 3]) -> P {
    let view = View::new(v.direction, v.up).expect("planned view");
    let centre = extent(a, &view).1;
    let q = view.map(p);
    [0, 1].map(|k| v.at[k] + v.scale * (q[k] - centre[k]))
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}

fn removed(
    cut: Option<&crate::model::Cut>,
    at: [f64; 3],
    d: [f64; 3],
    length: f64,
    diameter: f64,
) -> bool {
    cut.is_some_and(|c| {
        let axial = dot(d, c.normal).abs().min(1.);
        let support = (length * axial + diameter * (1. - axial * axial).sqrt()) / 2.;
        dot([0, 1, 2].map(|k| at[k] - c.origin[k]), c.normal) < -support - 1e-6
    })
}

fn cross(id: String, c: P, radius: f64, radial: Option<P>) -> LineWork {
    let u = radial.unwrap_or([1.0, 0.0]);
    let r = radius + overrun(radius);
    let curves = [u, [-u[1], u[0]]].map(|d| {
        Curve::Line(
            [c[0] - r * d[0], c[1] - r * d[1]],
            [c[0] + r * d[0], c[1] + r * d[1]],
        )
    });
    LineWork {
        id,
        kind: "cross".into(),
        curves: curves.into(),
        centre: Some(c),
        radial: radial.is_some(),
        global_axis: false,
    }
}

fn end_on(v: &PlannedView, d: [f64; 3]) -> bool {
    let k = (0..3)
        .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()).then(b.cmp(&a)))
        .expect("three components");
    View::new(v.direction, v.up).unwrap().toward[k].abs() > 1. - 1e-6
}

fn end_cross(
    v: &PlannedView,
    id: String,
    c: P,
    d: [f64; 3],
    r: f64,
    radial: Option<P>,
) -> LineWork {
    let view = View::new(v.direction, v.up).unwrap();
    let p = view.map(d);
    let n = p[0].hypot(p[1]);
    let mut w = cross(id, c, r, radial);
    if n > 1e-6 && radial.is_none() {
        let u = p.map(|x| x / n);
        let minor = r * dot(d, view.toward).abs().min(1.);
        w.curves = [(u, minor), ([-u[1], u[0]], r)]
            .map(|(d, r)| {
                let r = r + overrun(r);
                Curve::Line(
                    [c[0] - r * d[0], c[1] - r * d[1]],
                    [c[0] + r * d[0], c[1] + r * d[1]],
                )
            })
            .into();
    }
    w
}

#[allow(clippy::too_many_arguments)]
fn axis(
    a: &Analysis,
    v: &PlannedView,
    id: String,
    at: [f64; 3],
    direction: [f64; 3],
    length: f64,
    diameter: f64,
    global: bool,
) -> Option<LineWork> {
    let view = View::new(v.direction, v.up)?;
    if dot(direction, view.toward).abs() > 1.0 - 1e-6 {
        return None;
    }
    let (p, q) = (
        page(a, v, [0, 1, 2].map(|k| at[k] - direction[k] * length / 2.0)),
        page(a, v, [0, 1, 2].map(|k| at[k] + direction[k] * length / 2.0)),
    );
    let d = [q[0] - p[0], q[1] - p[1]];
    let n = d[0].hypot(d[1]);
    if n < 1e-6 {
        return None;
    }
    let extra = if global {
        5.0
    } else {
        overrun(diameter * v.scale / 2.0)
    };
    let d = d.map(|x| x * extra / n);
    Some(LineWork {
        id,
        kind: "axis".into(),
        curves: vec![Curve::Line(
            [p[0] - d[0], p[1] - d[1]],
            [q[0] + d[0], q[1] + d[1]],
        )],
        centre: None,
        radial: false,
        global_axis: global,
    })
}

/// Populate every shown view; dimensional omissions never affect furniture.
pub(crate) fn plan(a: &Analysis, views: &mut [PlannedView]) {
    let frame = holes::frame(a);
    let holes: Vec<_> = holes::read(a)
        .into_iter()
        .filter_map(|(_, h)| h.ok())
        .collect();
    let bands: Vec<_> = a
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .filter_map(|f| crate::lint::turned::band(&frame, f).ok().map(|b| (f, b)))
        .collect();
    let mut round = holes::bosses(a);
    round.extend(bands.iter().map(|(_, b)| {
        let mut direction = [0.; 3];
        direction[b.axis] = 1.;
        holes::Axis {
            at: b.on,
            direction,
        }
    }));
    let [lo, hi] = a.envelope;
    for v in views.iter_mut() {
        v.line_work.clear();
        if v.id == "v:iso" {
            continue;
        }
        let view = View::new(v.direction, v.up).expect("planned view");
        let hole_length = |h: &holes::Hole<'_>| {
            h.depth
                .unwrap_or_else(|| (0..3).map(|k| (hi[k] - lo[k]) * h.direction[k].abs()).sum())
        };
        let hole_diameter = |h: &holes::Hole<'_>| {
            h.counterbore
                .map_or(h.diameter, |r| r.diameter)
                .max(h.spotface.map_or(0., |r| r.diameter))
                .max(h.countersink.map_or(0., |r| r.diameter))
        };
        let hole_removed = |h: &holes::Hole<'_>| {
            removed(
                v.cut.as_ref(),
                h.feature.fingerprint.position,
                h.direction,
                hole_length(h),
                hole_diameter(h),
            )
        };
        let mut radial = BTreeMap::new();
        for p in a
            .recognition
            .iter()
            .flat_map(|r| &r.patterns)
            .filter(|p| p.arrangement == "bolt_circle")
        {
            if p.members.len() < 5
                && holes::established_with(p, &holes.iter().collect::<Vec<_>>(), &round).is_err()
            {
                continue;
            }
            let Some(h) = holes.iter().find(|h| p.members.contains(&h.feature.id)) else {
                continue;
            };
            if dot(h.direction, view.toward).abs() < 1.0 - 1e-6 {
                continue;
            }
            let Some(c) =
                holes::point(p.parameters.get("center")).map(|p| holes::to_part(&frame, p))
            else {
                continue;
            };
            let Some(diameter) = p.parameters.get("diameter").and_then(|x| x.as_f64()) else {
                continue;
            };
            if diameter <= 0.0 {
                continue;
            }
            let c = page(a, v, c);
            let r = diameter * v.scale / 2.0;
            // Require every member to lie on the recorded circle, rather than inventing one.
            let members: Vec<_> = p
                .members
                .iter()
                .filter_map(|id| holes.iter().find(|h| &h.feature.id == id))
                .collect();
            if members.len() != p.members.len()
                || members.iter().any(|h| hole_removed(h))
                || members.iter().any(|h| {
                    let q = page(a, v, h.at);
                    ((q[0] - c[0]).hypot(q[1] - c[1]) - r).abs() > 0.05
                })
            {
                continue;
            }
            for h in members {
                let q = page(a, v, h.at);
                radial.insert(
                    h.feature.id.as_str(),
                    [(q[0] - c[0]) / r, (q[1] - c[1]) / r],
                );
            }
            v.line_work.push(LineWork {
                id: format!("ink:{}:{}", v.id, p.id),
                kind: "pitch_circle".into(),
                curves: vec![Curve::Conic {
                    centre: c,
                    radii: [r, r],
                    x_axis: [1., 0.],
                    y_axis: [0., 1.],
                    t0: 0.,
                    t1: std::f64::consts::TAU,
                }],
                centre: None,
                radial: false,
                global_axis: false,
            });
        }
        // Native recognition may publish the members without their bolt-circle record.
        // Five equal, coplanar bores must all fit one circle; a rectangular grid is not evidence.
        let mut groups: BTreeMap<Vec<i64>, Vec<&holes::Hole<'_>>> = BTreeMap::new();
        for h in &holes {
            if dot(h.direction, view.toward).abs() < 1.0 - 1e-6
                || radial.contains_key(h.feature.id.as_str())
            {
                continue;
            }
            let mut key = h.requirement();
            key.push(holes::size_key(dot(h.at, view.toward)));
            groups.entry(key).or_default().push(h);
        }
        for members in groups.values().filter(|g| g.len() >= 5) {
            if members.iter().any(|h| hole_removed(h)) {
                continue;
            }
            let points: Vec<_> = members.iter().map(|h| page(a, v, h.at)).collect();
            if let Some((c, r)) = fitted_circle(&points, v.scale) {
                for (h, q) in members.iter().zip(&points) {
                    radial.insert(
                        h.feature.id.as_str(),
                        [(q[0] - c[0]) / r, (q[1] - c[1]) / r],
                    );
                }
                v.line_work.push(LineWork {
                    id: format!("ink:{}:pitch:{}", v.id, members[0].feature.id),
                    kind: "pitch_circle".into(),
                    curves: vec![Curve::Conic {
                        centre: c,
                        radii: [r, r],
                        x_axis: [1., 0.],
                        y_axis: [0., 1.],
                        t0: 0.,
                        t1: std::f64::consts::TAU,
                    }],
                    centre: None,
                    radial: false,
                    global_axis: false,
                });
            }
        }
        for h in &holes {
            let at = h.feature.fingerprint.position;
            if hole_removed(h) {
                continue;
            }
            let id = format!("ink:{}:{}", v.id, h.feature.id);
            let radius = hole_diameter(h) * v.scale / 2.;
            if end_on(v, h.direction) {
                v.line_work.push(end_cross(
                    v,
                    id,
                    page(a, v, h.at),
                    h.direction,
                    radius,
                    radial.get(h.feature.id.as_str()).copied(),
                ));
            } else {
                let length = hole_length(h);
                if let Some(w) = axis(a, v, id, at, h.direction, length, h.diameter, false) {
                    v.line_work.push(w);
                }
            }
        }
        for f in a.features.iter().filter(|f| f.kind == "double_d_bore") {
            let p = &f.parameters;
            if let (Some(at), Some(d), Some(diameter)) = (
                holes::point(p.get("location")),
                holes::point(p.get("axis")),
                p.get("major_diameter").and_then(|x| x.as_f64()),
            ) {
                let (at, d) = (holes::to_part(&frame, at), holes::dir_to_part(&frame, d));
                let length = p
                    .get("depth")
                    .and_then(|x| x.as_f64())
                    .unwrap_or_else(|| (0..3).map(|k| (hi[k] - lo[k]) * d[k].abs()).sum());
                let centre = f.fingerprint.position;
                if removed(v.cut.as_ref(), centre, d, length, diameter) {
                    continue;
                }
                let id = format!("ink:{}:{}", v.id, f.id);
                if end_on(v, d) {
                    v.line_work.push(end_cross(
                        v,
                        id,
                        page(a, v, at),
                        d,
                        diameter * v.scale / 2.,
                        None,
                    ));
                } else if let Some(w) = axis(a, v, id, centre, d, length, diameter, false) {
                    v.line_work.push(w);
                }
            }
        }
        for (f, b) in &bands {
            let mut direction = [0.; 3];
            direction[b.axis] = 1.;
            let mut at = b.on;
            at[b.axis] = (b.lo + b.hi) / 2.;
            if removed(v.cut.as_ref(), at, direction, b.hi - b.lo, b.diameter) {
                continue;
            }
            let global = bands.iter().all(|(_, other)| {
                other.axis == b.axis
                    && (0..3).all(|k| k == b.axis || (other.on[k] - b.on[k]).abs() < 1e-3)
            }) && (0..3).filter(|&k| k != b.axis).all(|k| {
                ((hi[k] - lo[k]) - bands.iter().map(|(_, b)| b.diameter).fold(0., f64::max)).abs()
                    < 1e-3
            });
            let length = if global {
                at[b.axis] = (lo[b.axis] + hi[b.axis]) / 2.;
                hi[b.axis] - lo[b.axis]
            } else {
                b.hi - b.lo
            };
            if let Some(w) = axis(
                a,
                v,
                format!("ink:{}:{}", v.id, f.id),
                at,
                direction,
                length,
                b.diameter,
                global,
            ) {
                v.line_work.push(w);
            } else if dot(direction, view.toward).abs() > 1. - 1e-6 {
                v.line_work.push(cross(
                    format!("ink:{}:{}", v.id, f.id),
                    page(a, v, at),
                    b.diameter * v.scale / 2.,
                    None,
                ));
            }
        }
        for f in a.features.iter().filter(|f| f.kind == "boss") {
            if let (Some(_at), Some(d), Some(diameter), Some(length)) = (
                holes::point(f.parameters.get("location")),
                holes::point(f.parameters.get("axis")),
                f.parameters.get("diameter").and_then(|x| x.as_f64()),
                f.parameters.get("height").and_then(|x| x.as_f64()),
            ) {
                let d = holes::dir_to_part(&frame, d);
                let at = f.fingerprint.position;
                if removed(v.cut.as_ref(), at, d, length, diameter) {
                    continue;
                }
                if end_on(v, d) {
                    v.line_work.push(end_cross(
                        v,
                        format!("ink:{}:{}", v.id, f.id),
                        page(
                            a,
                            v,
                            holes::to_part(
                                &frame,
                                holes::point(f.parameters.get("location")).unwrap(),
                            ),
                        ),
                        d,
                        diameter * v.scale / 2.,
                        None,
                    ));
                } else if let Some(w) = axis(
                    a,
                    v,
                    format!("ink:{}:{}", v.id, f.id),
                    at,
                    d,
                    length,
                    diameter,
                    false,
                ) {
                    v.line_work.push(w);
                }
            }
        }
        for (f, s) in crate::lint::channels::seats(a) {
            let Ok(s) = s else {
                continue;
            };
            let mut d = [0.; 3];
            d[s.run_axis] = 1.;
            if removed(v.cut.as_ref(), s.centre, d, s.length, 2. * s.radius) {
                continue;
            }
            if let Some(w) = axis(
                a,
                v,
                format!("ink:{}:{}", v.id, f.id),
                s.centre,
                d,
                s.length,
                2. * s.radius,
                false,
            ) {
                v.line_work.push(w);
            } else if dot(d, view.toward).abs() > 1. - 1e-6 {
                v.line_work.push(cross(
                    format!("ink:{}:{}", v.id, f.id),
                    page(a, v, s.centre),
                    s.radius * v.scale,
                    None,
                ));
            }
        }
        merge(&mut v.line_work);
    }
    let cuts: Vec<_> = views.iter().filter_map(|v| v.cut.clone()).collect();
    for c in cuts {
        if let Some(v) = views.iter_mut().find(|v| v.id == c.marked_in) {
            let p = page(a, v, [lo[0], c.origin[1], lo[2]]);
            let q = page(a, v, [hi[0], c.origin[1], hi[2]]);
            let y = page(a, v, c.origin)[1];
            v.line_work.push(LineWork {
                id: format!("ink:{}:cut:{}", v.id, c.label),
                kind: "cutting_plane".into(),
                curves: vec![Curve::Line(
                    [p[0].min(q[0]) - crate::section::OVERRUN, y],
                    [p[0].max(q[0]) + crate::section::OVERRUN, y],
                )],
                centre: None,
                radial: false,
                global_axis: false,
            });
        }
    }
}

fn fitted_circle(points: &[P], scale: f64) -> Option<(P, f64)> {
    let n = points.len() as f64;
    let mean = [0, 1].map(|k| points.iter().map(|p| p[k]).sum::<f64>() / n);
    let (mut xx, mut xy, mut yy, mut xz, mut yz) = (0., 0., 0., 0., 0.);
    for p in points {
        let (x, y) = (p[0] - mean[0], p[1] - mean[1]);
        let z = x * x + y * y;
        xx += x * x;
        xy += x * y;
        yy += y * y;
        xz += x * z;
        yz += y * z;
    }
    let det = xx * yy - xy * xy;
    if det <= 1e-10 * (xx + yy).powi(2) {
        return None;
    }
    let c = [
        mean[0] + (xz * yy - yz * xy) / (2. * det),
        mean[1] + (yz * xx - xz * xy) / (2. * det),
    ];
    let radii: Vec<_> = points
        .iter()
        .map(|p| (p[0] - c[0]).hypot(p[1] - c[1]))
        .collect();
    let r = radii.iter().sum::<f64>() / n;
    if r <= 0.
        || radii
            .iter()
            .any(|q| (q - r).abs() > (0.01 * scale).max(r * 1e-4))
    {
        return None;
    }
    Some((c, r))
}

fn merge(work: &mut Vec<LineWork>) {
    let mut out: Vec<LineWork> = Vec::new();
    for w in work.drain(..) {
        if w.kind == "cross"
            && let Some(p) = out.iter_mut().find(|p| {
                p.kind == w.kind
                    && p.centre
                        .zip(w.centre)
                        .is_some_and(|(p, q)| (p[0] - q[0]).hypot(p[1] - q[1]) < 1e-6)
            })
        {
            let length = |w: &LineWork| {
                let c = &w.curves[0];
                let (a, b) = (c.start(), c.end());
                (b[0] - a[0]).hypot(b[1] - a[1])
            };
            let size = length(p).max(length(&w));
            let c = p.centre.unwrap();
            let basis = if w.radial { &w } else { &*p };
            let curve = &basis.curves[0];
            let (a, b) = (curve.start(), curve.end());
            let n = (b[0] - a[0]).hypot(b[1] - a[1]);
            let u = [(b[0] - a[0]) / n, (b[1] - a[1]) / n];
            p.curves = [u, [-u[1], u[0]]]
                .into_iter()
                .map(|d| {
                    Curve::Line(
                        [c[0] - size * d[0] / 2., c[1] - size * d[1] / 2.],
                        [c[0] + size * d[0] / 2., c[1] + size * d[1] / 2.],
                    )
                })
                .collect();
            p.radial |= w.radial;
            continue;
        }
        if w.kind == "axis" {
            let Curve::Line(a, b) = w.curves[0] else {
                unreachable!()
            };
            let vertical = (a[0] - b[0]).abs() < 1e-6;
            if let Some(p) = out.iter_mut().find(|p| {
                p.kind == "axis" && {
                    let Curve::Line(c, d) = p.curves[0] else {
                        return false;
                    };
                    if vertical {
                        (c[0] - d[0]).abs() < 1e-6
                            && (a[0] - c[0]).abs() < 1e-6
                            && a[1].min(b[1]) <= c[1].max(d[1])
                            && c[1].min(d[1]) <= a[1].max(b[1])
                    } else {
                        (a[1] - b[1]).abs() < 1e-6
                            && (c[1] - d[1]).abs() < 1e-6
                            && (a[1] - c[1]).abs() < 1e-6
                            && a[0].min(b[0]) <= c[0].max(d[0])
                            && c[0].min(d[0]) <= a[0].max(b[0])
                    }
                }
            }) {
                let Curve::Line(c, d) = p.curves[0] else {
                    unreachable!()
                };
                p.curves[0] = if vertical {
                    Curve::Line(
                        [a[0], a[1].min(b[1]).min(c[1]).min(d[1])],
                        [a[0], a[1].max(b[1]).max(c[1]).max(d[1])],
                    )
                } else {
                    Curve::Line(
                        [a[0].min(b[0]).min(c[0]).min(d[0]), a[1]],
                        [a[0].max(b[0]).max(c[0]).max(d[0]), a[1]],
                    )
                };
                p.global_axis |= w.global_axis;
                continue;
            }
        }
        out.push(w);
    }
    *work = out;
}

pub fn segments(w: &LineWork) -> Vec<[P; 2]> {
    w.curves
        .iter()
        .flat_map(|c| {
            let n = match c {
                Curve::Line(..) => 1,
                Curve::Conic { radii, .. } => ((std::f64::consts::PI * radii[0].max(radii[1])
                    / 0.01)
                    .sqrt()
                    .ceil() as usize)
                    .max(32),
                _ => 128,
            };
            (0..n)
                .map(|i| [c.at(i as f64 / n as f64), c.at((i + 1) as f64 / n as f64)])
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn obstacles(views: &[PlannedView]) -> Vec<PlannedDimension> {
    views
        .iter()
        .flat_map(|v| {
            v.line_work.iter().map(|w| PlannedDimension {
                id: w.id.clone(),
                kind: "ink".into(),
                view: v.id.clone(),
                value: 0.,
                label: String::new(),
                terms: Vec::new(),
                measurements: Vec::new(),
                default: true,
                from_spec: Vec::new(),
                basis: if w.global_axis {
                    "global_axis".into()
                } else {
                    String::new()
                },
                state: crate::items::State::Shown,
                placed: Some(Placed {
                    lines: segments(w),
                    arrows: Vec::new(),
                    label_at: [0.; 2],
                    rotation: 0.,
                    label_box: [0.; 4],
                }),
            })
        })
        .collect()
}

/// Only a transverse attachment in the arrow neighbourhood of a marked global
/// turning axis is intentional. Shelf crossings and travel along the axis count.
pub(crate) fn leader_conflict(line: [P; 2], fixed: [P; 2], global: bool, shaft: bool) -> bool {
    let u = [line[1][0] - line[0][0], line[1][1] - line[0][1]];
    let v = [fixed[1][0] - fixed[0][0], fixed[1][1] - fixed[0][1]];
    let w = [fixed[0][0] - line[0][0], fixed[0][1] - line[0][1]];
    let cross = |a: P, b: P| a[0] * b[1] - a[1] * b[0];
    let (lu, lv) = (u[0].hypot(u[1]), v[0].hypot(v[1]));
    if lu < 1e-9 || lv < 1e-9 {
        return false;
    }
    let det = cross(u, v);
    if det.abs() < 1e-9 * lu * lv {
        if cross(w, u).abs() > 1e-6 * lu {
            return false;
        }
        let along = |p: P| ((p[0] - line[0][0]) * u[0] + (p[1] - line[0][1]) * u[1]) / lu;
        let (a, b) = (along(fixed[0]), along(fixed[1]));
        return a.max(b).min(lu) - a.min(b).max(0.) > 1e-6;
    }
    let (t, s) = (cross(w, v) / det, cross(w, u) / det);
    if !(-1e-9..=1. + 1e-9).contains(&t) || !(-1e-9..=1. + 1e-9).contains(&s) {
        return false;
    }
    !(global && shaft && t * lu <= crate::dims::ARROW && det.abs() / (lu * lv) > 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oblique_holes_mark_the_projected_ellipse_and_axes() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/plate.step");
        let (_, plate) = crate::model::analyse_native(&fixture).unwrap();
        let mut views = crate::model::plan(&plate, &crate::model::Spec::default())
            .unwrap()
            .views;
        let mut a = crate::lint::paired_ramp_steps::tests::part(
            "hole",
            serde_json::json!({"axis":[0.,0.6,0.8], "location":[50.,50.,5.],
                "diameter":20., "depth":10., "through":true}),
        );
        a.features[0].fingerprint.position = [50., 50., 5.];
        assert!(holes::read(&a)[0].1.is_ok());
        plan(&a, &mut views);
        let mut axes = 0;
        let mut marks = 0;
        for v in views.iter().filter(|v| v.id != "v:iso") {
            for w in &v.line_work {
                if w.kind == "axis" {
                    axes += 1;
                } else if w.kind == "cross" {
                    let mut lengths: Vec<_> = w
                        .curves
                        .iter()
                        .map(|c| {
                            let (p, q) = (c.start(), c.end());
                            (q[0] - p[0]).hypot(q[1] - p[1])
                        })
                        .collect();
                    lengths.sort_by(f64::total_cmp);
                    assert!(
                        (lengths[0] - 2. * (8. * v.scale + overrun(8. * v.scale))).abs() < 1e-6
                    );
                    assert!(
                        (lengths[1] - 2. * (10. * v.scale + overrun(10. * v.scale))).abs() < 1e-6
                    );
                    marks += 1;
                }
            }
        }
        assert_eq!(marks, 1);
        assert_eq!(axes, 2);
    }
    #[test]
    fn profiled_bores_have_side_axes_as_well_as_end_crosses() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/plate.step");
        let (_, plate) = crate::model::analyse_native(&fixture).unwrap();
        let mut views = crate::model::plan(&plate, &crate::model::Spec::default())
            .unwrap()
            .views;
        let mut a = crate::lint::paired_ramp_steps::tests::part(
            "double_d_bore",
            serde_json::json!({"axis":[0.,0.,1.], "location":[50.,50.,10.],
                "major_diameter":10., "across_flats":7.2, "depth":10., "through":true,
                "flat_direction":[1.,0.,0.]}),
        );
        a.features[0].fingerprint.position = [50., 50., 5.];
        plan(&a, &mut views);
        let mut sides = 0;
        let mut ends = 0;
        for v in views.iter().filter(|v| v.id != "v:iso") {
            let view = View::new(v.direction, v.up).unwrap();
            if dot([0., 0., 1.], view.toward).abs() < 1e-6 {
                let w = v.line_work.iter().find(|w| w.kind == "axis").unwrap();
                let (p, q) = (w.curves[0].start(), w.curves[0].end());
                let length = (q[0] - p[0]).hypot(q[1] - p[1]);
                assert!((length - (10. * v.scale + 2. * overrun(5. * v.scale))).abs() < 1e-6);
                let centre = page(&a, v, a.features[0].fingerprint.position);
                assert!(
                    ((p[0] + q[0]) / 2. - centre[0]).abs() < 1e-6
                        && ((p[1] + q[1]) / 2. - centre[1]).abs() < 1e-6
                );
                sides += 1;
            } else if dot([0., 0., 1.], view.toward).abs() > 1. - 1e-6 {
                assert!(v.line_work.iter().any(|w| w.kind == "cross"));
                ends += 1;
            }
        }
        assert!(sides > 0 && ends > 0);
        views.retain(|v| {
            v.id != "v:iso" && View::new(v.direction, v.up).unwrap().toward[2].abs() > 1. - 1e-6
        });
        a.envelope[1][2] = 50.;
        a.features[0]
            .parameters
            .insert("depth".into(), serde_json::json!(40.));
        a.features[0]
            .parameters
            .insert("location".into(), serde_json::json!([50., 50., 40.]));
        a.features[0].fingerprint.position[2] = 20.;
        views[0].id = "v:section".into();
        views[0].cut = Some(crate::model::Cut {
            label: "test".into(),
            origin: [50., 50., 27.],
            normal: [0., 0., 1.],
            marked_in: "not_shown".into(),
        });
        plan(&a, &mut views);
        assert!(views[0].line_work.iter().any(|w| w.kind == "cross"));
        views[0].cut.as_mut().unwrap().origin[2] = 45.;
        plan(&a, &mut views);
        assert!(views[0].line_work.is_empty());
    }
    #[test]
    fn only_a_global_axis_has_a_transverse_tip_exemption() {
        let axis = [[0., -10.], [0., 10.]];
        assert!(!leader_conflict([[-1., 0.], [5., 5.]], axis, true, true));
        assert!(leader_conflict([[-1., 0.], [5., 5.]], axis, false, true));
        assert!(leader_conflict([[-1., 0.], [5., 5.]], axis, true, false));
        assert!(leader_conflict([[0., 0.], [0., 9.]], axis, true, true));
        assert!(leader_conflict([[-5., 0.], [5., 5.]], axis, true, true));
    }
    #[test]
    fn circle_evidence_and_extents_do_not_depend_on_one_part() {
        let points: Vec<_> = (0..8)
            .map(|i| {
                let t = i as f64 * std::f64::consts::TAU / 8.;
                [13. + 20. * t.cos(), -5. + 20. * t.sin()]
            })
            .collect();
        let (c, r) = fitted_circle(&points, 1.).unwrap();
        assert!((c[0] - 13.).abs() < 1e-9 && (c[1] + 5.).abs() < 1e-9 && (r - 20.).abs() < 1e-9);
        let mut wrong = points;
        wrong[0][0] += 1.;
        assert!(fitted_circle(&wrong, 1.).is_none());
        assert!(fitted_circle(&[[0., 0.], [1., 0.], [2., 0.]], 1.).is_none());
        assert_eq!(overrun(0.1), 1.);
        assert_eq!(overrun(10.), 3.);
        assert_eq!(overrun(100.), 5.);

        // A rectangle fits a circle too; a provisional small record needs round corroboration.
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/plate.step");
        let (_, mut a) = crate::model::analyse_native(&fixture).unwrap();
        let mut views = crate::model::plan(&a, &crate::model::Spec::default())
            .unwrap()
            .views;
        let holes: Vec<_> = holes::read(&a)
            .into_iter()
            .map(|(_, h)| h.unwrap())
            .collect();
        assert_eq!(holes.len(), 4);
        let points: Vec<_> = holes.iter().map(|h| [h.at[0], h.at[1]]).collect();
        let (c, r) = fitted_circle(&points, 1.).unwrap();
        let p = crate::bridge::HolePattern {
            id: "f:hole_pattern:unproven".into(),
            arrangement: "bolt_circle".into(),
            members: holes.iter().map(|h| h.feature.id.clone()).collect(),
            faces: Vec::new(),
            parameters: serde_json::json!({"center":[c[0],c[1],holes[0].at[2]],"diameter":2.*r})
                .as_object()
                .unwrap()
                .clone(),
        };
        a.recognition.as_mut().unwrap().patterns = vec![p];
        plan(&a, &mut views);
        assert!(
            views
                .iter()
                .all(|v| v.line_work.iter().all(|w| w.kind != "pitch_circle"))
        );
    }
}
