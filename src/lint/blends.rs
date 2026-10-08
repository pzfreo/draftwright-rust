//! Blend coverage (Python's `linting/blend_coverage.py`): every recognised blend chain states its
//! radius (LINT-blend_coverage-radius, LINT-code-blend_requirement_state).
//!
//! Each of the Analysis's `blend` features (quiddity's `Blend`: a `radius`, a `side`, and a
//! `path`, straight (`at`, `direction`) or circular (`center`, `normal`, `radius`)) has one
//! requirement, `blend.radius`, judged by [`parameters::coverage`]. A blend whose path is neither
//! is still a radius to state; only its description in messages is shorter.
//!
//! Not ported: Python's check that a placed radius leader's tip touches the blend's trimmed arc
//! (`lint_blend_leader_targets`), which needs the drawn geometry.

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark};
use crate::bridge::Feature;
use crate::bridge::holes::frame;
use crate::model::Analysis;

/// The blends' requirements.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    analysis
        .features
        .iter()
        .filter(|f| f.kind == "blend")
        .map(|f| {
            let radius = parameters::size(f, "radius");
            Wanted {
                feature: f,
                parameter: "blend.radius".into(),
                callout: radius
                    .as_ref()
                    .map_or("R?".into(), |r| format!("R{}", fmt(*r))),
                values: radius.map(|r| vec![r]),
            }
        })
        .collect()
}

/// The blend coverage findings.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let p = &f.parameters;
        let mut s = format!("blend {}", f.id);
        if let Ok(r) = parameters::size(f, "radius") {
            s += &format!(" R{}", fmt(r));
        }
        if let Some(side) = p.get("side").and_then(|v| v.as_str()) {
            s += &format!(" {side}");
        }
        let path = p.get("path");
        let get = |k: &str| path.and_then(|v| v.get(k));
        if let (Some(at), Some(d)) = (
            parameters::place(&frame, get("at")),
            parameters::direction(&frame, get("direction")),
        ) {
            s += &format!(", straight along {} through {}", fmt3(d), fmt3(at));
        } else if let (Some(c), Some(n)) = (
            parameters::place(&frame, get("center")),
            parameters::direction(&frame, get("normal")),
        ) {
            s += &format!(", circular about {} centred at {}", fmt3(n), fmt3(c));
            if let Some(r) = get("radius").and_then(|v| v.as_f64()) {
                s += &format!(" (path radius {})", fmt(r));
            }
        }
        s
    };
    parameters::coverage(
        "blend",
        &wanted(analysis),
        marks,
        noun,
        "a radius callout, in a view that sees the blend's arc",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::{Value, json};

    fn part(records: &[Value]) -> Analysis {
        let features: Vec<Value> = records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                json!({
                    "id": format!("f:blend:{}", i + 1), "kind": "blend", "faces": [i],
                    "parameters": r,
                    "fingerprint": { "version": "test", "family": "blends", "subtype": "Blend",
                                     "sizes": {}, "position": [0.0, 0.0, 0.0],
                                     "neighbourhood": [] },
                })
            })
            .collect();
        serde_json::from_value(json!({
            "schema": "draftwright.analysis/2",
            "binding": { "source_sha256": "", "part": "test", "face_count": records.len(),
                         "loader": "test" },
            "envelope": [[0.0, 0.0, 0.0], [100.0, 100.0, 10.0]],
            "features": features,
            "iso_bounds": [0.0, 0.0, 0.0, 0.0],
        }))
        .unwrap()
    }

    /// LINT-blend_coverage-radius: a straight or a circular-path blend states its radius
    /// (string_post's `2× R0.2` states its two R0.25 blends at one decimal); a blend whose record
    /// gives no radius is unverifiable.
    #[test]
    fn a_blend_states_its_radius_whatever_its_path() {
        let a = part(&[
            json!({ "radius": 0.25, "side": "convex",
                    "path": { "center": [-4.546, 0.0, 0.0], "normal": [1.0, 0.0, 0.0],
                              "radius": 3.5 } }),
            json!({ "radius": 2.0, "side": "concave",
                    "path": { "at": [-15.739, 83.503, -119.75], "direction": [0.0, 0.0, 1.0] } }),
            json!({ "side": "concave",
                    "path": { "at": [0.0, 0.0, 0.0], "direction": [0.0, 0.0, 1.0] } }),
        ]);
        let mark = |text: &str, f: &str| Mark {
            name: format!("m:{f}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: f.into(),
                parameter: "blend.radius".into(),
            }],
        };
        let issues = coverage(&a, &[mark("2× R0.2", "f:blend:1")]);
        let found: Vec<String> = issues
            .iter()
            .map(|i| format!("{} {}", i.code, i.subjects[0]))
            .collect();
        assert_eq!(
            found,
            [
                "blend_requirement_missing f:blend:2",
                "blend_requirement_unverifiable f:blend:3"
            ]
        );
        assert_eq!(
            issues[0].message,
            "blend f:blend:2 R2 concave, straight along (0, 0, 1) through (-15.739, 83.503, -119.75): requirement blend.radius (R2) is not stated on the sheet"
        );
        // Misprinted: claimed, but R0.4 is not R0.25 at one decimal.
        assert_eq!(
            coverage(&a, &[mark("R0.4", "f:blend:1")])[0].code,
            "blend_requirement_unverifiable"
        );
    }
}
