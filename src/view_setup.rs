//! Authored cameras and derived views. All references are in model space.
use crate::model::{Analysis, Invalid, Spec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Orientation {
    /// Signed principal axis, stable planar face fingerprint, or feature ID.
    pub from: String,
    #[serde(default)]
    pub rotation: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuxiliarySpec {
    pub face: String,
    pub from: String,
    #[serde(default = "one")]
    pub scale_factor: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialSpec {
    pub around: crate::detail::Around,
    pub from: String,
    #[serde(default = "one")]
    pub scale_factor: f64,
}
fn one() -> f64 {
    1.0
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub direction: [f64; 3],
    pub up: [f64; 3],
}

fn fail(path: &str, message: impl std::fmt::Display) -> Invalid {
    // Required members cannot be removed alone: remove their authored view form.
    let remove = path
        .strip_suffix("/around")
        .or_else(|| path.strip_suffix("/from"))
        .unwrap_or(path);
    Invalid {
        message: format!(
            "{path}: {message}; choose a current feature ID / unique planar face fingerprint, correct the source view, or remove {remove}."
        ),
        choices: vec![format!("remove {remove}")],
    }
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f64; 3], path: &str) -> Result<[f64; 3], Invalid> {
    let n = dot(v, v).sqrt();
    if !n.is_finite() || n < 1e-9 {
        return Err(fail(path, "direction is degenerate"));
    }
    Ok(v.map(|x| x / n))
}

/// Resolve both published face-key vocabularies; never interpret a face index as identity.
fn face(a: &Analysis, id: &str, path: &str) -> Result<usize, Invalid> {
    let mut matches = Vec::new();
    for i in 0..a.face_attachments.len() {
        if crate::recognition_corrections::face_fingerprint(a, i).is_ok_and(|f| f == id) {
            matches.push(i);
        }
    }
    if let Some(records) = a.edge_ink.as_ref().and_then(|ink| ink.references.get(id)) {
        for r in records {
            if let crate::model::dimension_geometry::Reference::Plane { bounds, normal, .. } = r {
                if records.len() != 1 {
                    return Err(fail(path, "ambiguous face fingerprint"));
                }
                for (i, f) in a.face_attachments.iter().enumerate() {
                    if f.bounds.is_some_and(|b| {
                        (0..3).all(|k| {
                            (b[k] - bounds[0][k]).abs() < 1e-5
                                && (b[k + 3] - bounds[1][k]).abs() < 1e-5
                        })
                    }) && f.normal.is_some_and(|n| dot(n, *normal).abs() > 1.0 - 1e-6)
                    {
                        matches.push(i);
                    }
                }
            }
        }
    }
    matches.sort_unstable();
    matches.dedup();
    if matches.len() != 1 {
        return Err(fail(
            path,
            format!(
                "{id:?} resolves to {} faces (missing or ambiguous)",
                matches.len()
            ),
        ));
    }
    Ok(matches[0])
}

fn normal(a: &Analysis, id: &str, path: &str) -> Result<[f64; 3], Invalid> {
    let faces = if let Some(f) = a.features.iter().find(|f| f.id == id) {
        f.faces.clone()
    } else {
        vec![face(a, id, path)?]
    };
    let mut normals = Vec::new();
    for i in faces {
        if let Some(f) = a.face_attachments.get(i)
            && f.surface.as_deref() == Some("plane")
            && let Some(n) = f.normal
        {
            let n = unit(n, path)?;
            if !normals.iter().any(|v| dot(*v, n) > 1.0 - 1e-8) {
                normals.push(n);
            }
        }
    }
    if normals.len() == 1 {
        return Ok(normals[0]);
    }
    if let Some(axis) = a
        .features
        .iter()
        .find(|f| f.id == id)
        .and_then(|f| f.fingerprint.axis)
    {
        return unit(axis, path);
    }
    Err(fail(
        path,
        format!("{id:?} does not identify one planar normal or feature axis"),
    ))
}

pub(crate) fn check(s: &Spec) -> Result<(), Invalid> {
    for (name, v) in &s.views {
        let path = format!("views/{name}");
        if (v.orientation.is_some() || v.principal_views.is_some()) && name != "front" {
            return Err(fail(
                &path,
                "orientation and principal_views belong under views/front",
            ));
        }
        if let Some(o) = &v.orientation
            && (!o.rotation.is_finite() || o.rotation.abs() > 360.0)
        {
            return Err(fail(
                &format!("{path}/orientation"),
                "rotation must be finite degrees in -360..360",
            ));
        }
        if let Some(names) = &v.principal_views {
            let unique: std::collections::BTreeSet<_> = names.iter().collect();
            if names.is_empty()
                || unique.len() != names.len()
                || names
                    .iter()
                    .any(|n| !["front", "plan", "side"].contains(&n.as_str()))
            {
                return Err(fail(
                    &format!("{path}/principal_views"),
                    "use a nonempty set of distinct front, plan and side names",
                ));
            }
            for n in names {
                if s.views.get(n).is_some_and(|v| v.omit) {
                    return Err(fail(
                        &format!("{path}/principal_views"),
                        format!("{n} is both requested and omitted"),
                    ));
                }
            }
            for n in ["front", "plan", "side"] {
                if !names.iter().any(|v| v == n)
                    && s.views
                        .get(n)
                        .is_some_and(|v| !v.omit && (v.place.is_some() || v.sheet.is_some()))
                {
                    return Err(fail(
                        &format!("{path}/principal_views"),
                        format!("{n} has placement/sheet intent but is excluded"),
                    ));
                }
            }
        }
        let derived = usize::from(v.detail.is_some())
            + usize::from(v.auxiliary.is_some())
            + usize::from(v.partial.is_some());
        if derived > 1 || (derived > 0 && !crate::detail::NAMES.contains(&name.as_str())) {
            return Err(fail(
                &path,
                "choose exactly one detail, auxiliary or partial in a detail_a–detail_h letter slot",
            ));
        }
        for scale in v
            .auxiliary
            .iter()
            .map(|a| a.scale_factor)
            .chain(v.partial.iter().map(|p| p.scale_factor))
        {
            if !scale.is_finite() || scale <= 0.0 {
                return Err(fail(&path, "scale_factor must be finite and positive"));
            }
        }
    }
    Ok(())
}

pub(crate) fn requested(s: &Spec, name: &str) -> Option<bool> {
    s.views
        .get("front")?
        .principal_views
        .as_ref()
        .map(|names| names.iter().any(|n| n == name))
}
pub(crate) fn oriented(s: &Spec) -> bool {
    s.views
        .get("front")
        .is_some_and(|v| v.orientation.is_some())
}
pub(crate) fn authored(v: &crate::model::ViewSpec) -> bool {
    v.detail.is_some() || v.auxiliary.is_some() || v.partial.is_some()
}

pub(crate) fn cameras(a: &Analysis, s: &Spec) -> Result<[Camera; 3], Invalid> {
    let Some(o) = s.views.get("front").and_then(|v| v.orientation.as_ref()) else {
        return Ok(std::array::from_fn(|k| Camera {
            direction: crate::model::VIEWS[k].1,
            up: crate::model::VIEWS[k].2,
        }));
    };
    let path = "views/front/orientation";
    let d = match o.from.as_str() {
        "x" | "+x" => [1., 0., 0.],
        "-x" => [-1., 0., 0.],
        "y" | "+y" => [0., 1., 0.],
        "-y" => [0., -1., 0.],
        "z" | "+z" => [0., 0., 1.],
        "-z" => [0., 0., -1.],
        id => normal(a, id, path)?,
    };
    let seed = if d[2].abs() > 1.0 - 1e-8 {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let up = unit(std::array::from_fn(|k| seed[k] - dot(seed, d) * d[k]), path)?;
    let right = cross(up, d);
    let (sin, cos) = match o.rotation.rem_euclid(360.) {
        0. => (0., 1.),
        90. => (1., 0.),
        180. => (0., -1.),
        270. => (-1., 0.),
        _ => o.rotation.to_radians().sin_cos(),
    };
    // Positive rotation turns the drawing counterclockwise on the page.
    let up = std::array::from_fn(|k| up[k] * cos + right[k] * sin);
    let right = cross(up, d);
    Ok([
        Camera { direction: d, up },
        Camera {
            direction: up,
            up: d.map(|v| -v),
        },
        Camera {
            direction: right,
            up,
        },
    ])
}

pub(crate) fn camera(a: &Analysis, s: &Spec, name: &str) -> Result<Camera, Invalid> {
    if let Some(k) = ["front", "plan", "side"].iter().position(|n| *n == name) {
        if !crate::detail::source_available(a, s, name) {
            return Err(fail(
                &format!("views/{name}"),
                format!(
                    "source view v:{name} is excluded, omitted or automatically reduced; include {name} in views/front/principal_views or set views/{name} to {{}}"
                ),
            ));
        }
        return Ok(cameras(a, s)?[k]);
    }
    if name.starts_with("section_")
        && name
            == format!(
                "section_{0}{0}",
                crate::detail::section_label(s).to_lowercase()
            )
        && crate::detail::source_available(a, s, name)
    {
        return Ok(Camera {
            direction: [0., -1., 0.],
            up: [0., 0., 1.],
        });
    }
    if let Some(aux) = s.views.get(name).and_then(|v| v.auxiliary.as_ref()) {
        if s.views[name].omit && s.views[name].sheet.is_none_or(|n| n <= 1) {
            return Err(fail(&format!("views/{name}"), "source is omitted"));
        }
        return auxiliary_camera(a, s, name, aux);
    }
    Err(fail(
        &format!("views/{name}"),
        "source must be a shown principal or auxiliary view",
    ))
}
fn auxiliary_camera(
    a: &Analysis,
    s: &Spec,
    name: &str,
    aux: &AuxiliarySpec,
) -> Result<Camera, Invalid> {
    let path = format!("views/{name}/auxiliary");
    if !["front", "plan", "side"].contains(&aux.from.as_str()) {
        return Err(fail(&path, "from must name a principal edge view"));
    }
    let source = camera(a, s, &aux.from)?;
    let i = face(a, &aux.face, &path)?;
    let f = &a.face_attachments[i];
    if f.surface.as_deref() != Some("plane") {
        return Err(fail(&path, "true shape requires a planar face"));
    }
    let d = unit(
        f.normal
            .ok_or_else(|| fail(&path, "face lacks its normal; analyse again"))?,
        &path,
    )?;
    if dot(d, source.direction).abs() > 1e-6 {
        return Err(fail(
            &path,
            format!(
                "face {:?} is not edge-on in {:?}; choose a principal view perpendicular to its normal",
                aux.face, aux.from
            ),
        ));
    }
    // Unfold by the shortest quarter turn about the face's edge in the source.
    // That common edge keeps its paper angle; a rotated front rotates its auxiliary too.
    let edge = unit(cross(source.direction, d), &path)?;
    let turned = cross(edge, source.up);
    let up = std::array::from_fn(|k| turned[k] + edge[k] * dot(edge, source.up));
    Ok(Camera {
        direction: d,
        up: unit(up, &path)?,
    })
}

pub(crate) fn validate(a: &Analysis, s: &Spec) -> Result<(), Invalid> {
    check(s)?;
    if oriented(s) {
        cameras(a, s)?;
    }
    for (name, v) in &s.views {
        if !authored(v) {
            continue;
        }
        if let Some(aux) = &v.auxiliary {
            auxiliary_camera(a, s, name, aux)?;
        }
        if let Some(p) = &v.partial {
            if !["front", "plan", "side"].contains(&p.from.as_str())
                && !s.views.get(&p.from).is_some_and(|v| v.auxiliary.is_some())
            {
                return Err(fail(
                    &format!("views/{name}/partial/from"),
                    "source must be a principal or authored auxiliary view",
                ));
            }
            let c = camera(a, s, &p.from)?;
            if s.views
                .get(&p.from)
                .is_some_and(|v| v.omit && v.sheet.is_none_or(|n| n <= 1))
            {
                return Err(fail(&format!("views/{name}/partial"), "source is omitted"));
            }
            let b = region(a, name, &p.around)?;
            let (size, _) = crate::model::envelope_extent(
                b,
                &haecceity::hlr::View::new(c.direction, c.up).unwrap(),
            );
            if size.iter().any(|n| *n < 1e-6) {
                return Err(fail(
                    &format!("views/{name}/partial"),
                    "region is degenerate in its source view",
                ));
            }
        }
    }
    Ok(())
}

fn region(
    a: &Analysis,
    name: &str,
    around: &crate::detail::Around,
) -> Result<[[f64; 3]; 2], Invalid> {
    let path = format!("views/{name}/partial/around");
    if around.members().is_empty() {
        return Err(fail(&path, "around needs at least one feature or face"));
    }
    let mut out = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for id in around.members() {
        let b = if let Some(f) = a.features.iter().find(|f| f.id == *id) {
            crate::detail::feature_bounds(a, f)
        } else {
            let i = face(a, id, &path)?;
            let b = a.face_attachments[i]
                .bounds
                .ok_or_else(|| fail(&path, "face lacks bounds; analyse again"))?;
            [[b[0], b[1], b[2]], [b[3], b[4], b[5]]]
        };
        for k in 0..3 {
            out[0][k] = out[0][k].min(b[0][k]);
            out[1][k] = out[1][k].max(b[1][k]);
        }
    }
    Ok(out)
}

pub(crate) fn requests(a: &Analysis, s: &Spec, scale: f64) -> Vec<(String, crate::detail::Detail)> {
    let mut out = Vec::new();
    // Auxiliary parents precede partials independently of author letter order.
    for partial in [false, true] {
        for (name, v) in &s.views {
            if v.omit {
                continue;
            }
            let (kind, source, bounds, marked, camera, factor) =
                if !partial && let Some(aux) = &v.auxiliary {
                    let i = face(a, &aux.face, name).expect("validated auxiliary");
                    let b = a.face_attachments[i].bounds.expect("validated face");
                    (
                        "auxiliary",
                        aux.from.clone(),
                        a.envelope,
                        [[b[0], b[1], b[2]], [b[3], b[4], b[5]]],
                        auxiliary_camera(a, s, name, aux).expect("validated auxiliary"),
                        aux.scale_factor,
                    )
                } else if partial && let Some(p) = &v.partial {
                    let b = region(a, name, &p.around).expect("validated partial");
                    (
                        "partial",
                        p.from.clone(),
                        b,
                        b,
                        camera(a, s, &p.from).expect("validated source"),
                        p.scale_factor,
                    )
                } else {
                    continue;
                };
            out.push((
                name.clone(),
                crate::detail::Detail {
                    label: name.trim_start_matches("detail_").to_uppercase(),
                    axis: 2,
                    source_view: format!("v:{source}"),
                    source_cut: None,
                    camera: Some(camera),
                    kind: kind.into(),
                    bounds,
                    marked_bounds: marked,
                    dimensions: Vec::new(),
                    scale: scale * factor,
                    authored: true,
                    partial_profile: kind == "partial",
                },
            ));
        }
    }
    out
}

pub(crate) fn report(s: &Spec, views: &mut [crate::model::PlannedView]) {
    if s.views.is_empty() {
        return;
    }
    for v in views {
        let name = v.id.trim_start_matches("v:");
        if ["front", "plan", "side"].contains(&name) {
            if let Some(o) = s.views.get("front").and_then(|v| v.orientation.as_ref()) {
                v.from_spec.push("views/front/orientation".into());
                v.default = false;
                v.basis += &format!(
                    "; authored front from {} rotated {} degrees; principal cameras follow that frame",
                    o.from, o.rotation
                );
            }
            if requested(s, name).is_some() {
                v.from_spec.push("views/front/principal_views".into());
                v.default = false;
                v.basis += &format!(
                    "; explicit principal view set: {}",
                    s.views["front"]
                        .principal_views
                        .as_ref()
                        .unwrap()
                        .join(", ")
                );
            }
        }
        if let Some(auth) = s.views.get(name) {
            for (key, yes) in [
                ("detail", auth.detail.is_some()),
                ("auxiliary", auth.auxiliary.is_some()),
                ("partial", auth.partial.is_some()),
            ] {
                if yes {
                    v.from_spec.push(format!("views/{name}/{key}"));
                }
            }
        }
    }
}

pub(crate) fn copy_setup(s: &Spec, base: &mut Spec) {
    for (name, v) in &s.views {
        if v.orientation.is_some() || v.principal_views.is_some() || authored(v) {
            // A pin's canonical plan must admit the authored view's assigned sheet.
            base.sheets.clone_from(&s.sheets);
            let mut v = v.clone();
            v.place = None;
            base.views.insert(name.clone(), v);
        }
    }
}
pub(crate) fn layout_size(a: &Analysis, s: &Spec, cameras: &[Camera; 3]) -> [f64; 3] {
    let original = [0, 1, 2].map(|k| a.envelope[1][k] - a.envelope[0][k]);
    if !oriented(s) {
        return original;
    }
    let front = haecceity::hlr::View::new(cameras[0].direction, cameras[0].up).unwrap();
    let plan = haecceity::hlr::View::new(cameras[1].direction, cameras[1].up).unwrap();
    let f = crate::model::envelope_extent(a.envelope, &front).0;
    let p = crate::model::envelope_extent(a.envelope, &plan).0;
    // Automatic sections retain their physical cutting plane and camera.
    if crate::demand::read(a).section.is_some() {
        [f[0].max(original[0]), p[1], f[1].max(original[2])]
    } else {
        [f[0], p[1], f[1]]
    }
}

/// Feature references join the existing face-anchor rebinding transaction.
pub(crate) fn feature_ids(s: &Spec) -> Vec<String> {
    let mut out = Vec::new();
    for v in s.views.values() {
        if let Some(o) = &v.orientation {
            out.push(o.from.clone());
        }
        if let Some(p) = &v.partial {
            out.extend(p.around.members().iter().cloned());
        }
    }
    out.retain(|id| id.starts_with("f:") && !id.starts_with("f:declared:"));
    out.sort();
    out.dedup();
    out
}
pub(crate) fn remap(s: &mut Spec, ids: &std::collections::BTreeMap<String, String>) {
    let replace = |id: &mut String| {
        if let Some(new) = ids.get(id) {
            *id = new.clone();
        }
    };
    for v in s.views.values_mut() {
        if let Some(o) = &mut v.orientation {
            replace(&mut o.from);
        }
        if let Some(p) = &mut v.partial {
            match &mut p.around {
                crate::detail::Around::One(id) => replace(id),
                crate::detail::Around::Many(ids) => ids.iter_mut().for_each(&replace),
            }
        }
    }
}

/// A physical section remains on its model plane when the principal frame changes.
pub(crate) fn section_parent(s: &Spec, cameras: &[Camera; 3]) -> Option<String> {
    [1, 0, 2]
        .into_iter()
        .find(|&k| {
            let name = ["front", "plan", "side"][k];
            requested(s, name) != Some(false)
                && !s.views.get(name).is_some_and(|v| v.omit)
                && cameras[k].direction[1].abs() < 1e-6
        })
        .map(|k| format!("v:{}", ["front", "plan", "side"][k]))
}
pub(crate) fn cutting_segment(
    a: &Analysis,
    v: &crate::model::PlannedView,
    c: &crate::model::Cut,
) -> [[f64; 2]; 2] {
    let view = haecceity::hlr::View::new(v.direction, v.up).unwrap();
    let (_, centre) = crate::model::extent(a, &view);
    let p = view.map(c.origin);
    let at = [0, 1].map(|k| v.at[k] + v.scale * (p[k] - centre[k]));
    let n = view.map(c.normal);
    let length = n[0].hypot(n[1]);
    let tangent = [n[1] / length, -n[0] / length];
    let mut span = [f64::INFINITY, f64::NEG_INFINITY];
    for i in 0..8 {
        let p = view.map([0, 1, 2].map(|k| a.envelope[(i >> k) & 1][k]));
        let t = (p[0] - centre[0]) * tangent[0] + (p[1] - centre[1]) * tangent[1];
        span[0] = span[0].min(t * v.scale);
        span[1] = span[1].max(t * v.scale);
    }
    [0, 1].map(|k| {
        [0, 1].map(|j| {
            at[j]
                + tangent[j]
                    * (span[k]
                        + if k == 0 {
                            -crate::section::OVERRUN
                        } else {
                            crate::section::OVERRUN
                        })
        })
    })
}

/// A named viewing arrow on the parent's edge view, pointing towards the face.
pub(crate) fn auxiliary_marker(
    a: &Analysis,
    d: &crate::detail::Detail,
    source: &crate::model::PlannedView,
) -> (Vec<crate::output::Curve>, [f64; 2]) {
    use crate::output::Curve;
    let camera = haecceity::hlr::View::new(source.direction, source.up).unwrap();
    let centre = crate::detail::crop_extent(a, source).1;
    let p = camera.map([0, 1, 2].map(|k| (d.marked_bounds[0][k] + d.marked_bounds[1][k]) / 2.));
    let tip = [0, 1].map(|k| source.at[k] + source.scale * (p[k] - centre[k]));
    let n = camera.map(d.camera.expect("auxiliary camera").direction);
    let tail = [0, 1].map(|k| tip[k] + 10. * n[k]);
    let wings = [-1., 1.].map(|sign| {
        [
            tip[0] + 2.7 * n[0] - sign * 0.9 * n[1],
            tip[1] + 2.7 * n[1] + sign * 0.9 * n[0],
        ]
    });
    (
        vec![
            Curve::Line(tail, tip),
            Curve::Line(tip, wings[0]),
            Curve::Line(tip, wings[1]),
        ],
        [tail[0] + 3. * n[0], tail[1] + 3. * n[1]],
    )
}

pub(crate) fn unplaceable(e: &Invalid) -> bool {
    e.message.starts_with("Authored derived view ") && e.message.contains("has no room")
}

/// Recovery suggestions edit the explicit set rather than an ineffective omit flag.
pub(crate) fn include_principal(s: &Spec, name: &str) -> crate::items::Op {
    let names: Vec<_> = ["front", "plan", "side"]
        .into_iter()
        .filter(|n| *n == name || requested(s, n) == Some(true))
        .collect();
    crate::items::Op::set("views/front/principal_views", serde_json::json!(names))
}
pub(crate) fn reference_error(s: &Spec, id: &str, original: Invalid) -> Invalid {
    for (name, v) in &s.views {
        let field = if v.orientation.as_ref().is_some_and(|o| o.from == id) {
            "orientation"
        } else if v
            .partial
            .as_ref()
            .is_some_and(|p| p.around.members().iter().any(|m| m == id))
        {
            "partial"
        } else {
            continue;
        };
        let path = format!("views/{name}/{field}");
        return Invalid {
            message: format!(
                "{path}: {} Reattach {id:?} to a current feature ID, or remove {path}.",
                original.message
            ),
            choices: vec![format!("remove {path}")],
        };
    }
    original
}
