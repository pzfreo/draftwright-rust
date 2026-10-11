//! Visible edges take precedence over hidden edges in each placed view (ISO 128-2).
//! Work in page mm, preserving the exact curves when a hidden stretch is split.

use crate::output::{Curve, Item, P};

pub const PRECEDENCE: f64 = 0.15;
const CHORD: f64 = 0.001;
const DUPLICATE: f64 = 0.004;

fn visible_tolerance() -> f64 {
    crate::output::layer_styles()[0].1.width / 2.0 + PRECEDENCE + 2.0 * CHORD
}

/// An ellipse thinner than SVG's endpoint rounding can acquire a false return
/// arc on export. Its straight support retains the major-axis extrema, within
/// 1e-6 page mm, without retracing that support twice.
pub fn stable_curve(curve: Curve) -> Curve {
    let Curve::Conic {
        radii,
        x_axis,
        y_axis,
        t0,
        t1,
        ..
    } = &curve
    else {
        return curve;
    };
    if radii[0].min(radii[1]) > 1e-6 {
        return curve;
    }
    let shift = if radii[0] >= radii[1] {
        0.0
    } else {
        std::f64::consts::FRAC_PI_2
    };
    let mut parameters = vec![0.0, 1.0];
    let (lo, hi) = (t0.min(*t1), t0.max(*t1));
    let pi = std::f64::consts::PI;
    for k in ((lo - shift) / pi).ceil() as i64..=((hi - shift) / pi).floor() as i64 {
        let u = (shift + k as f64 * pi - t0) / (t1 - t0);
        if u > 0.0 && u < 1.0 {
            parameters.push(u);
        }
    }
    let axis = if radii[0] >= radii[1] {
        *x_axis
    } else {
        *y_axis
    };
    let mut points: Vec<P> = parameters.into_iter().map(|u| curve.at(u)).collect();
    points.sort_by(|a, b| dot(*a, axis).total_cmp(&dot(*b, axis)));
    let (a, b) = (points[0], *points.last().unwrap());
    if dot(curve.start(), axis) > dot(curve.end(), axis) {
        Curve::Line(b, a)
    } else {
        Curve::Line(a, b)
    }
}

type Segment = (P, P);
fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn distance(a: P, b: P) -> f64 {
    let d = sub(a, b);
    d[0].hypot(d[1])
}

/// Parameterised chords, refined on quarters as well as the midpoint so an inflection
/// cannot masquerade as a straight line. Piece boundaries are always sampled.
fn chords(curve: &Curve) -> Vec<(f64, P)> {
    fn refine(c: &Curve, lo: f64, hi: f64, out: &mut Vec<(f64, P)>, depth: usize) {
        let (a, b) = (c.at(lo), c.at(hi));
        let flat = [0.25, 0.5, 0.75].into_iter().all(|t| {
            distance(
                c.at(lo + t * (hi - lo)),
                [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])],
            ) <= CHORD
        });
        if flat || depth == 24 {
            out.push((hi, b));
        } else {
            let mid = (lo + hi) / 2.0;
            refine(c, lo, mid, out, depth + 1);
            refine(c, mid, hi, out, depth + 1);
        }
    }
    let n = match curve {
        Curve::Conic { t0, t1, .. } => {
            ((t1 - t0).abs() / std::f64::consts::FRAC_PI_2).ceil() as usize
        }
        Curve::Beziers(p) => p.len(),
        Curve::Rational(p) => p.len(),
        Curve::Line(..) => 1,
    }
    .max(1);
    let mut out = vec![(0.0, curve.start())];
    for i in 0..n {
        refine(
            curve,
            i as f64 / n as f64,
            (i + 1) as f64 / n as f64,
            &mut out,
            0,
        );
    }
    out
}

fn merge(mut intervals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (lo, hi) in intervals {
        let (lo, hi) = (lo.max(0.0), hi.min(1.0));
        if hi <= lo {
            continue;
        }
        if let Some(last) = out.last_mut()
            && lo <= last.1 + 1e-10
        {
            last.1 = last.1.max(hi);
        } else {
            out.push((lo, hi));
        }
    }
    out
}

/// The part of chord a–b inside the round-ended tolerance band of c–d.
fn capsule((a, b): Segment, (c, d): Segment, tolerance: f64) -> Vec<(f64, f64)> {
    if (0..2).any(|k| {
        a[k].max(b[k]) < c[k].min(d[k]) - tolerance || a[k].min(b[k]) > c[k].max(d[k]) + tolerance
    }) {
        return Vec::new();
    }
    let v = sub(b, a);
    let w = sub(d, c);
    let ww = dot(w, w);
    let mut out = Vec::new();
    let clip = |value: f64, slope: f64, min: f64, max: f64, range: &mut (f64, f64)| {
        if slope.abs() < 1e-15 {
            if value < min || value > max {
                range.1 = range.0;
            }
        } else {
            let (u, v) = ((min - value) / slope, (max - value) / slope);
            range.0 = range.0.max(u.min(v));
            range.1 = range.1.min(u.max(v));
        }
    };
    if ww > 1e-20 {
        let ac = sub(a, c);
        let mut range = (0.0, 1.0);
        clip(dot(ac, w), dot(v, w), 0.0, ww, &mut range);
        let cross = |p: P| p[0] * w[1] - p[1] * w[0];
        let band = tolerance * ww.sqrt();
        clip(cross(ac), cross(v), -band, band, &mut range);
        if range.1 > range.0 {
            out.push(range);
        }
    }
    let vv = dot(v, v);
    if vv > 1e-20 {
        for end in [c, d] {
            let q = sub(a, end);
            let mid = -dot(q, v) / vv;
            let discriminant = tolerance * tolerance - dot(q, q) + dot(q, v).powi(2) / vv;
            if discriminant >= 0.0 {
                let half = (discriminant / vv).sqrt();
                out.push((mid - half, mid + half));
            }
        }
    }
    merge(out)
}

struct Ink {
    segments: Vec<Segment>,
    grid: std::collections::HashMap<(i64, i64), Vec<usize>>,
}
fn cells((a, b): Segment, tolerance: f64) -> impl Iterator<Item = (i64, i64)> {
    let bounds = [0, 1].map(|k| {
        (
            ((a[k].min(b[k]) - tolerance) / 5.0).floor() as i64,
            ((a[k].max(b[k]) + tolerance) / 5.0).floor() as i64,
        )
    });
    (bounds[0].0..=bounds[0].1).flat_map(move |x| (bounds[1].0..=bounds[1].1).map(move |y| (x, y)))
}
impl Ink {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            grid: std::collections::HashMap::new(),
        }
    }
    fn add(&mut self, curve: &Curve, tolerance: f64) {
        for pair in chords(curve).windows(2) {
            let segment = (pair[0].1, pair[1].1);
            let index = self.segments.len();
            self.segments.push(segment);
            for cell in cells(segment, tolerance) {
                self.grid.entry(cell).or_default().push(index);
            }
        }
    }
    fn covered(&self, curve: &Curve, tolerance: f64) -> Vec<(f64, f64)> {
        let mut intervals = Vec::new();
        for pair in chords(curve).windows(2) {
            let (u, a) = pair[0];
            let (v, b) = pair[1];
            let mut candidates: Vec<usize> = cells((a, b), 0.0)
                .filter_map(|cell| self.grid.get(&cell))
                .flatten()
                .copied()
                .collect();
            candidates.sort_unstable();
            candidates.dedup();
            for i in candidates {
                intervals.extend(
                    capsule((a, b), self.segments[i], tolerance)
                        .into_iter()
                        .map(|(lo, hi)| (u + lo * (v - u), u + hi * (v - u))),
                );
            }
        }
        merge(intervals)
    }
}

fn remaining(curve: &Curve, covered: Vec<(f64, f64)>) -> Vec<Curve> {
    let mut start = 0.0;
    let mut out = Vec::new();
    for (lo, hi) in covered.into_iter().chain([(1.0, 1.0)]) {
        if lo > start + 1e-9 {
            out.push(curve.trim(start, lo));
        }
        start = hi;
    }
    out
}

/// Subtract visible curves from one hidden curve, retaining exact arc and spline pieces.
pub fn subtract(curve: &Curve, visible: &[Curve]) -> Vec<Curve> {
    let mut ink = Ink::new();
    for c in visible {
        ink.add(c, visible_tolerance());
    }
    remaining(curve, ink.covered(curve, visible_tolerance()))
}

/// Apply line precedence and draw overlapping hidden pieces once. Keep long pieces first,
/// then merge collinear fragments so overlapping pieces cannot fill one another's dash gaps.
pub fn hidden(visible: &[Item], hidden: Vec<Item>) -> Vec<Item> {
    let mut ink = Ink::new();
    for item in visible {
        if let Item::Edge(c) = item {
            ink.add(c, visible_tolerance());
        }
    }
    let mut curves: Vec<(f64, Curve)> = hidden
        .into_iter()
        .filter_map(|i| match i {
            Item::Edge(c) => {
                let length = chords(&c)
                    .windows(2)
                    .map(|w| distance(w[0].1, w[1].1))
                    .sum();
                Some((length, c))
            }
            _ => None,
        })
        .collect();
    curves.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut unique = Ink::new();
    let mut out = Vec::new();
    for (_, c) in curves {
        for c in remaining(&c, ink.covered(&c, visible_tolerance())) {
            for piece in remaining(&c, unique.covered(&c, DUPLICATE)) {
                unique.add(&piece, DUPLICATE);
                out.push(piece);
            }
        }
    }
    // A shared support and touching ranges form one dash run. Reversed pieces are equivalent.
    let mut i = 0;
    while i < out.len() {
        let mut j = i + 1;
        while j < out.len() {
            if let (Curve::Line(a, b), Curve::Line(c, d)) = (&out[i], &out[j]) {
                let v = sub(*b, *a);
                let length = distance(*a, *b);
                let along = |p: P| dot(sub(p, *a), v) / length;
                let across = |p: P| {
                    let q = sub(p, *a);
                    (q[0] * v[1] - q[1] * v[0]).abs() / length
                };
                let (lo, hi) = (along(*c).min(along(*d)), along(*c).max(along(*d)));
                if across(*c) < 1e-6
                    && across(*d) < 1e-6
                    && lo <= length + DUPLICATE + 1e-8
                    && hi >= -DUPLICATE - 1e-8
                {
                    let at = |t: f64| [a[0] + v[0] * t / length, a[1] + v[1] * t / length];
                    out[i] = Curve::Line(at(lo.min(0.0)), at(hi.max(length)));
                    out.remove(j);
                    j = i + 1;
                    continue;
                }
            }
            j += 1;
        }
        i += 1;
    }
    out.into_iter().map(Item::Edge).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    #[test]
    fn nearly_edge_on_arcs_cannot_double_back_after_svg_rounding() {
        let curve = Curve::Conic {
            centre: [150., 220.],
            radii: [2., 4e-9],
            x_axis: [1., 0.],
            y_axis: [0., 1.],
            t0: 0.2,
            t1: 1.4,
        };
        let stable = stable_curve(curve.clone());
        assert!(matches!(stable, Curve::Line(..)));
        assert_eq!(stable.start(), curve.start());
        assert_eq!(stable.end(), curve.end());
        let full = stable_curve(Curve::Conic {
            centre: [150., 220.],
            radii: [2., 4e-9],
            x_axis: [1., 0.],
            y_axis: [0., 1.],
            t0: 0.,
            t1: 2. * PI,
        });
        assert!((full.bbox()[0] - 148.).abs() < 1e-9 && (full.bbox()[2] - 152.).abs() < 1e-9);
    }
    #[test]
    fn partial_and_reversed_line_overlap() {
        let h = Curve::Line([0., 0.], [10., 0.]);
        let result = subtract(&h, &[Curve::Line([7., 0.], [3., 0.])]);
        assert_eq!(result.len(), 2);
        assert!(result[0].end()[0] < 3. && result[1].start()[0] > 7.);
        assert!(subtract(&h, &[Curve::Line([10., 0.], [0., 0.])]).is_empty());
    }
    #[test]
    fn arcs_are_split_and_kept_exact() {
        let arc = |t0, t1| Curve::Conic {
            centre: [0., 0.],
            radii: [5., 5.],
            x_axis: [1., 0.],
            y_axis: [0., 1.],
            t0,
            t1,
        };
        let result = subtract(&arc(0., PI), &[arc(PI / 3., 2. * PI / 3.)]);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|c| matches!(c, Curve::Conic { .. })));
        assert!(subtract(&arc(0., PI), &[arc(PI, 0.)]).is_empty());
    }
    #[test]
    fn distinct_edges_survive_and_overlapping_dashes_are_one_run() {
        let line = |a, b, y| Item::Edge(Curve::Line([a, y], [b, y]));
        let result = hidden(
            &[],
            vec![
                line(0., 6., 0.),
                line(10., 4., 0.),
                line(0., 6., 0.),
                line(0., 10., 0.1),
            ],
        );
        assert_eq!(result.len(), 2);
        assert!(result.contains(&line(0., 10., 0.)) || result.contains(&line(10., 0., 0.)));
    }
    #[test]
    fn partial_splines_remain_exact() {
        let curve = Curve::Beziers(vec![vec![[0., 0.], [2., 4.], [8., 4.], [10., 0.]]]);
        let middle = curve.trim(0.25, 0.75);
        let result = subtract(&curve, &[middle]);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|c| matches!(c, Curve::Beziers(_))));
    }
}
