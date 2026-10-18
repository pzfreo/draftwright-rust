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
            if let Some((physical, suffix)) = original.split_once(&raw_terms) {
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
            let text = owned
                .iter()
                .map(|o| o.suffix.as_str())
                .collect::<Vec<_>>()
                .join(" \n ");
            w.terms = text
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
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
                w.shape = Shape::Callout {
                    arcs: Vec::new(),
                    sites: vec![(a, 0.0), (b, 0.0)],
                };
            }
            for o in owned {
                let style = crate::callouts::style(spec, &o.owner);
                let referred = m.tags.iter().any(|(s, _)| s == &o.source);
                w.basis += &format!(
                    "; callouts/style {style:?}: {}{}",
                    o.source,
                    if referred {
                        " uses a source-bound manufacturing note (secondary terms or compact text exceeding two lines)"
                    } else {
                        " uses compact identity and drawing evidence for extent"
                    }
                );
            }
        }
        if matches!(w.shape, Shape::Callout { .. }) {
            w.terms = crate::callouts::wrap(&w.terms);
        }
    }
}
