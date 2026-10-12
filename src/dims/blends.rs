//! Blend radii: equal displayed values share exact occurrence credit. A frequent radius is
//! stated by one general note; other radii are exceptions, called out where their arc reads.
use std::collections::BTreeMap;

use super::{Context, Measurement, Planner, Problem, Shape, Want, end_on, fmt, want_id};
use crate::bridge::Feature;
use crate::bridge::holes::{frame, principal};
use crate::model::Analysis;
use crate::notes::NoteSpec;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["blend"],
    wants,
};
const GENERAL_COUNT: usize = 5;
type ViewSites<'a> = BTreeMap<(&'static str, usize), Vec<([f64; 3], &'a Feature)>>;

fn groups(a: &Analysis) -> Vec<Vec<&Feature>> {
    let mut groups = BTreeMap::<String, Vec<&Feature>>::new();
    for f in a.features.iter().filter(|f| f.kind == "blend") {
        if let Some(r) = radius(f) {
            groups.entry(fmt(r)).or_default().push(f);
        }
    }
    let mut out: Vec<_> = groups.into_values().collect();
    for group in &mut out {
        group.sort_by(|a, b| a.id.cmp(&b.id));
    }
    out.sort_by(|a, b| {
        radius(a[0])
            .unwrap()
            .total_cmp(&radius(b[0]).unwrap())
            .then(a[0].id.cmp(&b[0].id))
    });
    out
}
fn radius(f: &Feature) -> Option<f64> {
    f.parameters
        .get("radius")?
        .as_f64()
        .filter(|r| r.is_finite() && *r > 0.0)
}
fn general(groups: &[Vec<&Feature>]) -> Option<usize> {
    groups
        .iter()
        .enumerate()
        .filter(|(_, g)| {
            g.len() >= 2 && groups.iter().map(Vec::len).sum::<usize>() >= GENERAL_COUNT
        })
        .max_by_key(|(i, g)| (g.len(), std::cmp::Reverse(*i)))
        .map(|(i, _)| i)
}
fn measurements(group: &[&Feature]) -> Vec<Measurement> {
    group
        .iter()
        .map(|f| Measurement {
            feature: f.id.clone(),
            parameter: "blend.radius".into(),
            value: radius(f).unwrap(),
        })
        .collect()
}

fn sites_by_view<'a>(a: &Analysis, group: &[&'a Feature]) -> ViewSites<'a> {
    let mut by_view = ViewSites::new();
    for &f in group {
        let Some(path) = f.parameters.get("path") else {
            continue;
        };
        let Some(dir) = crate::lint::parameters::direction(
            &frame(a),
            path.get("direction").or(path.get("normal")),
        ) else {
            continue;
        };
        let Some(axis) = principal(dir) else { continue };
        if let Some(sites) = a.blend_sites.get(&f.id) {
            for &site in sites {
                let views = if path.get("normal").is_some() {
                    super::envelope::profile_views(axis)
                } else {
                    vec![end_on(axis)]
                };
                for view in views {
                    by_view.entry((view, axis)).or_default().push((site, f));
                }
            }
        }
    }
    by_view
}

/// Automatic notes carry explicit measurement identities; prose authored in Spec never does.
pub(crate) fn notes(a: &Analysis) -> Vec<(NoteSpec, Vec<Measurement>)> {
    let groups = groups(a);
    let Some(i) = general(&groups) else {
        return Vec::new();
    };
    let group = &groups[i];
    vec![(
        NoteSpec {
            id: format!("n:{}/blend.radius", group[0].id),
            text: format!("ALL UNSPECIFIED RADII R{}", fmt(radius(group[0]).unwrap())),
        },
        measurements(group),
    )]
}

/// Sampling along a cylinder's axis produces the same attachment in its end view.
/// Keep the first physical point for each projection, without repeating the leader search.
fn projected_sites(view: &str, sites: impl Iterator<Item = [f64; 3]>) -> Vec<[f64; 3]> {
    let toward = match view {
        "front" => 1,
        "plan" => 2,
        "side" => 0,
        _ => unreachable!(),
    };
    let mut out: Vec<[f64; 3]> = Vec::new();
    for p in sites {
        if !out
            .iter()
            .any(|q| (0..3).all(|k| k == toward || (p[k] - q[k]).abs() < 1e-9))
        {
            out.push(p);
        }
    }
    out
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let a = cx.analysis;
    let groups = groups(a);
    let general = general(&groups);
    let mut wants = Vec::new();
    let mut problems = Vec::new();
    for f in a
        .features
        .iter()
        .filter(|f| f.kind == "blend" && radius(f).is_none())
    {
        problems.push(problem(f, "its record gives no positive finite radius"));
    }
    for (i, group) in groups.iter().enumerate() {
        if Some(i) == general {
            continue;
        }
        let by_view = sites_by_view(a, group);
        let Some((&(view, _axis), sites)) = by_view
            .iter()
            .min_by_key(|(key, sites)| (std::cmp::Reverse(sites.len()), **key))
        else {
            for f in group {
                problems.push(problem(
                    f,
                    "no principal view has a proven attachment on its trimmed blend surface",
                ));
            }
            continue;
        };
        let r = radius(group[0]).unwrap();
        let terms = if group.len() > 1 {
            vec![format!("{}×", group.len()), format!("R{}", fmt(r))]
        } else {
            vec![format!("R{}", fmt(r))]
        };
        wants.push(Want {
            id: want_id(&group[0].id, "blend.radius"), views: vec![view], value: r,
            terms, measurements: measurements(group),
            basis: "equal displayed blend radii are stated once, with an exact measurement claim for every occurrence and a leader on a proven trimmed surface".into(),
            shape: Shape::Callout { sites: projected_sites(view, sites.iter().map(|(p, _)| *p)).into_iter().map(|p| (p, 0.0)).collect() }, turning: None,
        });
    }
    (wants, problems)
}
fn problem(f: &Feature, why: &str) -> Problem {
    Problem {
        id: format!("p:feature_not_dimensioned:{}", f.id),
        code: "feature_not_dimensioned".into(),
        severity: "warning".into(),
        message: format!("{} is not dimensioned: {why}.", f.id),
        subjects: vec![f.id.clone()],
        ..Default::default()
    }
}

/// All possible automatic radius-note identities, independent of layout or omission.
pub(crate) fn note_ids(a: &Analysis) -> Vec<String> {
    let ids: Vec<_> = groups(a)
        .iter()
        .map(|g| format!("n:{}/blend.radius", g[0].id))
        .collect();
    ids
}

/// A default cannot be laid out until every radius exception has a callout.
pub(crate) fn exceptions_unstated(
    a: &Analysis,
    dimensions: &[super::PlannedDimension],
    notes: &[(NoteSpec, Vec<Measurement>)],
) -> bool {
    let Some(value) = notes
        .first()
        .and_then(|(_, ms)| ms.first())
        .map(|m| m.value)
    else {
        return false;
    };
    a.features
        .iter()
        .filter(|f| matches!(f.kind.as_str(), "blend" | "fillet"))
        .any(|f| {
            radius(f).is_none_or(|r| fmt(r) != fmt(value))
                && !dimensions
                    .iter()
                    .any(|d| d.placed.is_some() && d.measurements.iter().any(|m| m.feature == f.id))
        })
}

fn leader_problem(code: &str) -> bool {
    matches!(
        code,
        "annotation_ink_overlap"
            | "annotation_overlap"
            | "annotation_over_view"
            | "annotation_line_over_view"
            | "annotation_leader_crossing"
            | "annotation_outside_area"
            | "annotation_off_sheet"
            | "leader_through_feature"
            | "annotation_leader_long"
            | "annotation_leader_reenters"
    )
}

/// Withdraw unsafe free leaders. A reference point cannot locate an equal-radius group.
pub(crate) fn withdraw_unsafe(
    spec: &crate::model::Spec,
    dimensions: &mut [super::PlannedDimension],
    problems: &mut Vec<Problem>,
) {
    for d in dimensions
        .iter_mut()
        .filter(|d| d.measurements.iter().any(|m| m.parameter == "blend.radius"))
    {
        if matches!(d.state, crate::items::State::Covered { .. }) {
            problems.retain(|p| !p.subjects.contains(&d.id) || !leader_problem(&p.code));
        }
        if d.placed.is_none()
            || spec
                .dimensions
                .get(&d.id)
                .is_some_and(|s| s.place.is_some() || s.tolerance.is_some())
        {
            continue;
        }
        if !problems
            .iter()
            .any(|p| p.subjects.contains(&d.id) && leader_problem(&p.code))
        {
            continue;
        }
        d.placed = None;
        d.state = crate::items::State::NotDrawable {
            reason_code: "blend_callout_dropped".into(),
        };
        problems.retain(|p| !p.subjects.contains(&d.id) || !leader_problem(&p.code));
        problems.push(Problem {
            id: format!("p:blend_callout_dropped:{}", d.id),
            code: "blend_callout_dropped".into(),
            severity: "warning".into(),
            message: format!(
                "{} cannot safely identify its radius group with a leader.",
                d.id
            ),
            subjects: vec![d.id.clone()],
            ..Default::default()
        });
    }
}

fn overridden(spec: &crate::model::Spec, given: &crate::model::Spec, id: &str) -> bool {
    // A bare pin gets a computed carrier in the effective Spec. Only authored placement
    // choices override grouping; inactive entries in the original Spec have no authority.
    spec.dimensions.get(id).is_some_and(|s| {
        s.omit
            || s.tolerance.is_some()
            || s.view.is_some()
            || given
                .dimensions
                .get(id)
                .and_then(|d| d.place.as_ref())
                .is_some_and(|p| {
                    p.view.is_some() || p.side.is_some() || p.lane.is_some() || p.along.is_some()
                })
    })
}

/// The recogniser's fillet/blend distinction is not a second radius requirement on paper.
/// Keep the established fillet leader and give it the equal blend group's exact claims.
pub(crate) fn consolidate(
    dimensions: &mut [super::PlannedDimension],
    spec: &crate::model::Spec,
    given: &crate::model::Spec,
) {
    for i in 0..dimensions.len() {
        if !dimensions[i]
            .measurements
            .iter()
            .any(|m| m.parameter == "blend.radius")
            || overridden(spec, given, &dimensions[i].id)
        {
            continue;
        }
        let Some(j) = dimensions.iter().position(|d| {
            d.placed.is_some()
                && d.measurements
                    .iter()
                    .all(|m| m.parameter == "fillet.radius")
                && !d.measurements.is_empty()
                && fmt(d.value) == fmt(dimensions[i].value)
                && !overridden(spec, given, &d.id)
        }) else {
            continue;
        };
        let ms = dimensions[i].measurements.clone();
        dimensions[j].measurements.extend(ms);
        let n = dimensions[j].measurements.len();
        let was = dimensions[j].terms.clone();
        dimensions[j].terms = vec![format!("{n}×"), format!("R{}", fmt(dimensions[j].value))];
        dimensions[j].label = dimensions[j].terms.join(" ");
        let placed = dimensions[j].placed.as_ref().unwrap();
        let want = Want {
            id: dimensions[j].id.clone(),
            views: Vec::new(),
            value: dimensions[j].value,
            terms: dimensions[j].terms.clone(),
            measurements: dimensions[j].measurements.clone(),
            basis: dimensions[j].basis.clone(),
            shape: Shape::Callout { sites: Vec::new() },
            turning: None,
        };
        dimensions[j].placed = Some(super::placement::relabel(placed, &was, &want));
        dimensions[i].placed = None;
        dimensions[i].state = crate::items::State::Covered {
            by: dimensions[j].id.clone(),
        };
    }
}

/// Only after a default is proven drawable may it replace equal fillet leaders.
pub(crate) fn cover_fillets(
    notes: &mut [crate::notes::PlannedNote],
    dimensions: &mut [super::PlannedDimension],
    spec: &crate::model::Spec,
    given: &crate::model::Spec,
) {
    for note in notes
        .iter_mut()
        .filter(|n| n.placed.is_some() && n.text.starts_with("ALL UNSPECIFIED RADII"))
    {
        let Some(value) = note.measurements.first().map(|m| m.value) else {
            continue;
        };
        for d in dimensions.iter_mut().filter(|d| {
            d.placed.is_some()
                && !d.measurements.is_empty()
                && d.measurements
                    .iter()
                    .all(|m| m.parameter == "fillet.radius")
                && fmt(d.value) == fmt(value)
                && !overridden(spec, given, &d.id)
        }) {
            note.measurements.extend(d.measurements.clone());
            d.placed = None;
            d.state = crate::items::State::Covered {
                by: note.id.clone(),
            };
        }
    }
}
