//! Assembly drawing packs. Identity is product first, then conservative, proper-rigid
//! geometric equality (see `docs/model.md`, Drawing packs). Pack intent never edits geometry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

mod buy;
mod keys;
pub use keys::part_keys;

use haecceity::Part;
use haecceity::cloud::FaceProbe;
use haecceity::geom;
use quiddity::correspondence::align::{Motion, align};
use quiddity::correspondence::fingerprint::{FaceFingerprint, Fingerprints, fingerprint};
use serde::{Deserialize, Serialize};

use crate::api::{Error, Files};
use crate::model::{self, Problem, Spec};
use crate::output::Drawing;
use crate::text::Font;

pub const SPEC_SCHEMA: &str = "draftwright.pack-spec/1";
pub const REPORT_SCHEMA: &str = "draftwright.pack/1";
pub const FORMATS: [&str; 3] = ["pdf", "csv", "json"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Supply {
    Make,
    Buy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartSpec {
    pub supply: Option<Supply>,
    pub item: Option<u32>,
    pub quantity: Option<usize>,
    /// Keep this product out of automatic geometry merges.
    pub separate: bool,
    pub drawing: Option<Spec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PackSpec {
    pub schema: String,
    pub parts: BTreeMap<String, PartSpec>,
}

impl Default for PackSpec {
    fn default() -> Self {
        Self {
            schema: SPEC_SCHEMA.into(),
            parts: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub products: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    /// Stable leaf or common-stem key; `products` retains every member key.
    pub key: String,
    pub item: u32,
    pub name: String,
    pub products: Vec<String>,
    pub part_indices: Vec<usize>,
    pub qty: usize,
    pub supply: Supply,
    pub buy_reasons: Vec<String>,
    /// One-based PDF page references, empty for bought or failed parts.
    pub sheets: Vec<usize>,
    pub problems: Vec<Problem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub rows: Vec<Row>,
    pub findings: Vec<Finding>,
    pub pages: usize,
}

/// A completed pack in memory. Drawings retain each part's own embedded drawing Spec.
/// An all-buy or all-failed pack has zero drawings and a valid zero-page PDF.
pub struct Pack {
    pub report: Report,
    pub drawings: Vec<Drawing>,
    pub timings: Timings,
}

/// Wall-clock seconds, kept outside the deterministic manufacturing report.
#[derive(Default, Debug, Serialize)]
pub struct Timings {
    pub read_seconds: f64,
    pub grouping_seconds: f64,
    pub parts: Vec<PartTiming>,
}

#[derive(Default, Debug, Serialize)]
pub struct PartTiming {
    pub key: String,
    pub analysis_seconds: f64,
    pub drawing_seconds: f64,
}

struct Group {
    row: Row,
    part: Result<Part, String>,
    identity: Option<Fingerprints>,
    separate: bool,
}

/// Absolute position/distance tolerance in mm. Intrinsic scalars use 1e-7 relative,
/// with a 1e-7 absolute floor. No reflected motion is ever considered.
const DISTANCE: f64 = 1e-5;

fn close(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-7 * a.abs().max(b.abs()).max(1.0)
}

fn face_equal(a: &FaceFingerprint, b: &FaceFingerprint, motion: &Motion) -> bool {
    a.surface == b.surface
        && a.adjacency == b.adjacency
        && a.parameters.len() == b.parameters.len()
        && a.parameters
            .iter()
            .all(|(k, x)| b.parameters.get(k).is_some_and(|y| close(*x, *y)))
        && match (a.area, b.area) {
            (Some(x), Some(y)) => close(x, y),
            _ => false,
        }
        && geom::dist(motion.point(a.centroid), b.centroid) <= DISTANCE
        && match (a.normal, b.normal) {
            (Some(x), Some(y)) => geom::dist(motion.dir(x), y) <= 1e-6,
            (None, None) => true,
            _ => false,
        }
}

fn equal_at(a: &Part, af: &Fingerprints, b: &Part, bf: &Fingerprints, m: &Motion) -> bool {
    let mut unused: BTreeSet<_> = (0..bf.faces.len()).collect();
    for face in &af.faces {
        let Some(j) = unused
            .iter()
            .copied()
            .find(|&j| face_equal(face, &bf.faces[j], m))
        else {
            return false;
        };
        // Face moments alone cannot distinguish equal boxes with different trims or freeform
        // surfaces. Test samples against exact trimmed faces in both directions as well.
        let probe = FaceProbe::new(b, j);
        if !a
            .face_cloud(face.index, 0.1)
            .iter()
            .all(|&p| probe.distance(m.point(p)) <= DISTANCE)
        {
            return false;
        }
        let probe = FaceProbe::new(a, face.index);
        if !b.face_cloud(j, 0.1).iter().all(|&p| {
            let q = geom::sub(p, m.t);
            let inverse = [0, 1, 2].map(|i| (0..3).map(|k| m.r[k][i] * q[k]).sum());
            probe.distance(inverse) <= DISTANCE
        }) {
            return false;
        }
        unused.remove(&j);
    }
    true
}

// An exact B-rep path also covers surfaces for which the kernel cannot integrate an
// area or invert a seam sample. Compare all defining geometry and trim topology, never
// entity IDs or cached samples. It is intentionally stricter than geometric equivalence.
fn same_surface(a: &geom::Surface, b: &geom::Surface) -> bool {
    use geom::Surface::*;
    match (a, b) {
        (Plane { frame: a }, Plane { frame: b }) => a == b,
        (
            Cylinder {
                frame: a,
                radius: ar,
            },
            Cylinder {
                frame: b,
                radius: br,
            },
        )
        | (
            Sphere {
                frame: a,
                radius: ar,
            },
            Sphere {
                frame: b,
                radius: br,
            },
        ) => a == b && ar == br,
        (
            Cone {
                frame: a,
                radius: ar,
                semi_angle: aa,
            },
            Cone {
                frame: b,
                radius: br,
                semi_angle: ba,
            },
        ) => a == b && ar == br && aa == ba,
        (
            Torus {
                frame: a,
                major: am,
                minor: an,
            },
            Torus {
                frame: b,
                major: bm,
                minor: bn,
            },
        ) => a == b && am == bm && an == bn,
        (
            Freeform {
                kind: ak,
                surface: a,
            },
            Freeform {
                kind: bk,
                surface: b,
            },
        ) => {
            ak == bk
                && a.degree_u == b.degree_u
                && a.degree_v == b.degree_v
                && a.control_points == b.control_points
                && a.weights == b.weights
                && a.knots_u == b.knots_u
                && a.knots_v == b.knots_v
        }
        _ => false,
    }
}

fn same_pcurve(a: &haecceity::brep::Pcurve, b: &haecceity::brep::Pcurve) -> bool {
    use haecceity::brep::Pcurve::*;
    match (a, b) {
        (Poles(a), Poles(b)) => a == b,
        (Line { point: a, dir: ad }, Line { point: b, dir: bd }) => a == b && ad == bd,
        _ => false,
    }
}

fn exact_brep(a: &Part, b: &Part) -> bool {
    a.faces.len() == b.faces.len()
        && a.edges.len() == b.edges.len()
        && a.solids.len() == b.solids.len()
        && !a.faces.is_empty()
        && a.unresolved_faces().is_empty()
        && b.unresolved_faces().is_empty()
        && a.unresolved_edges().is_empty()
        && b.unresolved_edges().is_empty()
        && a.solids
            .iter()
            .zip(&b.solids)
            .all(|(a, b)| a.faces == b.faces)
        && a.edges.iter().zip(&b.edges).all(|(a, b)| {
            a.curve == b.curve
                && a.start == b.start
                && a.end == b.end
                && a.vertices == b.vertices
                && a.same_sense == b.same_sense
        })
        && a.faces.iter().zip(&b.faces).all(|(a, b)| {
            a.reversed == b.reversed
                && a.solid == b.solid
                && same_surface(&a.surface, &b.surface)
                && a.loops.len() == b.loops.len()
                && a.loops
                    .iter()
                    .zip(&b.loops)
                    .all(|(a, b)| a.edges == b.edges && a.vertex == b.vertex)
                && a.pcurves.len() == b.pcurves.len()
                && a.pcurves
                    .iter()
                    .zip(&b.pcurves)
                    .all(|((ai, a), (bi, b))| ai == bi && same_pcurve(a, b))
        })
}

fn identical(a: &Part, af: &Fingerprints, b: &Part, bf: &Fingerprints) -> bool {
    if exact_brep(a, b) {
        return true;
    }
    if a.faces.len() != b.faces.len()
        || a.edges.len() != b.edges.len()
        || a.solids.len() != b.solids.len()
        || !close(af.scale, bf.scale)
        || af.faces.is_empty()
        || !a.unresolved_faces().is_empty()
        || !b.unresolved_faces().is_empty()
        || !a.unresolved_edges().is_empty()
        || !b.unresolved_edges().is_empty()
    {
        return false;
    }
    // Most exports retain the same product frame. Try it before a proper-rigid alignment.
    if equal_at(a, af, b, bf, &Motion::IDENTITY) {
        return true;
    }
    let (motion, alignment) = align(af, bf, &Default::default());
    alignment.found && equal_at(a, af, b, bf, &motion)
}

fn finding(code: &str, message: String, products: Vec<String>) -> Finding {
    Finding {
        code: code.into(),
        severity: "info".into(),
        message,
        products,
    }
}

struct Prepared {
    part: Part,
    analysis: model::Analysis,
    plan: model::Plan,
    projections: model::Projections,
}

/// Build a pack, reading the assembly definitions once and recognising/drawing each group
/// once. Invalid pack intent refuses the request; individual failures retain their rows.
pub fn build(step: &Path, spec: &PackSpec) -> Result<Pack, Error> {
    if spec.schema != SPEC_SCHEMA {
        return Err(Error::invalid(format!(
            "pack Spec schema must be {SPEC_SCHEMA}"
        )));
    }
    let mut timings = Timings::default();
    let started = Instant::now();
    let bytes = model::read_step_file(step).map_err(Error::invalid)?;
    let file = haecceity::step::StepFile::read(&bytes).map_err(|e| Error::failed(e.to_string()))?;
    timings.read_seconds = started.elapsed().as_secs_f64();
    let started = Instant::now();
    let keys = keys::keys(
        file.parts
            .iter()
            .map(|d| (d.name.as_str(), d.product_id.as_str())),
    );
    let mut groups: Vec<Group> = Vec::new();
    let mut findings = Vec::new();
    for (i, d) in file.parts.iter().enumerate() {
        let key = &keys[i];
        let separate = spec.parts.get(key).is_some_and(|p| p.separate);
        let part = file.part(i).map_err(|e| e.to_string());
        let mut identity = None;
        let merged = if separate {
            None
        } else {
            groups.iter_mut().position(|g| {
                if g.separate {
                    return false;
                }
                let (Ok(a), Ok(b)) = (&part, &g.part) else {
                    return false;
                };
                if a.faces.len() != b.faces.len()
                    || a.edges.len() != b.edges.len()
                    || a.solids.len() != b.solids.len()
                {
                    return false;
                }
                if exact_brep(a, b) {
                    return true;
                }
                // Fingerprint only genuinely different B-reps with compatible topology.
                // Re-exported twins take the exact path without expensive sampling.
                let af = identity.get_or_insert_with(|| fingerprint(a, &Default::default()));
                let bf = g
                    .identity
                    .get_or_insert_with(|| fingerprint(b, &Default::default()));
                identical(a, af, b, bf)
            })
        };
        let reason = buy::name_reason(&d.name);
        if let Some(j) = merged {
            let row = &mut groups[j].row;
            row.products.push(key.clone());
            row.part_indices.push(i);
            row.qty = row
                .qty
                .checked_add(d.placements.len())
                .ok_or_else(|| Error::invalid("pack quantity overflow"))?;
            if let Some(reason) = reason
                && !row.buy_reasons.contains(&reason)
            {
                row.buy_reasons.push(reason);
            }
            continue;
        }
        groups.push(Group {
            row: Row {
                key: key.clone(),
                item: 0,
                name: d.name.clone(),
                products: vec![key.clone()],
                part_indices: vec![i],
                qty: d.placements.len(),
                supply: Supply::Make,
                buy_reasons: reason.into_iter().collect(),
                sheets: Vec::new(),
                problems: Vec::new(),
                error: None,
            },
            part,
            identity,
            separate,
        });
    }
    let mut canonical = BTreeSet::new();
    for g in &mut groups {
        let row = &mut g.row;
        if let Some(stem) = keys::common_stem(
            row.part_indices
                .iter()
                .map(|&i| file.parts[i].name.as_str()),
        ) {
            row.name = stem.clone();
            // A stem must not steal another leaf's key, or another group's alias.
            if !keys
                .iter()
                .any(|key| key == &stem && !row.products.contains(key))
                && canonical.insert(stem.clone())
            {
                row.key = stem;
            }
        }
    }
    timings.grouping_seconds = started.elapsed().as_secs_f64();
    let valid: BTreeSet<_> = keys
        .iter()
        .chain(groups.iter().map(|g| &g.row.key))
        .cloned()
        .collect();
    for (key, intent) in &spec.parts {
        if !valid.contains(key) {
            return Err(Error::invalid(format!(
                "unknown pack part key {key:?}; choose from {}",
                valid.iter().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        if intent.item == Some(0) || intent.quantity == Some(0) {
            return Err(Error::invalid(format!(
                "{key}: item and quantity must be positive"
            )));
        }
        if intent.separate && !keys.contains(key) {
            return Err(Error::invalid(format!(
                "{key}: separate must address leaf keys from parts, not a merged-group stem"
            )));
        }
    }
    let mut reserved = BTreeSet::new();
    let mut rows = Vec::new();
    let mut prepared = Vec::new();
    for g in groups {
        let mut row = g.row;
        let mut timing = PartTiming {
            key: row.key.clone(),
            ..Default::default()
        };
        if row.products.len() > 1 {
            findings.push(finding("pack_geometry_merge", format!("identical geometry: {} (proper rigid motion, distance tolerance {DISTANCE} mm)", row.products.join(", ")), row.products.clone()));
        }
        let addresses: BTreeSet<_> = row
            .products
            .iter()
            .chain(std::iter::once(&row.key))
            .collect();
        let intents: Vec<_> = addresses
            .iter()
            .filter_map(|k| spec.parts.get(*k))
            .collect();
        macro_rules! resolve {
            ($field:ident) => {{
                let values: Vec<_> = intents.iter().filter_map(|p| p.$field.as_ref()).collect();
                if values.windows(2).any(|w| w[0] != w[1]) {
                    return Err(Error::invalid(format!(
                        "{}: conflicting {} overrides in merged group; use separate",
                        row.key,
                        stringify!($field)
                    )));
                }
                values.first().map(|v| (*v).clone())
            }};
        }
        let supply = resolve!(supply);
        if let Some(qty) = resolve!(quantity) {
            row.qty = qty;
        }
        if let Some(item) = resolve!(item) {
            if !reserved.insert(item) {
                return Err(Error::invalid(format!("duplicate pack item {item}")));
            }
            row.item = item;
        }
        let values: Vec<_> = intents.iter().filter_map(|p| p.drawing.as_ref()).collect();
        if values
            .windows(2)
            .any(|w| serde_json::to_value(w[0]).ok() != serde_json::to_value(w[1]).ok())
        {
            return Err(Error::invalid(format!(
                "{}: conflicting drawing overrides in merged group; use separate",
                row.key
            )));
        }
        let drawing_spec = values.first().map(|v| (*v).clone()).unwrap_or_default();
        if file.parts.len() == 1 && file.parts[0].placements.len() > 1 {
            findings.push(finding("pack_single_product", "one product with multiple placements; quantity counts placements, geometry is drawn once in its own frame".into(), row.products.clone()));
        }
        if let Ok(part) = &g.part
            && part.solids.len() > 1
        {
            findings.push(finding("pack_compound", format!("{} contains {} solids in one product; upstream haecceity must label/split it if these are separate parts", row.name, part.solids.len()), row.products.clone()));
        }
        let started = Instant::now();
        // Known bought names need no geometry analysis unless the Spec requests a drawing.
        // Unknown shapes share their ONE native recognition with subsequent drawing.
        let read_error = g.part.as_ref().err().cloned();
        let analysed = if row.buy_reasons.is_empty()
            || supply == Some(Supply::Make)
            || file.parts.len() == 1
        {
            Some((|| -> Result<(Part, model::Analysis), Error> {
                let (part, mut analysis) =
                    if file.parts.len() == 1 && file.parts[0].placements.len() == 1 {
                        model::analyse_step_part(step, &bytes, Some(0))?
                    } else {
                        model::analyse_pack_part(
                            &step.display().to_string(),
                            &bytes,
                            g.part.map_err(Error::failed)?,
                            &file.parts,
                            row.part_indices[0],
                        )?
                    };
                let recognition = quiddity::correspondence::recognise(&part);
                let doc = quiddity::recognition::document(&recognition, part.faces.len());
                if row.buy_reasons.is_empty()
                    && !row
                        .part_indices
                        .iter()
                        .any(|&i| buy::custom(&file.parts[i].name))
                    && let Some(reason) = buy::shape_reason(&part, &doc)
                {
                    row.buy_reasons.push(reason);
                }
                model::recognise_document(&part, &mut analysis, &doc).map_err(Error::failed)?;
                Ok((part, analysis))
            })())
        } else {
            None
        };
        timing.analysis_seconds = started.elapsed().as_secs_f64();
        let guessed = if !row.buy_reasons.is_empty() && file.parts.len() > 1 {
            Supply::Buy
        } else {
            Supply::Make
        };
        row.supply = supply.unwrap_or(guessed);
        if !row.buy_reasons.is_empty() {
            findings.push(finding(
                "pack_buy_guess",
                format!(
                    "{}: guessed buy because {}; supply override wins",
                    row.name,
                    row.buy_reasons.join("; ")
                ),
                row.products.clone(),
            ));
        } else {
            findings.push(finding("pack_make_guess", format!("{}: guessed make; custom drawing identifier or no bounded purchased-part name/shape evidence (supply override wins)", row.name), row.products.clone()));
        }
        let started = Instant::now();
        let planned = if row.supply == Supply::Make {
            match analysed
                .expect("made groups analysed")
                .and_then(|(part, analysis)| {
                    let plan = crate::api::plan_analysed(&analysis, &drawing_spec)?;
                    Ok(Prepared {
                        part,
                        analysis,
                        plan,
                        projections: Default::default(),
                    })
                }) {
                Ok(p) => Some(p),
                Err(error) => {
                    row.error = Some(error);
                    None
                }
            }
        } else {
            if let Some(error) = read_error {
                row.error = Some(Error::failed(error));
            }
            None
        };
        timing.drawing_seconds = started.elapsed().as_secs_f64();
        rows.push((
            row,
            planned,
            timing,
            drawing_spec.sheet.title.contains_key("title"),
        ));
    }
    rows.sort_by_key(|(row, ..)| row.supply == Supply::Buy);
    let mut next = 1u32;
    for (row, ..) in &mut rows {
        if row.item == 0 {
            while reserved.contains(&next) {
                next = next
                    .checked_add(1)
                    .ok_or_else(|| Error::invalid("pack item numbers exhausted"))?;
            }
            row.item = next;
            reserved.insert(next);
        }
    }
    let font = Font::new(crate::api::MONO.to_vec()).expect("bundled font");
    // Final sheet totals exclude failed parts. Rendering normally happens once; if a
    // renderer fails, retain its row and renumber survivors using their cached projections.
    let drawings = loop {
        let total: usize = rows
            .iter()
            .filter_map(|(_, p, ..)| p.as_ref())
            .map(|p| 1 + p.plan.sheets.len())
            .sum();
        let mut drawings = Vec::new();
        let mut failed = false;
        for (row, p, timing, explicit_title) in &mut rows {
            row.sheets.clear();
            let Some(p) = p else {
                continue;
            };
            let started = Instant::now();
            for (i, sheet) in std::iter::once(&mut p.plan.sheet)
                .chain(p.plan.sheets.iter_mut())
                .enumerate()
            {
                if file.parts.len() > 1 && !*explicit_title {
                    sheet.title.insert("title".into(), row.name.clone());
                }
                sheet.title.insert(
                    "sheet".into(),
                    format!("{}/{}", drawings.len() + i + 1, total),
                );
                crate::title_block::pack_cells(sheet, row.item, row.qty);
            }
            match model::render_sheets_with(
                &p.part,
                &p.analysis,
                &p.plan,
                &font,
                &mut p.projections,
            ) {
                Ok(sheets) => {
                    row.sheets = (drawings.len() + 1..=drawings.len() + sheets.len()).collect();
                    drawings.extend(sheets);
                }
                Err(error) => {
                    row.error = Some(Error::failed(error.to_string()));
                    failed = true;
                }
            }
            timing.drawing_seconds += started.elapsed().as_secs_f64();
        }
        if !failed {
            break drawings;
        }
        for (row, p, ..) in &mut rows {
            if row.error.is_some() {
                *p = None;
            }
        }
    };
    for (mut row, p, timing, _) in rows {
        if let Some(p) = p {
            row.problems.extend(p.plan.problems);
            for sheet in std::iter::once(&p.plan.sheet).chain(p.plan.sheets.iter()) {
                row.problems
                    .extend(crate::title_block::problems(&sheet.title_cells, sheet.page));
            }
            row.problems.extend(
                row.sheets
                    .iter()
                    .flat_map(|&i| drawings[i - 1].problems.iter().cloned()),
            );
        }
        timings.parts.push(timing);
        prepared.push(row);
    }
    Ok(Pack {
        report: Report {
            schema: REPORT_SCHEMA.into(),
            rows: prepared,
            findings,
            pages: drawings.len(),
        },
        drawings,
        timings,
    })
}

impl Report {
    /// RFC 4180 CSV, with semicolon-separated product keys, reasons and page references.
    pub fn csv(&self) -> String {
        fn quote(s: &str) -> String {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
        let mut out = "item,name,products,qty,supply,buy_reasons,sheets\r\n".to_string();
        for row in &self.rows {
            let fields = [
                row.item.to_string(),
                row.name.clone(),
                row.products.join("; "),
                row.qty.to_string(),
                if row.supply == Supply::Make {
                    "make"
                } else {
                    "buy"
                }
                .into(),
                row.buy_reasons.join("; "),
                row.sheets
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            ];
            out.push_str(
                &fields
                    .iter()
                    .map(|s| quote(s))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push_str("\r\n");
        }
        out
    }
}

/// Build once and encode the requested outputs. Files are named `pack.pdf`, `pack.csv`,
/// `pack.json` when written using `api::write(&directory.join("pack"), &files)`.
pub fn render(step: &Path, spec: &PackSpec, formats: &[&str]) -> Result<(Report, Files), Error> {
    let (report, files, _) = render_profiled(step, spec, formats)?;
    Ok((report, files))
}

/// Render with stage and per-group wall-clock seconds, separate from the stable report.
pub fn render_profiled(
    step: &Path,
    spec: &PackSpec,
    formats: &[&str],
) -> Result<(Report, Files, Timings), Error> {
    if formats.is_empty() || formats.iter().any(|f| !FORMATS.contains(f)) {
        return Err(Error::invalid("pack formats must be pdf,csv,json"));
    }
    let pack = build(step, spec)?;
    let mut files = Vec::new();
    for &format in formats {
        let bytes = match format {
            "pdf" => Drawing::pdf_pages(&pack.drawings, &|name| match name {
                "IBMPlexMono-Regular.ttf" => Some(crate::api::MONO.to_vec()),
                crate::title_block::FONT => Some(crate::title_block::FONT_DATA.to_vec()),
                _ => None,
            }),
            "csv" => pack.report.csv().into_bytes(),
            _ => serde_json::to_vec_pretty(&pack.report).expect("report serialises"),
        };
        files.push((format.into(), bytes));
    }
    Ok((pack.report, files, pack.timings))
}
