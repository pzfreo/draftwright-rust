//! Hole-pattern location and spacing, from physical members and printed measurement claims.
//! A proven grid is located at its centre; other arrays need one physical member anchor.

use super::{LintIssue, Mark, holes};
use crate::bridge::holes::{self as geometry, frame, point, to_part};
use crate::model::Analysis;

const TOL: f64 = 0.01;

fn stations(centres: &[[f64; 3]], axis: usize) -> Vec<f64> {
    let mut values: Vec<f64> = centres.iter().map(|c| c[axis]).collect();
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() <= TOL);
    values
}

fn even(values: &[f64]) -> bool {
    values.len() < 3
        || values
            .windows(2)
            .all(|w| ((w[1] - w[0]) - (values[1] - values[0])).abs() <= TOL)
}

/// Only marks on the exact pattern owner can satisfy its location or spacing requirements.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let read: Vec<_> = geometry::read(analysis)
        .into_iter()
        .filter_map(|(_, h)| h.ok())
        .collect();
    let refs: Vec<_> = read.iter().collect();
    let bosses = geometry::bosses(analysis);
    let mut issues = Vec::new();
    let Some(recognition) = &analysis.recognition else {
        return issues;
    };
    for p in &recognition.patterns {
        let Ok(members) = geometry::established_with(p, &refs, &bosses) else {
            continue;
        };
        let Some((axis, _)) = read[members[0]].principal else {
            continue;
        };
        let axes: Vec<_> = (0..3).filter(|&k| k != axis).collect();
        let (u, v) = (axes[0], axes[1]);
        let centres: Vec<_> = members.iter().map(|&i| read[i].at).collect();
        let cols = stations(&centres, u);
        let rows = stations(&centres, v);
        let lattice = p.arrangement == "grid"
            && cols.len() * rows.len() == centres.len()
            && even(&cols)
            && even(&rows)
            && cols.iter().all(|x| {
                rows.iter().all(|y| {
                    centres
                        .iter()
                        .filter(|c| (c[u] - x).abs() <= TOL && (c[v] - y).abs() <= TOL)
                        .count()
                        == 1
                })
            });
        let mut anchors = centres.clone();
        let mut requirements: Vec<(String, Option<f64>)> = Vec::new();
        if p.arrangement == "bolt_circle" {
            anchors = point(p.parameters.get("center"))
                .map(|c| vec![to_part(&frame(analysis), c)])
                .unwrap_or_default();
            requirements.push((
                "bolt_circle.diameter".into(),
                p.parameters.get("diameter").and_then(|d| d.as_f64()),
            ));
        } else if lattice {
            let mut centre = centres[0];
            centre[u] = (cols[0] + cols[cols.len() - 1]) / 2.0;
            centre[v] = (rows[0] + rows[rows.len() - 1]) / 2.0;
            anchors = vec![centre];
            for (name, values) in [("col", &cols), ("row", &rows)] {
                if values.len() > 1 {
                    requirements.push((
                        format!("grid_pitch.length.{name}"),
                        Some(values[1] - values[0]),
                    ));
                }
            }
        } else if p.arrangement == "linear" {
            let pitch = if rows.len() == 1 && even(&cols) && cols.len() > 1 {
                Some(cols[1] - cols[0])
            } else if cols.len() == 1 && even(&rows) && rows.len() > 1 {
                Some(rows[1] - rows[0])
            } else {
                p.parameters.get("pitch").and_then(|x| x.as_f64())
            };
            requirements.push(("pitch.length".into(), pitch));
        } else {
            for name in ["col", "row"] {
                requirements.push((format!("grid_pitch.length.{name}"), None));
            }
        }
        for k in [u, v] {
            let parameter = format!("location.length.{}", ["x", "y", "z"][k]);
            let distances: Vec<_> = anchors
                .iter()
                .map(|c| c[k] - analysis.envelope[0][k])
                .collect();
            // A zero datum distance needs no numeric ordinate, as in the location backstop.
            if distances.iter().any(|d| d.abs() <= 1e-6) {
                continue;
            }
            let claiming: Vec<_> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.feature == p.id && c.parameter == parameter)
                })
                .collect();
            if claiming
                .iter()
                .any(|m| distances.iter().any(|d| holes::states(&m.text, *d)))
            {
                continue;
            }
            let mut issue = super::uncovered(
                analysis,
                "hole_pattern_requirement_missing",
                &p.id,
                &parameter,
                format!(
                    "{} has no proven {} location of its {} on the drawing.",
                    p.id,
                    ["x", "y", "z"][k],
                    if lattice || p.arrangement == "bolt_circle" {
                        "centre"
                    } else {
                        "physical member anchor"
                    }
                ),
            );
            issue.related = p.members.clone();
            issues.push(issue);
        }
        for (parameter, value) in requirements {
            let claiming: Vec<_> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.feature == p.id && c.parameter == parameter)
                })
                .collect();
            if value.is_some_and(|d| claiming.iter().any(|m| holes::states(&m.text, d))) {
                continue;
            }
            let code = if value.is_none() || !claiming.is_empty() {
                "hole_pattern_requirement_unverifiable"
            } else {
                "hole_pattern_requirement_missing"
            };
            let mut issue = super::uncovered(
                analysis,
                code,
                &p.id,
                &parameter,
                format!(
                    "{}: {parameter} is not proved by the printed drawing.",
                    p.id
                ),
            );
            issue.related = p.members.clone();
            issue.annotation = claiming.first().map(|m| m.name.clone());
            issue.evidence = value.map(|d| format!("Expected {}", super::parameters::fmt(d)));
            issues.push(issue);
        }
    }
    issues
}
