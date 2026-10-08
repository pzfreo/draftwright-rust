//! Hole callouts, in draftwright's compound grammar (`4× ⌀10 THRU ⌴ ⌀16 ↧ 5 (2×2)`): identical
//! holes grouped and counted, quiddity's hole patterns called out as patterns with the pitches
//! that space them, and every hole or pattern located from the part's box in the view that looks
//! along it.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::turned::Turned;
use super::{
    AXES, Context, Measurement, NAMES, P3, Planner, Problem, Shape, Want, end_on, fmt, key, point,
    want_id,
};
use crate::bridge::{HolePattern, holes};

pub(super) const PLANNER: Planner = Planner {
    kinds: &["hole"],
    wants,
};

/// Member sites projected into a view, as integer keys.
type Sites = Vec<(i64, i64)>;

/// A recognised hole the callout grammar can state: read by [`holes::Hole::read`], along a
/// principal axis, with its depth when it is blind.
#[derive(Clone, Debug)]
struct Hole<'a> {
    read: holes::Hole<'a>,
    /// The world axis it lies along.
    axis: usize,
}

impl<'a> std::ops::Deref for Hole<'a> {
    type Target = holes::Hole<'a>;
    fn deref(&self) -> &holes::Hole<'a> {
        &self.read
    }
}

impl Hole<'_> {
    /// The recesses the grammar prints, each its role and sizes: the counterbore, then the
    /// spotface, in draftwright's segment order (`callout.py`'s `_CALLOUT_SEGMENTS`).
    ///
    /// draftwright prints one recess slot, the counterbore shadowing the spotface (`_recess`,
    /// `_shadowed`), so a hole quiddity finds both on (`holes.py`'s `_near_side_steps`: a shallow
    /// step is a spotface, a deep one a counterbore) loses its spotface on the sheet while its
    /// own lint still requires it (`_physical_requirement_count`). The port prints both, each a
    /// `⌴` segment as the spotface is printed alone (whether a spotface reads `⌴` or `SF` is a
    /// maintainer question): rust-correct, since the spotface is a machining requirement the
    /// callout would otherwise leave unstated.
    fn recesses(&self) -> impl Iterator<Item = (&'static str, holes::Recess)> {
        [
            ("counterbore", self.counterbore),
            ("spotface", self.spotface),
        ]
        .into_iter()
        .filter_map(|(role, r)| r.map(|r| (role, r)))
    }

    /// The terms after the count: `⌀d`, `THRU` or `↧ depth`, the recesses, the countersink.
    fn terms(&self) -> Vec<String> {
        let mut t = vec![format!("⌀{}", fmt(self.diameter))];
        match self.depth {
            Some(depth) if !self.through => t.extend(["↧".into(), fmt(depth)]),
            _ => t.push("THRU".into()),
        }
        for (_, r) in self.recesses() {
            t.extend(["⌴".into(), format!("⌀{}", fmt(r.diameter))]);
            if let Some(depth) = r.depth {
                t.extend(["↧".into(), fmt(depth)]);
            }
        }
        if let Some(c) = self.countersink {
            t.extend(["⌵".into(), format!("⌀{}", fmt(c.diameter))]);
            if let Some(a) = c.angle {
                t.extend(["×".into(), format!("{}°", fmt(a))]);
            }
        }
        t
    }

    fn measurements(&self) -> Vec<Measurement> {
        let id = &self.feature.id;
        let m = |p: &str, v: f64| Measurement {
            feature: id.clone(),
            parameter: p.into(),
            value: v,
        };
        let mut out = vec![m("bore.diameter", self.diameter)];
        if let Some(depth) = self.depth.filter(|_| !self.through) {
            out.push(m("bore.depth", depth));
        }
        for (role, r) in self.recesses() {
            out.push(m(&format!("{role}.diameter"), r.diameter));
            if let Some(depth) = r.depth {
                out.push(m(&format!("{role}.depth"), depth));
            }
        }
        if let Some(c) = self.countersink {
            out.push(m("countersink.diameter", c.diameter));
            if let Some(a) = c.angle {
                out.push(m("countersink.angle", a));
            }
        }
        out
    }
}

/// A hole as the callout grammar reads it, or why it cannot state it.
fn hole<'a>(read: Result<holes::Hole<'a>, String>) -> Result<Hole<'a>, String> {
    let read = read?;
    let (axis, _) = read
        .principal
        .ok_or("its axis is not along x, y or z, so no standard view looks along it")?;
    if !read.through && read.depth.is_none() {
        return Err("it is blind and has no depth".into());
    }
    Ok(Hole { read, axis })
}

/// The holes' callouts, the pitches of their patterns and their locations, with the problems met
/// on the way.
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let analysis = cx.analysis;
    let frame = &cx.frame;
    let turned = &cx.turned;
    let [lo, _] = analysis.envelope;
    let mut out = Vec::new();
    let mut problems = Vec::new();

    let mut holes: Vec<Hole> = Vec::new();
    for (f, read) in holes::read(analysis) {
        match hole(read) {
            Ok(h) => {
                let bottom = h.bottom;
                if !matches!(bottom, Some("through" | "flat" | "drill_point")) {
                    // Called out blind to its recognised depth, as draftwright does, but the
                    // recogniser did not establish that it stops there.
                    problems.push(Problem {
                        id: format!("p:hole_bottom_unknown:{}", f.id),
                        code: "hole_bottom_unknown".into(),
                        severity: "info".into(),
                        message: format!(
                            "{} is called out blind to its recognised depth ({}), but recognition did not establish its bottom ({}); check whether it is through.",
                            f.id,
                            fmt(h.depth.expect("a blind hole has its depth")),
                            bottom.unwrap_or("none given")
                        ),
                        subjects: vec![f.id.clone()],
                    });
                }
                holes.push(h);
            }
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!("{} is not dimensioned: {why}.", f.id),
                subjects: vec![f.id.clone()],
            }),
        }
    }
    let index: BTreeMap<&str, usize> = holes
        .iter()
        .enumerate()
        .map(|(i, h)| (h.feature.id.as_str(), i))
        .collect();
    let patterns: &[HolePattern] = analysis
        .recognition
        .as_ref()
        .map_or(&[], |r| r.patterns.as_slice());

    // Patterns first: each that holds together takes its members.
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    let mut callouts: Vec<Want> = Vec::new();
    let mut located: Vec<(String, P3, usize, String)> = Vec::new(); // owner, point, view axis…
    let mut pitches: Vec<Want> = Vec::new();
    let mut declined: BTreeMap<usize, String> = BTreeMap::new();
    let mut coincident: Vec<(usize, P3, Sites)> = Vec::new(); // callout index, station
    let reads: Vec<&holes::Hole> = holes.iter().map(|h| &h.read).collect();
    let bosses = holes::bosses(analysis);
    for p in patterns {
        let members = match holes::established_with(p, &reads, &bosses) {
            Ok(members) => members,
            Err(why) => {
                for m in &p.members {
                    if let Some(&i) = index.get(m.as_str()) {
                        declined.entry(i).or_insert_with(|| why.clone());
                    }
                }
                continue;
            }
        };
        if members.iter().any(|m| taken.contains(m)) {
            continue;
        }
        let first = &holes[members[0]];
        let k = first.axis;
        let view = end_on(k);
        let (u, v) = in_view(k);
        let centres: Vec<P3> = members.iter().map(|&m| holes[m].at).collect();
        let n = members.len();
        let (suffix, bcd, why): (Option<String>, Option<f64>, String) = match p.arrangement.as_str()
        {
            "bolt_circle" => {
                let d = p.parameters.get("diameter").and_then(Value::as_f64);
                let c = point(p.parameters.get("center")).map(|c| frame.point(c));
                match (d, c) {
                    (Some(d), Some(c)) => {
                        located_pattern(&mut located, &p.id, c, k, turned.as_ref());
                        (
                            Some(format!("EQ SP ON ø{} BC", fmt(d))),
                            Some(d),
                            if n >= 5 {
                                "a bolt circle of five or more holes is self-evident (ISSUE-1596-a): it is called out with its circle and located by its centre".to_string()
                            } else {
                                "a bolt circle of fewer than five holes is corroborated by a hole or boss on its centre (ISSUE-1596-a): it is called out with its circle and located by its centre".to_string()
                            },
                        )
                    }
                    _ => {
                        for &m in &members {
                            declined.insert(m, format!("quiddity's bolt circle {} gives no diameter or centre, so its holes are dimensioned singly", p.id));
                        }
                        continue;
                    }
                }
            }
            "grid" => match lattice(&centres, u, v) {
                Some((cols, rows)) if cols.len() * rows.len() == n => {
                    let centre = {
                        let mut c = centres[0];
                        c[u] = 0.5 * (cols[0] + cols[cols.len() - 1]);
                        c[v] = 0.5 * (rows[0] + rows[rows.len() - 1]);
                        c
                    };
                    located_pattern(&mut located, &p.id, centre, k, turned.as_ref());
                    for (along, values, name) in [(u, &cols, "col"), (v, &rows, "row")] {
                        if values.len() > 1 {
                            pitches.push(pitch_want(
                                &p.id,
                                &format!("grid_pitch.length.{name}"),
                                view,
                                centres[0],
                                along,
                                values,
                            ));
                        }
                    }
                    (
                        Some(format!("({}×{})", rows.len(), cols.len())),
                        None,
                        format!(
                            "a grid of {}×{} identical holes is called out once with its size, located by its centre and spaced by its pitches",
                            rows.len(),
                            cols.len()
                        ),
                    )
                }
                _ => {
                    for &m in &members {
                        declined.insert(m, format!("quiddity's grid {} is not a square lattice in the {view} view, so its holes are dimensioned singly", p.id));
                    }
                    continue;
                }
            },
            _ => match line(&centres, u, v) {
                Some((along, values)) => {
                    let mut start = centres[0];
                    start[along] = values[0];
                    if let Some(c) = centres.iter().find(|c| (c[along] - values[0]).abs() < 1e-4) {
                        start = *c;
                    }
                    located_pattern(&mut located, &p.id, start, k, turned.as_ref());
                    pitches.push(pitch_want(
                        &p.id,
                        "pitch.length",
                        view,
                        start,
                        along,
                        &values,
                    ));
                    (
                        None,
                        None,
                        "a linear pattern is called out once, located by its first member and spaced by its pitch".to_string(),
                    )
                }
                None => {
                    for &m in &members {
                        declined.insert(m, format!("quiddity's linear pattern {} is not evenly spaced along one axis of the {view} view, so its holes are dimensioned singly", p.id));
                    }
                    continue;
                }
            },
        };
        taken.extend(members.iter().copied());
        let mut terms = vec![format!("{n}×")];
        terms.extend(first.terms());
        let mut measurements: Vec<Measurement> = members
            .iter()
            .flat_map(|&m| holes[m].measurements())
            .collect();
        terms.extend(suffix);
        if let Some(d) = bcd {
            measurements.push(Measurement {
                feature: p.id.clone(),
                parameter: "bolt_circle.diameter".into(),
                value: d,
            });
        }
        let sites: Vec<(P3, f64)> = members
            .iter()
            .map(|&m| (holes[m].at, 0.5 * holes[m].diameter))
            .collect();
        let mut projected: Vec<(i64, i64)> =
            sites.iter().map(|(c, _)| (key(c[u]), key(c[v]))).collect();
        projected.sort();
        if k == 2 {
            coincident.push((callouts.len(), first.at, projected));
        }
        callouts.push(Want {
            id: want_id(&p.id, "bore.diameter", view),
            views: vec![view],
            value: first.diameter,
            terms,
            measurements,
            turning: None,
            basis: format!("{why}; called out where it is seen end-on"),
            shape: Shape::Callout { sites },
        });
    }

    // Draftwright names the axial face of z patterns whose members coincide in the plan.
    let mut by_sites: BTreeMap<&Sites, Vec<(usize, f64)>> = BTreeMap::new();
    for (i, c, sites) in &coincident {
        by_sites.entry(sites).or_default().push((*i, c[2]));
    }
    for group in by_sites.values().filter(|g| g.len() > 1) {
        let mut stations: Vec<f64> = group.iter().map(|g| g.1).collect();
        stations.sort_by(f64::total_cmp);
        stations.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        if stations.len() < 2 {
            continue;
        }
        for (i, z) in group {
            let n = stations
                .iter()
                .position(|s| (s - z).abs() < 1e-6)
                .unwrap_or(0);
            let which = if stations.len() == 2 {
                ["LOWER FACE", "UPPER FACE"][n].to_string()
            } else {
                format!("FACE {} OF {} FROM LOWER END", n + 1, stations.len())
            };
            let w = &mut callouts[*i];
            match w.terms.last_mut() {
                Some(last) if last.starts_with("EQ SP") || last.starts_with('(') => {
                    *last = format!("{last} {which}")
                }
                _ => w.terms.push(which),
            }
            w.basis += "; its members project onto another pattern's, so the face it is drilled from is named";
        }
    }

    // The holes left: identical ones grouped and counted (ARCH0008-2).
    let mut groups: Vec<(Vec<i64>, Vec<usize>)> = Vec::new();
    for (i, h) in holes.iter().enumerate() {
        if taken.contains(&i) {
            continue;
        }
        let req = h.requirement();
        match groups.iter_mut().find(|g| g.0 == req) {
            Some(g) => g.1.push(i),
            None => groups.push((req, vec![i])),
        }
    }
    for (_, members) in &groups {
        let first = &holes[members[0]];
        let view = end_on(first.axis);
        let mut terms = Vec::new();
        if members.len() > 1 {
            terms.push(format!("{}×", members.len()));
        }
        terms.extend(first.terms());
        let note = members
            .iter()
            .find_map(|m| declined.get(m))
            .map_or(String::new(), |d| format!("; {d}"));
        callouts.push(Want {
            id: want_id(&first.feature.id, "bore.diameter", view),
            views: vec![view],
            value: first.diameter,
            terms,
            measurements: members.iter().flat_map(|&m| holes[m].measurements()).collect(),
            turning: None,
            basis: if members.len() > 1 {
                format!("identical holes drilled from one face are called out once and counted, where they are seen end-on{note}")
            } else {
                format!("a hole's size is called out where it is seen end-on{note}")
            },
            shape: Shape::Callout {
                sites: members
                    .iter()
                    .map(|&m| (holes[m].at, 0.5 * holes[m].diameter))
                    .collect(),
            },
        });
        for &m in members {
            let h = &holes[m];
            let (u, v) = in_view(h.axis);
            for k in [u, v] {
                if let Some(t) = &turned
                    && k != t.axis
                    && (h.at[k] - t.on[k]).abs() < 1e-3
                {
                    continue; // on the turning axis: the centre line locates it
                }
                located.push((h.feature.id.clone(), h.at, k, view.into()));
            }
        }
    }
    out.extend(callouts);
    out.extend(pitches);

    // Locations from the part's box: one dimension per distinct distance (ARCH0009-6).
    let mut by: BTreeMap<(String, usize, i64), Vec<(String, P3)>> = BTreeMap::new();
    for (owner, p, k, view) in located {
        let d = p[k] - lo[k];
        if d.abs() < 1e-6 {
            problems.push(Problem {
                id: format!("p:location_on_datum:{owner}/{}", AXES[k]),
                code: "location_on_datum".into(),
                severity: "info".into(),
                message: format!(
                    "{owner} lies on the part's {} face, so its {} location measures nothing and is not dimensioned.",
                    ["left", "front", "bottom"][k],
                    AXES[k]
                ),
                subjects: vec![owner],
            });
            continue;
        }
        by.entry((view, k, key(d))).or_default().push((owner, p));
    }
    let rank = |id: &str| {
        analysis
            .features
            .iter()
            .position(|f| f.id == id)
            .unwrap_or(usize::MAX)
    };
    for ((view, k, _), mut owners) in by {
        owners.sort_by_key(|(o, _)| (rank(o), o.clone()));
        owners.dedup_by(|a, b| a.0 == b.0);
        let (first, p) = &owners[0];
        let d = p[k] - lo[k];
        let parameter = format!("location.length.{}", AXES[k]);
        let view: &'static str = NAMES.iter().find(|n| **n == view).expect("a view");
        out.push(Want {
            id: want_id(first, &parameter, view),
            views: vec![view],
            value: d,
            terms: vec![fmt(d)],
            measurements: owners
                .iter()
                .map(|(o, _)| Measurement {
                    feature: o.clone(),
                    parameter: parameter.clone(),
                    value: d,
                })
                .collect(),
            turning: None,
            basis: format!(
                "located in {} from the part's {} face, in the view that looks along the hole{}",
                AXES[k],
                ["left", "front", "bottom"][k],
                if owners.len() > 1 {
                    "; one dimension serves every feature at this distance"
                } else {
                    ""
                }
            ),
            shape: Shape::Linear {
                a: {
                    let mut a = *p;
                    a[k] = lo[k];
                    a
                },
                b: *p,
                axis: k,
                a_edge: true,
                b_edge: false,
                order: 1,
            },
        });
    }
    (out, problems)
}

/// The world axes a view looking along *k* shows across and up the page.
fn in_view(k: usize) -> (usize, usize) {
    match k {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

fn distinct(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut v: Vec<f64> = values.collect();
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    v
}

fn even(values: &[f64]) -> bool {
    values.len() < 3
        || values
            .windows(2)
            .all(|w| ((w[1] - w[0]) - (values[1] - values[0])).abs() < 1e-4)
}

/// A grid's distinct column and row positions, when its members are an evenly spaced lattice
/// along the view's axes.
fn lattice(centres: &[P3], u: usize, v: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    let cols = distinct(centres.iter().map(|c| c[u]));
    let rows = distinct(centres.iter().map(|c| c[v]));
    (even(&cols) && even(&rows)).then_some((cols, rows))
}

/// A linear pattern's axis and its members' positions along it, when they lie evenly along one
/// of the view's axes.
fn line(centres: &[P3], u: usize, v: usize) -> Option<(usize, Vec<f64>)> {
    for (along, across) in [(u, v), (v, u)] {
        let a = distinct(centres.iter().map(|c| c[along]));
        let b = distinct(centres.iter().map(|c| c[across]));
        if b.len() == 1 && a.len() == centres.len() && even(&a) {
            return Some((along, a));
        }
    }
    None
}

fn located_pattern(
    located: &mut Vec<(String, P3, usize, String)>,
    id: &str,
    centre: P3,
    k: usize,
    turned: Option<&Turned>,
) {
    let (u, v) = in_view(k);
    for a in [u, v] {
        if let Some(t) = turned
            && a != t.axis
            && (centre[a] - t.on[a]).abs() < 1e-3
        {
            continue;
        }
        located.push((id.into(), centre, a, end_on(k).into()));
    }
}

fn pitch_want(
    pattern: &str,
    parameter: &str,
    view: &'static str,
    at: P3,
    along: usize,
    values: &[f64],
) -> Want {
    let p = values[1] - values[0];
    let intervals = values.len() - 1;
    let mut terms = Vec::new();
    let text = if intervals > 1 {
        format!("{intervals}× {}", fmt(p))
    } else {
        fmt(p)
    };
    terms.push(text);
    let (mut a, mut b) = (at, at);
    a[along] = values[0];
    b[along] = values[1];
    Want {
        id: want_id(pattern, parameter, view),
        views: vec![view],
        value: p,
        terms,
        measurements: vec![Measurement {
            feature: pattern.into(),
            parameter: parameter.into(),
            value: p,
        }],
        turning: None,
        basis: format!(
            "the pattern's spacing in {}, between its first two members, counted across its {intervals} interval{}",
            AXES[along],
            if intervals == 1 { "" } else { "s" }
        ),
        shape: Shape::Linear {
            a,
            b,
            axis: along,
            a_edge: false,
            b_edge: false,
            order: 0,
        },
    }
}
