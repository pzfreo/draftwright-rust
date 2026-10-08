//! Hole coverage (Python's `linting/hole_coverage.py`): every recognised hole, or group of
//! identical holes, states its sizes on the sheet.
//!
//! The requirements come from the Analysis's hole features alone, read as the dimensions read
//! them ([`crate::bridge::holes`]: one reader, one principal-axis rule, one size resolution):
//!
//! - **The groups** ([`groups`]): the members of a recognised hole pattern
//!   (`Analysis.recognition.patterns`) are one group when the pattern is established
//!   ([`holes::established_with`]: every member read, all identical, drilled along an axis of
//!   the part and on distinct axis lines, and a bolt circle of five or more, or of fewer with a
//!   hole or boss on its centre). A grid or linear pattern is one group however it is spaced,
//!   as in Python, whose lint takes every pattern its drafting did not refuse, and whose
//!   drafting refuses only those (`_append_hole_features`); the port's dimensions also decline a
//!   grid that is not a lattice, or a linear pattern not evenly spaced, in the view (and call its
//!   holes out loose), which Python's do not;
//!   every other hole joins the loose holes with the same machining requirement, signed drilling
//!   direction included (LINT-hole_coverage-opposed_bores_distinct). An Analysis without a
//!   recognition record falls back to the fingerprints' pattern count and pitch.
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
//! does not state it, or when the Analysis does not give it (a blind hole without a depth);
//! otherwise a requirement is **missing**, THRU and the quantity included when the diameter's
//! callout leaves them off (LINT-code-hole_requirement_state). Only those two are reported
//! (LINT-_parameter_coverage-report_uncovered_only). Lint cannot see the author's omissions or
//! placement drops, so the `suppressed` and `dropped` outcomes are not reached here.
//!
//! A size counts as printed when any number in the claiming callout rounds from it, so a callout
//! that misprints one size but prints another of the same value is not caught.
//!
//! Not yet ported: hole and pattern locations, pattern spacing, threads and profiled bores, the
//! assembly severity (the Analysis does not say whether the part is several bodies), and the
//! `suppressed` outcome.

use std::collections::BTreeMap;

use super::{LintIssue, Mark, Requirement, Severity};
use crate::bridge::holes::{self, Countersink, Hole, Recess};
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
    /// The bore's depth, when the Analysis gives it; for a through hole, the thickness it passes
    /// through.
    pub depth: Option<f64>,
    pub counterbore: Option<Recess>,
    pub spotface: Option<Recess>,
    pub countersink: Option<Countersink>,
}

impl HoleSizes {
    fn of(h: &Hole) -> HoleSizes {
        HoleSizes {
            diameter: h.diameter,
            through: h.through,
            depth: h.depth,
            counterbore: h.counterbore,
            spotface: h.spotface,
            countersink: h.countersink,
        }
    }
}

impl HoleGroup {
    /// The group's physical requirements, each with the value its text must state: none for
    /// `bore.through`, the group's size for `grouping.count`, and none for a size the Analysis
    /// does not give.
    pub fn requirements(&self) -> Vec<(&'static str, Option<f64>)> {
        let s = &self.sizes;
        let mut out = vec![("bore.diameter", Some(s.diameter))];
        if s.through {
            out.push(("bore.through", None));
        } else {
            out.push(("bore.depth", s.depth));
        }
        let recess = |r: Option<Recess>| r.map(|r| (r.diameter, r.depth));
        for (sizes, names) in [
            (
                recess(s.counterbore),
                ["counterbore.diameter", "counterbore.depth"],
            ),
            (recess(s.spotface), ["spotface.diameter", "spotface.depth"]),
            (
                s.countersink.map(|c| (c.diameter, c.angle)),
                ["countersink.diameter", "countersink.angle"],
            ),
        ] {
            if let Some((a, b)) = sizes {
                out.push((names[0], Some(a)));
                out.push((names[1], b));
            }
        }
        if self.members.len() > 1 {
            out.push(("grouping.count", Some(self.members.len() as f64)));
        }
        out
    }

    /// The callout that would state every size, as a fix suggests it (`?` for a size the
    /// Analysis does not give).
    pub fn callout(&self) -> String {
        let s = &self.sizes;
        let f = |v: Option<f64>| v.map_or("?".into(), fmt);
        let mut out = String::new();
        if self.members.len() > 1 {
            out += &format!("{}× ", self.members.len());
        }
        out += &format!("⌀{}", fmt(s.diameter));
        out += &if s.through {
            " THRU".to_string()
        } else {
            format!(" ↧ {}", f(s.depth))
        };
        if let Some(r) = s.counterbore {
            out += &format!(" ⌴ ⌀{} ↧ {}", fmt(r.diameter), f(r.depth));
        }
        if let Some(r) = s.spotface {
            out += &format!(" SF ⌀{} ↧ {}", fmt(r.diameter), f(r.depth));
        }
        if let Some(c) = s.countersink {
            out += &format!(" ⌵ ⌀{} × {}°", fmt(c.diameter), f(c.angle));
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
        let operation = match s.depth {
            _ if s.through => "THRU".to_string(),
            Some(d) => format!("blind depth {}", fmt(d)),
            None => "blind, depth not given".to_string(),
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

/// The Analysis's holes in groups that share a callout. A hole whose record cannot be read is
/// in no group ([`coverage`] reports it).
pub fn groups(analysis: &Analysis) -> Vec<HoleGroup> {
    let holes: Vec<Hole> = holes::read(analysis)
        .into_iter()
        .filter_map(|(_, h)| h.ok())
        .collect();
    // `holes` is in Analysis order, so sorted indices give the members in Analysis order.
    let group = |members: &[usize], pattern: bool| {
        let mut members = members.to_vec();
        members.sort_unstable();
        HoleGroup {
            members: members
                .iter()
                .map(|&i| holes[i].feature.id.clone())
                .collect(),
            pattern,
            sizes: HoleSizes::of(&holes[members[0]]),
            centres: members
                .iter()
                .map(|&i| holes[i].feature.fingerprint.position)
                .collect(),
            axis: holes[members[0]].feature.fingerprint.axis,
        }
    };
    let mut out = Vec::new();
    let mut taken = vec![false; holes.len()];
    match &analysis.recognition {
        // Patterns as quiddity recognised them: each established one is a group.
        Some(r) => {
            let refs: Vec<&Hole> = holes.iter().collect();
            let bosses = holes::bosses(analysis);
            for p in &r.patterns {
                if let Ok(members) = holes::established_with(p, &refs, &bosses)
                    && members.iter().all(|&m| !taken[m])
                {
                    out.push(group(&members, true));
                    members.iter().for_each(|&m| taken[m] = true);
                }
            }
        }
        // No recognition record (an Analysis assembled by hand): a pattern is read from the
        // fingerprints, accepted only when exactly its count of holes share its count and pitch,
        // the requirement and the opening's level along the drilling direction (to 1e-3 mm).
        None => {
            let mut patterns: BTreeMap<(usize, i64, Vec<i64>, i64), Vec<usize>> = BTreeMap::new();
            for (i, h) in holes.iter().enumerate() {
                if let Some(p) = &h.feature.fingerprint.pattern {
                    let level = (0..3).map(|k| h.direction[k] * h.at[k]).sum::<f64>();
                    let key = (
                        p.count,
                        holes::size_key(p.pitch),
                        h.requirement(),
                        (level * 1e3).round() as i64,
                    );
                    patterns.entry(key).or_default().push(i);
                }
            }
            for ((count, ..), members) in &patterns {
                if members.len() == *count && *count > 1 {
                    out.push(group(members, true));
                    members.iter().for_each(|&i| taken[i] = true);
                }
            }
        }
    }
    let mut loose: BTreeMap<Vec<i64>, Vec<usize>> = BTreeMap::new();
    for (i, h) in holes.iter().enumerate() {
        if !taken[i] {
            loose.entry(h.requirement()).or_default().push(i);
        }
    }
    for members in loose.values() {
        out.push(group(members, false));
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
            let synthetic = carrier != parameter;
            let claiming: Vec<&Mark> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.parameter == carrier && g.members.contains(&c.feature))
                })
                .collect();
            let requirements: Vec<Requirement> = g
                .members
                .iter()
                .map(|f| Requirement {
                    feature: f.clone(),
                    parameter: parameter.into(),
                })
                .collect();
            let measurements: Vec<String> =
                g.members.iter().map(|f| format!("{f}/{carrier}")).collect();
            // A size the Analysis does not give cannot be checked against any text (THRU has
            // no value to give).
            let value = match value {
                Some(v) => v,
                None if parameter == "bore.through" => f64::NAN,
                None => {
                    issues.push(LintIssue {
                    id: format!("l:hole_requirement_unverifiable:{}/{parameter}", g.members[0]),
                    code: "hole_requirement_unverifiable".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "{}: requirement {parameter} cannot be checked: the analysis does not give its value",
                        g.noun()
                    ),
                    at: None,
                    view: claiming.first().and_then(|m| m.view.clone()),
                    subjects: g.members.clone(),
                    requirements,
                    measurements,
                    sources: Vec::new(),
                    stage: None,
                    annotation: claiming.first().map(|m| m.name.clone()),
                    related: claiming.iter().skip(1).map(|m| m.name.clone()).collect(),
                    evidence: Some(format!("the hole record gives no {parameter}")),
                    fix: None,
                });
                    continue;
                }
            };
            let stated = |m: &&Mark| match parameter {
                "bore.through" => says_thru(&m.text),
                "grouping.count" => says_count(&m.text, g.members.len()),
                _ => states(&m.text, value),
            };
            if claiming.iter().any(stated) {
                continue;
            }
            let what = match parameter {
                "bore.through" => "THRU".to_string(),
                "grouping.count" => format!("{}×", g.members.len()),
                _ => fmt(value),
            };
            // A size claimed but misprinted cannot be credited; THRU or the quantity left off
            // the diameter's callout is simply not stated (the callout claims the diameter).
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
                measurements,
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
    // A hole whose record cannot be read is reported, never left out of the count.
    for (f, read) in holes::read(analysis) {
        if let Err(why) = read {
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
                evidence: Some(why),
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
    use serde_json::{Value, json};

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

    /// A blind hole whose record gives no depth has a depth requirement that cannot be checked:
    /// it is reported unverifiable, whatever the sheet prints, never compared with a made-up
    /// value. A through hole needs no depth.
    #[test]
    fn a_blind_hole_without_a_depth_is_unverifiable() {
        let mut blind = hole([10.0, 10.0, 10.0], true);
        blind["bottom"] = json!("flat");
        blind.as_object_mut().unwrap().remove("depth");
        let mut through = hole([40.0, 10.0, 10.0], true);
        through.as_object_mut().unwrap().remove("depth");
        through["diameter"] = json!(8.0);
        let a = part(&[blind, through]);
        let marks = [
            mark(
                "⌀6.6 ↧ 10",
                &[("f:hole:1", "bore.diameter"), ("f:hole:1", "bore.depth")],
            ),
            mark("⌀8 THRU", &[("f:hole:2", "bore.diameter")]),
        ];
        let issues = coverage(&a, &marks);
        assert_eq!(
            found(&a, &marks),
            ["hole_requirement_unverifiable f:hole:1 bore.depth"]
        );
        assert!(!issues[0].message.contains("NaN"), "{}", issues[0].message);
        assert_eq!(issues[0].annotation.as_deref(), Some("m:⌀6.6 ↧ 10"));
        assert_eq!(groups(&a)[0].callout(), "⌀6.6 ↧ ?");
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

    /// With a recognition record, the recognised patterns are the groups: a grid of identical
    /// holes is one, even these four in a row, which the port's dimensions decline as no lattice
    /// (Python's lint and drafting keep it); a bolt circle of fewer than five holes with nothing on its centre is not
    /// established (ISSUE-1596-a), so its holes join the loose holes of the same requirement, as
    /// the dimensions call them out; with a bore on its centre it is a pattern
    /// (LINT-hole_coverage-bolt_circle_corroboration).
    #[test]
    fn recognised_patterns_are_the_groups() {
        let mut a = part(
            &(0..9)
                .map(|i| hole([10.0 * f64::from(i), 0.0, 10.0], true))
                .collect::<Vec<_>>(),
        );
        let ids = |r: std::ops::Range<usize>| -> Vec<String> {
            r.map(|i| format!("f:hole:{}", i + 1)).collect()
        };
        let pattern = |n: usize, arrangement: &str, members: Vec<String>| {
            json!({ "id": format!("f:hole_pattern:{n}"), "arrangement": arrangement,
                    "members": members, "faces": [], "parameters": {} })
        };
        a.recognition = Some(
            serde_json::from_value(json!({
                "recogniser": "test",
                "frame": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                "patterns": [pattern(1, "grid", ids(0..4)), pattern(2, "bolt_circle", ids(4..8))],
            }))
            .unwrap(),
        );
        let g = groups(&a);
        assert_eq!(
            g.iter()
                .map(|g| (g.members.clone(), g.pattern))
                .collect::<Vec<_>>(),
            [(ids(0..4), true), (ids(4..9), false)]
        );
        // Four holes on a ø40 circle about (50, 50) with a ø20 bore on its centre: the circle
        // is corroborated, and its four holes are one group, the bore another.
        let mut circle: Vec<Value> = [[70.0, 50.0], [50.0, 70.0], [30.0, 50.0], [50.0, 30.0]]
            .iter()
            .map(|c| hole([c[0], c[1], 10.0], true))
            .collect();
        let mut bore = hole([50.0, 50.0, 10.0], true);
        bore["diameter"] = json!(20.0);
        circle.push(bore);
        let mut a = part(&circle);
        let bolt_circle = |members: Vec<String>| {
            json!({ "id": "f:hole_pattern:1", "arrangement": "bolt_circle", "members": members,
                    "faces": [], "parameters": { "center": [50.0, 50.0, 10.0], "diameter": 40.0 } })
        };
        let recognition = |members: Vec<String>| {
            serde_json::from_value(json!({
                "recogniser": "test",
                "frame": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                "patterns": [bolt_circle(members)],
            }))
            .unwrap()
        };
        a.recognition = Some(recognition(ids(0..4)));
        let g = groups(&a);
        assert_eq!(
            g.iter()
                .map(|g| (g.members.clone(), g.pattern))
                .collect::<Vec<_>>(),
            [(ids(0..4), true), (ids(4..5), false)]
        );
        // Without the bore, the four are loose holes.
        let mut a = part(&circle[..4]);
        a.recognition = Some(recognition(ids(0..4)));
        assert!(!groups(&a)[0].pattern);
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
