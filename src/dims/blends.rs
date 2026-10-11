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
            text: format!(
                "ALL UNSPECIFIED BLEND RADII R{}",
                fmt(radius(group[0]).unwrap())
            ),
        },
        measurements(group),
    )]
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
            shape: Shape::Callout { sites: sites.iter().map(|(p,_)| (*p,0.0)).collect() }, turning: None,
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
    let mut ids: Vec<_> = groups(a)
        .iter()
        .map(|g| format!("n:{}/blend.radius", g[0].id))
        .collect();
    ids.push("n:blend-sites/reference".into());
    ids
}

/// An omitted exception must not be mistaken for the blanket default radius.
pub(crate) fn qualify_notes(
    a: &Analysis,
    spec: &crate::model::Spec,
    dimensions: &[super::PlannedDimension],
    notes: &mut [(NoteSpec, Vec<Measurement>)],
) {
    let stated: std::collections::BTreeSet<_> = dimensions
        .iter()
        .filter(|d| d.placed.is_some())
        .flat_map(|d| d.measurements.iter().map(|m| m.feature.clone()))
        .chain(
            notes
                .iter()
                .filter(|(n, _)| {
                    !spec.annotations.get(&n.id).is_some_and(|e| e.omit)
                        && (spec.dimension_source != crate::model::DimensionSource::Authored
                            || spec.annotations.contains_key(&n.id))
                })
                .flat_map(|(_, ms)| ms.iter().map(|m| m.feature.clone())),
        )
        .collect();
    if a.features
        .iter()
        .any(|f| f.kind == "blend" && !stated.contains(&f.id))
    {
        for (n, ms) in notes
            .iter_mut()
            .filter(|(n, _)| n.text.starts_with("ALL UNSPECIFIED"))
        {
            n.text = format!(
                "R{}, {} BLENDS; OTHER BLEND RADII UNSTATED",
                fmt(ms[0].value),
                ms.len()
            );
        }
    }
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

/// A free radius leader that crosses ink or features is replaced by a located structured
/// note. Reference coordinates identify an actual surface point, from the model envelope's
/// left/front/bottom faces; one representative point locates the group, which retains every occurrence's exact credit.
pub(crate) fn located_notes(
    a: &Analysis,
    spec: &crate::model::Spec,
    dimensions: &mut [super::PlannedDimension],
    problems: &mut Vec<Problem>,
) -> Vec<(NoteSpec, Vec<Measurement>)> {
    let mut out = Vec::new();
    for d in dimensions
        .iter_mut()
        .filter(|d| d.measurements.iter().any(|m| m.parameter == "blend.radius"))
    {
        if d.placed.is_none()
            || spec
                .dimensions
                .get(&d.id)
                .is_some_and(|s| s.place.is_some() || s.tolerance.is_some())
        {
            continue;
        }
        let obstructed = problems
            .iter()
            .any(|p| p.subjects.contains(&d.id) && leader_problem(&p.code));
        if !obstructed {
            continue;
        }
        let Some(site) = d.measurements.iter().find_map(|m| {
            a.blend_sites
                .get(&m.feature)
                .and_then(|sites| sites.first())
        }) else {
            continue;
        };
        let p = [0, 1, 2].map(|k| crate::lint::parameters::fmt(site[k] - a.envelope[0][k]));
        let id = d.id.replacen("d:", "n:", 1);
        out.push((
            NoteSpec {
                id: id.clone(),
                text: format!(
                    "R{}, {} {}; REF SITE XYZ ({})",
                    fmt(d.value),
                    d.measurements.len(),
                    if d.measurements.len() == 1 {
                        "BLEND"
                    } else {
                        "BLENDS"
                    },
                    p.join(", ")
                ),
            },
            d.measurements.clone(),
        ));
        d.placed = None;
        d.state = crate::items::State::Covered { by: id };
        d.basis += "; a crossing leader is replaced by a structured note identifying the equal-radius group by a representative reference surface point";
        problems.retain(|p| !p.subjects.contains(&d.id) || !leader_problem(&p.code));
    }
    if !out.is_empty() {
        out.push((
            NoteSpec {
                id: "n:blend-sites/reference".into(),
                text: "BLEND REFERENCE SITES: X/Y/Z FROM LEFT/FRONT/BOTTOM OF ENVELOPE".into(),
            },
            Vec::new(),
        ));
    }
    out
}
