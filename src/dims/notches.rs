//! A V is specified by included angle, theoretical sharp depth, and mouth position.
use super::{AXES, Context, Measurement, Planner, Problem, Shape, Want, end_on, fmt, want_id};

/// Profile requirements belong to the part envelope; the support stamp distinguishes
/// physical corners without pretending that the provider published a notch feature.
pub(super) fn id(feature: &str, parameter: &str) -> String {
    let role = parameter.strip_prefix("location.length.").map_or_else(
        || parameter.to_owned(),
        |axis| format!("notch_location_{axis}.length"),
    );
    let support = feature
        .strip_prefix("notch:")
        // Invariant: Notch requirements are emitted only after validating their source profile.
        .expect("source-proven notch")
        .replace(':', "_");
    want_id("envelope", &format!("{role}.{support}"))
}

pub(super) const PLANNER: Planner = Planner { kinds: &[], wants };
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let notches = crate::notches::requirements(cx.analysis);
    // Keep the existing front corridor when it also carries GD&T. Splitting
    // these stations across views removes a front tier and can displace a
    // control frame into a datum's otherwise clear compact route.
    let front_gdt = !notches.is_empty()
        && crate::pmi_lowering::lower(cx.analysis, &|_| true)
            .0
            .iter()
            .any(|d| {
                d.view == "front"
                    && matches!(
                        &d.glyph,
                        crate::pmi_lowering::Glyph::ControlFrame { .. }
                            | crate::pmi_lowering::Glyph::Datum { .. }
                    )
            });
    for n in notches {
        let view = end_on(n.axis);
        let value = n.angle.degrees();
        out.push(Want {
            id: id(&n.id,"notch.angle"), views:vec![view], value,
            terms:vec![format!("{}°",fmt(value))],
            measurements:vec![Measurement {feature:n.id.clone(),parameter:"notch.angle".into(),value}],
            basis:"a V-notch's included angle, in the normal view that shows it true; rounded roots extend to the theoretical sharp apex".into(),
            shape:Shape::Angular {angle:(&n.angle).into(), alternates: Vec::new()}, turning:None,
        });
        let k = n.depth_axis;
        let value = n.depth();
        out.push(Want {
            id:id(&n.id,"notch_depth.length"),views:vec![view],value,terms:vec![fmt(value)],
            measurements:vec![Measurement {feature:n.id.clone(),parameter:"notch_depth.length".into(),value}],
            basis:"V-notch depth from the mouth to the theoretical sharp apex; the included angle and depth define the mouth width".into(),
            shape:Shape::Linear {a:n.mouth,b:n.angle.vertex,axis:k,a_edge:false,b_edge:false,order:0},turning:None,
        });
        // Locate the mouth centre from the part's minimum datums; consolidation shares
        // coincident stations with existing dimensions and schemes can take ownership.
        for k in [n.width_axis, n.depth_axis] {
            let b = n.mouth;
            let mut a = b;
            a[k] = cx.analysis.envelope[0][k];
            let value = (b[k] - a[k]).abs();
            if value < 1e-6 {
                continue;
            }
            let parameter = format!("location.length.{}", AXES[k]);
            out.push(Want {
                id: id(&n.id, &parameter),
                // An X station is equally true in the aligned front projection.
                // Keep the plan's angular sector free of long datum witnesses; an
                // explicit scheme or placement can still choose the normal view.
                views: if n.axis == 2 && k == 0 {
                    if !front_gdt
                        && n.mouth[k]
                            < 0.5 * (cx.analysis.envelope[0][k] + cx.analysis.envelope[1][k])
                    {
                        vec![view, "front"]
                    } else {
                        vec!["front", view]
                    }
                } else if n.axis == 2 && k == 1 {
                    vec!["side", view]
                } else {
                    vec![view]
                },
                value,
                terms: vec![fmt(value)],
                measurements: vec![Measurement {
                    feature: n.id.clone(),
                    parameter,
                    value,
                }],
                basis: "the V-notch mouth centre located from the part's minimum datum".into(),
                shape: Shape::Linear {
                    a,
                    b,
                    axis: k,
                    a_edge: true,
                    b_edge: false,
                    order: 1,
                },
                turning: None,
            });
        }
    }
    // A bevel is an edge treatment: its C/size-and-angle callout offsets the drawn
    // supports. Its provider corner is the midpoint of a run, not a design datum;
    // do not invent a station requirement there. Explicit schemes can still name it.
    (out, vec![])
}

/// Only bounded profile angles yield to their source PMI, retaining its tolerance
/// and implicit degree notation. Other angular planning keeps its existing policy.
pub(super) fn share_pmi(
    analysis: &crate::model::Analysis,
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<super::Omitted>,
) {
    let mut gone = std::collections::BTreeSet::new();
    for n in crate::notches::requirements(analysis) {
        let sources = crate::lint::notch_evidence::pmi_angles(analysis, &n);
        let Some(p) = kept.iter().position(|(w, ..)| {
            matches!(w.shape, Shape::Angular { .. })
                && w.measurements.iter().any(|m| {
                    sources.contains(&m.feature)
                        && matches!(m.parameter.as_str(), "angular" | "angular.basic")
                })
        }) else {
            continue;
        };
        if kept[p].2 != Some(end_on(n.axis)) {
            continue;
        }
        // The live source statement proves the role even when the user has
        // omitted its automatic alias. Only a live, true-view PMI can own it.
        if !kept[p]
            .0
            .measurements
            .iter()
            .any(|m| m.feature == n.id && m.parameter == "notch.angle")
        {
            kept[p].0.measurements.push(Measurement {
                feature: n.id.clone(),
                parameter: "notch.angle".into(),
                value: n.angle.degrees(),
            });
            kept[p].0.basis +=
                "; also states the bounded V-notch angle on its source support faces";
        }
        let Some(i) = kept.iter().position(|(w, default, view, _)| {
            *default && *view == kept[p].2 && w.id == id(&n.id, "notch.angle")
        }) else {
            continue;
        };
        if !given[i].is_empty() {
            continue;
        }
        let owner = kept[p].0.id.clone();
        omitted.push(super::Omitted {
            id: kept[i].0.id.clone(), label: kept[i].0.terms.join(" "),
            basis: format!("conveyed by {owner}: the source PMI states this bounded notch angle on the same physical support faces in the true normal view"),
            state: crate::items::State::Covered { by: owner },
        });
        gone.insert(i);
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

/// Reuse an already printed first bevel leg instead of adding the same depth line.
pub(super) fn share_bevel_depths(
    analysis: &crate::model::Analysis,
    kept: &mut Vec<(Want, bool, Option<&'static str>, Vec<&'static str>)>,
    given: &mut Vec<Vec<String>>,
    omitted: &mut Vec<super::Omitted>,
) {
    let mut gone = std::collections::BTreeSet::new();
    for n in crate::notches::requirements(analysis) {
        let alias = id(&n.id, "notch_depth.length");
        // A source PMI interval or another truthful owner has first claim.
        if kept.iter().any(|(w, ..)| {
            w.id != alias
                && w.measurements
                    .iter()
                    .any(|m| m.feature == n.id && m.parameter == "notch_depth.length")
        }) {
            continue;
        }
        let sources = crate::lint::notch_evidence::bevel_depths(analysis, &n);
        let Some(p) = kept.iter().position(|(w, _, view, _)| {
            *view == Some(end_on(n.axis))
                && w.measurements.iter().any(|m| {
                    sources.contains(&m.feature)
                        && matches!(
                            m.parameter.as_str(),
                            "chamfer.length" | "oriented_chamfer.length"
                        )
                })
        }) else {
            continue;
        };
        kept[p].0.measurements.push(Measurement {
            feature: n.id.clone(),
            parameter: "notch_depth.length".into(),
            value: n.depth(),
        });
        kept[p].0.basis += "; its first support leg also states the bounded V-notch depth";
        let Some(i) = kept
            .iter()
            .position(|(w, default, view, _)| *default && *view == kept[p].2 && w.id == alias)
        else {
            continue;
        };
        if !given[i].is_empty() {
            continue;
        }
        let owner = kept[p].0.id.clone();
        omitted.push(super::Omitted {
            id: alias, label: kept[i].0.terms.join(" "),
            basis: format!("conveyed by {owner}: the existing bevel first leg spans this same physical notch depth"),
            state: crate::items::State::Covered { by: owner },
        });
        gone.insert(i);
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
