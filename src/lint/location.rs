//! Rooted physical station graphs. Both precedence and the drawn-set audit use this walk.
use super::{LintIssue, Mark, uncovered};
use crate::model::Analysis;
use crate::pmi::{Category, Reading};
use std::collections::BTreeSet;

const EPS: f64 = 1e-6;

/// The part and witnesses use millimetres; authored PMI can explicitly print inches.
fn states_mm(text: &str, value: f64) -> bool {
    let inch = text
        .split_whitespace()
        .any(|word| word.eq_ignore_ascii_case("INCH") || word.eq_ignore_ascii_case("INCHES"));
    super::holes::states(text, value / if inch { 25.4 } else { 1. })
}
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
        // A feature on an origin plane needs no additional zero-length dimension.
        // Analytic feature axes use the tighter tolerance; rounded planes use TOL.
        if roots.iter().any(|r| {
            (r - p).abs()
                < if node.is_none() {
                    super::redundancy::TOL
                } else {
                    EPS
                }
        }) {
            return Some(path);
        }
        for (i, e) in edges.iter().enumerate() {
            if e.axis != axis || excluded.contains(&i) {
                continue;
            }
            let (q, next_node) = if node == e.a_feature && (node.is_some() || (p - e.a).abs() < EPS)
            {
                (e.b, e.b_feature.clone())
            } else if node == e.b_feature && (node.is_some() || (p - e.b).abs() < EPS) {
                (e.a, e.a_feature.clone())
            } else {
                continue;
            };
            if seen
                .iter()
                .any(|(v, n)| *n == next_node && (n.is_some() || (q - v).abs() < EPS))
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
        if let Some(key) = c.feature.strip_prefix("scheme:")
            && let [_, datum, _, _] = key.split('|').collect::<Vec<_>>().as_slice()
            && let Some((style, axis)) = c.parameter.split_once('.')
            && ["chain", "baseline", "ordinate"].contains(&style)
            && let Some(k) = AXES.iter().position(|v| *v == axis)
            && let Ok(p) = a.scheme_station(datum, k)
        {
            out[k].push(p[k]);
        }
        if let Some(word) = c.feature.strip_prefix("station_root:")
            && let Some((axis, value)) = word.split_once('|')
            && let (Ok(k), Ok(v)) = (axis.parse::<usize>(), value.parse::<f64>())
            && k < 3
            && v.is_finite()
        {
            out[k].push(v);
        }
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
        let ink = mark.claims.iter().any(|c| c.feature == "station_ink");
        let physical = ink
            || mark
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
                    && (states_mm(&mark.text, (q - p).abs())
                        || mark.text.split_once('×').is_some_and(|(n, rest)| {
                            n.trim()
                                .parse::<f64>()
                                .is_ok_and(|n| states_mm(rest.trim(), (q - p).abs() / n))
                        }))
                {
                    spans.push((k, p, q));
                }
            } else if !ink
                && let Some((r, ids)) = &report
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
                        && states_mm(&mark.text, (g.stations[1][k] - g.stations[0][k]).abs())
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

/// Re-measure a principal-axis constraint from the actual two witnesses and the drawn view
/// transform. This also works for cropped details: the frame names the whole-part centre.
pub fn ink_span(
    a: &Analysis,
    frame: &crate::output::DrawnFrame,
    lines: &[[[f64; 2]; 2]],
) -> Option<(usize, f64, f64)> {
    let ink = crate::output::linear_ink(lines)?;
    if !frame.scale.is_finite() || frame.scale <= 0. {
        return None;
    }
    let view = haecceity::hlr::View::new(frame.direction, frame.up)?;
    let (axis, sign) = (0..3).find_map(|k| {
        let mut unit = [0.; 3];
        unit[k] = 1.;
        let q = view.map(unit);
        ((q[ink.k].abs() - 1.).abs() < EPS && q[1 - ink.k].abs() < EPS).then_some((k, q[ink.k]))
    })?;
    let station = |e: &[[f64; 2]; 2]| {
        let run = e[1][1 - ink.k] - e[0][1 - ink.k];
        let p = e[0][ink.k]
            + (lines[2][0][1 - ink.k] - e[0][1 - ink.k]) * (e[1][ink.k] - e[0][ink.k]) / run;
        0.5 * (a.envelope[0][axis] + a.envelope[1][axis])
            + sign * (p - frame.at[ink.k]) / frame.scale
    };
    Some((axis, station(&lines[0]), station(&lines[1])))
}

/// Ink evidence is local to lint. It is never written into the drawing or its Spec.
pub(crate) fn drawn_constraints(
    a: &Analysis,
    drawing: &crate::output::Drawing,
    marks: &mut [Mark],
) {
    if drawing.sheet.frames.is_empty()
        || (!crate::dims::pmi_policy::carries(a)
            && !drawing.annotations.iter().any(|annotation| {
                annotation.coordinate_origin.is_some()
                    && drawing.sheet.items.iter().any(|item| {
                        item.name == annotation.name && item.kind == crate::output::Kind::Dimension
                    })
            }))
    {
        return;
    }
    for mark in marks {
        let Some(item) = drawing.sheet.items.iter().find(|d| d.name == mark.name) else {
            continue;
        };
        if item.kind != crate::output::Kind::Dimension || item.elbow.is_some() {
            continue;
        }
        if let Some(origin) = drawing
            .annotations
            .iter()
            .find(|a| a.name == mark.name)
            .and_then(|a| a.coordinate_origin)
        {
            for (k, v) in origin.into_iter().enumerate() {
                mark.claims.push(crate::output::Claim {
                    feature: format!("station_root:{k}|{v}"),
                    parameter: "origin".into(),
                });
            }
        }
        mark.claims
            .retain(|c| !c.feature.starts_with("station_span:") && c.feature != "station_ink");
        mark.claims.push(crate::output::Claim {
            feature: "station_ink".into(),
            parameter: "witnesses".into(),
        });
        let span = mark
            .view
            .as_deref()
            .and_then(|v| drawing.sheet.frames.get(v.trim_start_matches("v:")))
            .and_then(|f| ink_span(a, f, &item.lines));
        if let Some((axis, p, q)) = span {
            mark.claims.push(crate::output::Claim {
                feature: span_claim(axis, p, q),
                parameter: "station.distance".into(),
            });
        }
    }
}

/// Feature axes remain distinct even when they share a coordinate in a projection.
pub(crate) fn node_feature(a: &Analysis, id: &str, k: usize) -> Option<String> {
    if let Some(p) = pattern(a, id)
        && p.members.iter().all(|id| node_feature(a, id, k).is_some())
    {
        return Some(id.into());
    }
    if let Some(f) = a.features.iter().find(|f| f.id == id && f.kind == "slot") {
        return ["location_slot.length", "location_slot_across.length"]
            .iter()
            .any(|role| super::redundancy::span(a, &f.id, role).is_some_and(|span| span.axis == k))
            .then(|| id.into());
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

pub(crate) fn pattern<'a>(
    a: &'a Analysis,
    id: &str,
) -> Option<std::borrow::Cow<'a, crate::bridge::HolePattern>> {
    if let Some(p) = a
        .recognition
        .as_ref()
        .and_then(|r| r.patterns.iter().find(|p| p.id == id))
    {
        return Some(std::borrow::Cow::Borrowed(p));
    }
    let f = a
        .features
        .iter()
        .find(|f| f.id == id && f.kind == "slot_pattern")?;
    let members = f
        .parameters
        .get("slots")?
        .as_array()?
        .iter()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect();
    Some(std::borrow::Cow::Owned(crate::bridge::HolePattern {
        id: f.id.clone(),
        arrangement: if f.parameters.contains_key("col_pitch") {
            "grid"
        } else {
            "linear"
        }
        .into(),
        members,
        faces: f.faces.clone(),
        parameters: f.parameters.clone(),
    }))
}

pub(crate) fn station(a: &Analysis, id: &str, k: usize) -> Option<f64> {
    if let Some(p) = pattern(a, id) {
        let points: Option<Vec<_>> = p.members.iter().map(|id| station(a, id, k)).collect();
        let points = points?;
        return (!points.is_empty()).then(|| points.iter().sum::<f64>() / points.len() as f64);
    }
    if let Some(f) = a.features.iter().find(|f| f.id == id) {
        if f.kind == "hole"
            && let Ok(h) = crate::bridge::holes::Hole::read(f, &crate::bridge::holes::frame(a))
        {
            return Some(h.at[k]);
        }
        if f.kind == "slot" {
            let p = super::slots::centre(f).ok()?;
            return Some(crate::bridge::holes::to_part(&crate::bridge::holes::frame(a), p)[k]);
        }
        if let Some(v) = a.axis_station(id, k) {
            return Some(v);
        }
        return Some(f.fingerprint.position[k]);
    }
    a.scheme_station(id, k).ok().map(|p| p[k])
}

/// A cylindrical datum establishes its own axis, even though it cannot be a
/// planar origin for ± dimensions. Only features coaxial in both radial axes qualify.
fn datum_axis_root(a: &Analysis, feature: &str, k: usize) -> bool {
    let Some(f) = a.features.iter().find(|f| f.id == feature) else {
        return false;
    };
    let Some(direction) = f.fingerprint.axis else {
        return false;
    };
    let Some(axis) = direction.iter().position(|v| v.abs() > 1. - EPS) else {
        return false;
    };
    if axis == k {
        return false;
    }
    let Some(Reading::Read(r)) = &a.pmi else {
        return false;
    };
    let radial: Vec<_> = (0..3).filter(|v| *v != axis).collect();
    r.records
        .iter()
        .filter(|r| r.category == Category::Datum)
        .flat_map(|r| &r.faces)
        .any(|&i| {
            a.face_attachments.get(i).is_some_and(|face| {
                face.surface.as_deref() == Some("cylinder")
                    && face.exact_axis == Some(axis)
                    && face.support.len() == 2
                    && radial.iter().enumerate().all(|(j, &k)| {
                        station(a, feature, k).is_some_and(|v| (v - face.support[j]).abs() < EPS)
                    })
            })
        })
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
    if node.is_some_and(|feature| datum_axis_root(a, feature, axis)) {
        return Some(Vec::new());
    }
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

fn pitch_states(a: &Analysis, p: &crate::bridge::HolePattern, k: usize, text: &str) -> bool {
    let Some(mut xs): Option<Vec<_>> = p.members.iter().map(|id| station(a, id, k)).collect()
    else {
        return false;
    };
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|x, y| (*x - *y).abs() < EPS);
    let (count, body) = text.split_once('×').map_or((Some(1), text), |(n, body)| {
        (n.trim().parse::<usize>().ok(), body.trim())
    });
    xs.len() >= 2
        && count == Some(xs.len() - 1)
        && states_mm(body, xs[1] - xs[0])
        && xs
            .windows(2)
            .all(|q| ((q[1] - q[0]) - (xs[1] - xs[0])).abs() < super::redundancy::TOL)
}

pub(crate) fn identify(a: &Analysis, mark: &Mark, spans: Vec<Edge>) -> Vec<Edge> {
    let report = match &a.pmi {
        Some(Reading::Read(r)) => Some((r, r.identities())),
        _ => None,
    };
    let mut out = Vec::new();
    for span in spans {
        if let Some(c) = mark
            .claims
            .iter()
            .find(|c| c.feature.starts_with("scheme:"))
        {
            if let Some(key) = c.feature.strip_prefix("scheme:")
                && let [_, _, from, to] = key.split('|').collect::<Vec<_>>().as_slice()
                && let Some((style, axis)) = c.parameter.split_once('.')
                && AXES[span.axis] == axis
                && let (Ok(p), Ok(q)) = (
                    a.scheme_station(from, span.axis),
                    a.scheme_station(to, span.axis),
                )
                && mark
                    .text
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .is_some_and(|n| {
                        let value = if style == "ordinate" {
                            q[span.axis] - p[span.axis]
                        } else {
                            (q[span.axis] - p[span.axis]).abs()
                        };
                        n.signum() == value.signum() && states_mm(&mark.text, value.abs())
                    })
            {
                let node = |key: &str| {
                    node_feature(a, key.strip_prefix("axis:").unwrap_or(key), span.axis)
                };
                let mut e = span.clone();
                if (p[span.axis] - span.a).abs() < super::redundancy::TOL
                    && (q[span.axis] - span.b).abs() < super::redundancy::TOL
                {
                    e.a_feature = node(from);
                    e.b_feature = node(to);
                    out.push(e);
                } else if (q[span.axis] - span.a).abs() < super::redundancy::TOL
                    && (p[span.axis] - span.b).abs() < super::redundancy::TOL
                {
                    e.a_feature = node(to);
                    e.b_feature = node(from);
                    out.push(e);
                }
            }
            continue;
        }
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
                            && station(a, &f.id, e.axis)
                                .is_some_and(|p| (p - at).abs() < super::redundancy::TOL)
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
        let mut expanded_pitch = false;
        for c in mark
            .claims
            .iter()
            .filter(|c| c.parameter.contains("pitch.length"))
        {
            let Some(p) = pattern(a, &c.feature) else {
                continue;
            };
            if !pitch_states(a, &p, span.axis, &mark.text) {
                continue;
            }
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
                expanded_pitch = true;
            }
        }
        if expanded_pitch {
            continue;
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
            if let Some(p) = pattern(a, &c.feature) {
                members.extend(p.members.iter().cloned());
                members.push(p.id.clone());
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
            .filter(|id| {
                at(id).is_some_and(|p| (p[span.axis] - span.a).abs() < super::redundancy::TOL)
            })
            .collect();
        let right: Vec<_> = members
            .iter()
            .filter(|id| {
                at(id).is_some_and(|p| (p[span.axis] - span.b).abs() < super::redundancy::TOL)
            })
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
        } else if location
            && right.is_empty()
            && members.len() == 1
            && mark
                .claims
                .iter()
                .any(|c| c.parameter.starts_with("location_slot."))
        {
            let mut e = span;
            e.b_feature = Some(members[0].clone());
            out.push(e);
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
                        out.push((
                            f.id.clone(),
                            span.axis,
                            station(a, &f.id, span.axis).unwrap_or(span.hi),
                        ));
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
    let edges = edges(a, marks);
    let roots = drawing_roots(a, marks);
    let mut issues = Vec::new();
    for (feature, k, at) in targets(a) {
        if datum_axis_root(a, &feature, k) {
            continue;
        }
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
            None => {
                let node = node_feature(a, &feature, k);
                let involved: Vec<_> = edges
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        e.axis == k
                            && ((e.a_feature == node && (node.is_some() || (e.a - at).abs() < EPS))
                                || (e.b_feature == node
                                    && (node.is_some() || (e.b - at).abs() < EPS)))
                    })
                    .map(|(i, _)| i)
                    .collect();
                ("feature_not_located", involved)
            }
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
                    && !c.parameter.starts_with("seat_location.")
                    && !c.parameter.starts_with("step_position.")
                    && !c.parameter.starts_with("grouping")
                    && c.parameter != "thread"
            })
        }) || report.as_ref().is_some_and(|(r, ids)| {
            r.records.iter().any(|r| {
                ((r.category == Category::ManufacturingRequirement && r.kind.ends_with("thread"))
                    || (r.category == Category::Dimension
                        && matches!(
                            r.kind.as_str(),
                            "diameter" | "radius" | "thickness" | "linear"
                        )))
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
        || marks.iter().flat_map(|m| &m.claims).any(|c| {
            c.feature.starts_with("station_origin:") || c.feature.starts_with("station_root:")
        })
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
    fn explicit_imperial_pmi_locates_in_the_parts_millimetre_frame() {
        assert!(states_mm("0.30 ±0.02 INCH", 7.62));
        assert!(!states_mm("0.30 INCH", 0.30));
        assert!(states_mm("7.62", 7.62));
    }
    #[test]
    fn rounded_recognition_stations_still_reach_the_physical_origin() {
        assert!(
            route(
                &[edge(-0.000369, 5.5, "baseline")],
                &[0.],
                0,
                5.5,
                None,
                &BTreeSet::new()
            )
            .is_some()
        );
    }
    #[test]
    fn a_feature_axis_on_the_origin_needs_no_zero_dimension() {
        assert_eq!(
            route(&[], &[0.], 0, 0., Some("hole"), &BTreeSet::new()),
            Some(vec![])
        );
        assert!(route(&[], &[0.], 0, 0.005, Some("hole"), &BTreeSet::new()).is_none());
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
