//! Placement intent relative to views and feature anchors (ADR R1 invariant 1).
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::items::{Op, key};
use crate::model::{Analysis, Invalid, Plan, PlannedSheet, PlannedView, Problem, Spec};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Place {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub along: Option<Along>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Above,
    Below,
    Left,
    Right,
}
impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Self::Above => "above",
            Self::Below => "below",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Along {
    Fraction(f64),
    Slot(Slot),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Start,
    Middle,
    End,
    Outside,
}
impl Along {
    pub fn fraction(self) -> f64 {
        match self {
            Self::Fraction(f) => f,
            Self::Slot(Slot::Start) => 0.0,
            Self::Slot(Slot::Middle) => 0.5,
            Self::Slot(Slot::End) => 1.0,
            Self::Slot(Slot::Outside) => 1.25,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnnotationSpec {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub omit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<Place>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewPlace {
    /// A view name; `sheet` means the drawing area's lower left, never a page coordinate.
    pub relative_to: String,
    pub side: Side,
    /// Clear space between the two view footprints, in paper mm.
    #[serde(default = "default_gap")]
    pub gap: f64,
    #[serde(default = "middle")]
    pub align: Slot,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}
fn default_gap() -> f64 {
    10.0
}
fn middle() -> Slot {
    Slot::Middle
}

pub(crate) fn carrier(plan: &Plan, id: &str) -> Option<String> {
    let owner = plan
        .omitted
        .iter()
        .find(|o| o.id == id)
        .and_then(|o| match &o.state {
            crate::items::State::Covered { by } => Some(by.as_str()),
            _ => None,
        })
        .unwrap_or(id);
    plan.dimensions
        .iter()
        .find(|d| d.id == owner)
        .map(|d| d.view.clone())
        .or_else(|| plan.gdt.iter().find(|g| g.id == id).map(|g| g.view.clone()))
}

pub(crate) fn has_pins(s: &Spec) -> bool {
    s.dimensions
        .values()
        .any(|d| d.place.as_ref().is_some_and(|p| p.pinned))
        || s.annotations
            .values()
            .any(|d| d.place.as_ref().is_some_and(|p| p.pinned))
        || s.views
            .values()
            .any(|v| v.place.as_ref().is_some_and(|p| p.pinned))
}
pub(crate) fn check(s: &Spec) -> Result<(), Invalid> {
    let error = |message: String, choices: Vec<String>| Invalid { message, choices };
    for (id, p) in s
        .dimensions
        .iter()
        .filter_map(|(id, d)| Some((id, d.place.as_ref()?)))
        .chain(
            s.annotations
                .iter()
                .filter_map(|(id, d)| Some((id, d.place.as_ref()?))),
        )
    {
        if let Some(v) = &p.view
            && !["front", "plan", "side"].contains(&v.as_str())
        {
            return Err(error(
                format!("The placement view of {id} must be a principal view."),
                ["front", "plan", "side"].map(String::from).to_vec(),
            ));
        }
        if p.lane.is_some_and(|n| n > 1000) {
            return Err(error(
                format!("The placement lane of {id} exceeds 1000."),
                vec!["0..1000".into()],
            ));
        }
        if let Some(Along::Fraction(f)) = p.along
            && (!f.is_finite() || !(0.0..=1.0).contains(&f))
        {
            return Err(error(
                format!("The along fraction of {id} must be between 0 and 1."),
                vec![
                    "0..1".into(),
                    "start".into(),
                    "middle".into(),
                    "end".into(),
                    "outside".into(),
                ],
            ));
        }
    }
    for (name, v) in &s.views {
        if let Some(p) = &v.place {
            if !p.gap.is_finite() || !(0.0..=1000.0).contains(&p.gap) {
                return Err(error(
                    format!("The gap of {name} must be between 0 and 1000 paper mm."),
                    vec!["0..1000".into()],
                ));
            }
            if p.relative_to == *name
                || !["sheet", "front", "plan", "side", "iso", "section_aa"]
                    .contains(&p.relative_to.as_str())
            {
                return Err(error(
                    format!("The placement anchor of {name} is not another view or the sheet."),
                    ["sheet", "front", "plan", "side", "iso"]
                        .into_iter()
                        .filter(|v| *v != name)
                        .map(String::from)
                        .collect(),
                ));
            }
            let mut seen = BTreeSet::new();
            let mut at = name.as_str();
            while let Some(p) = s.views.get(at).and_then(|v| v.place.as_ref()) {
                if !seen.insert(at) {
                    return Err(error(
                        format!("View placement anchors form a cycle at {name}."),
                        vec!["sheet".into()],
                    ));
                }
                at = &p.relative_to;
            }
        }
    }
    Ok(())
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}
fn shift(v: &mut PlannedView, d: [f64; 2]) {
    v.at = [v.at[0] + d[0], v.at[1] + d[1]];
    v.area = [
        v.area[0] + d[0],
        v.area[1] + d[1],
        v.area[2] + d[0],
        v.area[3] + d[1],
    ];
}
fn problem(id: &str, code: &str, message: String, path: &str) -> Problem {
    Problem {
        id: format!("p:{code}:{id}"),
        code: code.into(),
        severity: "warning".into(),
        message,
        subjects: vec![id.into()],
        fix: vec![if path.starts_with("views/") {
            Op::remove(path)
        } else {
            Op::set(path, json!({}))
        }],
        ..Default::default()
    }
}

/// Explicit positions resolve in dependency order. Pinned carriers retain the canonical frame;
/// unrelated omissions, text edits and recovery cannot change their geometry.
pub(crate) fn views(
    s: &Spec,
    views: &mut [PlannedView],
    canonical: Option<&Plan>,
    sheet: &PlannedSheet,
    obstacles: &[[f64; 4]],
    problems: &mut Vec<Problem>,
) {
    let page = sheet.page;
    if let Some(c) = canonical {
        for v in views.iter_mut() {
            if let Some(base) = c.views.iter().find(|b| b.id == v.id) {
                v.at = base.at;
                v.area = base.area;
                v.scale = base.scale;
            }
        }
    }
    let mut resolved = BTreeSet::new();
    for _ in 0..views.len() {
        for i in 0..views.len() {
            let name = views[i].id.trim_start_matches("v:").to_string();
            let Some(p) = s.views.get(&name).and_then(|v| v.place.as_ref()) else {
                resolved.insert(name);
                continue;
            };
            if resolved.contains(&name) {
                continue;
            }
            let b = if p.relative_to == "sheet" {
                [crate::layout::MARGIN; 4]
            } else {
                if s.views
                    .get(&p.relative_to)
                    .is_some_and(|v| v.place.is_some())
                    && !resolved.contains(&p.relative_to)
                {
                    continue;
                }
                let Some(anchor) = views
                    .iter()
                    .find(|v| v.id == format!("v:{}", p.relative_to))
                else {
                    continue;
                };
                anchor.area
            };
            let a = views[i].area;
            let horizontal = matches!(p.side, Side::Left | Side::Right);
            let k = usize::from(horizontal);
            let offset = match p.align {
                Slot::Start => b[k] - a[k],
                Slot::Middle => 0.5 * (b[k] + b[k + 2] - a[k] - a[k + 2]),
                Slot::End | Slot::Outside => b[k + 2] - a[k + 2],
            };
            let out = match p.side {
                Side::Above => b[3] + p.gap - a[1],
                Side::Below => b[1] - p.gap - a[3],
                Side::Right => b[2] + p.gap - a[0],
                Side::Left => b[0] - p.gap - a[2],
            };
            shift(
                &mut views[i],
                if horizontal {
                    [out, offset]
                } else {
                    [offset, out]
                },
            );
            views[i].default = false;
            views[i].basis += "; placed relative to its anchor by the spec";
            resolved.insert(name);
        }
    }
    for (name, v) in &s.views {
        if v.place.is_some() && !resolved.contains(name) {
            let id = format!("v:{name}");
            let mut p = problem(
                &id,
                "placement_anchor_not_shown",
                format!("The placement anchor of {id} is not shown; its intent is retained."),
                &format!("views/{name}/place"),
            );
            if !views.iter().any(|v| v.id == id) {
                p.subjects.clear();
                p.refs.push(id);
            }
            problems.push(p);
        }
    }
    let mut fixed: BTreeSet<String> = s
        .views
        .iter()
        .filter(|(_, v)| v.place.as_ref().is_some_and(|p| p.pinned))
        .map(|(n, _)| format!("v:{n}"))
        .collect();
    // Annotation anchors are fixed too, so filling around one cannot move its carrying view.
    if canonical.is_some() {
        for (id, p) in s
            .dimensions
            .iter()
            .filter_map(|(id, d)| Some((id, d.place.as_ref()?)))
            .chain(
                s.annotations
                    .iter()
                    .filter_map(|(id, d)| Some((id, d.place.as_ref()?))),
            )
        {
            if !p.pinned {
                continue;
            }
            let name = p
                .view
                .as_deref()
                .map(|n| format!("v:{n}"))
                .or_else(|| canonical.and_then(|c| carrier(c, id)));
            if let Some(n) = name {
                fixed.insert(n);
            }
        }
    }
    for _ in 0..views.len() {
        for v in views
            .iter()
            .filter(|v| fixed.contains(&v.id))
            .collect::<Vec<_>>()
        {
            if let Some(p) = s
                .views
                .get(v.id.trim_start_matches("v:"))
                .and_then(|v| v.place.as_ref())
                && p.relative_to != "sheet"
            {
                fixed.insert(format!("v:{}", p.relative_to));
            }
        }
    }
    let margin = crate::layout::MARGIN;
    let mut occupied: Vec<[f64; 4]> = views
        .iter()
        .filter(|v| fixed.contains(&v.id))
        .map(|v| v.area)
        .collect();
    occupied.push(sheet.title_block);
    occupied.extend_from_slice(obstacles);
    for v in views.iter_mut().filter(|v| !fixed.contains(&v.id)) {
        if occupied.iter().any(|b| overlap(v.area, *b)) {
            let base = v.clone();
            'search: for shrink in 0..if v.id == "v:iso" { 10 } else { 1 } {
                let factor = 0.9_f64.powi(shrink);
                let mut trial = base.clone();
                trial.scale *= factor;
                trial.area = [
                    base.at[0] + factor * (base.area[0] - base.at[0]),
                    base.at[1] + factor * (base.area[1] - base.at[1]),
                    base.at[0] + factor * (base.area[2] - base.at[0]),
                    base.at[1] + factor * (base.area[3] - base.at[1]),
                ];
                let (w, h) = (trial.area[2] - trial.area[0], trial.area[3] - trial.area[1]);
                let mut xs = vec![trial.area[0], margin, page[0] - margin - w];
                let mut ys = vec![trial.area[1], margin, page[1] - margin - h];
                for b in &occupied {
                    xs.extend([b[2] + 1.0, b[0] - w - 1.0]);
                    ys.extend([b[3] + 1.0, b[1] - h - 1.0]);
                }
                let mut candidates: Vec<[f64; 2]> = xs
                    .iter()
                    .flat_map(|x| ys.iter().map(move |y| [*x, *y]))
                    .collect();
                candidates.sort_by(|a, b| {
                    ((a[0] - base.area[0]).abs() + (a[1] - base.area[1]).abs())
                        .total_cmp(&((b[0] - base.area[0]).abs() + (b[1] - base.area[1]).abs()))
                        .then(a[0].total_cmp(&b[0]))
                        .then(a[1].total_cmp(&b[1]))
                });
                for at in candidates {
                    let a = [at[0], at[1], at[0] + w, at[1] + h];
                    if a[0] >= margin
                        && a[1] >= margin
                        && a[2] <= page[0] - margin
                        && a[3] <= page[1] - margin
                        && !occupied.iter().any(|b| overlap(a, *b))
                    {
                        let d = [at[0] - trial.area[0], at[1] - trial.area[1]];
                        shift(&mut trial, d);
                        trial.basis += "; fitted around fixed placement obstacles";
                        *v = trial;
                        break 'search;
                    }
                }
            }
        }
        occupied.push(v.area);
    }
    for v in views.iter().filter(|v| fixed.contains(&v.id)) {
        let a = v.area;
        let path = format!("views/{}/place", v.id.trim_start_matches("v:"));
        if a[0] < crate::layout::MARGIN
            || a[1] < crate::layout::MARGIN
            || a[2] > page[0] - crate::layout::MARGIN
            || a[3] > page[1] - crate::layout::MARGIN
        {
            // Only an explicit view pin has an editable view placement entry.
            if s.views
                .get(v.id.trim_start_matches("v:"))
                .is_some_and(|v| v.place.is_some())
            {
                problems.push(problem(
                    &v.id,
                    "pinned_view_off_sheet",
                    format!(
                        "{} is pinned outside the drawing area and is kept there.",
                        v.id
                    ),
                    &path,
                ));
            }
        }
        if (views
            .iter()
            .any(|other| other.id != v.id && overlap(a, other.area))
            || overlap(a, sheet.title_block))
            && s.views
                .get(v.id.trim_start_matches("v:"))
                .is_some_and(|v| v.place.is_some())
        {
            problems.push(problem(
                &v.id,
                "pinned_view_overlap",
                format!(
                    "{} overlaps another view or the title block and is kept at its pinned position.",
                    v.id
                ),
                &path,
            ));
        }
    }
}

pub(crate) fn obstacles(
    s: &Spec,
    dimensions: &[crate::dims::PlannedDimension],
    fixed: &[crate::dims::PlannedDimension],
) -> Vec<[f64; 4]> {
    dimensions
        .iter()
        .filter(|d| {
            s.dimensions
                .get(&d.id)
                .and_then(|e| e.place.as_ref())
                .is_some_and(|p| p.pinned)
        })
        .chain(fixed)
        .filter_map(crate::dims::ink_box)
        .collect()
}

pub(crate) fn finish(plan: &mut Plan, _analysis: &Analysis) {
    let pins: BTreeMap<String, String> = plan
        .spec
        .dimensions
        .iter()
        .filter(|(_, d)| d.place.as_ref().is_some_and(|p| p.pinned))
        .map(|(id, _)| (id.clone(), format!("dimensions/{}/place", key(id))))
        .chain(
            plan.spec
                .annotations
                .iter()
                .filter(|(_, d)| d.place.as_ref().is_some_and(|p| p.pinned))
                .map(|(id, _)| (id.clone(), format!("annotations/{}/place", key(id)))),
        )
        .collect();
    for p in &mut plan.problems {
        if p.code.starts_with("annotation_") {
            if let Some((_, path)) = p.subjects.iter().find_map(|id| pins.get_key_value(id)) {
                p.fix = vec![Op::set(path, json!({}))];
            } else if p.fix.is_empty() {
                if let Some(d) = p
                    .subjects
                    .iter()
                    .find_map(|id| plan.dimensions.iter().find(|d| d.id == *id))
                {
                    if let Some(placed) = &d.placed {
                        let side = if placed.rotation == 90.0 {
                            "left"
                        } else {
                            "above"
                        };
                        p.fix = vec![Op::set(
                            format!("dimensions/{}/place", key(&d.id)),
                            json!({"view":d.view.trim_start_matches("v:"),"side":side,"lane":0,"along":"outside"}),
                        )];
                    }
                } else if let Some(g) = p
                    .subjects
                    .iter()
                    .find_map(|id| plan.gdt.iter().find(|g| g.id == *id))
                {
                    let side = match g.side.as_str() {
                        "above" => "below",
                        "below" => "above",
                        "left" => "right",
                        _ => "left",
                    };
                    p.fix = vec![Op::set(
                        format!("annotations/{}/place", key(&g.id)),
                        json!({"view":g.view.trim_start_matches("v:"),"side":side,"lane":0}),
                    )];
                }
            }
        }
    }
    for (id, path) in pins {
        let state = crate::items::items(plan)
            .into_iter()
            .find(|i| i.id == id)
            .map(|i| i.state);
        if !matches!(state, Some(crate::items::State::Shown)) {
            let mut p = problem(
                &id,
                "pinned_item_not_drawn",
                format!("{id} cannot be drawn at its pinned anchor; its placement is retained."),
                &path,
            );
            if state.is_none() {
                p.subjects.clear();
                p.refs.push(id);
            }
            plan.problems.push(p);
        }
    }
}

pub(crate) fn automatic_pin(p: &Place) -> bool {
    p.pinned && p.side.is_none() && p.lane.is_none() && p.along.is_none()
}

pub(crate) fn anchor_delta(base: &Plan, views: &[PlannedView], view: &str) -> Option<[f64; 2]> {
    let before = base.views.iter().find(|v| v.id == view)?;
    let after = views.iter().find(|v| v.id == view)?;
    Some([after.at[0] - before.at[0], after.at[1] - before.at[1]])
}

pub(crate) fn move_point(p: &mut [f64; 2], delta: [f64; 2]) {
    if delta != [0.0, 0.0] {
        p[0] += delta[0];
        p[1] += delta[1];
    }
}
pub(crate) fn move_box(b: &mut [f64; 4], delta: [f64; 2]) {
    if delta != [0.0, 0.0] {
        for k in 0..4 {
            b[k] += delta[k % 2];
        }
    }
}

pub(crate) fn held_dimensions(
    base: Option<&Plan>,
    views: &[PlannedView],
) -> Vec<crate::dims::PlannedDimension> {
    let Some(base) = base else {
        return Vec::new();
    };
    base.dimensions
        .iter()
        .filter_map(|d| {
            let delta = anchor_delta(base, views, &d.view)?;
            let mut d = d.clone();
            if let Some(p) = &mut d.placed {
                for line in p.lines.iter_mut().chain(&mut p.arrows) {
                    for q in line {
                        move_point(q, delta);
                    }
                }
                move_point(&mut p.label_at, delta);
                move_box(&mut p.label_box, delta);
            }
            Some(d)
        })
        .collect()
}
