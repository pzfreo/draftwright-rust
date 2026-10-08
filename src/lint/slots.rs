//! Slot coverage (Python's `linting/slot_coverage.py`): a lone through slot states its width, its
//! length, and its end radius when its ends are proven arcs (LINT-slot_coverage-lone_slot,
//! LINT-code-slot_requirement_state).
//!
//! Each of the Analysis's `slot` features (quiddity's `Slot`: `width`, `length`, `end_radius`
//! when the ends are proven arcs, and its place: `long_axis`, `lo`/`hi` along it, `w_center` on
//! `width_axis`) has the requirements `slot_width.length`, `slot_length.length`, for an
//! obround `slot_end_radius.radius`, and its location `location_slot.length`, judged by
//! [`parameters::coverage`]. The location is draftwright's (`compiled.py`, `location_datum` is
//! `bbox` for a slot): the offset of the slot's low end along its long axis from the part's box
//! ([`location`]); a slot whose long axis lies along no axis of the part has a location that
//! cannot be checked.
//!
//! A slot that is a member of a recognised slot pattern (quiddity's `SlotArray` or `SlotGrid`, a
//! `slot_pattern` feature whose faces include the slot's) is the pattern's, not a lone slot,
//! as in Python. The pattern's own requirements (the member's sizes, its pitch or row and column
//! pitches, and its location) are not ported: each pattern is reported unverifiable rather than
//! left out.

use serde_json::Value;

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark, Requirement, Severity};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, principal};
use crate::model::Analysis;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A slot's location: the part's axis its long axis lies along, and the offset of its low end
/// along it from the part's box (the Analysis's envelope), or why it cannot be read.
pub fn location(analysis: &Analysis, f: &Feature) -> Result<(&'static str, f64), String> {
    let p = &f.parameters;
    let long = p
        .get("long_axis")
        .and_then(Value::as_str)
        .and_then(|a| AXES.iter().position(|x| *x == a))
        .ok_or("its record gives no long axis")?;
    let (lo, hi) = (parameters::size(f, "lo")?, parameters::size(f, "hi")?);
    let frame = frame(analysis);
    let mut d = [0.0; 3];
    d[long] = 1.0;
    let d = dir_to_part(&frame, d);
    let k = principal(d).ok_or("its long axis lies along no axis of the part")?;
    // The ends' coordinates along the part's axis k: quiddity's origin, and its long axis
    // carried onto k (its other axes are square to k, so they add nothing).
    let ends = [lo, hi].map(|t| frame[0][k] + t * d[k]);
    Ok((AXES[k], ends[0].min(ends[1]) - analysis.envelope[0][k]))
}

/// The slot patterns, and the lone slots: every `slot` feature not inside a pattern's faces.
fn split(analysis: &Analysis) -> (Vec<&Feature>, Vec<&Feature>) {
    let patterns: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot_pattern")
        .collect();
    let lone = analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot")
        .filter(|f| {
            !patterns
                .iter()
                .any(|p| f.faces.iter().all(|i| p.faces.binary_search(i).is_ok()))
        })
        .collect();
    (patterns, lone)
}

/// The lone slots' requirements.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let (_, lone) = split(analysis);
    let mut out = Vec::new();
    for f in lone {
        for (parameter, field) in [
            ("slot_width.length", "width"),
            ("slot_length.length", "length"),
        ] {
            let v = parameters::size(f, field);
            out.push(Wanted {
                feature: f,
                parameter: parameter.into(),
                callout: v
                    .as_ref()
                    .map_or(format!("its {field}"), |v| format!("{field} {}", fmt(*v))),
                values: v.map(|v| vec![v]),
            });
        }
        // An end radius quiddity did not prove (null) is no requirement: the ends may be square.
        if let Some(r) = f.parameters.get("end_radius").and_then(|v| v.as_f64()) {
            out.push(Wanted {
                feature: f,
                parameter: "slot_end_radius.radius".into(),
                values: Ok(vec![r]),
                callout: format!("R{}", fmt(r)),
            });
        }
        let at = location(analysis, f);
        out.push(Wanted {
            feature: f,
            parameter: "location_slot.length".into(),
            callout: at.as_ref().map_or("its location".into(), |(axis, v)| {
                format!("its low end {} from the part's {axis} edge", fmt(*v))
            }),
            values: at.map(|(_, v)| vec![v]),
        });
    }
    out
}

/// The slot coverage findings: the lone slots' requirements, and each slot pattern as one
/// requirement that cannot be checked yet.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let p = &f.parameters;
        let mut s = format!("slot {}", f.id);
        if let (Ok(w), Ok(l)) = (parameters::size(f, "width"), parameters::size(f, "length")) {
            s += &format!(" {} × {}", fmt(w), fmt(l));
        }
        if let Ok(r) = parameters::size(f, "end_radius") {
            s += &format!(", ends R{}", fmt(r));
        }
        if let Some(a) = parameters::direction(&frame, p.get("long_axis")) {
            s += &format!(", long along {}", fmt3(a));
        }
        s
    };
    let mut issues = parameters::coverage(
        "slot",
        &wanted(analysis),
        marks,
        noun,
        "a linear dimension or radius callout, in the view that sees the slot's outline",
    );
    let (patterns, _) = split(analysis);
    for p in patterns {
        issues.push(LintIssue {
            id: format!("l:slot_requirement_unverifiable:{}/?", p.id),
            code: "slot_requirement_unverifiable".into(),
            severity: Severity::Warning,
            message: format!(
                "slot pattern {}: its requirements cannot be checked: the port does not check a slot pattern's sizes, pitch or location yet",
                p.id
            ),
            at: None,
            view: None,
            subjects: vec![p.id.clone()],
            requirements: vec![Requirement {
                feature: p.id.clone(),
                parameter: "?".into(),
            }],
            measurements: Vec::new(),
            sources: Vec::new(),
            stage: None,
            annotation: None,
            related: Vec::new(),
            evidence: Some("slot pattern coverage is not ported".into()),
            fix: None,
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::{Value, json};

    fn part(features: &[(&str, &[usize], Value)]) -> Analysis {
        let features: Vec<Value> = features
            .iter()
            .enumerate()
            .map(|(i, (kind, faces, r))| {
                json!({
                    "id": format!("f:{kind}:{}", i + 1), "kind": kind, "faces": faces,
                    "parameters": r,
                    "fingerprint": { "version": "test", "family": "slots", "subtype": "Slot",
                                     "sizes": {}, "position": [0.0, 0.0, 0.0],
                                     "neighbourhood": [] },
                })
            })
            .collect();
        serde_json::from_value(json!({
            "schema": "draftwright.analysis/2",
            "binding": { "source_sha256": "", "part": "test", "face_count": 9, "loader": "test" },
            "envelope": [[0.0, 0.0, 0.0], [100.0, 100.0, 10.0]],
            "features": features,
            "iso_bounds": [0.0, 0.0, 0.0, 0.0],
        }))
        .unwrap()
    }

    fn slot(end_radius: Option<f64>) -> Value {
        json!({ "width": 6.0, "length": 14.97, "end_radius": end_radius, "long_axis": "z",
                "width_axis": "y", "lo": -110.93, "hi": -95.96, "w_center": 41.5,
                "d_lo": -13.74, "d_hi": -11.74 })
    }

    /// LINT-slot_coverage-lone_slot: width, length and location, and the end radius only when
    /// quiddity proved the ends are arcs. cgb245's slots, in its frame (quiddity's z is the
    /// part's x) and box (from x = -190): stated as `6`, `15.0`, `2× R3` and `79.1`, the low
    /// end's offset from the box along the long axis.
    #[test]
    fn a_lone_slot_states_its_width_length_location_and_proven_end_radius() {
        let mut a = part(&[("slot", &[0], slot(Some(3.0))), ("slot", &[1], slot(None))]);
        a.recognition = Some(
            serde_json::from_value(json!({
                "recogniser": "test",
                "frame": [[-0.000128, 7.68098, -3.64814], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0],
                          [1.0, 0.0, 0.0]],
            }))
            .unwrap(),
        );
        a.envelope = [[-190.000128, -84.0, -18.6], [190.0, 99.0, 6.6]];
        let w = wanted(&a);
        let names: Vec<String> = w
            .iter()
            .map(|w| format!("{} {}", w.feature.id, w.parameter))
            .collect();
        assert_eq!(
            names,
            [
                "f:slot:1 slot_width.length",
                "f:slot:1 slot_length.length",
                "f:slot:1 slot_end_radius.radius",
                "f:slot:1 location_slot.length",
                "f:slot:2 slot_width.length",
                "f:slot:2 slot_length.length",
                "f:slot:2 location_slot.length"
            ]
        );
        assert_eq!(
            location(&a, w[0].feature).map(|(k, v)| (k, fmt(v))),
            Ok(("x", "79.07".into()))
        );
        let mark = |text: &str, f: &str, p: &str| Mark {
            name: format!("m:{f}:{p}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: f.into(),
                parameter: p.into(),
            }],
        };
        let marks = [
            mark("6", "f:slot:1", "slot_width.length"),
            mark("15.0", "f:slot:1", "slot_length.length"),
            mark("2× R3", "f:slot:1", "slot_end_radius.radius"),
            mark("79.1", "f:slot:1", "location_slot.length"),
            mark("6", "f:slot:2", "slot_width.length"),
            mark("15.1", "f:slot:2", "slot_length.length"),
        ];
        let found: Vec<String> = coverage(&a, &marks)
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
            .collect();
        assert_eq!(
            found,
            [
                "slot_requirement_unverifiable slot_length.length",
                "slot_requirement_missing location_slot.length"
            ]
        );
        // A long axis along no axis of the part: the location cannot be checked.
        let first = w[0].feature.clone();
        a.recognition.as_mut().unwrap().frame[3] = [0.6, 0.8, 0.0];
        assert_eq!(
            location(&a, &first),
            Err("its long axis lies along no axis of the part".into())
        );
    }

    /// A pattern's members are the pattern's: no lone-slot requirement, and the pattern is
    /// reported unverifiable (its coverage is not ported) rather than left out.
    #[test]
    fn a_slot_patterns_members_are_not_lone_slots() {
        let a = part(&[
            ("slot", &[0, 1], slot(None)),
            ("slot", &[2, 3], slot(None)),
            ("slot", &[5], slot(None)),
            ("slot_pattern", &[0, 1, 2, 3], json!({ "pitch": 20.0 })),
        ]);
        let found: Vec<String> = coverage(&a, &[])
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.subjects[0]))
            .collect();
        assert_eq!(
            found,
            [
                "slot_requirement_missing f:slot:3",
                "slot_requirement_missing f:slot:3",
                "slot_requirement_missing f:slot:3",
                "slot_requirement_unverifiable f:slot_pattern:4"
            ]
        );
    }
}
