//! Local fixes retain sheet intent, never page coordinates or item inventories.
use crate::model::{self, Invalid, Plan, Spec};

fn unavailable() -> Invalid {
    Invalid {
 path: None,
        message: "This edit cannot keep the retained sheet layout. No local fix is available for this edit; remove sheet/keep_layout to allow the whole sheet to be laid out again, including hand-moved placements.".into(),
        choices: Vec::new(),
    }
}

/// Apply-time information is deliberately ephemeral: it never enters the Spec.
pub(crate) fn verify(before: &Plan, after: &Plan) -> Result<(), Invalid> {
    let items = crate::items::items(after);
    let shown: std::collections::BTreeSet<_> = items
        .iter()
        .filter(|i| i.state == crate::items::State::Shown)
        .map(|i| i.id.as_str())
        .collect();
    let conveyed = |id: &str| {
        shown.contains(id) || items.iter().any(|i| i.id == id
        && matches!(&i.state, crate::items::State::Covered { by } if shown.contains(by.as_str())))
    };
    let required = crate::items::items(before)
        .into_iter()
        .filter(|i| i.state == crate::items::State::Shown)
        .map(|i| i.id)
        .chain(
            before
                .dimensions
                .iter()
                .filter(|d| {
                    matches!(&d.state,
            crate::items::State::Covered { by } if by == "t:radius-centres")
                })
                .map(|d| d.id.clone()),
        );
    if before.sheet.size != after.sheet.size
        || before.sheet.scale != after.sheet.scale
        || before.sheets.len() != after.sheets.len()
        || required.into_iter().any(|id| !conveyed(&id))
        || before.radius_table.as_ref().is_some_and(|table| {
            after
                .radius_table
                .as_ref()
                .is_none_or(|next| next.rows != table.rows)
        })
    {
        return Err(unavailable());
    }
    Ok(())
}

pub(crate) fn forced(spec: &Spec) -> Result<Spec, Invalid> {
    if spec.sheet.size.is_none()
        || spec.sheet.scale.is_none()
        || spec.sheets.as_ref().and_then(|s| s.max) != Some(1)
    {
        return Err(unavailable());
    }
    Ok(spec.clone())
}

/// An edit report compares the actual before/after sheets, not two candidates in one search.
pub(crate) fn notice(before: &Plan, after: &Plan) -> Option<model::Problem> {
    let sheets = |p: &Plan| {
        std::iter::once(&p.sheet)
            .chain(&p.sheets)
            .map(|s| (s.size.clone(), s.scale.clone()))
            .collect::<Vec<_>>()
    };
    let moved: Vec<_> = after
        .views
        .iter()
        .filter(|v| {
            before.views.iter().any(|old| {
                old.id == v.id
                    && (old.at != v.at
                        || old.area != v.area
                        || old.scale != v.scale
                        || old.sheet != v.sheet
                        || old.direction != v.direction
                        || old.up != v.up)
            })
        })
        .map(|v| v.id.clone())
        .collect();
    if sheets(before) != sheets(after) || !moved.is_empty() {
        Some(model::Problem {
            id: "p:layout_replanned:edit".into(),
            code: "layout_replanned".into(),
            severity: "info".into(),
            message: format!(
                "The sheet was re-planned: {} at {} ({} sheet(s)) → {} at {} ({} sheet(s)). Moved views: {}. {}",
                before.sheet.size,
                before.sheet.scale,
                1 + before.sheets.len(),
                after.sheet.size,
                after.sheet.scale,
                1 + after.sheets.len(),
                if moved.is_empty() {
                    "none".into()
                } else {
                    moved
                        .iter()
                        .map(|id| crate::wording::view(id.strip_prefix("v:").unwrap_or(id)))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                crate::fixes::SHEET_CONSEQUENCE
            ),
            subjects: std::iter::once("sheet".into()).chain(moved).collect(),
            ..Default::default()
        })
    } else {
        None
    }
}
