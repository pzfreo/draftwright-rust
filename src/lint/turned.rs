//! Turned-step completeness (Python's `linting/turned_step_coverage.py`): each outside-diameter
//! band of a turned profile states its axial length and its diameter
//! (LINT-turned_step_coverage-length_and_od).
//!
//! Python uses this ledger in its quality report. The port also reports each uncovered band
//! requirement, retaining `axial_length_missing` for missing lengths.
//!
//! Each of the Analysis's `turned_step` features (quiddity's `TurnedStep`: its `axis`, the band's
//! `lo` and `hi` along it, its `diameter`, and its `profile`: the turned body's `axis_origin` and
//! bounds) that no groove owns has two requirements, `step.length` and `step.diameter`:
//!
//! - **placed**: an annotation claims the step's parameter and its text states the value (the
//!   length is `hi − lo`); the diameter is placed too when the part's overall diameter
//!   (`envelope`/`od.diameter`, which Python's `rotational` OD stands for) is claimed and stated,
//!   and the band is the unique largest of its profile and the only band of its axis line and
//!   diameter;
//! - **unverifiable**: an annotation claims it but its text does not state the value, the record
//!   cannot be read, or groove ownership is ambiguous (then every band is unverifiable, as in
//!   Python, since ownership changes the count);
//! - **missing**: nothing claims it.
//!
//! A groove owns a band (draftwright's `groove_owns_turned_step_band`) when it lies on the
//! band's axis, its station is within the band (0.1 mm), and the band is no longer than the
//! groove's width plus 1 mm; the groove must belong to exactly one turned profile: the profile
//! its record names, whose axis line it lies on (within 0.0005 mm) and whose shoulders contain
//! its width. `suppressed`, `dropped` and `satisfied_by_structured_note` are not reached: lint
//! cannot see the author's omissions or the placement drops.

use serde::Serialize;
use serde_json::Value;

use super::parameters::{self, fmt};
use super::{Mark, holes};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, point, principal, to_part};
use crate::model::Analysis;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A requirement's outcome in a completeness ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Placed,
    Missing,
    Unverifiable,
}

/// One requirement of one turned band, and how the marks meet it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Outcome {
    pub feature: String,
    /// `step.length` or `step.diameter`.
    pub parameter: String,
    pub state: State,
    /// The annotation that states it, or that claims it without stating it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    /// Why it is unverifiable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// A turned step's band in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    /// The part's axis the band turns about.
    pub axis: usize,
    /// A point on its axis line.
    pub on: [f64; 3],
    /// Its ends along the axis, `lo < hi`.
    pub lo: f64,
    pub hi: f64,
    pub diameter: f64,
}

/// A turned step's record axis (its index in quiddity's frame), `lo`, `hi`, diameter and
/// `profile.axis_origin`, or why they cannot be read.
fn record(f: &Feature) -> Result<(usize, f64, f64, f64, [f64; 3]), String> {
    let p = &f.parameters;
    let axis = p
        .get("axis")
        .and_then(Value::as_str)
        .and_then(|a| AXES.iter().position(|x| *x == a))
        .ok_or("its record gives no principal axis")?;
    let (lo, hi) = (parameters::size(f, "lo")?, parameters::size(f, "hi")?);
    if hi <= lo {
        return Err("its band has no length".into());
    }
    let diameter = parameters::size(f, "diameter")?;
    if diameter <= 0.0 {
        return Err("its diameter is not positive".into());
    }
    let origin = point(p.get("profile").and_then(|v| v.get("axis_origin")))
        .ok_or("its record gives no profile axis origin")?;
    Ok((axis, lo, hi, diameter, origin))
}

/// A turned step's band in the part's frame, or why it cannot be read.
pub fn band(frame: &[[f64; 3]; 4], f: &Feature) -> Result<Band, String> {
    let (a, lo, hi, diameter, origin) = record(f)?;
    let mut d = [0.0; 3];
    d[a] = 1.0;
    let k = principal(dir_to_part(frame, d)).ok_or("its axis lies along no axis of the part")?;
    let at = |t: f64| {
        let mut q = origin;
        q[a] = t;
        to_part(frame, q)
    };
    let (p, q) = (at(lo), at(hi));
    Ok(Band {
        axis: k,
        on: p,
        lo: p[k].min(q[k]),
        hi: p[k].max(q[k]),
        diameter,
    })
}

/// The turned steps that are bands of their own (no groove owns them), or why groove ownership
/// cannot be decided.
pub fn bands(analysis: &Analysis) -> Result<Vec<&Feature>, String> {
    let steps: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .collect();
    // The turned profiles, by their records' `profile` (Python's `TurnedProfileKey`): each its
    // key and the shoulders of its readable bands.
    let mut profiles: Vec<(&Value, Vec<f64>)> = Vec::new();
    for s in &steps {
        let (Some(key), Ok((_, lo, hi, _, _))) = (s.parameters.get("profile"), record(s)) else {
            continue;
        };
        match profiles.iter_mut().find(|(k, _)| *k == key) {
            Some((_, shoulders)) => shoulders.extend([lo, hi]),
            None => profiles.push((key, vec![lo, hi])),
        }
    }
    let mut owned = vec![false; steps.len()];
    for g in analysis.features.iter().filter(|f| f.kind == "groove") {
        let p = &g.parameters;
        let axis = p
            .get("axis")
            .and_then(Value::as_str)
            .and_then(|a| AXES.iter().position(|x| *x == a));
        let (Some(a), Some(at), Ok(width)) =
            (axis, point(p.get("at")), parameters::size(g, "width"))
        else {
            return Err(format!(
                "groove {} has no readable axis, place and width",
                g.id
            ));
        };
        let named = p.get("profile");
        let candidates: Vec<&(&Value, Vec<f64>)> = profiles
            .iter()
            .filter(|(k, _)| named.is_none_or(|n| n == *k))
            .collect();
        let owners: Vec<&&(&Value, Vec<f64>)> = candidates
            .iter()
            .filter(|(k, shoulders)| {
                let same_axis = k.get("axis").and_then(Value::as_str) == Some(AXES[a]);
                let on_line = point(k.get("axis_origin")).is_some_and(|o| {
                    (0..3).all(|j| j == a || (at[j] - o[j]).abs() <= 0.0005 + 5e-9)
                });
                let lo = shoulders.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = shoulders.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let tol = 1e-3 + 1e-9;
                same_axis
                    && on_line
                    && lo - tol <= at[a] - width / 2.0
                    && at[a] + width / 2.0 <= hi + tol
            })
            .collect();
        if owners.len() > 1 || (named.is_some() && !candidates.is_empty() && owners.is_empty()) {
            return Err(format!(
                "groove {} does not belong to exactly one turned profile",
                g.id
            ));
        }
        let Some((key, _)) = owners.first() else {
            continue;
        };
        for (i, s) in steps.iter().enumerate() {
            if s.parameters.get("profile") != Some(*key) {
                continue;
            }
            let Ok((sa, lo, hi, _, _)) = record(s) else {
                continue;
            };
            if sa == a && lo - 0.1 <= at[a] && at[a] <= hi + 0.1 && hi - lo <= width + 1.0 {
                owned[i] = true;
            }
        }
    }
    Ok(steps
        .into_iter()
        .zip(owned)
        .filter(|(_, o)| !o)
        .map(|(s, _)| s)
        .collect())
}

/// How the marks meet the requirement *parameter* of *id*, whose value is *value*: the first
/// claiming mark that states it, else the first that claims it.
fn judged<'a>(
    marks: &'a [Mark],
    id: &str,
    parameter: &str,
    value: f64,
) -> (State, Option<&'a Mark>) {
    let claiming: Vec<&Mark> = marks
        .iter()
        .filter(|m| {
            m.claims
                .iter()
                .any(|c| c.feature == id && c.parameter == parameter)
        })
        .collect();
    match claiming.iter().find(|m| holes::states(&m.text, value)) {
        Some(m) => (State::Placed, Some(m)),
        None => match claiming.first() {
            Some(m) => (State::Unverifiable, Some(m)),
            None => (State::Missing, None),
        },
    }
}

/// The turned-step ledger: each band's length and diameter, in Analysis order.
pub fn outcomes(analysis: &Analysis, marks: &[Mark]) -> Vec<Outcome> {
    let frame = frame(analysis);
    let unverifiable = |f: &Feature, parameter: &str, why: String| Outcome {
        feature: f.id.clone(),
        parameter: parameter.into(),
        state: State::Unverifiable,
        annotation: None,
        evidence: Some(why),
    };
    let steps = match bands(analysis) {
        Ok(s) => s,
        Err(why) => {
            return analysis
                .features
                .iter()
                .filter(|f| f.kind == "turned_step")
                .flat_map(|f| {
                    ["step.length", "step.diameter"].map(|p| unverifiable(f, p, why.clone()))
                })
                .collect();
        }
    };
    let read: Vec<(&Feature, Result<Band, String>)> =
        steps.iter().map(|f| (*f, band(&frame, f))).collect();
    let mut out = Vec::new();
    for (f, b) in &read {
        let b = match b {
            Ok(b) => b,
            Err(why) => {
                for p in ["step.length", "step.diameter"] {
                    out.push(unverifiable(f, p, why.clone()));
                }
                continue;
            }
        };
        for (parameter, value) in [("step.length", b.hi - b.lo), ("step.diameter", b.diameter)] {
            let (mut state, mut mark) = judged(marks, &f.id, parameter, value);
            if parameter == "step.diameter" && state == State::Missing && overall(&read, f, b) {
                let (s, m) = judged(marks, "envelope", "od.diameter", b.diameter);
                if s == State::Placed {
                    (state, mark) = (s, m);
                }
            }
            out.push(Outcome {
                feature: f.id.clone(),
                parameter: parameter.into(),
                state,
                annotation: mark.map(|m| m.name.clone()),
                evidence: (state == State::Unverifiable)
                    .then(|| format!("{:?} does not state {}", mark.unwrap().text, fmt(value))),
            });
        }
    }
    out
}

/// Whether band *b* of step *f* is the one the part's overall diameter can stand for: the unique
/// largest band of its profile, and the only band on its axis line with its diameter.
fn overall(read: &[(&Feature, Result<Band, String>)], f: &Feature, b: &Band) -> bool {
    let profile = f.parameters.get("profile");
    let same_line = |c: &Band| {
        c.axis == b.axis && (0..3).all(|j| j == b.axis || (c.on[j] - b.on[j]).abs() <= 1e-6)
    };
    let mut in_profile = read
        .iter()
        .filter(|(g, _)| g.parameters.get("profile") == profile)
        .filter_map(|(_, c)| c.as_ref().ok());
    let unique_max = in_profile.clone().all(|c| c.diameter <= b.diameter)
        && in_profile
            .clone()
            .filter(|c| c.diameter == b.diameter)
            .count()
            == 1
        && in_profile.next().is_some();
    let alone = read
        .iter()
        .filter_map(|(_, c)| c.as_ref().ok())
        .filter(|c| same_line(c) && c.diameter == b.diameter)
        .count()
        == 1;
    unique_max && alone
}

/// Band requirements the printed drawing does not prove.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<super::LintIssue> {
    outcomes(analysis, marks)
        .into_iter()
        .filter(|o| o.state != State::Placed)
        .map(|o| {
            let code = if o.state == State::Unverifiable {
                "turned_step_requirement_unverifiable"
            } else if o.parameter == "step.length" {
                "axial_length_missing"
            } else {
                "turned_step_requirement_missing"
            };
            let mut issue = super::uncovered(
                analysis,
                code,
                &o.feature,
                &o.parameter,
                format!(
                    "{}: {} is {} on the drawing.",
                    o.feature,
                    o.parameter,
                    if o.state == State::Missing {
                        "missing"
                    } else {
                        "unverifiable"
                    }
                ),
            );
            issue.annotation = o.annotation;
            issue.evidence = o.evidence;
            issue
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    /// string_post's turned profile (quiddity 0.3.9), with connector109's first groove moved
    /// onto its ø6 band: three bands of their own, each length and diameter judged.
    fn part(groove: Option<Value>) -> Analysis {
        let profile = json!({ "axis": "x", "axis_origin": [0.0, 9.46e-06, 2.81e-06],
                              "body_bounds": [-4.79568699, 10.00431301, -3.74999064, 3.75000956,
                                              -3.74999729, 3.75000291] });
        let step = |lo: f64, hi: f64, d: f64| json!({ "axis": "x", "lo": lo, "hi": hi, "diameter": d, "profile": profile.clone() });
        let mut features = vec![
            ("turned_step", step(-4.796, -3.796, 7.5)),
            ("turned_step", step(-3.796, 1.704, 6.0)),
            ("turned_step", step(1.704, 2.804, 5.0)),
        ];
        if let Some(mut g) = groove {
            g["profile"] = profile.clone();
            features.push(("groove", g));
        }
        let features: Vec<Value> = features
            .into_iter()
            .enumerate()
            .map(|(i, (kind, r))| {
                json!({
                    "id": format!("f:{kind}:{}", i + 1), "kind": kind, "faces": [i],
                    "parameters": r,
                    "fingerprint": { "version": "test", "family": "test", "subtype": "test",
                                     "sizes": {}, "position": [0.0, 0.0, 0.0],
                                     "neighbourhood": [] },
                })
            })
            .collect();
        // quiddity's frame for string_post: its x is the part's -z.
        serde_json::from_value(json!({
            "schema": "draftwright.analysis/2",
            "binding": { "source_sha256": "", "part": "test", "face_count": 4, "loader": "test" },
            "envelope": [[-3.75, -3.75, 0.0], [3.75, 3.75, 14.8]],
            "features": features,
            "iso_bounds": [0.0, 0.0, 0.0, 0.0],
            "recognition": { "recogniser": "test",
                             "frame": [[0.0, 0.0, 10.004], [0.0, 0.0, -1.0], [-1.0, 0.0, 0.0],
                                       [0.0, 1.0, 0.0]] },
        }))
        .unwrap()
    }

    fn mark(name: &str, text: &str, feature: &str, parameter: &str) -> Mark {
        Mark {
            name: name.into(),
            view: Some("v:front".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: feature.into(),
                parameter: parameter.into(),
            }],
        }
    }

    fn states(a: &Analysis, marks: &[Mark]) -> Vec<String> {
        outcomes(a, marks)
            .into_iter()
            .map(|o| format!("{} {} {:?}", o.feature, o.parameter, o.state))
            .collect()
    }

    /// Each band's length and diameter are placed when a claiming annotation prints them,
    /// unverifiable when it misprints them, and missing when nothing claims them. The part's
    /// overall diameter stands for its largest band's.
    #[test]
    fn a_turned_band_states_its_length_and_diameter() {
        let a = part(None);
        let b = band(&frame(&a), &a.features[0]).unwrap();
        assert_eq!(b.axis, 2);
        assert!((b.lo - 13.8).abs() < 1e-9 && (b.hi - 14.8).abs() < 1e-9);
        let marks = [
            mark("m_steplen0", "1", "f:turned_step:1", "step.length"),
            mark("m_steplen1", "5.5", "f:turned_step:2", "step.length"),
            mark("m_steplen2", "1.2", "f:turned_step:3", "step.length"),
            mark("m_dia_z1", "ø6", "f:turned_step:2", "step.diameter"),
            mark("dim_od", "ø7.5", "envelope", "od.diameter"),
        ];
        assert_eq!(
            states(&a, &marks),
            [
                "f:turned_step:1 step.length Placed",
                "f:turned_step:1 step.diameter Placed",
                "f:turned_step:2 step.length Placed",
                "f:turned_step:2 step.diameter Placed",
                "f:turned_step:3 step.length Unverifiable",
                "f:turned_step:3 step.diameter Missing",
            ]
        );
        assert_eq!(
            outcomes(&a, &marks)[4].evidence.as_deref(),
            Some("\"1.2\" does not state 1.1")
        );
    }

    /// A groove on a band owns it: the band is the groove's (its width and floor diameter), not
    /// a turned step's. A groove no profile can own leaves every band unverifiable.
    #[test]
    fn a_band_a_groove_owns_is_the_grooves() {
        let groove = json!({ "axis": "x", "at": [2.254, 0.0, 0.0], "width": 1.1,
                             "diameter": 5.0 });
        let a = part(Some(groove.clone()));
        let ids: Vec<&str> = bands(&a).unwrap().iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["f:turned_step:1", "f:turned_step:2"]);
        let mut astray = groove;
        astray["at"] = json!([2.254, 1.0, 0.0]);
        let a = part(Some(astray));
        assert!(states(&a, &[]).iter().all(|s| s.ends_with("Unverifiable")));
    }
}
