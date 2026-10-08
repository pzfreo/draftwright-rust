//! Lone blind pocket coverage (Python's `linting/pocket_coverage.py`): a blind pocket states its
//! width, length and depth, and its location in both directions across it
//! (LINT-pocket_coverage-size_depth_location, LINT-code-pocket_requirement_state).
//!
//! Pockets reach the Analysis as `section_recess` features; those that
//! [`section_recesses::grammar`] reads as a pocket (rectangular, edge-open polygonal, obround or
//! rounded rectangle) are read here in the part's frame ([`placed`]): its depth axis, the longer
//! and shorter sides of its section (the later axis winning a tie, as draftwright's
//! `_pocket_from_bounds` does in the part's frame), and its box. Each has, judged by
//! [`parameters::coverage`]:
//!
//! - `pocket_width.length`, `pocket_length.length`, and `pocket_depth.length`, or
//!   `pocket_max_depth.length` for a pocket whose mouth is a cylinder;
//! - unless it opens at the part's edge (its location is then fixed by the edge, inapplicable in
//!   Python), its location from draftwright's common datum, the corner of the part's box
//!   (`datum_xy`, the Analysis's envelope minimum), to its reference point: its centre, or along
//!   its length its near end when that end is on the datum (`pocket_location_reference`). A pocket
//!   cut along z is located by `location_pocket.location.x` and `.y`; any other by
//!   `location_pocket.<long axis>` and `location_pocket.<width axis>`. A direction in which the
//!   reference is on the datum (within 1e-6) needs no location: draftwright's compiler omits it
//!   as `pocket_location_coincident_with_datum`, inapplicable in its lint.
//!
//! Python follows a Z pocket's two locations through one structured fact that covers both
//! (`location_pocket.location`); here each is a requirement its claiming annotation must print.
//! Not ported: a pocket that is a member of a recognised pocket pattern belongs to the pattern,
//! not to this family; the bridge drops quiddity's derived `section_recess_patterns`, so
//! membership cannot be read, and every pocket is judged as a lone one (no corpus recognition has
//! a recess pattern). Not reached: `_suppressed` and `dropped` (lint cannot read the author's
//! omissions or the placement's drops) and the assembly severity.

use super::parameters::{self, Wanted, fmt, fmt3};
use super::section_recesses::{Grammar, grammar, recesses};
use super::{LintIssue, Mark};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, principal, to_part};
use crate::model::Analysis;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A pocket in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// The axis it is cut along, and the axes of its section's longer and shorter sides.
    pub depth_axis: usize,
    pub long_axis: usize,
    pub width_axis: usize,
    pub centre: [f64; 3],
    pub width: f64,
    pub length: f64,
    pub depth: f64,
    /// Its extent along `long_axis`, and its centre across `width_axis`.
    pub lo: f64,
    pub hi: f64,
    pub w_center: f64,
    pub edge_anchored: bool,
    /// Its mouth is a cylinder, so its depth is its greatest.
    pub mouth: bool,
}

/// The Analysis's pockets in quiddity's order, each read in the part's frame or why it cannot be.
pub fn placed(analysis: &Analysis) -> Vec<(&Feature, Result<Placed, String>)> {
    let frame = frame(analysis);
    recesses(analysis)
        .into_iter()
        .filter_map(|f| match grammar(f) {
            Ok(Grammar::Pocket(p)) => Some((f, p)),
            _ => None,
        })
        .map(|(f, p)| {
            // The pocket's box in quiddity's frame, carried corner to corner into the part's.
            let mut lo = [0.0; 3];
            let mut hi = [0.0; 3];
            lo[p.axis] = p.origin[p.axis] - p.depth / 2.0;
            hi[p.axis] = p.origin[p.axis] + p.depth / 2.0;
            lo[p.long_axis] = p.lo;
            hi[p.long_axis] = p.hi;
            lo[p.width_axis] = p.w_center - p.width / 2.0;
            hi[p.width_axis] = p.w_center + p.width / 2.0;
            let world = |k: usize| {
                let mut e = [0.0; 3];
                e[k] = 1.0;
                principal(dir_to_part(&frame, e))
            };
            let Some(depth_axis) = world(p.axis) else {
                return (
                    f,
                    Err("its depth lies along no axis of the part".to_string()),
                );
            };
            if world(p.long_axis).is_none() || world(p.width_axis).is_none() {
                return (
                    f,
                    Err("its section lies along no axes of the part".to_string()),
                );
            }
            let (a, b) = (to_part(&frame, lo), to_part(&frame, hi));
            let bounds = [0, 1, 2].map(|k| [a[k].min(b[k]), a[k].max(b[k])]);
            let extent = |k: usize| bounds[k][1] - bounds[k][0];
            let across: Vec<usize> = (0..3).filter(|&k| k != depth_axis).collect();
            let (u, v) = (across[0], across[1]);
            let long_axis = if (extent(v), v) > (extent(u), u) {
                v
            } else {
                u
            };
            let width_axis = if long_axis == u { v } else { u };
            let centre = to_part(&frame, p.origin);
            (
                f,
                Ok(Placed {
                    depth_axis,
                    long_axis,
                    width_axis,
                    centre,
                    width: extent(width_axis),
                    length: extent(long_axis),
                    depth: p.depth,
                    lo: bounds[long_axis][0],
                    hi: bounds[long_axis][1],
                    w_center: centre[width_axis],
                    edge_anchored: p.edge_anchored,
                    mouth: p.mouth.is_some(),
                }),
            )
        })
        .collect()
}

/// A pocket's locations from the datum *datum* (the part's box corner): each direction's
/// parameter and distance, leaving out a direction in which it is on the datum.
pub fn locations(p: &Placed, datum: [f64; 3]) -> Vec<(String, f64)> {
    let mut reference = p.centre;
    reference[p.long_axis] = if (p.lo - datum[p.long_axis]).abs() <= 1e-6 {
        p.lo
    } else {
        (p.lo + p.hi) / 2.0
    };
    reference[p.width_axis] = p.w_center;
    let axes: Vec<(String, usize)> = if p.depth_axis == 2 {
        vec![
            ("location_pocket.location.x".into(), 0),
            ("location_pocket.location.y".into(), 1),
        ]
    } else {
        [p.long_axis, p.width_axis]
            .iter()
            .map(|&k| (format!("location_pocket.{}", AXES[k]), k))
            .collect()
    };
    axes.into_iter()
        .map(|(parameter, k)| (parameter, (reference[k] - datum[k]).abs()))
        .filter(|(_, v)| *v > 1e-6)
        .collect()
}

/// The lone pockets' requirements.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let mut out = Vec::new();
    for (f, p) in placed(analysis) {
        let p = match p {
            Ok(p) => p,
            Err(why) => {
                out.push(Wanted {
                    feature: f,
                    parameter: "pocket.?".into(),
                    values: Err(why),
                    callout: "its sizes and location".into(),
                });
                continue;
            }
        };
        let depth = if p.mouth {
            "pocket_max_depth.length"
        } else {
            "pocket_depth.length"
        };
        for (parameter, v, word) in [
            ("pocket_width.length", p.width, "wide"),
            ("pocket_length.length", p.length, "long"),
            (depth, p.depth, "deep"),
        ] {
            out.push(Wanted {
                feature: f,
                parameter: parameter.into(),
                values: Ok(vec![v]),
                callout: format!("{} {word}", fmt(v)),
            });
        }
        if p.edge_anchored {
            continue;
        }
        for (parameter, v) in locations(&p, analysis.envelope[0]) {
            let axis = parameter.rsplit('.').next().unwrap_or("?").to_string();
            out.push(Wanted {
                feature: f,
                parameter,
                values: Ok(vec![v]),
                callout: format!(
                    "its location {} along {axis} from the part's corner",
                    fmt(v)
                ),
            });
        }
    }
    out
}

/// The lone pocket findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let read = placed(analysis);
    let noun = |f: &Feature| {
        let mut s = format!("blind pocket {}", f.id);
        if let Some((_, Ok(p))) = read.iter().find(|(g, _)| g.id == f.id) {
            s += &format!(
                " {} × {} × {} deep along {}, at {}",
                fmt(p.width),
                fmt(p.length),
                fmt(p.depth),
                AXES[p.depth_axis],
                fmt3(p.centre)
            );
        }
        s
    };
    parameters::coverage(
        "pocket",
        &wanted(analysis),
        marks,
        noun,
        "the pocket's callout in the view that looks into it, and linear dimensions of its \
         location from the part's corner",
    )
}

#[cfg(test)]
mod tests {
    use super::super::paired_ramp_steps::tests::part;
    use super::super::section_recesses::tests::{Z, recess};
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    fn mark(text: &str, parameters: &[&str]) -> Mark {
        Mark {
            name: format!("m:{text}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: parameters
                .iter()
                .map(|p| Claim {
                    feature: "f:section_recess:1".into(),
                    parameter: p.to_string(),
                })
                .collect(),
        }
    }

    /// m1000's pocket, 2 × 2.81 × 3.445 deep along z, centred at (39.336, 10.187) on a part whose
    /// box starts at the origin: Python's sheet states `2 × 2.8 × 3.4 DEEP`, `39.3` and `10.2`
    /// (test_pocket_completeness_evidence's directional locations); a location left off is
    /// missing, a misprinted one unverifiable.
    #[test]
    fn a_pocket_states_its_sizes_and_location() {
        let rect = [
            ([-1.0, -1.405], 0.0),
            ([1.0, -1.405], 0.0),
            ([1.0, 1.405], 0.0),
            ([-1.0, 1.405], 0.0),
        ];
        let frame = [[39.336, 10.187, 0.0], Z[1], Z[2], Z[3]];
        let a = part(
            "section_recess",
            recess(
                "pocket",
                "rectangular",
                frame,
                [0.0, 3.445],
                [false, true],
                true,
                &rect,
            ),
        );
        let got: Vec<(String, String)> = wanted(&a)
            .into_iter()
            .map(|w| (w.parameter, fmt(w.values.unwrap()[0])))
            .collect();
        let want: Vec<(String, String)> = [
            ("pocket_width.length", "2"),
            ("pocket_length.length", "2.81"),
            ("pocket_depth.length", "3.445"),
            ("location_pocket.location.x", "39.336"),
            ("location_pocket.location.y", "10.187"),
        ]
        .iter()
        .map(|(p, v)| (p.to_string(), v.to_string()))
        .collect();
        assert_eq!(got, want);
        let sizes = [
            "pocket_width.length",
            "pocket_length.length",
            "pocket_depth.length",
        ];
        let both = ["location_pocket.location.x", "location_pocket.location.y"];
        let callout = mark("2 × 2.8 × 3.4 DEEP", &sizes);
        let x = mark("39.3", &both);
        let y = mark("10.2", &both);
        assert!(coverage(&a, &[callout.clone(), x.clone(), y]).is_empty());
        let codes = |marks: &[Mark]| -> Vec<String> {
            coverage(&a, marks)
                .into_iter()
                .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
                .collect()
        };
        assert_eq!(
            codes(&[callout.clone(), mark("39.3", &both[..1])]),
            ["pocket_requirement_missing location_pocket.location.y"]
        );
        assert_eq!(
            codes(&[callout, x, mark("10.4", &both)]),
            ["pocket_requirement_unverifiable location_pocket.location.y"]
        );
    }

    /// A pocket cut along y is located along its long and width axes (mfcadpp 10245's 2 × 2
    /// pocket: `location_pocket.z` 40.57 and `location_pocket.x` 8.392 on Python's sheet); one
    /// whose near end is on the datum is located only across; an edge-open pocket is not located
    /// (test_edge_anchored_pocket_reports_location_as_intentionally_inapplicable).
    #[test]
    fn side_and_edge_pockets_are_located_as_draftwright_does() {
        let y_run = [
            [8.392, 0.0, 40.57],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ];
        let square = [
            ([-1.0, -1.0], 0.0),
            ([1.0, -1.0], 0.0),
            ([1.0, 1.0], 0.0),
            ([-1.0, 1.0], 0.0),
        ];
        let a = part(
            "section_recess",
            recess(
                "pocket",
                "rectangular",
                y_run,
                [0.0, 16.575],
                [false, true],
                true,
                &square,
            ),
        );
        let (_, p) = &placed(&a)[0];
        let p = p.as_ref().unwrap();
        // A square: the later axis (z) is its length, as in draftwright.
        assert_eq!((p.depth_axis, p.long_axis, p.width_axis), (1, 2, 0));
        let named = |l: Vec<(String, f64)>| -> Vec<(String, String)> {
            l.into_iter().map(|(p, v)| (p, fmt(v))).collect()
        };
        assert_eq!(
            named(locations(p, [0.0; 3])),
            [
                ("location_pocket.z".to_string(), "40.57".to_string()),
                ("location_pocket.x".to_string(), "8.392".to_string())
            ]
        );
        assert_eq!(
            named(locations(p, [0.0, 0.0, 39.57])),
            [("location_pocket.x".to_string(), "8.392".to_string())]
        );
        let open = [
            ([-1.0, 1.0], 0.0),
            ([-1.0, -1.0], 0.0),
            ([1.0, -1.0], 0.0),
            ([1.0, 1.0], 0.0),
        ];
        let mut edge = recess(
            "edge_open_recess",
            "polygonal",
            y_run,
            [0.0, 5.0],
            [false, true],
            false,
            &open,
        );
        edge["geometry"]["profile"]["opening"] = json!([[1.0, 1.0], [-1.0, 1.0]]);
        let e = part("section_recess", edge);
        let names: Vec<String> = wanted(&e).into_iter().map(|w| w.parameter).collect();
        assert_eq!(
            names,
            [
                "pocket_width.length",
                "pocket_length.length",
                "pocket_depth.length"
            ]
        );
    }
}
