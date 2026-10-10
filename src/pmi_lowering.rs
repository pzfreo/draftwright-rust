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
//! - **Drawing-wide requirements** ([`sheet_requirements`], `lower_ap242_document_requirements`):
//!   the one general tolerance fills the title block's tolerance cell, the one default surface
//!   texture (`Ra 3.2 µm unless otherwise specified`) is drawn beside the title block, and each
//!   edge condition (`Break sharp edges 0.2 max`) is a general note; two of one default are
//!   ambiguous and neither is drawn.
//!
//! What is not lowered is said, never guessed: a record whose reading left blockers (datum
//! targets, composite relations, projected or non-uniform zones, datum modifiers, common
//! datums, …), a modifier draftwright has no drawing for (only all-around, all-over, the zone's
//! form and the material condition are drawn, as in draftwright), or an attachment its faces do
//! not prove (`pmi_not_lowered`); a source the reading did not read (`pmi_not_extracted`); a file
//! whose PMI could not be read (`pmi_not_read`); presentation, which is counted, not read
//! (`pmi_presentation_not_read`). Toleranced and basic dimensions are lowered with the other
//! dimensions ([`crate::dims`]' PMI dimensions, stage 2b), from where their faces put them
//! ([`dimension_geometry`], here).
//!
//! draftwright draws its PMI only when asked (`pmi="annotate"`; its default is off). The port has
//! no such option: a part's PMI is drawn whenever it has some (a question for the maintainer in
//! the stream's report).

use std::collections::BTreeSet;

use haecceity::Part;
use haecceity::geom::Surface;
use serde::{Deserialize, Serialize};

use crate::model::{Analysis, Problem};
use crate::pmi::{Category, Geometry, Outcome, Reading, Record};

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
    /// A surface or common label's text.
    Label { text: String },
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
                            item.side = if item.side == "left" {
                                "below"
                            } else {
                                "above"
                            };
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
            // A surface label, and a common label (read as a dimension), is drawn as its text.
            Category::SurfaceLabel => labelled(&mut items, &mut problems, r, centre),
            Category::Dimension if r.kind == "common_label" => {
                labelled(&mut items, &mut problems, r, centre)
            }
            // A dimension is stated or refused by stage 2b (`dims::pmi_dimensions`), never here.
            Category::Dimension => {}
            // A drawing-wide requirement is the sheet's ([`sheet_requirements`]).
            Category::ManufacturingRequirement if sheet_kind(&r.kind) => {}
            // Other manufacturing requirements are read, not drawn yet: said with draftwright's
            // code for a record that produced no feature, never dropped. A face's requirement is
            // a warning; a part's note is information. draftwright draws no leader note for a
            // thread or knurl: `lower_ap242_manufacturing_requirements` gives it to the one hole
            // or turned step its cylinder proves, and that feature's callout prints its
            // `callout_suffix` (`⌀1.6 ↧ 4 M2 x 0.4-6H RH; 2.5 MIN FULL THREAD`), or `SEE MFG n`
            // with a manufacturing requirements table (`manufacturing_schedule`): the port's
            // hole and diameter callouts (`dims`) would carry it. Face finishes are not drawn
            // either (`lower_ap242_face_finishes`).
            _ => {
                let face = r.kind.ends_with("_thread")
                    || r.kind == "knurl"
                    || !r.faces.is_empty()
                    || !r.edges.is_empty();
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
                    subjects: vec![r.source_id.clone()],
                });
            }
        }
    }
    (items, problems)
}

/// A surface or common label lowered into *items*, or its refusal into *problems*.
fn labelled(items: &mut Vec<Lowered>, problems: &mut Vec<Problem>, r: &Record, centre: [f64; 3]) {
    match label(r, centre) {
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
            subjects: vec![r.source_id.clone()],
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
        id: format!("pmi:{}", r.source_id),
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
    pub source: String,
    /// `general_tolerance` (the title block's tolerance cell), `default_surface_finish` (a
    /// surface texture symbol beside the title block) or `document_note` (a general note).
    pub kind: &'static str,
    /// What the sheet states: the tolerance's designation, the finish's Ra value in µm, the
    /// note's text.
    pub value: String,
    pub basis: String,
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
            subjects: vec![r.source_id.clone()],
        })
    };
    let records: Vec<&Record> = report
        .records
        .iter()
        .filter(|r| r.category == Category::ManufacturingRequirement && sheet_kind(&r.kind))
        .collect();
    for r in records.iter().filter(|r| !r.blockers.is_empty()) {
        refuse(&mut problems, r, &dedup(r.blockers.clone()).join("; "));
    }
    let open: Vec<&Record> = records
        .iter()
        .copied()
        .filter(|r| r.blockers.is_empty())
        .collect();
    let of =
        |kind: &str| -> Vec<&Record> { open.iter().copied().filter(|r| r.kind == kind).collect() };
    let tolerances = of("general_tolerances");
    match tolerances[..] {
        [] => {}
        [r] => match designation(r) {
            Ok(d) => out.push(SheetRequirement {
                source: r.source_id.clone(),
                kind: "general_tolerance",
                basis: format!(
                    "source-authored: the AP242 general tolerance {} ({}), the part's one, in the title block's tolerance cell (draftwright's GeneralTolerance)",
                    d, r.source_id
                ),
                value: d,
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
                source: r.source_id.clone(),
                kind: "default_surface_finish",
                basis: format!(
                    "source-authored: the AP242 surface texture '{}' ({}), the part's default finish, drawn as a surface texture symbol beside the title block (draftwright's DefaultSurfaceFinish)",
                    r.label, r.source_id
                ),
                value: ra,
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
                source: r.source_id.clone(),
                kind: "document_note",
                value: r.label.trim().to_string(),
                basis: format!(
                    "source-authored: the AP242 edge condition '{}' ({}), a general note (draftwright's DocumentNote)",
                    r.label.trim(),
                    r.source_id
                ),
            });
        }
    }
    (out, problems)
}

/// A drawing-wide requirement as the plan carries it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedRequirement {
    /// `pmi:<source>`.
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
        let id = format!("pmi:{}", r.source);
        let mut planned = PlannedRequirement {
            id: id.clone(),
            kind: r.kind.into(),
            source: r.source.clone(),
            text: r.value.clone(),
            carrier: "notes".into(),
            basis: r.basis.clone(),
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
                            subjects: vec![r.source.clone(), "sheet/title/tolerance".into()],
                        });
                        continue;
                    }
                }
            }
            "default_surface_finish" => {
                planned.carrier = "sheet".into();
                planned.text = format!("Ra {}", r.value);
                planned.placed = Some(crate::title_block::default_finish(block, &planned.text));
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
/// diameter's cylinders are, with a proven site on its arc; an angle and the curved kinds keep
/// no geometry (not drawn: see `pmi_dimensions`).
pub fn dimension_geometry(part: &Part, r: &Record) -> Geometry {
    let mut g = Geometry::default();
    g.reasons.extend(crate::pmi::placements_reason(r));
    let value = r.value.unwrap_or(0.0);
    match r.kind.as_str() {
        "linear" | "thickness" => linear_geometry(part, r, value, &mut g),
        "diameter" | "radius" => cylinder_geometry(part, r, value, &mut g),
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
                g.view = Some("plan".into());
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

/// A location from a principal x or y plane to a z bore's axis (draftwright's
/// `_planar_bore_axis_witness`): the stations, at the plane and at the axis, on one point of the
/// plane's face beside the axis halfway up their common height; and the plane's normal.
fn plane_to_bore(
    part: &Part,
    groups: &[Vec<usize>],
    boxes: &[Vec<[f64; 6]>],
) -> Option<([[f64; 3]; 2], usize)> {
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
    let axis = exact_axis(plane.z).filter(|&k| k < 2)?;
    if exact_axis(bore.z) != Some(2) {
        return None;
    }
    let (pb, bb) = (boxes[plane_i][0], boxes[bore_i][0]);
    let (z0, z1) = (pb[2].max(bb[2]), pb[5].min(bb[5]));
    if z1 - z0 <= 1e-6 || pb[axis + 3] - pb[axis] > 1e-5 {
        return None;
    }
    let centre = bore.origin;
    let mut witness = centre;
    witness[axis] = 0.5 * (pb[axis] + pb[axis + 3]);
    witness[2] = 0.5 * (z0 + z1);
    if !on_face(part, groups[plane_i][0], witness) {
        return None;
    }
    let mut points = [witness, witness];
    points[bore_i][axis] = centre[axis];
    Some((points, axis))
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
}
