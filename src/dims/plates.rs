//! Plate and wall thicknesses, routed as Python's `_thin_profiles`: z to the
//! front's left strip, y above the side, x below the front. The native inventory
//! also retains single-axis walls; proven alternative statements cover them.

use super::{Context, Measurement, Omitted, Planner, Problem, Shape, Want, fmt, want_id};
use crate::items::State;
use crate::lint::plates;
use crate::model::Analysis;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["plate"],
    wants,
};

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for f in cx.analysis.features.iter().filter(|f| f.kind == "plate") {
        let p = match plates::read(cx.analysis, f) {
            Ok(p) => p,
            Err(why) => {
                problems.push(Problem {
                    id: format!("p:feature_not_dimensioned:{}", f.id),
                    code: "feature_not_dimensioned".into(),
                    severity: "warning".into(),
                    message: format!("{} is not dimensioned: {why}.", f.id),
                    subjects: vec![f.id.clone()],
                    ..Default::default()
                });
                continue;
            }
        };
        let k = p.axis;
        let value = p.thickness();
        out.push(Want {
            id: want_id(&f.id, "thickness.length"),
            views: match k {
                0 => vec!["front", "plan"],
                1 => vec!["side", "plan"],
                _ => vec!["front", "side"],
            },
            value,
            terms: vec![fmt(value)],
            measurements: vec![Measurement {
                feature: f.id.clone(),
                parameter: "thickness.length".into(),
                value,
            }],
            basis: "a plate's thickness, face to face in the view showing its thin profile".into(),
            shape: Shape::Linear {
                a: p.span[0],
                b: p.span[1],
                axis: k,
                a_edge: true,
                b_edge: true,
                order: 0,
            },
            turning: None,
        });
    }
    (out, problems)
}

type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

/// Covered items keep their IDs. Only actually retained owner measurements count;
/// omitting an alternative restores the plate's own dimension.
pub(super) fn consolidate(a: &Analysis, kept: &mut Vec<Kept>, omitted: &mut Vec<Omitted>) {
    let mut gone = Vec::new();
    for (i, (w, ..)) in kept.iter().enumerate() {
        let Some(m) = w
            .measurements
            .iter()
            .find(|m| m.parameter == "thickness.length")
        else {
            continue;
        };
        let Some(f) = a.features.iter().find(|f| f.id == m.feature) else {
            continue;
        };
        let intrinsic = plates::exclusion(a, f);
        let alternative = plates::alternatives(a, f).into_iter().find_map(|terms| {
            // In a base-and-walls construction the explicit thickness is the
            // characteristic size; subtracting two ladder heights is secondary.
            if crate::lint::channels::multi_axis(a)
                && terms.iter().any(|t| t.feature == "step_level")
            {
                return None;
            }
            let owners: Option<Vec<_>> = terms
                .iter()
                .map(|t| {
                    kept.iter()
                        .find(|(owner, ..)| {
                            owner.id != w.id
                                && owner.measurements.iter().any(|m| {
                                    m.feature == t.feature
                                        && m.parameter == t.parameter
                                        && (m.value - t.value).abs() <= 1e-3
                                })
                        })
                        .map(|(owner, ..)| owner.id.clone())
                })
                .collect();
            owners.filter(|o| !o.is_empty())
        });
        if intrinsic.is_some() || alternative.is_some() {
            let by = alternative
                .as_ref()
                .and_then(|o| o.first())
                .cloned()
                .unwrap_or_else(|| intrinsic.as_ref().unwrap().by.clone());
            omitted.push(Omitted {
                id: w.id.clone(),
                label: w.terms.join(" "),
                basis: intrinsic.as_ref().map_or_else(
                    || {
                        format!(
                            "conveyed by {by}: the proven interval is already stated; dependencies: {}",
                            alternative.unwrap().join(", ")
                        )
                    },
                    |proof| format!("conveyed by {by}: recognised ownership ({})", proof.reason),
                ),
                state: State::Covered { by },
            });
            gone.push(i);
        }
    }
    let mut i = 0;
    kept.retain(|_| {
        let retain = !gone.contains(&i);
        i += 1;
        retain
    });
}

/// Equal thicknesses on one body share a counted statement, including walls on
/// different axes. Distinct authored bands stay distinct; unrelated bodies never join.
pub(super) fn share(
    a: &Analysis,
    kept: &mut Vec<Kept>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<Omitted>,
) {
    let body = |w: &Want| {
        let m = w
            .measurements
            .iter()
            .find(|m| m.parameter == "thickness.length")?;
        a.features
            .iter()
            .find(|f| f.id == m.feature)?
            .parameters
            .get("body_key")
            .filter(|key| !key.is_null())
            .cloned()
    };
    let mut gone = std::collections::BTreeSet::new();
    for i in 0..kept.len() {
        if gone.contains(&i) {
            continue;
        }
        let Some(key) = body(&kept[i].0) else {
            continue;
        };
        let mut group: Vec<_> = (i..kept.len())
            .filter(|j| {
                !gone.contains(j)
                    && body(&kept[*j].0).as_ref() == Some(&key)
                    && (kept[*j].0.value - kept[i].0.value).abs() <= 1e-6
                    && kept[*j].0.terms == kept[i].0.terms
            })
            .collect();
        if group.len() < 2 {
            continue;
        }
        group.sort_by(|x, y| {
            kept[*x]
                .1
                .cmp(&kept[*y].1)
                .then(kept[*x].0.id.cmp(&kept[*y].0.id))
        });
        let winner = group[0];
        let by = kept[winner].0.id.clone();
        let count = group.len();
        for loser in group.into_iter().skip(1) {
            omitted.push(Omitted {
                id: kept[loser].0.id.clone(),
                label: kept[loser].0.terms.join(" "),
                basis: format!(
                    "conveyed by {by}: equal plate thicknesses on one body are stated once, counted"
                ),
                state: State::Covered { by: by.clone() },
            });
            let claims = kept[loser].0.measurements.clone();
            kept[winner].0.measurements.extend(claims);
            gone.insert(loser);
        }
        kept[winner].0.terms.insert(0, format!("{count}×"));
    }
    let mut i = 0;
    kept.retain(|_| {
        let keep = !gone.contains(&i);
        i += 1;
        keep
    });
    let mut i = 0;
    given.retain(|_| {
        let keep = !gone.contains(&i);
        i += 1;
        keep
    });
}

pub(super) fn prefer_readable(
    wants: &mut [Want],
    frames: &std::collections::BTreeMap<&str, super::Frame>,
    spec: Option<&crate::model::Spec>,
) {
    for w in wants {
        if !w
            .measurements
            .iter()
            .any(|m| m.parameter == "thickness.length")
            || !matches!(w.shape, Shape::Linear { axis: 1, .. })
            || spec
                .and_then(|s| s.dimensions.get(&w.id))
                .is_some_and(|entry| {
                    entry.view.is_some() || entry.place.as_ref().is_some_and(|p| p.view.is_some())
                })
        {
            continue;
        }
        // A wall narrower than two arrowheads cannot carry inward arrows in
        // the end profile. The plan shows the same interval in its clear left band.
        if frames
            .get("side")
            .is_some_and(|f| w.value * f.scale() < 2.0 * super::ARROW)
            && frames.contains_key("plan")
        {
            w.views.retain(|v| *v != "plan");
            w.views.insert(0, "plan");
        }
    }
}

pub(super) fn default_terms(a: &Analysis, wants: &mut [Want], chosen: &[Option<&'static str>]) {
    let mut kept: Vec<_> = wants
        .iter()
        .zip(chosen)
        .map(|(w, v)| (w.clone(), true, *v, Vec::new()))
        .collect();
    let mut omitted = Vec::new();
    consolidate(a, &mut kept, &mut omitted);
    let mut given = vec![Vec::new(); kept.len()];
    share(a, &mut kept, &mut given, &mut omitted);
    for (w, ..) in kept {
        if let Some(raw) = wants.iter_mut().find(|raw| raw.id == w.id) {
            raw.terms = w.terms;
        }
    }
}
