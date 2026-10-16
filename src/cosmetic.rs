//! Conventional representations drawn over a turned step's outline: an external thread per
//! ISO 6410-1 (a thin continuous line at the minor diameter over the thread length) and a
//! knurl per the usual simplified convention (ISO 13444's knurled surface shown by a few thin
//! lines over part of its length: straight along the axis, diamond crossed at 30°). Python
//! draftwright draws neither (its threads are cosmetic and unrecognised); the port draws them
//! for the AP242 threads and knurls [`crate::pmi_lowering::manufacturing`] gives to a turned step
//! (draftwright-rust #23).
//!
//! Only profile views draw them (the step's axis in the view plane), and not section views,
//! whose hatching would need to stop at the minor diameter. The thread's limit is the step's own
//! end: ownership requires the stated thread length to be the step's cylinder or span, so the
//! shoulder edge the view already draws is the limit line, and the step length dimensions it.

use crate::model::{Analysis, PlannedView, extent};
use crate::output::{Curve, P};
use crate::pmi_lowering::{Depiction, DepictionKind, Manufacturing};
use haecceity::hlr::View;

/// The paper spacing of a knurl band's lines, at least this (mm).
const KNURL_GAP: f64 = 3.0;

/// The helix angle a diamond knurl's lines are drawn at, to the axis.
const DIAMOND_ANGLE: f64 = 30.0;

/// The thin lines view *v* draws for the threads and knurls *m* gives to turned steps, in a
/// fixed order (the requirements' order).
pub fn ink(analysis: &Analysis, m: &Manufacturing, v: &PlannedView) -> Vec<Curve> {
    let mut out = Vec::new();
    if v.id == "v:iso" || v.cut.is_some() {
        return out;
    }
    let Some(view) = View::new(v.direction, v.up) else {
        return out;
    };
    let (size, centre) = if v.detail.is_some() {
        crate::detail::crop_extent(analysis, v)
    } else {
        extent(analysis, &view)
    };
    let page = |p: [f64; 3]| -> P {
        let q = view.map(p);
        [0, 1].map(|k| v.at[k] + v.scale * (q[k] - centre[k]))
    };
    for d in m.owned.iter().filter_map(|o| o.depiction.as_ref()) {
        let mut axis = [0.0; 3];
        axis[d.axis] = 1.0;
        if view.toward[d.axis].abs() > 1e-6 {
            continue;
        }
        // The radial direction in the view plane, perpendicular to the axis.
        let t = view.toward;
        let u = [
            axis[1] * t[2] - axis[2] * t[1],
            axis[2] * t[0] - axis[0] * t[2],
            axis[0] * t[1] - axis[1] * t[0],
        ];
        let at = |x: f64, r: f64| -> P {
            let mut p = d.on;
            p[d.axis] = x;
            page([0, 1, 2].map(|k| p[k] + r * u[k]))
        };
        match d.kind {
            DepictionKind::Thread { minor } => {
                let r = minor / 2.0;
                if r <= 0.0 || r >= d.diameter / 2.0 {
                    continue;
                }
                for side in [r, -r] {
                    out.push(Curve::Line(at(d.span[0], side), at(d.span[1], side)));
                }
            }
            DepictionKind::Knurl { diamond, pitch } => {
                out.extend(knurl(d, diamond, pitch * v.scale, v.scale, &at));
            }
        }
    }
    if v.detail.is_some() {
        let rect = [
            v.at[0] - size[0] * v.scale / 2.0,
            v.at[1] - size[1] * v.scale / 2.0,
            v.at[0] + size[0] * v.scale / 2.0,
            v.at[1] + size[1] * v.scale / 2.0,
        ];
        let mut boundary = crate::detail::Boundary::default();
        out = out
            .into_iter()
            .flat_map(|c| {
                let clipped = crate::detail::clip(c.clone(), rect);
                boundary.add(&c, &clipped, rect);
                clipped
            })
            .collect();
        out.extend(boundary.ink(rect));
    }
    out
}

/// A knurl's band over the middle half of its length (all of it when that half is under a gap
/// long, so a narrow knurl is not reduced to specks), across its diameter: lines along the axis
/// (straight) or two families at ±30° (diamond), *gap* apart on paper (its pitch, at least
/// [`KNURL_GAP`]), clipped to the band and kept half a gap inside its outline.
fn knurl(
    d: &Depiction,
    diamond: bool,
    gap: f64,
    scale: f64,
    at: &dyn Fn(f64, f64) -> P,
) -> Vec<Curve> {
    let gap = gap.max(KNURL_GAP);
    let [lo, hi] = d.span;
    let quarter = (hi - lo) / 4.0;
    // The band in paper units: x along the axis from its middle, y across it.
    let half_x = (quarter * scale).max(gap.min(2.0 * quarter * scale));
    let half_y = d.diameter / 2.0 * scale - gap / 2.0;
    if half_x <= 0.0 || half_y <= 0.0 {
        return Vec::new();
    }
    let mid = (lo + hi) / 2.0;
    let map = |[x, y]: P| at(mid + x / scale, y / scale);
    let mut out = Vec::new();
    let directions: Vec<P> = if diamond {
        let a = DIAMOND_ANGLE.to_radians();
        vec![[a.cos(), a.sin()], [a.cos(), -a.sin()]]
    } else {
        vec![[1.0, 0.0]]
    };
    for dir in directions {
        let normal = [-dir[1], dir[0]];
        // Offsets of the family's lines from the middle, half a gap off it (clear of the axis).
        let reach = half_x * normal[0].abs() + half_y * normal[1].abs();
        let n = (reach / gap).floor() as i64;
        for j in -n - 1..=n {
            let offset = (j as f64 + 0.5) * gap;
            let c = [offset * normal[0], offset * normal[1]];
            if let Some([a, b]) = clip(c, dir, half_x, half_y) {
                out.push(Curve::Line(map(a), map(b)));
            }
        }
    }
    out
}

/// The part of the line through *c* along unit *dir* inside the box |x| ≤ *hx*, |y| ≤ *hy*
/// (Liang–Barsky), unless it is shorter than a millimetre.
fn clip(c: P, dir: P, hx: f64, hy: f64) -> Option<[P; 2]> {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    for (p, q, h) in [(c[0], dir[0], hx), (c[1], dir[1], hy)] {
        if q.abs() < 1e-12 {
            if p.abs() > h {
                return None;
            }
            continue;
        }
        let (a, b) = ((-h - p) / q, (h - p) / q);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    (t1 - t0 >= 1.0).then(|| [t0, t1].map(|t| [c[0] + t * dir[0], c[1] + t * dir[1]]))
}
