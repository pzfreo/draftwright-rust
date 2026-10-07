//! The document model ([ADR R1](../docs/adr/R1-document-model.md), [docs/model.md]):
//! - **Analysis**: what the part is;
//! - **Spec**: what the author decided, keyed by stable IDs;
//! - **Plan**: what the sheet will say, every item with its ID, default flag and basis.
//!
//! The operations over them are [`analyse`], [`plan`], [`edit`] and [`render`]: pure, stateless
//! and deterministic.
//!
//! This first cut covers the views (which views, the sheet, the scale and the placement) and a
//! title block. Features, dimensions and annotations join the same documents as they are ported.

use std::collections::BTreeMap;
use std::path::Path;

use haecceity::Part;
use haecceity::hlr::{Exact, View, project};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::output::{
    Curve, Drawing, HAlign, Item, Layer, TextRun, VAlign, layer_styles, projected,
};
use crate::text::{Font, Segment};

pub const ANALYSIS_SCHEMA: &str = "draftwright.analysis/1";
pub const SPEC_SCHEMA: &str = "draftwright.spec/1";
pub const PLAN_SCHEMA: &str = "draftwright.plan/1";

/// Which file, and which reading of it, a document was made from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_sha256: String,
    pub part: String,
    pub face_count: usize,
    pub loader: String,
}

/// What the part is: regenerable from its STEP file, never edited.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub schema: String,
    pub binding: Binding,
    /// The part's box in its own frame, mm.
    pub envelope: [[f64; 3]; 2],
}

/// What the author decided. Anything absent takes its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Spec {
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<Binding>,
    pub sheet: SheetSpec,
    /// Per standard view (`front`, `plan`, `side`, `iso`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, ViewSpec>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            schema: SPEC_SCHEMA.into(),
            binding: None,
            sheet: SheetSpec::default(),
            views: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SheetSpec {
    /// `A4` … `A0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// `n:m`, from ISO 5455's series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub title: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ViewSpec {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub omit: bool,
}

/// What the sheet will say.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub schema: String,
    pub binding: Binding,
    pub sheet: PlannedSheet,
    pub views: Vec<PlannedView>,
    pub problems: Vec<Problem>,
    /// The spec the plan was made from, embedded in every output.
    pub spec: Spec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedSheet {
    pub size: String,
    /// Width and height, mm.
    pub page: [f64; 2],
    pub scale: String,
    pub default: bool,
    pub basis: String,
    pub title: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedView {
    pub id: String,
    /// Towards the viewer, and the view's up.
    pub direction: [f64; 3],
    pub up: [f64; 3],
    /// The view's centre on the page, mm.
    pub at: [f64; 2],
    pub default: bool,
    pub basis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub code: String,
    pub severity: String,
    pub message: String,
    pub subjects: Vec<String>,
}

/// A request that cannot be met as asked: what was wrong, and what would be accepted.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Invalid {
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.choices.is_empty() {
            write!(f, " Choose one of: {}.", self.choices.join(", "))?;
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>, choices: &[&str]) -> Invalid {
    Invalid {
        message: message.into(),
        choices: choices.iter().map(|c| c.to_string()).collect(),
    }
}

/// The standard views, third-angle (draftwright's default projection): towards the viewer, up.
const VIEWS: [(&str, [f64; 3], [f64; 3]); 4] = [
    ("front", [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
    ("plan", [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ("side", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ("iso", [1.0, -1.0, 1.0], [0.0, 0.0, 1.0]),
];
const SHEETS: [(&str, [f64; 2]); 5] = [
    ("A4", [297.0, 210.0]),
    ("A3", [420.0, 297.0]),
    ("A2", [594.0, 420.0]),
    ("A1", [841.0, 594.0]),
    ("A0", [1189.0, 841.0]),
];
/// ISO 5455's recommended scales, largest first.
const SCALES: [(&str, f64); 13] = [
    ("10:1", 10.0),
    ("5:1", 5.0),
    ("2:1", 2.0),
    ("1:1", 1.0),
    ("1:2", 0.5),
    ("1:5", 0.2),
    ("1:10", 0.1),
    ("1:20", 0.05),
    ("1:50", 0.02),
    ("1:100", 0.01),
    ("1:200", 0.005),
    ("1:500", 0.002),
    ("1:1000", 0.001),
];
/// The border's distance from the page edge; the views' clearance inside it; the title block's
/// band; the gap between views. All mm.
const MARGIN: f64 = 10.0;
const INSET: f64 = 10.0;
const TITLE_BAND: f64 = 40.0;
const GAP: f64 = 20.0;
const LOADER: &str = concat!("haecceity via draftwright ", env!("CARGO_PKG_VERSION"));

/// Reads the STEP file at *path*: the part, and its analysis.
pub fn analyse(path: &Path) -> Result<(Part, Analysis), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let part = haecceity::read_step(&bytes)
        .map_err(|e| format!("cannot read {} as STEP: {e:?}", path.display()))?;
    let b = part.bounds();
    let analysis = Analysis {
        schema: ANALYSIS_SCHEMA.into(),
        binding: Binding {
            source_sha256: Sha256::digest(&bytes)
                .iter()
                .map(|x| format!("{x:02x}"))
                .collect(),
            part: path.file_stem().map_or(String::new(), |s| {
                s.to_string_lossy().trim_end_matches(".step").to_string()
            }),
            face_count: part.faces.len(),
            loader: LOADER.into(),
        },
        envelope: [b.min, b.max],
    };
    Ok((part, analysis))
}

/// The box (width, height) and centre of the part's envelope seen in *view*, mm at 1:1.
fn extent(analysis: &Analysis, view: &View) -> ([f64; 2], [f64; 2]) {
    let [lo, hi] = analysis.envelope;
    let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for i in 0..8 {
        let corner = [0, 1, 2].map(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] });
        let p = view.map(corner);
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    (
        [max[0] - min[0], max[1] - min[1]],
        [0.5 * (min[0] + max[0]), 0.5 * (min[1] + max[1])],
    )
}

/// Plans the sheet: which views, where, at what scale and on what sheet. Each choice the spec
/// leaves open is marked as a default, with the reason for it.
pub fn plan(analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    let mut problems = Vec::new();
    if let Some(b) = &spec.binding
        && b.source_sha256 != analysis.binding.source_sha256
    {
        // Rebinding (ADR R3) arrives with feature overrides; views carry over as they are.
        problems.push(Problem {
            id: "p:rebind:part".into(),
            code: "spec_from_other_revision".into(),
            severity: "info".into(),
            message: format!("The spec was made for a different revision of {}; its view choices are carried over.", b.part),
            subjects: vec![],
        });
    }
    for name in spec.views.keys() {
        if !VIEWS.iter().any(|(v, ..)| v == name) {
            return Err(invalid(
                format!("There is no view called {name:?}."),
                &VIEWS.map(|v| v.0),
            ));
        }
    }
    let shown: Vec<_> = VIEWS
        .iter()
        .filter(|(name, ..)| !spec.views.get(*name).is_some_and(|v| v.omit))
        .collect();
    if shown.is_empty() {
        return Err(invalid(
            "Every view is omitted; a sheet needs at least one.",
            &[],
        ));
    }
    let size_of = |name: &str| {
        shown
            .iter()
            .find(|v| v.0 == name)
            .map(|(_, d, u)| extent(analysis, &View::new(*d, *u).expect("standard view")).0)
    };
    // Third-angle: plan above front, side to its right, iso above side.
    let none = [0.0, 0.0];
    let (front, plan_, side, iso) = (
        size_of("front").unwrap_or(none),
        size_of("plan").unwrap_or(none),
        size_of("side").unwrap_or(none),
        size_of("iso").unwrap_or(none),
    );
    let (left_w, right_w) = (front[0].max(plan_[0]), side[0].max(iso[0]));
    let (low_h, high_h) = (front[1].max(side[1]), plan_[1].max(iso[1]));
    let fits = |page: [f64; 2], s: f64| {
        let (w, h) = (s * (left_w + right_w), s * (low_h + high_h));
        let gaps = |a: f64, b: f64| if a > 0.0 && b > 0.0 { GAP } else { 0.0 };
        w + gaps(left_w, right_w) <= page[0] - 2.0 * (MARGIN + INSET)
            && h + gaps(low_h, high_h) <= page[1] - 2.0 * (MARGIN + INSET) - TITLE_BAND
    };

    let sheets: Vec<(&str, [f64; 2])> =
        match &spec.sheet.size {
            Some(s) => vec![*SHEETS.iter().find(|(n, _)| n == s).ok_or_else(|| {
                invalid(format!("{s:?} is not a sheet size."), &SHEETS.map(|s| s.0))
            })?],
            None => SHEETS.to_vec(),
        };
    let scales: Vec<(&str, f64)> = match &spec.sheet.scale {
        Some(s) => vec![*SCALES.iter().find(|(n, _)| n == s).ok_or_else(|| {
            invalid(
                format!("{s:?} is not a standard scale."),
                &SCALES.map(|s| s.0),
            )
        })?],
        None => SCALES.to_vec(),
    };
    // The smallest sheet, then the largest scale, that holds the views.
    let chosen = sheets.iter().find_map(|(name, page)| {
        scales
            .iter()
            .find(|(_, s)| fits(*page, *s))
            .map(|(scale, s)| (*name, *page, *scale, *s))
    });
    let (size, page, scale, s) = match chosen {
        Some(c) => c,
        None => {
            let (name, page) = *sheets.last().expect("one sheet at least");
            let (scale, s) = *scales.last().expect("one scale at least");
            problems.push(Problem {
                id: "p:views_do_not_fit:sheet".into(),
                code: "views_do_not_fit".into(),
                severity: "error".into(),
                message: format!("The views do not fit {name} at {scale}."),
                subjects: vec!["sheet".into()],
            });
            (name, page, scale, s)
        }
    };
    let basis = match (&spec.sheet.size, &spec.sheet.scale) {
        (None, None) => format!(
            "the smallest sheet, at the largest standard scale, that holds the views ({size} at {scale})"
        ),
        (Some(_), None) => format!("the largest standard scale at which the views fit {size}"),
        (None, Some(_)) => format!("the smallest sheet that holds the views at {scale}"),
        (Some(_), Some(_)) => "as the spec asks".into(),
    };

    // Columns and rows from the page's lower left, inside the border and above the title band.
    let (x0, y0) = (MARGIN + INSET, MARGIN + INSET + TITLE_BAND);
    let gap_x = if left_w > 0.0 && right_w > 0.0 {
        GAP
    } else {
        0.0
    };
    let gap_y = if low_h > 0.0 && high_h > 0.0 {
        GAP
    } else {
        0.0
    };
    let (cx_left, cx_right) = (
        x0 + s * left_w / 2.0,
        x0 + s * left_w + gap_x + s * right_w / 2.0,
    );
    let (cy_low, cy_high) = (
        y0 + s * low_h / 2.0,
        y0 + s * low_h + gap_y + s * high_h / 2.0,
    );
    let views = shown
        .iter()
        .map(|(name, direction, up)| {
            let (at, basis) = match *name {
                "front" => ([cx_left, cy_low], "the principal view, lower left"),
                "plan" => (
                    [cx_left, cy_high],
                    "third-angle: the plan sits above the front",
                ),
                "side" => (
                    [cx_right, cy_low],
                    "third-angle: the right side sits right of the front",
                ),
                _ => ([cx_right, cy_high], "the isometric view, upper right"),
            };
            PlannedView {
                id: format!("v:{name}"),
                direction: *direction,
                up: *up,
                at,
                default: !spec.views.contains_key(*name),
                basis: basis.into(),
            }
        })
        .collect();
    let mut title = spec.sheet.title.clone();
    title
        .entry("title".into())
        .or_insert_with(|| analysis.binding.part.to_uppercase());
    title
        .entry("number".into())
        .or_insert_with(|| "DWG-001".into());
    let mut spec = spec.clone();
    spec.schema = SPEC_SCHEMA.into();
    spec.binding = Some(analysis.binding.clone());
    Ok(Plan {
        schema: PLAN_SCHEMA.into(),
        binding: analysis.binding.clone(),
        sheet: PlannedSheet {
            size: size.into(),
            page,
            scale: scale.into(),
            default: spec.sheet.size.is_none() || spec.sheet.scale.is_none(),
            basis,
            title,
        },
        views,
        problems,
        spec,
    })
}

/// Applies edit operations to a spec. Each operation is `{"op": "set" | "remove", "path":
/// "a/b/c", "value": …}`, with paths into the spec's JSON. The result is validated whole, so an
/// unknown field or a value of the wrong kind is refused with the reason.
pub fn edit(spec: &Spec, ops: &serde_json::Value) -> Result<Spec, Invalid> {
    let mut doc = serde_json::to_value(spec).expect("a spec serialises");
    let ops = ops
        .as_array()
        .ok_or_else(|| invalid("Edits are a list of operations.", &[]))?;
    for op in ops {
        let kind = op["op"].as_str().unwrap_or("");
        let path: Vec<&str> = op["path"]
            .as_str()
            .ok_or_else(|| invalid("An edit needs a \"path\".", &[]))?
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let Some((last, parents)) = path.split_last() else {
            return Err(invalid("An edit's path is empty.", &[]));
        };
        let mut at = &mut doc;
        for key in parents {
            if !at.is_object() {
                return Err(invalid(format!("{key:?} is not inside an object."), &[]));
            }
            at = at
                .as_object_mut()
                .expect("checked")
                .entry(key.to_string())
                .or_insert_with(|| serde_json::json!({}));
        }
        let object = at
            .as_object_mut()
            .ok_or_else(|| invalid(format!("{:?} is not an object.", parents.join("/")), &[]))?;
        match kind {
            "set" => {
                object.insert(last.to_string(), op["value"].clone());
            }
            "remove" => {
                object.remove(*last);
            }
            other => {
                return Err(invalid(
                    format!("{other:?} is not an edit."),
                    &["set", "remove"],
                ));
            }
        }
    }
    serde_json::from_value(doc)
        .map_err(|e| invalid(format!("The edited spec is not valid: {e}."), &[]))
}

/// Draws the plan: each view projected with hidden lines, placed and scaled, and the title
/// block. The spec travels in the output.
pub fn render(part: &Part, analysis: &Analysis, plan: &Plan, font: &Font) -> Drawing {
    let s = SCALES
        .iter()
        .find(|(n, _)| *n == plan.sheet.scale)
        .map_or(1.0, |x| x.1);
    let mut layers: Vec<Layer> = layer_styles()
        .into_iter()
        .map(|(name, style)| Layer {
            name: name.into(),
            style,
            items: Vec::new(),
        })
        .collect();
    for v in &plan.views {
        let view = View::new(v.direction, v.up).expect("a planned view");
        let centre = extent(analysis, &view).1;
        let place = |q: [f64; 2]| {
            [
                v.at[0] + s * (q[0] - centre[0]),
                v.at[1] + s * (q[1] - centre[1]),
            ]
        };
        for piece in project(part, &view) {
            let mut moved = piece.clone();
            moved.points = piece.points.iter().map(|q| place(*q)).collect();
            moved.exact = piece.exact.as_ref().map(|e| placed(e, s, centre, v.at));
            layers[usize::from(!piece.visible)]
                .items
                .push(Item::Edge(projected(&moved)));
        }
    }
    // The border and the title block, lower right.
    let [w, h] = plan.sheet.page;
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
            .windows(2)
            .map(|p| Item::Edge(Curve::Line(p[0], p[1])))
            .collect::<Vec<_>>()
    };
    let part_layer = &mut layers[0].items;
    part_layer.extend(rect(MARGIN, MARGIN, w - MARGIN, h - MARGIN));
    let (tx0, tx1, ty0, ty1) = (w - MARGIN - 180.0, w - MARGIN, MARGIN, MARGIN + 30.0);
    part_layer.extend(rect(tx0, ty0, tx1, ty1));
    let mut text = Vec::new();
    let mut label = |s: &str, at: [f64; 2], size: f64, layer: &mut Vec<Item>| {
        let laid = font.text(s, size);
        let shift = |p: [f64; 2]| [p[0] + at[0], p[1] + at[1]];
        let curves: Vec<Curve> = laid
            .contours
            .iter()
            .flat_map(|c| c.iter())
            .map(|seg| match *seg {
                Segment::Line(a, b) => Curve::Line(shift(a), shift(b)),
                Segment::Quad(a, b, c) => Curve::Beziers(vec![vec![shift(a), shift(b), shift(c)]]),
                Segment::Cubic(a, b, c, d) => {
                    Curve::Beziers(vec![vec![shift(a), shift(b), shift(c), shift(d)]])
                }
            })
            .collect();
        if !curves.is_empty() {
            // One path of every contour: TrueType's winding makes the holes.
            let wires = laid
                .contours
                .iter()
                .scan(0usize, |start, c| {
                    let wire: Vec<Curve> = curves[*start..*start + c.len()].to_vec();
                    *start += c.len();
                    Some(wire)
                })
                .filter(|w| !w.is_empty())
                .collect();
            layer.push(Item::Face(wires));
        }
        text.push(TextRun {
            text: s.into(),
            at,
            size,
            rotation: 0.0,
            font: "IBMPlexMono-Regular.ttf".into(),
            h_align: HAlign::Center,
            v_align: VAlign::Middle,
        });
    };
    let dims = &mut layers[2].items;
    let title = |k: &str| plan.sheet.title.get(k).cloned().unwrap_or_default();
    label(&title("title"), [0.5 * (tx0 + tx1), ty1 - 10.0], 5.0, dims);
    label(
        &format!(
            "{}   SCALE {}   {}",
            title("number"),
            plan.sheet.scale,
            plan.sheet.size
        ),
        [0.5 * (tx0 + tx1), ty0 + 9.0],
        3.5,
        dims,
    );
    Drawing {
        page: plan.sheet.page,
        layers,
        link: None,
        text,
        spec: Some(serde_json::to_string(&plan.spec).expect("a spec serialises")),
    }
}

/// A projected curve scaled by *s* about *centre* and moved to *at*.
fn placed(e: &Exact, s: f64, centre: [f64; 2], at: [f64; 2]) -> Exact {
    let m = |q: [f64; 2]| {
        [
            at[0] + s * (q[0] - centre[0]),
            at[1] + s * (q[1] - centre[1]),
        ]
    };
    match e.clone() {
        Exact::Line(a, b) => Exact::Line(m(a), m(b)),
        Exact::Conic {
            centre: c,
            radii,
            x_axis,
            y_axis,
            t0,
            t1,
        } => Exact::Conic {
            centre: m(c),
            radii: [radii[0] * s, radii[1] * s],
            x_axis,
            y_axis,
            t0,
            t1,
        },
        Exact::Nurbs {
            degree,
            poles,
            weights,
            knots,
            t0,
            t1,
        } => Exact::Nurbs {
            degree,
            poles: poles.into_iter().map(m).collect(),
            weights,
            knots,
            t0,
            t1,
        },
    }
}

/// The spec embedded in a rendered SVG or PDF, so a drawing can be resumed from the file alone.
pub fn embedded_spec(file: &[u8]) -> Option<Spec> {
    let text = String::from_utf8_lossy(file);
    let json = if let Some(i) = text.find("<draftwright:spec") {
        let rest = &text[i..];
        let rest = &rest[rest.find("<![CDATA[")? + "<![CDATA[".len()..];
        let end = rest.find("</draftwright:spec>")?;
        // A "]]>" in the JSON was written split across two sections.
        rest[..end]
            .trim_end_matches("]]>")
            .replace("]]]]><![CDATA[>", "]]>")
    } else {
        let i = text.find("/DraftwrightSpec (")? + "/DraftwrightSpec (".len();
        crate::output::pdf_unescape(&text[i..])?
    };
    serde_json::from_str(&json).ok()
}
