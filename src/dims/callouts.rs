//! Apply manufacturing presentation before routing and measuring any dimension demand.
use super::{Shape, Want};
use crate::model::{Analysis, Spec};

pub(super) fn apply(analysis: &Analysis, spec: &Spec, wants: &mut [Want]) {
    let raw = crate::pmi_lowering::manufacturing(analysis);
    let m = crate::pmi_lowering::manufacturing_for(analysis, spec);
    for w in wants {
        let owned: Vec<_> = m
            .owned
            .iter()
            .filter(|o| {
                w.measurements
                    .iter()
                    .any(|v| v.feature == o.id && v.parameter == o.aspect)
            })
            .collect();
        if !owned.is_empty() {
            let count = w.terms.first().filter(|s| s.ends_with('×')).cloned();
            let original = w.terms.join(" ");
            let raw_terms = raw.terms(&owned[0].owner).0.join(" ");
            let mut riders = String::new();
            let mut qualified_bore = None;
            let mut physical_diameter = None;
            if let Some((physical, suffix)) = original.split_once(&raw_terms) {
                if owned.iter().any(|o| o.knurl_values().is_some()) {
                    // The knurl's note moves manufacturing wording only. The size's
                    // own PMI band and qualifier must stay beside its diameter.
                    physical_diameter = Some(
                        physical
                            .trim()
                            .strip_prefix(count.as_deref().unwrap_or(""))
                            .unwrap_or(physical.trim())
                            .trim()
                            .to_string(),
                    );
                }
                if owned.iter().any(|o| o.thread().is_some_and(|t| t.internal)) {
                    let bore = physical
                        .trim()
                        .strip_prefix(count.as_deref().unwrap_or(""))
                        .unwrap_or(physical.trim())
                        .trim();
                    let end = bore.find(['⌴', '⌵']).unwrap_or(bore.len());
                    let bore = bore[..end].trim();
                    let qualified = bore.split_whitespace().any(|word| {
                        let value = word.trim_start_matches(['ø', '⌀', 'Ø']);
                        !matches!(word, "↧" | "THRU") && value.parse::<f64>().is_err()
                    });
                    let specified = spec
                        .features
                        .get(&owned[0].owner)
                        .is_some_and(|f| f.fit.is_some())
                        || spec
                            .dimensions
                            .get(&w.id)
                            .is_some_and(|d| d.fit.is_some() || d.tolerance.is_some());
                    let precise = spec.precision.contains_key(&w.id)
                        || w.measurements.iter().any(|m| {
                            matches!(m.parameter.as_str(), "bore.diameter" | "bore.depth")
                                && spec
                                    .precision
                                    .contains_key(&format!("d:{}/{}", m.feature, m.parameter))
                        });
                    if qualified || specified || precise {
                        qualified_bore = Some(bore.to_string());
                    }
                }
                // Counterbores, spotfaces and countersinks remain independent physical
                // requirements. Pattern shape/pitch/face riders follow the manufacturing text.
                if let Some(start) = physical.find(['⌴', '⌵']) {
                    riders += physical[start..].trim();
                }
                if !suffix.trim().is_empty() {
                    if !riders.is_empty() {
                        riders.push(' ');
                    }
                    riders += suffix.trim();
                }
            }
            let mut text = owned
                .iter()
                .map(|o| o.suffix.as_str())
                .collect::<Vec<_>>()
                .join(" \n ");
            if let Some(diameter) = physical_diameter
                && let Some((identity, rest)) = text.split_once(' ')
                && identity.starts_with(['ø', '⌀', 'Ø'])
            {
                // A common MAX already on the size also qualifies MAX AFTER KNURL.
                let rest = if diameter.ends_with(" MAX") {
                    rest.strip_prefix("MAX ").unwrap_or(rest)
                } else {
                    rest
                };
                text = format!("{diameter} {rest}");
            }
            w.terms = text
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if let Some(bore) = qualified_bore {
                if crate::callouts::style(spec, &owned[0].owner) == crate::callouts::Style::Full {
                    let suffix = raw
                        .on(&owned[0].owner)
                        .iter()
                        .map(|o| o.suffix.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    w.terms = format!("{bore} {suffix}")
                        .split_whitespace()
                        .map(str::to_string)
                        .collect();
                } else {
                    w.terms.truncate(
                        w.terms
                            .iter()
                            .position(|s| s == crate::callouts::BREAK)
                            .unwrap_or(w.terms.len()),
                    );
                    w.terms.push(crate::callouts::BREAK.into());
                    w.terms.extend(bore.split_whitespace().map(str::to_string));
                }
                w.basis += "; retains the source tapping-drill fit on its own drill line";
            }
            if !riders.is_empty() {
                w.terms
                    .extend(riders.split_whitespace().map(str::to_string));
            }
            if let Some(count) = count {
                w.terms.insert(0, count);
            }
            // Manufacturing requirements always read horizontally, including a lone knurl
            // on an overall diameter. Its measured diameter remains claimed by this leader.
            if let Shape::Linear { a, b, .. } = w.shape {
                let frame = crate::bridge::holes::frame(analysis);
                let sites: Vec<_> = owned
                    .iter()
                    .filter_map(|o| analysis.features.iter().find(|f| f.id == o.owner))
                    .filter_map(|f| crate::lint::turned::band(&frame, f).ok())
                    .flat_map(|band| {
                        let mut mid = band.on;
                        mid[band.axis] = (band.lo + band.hi) / 2.0;
                        (0..3).filter(move |k| *k != band.axis).flat_map(move |k| {
                            [-0.5 * band.diameter, 0.5 * band.diameter].map(|r| {
                                let mut p = mid;
                                p[k] += r;
                                (p, 0.0)
                            })
                        })
                    })
                    .collect();
                w.shape = Shape::Callout {
                    arcs: Vec::new(),
                    sites: if sites.is_empty() {
                        vec![(a, 0.0), (b, 0.0)]
                    } else {
                        sites
                    },
                };
            }
            for o in owned {
                let style = crate::callouts::style(spec, &o.owner);
                w.basis += &format!(
                    "; callouts/style {style:?}: {} {}",
                    o.source,
                    o.presentation_reason()
                );
            }
        }
        if matches!(w.shape, Shape::Callout { .. }) {
            w.terms = crate::callouts::wrap(&w.terms);
        }
    }
}
