//! ISO 6410 depiction of recognised physical screw threads. The source Part and
//! recognition remain immutable; a drawing-only carrier replaces the claimed flanks.
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, TAU};

use haecceity::brep::{Edge, Face, Loop, Part, Solid};
use haecceity::geom::{self, Frame, Surface, V3};
use haecceity::hlr::View;
use serde::{Deserialize, Serialize};

use crate::bridge::Feature;
use crate::model::{Analysis, PlannedView, Spec};
use crate::output::{Curve, Item};

/// Author intent only; measured geometry always remains in the Analysis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depiction: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Thread<'a> {
    pub feature: &'a Feature,
    pub on: V3,
    pub axis: V3,
    pub internal: bool,
    pub major: f64,
    pub minor: f64,
    pub pitch: f64,
    pub span: [f64; 2],
    pub left: bool,
    pub starts: u64,
}

impl<'a> Thread<'a> {
    pub(crate) fn read(f: &'a Feature) -> Result<Self, &'static str> {
        let p = &f.parameters;
        let number = |name: &str| {
            p.get(name)
                .and_then(|v| v.as_f64())
                .filter(|x| x.is_finite())
        };
        let point = |v: &serde_json::Value| -> Option<V3> {
            let a = v.as_array()?;
            (a.len() == 3).then_some(())?;
            Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
        };
        let on = p
            .get("axis")
            .and_then(|a| point(&a["point"]))
            .ok_or("missing axis point")?;
        let axis = p
            .get("axis")
            .and_then(|a| point(&a["direction"]))
            .and_then(geom::unit)
            .ok_or("missing axis direction")?;
        let major = number("major_diameter").ok_or("missing major diameter")?;
        let minor = number("minor_diameter").ok_or("missing minor diameter")?;
        let pitch = number("pitch").ok_or("missing pitch")?;
        let span = [
            number("start").ok_or("missing start")?,
            number("end").ok_or("missing end")?,
        ];
        if !(major > minor && minor > 0. && pitch > 0. && span[1] > span[0]) || f.faces.is_empty() {
            return Err("invalid diameters, pitch, extent or face claims");
        }
        let internal = match p.get("side").and_then(|v| v.as_str()) {
            Some("internal") => true,
            Some("external") => false,
            _ => return Err("invalid material side"),
        };
        let left = match p.get("hand").and_then(|v| v.as_str()) {
            Some("left") => true,
            Some("right") => false,
            _ => return Err("invalid hand"),
        };
        let starts = p
            .get("starts")
            .and_then(|v| v.as_u64())
            .filter(|s| *s > 0)
            .ok_or("invalid starts")?;
        Ok(Self {
            feature: f,
            on,
            axis,
            internal,
            major,
            minor,
            pitch,
            span,
            left,
            starts,
        })
    }
    pub(crate) fn at(&self, t: f64) -> V3 {
        geom::add(self.on, geom::scale(self.axis, t))
    }
    pub(crate) fn crest(&self) -> f64 {
        (if self.internal {
            self.minor
        } else {
            self.major
        }) / 2.
    }
    pub(crate) fn root(&self) -> f64 {
        (if self.internal {
            self.major
        } else {
            self.minor
        }) / 2.
    }
    pub(crate) fn enabled(&self, spec: &Spec) -> bool {
        spec.features
            .get(&self.feature.id)
            .and_then(|f| f.modelled_thread.as_ref())
            .and_then(|t| t.depiction)
            != Some(false)
    }
    pub(crate) fn designation(&self) -> (String, bool) {
        // External crests match nominal directly. An internal model can truncate its
        // roots: allow up to 0.26 P (capped at 0.4 mm) below nominal, never above it.
        // This is an inference policy, not an ISO manufacturing tolerance/class.
        let below = if self.internal {
            (0.26 * self.pitch).clamp(0.02, 0.4)
        } else {
            0.02
        };
        let candidates: Vec<_> = metric_pairs()
            .into_iter()
            .filter(|&(d, p)| {
                (p - self.pitch).abs() <= 0.01 + 1e-10
                    && d - self.major >= -0.02 - 1e-10
                    && d - self.major <= below + 1e-10
            })
            .collect();
        let standard = candidates.len() == 1;
        let mut text = if let [(d, p)] = candidates.as_slice() {
            if self.starts == 1 {
                format!("M{}×{}", crate::dims::fmt(*d), crate::dims::fmt(*p))
            } else {
                format!(
                    "M{}×Ph{}P{}",
                    crate::dims::fmt(*d),
                    crate::dims::fmt(*p * self.starts as f64),
                    crate::dims::fmt(*p)
                )
            }
        } else {
            format!(
                "Thread Ø{} × P{}",
                crate::dims::fmt(self.major),
                crate::dims::fmt(self.pitch)
            )
        };
        if !standard && self.starts > 1 {
            text += &format!(" {} STARTS", self.starts);
        }
        if self.left {
            text += " LH";
        }
        (text, standard)
    }
}

// ISO 261:1998 tables 1/2 (ISO 262 is the selected subset). Coarse entries
// shared with authored callouts; fine pairs are explicit, never a pitch formula.
fn metric_pairs() -> Vec<(f64, f64)> {
    let mut pairs = crate::spec_notes::COARSE.to_vec();
    let fine: &[(f64, &[f64])] = &[
        (1., &[0.2]),
        (1.1, &[0.2]),
        (1.2, &[0.2]),
        (1.4, &[0.2]),
        (1.6, &[0.2]),
        (1.8, &[0.2]),
        (2., &[0.25]),
        (2.2, &[0.25]),
        (2.5, &[0.35]),
        (3., &[0.35]),
        (3.5, &[0.35]),
        (4., &[0.5]),
        (4.5, &[0.5]),
        (5., &[0.5]),
        (5.5, &[0.5]),
        (6., &[0.75, 0.5]),
        (7., &[0.75]),
        (8., &[1., 0.75]),
        (9., &[1., 0.75]),
        (10., &[1.25, 1., 0.75]),
        (11., &[1., 0.75]),
        (12., &[1.5, 1.25, 1.]),
        (14., &[1.5, 1.]),
        (15., &[1.5, 1.]),
        (16., &[1.5, 1.]),
        (17., &[1.5, 1.]),
        (18., &[2., 1.5, 1.]),
        (20., &[2., 1.5, 1.]),
        (22., &[2., 1.5, 1.]),
        (24., &[2., 1.5, 1.]),
        (25., &[2., 1.5, 1.]),
        (26., &[1.5]),
        (27., &[2., 1.5, 1.]),
        (28., &[2., 1.5, 1.]),
        (30., &[3., 2., 1.5, 1.]),
        (32., &[2., 1.5]),
        (33., &[3., 2., 1.5]),
        (35., &[1.5]),
        (36., &[3., 2., 1.5]),
        (38., &[1.5]),
        (39., &[3., 2., 1.5]),
    ];
    for &(d, ps) in fine {
        pairs.extend(ps.iter().map(|p| (d, *p)));
    }
    for d in [
        40, 42, 45, 48, 50, 52, 55, 56, 58, 60, 62, 64, 65, 68, 70, 72, 75, 76, 78, 80, 82, 85, 90,
        95, 100, 105, 110, 115, 120, 125, 130, 135, 140, 145, 150, 155, 160, 165, 170, 175, 180,
        185, 190, 195, 200, 205, 210, 215, 220, 225, 230, 235, 240, 245, 250, 255, 260, 265, 270,
        275, 280, 285, 290, 295, 300,
    ] {
        for p in [1.5, 2., 3., 4., 6., 8.] {
            let listed = match p {
                1.5 => d <= 80,
                2. => d <= 245,
                3. => d <= 250 && d != 78 && d != 82,
                4. => ![40, 50, 78, 82].contains(&d),
                6. => d >= 68 && d != 75 && d != 78 && d != 82,
                8. => [
                    125, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250, 260, 270,
                    280, 290, 300,
                ]
                .contains(&d),
                _ => false,
            };
            if listed {
                pairs.push((d as f64, p));
            }
        }
        // ISO 261 clause 5.4 / table 1 permits finer pitches on listed diameters.
        for (p, max) in [(3., 300), (2., 200), (1.5, 150), (1., 80)] {
            if d <= max
                && pairs
                    .iter()
                    .filter(|(diameter, _)| *diameter == d as f64)
                    .all(|(_, pitch)| p < *pitch)
            {
                pairs.push((d as f64, p));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    pairs.dedup();
    pairs
}

pub(crate) fn threads(a: &Analysis) -> impl Iterator<Item = Thread<'_>> {
    a.features
        .iter()
        .filter(|f| f.kind == "modelled_thread")
        .filter_map(|f| Thread::read(f).ok())
}

/// A drawing carrier: all unclaimed faces and boundaries are retained. Each
/// claimed surface is replaced by a smooth cylinder at the ISO crest diameter.
/// Its span includes indivisible carrier faces beyond the physical thread extent.
/// The kernel still owns projection, visibility and section/hatch intersections.
pub(crate) fn carrier(part: &Part, a: &Analysis, spec: &Spec, enabled: bool) -> Option<Part> {
    if !enabled {
        return None;
    }
    let ts: Vec<_> = threads(a)
        .filter(|t| t.enabled(spec) && t.feature.faces.iter().all(|&i| i < part.faces.len()))
        .collect();
    if ts.is_empty() {
        return None;
    }
    let claimed: BTreeSet<_> = ts
        .iter()
        .flat_map(|t| t.feature.faces.iter().copied())
        .collect();
    let mut faces = Vec::new();
    let mut mapping = vec![None; part.faces.len()];
    for (i, f) in part.faces.iter().enumerate() {
        if !claimed.contains(&i) {
            mapping[i] = Some(faces.len());
            faces.push(f.clone());
        }
    }
    let mut edges = part.edges.clone();
    let mut solids: Vec<_> = part
        .solids
        .iter()
        .map(|s| Solid {
            faces: s.faces.iter().filter_map(|i| mapping[*i]).collect(),
        })
        .collect();
    for t in ts {
        let r = t.crest();
        let seed = if t.axis[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let x = geom::unit(geom::cross(seed, t.axis)).unwrap();
        let frame = Frame {
            origin: t.on,
            x,
            y: geom::cross(t.axis, x),
            z: t.axis,
        };
        let (lo, hi) = part.extent_along(&t.feature.faces, t.axis);
        let shift = geom::dot(t.on, t.axis);
        let span = [(lo - shift).min(t.span[0]), (hi - shift).max(t.span[1])];
        let base = edges.len();
        let vertex = edges
            .iter()
            .flat_map(|e| [e.vertices.0, e.vertices.1])
            .max()
            .unwrap_or(0)
            + 1;
        for (i, z) in span.into_iter().enumerate() {
            let c = haecceity::geom::Curve::Circle {
                frame: Frame {
                    origin: t.at(z),
                    ..frame
                },
                radius: r,
            };
            let p = c.value(0.);
            edges.push(Edge {
                curve: c.clone(),
                start: p,
                end: p,
                vertices: (vertex + i, vertex + i),
                same_sense: true,
                samples: (0..=64).map(|k| c.value(TAU * k as f64 / 64.)).collect(),
            });
        }
        let p = edges[base].start;
        let q = edges[base + 1].start;
        edges.push(Edge {
            curve: haecceity::geom::Curve::Line {
                origin: p,
                dir: t.axis,
            },
            start: p,
            end: q,
            vertices: (vertex, vertex + 1),
            same_sense: true,
            samples: vec![p, q],
        });
        let solid = t.feature.faces.iter().find_map(|&i| part.faces[i].solid);
        if let Some(s) = solid.and_then(|s| solids.get_mut(s)) {
            s.faces.push(faces.len());
        }
        faces.push(Face {
            surface: Surface::Cylinder { frame, radius: r },
            reversed: t.internal,
            loops: vec![Loop {
                edges: vec![
                    (base, true),
                    (base + 2, true),
                    (base + 1, false),
                    (base + 2, false),
                ],
                vertex: None,
            }],
            solid,
            pcurves: Vec::new(),
        });
    }
    Some(Part::new(faces, edges, solids))
}

/// Extra ISO root and termination curves. Witnesses lie on the carrier crests,
/// so hidden-line tests do not mistake an internal root line inside material for
/// a visible surface. Kept-half clipping and crop clipping apply to these too.
pub(crate) fn ink(
    a: &Analysis,
    spec: &Spec,
    v: &PlannedView,
    part: &Part,
) -> (Vec<Item>, Vec<Item>, Vec<Item>) {
    let (mut thick, mut thin, mut hidden) = (Vec::new(), Vec::new(), Vec::new());
    let ts: Vec<_> = threads(a).filter(|t| t.enabled(spec)).collect();
    if ts.is_empty() || v.line_style.thread_depiction == Some(false) {
        return (thick, thin, hidden);
    }
    let view = View::new(v.direction, v.up).expect("planned view");
    let centre = crate::detail::crop_extent(a, v).1;
    let page = |p: V3| {
        let q = view.map(p);
        [0, 1].map(|k| v.at[k] + v.scale * (q[k] - centre[k]))
    };
    let faces: Vec<_> = part
        .solids
        .iter()
        .flat_map(|s| s.faces.iter().copied())
        .collect();
    let rays = haecceity::rays::RayCaster::for_faces(
        part,
        if faces.is_empty() {
            (0..part.faces.len()).collect()
        } else {
            faces
        },
    );
    let reach = part.bounds().diagonal() * 2. + 1.;
    let epsilon = 1e-5 * reach;
    let cut = v
        .cut
        .as_ref()
        .or_else(|| v.detail.as_ref().and_then(|d| d.source_cut.as_ref()));
    // Adjacent equal verdicts remain one exact curve; the open quarter and line
    // widths survive SVG/PDF/PNG export without tessellating the conventional ink.
    let mut draw = |curve: Curve, witness: &dyn Fn(f64) -> V3, bold: bool, force_hidden: bool| {
        let verdict = |u: f64| -> Option<bool> {
            let p = witness(u);
            let mut end = reach;
            if let Some(c) = cut {
                let d = geom::dot(geom::sub(p, c.origin), c.normal);
                if d < -epsilon {
                    return None;
                }
                let rate = -geom::dot(view.toward, c.normal);
                if rate > 1e-8 {
                    end = (d / rate).max(0.);
                }
            }
            Some(
                force_hidden
                    || (end > epsilon && rays.any_hit(p, view.toward, epsilon, end) == Some(true)),
            )
        };
        let count = if matches!(curve, Curve::Conic { .. }) {
            72
        } else {
            24
        };
        let mut start = 0.;
        let mut previous = verdict(0.5 / count as f64);
        for i in 1..=count {
            let next = if i == count {
                None
            } else {
                verdict((i as f64 + 0.5) / count as f64)
            };
            if next != previous || i == count {
                if let Some(h) = previous {
                    let item = Item::Edge(curve.trim(start, i as f64 / count as f64));
                    if h {
                        if v.line_style.hidden.unwrap_or(v.id != "v:iso") && cut.is_none() {
                            hidden.push(item);
                        }
                    } else if bold {
                        thick.push(item);
                    } else {
                        thin.push(item);
                    }
                }
                start = i as f64 / count as f64;
                previous = next;
            }
        }
    };
    for t in ts {
        let along = geom::dot(t.axis, view.toward);
        let section = cut.is_some_and(|c| {
            geom::dot(t.axis, c.normal).abs() < 1e-6
                && geom::dot(geom::sub(t.on, c.origin), c.normal).abs() < t.crest()
        });
        let concealed = t.internal && along.abs() < 1e-6 && !section;
        let gap = ((t.root() - t.crest()).abs() * v.scale).max(1.0);
        let root = if t.internal {
            t.crest() + gap / v.scale
        } else {
            (t.crest() - gap / v.scale).max(t.crest() * 0.1)
        };
        if along.abs() > 1. - 1e-6 {
            let z = if along > 0. { t.span[1] } else { t.span[0] };
            let radial = |u: f64, r: f64| {
                let theta = FRAC_PI_2 + u * (TAU - FRAC_PI_2);
                geom::add(
                    t.at(z),
                    geom::add(
                        geom::scale(view.x, r * theta.cos()),
                        geom::scale(view.y, r * theta.sin()),
                    ),
                )
            };
            let witness = |u| radial(u, t.crest() + if t.internal { -epsilon } else { epsilon });
            draw(
                Curve::Conic {
                    centre: page(t.at(z)),
                    radii: [root * v.scale; 2],
                    x_axis: [1., 0.],
                    y_axis: [0., 1.],
                    t0: FRAC_PI_2,
                    t1: TAU,
                },
                &witness,
                false,
                false,
            );
        } else {
            let u = geom::unit(geom::cross(t.axis, view.toward)).unwrap();
            let mut offset = [0.; 3];
            let mut crest = t.crest();
            let mut root_chord = root;
            if section && let Some(c) = cut {
                let d = geom::dot(geom::sub(t.on, c.origin), c.normal);
                offset = geom::scale(c.normal, -d);
                crest = (crest * crest - d * d).sqrt();
                root_chord = (root * root - d * d).max(0.).sqrt();
            }
            let at = |z, r| geom::add(geom::add(t.at(z), offset), geom::scale(u, r));
            for side in [-1., 1.] {
                let witness = |f| {
                    at(
                        t.span[0] + f * (t.span[1] - t.span[0]),
                        side * (crest + if t.internal { -epsilon } else { epsilon }),
                    )
                };
                draw(
                    Curve::Line(
                        page(at(t.span[0], side * root_chord)),
                        page(at(t.span[1], side * root_chord)),
                    ),
                    &witness,
                    false,
                    concealed,
                );
            }
            for z in t.span {
                let toward =
                    geom::unit(geom::sub(view.toward, geom::scale(t.axis, along))).unwrap();
                let witness = |f| {
                    let radial = (2. * f - 1.) * crest;
                    if section || t.internal {
                        at(z, radial)
                    } else {
                        geom::add(
                            at(z, radial),
                            geom::scale(
                                toward,
                                (crest * crest - radial * radial).max(0.).sqrt() + epsilon,
                            ),
                        )
                    }
                };
                draw(
                    Curve::Line(page(at(z, -crest)), page(at(z, crest))),
                    &witness,
                    true,
                    concealed,
                );
            }
        }
    }
    if v.detail.is_some() {
        let size = crate::detail::crop_extent(a, v).0;
        let rect = [
            v.at[0] - size[0] * v.scale / 2.,
            v.at[1] - size[1] * v.scale / 2.,
            v.at[0] + size[0] * v.scale / 2.,
            v.at[1] + size[1] * v.scale / 2.,
        ];
        for list in [&mut thick, &mut thin, &mut hidden] {
            *list = std::mem::take(list)
                .into_iter()
                .flat_map(|i| match i {
                    Item::Edge(c) => crate::detail::clip(c, rect)
                        .into_iter()
                        .map(Item::Edge)
                        .collect(),
                    Item::Face(_) => Vec::new(),
                })
                .collect();
        }
    }
    (thick, thin, hidden)
}

pub(crate) fn cache_key(a: &Analysis, s: &Spec, v: &PlannedView) -> Vec<u64> {
    let ts: Vec<_> = threads(a).collect();
    if ts.is_empty() {
        return Vec::new();
    }
    let mut key = vec![
        0x6410,
        u64::from(v.line_style.thread_depiction != Some(false)),
    ];
    key.extend(ts.iter().map(|t| u64::from(t.enabled(s))));
    key
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub edges: Vec<usize>,
    pub curved: Vec<usize>,
    pub span: [f64; 2],
}
pub(crate) fn evidence(part: &Part, a: &Analysis) -> std::collections::BTreeMap<String, Evidence> {
    let mut curved = vec![None; part.faces.len()];
    let mut n = 0;
    for (i, f) in part.faces.iter().enumerate() {
        if !matches!(f.surface, Surface::Plane { .. }) {
            curved[i] = Some(n);
            n += 1;
        }
    }
    threads(a)
        .map(|t| {
            let edges = (0..part.edges.len())
                .filter(|&e| {
                    !part.edge_faces()[e].is_empty()
                        && part.edge_faces()[e]
                            .iter()
                            .all(|i| t.feature.faces.contains(i))
                })
                .collect();
            let curved = t
                .feature
                .faces
                .iter()
                .filter_map(|&i| curved.get(i).copied().flatten())
                .collect();
            let (lo, hi) = part.extent_along(&t.feature.faces, t.axis);
            let shift = geom::dot(t.on, t.axis);
            (
                t.feature.id.clone(),
                Evidence {
                    edges,
                    curved,
                    span: [(lo - shift).min(t.span[0]), (hi - shift).max(t.span[1])],
                },
            )
        })
        .collect()
}

pub(crate) fn planning<'a>(a: &'a Analysis, s: &Spec) -> std::borrow::Cow<'a, Analysis> {
    if a.modelled_thread_ink.is_empty()
        || s.sheet.thread_depiction == Some(false)
        || s.views.values().any(|v| v.thread_depiction == Some(false))
    {
        return std::borrow::Cow::Borrowed(a);
    }
    let ts: Vec<_> = threads(a).filter(|t| t.enabled(s)).collect();
    if ts.is_empty() {
        return std::borrow::Cow::Borrowed(a);
    }
    let mut prepared = a.clone();
    if let Some(ink) = &mut prepared.edge_ink {
        let mut omitted = BTreeSet::new();
        for t in ts {
            if let Some(e) = a.modelled_thread_ink.get(&t.feature.id) {
                for &i in &e.edges {
                    if let Some(edge) = ink.edges.get_mut(i) {
                        edge.clear();
                    }
                }
                omitted.extend(e.curved.iter().copied());
                let r = t.crest();
                let ends = e.span.map(|z| t.at(z));
                let lo = [0, 1, 2]
                    .map(|k| ends[0][k].min(ends[1][k]) - r * (1. - t.axis[k] * t.axis[k]).sqrt());
                let hi = [0, 1, 2]
                    .map(|k| ends[0][k].max(ends[1][k]) + r * (1. - t.axis[k] * t.axis[k]).sqrt());
                ink.curved.push(crate::model::projected_ink::CurvedFace {
                    bounds: [lo, hi],
                    cylinder: Some(crate::model::projected_ink::Cylinder {
                        origin: t.on,
                        axis: t.axis,
                        radius: r,
                    }),
                });
            }
        }
        ink.curved = std::mem::take(&mut ink.curved)
            .into_iter()
            .enumerate()
            .filter_map(|(i, c)| (!omitted.contains(&i)).then_some(c))
            .collect();
        ink.resolved.clear();
    }
    // Preparing an already prepared analysis is unnecessary during recovery.
    prepared.modelled_thread_ink.clear();
    std::borrow::Cow::Owned(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn feature(d: f64, p: f64, internal: bool) -> Feature {
        serde_json::from_value(json!({"id":"f:modelled_thread:1","kind":"modelled_thread","faces":[0],
            "fingerprint":{"version":"test","family":"modelled_threads","subtype":"ModelledThread","sizes":{},"axis":[0,0,1],"position":[0,0,4],"neighbourhood":[]},
            "parameters":{"axis":{"point":[0,0,0],"direction":[0,0,1]},"side":if internal {"internal"}else{"external"},"major_diameter":d,"minor_diameter":d-p,"pitch":p,"hand":"right","starts":1,"start":0,"end":8,"length":8}
        })).unwrap()
    }
    fn analysis(f: Feature) -> Analysis {
        serde_json::from_value(json!({"schema":crate::model::ANALYSIS_SCHEMA,"binding":{"source_sha256":"fixture","part":"thread","face_count":1,"loader":"test"},"envelope":[[-10,-10,0],[10,10,8]],"iso_bounds":[-10,-10,10,10],"features":[f]})).unwrap()
    }
    fn view(end: bool) -> PlannedView {
        serde_json::from_value(json!({"id":if end {"v:plan"}else{"v:front"},"direction":if end {[0,0,1]}else{[0,-1,0]},"up":if end {[0,1,0]}else{[0,0,1]},"at":[50,50],"scale":1.,"area":[20,20,80,80],"default":true,"basis":"fixture","state":{"kind":"shown"}})).unwrap()
    }
    fn plain_carrier(a: &Analysis) -> Part {
        // A single full cylindrical face, with circular end trims. A fixture of
        // the line grammar need not import a helix just to measure its replacement.
        let frame = Frame {
            origin: [0.; 3],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            z: [0., 0., 1.],
        };
        let mut edges = Vec::new();
        for (i, z) in [0., 8.].into_iter().enumerate() {
            let c = haecceity::geom::Curve::Circle {
                frame: Frame {
                    origin: [0., 0., z],
                    ..frame
                },
                radius: 10.,
            };
            let p = c.value(0.);
            edges.push(Edge {
                curve: c.clone(),
                start: p,
                end: p,
                vertices: (i, i),
                same_sense: true,
                samples: (0..=64).map(|k| c.value(k as f64 * TAU / 64.)).collect(),
            });
        }
        let source = Part::new(
            vec![Face {
                surface: Surface::Cylinder { frame, radius: 10. },
                reversed: false,
                loops: vec![
                    Loop {
                        edges: vec![(0, true)],
                        vertex: None,
                    },
                    Loop {
                        edges: vec![(1, false)],
                        vertex: None,
                    },
                ],
                solid: None,
                pcurves: vec![],
            }],
            edges,
            vec![],
        );
        carrier(&source, a, &Spec::default(), true).unwrap()
    }
    #[test]
    fn coarse_fine_and_corpus_designations() {
        for (d, p, internal, expected) in [
            (8., 1.25, false, "M8×1.25"),
            (14., 1.5, false, "M14×1.5"),
            (13.623798, 1.5, true, "M14×1.5"),
            (61.623798, 1.5, true, "M62×1.5"),
            (75., 1.5, false, "M75×1.5"),
            (20., 2.5, false, "M20×2.5"),
        ] {
            assert_eq!(
                Thread::read(&feature(d, p, internal))
                    .unwrap()
                    .designation(),
                (expected.into(), true)
            );
        }
    }
    #[test]
    fn matching_tolerance_edges_and_unlisted_pairs() {
        for (d, p, inside) in [
            (8.02, 1.26, true),
            (8.020001, 1.25, false),
            (8., 1.260001, false),
            (7.98, 1.25, true),
            (7.979999, 1.25, false),
            (13.61, 1.5, true),
            (13.609999, 1.5, false),
            (110., 6.35, false),
            (13., 1.5, false),
        ] {
            assert_eq!(
                Thread::read(&feature(d, p, d > 13. && d < 14.))
                    .unwrap()
                    .designation()
                    .1,
                inside,
                "{d}/{p}"
            );
        }
        assert_eq!(
            Thread::read(&feature(110., 6.35, false))
                .unwrap()
                .designation(),
            ("Thread Ø110 × P6.35".into(), false)
        );
    }
    #[test]
    fn left_hand_and_multiple_starts_keep_pitch_and_lead_distinct() {
        let mut f = feature(20., 1., false);
        f.parameters.insert("hand".into(), json!("left"));
        f.parameters.insert("starts".into(), json!(2));
        assert_eq!(Thread::read(&f).unwrap().designation().0, "M20×Ph2P1 LH");
        f.parameters.insert("major_diameter".into(), json!(110.));
        f.parameters.insert("pitch".into(), json!(6.35));
        assert_eq!(
            Thread::read(&f).unwrap().designation().0,
            "Thread Ø110 × P6.35 2 STARTS LH"
        );
    }
    #[test]
    fn external_profile_and_end_arc_use_iso_weights_and_open_quarter() {
        let a = analysis(feature(20., 1., false));
        let source = plain_carrier(&a);
        let s = Spec::default();
        let (bold, thin, hidden) = ink(&a, &s, &view(false), &source);
        assert!(!bold.is_empty());
        assert!(hidden.is_empty());
        assert_eq!(thin.len(), 2);
        for i in &thin {
            let Item::Edge(Curve::Line(p, q)) = i else {
                panic!("{i:?}")
            };
            assert!((p[0] - q[0]).abs() < 1e-9);
            assert!(((p[1] - q[1]).abs() - 8.).abs() < 1e-9);
            assert!(((p[0] - 50.).abs() - 9.0).abs() < 1e-9);
        }
        let (_, thin, hidden) = ink(&a, &s, &view(true), &source);
        assert!(hidden.is_empty());
        assert_eq!(thin.len(), 1);
        let Item::Edge(Curve::Conic { radii, t0, t1, .. }) = &thin[0] else {
            panic!()
        };
        assert_eq!(*radii, [9.0; 2]);
        assert!((t1 - t0 - TAU * 0.75).abs() < 1e-9);
        assert_eq!(*t0, FRAC_PI_2);
        let styles = crate::output::layer_styles();
        assert!(styles[0].1.width > crate::output::thin_style().width);
        assert!(crate::output::thin_style().dash.is_none());
    }
    #[test]
    fn internal_hidden_and_section_roots_follow_bore_and_hatching_boundary() {
        let a = analysis(feature(20., 1.5, true));
        let source = plain_carrier(&a);
        let s = Spec::default();
        let (_, thin, hidden) = ink(&a, &s, &view(false), &source);
        assert!(thin.is_empty());
        assert!(!hidden.is_empty());
        let mut v = view(false);
        v.cut = Some(
            serde_json::from_value(
                json!({"origin":[0,0,4],"normal":[0,1,0],"label":"A","marked_in":"v:plan"}),
            )
            .unwrap(),
        );
        let (_, thin, hidden) = ink(&a, &s, &v, &source);
        assert_eq!(thin.len(), 2);
        assert!(hidden.is_empty());
        assert!(
            matches!(source.faces[0].surface,Surface::Cylinder {radius,..} if (radius-9.25).abs()<1e-9)
        );
        let section = haecceity::hlr::section(
            &source,
            &haecceity::hlr::Plane::new([0., 0., 4.], [0., 1., 0.]).unwrap(),
        )
        .unwrap();
        assert!(
            section
                .iter()
                .flatten()
                .all(|p| (p[0].abs() - 9.25).abs() < 1e-5
                    || p[2].abs() < 1e-5
                    || (p[2] - 8.).abs() < 1e-5)
        );
    }
    #[test]
    fn spec_off_restores_source_and_designation_override_has_no_geometry_effect() {
        let a = analysis(feature(110., 6.35, false));
        let source = plain_carrier(&a);
        let s:Spec=serde_json::from_value(json!({"features":{"f:modelled_thread:1":{"modelled_thread":{"depiction":false,"designation":"SPECIAL THREAD"}}}})).unwrap();
        assert!(carrier(&source, &a, &s, true).is_none());
        assert_eq!(ink(&a, &s, &view(false), &source), (vec![], vec![], vec![]));
        let on:Spec=serde_json::from_value(json!({"features":{"f:modelled_thread:1":{"modelled_thread":{"designation":"SPECIAL THREAD"}}}})).unwrap();
        assert_eq!(
            cache_key(&a, &Spec::default(), &view(false)),
            cache_key(&a, &on, &view(false))
        );
        let plan = crate::model::plan(&a, &s).unwrap();
        assert!(
            plan.dimensions
                .iter()
                .any(|d| d.label.contains("SPECIAL THREAD"))
        );
        assert!(
            plan.problems
                .iter()
                .any(|p| p.code == "modelled_thread_not_depicted")
        );
        assert!(
            !plan
                .problems
                .iter()
                .any(|p| p.code == "thread_designation_measured")
        );
    }
    #[test]
    fn pmi_thread_callout_precedes_inference_in_first_and_strict_modes() {
        use std::io::Read;
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(
            &include_bytes!("../tests/fixtures/pmi/step/bolt_thread_knurl.step.gz")[..],
        )
        .read_to_end(&mut bytes)
        .unwrap();
        let (_, mut a) =
            crate::model::analyse_native_step(std::path::Path::new("bolt.step"), &bytes).unwrap();
        let manufacturing = crate::pmi_lowering::manufacturing(&a);
        let owner = manufacturing
            .owned
            .iter()
            .find(|o| o.aspect == "thread")
            .unwrap();
        let original = a.features.iter().find(|f| f.id == owner.owner).unwrap();
        let mut f = feature(6., 0.5, false);
        f.faces = original.faces.clone();
        let crate::pmi::Reading::Read(report) = a.pmi.as_ref().unwrap() else {
            panic!()
        };
        let c = report
            .records
            .iter()
            .find(|r| r.source_id == owner.source)
            .unwrap()
            .geometry
            .as_ref()
            .unwrap()
            .cylinders[0]
            .clone();
        f.parameters.insert(
            "axis".into(),
            json!({"point":c.origin,"direction":c.direction}),
        );
        f.parameters.insert("start".into(), json!(c.interval[0]));
        f.parameters.insert("end".into(), json!(c.interval[1]));
        a.features.push(f);
        for mode in [crate::model::PmiMode::First, crate::model::PmiMode::Strict] {
            let s = Spec {
                pmi: mode,
                ..Spec::default()
            };
            let p = crate::model::plan(&a, &s).unwrap();
            assert!(
                !p.dimensions
                    .iter()
                    .any(|d| d.id == "d:f:modelled_thread:1/thread")
            );
            assert_eq!(
                p.dimensions
                    .iter()
                    .filter(|d| d
                        .measurements
                        .iter()
                        .any(|m| m.feature == owner.id && m.parameter == "thread"))
                    .count(),
                1
            );
        }
        let p = crate::model::plan(
            &a,
            &Spec {
                pmi: crate::model::PmiMode::Ignore,
                ..Spec::default()
            },
        )
        .unwrap();
        assert!(
            p.dimensions
                .iter()
                .any(|d| d.id == "d:f:modelled_thread:1/thread")
        );
    }
}

pub(crate) fn source_hash(
    original: &Analysis,
    prepared: &std::borrow::Cow<'_, Analysis>,
    plan: &mut crate::model::Plan,
) {
    if matches!(prepared, std::borrow::Cow::Owned(_)) {
        use sha2::{Digest, Sha256};
        plan.analysis_sha256 = Some(
            Sha256::digest(serde_json::to_vec(original).expect("analysis serializes"))
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        );
    }
}
