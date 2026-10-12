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

pub(crate) fn span(a: &Analysis, feature: &str, parameter: &str) -> Option<Span> {
    if feature == "envelope" {
        let k = super::envelope::PARAMETERS
            .iter()
            .position(|p| *p == parameter)?;
        return Some(Span::new(k, a.envelope[0][k], a.envelope[1][k]));
    }
    if feature == "step_level" && parameter.starts_with("step_height.length.h") {
        let h: f64 = parameter
            .trim_start_matches("step_height.length.h")
            .replace('_', ".")
            .parse()
            .ok()?;
        let z = a
            .levels
            .iter()
            .find(|z| (**z - a.envelope[0][2] - h).abs() <= TOL)?;
        return Some(Span::new(2, a.envelope[0][2], *z));
    }
    if feature == "step_level" && parameter.starts_with("step_position.length.") {
        let tail = parameter.trim_start_matches("step_position.length.");
        let k = ["x", "y", "z"]
            .iter()
            .position(|axis| tail.starts_with(axis))?;
        let at = tail[1..].replace('_', ".").parse::<f64>().ok()?;
        return Some(Span::new(k, a.envelope[0][k], at));
    }
    let f = a.features.iter().find(|f| f.id == feature)?;
    if f.kind == "hole" && parameter.starts_with("location.length.") {
        let k = ["x", "y", "z"]
            .iter()
            .position(|k| Some(*k) == parameter.strip_prefix("location.length."))?;
        let h = crate::bridge::holes::read(a)
            .into_iter()
            .find(|(f, _)| f.id == feature)?
            .1
            .ok()?;
        return Some(Span::new(k, a.envelope[0][k], h.at[k]));
    }
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
    }
    if f.kind == "boss" && parameter == "boss_height.length" {
        let axis = super::parameters::direction(&frame(a), f.parameters.get("axis"))?;
        let k = principal(axis)?;
        let at = to_part(&frame(a), point(f.parameters.get("location"))?);
        let h = f.parameters.get("height")?.as_f64()?;
        return Some(Span::new(k, at[k], at[k] - axis[k].signum() * h));
    }
    None
}

pub fn coverage(a: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
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
        let mut own_spans = Vec::new();
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
            if let Some(s) = span(a, &c.feature, &c.parameter)
                && holes::states(&m.text, s.hi - s.lo)
                && !own_spans.iter().any(|p: &Span| p.same(s))
            {
                own_spans.push(s);
            }
        }
        for s in own_spans {
            if let Some(route) = path(&edges, s) {
                let duplicate = route.len() == 1;
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
                issues.push(issue);
            } else {
                edges.push((s, i));
            }
        }
    }
    issues
}

/// An outside size's physical support planes; a bore never supplies this evidence.
pub(crate) fn external_span(a: &Analysis, id: &str, parameter: &str, k: usize) -> Option<Span> {
    let f = a.features.iter().find(|f| f.id == id)?;
    if f.kind == "turned_step" && parameter == "step.diameter" {
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
