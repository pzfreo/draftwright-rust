//! Independent sampled-ink oracle: provenance alone never excuses a crossing.
use draftwright::dims::PlannedDimension;
use draftwright::model::{Analysis, PlannedView};
use draftwright::output::Curve;
type P = [f64; 2];

fn distance(p: P, [a, b]: [P; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let n = d[0] * d[0] + d[1] * d[1];
    let t = if n == 0.0 {
        0.0
    } else {
        ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / n
    }
    .clamp(0.0, 1.0);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

fn meet(s: [P; 2], t: [P; 2]) -> bool {
    let cross = |a: P, b: P, c: P| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let a = cross(s[0], s[1], t[0]);
    let b = cross(s[0], s[1], t[1]);
    let c = cross(t[0], t[1], s[0]);
    let d = cross(t[0], t[1], s[1]);
    (a * b < 0.0 && c * d < 0.0)
        || s.iter().any(|p| distance(*p, t) < 0.1)
        || t.iter().any(|p| distance(*p, s) < 0.1)
}

pub fn assert_clear(a: &Analysis, v: &PlannedView, d: &PlannedDimension) {
    let p = d.placed.as_ref().unwrap();
    let ink = &a.projected_views[&v.id];
    let view = draftwright::haecceity::hlr::View::new(v.direction, v.up).unwrap();
    let centre = view.map(std::array::from_fn(|i| {
        (a.envelope[0][i] + a.envelope[1][i]) / 2.0
    }));
    let page = |q: P| {
        [
            v.at[0] + v.scale * (q[0] - centre[0]),
            v.at[1] + v.scale * (q[1] - centre[1]),
        ]
    };
    let segments = |c: &Curve| {
        if let Curve::Line(a, b) = c {
            return vec![[page(*a), page(*b)]];
        }
        let b = c.bbox();
        let n = ((b[2] - b[0]).hypot(b[3] - b[1]) * v.scale * 40.0)
            .ceil()
            .max(32.0) as usize;
        (0..n)
            .map(|i| {
                [
                    page(c.at(i as f64 / n as f64)),
                    page(c.at((i + 1) as f64 / n as f64)),
                ]
            })
            .collect()
    };
    let edges: Vec<_> = ink
        .visible
        .iter()
        .chain(&ink.hidden)
        .flat_map(segments)
        .collect();
    if d.kind == "callout" {
        assert!(
            ink.visible
                .iter()
                .flat_map(segments)
                .any(|s| distance(p.lines[0][0], s) < 0.08),
            "{} tip misses visible edge",
            d.id
        );
    }
    let mut strokes: Vec<_> = p
        .lines
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, d.kind == "callout" && i == 0))
        .collect();
    for [tip, tail] in &p.arrows {
        let delta = [tail[0] - tip[0], tail[1] - tip[1]];
        let length = delta[0].hypot(delta[1]).max(1e-12);
        let k = draftwright::dims::ARROW / 6.0 / length;
        let a = [tail[0] - k * delta[1], tail[1] + k * delta[0]];
        let b = [tail[0] + k * delta[1], tail[1] - k * delta[0]];
        strokes.extend([
            ([*tip, *tail], d.kind == "callout"),
            ([*tip, a], d.kind == "callout"),
            ([*tip, b], d.kind == "callout"),
            ([a, b], false),
        ]);
    }
    for (i, (s, attachment)) in strokes.iter().enumerate() {
        let mut s = *s;
        if *attachment {
            let n = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
            let t = (0.6 / n.max(1e-12)).min(1.0);
            s[0] = [
                s[0][0] + t * (s[1][0] - s[0][0]),
                s[0][1] + t * (s[1][1] - s[0][1]),
            ];
        }
        assert!(
            !edges.iter().any(|e| meet(s, *e)),
            "{} line/arrow {i} crosses projected ink",
            d.id
        );
    }
}
