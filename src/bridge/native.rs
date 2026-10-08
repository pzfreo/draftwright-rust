//! Features from Rust quiddity directly: its versioned recognition document
//! (`quiddity::recognition`), run on the part draftwright has already read, in place of Python
//! quiddity's JSON ([`super::read`]), family by family ([`READ`]): holes and hole patterns, then
//! fillets, chamfers, countersinks, bosses and turned steps, then grooves, flats, the steps
//! (angled, paired-ramp, circular blind, through and oblique through) and blends. Face levels
//! and risers are evidence Rust quiddity finds (`features::levels`) but its document does not
//! carry, so they still come through the bridge.
//!
//! The document is read fail-closed (ADR3-4): one schema version ([`SCHEMA_VERSION`]), the STEP
//! file's coordinates, the part's face count, and only the families and record types that schema
//! has ([`FAMILIES`]); anything else is refused, not guessed at. A record of a family read here
//! must carry every field draftwright reads of it ([`FIELDS`]), or it is refused. Families not in
//! [`READ`] are known but not taken here: they still come through the bridge.
//!
//! What comes out is draftwright's own ([`Feature`], [`HolePattern`]); no quiddity object crosses
//! (ADR3-6). IDs are draftwright's (`f:<kind>:<n>`, `f:hole_pattern:<n>`, ranked by faces within
//! their kind, as [`super::read`] ranks them), and so are fingerprints
//! ([`super::FINGERPRINT_VERSION`]): the document's own `<family>/<index>` IDs and quiddity's
//! fingerprints are not adopted (who owns fingerprints is an open question).
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
//! difference). The other families' faces are the document's defining faces too, which for a
//! boss leave out the face at its free end, and for an angled step the flat that closes it, that
//! Python's constituent faces add. The document carries no `named_dimensions` (Python's
//! enrichment), so the fingerprint sizes Python's bridge takes from them (a boss's
//! `axial_length` and `height_above_base`, a turned step's `axial_length`) are not in a native
//! fingerprint. Rust quiddity's families are independent: Python's aggregate rejects a boss
//! whose faces are a turned step's, a blend whose faces are fillets', and a chamfer on an angled
//! step's slant; Rust's document carries both, and the reader takes what the document says
//! (`tests/native.rs` lists each).

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
/// two agree). The families in [`READ`] are read here; the rest are known and left to the bridge.
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
    ("sheet_metal_bodies", &["SheetMetalBody"]),
];

/// The families read here.
pub const READ: [&str; 15] = [
    "holes",
    "hole_patterns",
    "fillets",
    "chamfers",
    "countersinks",
    "bosses",
    "turned_steps",
    "grooves",
    "flats",
    "angled_steps",
    "paired_ramp_steps",
    "circular_blind_steps",
    "through_steps",
    "oblique_through_steps",
    "blends",
];

/// What a field of a record must be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Field {
    Number,
    Bool,
    Text,
    /// Three numbers.
    Point,
    /// Two numbers.
    Pair,
    /// This many points of two numbers (a section across a run).
    Section(usize),
    /// This many points of three numbers.
    Points(usize),
    /// An axis letter (`x`, `y`, `z`) or three numbers.
    Axis,
    /// An axis letter (`x`, `y`, `z`).
    Letter,
    /// Numbers, or null.
    NumbersOrNull,
    /// An object, or null.
    ObjectOrNull,
    /// A blend's rolling path: straight (`at` and `direction`, three numbers each) or circular
    /// (`center` and `normal`, three numbers each, and a `radius`).
    BlendPath,
}

/// The fields draftwright reads of each record type it takes besides holes and hole patterns
/// (dims, lint, demand, and the fingerprint's axis and sizes), as Python's records carry them: a
/// record without one, or with one of another shape, is refused rather than filled in. A field
/// nothing reads (a fillet's `side`, a chamfer's `corner`) is not required, so a change to it does
/// not refuse a read that would not use it. `a.b` is field `b` of object field `a`, checked when
/// `a` is not null (`a` is listed first).
pub const FIELDS: &[(&str, &[(&str, Field)])] = &[
    (
        "Fillet",
        &[
            ("axis", Field::Axis),
            ("radius", Field::Number),
            ("at", Field::Point),
            ("turned", Field::Bool),
        ],
    ),
    (
        "Chamfer",
        &[
            ("axis", Field::Axis),
            ("leg1", Field::Number),
            ("leg2", Field::Number),
            ("angle", Field::Number),
            ("at", Field::Point),
            ("turned", Field::Bool),
        ],
    ),
    (
        "CounterSink",
        &[
            ("axis", Field::Axis),
            ("location", Field::Point),
            ("major_diameter", Field::Number),
            ("drill_diameter", Field::Number),
            ("included_angle", Field::Number),
            ("depth", Field::Number),
        ],
    ),
    (
        "BossRecord",
        &[
            ("axis", Field::Axis),
            ("location", Field::Point),
            ("diameter", Field::Number),
            ("height", Field::Number),
        ],
    ),
    (
        "TurnedStep",
        &[
            ("axis", Field::Axis),
            ("lo", Field::Number),
            ("hi", Field::Number),
            ("diameter", Field::Number),
            ("profile", Field::ObjectOrNull),
            ("profile.axis", Field::Letter),
            ("profile.axis_origin", Field::Point),
            ("profile.body_key", Field::NumbersOrNull),
        ],
    ),
    (
        "Groove",
        &[
            ("axis", Field::Letter),
            ("width", Field::Number),
            ("diameter", Field::Number),
            ("at", Field::Point),
            ("profile", Field::ObjectOrNull),
        ],
    ),
    (
        "Flat",
        &[
            ("axis", Field::Letter),
            ("axis_direction", Field::Point),
            ("axis_line", Field::Pair),
            ("stock_span", Field::Pair),
            ("across", Field::Number),
            ("at", Field::Point),
        ],
    ),
    (
        "AngledStep",
        &[
            ("axis", Field::Axis),
            ("leg1", Field::Number),
            ("leg2", Field::Number),
            ("angle", Field::Number),
            ("length", Field::Number),
            ("at", Field::Point),
        ],
    ),
    (
        "PairedRampStep",
        &[
            ("axis", Field::Axis),
            ("angle", Field::Number),
            ("length", Field::Number),
            ("half_width", Field::Number),
            ("at", Field::Point),
        ],
    ),
    (
        "CircularBlindStep",
        &[
            ("axis", Field::Axis),
            ("radius", Field::Number),
            ("length", Field::Number),
            ("centreline", Field::Points(2)),
        ],
    ),
    (
        "ThroughStep",
        &[
            ("axis", Field::Letter),
            ("length", Field::Number),
            ("section", Field::Section(3)),
            ("at", Field::Point),
        ],
    ),
    (
        "ObliqueThroughStep",
        &[("length", Field::Number), ("depth", Field::Number)],
    ),
    (
        "Blend",
        &[
            ("radius", Field::Number),
            ("side", Field::Text),
            ("path", Field::BlendPath),
        ],
    ),
];

/// Features read from Rust quiddity.
#[derive(Clone, Debug, PartialEq)]
pub struct Native {
    /// `quiddity-rust <version>` and the schema it was read at.
    pub recogniser: String,
    /// The hole features, by ID; their parameters are in the part's frame ([`IDENTITY`]).
    pub holes: Vec<Feature>,
    /// The hole patterns, by ID; members name [`Native::holes`].
    pub patterns: Vec<HolePattern>,
    /// The other families' features, by kind and then ID; their parameters are in the part's
    /// frame ([`IDENTITY`]).
    pub features: Vec<Feature>,
}

/// Runs Rust quiddity's recognition on *part* and reads the families of [`READ`].
pub fn recognise(part: &Part) -> Result<Native, String> {
    let recognition = quiddity::correspondence::recognise(part);
    read(
        part,
        &quiddity::recognition::document(&recognition, part.faces.len()),
    )
}

/// Reads the families of [`READ`] from *doc*, Rust quiddity's recognition of *part*.
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
    let mut others = Vec::new();
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
            f if READ.contains(&f) => others.push(r),
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
        if !ascending(&faces) {
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

    let mut others = others
        .into_iter()
        .map(|r| feature(part, &qs, r))
        .collect::<Result<Vec<_>, String>>()?;
    others.sort_by(|a, b| (&a.kind, &a.faces).cmp(&(&b.kind, &b.faces)));
    if let Some(w) = others
        .windows(2)
        .find(|w| w[0].kind == w[1].kind && w[0].faces == w[1].faces)
    {
        return Err(format!(
            "quiddity reports two {}s on faces {:?}; they cannot be told apart",
            w[0].kind, w[0].faces
        ));
    }
    let mut n = 0;
    for i in 0..others.len() {
        n = if i > 0 && others[i - 1].kind == others[i].kind {
            n + 1
        } else {
            1
        };
        others[i].id = format!("f:{}:{n}", others[i].kind);
    }
    Ok(Native {
        recogniser: format!(
            "{} {} ({})",
            doc.package.name, doc.package.version, doc.schema_version
        ),
        holes: features,
        patterns,
        features: others,
    })
}

/// A feature of one of the families after holes and hole patterns in [`READ`], without its ID:
/// its record checked against [`FIELDS`], fingerprinted as [`super::read`] fingerprints it.
fn feature(
    part: &Part,
    qs: &[QHolePattern],
    r: &quiddity::recognition::Record,
) -> Result<Feature, String> {
    let fields = FIELDS
        .iter()
        .find(|(t, _)| *t == r.record_type)
        .ok_or_else(|| format!("{} ({}) has no fields known here", r.id, r.record_type))?
        .1;
    for &(name, field) in fields {
        let v = match name.split_once('.') {
            Some((outer, inner)) => match r.parameters.get(outer) {
                Some(Value::Null) => continue,
                o => o.and_then(|o| o.get(inner)),
            },
            None => r.parameters.get(name),
        };
        let v = v.ok_or_else(|| {
            format!(
                "{} ({}) has no {name}, which draftwright reads; the {} are not read",
                r.id, r.record_type, r.family
            )
        })?;
        let numbers = |v: &Value, n: usize| {
            v.as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(Value::is_number))
        };
        let points = |v: &Value, n: usize, d: usize| {
            v.as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(|p| numbers(p, d)))
        };
        let letter = |v: &Value| ["x", "y", "z"].contains(&v.as_str().unwrap_or(""));
        let point = |k: &str| v.get(k).is_some_and(|p| numbers(p, 3));
        let ok = match field {
            Field::Number => v.is_number(),
            Field::Bool => v.is_boolean(),
            Field::Text => v.is_string(),
            Field::Point => numbers(v, 3),
            Field::Pair => numbers(v, 2),
            Field::Section(n) => points(v, n, 2),
            Field::Points(n) => points(v, n, 3),
            Field::Axis => numbers(v, 3) || letter(v),
            Field::Letter => letter(v),
            Field::NumbersOrNull => {
                v.is_null() || v.as_array().is_some_and(|a| a.iter().all(Value::is_number))
            }
            Field::ObjectOrNull => v.is_null() || v.is_object(),
            Field::BlendPath => v.as_object().is_some_and(|o| {
                let straight = o.len() == 2 && point("at") && point("direction");
                let circular = o.len() == 3
                    && point("center")
                    && point("normal")
                    && o.get("radius").is_some_and(Value::is_number);
                straight || circular
            }),
        };
        if !ok {
            return Err(format!(
                "{} ({})'s {name} is {v}, not {field:?}; the {} are not read",
                r.id, r.record_type, r.family
            ));
        }
    }
    let faces = r.faces.clone();
    if !ascending(&faces) {
        return Err(format!("{} has no ascending faces to anchor it", r.id));
    }
    Ok(Feature {
        id: String::new(),
        kind: super::kind(&r.family),
        fingerprint: Fingerprint {
            version: FINGERPRINT_VERSION.into(),
            family: r.family.clone(),
            subtype: r.record_type.clone(),
            sizes: super::sizes(&r.parameters),
            axis: super::axis(&r.parameters, &IDENTITY)
                .map_err(|e| format!("{}'s axis {e}", r.id))?,
            position: super::centre(part, &faces).map(super::round),
            pattern: super::hole_pattern(qs, &r.parameters)?,
            neighbourhood: super::neighbourhood(part, &faces),
        },
        faces,
        parameters: r.parameters.clone(),
    })
}

/// Whether *faces* are some faces, strictly ascending: a face named twice is not a record's.
fn ascending(faces: &[usize]) -> bool {
    !faces.is_empty() && faces.windows(2).all(|w| w[0] < w[1])
}
