//! The bridge from Python quiddity: its recognition JSON, written by `tools/recognise.py`, read
//! into the Analysis's features until Rust quiddity recognises them itself ([docs/model.md]).
//!
//! The JSON is proved to belong to the part before anything is taken from it: the same file
//! (sha256), the same faces in the same order (each face's surface type and area). Face indices
//! are haecceity's, which are the STEP file's own order; quiddity's working-shape indices are
//! mapped through its `caller_index`.
//!
//! Every record quiddity reports has one home, by its type ([`home`]): a feature, evidence (facts
//! about faces that are not features, such as face levels), or a refusal (quiddity saw something
//! and declined to call it a feature). A record type the bridge does not know is refused.
//!
//! A feature's ID is `f:<kind>:<n>`: its rank, from 1, among the features of its kind ordered by
//! their faces. Its fingerprint follows quiddity-rust's `docs/correspondence.md` until quiddity
//! issues its own: family and subtype, intrinsic sizes, axis, position in the part's frame, and
//! pattern count and pitch.

use std::collections::BTreeMap;

use haecceity::Part;
use haecceity::geom::Bounds;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const RECOGNITION_FORMAT: &str = "draftwright.recognition/1";
pub const FINGERPRINT_VERSION: &str = "draftwright-bridge/2";

/// Where the features came from, and the frame quiddity's coordinates are in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recognition {
    pub recogniser: String,
    /// quiddity's working frame in the part's: a point `l` in it is
    /// `origin + l[0] x + l[1] y + l[2] z`. Coordinates and axis letters in a feature's
    /// parameters are in this frame.
    pub frame: [[f64; 3]; 4],
    /// quiddity's evidence records: facts about faces, not features.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Record>,
    /// What quiddity declined to recognise, and why (in each record's parameters).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusals: Vec<Record>,
}

/// A quiddity record that is not a feature, ordered by family, subtype and faces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub family: String,
    pub subtype: String,
    /// The part's faces, ascending.
    pub faces: Vec<usize>,
    pub parameters: Map<String, Value>,
}

/// Where a quiddity record goes in the Analysis.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Home {
    Feature,
    Evidence,
    Refusal,
}

/// The home of each record type quiddity's recognition document can carry: its capability
/// manifest's `output` records are features, its `evidence` records evidence, and its section
/// recess refusals refusals. Anything else is refused until it is given a home here.
fn home(record_type: &str) -> Option<Home> {
    Some(match record_type {
        "AngledStep"
        | "Blend"
        | "BossRecord"
        | "Chamfer"
        | "CircularBlindStep"
        | "CircularFacePattern"
        | "CounterSink"
        | "DoubleDBore"
        | "Fillet"
        | "Flat"
        | "FreeformSurface"
        | "Groove"
        | "GussetRib"
        | "GussetRibArray"
        | "GussetRibMirrorPair"
        | "HoleRecord"
        | "InteriorVoid"
        | "ObliqueThroughStep"
        | "OrientedChamfer"
        | "OrientedSlot"
        | "OrientedSlotArray"
        | "OrientedSlotGrid"
        | "PairedRampStep"
        | "Plate"
        | "PolygonalBoss"
        | "PolygonalStock"
        | "RaisedPad"
        | "SectionRecess"
        | "SheetMetalBody"
        | "Slot"
        | "SlotArray"
        | "SlotGrid"
        | "ThinWallBody"
        | "ThroughStep"
        | "TurnedStep" => Home::Feature,
        "FaceLevel" | "HoleSpec" | "RepeatingRadialProfile" | "RiserEvidence" => Home::Evidence,
        "SectionRecessRefusal" => Home::Refusal,
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub id: String,
    pub kind: String,
    /// The part's faces, ascending.
    pub faces: Vec<usize>,
    /// quiddity's record, without its links to other records (those are by discovery order).
    pub parameters: Map<String, Value>,
    pub fingerprint: Fingerprint,
}

/// A revision-stable signature of a feature in the part's frame. Lengths are mm, angles degrees,
/// all rounded to 1e-6.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub version: String,
    pub family: String,
    pub subtype: String,
    /// Intrinsic sizes by name; a counterbore's diameter is `cbore.diameter`.
    pub sizes: BTreeMap<String, f64>,
    /// The axis or normal as quiddity gives it, as a line: its first non-zero component positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[f64; 3]>,
    /// The centre of the box around the feature's faces.
    pub position: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
    /// The feature's neighbourhood in the attributed adjacency graph (Joshi & Chang): for each of
    /// its faces and each face that face meets, `type>in|out:type:convex|concave|smooth|unknown`
    /// (`in` when the neighbour is the feature's own), sorted. Rigid motion leaves it unchanged, and
    /// it survives a feature moving or resizing (quiddity-rust docs/correspondence.md).
    pub neighbourhood: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    pub count: usize,
    /// The least distance between two members, mm.
    pub pitch: f64,
}

/// The record fields, and named dimensions, that are a feature's intrinsic sizes: the size names
/// the corpus's records use. A size under another name is not in the fingerprint (it is still in
/// the parameters).
const SIZES: [&str; 18] = [
    "across_flats",
    "angle",
    "axial_length",
    "corner_radius",
    "depth",
    "diameter",
    "end_radius",
    "half_width",
    "height",
    "height_above_base",
    "leg1",
    "leg2",
    "length",
    "major_radius",
    "minor_radius",
    "radius",
    "thickness",
    "width",
];

#[derive(Deserialize)]
struct Wrapper {
    format: String,
    source_sha256: String,
    recogniser: String,
    faces: Vec<FaceCheck>,
    quiddity: Document,
}

#[derive(Deserialize)]
struct FaceCheck {
    surface: String,
    area: f64,
}

#[derive(Deserialize)]
struct Document {
    frame: QFrame,
    faces: Vec<QFace>,
    features: Vec<QFeature>,
    derived: Derived,
}

#[derive(Deserialize)]
struct QFrame {
    origin: [f64; 3],
    x: [f64; 3],
    y: [f64; 3],
    z: [f64; 3],
}

#[derive(Deserialize)]
struct QFace {
    index: usize,
    caller_index: usize,
}

#[derive(Deserialize)]
struct QFeature {
    family: String,
    record_type: String,
    record: Map<String, Value>,
    constituent_faces: Vec<usize>,
    #[serde(default)]
    instances: Vec<QInstance>,
}

#[derive(Deserialize)]
struct QInstance {
    face_indices: Vec<usize>,
}

#[derive(Deserialize)]
struct Derived {
    #[serde(default)]
    hole_patterns: Vec<HolePattern>,
}

#[derive(Deserialize)]
struct HolePattern {
    holes: Vec<Map<String, Value>>,
}

/// Reads recognition JSON for *part*, the STEP file whose sha256 is *sha256*.
pub fn read(part: &Part, sha256: &str, json: &str) -> Result<(Recognition, Vec<Feature>), String> {
    let w: Wrapper = serde_json::from_str(json).map_err(|e| {
        format!("the recognition JSON is not valid ({e}); write it with tools/recognise.py")
    })?;
    if w.format != RECOGNITION_FORMAT {
        return Err(format!(
            "the recognition JSON is {:?}, not {RECOGNITION_FORMAT:?}; write it with tools/recognise.py",
            w.format
        ));
    }
    if w.source_sha256 != sha256 {
        return Err(
            "the recognition JSON was made from a different file; recognise this one".into(),
        );
    }
    check_faces(part, &w.faces)?;

    let doc = w.quiddity;
    let mut to_part = vec![None; doc.faces.len()];
    for f in &doc.faces {
        if f.index >= to_part.len() || f.caller_index >= part.faces.len() {
            return Err(format!("quiddity's face {} is out of range", f.index));
        }
        to_part[f.index] = Some(f.caller_index);
    }
    let map = |faces: &[usize]| -> Result<Vec<usize>, String> {
        let mut out = faces
            .iter()
            .map(|&i| {
                to_part
                    .get(i)
                    .copied()
                    .flatten()
                    .ok_or_else(|| format!("quiddity names face {i}, which it does not list"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.sort_unstable();
        Ok(out)
    };
    let f = &doc.frame;
    let frame = [f.origin, f.x, f.y, f.z];

    let mut features = Vec::new();
    let (mut evidence, mut refusals) = (Vec::new(), Vec::new());
    for q in &doc.features {
        let home = home(&q.record_type).ok_or_else(|| {
            format!(
                "quiddity reports a {} ({}), which the bridge has no home for",
                q.record_type, q.family
            )
        })?;
        let faces = map(&q.constituent_faces)?;
        let mut parameters = q.record.clone();
        parameters.remove("dependents");
        if home != Home::Feature {
            let record = Record {
                family: q.family.clone(),
                subtype: q.record_type.clone(),
                faces,
                parameters,
            };
            match home {
                Home::Evidence => evidence.push(record),
                _ => refusals.push(record),
            }
            continue;
        }
        if faces.is_empty() {
            return Err(format!("a {} has no faces to anchor it", q.family));
        }
        let pattern = if q.instances.len() > 1 {
            let centres = q
                .instances
                .iter()
                .map(|i| Ok(centre(part, &map(&i.face_indices)?)))
                .collect::<Result<Vec<_>, String>>()?;
            Some(pattern(&centres))
        } else {
            hole_pattern(&doc.derived.hole_patterns, &q.record)?
        };
        features.push(Feature {
            id: String::new(),
            kind: kind(&q.family),
            fingerprint: Fingerprint {
                version: FINGERPRINT_VERSION.into(),
                family: q.family.clone(),
                subtype: q.record_type.clone(),
                sizes: sizes(&q.record),
                axis: axis(&q.record, &frame)
                    .map_err(|e| format!("a {}'s axis {e}", q.record_type))?,
                position: centre(part, &faces).map(round),
                pattern,
                neighbourhood: neighbourhood(part, &faces),
            },
            faces,
            parameters,
        });
    }

    // IDs: by kind, then by faces. Two features of one kind on the same faces have no ID apart.
    features.sort_by(|a, b| (&a.kind, &a.faces).cmp(&(&b.kind, &b.faces)));
    for i in 1..features.len() {
        let (a, b) = (&features[i - 1], &features[i]);
        if a.kind == b.kind && a.faces == b.faces {
            return Err(format!(
                "quiddity reports two {}s on faces {:?}; they cannot be told apart",
                a.kind, a.faces
            ));
        }
    }
    let mut n = 0;
    for i in 0..features.len() {
        n = if i > 0 && features[i - 1].kind == features[i].kind {
            n + 1
        } else {
            1
        };
        features[i].id = format!("f:{}:{n}", features[i].kind);
    }
    // Records with no ID are ordered by everything they carry.
    let key = |r: &Record| {
        (
            r.family.clone(),
            r.subtype.clone(),
            r.faces.clone(),
            Value::from(r.parameters.clone()).to_string(),
        )
    };
    evidence.sort_by_cached_key(key);
    refusals.sort_by_cached_key(key);
    let recognition = Recognition {
        recogniser: format!("{} via tools/recognise.py", w.recogniser),
        frame,
        evidence,
        refusals,
    };
    Ok((recognition, features))
}

/// Proves that the JSON's faces are the part's: in number, and each in surface type (where both
/// readings name an analytic surface) and area. A face whose area haecceity cannot integrate
/// cannot be proved, so it is refused too. haecceity's areas agree with OpenCascade's
/// within 1e-4 relative on analytic faces and 1e-3 on B-splines across the corpus.
fn check_faces(part: &Part, faces: &[FaceCheck]) -> Result<(), String> {
    if faces.len() != part.faces.len() {
        return Err(format!(
            "the recognition JSON has {} faces and the part {}; recognise this file",
            faces.len(),
            part.faces.len()
        ));
    }
    let analytic = |s: &str| ["plane", "cylinder", "cone", "sphere", "torus"].contains(&s);
    for (i, check) in faces.iter().enumerate() {
        let kind = format!("{:?}", part.faces[i].surface.kind()).to_lowercase();
        let both = analytic(&check.surface) && analytic(&kind);
        if both && check.surface != kind {
            return Err(format!(
                "face {i} is a {kind} here and a {} in the recognition JSON",
                check.surface
            ));
        }
        let tol = if both { 1e-3 } else { 1e-2 };
        let Some([area, _]) = part.face_mass(i) else {
            return Err(format!(
                "face {i}'s area cannot be measured here, so it cannot be matched to the recognition JSON"
            ));
        };
        if (area - check.area).abs() > tol * check.area.max(1.0) {
            return Err(format!(
                "face {i} has area {area:.6} here and {:.6} in the recognition JSON",
                check.area
            ));
        }
    }
    Ok(())
}

/// A family's name for one of its features: `holes` → `hole`, `bosses` → `boss`.
fn kind(family: &str) -> String {
    if let Some(stem) = family.strip_suffix("sses") {
        format!("{stem}ss")
    } else {
        family.strip_suffix('s').unwrap_or(family).to_string()
    }
}

fn round(x: f64) -> f64 {
    let r = (x * 1e6).round() / 1e6;
    if r == 0.0 { 0.0 } else { r }
}

fn sizes(record: &Map<String, Value>) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let mut take = |prefix: &str, fields: &Map<String, Value>| {
        for (k, v) in fields {
            if let Some(x) = v.as_f64()
                && SIZES.contains(&k.as_str())
            {
                out.insert(format!("{prefix}{k}"), round(x));
            }
        }
    };
    take("", record);
    for (k, v) in record {
        if let Some(nested) = v.as_object() {
            let prefix = if k == "named_dimensions" {
                String::new()
            } else {
                format!("{k}.")
            };
            take(&prefix, nested);
        }
    }
    out
}

/// The record's `axis` in the part's frame; none if it has none, an error if it is unreadable.
fn axis(record: &Map<String, Value>, frame: &[[f64; 3]; 4]) -> Result<Option<[f64; 3]>, String> {
    let Some(value) = record.get("axis") else {
        return Ok(None);
    };
    let local = match value {
        Value::String(s) if s == "x" => [1.0, 0.0, 0.0],
        Value::String(s) if s == "y" => [0.0, 1.0, 0.0],
        Value::String(s) if s == "z" => [0.0, 0.0, 1.0],
        Value::Null => return Ok(None),
        v => point(v).ok_or_else(|| format!("{v} is not an axis"))?,
    };
    let mut d = [0, 1, 2].map(|k| (0..3).map(|j| local[j] * frame[j + 1][k]).sum::<f64>());
    if let Some(first) = d.iter().find(|c| c.abs() > 1e-9)
        && *first < 0.0
    {
        d = d.map(|c| -c);
    }
    Ok(Some(d.map(round)))
}

fn point(v: &Value) -> Option<[f64; 3]> {
    let c: Vec<f64> = v
        .as_array()?
        .iter()
        .map(Value::as_f64)
        .collect::<Option<_>>()?;
    <[f64; 3]>::try_from(c).ok()
}

/// The centre of the box around *faces*, in the part's frame.
/// The feature's attributed-adjacency neighbourhood: each of its faces, each face that one
/// meets, and how they meet, as sorted strings.
fn neighbourhood(part: &Part, faces: &[usize]) -> Vec<String> {
    use haecceity::brep::Arc;
    let kind = |f: usize| format!("{:?}", part.faces[f].surface.kind()).to_lowercase();
    let mut out: Vec<String> = faces
        .iter()
        .flat_map(|&f| {
            part.neighbours(f).into_iter().map(move |g| {
                let arc = match part.arc(f, g) {
                    Some(Arc::Convex) => "convex",
                    Some(Arc::Concave) => "concave",
                    Some(Arc::Smooth) => "smooth",
                    _ => "unknown",
                };
                let side = if faces.contains(&g) { "in" } else { "out" };
                format!("{}>{side}:{}:{arc}", kind(f), kind(g))
            })
        })
        .collect();
    out.sort();
    out
}

fn centre(part: &Part, faces: &[usize]) -> [f64; 3] {
    let mut b = Bounds::empty();
    for &f in faces {
        b.merge(&part.face_bounds(f));
    }
    b.centre()
}

fn pattern(members: &[[f64; 3]]) -> Pattern {
    let mut pitch = f64::INFINITY;
    for (i, a) in members.iter().enumerate() {
        for b in &members[i + 1..] {
            pitch = pitch.min(haecceity::geom::dist(*a, *b));
        }
    }
    Pattern {
        count: members.len(),
        pitch: round(pitch),
    }
}

/// The hole pattern a hole belongs to: quiddity lists its members' records.
fn hole_pattern(
    patterns: &[HolePattern],
    record: &Map<String, Value>,
) -> Result<Option<Pattern>, String> {
    let at = |r: &Map<String, Value>| (r.get("location").cloned(), r.get("diameter").cloned());
    let this = at(record);
    if this.0.is_none() {
        return Ok(None);
    }
    let Some(p) = patterns
        .iter()
        .find(|p| p.holes.iter().any(|h| at(h) == this))
    else {
        return Ok(None);
    };
    let members = p
        .holes
        .iter()
        .map(|h| {
            h.get("location")
                .and_then(point)
                .ok_or("a hole pattern has a member with no location")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(pattern(&members)))
}
