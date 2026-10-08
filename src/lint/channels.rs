//! Open channel coverage (Python's `linting/channel_coverage.py`): a full-span open channel in a
//! base-and-walls construction is dimensioned by its width, the thickness of the wall below it,
//! and the part's overall extent across it (LINT-channel_coverage-width_scheme,
//! LINT-code-channel_requirement_state).
//!
//! Channels reach the Analysis as `section_recess` features; those that
//! [`section_recesses::grammar`] reads as a rectangular channel are read here in the part's frame.
//! As in Python, the scheme is required only when quiddity's plates lie along at least two axes
//! (`has_multi_axis_plates`: a base and walls); otherwise the same recess is a centred rebate the
//! step ladder owns, and nothing is required. Each channel then has three requirements:
//!
//! - its width, `channel_width.length`, claimed on the channel;
//! - the thickness of the wall below it, `thickness.length`, claimed on the one `plate` feature
//!   along the channel's width axis that runs from the part's box to the channel's near wall
//!   (within 1e-3); with none, or several, the requirement cannot be checked;
//! - the part's overall extent across it, `width.length`, `depth.length` or `height.length`,
//!   claimed on `envelope`.
//!
//! Each is stated when an annotation claims it and prints its value. Python finds the lower wall
//! among draftwright's own plate features (its conversion of quiddity's plates); the port among
//! quiddity's plates in the Analysis. Not reached: `_suppressed` and `dropped` (lint cannot read
//! the author's omissions or the placement's drops) and the assembly severity.

use super::envelope::PARAMETERS;
use super::parameters::{fmt, fmt3};
use super::section_recesses::{Grammar, grammar, recesses};
use super::{LintIssue, Mark, Requirement, Severity, holes};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, principal, to_part};
use crate::model::Analysis;
use serde_json::Value;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A channel in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// The axis it runs along and the axis across it.
    pub run_axis: usize,
    pub width_axis: usize,
    pub centre: [f64; 3],
    pub width: f64,
    /// Its walls' coordinates along `width_axis`.
    pub walls: [f64; 2],
}

/// *k*'s axis of quiddity's frame as the part's axis, if it lies along one.
fn world(frame: &[[f64; 3]; 4], k: usize) -> Option<usize> {
    let mut e = [0.0; 3];
    e[k] = 1.0;
    principal(dir_to_part(frame, e))
}

/// The Analysis's channels in quiddity's order, each read in the part's frame or why not.
pub fn placed(analysis: &Analysis) -> Vec<(&Feature, Result<Placed, String>)> {
    let frame = frame(analysis);
    recesses(analysis)
        .into_iter()
        .filter_map(|f| match grammar(f) {
            Ok(Grammar::Channel(c)) => Some((f, c)),
            _ => None,
        })
        .map(|(f, c)| {
            let (Some(run_axis), Some(width_axis)) =
                (world(&frame, c.axis), world(&frame, c.width_axis))
            else {
                return (f, Err("its axes lie along no axes of the part".to_string()));
            };
            let mut lo = c.origin;
            let mut hi = c.origin;
            lo[c.width_axis] = c.w_center - c.width / 2.0;
            hi[c.width_axis] = c.w_center + c.width / 2.0;
            let (a, b) = (to_part(&frame, lo), to_part(&frame, hi));
            (
                f,
                Ok(Placed {
                    run_axis,
                    width_axis,
                    centre: to_part(&frame, c.origin),
                    width: c.width,
                    walls: [
                        a[width_axis].min(b[width_axis]),
                        a[width_axis].max(b[width_axis]),
                    ],
                }),
            )
        })
        .collect()
}

/// A plate's axis and its interval along it, in the part's frame.
fn plate(frame: &[[f64; 3]; 4], f: &Feature) -> Option<(usize, [f64; 2])> {
    let p = &f.parameters;
    let axis = AXES
        .iter()
        .position(|a| p.get("axis").and_then(Value::as_str) == Some(a))?;
    let get = |k: &str| p.get(k).and_then(Value::as_f64);
    let (lo, hi, u, v) = (get("lo")?, get("hi")?, get("u")?, get("v")?);
    let others: Vec<usize> = (0..3).filter(|&k| k != axis).collect();
    let point = |t: f64| {
        let mut q = [0.0; 3];
        q[axis] = t;
        q[others[0]] = u;
        q[others[1]] = v;
        to_part(frame, q)
    };
    let k = world(frame, axis)?;
    let (a, b) = (point(lo)[k], point(hi)[k]);
    Some((k, [a.min(b), a.max(b)]))
}

/// Whether quiddity's plates prove a base-and-walls construction: they lie along two axes or more.
pub fn multi_axis(analysis: &Analysis) -> bool {
    let mut axes: Vec<&str> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "plate")
        .filter_map(|f| f.parameters.get("axis").and_then(Value::as_str))
        .collect();
    axes.sort_unstable();
    axes.dedup();
    axes.len() >= 2
}

/// One channel requirement: the feature and parameter that state it, and its value, or why it
/// cannot be checked.
pub struct Wanted<'a> {
    pub channel: &'a Feature,
    /// What owns the measurement: the channel, the plate below it, or the `envelope`.
    pub owner: Result<String, String>,
    pub parameter: String,
    pub value: Result<f64, String>,
    /// Python's name for it, `<owner kind>.<parameter>`.
    pub role: String,
}

/// The channels' requirements: none unless the plates lie along two axes or more.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    if !multi_axis(analysis) {
        return Vec::new();
    }
    let frame = frame(analysis);
    let mut out = Vec::new();
    for (f, c) in placed(analysis) {
        let c = match c {
            Ok(c) => c,
            Err(why) => {
                for (role, parameter) in [
                    ("channel", "channel_width.length"),
                    ("plate", "thickness.length"),
                    ("envelope", "?"),
                ] {
                    out.push(Wanted {
                        channel: f,
                        owner: Err(why.clone()),
                        parameter: parameter.into(),
                        value: Err(why.clone()),
                        role: format!("{role}.{parameter}"),
                    });
                }
                continue;
            }
        };
        let k = c.width_axis;
        out.push(Wanted {
            channel: f,
            owner: Ok(f.id.clone()),
            parameter: "channel_width.length".into(),
            value: Ok(c.width),
            role: "channel.channel_width.length".into(),
        });
        let box_lo = analysis.envelope[0][k];
        let below: Vec<(&Feature, f64)> = analysis
            .features
            .iter()
            .filter(|p| p.kind == "plate")
            .filter_map(|p| {
                let (axis, [lo, hi]) = plate(&frame, p)?;
                (axis == k && (lo - box_lo).abs() <= 1e-3 && (hi - c.walls[0]).abs() <= 1e-3)
                    .then_some((p, hi - lo))
            })
            .collect();
        let (owner, value) = match below[..] {
            [(p, t)] => (Ok(p.id.clone()), Ok(t)),
            _ => {
                let why = format!(
                    "{} of quiddity's plates along {} run from the part's box to the channel's \
                     near wall, not one",
                    below.len(),
                    AXES[k]
                );
                (Err(why.clone()), Err(why))
            }
        };
        out.push(Wanted {
            channel: f,
            owner,
            parameter: "thickness.length".into(),
            value,
            role: "plate.thickness.length".into(),
        });
        let extent = analysis.envelope[1][k] - analysis.envelope[0][k];
        out.push(Wanted {
            channel: f,
            owner: Ok("envelope".into()),
            parameter: PARAMETERS[k].into(),
            value: Ok(extent),
            role: format!("envelope.{}", PARAMETERS[k]),
        });
    }
    out
}

/// The channel findings, each `channel_requirement_<state>`.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let read = placed(analysis);
    let mut issues = Vec::new();
    for w in wanted(analysis) {
        let id = &w.channel.id;
        let mut noun = format!("open channel {id}");
        if let Some((_, Ok(c))) = read.iter().find(|(f, _)| f.id == *id) {
            noun += &format!(
                " {} wide, running along {}, at {}",
                fmt(c.width),
                AXES[c.run_axis],
                fmt3(c.centre)
            );
        }
        let claiming: Vec<&Mark> = match &w.owner {
            Ok(owner) => marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| &c.feature == owner && c.parameter == w.parameter)
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        let owner = w.owner.as_ref().unwrap_or(id);
        let issue =
            |state: &str, message: String, evidence: Option<String>, fix: Option<String>| {
                let code = format!("channel_requirement_{state}");
                LintIssue {
                    id: format!("l:{code}:{id}/{}", w.role),
                    code,
                    severity: Severity::Warning,
                    message,
                    at: None,
                    view: claiming.first().and_then(|m| m.view.clone()),
                    subjects: vec![id.clone()],
                    requirements: vec![Requirement {
                        feature: owner.clone(),
                        parameter: w.parameter.clone(),
                    }],
                    measurements: vec![format!("{owner}/{}", w.parameter)],
                    sources: Vec::new(),
                    stage: None,
                    annotation: claiming.first().map(|m| m.name.clone()),
                    related: claiming.iter().skip(1).map(|m| m.name.clone()).collect(),
                    evidence,
                    fix,
                }
            };
        let value = match &w.value {
            Ok(v) => *v,
            Err(why) => {
                issues.push(issue(
                    "unverifiable",
                    format!("{noun}: measurement {} cannot be checked: {why}", w.role),
                    Some(why.clone()),
                    None,
                ));
                continue;
            }
        };
        if claiming.iter().any(|m| holes::states(&m.text, value)) {
            continue;
        }
        let fix = Some(format!(
            "State {} for {id} ({owner}): a linear dimension across the channel, in the view that \
             looks along it.",
            fmt(value)
        ));
        issues.push(match claiming.first() {
            Some(m) => issue(
                "unverifiable",
                format!(
                    "{noun}: measurement {} is claimed by {} but its text does not state {}",
                    w.role,
                    m.name,
                    fmt(value)
                ),
                Some(format!("{:?} does not state {}", m.text, fmt(value))),
                fix,
            ),
            None => issue(
                "missing",
                format!(
                    "{noun}: measurement {} ({}) is not stated on the sheet",
                    w.role,
                    fmt(value)
                ),
                None,
                fix,
            ),
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::super::paired_ramp_steps::tests::part;
    use super::super::section_recesses::tests::recess;
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    /// test_issue_917's U channel: a 50 × 50 × 12 base with two 12.5 thick walls up to z = 24,
    /// the channel between them 25 wide along x, its floor at z = 6.
    fn u_channel() -> Analysis {
        let u = [
            ([-12.5, 24.0], 0.0),
            ([-12.5, 6.0], 0.0),
            ([12.5, 6.0], 0.0),
            ([12.5, 24.0], 0.0),
        ];
        let x_run = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let mut a = part(
            "section_recess",
            recess(
                "channel",
                "rectangular",
                x_run,
                [-25.0, 25.0],
                [true, true],
                false,
                &u,
            ),
        );
        a.envelope = [[-25.0, -25.0, -6.0], [25.0, 25.0, 24.0]];
        for (i, (axis, lo, hi)) in [("y", -25.0, -12.5), ("y", 12.5, 25.0), ("z", -6.0, 6.0)]
            .into_iter()
            .enumerate()
        {
            let mut p = a.features[0].clone();
            p.id = format!("f:plate:{}", i + 1);
            p.kind = "plate".into();
            p.faces = vec![10 + i];
            p.parameters = json!({ "axis": axis, "lo": lo, "hi": hi, "u": 0.0, "v": 9.0 })
                .as_object()
                .unwrap()
                .clone();
            a.features.push(p);
        }
        a
    }

    fn mark(name: &str, text: &str, feature: &str, parameter: &str) -> Mark {
        Mark {
            name: name.into(),
            view: Some("v:side".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: feature.into(),
                parameter: parameter.into(),
            }],
        }
    }

    /// test_issue_917 (`test_every_independent_channel_chain_measurement_has_actionable_lint`):
    /// the channel's width `25`, the lower wall's thickness `12.5` and the overall depth `50`
    /// state the scheme; each taken away is `channel_requirement_missing`, named by its role.
    #[test]
    fn an_open_channel_states_its_width_wall_and_extent() {
        let a = u_channel();
        let marks = [
            mark(
                "dim_channel_y0",
                "25",
                "f:section_recess:1",
                "channel_width.length",
            ),
            mark("dim_plate_y0", "12.5", "f:plate:1", "thickness.length"),
            mark("m_env_depth", "50", "envelope", "depth.length"),
        ];
        assert!(coverage(&a, &marks).is_empty());
        for (name, role) in [
            ("dim_channel_y0", "channel.channel_width.length"),
            ("dim_plate_y0", "plate.thickness.length"),
            ("m_env_depth", "envelope.depth.length"),
        ] {
            let left: Vec<Mark> = marks.iter().filter(|m| m.name != name).cloned().collect();
            let found = coverage(&a, &left);
            assert_eq!(found.len(), 1, "{name}");
            assert_eq!(found[0].code, "channel_requirement_missing");
            assert!(found[0].message.contains(role), "{}", found[0].message);
        }
        // With one plate axis the recess is the step ladder's rebate: nothing is required.
        let mut rebate = a.clone();
        rebate.features.retain(|f| f.id != "f:plate:3");
        assert!(coverage(&rebate, &[]).is_empty());
        // No plate below the channel: its thickness cannot be checked.
        let mut thin = a;
        thin.features.retain(|f| f.id != "f:plate:1");
        thin.features[1].parameters["axis"] = json!("x");
        let found = coverage(&thin, &marks);
        assert_eq!(
            found
                .iter()
                .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
                .collect::<Vec<_>>(),
            ["channel_requirement_unverifiable thickness.length"]
        );
    }
}
