//! Author intent over recognition, resolved before the one planning/lint waist (R4 ADR1-4).
//! No recognition runs here. Only explicitly named faces are interpreted, conservatively,
//! using the existing Analysis evidence. An empty correction set borrows the Analysis.
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::bridge::{Feature, Fingerprint, HolePattern, Recognition, Record, holes};
use crate::model::{Analysis, FeatureSpec, Invalid, Plan, Spec};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const MARKER: &str = "authored_recognition_corrections";
const TOL: f64 = 1e-5;
const AXES: [&str; 3] = ["x", "y", "z"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Hole,
    Boss,
    TurnedStep,
    Slot,
    Pocket,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Hole => "hole",
            Self::Boss => "boss",
            Self::TurnedStep => "turned_step",
            Self::Slot => "slot",
            Self::Pocket => "section_recess",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub faces: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// Author-chosen identity, independent of discovery order: `f:declared:<name>`.
    pub id: String,
    pub kind: Kind,
    pub faces: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arrangement {
    Linear,
    Grid,
    BoltCircle,
}
impl Arrangement {
    fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Grid => "grid",
            Self::BoltCircle => "bolt_circle",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternSpec {
    pub arrangement: Arrangement,
    pub members: Vec<String>,
}

fn invalid(path: &str, reason: impl std::fmt::Display) -> Invalid {
    let remedy = if path.starts_with("feature_declarations") || path.starts_with("f:declared:") {
        "remove this declaration by setting feature_declarations to the remaining list".to_string()
    } else if path.starts_with("features/") {
        format!("remove {path}")
    } else {
        "remove the offending correction from features or feature_declarations".into()
    };
    Invalid {
        message: format!(
            "{path}: {reason}; re-analyse and reattach to the current feature/face fingerprints, or {remedy}."
        ),
        choices: Vec::new(),
    }
}

fn authored(f: &FeatureSpec) -> bool {
    f.suppress || f.split || f.reclassify.is_some() || f.pattern.is_some()
}
pub fn authors(s: &Spec) -> bool {
    !s.feature_declarations.is_empty() || s.features.values().any(authored)
}
fn unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
fn face_list(path: &str, faces: &[String]) -> Result<(), Invalid> {
    if faces.is_empty()
        || !unique(faces)
        || faces.iter().any(|f| {
            !f.strip_prefix("face:sha256:")
                .is_some_and(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        })
    {
        return Err(invalid(
            path,
            "faces must be a nonempty, unique list of face:sha256:<64 hex digits> fingerprints",
        ));
    }
    Ok(())
}
/// Shape validation, also used by stateless edits before there is a part.
pub fn check(s: &Spec) -> Result<(), Invalid> {
    if !authors(s) {
        return Ok(());
    }
    let mut ids = BTreeSet::new();
    for d in &s.feature_declarations {
        if !d.id.starts_with("f:declared:")
            || d.id.len() == "f:declared:".len()
            || !ids.insert(&d.id)
        {
            return Err(invalid(
                "feature_declarations",
                "each declaration needs a unique f:declared:<name> id",
            ));
        }
        face_list(&format!("feature_declarations/{}", d.id), &d.faces)?;
    }
    let mut grouped = BTreeSet::new();
    for (id, f) in &s.features {
        let path = format!("features/{id}");
        let actions = usize::from(f.suppress)
            + usize::from(f.split)
            + usize::from(f.reclassify.is_some())
            + usize::from(f.pattern.is_some());
        if actions > 1 {
            return Err(invalid(
                &path,
                "choose one of suppress, split, reclassify or pattern",
            ));
        }
        if let Some(a) = &f.anchor {
            face_list(&format!("{path}/anchor"), &a.faces)?;
        }
        if f.suppress
            && (f.fit.is_some()
                || f.thread.is_some()
                || f.knurl.is_some()
                || f.datum.is_some()
                || !f.controls.is_empty()
                || f.finish.is_some()
                || f.note.is_some()
                || !f.note_satisfies.is_empty())
        {
            return Err(invalid(
                &path,
                "a suppressed feature cannot also carry manufacturing requirements",
            ));
        }
        if let Some(p) = &f.pattern {
            if f.anchor.is_some() {
                return Err(invalid(
                    &path,
                    "an authored group is anchored through its members' feature entries, not a group anchor",
                ));
            }
            if !id.starts_with("f:hole_pattern:")
                || id.len() == "f:hole_pattern:".len()
                || p.members.len() < 2
                || !unique(&p.members)
            {
                return Err(invalid(
                    &path,
                    "a pattern needs a f:hole_pattern:<name> id and at least two distinct holes",
                ));
            }
            for m in &p.members {
                if !grouped.insert(m) {
                    return Err(invalid(
                        &path,
                        format!("{m} belongs to two authored patterns"),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// A named face's signature from existing, view-independent evidence. No B-rep rescan.
/// Bounds, surface and support are rounded to a micrometre; index and sampling order are absent.
pub fn face_fingerprint(a: &Analysis, face: usize) -> Result<String, Invalid> {
    let f = a.face_attachments.get(face).ok_or_else(|| {
        invalid(
            "face",
            "this Analysis lacks face attachments; analyse again",
        )
    })?;
    let b = f
        .bounds
        .ok_or_else(|| invalid("face", "the face has no finite bounds"))?;
    let round = |v: f64| (v * 1e6).round() / 1e6 + 0.0;
    let data = json!([
        f.surface,
        f.axis,
        f.support.iter().copied().map(round).collect::<Vec<_>>(),
        b.map(round),
        f.normal.map(|n| n.map(round))
    ]);
    Ok(format!(
        "face:sha256:{}",
        digest(&serde_json::to_vec(&data).expect("face signature"))
    ))
}
fn anchors(a: &Analysis, faces: &[usize]) -> Result<Anchor, Invalid> {
    let mut faces = faces
        .iter()
        .map(|&f| face_fingerprint(a, f))
        .collect::<Result<Vec<_>, _>>()?;
    faces.sort();
    face_list("anchor", &faces)?;
    Ok(Anchor { faces })
}
fn resolve_faces(a: &Analysis, faces: &[String], path: &str) -> Result<Vec<usize>, Invalid> {
    face_list(path, faces)?;
    let mut out = Vec::new();
    for name in faces {
        let matches: Vec<_> = (0..a.face_attachments.len())
            .filter(|&i| face_fingerprint(a, i).is_ok_and(|f| &f == name))
            .collect();
        if matches.len() != 1 {
            return Err(invalid(
                path,
                format!(
                    "face fingerprint {name} resolves to {} faces (missing or ambiguous)",
                    matches.len()
                ),
            ));
        }
        out.push(matches[0]);
    }
    out.sort_unstable();
    Ok(out)
}
fn target_faces(a: &Analysis, id: &str) -> Option<Vec<usize>> {
    a.features
        .iter()
        .find(|f| f.id == id)
        .map(|f| f.faces.clone())
        .or_else(|| {
            a.recognition
                .as_ref()?
                .patterns
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.faces.clone())
        })
        .or_else(|| {
            serde_json::from_value(marker(a)?.parameters.get("targets")?.get(id)?.clone()).ok()
        })
}
fn resolved_id(a: &Analysis, s: &Spec, id: &str) -> Result<String, Invalid> {
    let path = format!("features/{id}");
    if let Some(anchor) = s.features.get(id).and_then(|f| f.anchor.as_ref()) {
        let faces = resolve_faces(a, &anchor.faces, &path)?;
        let mut candidates: Vec<_> = a
            .features
            .iter()
            .filter(|f| f.faces == faces)
            .map(|f| f.id.clone())
            .chain(
                a.recognition
                    .iter()
                    .flat_map(|r| &r.patterns)
                    .filter(|p| p.faces == faces)
                    .map(|p| p.id.clone()),
            )
            .collect();
        if target_faces(a, id).as_ref() == Some(&faces) && !candidates.contains(&id.to_string()) {
            candidates.push(id.to_string());
        }
        // Kind-independent face anchors may name a boss and turned band on the same face.
        if candidates.contains(&id.to_string()) {
            return Ok(id.to_string());
        }
        if candidates.len() == 1 {
            return Ok(candidates[0].clone());
        }
        return Err(invalid(
            &path,
            format!(
                "the defining faces resolve to {} features; select one current feature",
                candidates.len()
            ),
        ));
    }
    if s.binding
        .as_ref()
        .is_some_and(|b| b.source_sha256 != a.binding.source_sha256)
    {
        return Err(invalid(
            &path,
            "a correction from another revision needs a saved face anchor",
        ));
    }
    if target_faces(a, id).is_none() {
        let mut e = invalid(&path, "unknown feature or pattern");
        e.choices = a
            .features
            .iter()
            .map(|f| f.id.clone())
            .chain(
                a.recognition
                    .iter()
                    .flat_map(|r| &r.patterns)
                    .map(|p| p.id.clone()),
            )
            .collect();
        return Err(e);
    }
    Ok(id.to_string())
}
/// Save defining-face anchors beside corrections and every grouped member. Explicit anchors
/// are never silently replaced. The authored IDs of declarations and groups remain unchanged.
pub fn bind(a: &Analysis, s: Spec) -> Result<Spec, Invalid> {
    if !authors(&s) {
        return Ok(s);
    }
    check(&s)?;
    let mut out = s.clone();
    let mut ids: BTreeSet<_> = s
        .features
        .iter()
        .filter(|(_, f)| authored(f) && f.pattern.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    ids.extend(
        s.features
            .values()
            .filter_map(|f| f.pattern.as_ref())
            .flat_map(|p| p.members.clone())
            .filter(|id| !id.starts_with("f:declared:")),
    );
    for id in ids {
        let resolved = resolved_id(a, &s, &id)?;
        let entry = out.features.entry(id).or_default();
        if entry.anchor.is_none() {
            entry.anchor = Some(anchors(
                a,
                &target_faces(a, &resolved).expect("resolved target"),
            )?);
        }
    }
    if out
        .binding
        .as_ref()
        .is_some_and(|b| b.source_sha256 != a.binding.source_sha256)
    {
        let known = |id: &str| {
            out.features
                .get(id)
                .is_some_and(|f| f.anchor.is_some() || f.pattern.is_some())
                || out.feature_declarations.iter().any(|d| d.id == id)
        };
        let measurement = |id: &str| {
            id.starts_with("d:envelope/")
                || id
                    .strip_prefix("d:")
                    .and_then(|v| v.rsplit_once('/'))
                    .is_some_and(|(f, _)| known(f))
        };
        if !out.faces.is_empty()
            || !out.gears.is_empty()
            || !out.annotations.is_empty()
            || out.features.keys().any(|id| !known(id))
            || out.dimensions.keys().any(|id| !measurement(id))
            || out.precision.keys().any(|id| !measurement(id))
        {
            return Err(invalid(
                "binding",
                "this correction Spec also contains unanchored revision-specific edits; reattach those edits before applying to another revision",
            ));
        }
        out.binding = Some(a.binding.clone());
    }
    Ok(out)
}

/// Keep recovery's restored Specs bound to the same correction anchors as its first plan.
pub(crate) fn canonical<'a>(a: &Analysis, s: &'a Spec) -> Result<Cow<'a, Spec>, Invalid> {
    if authors(s) {
        Ok(Cow::Owned(bind(a, s.clone())?))
    } else {
        Ok(Cow::Borrowed(s))
    }
}

fn object(v: Value) -> Map<String, Value> {
    v.as_object().expect("record object").clone()
}
fn unit(k: usize) -> [f64; 3] {
    [0, 1, 2].map(|i| f64::from(i == k))
}
fn local_point(a: &Analysis, p: [f64; 3]) -> [f64; 3] {
    let frame = holes::frame(a);
    [0, 1, 2].map(|i| (0..3).map(|j| (p[j] - frame[0][j]) * frame[i + 1][j]).sum())
}
fn local_dir(a: &Analysis, p: [f64; 3]) -> [f64; 3] {
    let frame = holes::frame(a);
    [0, 1, 2].map(|i| (0..3).map(|j| p[j] * frame[i + 1][j]).sum())
}
fn feature(
    a: &Analysis,
    id: &str,
    kind: Kind,
    faces: Vec<usize>,
    parameters: Value,
) -> Result<Feature, Invalid> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for &i in &faces {
        let b = a
            .face_attachments
            .get(i)
            .and_then(|f| f.bounds)
            .ok_or_else(|| invalid(id, "face geometry is unavailable"))?;
        for k in 0..3 {
            lo[k] = lo[k].min(b[k]);
            hi[k] = hi[k].max(b[k + 3]);
        }
    }
    let p = object(parameters);
    let sizes = ["diameter", "height", "width", "length", "depth"]
        .into_iter()
        .filter_map(|n| Some((n.into(), p.get(n)?.as_f64()?)))
        .collect();
    Ok(Feature {
        id: id.into(),
        kind: kind.name().into(),
        faces,
        parameters: p,
        fingerprint: Fingerprint {
            version: crate::bridge::FINGERPRINT_VERSION.into(),
            family: kind.name().into(),
            subtype: "Authored".into(),
            sizes,
            axis: None,
            position: [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5),
            pattern: None,
            neighbourhood: Vec::new(),
        },
    })
}

/// Read an explicitly selected cylindrical surface. Requiring its full radial box avoids
/// mistaking a fillet/partial cylinder for a bore or turned band. All parameters come from faces.
fn cylindrical(a: &Analysis, id: &str, kind: Kind, faces: Vec<usize>) -> Result<Feature, Invalid> {
    let fail = |why: &str| invalid(id, why);
    let mut cylinder: Option<(usize, [f64; 3], f64)> = None;
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut floors = Vec::new();
    for &i in &faces {
        let f = a
            .face_attachments
            .get(i)
            .ok_or_else(|| fail("face attachments are unavailable"))?;
        let b = f.bounds.ok_or_else(|| fail("a named face has no bounds"))?;
        match f.surface.as_deref() {
            Some("cylinder") => {
                let k = f.exact_axis.ok_or_else(|| {
                    fail("the cylinder is not principal; no supported planner can carry it")
                })?;
                if f.support.len() != 2 {
                    return Err(fail("the cylinder has no axis support"));
                }
                let mut on = [0.; 3];
                let mut across = (0..3).filter(|&j| j != k);
                let u = across.next().unwrap();
                let v = across.next().unwrap();
                on[u] = f.support[0];
                on[v] = f.support[1];
                let site = f
                    .sites
                    .first()
                    .ok_or_else(|| fail("the cylinder has no proven surface site"))?;
                let radius = ((site[u] - on[u]).powi(2) + (site[v] - on[v]).powi(2)).sqrt();
                if radius <= TOL
                    || [u, v].iter().any(|&j| {
                        (b[j] - (on[j] - radius)).abs() > TOL
                            || (b[j + 3] - (on[j] + radius)).abs() > TOL
                    })
                {
                    return Err(fail(
                        "named faces do not prove a complete circular cylinder; name the complete bore/band faces",
                    ));
                }
                if let Some((k0, on0, r0)) = cylinder {
                    if k0 != k
                        || (r0 - radius).abs() > TOL
                        || [u, v].iter().any(|&j| (on0[j] - on[j]).abs() > TOL)
                    {
                        return Err(fail("named cylinders are not coaxial and equal-radius"));
                    }
                }
                cylinder = Some((k, on, radius));
                lo = lo.min(b[k]);
                hi = hi.max(b[k + 3]);
            }
            Some("plane") => floors.push(f),
            _ => {
                return Err(fail(
                    "this interpretation needs cylindrical walls and optional planar ends",
                ));
            }
        }
    }
    let (k, mut on, r) = cylinder.ok_or_else(|| fail("there is no cylindrical face"))?;
    // A circular inner boundary proves an opening. This prevents an authored bore from
    // turning a solid shaft into a hole, and an external feature from consuming a bore.
    let opening = a
        .principal_profiles
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|f| f.axis == k && ((f.at - lo).abs() < TOL || (f.at - hi).abs() < TOL))
        .flat_map(|f| &f.inner)
        .any(|w| {
            !w.edges.is_empty()
                && w.edges.iter().all(|e| {
                    e.radius.is_some_and(|v| (v - r).abs() < TOL)
                        && e.centre.is_some_and(|c| {
                            (0..3)
                                .filter(|&j| j != k)
                                .all(|j| (c[j] - on[j]).abs() < TOL)
                        })
                })
        });
    if kind == Kind::Hole && !opening {
        return Err(fail(
            "no circular opening in the face evidence proves a bore; name a bore's wall and floor, not an external cylinder",
        ));
    }
    if kind != Kind::Hole && opening {
        return Err(fail(
            "the cylinder bounds an opening, so it cannot be an external boss or turned band",
        ));
    }
    let external = a
        .features
        .iter()
        .any(|f| f.faces == faces && matches!(f.kind.as_str(), "boss" | "turned_step"))
        || a.principal_profiles
            .iter()
            .flat_map(|s| &s.faces)
            .filter(|f| f.axis == k && ((f.at - lo).abs() < TOL || (f.at - hi).abs() < TOL))
            .flat_map(|f| &f.outer_arcs)
            .any(|e| {
                e.radius.is_some_and(|v| (v - r).abs() < TOL)
                    && e.centre.is_some_and(|c| {
                        (0..3)
                            .filter(|&j| j != k)
                            .all(|j| (c[j] - on[j]).abs() < TOL)
                    })
            });
    if kind != Kind::Hole && !external {
        return Err(fail(
            "the evidence does not prove external material on this cylinder; name a supported external band with its end face",
        ));
    }

    if hi - lo <= TOL {
        return Err(fail("the cylinder has no axial length"));
    }
    // No gap or detached band can be turned into one length.
    let mut spans: Vec<_> = faces
        .iter()
        .filter_map(|&i| {
            let f = &a.face_attachments[i];
            (f.surface.as_deref() == Some("cylinder")).then(|| {
                let b = f.bounds.unwrap();
                [b[k], b[k + 3]]
            })
        })
        .collect();
    spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut end = spans[0][1];
    for s in &spans[1..] {
        if s[0] > end + TOL {
            return Err(fail("named cylindrical bands are disconnected"));
        }
        end = end.max(s[1]);
    }
    let mut caps = Vec::new();
    for f in floors {
        let b = f.bounds.unwrap();
        if f.axis != Some(k)
            || f.support.len() != 1
            || ((b[k] - lo).abs() > TOL && (b[k] - hi).abs() > TOL)
        {
            return Err(fail(
                "a named planar end is not perpendicular at a cylinder shoulder",
            ));
        }
        if (0..3)
            .filter(|&j| j != k)
            .any(|j| (b[j] - (on[j] - r)).abs() > TOL || (b[j + 3] - (on[j] + r)).abs() > TOL)
        {
            return Err(fail(
                "a named planar end does not match the cylinder's circular footprint",
            ));
        }
        if kind == Kind::Hole
            && f.normal
                .is_none_or(|n| (n[k] - if (b[k] - lo).abs() < TOL { 1. } else { -1. }).abs() > TOL)
        {
            return Err(fail(
                "the selected floor normal does not face into the bore",
            ));
        }
        caps.push(f.support[0]);
    }
    on[k] = (lo + hi) * 0.5;
    let dir = local_dir(a, unit(k));
    let qk = holes::principal(dir)
        .ok_or_else(|| fail("the recognition frame has no principal cylinder axis"))?;
    let mut low = on;
    low[k] = lo;
    let mut high = on;
    high[k] = hi;
    let l = local_point(a, low);
    let h = local_point(a, high);
    let params = match kind {
        Kind::Hole => {
            if caps.len() > 1 {
                return Err(fail("a hole cannot have both ends capped"));
            }
            let through = caps.is_empty();
            // A lone cylinder must reach both outer stock planes to prove throughness.
            if through
                && ((lo - a.envelope[0][k]).abs() > TOL || (hi - a.envelope[1][k]).abs() > TOL)
            {
                return Err(fail(
                    "throughness is unproven; include the blind floor face",
                ));
            }
            let opens_high = caps.first().is_none_or(|z| (*z - lo).abs() < TOL);
            let location = if opens_high { h } else { l };
            let axis = local_dir(a, unit(k).map(|v| v * if opens_high { -1. } else { 1. }));
            json!({"diameter":2.*r,"depth":hi-lo,"axis":axis,"location":location,"bottom":if through {"through"} else {"flat"}})
        }
        Kind::Boss => json!({"diameter":2.*r,"height":hi-lo,"axis":dir,"location":h}),
        Kind::TurnedStep => {
            json!({"diameter":2.*r,"lo":l[qk].min(h[qk]),"hi":l[qk].max(h[qk]),"axis":AXES[qk],"profile":{"axis_origin":local_point(a,on),"lo":l[qk].min(h[qk]),"hi":l[qk].max(h[qk])}})
        }
        _ => unreachable!(),
    };
    feature(a, id, kind, faces, params)
}

fn derive(a: &Analysis, id: &str, kind: Kind, faces: Vec<usize>) -> Result<Feature, Invalid> {
    match kind {
        Kind::Hole | Kind::Boss | Kind::TurnedStep => cylindrical(a, id, kind, faces),
        Kind::Pocket | Kind::Slot => rectangular(a, id, kind, faces),
    }
}

/// Four inward-facing rectangular walls, with one optional planar floor. Bounding boxes
/// alone do not define a pocket: the signed support planes and their common spans must agree.
fn rectangular(a: &Analysis, id: &str, kind: Kind, faces: Vec<usize>) -> Result<Feature, Invalid> {
    let fail = |why: &str| invalid(id, why);
    if !(4..=5).contains(&faces.len()) {
        return Err(fail(
            "name four rectangular side walls and, for a pocket, its floor",
        ));
    }
    let mut planes: [Vec<(usize, f64)>; 3] = std::array::from_fn(|_| Vec::new());
    for &i in &faces {
        let f = a
            .face_attachments
            .get(i)
            .ok_or_else(|| fail("face evidence is unavailable"))?;
        if f.surface.as_deref() != Some("plane")
            || f.support.len() != 1
            || f.normal.is_none()
            || f.bounds.is_none()
        {
            return Err(fail(
                "this interpretation needs principal planar walls and ends",
            ));
        }
        let k = f.axis.ok_or_else(|| fail("a wall is not principal"))?;
        planes[k].push((i, f.support[0]));
    }
    let depths: Vec<_> = (0..3)
        .filter(|&k| {
            planes[k].len() <= 1 && (0..3).filter(|&j| j != k).all(|j| planes[j].len() == 2)
        })
        .collect();
    if depths.len() != 1 {
        return Err(fail("faces do not enclose one rectangular cross-section"));
    }
    let k = depths[0];
    let u = (k + 1) % 3;
    let v = (k + 2) % 3;
    let mut lo = [0.; 3];
    let mut hi = [0.; 3];
    for j in [u, v] {
        planes[j].sort_by(|a, b| a.1.total_cmp(&b.1));
        lo[j] = planes[j][0].1;
        hi[j] = planes[j][1].1;
        if hi[j] - lo[j] <= TOL {
            return Err(fail("the section has no positive width"));
        }
        for (n, &(i, _)) in planes[j].iter().enumerate() {
            let normal = a.face_attachments[i].normal.unwrap();
            if (normal[j] - if n == 0 { 1. } else { -1. }).abs() > TOL {
                return Err(fail(
                    "wall normals point out of the selected region; it is material, not a recess",
                ));
            }
        }
    }
    let b = a.face_attachments[planes[u][0].0].bounds.unwrap();
    lo[k] = b[k];
    hi[k] = b[k + 3];
    if hi[k] - lo[k] <= TOL {
        return Err(fail("the walls have no depth"));
    }
    for j in [u, v] {
        for &(i, _) in &planes[j] {
            let b = a.face_attachments[i].bounds.unwrap();
            let other = if j == u { v } else { u };
            if [k, other]
                .iter()
                .any(|&t| (b[t] - lo[t]).abs() > TOL || (b[t + 3] - hi[t]).abs() > TOL)
            {
                return Err(fail(
                    "wall spans disagree; a bounding rectangle would invent geometry",
                ));
            }
        }
    }
    let capped = !planes[k].is_empty();
    let mut opens_high = true;
    if capped {
        let (i, t) = planes[k][0];
        let f = &a.face_attachments[i];
        let b = f.bounds.unwrap();
        opens_high = (t - lo[k]).abs() <= TOL;
        if !opens_high && (t - hi[k]).abs() > TOL
            || [u, v]
                .iter()
                .any(|&j| (b[j] - lo[j]).abs() > TOL || (b[j + 3] - hi[j]).abs() > TOL)
            || (f.normal.unwrap()[k] - if opens_high { 1. } else { -1. }).abs() > TOL
        {
            return Err(fail(
                "the floor does not close the complete section at a wall end",
            ));
        }
    }
    if kind == Kind::Pocket && !capped {
        return Err(fail(
            "a pocket requires a capped floor; these faces prove a through slot",
        ));
    }
    if kind == Kind::Slot && capped {
        return Err(fail(
            "the slot planner requires a through opening; these faces have a pocket floor",
        ));
    }
    let low = local_point(a, lo);
    let high = local_point(a, hi);
    let qk = holes::principal(local_dir(a, unit(k)))
        .ok_or_else(|| fail("the recognition frame is not principal"))?;
    let qu = holes::principal(local_dir(a, unit(u)))
        .ok_or_else(|| fail("the recognition frame is not principal"))?;
    let qv = 3 - qk - qu;
    let low = [0, 1, 2].map(|j| low[j].min(high[j]));
    let high = local_point(a, lo)
        .iter()
        .zip(high)
        .map(|(a, b)| a.max(b))
        .collect::<Vec<_>>();
    let params = if kind == Kind::Slot {
        let (long, wide) = if high[qu] - low[qu] >= high[qv] - low[qv] {
            (qu, qv)
        } else {
            (qv, qu)
        };
        json!({"long_axis":AXES[long],"width_axis":AXES[wide],"width":high[wide]-low[wide],"length":high[long]-low[long],"lo":low[long],"hi":high[long],"w_center":(low[wide]+high[wide])*0.5,"d_lo":low[qk],"d_hi":high[qk]})
    } else {
        // Positive local frame; choose u,v cyclically so u×v=run.
        let qu = (qk + 1) % 3;
        let qv = (qk + 2) % 3;
        let open_positive = opens_high == (local_dir(a, unit(k))[qk] > 0.);
        let boundary = vec![
            json!({"point":[low[qu],low[qv]],"bulge":0.}),
            json!({"point":[high[qu],low[qv]],"bulge":0.}),
            json!({"point":[high[qu],high[qv]],"bulge":0.}),
            json!({"point":[low[qu],high[qv]],"bulge":0.}),
        ];
        let end = |open, t| json!({"condition":if open {"open"} else {"capped"},"surface":{"type":"plane","gradient":[0.,0.],"offset":t}});
        json!({"classification":{"feature_kind":"pocket","section_shape":"rectangular"},"evidence":{"defining_faces":faces,"constituent_faces":faces},"geometry":{"frame":{"origin":[0.,0.,0.],"run":unit(qk),"u":unit(qu),"v":unit(qv)},"run_interval":[low[qk],high[qk]],"profile":{"closure":"closed","boundary":boundary},"ends":{"low":end(!open_positive,low[qk]),"high":end(open_positive,high[qk])}}})
    };
    let f = feature(a, id, kind, faces, params)?;
    if kind == Kind::Pocket {
        crate::lint::section_recesses::grammar(&f).map_err(|e| fail(e.reason()))?;
    }
    Ok(f)
}

fn stations(points: &[[f64; 3]], k: usize) -> Vec<f64> {
    let mut out: Vec<_> = points.iter().map(|p| p[k]).collect();
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < TOL);
    out
}
fn even(v: &[f64]) -> bool {
    v.len() < 3
        || v.windows(2)
            .all(|w| ((w[1] - w[0]) - (v[1] - v[0])).abs() < TOL)
}
fn pattern(
    a: &Analysis,
    id: &str,
    p: &PatternSpec,
    members: Vec<String>,
) -> Result<HolePattern, Invalid> {
    let fail = |why: &str| invalid(&format!("features/{id}/pattern"), why);
    let mut reads = Vec::new();
    let mut faces = Vec::new();
    for m in &members {
        let f = a
            .features
            .iter()
            .find(|f| &f.id == m && f.kind == "hole")
            .ok_or_else(|| fail("every member must resolve to a nonsuppressed hole"))?;
        reads.push(holes::Hole::read(f, &holes::frame(a)).map_err(|why| fail(&why))?);
        faces.extend(&f.faces);
    }
    let first = &reads[0];
    let (k, _) = first
        .principal
        .ok_or_else(|| fail("the hole axes are not principal"))?;
    if reads
        .iter()
        .any(|h| h.requirement() != first.requirement() || (h.at[k] - first.at[k]).abs() > TOL)
    {
        return Err(fail(
            "members must have identical sizes, depth and direction, opening on one plane",
        ));
    }
    let points: Vec<_> = reads.iter().map(|h| h.at).collect();
    for (i, h) in points.iter().enumerate() {
        if points[..i]
            .iter()
            .any(|b| (0..3).all(|j| (b[j] - h[j]).abs() < TOL))
        {
            return Err(fail("members have coincident axes"));
        }
    }
    let u = (k + 1) % 3;
    let v = (k + 2) % 3;
    let us = stations(&points, u);
    let vs = stations(&points, v);
    let mut centre = points[0];
    centre[u] = (us[0] + us[us.len() - 1]) * 0.5;
    centre[v] = (vs[0] + vs[vs.len() - 1]) * 0.5;
    let params = match p.arrangement {
        Arrangement::Linear => {
            let values = if us.len() == 1 {
                &vs
            } else if vs.len() == 1 {
                &us
            } else {
                return Err(fail("linear members must lie along one principal axis"));
            };
            if !even(values) {
                return Err(fail("linear members are not evenly spaced"));
            }
            json!({"pitch":values[1]-values[0],"count":members.len(),"center":local_point(a,centre)})
        }
        Arrangement::Grid => {
            if us.len() < 2
                || vs.len() < 2
                || us.len() * vs.len() != members.len()
                || !even(&us)
                || !even(&vs)
            {
                return Err(fail(
                    "grid members must fill an evenly spaced rectangular lattice",
                ));
            }
            json!({"rows":vs.len(),"cols":us.len(),"row_pitch":vs[1]-vs[0],"col_pitch":us[1]-us[0],"center":local_point(a,centre)})
        }
        Arrangement::BoltCircle => {
            if points.len() < 3 {
                return Err(fail(
                    "a bolt circle needs at least three noncollinear holes",
                ));
            }
            // Circumcentre from a non-collinear triple, not from its box (odd counts differ).
            let p0 = points[0];
            let p1 = points[1];
            let mut found = None;
            for p2 in &points[2..] {
                let x = p1[u] - p0[u];
                let y = p1[v] - p0[v];
                let xx = p2[u] - p0[u];
                let yy = p2[v] - p0[v];
                let det = 2. * (x * yy - y * xx);
                if det.abs() > TOL {
                    let s = x * x + y * y;
                    let t = xx * xx + yy * yy;
                    let mut c = p0;
                    c[u] += (s * yy - y * t) / det;
                    c[v] += (x * t - s * xx) / det;
                    found = Some(c);
                    break;
                }
            }
            centre = found.ok_or_else(|| fail("bolt-circle members are collinear"))?;
            let r = (points[0][u] - centre[u]).hypot(points[0][v] - centre[v]);
            if points
                .iter()
                .any(|p| ((p[u] - centre[u]).hypot(p[v] - centre[v]) - r).abs() > TOL)
            {
                return Err(fail("members do not lie on one circle"));
            }
            let mut angles: Vec<_> = points
                .iter()
                .map(|p| {
                    (p[v] - centre[v])
                        .atan2(p[u] - centre[u])
                        .rem_euclid(std::f64::consts::TAU)
                })
                .collect();
            angles.sort_by(f64::total_cmp);
            let pitch = std::f64::consts::TAU / points.len() as f64;
            if (0..angles.len()).any(|i| {
                ((angles[(i + 1) % angles.len()] - angles[i]).rem_euclid(std::f64::consts::TAU)
                    - pitch)
                    .abs()
                    > 1e-5
            }) {
                return Err(fail("a counted bolt circle requires equal angular spacing"));
            }
            json!({"diameter":2.*r,"center":local_point(a,centre)})
        }
    };
    faces.sort_unstable();
    faces.dedup();
    let result = HolePattern {
        id: id.into(),
        arrangement: p.arrangement.name().into(),
        members,
        faces,
        parameters: object(params),
    };
    holes::established_with(
        &result,
        &reads.iter().collect::<Vec<_>>(),
        &holes::bosses(a),
    )
    .map_err(|why| fail(&why))?;
    Ok(result)
}

/// A correction's explicit provenance, including corrections producing no annotation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applied {
    pub id: String,
    pub from_spec: Vec<String>,
    pub basis: String,
}
fn marker(a: &Analysis) -> Option<&Record> {
    a.recognition
        .as_ref()?
        .evidence
        .iter()
        .find(|r| r.family == MARKER)
}
fn signature(s: &Spec) -> String {
    let actions: Vec<_> = s
        .features
        .iter()
        .filter(|(_, f)| authored(f) || f.anchor.is_some())
        .map(|(id, f)| json!([id, f.suppress, f.split, f.reclassify, f.pattern, f.anchor]))
        .collect();
    digest(&serde_json::to_vec(&json!([actions, s.feature_declarations])).unwrap())
}
/// Resolve corrections into a cloned inventory; the caller passes this same inventory to
/// demand, dimensions, schedules and independent lint. No lint check is bypassed.
pub fn effective<'a>(a: &'a Analysis, s: &Spec) -> Result<Cow<'a, Analysis>, Invalid> {
    if !authors(s) {
        return Ok(Cow::Borrowed(a));
    }
    let bound = bind(a, s.clone())?;
    let s = &bound;
    let signature = signature(s);
    if let Some(m) = marker(a) {
        if m.subtype == signature {
            return Ok(Cow::Borrowed(a));
        }
        return Err(invalid(
            "features",
            "apply changed corrections to the original Analysis",
        ));
    }
    let mut out = a.clone();
    let mut targets = Map::new();
    // Resolve all original identities before any mutation. This also makes action order irrelevant.
    let mut resolved = BTreeMap::new();
    for (id, f) in &s.features {
        if authored(f) && f.pattern.is_none() {
            let actual = resolved_id(a, s, id)?;
            targets.insert(id.clone(), json!(target_faces(a, &actual).unwrap()));
            resolved.insert(id.clone(), actual);
        }
        if let Some(p) = &f.pattern {
            for m in &p.members {
                if !m.starts_with("f:declared:") {
                    let actual = resolved_id(a, s, m)?;
                    targets.insert(m.clone(), json!(target_faces(a, &actual).unwrap()));
                    resolved.insert(m.clone(), actual);
                }
            }
        }
    }
    for (id, f) in &s.features {
        if f.pattern.is_some() || !authored(f) {
            continue;
        }
        let actual = &resolved[id];
        let fi = out.features.iter().position(|f| &f.id == actual);
        let pi = out
            .recognition
            .as_ref()
            .and_then(|r| r.patterns.iter().position(|p| &p.id == actual));
        if f.split {
            if let Some(i) = pi {
                out.recognition.as_mut().unwrap().patterns.remove(i);
            } else if fi.is_some_and(|i| out.features[i].kind == "slot_pattern") {
                out.features.remove(fi.unwrap());
            } else {
                return Err(invalid(
                    &format!("features/{id}/split"),
                    "only a recognised hole or slot pattern can be split",
                ));
            }
        } else if f.suppress {
            let i = fi.ok_or_else(|| {
                invalid(
                    &format!("features/{id}/suppress"),
                    "use split to dissolve a pattern",
                )
            })?;
            out.features.remove(i);
            // A partial pattern is no longer the recognised pattern: retain its other members singly.
            if let Some(r) = &mut out.recognition {
                r.patterns.retain(|p| !p.members.contains(actual));
            }
            out.features.retain(|f| {
                f.kind != "slot_pattern"
                    || !f
                        .parameters
                        .get("slots")
                        .and_then(Value::as_array)
                        .is_some_and(|v| v.contains(&json!(actual)))
            });
            out.blend_sites.remove(actual);
        } else if let Some(kind) = f.reclassify {
            let i = fi.ok_or_else(|| {
                invalid(
                    &format!("features/{id}/reclassify"),
                    "a pattern has members, not a physical surface; split it first",
                )
            })?;
            let original = &out.features[i];
            if original.kind == kind.name()
                && (kind != Kind::Pocket
                    || matches!(
                        crate::lint::section_recesses::grammar(original),
                        Ok(crate::lint::section_recesses::Grammar::Pocket(_))
                    ))
            {
                continue;
            }
            if !matches!(
                (original.kind.as_str(), kind),
                ("boss", Kind::TurnedStep)
                    | ("turned_step", Kind::Boss)
                    | ("slot", Kind::Pocket)
                    | ("section_recess", Kind::Slot)
                    | ("section_recess", Kind::Pocket)
            ) {
                return Err(invalid(
                    &format!("features/{id}/reclassify"),
                    "supported reinterpretations are boss ↔ turned_step and slot ↔ pocket; other changes would reverse material or invent geometry",
                ));
            }
            let converted = derive(a, actual, kind, original.faces.clone())?;
            let duplicate_ids: Vec<_> = out
                .features
                .iter()
                .filter(|other| {
                    other.id != *actual
                        && other.kind == converted.kind
                        && other.faces == converted.faces
                })
                .map(|f| f.id.clone())
                .collect();
            if duplicate_ids.iter().any(|id| s.features.contains_key(id)) {
                return Err(invalid(
                    &format!("features/{id}/reclassify"),
                    "the target interpretation already has authored requirements on the same faces; keep that feature or combine the requirements explicitly",
                ));
            }
            out.features[i] = converted;
            // A boss and turned band can be two readings of the same cylindrical face.
            // The explicit interpretation owns it once, under the selected feature's ID.
            out.features.retain(|f| !duplicate_ids.contains(&f.id));
            if let Some(r) = &mut out.recognition {
                r.patterns.retain(|p| !p.members.contains(actual));
            }
            out.features.retain(|f| {
                f.kind != "slot_pattern"
                    || !f
                        .parameters
                        .get("slots")
                        .and_then(Value::as_array)
                        .is_some_and(|v| v.contains(&json!(actual)))
            });
        }
    }
    for d in &s.feature_declarations {
        if out.features.iter().any(|f| f.id == d.id) {
            return Err(invalid(&d.id, "declaration id already exists"));
        }
        let faces = resolve_faces(a, &d.faces, &format!("feature_declarations/{}", d.id))?;
        if out.features.iter().any(|f| f.faces == faces) {
            return Err(invalid(
                &d.id,
                "these faces already have a feature; reclassify it, or suppress it explicitly before declaring a replacement",
            ));
        }
        out.features.push(derive(a, &d.id, d.kind, faces)?);
    }
    for (id, f) in &s.features {
        if let Some(p) = &f.pattern {
            let members = p
                .members
                .iter()
                .map(|m| resolved.get(m).cloned().unwrap_or_else(|| m.clone()))
                .collect::<Vec<_>>();
            // Explicit grouping replaces any automatic group touching these members.
            let grouped = pattern(&out, id, p, members.clone())?;
            let r = out.recognition.get_or_insert_with(|| Recognition {
                recogniser: "authored corrections".into(),
                frame: crate::bridge::native::IDENTITY,
                evidence: vec![],
                refusals: vec![],
                patterns: vec![],
            });
            r.patterns
                .retain(|old| !old.members.iter().any(|m| members.contains(m)));
            if r.patterns.iter().any(|old| old.id == *id) {
                return Err(invalid(
                    &format!("features/{id}/pattern"),
                    "id is already owned by an unrelated recognised pattern; choose another name",
                ));
            }
            r.patterns.push(grouped);
        }
    }
    let r = out.recognition.get_or_insert_with(|| Recognition {
        recogniser: "authored corrections".into(),
        frame: crate::bridge::native::IDENTITY,
        evidence: vec![],
        refusals: vec![],
        patterns: vec![],
    });
    r.evidence.push(Record {
        family: MARKER.into(),
        subtype: signature,
        faces: vec![],
        parameters: object(json!({"targets":targets})),
    });
    Ok(Cow::Owned(out))
}

pub(crate) fn report(a: &Analysis, p: &mut Plan) {
    if !authors(&p.spec) {
        return;
    }
    let mut changes = Vec::new();
    for (id, f) in &p.spec.features {
        let action = if f.suppress {
            "suppress"
        } else if f.split {
            "split"
        } else if f.reclassify.is_some() {
            "reclassify"
        } else if f.pattern.is_some() {
            "pattern"
        } else {
            continue;
        };
        changes.push(Applied{id:id.clone(),from_spec:vec![format!("features/{id}/{action}")],basis:format!("authored recognition correction: {action} {id}; remaining output is checked against the corrected inventory")});
    }
    for d in &p.spec.feature_declarations {
        changes.push(Applied{id:d.id.clone(),from_spec:vec![format!("feature_declarations/{}",d.id)],basis:"authored feature; kind chosen by the author, parameters measured from the named faces".into()});
    }
    for d in &mut p.dimensions {
        for c in &changes {
            let directly = d.measurements.iter().any(|m| m.feature == c.id);
            let grouped = p
                .spec
                .features
                .get(&c.id)
                .and_then(|f| f.pattern.as_ref())
                .is_some_and(|g| {
                    g.members
                        .iter()
                        .any(|m| d.measurements.iter().any(|v| &v.feature == m))
                });
            let split = p.spec.features.get(&c.id).is_some_and(|f| f.split)
                && marker(a)
                    .and_then(|m| m.parameters.get("targets"))
                    .and_then(|t| t.get(&c.id))
                    .and_then(Value::as_array)
                    .is_some_and(|faces| {
                        a.features
                            .iter()
                            .filter(|f| d.measurements.iter().any(|m| m.feature == f.id))
                            .any(|f| f.faces.iter().any(|i| faces.contains(&json!(i))))
                    });
            if directly || grouped || split {
                d.default = false;
                d.from_spec.extend(c.from_spec.clone());
                d.basis = format!("{}; {}", d.basis, c.basis);
            }
        }
    }
    p.recognition_corrections = changes;
}
