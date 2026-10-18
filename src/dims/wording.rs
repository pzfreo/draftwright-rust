use super::{Omitted, Want};
use crate::{
    label_text::Style,
    model::{Invalid, Spec},
};

type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

pub(super) fn apply(
    spec: &Spec,
    kept: &mut [Kept],
    given: &mut [Vec<String>],
) -> Result<(), Invalid> {
    for ((w, ..), from_spec) in kept.iter_mut().zip(given) {
        if matches!(w.shape, super::Shape::Callout { .. }) {
            w.terms = crate::callouts::wrap(&w.terms);
        }
        let Some(entry) = spec.dimensions.get(&w.id) else {
            continue;
        };
        if entry.text.is_none() && entry.style.is_none() {
            continue;
        }
        let old = w.terms.join(" ");
        let reference = old.starts_with('(') && old.ends_with(')');
        if entry.style == Some(Style::Reference) && super::is_basic(&w.measurements) {
            return Err(Invalid {
                message: format!(
                    "{} is a basic PMI dimension; remove its reference style or correct the source PMI.",
                    w.id
                ),
                choices: vec!["basic".into()],
            });
        }
        // Keep the reference enclosure outside all authored wording.
        if reference {
            w.terms = vec![old[1..old.len() - 1].into()];
        }
        if let Some(text) = &entry.text {
            w.terms = text.terms(&w.id, &w.terms)?;
            from_spec.push("text".into());
            w.basis += "; label wording from_spec: text; generated measured terms retained";
        }
        if entry.style == Some(Style::Reference) || (reference && entry.style.is_none()) {
            w.terms = vec![format!("({})", w.terms.join(" "))];
        }
        if let Some(style) = entry.style {
            let field = match style {
                Style::Reference => "style.reference",
                Style::Basic => "style.basic",
            };
            from_spec.push(field.into());
            w.basis += &format!("; from_spec: {field}");
        }
        if matches!(w.shape, super::Shape::Callout { .. }) {
            w.terms = crate::callouts::wrap(&w.terms);
        }
    }
    Ok(())
}

pub(super) fn check_covered(
    spec: &Spec,
    kept: &[Kept],
    omitted: &[Omitted],
) -> Result<(), Invalid> {
    for (id, entry) in &spec.dimensions {
        if !entry.omit
            && (entry.text.is_some() || entry.style.is_some())
            && !kept.iter().any(|(w, ..)| &w.id == id)
            && let Some(o) = omitted.iter().find(|o| &o.id == id)
        {
            let choices = match &o.state {
                crate::items::State::Covered { by } => vec![by.clone()],
                _ => Vec::new(),
            };
            return Err(Invalid {
                message: format!(
                    "Label override for {id} has no separate printed label: {}. Apply the text/style to its owning dimension instead, or remove the override.",
                    o.basis
                ),
                choices,
            });
        }
    }
    Ok(())
}
