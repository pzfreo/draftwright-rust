//! Through-step coverage (Python's `linting/through_step_coverage.py`): a rectangular through
//! step states both legs of its open section (LINT-through_step_coverage-two_legs,
//! LINT-code-through_step_requirement_state).
//!
//! quiddity's `ThroughStep` gives the step's run `axis`, and its `section`: three points in the
//! plane across the run (its coordinates along the two other axes, in `x`, `y`, `z` order), whose
//! two segments are the legs. Each segment changes one coordinate; its length is the leg, a
//! requirement `through_step_leg.length.<axis>` named by the part's axis it lies along (quiddity's
//! axis taken through its frame; Python's IR is in the part's frame too). Each is judged by
//! [`parameters::coverage`]. A section that is not two legs along two different axes, or a leg
//! that lies along no axis of the part, cannot be checked and is reported unverifiable.
//!
//! Not ported: Python's alternative grammars, where a leg is stated instead by a plate's
//! thickness, a step level's height or position, or either of those with an envelope extent
//! (within 0.5 mm of the leg's interval). Until they are, such a leg is reported missing here.

use serde_json::Value;

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, principal};
use crate::model::Analysis;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A through step's legs in section order, each the part's axis it lies along and its length, or
/// why they cannot be read.
pub fn legs(f: &Feature, frame: &[[f64; 3]; 4]) -> Result<Vec<(&'static str, f64)>, String> {
    let p = &f.parameters;
    let run = p
        .get("axis")
        .and_then(Value::as_str)
        .and_then(|a| AXES.iter().position(|x| *x == a))
        .ok_or("its record gives no run axis")?;
    let section: Vec<[f64; 2]> = p
        .get("section")
        .and_then(Value::as_array)
        .ok_or("its record gives no section")?
        .iter()
        .map(|q| {
            let q = q.as_array()?;
            Some([q.first()?.as_f64()?, q.get(1)?.as_f64()?])
        })
        .collect::<Option<_>>()
        .ok_or("its section is not a list of points")?;
    if section.len() != 3 {
        return Err(format!(
            "its section has {} points, not the three of two legs",
            section.len()
        ));
    }
    let across: Vec<usize> = (0..3).filter(|&k| k != run).collect();
    let mut out = Vec::new();
    for w in section.windows(2) {
        let changed: Vec<usize> = (0..2).filter(|&i| w[0][i] != w[1][i]).collect();
        let [i] = changed[..] else {
            return Err("a segment of its section is not along one axis".into());
        };
        let mut local = [0.0; 3];
        local[across[i]] = 1.0;
        let along =
            principal(dir_to_part(frame, local)).ok_or("a leg lies along no axis of the part")?;
        out.push((AXES[along], (w[1][i] - w[0][i]).abs()));
    }
    if out[0].0 == out[1].0 {
        return Err("its two legs lie along one axis".into());
    }
    Ok(out)
}

/// The through steps' requirements: each leg, or, when the legs cannot be read, one requirement
/// (`through_step_leg.length`) reported unverifiable.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let frame = frame(analysis);
    let mut out = Vec::new();
    for f in analysis
        .features
        .iter()
        .filter(|f| f.kind == "through_step")
    {
        match legs(f, &frame) {
            Ok(legs) => out.extend(legs.into_iter().map(|(axis, length)| Wanted {
                feature: f,
                parameter: format!("through_step_leg.length.{axis}"),
                values: Ok(vec![length]),
                callout: format!("{} along {axis}", fmt(length)),
            })),
            Err(why) => out.push(Wanted {
                feature: f,
                parameter: "through_step_leg.length".into(),
                values: Err(why),
                callout: "its legs".into(),
            }),
        }
    }
    out
}

/// The through-step coverage findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let p = &f.parameters;
        let mut s = format!("through step {}", f.id);
        if let Some(a) = parameters::direction(&frame, p.get("axis")) {
            s += &format!(" along {}", fmt3(a));
        }
        if let Ok(legs) = legs(f, &frame) {
            let legs: Vec<String> = legs
                .iter()
                .map(|(axis, l)| format!("{} ({axis})", fmt(*l)))
                .collect();
            s += &format!(", legs {}", legs.join(" × "));
        }
        if let Some(at) = parameters::place(&frame, p.get("at")) {
            s += &format!(", at {}", fmt3(at));
        }
        s
    };
    parameters::coverage(
        "through_step",
        &wanted(analysis),
        marks,
        noun,
        "a linear dimension of the leg, in a view that sees the step's section",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    fn part(frame: [[f64; 3]; 4], record: Value) -> Analysis {
        serde_json::from_value(json!({
            "schema": "draftwright.analysis/2",
            "binding": { "source_sha256": "", "part": "test", "face_count": 2, "loader": "test" },
            "envelope": [[0.0, 0.0, 0.0], [100.0, 100.0, 10.0]],
            "features": [{
                "id": "f:through_step:1", "kind": "through_step", "faces": [0, 1],
                "parameters": record,
                "fingerprint": { "version": "test", "family": "through_steps",
                                 "subtype": "ThroughStep", "sizes": {},
                                 "position": [0.0, 0.0, 0.0], "neighbourhood": [] },
            }],
            "iso_bounds": [0.0, 0.0, 0.0, 0.0],
            "recognition": { "recogniser": "test", "frame": frame },
        }))
        .unwrap()
    }

    /// m10127's through step (quiddity 0.3.9): its section's first segment runs along y
    /// (11.512), its second along x (18.734); quiddity's frame turns x and y about, which leaves
    /// the axes' names alone.
    #[test]
    fn a_through_step_states_both_legs_named_by_the_parts_axes() {
        let record = json!({ "axis": "z", "at": [6.876, 13.323, -0.044], "length": 44.729,
                             "section": [[-2.491, 19.079], [-2.491, 7.567], [16.243, 7.567]] });
        let flip = [
            [16.0, 19.0, 22.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let a = part(flip, record.clone());
        let w = wanted(&a);
        let got: Vec<(&str, String)> = w
            .iter()
            .map(|w| (w.parameter.as_str(), fmt(w.values.as_ref().unwrap()[0])))
            .collect();
        assert_eq!(
            got,
            [
                ("through_step_leg.length.y", "11.512".to_string()),
                ("through_step_leg.length.x", "18.734".to_string())
            ]
        );
        let mark = |text: &str, axis: &str| Mark {
            name: format!("m:{axis}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: "f:through_step:1".into(),
                parameter: format!("through_step_leg.length.{axis}"),
            }],
        };
        assert!(coverage(&a, &[mark("18.7", "x"), mark("11.5", "y")]).is_empty());
        let found: Vec<String> = coverage(&a, &[mark("18.7", "x")])
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
            .collect();
        assert_eq!(
            found,
            ["through_step_requirement_missing through_step_leg.length.y"]
        );
        // quiddity's frame turning its x onto the part's z: the legs are named by the part's.
        let turned = [[0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let mut r = record.clone();
        r["axis"] = json!("z");
        let names: Vec<String> = wanted(&part(turned, r))
            .into_iter()
            .map(|w| w.parameter)
            .collect();
        assert_eq!(
            names,
            ["through_step_leg.length.y", "through_step_leg.length.z"]
        );
        // A section that is not two legs cannot be checked.
        let mut bad = record;
        bad["section"] = json!([[0.0, 0.0], [1.0, 1.0], [2.0, 1.0]]);
        let issues = coverage(&part(flip, bad), &[]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "through_step_requirement_unverifiable");
    }
}
