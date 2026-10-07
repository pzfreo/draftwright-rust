//! A section view's furniture, as draftwright draws it (`annotations/sections.py`): the cut face
//! hatched at 45° by the even-odd rule, and the cutting-plane line on the view that marks it, a
//! chain line with arrowed wing stubs at its ends pointing the way the section is seen.
//!
//! The cut face's boundary comes from haecceity's `hlr::section`, already in page coordinates.

use crate::output::{Curve, Item};

/// Text height of draftwright's annotations, page mm.
pub const FONT_SIZE: f64 = 3.0;
/// Hatch line spacing: one and a half text heights.
pub const HATCH_SPACING: f64 = 1.5 * FONT_SIZE;
const ARROW: f64 = 2.7;
const LINE_WIDTH: f64 = 0.1;
const CENTRELINE_WIDTH: f64 = 0.15;
/// How far the cutting-plane line runs past the part, page mm.
pub const OVERRUN: f64 = 4.0;

type P = [f64; 2];

/// The 45° hatch of the region the closed *loops* bound (page mm): the lines `y − x = c` a
/// *spacing* apart, each cut where it crosses the boundary (draftwright's
/// `_section_hatch_edges`). A span of 0.2 mm or less is left out. A crossing through a vertex is
/// counted once, whichever way the loops run.
///
/// With no *inside* test, a line is drawn between alternate crossings: draftwright's even-odd
/// rule, right when the loops are exactly the cut face's boundary. With one, each stretch between
/// successive crossings is drawn when its middle is inside: right also when the boundary carries
/// a stray or doubled piece (a face lying in the cutting plane), which flips the even-odd parity.
pub fn hatch(loops: &[Vec<P>], spacing: f64, inside: Option<&dyn Fn(P) -> bool>) -> Vec<[P; 2]> {
    let segs: Vec<(P, P)> = loops
        .iter()
        .flat_map(|l| l.windows(2).map(|w| (w[0], w[1])))
        .collect();
    if segs.is_empty() {
        return Vec::new();
    }
    let pts = || segs.iter().flat_map(|s| [s.0, s.1]);
    let (min_x, max_x) = pts().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p[0]), a.1.max(p[0]))
    });
    let (min_y, max_y) = pts().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p[1]), a.1.max(p[1]))
    });
    let c_min = min_y - max_x - spacing;
    let c_max = max_y - min_x + spacing;
    let mut out = Vec::new();
    let mut k = 1.0;
    loop {
        let c = c_min + k * spacing;
        if c >= c_max {
            break;
        }
        k += 1.0;
        let mut hits: Vec<f64> = segs
            .iter()
            .filter_map(|&(a, b)| {
                let (ua, ub) = (a[1] - a[0], b[1] - b[0]);
                ((ua <= c) != (ub <= c)).then(|| {
                    let t = (c - ua) / (ub - ua);
                    a[0] + t * (b[0] - a[0])
                })
            })
            .collect();
        hits.sort_by(f64::total_cmp);
        match inside {
            None => {
                for pair in hits.as_chunks::<2>().0 {
                    let (xa, xb) = (pair[0], pair[1]);
                    if xb - xa > 0.2 {
                        out.push([[xa, xa + c], [xb, xb + c]]);
                    }
                }
            }
            Some(inside) => {
                for pair in hits.windows(2) {
                    let (xa, xb) = (pair[0], pair[1]);
                    let xm = 0.5 * (xa + xb);
                    if xb - xa > 0.2 && inside([xm, xm + c]) {
                        out.push([[xa, xa + c], [xb, xb + c]]);
                    }
                }
            }
        }
    }
    out
}

/// A chain line's dashes along a run of *length* (build123d-drafting's `_dash_spans`): a short
/// dash centred on the middle, long dashes outward, the outermost run out to the end; a run too
/// short for the pattern is solid. Offsets from the middle.
pub fn chain_spans(length: f64) -> Vec<(f64, f64)> {
    let (long, gap, short) = (3.0 * FONT_SIZE, 0.4 * FONT_SIZE, 0.6 * FONT_SIZE);
    let half = length / 2.0;
    if half <= short / 2.0 + gap + long * 0.5 {
        return vec![(-half, half)];
    }
    let mut dashes: Vec<(f64, f64)> = Vec::new();
    let mut pos = short / 2.0;
    let seq = [(gap, false), (long, true), (gap, false), (short, true)];
    let mut i = 0;
    while pos < half - 1e-9 {
        let (len, dash) = seq[i % 4];
        let end = pos + len;
        if dash {
            dashes.push((pos, end.min(half)));
        }
        pos = end;
        i += 1;
    }
    if let Some(last) = dashes.last_mut()
        && last.1 < half - 1e-9
    {
        last.1 = half;
    }
    let mut spans = vec![(-short / 2.0, short / 2.0)];
    for (a, b) in dashes {
        spans.push((a, b));
        spans.push((-b, -a));
    }
    spans
}

/// A stroke from *a* to *b*, *width* wide, as a filled outline (draftwright draws furniture
/// lines as thin faces).
fn stroke(a: P, b: P, width: f64) -> Item {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let n = dx.hypot(dy).max(1e-12);
    let (ox, oy) = (-dy / n * width / 2.0, dx / n * width / 2.0);
    let c = [
        [a[0] + ox, a[1] + oy],
        [b[0] + ox, b[1] + oy],
        [b[0] - ox, b[1] - oy],
        [a[0] - ox, a[1] - oy],
    ];
    Item::Face(vec![
        (0..4).map(|i| Curve::Line(c[i], c[(i + 1) % 4])).collect(),
    ])
}

/// The cutting-plane line from `x0` to `x1` at height *y* (page mm) on the view that marks it:
/// a chain line, and at each end a wing stub from 2.5 arrow lengths below the line with a solid
/// arrowhead whose tip is on it, pointing up the page (towards the material kept). Returns the
/// ink and where the two letters go.
pub fn cutting_plane(x0: f64, x1: f64, y: f64) -> (Vec<Item>, [P; 2]) {
    let mid = 0.5 * (x0 + x1);
    let mut items: Vec<Item> = chain_spans(x1 - x0)
        .into_iter()
        .map(|(a, b)| stroke([mid + a, y], [mid + b, y], CENTRELINE_WIDTH))
        .collect();
    for x in [x0, x1] {
        let tail = y - 2.5 * ARROW;
        items.push(stroke([x, tail], [x, y - ARROW], LINE_WIDTH));
        let head = [
            [x, y],
            [x - ARROW / 3.0, y - ARROW],
            [x + ARROW / 3.0, y - ARROW],
        ];
        items.push(Item::Face(vec![
            (0..3)
                .map(|i| Curve::Line(head[i], head[(i + 1) % 3]))
                .collect(),
        ]));
    }
    let lift = 1.4 * FONT_SIZE;
    (items, [[x0 - 3.0, y + lift], [x1 + 3.0, y + lift]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hatch of a square with a square hole stays in the ring: no hatch line has a point
    /// inside the hole, and every line runs between the boundaries at 45°.
    #[test]
    fn hatch_fills_by_the_even_odd_rule() {
        let square = |c: f64, r: f64| {
            vec![
                [c - r, c - r],
                [c + r, c - r],
                [c + r, c + r],
                [c - r, c + r],
                [c - r, c - r],
            ]
        };
        // The hole runs the other way round: the rule does not care.
        let mut hole = square(10.0, 3.0);
        hole.reverse();
        let loops = [square(10.0, 10.0), hole];
        let lines = hatch(&loops, HATCH_SPACING, None);
        assert!(lines.len() > 5);
        for [a, b] in &lines {
            assert!(((b[1] - a[1]) - (b[0] - a[0])).abs() < 1e-9, "45°");
            for t in [0.25, 0.5, 0.75] {
                let p = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                let in_hole = (p[0] - 10.0).abs() < 3.0 - 1e-6 && (p[1] - 10.0).abs() < 3.0 - 1e-6;
                assert!(!in_hole, "{p:?} is in the hole");
                assert!(p.iter().all(|v| (-1e-9..=20.0 + 1e-9).contains(v)));
            }
        }
        // Lines are a spacing apart, measured along x at fixed y (√2 · spacing / √2 per step).
        let mut cs: Vec<f64> = lines.iter().map(|[a, _]| a[1] - a[0]).collect();
        cs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for w in cs.windows(2) {
            assert!((w[1] - w[0] - HATCH_SPACING).abs() < 1e-9);
        }
        // The same ring with a stray chord across the hole: even-odd hatches the hole's half,
        // the inside test does not, and agrees with even-odd on the clean boundary.
        let ring = |p: P| {
            let (dx, dy) = ((p[0] - 10.0).abs(), (p[1] - 10.0).abs());
            dx < 10.0 && dy < 10.0 && !(dx < 3.0 && dy < 3.0)
        };
        assert_eq!(hatch(&loops, HATCH_SPACING, Some(&ring)), lines);
        let mut stray = loops.to_vec();
        stray.push(vec![[7.0, 10.0], [13.0, 10.0]]);
        for [a, b] in hatch(&stray, HATCH_SPACING, Some(&ring)) {
            let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
            assert!(ring(m), "{m:?}");
        }
    }

    #[test]
    fn chain_line_covers_its_run_symmetrically() {
        let spans = chain_spans(40.0);
        let lo = spans.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
        let hi = spans.iter().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
        assert!((lo + 20.0).abs() < 1e-9 && (hi - 20.0).abs() < 1e-9);
        assert_eq!(chain_spans(5.0), vec![(-2.5, 2.5)]);
    }
}
