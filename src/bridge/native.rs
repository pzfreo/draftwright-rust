//! Native recognition/3, /4 and /5 reader. Unified section recesses and their patterns use the same
//! parameters, constituent faces, feature IDs and grammar as the Python bridge. Source
//! recess families are represented by this projection; source refusals remain lint evidence.
//! Step levels and risers are retained as evidence, as on the bridge path.
//! Oriented chamfers supply true normal-section bevels; radial profiles remain evidence only.
//!
//! The document is read fail-closed (ADR3-4): checked schema versions ([`SCHEMA_VERSIONS`]), the STEP
//! file's coordinates, the part's face count, and only the families and record types that schema
//! has ([`FAMILIES`]); anything else is refused, not guessed at. [`recognise`] runs quiddity's
//! recognition, which recognises in the part's own frame and maps records back to the file's;
//! a value with no file axis to map to is listed in the record's `local`. A record taken here
//! whose `local` names a field draftwright reads is not read in the wrong axes: it is left out
//! and counted as unrecognised ([`left_in_frame`]), or, where patterns depend on it, the
//! document is refused. Step levels and risers are kept as evidence whatever their frame (their
//! parameters are not read). A record of a family read here
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
pub const SCHEMA_VERSION: &str = "quiddity-rust/recognition/5";
/// Older documents still read: v3 has no thread records or thread refusals, and v4 no face
/// accounting (which this reader does not use).
pub const SCHEMA_VERSIONS: &[&str] = &[
    "quiddity-rust/recognition/3",
    "quiddity-rust/recognition/4",
    SCHEMA_VERSION,
];

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
    ("interior_voids", &["InteriorVoid", "InteriorVoidRefusal"]),
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
    ("modelled_threads", &["ModelledThread"]),
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
    (
        "section_recess_patterns",
        &["SectionRecessArray", "SectionRecessGrid"],
    ),
    ("modelled_thread_refusals", &["ModelledThreadRefusal"]),
];

/// The families consumed here, including evidence and refusals. Oriented slot patterns are
/// checked and not kept (see the module docs). Threads are retained pending depiction.
pub const READ: [&str; 33] = [
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
    "section_recesses",
    "section_recess_patterns",
    "step_levels",
    "risers",
    "modelled_threads",
    "modelled_thread_refusals",
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
    /// Strictly ascending face indices (possibly empty).
    Faces,
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
        "ModelledThread",
        &[
            ("axis", Field::Object),
            ("axis.point", Field::Point),
            ("axis.direction", Field::Point),
            ("side", Field::Text),
            ("major_diameter", Field::Number),
            ("minor_diameter", Field::Number),
            ("pitch", Field::Number),
            ("hand", Field::Text),
            ("starts", Field::Count),
            ("start", Field::Number),
            ("end", Field::Number),
            ("length", Field::Number),
            ("tolerances", Field::Object),
            ("tolerances.diameter", Field::Number),
            ("tolerances.pitch", Field::Number),
            ("tolerances.axial", Field::Number),
            ("flank_faces", Field::Faces),
            ("crest_faces", Field::Faces),
            ("root_faces", Field::Faces),
            ("runout_faces", Field::Faces),
        ],
    ),
    (
        "ModelledThreadRefusal",
        &[
            ("axis", Field::Object),
            ("axis.point", Field::Point),
            ("axis.direction", Field::Point),
            ("faces", Field::Faces),
            ("reason", Field::Text),
        ],
    ),
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
        "SectionRecess",
        &[
            ("index", Field::Count),
            ("body", Field::Count),
            ("geometry", Field::Object),
            ("classification", Field::Object),
            ("evidence", Field::Object),
        ],
    ),
    (
        "SectionRecessRefusal",
        &[
            ("body", Field::Count),
            ("reason", Field::Text),
            ("evidence", Field::Object),
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
    pub evidence: Vec<super::Record>,
    pub refusals: Vec<super::Record>,
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
    let snapped = on_file_grid(doc);
    let doc = snapped.as_ref().unwrap_or(doc);
    if !SCHEMA_VERSIONS.contains(&doc.schema_version.as_str()) {
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
    let mut recess_patterns = Vec::new();
    let mut evidence = Vec::new();
    let mut refusals = Vec::new();
    for r in &doc.records {
        if doc.schema_version == SCHEMA_VERSIONS[0]
            && matches!(
                r.family.as_str(),
                "modelled_threads" | "modelled_thread_refusals"
            )
        {
            return Err(format!("{} requires {} or later", r.id, SCHEMA_VERSIONS[1]));
        }
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
        let local = local_read_fields(r);
        if !local.is_empty() {
            if !PATTERN_LINKED.contains(&r.family.as_str()) {
                // Left out and counted as unrecognised ([`left_in_frame`]).
                continue;
            }
            return Err(format!(
                "{} leaves {} in the part's own frame, not the file's",
                r.id,
                local.join(", ")
            ));
        }
        match r.family.as_str() {
            "section_recess_patterns" => {
                if (r.record_type == "SectionRecessGrid") != r.parameters.contains_key("rows") {
                    return Err(format!(
                        "{} has the wrong section-recess pattern shape",
                        r.id
                    ));
                }
                recess_patterns.push(r.parameters.clone());
            }
            "step_levels" | "risers" => evidence.push(super::Record {
                family: r.family.clone(),
                subtype: r.record_type.clone(),
                faces: r.faces.clone(),
                parameters: r.parameters.clone(),
            }),
            "section_recesses" if r.record_type == "SectionRecessRefusal" => {
                check(&r.parameters, &r.record_type)?;
                refusals.push(super::Record {
                    family: r.family.clone(),
                    subtype: r.record_type.clone(),
                    faces: super::recess_patterns::constituent_faces(
                        &r.parameters,
                        part.faces.len(),
                    )?,
                    parameters: r.parameters.clone(),
                });
            }
            "modelled_thread_refusals" => {
                check(&r.parameters, &r.record_type)?;
                let candidates = face_list(&r.parameters["faces"]);
                if candidates != r.faces {
                    return Err(format!("{} has inconsistent candidate faces", r.id));
                }
                refusals.push(super::Record {
                    family: r.family.clone(),
                    subtype: r.record_type.clone(),
                    faces: r.faces.clone(),
                    parameters: r.parameters.clone(),
                });
            }
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
    super::flats::merge(part, &IDENTITY, &mut others);
    name_slot_members(&mut others)?;
    check_oriented_slot_patterns(&others, &oriented_patterns)?;
    super::recess_patterns::append(part, &IDENTITY, &mut others, &recess_patterns)?;
    // Match the bridge's stable record order; provider family order is not its order.
    let key = |r: &super::Record| {
        (
            r.family.clone(),
            r.subtype.clone(),
            r.faces.clone(),
            Value::Object(r.parameters.clone()).to_string(),
        )
    };
    evidence.sort_by_cached_key(key);
    refusals.sort_by_cached_key(key);
    Ok(Native {
        recogniser: format!(
            "{} {} ({})",
            doc.package.name, doc.package.version, doc.schema_version
        ),
        holes: features,
        patterns,
        features: others,
        evidence,
        refusals,
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
    let faces = if r.family == "section_recesses" {
        super::recess_patterns::constituent_faces(&r.parameters, part.faces.len())?
    } else {
        r.faces.clone()
    };
    if !ascending(&faces) {
        return Err(format!("{} has no ascending faces to anchor it", r.id));
    }
    if r.family == "modelled_threads" {
        let mut claims = ["flank_faces", "crest_faces", "root_faces", "runout_faces"]
            .into_iter()
            .flat_map(|name| face_list(&r.parameters[name]))
            .collect::<Vec<_>>();
        claims.sort_unstable();
        if claims != faces {
            return Err(format!(
                "{} has inconsistent or overlapping thread face claims",
                r.id
            ));
        }
        // Invariant: check validates side as text before the modelled_threads branch.
        if !["internal", "external"].contains(&r.parameters["side"].as_str().unwrap())
            // Invariant: check validates hand as text before the modelled_threads branch.
            || !["left", "right"].contains(&r.parameters["hand"].as_str().unwrap())
            || r.parameters["starts"].as_u64() == Some(0)
        {
            return Err(format!("{} has invalid thread side, hand or starts", r.id));
        }
    }
    Ok(Feature {
        id: String::new(),
        kind: super::kind(&r.family),
        fingerprint: Fingerprint {
            version: FINGERPRINT_VERSION.into(),
            family: r.family.clone(),
            subtype: r.record_type.clone(),
            sizes: {
                let mut sizes = super::sizes(&r.parameters);
                if r.family == "modelled_threads" {
                    for name in ["major_diameter", "minor_diameter", "pitch", "starts"] {
                        sizes.insert(
                            name.into(),
                            // Invariant: check validates each required thread size as a finite number.
                            super::round(r.parameters[name].as_f64().unwrap()),
                        );
                    }
                }
                sizes
            },
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

/// Face lists are checked as whole indices before conversion (also on wasm32).
fn face_list(value: &Value) -> Vec<usize> {
    value
        .as_array()
        // Invariant: check validates face lists as arrays before face_list is called.
        .unwrap()
        .iter()
        // Invariant: check validates every face-list element as an in-range whole index.
        .map(|v| v.as_u64().unwrap() as usize)
        .collect()
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
            Field::Faces => v.as_array().is_some_and(|a| {
                a.iter()
                    .all(|v| v.as_u64().is_some_and(|n| usize::try_from(n).is_ok()))
                    && a.windows(2).all(|w| w[0].as_u64() < w[1].as_u64())
            }),
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

/// quiddity's publication grid for positions (mm).
const GRID: f64 = 1e-3;

/// *doc* with its records restored to the file's axes where quiddity's frame allows it, or
/// `None` where there is nothing to restore.
///
/// quiddity 0.1.0 recognises in the part's own frame, whose origin is the solids' centroid,
/// rounds positions on the grid there, and moves them to the file's coordinates without
/// rounding again (`framed_records`). So a hole at x = 40 can be published at 39.999728, and
/// a drawing would print 40.000 where it printed 40. Where the frame's axes are the file's up
/// to sign and order, a coordinate along file axis *w* is the frame's origin there plus or
/// minus a value on the grid. Any number that lies on the grid relative to one of the origin's
/// coordinates is taken to be such a coordinate and is rounded on the file's grid, as
/// quiddity published it before 0.1.0. Integers and values already on the grid are unchanged.
///
/// Where a frame axis is a file axis only to within 1e-6 (sm-hanger's frame is 2.4e-9 off), or
/// where only some frame axes are file axes (a hexagonal bar's axis is, its in-plane axes are
/// not), quiddity maps no axis letter and lists each in `local`. Each local top-level axis
/// letter whose frame axis is a file axis is renamed to that file axis, and an interval along
/// the record's own axis (a plate's `lo`/`hi`, a prism's `base`/`top`) and a plate's position
/// across it are moved as quiddity moves them on an exact frame. A local letter whose frame
/// axis is no file axis, in a field that may also be a direction ([`Field::Axis`]: a fillet's
/// axis), is replaced by that axis's direction in the file. Each restored field leaves `local`.
pub(crate) fn on_file_grid(doc: &RecognitionDocument) -> Option<RecognitionDocument> {
    let quiddity::frames::RecordFrame::Inferred(frame) = &doc.frame else {
        return None;
    };
    let axes = [frame.x, frame.y, frame.z];
    // Each frame axis's file axis and sense, where its components are within 1e-6 of 0 or ±1.
    let file_axes: [Option<(usize, f64)>; 3] = axes.map(|axis| {
        let ones: Vec<usize> = (0..3)
            .filter(|&w| (axis[w].abs() - 1.0).abs() <= 1e-6)
            .collect();
        let [w] = ones[..] else { return None };
        ((0..3).all(|v| v == w || axis[v].abs() <= 1e-6)).then_some((w, axis[w].signum()))
    });
    let exact = axes.iter().flatten().all(|&c| c == 0.0 || c.abs() == 1.0);
    let origin = frame.origin;
    fn on_grid(v: f64) -> bool {
        let k = v / GRID;
        (k - k.round()).abs() <= 1e-4
    }
    // `origin` is `None` off an exact frame: only round-off is removed there.
    fn snap(value: &mut Value, origin: Option<&[f64; 3]>) {
        match value {
            Value::Number(n) => {
                if let Some(x) = n.as_f64()
                    && !n.is_i64()
                    && !n.is_u64()
                {
                    let rounded = haecceity::py::round_to3(x);
                    let translated = || origin.is_some_and(|o| o.iter().any(|&o| on_grid(x - o)));
                    if rounded != x
                        && (on_grid(x) || translated())
                        && let Some(rounded) = serde_json::Number::from_f64(rounded)
                    {
                        *n = rounded;
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| snap(v, origin)),
            Value::Object(map) => map.values_mut().for_each(|v| snap(v, origin)),
            _ => {}
        }
    }
    let mut out = doc.clone();
    for r in &mut out.records {
        if !exact && !r.local.is_empty() {
            restore_local(r, &axes, &file_axes, &origin);
        }
        let grid = exact.then_some(&origin);
        r.parameters.values_mut().for_each(|v| snap(v, grid));
    }
    Some(out)
}

/// The local fields of *r* that [`on_file_grid`] can restore, restored.
fn restore_local(
    r: &mut quiddity::recognition::Record,
    axes: &[[f64; 3]; 3],
    file_axes: &[Option<(usize, f64)>; 3],
    origin: &[f64; 3],
) {
    const LETTERS: [&str; 3] = ["x", "y", "z"];
    let letter = |v: Option<&Value>| {
        v.and_then(Value::as_str)
            .and_then(|l| LETTERS.iter().position(|x| *x == l))
    };
    let number = |p: &Map<String, Value>, k: &str| p.get(k).and_then(Value::as_f64);
    let parameters = &mut r.parameters;
    let interval = match r.family.as_str() {
        "plates" => Some(("lo", "hi")),
        "polygonal_stock" | "polygonal_bosses" => Some(("base", "top")),
        _ => None,
    };
    if let Some((lo_key, hi_key)) = interval
        && r.local.iter().any(|p| p == lo_key)
        && let Some(i) = letter(parameters.get("axis"))
        && let Some((w, s)) = file_axes[i]
        && let (Some(lo), Some(hi)) = (number(parameters, lo_key), number(parameters, hi_key))
    {
        let (lo, hi) = (origin[w] + s * lo, origin[w] + s * hi);
        let (lo, hi) = if s > 0.0 { (lo, hi) } else { (hi, lo) };
        parameters.insert(lo_key.into(), serde_json::json!(lo));
        parameters.insert(hi_key.into(), serde_json::json!(hi));
        r.local.retain(|p| p != lo_key && p != hi_key);
        let [a, b] = match i {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        if r.family == "plates"
            && let (Some((wa, sa)), Some((wb, sb))) = (file_axes[a], file_axes[b])
            && let (Some(u), Some(v)) = (number(parameters, "u"), number(parameters, "v"))
        {
            let (va, vb) = (origin[wa] + sa * u, origin[wb] + sb * v);
            let (u, v) = if wa < wb { (va, vb) } else { (vb, va) };
            parameters.insert("u".into(), serde_json::json!(u));
            parameters.insert("v".into(), serde_json::json!(v));
            r.local.retain(|p| p != "u" && p != "v");
        }
    }
    let directions: Vec<&str> = FIELDS
        .iter()
        .find(|(t, _)| *t == r.record_type)
        .map(|(_, fields)| {
            fields
                .iter()
                .filter(|(_, kind)| *kind == Field::Axis)
                .map(|(name, _)| *name)
                .collect()
        })
        .unwrap_or_default();
    r.local.retain(|path| {
        let Some(k) = letter(parameters.get(path.as_str())) else {
            return true;
        };
        let restored = match file_axes[k] {
            Some((w, _)) => Value::String(LETTERS[w].into()),
            None if directions.contains(&path.as_str()) => serde_json::json!(axes[k]),
            None => return true,
        };
        parameters.insert(path.clone(), restored);
        false
    });
}

/// Families whose records patterns name, or that name others: one cannot be left out alone.
const PATTERN_LINKED: [&str; 10] = [
    "holes",
    "hole_patterns",
    "slots",
    "slot_patterns",
    "gusset_ribs",
    "gusset_rib_patterns",
    "oriented_slots",
    "oriented_slot_patterns",
    "section_recesses",
    "section_recess_patterns",
];

/// The fields of *r*, a record of a family read here, that draftwright reads ([`FIELDS`]) and
/// quiddity left in the part's own frame (`local`). quiddity 0.1.0 recognises in the part's frame
/// and maps records back to the file's; a value with no file axis to map to stays `local`. Step
/// levels and risers are horizontal in that frame, so on a part whose frame's z is not the file's
/// all their fields are; draftwright keeps them only as evidence by their faces, and reads its
/// own levels in the file's axes ([`crate::dims::levels::evidence`]), so none is read. A record
/// type without a [`FIELDS`] entry (holes, hole patterns) reads every field.
fn local_read_fields(r: &quiddity::recognition::Record) -> Vec<&str> {
    let family = r.family.as_str();
    let taken = matches!(family, "holes" | "hole_patterns")
        || READ.contains(&family)
        || EVIDENCE_READ.contains(&family);
    if !taken || matches!(family, "step_levels" | "risers") {
        return Vec::new();
    }
    let read = FIELDS.iter().find(|(t, _)| *t == r.record_type);
    r.local
        .iter()
        .map(String::as_str)
        .filter(|path| {
            let top = path.split(['.', '[']).next().unwrap_or(path);
            read.is_none_or(|(_, fields)| {
                fields
                    .iter()
                    .any(|(name, _)| name.split('.').next() == Some(top))
            })
        })
        .collect()
}

/// Whether [`read`] leaves *r* out: a field draftwright reads of it is in the part's own frame
/// ([`local_read_fields`]), and no pattern depends on it. Such records are counted as
/// unrecognised rather than drawn in the wrong axes or refusing the whole part.
pub fn left_in_frame(r: &quiddity::recognition::Record) -> bool {
    !PATTERN_LINKED.contains(&r.family.as_str()) && !local_read_fields(r).is_empty()
}

/// Legacy recess readings represented by the unified projection, including its refusals.
pub fn represented(family: &str) -> bool {
    READ.contains(&family)
        || EVIDENCE_READ.contains(&family)
        || [
            "pockets",
            "pocket_patterns",
            "channels",
            "prismatic_pockets",
            "round_bottom_blind_slots",
            "rectangular_blind_slots",
            "section_passages",
            "edge_open_circular_pockets",
            "edge_open_prismatic_recesses",
        ]
        .contains(&family)
}
