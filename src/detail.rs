//! Enlarged orthographic regions, planned as late furniture before the isometric.
//! Python: _core.DetailRequest, analysis._automatic_*_detail_footprints and
//! annotations.sections._resolve_details. The Rust kernel has no solid Booleans: the
//! detail clips the source projection, retaining exact curve intervals rather than
//! manufacturing a cut face. Bounds and recovered dimension IDs remain in the plan.
use serde::{Deserialize, Serialize};

use crate::bridge::holes;
use crate::layout::{Rect, Scaled};
use crate::model::{Analysis, Invalid, PlannedView, Spec};

pub const NAMES: [&str; 8] = [
    "detail_a", "detail_b", "detail_c", "detail_d", "detail_e", "detail_f", "detail_g", "detail_h",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailSpec {
    /// Semantic feature or face IDs; the string form remains stable on saving.
    pub around: Around,
    /// Principal or section view ID, without the `v:` prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default = "two")]
    pub scale_factor: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Around {
    One(String),
    Many(Vec<String>),
}
impl Around {
    fn members(&self) -> &[String] {
        match self {
            Self::One(id) => std::slice::from_ref(id),
            Self::Many(ids) => ids,
        }
    }
}

impl DetailSpec {
    pub(crate) fn around_ids(&self) -> String {
        self.around.members().join(", ")
    }
}
fn two() -> f64 {
    2.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detail {
    pub label: String,
    pub axis: usize,
    pub source_view: String,
    /// The source section's projection plane, without duplicating its furniture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cut: Option<crate::model::Cut>,
    pub kind: String,
    /// Part-frame crop, in model mm; complete dimensional witnesses must survive.
    pub bounds: [[f64; 3]; 2],
    pub marked_bounds: [[f64; 3]; 2],
    pub dimensions: Vec<String>,
    pub scale: f64,
    pub authored: bool,
    pub partial_profile: bool,
}

pub(crate) fn check(analysis: &Analysis, spec: &Spec) -> Result<(), Invalid> {
    for (name, view) in &spec.views {
        if let Some(d) = &view.detail {
            if !NAMES.contains(&name.as_str())
                || !d.scale_factor.is_finite()
                || d.scale_factor <= 1.0
            {
                return Err(Invalid {
                    message: format!(
                        "Detail {name} needs an identifier detail_a–detail_h and a scale_factor greater than one."
                    ),
                    choices: vec![],
                });
            }
            let members = members(analysis, name, d)?;
            if let Some(source) = &d.from {
                if !matches!(source.as_str(), "front" | "plan" | "side")
                    && !(source.starts_with("section_")
                        && source == &format!("section_{0}{0}", section_label(spec).to_lowercase()))
                {
                    return Err(Invalid {
                        message: format!(
                            "Detail {name}: source view {source:?} does not exist in the plan for {}.",
                            d.around.members().join(", ")
                        ),
                        choices: vec![
                            "front".into(),
                            "plan".into(),
                            "side".into(),
                            "section_aa".into(),
                        ],
                    });
                }
                if !source_available(analysis, spec, source) {
                    return Err(Invalid {
                        message: format!(
                            "Detail {name}: source view v:{source} does not exist in the plan for {}.",
                            d.around_ids()
                        ),
                        choices: vec![],
                    });
                }
                for (id, crop) in d.around.members().iter().zip(&members) {
                    if !visible(crop.bounds, source) {
                        return Err(Invalid {
                            message: format!(
                                "Detail {name}: feature or face {id:?} is not visible in view {source:?}: its crop projects to a degenerate region."
                            ),
                            choices: vec![],
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

struct MemberCrop {
    bounds: [[f64; 3]; 2],
    source: &'static str,
    primary: usize,
    dimensions: Vec<String>,
}

fn union(a: [[f64; 3]; 2], b: [[f64; 3]; 2]) -> [[f64; 3]; 2] {
    [
        std::array::from_fn(|k| a[0][k].min(b[0][k])),
        std::array::from_fn(|k| a[1][k].max(b[1][k])),
    ]
}

fn axes(source: &str) -> [usize; 2] {
    match source {
        "plan" => [0, 1],
        "side" => [1, 2],
        _ => [0, 2],
    }
}

fn visible(bounds: [[f64; 3]; 2], source: &str) -> bool {
    axes(source)
        .iter()
        .all(|&k| (bounds[1][k] - bounds[0][k]).is_finite() && bounds[1][k] - bounds[0][k] > 1e-6)
}

fn source_available(a: &Analysis, spec: &Spec, source: &str) -> bool {
    if spec.views.get(source).is_some_and(|v| v.omit) {
        return false;
    }
    let demand = crate::demand::read(a);
    let reduction = crate::demand::turned_views(&demand).filter(|(kept, dropped, _)| {
        !spec.views.contains_key(*dropped)
            && !spec.views.get(kept[0]).is_some_and(|v| v.omit)
            && !crate::pmi_lowering::views(a).contains(dropped)
    });
    if reduction.is_some_and(|r| r.1 == source) {
        return false;
    }
    if source.starts_with("section_") {
        return demand.section.is_some()
            && !spec
                .views
                .get(crate::model::SECTION)
                .is_some_and(|v| v.omit)
            && source_available(a, spec, "plan")
            && (source_available(a, spec, "front") || source_available(a, spec, "side"));
    }
    true
}

fn members(a: &Analysis, name: &str, auth: &DetailSpec) -> Result<Vec<MemberCrop>, Invalid> {
    if auth.around.members().is_empty() {
        return Err(Invalid {
            message: format!("Detail {name} needs at least one feature or face in around."),
            choices: vec![],
        });
    }
    auth.around.members().iter().map(|id| {
        if let Some(f) = a.features.iter().find(|f| &f.id == id) {
            return Ok(feature_crop(a, f));
        }
        let faces = crate::spec_gdt::named_faces(a, id).map_err(|_| Invalid {
            message: format!("Detail {name} names unknown feature or face {id:?}."),
            choices: a.features.iter().map(|f| f.id.clone()).chain(["face:<n>".into(), "top".into(), "bottom".into(), "left".into(), "right".into(), "front".into(), "back".into()]).collect(),
        })?;
        let mut bounds = None;
        for face in faces {
            let b = a.face_attachments[face].bounds.ok_or_else(|| Invalid {
                message: format!("Detail {name}: face {id:?} has no model-space crop; analyse the part again."), choices: vec![],
            })?;
            let b = [[b[0], b[1], b[2]], [b[3], b[4], b[5]]];
            bounds = Some(bounds.map_or(b, |previous| union(previous, b)));
        }
        let bounds = bounds.expect("named faces is nonempty");
        let source = ["front", "plan", "side"].into_iter().find(|source| visible(bounds, source)).ok_or_else(|| Invalid {
            message: format!("Detail {name}: face {id:?} has no non-degenerate crop."), choices: vec![],
        })?;
        Ok(MemberCrop { bounds, source, primary: axes(source)[0], dimensions: vec![] })
    }).collect()
}

fn feature_crop(a: &Analysis, f: &crate::bridge::Feature) -> MemberCrop {
    let mut bounds = a.envelope;
    let mut ids = Vec::new();
    let source;
    let primary;
    if let Some(b) = (f.kind == "turned_step")
        .then(|| crate::lint::turned::band(&holes::frame(a), f).ok())
        .flatten()
    {
        source = axis_view(b.axis);
        primary = b.axis;
        let context = 1.0_f64.max((b.hi - b.lo) / 2.0);
        bounds[0][b.axis] = (b.lo - context).max(a.envelope[0][b.axis]);
        bounds[1][b.axis] = (b.hi + context).min(a.envelope[1][b.axis]);
        ids.push(dimension(&f.id, "step.length"));
        ids.push(dimension(&f.id, "step.diameter"));
    } else {
        let origin = crate::dims::feature_origin(a, &f.id)
            .unwrap_or_else(|| [0, 1, 2].map(|k| (bounds[0][k] + bounds[1][k]) / 2.0));
        let axis = f.fingerprint.axis.and_then(holes::principal).unwrap_or(1);
        source = ["side", "front", "plan"][axis];
        primary = (0..3).find(|k| *k != axis).unwrap();
        let half = f.fingerprint.sizes.values().copied().fold(4.0, f64::max) * 0.75;
        for k in 0..3 {
            if k != axis {
                bounds[0][k] = (origin[k] - half).max(bounds[0][k]);
                bounds[1][k] = (origin[k] + half).min(bounds[1][k]);
            }
        }
    }

    MemberCrop {
        bounds,
        source,
        primary,
        dimensions: ids,
    }
}

pub(crate) fn basis(a: &Analysis, spec: &Spec, name: &str, d: &Detail) -> String {
    let mut basis = format!(
        "{}: enlarged orthographic crop of {}",
        d.kind, d.source_view
    );
    if let Some(auth) = spec.views.get(name).and_then(|v| v.detail.as_ref())
        && auth.from.is_none()
        && let Ok(crops) = members(a, name, auth)
        && crops.iter().any(|c| c.source != crops[0].source)
    {
        let source = d.source_view.trim_start_matches("v:");
        let count = crops.iter().filter(|c| visible(c.bounds, source)).count();
        basis += &format!(
            "; natural source views differ: chose {source}, where {count}/{} members are visible (ties: front, plan, side, section)",
            crops.len()
        );
    }
    basis
}

fn selected_scale(sheet: f64, needed: f64, head: bool) -> f64 {
    if head && (10.0..=15.0).contains(&needed) && sheet <= 5.0 {
        return 10.0;
    }
    [2.0, 5.0, 10.0]
        .into_iter()
        .map(|f| sheet * f)
        .find(|s| *s >= needed)
        .unwrap_or(sheet * 10.0)
}

fn axis_view(axis: usize) -> &'static str {
    if axis == 1 { "side" } else { "front" }
}
fn dimension(feature: &str, parameter: &str) -> String {
    format!("d:{feature}/{parameter}")
}

pub(crate) fn requests(a: &Analysis, spec: &Spec, sheet: f64) -> Vec<(String, Detail)> {
    // Sheet convergence asks for the same scale repeatedly. Keep this immutable
    // preparation in the planning scope, shared with its recovery candidates.
    let key = serde_json::to_vec(&(spec, sheet.to_bits())).expect("detail inputs serialize");
    crate::planning::memo(a, "detail_requests", key, || {
        requests_uncached(a, spec, sheet)
    })
}

fn requests_uncached(a: &Analysis, spec: &Spec, sheet: f64) -> Vec<(String, Detail)> {
    let frame = holes::frame(a);
    let mut bands: Vec<_> = a
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .filter_map(|f| crate::lint::turned::band(&frame, f).ok().map(|b| (f, b)))
        .collect();
    bands.sort_by(|a, b| a.1.axis.cmp(&b.1.axis).then(a.1.lo.total_cmp(&b.1.lo)));
    let mut out = Vec::new();
    for (name, view) in &spec.views {
        let Some(auth) = &view.detail else { continue };
        if view.omit {
            continue;
        }
        let crops = members(a, name, auth).expect("checked detail members");
        let mut bounds = crops[0].bounds;
        let mut ids = Vec::new();
        for crop in &crops {
            bounds = union(bounds, crop.bounds);
            ids.extend(crop.dimensions.iter().cloned());
        }
        let mut seen = std::collections::BTreeSet::new();
        ids.retain(|id| seen.insert(id.clone()));
        let primary = crops[0].primary;
        let source = if let Some(source) = &auth.from {
            source.clone()
        } else if crops.iter().all(|c| c.source == crops[0].source) {
            crops[0].source.into()
        } else {
            // Deterministic ties: front, plan, side, then the section.
            let section = format!("section_{0}{0}", section_label(spec).to_lowercase());
            let mut best = ("front", 0);
            for candidate in ["front", "plan", "side", &section] {
                if !source_available(a, spec, candidate) {
                    continue;
                }
                let count = crops
                    .iter()
                    .filter(|c| visible(c.bounds, candidate))
                    .count();
                if count > best.1 {
                    best = (candidate, count);
                }
            }
            best.0.to_string()
        };
        out.push((
            name.clone(),
            Detail {
                label: name.trim_start_matches("detail_").to_uppercase(),
                axis: primary,
                source_view: format!("v:{source}"),
                source_cut: None,
                kind: "authored-feature".into(),
                marked_bounds: bounds,
                bounds,
                dimensions: ids,
                scale: sheet * auth.scale_factor,
                authored: true,
                partial_profile: false,
            },
        ));
    }
    if spec.sheet.automatic_details == Some(false) {
        return out;
    }
    let available = |source: &str| !spec.views.get(source).is_some_and(|v| v.omit);
    let demand = crate::demand::read(a);
    let section_letter = (demand.section.is_some()
        || spec.views.contains_key(crate::model::SECTION))
        && !spec
            .views
            .get(crate::model::SECTION)
            .is_some_and(|v| v.omit && v.sheet.is_none_or(|n| n <= 1));
    let mut allocated: std::collections::BTreeSet<String> = spec
        .views
        .iter()
        .filter(|(_, v)| v.detail.is_some())
        .map(|(name, _)| name.clone())
        .collect();
    if section_letter {
        allocated.insert(format!("detail_{}", section_label(spec).to_lowercase()));
    }
    let mut automatic_count = 0;
    let mut add = |mut d: Detail| {
        if automatic_count == 2 || (d.kind.starts_with("readability") && !context_bounds(a, &mut d))
        {
            return;
        }
        d.marked_bounds = d.bounds;
        d.dimensions.retain(|id| {
            id.starts_with("d:step_level/step_interval.length.")
                || !out
                    .iter()
                    .any(|(_, previous)| previous.dimensions.contains(id))
        });
        if d.dimensions.is_empty() {
            return;
        }
        let Some(name) = NAMES.iter().find(|n| !allocated.contains(**n)) else {
            return;
        };
        allocated.insert(name.to_string());
        if spec.views.get(*name).is_some_and(|v| v.omit) {
            return;
        }
        d.label = name.trim_start_matches("detail_").to_uppercase();
        automatic_count += 1;
        out.push((name.to_string(), d));
    };
    // One physical connected profile at a time. Repeated equal pitches of three or more
    // stay counted on the parent, as Python's axial chain compiler does.
    let connected = bands.len() >= 2
        && bands.windows(2).all(|w| {
            w[0].1.axis == w[1].1.axis
                && (w[0].1.hi - w[1].1.lo).abs() < 1e-3
                && w[0].0.parameters.get("profile") == w[1].0.parameters.get("profile")
                && (0..3).all(|k| k == w[0].1.axis || (w[0].1.on[k] - w[1].1.on[k]).abs() < 1e-3)
        });
    if connected && spec.sheet.automatic_details != Some(true) {
        let axis = bands[0].1.axis;
        let source = axis_view(axis);
        let repeated = bands.windows(3).any(|w| {
            (w[0].1.hi - w[0].1.lo - (w[1].1.hi - w[1].1.lo)).abs() < 1e-3
                && (w[0].1.hi - w[0].1.lo - (w[2].1.hi - w[2].1.lo)).abs() < 1e-3
        });
        // The vertical-head extension describes a whole turned part. A pin on a plate
        // has a connected profile too, but its optional crop must not commandeer the
        // assembly's sheet search or its per-body dimensions (K5).
        if available(source)
            && !repeated
            // Preserve the established principal scale and aligned frame corridors for
            // GD&T drawings. An authored crop still takes precedence above.
            && demand.gdt.iter().all(Vec::is_empty)
            // A declared schedule and explicit minimum main scale own the spare paper.
            // Do not repeatedly add then withdraw an optional turned crop in each
            // multi-sheet prefix; all its dimensions remain on the principal views.
            && !(crate::sheets::floor(spec).is_some() && !spec.tables.is_empty())
            && (axis != 2 || crate::dims::inspection_datum(a).is_some())
        {
            let mut runs: Vec<Vec<usize>> = Vec::new();
            if axis == 1 {
                let short = bands.iter().any(|(_, b)| {
                    (b.hi - b.lo) * sheet
                        < crate::dims::font()
                            .advance(&crate::dims::fmt(b.hi - b.lo), crate::dims::TEXT)
                            + 2.0 * crate::dims::ARROW
                            + 2.0
                });
                if short {
                    runs.push((0..bands.len()).collect());
                }
            } else {
                for (i, (_, b)) in bands.iter().enumerate() {
                    if (b.hi - b.lo) * sheet < 2.0 * crate::dims::ARROW {
                        if let Some(r) = runs
                            .last_mut()
                            .filter(|r| r.last() == Some(&i.saturating_sub(1)))
                        {
                            r.push(i);
                        } else {
                            runs.push(vec![i]);
                        }
                    }
                }
                // Vertical heads: an isolated thin terminal flange also warrants a detail.
                runs.retain(|r| {
                    r.len() >= 2 || axis == 2 && (r[0] == 0 || r[0] == bands.len() - 1)
                });
            }
            for run in runs {
                if run.iter().any(|i| {
                    spec.views
                        .values()
                        .filter_map(|v| v.detail.as_ref())
                        .any(|d| d.around.members().contains(&bands[*i].0.id))
                }) {
                    continue;
                }
                let min = run
                    .iter()
                    .map(|&i| bands[i].1.hi - bands[i].1.lo)
                    .fold(f64::INFINITY, f64::min);
                let needed = if axis == 1 {
                    run.iter()
                        .map(|&i| {
                            (crate::dims::font().advance(
                                &crate::dims::fmt(bands[i].1.hi - bands[i].1.lo),
                                crate::dims::TEXT,
                            ) + 2.0 * crate::dims::ARROW
                                + 2.0)
                                / (bands[i].1.hi - bands[i].1.lo)
                        })
                        .fold(0.0, f64::max)
                } else {
                    crate::layout::MIN_STEP_SEP / min
                };
                let scale = selected_scale(sheet, needed, axis != 1);
                if needed > sheet * 10.0 {
                    continue;
                }
                let mut bounds = a.envelope;
                let lo = bands[run[0]].1.lo;
                let hi = bands[*run.last().unwrap()].1.hi;
                bounds[0][axis] = lo;
                bounds[1][axis] = hi;
                let marked_bounds = bounds;
                // A little adjoining shaft gives the reader the shoulder's context.
                if axis == 2 {
                    bounds[0][axis] = (lo - 1.0).max(a.envelope[0][axis]);
                    bounds[1][axis] = (hi + 1.0).min(a.envelope[1][axis]);
                }
                if axis == 0 {
                    // Keep the complete selected profiles. A crop around only their
                    // positive rims can reduce a smaller adjacent lip to a meaningless strip.
                    let context = 0.1_f64.max(2.0 / needed);
                    let profiles: Vec<_> = run
                        .iter()
                        .map(|&i| {
                            let b = &bands[i].1;
                            [b.on[2] - b.diameter / 2.0, b.on[2] + b.diameter / 2.0]
                        })
                        .collect();
                    bounds[0][2] = (profiles.iter().map(|b| b[0]).fold(f64::INFINITY, f64::min)
                        - context)
                        .max(a.envelope[0][2]);
                    bounds[1][2] = (profiles
                        .iter()
                        .map(|b| b[1])
                        .fold(f64::NEG_INFINITY, f64::max)
                        + context)
                        .min(a.envelope[1][2]);
                }
                if axis == 1 {
                    let z = bands[0].1.on[2];
                    let half =
                        0.1_f64.max(((a.envelope[1][2] - a.envelope[0][2]) / 4.0).min(6.0 / scale));
                    bounds[0][2] = z - half;
                    bounds[1][2] = z + half;
                }
                let ids = run
                    .iter()
                    .flat_map(|&i| {
                        [
                            dimension(&bands[i].0.id, "step.length"),
                            dimension(&bands[i].0.id, "step.diameter"),
                        ]
                    })
                    .collect();
                add(Detail {
                    label: String::new(),
                    axis,
                    source_view: format!("v:{source}"),
                    source_cut: None,
                    kind: if axis == 1 {
                        "y-turned-chain"
                    } else {
                        "turned-head"
                    }
                    .into(),
                    bounds,
                    marked_bounds: if axis == 0 || axis == 1 {
                        bounds
                    } else {
                        marked_bounds
                    },
                    dimensions: ids,
                    scale,
                    authored: false,
                    partial_profile: false,
                });
            }
        }
    } else if spec.sheet.automatic_details != Some(true)
        && bands.is_empty()
        && available("front")
        && !a.level_supports.is_empty()
    {
        // Recover only approved height rungs rejected by the parent spacing gate.
        let mut levels = a.levels.clone();
        levels.sort_by(f64::total_cmp);
        let legible = crate::layout::legible_steps(&levels, a.envelope[0][2], sheet);
        let omitted: Vec<_> = levels
            .iter()
            .filter(|z| !legible.contains(z))
            .copied()
            .collect();
        let wants = crate::dims::detail_levels(a);
        let ids: Vec<_> = wants
            .iter()
            .filter(|w| {
                w.0.contains("/step_height.length.")
                    && omitted
                        .iter()
                        .any(|z| (w.1 - (*z - a.envelope[0][2])).abs() < 1e-3)
            })
            .map(|w| w.0.clone())
            .collect();
        if levels.len() >= 2 && !ids.is_empty() {
            let min = levels
                .windows(2)
                .map(|w| w[1] - w[0])
                .fold(f64::INFINITY, f64::min);
            let needed = crate::layout::MIN_STEP_SEP / min;
            if needed.is_finite() && needed <= sheet * 10.0 {
                let scale = selected_scale(sheet, needed, false);
                let mut bounds = a.envelope;
                let pad = 0.08 * (levels.last().unwrap() - levels[0]) + 1.0;
                bounds[1][2] = (levels.last().unwrap() + pad).min(bounds[1][2]);
                // Keep the whole feature silhouette. A support-centred strip can show
                // unrelated edge fragments instead of the step being dimensioned.
                add(Detail {
                    label: String::new(),
                    axis: 2,
                    source_view: "v:front".into(),
                    source_cut: None,
                    kind: "prismatic-steps".into(),
                    marked_bounds: {
                        let mut marked = bounds;
                        marked[0][2] = (levels[0] - pad).max(marked[0][2]);
                        marked
                    },
                    bounds,
                    dimensions: ids,
                    scale,
                    authored: false,
                    partial_profile: false,
                });
            }
        }
    }
    // Local floors use the established crop/marker/dimension-ownership machinery.
    // Existing specialised crops retain priority; nearby candidates share a crop.
    if spec.sheet.automatic_details == Some(true) {
        for d in crate::dims::readability_details(a, spec, sheet)
            .into_iter()
            .chain(depiction_details(a, sheet))
        {
            if !source_available(a, spec, d.source_view.trim_start_matches("v:")) {
                continue;
            }
            add(d);
        }
    }
    for (_, d) in &mut out {
        d.partial_profile = (0..3).any(|k| {
            k != d.axis
                && (d.bounds[0][k] > a.envelope[0][k] + 1e-6
                    || d.bounds[1][k] < a.envelope[1][k] - 1e-6)
        });
    }
    out
}

/// A few complete repetition periods identify dense modelled ink without
/// enlarging the whole component. Cosmetic threads use their owned cylinder.
pub(crate) fn depiction_details(a: &Analysis, sheet: f64) -> Vec<Detail> {
    let needed = crate::sheet_fit::modelled_line_scale(a);
    if needed <= sheet {
        return Vec::new();
    }
    let Some(ink) = &a.edge_ink else {
        return Vec::new();
    };
    let mut groups = std::collections::BTreeMap::<[i64; 4], Vec<_>>::new();
    for face in &ink.curved {
        let Some(c) = &face.cylinder else { continue };
        let Some(axis) = holes::principal(c.axis) else {
            continue;
        };
        let mut key = c.origin.map(|x| (x * 1000.0).round() as i64);
        key[axis] = 0;
        groups
            .entry([axis as i64, key[0], key[1], key[2]])
            .or_default()
            .push(face);
    }
    let scale = crate::layout::SCALES
        .into_iter()
        .rev()
        .find(|s| *s >= needed && *s > sheet)
        .unwrap_or(needed);
    let mut result = Vec::new();
    for (key, faces) in groups {
        let axis = key[0] as usize;
        let mut starts: Vec<_> = faces.iter().map(|f| f.bounds[0][axis]).collect();
        starts.sort_by(f64::total_cmp);
        starts.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        let Some(four) = starts.windows(4).find(|four| {
            let pitch = (four[3] - four[0]) / 3.0;
            pitch > 0.01
                && four
                    .windows(2)
                    .all(|w| ((w[1] - w[0]) - pitch).abs() < 0.02 * pitch)
        }) else {
            continue;
        };
        let mut bounds = faces[0].bounds;
        for f in &faces {
            bounds = union(bounds, f.bounds);
        }
        let pad = 2.0 / scale;
        bounds[0][axis] = (four[0] - pad).max(a.envelope[0][axis]);
        bounds[1][axis] = (four[3] + pad).min(a.envelope[1][axis]);
        for k in (0..3).filter(|k| *k != axis) {
            bounds[0][k] = (bounds[0][k] - pad).max(a.envelope[0][k]);
            bounds[1][k] = (bounds[1][k] + pad).min(a.envelope[1][k]);
        }
        // A few periods at one radial edge show both roots and crests. The
        // opposite rim adds the component's diameter, not local information,
        // and can make an otherwise useful thread detail larger than its page.
        let source = axis_view(axis);
        let radial = axes(source)
            .into_iter()
            .find(|k| *k != axis)
            .expect("axial view");
        let radii: Vec<_> = faces
            .iter()
            .filter(|f| f.bounds[0][axis] < four[3] && f.bounds[1][axis] > four[0])
            .filter_map(|f| f.cylinder.as_ref().map(|c| c.origin[radial] + c.radius))
            .collect();
        if let (Some(root), Some(crest)) = (
            radii.iter().copied().min_by(f64::total_cmp),
            radii.iter().copied().max_by(f64::total_cmp),
        ) {
            bounds[0][radial] = (root - pad).max(a.envelope[0][radial]);
            bounds[1][radial] = (crest + pad).min(a.envelope[1][radial]);
        }
        result.push(Detail {
            label: String::new(),
            axis,
            source_view: format!("v:{source}"),
            source_cut: None,
            kind: "readability-depiction".into(),
            bounds,
            marked_bounds: bounds,
            dimensions: Vec::new(),
            scale,
            authored: false,
            partial_profile: true,
        });
    }
    result
}

/// Expand a crop around its actual witnesses so it carries surrounding outline.
/// The marker and the clipped view always describe the same model-space region.
/// If the part envelope cannot supply a compact region, decline the proposal.
pub(crate) fn context_bounds(a: &Analysis, d: &mut Detail) -> bool {
    let projected = axes(d.source_view.trim_start_matches("v:"));
    let longest = projected
        .iter()
        .map(|&k| d.bounds[1][k] - d.bounds[0][k])
        .fold(0.0, f64::max);
    let minimum = (longest / 3.0).max(8.0 / d.scale);
    for k in projected {
        let width = (d.bounds[1][k] - d.bounds[0][k]).max(minimum);
        let lo = ((d.bounds[0][k] + d.bounds[1][k] - width) / 2.0)
            .max(a.envelope[0][k])
            .min((a.envelope[1][k] - width).max(a.envelope[0][k]));
        d.bounds[0][k] = lo;
        d.bounds[1][k] = (lo + width).min(a.envelope[1][k]);
    }
    d.marked_bounds = d.bounds;
    let [w, h] = size(d);
    w.min(h) > 0.0 && w.max(h) <= 3.0 * w.min(h) + 1e-6
}

/// Text and page blocks used by both detail qualification and rendering.
pub(crate) fn text_obstacles(p: &crate::model::Plan) -> Vec<Rect> {
    p.dimensions
        .iter()
        .filter_map(|d| d.placed.as_ref().map(|q| q.label_box))
        .chain(
            p.gdt
                .iter()
                .filter_map(|g| g.placed.as_ref().map(|q| q.label_box)),
        )
        .chain(p.balloons.iter().filter_map(|b| b.label_box))
        .chain(p.captions.iter().map(|c| c.ink))
        .chain(
            p.notes
                .iter()
                .filter_map(|n| n.placed.as_ref().map(|q| q.cells)),
        )
        .chain(
            p.tables
                .iter()
                .filter_map(|t| t.placed.as_ref().map(|q| q.cells)),
        )
        .collect()
}

/// Finished automatic crops have to earn their paper with a moved controlling
/// annotation. Test actual ownership after redundancy and placement, not wants.
pub(crate) fn readable_automatic(
    a: &Analysis,
    parent: &crate::model::Plan,
    p: &crate::model::Plan,
) -> bool {
    let automatic: Vec<_> = p
        .views
        .iter()
        .filter(|v| v.detail.as_ref().is_some_and(|d| !d.authored))
        .collect();
    if automatic.is_empty() {
        return true;
    }
    if p.sheets.len() > parent.sheets.len()
        || p.sheet.scale_factor + 1e-6 < parent.sheet.scale_factor
        || crate::layout::PAGES
            .iter()
            .position(|page| page.0 == p.sheet.size)
            > crate::layout::PAGES
                .iter()
                .position(|page| page.0 == parent.sheet.size)
        || automatic.len() > 2
    {
        return false;
    }
    for v in automatic {
        let d = v.detail.as_ref().unwrap();
        if p.dimensions.iter().any(|dimension| {
            dimension.view == v.id && crate::dims::text_off_span(dimension, &p.dimensions)
        }) {
            return false;
        }
        if !p.dimensions.iter().any(|dimension| {
            dimension.view == v.id
                && !dimension.reference
                && dimension.placed.is_some()
                && parent.dimensions.iter().any(|old| {
                    old.id == dimension.id
                        && old.view == d.source_view
                        && !old.reference
                        && old.placed.is_some()
                })
        }) {
            return false;
        }
        if d.bounds != d.marked_bounds {
            return false;
        }
        if d.kind.starts_with("readability") {
            let [w, h] = size(d);
            if w.max(h) > 3.0 * w.min(h) + 1e-6 {
                return false;
            }
        }
    }
    // All captions are obstacles from the outset. Simulate the renderer's letter
    // placement against every caption and previous letter, with normal text gap.
    let text_box = |text: &str, at: [f64; 2]| {
        let glyph = crate::dims::font().text(text, crate::section::FONT_SIZE);
        [
            at[0] + glyph.min[0],
            at[1] + glyph.min[1],
            at[0] + glyph.max[0],
            at[1] + glyph.max[1],
        ]
    };
    let clear = |b: Rect, other: Rect| {
        b[2] + 1.75 <= other[0]
            || other[2] + 1.75 <= b[0]
            || b[3] + 1.75 <= other[1]
            || other[3] + 1.75 <= b[1]
    };
    let mut labels = text_obstacles(p);
    for v in p.views.iter().filter(|v| v.detail.is_some()) {
        let b = text_box(&caption(v.detail.as_ref().unwrap()), caption_at(v));
        if labels.iter().any(|other| !clear(b, *other)) {
            return false;
        }
        labels.push(b);
    }
    for v in p.views.iter().filter(|v| v.detail.is_some()) {
        let d = v.detail.as_ref().unwrap();
        let Some(source) = p.views.iter().find(|s| s.id == d.source_view) else {
            return false;
        };
        let mut avoid = labels.clone();
        avoid.push(p.sheet.title_block);
        avoid.extend(
            p.dimensions
                .iter()
                .filter_map(|d| d.placed.as_ref())
                .flat_map(|q| q.lines.iter())
                .map(|s| {
                    [
                        s[0][0].min(s[1][0]) - 0.5,
                        s[0][1].min(s[1][1]) - 0.5,
                        s[0][0].max(s[1][0]) + 0.5,
                        s[0][1].max(s[1][1]) + 0.5,
                    ]
                }),
        );
        let (_, at) = marker(a, d, source, &p.views, p.sheet.page, &avoid);
        let b = text_box(&d.label, at);
        if labels.iter().any(|other| !clear(b, *other)) {
            return false;
        }
        labels.push(b);
    }
    true
}

pub(crate) fn size(d: &Detail) -> [f64; 2] {
    let axes = if d.source_view == "v:side" {
        [1, 2]
    } else if d.source_view == "v:plan" {
        [0, 1]
    } else {
        [0, 2]
    };
    axes.map(|k| (d.bounds[1][k] - d.bounds[0][k]) * d.scale)
}
fn pads(d: &Detail) -> [f64; 4] {
    let mut pads = if d.kind == "readability-depiction" {
        [8.0, 8.0, 12.0, 8.0]
    } else if (d.kind.starts_with("readability-height-interval")
        && d.dimensions
            .iter()
            .all(|id| id.contains("/step_interval.length.")))
        || (d.kind == "readability"
            && d.dimensions
                .iter()
                .all(|id| id.contains("/step_height.length")))
    {
        // Baseline heights own vertical lanes only. Reserving the generic
        // diameter/callout bands here needlessly purchases a larger sheet.
        [4.0, 28.0, 8.0, 8.0]
    } else if d.kind.starts_with("readability") {
        [12.0, 42.0, 12.0, 12.0]
    } else if d.kind == "prismatic-steps" {
        [0.0, 42.0, 14.0, 6.0]
    } else if d.kind == "y-turned-chain" {
        [
            12.0,
            42.0,
            14.0 + 7.0
                * d.dimensions
                    .iter()
                    .filter(|id| id.ends_with("/step.length"))
                    .count() as f64,
            12.0,
        ]
    } else {
        // X-profile lengths stack above the silhouette. Reserve every owned row,
        // including the text's baseline lift, before placing this detail block.
        let top = if d.axis == 0 {
            let rows = d
                .dimensions
                .iter()
                .filter(|id| id.ends_with("/step.length"))
                .count();
            crate::dims::FIRST
                + rows.saturating_sub(1) as f64 * crate::dims::LANE
                + 2.0 * crate::dims::TEXT
        } else {
            12.0
        };
        [12.0, 42.0, 28.0, top]
    };
    if d.kind.starts_with("readability") {
        let half_caption = (crate::dims::font().advance(&caption(d), crate::section::FONT_SIZE)
            + crate::section::FONT_SIZE)
            / 2.0;
        let half_view = size(d)[0] / 2.0;
        pads[0] = pads[0].max(half_caption - half_view);
        pads[1] = pads[1].max(half_caption - half_view);
    }
    pads
}
pub(crate) fn at(d: &Detail, area: Rect) -> [f64; 2] {
    let [w, h] = size(d);
    let [left, _, bottom, _] = pads(d);
    [area[0] + left + w / 2.0, area[1] + bottom + h / 2.0]
}
pub(crate) fn scaled(a: &Analysis, spec: &Spec, sheet: f64) -> Scaled {
    let requests = requests(a, spec, sheet);
    let mut scaled = Scaled {
        angular: crate::dims::angular::notch_reservations(a, spec, sheet),
        pack_derived_at_edges: !requests.is_empty()
            && requests
                .iter()
                .all(|(_, d)| d.kind.starts_with("readability")),
        derived: requests
            .iter()
            .map(|(id, d)| {
                let [w, h] = size(d);
                let [left, right, bottom, top] = pads(d);
                (
                    id.clone(),
                    (w + left + right).max(if d.kind.starts_with("readability") {
                        45.0
                    } else {
                        65.0
                    }),
                    h + bottom + top,
                )
            })
            .collect(),
    };
    if scaled.pack_derived_at_edges {
        scaled.derived.sort_by(|a, b| {
            (b.1 * b.2)
                .total_cmp(&(a.1 * a.2))
                .then_with(|| a.0.cmp(&b.0))
        });
    }
    scaled
}
pub(crate) fn section_label(spec: &Spec) -> String {
    if spec.views.contains_key(crate::model::SECTION) {
        return "A".into();
    }
    NAMES
        .iter()
        .find(|n| {
            !spec
                .views
                .get(**n)
                .is_some_and(|v| !v.omit && v.detail.is_some())
        })
        .unwrap_or(&"detail_a")
        .trim_start_matches("detail_")
        .to_uppercase()
}
pub(crate) fn caption(d: &Detail) -> String {
    format!(
        "DETAIL {} ({}){}",
        d.label,
        crate::model::scale_name(d.scale),
        if d.partial_profile && !d.kind.starts_with("readability") {
            " — PARTIAL PROFILE"
        } else {
            ""
        }
    )
}
/// Use the crop's reserved row for both placement and drawing, even when the
/// projected silhouette leaves blank space inside the crop.
pub(crate) fn caption_at(v: &PlannedView) -> [f64; 2] {
    let [_, h] = size(v.detail.as_ref().expect("detail caption"));
    [v.at[0], v.at[1] - h / 2.0 - 8.0]
}

/// Reserve the caption before solving dimensions, including its half-text-height gap.
/// Otherwise a dimension below the crop can occupy the newly corrected caption row.
pub(crate) fn caption_obstacles(views: &[PlannedView]) -> Vec<crate::dims::PlannedDimension> {
    views
        .iter()
        .filter_map(|v| {
            let d = v.detail.as_ref()?;
            let text = caption(d);
            let height = crate::section::FONT_SIZE;
            let laid = crate::dims::font().text(&text, height);
            let at = caption_at(v);
            let gap = height / 2.0;
            let mut obstacle = crate::iso_caption::proxy(&crate::iso_caption::PlannedCaption {
                id: format!("detail_caption_{}", d.label),
                view: v.id.clone(),
                sheet: v.sheet,
                text,
                height,
                at,
                ink: [
                    at[0] + laid.min[0] - gap,
                    at[1] + laid.min[1] - gap,
                    at[0] + laid.max[0] + gap,
                    at[1] + laid.max[1] + gap,
                ],
                default: true,
                basis: "detail caption and readable clearance".into(),
                state: crate::items::State::Shown,
            });
            obstacle.kind = "ink".into();
            Some(obstacle)
        })
        .collect()
}
pub(crate) fn crop_extent(a: &Analysis, v: &PlannedView) -> ([f64; 2], [f64; 2]) {
    crate::model::envelope_extent(
        v.detail.as_ref().map_or(a.envelope, |d| d.bounds),
        &haecceity::hlr::View::new(v.direction, v.up).expect("planned view"),
    )
}

const CLIP_ROUNDOFF: f64 = 1e-8;

/// Clip exact curves recursively by their conservative bounding boxes. Fully contained
/// intervals retain their exact representation; only boundary intervals are subdivided.
pub(crate) fn clip(curve: crate::output::Curve, rect: Rect) -> Vec<crate::output::Curve> {
    // Lines have an exact intersection; avoid dozens of tiny recursive intervals at a cut.
    if let crate::output::Curve::Line(a, b) = curve {
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for k in 0..2 {
            let delta = b[k] - a[k];
            if delta.abs() < 1e-12 {
                if a[k] < rect[k] || a[k] > rect[k + 2] {
                    return Vec::new();
                }
            } else {
                let p = (rect[k] - a[k]) / delta;
                let q = (rect[k + 2] - a[k]) / delta;
                lo = lo.max(p.min(q));
                hi = hi.min(p.max(q));
            }
        }
        return if hi > lo {
            vec![crate::output::Curve::Line(
                [0, 1].map(|k| a[k] + lo * (b[k] - a[k])),
                [0, 1].map(|k| a[k] + hi * (b[k] - a[k])),
            )]
        } else {
            Vec::new()
        };
    }
    fn visit(c: crate::output::Curve, r: Rect, depth: u8, out: &mut Vec<crate::output::Curve>) {
        let b = c.bbox();
        if b[2] < r[0] - CLIP_ROUNDOFF
            || b[0] > r[2] + CLIP_ROUNDOFF
            || b[3] < r[1] - CLIP_ROUNDOFF
            || b[1] > r[3] + CLIP_ROUNDOFF
        {
            return;
        }
        if b[0] >= r[0] - CLIP_ROUNDOFF
            && b[1] >= r[1] - CLIP_ROUNDOFF
            && b[2] <= r[2] + CLIP_ROUNDOFF
            && b[3] <= r[3] + CLIP_ROUNDOFF
        {
            out.push(c);
            return;
        }
        if depth == 32 || (b[2] - b[0]).max(b[3] - b[1]) < 1e-7 {
            return;
        }
        visit(c.trim(0.0, 0.5), r, depth + 1, out);
        visit(c.trim(0.5, 1.0), r, depth + 1, out);
    }
    let mut out = Vec::new();
    visit(curve, rect, 0, &mut out);
    out
}

/// Keep page-marker ink inside the exact drawable border. The curve clipper's
/// numerical allowance is useful at feature boundaries; an inset of twice that
/// allowance prevents it from exporting a page marker just outside the sheet.
pub(crate) fn clip_marker(
    circle: crate::output::Curve,
    drawable: Rect,
) -> Vec<crate::output::Curve> {
    let guard = 2.0 * CLIP_ROUNDOFF;
    clip(
        circle,
        [
            drawable[0] + guard,
            drawable[1] + guard,
            drawable[2] - guard,
            drawable[3] - guard,
        ],
    )
}

/// A rectangular crop is an interrupted view, not a manufactured closing edge.
/// ISO 128 thin zigzag break lines join every cut endpoint on each affected side.
/// The parent circle identifies the region; the break line states where material continues.
#[derive(Default)]
pub(crate) struct Boundary {
    contacts: [Vec<f64>; 4],
}
impl Boundary {
    pub(crate) fn add(
        &mut self,
        original: &crate::output::Curve,
        clipped: &[crate::output::Curve],
        r: Rect,
    ) {
        let b = original.bbox();
        for (side, axis, edge, cut) in [
            (0, 0, r[0], b[0] < r[0] - 1e-7),
            (1, 1, r[1], b[1] < r[1] - 1e-7),
            (2, 0, r[2], b[2] > r[2] + 1e-7),
            (3, 1, r[3], b[3] > r[3] + 1e-7),
        ] {
            if cut {
                // An adjoining edge may start exactly on the crop and lie entirely
                // outside it. Its zero-length intersection still needs a break contact.
                for p in clipped
                    .iter()
                    .flat_map(|c| [c.at(0.0), c.at(1.0)])
                    .chain([original.at(0.0), original.at(1.0)])
                {
                    if (p[axis] - edge).abs() < 1e-6
                        && p[1 - axis] >= r[1 - axis] - 1e-6
                        && p[1 - axis] <= r[3 - axis] + 1e-6
                    {
                        self.contacts[side].push(p[1 - axis]);
                    }
                }
            }
        }
    }
    pub(crate) fn ink(mut self, r: Rect) -> Vec<crate::output::Curve> {
        use crate::output::Curve;
        let mut out = Vec::new();
        for (side, points) in self.contacts.iter_mut().enumerate() {
            if points.is_empty() {
                continue;
            }
            points.sort_by(f64::total_cmp);
            points.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
            let axis = side % 2;
            let edge = r[side];
            let inward = if side < 2 { 1.0 } else { -1.0 };
            let lo = (points[0] - 1.0).max(r[1 - axis]);
            let hi = (points[points.len() - 1] + 1.0).min(r[3 - axis]);
            let map = |t: f64, offset: f64| {
                if axis == 0 {
                    [edge + inward * offset, t]
                } else {
                    [t, edge + inward * offset]
                }
            };
            let mut stops = vec![lo];
            stops.extend(points.iter().copied());
            stops.push(hi);
            stops.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
            for pair in stops.windows(2) {
                let [a, b] = [pair[0], pair[1]];
                let mid = (a + b) / 2.0;
                let tooth = ((b - a) / 4.0).min(0.6);
                let line = [
                    map(a, 0.0),
                    map(mid - tooth, 0.0),
                    map(mid, tooth),
                    map(mid + tooth, 0.0),
                    map(b, 0.0),
                ];
                out.extend(
                    line.windows(2)
                        .filter(|p| p[0] != p[1])
                        .map(|p| Curve::Line(p[0], p[1])),
                );
            }
        }
        out
    }
}

pub(crate) fn marker(
    a: &Analysis,
    d: &Detail,
    source: &PlannedView,
    views: &[PlannedView],
    page: [f64; 2],
    avoid: &[Rect],
) -> (crate::output::Curve, [f64; 2]) {
    let view = haecceity::hlr::View::new(source.direction, source.up).expect("source view");
    let (size, centre) = crate::model::envelope_extent(d.marked_bounds, &view);
    let parent = crate::model::extent(a, &view).1;
    let at = [0, 1].map(|k| source.at[k] + source.scale * (centre[k] - parent[k]));
    let radius = source.scale * size[0].hypot(size[1]) / 2.0 + 1.0;
    let glyph = crate::dims::font().text(&d.label, crate::section::FONT_SIZE);
    let mut obstacles: Vec<_> = views
        .iter()
        .map(|v| {
            if v.id == "v:iso" {
                let b = a.iso_bounds;
                [
                    v.at[0] + v.scale * b[0],
                    v.at[1] + v.scale * b[1],
                    v.at[0] + v.scale * b[2],
                    v.at[1] + v.scale * b[3],
                ]
            } else {
                let (size, _) = crop_extent(a, v);
                [
                    v.at[0] - v.scale * size[0] / 2.0,
                    v.at[1] - v.scale * size[1] / 2.0,
                    v.at[0] + v.scale * size[0] / 2.0,
                    v.at[1] + v.scale * size[1] / 2.0,
                ]
            }
        })
        .collect();
    obstacles.extend_from_slice(avoid);
    let preferred = [at[0] - radius - 2.0, at[1] + radius + 2.0];
    let positions = std::iter::once(preferred).chain(
        [4.0, 8.0, 12.0, 20.0, 28.0].into_iter().flat_map(|gap| {
            (0..8).map(move |i| {
                let theta = (3.0 - i as f64) * std::f64::consts::FRAC_PI_4;
                [
                    at[0] + (radius + gap) * theta.cos(),
                    at[1] + (radius + gap) * theta.sin(),
                ]
            })
        }),
    );
    let gap = if !d.authored { 1.75 } else { 1.0 };
    let label_at = positions
        .into_iter()
        .find(|p| {
            let b = [
                p[0] + glyph.min[0] - gap,
                p[1] + glyph.min[1] - gap,
                p[0] + glyph.max[0] + gap,
                p[1] + glyph.max[1] + gap,
            ];
            b[0] >= crate::layout::MARGIN
                && b[1] >= crate::layout::MARGIN
                && b[2] <= page[0] - crate::layout::MARGIN
                && b[3] <= page[1] - crate::layout::MARGIN
                && obstacles
                    .iter()
                    .all(|o| b[2] <= o[0] || b[0] >= o[2] || b[3] <= o[1] || b[1] >= o[3])
        })
        .unwrap_or(preferred);
    (
        crate::output::Curve::Conic {
            centre: at,
            radii: [radius; 2],
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            t0: 0.0,
            t1: std::f64::consts::TAU,
        },
        label_at,
    )
}

/// Detail boundaries are late furniture: interrupt the circle behind annotation text,
/// retaining exact arcs and a small readable gap. Dimensions keep their settled ink.
pub(crate) fn marker_ink(
    circle: crate::output::Curve,
    labels: &[Rect],
) -> Vec<crate::output::Curve> {
    fn subtract(c: crate::output::Curve, r: Rect, depth: u8, out: &mut Vec<crate::output::Curve>) {
        let b = c.bbox();
        if b[2] < r[0] || b[0] > r[2] || b[3] < r[1] || b[1] > r[3] {
            out.push(c);
            return;
        }
        if b[0] >= r[0] && b[1] >= r[1] && b[2] <= r[2] && b[3] <= r[3] {
            return;
        }
        if depth == 32 || (b[2] - b[0]).max(b[3] - b[1]) < 1e-7 {
            return;
        }
        subtract(c.trim(0.0, 0.5), r, depth + 1, out);
        subtract(c.trim(0.5, 1.0), r, depth + 1, out);
    }
    let mut arcs = vec![circle];
    for b in labels {
        let r = [b[0] - 1.0, b[1] - 1.0, b[2] + 1.0, b[3] + 1.0];
        arcs = arcs
            .into_iter()
            .flat_map(|c| {
                let mut out = Vec::new();
                subtract(c, r, 0, &mut out);
                out
            })
            .collect();
    }
    arcs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Curve;

    #[test]
    fn page_marker_arcs_do_not_export_the_clippers_roundoff_outside_the_border() {
        let circle = Curve::Conic {
            centre: [149.75, 67.1500272280282],
            radii: [66.06919519533733; 2],
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            t0: 0.0,
            t1: std::f64::consts::TAU,
        };
        let drawable = [10.0, 10.0, 584.0, 410.0];
        assert!(circle.bbox()[1] < drawable[1]);
        let arcs = clip_marker(circle, drawable);
        assert!(!arcs.is_empty());
        for arc in &arcs {
            let b = arc.bbox();
            assert!(b[0] >= drawable[0] && b[1] >= drawable[1]);
            assert!(b[2] <= drawable[2] && b[3] <= drawable[3]);
        }
        let bottom = arcs
            .iter()
            .map(|arc| arc.bbox()[1])
            .fold(f64::INFINITY, f64::min);
        assert!(bottom < drawable[1] + 1e-6);
    }

    #[test]
    fn every_cut_endpoint_meets_a_thin_break_and_whole_curves_need_none() {
        let r = [-1.0, -1.0, 1.0, 1.0];
        let curves = [
            Curve::Line([-2.0, -0.5], [2.0, -0.5]),
            Curve::Line([-2.0, 0.5], [2.0, 0.5]),
        ];
        let mut boundary = Boundary::default();
        for c in &curves {
            boundary.add(c, &clip(c.clone(), r), r);
        }
        let ink = boundary.ink(r);
        assert!(!ink.is_empty());
        for x in [-1.0, 1.0] {
            for y in [-0.5, 0.5] {
                assert!(ink.iter().any(|c| {
                    [c.at(0.0), c.at(1.0)]
                        .iter()
                        .any(|p| (p[0] - x).abs() < 1e-6 && (p[1] - y).abs() < 1e-6)
                }));
            }
        }
        // A zigzag has depth, so it cannot be mistaken for a manufactured closing edge.
        assert!(
            ink.iter()
                .any(|c| (c.at(0.0)[0] - c.at(1.0)[0]).abs() > 0.1)
        );
        let c = Curve::Line([-0.5, 0.0], [0.5, 0.0]);
        let mut boundary = Boundary::default();
        boundary.add(&c, &clip(c.clone(), r), r);
        assert!(boundary.ink(r).is_empty());
    }

    #[test]
    fn a_crop_at_a_shoulder_joins_continuing_edges_even_with_zero_retained_length() {
        let r = [-1.0, -1.0, 1.0, 1.0];
        let mut boundary = Boundary::default();
        for y in [-0.5, 0.5] {
            let edge = Curve::Line([1.0, y], [3.0, y]);
            let clipped = clip(edge.clone(), r);
            assert!(clipped.is_empty());
            boundary.add(&edge, &clipped, r);
        }
        let ink = boundary.ink(r);
        for y in [-0.5, 0.5] {
            assert!(ink.iter().any(|c| [c.start(), c.end()].contains(&[1.0, y])));
        }
        assert!(ink.iter().any(|c| c.start()[0] != c.end()[0]));
    }

    #[test]
    fn a_crop_keeps_exact_curves_and_removes_external_geometry() {
        let c = Curve::Conic {
            centre: [0.0, 0.0],
            radii: [2.0, 2.0],
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            t0: 0.0,
            t1: std::f64::consts::TAU,
        };
        let pieces = clip(c, [-1.0, -3.0, 1.0, 3.0]);
        assert!(!pieces.is_empty());
        for p in pieces {
            assert!(matches!(p, Curve::Conic { .. }));
            let b = p.bbox();
            assert!(b[0] >= -1.0 - 1e-7 && b[2] <= 1.0 + 1e-7);
        }
        assert!(clip(Curve::Line([3.0, 0.0], [4.0, 0.0]), [-1.0, -1.0, 1.0, 1.0]).is_empty());
    }
}
