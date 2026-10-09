//! Oriented slot coverage (Python's `linting/oriented_slot_coverage.py`): a free-direction slot
//! (quiddity's `OrientedSlot`, a rectangular through passage whose width and length need not lie
//! along the part's axes) states its width and its length (LINT-oriented_slot_coverage-width_length,
//! LINT-code-oriented_slot_requirement_state).
//!
//! Each `oriented_slot` feature has two requirements, `oriented_slot_width.length` and
//! `oriented_slot_length.length`, the record's `width` and `length`, judged by
//! [`parameters::coverage`] (draftwright's callout prints both: `ORIENTED SLOT 2 WIDE × 4.5 LONG`).
//! Python joins each record to exactly one IR feature by every structural fact, and reports both
//! requirements unverifiable when it cannot; here the Analysis feature is the record, so the join
//! fails only when a record does not give a positive width and length, or when two records are
//! the same (Python's two sources with one key), and then both requirements are unverifiable.
//!
//! Not ported: Python excludes the members of oriented slot patterns (`standalone_oriented_slots`:
//! draftwright gives the patterns no drafting grammar, #1430, and its quality report lists them
//! as an undecided family). The Analysis does not record the patterns (the bridge reads their
//! members one by one, as oriented slots), so every oriented slot is judged here, and a pattern's
//! members are reported where Python reports nothing. Not reached either: `_suppressed` and
//! `_dropped` (lint cannot read the author's omissions or placement drops) and the assembly
//! severity.

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark};
use crate::bridge::Feature;
use crate::bridge::holes::frame;
use crate::model::Analysis;

/// A record's size *name*, if it is a positive finite number.
fn size(f: &Feature, name: &str) -> Result<f64, String> {
    match parameters::size(f, name)? {
        v if v.is_finite() && v > 0.0 => Ok(v),
        v => Err(format!("its {name} {v} is not a positive size")),
    }
}

/// The oriented slots' requirements: each slot's width and length, or, when its record cannot be
/// read or another record is the same, both unverifiable.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let slots: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "oriented_slot")
        .collect();
    let mut out = Vec::new();
    for f in &slots {
        let twin = slots
            .iter()
            .any(|g| g.id != f.id && g.parameters == f.parameters);
        for (parameter, name, word) in [
            ("oriented_slot_width.length", "width", "wide"),
            ("oriented_slot_length.length", "length", "long"),
        ] {
            let values = if twin {
                Err("another oriented slot's record is the same, so neither is one slot".into())
            } else {
                size(f, name).map(|v| vec![v])
            };
            let callout = match &values {
                Ok(v) => format!("{} {word}", fmt(v[0])),
                Err(_) => format!("its {name}"),
            };
            out.push(Wanted {
                feature: f,
                parameter: parameter.into(),
                values,
                callout,
            });
        }
    }
    out
}

/// The oriented slot coverage findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let mut s = format!("oriented slot {}", f.id);
        if let (Ok(w), Ok(l)) = (size(f, "width"), size(f, "length")) {
            s += &format!(" {} wide × {} long", fmt(w), fmt(l));
        }
        if let Some(at) = parameters::place(&frame, f.parameters.get("center")) {
            s += &format!(", at {}", fmt3(at));
        }
        s
    };
    parameters::coverage(
        "oriented_slot",
        &wanted(analysis),
        marks,
        noun,
        "the slot's callout (ORIENTED SLOT W WIDE × L LONG), in a view that looks along its run",
    )
}

#[cfg(test)]
mod tests {
    use super::super::paired_ramp_steps::tests::part;
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    fn mark(text: &str, parameters: &[&str]) -> Mark {
        Mark {
            name: "m_oriented_slot_x_0".into(),
            view: Some("v:side".into()),
            text: text.into(),
            claims: parameters
                .iter()
                .map(|p| Claim {
                    feature: "f:oriented_slot:1".into(),
                    parameter: p.to_string(),
                })
                .collect(),
        }
    }

    const BOTH: [&str; 2] = ["oriented_slot_width.length", "oriented_slot_length.length"];

    /// mfcadpp_holdout/467's oriented slot (quiddity 0.3.9: 2 wide, 4.543 long): draftwright's
    /// callout `ORIENTED SLOT 2 WIDE × 4.5 LONG` states both; one stating the width alone leaves
    /// the length missing; a misprinted length is unverifiable; a record without a length, or
    /// two records the same, cannot be checked.
    #[test]
    fn an_oriented_slot_states_its_width_and_length() {
        let record = json!({ "center": [4.675, 0.369, 0.703], "width": 2.0, "length": 4.543,
                             "width_direction": [0.0, 0.6, 0.8],
                             "long_direction": [0.0, -0.8, 0.6] });
        let a = part("oriented_slot", record.clone());
        let got: Vec<(String, f64)> = wanted(&a)
            .into_iter()
            .map(|w| (w.parameter, w.values.unwrap()[0]))
            .collect();
        assert_eq!(
            got,
            [(BOTH[0].to_string(), 2.0), (BOTH[1].to_string(), 4.543)]
        );
        assert!(coverage(&a, &[mark("ORIENTED SLOT 2 WIDE × 4.5 LONG", &BOTH)]).is_empty());
        let codes = |a: &Analysis, marks: &[Mark]| -> Vec<String> {
            coverage(a, marks)
                .into_iter()
                .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
                .collect()
        };
        assert_eq!(
            codes(&a, &[mark("2", &BOTH[..1])]),
            ["oriented_slot_requirement_missing oriented_slot_length.length"]
        );
        assert_eq!(
            codes(&a, &[mark("ORIENTED SLOT 2 WIDE × 4.6 LONG", &BOTH)]),
            ["oriented_slot_requirement_unverifiable oriented_slot_length.length"]
        );
        assert_eq!(
            coverage(&a, &[])[0].message,
            "oriented slot f:oriented_slot:1 2 wide × 4.543 long, at (4.675, 0.369, 0.703): \
             requirement oriented_slot_width.length (2 wide) is not stated on the sheet"
        );

        let mut r = record.clone();
        r.as_object_mut().unwrap().remove("length");
        let short = part("oriented_slot", r);
        assert_eq!(
            codes(&short, &[mark("ORIENTED SLOT 2 WIDE × 4.5 LONG", &BOTH)]),
            ["oriented_slot_requirement_unverifiable oriented_slot_length.length"]
        );

        let mut twins = part("oriented_slot", record);
        let mut second = twins.features[0].clone();
        second.id = "f:oriented_slot:2".into();
        twins.features.push(second);
        let found = coverage(&twins, &[mark("ORIENTED SLOT 2 WIDE × 4.5 LONG", &BOTH)]);
        assert_eq!(found.len(), 4);
        assert!(
            found
                .iter()
                .all(|i| i.code == "oriented_slot_requirement_unverifiable"),
            "{found:#?}"
        );
    }
}
