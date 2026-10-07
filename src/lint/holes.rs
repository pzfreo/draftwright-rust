//! Hole coverage (Python's `linting/hole_coverage.py`): every recognised hole, or group of
//! identical holes, states its sizes on the sheet.
//!
//! The requirements come from the Analysis's hole features alone:
//!
//! - **The groups** ([`groups`]): the members of a recognised hole pattern are one group; every
//!   other hole joins the loose holes with the same specification and the same signed drilling
//!   direction (LINT-hole_coverage-opposed_bores_distinct). A pattern whose members cannot all be
//!   found falls back to loose holes (LINT-hole_coverage-bolt_circle_corroboration's fallback).
//! - **Each group's requirements** ([`HoleGroup::requirements`]): its bore diameter
//!   (LINT-hole_coverage-bore_diameter); THRU for a through hole, or else its depth
//!   (-through_indicator, -blind_depth); the counterbore's, spotface's or countersink's two sizes
//!   (-cbore_diameter_depth, -spotface_diameter_depth, -csink_diameter_angle); and, for more than
//!   one hole, the quantity (-grouping_count).
//!
//! A requirement is **placed** when an annotation claims that measurement of one of the group's
//! holes *and* its printed text states the part's value at the precision printed (THRU for
//! `bore.through`; `<n>×` with n the group's size for `grouping.count`, both carried by the
//! diameter's callout). A size is **unverifiable** when an annotation claims it but its text
//! does not state it; otherwise a requirement is **missing**, THRU and the quantity included
//! when the diameter's callout leaves them off (LINT-code-hole_requirement_state). Only
//! those two are reported (LINT-_parameter_coverage-report_uncovered_only). Lint cannot see the
//! author's omissions or placement drops, so the `suppressed` and `dropped` outcomes are not
//! reached here.
//!
//! A size counts as printed when any number in the claiming callout rounds from it, so a callout
//! that misprints one size but prints another of the same value is not caught.
//!
//! Not yet ported: hole and pattern locations, pattern spacing, threads and profiled bores, the
//! assembly severity (the Analysis does not say whether the part is several bodies), and the
//! `suppressed` outcome.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{LintIssue, Mark, Requirement, Severity};
use crate::bridge::Feature;
use crate::model::Analysis;

/// Holes that share one machining specification, and so one callout.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleGroup {
    /// The holes' Analysis IDs, in Analysis order.
    pub members: Vec<String>,
    /// Whether quiddity recognised them as a pattern (rather than as loose identical holes).
    pub pattern: bool,
    pub sizes: HoleSizes,
    /// The members' centres in the part's frame (the centres of their faces' boxes).
    pub centres: Vec<[f64; 3]>,
    /// The axis in the part's frame, as the fingerprint gives it (its sign is arbitrary).
    pub axis: Option<[f64; 3]>,
}

/// A hole's machining sizes, as recognised.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleSizes {
    pub diameter: f64,
    pub through: bool,
    /// The bore's depth; for a through hole, the thickness it passes through.
    pub depth: f64,
    /// The counterbore's, spotface's and countersink's two sizes: diameter and depth, or for a
    /// countersink diameter and included angle.
    pub counterbore: Option<[f64; 2]>,
    pub spotface: Option<[f64; 2]>,
    pub countersink: Option<[f64; 2]>,
}

impl HoleSizes {
    fn of(f: &Feature) -> Option<HoleSizes> {
        let p = &f.parameters;
        let num = |v: Option<&Value>, k: &str| v.and_then(|v| v.get(k)).and_then(Value::as_f64);
        let pair = |k: &str, a: &str, b: &str| {
            let v = p.get(k).filter(|v| !v.is_null());
            Some([num(v, a)?, num(v, b)?])
        };
        Some(HoleSizes {
            diameter: p.get("diameter")?.as_f64()?,
            through: p.get("bottom").and_then(Value::as_str) == Some("through"),
            depth: p.get("depth").and_then(Value::as_f64).unwrap_or(f64::NAN),
            counterbore: pair("cbore", "diameter", "depth"),
            spotface: pair("spotface", "diameter", "depth"),
            countersink: pair("csink", "major_diameter", "included_angle"),
        })
    }

    /// What groups holes: every size (to the micrometre), and the depth only for a blind hole.
    fn key(&self) -> String {
        let r = |x: f64| format!("{:.3}", x);
        let two = |v: Option<[f64; 2]>| v.map_or("-".into(), |[a, b]| format!("{}/{}", r(a), r(b)));
        format!(
            "{} {} {} {} {}",
            r(self.diameter),
            if self.through {
                "THRU".into()
            } else {
                r(self.depth)
            },
            two(self.counterbore),
            two(self.spotface),
            two(self.countersink)
        )
    }
}

impl HoleGroup {
    /// The group's physical requirements, each with the value its text must state (none for
    /// `bore.through`; the group's size for `grouping.count`).
    pub fn requirements(&self) -> Vec<(&'static str, f64)> {
        let s = &self.sizes;
        let mut out = vec![("bore.diameter", s.diameter)];
        if s.through {
            out.push(("bore.through", f64::NAN));
        } else {
            out.push(("bore.depth", s.depth));
        }
        for (sizes, names) in [
            (s.counterbore, ["counterbore.diameter", "counterbore.depth"]),
            (s.spotface, ["spotface.diameter", "spotface.depth"]),
            (s.countersink, ["countersink.diameter", "countersink.angle"]),
        ] {
            if let Some([a, b]) = sizes {
                out.push((names[0], a));
                out.push((names[1], b));
            }
        }
        if self.members.len() > 1 {
            out.push(("grouping.count", self.members.len() as f64));
        }
        out
    }

    /// The callout that would state every size, as a fix suggests it.
    pub fn callout(&self) -> String {
        let s = &self.sizes;
        let mut out = String::new();
        if self.members.len() > 1 {
            out += &format!("{}× ", self.members.len());
        }
        out += &format!("⌀{}", fmt(s.diameter));
        out += &if s.through {
            " THRU".to_string()
        } else {
            format!(" ↧ {}", fmt(s.depth))
        };
        if let Some([d, h]) = s.counterbore {
            out += &format!(" ⌴ ⌀{} ↧ {}", fmt(d), fmt(h));
        }
        if let Some([d, h]) = s.spotface {
            out += &format!(" SF ⌀{} ↧ {}", fmt(d), fmt(h));
        }
        if let Some([d, a]) = s.countersink {
            out += &format!(" ⌵ ⌀{} × {}°", fmt(d), fmt(a));
        }
        out
    }

    /// The centre of the members' centres.
    pub fn centre(&self) -> [f64; 3] {
        let n = self.centres.len().max(1) as f64;
        [0, 1, 2].map(|k| self.centres.iter().map(|c| c[k]).sum::<f64>() / n)
    }

    fn noun(&self) -> String {
        let s = &self.sizes;
        let operation = if s.through {
            "THRU".to_string()
        } else {
            format!("blind depth {}", fmt(s.depth))
        };
        let axis = self
            .axis
            .map_or("unknown".into(), |a| format!("({})", a.map(fmt).join(", ")));
        let c = self.centre();
        format!(
            "{}× ø{} {operation} {}, axis {axis}, centred at ({})",
            self.members.len(),
            fmt(s.diameter),
            if self.pattern {
                "hole pattern"
            } else {
                "holes"
            },
            c.map(fmt).join(", ")
        )
    }
}

/// A number as messages print it: at most three decimals, trailing zeros dropped.
fn fmt(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn vector(v: Option<&Value>) -> Option<[f64; 3]> {
    let a = v?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

/// The Analysis's holes in groups that share a callout.
pub fn groups(analysis: &Analysis) -> Vec<HoleGroup> {
    let holes: Vec<(&Feature, HoleSizes)> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "hole")
        .filter_map(|f| Some((f, HoleSizes::of(f)?)))
        .collect();
    // The signed drilling direction and the opening's level along it, in quiddity's frame (one
    // frame for every feature of the part), to the micrometre.
    let direction = |f: &Feature| {
        vector(f.parameters.get("axis")).map_or("-".into(), |a| {
            a.map(|c| format!("{:.6}", c + 0.0)).join(",")
        })
    };
    let level = |f: &Feature| {
        let (a, o) = (
            vector(f.parameters.get("axis")),
            vector(f.parameters.get("location")),
        );
        a.zip(o).map_or("-".into(), |(a, o)| {
            format!("{:.3}", a[0] * o[0] + a[1] * o[1] + a[2] * o[2] + 0.0)
        })
    };
    let group = |members: &[&(&Feature, HoleSizes)], pattern: bool| HoleGroup {
        members: members.iter().map(|(f, _)| f.id.clone()).collect(),
        pattern,
        sizes: members[0].1.clone(),
        centres: members
            .iter()
            .map(|(f, _)| f.fingerprint.position)
            .collect(),
        axis: members[0].0.fingerprint.axis,
    };

    // Patterns: members share the pattern's count and pitch, the specification, the direction
    // and the opening's level. The Analysis does not name a hole's pattern, so a pattern is
    // accepted only when exactly its count of holes share all of that.
    let mut patterns: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (f, sizes)) in holes.iter().enumerate() {
        if let Some(p) = &f.fingerprint.pattern {
            let key = format!(
                "{} {:.6} {} {} {}",
                p.count,
                p.pitch,
                sizes.key(),
                direction(f),
                level(f)
            );
            patterns.entry(key).or_default().push(i);
        }
    }
    let mut out = Vec::new();
    let mut taken = vec![false; holes.len()];
    for members in patterns.values() {
        let count = holes[members[0]]
            .0
            .fingerprint
            .pattern
            .as_ref()
            .unwrap()
            .count;
        if members.len() == count && count > 1 {
            let m: Vec<_> = members.iter().map(|&i| &holes[i]).collect();
            out.push(group(&m, true));
            members.iter().for_each(|&i| taken[i] = true);
        }
    }
    let mut loose: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (f, sizes)) in holes.iter().enumerate() {
        if !taken[i] {
            loose
                .entry(format!("{} {}", sizes.key(), direction(f)))
                .or_default()
                .push(i);
        }
    }
    for members in loose.values() {
        let m: Vec<_> = members.iter().map(|&i| &holes[i]).collect();
        out.push(group(&m, false));
    }
    // In Analysis order of their first member.
    let order: BTreeMap<&str, usize> = analysis
        .features
        .iter()
        .enumerate()
        .map(|(i, f)| (f.id.as_str(), i))
        .collect();
    out.sort_by_key(|g| order[g.members[0].as_str()]);
    out
}

/// The numbers a text prints, each with its count of decimals, and whether `×` (or `X`, `x`)
/// follows it directly.
fn numbers(text: &str) -> Vec<(f64, usize, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut decimals = 0;
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    decimals += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            let times = matches!(chars.get(i), Some('×' | 'X' | 'x'));
            if let Ok(v) = s.parse() {
                out.push((v, decimals, times));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Whether *text* states *value* at the precision it prints: some number in it rounds from it.
pub fn states(text: &str, value: f64) -> bool {
    numbers(text).iter().any(|&(n, decimals, _)| {
        (n - value).abs() <= 0.5 * 10f64.powi(-(decimals as i32)) + 1e-9 * value.abs().max(1.0)
    })
}

/// Whether *text* states THRU.
fn says_thru(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .any(|w| w == "THRU")
}

/// Whether *text* states the quantity *n* (`n×`).
fn says_count(text: &str, n: usize) -> bool {
    numbers(text)
        .iter()
        .any(|&(v, d, times)| times && d == 0 && v == n as f64)
}

/// The hole coverage findings: each requirement of each group that the marks do not state.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for g in groups(analysis) {
        for (parameter, value) in g.requirements() {
            // THRU and the quantity are stated by the diameter's callout.
            let carrier = match parameter {
                "bore.through" | "grouping.count" => "bore.diameter",
                p => p,
            };
            let claiming: Vec<&Mark> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.parameter == carrier && g.members.contains(&c.feature))
                })
                .collect();
            let stated = |m: &&Mark| match parameter {
                "bore.through" => says_thru(&m.text),
                "grouping.count" => says_count(&m.text, g.members.len()),
                _ => states(&m.text, value),
            };
            if claiming.iter().any(stated) {
                continue;
            }
            let requirements: Vec<Requirement> = g
                .members
                .iter()
                .map(|f| Requirement {
                    feature: f.clone(),
                    parameter: parameter.into(),
                })
                .collect();
            let what = match parameter {
                "bore.through" => "THRU".to_string(),
                "grouping.count" => format!("{}×", g.members.len()),
                _ => fmt(value),
            };
            // A size claimed but misprinted cannot be credited; THRU or the quantity left off
            // the diameter's callout is simply not stated (the callout claims the diameter).
            let synthetic = carrier != parameter;
            let (code, message, evidence, annotation, view) = match claiming.first() {
                Some(m) if !synthetic => (
                    "hole_requirement_unverifiable",
                    format!(
                        "{}: requirement {parameter} is claimed by {} but its text does not state {what}",
                        g.noun(),
                        m.name
                    ),
                    Some(format!("{:?} does not state {what}", m.text)),
                    Some(m.name.clone()),
                    m.view.clone(),
                ),
                first => (
                    "hole_requirement_missing",
                    format!(
                        "{}: requirement {parameter} ({what}) is not stated on the sheet",
                        g.noun()
                    ),
                    first.map(|m| {
                        format!("the diameter's callout {:?} does not state {what}", m.text)
                    }),
                    first.map(|m| m.name.clone()),
                    first.and_then(|m| m.view.clone()),
                ),
            };
            issues.push(LintIssue {
                id: format!("l:{code}:{}/{parameter}", g.members[0]),
                code: code.into(),
                severity: Severity::Warning,
                message,
                at: None,
                view,
                subjects: g.members.clone(),
                measurements: g
                    .members
                    .iter()
                    .map(|f| format!("{f}/{carrier}"))
                    .collect(),
                requirements,
                sources: Vec::new(),
                stage: None,
                annotation,
                related: claiming.iter().skip(1).map(|m| m.name.clone()).collect(),
                evidence,
                fix: Some(format!(
                    "State {what} for {}: a hole callout reading \"{}\", in a view that sees the holes end-on.",
                    g.members.join(", "),
                    g.callout()
                )),
            });
        }
    }
    // A hole whose sizes cannot be read is reported, never left out of the count.
    for f in analysis.features.iter().filter(|f| f.kind == "hole") {
        if HoleSizes::of(f).is_none() {
            issues.push(LintIssue {
                id: format!("l:hole_requirement_unverifiable:{}/?", f.id),
                code: "hole_requirement_unverifiable".into(),
                severity: Severity::Warning,
                message: format!(
                    "hole {}: its sizes cannot be read from the analysis, so its requirements cannot be checked",
                    f.id
                ),
                at: None,
                view: None,
                subjects: vec![f.id.clone()],
                requirements: vec![Requirement {
                    feature: f.id.clone(),
                    parameter: "?".into(),
                }],
                measurements: Vec::new(),
                sources: Vec::new(),
                stage: None,
                annotation: None,
                related: Vec::new(),
                evidence: Some("the feature has no numeric diameter".into()),
                fix: None,
            });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::json;

    /// An Analysis of the holes *records* (quiddity's record fields, each with its `pattern`
    /// count if any), each on its own face, along the part's z.
    fn part(records: &[Value]) -> Analysis {
        let features: Vec<Value> = records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut parameters = r.clone();
                let pattern = parameters.as_object_mut().unwrap().remove("pattern");
                let location = r["location"].clone();
                json!({
                    "id": format!("f:hole:{}", i + 1), "kind": "hole", "faces": [i],
                    "parameters": parameters,
                    "fingerprint": {
                        "version": "test", "family": "holes", "subtype": "HoleRecord",
                        "sizes": {}, "axis": [0.0, 0.0, 1.0], "position": location,
                        "pattern": pattern.map(|n| json!({"count": n, "pitch": 10.0})),
                        "neighbourhood": [],
                    },
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

    fn hole(at: [f64; 3], down: bool) -> Value {
        json!({ "axis": [0.0, 0.0, if down { -1.0 } else { 1.0 }], "location": at,
                "diameter": 6.6, "depth": 10.0, "bottom": "through",
                "cbore": null, "spotface": null, "csink": null })
    }

    fn mark(text: &str, claims: &[(&str, &str)]) -> Mark {
        Mark {
            name: format!("m:{text}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: claims
                .iter()
                .map(|(f, p)| Claim {
                    feature: f.to_string(),
                    parameter: p.to_string(),
                })
                .collect(),
        }
    }

    fn found(analysis: &Analysis, marks: &[Mark]) -> Vec<String> {
        coverage(analysis, marks)
            .into_iter()
            .map(|i| {
                format!(
                    "{} {} {}",
                    i.code,
                    i.subjects.join("+"),
                    i.requirements[0].parameter
                )
            })
            .collect()
    }

    /// LINT-hole_coverage-opposed_bores_distinct: holes drilled from opposite faces are two
    /// requirements even when every size matches; identical holes drilled one way are one.
    #[test]
    fn holes_drilled_from_opposite_faces_are_separate_groups() {
        let a = part(&[
            hole([10.0, 10.0, 10.0], true),
            hole([20.0, 10.0, 10.0], true),
            hole([30.0, 10.0, 0.0], false),
        ]);
        let g = groups(&a);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].members, ["f:hole:1", "f:hole:2"]);
        assert_eq!(g[1].members, ["f:hole:3"]);
        // One callout for the pair leaves the opposed hole unstated.
        let pair = mark("2× ⌀6.6 THRU", &[("f:hole:1", "bore.diameter")]);
        assert_eq!(
            found(&a, &[pair]),
            [
                "hole_requirement_missing f:hole:3 bore.diameter",
                "hole_requirement_missing f:hole:3 bore.through"
            ]
        );
        // A callout claiming all three but printing "3×" states no group's quantity.
        let all = mark(
            "3× ⌀6.6 THRU",
            &[("f:hole:1", "bore.diameter"), ("f:hole:3", "bore.diameter")],
        );
        assert_eq!(
            found(&a, &[all]),
            ["hole_requirement_missing f:hole:1+f:hole:2 grouping.count"]
        );
    }

    #[test]
    fn a_hole_without_sizes_is_reported_not_dropped() {
        let mut odd = hole([10.0, 10.0, 10.0], true);
        odd["diameter"] = Value::Null;
        let a = part(&[odd]);
        assert!(groups(&a).is_empty());
        assert_eq!(found(&a, &[]), ["hole_requirement_unverifiable f:hole:1 ?"]);
    }

    /// A recognised pattern is one group; one whose members are not all present falls back to
    /// loose holes (LINT-hole_coverage-bolt_circle_corroboration's fallback).
    #[test]
    fn a_pattern_is_a_group_only_when_all_its_members_are_found() {
        let mut members: Vec<Value> = (0..4)
            .map(|i| {
                let mut h = hole([10.0 * f64::from(i), 0.0, 10.0], true);
                h["pattern"] = json!(4);
                h
            })
            .collect();
        members.push(hole([50.0, 50.0, 10.0], true));
        let g = groups(&part(&members));
        assert_eq!(g.len(), 2);
        assert!(g[0].pattern && g[0].members.len() == 4);
        assert!(!g[1].pattern && g[1].members == ["f:hole:5"]);
        members.truncate(3);
        let g = groups(&part(&members));
        assert_eq!(g.len(), 1);
        assert!(!g[0].pattern && g[0].members.len() == 3);
    }

    /// LINT-hole_coverage-blind_depth, -cbore_diameter_depth, -spotface_diameter_depth and
    /// -csink_diameter_angle: each recess's two sizes are requirements, stated by the callout.
    #[test]
    fn a_blind_counterbored_or_countersunk_hole_states_its_recess() {
        let mut blind = hole([10.0, 10.0, 10.0], true);
        blind["bottom"] = json!("flat");
        blind["cbore"] = json!({ "diameter": 11.0, "depth": 6.4 });
        let mut sunk = hole([40.0, 10.0, 10.0], true);
        sunk["csink"] = json!({ "major_diameter": 12.4, "included_angle": 90.0,
                                "drill_diameter": 6.6, "depth": 2.9 });
        let mut faced = hole([70.0, 10.0, 0.0], false);
        faced["spotface"] = json!({ "diameter": 14.0, "depth": 0.5 });
        let a = part(&[blind, sunk, faced]);
        let g = groups(&a);
        let names = |i: usize| g[i].requirements().iter().map(|r| r.0).collect::<Vec<_>>();
        assert_eq!(
            names(0),
            [
                "bore.diameter",
                "bore.depth",
                "counterbore.diameter",
                "counterbore.depth"
            ]
        );
        assert_eq!(
            names(1),
            [
                "bore.diameter",
                "bore.through",
                "countersink.diameter",
                "countersink.angle"
            ]
        );
        assert_eq!(
            names(2),
            [
                "bore.diameter",
                "bore.through",
                "spotface.diameter",
                "spotface.depth"
            ]
        );
        let claims = |f: &'static str, ps: &[&'static str]| -> Vec<(&'static str, &'static str)> {
            ps.iter().map(|p| (f, *p)).collect()
        };
        let stated = [
            mark(
                "⌀6.6 ↧ 10 ⌴ ⌀11 ↧ 6.4",
                &claims(
                    "f:hole:1",
                    &[
                        "bore.diameter",
                        "bore.depth",
                        "counterbore.diameter",
                        "counterbore.depth",
                    ],
                ),
            ),
            mark(
                "⌀6.6 THRU ⌵ ⌀12.4 × 90°",
                &claims(
                    "f:hole:2",
                    &["bore.diameter", "countersink.diameter", "countersink.angle"],
                ),
            ),
            mark(
                "⌀6.6 THRU SF ⌀14 ↧ 0.5",
                &claims(
                    "f:hole:3",
                    &["bore.diameter", "spotface.diameter", "spotface.depth"],
                ),
            ),
        ];
        assert_eq!(found(&a, &stated), Vec::<String>::new());
        // The fix states every size.
        assert_eq!(g[0].callout(), "⌀6.6 ↧ 10 ⌴ ⌀11 ↧ 6.4");
        assert_eq!(g[1].callout(), "⌀6.6 THRU ⌵ ⌀12.4 × 90°");
        // Leaving the countersink's angle off, and misprinting the counterbore's depth.
        let mut short = stated.clone();
        short[0].text = "⌀6.6 ↧ 10 ⌴ ⌀11 ↧ 6.9".into();
        short[1].text = "⌀6.6 THRU ⌵ ⌀12.4".into();
        short[1].claims.pop();
        assert_eq!(
            found(&a, &short),
            [
                "hole_requirement_unverifiable f:hole:1 counterbore.depth",
                "hole_requirement_missing f:hole:2 countersink.angle"
            ]
        );
    }

    #[test]
    fn a_number_is_stated_at_the_precision_printed() {
        assert!(states("⌀13.0 ↧ 5.7", 12.9898));
        assert!(states("⌀13.0 ↧ 5.7", 5.73));
        assert!(!states("⌀13.0 ↧ 5.7", 12.94));
        assert!(states("12× ⌀8 THRU", 8.0));
        assert!(states("12× ⌀8 THRU", 8.4));
        assert!(!states("12× ⌀8 THRU", 8.6));
        assert!(states("⌵ ⌀10 × 90°", 90.0));
    }

    #[test]
    fn thru_and_the_quantity_are_read_as_words() {
        assert!(says_thru("12× ⌀8 THRU EQ SP"));
        assert!(!says_thru("THRUST"));
        assert!(says_count("12× ⌀8 THRU", 12));
        assert!(says_count("4X ⌀5", 4));
        assert!(!says_count("12× ⌀8 THRU", 8));
        assert!(!says_count("1.5× ⌀8", 1));
    }
}
