//! Physical support-plane redundancy, independent of the drawing's authoring decisions.

use super::{LintIssue, Mark, holes};
use crate::bridge::holes::{frame, point, principal, to_part};
use crate::model::Analysis;

/// Recognition records round stations to hundredths; never compare printed values alone.
pub(crate) const TOL: f64 = 0.01;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Span {
    pub axis: usize,
    pub lo: f64,
    pub hi: f64,
}
impl Span {
    pub(crate) fn new(axis: usize, a: f64, b: f64) -> Self {
        Self {
            axis,
            lo: a.min(b),
            hi: a.max(b),
        }
    }
    pub(crate) fn same(self, other: Self) -> bool {
        self.axis == other.axis
            && (self.lo - other.lo).abs() <= TOL
            && (self.hi - other.hi).abs() <= TOL
    }
}

/// A bounded graph walk through measured stations, returning the carrying edge indices.
pub(crate) fn path(edges: &[(Span, usize)], target: Span) -> Option<Vec<usize>> {
    let mut queue = vec![(target.lo, Vec::new())];
    let mut seen = vec![target.lo];
    let mut cursor = 0;
    while cursor < queue.len() {
        let (station, route) = queue[cursor].clone();
        cursor += 1;
        if (station - target.hi).abs() <= TOL {
            return Some(route);
        }
        for &(s, i) in edges {
            if s.axis != target.axis {
                continue;
            }
            let next = if (s.lo - station).abs() <= TOL {
                s.hi
            } else if (s.hi - station).abs() <= TOL {
                s.lo
            } else {
                continue;
            };
            if seen.iter().any(|v| (v - next).abs() <= TOL) {
                continue;
            }
            seen.push(next);
            let mut next_route = route.clone();
            next_route.push(i);
            queue.push((next, next_route));
        }
    }
    None
}

pub(crate) fn level_span(a: &Analysis, parameter: &str) -> Option<Span> {
    let h: f64 = parameter
        .strip_prefix("step_height.length.h")?
        .replace('_', ".")
        .parse()
        .ok()?;
    let z = a
        .levels
        .iter()
        .find(|z| (**z - a.envelope[0][2] - h).abs() <= TOL)?;
    Some(Span::new(2, a.envelope[0][2], *z))
}

/// A ladder shoulder `step_position.length.<axis><position>`, measured from the part's box.
pub(crate) fn position_span(a: &Analysis, parameter: &str) -> Option<Span> {
    let word = parameter.strip_prefix("step_position.length.")?;
    let k = ["x", "y", "z"].iter().position(|x| word.starts_with(x))?;
    let v: f64 = word[1..].replace('_', ".").parse().ok()?;
    Some(Span::new(k, a.envelope[0][k], a.envelope[0][k] + v))
}

/// A slot's walls across its width, and the centre line between them, in part coordinates.
pub(crate) fn slot_walls(a: &Analysis, feature: &str) -> Option<(Span, f64)> {
    let f = a
        .features
        .iter()
        .find(|f| f.id == feature && f.kind == "slot")?;
    let p = &f.parameters;
    let local = ["x", "y", "z"]
        .iter()
        .position(|k| Some(*k) == p.get("width_axis").and_then(|v| v.as_str()))?;
    let mut unit = [0.; 3];
    unit[local] = 1.;
    let k = principal(crate::bridge::holes::dir_to_part(&frame(a), unit))?;
    let (c, w) = (p.get("w_center")?.as_f64()?, p.get("width")?.as_f64()?);
    let station = |v: f64| {
        let mut q = [0.; 3];
        q[local] = v;
        to_part(&frame(a), q)[k]
    };
    Some((
        Span::new(k, station(c - w / 2.), station(c + w / 2.)),
        station(c),
    ))
}

pub(crate) fn span(a: &Analysis, feature: &str, parameter: &str) -> Option<Span> {
    if feature == "envelope" {
        let k = super::envelope::PARAMETERS
            .iter()
            .position(|p| *p == parameter)?;
        return Some(Span::new(k, a.envelope[0][k], a.envelope[1][k]));
    }
    if feature == "step_level" {
        return level_span(a, parameter).or_else(|| position_span(a, parameter));
    }
    if parameter.starts_with("location.length.") {
        if !feature.starts_with("notch:") {
            return None;
        }
        let n = super::notch_evidence::requirements(a)
            .into_iter()
            .find(|n| n.id == feature)?;
        let k = ["x", "y", "z"]
            .iter()
            .position(|axis| parameter == format!("location.length.{axis}"))?;
        return [n.width_axis, n.depth_axis]
            .contains(&k)
            .then(|| Span::new(k, a.envelope[0][k], n.mouth[k]));
    }
    if parameter.starts_with("pocket_") || parameter.starts_with("location_pocket.") {
        return pocket_span(a, feature, parameter);
    }
    let f = a.features.iter().find(|f| f.id == feature)?;
    if f.kind == "turned_step" && parameter == "step.length" && coaxial(a) {
        let b = super::turned::band(&frame(a), f).ok()?;
        return Some(Span::new(b.axis, b.lo, b.hi));
    }
    if f.kind == "plate" && parameter == "thickness.length" {
        let p = super::plates::read(a, f).ok()?;
        return Some(Span::new(p.axis, p.span[0][p.axis], p.span[1][p.axis]));
    }
    if f.kind == "slot" {
        let p = &f.parameters;
        let local = ["x", "y", "z"]
            .iter()
            .position(|k| Some(*k) == p.get("long_axis").and_then(|v| v.as_str()))?;
        let mut unit = [0.; 3];
        unit[local] = 1.;
        let k = principal(crate::bridge::holes::dir_to_part(&frame(a), unit))?;
        let station = |name| {
            let mut q = [0.; 3];
            q[local] = p.get(name)?.as_f64()?;
            Some(to_part(&frame(a), q)[k])
        };
        if parameter == "slot_length.length" {
            return Some(Span::new(k, station("lo")?, station("hi")?));
        }
        if parameter == "location_slot.length" {
            return Some(Span::new(k, a.envelope[0][k], station("lo")?));
        }
        if parameter == "slot_width.length" {
            return slot_walls(a, feature).map(|(s, _)| s);
        }
        if parameter == "location_slot_across.length" {
            let (s, c) = slot_walls(a, feature)?;
            return Some(Span::new(s.axis, a.envelope[0][s.axis], c));
        }
    }
    if f.kind == "boss" && parameter == "boss_height.length" {
        let axis = super::parameters::direction(&frame(a), f.parameters.get("axis"))?;
        let k = principal(axis)?;
        let at = to_part(&frame(a), point(f.parameters.get("location"))?);
        let h = f.parameters.get("height")?.as_f64()?;
        return Some(Span::new(k, at[k], at[k] - axis[k].signum() * h));
    }
    // A polygonal boss's record gives its axis letter, centre and the base and top stations
    // along that axis, in quiddity's frame (ctc01's hexagon: 0 to 50 along z).
    if f.kind == "polygonal_boss" && parameter == "boss_height.length" {
        let p = &f.parameters;
        let local = ["x", "y", "z"]
            .iter()
            .position(|k| Some(*k) == p.get("axis").and_then(|v| v.as_str()))?;
        let mut unit = [0.; 3];
        unit[local] = 1.;
        let k = principal(crate::bridge::holes::dir_to_part(&frame(a), unit))?;
        let centre = point(p.get("center"))?;
        let station = |name: &str| {
            let mut q = centre;
            q[local] = p.get(name)?.as_f64()?;
            Some(to_part(&frame(a), q)[k])
        };
        return Some(Span::new(k, station("base")?, station("top")?));
    }
    None
}

/// ADR1-8: a lone blind pocket's width, length and depth run between its walls and from its floor
/// to the face it opens into, along the part's axes, and its locations from the box's faces to
/// its reference point (a pattern member's are its pattern's; a pocket whose mouth is a cylinder
/// has a greatest depth, no second plane).
fn pocket_span(a: &Analysis, feature: &str, parameter: &str) -> Option<Span> {
    if super::pocket_patterns::member(a, feature) {
        return None;
    }
    let (_, p) = super::pockets::placed(a)
        .into_iter()
        .find(|(f, _)| f.id == feature)?;
    let p = p.ok()?;
    let at = |k: usize, size: f64| Span::new(k, p.centre[k] - size / 2., p.centre[k] + size / 2.);
    // Its location: the box's face to its reference point, the station pocket coverage reads.
    if let Some(k) = parameter
        .strip_prefix("location_pocket.")
        .map(|w| w.trim_start_matches("location."))
        .and_then(|w| ["x", "y", "z"].iter().position(|x| *x == w))
    {
        let datum = a.envelope[0];
        let station = if k == p.long_axis && (p.lo - datum[k]).abs() <= 1e-6 {
            p.lo
        } else if k == p.long_axis {
            (p.lo + p.hi) / 2.
        } else if k == p.width_axis {
            p.w_center
        } else {
            return None;
        };
        return Some(Span::new(k, datum[k], station));
    }
    match parameter {
        "pocket_width.length" => Some(Span::new(
            p.width_axis,
            p.w_center - p.width / 2.,
            p.w_center + p.width / 2.,
        )),
        "pocket_length.length" => Some(Span::new(p.long_axis, p.lo, p.hi)),
        "pocket_depth.length" if !p.mouth => Some(at(p.depth_axis, p.depth)),
        _ => None,
    }
}

/// Stream K6 (#33, #26): a dimension drawn in brackets is reference, for information only; one
/// whose printed text carries a tolerance or a fit is a requirement drawn as reference.
pub fn references(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    for m in marks {
        if m.text.starts_with('(') && m.text.ends_with(')') && toleranced(&m.text) {
            let (feature, parameter) = m
                .claims
                .first()
                .map_or(("", ""), |c| (c.feature.as_str(), c.parameter.as_str()));
            let mut issue = super::uncovered(
                a,
                "toleranced_reference",
                feature,
                parameter,
                format!(
                    "{} is drawn as a reference dimension but carries a tolerance or fit; a reference dimension is for information only, so a toleranced one must be controlling.",
                    m.text
                ),
            );
            issue.id = format!("l:toleranced_reference:{}", m.name);
            issue.annotation = Some(m.name.clone());
            issue.fix = Some("Draw it as a controlling dimension, or remove its tolerance.".into());
            issues.push(issue);
        }
    }
    issues
}

/// Whether reference text *text* prints a tolerance: a `±` band, a `+0.1 -0.02` deviation pair
/// or an ISO 286 class after its size (`H7`, `h6`, `js9`).
fn toleranced(text: &str) -> bool {
    text.contains('±')
        || text
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .any(|t| {
                let digits = t.trim_start_matches(|c: char| c.is_ascii_alphabetic());
                let letters = &t[..t.len() - digits.len()];
                (t.starts_with('+') && t[1..].starts_with(|c: char| c.is_ascii_digit()))
                    || (["H", "G", "F", "h", "g", "f", "js", "k", "n", "p"].contains(&letters)
                        && !digits.is_empty()
                        && digits.parse::<u32>().is_ok_and(|g| (5..=11).contains(&g)))
            })
}

pub fn coverage(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    // Native location ink is local audit evidence, outside the semantic claim ledger.
    let semantic;
    let marks = if super::location::active(a, marks) {
        semantic = marks
            .iter()
            .map(|m| {
                let mut m = m.clone();
                m.claims.retain(|c| !c.feature.starts_with("station_"));
                m
            })
            .collect::<Vec<_>>();
        &semantic
    } else {
        marks
    };
    if a.solid_count.is_some_and(|n| n > 1) {
        return Vec::new();
    }
    let mut edges: Vec<(Span, usize)> = Vec::new();
    let mut claims = std::collections::BTreeMap::new();
    let mut issues = Vec::new();
    for (i, m) in marks.iter().enumerate() {
        if m.text.starts_with('(') && m.text.ends_with(')') || m.name.starts_with("suppressed:") {
            continue;
        }
        for c in &m.claims {
            let key = (&c.feature, &c.parameter);
            let repeated = claims.insert(key, i).filter(|j| *j != i);
            if let Some(j) = repeated {
                let mut issue = super::uncovered(
                    a,
                    "duplicate_measurement",
                    &c.feature,
                    &c.parameter,
                    "One physical measurement is stated by two annotations.".into(),
                );
                issue.annotation = Some(m.name.clone());
                issue.related = vec![marks[j].name.clone()];
                issues.push(issue);
            }
        }
        let own_spans = stated_spans(a, m);
        // A stated width fixes the centre line between the walls too.
        let halves: Vec<Span> = m
            .claims
            .iter()
            .filter(|c| c.parameter != "slot_width.length" || !m.text.contains('×'))
            .flat_map(|c| halves(a, &c.feature, &c.parameter, &own_spans))
            .collect();
        for s in own_spans {
            if let Some(route) = path(&edges, s) {
                let duplicate = route.len() == 1;
                if duplicate
                    && m.claims.iter().all(|c| c.parameter == "thickness.length")
                    && marks[route[0]]
                        .claims
                        .iter()
                        .all(|c| c.parameter == "thickness.length")
                {
                    let body = |m: &Mark| {
                        m.claims
                            .first()
                            .and_then(|c| a.features.iter().find(|f| f.id == c.feature))
                            .and_then(|f| f.parameters.get("body_key"))
                            .filter(|key| !key.is_null())
                    };
                    if body(m).is_none() || body(m) != body(&marks[route[0]]) {
                        edges.push((s, i));
                        continue;
                    }
                }
                if duplicate
                    && marks[route[0]].view != m.view
                    && m.claims.iter().all(|c| {
                        c.feature.starts_with("f:hole:") || c.feature.starts_with("f:hole_pattern:")
                    })
                    && marks[route[0]].claims.iter().all(|c| {
                        c.feature.starts_with("f:hole:") || c.feature.starts_with("f:hole_pattern:")
                    })
                {
                    edges.push((s, i));
                    continue;
                }
                let mut issue=super::uncovered(a,if duplicate {"duplicate_measurement"} else {"closed_dimension_chain"},&m.claims[0].feature,&m.claims[0].parameter,
                    if duplicate {"The same support-plane interval is stated twice."} else {"A dimension chain and its total are all stated; omit a redundant member or show it in parentheses as reference."}.into());
                issue.annotation = Some(m.name.clone());
                issue.related = route.iter().map(|j| marks[*j].name.clone()).collect();
                issue.related.dedup();
                issues.push(issue);
            } else {
                edges.push((s, i));
            }
        }
        edges.extend(halves.into_iter().map(|s| (s, i)));
    }
    issues
}

/// The support-plane intervals mark *m* states: each claim's plane interval, or an outside
/// diameter's or A/F's support planes across it, whose length its text prints. ADR1-8: a counted
/// mark (`4× 255`) states each member's interval its size prints, never through its count; a
/// compound callout (`2 × 2.81 × 3.445 DEEP`) states each of its claims' intervals its terms print.
fn stated_spans(a: &Analysis, m: &Mark) -> Vec<Span> {
    stated(
        a,
        &m.text,
        m.claims
            .iter()
            .map(|c| (c.feature.as_str(), c.parameter.as_str())),
    )
}

/// The support-plane intervals a label printing *text* states for its *claims* (feature,
/// parameter), as [`stated_spans`] reads a mark; the planner reads its own labels the same way.
pub(crate) fn stated<'c>(
    a: &Analysis,
    text: &str,
    claims: impl IntoIterator<Item = (&'c str, &'c str)>,
) -> Vec<Span> {
    let mut own: Vec<Span> = Vec::new();
    let text = repeated(text).unwrap_or(text);
    for (feature, parameter) in claims {
        let found: Vec<Span> = match span(a, feature, parameter) {
            Some(s) => vec![s],
            None => (0..3)
                .filter_map(|k| external_span(a, feature, parameter, k))
                .collect(),
        };
        for s in found {
            if holes::states(text, s.hi - s.lo) && !own.iter().any(|p| p.same(s)) {
                own.push(s);
            }
        }
    }
    own
}

/// The two halves of a stated slot width, or of a pocket's stated width or length, each from a
/// wall to the centre line between them: a stated size fixes that centre line too. *own* is what
/// the label states; a size it does not state has no halves.
pub(crate) fn halves(a: &Analysis, feature: &str, parameter: &str, own: &[Span]) -> Vec<Span> {
    let walls = match parameter {
        "slot_width.length" => slot_walls(a, feature),
        "pocket_width.length" | "pocket_length.length" => {
            pocket_span(a, feature, parameter).map(|s| (s, (s.lo + s.hi) / 2.))
        }
        _ => None,
    };
    walls
        .filter(|(s, _)| own.iter().any(|p| p.same(*s)))
        .map(|(s, c)| vec![Span::new(s.axis, s.lo, c), Span::new(s.axis, c, s.hi)])
        .unwrap_or_default()
}

/// The size a counted label repeats: `4× 255` repeats ` 255`. A compound callout's `×` (spaced,
/// `2 × 3.7`) is no count.
fn repeated(text: &str) -> Option<&str> {
    let (count, size) = text.split_once('×')?;
    (count.trim().parse::<usize>().is_ok() && !count.ends_with(' ')).then_some(size)
}

/// ISSUE-1505-b: the planner leaves one member of a closed chain to its stated total and the
/// other members (`State::Covered`), so a length requirement nothing claims is still met when
/// the drawing's controlling dimensions walk from one of its support planes to the other. Drops
/// each `missing` finding on an axial length, boss height or polygonal boss height they determine.
pub fn determined(a: &Analysis, marks: &[Mark], issues: &mut Vec<LintIssue>) {
    const CODES: [&str; 3] = [
        "axial_length_missing",
        "boss_height_missing",
        "polygonal_boss_requirement_missing",
    ];
    if a.solid_count.is_some_and(|n| n > 1)
        || !issues.iter().any(|i| CODES.contains(&i.code.as_str()))
    {
        return;
    }
    let edges: Vec<(Span, usize)> = marks
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            !(m.text.starts_with('(') && m.text.ends_with(')') || m.name.starts_with("suppressed:"))
        })
        .flat_map(|(i, m)| stated_spans(a, m).into_iter().map(move |s| (s, i)))
        .collect();
    issues.retain(|i| {
        let [r] = i.requirements.as_slice() else {
            return true;
        };
        !(CODES.contains(&i.code.as_str())
            && i.annotation.is_none()
            && span(a, &r.feature, &r.parameter).is_some_and(|s| path(&edges, s).is_some()))
    });
}

/// An outside size's physical support planes; a bore never supplies this evidence.
pub(crate) fn external_span(a: &Analysis, id: &str, parameter: &str, k: usize) -> Option<Span> {
    let f = a.features.iter().find(|f| f.id == id)?;
    if f.kind == "turned_step" && parameter == "step.diameter" && coaxial(a) {
        let b = super::turned::band(&frame(a), f).ok()?;
        if k == b.axis {
            return None;
        }
        let same = a
            .features
            .iter()
            .filter(|s| s.kind == "turned_step")
            .filter_map(|s| super::turned::band(&frame(a), s).ok())
            .filter(|c| {
                c.axis == b.axis
                    && (c.diameter - b.diameter).abs() <= 1e-6
                    && (0..3).all(|j| j == b.axis || (c.on[j] - b.on[j]).abs() <= 1e-3)
            })
            .count();
        if same != 1 {
            return None;
        }
        return Some(Span::new(
            k,
            b.on[k] - b.diameter / 2.,
            b.on[k] + b.diameter / 2.,
        ));
    }
    if f.kind == "flat" && parameter == "flat.length" {
        let g = super::flats::groups(a)
            .into_iter()
            .find(|g| g.members.iter().any(|m| m.id == id))?;
        let points: Vec<_> = g
            .members
            .iter()
            .filter_map(|m| point(m.parameters.get("at")).map(|p| to_part(&frame(a), p)))
            .collect();
        let size = f.parameters.get("across")?.as_f64()?;
        for p in &points {
            for q in &points {
                if ((p[k] - q[k]).abs() - size).abs() <= 1e-3 {
                    return Some(Span::new(k, p[k], q[k]));
                }
            }
        }
    }
    None
}

/// A profile chain has one physical axis line; eccentric bands cannot close it.
pub(crate) fn coaxial(a: &Analysis) -> bool {
    let bands: Vec<_> = a
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .filter_map(|f| super::turned::band(&frame(a), f).ok())
        .collect();
    bands.first().is_some_and(|b| {
        bands.iter().all(|c| {
            c.axis == b.axis && (0..3).all(|k| k == b.axis || (c.on[k] - b.on[k]).abs() <= 1e-3)
        })
    })
}
