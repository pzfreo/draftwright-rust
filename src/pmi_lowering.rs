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
//! - **A surface label or common label** (text on edges or faces: `A`, `F1.1`, `2X R`) carries
//!   its text, drawn with a leader from its first proven site (draftwright's `ref_pts[0]`: a
//!   surface label's edge at its parameter midpoint, a common label's face at the vertex nearest
//!   the face's box centre, [`label_geometry`]) in the front view, or the side view when its
//!   references' box is longest along y, above or below by whether the site is above the part's
//!   centre (`build_pmi_features`' label branch, drawn as `render_gdt` draws a `Note`).
//! - **A thread or knurl** ([`manufacturing`], `lower_ap242_manufacturing_requirements`) is no
//!   item of its own: it is given to the one hole or turned step its cylinder proves, its words
//!   and structured statement reconciled and its text checked against the cylinder, and that
//!   feature's callout prints its suffix (`⌀1.6 ↧ 4 M2 x 0.4-6H RH; 2.5 MIN FULL THREAD`), or,
//!   with two or more and one suffix over 52 characters, `SEE MFG n` and the manufacturing
//!   requirements table states it ([`crate::tables::manufacturing`]).
//! - **Drawing-wide requirements** ([`sheet_requirements`], `lower_ap242_document_requirements`):
//!   the one general tolerance fills the title block's tolerance cell, the one default surface
//!   texture (`Ra 3.2 µm unless otherwise specified`) is drawn beside the title block, and each
//!   edge condition (`Break sharp edges 0.2 max`) is a general note; two of one default are
//!   ambiguous and neither is drawn.
//!
//! What is not lowered is said, never guessed: a record whose reading left blockers (datum
//! targets, composite relations, projected or non-uniform zones, datum modifiers, common
//! datums, …), a modifier draftwright has no drawing for (only all-around, all-over, the zone's
//! form and the material condition are drawn, as in draftwright), an attachment its faces do
//! not prove, or a thread or knurl no one feature owns (`pmi_not_lowered`); a source the reading
//! did not read (`pmi_not_extracted`); a file whose PMI could not be read (`pmi_not_read`);
//! presentation, which is counted, not read (`pmi_presentation_not_read`). Toleranced and basic
//! dimensions are lowered with the other dimensions ([`crate::dims`]' PMI dimensions, stage 2b),
//! from where their faces put them ([`dimension_geometry`], here).
//!
//! draftwright draws its PMI only when asked (`pmi="annotate"`; its default is off). The port has
//! no such option: a part's PMI is drawn whenever it has some (a question for the maintainer in
//! the stream's report).

use std::collections::{BTreeMap, BTreeSet};

use haecceity::Part;
use haecceity::geom::Surface;
use serde::{Deserialize, Serialize};

use crate::model::{Analysis, Problem};
use crate::pmi::{Category, Geometry, Identities, Outcome, Reading, Record};

/// A record lowered to a drawing item, before placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Lowered {
    /// The record's item ID, `pmi:<identity>` ([`crate::pmi::identity`]), e.g.
    /// `pmi:geometric_tolerance.flatness:f12`; never its Part 21 instance, which is `source`.
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
    /// A surface or common label's text.
    Label { text: String },
    /// A face's ISO 1302 surface texture symbol: its parameters, one per line (`Ra 0.8`, in
    /// µm), and its material removal condition (ISO 10303-1110's term: `any process allowed`,
    /// `material removal required` or `no material removal`).
    Finish {
        lines: Vec<String>,
        removal: String,
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
                subjects: Vec::new(),
                refs: vec!["part".into()],
                ..Default::default()
            });
            return (items, problems);
        }
        Some(Reading::Read(r)) => r,
    };
    // The Spec's datums and controls merged into the reading: what overriding met
    // ([`crate::spec_gdt`]).
    problems.extend(report.spec_problems.iter().cloned());
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
            subjects: Vec::new(),
            refs: vec![s.id.clone()],
            ..Default::default()
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
            subjects: Vec::new(),
            refs: vec!["part".into()],
            ..Default::default()
        });
    }
    let centre = {
        let [lo, hi] = analysis.envelope;
        [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]))
    };
    let mut owned: Option<Manufacturing> = None;
    let ids = report.identities();
    for r in &report.records {
        match r.category {
            Category::GeometricTolerance | Category::Datum => {
                let lowered = if r.category == Category::Datum {
                    datum(r, centre)
                } else {
                    control_frame(r, centre)
                };
                let lowered = identified(lowered, &ids, r);
                match lowered {
                    Ok(mut item) => {
                        alternate_view(&mut item, shown);
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
                        subjects: Vec::new(),
                        refs: vec![r.source_id.clone()],
                        ..Default::default()
                    }),
                }
            }
            // A face's surface texture is its ISO 1302 symbol, attached as a datum's symbol is.
            Category::ManufacturingRequirement if r.kind == "surface_finish" => {
                match identified(surface_finish(r, centre), &ids, r) {
                    Ok(mut item) => {
                        alternate_view(&mut item, shown);
                        items.push(item);
                    }
                    Err(reasons) => problems.push(Problem {
                        id: format!("p:pmi_not_lowered:{}", r.source_id),
                        code: "pmi_not_lowered".into(),
                        severity: "error".into(),
                        message: format!(
                            "PMI surface finish {} ({}) is not drawn: {}.",
                            r.source_id,
                            r.label,
                            reasons.join("; ")
                        ),
                        subjects: Vec::new(),
                        refs: vec![r.source_id.clone()],
                        ..Default::default()
                    }),
                }
            }
            // A surface label, and a common label (read as a dimension), is drawn as its text.
            Category::SurfaceLabel => labelled(&mut items, &mut problems, &ids, r, centre),
            Category::Dimension if r.kind == "common_label" => {
                labelled(&mut items, &mut problems, &ids, r, centre)
            }
            // A dimension is stated or refused by stage 2b (`dims::pmi_dimensions`), never here.
            Category::Dimension => {}
            // A drawing-wide requirement is the sheet's ([`sheet_requirements`]).
            Category::ManufacturingRequirement if sheet_kind(&r.kind) => {}
            // A thread or knurl is printed on the callout of the one feature its cylinder proves,
            // or refused with draftwright's reason ([`manufacturing`]).
            Category::ManufacturingRequirement if owned_kind(&r.kind) => {
                let m = owned.get_or_insert_with(|| manufacturing(analysis));
                problems.extend(
                    m.refused
                        .iter()
                        .filter(|p| p.refs.first() == Some(&r.source_id))
                        .cloned(),
                );
            }
            // Other manufacturing requirements are read, not drawn yet: said with draftwright's
            // code for a record that produced no feature, never dropped. A face's requirement is
            // a warning; a part's note is information.
            _ => {
                let face = !r.faces.is_empty() || !r.edges.is_empty();
                problems.push(Problem {
                    id: format!("p:pmi_not_lowered:{}", r.source_id),
                    code: "pmi_not_lowered".into(),
                    severity: if face { "warning" } else { "info" }.into(),
                    message: format!(
                        "PMI {} {} ({}) is read but not drawn: drawing it is not ported yet.",
                        r.kind.replace('_', " "),
                        r.source_id,
                        r.label
                    ),
                    subjects: Vec::new(),
                    refs: vec![r.source_id.clone()],
                    ..Default::default()
                });
            }
        }
    }
    (items, problems)
}

/// draftwright's `_datum_alternate_view`: a y-normal plane whose side view is not drawn is shown
/// in the plan, below for left and above for right (a datum's symbol, and a face finish's).
fn alternate_view(item: &mut Lowered, shown: &dyn Fn(&str) -> bool) {
    if let Glyph::Datum { surface, .. } | Glyph::Finish { surface, .. } = &item.glyph
        && surface == "plane"
        && item.view == "side"
        && !shown("side")
        && shown("plan")
    {
        item.side = if item.side == "left" {
            "below"
        } else {
            "above"
        };
        item.view = "plan";
        item.basis += "; the side view is not drawn, so the plan carries it";
    }
}

/// *lowered* with its record's item ID, or refused when the record has none: its identity is
/// another record's too (ADR R1 invariant 5), said beside any other reason.
fn identified(
    lowered: Result<Lowered, Vec<String>>,
    ids: &Identities,
    r: &Record,
) -> Result<Lowered, Vec<String>> {
    match (lowered, ids.id(&r.source_id)) {
        (Ok(mut item), Ok(id)) => {
            item.id = id.to_string();
            Ok(item)
        }
        (Ok(_), Err(why)) => Err(vec![why]),
        (Err(mut reasons), Err(why)) => {
            reasons.push(why);
            Err(reasons)
        }
        (Err(reasons), Ok(_)) => Err(reasons),
    }
}

/// A surface or common label lowered into *items*, or its refusal into *problems*.
fn labelled(
    items: &mut Vec<Lowered>,
    problems: &mut Vec<Problem>,
    ids: &Identities,
    r: &Record,
    centre: [f64; 3],
) {
    match identified(label(r, centre), ids, r) {
        Ok(item) => items.push(item),
        Err(reasons) => problems.push(Problem {
            id: format!("p:pmi_not_lowered:{}", r.source_id),
            code: "pmi_not_lowered".into(),
            severity: "error".into(),
            message: format!(
                "PMI {} {} ({}) is not drawn: {}.",
                r.kind.replace('_', " "),
                r.source_id,
                r.label,
                reasons.join("; ")
            ),
            subjects: Vec::new(),
            refs: vec![r.source_id.clone()],
            ..Default::default()
        }),
    }
}

/// A surface or common label as a leadered text label (`build_pmi_features`' label branch), or
/// why it is not one: its reading's or attachment's blockers, no text, or no proven site.
fn label(r: &Record, centre: [f64; 3]) -> Result<Lowered, Vec<String>> {
    let mut reasons = blockers(r);
    if r.label.trim().is_empty() {
        reasons.push("the label has no text".into());
    }
    let g = r.geometry.as_ref();
    let site = g.and_then(|g| g.sites.first().copied());
    if site.is_none() && reasons.is_empty() {
        reasons.push("the label's references give no proven site".into());
    }
    let (Some(site), true) = (site, reasons.is_empty()) else {
        return Err(dedup(reasons));
    };
    let axis = g
        .and_then(|g| g.axis.as_deref())
        .unwrap_or(r.dominant_axis.as_str());
    let view = if axis == "Y" { "side" } else { "front" };
    let side = if site[2] >= centre[2] {
        "above"
    } else {
        "below"
    };
    let what = if r.category == Category::SurfaceLabel {
        "surface label"
    } else {
        "common label"
    };
    Ok(Lowered {
        id: String::new(),
        source: r.source_id.clone(),
        glyph: Glyph::Label {
            text: r.label.clone(),
        },
        view,
        side,
        site,
        alternatives: Vec::new(),
        faces: r.faces.clone(),
        basis: format!(
            "source-authored: the AP242 {what} {} ({}), drawn in the {view} view (its references' longest extent is {}) on the {side} side, as its site lies {side} the part's centre, its leader from its first proven site",
            r.label,
            r.source_id,
            axis.to_lowercase()
        ),
    })
}

/// Where a surface or common label attaches (draftwright's `_surface_label_topology` and
/// `_common_label_topology`): a surface label's edges each at their parameter midpoint, a common
/// label's faces each at the vertex nearest the face's box centre; `axis` is the longest extent
/// of their box (`_dominant_from_bbox`), none when it has none.
pub fn label_geometry(part: &Part, r: &Record) -> Geometry {
    let mut g = Geometry::default();
    g.reasons.extend(crate::pmi::placements_reason(r));
    let surface = r.category == Category::SurfaceLabel;
    let (noun, wrong) = if surface {
        ("surface label", !r.faces.is_empty())
    } else {
        ("common-label", !r.edges.is_empty())
    };
    if wrong {
        g.reasons.push(format!(
            "one {} reference is not {}",
            noun.replace('-', " "),
            if surface { "an edge" } else { "a face" }
        ));
    }
    let mut boxes = Vec::new();
    if surface {
        for &e in &r.edges {
            let Some(edge) = part.edges.get(e) else {
                continue;
            };
            let closed = edge.vertices.0 == edge.vertices.1;
            let (a, b) = haecceity::sampling::edge_interval(
                &edge.curve,
                edge.start,
                edge.end,
                edge.same_sense,
                closed,
            );
            let p = edge.curve.value(0.5 * (a + b));
            if a.is_finite() && b.is_finite() && p.iter().all(|c| c.is_finite()) {
                g.sites.push(p);
            } else {
                g.reasons.push(format!(
                    "{noun} reference could not be measured (edge has no finite parameter interval)"
                ));
            }
            boxes.extend(edge_box(part, e));
        }
    } else {
        for &f in &r.faces {
            let b = face_box(part, f);
            let c = [0, 1, 2].map(|i| 0.5 * (b[i] + b[i + 3]));
            let d = |p: &[f64; 3]| (0..3).map(|i| (p[i] - c[i]).powi(2)).sum::<f64>();
            let face = &part.faces[f];
            let vertices = face.loops.iter().flat_map(|l| {
                l.edges
                    .iter()
                    .flat_map(|&(e, _)| [part.edges[e].start, part.edges[e].end])
                    .chain(l.vertex)
            });
            match vertices.fold(None, |best: Option<[f64; 3]>, p| match best {
                Some(q) if d(&q) <= d(&p) => Some(q),
                _ => Some(p),
            }) {
                Some(p) => g.sites.push(p),
                None => g.reasons.push(format!(
                    "{noun} reference could not be measured (face has no topological vertices)"
                )),
            }
            boxes.push(b);
        }
    }
    if g.sites.is_empty() && g.reasons.is_empty() {
        g.reasons
            .push(format!("{noun} reference geometry is unavailable"));
    }
    if let Some(first) = boxes.first().copied() {
        let b = boxes.iter().fold(first, |mut m, b| {
            for i in 0..3 {
                m[i] = m[i].min(b[i]);
                m[i + 3] = m[i + 3].max(b[i + 3]);
            }
            m
        });
        let spans = [0, 1, 2].map(|i| (b[i + 3] - b[i]).abs());
        let k = (0..3).fold(0, |k, i| if spans[i] > spans[k] { i } else { k });
        if spans[k] > 1e-6 {
            g.axis = Some(XYZ[k].to_string());
        }
    }
    g.reasons = dedup(std::mem::take(&mut g.reasons));
    g
}

/// The drawing-wide manufacturing requirements the sheet carries
/// ([`sheet_requirements`]): general tolerances, a default surface texture, edge conditions.
pub fn sheet_kind(kind: &str) -> bool {
    matches!(
        kind,
        "general_tolerances" | "surface_texture" | "edge_condition"
    )
}

/// A drawing-wide requirement the sheet states.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetRequirement {
    /// The record's item ID, `pmi:<identity>`.
    pub id: String,
    pub source: String,
    /// `general_tolerance` (the title block's tolerance cell), `default_surface_finish` (a
    /// surface texture symbol beside the title block) or `document_note` (a general note).
    pub kind: &'static str,
    /// What the sheet states: the tolerance's designation, the finish's Ra value in µm, the
    /// note's text.
    pub value: String,
    pub basis: String,
    /// A default finish's material removal condition as an AP242 surface texture states it
    /// (`material removal required`, `no material removal`); empty for any process allowed, and
    /// for a finish stated in words.
    pub removal: String,
}

/// The Analysis's drawing-wide requirements (draftwright's `lower_ap242_document_requirements`),
/// and the problems met: the one general tolerance not blocked (its designation: the prose before
/// any `;`, or the structured tolerance class, which must agree with it), the one surface
/// texture stated as `Ra <value> µm unless otherwise specified`, and every edge condition on no
/// face or edge, with text. Two general tolerances, or two surface textures, are ambiguous:
/// neither is drawn. A record the reading blocked, or one whose statement is not a supported
/// default, is said with its reason (`pmi_not_lowered`). Other document kinds draftwright knows
/// (a datum scheme, a model representation note) are reported as not ported.
pub fn sheet_requirements(analysis: &Analysis) -> (Vec<SheetRequirement>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let Some(Reading::Read(report)) = &analysis.pmi else {
        return (out, problems);
    };
    let refuse = |problems: &mut Vec<Problem>, r: &Record, why: &str| {
        problems.push(Problem {
            id: format!("p:pmi_not_lowered:{}", r.source_id),
            code: "pmi_not_lowered".into(),
            severity: "error".into(),
            message: format!(
                "PMI {} {} ({}) is not drawn: {why}.",
                r.kind.replace('_', " "),
                r.source_id,
                r.label
            ),
            subjects: Vec::new(),
            refs: vec![r.source_id.clone()],
            ..Default::default()
        })
    };
    let records: Vec<&Record> = report
        .records
        .iter()
        .filter(|r| r.category == Category::ManufacturingRequirement && sheet_kind(&r.kind))
        .collect();
    let ids = report.identities();
    for r in &records {
        let mut why = dedup(r.blockers.clone());
        why.extend(ids.id(&r.source_id).err());
        if !why.is_empty() {
            refuse(&mut problems, r, &why.join("; "));
        }
    }
    let open: Vec<&Record> = records
        .iter()
        .copied()
        .filter(|r| r.blockers.is_empty() && ids.ids.contains_key(&r.source_id))
        .collect();
    let id = |r: &Record| ids.ids[&r.source_id].clone();
    let of =
        |kind: &str| -> Vec<&Record> { open.iter().copied().filter(|r| r.kind == kind).collect() };
    let tolerances = of("general_tolerances");
    match tolerances[..] {
        [] => {}
        [r] => match designation(r) {
            Ok(d) => out.push(SheetRequirement {
                id: id(r),
                source: r.source_id.clone(),
                kind: "general_tolerance",
                basis: format!(
                    "source-authored: the AP242 general tolerance {} ({}), the part's one, in the title block's tolerance cell (draftwright's GeneralTolerance)",
                    d, r.source_id
                ),
                value: d,
                removal: String::new(),
            }),
            Err(why) => refuse(&mut problems, r, why),
        },
        _ => {
            for r in &tolerances {
                refuse(
                    &mut problems,
                    r,
                    "ambiguous document default: multiple general-tolerance requirements",
                );
            }
        }
    }
    let finishes = of("surface_texture");
    match finishes[..] {
        [] => {}
        [r] => match default_ra(&r.label) {
            Some(ra) => out.push(SheetRequirement {
                id: id(r),
                source: r.source_id.clone(),
                kind: "default_surface_finish",
                basis: format!(
                    "source-authored: the AP242 surface texture '{}' ({}), the part's default finish, drawn as a surface texture symbol beside the title block (draftwright's DefaultSurfaceFinish)",
                    r.label, r.source_id
                ),
                value: ra,
                removal: r
                    .fields
                    .iter()
                    .find_map(|(n, f)| match f {
                        crate::pmi::Field::Text(t)
                            if n == "material removal" && t != "any process allowed" =>
                        {
                            Some(t.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_default(),
            }),
            None => refuse(
                &mut problems,
                r,
                "surface-texture requirement is not a supported document default",
            ),
        },
        _ => {
            for r in &finishes {
                refuse(
                    &mut problems,
                    r,
                    "ambiguous document default: multiple surface-texture requirements",
                );
            }
        }
    }
    for r in of("edge_condition") {
        if r.label.trim().is_empty() {
            refuse(&mut problems, r, "document requirement text is empty");
        } else if !r.faces.is_empty() || !r.edges.is_empty() {
            refuse(
                &mut problems,
                r,
                "face-specific edge condition cannot be a document-wide note",
            );
        } else {
            out.push(SheetRequirement {
                id: id(r),
                source: r.source_id.clone(),
                kind: "document_note",
                value: r.label.trim().to_string(),
                basis: format!(
                    "source-authored: the AP242 edge condition '{}' ({}), a general note (draftwright's DocumentNote)",
                    r.label.trim(),
                    r.source_id
                ),
                removal: String::new(),
            });
        }
    }
    (out, problems)
}

/// A drawing-wide requirement as the plan carries it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedRequirement {
    /// The record's item ID, `pmi:<identity>`; `source` is its Part 21 provenance.
    pub id: String,
    /// `general_tolerance`, `default_surface_finish` or `document_note`.
    pub kind: String,
    pub source: String,
    /// What the sheet states: the designation, `Ra <value>`, the note's text.
    pub text: String,
    /// What carries it: `title_block` (its tolerance cell), `sheet` (the symbol beside the title
    /// block), or `notes` (the general note of the same ID).
    pub carrier: String,
    pub basis: String,
    /// Shown, or not drawable as its carrier is not (a general note with no place).
    pub state: crate::items::State,
    /// The default finish's symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<crate::title_block::FinishPlaced>,
}

/// The drawing-wide *requirements* given their carriers on a sheet whose title block is *block*
/// and whose title the spec gives as *given* (the title to draw, *title*, takes the general
/// tolerance), and the problems met. draftwright's precedence (`_resolve_title_document_defaults`):
/// a tolerance the spec states, even blank, wins over the PMI's, which is then not drawn and,
/// when they differ, reported (`step_general_tolerance_disagreement`, a warning, as
/// draftwright's PMI lint reports it).
pub fn plan_sheet(
    requirements: &[SheetRequirement],
    given: &std::collections::BTreeMap<String, String>,
    title: &mut std::collections::BTreeMap<String, String>,
    block: [f64; 4],
) -> (Vec<PlannedRequirement>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for r in requirements {
        let id = r.id.clone();
        let mut planned = PlannedRequirement {
            id: id.clone(),
            kind: r.kind.into(),
            source: r.source.clone(),
            text: r.value.clone(),
            carrier: "notes".into(),
            basis: r.basis.clone(),
            // A general note's is its note's, set when the notes are placed.
            state: crate::items::State::Shown,
            placed: None,
        };
        match r.kind {
            "general_tolerance" => {
                planned.carrier = "title_block".into();
                match given.get("tolerance") {
                    None => {
                        title.insert("tolerance".into(), r.value.clone());
                    }
                    Some(v) if v.trim() == r.value => {
                        planned.basis +=
                            "; the spec states the same tolerance (sheet/title/tolerance)";
                    }
                    Some(v) => {
                        problems.push(Problem {
                            id: format!("p:step_general_tolerance_disagreement:{}", r.source),
                            code: "step_general_tolerance_disagreement".into(),
                            severity: "warning".into(),
                            message: format!(
                                "The title block states the spec's general tolerance {v:?} (sheet/title/tolerance), which wins; the part's AP242 PMI {} states {:?}, which is not drawn.",
                                r.source, r.value
                            ),
                            subjects: vec!["sheet".into()],
                            refs: vec![r.source.clone(), "sheet/title/tolerance".into()],
                            ..Default::default()
                        });
                        continue;
                    }
                }
            }
            "default_surface_finish" => {
                planned.carrier = "sheet".into();
                planned.text = format!("Ra {}", r.value);
                planned.placed = Some(crate::title_block::default_finish(
                    block,
                    &planned.text,
                    &r.removal,
                ));
            }
            _ => {}
        }
        out.push(planned);
    }
    (out, problems)
}

/// A general tolerance's designation (`_general_tolerance_designation`): its prose before any
/// `;`, or its structured tolerance class, which must agree with the prose unless the prose is
/// only `general tolerances`.
fn designation(r: &Record) -> Result<String, &'static str> {
    let prose = r.label.split(';').next().unwrap_or("").trim().to_string();
    let class = r.fields.iter().find(|(n, _)| n == "tolerance class");
    let d = match (r.fields.is_empty(), class) {
        (true, _) => prose,
        (false, Some((_, crate::pmi::Field::Text(t)))) if !t.trim().is_empty() => {
            if r.label != "general tolerances" && prose != *t {
                return Err("structured general-tolerance class disagrees with prose");
            }
            t.clone()
        }
        (false, _) => return Err("structured general-tolerance class is missing"),
    };
    if d.is_empty() {
        return Err("general-tolerance designation is empty");
    }
    Ok(d)
}

/// The Ra value of a default surface texture stated as draftwright reads one
/// (`Ra <value> µm unless otherwise specified`, case and spacing free, `um`, `µm` or `μm`).
fn default_ra(label: &str) -> Option<String> {
    let lower = label.trim().to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let (ra, rest) = match words.as_slice() {
        ["ra", value, rest @ ..] => (*value, rest.to_vec()),
        _ => return None,
    };
    // The unit may be written against the value (`3.2µm`).
    let (value, rest) = match ["um", "µm", "μm"]
        .iter()
        .find_map(|u| ra.strip_suffix(u).filter(|v| !v.is_empty()))
    {
        Some(v) => (v, rest),
        None => match rest.split_first() {
            Some((u, tail)) if ["um", "µm", "μm"].contains(u) => (ra, tail.to_vec()),
            _ => return None,
        },
    };
    let number = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let ok = match value.split_once('.') {
        Some((i, f)) => number(i) && number(f),
        None => number(value),
    };
    (ok && rest == ["unless", "otherwise", "specified"]).then(|| value.to_string())
}

/// The principal views the Analysis's PMI items are drawn in, as lowering targets them with
/// every view shown: a turned part keeps a view one of them needs (draftwright's
/// `annotation_view_required`, which vetoes the reduction for any GD&T, datum or label aspect
/// targeting a view the reduced set lacks).
pub fn views(analysis: &Analysis) -> BTreeSet<&'static str> {
    lower(analysis, &|_| true)
        .0
        .iter()
        .map(|i| i.view)
        .collect()
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
        id: String::new(),
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
            "{}: the {} {} tolerance {}, drawn where {} ({}) is seen end-on, in the {view} view's {side} strip (draftwright's face-on rule), its leader from the centre of its faces",
            authored_by(r).0,
            authored_by(r).1,
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
    if r.label.trim().is_empty() {
        reasons.push("the datum has no letter".into());
    }
    let at = edge_on(r, centre, "datum", reasons)?;
    Ok(Lowered {
        id: String::new(),
        source: r.source_id.clone(),
        glyph: Glyph::Datum {
            letter: r.label.trim().to_string(),
            surface: at.surface.clone(),
        },
        view: at.view,
        side: at.side,
        site: at.site,
        alternatives: at.alternatives,
        faces: r.faces.clone(),
        basis: format!(
            "{}: the {} datum {} ({}) on a {} along {}, drawn edge-on in the {} view on the {} side {}",
            authored_by(r).0,
            authored_by(r).1,
            r.label.trim(),
            r.source_id,
            at.surface,
            at.axis.to_lowercase(),
            at.view,
            at.side,
            at.how
        ),
    })
}

/// Who authored record *r*, for its basis: the STEP's AP242 PMI, or the Spec
/// ([`crate::spec_gdt`], whose records' sources are `spec:<path>`).
fn authored_by(r: &Record) -> (&'static str, &'static str) {
    if r.source_id.starts_with(crate::spec_gdt::SOURCE_PREFIX) {
        ("spec-authored", "spec's")
    } else {
        ("source-authored", "AP242")
    }
}

/// Where a datum feature symbol, or a face's surface finish symbol, attaches.
struct EdgeOn {
    view: &'static str,
    side: &'static str,
    site: [f64; 3],
    alternatives: Vec<[f64; 3]>,
    surface: String,
    axis: String,
    how: &'static str,
}

/// The view that sees record *r*'s plane or cylinder edge-on (x and z: the front; y: the side),
/// its first proven site and the strip its outward normal points to (a plane's) or its site lies
/// on (a cylinder's), or why there is none (*reasons* already met, then the attachment's; *noun*
/// names the record: `datum` or `surface finish`).
fn edge_on(
    r: &Record,
    centre: [f64; 3],
    noun: &str,
    reasons: Vec<String>,
) -> Result<EdgeOn, Vec<String>> {
    let mut reasons = reasons;
    let g = r.geometry.as_ref();
    let axis = g.and_then(|g| g.axis.as_deref());
    let surface = g.and_then(|g| g.surface.clone());
    let site = g.and_then(|g| g.sites.first().copied());
    let (Some(axis), Some(surface), Some(site)) = (axis, surface, site) else {
        if reasons.is_empty() {
            reasons.push(format!("{noun} reference surface is unavailable"));
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
            return Err(vec![format!(
                "{noun} surface normal cannot be shown by an axial leader in this view"
            )]);
        }
        Some(n) => {
            if n[across] > 0.0 {
                high
            } else {
                low
            }
        }
        None if planar => {
            return Err(vec![format!(
                "{noun} reference surface has no outward normal"
            )]);
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
    Ok(EdgeOn {
        view,
        side,
        site,
        alternatives: g.map(|g| g.alternatives.clone()).unwrap_or_default(),
        surface,
        axis: axis.to_string(),
        how,
    })
}

/// A face's AP242 surface texture (`surface_finish`) as its ISO 1302 symbol, attached as a
/// datum feature symbol is ([`edge_on`]), or why it is not: its reading's or attachment's
/// blockers, or no parameter to state.
fn surface_finish(r: &Record, centre: [f64; 3]) -> Result<Lowered, Vec<String>> {
    let mut reasons = blockers(r);
    let mut removal = String::new();
    let mut lines = Vec::new();
    for (name, value) in &r.fields {
        match value {
            crate::pmi::Field::Text(t) if name == "material removal" => removal = t.clone(),
            crate::pmi::Field::Length(mm) => {
                lines.push(format!("{name} {}", crate::pmi::fmt_g3(mm * 1000.0)));
            }
            crate::pmi::Field::Text(_) => {}
        }
    }
    if lines.is_empty() {
        reasons.push("the surface texture states no parameter".into());
    }
    let at = edge_on(r, centre, "surface finish", reasons)?;
    Ok(Lowered {
        id: String::new(),
        source: r.source_id.clone(),
        glyph: Glyph::Finish {
            lines,
            removal,
            surface: at.surface.clone(),
        },
        view: at.view,
        side: at.side,
        site: at.site,
        alternatives: at.alternatives,
        faces: r.faces.clone(),
        basis: format!(
            "source-authored: the AP242 surface texture {} ({}) on a {} along {}, its ISO 1302 symbol on a leader from the face's edge in the {} view on the {} side {}",
            r.label,
            r.source_id,
            at.surface,
            at.axis.to_lowercase(),
            at.view,
            at.side,
            at.how
        ),
    })
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    v.retain(|s| seen.insert(s.clone()));
    v
}

/// One exact finite cylinder a diameter or radius refers to (draftwright's
/// `CylindricalReference.canonical`): its axis as the foot of the perpendicular from the origin
/// and a unit direction whose largest component is positive, its radius, its extent along that
/// direction from the foot, and whether the material is outside it (`internal`: a bore).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cylinder {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub radius: f64,
    pub interval: [f64; 2],
    pub internal: bool,
}

impl Cylinder {
    /// The point of its axis halfway along it.
    pub fn midpoint(&self) -> [f64; 3] {
        let t = 0.5 * (self.interval[0] + self.interval[1]);
        [0, 1, 2].map(|i| self.origin[i] + t * self.direction[i])
    }

    /// The world axis (0, 1, 2) it lies along, when it is principal (draftwright's
    /// `principal_axis`, to 1e-6).
    pub fn principal(&self) -> Option<usize> {
        let d = self.direction;
        let k = (0..3).fold(0, |k, i| if d[i].abs() > d[k].abs() { i } else { k });
        ((d[k] - 1.0).abs() <= 1e-6 && (0..3).all(|i| i == k || d[i].abs() <= 1e-6)).then_some(k)
    }
}

const XYZ: [&str; 3] = ["X", "Y", "Z"];

/// Whether *p* lies on face *f*, its boundary included (to 1e-6 mm), as `face_sites` tests it.
fn on_face(part: &Part, f: usize, p: [f64; 3]) -> bool {
    let surface = &part.faces[f].surface;
    let (Some(domain), Some((u, v))) = (part.domain(f), surface.parameters(p, None)) else {
        return false;
    };
    let q = surface.value(u, v);
    let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
    let e = 1e-9;
    d <= 1e-6
        && [(0.0, 0.0), (e, 0.0), (-e, 0.0), (0.0, e), (0.0, -e)]
            .iter()
            .any(|(du, dv)| domain.contains(u + du, v + dv))
}

/// A face's box as `[xmin, ymin, zmin, xmax, ymax, zmax]`.
fn face_box(part: &Part, f: usize) -> [f64; 6] {
    let b = part.face_bounds(f);
    [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]]
}

/// An edge's box, from the points that stand in for its curve.
fn edge_box(part: &Part, e: usize) -> Option<[f64; 6]> {
    let edge = part.edges.get(e)?;
    let mut b: Option<[f64; 6]> = None;
    for p in edge.samples.iter().chain([&edge.start, &edge.end]) {
        let o = b.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
        for i in 0..3 {
            o[i] = o[i].min(p[i]);
            o[i + 3] = o[i + 3].max(p[i]);
        }
    }
    b
}

/// The axis (0, 1, 2) a unit direction lies along exactly (to 1e-6), as draftwright's planar
/// tests read a plane's normal.
fn exact_axis(d: [f64; 3]) -> Option<usize> {
    let k = (0..3).fold(0, |k, i| if d[i].abs() > d[k].abs() { i } else { k });
    ((d[k].abs() - 1.0).abs() <= 1e-6 && (0..3).all(|i| i == k || d[i].abs() <= 1e-6)).then_some(k)
}

/// The face's cylinder, canonical, or why it is none.
fn cylinder(part: &Part, f: usize) -> Result<Cylinder, &'static str> {
    let face = &part.faces[f];
    let Surface::Cylinder { frame, radius } = &face.surface else {
        return Err("one diameter reference face is not cylindrical");
    };
    let (u0, u1, v0, v1) = part
        .uv_bounds(f)
        .ok_or("one cylindrical reference could not be measured (no parameter range)")?;
    let raw = frame.z;
    let n = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
    if n <= 1e-12 || v1 <= v0 || v1.is_nan() || v0.is_nan() {
        return Err("one cylindrical reference could not be measured (degenerate axis or span)");
    }
    let unit = raw.map(|c| c / n);
    let k = (0..3).fold(0, |k, i| if unit[i].abs() > unit[k].abs() { i } else { k });
    let sign = if unit[k] < 0.0 { -1.0 } else { 1.0 };
    let direction = unit.map(|c| {
        let c = sign * c;
        if c.abs() < 1e-12 { 0.0 } else { c }
    });
    let p = frame.origin;
    let along = p[0] * direction[0] + p[1] * direction[1] + p[2] * direction[2];
    let origin = [0, 1, 2].map(|i| {
        let c = p[i] - along * direction[i];
        if c.abs() < 1e-12 { 0.0 } else { c }
    });
    let stations = [along + sign * v0, along + sign * v1];
    // The material side: the face's normal at its parameter centre against the radial direction
    // (draftwright reads OpenCascade's face orientation, the same fact for a cylinder whose
    // surface normal points away from its axis).
    let (u, v) = (0.5 * (u0 + u1), 0.5 * (v0 + v1));
    let q = face.surface.value(u, v);
    let foot = [0, 1, 2].map(|i| p[i] + v * unit[i]);
    let radial = [0, 1, 2].map(|i| q[i] - foot[i]);
    let normal = part
        .face_normal(f, u, v)
        .ok_or("one cylindrical reference could not be measured (no normal)")?;
    let out = normal[0] * radial[0] + normal[1] * radial[1] + normal[2] * radial[2];
    Ok(Cylinder {
        origin,
        direction,
        radius: *radius,
        interval: [stations[0].min(stations[1]), stations[0].max(stations[1])],
        internal: out < 0.0,
    })
}

/// Where a dimension attaches, from its faces (draftwright's `_proved_planar_linear` with
/// `_linear_reference_stations` for a linear or thickness dimension, `_cylindrical_references`
/// with `_diameter_reference_blockers` for a diameter): what [`crate::dims`] draws it from, and
/// the reasons it cannot be drawn when its faces prove no witness. A radius is read as a
/// diameter's cylinders are, with a proven site on its arc; an angle by its two planar sides
/// ([`crate::dims::angular::geometry`]); the curved kinds keep no geometry (not drawn: see
/// `pmi_dimensions`).
pub fn dimension_geometry(part: &Part, r: &Record) -> Geometry {
    let mut g = Geometry::default();
    g.reasons.extend(crate::pmi::placements_reason(r));
    let value = r.value.unwrap_or(0.0);
    match r.kind.as_str() {
        "linear" | "thickness" => linear_geometry(part, r, value, &mut g),
        "diameter" | "radius" => cylinder_geometry(part, r, value, &mut g),
        "angular" => crate::dims::angular::geometry(part, r, &mut g),
        "common_label" => return label_geometry(part, r),
        _ => {}
    }
    g.reasons = dedup(std::mem::take(&mut g.reasons));
    g
}

fn linear_geometry(part: &Part, r: &Record, value: f64, g: &mut Geometry) {
    let word = |s: &str| {
        if r.kind == "thickness" {
            s.replace("linear dimension", "thickness dimension")
                .replace("linear reference", "thickness reference")
        } else {
            s.to_string()
        }
    };
    let groups = &r.face_groups;
    let edges = |i: usize| r.edge_groups.get(i).map_or(&[][..], Vec::as_slice);
    if groups.len() != 2 || (0..2).any(|i| groups[i].is_empty() && edges(i).is_empty()) {
        g.reasons.push(word(
            "linear dimension needs two measurable authored reference groups",
        ));
        return;
    }
    // Each group's items' boxes, its faces' then its edges' (an edge's from its samples).
    let all_boxes: Vec<Vec<[f64; 6]>> = (0..2)
        .map(|i| {
            let mut bs: Vec<[f64; 6]> = groups[i].iter().map(|&f| face_box(part, f)).collect();
            bs.extend(edges(i).iter().filter_map(|&e| edge_box(part, e)));
            bs
        })
        .collect();
    let faces_only = (0..2).all(|i| edges(i).is_empty() && !groups[i].is_empty());
    let boxes = &all_boxes;
    let centre = |bs: &[[f64; 6]]| {
        let mut m = bs[0];
        for b in &bs[1..] {
            for i in 0..3 {
                m[i] = m[i].min(b[i]);
                m[i + 3] = m[i + 3].max(b[i + 3]);
            }
        }
        [0, 1, 2].map(|i| 0.5 * (m[i] + m[i + 3]))
    };
    if boxes.iter().any(Vec::is_empty) {
        g.reasons.push(word(
            "linear dimension needs two measurable authored reference groups",
        ));
        return;
    }
    let mut stations = [centre(&boxes[0]), centre(&boxes[1])];
    let mut plane_axis = None;
    let proof = if faces_only {
        planar(part, groups, boxes)
    } else {
        Planar::Not
    };
    match proof {
        Planar::Not => {
            if r.kind == "linear"
                && faces_only
                && let Some(bore) = plane_to_bore(part, groups, boxes)
            {
                stations = bore.0;
                plane_axis = Some(bore.1);
                g.view = Some(bore.2.into());
            } else if r.kind == "linear" && faces_only && mixed(part, groups) {
                g.reasons.extend(
                    stations_proof(stations, value, None)
                        .2
                        .into_iter()
                        .map(|s| word(&s)),
                );
                g.reasons.push(
                    "plane-to-bore location has no proven face-supported axis witness".into(),
                );
                g.stations = stations.to_vec();
                return;
            }
        }
        Planar::Unwitnessed => {
            g.stations = stations.to_vec();
            g.reasons
                .push("parallel planar source faces have no proven shared witness".into());
            return;
        }
        Planar::Axis(k, view, witness) => {
            plane_axis = k;
            g.view = view.map(String::from);
            for (i, c) in witness {
                for s in &mut stations {
                    s[i] = c;
                }
            }
        }
    }
    let (points, axis, reasons) = stations_proof(stations, value, plane_axis);
    g.stations = points.to_vec();
    g.axis = axis.map(|k| XYZ[k].to_string());
    g.reasons.extend(reasons.into_iter().map(|s| word(&s)));
}

/// What two groups of faces prove as parallel planes (draftwright's
/// `_parallel_planar_reference_witness`).
enum Planar {
    /// Not every face is a principal plane, thin along its normal, one station per group, one
    /// normal for both.
    Not,
    /// Parallel planes along x with no witness both share.
    Unwitnessed,
    /// Parallel planes: their normal, when a view's witness proves it; the one view that does,
    /// when only one does; the witness coordinates the stations take.
    Axis(Option<usize>, Option<&'static str>, Vec<(usize, f64)>),
}

fn planar(part: &Part, groups: &[Vec<usize>], boxes: &[Vec<[f64; 6]>]) -> Planar {
    let mut axes = Vec::new();
    for (faces, bs) in groups.iter().zip(boxes) {
        let mut axis: Option<(usize, f64)> = None;
        for (&f, b) in faces.iter().zip(bs) {
            let Surface::Plane { frame } = &part.faces[f].surface else {
                return Planar::Not;
            };
            let Some(k) = exact_axis(frame.z) else {
                return Planar::Not;
            };
            if b[k + 3] - b[k] > 1e-5 {
                return Planar::Not;
            }
            let station = 0.5 * (b[k] + b[k + 3]);
            match axis {
                Some((a, s)) if a != k || (s - station).abs() > 1e-5 => return Planar::Not,
                _ => axis = Some((k, station)),
            }
        }
        axes.push(axis.expect("a group has a face").0);
    }
    if axes[0] != axes[1] {
        return Planar::Not;
    }
    let k = axes[0];
    let midpoint = |i: usize| {
        let all: Vec<&[f64; 6]> = boxes.iter().flatten().collect();
        let lo = all.iter().map(|b| b[i]).fold(f64::INFINITY, f64::min);
        let hi = all
            .iter()
            .map(|b| b[i + 3])
            .fold(f64::NEG_INFINITY, f64::max);
        let m = 0.5 * (lo + hi);
        boxes
            .iter()
            .all(|bs| bs.iter().any(|b| b[i] - 1e-6 <= m && m <= b[i + 3] + 1e-6))
    };
    match k {
        0 if midpoint(2) => Planar::Axis(Some(0), None, Vec::new()),
        0 => match common_x_witness(part, groups, boxes) {
            Some((y, z)) => Planar::Axis(Some(0), Some("plan"), vec![(1, y), (2, z)]),
            None => Planar::Unwitnessed,
        },
        2 => Planar::Axis(midpoint(0).then_some(2), None, Vec::new()),
        _ => match (midpoint(2), midpoint(0)) {
            (true, true) => Planar::Axis(Some(1), None, Vec::new()),
            (true, false) => Planar::Axis(Some(1), Some("side"), Vec::new()),
            (false, true) => Planar::Axis(Some(1), Some("plan"), Vec::new()),
            (false, false) => Planar::Axis(None, None, Vec::new()),
        },
    }
}

/// A point inside both trimmed x faces (draftwright's `_common_x_face_witness`): the largest
/// overlap of their boxes first, its centre then a 9 × 9 grid over it.
fn common_x_witness(
    part: &Part,
    groups: &[Vec<usize>],
    boxes: &[Vec<[f64; 6]>],
) -> Option<(f64, f64)> {
    let mut candidates = Vec::new();
    for (i, a) in boxes[0].iter().enumerate() {
        for (j, b) in boxes[1].iter().enumerate() {
            let (y0, z0, y1, z1) = (
                a[1].max(b[1]),
                a[2].max(b[2]),
                a[4].min(b[4]),
                a[5].min(b[5]),
            );
            if y1 >= y0 && z1 >= z0 {
                candidates.push(((y1 - y0) * (z1 - z0), y0, z0, y1, z1, i, j));
            }
        }
    }
    candidates.sort_by(|p, q| {
        q.0.total_cmp(&p.0)
            .then(p.1.total_cmp(&q.1))
            .then(p.2.total_cmp(&q.2))
    });
    let fractions = std::iter::once((0.5, 0.5)).chain(
        (1..10).flat_map(|i| (1..10).map(move |j| (f64::from(i) / 10.0, f64::from(j) / 10.0))),
    );
    let fractions: Vec<(f64, f64)> = fractions.collect();
    for (_, y0, z0, y1, z1, i, j) in candidates {
        for &(fy, fz) in &fractions {
            let (y, z) = (y0 + (y1 - y0) * fy, z0 + (z1 - z0) * fz);
            let inside = [(groups[0][i], boxes[0][i]), (groups[1][j], boxes[1][j])]
                .iter()
                .all(|(f, b)| on_face(part, *f, [0.5 * (b[0] + b[3]), y, z]));
            if inside {
                return Some((y, z));
            }
        }
    }
    None
}

/// Whether the two groups are one plane and one cylinder (draftwright's `_is_mixed_planar_bore`).
fn mixed(part: &Part, groups: &[Vec<usize>]) -> bool {
    if groups.iter().any(|g| g.len() != 1) {
        return false;
    }
    let kinds: Vec<u8> = groups
        .iter()
        .map(|g| match part.faces[g[0]].surface {
            Surface::Plane { .. } => 1,
            Surface::Cylinder { .. } => 2,
            _ => 0,
        })
        .collect();
    kinds == [1, 2] || kinds == [2, 1]
}

/// A location from a principal plane to a principal bore's axis across it (draftwright's
/// `_planar_bore_axis_witness`): the stations, at the plane and at the axis, on one point of the
/// plane's face beside the axis halfway along their common extent down the bore; the plane's
/// normal; and the view that sees the bore end on. draftwright proves only an x or y plane to a
/// z bore, seen in plan; any other pair is proved the same way here (#34: a turned part's cross
/// hole located from its end face, seen in the side or front view), and the z bore keeps
/// draftwright's witness and view.
fn plane_to_bore(
    part: &Part,
    groups: &[Vec<usize>],
    boxes: &[Vec<[f64; 6]>],
) -> Option<([[f64; 3]; 2], usize, &'static str)> {
    if !mixed(part, groups) {
        return None;
    }
    let plane_i = usize::from(!matches!(
        part.faces[groups[0][0]].surface,
        Surface::Plane { .. }
    ));
    let bore_i = 1 - plane_i;
    let Surface::Plane { frame: plane } = &part.faces[groups[plane_i][0]].surface else {
        return None;
    };
    let Surface::Cylinder { frame: bore, .. } = &part.faces[groups[bore_i][0]].surface else {
        return None;
    };
    let axis = exact_axis(plane.z)?;
    // The bore's axis, across the plane's normal.
    let along = exact_axis(bore.z).filter(|&j| j != axis)?;
    let (pb, bb) = (boxes[plane_i][0], boxes[bore_i][0]);
    let (z0, z1) = (pb[along].max(bb[along]), pb[along + 3].min(bb[along + 3]));
    if z1 - z0 <= 1e-6 || pb[axis + 3] - pb[axis] > 1e-5 {
        return None;
    }
    let centre = bore.origin;
    let mut witness = centre;
    witness[axis] = 0.5 * (pb[axis] + pb[axis + 3]);
    witness[along] = 0.5 * (z0 + z1);
    if !on_face(part, groups[plane_i][0], witness) {
        return None;
    }
    let mut points = [witness, witness];
    points[bore_i][axis] = centre[axis];
    Some((points, axis, ["side", "front", "plan"][along]))
}

/// draftwright's `_linear_reference_stations`: the axis two stations are measured along (their
/// largest difference, or the planes' normal), refused when they coincide, when they are
/// oblique outside a principal view's plane (an oblique one inside it keeps no axis), or when
/// the span they prove is not the nominal value.
fn stations_proof(
    s: [[f64; 3]; 2],
    nominal: f64,
    plane_axis: Option<usize>,
) -> ([[f64; 3]; 2], Option<usize>, Vec<String>) {
    let delta = [0, 1, 2].map(|i| s[1][i] - s[0][i]);
    let m = delta.map(f64::abs);
    let k = plane_axis.unwrap_or_else(|| (0..3).fold(0, |k, i| if m[i] > m[k] { i } else { k }));
    let primary = m[k];
    if primary <= 1e-9 {
        return (
            s,
            None,
            vec!["linear reference groups occupy the same station".into()],
        );
    }
    let transverse = (0..3).filter(|&i| i != k).map(|i| m[i]).fold(0.0, f64::max);
    let oblique = plane_axis.is_none() && transverse > 0.005f64.max(primary * 1e-3);
    if oblique && projection_view(delta).is_none() {
        return (
            s,
            None,
            vec![format!(
                "linear reference relationship does not lie in a principal projection plane (delta=({}, {}, {}) mm)",
                g6(delta[0]),
                g6(delta[1]),
                g6(delta[2])
            )],
        );
    }
    let span = if oblique {
        (delta[0].powi(2) + delta[1].powi(2) + delta[2].powi(2)).sqrt()
    } else {
        primary
    };
    let tolerance = (if oblique { 0.05 } else { 0.01f64 }).max(nominal.abs() * 5e-4);
    let axis = (!oblique).then_some(k);
    if (span - nominal).abs() > tolerance {
        return (
            s,
            axis,
            vec![format!(
                "linear reference-station span {} mm differs from nominal {} mm",
                g6(span),
                g6(nominal)
            )],
        );
    }
    (s, axis, Vec::new())
}

/// The principal view whose plane holds a span (draftwright's `_linear_projection_view`): the
/// one axis its difference has none of.
fn projection_view(delta: [f64; 3]) -> Option<&'static str> {
    let primary = delta.iter().map(|d| d.abs()).fold(0.0, f64::max);
    if (delta[0].powi(2) + delta[1].powi(2) + delta[2].powi(2)).sqrt() <= 1e-9 {
        return None;
    }
    let tolerance = 0.005f64.max(primary * 1e-3);
    let zeros: Vec<usize> = (0..3).filter(|&i| delta[i].abs() <= tolerance).collect();
    match zeros[..] {
        [i] => Some(["side", "front", "plan"][i]),
        _ => None,
    }
}

fn cylinder_geometry(part: &Part, r: &Record, value: f64, g: &mut Geometry) {
    let radius = r.kind == "radius";
    let noun = if radius { "radius" } else { "diameter" };
    if !r.edges.is_empty() {
        g.reasons
            .push(format!("one {noun} reference is not a face"));
    }
    for &f in &r.faces {
        match cylinder(part, f) {
            Ok(c) => {
                if !g.cylinders.iter().any(|d| same_cylinder(d, &c)) {
                    g.cylinders.push(c);
                }
            }
            Err(why) => g.reasons.push(why.replace("diameter", noun)),
        }
    }
    if g.cylinders.is_empty() {
        if g.reasons.is_empty() {
            g.reasons.push(format!(
                "{noun} dimension needs a measurable cylindrical-face reference"
            ));
        }
        return;
    }
    let first = g.cylinders[0].direction;
    let directions_agree = g
        .cylinders
        .iter()
        .all(|c| (0..3).all(|i| (c.direction[i] - first[i]).abs() <= 1e-9));
    if !directions_agree {
        g.reasons.push(format!(
            "{noun} references do not share one cylinder axis direction"
        ));
    } else if first.iter().all(|c| c.abs() > 1e-6) {
        g.reasons.push(format!(
            "{noun} cylinder axis does not lie in a principal projection plane"
        ));
    }
    if g.cylinders
        .iter()
        .any(|c| c.internal != g.cylinders[0].internal)
    {
        g.reasons.push(format!(
            "{noun} references mix internal and external cylindrical faces"
        ));
    }
    let size = |c: &Cylinder| if radius { c.radius } else { 2.0 * c.radius };
    let off: Vec<String> = g
        .cylinders
        .iter()
        .filter(|c| (size(c) - value).abs() > 0.01f64.max(value.abs() * 5e-4))
        .map(|c| g6(size(c)))
        .collect();
    if !off.is_empty() {
        g.reasons.push(format!(
            "cylindrical reference {noun}(s) {} mm differ from nominal {} mm",
            off.join(", "),
            g6(value)
        ));
    }
    let axes: BTreeSet<Option<usize>> = g.cylinders.iter().map(Cylinder::principal).collect();
    if axes.len() == 1
        && let Some(Some(k)) = axes.first()
    {
        g.axis = Some(XYZ[*k].to_string());
    }
    if radius {
        // A radius is drawn to its arc: a proven point of its first face, at its parameter
        // centre.
        let f = r.faces[0];
        if let Some((u0, u1, v0, v1)) = part.uv_bounds(f) {
            let p = part.faces[f]
                .surface
                .value(0.5 * (u0 + u1), 0.5 * (v0 + v1));
            if on_face(part, f, p) {
                g.sites.push(p);
            }
        }
        if g.sites.is_empty() {
            g.reasons
                .push("the radius's face has no proven point at its parameter centre".into());
        }
    }
}

/// Geometrically identical cylinders (one face XCAF repeats in a group): draftwright coalesces
/// them.
fn same_cylinder(a: &Cylinder, b: &Cylinder) -> bool {
    let near = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(p, q)| (p - q).abs() <= 1e-9);
    near(&a.origin, &b.origin)
        && near(&a.direction, &b.direction)
        && (a.radius - b.radius).abs() <= 1e-9
        && near(&a.interval, &b.interval)
        && a.internal == b.internal
}

/// `%.6g`, as draftwright writes a measurement in a reason (`f"{value:.6g}"`).
fn g6(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let sci = format!("{v:.5e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent");
    if !(-4..6).contains(&exp) {
        let m = mantissa.trim_end_matches('0').trim_end_matches('.');
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{m}e{sign}{:02}", exp.abs());
    }
    let places = usize::try_from(5 - exp).unwrap_or(0);
    let s = format!("{v:.places$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        s
    }
}

// ---- Thread and knurl requirements on their owners' callouts (stage 2c) ----

/// The manufacturing requirement kinds draftwright gives to the feature their cylinder proves
/// (`lower_ap242_manufacturing_requirements`).
const OWNED_KINDS: [&str; 3] = ["internal_thread", "external_thread", "knurl"];

/// Whether a manufacturing requirement's kind is a thread or knurl ([`manufacturing`]'s).
pub fn owned_kind(kind: &str) -> bool {
    OWNED_KINDS.contains(&kind)
}

/// A suffix longer than this, with two or more requirements imported, moves them all into the
/// manufacturing requirements table (draftwright's `manufacturing_schedule` threshold).
const SCHEDULE_SUFFIX: usize = 52;

/// The width a table row's requirement text wraps at (`_wrap_requirement`).
const SCHEDULE_WRAP: usize = 44;

/// The manufacturing requirements table's ID on the plan (draftwright's annotation name).
pub const SCHEDULE_ID: &str = "manufacturing_requirements";

/// A thread or knurl requirement given to the one feature its cylinder proves.
#[derive(Clone, Debug, PartialEq)]
pub struct Owned {
    /// The record's item ID, `pmi:<identity>`: what its owner's callout claims.
    pub id: String,
    /// The record's `source_id`.
    pub source: String,
    /// The feature that carries it (`f:hole:2`, `f:turned_step:4`).
    pub owner: String,
    /// `thread` or `knurl`.
    pub aspect: &'static str,
    /// The terms after the owner's size (`callout_suffix`): `M2 x 0.4-6H RH; 2.5 MIN FULL
    /// THREAD`.
    pub suffix: String,
    /// The whole requirement, as a table row states it (`callout_text`): a knurl's maximum
    /// diameter leads (`ø6 MAX AFTER KNURL; …`).
    pub text: String,
    /// A through tap: its suffix states `THRU`, so the hole callout's own `THRU` is not
    /// repeated.
    pub through: bool,
    /// How the owner's profile views draw it ([`crate::cosmetic`]): an external thread's
    /// minor diameter or a knurl's band; none for an internal thread.
    pub depiction: Option<Depiction>,
}

/// Where and what a turned step's thread or knurl is drawn, in the drawing's frame (as
/// [`crate::lint::turned::band`] reads the step).
#[derive(Clone, Debug, PartialEq)]
pub struct Depiction {
    /// The world axis the step turns about.
    pub axis: usize,
    /// A point on that axis.
    pub on: [f64; 3],
    /// The threaded or knurled length along the axis.
    pub span: [f64; 2],
    /// The major (thread) or maximum (knurl) diameter.
    pub diameter: f64,
    pub kind: DepictionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DepictionKind {
    /// ISO 6410: a thin line at the minor diameter over the thread length.
    Thread { minor: f64 },
    /// A band of thin lines over part of the knurled length: straight (along the axis) or
    /// diamond (crossed); its pitch along the circumference.
    Knurl { diamond: bool, pitch: f64 },
}

/// What the part's thread and knurl requirements come to ([`manufacturing`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Manufacturing {
    /// Each requirement given to its owner, in the reading's order.
    pub owned: Vec<Owned>,
    /// Each requirement not given to one, and why (`pmi_not_lowered`).
    pub refused: Vec<Problem>,
    /// Each owned requirement's table reference (`MFG 1`, by source), when the table is used.
    pub tags: Vec<(String, String)>,
}

impl Manufacturing {
    /// What *o*'s owner's callout prints for it: its suffix, or `SEE MFG n` when the table
    /// states it (`manufacturing_callout_suffix`).
    pub fn printed(&self, o: &Owned) -> String {
        self.tags
            .iter()
            .find(|(s, _)| *s == o.source)
            .map_or_else(|| o.suffix.clone(), |(_, tag)| format!("SEE {tag}"))
    }

    /// The requirements feature *id* carries, its thread before its knurl (`_manufacturing_suffix`).
    pub fn on(&self, id: &str) -> Vec<&Owned> {
        let mut out: Vec<&Owned> = self.owned.iter().filter(|o| o.owner == id).collect();
        out.sort_by_key(|o| o.aspect != "thread");
        out
    }

    /// Whether feature *id*'s callout prints an external thread's designation itself (not
    /// `SEE MFG n`): the designation then stands in place of the plain diameter, as ISO 6410
    /// designates a thread (`M10 x 1.5-6g RH`, not `ø10 M10 x 1.5-6g RH`). Beyond
    /// draftwright, which prints the diameter before it (draftwright-rust #23).
    pub fn replaces_diameter(&self, id: &str) -> bool {
        self.on(id).iter().any(|o| {
            o.aspect == "thread"
                && matches!(
                    o.depiction,
                    Some(Depiction {
                        kind: DepictionKind::Thread { .. },
                        ..
                    })
                )
                && self.printed(o) == o.suffix
        })
    }

    /// The terms feature *id*'s callout adds after its size, the requirements' printed text
    /// joined by `; ` and split at its spaces, and whether a through tap's suffix states the
    /// callout's `THRU` (draftwright then drops the bore's own).
    pub fn terms(&self, id: &str) -> (Vec<String>, bool) {
        let on = self.on(id);
        let printed: Vec<String> = on.iter().map(|o| self.printed(o)).collect();
        let through = on
            .iter()
            .zip(&printed)
            .any(|(o, p)| o.through && *p == o.suffix);
        let text = printed.join("; ");
        (
            text.split_whitespace().map(str::to_string).collect(),
            through,
        )
    }

    /// The manufacturing requirements table's rows, the header first (`ManufacturingSchedule.rows`:
    /// `REF | MANUFACTURING REQUIREMENT`, each requirement's text wrapped at its words, its tag on
    /// its first line), or none when the callouts print the requirements themselves.
    pub fn rows(&self) -> Vec<Vec<String>> {
        if self.tags.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![vec![
            "REF".to_string(),
            "MANUFACTURING REQUIREMENT".to_string(),
        ]];
        for (source, tag) in &self.tags {
            let Some(o) = self.owned.iter().find(|o| o.source == *source) else {
                continue;
            };
            for (i, line) in wrap(&o.text, SCHEDULE_WRAP).into_iter().enumerate() {
                rows.push(vec![if i == 0 { tag.clone() } else { String::new() }, line]);
            }
        }
        rows
    }

    /// The item IDs of the requirements the table states, in its order.
    pub fn scheduled(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter_map(|(s, _)| self.owned.iter().find(|o| o.source == *s))
            .map(|o| o.id.clone())
            .collect()
    }
}

/// *text* broken at its words into lines of at most *width* characters, a longer word alone on
/// its line (`_wrap_requirement`).
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let n = current.chars().count();
        if n > 0 && n + word.chars().count() + 1 > width {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            if n > 0 {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// A thread requirement (draftwright's `ThreadRequirement`), from its words, its structured
/// statement, or both reconciled.
#[derive(Clone, Debug, PartialEq)]
struct Thread {
    internal: bool,
    designation: String,
    nominal: f64,
    pitch: f64,
    class: String,
    hand: String,
    full_available_length: bool,
    minimum_full_thread: Option<f64>,
    drill_diameter: Option<f64>,
    drill_depth: Option<f64>,
    drill_point_angle: Option<f64>,
    through: bool,
}

impl Thread {
    /// `ThreadRequirement.__post_init__`'s refusals.
    fn check(self) -> Result<Self, String> {
        for (name, v) in [("nominal_diameter", self.nominal), ("pitch", self.pitch)] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("thread {name} must be finite and positive"));
            }
        }
        for (name, v) in [
            ("minimum_full_thread", self.minimum_full_thread),
            ("drill_diameter", self.drill_diameter),
            ("drill_depth", self.drill_depth),
            ("drill_point_angle", self.drill_point_angle),
        ] {
            if v.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
                return Err(format!("thread {name} must be finite and positive"));
            }
        }
        if self.internal
            && let (Some(full), Some(depth)) = (self.minimum_full_thread, self.drill_depth)
            && full > depth
        {
            return Err("thread minimum full thread cannot exceed drill depth".into());
        }
        if self.through
            && (!self.internal || self.drill_depth.is_some() || self.drill_point_angle.is_some())
        {
            return Err("through thread must be internal without drill depth or point".into());
        }
        Ok(self)
    }

    /// `callout_suffix`: the terms the owner's own dimensions do not state.
    fn suffix(&self) -> String {
        if !self.internal {
            let length = if self.full_available_length {
                ", FULL AVAILABLE LENGTH"
            } else {
                ""
            };
            return format!("{}{length}", self.designation);
        }
        let mut terms = vec![self.designation.clone()];
        if self.through {
            terms.push("THRU".into());
        }
        if let Some(full) = self.minimum_full_thread {
            terms.push(format!("{} MIN FULL THREAD", crate::dims::fmt(full)));
        }
        if let Some(angle) = self.drill_point_angle {
            terms.push(format!(
                "{}° CONVENTIONAL DRILL POINT",
                crate::dims::fmt(angle)
            ));
        }
        terms.join("; ")
    }
}

/// A knurl requirement (draftwright's `KnurlRequirement`).
#[derive(Clone, Debug, PartialEq)]
struct Knurl {
    /// `straight` or `diamond`.
    pattern: String,
    pitch: f64,
    full_width: bool,
    edge_chamfer: Option<f64>,
    maximum_diameter: Option<f64>,
    /// `cut`, `formed`: the processes permitted.
    processes: Vec<&'static str>,
}

impl Knurl {
    fn check(self) -> Result<Self, String> {
        if !(self.pitch.is_finite() && self.pitch > 0.0) {
            return Err("knurl pitch must be finite and positive".into());
        }
        for (name, v) in [
            ("edge_chamfer", self.edge_chamfer),
            ("maximum_diameter", self.maximum_diameter),
        ] {
            if v.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
                return Err(format!("knurl {name} must be finite and positive"));
            }
        }
        Ok(self)
    }

    /// `callout_suffix`: compact terms after the owner's diameter.
    fn suffix(&self) -> String {
        let magnitude = |v: f64| crate::pmi::fmt_magnitude(v, None);
        let mut terms = Vec::new();
        if self.maximum_diameter.is_some() {
            terms.push("MAX AFTER KNURL".to_string());
        }
        let mut knurl = format!(
            "{} KNURL P{}",
            self.pattern.to_uppercase(),
            magnitude(self.pitch)
        );
        if self.full_width {
            knurl += " FULL WIDTH";
            if let Some(c) = self.edge_chamfer {
                knurl += &format!(" TO C{} CHAMFERS", magnitude(c));
            }
        }
        terms.push(knurl);
        if !self.processes.is_empty() {
            let p: Vec<String> = self.processes.iter().map(|p| p.to_uppercase()).collect();
            terms.push(format!("{} PERMITTED", p.join("/")));
        }
        terms.join("; ")
    }

    /// `callout_text`: the whole requirement, its maximum diameter leading.
    fn text(&self) -> String {
        match self.maximum_diameter {
            None => self.suffix(),
            Some(d) => format!("ø{} {}", crate::pmi::fmt_magnitude(d, None), self.suffix()),
        }
    }
}

/// A position in a requirement's words, matched as draftwright's patterns match them
/// (case-insensitive, `\s` runs where they allow them).
struct Words<'a> {
    s: &'a str,
    at: usize,
}

impl<'a> Words<'a> {
    /// Whitespace skipped; how much.
    fn spaces(&mut self) -> usize {
        let rest = &self.s[self.at..];
        let n = rest.len() - rest.trim_start().len();
        self.at += n;
        n
    }

    /// At least one whitespace character (`\s+`).
    fn space(&mut self) -> bool {
        self.spaces() > 0
    }

    /// *word* exactly, letters in either case.
    fn lit(&mut self, word: &str) -> bool {
        let rest = &self.s[self.at..];
        let ok = rest.len() >= word.len()
            && rest.is_char_boundary(word.len())
            && rest[..word.len()].eq_ignore_ascii_case(word);
        if ok {
            self.at += word.len();
        }
        ok
    }

    /// `\d+(?:\.\d+)?`.
    fn number(&mut self) -> Option<f64> {
        let rest = &self.s[self.at..];
        let int = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if int == 0 {
            return None;
        }
        let frac = rest[int..]
            .strip_prefix('.')
            .map(|f| f.find(|c: char| !c.is_ascii_digit()).unwrap_or(f.len()))
            .filter(|&n| n > 0)
            .map_or(0, |n| n + 1);
        let v = rest[..int + frac].parse().ok()?;
        self.at += int + frac;
        Some(v)
    }

    /// `[A-Za-z0-9]+`.
    fn alnum(&mut self) -> Option<&'a str> {
        let rest = &self.s[self.at..];
        let n = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        (n > 0).then(|| {
            self.at += n;
            &rest[..n]
        })
    }

    fn end(&self) -> bool {
        self.at == self.s.len()
    }
}

/// A thread designation at the start of the words (`M<nominal>\s*x\s*<pitch>-<class>\s+RH|LH`):
/// the designation as written, its nominal size, pitch, class and hand.
fn thread_designation(w: &mut Words) -> Option<(String, f64, f64, String, String)> {
    let start = w.at;
    if !w.lit("M") {
        return None;
    }
    let nominal = w.number()?;
    w.spaces();
    if !w.lit("x") {
        return None;
    }
    w.spaces();
    let pitch = w.number()?;
    if !w.lit("-") {
        return None;
    }
    let class = w.alnum()?.to_string();
    if !w.space() {
        return None;
    }
    let hand = if w.lit("RH") {
        "RH"
    } else if w.lit("LH") {
        "LH"
    } else {
        return None;
    };
    Some((
        w.s[start..w.at].to_string(),
        nominal,
        pitch,
        class,
        hand.to_string(),
    ))
}

/// A thread from its words (`_text_thread_requirement`, `_EXTERNAL_THREAD`, `_INTERNAL_THREAD`).
fn prose_thread(kind: &str, label: &str) -> Result<Thread, String> {
    let internal = kind == "internal_thread";
    let syntax = || format!("unsupported {} syntax", kind.replace('_', " "));
    let text = label.trim();
    let mut w = Words { s: text, at: 0 };
    let (designation, nominal, pitch, class, hand) =
        thread_designation(&mut w).ok_or_else(syntax)?;
    if !w.lit(",") {
        return Err(syntax());
    }
    w.spaces();
    let mut t = Thread {
        internal,
        designation,
        nominal,
        pitch,
        class,
        hand,
        full_available_length: !internal,
        minimum_full_thread: None,
        drill_diameter: None,
        drill_depth: None,
        drill_point_angle: None,
        through: false,
    };
    if !internal {
        let region = (|| {
            if !w.lit("full available length on nominal DIA") || !w.space() {
                return None;
            }
            let region = w.number()?;
            (w.space() && w.lit("region") && w.end()).then_some(region)
        })()
        .ok_or_else(syntax)?;
        if !same_number(nominal, region, 1e-6) {
            return Err("external thread designation and nominal region disagree".into());
        }
        return t.check();
    }
    let after = w.at;
    // `<full> mm minimum full thread; DIA <drill> tapping drill x <depth> mm full-diameter
    // depth[; conventional <angle> degree drill point]`.
    let blind = (|| {
        let full = w.number()?;
        w.spaces();
        if !w.lit("mm minimum full thread;") {
            return None;
        }
        w.spaces();
        if !(w.lit("DIA") && w.space()) {
            return None;
        }
        let drill = w.number()?;
        if !(w.space() && w.lit("tapping drill") && w.space() && w.lit("x") && w.space()) {
            return None;
        }
        let depth = w.number()?;
        w.spaces();
        if !w.lit("mm full-diameter depth") {
            return None;
        }
        let before = w.at;
        let angle = (|| {
            if !w.lit(";") {
                return None;
            }
            w.spaces();
            if !(w.lit("conventional") && w.space()) {
                return None;
            }
            let angle = w.number()?;
            (w.space() && w.lit("degree drill point")).then_some(angle)
        })();
        if angle.is_none() {
            w.at = before;
        }
        w.end().then_some((full, drill, depth, angle))
    })();
    if let Some((full, drill, depth, angle)) = blind {
        t.minimum_full_thread = Some(full);
        t.drill_diameter = Some(drill);
        t.drill_depth = Some(depth);
        t.drill_point_angle = angle;
        return t.check();
    }
    w.at = after;
    // `full thread through; DIA <drill> tapping drill through`.
    let through = (|| {
        if !w.lit("full thread through;") {
            return None;
        }
        w.spaces();
        if !(w.lit("DIA") && w.space()) {
            return None;
        }
        let drill = w.number()?;
        (w.space() && w.lit("tapping drill through") && w.end()).then_some(drill)
    })();
    let drill = through.ok_or_else(syntax)?;
    t.drill_diameter = Some(drill);
    t.through = true;
    t.check()
}

/// A knurl from its words (`_text_knurl_requirement`, `_KNURL`).
fn prose_knurl(label: &str) -> Result<Knurl, String> {
    let text = label.trim();
    let mut w = Words { s: text, at: 0 };
    let parsed = (|| {
        let pattern = if w.lit("Straight") {
            "straight"
        } else if w.lit("Diamond") {
            "diamond"
        } else {
            return None;
        };
        if !w.lit(" knurl,") {
            return None;
        }
        w.spaces();
        let pitch = w.number()?;
        w.spaces();
        if !w.lit("mm pitch,") {
            return None;
        }
        w.spaces();
        let before = w.at;
        let chamfer = (|| {
            if !w.lit("full width between C") {
                return None;
            }
            let c = w.number()?;
            if !w.lit(" chamfers,") {
                return None;
            }
            w.spaces();
            Some(c)
        })();
        if chamfer.is_none() {
            w.at = before;
        }
        if !(w.lit("DIA") && w.space()) {
            return None;
        }
        let diameter = w.number()?;
        w.spaces();
        if !w.lit("mm maximum after knurling;") {
            return None;
        }
        w.spaces();
        (w.lit("cut or formed process permitted") && w.end())
            .then_some((pattern, pitch, chamfer, diameter))
    })();
    let (pattern, pitch, chamfer, diameter) =
        parsed.ok_or("unsupported knurl requirement syntax")?;
    Knurl {
        pattern: pattern.into(),
        pitch,
        full_width: chamfer.is_some(),
        edge_chamfer: chamfer,
        maximum_diameter: Some(diameter),
        processes: vec!["cut", "formed"],
    }
    .check()
}

/// `_same_number`: within *tol*, or a millionth of *b*.
fn same_number(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol.max(b.abs() * 1e-6)
}

/// The structured field names draftwright reads for each kind
/// (`_STRUCTURED_MANUFACTURING_FIELDS`).
fn structured_names(kind: &str) -> &'static [&'static str] {
    match kind {
        "internal_thread" => &[
            "thread side",
            "designation",
            "nominal size",
            "pitch",
            "fit class",
            "hand",
            "through",
            "tapping drill diameter",
            "tapping drill depth",
            "minimum full thread",
            "full thread length",
            "drill diameter",
            "drill depth",
        ],
        "external_thread" => &[
            "thread side",
            "designation",
            "nominal size",
            "pitch",
            "fit class",
            "hand",
            "thread length",
        ],
        _ => &[
            "pattern",
            "diametral pitch",
            "major diameter",
            "pitch",
            "maximum diameter",
        ],
    }
}

/// A requirement's structured fields, checked (`_structured_fields`, `_structured_number`,
/// `_structured_text`).
struct Fields<'a>(&'a [(String, crate::pmi::Field)]);

impl<'a> Fields<'a> {
    fn checked(r: &'a Record) -> Result<Self, String> {
        let names: Vec<&str> = r.fields.iter().map(|(n, _)| n.as_str()).collect();
        let distinct: BTreeSet<&str> = names.iter().copied().collect();
        if distinct.len() != names.len() || names.iter().any(|n| n.is_empty()) {
            return Err("structured manufacturing fields have duplicate or empty names".into());
        }
        let known = structured_names(&r.kind);
        let unknown: Vec<&str> = distinct
            .iter()
            .copied()
            .filter(|n| !known.contains(n))
            .collect();
        if !unknown.is_empty() {
            return Err(format!(
                "unsupported structured manufacturing fields: {}",
                unknown.join(", ")
            ));
        }
        Ok(Fields(&r.fields))
    }

    fn get(&self, name: &str) -> Option<&'a crate::pmi::Field> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    fn number(&self, name: &str) -> Result<f64, String> {
        match self.get(name) {
            Some(crate::pmi::Field::Length(v)) if v.is_finite() && *v > 0.0 => Ok(*v),
            _ => Err(format!(
                "structured manufacturing {name} needs a positive length measure"
            )),
        }
    }

    fn alias(&self, name: &str, alias: &str) -> Result<f64, String> {
        if self.has(name) && self.has(alias) {
            return Err(format!(
                "structured manufacturing has both {name} and {alias}"
            ));
        }
        self.number(if self.has(name) { name } else { alias })
    }

    fn text(&self, name: &str) -> Result<String, String> {
        match self.get(name) {
            Some(crate::pmi::Field::Text(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
            _ => Err(format!(
                "structured manufacturing {name} needs descriptive text"
            )),
        }
    }
}

/// `M<nominal>\s*x\s*<pitch>` as the whole of *s*.
fn metric_size(s: &str) -> Option<(f64, f64)> {
    let mut w = Words { s, at: 0 };
    if !w.lit("M") {
        return None;
    }
    let nominal = w.number()?;
    w.spaces();
    if !w.lit("x") {
        return None;
    }
    w.spaces();
    let pitch = w.number()?;
    w.end().then_some((nominal, pitch))
}

/// A thread from its structured statement (`_structured_thread_requirement` with
/// `_structured_thread_extent`), *prose* its words' reading when they have one.
fn structured_thread(
    r: &Record,
    prose: Option<&Thread>,
    cylinders: &[Cylinder],
) -> Result<Thread, String> {
    let f = Fields::checked(r)?;
    let side = f.text("thread side")?.to_lowercase();
    if !matches!(side.as_str(), "internal" | "external") || r.kind != format!("{side}_thread") {
        return Err("structured thread side disagrees with requirement kind".into());
    }
    let internal = side == "internal";
    let (nominal, pitch) = metric_size(&f.text("designation")?)
        .ok_or("structured thread designation or nominal size is unsupported")?;
    let disagree = "structured thread designation disagrees with nominal size or pitch";
    if f.has("nominal size") {
        let size = f.text("nominal size")?;
        let mut w = Words { s: &size, at: 0 };
        let stated = w.lit("M").then(|| w.number()).flatten().filter(|_| w.end());
        if !stated.is_some_and(|n| same_number(nominal, n, 1e-6)) {
            return Err(disagree.into());
        }
    }
    if f.has("pitch") && !same_number(pitch, f.number("pitch")?, 1e-6) {
        return Err(disagree.into());
    }
    let class = f.text("fit class")?;
    if !class.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("structured thread fit class is unsupported".into());
    }
    let hand = match f.text("hand")?.to_lowercase().as_str() {
        "right" | "rh" => "RH",
        "left" | "lh" => "LH",
        _ => return Err("structured thread hand is unsupported".into()),
    };
    let through = match f.get("through") {
        None => false,
        Some(crate::pmi::Field::Text(s)) if matches!(s.to_lowercase().as_str(), "true" | "yes") => {
            true
        }
        Some(crate::pmi::Field::Text(s)) if matches!(s.to_lowercase().as_str(), "false" | "no") => {
            false
        }
        Some(_) => return Err("structured thread through flag is unsupported".into()),
    };
    let designation = format!("M{} x {}-{class} {hand}", g6(nominal), g6(pitch));
    let (mut drill, mut depth, mut full) = (None, None, None);
    let full_available_length = if internal {
        drill = Some(f.alias("tapping drill diameter", "drill diameter")?);
        depth = if through {
            None
        } else {
            Some(f.alias("tapping drill depth", "drill depth")?)
        };
        if through && (f.has("tapping drill depth") || f.has("drill depth")) {
            return Err("structured through tap cannot declare a blind drill depth".into());
        }
        full = if f.has("full thread length") {
            if !prose.is_some_and(|p| p.minimum_full_thread.is_some()) {
                return Err("structured full thread length needs a matching minimum-full-thread prose requirement".into());
            }
            Some(f.alias("minimum full thread", "full thread length")?)
        } else if f.has("minimum full thread") {
            Some(f.number("minimum full thread")?)
        } else {
            None
        };
        if through && let Some(full) = full {
            let fits = match cylinders {
                [c] => full <= c.interval[1] - c.interval[0] + 0.01,
                _ => false,
            };
            if !fits {
                return Err("structured minimum full thread exceeds source cylinder".into());
            }
        }
        false
    } else {
        if through {
            return Err("structured external thread cannot be a through tap".into());
        }
        if f.has("thread length") {
            // A tip chamfer can shorten the source face: its owner's span is checked once the
            // owner is known ([`manufacturing`]).
            f.number("thread length")?;
        } else if !prose.is_some_and(|p| p.full_available_length) {
            return Err("structured external thread has no length requirement".into());
        }
        true
    };
    Thread {
        internal,
        designation,
        nominal,
        pitch,
        class,
        hand: hand.into(),
        full_available_length,
        minimum_full_thread: full,
        drill_diameter: drill,
        drill_depth: depth,
        drill_point_angle: None,
        through,
    }
    .check()
}

/// Whether record *r* pairs a requirement in words with its structured statement (draftwright's
/// two source IDs): it has fields, and its label is its words, not its kind (an unpaired
/// structured statement is labelled with its kind, [`crate::pmi`]).
fn paired(r: &Record) -> bool {
    !r.fields.is_empty() && r.label != r.kind.replace('_', " ")
}

/// A thread from its record (`_thread_requirement`): its words and its structured statement
/// reconciled, value by value.
fn thread(r: &Record, cylinders: &[Cylinder]) -> Result<Thread, String> {
    if r.fields.is_empty() {
        return prose_thread(&r.kind, &r.label);
    }
    let prose = match prose_thread(&r.kind, &r.label) {
        Ok(t) => Some(t),
        Err(_) if paired(r) => return Err("structured thread cannot reconcile with prose".into()),
        Err(_) => None,
    };
    let structured = structured_thread(r, prose.as_ref(), cylinders)?;
    let Some(prose) = prose else {
        return Ok(structured);
    };
    let num = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => same_number(a, b, 1e-6),
        (a, b) => a == b,
    };
    let s = &structured;
    let agree = s.internal == prose.internal
        && same_number(s.nominal, prose.nominal, 1e-6)
        && same_number(s.pitch, prose.pitch, 1e-6)
        && s.class == prose.class
        && s.hand == prose.hand
        && num(s.minimum_full_thread, prose.minimum_full_thread)
        && num(s.drill_diameter, prose.drill_diameter)
        && num(s.drill_depth, prose.drill_depth)
        && s.through == prose.through
        && s.full_available_length == prose.full_available_length;
    if !agree {
        return Err("structured thread values disagree with prose".into());
    }
    Thread {
        drill_point_angle: prose.drill_point_angle,
        ..structured
    }
    .check()
}

/// A knurl from its record (`_knurl_requirement`, `_structured_knurl_requirement`).
fn knurl(r: &Record) -> Result<Knurl, String> {
    if r.fields.is_empty() {
        return prose_knurl(&r.label);
    }
    let prose = match prose_knurl(&r.label) {
        Ok(k) => Some(k),
        Err(_) if paired(r) => return Err("structured knurl cannot reconcile with prose".into()),
        Err(_) => None,
    };
    let f = Fields::checked(r)?;
    let pattern = f.text("pattern")?.to_lowercase();
    if !matches!(pattern.as_str(), "straight" | "diamond") {
        return Err("structured knurl pattern is unsupported".into());
    }
    if prose.is_none() && (f.has("diametral pitch") || f.has("major diameter")) {
        // These labels alone do not establish a linear pitch or a maximum after knurling.
        return Err("structured knurl aliases need a matching prose requirement".into());
    }
    let structured = Knurl {
        pattern,
        pitch: f.alias("diametral pitch", "pitch")?,
        full_width: false,
        edge_chamfer: None,
        maximum_diameter: Some(f.alias("major diameter", "maximum diameter")?),
        processes: Vec::new(),
    }
    .check()?;
    let Some(prose) = prose else {
        return Ok(structured);
    };
    let agree = structured.pattern == prose.pattern
        && same_number(structured.pitch, prose.pitch, 1e-6)
        && match (structured.maximum_diameter, prose.maximum_diameter) {
            (Some(a), Some(b)) => same_number(a, b, 1e-6),
            _ => false,
        };
    if !agree {
        return Err("structured knurl values disagree with prose".into());
    }
    Ok(Knurl {
        full_width: prose.full_width,
        edge_chamfer: prose.edge_chamfer,
        processes: prose.processes,
        ..structured
    })
}

/// Where a thread or knurl requirement applies (draftwright's `_manufacturing_requirement_topology`
/// with `_cylindrical_references_from_shapes`): its faces' exact cylinders, identical ones once,
/// and the reasons a face is none. A requirement on an edge, or on no face, has none.
pub fn requirement_geometry(part: &Part, r: &Record) -> Geometry {
    let mut g = Geometry::default();
    let noun = "manufacturing requirement";
    if !r.edges.is_empty() {
        g.reasons
            .push(format!("one {noun} reference is not a face"));
    }
    for &f in &r.faces {
        match cylinder(part, f) {
            Ok(c) => {
                if !g.cylinders.iter().any(|d| same_cylinder(d, &c)) {
                    g.cylinders.push(c);
                }
            }
            Err(why) => g.reasons.push(
                why.replace("one diameter reference", &format!("one {noun} reference"))
                    .replace(
                        "one cylindrical reference",
                        &format!("one {noun} reference"),
                    ),
            ),
        }
    }
    if g.cylinders.is_empty() && g.reasons.is_empty() {
        g.reasons
            .push(format!("{noun} reference geometry is unavailable"));
    }
    g.reasons = dedup(g.reasons);
    g
}

/// A feature a thread or knurl may belong to (draftwright's `StepFeature`, `BossFeature`,
/// `HoleFeature`), read in the drawing's frame.
struct Owner {
    id: String,
    /// `hole`, `step` or `boss`.
    kind: &'static str,
    axis: usize,
    diameter: f64,
    /// A hole's opening (its member station); a point on a step's or boss's axis.
    at: [f64; 3],
    /// A step's or boss's span along its axis; a hole's none.
    span: Option<[f64; 2]>,
    through: bool,
    depth: Option<f64>,
}

/// The features that may own a thread or knurl: each hole the callouts state on its own (a
/// member of a hole pattern the callouts state as a pattern is not one: draftwright's pattern
/// owns a thread only when it has one member), each turned step that is a band of its own, and
/// each cylindrical boss that is neither a groove's floor nor a turned step's band (as
/// [`crate::dims`] reads them).
fn owners(analysis: &Analysis) -> Vec<Owner> {
    use crate::bridge::holes;
    let frame = holes::frame(analysis);
    let mut out = Vec::new();
    let readable: Vec<holes::Hole> = holes::read(analysis)
        .into_iter()
        .filter_map(|(_, h)| h.ok())
        .filter(|h| h.principal.is_some() && (h.through || h.depth.is_some()))
        .collect();
    let reads: Vec<&holes::Hole> = readable.iter().collect();
    let bosses = holes::bosses(analysis);
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    for p in analysis
        .recognition
        .as_ref()
        .map_or(&[][..], |r| r.patterns.as_slice())
    {
        if let Ok(members) = holes::established_with(p, &reads, &bosses)
            && !members.iter().any(|m| taken.contains(m))
        {
            taken.extend(members);
        }
    }
    for (i, h) in readable.iter().enumerate() {
        if taken.contains(&i) {
            continue;
        }
        out.push(Owner {
            id: h.feature.id.clone(),
            kind: "hole",
            axis: h.principal.expect("a principal hole").0,
            diameter: h.diameter,
            at: h.at,
            span: None,
            through: h.through,
            depth: h.depth,
        });
    }
    if let Ok(steps) = crate::lint::turned::bands(analysis) {
        for f in steps {
            if let Ok(b) = crate::lint::turned::band(&frame, f) {
                out.push(Owner {
                    id: f.id.clone(),
                    kind: "step",
                    axis: b.axis,
                    diameter: b.diameter,
                    at: b.on,
                    span: Some([b.lo, b.hi]),
                    through: false,
                    depth: None,
                });
            }
        }
    }
    for f in analysis.features.iter().filter(|f| f.kind == "boss") {
        if crate::lint::bosses::groove_floor(analysis, f)
            || crate::lint::bosses::turned_band(analysis, f)
        {
            continue;
        }
        let p = &f.parameters;
        let (Some(top), Some(dir), Some(diameter)) = (
            holes::point(p.get("location")),
            holes::point(p.get("axis")),
            p.get("diameter").and_then(serde_json::Value::as_f64),
        ) else {
            continue;
        };
        let top = holes::to_part(&frame, top);
        let dir = holes::dir_to_part(&frame, dir);
        let Some(k) = holes::principal(dir) else {
            continue;
        };
        let height = p
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .filter(|h| *h > 0.0);
        let span = height.map(|h| {
            let base = top[k] - dir[k].signum() * h;
            [base.min(top[k]), base.max(top[k])]
        });
        out.push(Owner {
            id: f.id.clone(),
            kind: "boss",
            axis: k,
            diameter,
            at: top,
            span,
            through: false,
            depth: None,
        });
    }
    out
}

/// Whether *c*'s axis is the line along world axis *k* through *p* (`_same_axis_line`, 0.01 mm).
fn same_axis_line(c: &Cylinder, p: [f64; 3], k: usize) -> bool {
    (0..3).all(|i| i == k || (c.origin[i] - p[i]).abs() <= 0.01)
}

/// `_external_owner_matches`: an external cylinder on the step's or boss's axis, of its
/// diameter, within its span.
fn external_owner(c: &Cylinder, o: &Owner) -> bool {
    if c.internal || c.principal() != Some(o.axis) {
        return false;
    }
    if !same_number(2.0 * c.radius, o.diameter, 0.01) || !same_axis_line(c, o.at, o.axis) {
        return false;
    }
    let Some([lo, hi]) = o.span else {
        return false;
    };
    let [a, b] = c.interval;
    lo - 0.01 <= a && a < b && b <= hi + 0.01
}

/// `_internal_member_matches`: an internal cylinder on the hole's axis, of its diameter, running
/// from its opening its depth (a through hole's within the part's box).
fn internal_owner(c: &Cylinder, o: &Owner, envelope: [[f64; 3]; 2]) -> bool {
    if !c.internal || c.principal() != Some(o.axis) {
        return false;
    }
    if !same_number(2.0 * c.radius, o.diameter, 0.01) || !same_axis_line(c, o.at, o.axis) {
        return false;
    }
    let k = o.axis;
    let [lo, hi] = c.interval;
    let station = o.at[k];
    if o.through {
        let [box_lo, box_hi] = [envelope[0][k], envelope[1][k]];
        if !(lo - 0.01 <= station && station <= hi + 0.01) {
            return false;
        }
        if lo < box_lo - 0.01 || hi > box_hi + 0.01 || hi.min(box_hi) - lo.max(box_lo) <= 0.01 {
            return false;
        }
        return o.depth.is_none_or(|d| same_number(hi - lo, d, 0.01));
    }
    let Some(depth) = o.depth else {
        return false;
    };
    same_number(hi - lo, depth, 0.01) && (lo - station).abs().min((hi - station).abs()) <= 0.01
}

/// A requirement's owner (its index), its reading, and how its owner's views draw it.
type Proposal = (usize, Aspect, Option<Depiction>);

/// A requirement read from its record.
enum Aspect {
    Thread(Thread),
    Knurl(Knurl),
}

/// `_manufacturing_owner_matches`: whether *o* owns requirement *a* on cylinder *c*.
fn owns(a: &Aspect, c: &Cylinder, o: &Owner, envelope: [[f64; 3]; 2]) -> bool {
    match a {
        Aspect::Thread(t) if t.internal => {
            o.kind == "hole"
                && if t.through {
                    o.through && internal_owner(c, o, envelope)
                } else {
                    !o.through
                        && o.depth.is_some_and(|d| {
                            t.drill_depth.is_some_and(|dd| same_number(dd, d, 0.01))
                        })
                        && internal_owner(c, o, envelope)
                }
        }
        _ => matches!(o.kind, "step" | "boss") && external_owner(c, o),
    }
}

/// The part's turned chamfers as `_turned_tip_chamfers_cover_gap` reads them: axis, legs, angle
/// and centre, in the drawing's frame.
fn turned_chamfers(analysis: &Analysis) -> Vec<(usize, f64, f64, f64, [f64; 3])> {
    use crate::bridge::holes;
    let frame = holes::frame(analysis);
    analysis
        .features
        .iter()
        .filter(|f| f.kind == "chamfer")
        .filter_map(|f| {
            let p = &f.parameters;
            if p.get("turned").and_then(serde_json::Value::as_bool) != Some(true) {
                return None;
            }
            let letter = p.get("axis")?.as_str()?;
            let q = ["x", "y", "z"].iter().position(|a| *a == letter)?;
            let mut d = [0.0; 3];
            d[q] = 1.0;
            let k = holes::principal(holes::dir_to_part(&frame, d))?;
            let at = holes::to_part(&frame, holes::point(p.get("at"))?);
            let n = |name: &str| p.get(name).and_then(serde_json::Value::as_f64);
            Some((k, n("leg1")?, n("leg2")?, n("angle")?, at))
        })
        .collect()
}

/// `_turned_tip_chamfers_cover_gap`: the owner's span runs past the thread's cylinder only where
/// a recognised equal-leg 45° end chamfer fills the gap.
fn tip_chamfers_cover(
    c: &Cylinder,
    o: &Owner,
    chamfers: &[(usize, f64, f64, f64, [f64; 3])],
) -> bool {
    let Some([owner_lo, owner_hi]) = o.span else {
        return false;
    };
    let k = o.axis;
    let [source_lo, source_hi] = c.interval;
    let mut found = false;
    for (lo, hi) in [(owner_lo, source_lo), (source_hi, owner_hi)] {
        let gap = hi - lo;
        if gap <= 0.01 {
            continue;
        }
        found = true;
        let station = 0.5 * (lo + hi);
        let covered = chamfers.iter().any(|&(axis, leg1, leg2, angle, at)| {
            let radial = (0..3)
                .filter(|&i| i != k)
                .map(|i| (at[i] - c.origin[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            axis == k
                && same_number(leg1, gap, 0.01)
                && same_number(leg2, gap, 0.01)
                && same_number(angle, 45.0, 0.01)
                && same_number(at[k], station, 0.01)
                && same_number(radial, c.radius - gap / 2.0, 0.01)
        });
        if !covered {
            return false;
        }
    }
    found
}

/// The part's thread and knurl requirements given to their owners, as draftwright's
/// `lower_ap242_manufacturing_requirements` and `manufacturing_schedule` decide it:
///
/// - each record of kind `internal_thread`, `external_thread` or `knurl` is read from its words
///   and its structured statement, reconciled value by value (`_thread_requirement`,
///   `_knurl_requirement`); its one cylinder must be the size its text states (a tap's drill
///   diameter and drill depth, an external thread's nominal size, a knurl's maximum diameter);
/// - its owner is the one feature that cylinder proves: a blind hole of the drill depth or a
///   through hole whose bore it is (an internal thread), a turned step or boss whose diameter
///   and span hold it (an external thread or knurl; a structured thread length is the cylinder's,
///   or the owner's when recognised 45° tip chamfers fill the difference);
/// - two requirements of one aspect on one owner are both refused;
/// - with two or more requirements owned and one suffix longer than 52 characters, each owner's
///   callout prints `SEE MFG n` and the manufacturing requirements table states them, in the
///   order of their source IDs.
///
/// What is not owned is refused with draftwright's reason, never dropped: a record a face of
/// which another requirement already claims, whose reading left blockers, whose faces are not
/// one exact cylinder, whose text disagrees with its cylinder, or that no feature or several
/// own. A requirement a boss owns is refused too: the port prints none on a boss's diameter yet.
pub fn manufacturing(analysis: &Analysis) -> Manufacturing {
    let mut out = Manufacturing::default();
    let Some(Reading::Read(report)) = &analysis.pmi else {
        return out;
    };
    let records: Vec<&Record> = report
        .records
        .iter()
        .filter(|r| r.category == Category::ManufacturingRequirement && owned_kind(&r.kind))
        .collect();
    if records.is_empty() {
        return out;
    }
    let candidates = owners(analysis);
    let chamfers = turned_chamfers(analysis);
    let mut claimed: Vec<(usize, u64)> = Vec::new();
    // Each record's proposal: its owner's index and its aspect, or why it has none.
    let mut proposals: Vec<Result<Proposal, String>> = Vec::new();
    for r in &records {
        let proposal = (|| {
            let mut reasons = blockers(r);
            let g = r.geometry.clone().unwrap_or_default();
            if reasons.is_empty() {
                // draftwright's topology resolver gives each imported face to one requirement.
                let held: Vec<u64> = r
                    .faces
                    .iter()
                    .filter_map(|f| claimed.iter().find(|(c, _)| c == f).map(|(_, by)| *by))
                    .filter(|by| *by != r.part21_id)
                    .collect();
                for by in held {
                    reasons.push(format!(
                        "one imported face is already claimed by manufacturing requirement #{by}"
                    ));
                }
                if reasons.is_empty() {
                    claimed.extend(r.faces.iter().map(|&f| (f, r.part21_id)));
                }
            }
            if !reasons.is_empty() {
                return Err(dedup(reasons).join("; "));
            }
            let aspect = if r.kind == "knurl" {
                Aspect::Knurl(knurl(r)?)
            } else {
                Aspect::Thread(thread(r, &g.cylinders)?)
            };
            let disagree =
                || "manufacturing requirement text disagrees with source cylinder".to_string();
            let c = g.cylinders.first().ok_or_else(disagree)?;
            let expected = match &aspect {
                Aspect::Knurl(k) => k.maximum_diameter,
                Aspect::Thread(t) if t.internal => t.drill_diameter,
                Aspect::Thread(t) => Some(t.nominal),
            };
            if !expected.is_some_and(|d| same_number(2.0 * c.radius, d, 0.01)) {
                return Err(disagree());
            }
            if let Aspect::Thread(t) = &aspect
                && t.internal
                && !t.through
                && !t
                    .drill_depth
                    .is_some_and(|d| same_number(c.interval[1] - c.interval[0], d, 0.01))
            {
                return Err(disagree());
            }
            let matches: Vec<usize> = if g.cylinders.len() == 1 {
                (0..candidates.len())
                    .filter(|&i| owns(&aspect, c, &candidates[i], analysis.envelope))
                    .collect()
            } else {
                Vec::new()
            };
            let owner = match matches[..] {
                [i] => i,
                [] => {
                    return Err(
                        "unmatched manufacturing requirement: no canonical feature matches source topology"
                            .into(),
                    );
                }
                _ => {
                    return Err(format!(
                        "ambiguous manufacturing requirement: source topology matches {} canonical features",
                        matches.len()
                    ));
                }
            };
            let o = &candidates[owner];
            if let Aspect::Thread(t) = &aspect
                && !t.internal
                && let Ok(length) = Fields(&r.fields).number("thread length")
            {
                let span = o.span.ok_or("external thread owner has no axial span")?;
                let source = c.interval[1] - c.interval[0];
                if !(same_number(source, length, 0.01)
                    || (same_number(span[1] - span[0], length, 0.01)
                        && tip_chamfers_cover(c, o, &chamfers)))
                {
                    return Err("structured thread length disagrees with source cylinder or evidenced owner span".into());
                }
            }
            // The drawn length is the requirement's cylinder: where a stated thread length is the
            // owner's span, the difference is a tip chamfer, whose narrower profile the minor
            // diameter line must not run past.
            let span = c.interval;
            let kind = match &aspect {
                Aspect::Thread(t) if t.internal => None,
                // ISO 724's basic minor diameter of an external thread, d3 = d − 1.22687·P.
                Aspect::Thread(t) => Some(DepictionKind::Thread {
                    minor: t.nominal - 1.226_87 * t.pitch,
                }),
                Aspect::Knurl(k) => Some(DepictionKind::Knurl {
                    diamond: k.pattern == "diamond",
                    pitch: k.pitch,
                }),
            };
            let depiction = kind.filter(|_| o.kind == "step").map(|kind| Depiction {
                axis: o.axis,
                on: o.at,
                span,
                diameter: o.diameter,
                kind,
            });
            Ok((owner, aspect, depiction))
        })();
        proposals.push(proposal);
    }
    // One requirement of each aspect per owner.
    let aspect_name = |a: &Aspect| match a {
        Aspect::Thread(_) => "thread",
        Aspect::Knurl(_) => "knurl",
    };
    let mut claims: BTreeMap<(usize, &'static str), usize> = BTreeMap::new();
    for p in proposals.iter().flatten() {
        *claims.entry((p.0, aspect_name(&p.1))).or_default() += 1;
    }
    let ids = report.identities();
    for (r, p) in records.iter().zip(proposals) {
        let refusal = match (p, ids.id(&r.source_id)) {
            (Err(why), _) | (Ok(_), Err(why)) => Some(why),
            (Ok((owner, aspect, depiction)), Ok(id)) => {
                let name = aspect_name(&aspect);
                let o = &candidates[owner];
                if claims[&(owner, name)] > 1 {
                    Some(format!(
                        "ambiguous {name} ownership: multiple manufacturing requirements claim canonical feature"
                    ))
                } else if o.kind == "boss" {
                    Some(format!(
                        "it belongs to boss {} (draftwright prints it on the boss's diameter), and the port prints no thread or knurl on a boss's callout yet",
                        o.id
                    ))
                } else {
                    let (suffix, text, through) = match &aspect {
                        Aspect::Thread(t) => (t.suffix(), t.suffix(), t.through),
                        Aspect::Knurl(k) => (k.suffix(), k.text(), false),
                    };
                    out.owned.push(Owned {
                        id: id.to_string(),
                        source: r.source_id.clone(),
                        owner: o.id.clone(),
                        aspect: name,
                        suffix,
                        text,
                        through,
                        depiction,
                    });
                    None
                }
            }
        };
        if let Some(why) = refusal {
            out.refused.push(Problem {
                id: format!("p:pmi_not_lowered:{}", r.source_id),
                code: "pmi_not_lowered".into(),
                severity: "error".into(),
                message: format!(
                    "PMI {} {} ({}) is not drawn: {why}.",
                    r.kind.replace('_', " "),
                    r.source_id,
                    r.label
                ),
                subjects: Vec::new(),
                refs: vec![r.source_id.clone()],
                ..Default::default()
            });
        }
    }
    // The table, for two or more requirements one of which is too long for its callout.
    if out.owned.len() >= 2
        && out
            .owned
            .iter()
            .any(|o| o.suffix.chars().count() > SCHEDULE_SUFFIX)
    {
        let mut sources: Vec<&str> = out.owned.iter().map(|o| o.source.as_str()).collect();
        sources.sort();
        out.tags = sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), format!("MFG {}", i + 1)))
            .collect();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// draftwright's default finish statement (`Ra <value> µm unless otherwise specified`), and
    /// what is not one.
    #[test]
    fn a_default_surface_texture_is_read_as_draftwright_reads_it() {
        for (label, ra) in [
            ("Ra 3.2 µm unless otherwise specified", Some("3.2")),
            ("  RA 0.8um UNLESS  OTHERWISE specified ", Some("0.8")),
            ("Ra 12 μm unless otherwise specified", Some("12")),
            ("Ra 3.2 µm", None),
            ("Ra 3.2 mm unless otherwise specified", None),
            ("Rz 3.2 µm unless otherwise specified", None),
            ("Ra .8 µm unless otherwise specified", None),
            ("Ra 3.2 µm unless otherwise specified; ground", None),
        ] {
            assert_eq!(default_ra(label).as_deref(), ra, "{label:?}");
        }
    }

    /// draftwright's thread and knurl grammars (`_INTERNAL_THREAD`, `_EXTERNAL_THREAD`, `_KNURL`)
    /// and their `callout_suffix` and `callout_text`, against draftwright's own output
    /// (`_thread_requirement`, `_knurl_requirement` on these labels, origin/main 4dfc63f).
    #[test]
    fn requirements_in_words_read_and_print_as_draftwrights() {
        let thread = |kind: &str, label: &str| prose_thread(kind, label).map(|t| t.suffix());
        let cases = [
            (
                "internal_thread",
                "M2 x 0.4-6H RH, 2.5 mm minimum full thread; DIA 1.6 tapping drill x 4 mm full-diameter depth",
                Ok("M2 x 0.4-6H RH; 2.5 MIN FULL THREAD"),
            ),
            (
                "internal_thread",
                "M2 x 0.4-6H RH, 2.5 mm minimum full thread; DIA 1.6 tapping drill x 4 mm full-diameter depth; conventional 118 degree drill point",
                Ok("M2 x 0.4-6H RH; 2.5 MIN FULL THREAD; 118° CONVENTIONAL DRILL POINT"),
            ),
            (
                "internal_thread",
                "M5 x 0.8-6H RH, full thread through; DIA 4.2 tapping drill through",
                Ok("M5 x 0.8-6H RH; THRU"),
            ),
            (
                "internal_thread",
                "  m2x0.4-6h lh,2.5mm minimum full thread;dia 1.6 tapping drill x 4mm full-diameter depth ",
                Ok("m2x0.4-6h lh; 2.5 MIN FULL THREAD"),
            ),
            (
                "internal_thread",
                "M2 x 0.4-6H RH, 5 mm minimum full thread; DIA 1.6 tapping drill x 4 mm full-diameter depth",
                Err("thread minimum full thread cannot exceed drill depth"),
            ),
            (
                "internal_thread",
                "M5 thread",
                Err("unsupported internal thread syntax"),
            ),
            (
                "external_thread",
                "M5 x 0.8-6g RH, full available length on nominal DIA 5 region",
                Ok("M5 x 0.8-6g RH, FULL AVAILABLE LENGTH"),
            ),
            (
                "external_thread",
                "M5 x 0.8-6g RH, full available length on nominal DIA 6 region",
                Err("external thread designation and nominal region disagree"),
            ),
        ];
        for (kind, label, want) in cases {
            assert_eq!(
                thread(kind, label),
                want.map(str::to_string).map_err(str::to_string),
                "{label:?}"
            );
        }
        let k = prose_knurl(
            "Straight knurl, 1 mm pitch, full width between C0.3 chamfers, DIA 6.25 mm maximum after knurling; cut or formed process permitted",
        )
        .unwrap();
        assert_eq!(
            k.suffix(),
            "MAX AFTER KNURL; STRAIGHT KNURL P1 FULL WIDTH TO C0.3 CHAMFERS; CUT/FORMED PERMITTED"
        );
        assert_eq!(
            k.text(),
            "ø6.25 MAX AFTER KNURL; STRAIGHT KNURL P1 FULL WIDTH TO C0.3 CHAMFERS; CUT/FORMED PERMITTED"
        );
        assert_eq!(
            wrap(&k.text(), SCHEDULE_WRAP),
            [
                "ø6.25 MAX AFTER KNURL; STRAIGHT KNURL P1",
                "FULL WIDTH TO C0.3 CHAMFERS; CUT/FORMED",
                "PERMITTED"
            ]
        );
        assert_eq!(
            prose_knurl("Diamond knurl, 1 mm pitch, DIA 6 mm maximum after knurling"),
            Err("unsupported knurl requirement syntax".to_string())
        );
    }

    /// The table is used for two or more requirements one of whose suffixes is over 52
    /// characters, tagged in their source IDs' order; a callout then prints its tag.
    #[test]
    fn long_requirements_go_to_the_table_in_source_order() {
        let owned = |source: &str, suffix: &str| Owned {
            id: format!("pmi:manufacturing_requirement.internal_thread:t={source}"),
            source: source.into(),
            owner: format!("f:{source}"),
            aspect: "thread",
            suffix: suffix.into(),
            text: suffix.into(),
            through: false,
            depiction: None,
        };
        let mut m = Manufacturing {
            owned: vec![
                owned("manufacturing_requirement:#9", "M2 x 0.4-6H RH"),
                owned("manufacturing_requirement:#10", &"X".repeat(53)),
            ],
            ..Default::default()
        };
        assert!(m.rows().is_empty());
        assert_eq!(m.printed(&m.owned[0].clone()), "M2 x 0.4-6H RH");
        // As draftwright sorts them: by source ID, as text.
        m.tags = vec![
            ("manufacturing_requirement:#10".into(), "MFG 1".into()),
            ("manufacturing_requirement:#9".into(), "MFG 2".into()),
        ];
        assert_eq!(m.printed(&m.owned[0].clone()), "SEE MFG 2");
        assert_eq!(
            m.terms("f:manufacturing_requirement:#9").0,
            ["SEE", "MFG", "2"]
        );
        assert_eq!(m.rows()[0], ["REF", "MANUFACTURING REQUIREMENT"]);
        assert_eq!(m.rows()[1][0], "MFG 1");
    }
}
