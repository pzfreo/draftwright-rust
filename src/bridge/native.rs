//! Holes and hole patterns from Rust quiddity directly: its versioned recognition document
//! (`quiddity::recognition`), run on the part draftwright has already read, in place of Python
//! quiddity's JSON ([`super::read`]), family by family. Holes and hole patterns are the first.
//!
//! The document is read fail-closed (ADR3-4): one schema version ([`SCHEMA_VERSION`]), the STEP
//! file's coordinates, the part's face count, and only the families and record types that schema
//! has ([`FAMILIES`]); anything else is refused, not guessed at. Families other than holes and
//! hole patterns are known but not taken here: they still come through the bridge.
//!
//! What comes out is draftwright's own ([`Feature`], [`HolePattern`]); no quiddity object crosses
//! (ADR3-6). IDs are draftwright's (`f:hole:<n>`, `f:hole_pattern:<n>`, ranked by faces), and so
//! are fingerprints ([`super::FINGERPRINT_VERSION`]): the document's own `<family>/<index>` IDs
//! and quiddity's fingerprints are not adopted (who owns fingerprints is an open question).
//!
//! Coordinates: the document's are the STEP file's, and its faces the file's own indices, so the
//! records need no mapping (the Python bridge maps a working frame and `caller_index`). The
//! records' parameters are in the identity frame ([`IDENTITY`]). The Analysis has one
//! recognition frame (`Recognition::frame`), so holding these beside the bridge's features,
//! which are in Python's working frame, needs one of the two mapped into the other's.
//!
//! A hole's faces are the document's: the faces that define it (its cylinders, a counterbore's
//! or spotface's included). Python's bridge gives a hole its constituent faces, which add the
//! face that closes it (a drill point's cone, a blind hole's floor); Rust quiddity finds that
//! face too, but its document does not carry it (`tests/native.rs` lists every such
//! difference).

use haecceity::Part;
use quiddity::recognition::RecognitionDocument;
use serde_json::{Map, Value};

use super::{FINGERPRINT_VERSION, Feature, Fingerprint, HolePattern, QHolePattern};

/// The one document schema this reader knows.
pub const SCHEMA_VERSION: &str = "quiddity-rust/recognition/1";

/// The identity frame: the document's coordinates are the part's.
pub const IDENTITY: [[f64; 3]; 4] = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Every family of [`SCHEMA_VERSION`] and the record types its records can have, as
/// `quiddity::recognition::RECORD_TYPES` lists them for that schema (`tests/native.rs` checks the
/// two agree). Holes and hole patterns are read here; the rest are known and left to the bridge.
pub const FAMILIES: &[(&str, &[&str])] = &[
    ("fillets", &["Fillet"]),
    ("chamfers", &["Chamfer"]),
    ("bosses", &["BossRecord"]),
    ("angled_steps", &["AngledStep"]),
    ("flats", &["Flat"]),
    ("paired_ramp_steps", &["PairedRampStep"]),
    ("oriented_chamfers", &["OrientedChamfer"]),
    ("circular_face_patterns", &["CircularFacePattern"]),
    ("oblique_through_steps", &["ObliqueThroughStep"]),
    ("circular_blind_steps", &["CircularBlindStep"]),
    ("holes", &["HoleRecord"]),
    ("countersinks", &["CounterSink"]),
    (
        "hole_patterns",
        &[
            "RectGrid",
            "RectangularHoleSet",
            "BoltCircle",
            "LinearArray",
        ],
    ),
    ("gusset_ribs", &["GussetRib"]),
    (
        "gusset_rib_patterns",
        &["GussetRibArray", "GussetRibMirrorPair"],
    ),
    ("thin_wall_bodies", &["ThinWallBody"]),
    ("interior_voids", &["InteriorVoid"]),
    ("through_steps", &["ThroughStep"]),
    ("turned_steps", &["TurnedStep"]),
    ("grooves", &["Groove"]),
    ("plates", &["Plate"]),
    ("round_bottom_blind_slots", &["RoundBottomBlindSlot"]),
    ("rectangular_blind_slots", &["RectangularBlindSlot"]),
    ("double_d_bores", &["DoubleDBore"]),
    ("edge_open_circular_pockets", &["EdgeOpenCircularPocket"]),
    ("edge_open_prismatic_recesses", &["EdgeOpenPrismaticRecess"]),
    ("blends", &["Blend"]),
];

/// The families read here.
pub const READ: [&str; 2] = ["holes", "hole_patterns"];

/// Holes and hole patterns read from Rust quiddity.
#[derive(Clone, Debug, PartialEq)]
pub struct Native {
    /// `quiddity-rust <version>` and the schema it was read at.
    pub recogniser: String,
    /// The hole features, by ID; their parameters are in the part's frame ([`IDENTITY`]).
    pub holes: Vec<Feature>,
    /// The hole patterns, by ID; members name [`Native::holes`].
    pub patterns: Vec<HolePattern>,
}

/// Runs Rust quiddity's recognition on *part* and reads its holes and hole patterns.
pub fn recognise(part: &Part) -> Result<Native, String> {
    let recognition = quiddity::correspondence::recognise(part);
    read(
        part,
        &quiddity::recognition::document(&recognition, part.faces.len()),
    )
}

/// Reads the holes and hole patterns of *doc*, Rust quiddity's recognition of *part*.
pub fn read(part: &Part, doc: &RecognitionDocument) -> Result<Native, String> {
    if doc.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "quiddity's recognition document is {:?}, not {SCHEMA_VERSION:?}",
            doc.schema_version
        ));
    }
    if doc.coordinate_space != "file" {
        return Err(format!(
            "quiddity's recognition document is in {:?} coordinates, not the file's",
            doc.coordinate_space
        ));
    }
    if doc.face_count != part.faces.len() {
        return Err(format!(
            "quiddity's recognition document has {} faces and the part {}",
            doc.face_count,
            part.faces.len()
        ));
    }
    let mut holes = Vec::new();
    let mut patterns = Vec::new();
    for r in &doc.records {
        let known = FAMILIES
            .iter()
            .find(|(f, _)| *f == r.family)
            .ok_or_else(|| {
                format!(
                    "quiddity reports a family {:?} this reader does not know",
                    r.family
                )
            })?;
        if !known.1.contains(&r.record_type.as_str()) {
            return Err(format!(
                "quiddity reports a {} ({}), a record type this reader does not know",
                r.record_type, r.family
            ));
        }
        if let Some(&f) = r.faces.iter().find(|&&f| f >= part.faces.len()) {
            return Err(format!(
                "{} names face {f}, which the part does not have",
                r.id
            ));
        }
        match r.family.as_str() {
            "holes" => holes.push(r),
            "hole_patterns" => patterns.push(r),
            _ => {}
        }
    }

    let qs = patterns
        .iter()
        .map(|r| {
            let mut rest = r.parameters.clone();
            let members = match rest.remove("holes") {
                Some(Value::Array(members)) => members
                    .into_iter()
                    .map(|m| match m {
                        Value::Object(m) => Ok(m),
                        _ => Err(format!("{}'s member is not a hole record", r.id)),
                    })
                    .collect::<Result<Vec<Map<String, Value>>, String>>()?,
                _ => return Err(format!("{} has no member holes", r.id)),
            };
            Ok(QHolePattern {
                holes: members,
                rest,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut features = Vec::new();
    for r in holes {
        let faces = r.faces.clone();
        if faces.is_empty() || !faces.is_sorted() {
            return Err(format!("{} has no ascending faces to anchor it", r.id));
        }
        features.push(Feature {
            id: String::new(),
            kind: "hole".into(),
            fingerprint: Fingerprint {
                version: FINGERPRINT_VERSION.into(),
                family: r.family.clone(),
                subtype: r.record_type.clone(),
                sizes: super::sizes(&r.parameters),
                axis: super::axis(&r.parameters, &IDENTITY)
                    .map_err(|e| format!("{}'s axis {e}", r.id))?,
                position: super::centre(part, &faces).map(super::round),
                pattern: super::hole_pattern(&qs, &r.parameters)?,
                neighbourhood: super::neighbourhood(part, &faces),
            },
            faces,
            parameters: r.parameters.clone(),
        });
    }
    features.sort_by(|a, b| a.faces.cmp(&b.faces));
    if let Some(w) = features.windows(2).find(|w| w[0].faces == w[1].faces) {
        return Err(format!(
            "quiddity reports two holes on faces {:?}; they cannot be told apart",
            w[0].faces
        ));
    }
    for (n, f) in features.iter_mut().enumerate() {
        f.id = format!("f:hole:{}", n + 1);
    }
    let patterns = super::hole_patterns(&qs, &features)?;
    Ok(Native {
        recogniser: format!(
            "{} {} ({})",
            doc.package.name, doc.package.version, doc.schema_version
        ),
        holes: features,
        patterns,
    })
}
