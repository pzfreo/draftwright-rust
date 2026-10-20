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

fn valid_sheet(size: &str, scale: &str) -> bool {
    crate::layout::PAGES.iter().any(|p| p.0 == size)
        && crate::layout::SCALES
            .iter()
            .any(|s| model::scale_name(*s) == scale)
}

fn valid_counts(required: usize, views: usize) -> bool {
    required <= 16384 && (1..=32).contains(&views)
}

fn valid_frame(id: &str, v: &Frame) -> bool {
    id.starts_with("v:")
        && v.scale.is_finite()
        && v.scale > 0.0
        && v.scale <= 1000.0
        && v.at
            .iter()
            .chain(&v.area)
            .all(|x| x.is_finite() && x.abs() <= 11890.0)
        && v.area[0] < v.area[2]
        && v.area[1] < v.area[3]
}

impl Layout {
    pub(crate) fn capture(plan: &Plan) -> Option<Self> {
        // Allocation can transport auxiliary views and tables after the single-sheet solve.
        // Until that can be constrained too, those remedies must say sheet scope.
        if !plan.sheets.is_empty()
            || plan
                .spec
                .sheet
                .size
                .as_ref()
                .is_some_and(|s| s != &plan.sheet.size)
            || plan
                .spec
                .sheet
                .scale
                .as_ref()
                .is_some_and(|s| s != &plan.sheet.scale)
        {
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
        if !valid_sheet(&self.size, &self.scale)
            || !valid_counts(self.required.len(), self.views.len())
            || !self.views.iter().all(|(id, v)| valid_frame(id, v))
        {
            return Err(unavailable());
        }
        Ok(())
    }

    /// Scope queries must not clone every required-item ID on every comparison or
    /// serialization. Read the same closed shape and bounds directly from the value;
    /// a Frame contains only fixed-size numeric arrays, so its decoding allocates no
    /// item strings. Keep validation shared with typed Spec validation above.
    pub(crate) fn valid_value(value: &serde_json::Value) -> bool {
        let Some(fields) = value.as_object() else {
            return false;
        };
        let (Some(size), Some(scale), Some(views), Some(required)) = (
            fields.get("size").and_then(serde_json::Value::as_str),
            fields.get("scale").and_then(serde_json::Value::as_str),
            fields.get("views").and_then(serde_json::Value::as_object),
            fields.get("required").and_then(serde_json::Value::as_array),
        ) else {
            return false;
        };
        fields.len() == 4
            && valid_sheet(size, scale)
            && valid_counts(required.len(), views.len())
            && required.iter().all(serde_json::Value::is_string)
            && views.iter().all(|(id, value)| {
                Frame::deserialize(value).is_ok_and(|frame| valid_frame(id, &frame))
            })
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
pub(crate) fn notice(before: &Plan, after: &Plan) -> Option<model::Problem> {
    let sheets = |p: &Plan| {
        std::iter::once(&p.sheet)
            .chain(&p.sheets)
            .map(|s| (s.size.clone(), s.scale.clone()))
            .collect::<Vec<_>>()
    };
    if sheets(before) != sheets(after) {
        Some(model::Problem {
            id: "p:layout_replanned:edit".into(),
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
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn borrowed_scope_check_matches_typed_layout_validation() {
        let base = json!({"size":"A4", "scale":"1:1", "required":["sheet","v:front"],
            "views":{"v:front":{"at":[0,0],"area":[0,0,100,100],"scale":1}}});
        let mut cases = vec![base.clone()];
        let wrong_types = [
            Value::Null,
            json!(true),
            json!(3),
            json!("x"),
            json!([]),
            json!({}),
        ];
        for value in &wrong_types {
            cases.push(value.clone());
            for field in ["size", "scale", "views", "required"] {
                let mut edited = base.clone();
                edited[field] = value.clone();
                cases.push(edited);
            }
            for field in ["at", "area", "scale"] {
                let mut edited = base.clone();
                edited["views"]["v:front"][field] = value.clone();
                cases.push(edited);
            }
        }
        for field in ["size", "scale", "views", "required"] {
            let mut edited = base.clone();
            edited.as_object_mut().unwrap().remove(field);
            cases.push(edited);
        }
        for (pointer, values) in [
            ("/unknown", vec![json!(true)]),
            ("/size", vec![json!("A0"), json!("A5")]),
            ("/scale", vec![json!("1:5"), json!("3:7")]),
            (
                "/required",
                vec![
                    json!([]),
                    json!([1]),
                    json!(vec!["sheet"; 16384]),
                    json!(vec!["sheet"; 16385]),
                ],
            ),
            ("/views/v:front/unknown", vec![json!(true)]),
            (
                "/views/v:front/scale",
                vec![json!(-1), json!(0), json!(1000), json!(1001)],
            ),
            (
                "/views/v:front/at",
                vec![
                    json!([0]),
                    json!([0, 0, 0]),
                    json!(["0", 0]),
                    json!([-11890, 11890]),
                    json!([11891, 0]),
                ],
            ),
            (
                "/views/v:front/area",
                vec![
                    json!([0, 0, 0, 1]),
                    json!([0, 1, 1, 0]),
                    json!([0, 0, 1]),
                    json!([-11890, -11890, 11890, 11890]),
                ],
            ),
        ] {
            for value in values {
                let mut edited = base.clone();
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                edited.pointer_mut(parent).unwrap()[key] = value;
                cases.push(edited);
            }
        }
        for (count, prefix) in [(1, "wrong:"), (32, "v:"), (33, "v:")] {
            let mut edited = base.clone();
            edited["views"] = Value::Object(
                (0..count)
                    .map(|i| (format!("{prefix}{i}"), base["views"]["v:front"].clone()))
                    .collect(),
            );
            cases.push(edited);
        }
        for value in cases {
            let typed = serde_json::from_value::<Layout>(value.clone())
                .is_ok_and(|frame| frame.check().is_ok());
            assert_eq!(Layout::valid_value(&value), typed, "{value}");
            let constraint = crate::items::Op::set("sheet/keep_layout", value);
            let local = crate::items::Op::set("dimensions/d/place/lane", json!(1));
            assert_eq!(
                crate::fixes::scope(std::slice::from_ref(&constraint)),
                crate::fixes::Scope::Sheet
            );
            assert_eq!(
                crate::fixes::scope(&[local, constraint]),
                if typed {
                    crate::fixes::Scope::Local
                } else {
                    crate::fixes::Scope::Sheet
                }
            );
        }
    }
}
