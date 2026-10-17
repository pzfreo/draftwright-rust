//! Features from Rust quiddity directly: its versioned recognition document
//! (`quiddity::recognition`), run on the part draftwright has already read, in place of Python
//! quiddity's JSON ([`super::read`]), family by family ([`READ`]): holes and hole patterns, then
//! fillets, chamfers, countersinks, bosses and turned steps, then grooves, flats, the steps
//! (angled, paired-ramp, circular blind, through and oblique through) and blends, then through
//! slots and slot patterns, polygonal bosses and stock, double-D bores, plates, gusset ribs and
//! their patterns, oriented slots and their patterns, and raised pads, plus oblique-edge chamfers. Face levels and risers
//! are evidence Rust quiddity finds (`features::levels`) but its document does not carry, so they
//! still come through the bridge.
//!
//! The recess families the document carries (pockets, channels, pocket patterns, prismatic
//! pockets, round-bottom and rectangular blind slots, section passages, edge-open circular
//! pockets and edge-open prismatic recesses) are known and not read. draftwright draws and lints
//! none of them as such: it reads pockets, channels, blind slots and passages as `section_recess`
//! features (Python's published `SectionRecess`, one section-geometry record into which Python
//! projects its internal pocket, prismatic pocket, channel, blind slot and passage readings;
//! Python's document has no `prismatic_pockets`). Since quiddity-rust 0.1.0-alpha.2 the
//! document also publishes that projection, as the `section_recesses` family (`SectionRecess`
//! records and `SectionRecessRefusal`s, an additive family of the same schema); it is known and
//! not yet read here, so draftwright's section recesses come through the bridge as before
//! (reading them is the G7b reader stream). The other families draftwright neither draws
//! nor lints (circular face patterns, thin-wall bodies, interior voids,
//! sheet-metal bodies, freeform surfaces) are known and not read
//! either. Repeating radial profiles are read only as full-wire correspondence evidence;
//! they never populate a normative gear feature.
//!
//! The document is read fail-closed (ADR3-4): one schema version ([`SCHEMA_VERSION`]), the STEP
//! file's coordinates, the part's face count, and only the families and record types that schema
//! has ([`FAMILIES`]); anything else is refused, not guessed at. A record taken here that lists
//! fields left in the part's own frame (`local`, schema 2) is refused too: those fields are not in
//! the file's coordinates. [`recognise`] runs quiddity's caller-space recognition, whose `frame`
//! is `caller-space` and whose records list none. A record of a family read here
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
//! whose faces are a turned step's, a blend whose faces are fillets', a chamfer on an angled
//! step's slant and a slot whose walls are passages', and its orchestration looks for no plates on
//! a solid with turned steps; Rust's document carries them, and the reader takes what the document
//! says (`tests/native.rs` lists each). Python's document, for its part, publishes a gusset rib
//! pattern only as its ribs' links, which the bridge drops; Rust's carries the pattern.
//!
//! A slot pattern names its member slots by their feature IDs, in quiddity's order (`slots`), in
//! place of the member records the document gives, as the bridge does: a member must be exactly
//! one slot's record, and no slot in two patterns. A gusset rib pattern keeps its member ribs'
//! records (`ribs`), as the bridge does and `lint/gussets.rs` reads them. An oriented slot
//! pattern is checked (its record, each member exactly one oriented slot's record, no oriented
//! slot in two patterns) and does not become a feature: the bridge reads none of Python's
//! (`DERIVED_NOT_READ`), and the members are read one by one as oriented slots, with no pattern in
//! their fingerprints, as the bridge reads them.

use haecceity::Part;
use quiddity::recognition::RecognitionDocument;
use serde_json::{Map, Value};

use super::{FINGERPRINT_VERSION, Feature, Fingerprint, HolePattern, QHolePattern};

/// The one document schema this reader knows.
pub const SCHEMA_VERSION: &str = "quiddity-rust/recognition/2";

/// The identity frame: the document's coordinates are the part's.
pub const IDENTITY: [[f64; 3]; 4] = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Every family of [`SCHEMA_VERSION`] and the record types its records can have, as
/// `quiddity::recognition::RECORD_TYPES` lists them for that schema (`tests/native.rs` checks the
/// two agree). [`READ`] supplies features and [`EVIDENCE_READ`] geometry-only records;
/// the rest are known and left to the bridge.
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
    ("slots", &["Slot"]),
    ("pockets", &["Pocket"]),
    ("channels", &["Channel"]),
    ("slot_patterns", &["SlotGrid", "SlotArray"]),
    ("pocket_patterns", &["PocketGrid", "PocketArray"]),
    ("repeating_radial_profiles", &["RepeatingRadialProfile"]),
    ("freeform_surfaces", &["FreeformSurface"]),
    ("polygonal_bosses", &["PolygonalBoss"]),
    ("polygonal_stock", &["PolygonalStock"]),
    ("section_passages", &["SectionPassage"]),
    ("prismatic_pockets", &["PrismaticPocket"]),
    ("oriented_slots", &["OrientedSlot"]),
    (
        "oriented_slot_patterns",
        &["OrientedSlotGrid", "OrientedSlotArray"],
    ),
    ("pads", &["RaisedPad"]),
    ("step_levels", &["FaceLevel"]),
    ("risers", &["RiserEvidence"]),
    (
        "section_recesses",
        &["SectionRecess", "SectionRecessRefusal"],
    ),
];

/// The families read here: each record is a feature, except an oriented slot pattern's, which is
/// checked and not kept (see the module docs).
pub const READ: [&str; 27] = [
    "holes",
    "hole_patterns",
    "fillets",
    "chamfers",
    "oriented_chamfers",
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
    "slots",
    "slot_patterns",
    "polygonal_bosses",
    "polygonal_stock",
    "double_d_bores",
    "plates",
    "gusset_ribs",
    "gusset_rib_patterns",
    "oriented_slots",
    "oriented_slot_patterns",
    "pads",
];

/// Geometry-only records consumed by independent lint, never feature declarations.
pub const EVIDENCE_READ: [&str; 1] = ["repeating_radial_profiles"];

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
    /// A number, or null.
    NumberOrNull,
    /// A whole number, not negative.
    Count,
    /// Points of three numbers, any number of them.
    PointList,
    /// An axis letter and a coordinate along it (a plane across that axis).
    Plane,
    /// This many [`Field::Plane`]s.
    Planes(usize),
    /// Records of the named record type, at least one, each checked against its [`FIELDS`].
    Records(&'static str),
    /// Numbers, or null.
    NumbersOrNull,
    /// An object, or null.
    ObjectOrNull,
    /// An object.
    Object,
    /// A blend's rolling path: straight (`at` and `direction`, three numbers each) or circular
    /// (`center` and `normal`, three numbers each, and a `radius`).
    BlendPath,
}

/// The fields draftwright reads of each record type it takes besides holes and hole patterns
/// (dims, lint, demand, and the fingerprint's axis and sizes), as Python's records carry them: a
/// record without one, or with one of another shape, is refused rather than filled in. A field
/// nothing reads (a fillet's `side`, a chamfer's `corner`, a pad's `direction`) is not required,
/// so a change to it does not refuse a read that would not use it. `a.b` is field `b` of object
/// field `a`, checked when `a` is not null (`a` is listed first).
pub const FIELDS: &[(&str, &[(&str, Field)])] = &[
    (
        "OrientedChamfer",
        &[
            ("run", Field::Point),
            ("length", Field::Number),
            ("at", Field::Point),
            ("corner", Field::Point),
            ("leg1", Field::Number),
            ("leg2", Field::Number),
            ("leg1_direction", Field::Point),
            ("leg2_direction", Field::Point),
            ("support_spans", Field::Section(2)),
            ("angle", Field::Number),
        ],
    ),
    (
        "RepeatingRadialProfile",
        &[
            ("axis", Field::Letter),
            ("centre", Field::Point),
            ("span", Field::Pair),
            ("repeat_count", Field::Count),
        ],
    ),
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
    (
        "Slot",
        &[
            ("long_axis", Field::Letter),
            ("width_axis", Field::Letter),
            ("width", Field::Number),
            ("length", Field::Number),
            ("lo", Field::Number),
            ("hi", Field::Number),
            ("w_center", Field::Number),
            ("d_lo", Field::Number),
            ("d_hi", Field::Number),
            ("end_radius", Field::NumberOrNull),
        ],
    ),
    (
        "SlotGrid",
        &[
            ("slots", Field::Records("Slot")),
            ("rows", Field::Count),
            ("cols", Field::Count),
            ("row_pitch", Field::Number),
            ("col_pitch", Field::Number),
            ("angle", Field::Number),
            ("center", Field::Point),
        ],
    ),
    (
        "SlotArray",
        &[
            ("slots", Field::Records("Slot")),
            ("pitch", Field::Number),
            ("direction", Field::Point),
        ],
    ),
    ("PolygonalBoss", POLYGONAL),
    ("PolygonalStock", POLYGONAL),
    (
        "DoubleDBore",
        &[
            ("axis", Field::Point),
            ("location", Field::Point),
            ("major_diameter", Field::Number),
            ("across_flats", Field::Number),
            ("through", Field::Bool),
            ("flat_direction", Field::Point),
        ],
    ),
    (
        "Plate",
        &[
            ("axis", Field::Letter),
            ("lo", Field::Number),
            ("hi", Field::Number),
            ("u", Field::Number),
            ("v", Field::Number),
        ],
    ),
    (
        "GussetRib",
        &[
            ("thickness_axis", Field::Letter),
            ("thickness_bounds", Field::Pair),
            ("supports", Field::Planes(2)),
            ("legs", Field::Pair),
            ("directions", Field::Pair),
        ],
    ),
    (
        "GussetRibArray",
        &[
            ("ribs", Field::Records("GussetRib")),
            ("pitch", Field::Number),
        ],
    ),
    (
        "GussetRibMirrorPair",
        &[
            ("ribs", Field::Records("GussetRib")),
            ("mirror_plane", Field::Plane),
        ],
    ),
    (
        "OrientedSlot",
        &[
            ("width", Field::Number),
            ("length", Field::Number),
            ("center", Field::Point),
            ("width_direction", Field::Point),
            ("long_direction", Field::Point),
            ("source", Field::Object),
            ("source.frame", Field::Object),
            ("source.run_interval", Field::Pair),
            ("source.section", Field::Object),
            ("source.ends", Field::Object),
        ],
    ),
    (
        "OrientedSlotGrid",
        &[
            ("slots", Field::Records("OrientedSlot")),
            ("rows", Field::Count),
            ("cols", Field::Count),
            ("row_pitch", Field::Number),
            ("col_pitch", Field::Number),
            ("angle", Field::Number),
            ("center", Field::Point),
        ],
    ),
    (
        "OrientedSlotArray",
        &[
            ("slots", Field::Records("OrientedSlot")),
            ("pitch", Field::Number),
            ("direction", Field::Point),
        ],
    ),
    (
        "RaisedPad",
        &[
            ("axis", Field::Letter),
            ("x0", Field::Number),
            ("x1", Field::Number),
            ("y0", Field::Number),
            ("y1", Field::Number),
            ("z0", Field::Number),
            ("z1", Field::Number),
        ],
    ),
];

/// A polygonal prism's fields (`PolygonalBoss`, `PolygonalStock`), as `dims/bosses.rs` and
/// `lint/polygonal.rs` read them; the lint checks the ring's count against `side_count` itself.
const POLYGONAL: &[(&str, Field)] = &[
    ("axis", Field::Letter),
    ("center", Field::Point),
    ("side_count", Field::Count),
    ("across_flats", Field::Number),
    ("base", Field::Number),
    ("top", Field::Number),
    ("flat_directions", Field::PointList),
    ("flat_centres", Field::PointList),
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
    /// Full-wire geometry evidence; never authored gear requirements.
    pub evidence: Vec<super::Record>,
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
    let mut oriented_patterns = Vec::new();
    let mut evidence = Vec::new();
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
        let taken = matches!(r.family.as_str(), "holes" | "hole_patterns")
            || READ.contains(&r.family.as_str())
            || EVIDENCE_READ.contains(&r.family.as_str());
        if taken && !r.local.is_empty() {
            return Err(format!(
                "{} leaves {} in the part's own frame, not the file's",
                r.id,
                r.local.join(", ")
            ));
        }
        match r.family.as_str() {
            "holes" => holes.push(r),
            "hole_patterns" => patterns.push(r),
            "oriented_slot_patterns" => oriented_patterns.push(r),
            f if EVIDENCE_READ.contains(&f) => {
                check(&r.parameters, &r.record_type).map_err(|e| format!("{}{e}", r.id))?;
                evidence.push(super::Record {
                    family: r.family.clone(),
                    subtype: r.record_type.clone(),
                    faces: r.faces.clone(),
                    parameters: r.parameters.clone(),
                });
            }
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
    name_slot_members(&mut others)?;
    check_oriented_slot_patterns(&others, &oriented_patterns)?;
    Ok(Native {
        recogniser: format!(
            "{} {} ({})",
            doc.package.name, doc.package.version, doc.schema_version
        ),
        holes: features,
        patterns,
        features: others,
        evidence,
    })
}

/// A feature of one of the families after holes and hole patterns in [`READ`], without its ID:
/// its record checked against [`FIELDS`], fingerprinted as [`super::read`] fingerprints it.
fn feature(
    part: &Part,
    qs: &[QHolePattern],
    r: &quiddity::recognition::Record,
) -> Result<Feature, String> {
    check(&r.parameters, &r.record_type).map_err(|e| {
        format!(
            "{} ({}){e}; the {} are not read",
            r.id, r.record_type, r.family
        )
    })?;
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

/// Checks *record*, of *record_type*, against its [`FIELDS`]; the error is what is wrong, to
/// follow the record's name (` has no width, …`, `'s width is …`).
fn check(record: &Map<String, Value>, record_type: &str) -> Result<(), String> {
    let fields = FIELDS
        .iter()
        .find(|(t, _)| *t == record_type)
        .ok_or(" has no fields known here")?
        .1;
    for &(name, field) in fields {
        let v = match name.split_once('.') {
            Some((outer, inner)) => match record.get(outer) {
                Some(Value::Null) => continue,
                o => o.and_then(|o| o.get(inner)),
            },
            None => record.get(name),
        };
        let v = v.ok_or_else(|| format!(" has no {name}, which draftwright reads"))?;
        let numbers = |v: &Value, n: usize| {
            v.as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(Value::is_number))
        };
        let points = |v: &Value, n: usize, d: usize| {
            v.as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(|p| numbers(p, d)))
        };
        let letter = |v: &Value| ["x", "y", "z"].contains(&v.as_str().unwrap_or(""));
        let plane = |v: &Value| {
            v.as_array()
                .is_some_and(|a| a.len() == 2 && letter(&a[0]) && a[1].is_number())
        };
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
            Field::NumberOrNull => v.is_null() || v.is_number(),
            Field::Count => v.is_u64(),
            Field::PointList => v
                .as_array()
                .is_some_and(|a| a.iter().all(|p| numbers(p, 3))),
            Field::Plane => plane(v),
            Field::Planes(n) => v
                .as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(plane)),
            Field::Records(member) => match v.as_array() {
                Some(a) if !a.is_empty() => {
                    for (i, m) in a.iter().enumerate() {
                        let m = m
                            .as_object()
                            .ok_or_else(|| format!("'s {name}[{i}] is {m}, not a {member}"))?;
                        check(m, member).map_err(|e| format!("'s {name}[{i}] ({member}){e}"))?;
                    }
                    true
                }
                _ => false,
            },
            Field::NumbersOrNull => {
                v.is_null() || v.as_array().is_some_and(|a| a.iter().all(Value::is_number))
            }
            Field::ObjectOrNull => v.is_null() || v.is_object(),
            Field::Object => v.is_object(),
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
            return Err(format!("'s {name} is {v}, not {field:?}"));
        }
    }
    Ok(())
}

/// Names each slot pattern's member slots by their IDs, in quiddity's order, in place of their
/// records (`slots`), as the bridge does: a member is the one slot whose record it equals, and a
/// slot is in at most one pattern; anything else is refused. Each pattern and its members take
/// the members' count and pitch as their fingerprint's pattern, as the bridge's do.
fn name_slot_members(features: &mut [Feature]) -> Result<(), String> {
    let mut used: Vec<usize> = Vec::new();
    for p in 0..features.len() {
        if features[p].kind != "slot_pattern" {
            continue;
        }
        let records = match features[p].parameters.get("slots") {
            Some(Value::Array(a)) => a.clone(),
            _ => return Err(format!("{} has no member slots", features[p].id)),
        };
        let mut members = Vec::new();
        for m in &records {
            let found: Vec<usize> = (0..features.len())
                .filter(|&i| {
                    features[i].kind == "slot" && Some(&features[i].parameters) == m.as_object()
                })
                .collect();
            match found[..] {
                [i] if !used.contains(&i) => {
                    used.push(i);
                    members.push(i);
                }
                [_] => {
                    return Err(format!(
                        "quiddity puts one slot in a slot pattern twice ({})",
                        features[p].id
                    ));
                }
                _ => {
                    return Err(format!(
                        "{}'s member matches {} slots, not one",
                        features[p].id,
                        found.len()
                    ));
                }
            }
        }
        let centres = members
            .iter()
            .map(|&i| slot_centre(&features[i].parameters))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("{} has a member with no location", features[p].id))?;
        let pattern = super::pattern(&centres);
        let ids: Vec<Value> = members
            .iter()
            .map(|&i| features[i].id.clone().into())
            .collect();
        for &i in &members {
            features[i].fingerprint.pattern = Some(pattern.clone());
        }
        let f = &mut features[p];
        f.parameters.insert("slots".into(), ids.into());
        f.fingerprint.pattern = Some(pattern);
    }
    Ok(())
}

/// Checks each oriented slot pattern (*patterns*) as the bridge would need it to name its
/// members: its record against [`FIELDS`] (its member records too), each member exactly one
/// oriented slot's record among *features*, and no oriented slot in two patterns; anything else
/// is refused. The patterns do not become features: the bridge reads none of Python's
/// (`DERIVED_NOT_READ`, draftwright gives oriented slot arrays no drafting grammar, #1430), and
/// their members are read one by one, as oriented slots.
fn check_oriented_slot_patterns(
    features: &[Feature],
    patterns: &[&quiddity::recognition::Record],
) -> Result<(), String> {
    let mut used: Vec<usize> = Vec::new();
    for r in patterns {
        check(&r.parameters, &r.record_type).map_err(|e| {
            format!(
                "{} ({}){e}; the {} are not read",
                r.id, r.record_type, r.family
            )
        })?;
        let Some(Value::Array(members)) = r.parameters.get("slots") else {
            return Err(format!("{} has no member oriented slots", r.id));
        };
        for m in members {
            let found: Vec<usize> = (0..features.len())
                .filter(|&i| {
                    features[i].kind == "oriented_slot"
                        && Some(&features[i].parameters) == m.as_object()
                })
                .collect();
            match found[..] {
                [i] if !used.contains(&i) => used.push(i),
                [_] => {
                    return Err(format!(
                        "quiddity puts one oriented slot in an oriented slot pattern twice ({})",
                        r.id
                    ));
                }
                _ => {
                    return Err(format!(
                        "{}'s member matches {} oriented slots, not one",
                        r.id,
                        found.len()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// A slot's centre (quiddity's `Slot.location`): midway along its long axis, on its width
/// centre, midway through its depth, as the bridge's slot patterns take it.
fn slot_centre(r: &Map<String, Value>) -> Option<[f64; 3]> {
    let letter = |k: &str| {
        let a = r.get(k)?.as_str()?;
        ["x", "y", "z"].iter().position(|x| *x == a)
    };
    let number = |k: &str| r.get(k).and_then(Value::as_f64);
    let (long, wide) = (letter("long_axis")?, letter("width_axis")?);
    if long == wide {
        return None;
    }
    let mut c = [0.0; 3];
    c[long] = 0.5 * (number("lo")? + number("hi")?);
    c[wide] = number("w_center")?;
    c[3 - long - wide] = 0.5 * (number("d_lo")? + number("d_hi")?);
    Some(c)
}

/// Whether *faces* are some faces, strictly ascending: a face named twice is not a record's.
fn ascending(faces: &[usize]) -> bool {
    !faces.is_empty() && faces.windows(2).all(|w| w[0] < w[1])
}
