//! A local remedy carries the frame on which its placement was checked.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{self, Invalid, Plan, PlannedView, Spec};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub size: String,
    pub scale: String,
    pub views: BTreeMap<String, Frame>,
    /// Shown items, including table-covered radius groups, must still reach the drawing.
    pub required: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub at: [f64; 2],
    pub area: [f64; 4],
    pub scale: f64,
}

fn unavailable() -> Invalid {
    Invalid {
        message: "This edit cannot keep the retained sheet layout. No local fix is available for this edit; remove sheet/keep_layout to allow the whole sheet to be laid out again, including hand-moved placements.".into(),
        choices: Vec::new(),
    }
}

impl Layout {
    pub(crate) fn capture(plan: &Plan) -> Option<Self> {
        // Allocation can transport auxiliary views and tables after the single-sheet solve.
        // Until that can be constrained too, those remedies must say sheet scope.
        if !plan.sheets.is_empty() {
            return None;
        }
        Some(Self {
            size: plan.sheet.size.clone(),
            scale: plan.sheet.scale.clone(),
            required: {
                let mut ids: Vec<_> = crate::items::items(plan)
                    .into_iter()
                    .filter(|i| i.state == crate::items::State::Shown)
                    .map(|i| i.id)
                    .collect();
                // The table's centre coordinates are information beyond its radius callout.
                // Keep its exact automatic groups too, so freeing a leader corridor cannot
                // silently exchange the table for a representative callout.
                ids.extend(
                    plan.dimensions
                        .iter()
                        .filter(|d| {
                            matches!(&d.state,
                    crate::items::State::Covered { by } if by == "t:radius-centres")
                        })
                        .map(|d| d.id.clone()),
                );
                ids
            },
            views: plan
                .views
                .iter()
                .map(|v| {
                    (
                        v.id.clone(),
                        Frame {
                            at: v.at,
                            area: v.area,
                            scale: v.scale,
                        },
                    )
                })
                .collect(),
        })
    }

    pub(crate) fn check(&self) -> Result<(), Invalid> {
        if !crate::layout::PAGES.iter().any(|p| p.0 == self.size)
            || !crate::layout::SCALES
                .iter()
                .any(|s| model::scale_name(*s) == self.scale)
            || self.required.len() > 16384
            || self.views.is_empty()
            || self.views.len() > 32
            || self.views.iter().any(|(id, v)| {
                !id.starts_with("v:")
                    || !v.scale.is_finite()
                    || v.scale <= 0.0
                    || v.scale > 1000.0
                    || v.at
                        .iter()
                        .chain(&v.area)
                        .any(|x| !x.is_finite() || x.abs() > 11890.0)
                    || v.area[0] >= v.area[2]
                    || v.area[1] >= v.area[3]
            })
        {
            return Err(unavailable());
        }
        Ok(())
    }

    pub(crate) fn restore(&self, views: &mut [PlannedView]) -> Result<(), Invalid> {
        if views.len() != self.views.len() {
            return Err(unavailable());
        }
        for v in views {
            let f = self.views.get(&v.id).ok_or_else(unavailable)?;
            (v.at, v.area, v.scale) = (f.at, f.area, f.scale);
        }
        Ok(())
    }

    pub(crate) fn verify(&self, plan: &Plan) -> Result<(), Invalid> {
        let items = crate::items::items(plan);
        let shown: std::collections::BTreeSet<_> = items
            .iter()
            .filter(|i| i.state == crate::items::State::Shown)
            .map(|i| i.id.as_str())
            .collect();
        let conveyed = |id: &str| {
            shown.contains(id) || items.iter().any(|i| i.id == id
            && matches!(&i.state, crate::items::State::Covered { by } if shown.contains(by.as_str())))
        };
        if plan.sheet.size != self.size
            || plan.sheet.scale != self.scale
            || !plan.sheets.is_empty()
            || plan.views.len() != self.views.len()
            || self.required.iter().any(|id| !conveyed(id))
            || plan.views.iter().any(|v| {
                v.sheet != 1
                    || self
                        .views
                        .get(&v.id)
                        .is_none_or(|f| f.at != v.at || f.scale != v.scale)
            })
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

pub(crate) fn forced(spec: &Spec) -> Result<Spec, Invalid> {
    let mut fixed = spec.clone();
    let frame = spec.sheet.keep_layout.as_ref().ok_or_else(unavailable)?;
    frame.check()?;
    if spec.sheet.size.as_ref().is_some_and(|s| s != &frame.size)
        || spec.sheet.scale.as_ref().is_some_and(|s| s != &frame.scale)
    {
        return Err(unavailable());
    }
    fixed.sheet.size = Some(frame.size.clone());
    fixed.sheet.scale = Some(frame.scale.clone());
    fixed.sheets.get_or_insert_with(Default::default).max = Some(1);
    Ok(fixed)
}

/// An edit report compares the actual before/after sheets, not two candidates in one search.
pub(crate) fn report(before: &Plan, after: &mut Plan) {
    let sheets = |p: &Plan| {
        std::iter::once(&p.sheet)
            .chain(&p.sheets)
            .map(|s| (s.size.clone(), s.scale.clone()))
            .collect::<Vec<_>>()
    };
    if sheets(before) != sheets(after) {
        after.problems.retain(|p| p.code != "layout_replanned");
        after.problems.push(model::Problem {
            id: "p:layout_replanned:sheet".into(),
            code: "layout_replanned".into(),
            severity: "info".into(),
            message: format!(
                "The sheet was re-planned: {} at {} ({} sheet(s)) → {} at {} ({} sheet(s)). {}",
                before.sheet.size,
                before.sheet.scale,
                1 + before.sheets.len(),
                after.sheet.size,
                after.sheet.scale,
                1 + after.sheets.len(),
                crate::fixes::SHEET_CONSEQUENCE
            ),
            subjects: vec!["sheet".into()],
            ..Default::default()
        });
    }
}
