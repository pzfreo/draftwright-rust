//! A part's AP242 PMI lowered to drawing items: stage 2a of drawing PMI. draftwright lowers its
//! PMI records in `model/detect.py` (`build_pmi_features`): a complete geometric tolerance
//! becomes a feature control frame (`ControlFrame`), a complete datum a datum feature symbol
//! (`DatumRef`), each with the view and strip it is drawn in and the model-space site its leader
//! hangs from (`model/declare.py`'s `gdt_target`, and `annotations/_gdt.py`'s
//! `_datum_alternate_view`). This module makes the same decisions from the Analysis's PMI
//! ([`crate::pmi::Reading`]); [`crate::dims::gdt`] places and draws what it lowers.
//!
//! - **A feature control frame** carries the record's characteristic, its magnitude as
//!   draftwright prints it (`_fmt_pmi_magnitude`), the zone's `⌀` or `S⌀`, the material
//!   condition (`Ⓜ`, `Ⓛ`), the datum letters in precedence order with repeats removed, and the
//!   all-around or all-over scope. It is drawn in the view that sees its axis end-on (draftwright's
//!   face-on rule: the faces' common cylinder axis when every face is a cylinder on one, else
//!   their box's longest extent, else z), in that view's roomiest strip (below the front and the
//!   side, above the plan), its leader from the centre of the faces' box.
//! - **A datum feature symbol** carries its letter. It is drawn in the view that sees its plane
//!   or cylinder edge-on (x and z: the front; y: the side), from a proven site on its first face,
//!   on the side its outward normal points to (a plane's) or its site lies on (a cylinder's): a
//!   y-normal plane whose side view is not shown is drawn in the plan instead.
//!
//! What is not lowered is said, never guessed: a record whose reading left blockers (datum
//! targets, composite relations, projected or non-uniform zones, datum modifiers, common
//! datums, …), a modifier draftwright has no drawing for (only all-around, all-over, the zone's
//! form and the material condition are drawn, as in draftwright), or an attachment its faces do
//! not prove (`pmi_not_lowered`); a source the reading did not read (`pmi_not_extracted`); a file
//! whose PMI could not be read (`pmi_not_read`); presentation, which is counted, not read
//! (`pmi_presentation_not_read`). Toleranced and basic dimensions are the next stage: each is
//! reported as not drawn yet (`pmi_dimensions_not_ported`), as `dimensions_not_ported` reports a
//! feature whose dimensions are not ported.
//!
//! draftwright draws its PMI only when asked (`pmi="annotate"`; its default is off). The port has
//! no such option: a part's PMI is drawn whenever it has some (a question for the maintainer in
//! the stream's report).

use crate::model::{Analysis, Problem};
use crate::pmi::{Category, Outcome, Reading, Record};

/// A record lowered to a drawing item, before placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Lowered {
    /// `pmi:<source_id>`, e.g. `pmi:geometric_tolerance:#123`.
    pub id: String,
    pub source: String,
    pub glyph: Glyph,
    /// The view it is drawn in, `front`, `plan` or `side`, and its strip there: `above`,
    /// `below`, `left` or `right`.
    pub view: &'static str,
    pub side: &'static str,
    /// Where its leader hangs from, mm in the drawing's frame.
    pub site: [f64; 3],
    /// A datum's other proven sites on its face, for placement to fall back on.
    pub alternatives: Vec<[f64; 3]>,
    pub faces: Vec<usize>,
    pub basis: String,
}

/// What a lowered record draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Glyph {
    ControlFrame {
        /// draftwright's characteristic name (`position`, `flatness`, …).
        characteristic: String,
        /// The magnitude as printed.
        tolerance: String,
        /// `diameter` (`⌀`) or `spherical_diameter` (`S⌀`).
        zone: Option<&'static str>,
        /// `M` or `L`.
        material: Option<&'static str>,
        datums: Vec<String>,
        all_around: bool,
        all_over: bool,
    },
    Datum {
        letter: String,
        /// `plane` or `cylinder`.
        surface: String,
    },
}

/// Modifiers a feature control frame draws (draftwright's `_SUPPORTED_GTOL_SCOPE_MODIFIERS` and
/// its zone and material qualifiers).
const SCOPE: [&str; 2] = ["all_around", "all_over"];
const QUALIFIERS: [&str; 4] = [
    "diameter_zone",
    "spherical_diameter_zone",
    "maximum_material_requirement",
    "least_material_requirement",
];

/// The view that sees world axis *k* end-on (draftwright's `_END_ON`), and its feature strip
/// (`_FEATURE_SIDE`).
fn face_on(k: usize) -> (&'static str, &'static str) {
    match k {
        0 => ("side", "below"),
        1 => ("front", "below"),
        _ => ("plan", "above"),
    }
}

fn axis_index(a: &str) -> Option<usize> {
    ["X", "Y", "Z"].iter().position(|x| *x == a)
}

/// The Analysis's PMI lowered: the items to draw, and the problems met (none when the part has
/// no PMI). *shown* says whether a principal view (`front`, `plan`, `side`) is on the sheet.
pub fn lower(analysis: &Analysis, shown: &dyn Fn(&str) -> bool) -> (Vec<Lowered>, Vec<Problem>) {
    let mut items = Vec::new();
    let mut problems = Vec::new();
    let report = match &analysis.pmi {
        None => return (items, problems),
        Some(Reading::Refused(why)) => {
            problems.push(Problem {
                id: "p:pmi_not_read:part".into(),
                code: "pmi_not_read".into(),
                severity: "error".into(),
                message: format!(
                    "The STEP file carries AP242 semantic PMI that could not be read ({why}); none of it is drawn."
                ),
                subjects: vec!["part".into()],
            });
            return (items, problems);
        }
        Some(Reading::Read(r)) => r,
    };
    for s in report
        .sources
        .iter()
        .filter(|s| s.outcome == Outcome::NotExtracted)
    {
        // A requirement lost is an error; a shape aspect, property or standard the reader did
        // not read states no requirement on its own.
        let requirement = s.category != Category::Other;
        problems.push(Problem {
            id: format!("p:pmi_not_extracted:{}", s.id),
            code: "pmi_not_extracted".into(),
            severity: if requirement { "error" } else { "info" }.into(),
            message: format!(
                "PMI source {} ({}) is not read, so it is not drawn: {}.",
                s.id, s.entity, s.reason
            ),
            subjects: vec![s.id.clone()],
        });
    }
    if report.accounting.presentation > 0 {
        problems.push(Problem {
            id: "p:pmi_presentation_not_read:part".into(),
            code: "pmi_presentation_not_read".into(),
            severity: "info".into(),
            message: format!(
                "The STEP file carries {} instance(s) of PMI presentation (graphic annotation); presentation is counted, not read or drawn: only semantic PMI is.",
                report.accounting.presentation
            ),
            subjects: vec!["part".into()],
        });
    }
    let centre = {
        let [lo, hi] = analysis.envelope;
        [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]))
    };
    for r in &report.records {
        match r.category {
            Category::Dimension => problems.push(Problem {
                id: format!("p:pmi_dimensions_not_ported:{}", r.source_id),
                code: "pmi_dimensions_not_ported".into(),
                severity: "info".into(),
                message: format!(
                    "PMI dimension {} ({}) is not drawn: drawing AP242 dimensions is not ported yet.",
                    r.source_id,
                    if r.label.is_empty() { &r.kind } else { &r.label }
                ),
                subjects: vec![r.source_id.clone()],
            }),
            Category::GeometricTolerance | Category::Datum => {
                let lowered = if r.category == Category::Datum {
                    datum(r, centre)
                } else {
                    control_frame(r, centre)
                };
                match lowered {
                    Ok(mut item) => {
                        // draftwright's `_datum_alternate_view`: a y-normal plane whose side
                        // view is not drawn is shown in the plan, below for left and above for
                        // right.
                        if let Glyph::Datum { surface, .. } = &item.glyph
                            && surface == "plane"
                            && item.view == "side"
                            && !shown("side")
                            && shown("plan")
                        {
                            item.side = if item.side == "left" { "below" } else { "above" };
                            item.view = "plan";
                            item.basis += "; the side view is not drawn, so the plan carries it";
                        }
                        items.push(item);
                    }
                    Err(reasons) => problems.push(Problem {
                        id: format!("p:pmi_not_lowered:{}", r.source_id),
                        code: "pmi_not_lowered".into(),
                        severity: "error".into(),
                        message: format!(
                            "PMI {} ({}) is not drawn: {}.",
                            r.source_id,
                            r.label,
                            reasons.join("; ")
                        ),
                        subjects: vec![r.source_id.clone()],
                    }),
                }
            }
            // Manufacturing requirements and surface labels are read, not drawn yet (draftwright
            // draws them: a requirements table, thread and knurl callouts, labels): said, never
            // dropped. A face's requirement is a warning; a part's note or general tolerance is
            // information.
            _ => {
                let face = r.kind.ends_with("_thread")
                    || r.kind == "knurl"
                    || !r.faces.is_empty()
                    || !r.edges.is_empty();
                problems.push(Problem {
                    id: format!("p:pmi_not_drawn:{}", r.source_id),
                    code: "pmi_not_drawn".into(),
                    severity: if face { "warning" } else { "info" }.into(),
                    message: format!(
                        "PMI {} {} ({}) is read but not drawn: drawing it is not ported yet.",
                        r.kind.replace('_', " "),
                        r.source_id,
                        r.label
                    ),
                    subjects: vec![r.source_id.clone()],
                });
            }
        }
    }
    (items, problems)
}

/// What stops a record lowering: its reading's blockers, then its attachment's.
fn blockers(r: &Record) -> Vec<String> {
    let mut out = r.blockers.clone();
    if let Some(g) = &r.geometry {
        out.extend(g.reasons.iter().cloned());
    }
    out
}

/// A geometric tolerance as a feature control frame (`build_pmi_features`' first branch and
/// `control_frame`), or why it is not one.
fn control_frame(r: &Record, centre: [f64; 3]) -> Result<Lowered, Vec<String>> {
    let mut reasons = blockers(r);
    let scope: Vec<&str> = r
        .gtol_modifiers
        .iter()
        .map(String::as_str)
        .filter(|m| !QUALIFIERS.contains(m))
        .collect();
    for m in &scope {
        if !SCOPE.contains(m) {
            reasons.push(format!(
                "geometric-tolerance modifier '{m}' is not supported"
            ));
        }
    }
    if scope.len() > 1 {
        reasons.push(format!(
            "geometric-tolerance modifier combination ({}) is not supported",
            scope.join(", ")
        ));
    }
    let Some(value) = r.value else {
        reasons.push("the tolerance states no magnitude".into());
        return Err(dedup(reasons));
    };
    if !reasons.is_empty() {
        return Err(dedup(reasons));
    }
    let has = |m: &str| r.gtol_modifiers.iter().any(|x| x == m);
    let site = r
        .ref_bbox
        .map_or(centre, |b| [0, 1, 2].map(|k| 0.5 * (b[k] + b[k + 3])));
    let proven = r.geometry.as_ref().and_then(|g| g.axis.as_deref());
    let (axis, why) = match (proven, r.dominant_axis.as_str()) {
        (Some(a), _) => (a, "its faces' common cylinder axis"),
        (None, a @ ("X" | "Y" | "Z")) => (a, "its faces' longest extent"),
        _ => ("Z", "no axis of its own (z by default)"),
    };
    let k = axis_index(axis).expect("an axis letter");
    let (view, side) = face_on(k);
    let mut datums: Vec<String> = Vec::new();
    for d in &r.datum_refs {
        if !datums.contains(d) {
            datums.push(d.clone());
        }
    }
    Ok(Lowered {
        id: format!("pmi:{}", r.source_id),
        source: r.source_id.clone(),
        glyph: Glyph::ControlFrame {
            characteristic: r.kind.clone(),
            tolerance: crate::pmi::fmt_magnitude(value, None),
            zone: if has("diameter_zone") {
                Some("diameter")
            } else if has("spherical_diameter_zone") {
                Some("spherical_diameter")
            } else {
                None
            },
            material: if has("maximum_material_requirement") {
                Some("M")
            } else if has("least_material_requirement") {
                Some("L")
            } else {
                None
            },
            datums,
            all_around: has("all_around"),
            all_over: has("all_over"),
        },
        view,
        side,
        site,
        alternatives: Vec::new(),
        faces: r.faces.clone(),
        basis: format!(
            "source-authored: the AP242 {} tolerance {}, drawn where {} ({}) is seen end-on, in the {view} view's {side} strip (draftwright's face-on rule), its leader from the centre of its faces",
            r.kind.replace('_', " "),
            r.source_id,
            why,
            axis.to_lowercase()
        ),
    })
}

/// A datum as a datum feature symbol (`build_pmi_features`' datum branch and `datum`), or why it
/// is not one.
fn datum(r: &Record, centre: [f64; 3]) -> Result<Lowered, Vec<String>> {
    let mut reasons = blockers(r);
    let g = r.geometry.as_ref();
    let axis = g.and_then(|g| g.axis.as_deref());
    let surface = g.and_then(|g| g.surface.clone());
    let site = g.and_then(|g| g.sites.first().copied());
    if r.label.trim().is_empty() {
        reasons.push("the datum has no letter".into());
    }
    let (Some(axis), Some(surface), Some(site)) = (axis, surface, site) else {
        if reasons.is_empty() {
            reasons.push("datum reference surface is unavailable".into());
        }
        return Err(dedup(reasons));
    };
    if !reasons.is_empty() {
        return Err(dedup(reasons));
    }
    let k = axis_index(axis).expect("an axis letter");
    let view = if k == 1 { "side" } else { "front" };
    let planar = surface == "plane";
    // A plane's leader follows its normal in the edge-on view; a cylinder attaches radially to
    // its profile.
    let (across, low, high) = if planar && k < 2 {
        (k, "left", "right")
    } else if !planar && k == 2 {
        (0, "left", "right")
    } else {
        (2, "below", "above")
    };
    let side = match g.and_then(|g| g.normal) {
        Some(n) if n[across].abs() < 0.9 => {
            return Err(vec![
                "datum surface normal cannot be shown by an axial leader in this view".into(),
            ]);
        }
        Some(n) => {
            if n[across] > 0.0 {
                high
            } else {
                low
            }
        }
        None if planar => {
            return Err(vec!["datum reference surface has no outward normal".into()]);
        }
        None => {
            if site[across] >= centre[across] {
                high
            } else {
                low
            }
        }
    };
    let how = if planar {
        "its outward normal points to"
    } else {
        "its site on the profile lies on"
    };
    Ok(Lowered {
        id: format!("pmi:{}", r.source_id),
        source: r.source_id.clone(),
        glyph: Glyph::Datum {
            letter: r.label.trim().to_string(),
            surface: surface.clone(),
        },
        view,
        side,
        site,
        alternatives: g.map(|g| g.alternatives.clone()).unwrap_or_default(),
        faces: r.faces.clone(),
        basis: format!(
            "source-authored: the AP242 datum {} ({}) on a {surface} along {}, drawn edge-on in the {view} view on the {side} side {how}",
            r.label.trim(),
            r.source_id,
            axis.to_lowercase()
        ),
    })
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    v.retain(|s| seen.insert(s.clone()));
    v
}
