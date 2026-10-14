//! R5's allocation: recover paper first, then move cumulative groups of auxiliary content.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::items::{Op, State};
use crate::model::{self, Analysis, Invalid, Plan, PlannedSheet, PlannedView, Problem, Spec};
use crate::{hole_table, layout};

pub(crate) fn one() -> u32 {
    1
}
pub(crate) fn is_one(n: &u32) -> bool {
    *n == 1
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SheetsSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smallest_scale: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keep_together: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole_table: Option<u32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub gdt_on_sheet_1: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zones {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossRef {
    pub sheet: u32,
    pub zone: String,
    pub at: [f64; 2],
}

pub fn zones(page: [f64; 2]) -> Zones {
    let (columns, rows) = layout::PAGES
        .iter()
        .position(|(_, w, h)| [*w, *h] == page)
        .map(|i| [(6, 4), (8, 6), (12, 8), (16, 12), (24, 16)][i])
        .unwrap_or_else(|| {
            (
                ((page[0] / 100.0).round() as u32 * 2).max(2),
                ((page[1] / 100.0).round() as u32 * 2).max(2),
            )
        });
    Zones { columns, rows }
}

/// ISO 5457: numbers left to right, letters top to bottom, omitting I and O.
pub fn zone(page: [f64; 2], point: [f64; 2]) -> String {
    let grid = zones(page);
    let col = ((point[0] / page[0] * f64::from(grid.columns)).floor() as u32).min(grid.columns - 1);
    let row = (((page[1] - point[1]) / page[1] * f64::from(grid.rows)).floor() as usize)
        .min(grid.rows as usize - 1);
    let letter = b"ABCDEFGHJKLMNPQRSTUVWXYZ"[row.min(23)] as char;
    format!("{letter}{}", col + 1)
}

pub(crate) fn floor(spec: &Spec) -> Option<f64> {
    spec.sheets
        .as_ref()?
        .smallest_scale
        .as_ref()
        .and_then(|name| {
            layout::SCALES
                .into_iter()
                .find(|s| model::scale_name(*s) == *name)
        })
}

fn maximum(spec: &Spec) -> u32 {
    spec.sheets.as_ref().and_then(|s| s.max).unwrap_or(1)
}

fn assignments(spec: &Spec) -> BTreeMap<String, u32> {
    let mut out: BTreeMap<_, _> = spec
        .views
        .iter()
        .filter_map(|(name, v)| v.sheet.map(|s| (name.clone(), s)))
        .collect();
    if let Some(n) = spec.sheets.as_ref().and_then(|s| s.hole_table) {
        out.insert("hole_table".into(), n);
    }
    out
}

fn groups(spec: &Spec) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = ["iso", "section_aa", "hole_table", "front", "plan", "side"]
        .into_iter()
        .map(|s| vec![s.into()])
        .collect();
    let mut links = spec
        .sheets
        .as_ref()
        .map_or_else(Vec::new, |s| s.keep_together.clone());
    links.extend(spec.views.iter().filter_map(|(name, v)| {
        v.place
            .as_ref()
            .map(|p| vec![name.clone(), p.relative_to.clone()])
    }));
    for link in links {
        let mut combined = BTreeSet::new();
        out.retain(|g| {
            if g.iter().any(|s| link.contains(s)) {
                combined.extend(g.iter().cloned());
                false
            } else {
                true
            }
        });
        combined.extend(link);
        out.push(combined.into_iter().collect());
    }
    out
}

pub(crate) fn validate(spec: &Spec) -> Result<(), Invalid> {
    if spec.sheets.is_none()
        && spec.views.values().all(|v| v.sheet.is_none())
        && spec.tables.iter().all(|t| t.sheet.is_none())
    {
        return Ok(());
    }
    let max = maximum(spec);
    let refuse = |message, choices| Invalid { message, choices };
    if !(1..=9).contains(&max) {
        return Err(refuse(
            "sheets.max must be from 1 to 9.".into(),
            (1..=9).map(|n| n.to_string()).collect(),
        ));
    }
    for (name, n) in assignments(spec).into_iter().chain(
        spec.tables
            .iter()
            .map(|t| (t.id.clone(), t.sheet.unwrap_or(1))),
    ) {
        if n == 0 || n > max {
            let mut choices: Vec<_> = (1..=max).map(|n| n.to_string()).collect();
            if (2..=9).contains(&n) {
                choices.push(format!("set sheets/max {n}"));
            }
            return Err(refuse(
                format!(
                    "{name}'s sheet {n} is outside 1..{max}; set sheets/max to allow more sheets."
                ),
                choices,
            ));
        }
        if ["front", "plan", "side"].contains(&name.as_str()) && n != 1 {
            return Err(refuse(
                format!("Principal view {name} must stay on sheet 1."),
                vec!["1".into()],
            ));
        }
    }
    if let Some(s) = &spec.sheets {
        if let Some(name) = &s.smallest_scale {
            let Some(f) = floor(spec) else {
                return Err(refuse(
                    format!("{name:?} is not a standard smallest scale."),
                    layout::SCALES.map(model::scale_name).to_vec(),
                ));
            };
            if spec
                .sheet
                .scale
                .as_ref()
                .and_then(|name| {
                    layout::SCALES
                        .into_iter()
                        .find(|v| model::scale_name(*v) == *name)
                })
                .is_some_and(|scale| scale < f)
            {
                return Err(refuse(
                    "sheet.scale is below sheets.smallest_scale; the intents contradict.".into(),
                    vec![
                        "remove sheet/scale".into(),
                        format!(
                            "set sheets/smallest_scale {}",
                            spec.sheet.scale.as_ref().unwrap()
                        ),
                    ],
                ));
            }
        }
        for group in &s.keep_together {
            if group.len() < 2
                || group.iter().collect::<BTreeSet<_>>().len() != group.len()
                || group.iter().any(|n| {
                    !["front", "plan", "side", "iso", "section_aa", "hole_table"]
                        .contains(&n.as_str())
                })
            {
                return Err(refuse(
                    format!(
                        "Invalid keep-together group {group:?}; use distinct view names or hole_table."
                    ),
                    vec![
                        "front".into(),
                        "plan".into(),
                        "side".into(),
                        "iso".into(),
                        "section_aa".into(),
                        "hole_table".into(),
                    ],
                ));
            }
        }
    }
    let assigned = assignments(spec);
    for group in groups(spec) {
        let sheets: BTreeSet<_> = group
            .iter()
            .filter_map(|n| {
                assigned
                    .get(n)
                    .copied()
                    .or_else(|| ["front", "plan", "side"].contains(&n.as_str()).then_some(1))
            })
            .collect();
        if sheets.len() > 1 {
            let mut choices: Vec<_> = sheets.iter().map(|n| format!("assign sheet {n}")).collect();
            choices.extend(
                group
                    .iter()
                    .filter(|n| spec.views.get(*n).is_some_and(|v| v.place.is_some()))
                    .map(|n| format!("remove views/{n}/place")),
            );
            return Err(refuse(
                format!(
                    "Assignments split keep-together group {group:?}, including relative_to placement dependencies."
                ),
                choices,
            ));
        }
    }
    Ok(())
}

fn shown(spec: &Spec, name: &str) -> bool {
    !spec.views.get(name).is_some_and(|v| v.omit)
}

fn available(analysis: &Analysis, spec: &Spec, name: &str, long: bool) -> bool {
    match name {
        "iso" => shown(spec, name) && analysis.iso_bounds != [0.; 4],
        "section_aa" => {
            shown(spec, name)
                && shown(spec, "plan")
                && (shown(spec, "front") || shown(spec, "side"))
                && crate::demand::read(analysis).section.is_some()
        }
        "hole_table" => long,
        _ => false,
    }
}

fn candidates(analysis: &Analysis, spec: &Spec, long: bool) -> Vec<Vec<String>> {
    let assigned = assignments(spec);
    let mut out = Vec::new();
    for name in ["iso", "section_aa", "hole_table"] {
        let group = groups(spec)
            .into_iter()
            .find(|g| g.iter().any(|n| n == name))
            .unwrap();
        if out.contains(&group)
            || !group.iter().any(|n| available(analysis, spec, n, long))
            || group.iter().any(|n| {
                ["front", "plan", "side"].contains(&n.as_str())
                    || assigned.contains_key(n)
                    || spec
                        .views
                        .get(n)
                        .and_then(|v| v.place.as_ref())
                        .is_some_and(|p| p.pinned)
            })
        {
            continue;
        }
        out.push(group);
    }
    out
}

fn working(analysis: &Analysis, spec: &Spec, moved: &BTreeMap<String, u32>) -> Spec {
    let mut copy = spec.clone();
    for (name, sheet) in moved {
        if name == "hole_table" {
            copy.sheets.get_or_insert_with(Default::default).hole_table =
                Some(if *sheet == 0 { 2 } else { *sheet });
        } else if available(analysis, spec, name, false) {
            let view = copy.views.entry(name.clone()).or_default();
            view.omit = true;
            view.sheet = Some(if *sheet == 0 { 2 } else { *sheet });
        }
    }
    copy.tables.retain(|t| t.sheet.unwrap_or(1) == 1);
    copy
}

fn run(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    crate::planning::with(analysis, || {
        model::recover(analysis, spec, model::plan_escalated(analysis, spec)?)
    })
}

/// Specs without sheet controls take precisely the pre-R5 planner path.
pub(crate) fn allocate(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    allocate_counted(analysis, spec, &mut 0)
}

/// Count actual recovered sheet-1 runs, including prefixes whose packing later fails.
pub(crate) fn allocate_counted(
    analysis: &Analysis,
    spec: &Spec,
    steps: &mut usize,
) -> Result<Plan, Invalid> {
    if spec.sheets.is_none()
        && spec.views.values().all(|v| v.sheet.is_none())
        && spec.tables.iter().all(|t| t.sheet.is_none())
    {
        *steps += 1;
        return run(analysis, spec);
    }
    let migrated = model::migrate(spec, analysis)?;
    let spec = &migrated;
    validate(spec)?;
    let assigned = assignments(spec);
    let mut moved = BTreeMap::new();
    for group in groups(spec) {
        if let Some(n) = group
            .iter()
            .filter_map(|s| assigned.get(s))
            .copied()
            .find(|n| *n > 1)
        {
            for name in group {
                moved.insert(name, n);
            }
        }
    }
    *steps += 1;
    let mut first = run(analysis, &working(analysis, spec, &moved))?;
    let long = first.table_outcome == hole_table::TableOutcome::NoPlace;
    let possible = candidates(analysis, spec, long);
    let mut tried = Vec::new();
    let mut blocked = false;
    let mut best = None;
    let mut previous = first.clone();
    for step in 0..=possible.len() {
        if step > 0 {
            if maximum(spec) == 1 {
                blocked = true;
                break;
            }
            for name in &possible[step - 1] {
                moved.insert(name.clone(), 0);
            }
            // Preflight the cumulative prefix before paying for another recovered sheet 1.
            let mut trial = previous.clone();
            if !pack(analysis, spec, &moved, &mut trial) {
                tried.push(format!("step {step}: blocked by sheets.max"));
                blocked = true;
                continue;
            }
            let mut copy = working(analysis, spec, &moved);
            if moved.contains_key("hole_table") {
                copy.sheets.as_mut().unwrap().hole_table = Some(2);
            }
            *steps += 1;
            first = run(analysis, &copy)?;
        }
        let mut p = first.clone();
        if !pack_with_overflow(analysis, spec, &moved, &mut p, step == 0) {
            tried.push(format!("step {step}: blocked by sheets.max"));
            blocked = true;
            continue;
        }
        previous = p.clone();
        let (f, dropped) = score(analysis, spec, &p);
        let hard = f.hard;
        tried.push(format!(
            "step {step}: {} hard findings, {dropped} required drops, scale {}, moved {}",
            hard,
            p.sheet.scale,
            moved.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
        let rank = (hard, dropped, -p.sheet.scale_factor, step);
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, p));
        }
        if hard == 0 && dropped == 0 {
            break;
        }
    }
    let (_, mut p) = best.expect("step zero was packed");
    if maximum(spec) > 1 || !moved.is_empty() {
        p.sheet.basis += &format!("; sheet allocation: {}", tried.join("; "));
    }
    // Working omissions and table assignments are allocation mechanics, never author intent.
    let binding = p.spec.binding.clone();
    p.spec = spec.clone();
    p.spec.schema = model::SPEC_SCHEMA.into();
    p.spec.binding = binding;
    for table in &mut p.tables {
        if let Some((i, authored)) = spec
            .tables
            .iter()
            .enumerate()
            .find(|(_, t)| t.id == table.id)
        {
            table.basis = format!(
                "authored: the spec's table {} (tables/{i}), towards the sheet's {} corner as draftwright places a declared table",
                table.id,
                authored.prefer.as_deref().unwrap_or("tr")
            );
        }
    }
    if spec.sheets.as_ref().and_then(|s| s.hole_table).is_some() && p.hole_table.is_none() {
        p.problems.push(problem(
            "sheet_assignment_unused",
            "info",
            "sheets/hole_table",
            "No rescue hole table was made; its sheet assignment is unused.".into(),
            vec![Op::remove("sheets/hole_table")],
        ));
    }
    crate::title_block::number_sheets(&mut p);
    p.problems.extend(check_with_search(
        &p,
        blocked || (maximum(spec) == 1 && !possible.is_empty()),
    ));
    debug_assert!(
        p.dimensions
            .iter()
            .map(|d| &d.view)
            .chain(p.gdt.iter().map(|g| &g.view))
            .all(|id| p
                .views
                .iter()
                .find(|v| &v.id == id)
                .is_none_or(|v| v.sheet == 1)),
        "Stage I carriers must stay on sheet 1"
    );
    p.table_restore = None;
    p.status = crate::completeness::status(&p.problems);
    Ok(p)
}

fn score(analysis: &Analysis, spec: &Spec, plan: &Plan) -> (layout::Findings, usize) {
    // The search relieves sheet 1. An unplaceable assigned corner table on another sheet
    // cannot justify moving unrelated content off an otherwise clean first sheet.
    let mut first = plan.clone();
    let off: BTreeSet<_> = crate::items::items(plan)
        .into_iter()
        .filter(|i| i.sheet > 1)
        .map(|i| i.id)
        .collect();
    first
        .problems
        .retain(|p| p.subjects.is_empty() || p.subjects.iter().any(|id| !off.contains(id)));
    let mut f = model::findings(&first);
    let requested = floor(spec);
    let legible = crate::sheet_fit::min_legible_scale(analysis);
    let low_parent = spec.sheet.scale.is_none()
        && plan.sheet.scale_factor < requested.unwrap_or(0.).max(legible);
    let low_view =
        requested.is_some_and(|floor| plan.views.iter().any(|v| v.sheet == 1 && v.scale < floor));
    if (low_parent || low_view) && !requested.is_some_and(|floor| plan.sheet.scale_factor < floor) {
        f.hard += 1;
    }
    let dropped = f.dropped + model::undrawn_gdt(plan);
    (f, dropped)
}

/// A moved section still marks its parent during sheet-1 placement. A temporary carrier
/// supplies the cutting plane to the existing furniture planner without reserving view room.
pub(crate) fn line_work(analysis: &Analysis, spec: &Spec, views: &mut Vec<PlannedView>) {
    let moved = spec
        .views
        .get("section_aa")
        .is_some_and(|v| v.omit && v.sheet.is_some_and(|n| n > 1));
    let marker = moved
        .then(|| auxiliary(analysis, "section_aa", 1., 2, 1.))
        .flatten();
    if let Some(marker) = marker.as_ref() {
        views.push(marker.clone());
    }
    crate::centrelines::plan(analysis, views);
    if marker.is_some() {
        views.pop();
    }
}

fn auxiliary(
    analysis: &Analysis,
    name: &str,
    scale: f64,
    sheet: u32,
    parent_scale: f64,
) -> Option<PlannedView> {
    let section = if name == "section_aa" {
        Some(crate::demand::read(analysis).section?)
    } else {
        None
    };
    let direction = if name == "iso" {
        [1., -1., 1.]
    } else {
        [0., -1., 0.]
    };
    let up = [0., 0., 1.];
    let view = crate::haecceity::hlr::View::new(direction, up).expect("auxiliary view");
    let (extent, _) = model::extent(analysis, &view);
    let bounds = if name == "iso" {
        if analysis.iso_bounds == [0.; 4] {
            return None;
        }
        analysis.iso_bounds
    } else {
        [
            -extent[0] / 2.,
            -extent[1] / 2.,
            extent[0] / 2.,
            extent[1] / 2.,
        ]
    };
    let size = [bounds[2] - bounds[0], bounds[3] - bounds[1]];
    let label = if section.is_some() { 19. } else { 7. };
    let mut planned = PlannedView {
        id: format!("v:{name}"),
        sheet,
        scale_note: (scale != parent_scale).then(|| format!("SCALE {}", model::scale_name(scale))),
        xref: None,
        direction,
        up,
        at: [-bounds[0] * scale, -bounds[1] * scale + label],
        scale,
        area: [0., 0., size[0] * scale, size[1] * scale + label],
        default: true,
        basis: "auxiliary view moved as a whole after sheet-1 paper recovery".into(),
        state: State::Shown,
        line_work: Vec::new(),
        cut: section.map(|s| {
            let [lo, hi] = analysis.envelope;
            model::Cut {
                label: "A".into(),
                origin: [(lo[0] + hi[0]) / 2., s.cut_y, (lo[2] + hi[2]) / 2.],
                normal: [0., 1., 0.],
                marked_in: "v:plan".into(),
                xref: None,
            }
        }),
    };
    crate::centrelines::plan(analysis, std::slice::from_mut(&mut planned));
    for curve in planned.line_work.iter().flat_map(|w| &w.curves) {
        let b = curve.bbox();
        planned.area = [
            planned.area[0].min(b[0]),
            planned.area[1].min(b[1]),
            planned.area[2].max(b[2]),
            planned.area[3].max(b[3]),
        ];
    }
    // A narrow model does not narrow its caption/reference text. Reserve the longest
    // zone address on the supported paper grid; I2 draws it at this same caption centre.
    let text = if planned.cut.is_some() {
        "FROM SHEET 1, ZONE R24"
    } else {
        planned.scale_note.as_deref().unwrap_or("")
    };
    if !text.is_empty() {
        let half = crate::dims::font().advance(text, crate::dims::TEXT) / 2.;
        planned.area[0] = planned.area[0].min(planned.at[0] - half);
        planned.area[2] = planned.area[2].max(planned.at[0] + half);
    }
    planned.area[0] -= crate::tables::PAD;
    planned.area[1] -= crate::tables::PAD;
    planned.area[2] += crate::tables::PAD;
    planned.area[3] += crate::tables::PAD;
    Some(planned)
}

fn extra_sheet(first: &PlannedSheet, number: u32) -> PlannedSheet {
    let mut sheet = first.clone();
    sheet.number = number;
    sheet.fill = 0.;
    sheet.arrangement = "rows".into();
    sheet.basis = "same paper as sheet 1; assigned tables then auxiliary view blocks".into();
    sheet.projection_symbol = None;
    sheet
}

/// Pack assigned groups first, then fill the remaining sheets from 2 upwards. A group is atomic.
fn pack(analysis: &Analysis, spec: &Spec, moved: &BTreeMap<String, u32>, plan: &mut Plan) -> bool {
    pack_with_overflow(analysis, spec, moved, plan, false)
}

fn pack_with_overflow(
    analysis: &Analysis,
    spec: &Spec,
    moved: &BTreeMap<String, u32>,
    plan: &mut Plan,
    allow_overflow: bool,
) -> bool {
    plan.sheets.clear();
    plan.views
        .retain(|v| v.sheet == 1 && !moved.contains_key(v.id.trim_start_matches("v:")));
    plan.tables.retain(|t| t.sheet == 1);
    let max = maximum(spec);
    let mut sheets: BTreeMap<u32, PlannedSheet> = BTreeMap::new();
    let mut blocks: BTreeMap<u32, Vec<PlannedView>> = BTreeMap::new();
    let mut exclusive = BTreeSet::new();
    let mut problems = Vec::new();
    for n in 2..=max {
        let tables: Vec<_> = spec
            .tables
            .iter()
            .filter(|t| t.sheet == Some(n))
            .cloned()
            .collect();
        if tables.is_empty() {
            continue;
        }
        let sheet = extra_sheet(&plan.sheet, n);
        let (items, findings) = crate::tables::plan(&tables, analysis, &sheet, &[], &[], None);
        plan.tables.extend(items);
        problems.extend(findings);
        sheets.insert(n, sheet);
    }
    let mut reserved_table = None;
    let table_sheet = |sheets: &BTreeMap<u32, PlannedSheet>| {
        spec.sheets
            .as_ref()
            .and_then(|s| s.hole_table)
            .filter(|n| *n > 1)
            .or_else(|| moved.get("hole_table").copied().filter(|n| *n > 1))
            .or_else(|| {
                (2..=max).find(|n| !sheets.contains_key(n) && !moved.values().any(|s| s == n))
            })
    };
    // A preflight still has the prior NoPlace outcome, with the table withdrawn.
    // Reserve its exclusive sheet before testing the views; the real run escalates Some(n).
    if moved.contains_key("hole_table")
        && plan.table_outcome == hole_table::TableOutcome::NoPlace
        && plan.hole_table.is_none()
    {
        let Some(n) = table_sheet(&sheets) else {
            return false;
        };
        exclusive.insert(n);
        reserved_table = Some(n);
        sheets.insert(n, extra_sheet(&plan.sheet, n));
    }
    if let Some(mut table) = plan.hole_table.clone().filter(|t| t.sheet > 1) {
        let Some(n) = table_sheet(&sheets) else {
            return false;
        };
        let sheet = extra_sheet(&plan.sheet, n);
        let parts = hole_table::place_on_sheet(&table, &sheet);
        if let Some(parts) = parts {
            table.blocks = parts;
            table.sheet = n;
            table.basis = format!(
                "drop and escalate: every row is tagged on sheet 1; the table is on sheet {n}, towards the top right in {} blocks",
                table.blocks.len()
            );
            plan.hole_table = Some(table);
            exclusive.insert(n);
            sheets.insert(n, sheet);
        } else {
            hole_table::withdraw_no_place(analysis, plan);
        }
    }
    let mut bundles: Vec<_> = groups(spec)
        .into_iter()
        .filter(|g| g.iter().any(|n| moved.contains_key(n) && n != "hole_table"))
        .collect();
    bundles.sort_by_key(|g| {
        let assigned = g
            .iter()
            .filter_map(|n| moved.get(n))
            .copied()
            .find(|n| *n > 1);
        (
            assigned.is_none(),
            assigned.unwrap_or(0),
            if g.iter().any(|n| n == "iso") { 0 } else { 1 },
        )
    });
    for group in bundles {
        let names: Vec<_> = group
            .iter()
            .filter(|n| *n != "hole_table" && moved.contains_key(*n) && shown(spec, n))
            .collect();
        let with_table = group.iter().any(|n| n == "hole_table")
            && (plan.hole_table.as_ref().is_some_and(|t| t.sheet > 1) || reserved_table.is_some());
        let assigned = if with_table {
            plan.hole_table.as_ref().map(|t| t.sheet).or(reserved_table)
        } else {
            names
                .iter()
                .filter_map(|n| moved.get(*n))
                .copied()
                .find(|n| *n > 1)
        };
        if names.is_empty() {
            continue;
        }
        let targets: Vec<_> = assigned.map_or_else(
            || (2..=max).filter(|n| !exclusive.contains(n)).collect(),
            |n| vec![n],
        );
        // Full parent scale on occupied sheets, then a fresh sheet, then shrink on a fresh sheet.
        let mut found = None;
        for shrink in [false, true] {
            for &n in &targets {
                if (exclusive.contains(&n) && !with_table)
                    || (shrink && blocks.get(&n).is_some_and(|b| !b.is_empty()))
                {
                    continue;
                }
                let sheet = sheets
                    .get(&n)
                    .cloned()
                    .unwrap_or_else(|| extra_sheet(&plan.sheet, n));
                let existing = blocks.get(&n).cloned().unwrap_or_default();
                let mut obstacles: Vec<_> = plan
                    .tables
                    .iter()
                    .filter(|t| t.sheet == n)
                    .filter_map(|t| t.placed.as_ref().map(|p| p.cells))
                    .collect();
                obstacles.extend(
                    plan.hole_table
                        .iter()
                        .filter(|t| t.sheet == n)
                        .flat_map(|t| {
                            t.blocks
                                .iter()
                                .filter_map(|b| b.placed.as_ref().map(|p| p.cells))
                        }),
                );
                let scales: Vec<_> = if shrink {
                    layout::SCALES
                        .into_iter()
                        .filter(|s| *s < plan.sheet.scale_factor)
                        .collect()
                } else {
                    vec![plan.sheet.scale_factor]
                };
                for scale in scales {
                    let mut trial = existing.clone();
                    trial.extend(names.iter().filter_map(|name| {
                        auxiliary(analysis, name, scale, n, plan.sheet.scale_factor)
                    }));
                    if trial.len() == existing.len() {
                        found = Some((n, sheet.clone(), trial));
                        break;
                    }
                    let relative = names
                        .iter()
                        .any(|name| spec.views.get(*name).is_some_and(|v| v.place.is_some()));
                    let fits = if relative {
                        let mut next = trial.split_off(existing.len());
                        relate(spec, &mut next);
                        let mut carrier = next[0].clone();
                        carrier.area = next.iter().fold(
                            [
                                f64::INFINITY,
                                f64::INFINITY,
                                f64::NEG_INFINITY,
                                f64::NEG_INFINITY,
                            ],
                            |b, v| {
                                [
                                    b[0].min(v.area[0]),
                                    b[1].min(v.area[1]),
                                    b[2].max(v.area[2]),
                                    b[3].max(v.area[3]),
                                ]
                            },
                        );
                        let old = carrier.area;
                        let mut fixed = obstacles.clone();
                        fixed.extend(existing.iter().map(|v| v.area));
                        let fits =
                            layout::plan_sheet(std::slice::from_mut(&mut carrier), &fixed, &sheet);
                        for v in &mut next {
                            shift(v, [carrier.area[0] - old[0], carrier.area[1] - old[1]]);
                        }
                        trial.extend(next);
                        fits
                    } else {
                        layout::plan_sheet(&mut trial, &obstacles, &sheet)
                    };
                    if fits {
                        for v in trial.iter_mut().skip(existing.len()) {
                            v.default = !spec.views.contains_key(v.id.trim_start_matches("v:"));
                        }
                        found = Some((n, sheet.clone(), trial));
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if found.is_none()
            && allow_overflow
            && let Some(n) = assigned
        {
            let sheet = sheets
                .get(&n)
                .cloned()
                .unwrap_or_else(|| extra_sheet(&plan.sheet, n));
            let mut views = blocks.get(&n).cloned().unwrap_or_default();
            for name in &names {
                if let Some(mut view) = auxiliary(
                    analysis,
                    name,
                    layout::SCALES[15],
                    n,
                    plan.sheet.scale_factor,
                ) {
                    let top = views
                        .last()
                        .map_or(sheet.page[1] - layout::MARGIN, |v| v.area[1] - 10.);
                    let delta = [layout::MARGIN - view.area[0], top - view.area[3]];
                    shift(&mut view, delta);
                    views.push(view);
                }
            }
            // Keep assigned ink even when no standard scale fits. The finished-sheet audit
            // reports overflow; the floor check offers a corrective intent.
            found = Some((n, sheet, views));
        }
        let Some((n, sheet, views)) = found else {
            return false;
        };
        if !views.is_empty() {
            sheets.insert(n, sheet);
            blocks.insert(n, views);
        }
    }
    let mapping: BTreeMap<_, _> = sheets
        .keys()
        .enumerate()
        .map(|(i, &n)| (n, i as u32 + 2))
        .collect();
    for (old, mut sheet) in sheets {
        let n = mapping[&old];
        sheet.number = n;
        plan.sheets.push(sheet);
        for mut view in blocks.remove(&old).unwrap_or_default() {
            view.sheet = n;
            if spec
                .views
                .get(view.id.trim_start_matches("v:"))
                .and_then(|v| v.sheet)
                .is_some_and(|s| s != n)
            {
                problems.push(problem(
                    "sheet_renumbered",
                    "info",
                    &view.id,
                    format!("Empty sheets closed: {} is on sheet {n}.", view.id),
                    vec![Op::set(
                        format!("views/{}/sheet", view.id.trim_start_matches("v:")),
                        json!(n),
                    )],
                ));
            }
            plan.views.push(view);
        }
        for table in plan.tables.iter_mut().filter(|t| t.sheet == old) {
            table.sheet = n;
            if old != n {
                let i = spec.tables.iter().position(|t| t.id == table.id).unwrap();
                problems.push(problem(
                    "sheet_renumbered",
                    "info",
                    &table.id,
                    format!("Empty sheets closed: {} is on sheet {n}.", table.id),
                    vec![Op::set(format!("tables/{i}/sheet"), json!(n))],
                ));
            }
        }
        if let Some(t) = plan.hole_table.as_mut().filter(|t| t.sheet == old) {
            t.sheet = n;
            for b in &mut t.blocks {
                b.sheet = n;
            }
            if old != n {
                problems.push(problem(
                    "sheet_renumbered",
                    "info",
                    &t.id,
                    format!("Empty sheets closed: the hole table is on sheet {n}."),
                    vec![Op::set("sheets/hole_table", json!(n))],
                ));
            }
        }
    }
    plan.problems.extend(problems);
    if !plan.sheets.is_empty() {
        plan.sheet.zones = Some(zones(plan.sheet.page));
        for sheet in &mut plan.sheets {
            sheet.zones = Some(zones(sheet.page));
        }
        let mut generated = plan.views.clone();
        crate::centrelines::plan(analysis, &mut generated);
        for view in plan.views.iter_mut().filter(|v| v.sheet > 1) {
            view.line_work = generated
                .iter()
                .find(|v| v.id == view.id)
                .unwrap()
                .line_work
                .clone();
        }
        cross_references(analysis, plan);
    }
    true
}

fn shift(view: &mut PlannedView, delta: [f64; 2]) {
    view.at[0] += delta[0];
    view.at[1] += delta[1];
    for k in 0..4 {
        view.area[k] += delta[k % 2];
    }
    crate::centrelines::shift(view, delta);
}

fn relate(spec: &Spec, views: &mut [PlannedView]) {
    use crate::placement_intent::{Side, Slot};
    let mut done = BTreeSet::new();
    for _ in 0..views.len() {
        for i in 0..views.len() {
            let name = views[i].id.trim_start_matches("v:").to_string();
            let Some(place) = spec.views.get(&name).and_then(|v| v.place.as_ref()) else {
                done.insert(name);
                continue;
            };
            if done.contains(&name)
                || (spec
                    .views
                    .get(&place.relative_to)
                    .is_some_and(|v| v.place.is_some())
                    && !done.contains(&place.relative_to))
            {
                continue;
            }
            let Some(parent) = views
                .iter()
                .find(|v| v.id == format!("v:{}", place.relative_to))
            else {
                continue;
            };
            let b = parent.area;
            let a = views[i].area;
            let horizontal = matches!(place.side, Side::Left | Side::Right);
            let k = usize::from(horizontal);
            let along = match place.align {
                Slot::Start => b[k] - a[k],
                Slot::Middle => (b[k] + b[k + 2] - a[k] - a[k + 2]) / 2.,
                Slot::End | Slot::Outside => b[k + 2] - a[k + 2],
            };
            let across = match place.side {
                Side::Above => b[3] + place.gap - a[1],
                Side::Below => b[1] - place.gap - a[3],
                Side::Left => b[0] - place.gap - a[2],
                Side::Right => b[2] + place.gap - a[0],
            };
            shift(
                &mut views[i],
                if horizontal {
                    [across, along]
                } else {
                    [along, across]
                },
            );
            done.insert(name);
        }
    }
}

fn cross_references(analysis: &Analysis, plan: &mut Plan) {
    let refs: Vec<_> = plan
        .views
        .iter()
        .filter_map(|v| {
            let cut = v.cut.as_ref()?;
            let parent = plan.views.iter().find(|p| p.id == cut.marked_in)?;
            if v.sheet == parent.sheet {
                return None;
            }
            let letter = parent
                .line_work
                .iter()
                .find(|w| w.kind == "cutting_plane")
                .and_then(|w| w.curves.first())
                .map(|c| {
                    let b = c.bbox();
                    crate::section::cutting_plane(b[0], b[2], b[1]).1[0]
                })
                .unwrap_or(parent.at);
            let from = [letter[0], letter[1] + 5.];
            let projection =
                crate::haecceity::hlr::View::new(v.direction, v.up).expect("section view");
            let caption = [
                v.at[0],
                v.at[1] - model::extent(analysis, &projection).0[1] * v.scale / 2. - 7.,
            ];
            let at = [caption[0], caption[1] - 5.];
            Some((
                v.id.clone(),
                CrossRef {
                    sheet: parent.sheet,
                    zone: zone(plan.sheet.page, letter),
                    at,
                },
                CrossRef {
                    sheet: v.sheet,
                    zone: zone(plan.sheet.page, caption),
                    at: from,
                },
            ))
        })
        .collect();
    for (id, from, to) in refs {
        let v = plan.views.iter_mut().find(|v| v.id == id).unwrap();
        v.xref = Some(from);
        v.cut.as_mut().unwrap().xref = Some(to);
    }
}

fn problem(code: &str, severity: &str, subject: &str, message: String, fix: Vec<Op>) -> Problem {
    Problem {
        id: format!("p:{code}:{subject}"),
        code: code.into(),
        severity: severity.into(),
        subjects: vec![subject.into()],
        message,
        fix,
        ..Default::default()
    }
}

pub fn check(plan: &Plan) -> Vec<Problem> {
    check_with_search(plan, false)
}

fn check_with_search(plan: &Plan, blocked: bool) -> Vec<Problem> {
    let mut out = Vec::new();
    if !plan.sheets.is_empty() && plan.spec.sheet.title.contains_key("sheet") {
        let mut p = problem(
            "sheet_label_replaced",
            "info",
            "sheet",
            "Multi-sheet SHEET labels use n/m in place of the spec's sheet/title/sheet.".into(),
            vec![Op::remove("sheet/title/sheet")],
        );
        p.refs.push("title_block/sheet".into());
        out.push(p);
    }
    let sheet_of = |id: &str| {
        plan.views
            .iter()
            .find(|v| v.id == id)
            .map_or(1, |v| v.sheet)
    };
    for frame in plan
        .gdt
        .iter()
        .filter(|g| g.kind == "control_frame" && g.placed.is_some())
    {
        let n = sheet_of(&frame.view);
        for cell in frame.cells.iter().skip(2) {
            for letter in cell
                .split(|c: char| !c.is_ascii_alphabetic())
                .filter(|s| s.len() == 1)
            {
                if !plan.gdt.iter().any(|g| {
                    g.kind == "datum_feature"
                        && g.cells.first().is_some_and(|s| s == letter)
                        && g.placed.is_some()
                        && [1, n].contains(&sheet_of(&g.view))
                }) {
                    let mut p = problem(
                        "datum_not_identified",
                        "error",
                        &frame.id,
                        format!(
                            "Datum {letter} is not identified on sheet 1 or the frame's sheet {n}."
                        ),
                        vec![],
                    );
                    p.id += &format!(":{letter}");
                    p.refs.push(letter.into());
                    out.push(p);
                }
            }
        }
    }
    for view in &plan.views {
        if let Some(cut) = &view.cut {
            let parent = plan.views.iter().find(|v| v.id == cut.marked_in);
            if parent.is_none_or(|p| {
                p.sheet != view.sheet && (view.xref.is_none() || cut.xref.is_none())
            }) {
                let mut p = problem(
                    "cross_reference_missing",
                    "error",
                    &view.id,
                    "A section on another sheet needs cross-references both ways.".into(),
                    vec![],
                );
                p.refs.push(cut.marked_in.clone());
                out.push(p);
            }
        }
    }
    for sheet in std::iter::once(&plan.sheet).chain(&plan.sheets) {
        let views: Vec<_> = plan
            .views
            .iter()
            .filter(|v| v.sheet == sheet.number)
            .collect();
        if let Some(floor) = floor(&plan.spec) {
            let below: Vec<_> = views
                .iter()
                .filter(|v| v.scale < floor)
                .map(|v| v.id.clone())
                .collect();
            if sheet.scale_factor < floor || !below.is_empty() {
                let subject = if sheet.number == 1 {
                    "sheet".into()
                } else {
                    format!("sheet:{}", sheet.number)
                };
                let index = layout::PAGES
                    .iter()
                    .position(|(n, ..)| *n == sheet.size)
                    .unwrap_or(4);
                let scale = views
                    .iter()
                    .map(|v| v.scale)
                    .fold(sheet.scale_factor, f64::min);
                let fix = if sheet.number == 1 && blocked && maximum(&plan.spec) < 9 {
                    Op::set("sheets/max", json!(maximum(&plan.spec) + 1))
                } else if plan.spec.sheet.size.is_some() && index < 4 {
                    Op::set("sheet/size", json!(layout::PAGES[index + 1].0))
                } else {
                    Op::set(
                        "sheets/smallest_scale",
                        json!(model::scale_name(
                            layout::SCALES
                                .into_iter()
                                .find(|s| *s <= scale)
                                .unwrap_or(layout::SCALES[15])
                        )),
                    )
                };
                let mut p = problem(
                    "sheet_below_smallest_scale",
                    "warning",
                    &subject,
                    format!(
                        "Sheet {} has views below the smallest scale {}.",
                        sheet.number,
                        model::scale_name(floor)
                    ),
                    vec![fix],
                );
                p.refs = below;
                out.push(p);
            }
        }
        if sheet.number == 1 {
            continue;
        }
        let mut bounds: Vec<(String, [f64; 4])> =
            views.iter().map(|v| (v.id.clone(), v.area)).collect();
        bounds.extend(
            plan.tables
                .iter()
                .filter(|t| t.sheet == sheet.number)
                .filter_map(|t| t.placed.as_ref().map(|p| (t.id.clone(), p.cells))),
        );
        bounds.extend(
            plan.hole_table
                .iter()
                .chain(&plan.hole_tables)
                .filter(|t| t.sheet == sheet.number)
                .flat_map(|t| {
                    t.blocks
                        .iter()
                        .filter_map(|b| b.placed.as_ref().map(|p| (b.id.clone(), p.cells)))
                }),
        );
        for (i, (id, b)) in bounds.iter().enumerate() {
            if b[0] < layout::MARGIN
                || b[1] < layout::MARGIN
                || b[2] > sheet.page[0] - layout::MARGIN
                || b[3] > sheet.page[1] - layout::MARGIN
            {
                out.push(problem(
                    "annotation_off_sheet",
                    "error",
                    id,
                    format!("{id} is outside sheet {}.", sheet.number),
                    vec![],
                ));
            }
            if overlap(*b, sheet.title_block) {
                out.push(problem(
                    "view_on_title_block",
                    "error",
                    id,
                    format!("{id} overlaps sheet {}'s title block.", sheet.number),
                    vec![],
                ));
            }
            for (other, c) in &bounds[i + 1..] {
                if overlap(*b, *c) {
                    let mut p = problem(
                        "views_overlap",
                        "error",
                        id,
                        format!("{id} overlaps {other} on sheet {}.", sheet.number),
                        vec![],
                    );
                    p.refs.push(other.clone());
                    out.push(p);
                }
            }
        }
    }
    out
}

pub(crate) fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_table_preflight_reserves_an_exclusive_sheet() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/prismatic.step");
        let (_, analysis) = model::analyse_native(&path).unwrap();
        let mut spec: Spec = serde_json::from_value(json!({
            "sheets": {"max": 2}, "views": {"iso": {"sheet": 2}}
        }))
        .unwrap();
        let mut prior = model::plan(&analysis, &spec).unwrap();
        // Typed evidence from the prior escalation; no table ink survives NoPlace.
        prior.table_outcome = hole_table::TableOutcome::NoPlace;
        assert!(prior.hole_table.is_none());
        let moved = BTreeMap::from([("iso".into(), 2), ("hole_table".into(), 0)]);
        assert!(!pack(&analysis, &spec, &moved, &mut prior.clone()));
        spec.sheets.as_mut().unwrap().max = Some(3);
        assert!(pack(&analysis, &spec, &moved, &mut prior));
        assert_eq!(prior.sheets.len(), 2);
        assert_eq!(
            prior.views.iter().find(|v| v.id == "v:iso").unwrap().sheet,
            2
        );
    }

    #[test]
    fn a_forced_group_assigns_the_rescue_table_to_its_views_sheet() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/scattered_holes.step");
        let (_, analysis) = model::analyse_native(&path).unwrap();
        let spec: Spec = serde_json::from_value(json!({
            "sheets": {"max": 2, "keep_together": [["iso", "hole_table"]]},
            "views": {"iso": {"sheet": 2}}
        }))
        .unwrap();
        let mut p = model::plan(&analysis, &spec).unwrap();
        let id = p
            .dimensions
            .iter()
            .find(|d| {
                d.view == "v:plan"
                    && d.placed.is_some()
                    && d.kind == "linear"
                    && d.measurements.iter().any(|m| {
                        m.feature.starts_with("f:hole:")
                            && m.parameter.starts_with("location.length")
                    })
            })
            .unwrap()
            .id
            .clone();
        p.problems.push(Problem {
            id: format!("stand-in:{id}"),
            code: "annotation_off_sheet".into(),
            severity: "warning".into(),
            subjects: vec![id],
            ..Default::default()
        });
        assert_eq!(
            hole_table::escalate(&analysis, &mut p, Some(2)),
            hole_table::TableOutcome::Kept
        );
        let moved = BTreeMap::from([("iso".into(), 2), ("hole_table".into(), 2)]);
        assert!(pack(&analysis, &spec, &moved, &mut p));
        assert_eq!(p.sheets.len(), 1);
        assert_eq!(p.hole_table.as_ref().unwrap().sheet, 2);
        assert_eq!(p.hole_table.as_ref().unwrap().rows.len(), 18);
        assert_eq!(p.balloons.len(), 18);
        assert_eq!(p.views.iter().find(|v| v.id == "v:iso").unwrap().sheet, 2);
        assert!(!check(&p).iter().any(|p| p.code == "views_overlap"));
    }
}
