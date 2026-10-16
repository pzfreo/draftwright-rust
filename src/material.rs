//! One material re-entry predicate for the leader router and lint (ADR2-8, ISSUE-798-b/d,
//! draftwright's `material_intervals` and `material_reentry_span`): the page length of a leader
//! shaft inside the part's material after it has left the material at its tip. A shaft from a
//! feature's rim out through the body to clear page makes one traversal, the legitimate exit
//! every callout makes; going back in (the body again, or a second body) is the cut, measured
//! as a magnitude, never a flag. `dims::leaders` prices it and reports it
//! (`annotation_line_over_view`); lint's `leader_crosses_silhouette` reports it on the drawn
//! sheet; both build the material with [`solids`] and ask [`reentry`], so they cannot reach
//! opposite verdicts on one shaft.
//!
//! The material is each solid's box as projected (the bounds of its principal profile, or the
//! part's envelope when the Analysis has none): the plan is made from the Analysis (ADR R1),
//! which keeps no filled projected face, so a concave silhouette's notch reads as material. A
//! through hole is inside its solid's box, so a shaft leaving a bore is still one traversal.
//! Unlike draftwright's, a shaft whose tip is already clear of the material is charged for its
//! first traversal too: the router's tips lie on the part, and one that does not is cutting in.

use crate::model::Analysis;

/// A page point, mm.
pub type P = [f64; 2];

/// Material this long or shorter is not a traversal, page mm: a shaft grazing a box's edge, or
/// a tip a hair outside it (as the router's own-view test took it before).
pub const FLOOR: f64 = 0.05;

/// The material of *analysis* on the page, as *page* projects the part's frame: each solid's box.
pub fn solids(analysis: &Analysis, page: impl Fn([f64; 3]) -> P) -> Vec<[f64; 4]> {
    let corners = |b: [[f64; 3]; 2]| (0..8).map(move |i| [0, 1, 2].map(|k| b[(i >> k) & 1][k]));
    let project = |b: [[f64; 3]; 2]| {
        corners(b).map(&page).fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |a, p| {
                [
                    a[0].min(p[0]),
                    a[1].min(p[1]),
                    a[2].max(p[0]),
                    a[3].max(p[1]),
                ]
            },
        )
    };
    if analysis.principal_profiles.is_empty() {
        vec![project(analysis.envelope)]
    } else {
        analysis
            .principal_profiles
            .iter()
            .map(|s| project(s.bounds))
            .collect()
    }
}

/// The parts of segment *s* inside the open box *b*, as fractions of its length.
fn clip(s: [P; 2], b: [f64; 4]) -> Option<(f64, f64)> {
    let d = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        if d[k].abs() < 1e-12 {
            if s[0][k] <= b[k] || s[0][k] >= b[k + 2] {
                return None;
            }
        } else {
            let a = (b[k] - s[0][k]) / d[k];
            let c = (b[k + 2] - s[0][k]) / d[k];
            lo = lo.max(a.min(c));
            hi = hi.min(a.max(c));
        }
    }
    (hi > lo).then_some((lo, hi))
}

/// The traversals of segment *s* through *material*, in page mm from its first point, in order
/// and disjoint: overlapping boxes are one region, and a gap of [`FLOOR`] or less between
/// two is bridged (one traversal, not two).
pub fn intervals(material: &[[f64; 4]], s: [P; 2]) -> Vec<(f64, f64)> {
    let length = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
    if length <= 1e-12 {
        return Vec::new();
    }
    let mut raw: Vec<(f64, f64)> = material
        .iter()
        .filter_map(|b| clip(s, *b))
        .map(|(a, b)| (a * length, b * length))
        .collect();
    raw.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (a, b) in raw {
        match out.last_mut() {
            Some(last) if a <= last.1 + FLOOR => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// The page length of shaft *s* (tip first) inside *material* after its exit at the tip: every
/// traversal but a first one that starts at the tip (within [`FLOOR`]). Zero for a clear route
/// or a lone exit; a re-entry shorter than [`FLOOR`] is not counted.
pub fn reentry(material: &[[f64; 4]], s: [P; 2]) -> f64 {
    let spans = intervals(material, s);
    let skip = usize::from(spans.first().is_some_and(|(a, _)| *a <= FLOOR));
    let cut: f64 = spans[skip..].iter().map(|(a, b)| b - a).sum();
    if cut > FLOOR { cut } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: [f64; 4] = [0.0, 0.0, 10.0, 10.0];

    /// ISSUE-798-b: a leader leaving its own body is not a defect, whatever it crossed inside it.
    #[test]
    fn a_leader_exiting_its_own_body_is_not_a_reentry() {
        assert_eq!(reentry(&[BODY], [[5.0, 5.0], [25.0, 5.0]]), 0.0);
        // A tip on a bore's rim inside the body: the bore is inside the solid's box.
        assert_eq!(reentry(&[BODY], [[3.0, 5.0], [-20.0, 5.0]]), 0.0);
        // A clear route.
        assert_eq!(reentry(&[BODY], [[15.0, 15.0], [25.0, 25.0]]), 0.0);
    }

    /// ISSUE-798-b: cutting a second body is the defect, measured over only the part traversed.
    #[test]
    fn cutting_a_second_body_is_measured() {
        let two = [[0.0, 0.0, 5.0, 10.0], [15.0, 0.0, 20.0, 10.0]];
        assert!((reentry(&two, [[2.0, 5.0], [25.0, 5.0]]) - 5.0).abs() < 1e-9);
        // The body again: a tip clear of it, the shaft back across the box; a tip on its edge
        // and a shaft across it is the exit.
        assert_eq!(reentry(&[BODY], [[10.0, 5.0], [-5.0, 5.0]]), 0.0);
        assert!((reentry(&[BODY], [[11.0, 5.0], [-5.0, 5.0]]) - 10.0).abs() < 1e-9);
        // A 0.02 mm gap between two boxes is one traversal, a 10 mm one is two.
        let near = [[0.0, 0.0, 10.0, 10.0], [10.02, 0.0, 20.0, 10.0]];
        assert_eq!(reentry(&near, [[5.0, 5.0], [25.0, 5.0]]), 0.0);
    }

    /// ISSUE-798-b: the defect is a magnitude, so two imperfect routes can be ranked.
    #[test]
    fn the_reentry_is_a_magnitude_not_a_flag() {
        let slight = reentry(
            &[[0.0, 0.0, 5.0, 10.0], [15.0, 0.0, 17.0, 10.0]],
            [[2.0, 5.0], [25.0, 5.0]],
        );
        let severe = reentry(
            &[[0.0, 0.0, 5.0, 10.0], [15.0, 0.0, 30.0, 10.0]],
            [[2.0, 5.0], [40.0, 5.0]],
        );
        assert!(0.0 < slight && slight < severe, "{slight} {severe}");
        // Overlapping boxes are unioned, not summed.
        let doubled = reentry(&[[15.0, 0.0, 20.0, 10.0]; 2], [[2.0, 5.0], [25.0, 5.0]]);
        assert!((doubled - 5.0).abs() < 1e-9);
    }
}
