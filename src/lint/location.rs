//! Rooted physical station graphs. Both precedence and the drawn-set audit use this walk.
use super::{LintIssue, Mark, uncovered};
use crate::model::Analysis;
use crate::pmi::{Category, Reading};
use std::collections::BTreeSet;

const EPS: f64 = 1e-6;
pub(crate) const AXES: [&str; 3] = ["x", "y", "z"];
#[derive(Clone, Debug)]
pub(crate) struct Edge {
    pub axis: usize,
    pub a: f64,
    pub b: f64,
    pub owner: String,
    pub a_feature: Option<String>,
    pub b_feature: Option<String>,
}

/// The shortest chain to any root. Parallel authored edges remain separate edges.
pub(crate) fn route(
    edges: &[Edge],
    roots: &[f64],
    axis: usize,
    at: f64,
    feature: Option<&str>,
    excluded: &BTreeSet<usize>,
) -> Option<Vec<usize>> {
    let mut queue = vec![(at, feature.map(str::to_string), Vec::new())];
    let mut seen = vec![(at, feature.map(str::to_string))];
    let mut cursor = 0;
    while cursor < queue.len() {
        let (p, node, path) = queue[cursor].clone();
        cursor += 1;
        if node.is_none() && roots.iter().any(|r| (r - p).abs() < EPS) {
            return Some(path);
        }
        for (i, e) in edges.iter().enumerate() {
            if e.axis != axis || excluded.contains(&i) {
                continue;
            }
            let (q, next_node) = if (p - e.a).abs() < EPS && node == e.a_feature {
                (e.b, e.b_feature.clone())
            } else if (p - e.b).abs() < EPS && node == e.b_feature {
                (e.a, e.a_feature.clone())
            } else {
                continue;
            };
            if seen
                .iter()
                .any(|(v, n)| (q - v).abs() < EPS && *n == next_node)
            {
                continue;
            }
            seen.push((q, next_node.clone()));
            let mut next = path.clone();
            next.push(i);
            queue.push((q, next_node, next));
        }
    }
    None
}

pub(crate) fn roots(a: &Analysis) -> [Vec<f64>; 3] {
    let mut roots = std::array::from_fn(|k| vec![a.envelope[0][k], a.envelope[1][k]]);
    if let Some(Reading::Read(r)) = &a.pmi {
        for record in &r.records {
            if record.category == Category::Datum
                && let Some(g) = &record.geometry
                && g.surface.as_deref() == Some("plane")
                && let Some(k) = ["X", "Y", "Z"]
                    .iter()
                    .position(|v| Some(*v) == g.axis.as_deref())
                && let Some(p) = g.sites.first()
            {
                roots[k].push(p[k]);
            }
        }
    }
    roots
}

/// Explicit drawing origins are references whose stations can be re-measured on Analysis.
pub(crate) fn drawing_roots(a: &Analysis, marks: &[Mark]) -> [Vec<f64>; 3] {
    let mut out = roots(a);
    for c in marks.iter().flat_map(|m| &m.claims) {
        if let Some(word) = c.feature.strip_prefix("station_origin:")
            && let Some((axis, reference)) = word.split_once('|')
            && let Ok(k) = axis.parse::<usize>()
            && k < 3
        {
            let station = if let Some(f) = reference
                .strip_prefix("face:")
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| a.face_attachments.get(i))
            {
                if f.surface.as_deref() == Some("plane") && f.axis == Some(k) {
                    f.sites.first().map(|p| p[k])
                } else {
                    None
                }
            } else {
                a.scheme_station(reference, k).ok().map(|p| p[k])
            };
            if let Some(v) = station {
                out[k].push(v);
            }
        }
    }
    out
}

/// Physical spans retained as drawing claims. The audit validates their nominal against ink;
/// these are station constraints, not inferred coverage from a dimension's prose.
pub(crate) fn span_claim(axis: usize, a: f64, b: f64) -> String {
    format!("station_span:{axis}|{a}|{b}")
}

pub(crate) fn edges(a: &Analysis, marks: &[Mark]) -> Vec<Edge> {
    let mut out = Vec::new();
    let report = match &a.pmi {
        Some(Reading::Read(r)) => Some((r, r.identities())),
        _ => None,
    };
    for mark in marks.iter().filter(|m| !m.text.starts_with('(')) {
        let mut spans = Vec::new();
        let physical = mark
            .claims
            .iter()
            .any(|c| c.feature.starts_with("station_span:") || c.feature.starts_with("pmi:"));
        for c in &mark.claims {
            if let Some(word) = c.feature.strip_prefix("station_span:") {
                let words: Vec<_> = word.split('|').collect();
                if let [k, p, q] = words.as_slice()
                    && let (Ok(k), Ok(p), Ok(q)) =
                        (k.parse::<usize>(), p.parse::<f64>(), q.parse::<f64>())
                    && k < 3
                    && p.is_finite()
                    && q.is_finite()
                    && (super::holes::states(&mark.text, (q - p).abs())
                        || mark.text.split_once('×').is_some_and(|(n, rest)| {
                            n.trim()
                                .parse::<f64>()
                                .is_ok_and(|n| super::holes::states(rest.trim(), (q - p).abs() / n))
                        }))
                {
                    spans.push((k, p, q));
                }
            } else if let Some((r, ids)) = &report
                && c.feature.starts_with("pmi:")
            {
                for record in r
                    .records
                    .iter()
                    .filter(|r| ids.id(&r.source_id).ok() == Some(c.feature.as_str()))
                {
                    if let Some(g) = &record.geometry
                        && g.stations.len() == 2
                        && let Some(k) = ["X", "Y", "Z"]
                            .iter()
                            .position(|v| Some(*v) == g.axis.as_deref())
                        && super::holes::states(
                            &mark.text,
                            (g.stations[1][k] - g.stations[0][k]).abs(),
                        )
                    {
                        spans.push((k, g.stations[0][k], g.stations[1][k]));
                    }
                }
            } else if !physical {
                spans.extend(
                    super::schemes::supports(a, &c.feature, &c.parameter, &mark.text)
                        .into_iter()
                        .map(|s| (s.axis, s.lo, s.hi)),
                );
            }
        }
        spans.sort_by(|x, y| x.partial_cmp(y).unwrap());
        spans.dedup_by(|x, y| x.0 == y.0 && (x.1 - y.1).abs() < EPS && (x.2 - y.2).abs() < EPS);
        out.extend(
            spans
                .into_iter()
                .filter(|(_, p, q)| (p - q).abs() > EPS)
                .map(|(axis, p, q)| Edge {
                    axis,
                    a: p,
                    b: q,
                    owner: mark.name.clone(),
                    a_feature: None,
                    b_feature: None,
                }),
        );
    }
    let mut typed = Vec::new();
    for mark in marks {
        let spans = out
            .iter()
            .filter(|e| e.owner == mark.name)
            .cloned()
            .collect();
        typed.extend(identify(a, mark, spans));
    }
    typed
}

/// Feature axes remain distinct even when they share a coordinate in a projection.
pub(crate) fn node_feature(a: &Analysis, id: &str, k: usize) -> Option<String> {
    if let Some(p) = pattern(a, id)
        && p.members.iter().all(|id| node_feature(a, id, k).is_some())
    {
        return Some(id.into());
    }
    a.features
        .iter()
        .find(|f| {
            f.id == id
                && matches!(
                    f.kind.as_str(),
                    "hole" | "boss" | "polygonal_boss" | "oriented_slot"
                )
                && f.fingerprint.axis.is_some_and(|d| d[k].abs() < EPS)
        })
        .map(|f| f.id.clone())
}

pub(crate) fn pattern<'a>(a: &'a Analysis, id: &str) -> Option<&'a crate::bridge::HolePattern> {
    a.recognition.as_ref()?.patterns.iter().find(|p| p.id == id)
}

pub(crate) fn station(a: &Analysis, id: &str, k: usize) -> Option<f64> {
    if let Some(p) = pattern(a, id) {
        let points: Option<Vec<_>> = p.members.iter().map(|id| station(a, id, k)).collect();
        let points = points?;
        return (!points.is_empty()).then(|| points.iter().sum::<f64>() / points.len() as f64);
    }
    if let Some(f) = a.features.iter().find(|f| f.id == id) {
        if let Some(v) = a.axis_station(id, k) {
            return Some(v);
        }
        return Some(f.fingerprint.position[k]);
    }
    a.scheme_station(id, k).ok().map(|p| p[k])
}

/// A completely located pattern also fixes its centre, without an extra centre ordinate.
pub(crate) fn proof(
    a: &Analysis,
    edges: &[Edge],
    roots: &[f64],
    axis: usize,
    at: f64,
    node: Option<&str>,
) -> Option<Vec<usize>> {
    if let Some(path) = route(edges, roots, axis, at, node, &BTreeSet::new()) {
        return Some(path);
    }
    let p = pattern(a, node?)?;
    let mut path = Vec::new();
    for id in &p.members {
        path.extend(route(
            edges,
            roots,
            axis,
            station(a, id, axis)?,
            Some(id),
            &BTreeSet::new(),
        )?);
    }
    Some(path)
}

pub(crate) fn identify(a: &Analysis, mark: &Mark, spans: Vec<Edge>) -> Vec<Edge> {
    let report = match &a.pmi {
        Some(Reading::Read(r)) => Some((r, r.identities())),
        _ => None,
    };
    let mut out = Vec::new();
    for span in spans {
        let source = report.as_ref().and_then(|(report, ids)| {
            report.records.iter().find(|r| {
                r.category == Category::Dimension
                    && ids
                        .id(&r.source_id)
                        .ok()
                        .is_some_and(|id| mark.claims.iter().any(|c| c.feature == id))
            })
        });
        if let Some(r) = source {
            let mut e = span;
            let feature = |side: usize, at: f64| {
                let candidates: Vec<_> = a
                    .features
                    .iter()
                    .filter(|f| {
                        f.kind != "hole_pattern"
                            && node_feature(a, &f.id, e.axis).is_some()
                            && r.faces.iter().any(|i| f.faces.contains(i))
                            && station(a, &f.id, e.axis).is_some_and(|p| (p - at).abs() < EPS)
                    })
                    .collect();
                if candidates.len() == 1 {
                    return Some(candidates[0].id.clone());
                }
                r.face_groups
                    .get(side)
                    .and_then(|faces| {
                        candidates
                            .iter()
                            .find(|f| faces.iter().any(|i| f.faces.contains(i)))
                    })
                    .map(|f| f.id.clone())
            };
            e.a_feature = feature(0, e.a);
            e.b_feature = feature(1, e.b);
            out.push(e);
            continue;
        }
        let pitch = mark
            .claims
            .iter()
            .find(|c| c.parameter.contains("pitch.length"));
        if let Some(c) = pitch
            && let Some(p) = pattern(a, &c.feature)
        {
            let centre = station(a, &p.id, span.axis);
            if let Some(centre) = centre {
                for id in &p.members {
                    if let Some(at) = station(a, id, span.axis) {
                        let mut e = span.clone();
                        e.a = centre;
                        e.b = at;
                        e.a_feature = Some(p.id.clone());
                        e.b_feature = Some(id.clone());
                        out.push(e);
                    }
                }
                // A line of holes has one common transverse station on its other axis.
                for k in 0..3 {
                    if k == span.axis || node_feature(a, &p.id, k).is_none() {
                        continue;
                    }
                    let Some(c) = station(a, &p.id, k) else {
                        continue;
                    };
                    if p.members
                        .iter()
                        .all(|id| station(a, id, k).is_some_and(|v| (v - c).abs() < EPS))
                    {
                        for id in &p.members {
                            let mut e = span.clone();
                            e.axis = k;
                            e.a = c;
                            e.b = c;
                            e.a_feature = Some(p.id.clone());
                            e.b_feature = Some(id.clone());
                            out.push(e);
                        }
                    }
                }
                continue;
            }
        }
        let location = mark
            .claims
            .iter()
            .any(|c| c.parameter.starts_with("location"));
        let mut members = Vec::new();
        for c in &mark.claims {
            if !(c.parameter.starts_with("location") || c.parameter.contains("pitch.length")) {
                continue;
            }
            if node_feature(a, &c.feature, span.axis).is_some() {
                members.push(c.feature.clone());
            }
            if let Some(p) = a
                .recognition
                .as_ref()
                .and_then(|r| r.patterns.iter().find(|p| p.id == c.feature))
            {
                if pitch.is_some() {
                    members.extend(p.members.iter().cloned());
                } else {
                    members.push(p.id.clone());
                }
            }
        }
        members.sort();
        members.dedup();
        let at = |id: &String| {
            let mut p = a
                .features
                .iter()
                .find(|f| f.id == *id)
                .map_or([0.; 3], |f| f.fingerprint.position);
            p[span.axis] = station(a, id, span.axis)?;
            Some(p)
        };
        let left: Vec<_> = members
            .iter()
            .filter(|id| at(id).is_some_and(|p| (p[span.axis] - span.a).abs() < EPS))
            .collect();
        let right: Vec<_> = members
            .iter()
            .filter(|id| at(id).is_some_and(|p| (p[span.axis] - span.b).abs() < EPS))
            .collect();
        if pitch.is_some() && !left.is_empty() && !right.is_empty() {
            for l in &left {
                for r in &right {
                    let p = at(l).unwrap();
                    let q = at(r).unwrap();
                    let normal = a
                        .features
                        .iter()
                        .find(|f| f.id == **l)
                        .and_then(|f| f.fingerprint.axis)
                        .and_then(crate::bridge::holes::principal);
                    if (0..3)
                        .any(|k| k != span.axis && Some(k) != normal && (p[k] - q[k]).abs() > EPS)
                    {
                        continue;
                    }
                    let mut e = span.clone();
                    e.a_feature = Some((**l).clone());
                    e.b_feature = Some((**r).clone());
                    out.push(e);
                }
            }
        } else if location && !right.is_empty() {
            for r in right {
                let mut e = span.clone();
                e.b_feature = Some(r.clone());
                out.push(e);
            }
        } else {
            out.push(span);
        }
    }
    out
}

/// Requirements come from the part, not the engine inventory. Cylindrical features need their
/// transverse axes located; other prismatic features use their recognised locating station.
pub(crate) fn targets(a: &Analysis) -> Vec<(String, usize, f64)> {
    let mut out = Vec::new();
    for f in &a.features {
        match f.kind.as_str() {
            "hole" | "boss" | "polygonal_boss" | "oriented_slot" => {
                for k in (0..3).filter(|&k| f.fingerprint.axis.is_some_and(|d| d[k].abs() < EPS)) {
                    let at = station(a, &f.id, k).unwrap_or(f.fingerprint.position[k]);
                    out.push((f.id.clone(), k, at));
                }
            }
            "slot" => {
                for role in ["location_slot.length", "location_slot_across.length"] {
                    if let Some(span) = super::redundancy::span(a, &f.id, role) {
                        out.push((f.id.clone(), span.axis, span.hi));
                    }
                }
            }
            "turned_step" => {
                if let Ok(b) = super::turned::band(&crate::bridge::holes::frame(a), f) {
                    out.push((f.id.clone(), b.axis, b.lo));
                }
            }
            kind if kind.contains("step") => {
                for (i, face) in f
                    .faces
                    .iter()
                    .filter_map(|&i| a.face_attachments.get(i).map(|f| (i, f)))
                {
                    if face.surface.as_deref() == Some("plane")
                        && let Some(k) = face.axis
                        && let Some(p) = face.sites.first()
                    {
                        out.push((format!("face:{i}"), k, p[k]));
                    }
                }
            }
            _ => {}
        }
    }
    for (f, p) in super::pockets::placed(a) {
        let Ok(p) = p else { continue };
        if p.edge_anchored {
            continue;
        }
        for (role, distance) in super::pockets::locations(&p, a.envelope[0]) {
            if let Some(k) = AXES.iter().position(|axis| role.ends_with(axis)) {
                out.push((f.id.clone(), k, a.envelope[0][k] + distance));
            }
        }
    }
    for n in super::notch_evidence::requirements(a) {
        for k in [n.width_axis, n.depth_axis] {
            out.push((n.id.clone(), k, n.mouth[k]));
        }
    }
    out.sort_by(|p, q| p.partial_cmp(q).unwrap());
    out.dedup_by(|p, q| p.0 == q.0 && p.1 == q.1 && (p.2 - q.2).abs() < EPS);
    out
}

pub fn coverage(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    // Preserve the established non-PMI drawing/lint contract.
    if !matches!(&a.pmi, Some(Reading::Read(r)) if !r.records.is_empty())
        && !marks
            .iter()
            .flat_map(|m| &m.claims)
            .any(|c| c.feature.starts_with("station_span:"))
    {
        return vec![];
    }
    let edges = edges(a, marks);
    let roots = drawing_roots(a, marks);
    let mut issues = Vec::new();
    for (feature, k, at) in targets(a) {
        let parameter = format!("location.length.{}", AXES[k]);
        let first = route(
            &edges,
            &roots[k],
            k,
            at,
            node_feature(a, &feature, k).as_deref(),
            &BTreeSet::new(),
        );
        let (code, involved) = match first {
            None => ("feature_not_located", Vec::new()),
            Some(first) => {
                if first.is_empty() {
                    continue;
                }
                let owners: BTreeSet<_> = first.iter().map(|&i| edges[i].owner.as_str()).collect();
                let blocked = edges
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| owners.contains(e.owner.as_str()))
                    .map(|(i, _)| i)
                    .collect();
                let Some(second) = route(
                    &edges,
                    &roots[k],
                    k,
                    at,
                    node_feature(a, &feature, k).as_deref(),
                    &blocked,
                ) else {
                    continue;
                };
                (
                    "feature_located_twice",
                    first.into_iter().chain(second).collect(),
                )
            }
        };
        let mut names: Vec<_> = involved.iter().map(|&i| edges[i].owner.clone()).collect();
        names.sort();
        names.dedup();
        let mut issue = uncovered(
            a,
            code,
            &feature,
            &parameter,
            format!(
                "{feature} on {}: {}{}.",
                AXES[k],
                if code == "feature_not_located" {
                    "no drawn dimension chain reaches an origin"
                } else {
                    "independent drawn dimension chains reach an origin"
                },
                if names.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", names.join(", "))
                }
            ),
        );
        issue.related = names;
        issues.push(issue);
    }
    issues
}

/// Missing sizes are checked from physical feature claims or the source's attached faces.
pub fn sizes(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let report = match &a.pmi {
        Some(Reading::Read(r)) => Some((r, r.identities())),
        _ => None,
    };
    let mut issues = Vec::new();
    for f in a.features.iter().filter(|f| {
        matches!(
            f.kind.as_str(),
            "hole" | "slot" | "boss" | "pocket" | "turned_step"
        )
    }) {
        let sized = marks.iter().any(|m| {
            m.claims.iter().any(|c| {
                c.feature == f.id
                    && !c.parameter.starts_with("location")
                    && !c.parameter.starts_with("grouping")
                    && c.parameter != "thread"
            })
        }) || report.as_ref().is_some_and(|(r, ids)| {
            r.records.iter().any(|r| {
                r.category == Category::Dimension
                    && matches!(
                        r.kind.as_str(),
                        "diameter" | "radius" | "thickness" | "linear"
                    )
                    && !r.faces.is_empty()
                    && r.faces.iter().all(|i| f.faces.contains(i))
                    && ids.id(&r.source_id).ok().is_some_and(|id| {
                        marks
                            .iter()
                            .any(|m| m.claims.iter().any(|c| c.feature == id))
                    })
            })
        });
        if !sized {
            issues.push(uncovered(
                a,
                "feature_not_sized",
                &f.id,
                "size",
                format!("{} has no drawn size.", f.id),
            ));
        }
    }
    issues
}

/// Canonical legacy coverage roles derived from the physical network. These are arithmetic
/// evidence for the older family checks; they are never extra printed dimensions.
pub(crate) fn evidence(a: &Analysis, marks: &[Mark]) -> Vec<Mark> {
    if !active(a, marks) {
        return vec![];
    }
    let edges = edges(a, marks);
    let roots = drawing_roots(a, marks);
    let mut extra = Vec::new();
    let pockets = super::pockets::placed(a);
    for (feature, k, at) in targets(a) {
        if route(
            &edges,
            &roots[k],
            k,
            at,
            node_feature(a, &feature, k).as_deref(),
            &BTreeSet::new(),
        )
        .is_none()
        {
            continue;
        }
        let mut claims = vec![crate::output::Claim {
            feature: feature.clone(),
            parameter: format!("location.length.{}", AXES[k]),
        }];
        for role in ["location_slot.length", "location_slot_across.length"] {
            if super::redundancy::span(a, &feature, role).is_some_and(|s| s.axis == k) {
                claims.push(crate::output::Claim {
                    feature: feature.clone(),
                    parameter: role.into(),
                });
            }
        }
        for (f, read) in &pockets {
            if f.id != feature {
                continue;
            }
            if let Ok(p) = read {
                for (role, _) in super::pockets::locations(p, a.envelope[0]) {
                    if role.ends_with(AXES[k]) {
                        claims.push(crate::output::Claim {
                            feature: feature.clone(),
                            parameter: role,
                        });
                    }
                }
            }
        }
        extra.push(Mark {
            name: format!("location_network:{feature}/{}", AXES[k]),
            view: None,
            text: format!("{}", (at - a.envelope[0][k]).abs()),
            claims,
        });
    }
    extra
}

pub(crate) fn active(a: &Analysis, marks: &[Mark]) -> bool {
    matches!(&a.pmi,Some(Reading::Read(r)) if !r.records.is_empty())
        || marks
            .iter()
            .flat_map(|m| &m.claims)
            .any(|c| c.feature.starts_with("station_span:"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edge(a: f64, b: f64, name: &str) -> Edge {
        Edge {
            axis: 0,
            a,
            b,
            owner: name.into(),
            a_feature: None,
            b_feature: None,
        }
    }
    #[test]
    fn coincident_feature_axes_remain_distinct_nodes() {
        let mut a = edge(0., 50., "PMI A");
        a.b_feature = Some("hole A".into());
        let mut b = edge(0., 50., "PMI B");
        b.b_feature = Some("hole B".into());
        let edges = vec![a, b];
        assert!(route(&edges, &[0.], 0, 50., Some("hole A"), &BTreeSet::from([0])).is_none());
        assert!(route(&edges, &[0.], 0, 50., Some("hole B"), &BTreeSet::new()).is_some());
    }
    #[test]
    fn chains_require_an_origin_and_preserve_parallel_authored_constraints() {
        let edges = vec![
            edge(100., 95., "right PMI"),
            edge(95., 50., "PMI chain"),
            edge(0., 95., "left PMI"),
        ];
        let first = route(&edges, &[0., 100.], 0, 50., None, &BTreeSet::new()).unwrap();
        assert_eq!(first.len(), 2);
        assert!(route(&edges, &[0., 100.], 1, 50., None, &BTreeSet::new()).is_none());
        assert!(route(&edges, &[20.], 0, 50., None, &BTreeSet::new()).is_none());
        let first = route(&edges, &[0., 100.], 0, 95., None, &BTreeSet::new()).unwrap();
        let blocked = first.into_iter().collect();
        assert!(route(&edges, &[0., 100.], 0, 95., None, &blocked).is_some());
    }
}
