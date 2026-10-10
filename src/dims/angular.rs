//! Angular PMI dimensions: an AP242 angle between two planar faces, drawn where it shows true.
//!
//! - **Its sides** ([`geometry`], read with the part when it is analysed): draftwright's
//!   `_angular_reference` and `_angular_reference_from_planes` (`pmi.py`). Each reference group
//!   is one planar face; the vertex is the point of the planes' line of intersection nearest the
//!   midpoint of the two groups' box centres, and each side's witness is the point of its plane
//!   in the common normal section that its box centre selects (the minor sector between the two
//!   half-rays). The angle so measured must be the record's nominal to 0.01°; otherwise, or when
//!   a group is not one planar face, or the planes are parallel, it is not drawn, with
//!   draftwright's reason.
//! - **Its view**: the one that looks along the normal of the plane of its two sides, x, y or z to
//!   1e-6 (draftwright's `AngularReference.principal_axis` and `_angular_specs`: z the plan, y the
//!   front, x the side), so both sides project edge-on and the angle shows true. An angle whose
//!   sides' plane is oblique to every view is not drawn (draftwright's
//!   `dimension_kind_unsupported`: "supported angular ink requires explicit coplanar rays in a
//!   principal view"); it is never drawn foreshortened.
//! - **Its ink** ([`place`]): draftwright's `AngularGeometry` (`angular_geometry.py`) and
//!   `AngularDimension` (`annotations/angular.py`): two extension lines along the sides, from just
//!   past each witness to just past the arc; an arc about the vertex across the minor sector,
//!   broken around the label, an arrowhead at each end; the label inline on the bisector, aligned
//!   with the arc's tangent and turned to read from below or the right. The radius is the
//!   smallest that leaves room for both arrows and the whole label and reaches past the
//!   witnesses (draftwright's `minimum_radius`), then grows in 1 mm steps to the first place whose
//!   ink stays on the sheet, its label off the views, the title block and the other items, and
//!   whose lines cross no other view and no other label. With none, it keeps the smallest radius
//!   whose label is clear of the views, and the collision checks report what it hits: nothing is
//!   dropped. The arc is drawn as chords within 0.005 mm of it (draftwright's sampling).
//!
//! The label is the record's, as every PMI dimension's (`60 ±0.5`; draftwright's
//! `implicit_degrees`: its unit is the record's kind, not a printed `°`).

use haecceity::Part;
use haecceity::geom::Surface;
use serde::{Deserialize, Serialize};

use super::placement::{Frame, Page, boxes_overlap, crosses, ink_points, label_box};
use super::{ARROW, GAP, OVERSHOOT, P2, P3, Placed, Shape, Want, font, label_ink};
use crate::pmi::{Geometry, Record};

/// An angle's two sides, draftwright's `AngularReference` with its `minor` sector: the vertex
/// and a witness point on each side, world mm (the drawing's frame).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Angle {
    pub vertex: [f64; 3],
    pub first: [f64; 3],
    pub second: [f64; 3],
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: P3) -> f64 {
    dot(a, a).sqrt()
}

impl Angle {
    /// Its two sides' unit directions from the vertex; none when a witness is at the vertex or
    /// the sides are parallel (draftwright's `AngularReference` validation).
    pub fn rays(&self) -> Option<[P3; 2]> {
        let unit = |p: P3| {
            let d = sub(p, self.vertex);
            let n = norm(d);
            (n.is_finite() && n > 1e-9).then(|| d.map(|c| c / n))
        };
        let rays = [unit(self.first)?, unit(self.second)?];
        (norm(cross(rays[0], rays[1])) > 1e-9).then_some(rays)
    }

    /// The angle between its sides, degrees.
    pub fn degrees(&self) -> Option<f64> {
        let [a, b] = self.rays()?;
        Some(norm(cross(a, b)).atan2(dot(a, b)).to_degrees())
    }

    /// The world axis (0, 1, 2) along the normal of its sides' plane, when that is x, y or z to
    /// 1e-6 (draftwright's `principal_axis`): the view along it shows the angle true.
    pub fn principal(&self) -> Option<usize> {
        let [a, b] = self.rays()?;
        let c = cross(a, b);
        let n = c.map(|x| x / norm(c));
        let k = (0..3).fold(0, |k, i| if n[i].abs() > n[k].abs() { i } else { k });
        (0..3).all(|i| i == k || n[i].abs() <= 1e-6).then_some(k)
    }
}

/// An angular dimension's sides from its faces, into *g*: its [`Angle`] and the axis its view
/// looks along (`axis`, when principal), or the reasons it has none (draftwright's
/// `_angular_reference`; see the module's notes).
pub(crate) fn geometry(part: &Part, r: &Record, g: &mut Geometry) {
    let groups = &r.face_groups;
    let edges = |i: usize| r.edge_groups.get(i).map_or(0, Vec::len);
    if groups.len() != 2 || (0..2).any(|i| groups[i].len() + edges(i) != 1) {
        g.reasons
            .push("angular dimension needs one planar face in each reference group".into());
        return;
    }
    if (0..2).any(|i| groups[i].is_empty()) {
        g.reasons.push("one angular reference is not a face".into());
        return;
    }
    let mut planes = Vec::new();
    let mut stations = Vec::new();
    for group in groups {
        let f = group[0];
        let Surface::Plane { frame } = &part.faces[f].surface else {
            g.reasons
                .push("one angular reference face is not planar".into());
            return;
        };
        let n = norm(frame.z);
        if n.is_nan() || n <= 1e-12 {
            g.reasons.push(
                "one angular reference plane could not be measured (degenerate normal)".into(),
            );
            return;
        }
        planes.push((frame.origin, frame.z.map(|c| c / n)));
        let b = part.face_bounds(f);
        stations.push([0, 1, 2].map(|i| 0.5 * (b.min[i] + b.max[i])));
    }
    match from_planes([planes[0], planes[1]], [stations[0], stations[1]]) {
        Ok(a) => {
            match a.degrees() {
                Some(d) if (d - r.value.unwrap_or(f64::NAN)).abs() <= 0.01 => {}
                Some(d) => g.reasons.push(format!(
                    "angular reference angle {} deg differs from nominal {} deg",
                    sig6(d),
                    sig6(r.value.unwrap_or(f64::NAN))
                )),
                None => g.reasons.push(
                    "angular reference is invalid (angular reference rays must not be parallel or collinear)"
                        .into(),
                ),
            }
            g.axis = a.principal().map(|k| ["X", "Y", "Z"][k].to_string());
            g.angle = Some(a);
        }
        Err(why) => g.reasons.push(why.into()),
    }
}

/// *v* as Python's `{:.6g}`, enough for a reason's text.
fn sig6(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    let digits = 6 - 1 - v.abs().log10().floor().max(-300.0) as i32;
    let s = format!("{:.*}", digits.max(0) as usize, v);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// draftwright's `_angular_reference_from_planes`: the common normal-section angle of two planes
/// (each a point and a unit normal), its half-rays selected by the stations.
fn from_planes(planes: [(P3, P3); 2], stations: [P3; 2]) -> Result<Angle, &'static str> {
    let target = [0, 1, 2].map(|i| 0.5 * (stations[0][i] + stations[1][i]));
    let [(pa, na), (pb, nb)] = planes;
    let coupling = dot(na, nb);
    let determinant = 1.0 - coupling * coupling;
    if determinant <= 1e-12 {
        return Err("angular reference planes are parallel or coincident");
    }
    let ra = dot(na, sub(pa, target));
    let rb = dot(nb, sub(pb, target));
    let wa = (ra - coupling * rb) / determinant;
    let wb = (rb - coupling * ra) / determinant;
    let vertex = [0, 1, 2].map(|i| target[i] + wa * na[i] + wb * nb[i]);
    let line = cross(na, nb);
    let length = norm(line);
    if length <= 1e-12 {
        return Err("angular reference planes are parallel or coincident");
    }
    let line = line.map(|c| c / length);
    let mut witnesses = [[0.0; 3]; 2];
    for (w, (station, normal)) in witnesses
        .iter_mut()
        .zip([(stations[0], na), (stations[1], nb)])
    {
        let support = cross(line, normal);
        let s = dot(sub(station, vertex), support);
        if s.abs() <= 1e-9 {
            return Err("one angular reference face does not select a support half-ray");
        }
        *w = [0, 1, 2].map(|i| vertex[i] + s * support[i]);
    }
    let a = Angle {
        vertex,
        first: witnesses[0],
        second: witnesses[1],
    };
    if a.rays().is_none() {
        return Err(
            "angular reference is invalid (angular reference rays must not be parallel or collinear)",
        );
    }
    Ok(a)
}

/// The gap around the label in the arc, page mm (draftwright's `pad_around_text`).
const PAD: f64 = 2.0;
/// How far the arc's radius grows between the places tried, mm, and the most it tries.
const STEP: f64 = 1.0;
const TRIES: usize = 400;
/// The largest chord error of the arc's chords, mm (draftwright's `0.04` sampling constant gives
/// 0.005 mm).
const SAMPLING: f64 = 0.04;

/// One angle's ink on the page, its radius free: [`AngularGeometry`] in page mm.
struct Ink {
    vertex: P2,
    start: f64,
    sweep: f64,
    middle: f64,
    lengths: [f64; 2],
    /// The label's direction, radians, and its half-extents along and across the bisector.
    text_angle: f64,
    radial_half: f64,
    tangent_half: f64,
    minimum: f64,
}

impl Ink {
    /// The ink of *a* as *frame* sees it, for a label of ink *size* (width, height); none when
    /// its rays project to nothing or to a degenerate sector (the view does not look along its
    /// normal).
    fn new(frame: &Frame, a: &Angle, size: P2) -> Option<Ink> {
        let vertex = frame.page(a.vertex);
        let mut rays = Vec::new();
        let mut lengths = [0.0; 2];
        for (i, p) in [a.first, a.second].into_iter().enumerate() {
            let q = frame.page(p);
            let d = [q[0] - vertex[0], q[1] - vertex[1]];
            let n = d[0].hypot(d[1]);
            if n.is_nan() || n <= 0.0 {
                return None;
            }
            lengths[i] = n;
            rays.push([d[0] / n, d[1] / n]);
        }
        let (mut r0, r1) = (rays[0], rays[1]);
        let signed = (r0[0] * r1[1] - r0[1] * r1[0]).atan2(r0[0] * r1[0] + r0[1] * r1[1]);
        let sweep = signed.abs();
        if !(1e-9 < sweep && sweep < std::f64::consts::PI - 1e-9) {
            return None;
        }
        if signed < 0.0 {
            r0 = r1;
            lengths.swap(0, 1);
        }
        let start = r0[1].atan2(r0[0]);
        let middle = start + 0.5 * sweep;
        let pi = std::f64::consts::PI;
        let tangent = (middle + 0.5 * pi).rem_euclid(2.0 * pi);
        let text_angle = if 0.5 * pi < tangent && tangent <= 1.5 * pi {
            tangent - pi
        } else {
            tangent
        };
        let [width, height] = size;
        let relative = text_angle - middle;
        let radial_half = relative.cos().abs() * width / 2.0 + relative.sin().abs() * height / 2.0;
        let tangent_half = relative.sin().abs() * width / 2.0 + relative.cos().abs() * height / 2.0;
        let longest = lengths[0].max(lengths[1]);
        let minimum = (longest + GAP + ARROW).max(
            (2.0 * tangent_half + 2.0 * radial_half + 2.0 * PAD + 4.0 * ARROW) / sweep
                + radial_half
                + PAD,
        );
        Some(Ink {
            vertex,
            start,
            sweep,
            middle,
            lengths,
            text_angle,
            radial_half,
            tangent_half,
            minimum,
        })
    }

    fn point(&self, radius: f64, angle: f64) -> P2 {
        [
            self.vertex[0] + radius * angle.cos(),
            self.vertex[1] + radius * angle.sin(),
        ]
    }

    /// The ink at *radius* for a label of *terms*: extension lines, the arc's chords either side
    /// of the label, an arrowhead at each end, the label centred on the bisector; none when the
    /// arc's pieces are too short for their arrowheads.
    fn placed(&self, radius: f64, terms: &[String]) -> Option<Placed> {
        let end = self.start + self.sweep;
        let gap = (self.tangent_half + PAD).atan2(radius - self.radial_half - PAD);
        let pieces = [(self.start, self.middle - gap), (self.middle + gap, end)];
        if pieces.iter().any(|(a, b)| (b - a) * radius <= ARROW) {
            return None;
        }
        let mut lines = Vec::new();
        for (length, angle) in self.lengths.iter().zip([self.start, end]) {
            lines.push([
                self.point(length + GAP, angle),
                self.point(radius + OVERSHOOT, angle),
            ]);
        }
        for (a, b) in pieces {
            let count = ((b - a) * (radius / SAMPLING).sqrt()).ceil().max(1.0) as usize;
            let at = |i: usize| self.point(radius, a + (b - a) * i as f64 / count as f64);
            lines.extend((0..count).map(|i| [at(i), at(i + 1)]));
        }
        let turn = ARROW / radius;
        let arrows = vec![
            [
                self.point(radius, self.start),
                self.point(radius, self.start + turn),
            ],
            [self.point(radius, end), self.point(radius, end - turn)],
        ];
        let ink = label_ink(terms, font());
        let rotation = self.text_angle.to_degrees();
        let centre = self.point(radius, self.middle);
        let mid = [
            0.5 * (ink.min[0] + ink.max[0]),
            0.5 * (ink.min[1] + ink.max[1]),
        ];
        let (s, c) = self.text_angle.sin_cos();
        let label_at = [
            centre[0] - (c * mid[0] - s * mid[1]),
            centre[1] - (s * mid[0] + c * mid[1]),
        ];
        Some(Placed {
            lines,
            arrows,
            label_at,
            rotation,
            label_box: label_box(&ink, label_at, rotation),
        })
    }
}

/// Places each angular item of *items* in its carrying view (*views_of*) about its vertex (see
/// the module's notes), clear of what *placed* already holds where it can be; *placed* takes
/// its ink.
pub(super) fn place(
    frames: &std::collections::BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    placed: &mut [Option<Placed>],
    page: &Page,
) {
    for i in 0..items.len() {
        let (Shape::Angular { angle }, Some(view)) = (&items[i].shape, views_of[i]) else {
            continue;
        };
        let terms = &items[i].terms;
        let ink = label_ink(terms, font());
        let size = [ink.max[0] - ink.min[0], ink.max[1] - ink.min[1]];
        let Some(geometry) = Ink::new(&frames[view], angle, size) else {
            continue;
        };
        let candidates: Vec<Placed> = (0..TRIES)
            .filter_map(|n| geometry.placed(geometry.minimum + n as f64 * STEP, terms))
            .collect();
        let others: Vec<&Placed> = placed
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .filter_map(|(_, q)| q.as_ref())
            .collect();
        let own = format!("v:{view}");
        let furniture = |c: &Placed| {
            !boxes_overlap(c.label_box, page.title_block())
                && page
                    .views()
                    .iter()
                    .all(|(_, b)| !boxes_overlap(c.label_box, *b))
                && !c.lines.iter().any(|l| crosses(*l, page.title_block()))
        };
        let on_sheet = |c: &Placed| {
            let d = page.drawable();
            ink_points(c).all(|q| q[0] >= d[0] && q[0] <= d[2] && q[1] >= d[1] && q[1] <= d[3])
        };
        let clear = |c: &Placed| {
            page.views()
                .iter()
                .filter(|(id, _)| *id != own)
                .all(|(_, b)| !c.lines.iter().any(|l| crosses(*l, *b)))
                && others.iter().all(|q| {
                    !boxes_overlap(c.label_box, q.label_box)
                        && !q.lines.iter().any(|l| crosses(*l, c.label_box))
                        && !c.lines.iter().any(|l| crosses(*l, q.label_box))
                })
        };
        let chosen = candidates
            .iter()
            .find(|c| on_sheet(c) && furniture(c) && clear(c))
            .or_else(|| candidates.iter().find(|c| furniture(c)))
            .or_else(|| candidates.first())
            .cloned();
        placed[i] = chosen;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_planes_give_their_normal_section_angle() {
        // ctc_01's 60° V: planes through (110, -75) at ±30° from -y, normals in the xy plane.
        let (s, c) = 30f64.to_radians().sin_cos();
        let a = from_planes(
            [
                ([110.0, -75.0, 0.0], [c, s, 0.0]),
                ([110.0, -75.0, 0.0], [-c, s, 0.0]),
            ],
            [[128.76, -107.5, -25.0], [91.24, -107.5, -25.0]],
        )
        .unwrap();
        assert!((a.vertex[0] - 110.0).abs() < 1e-9 && (a.vertex[1] + 75.0).abs() < 1e-9);
        assert!((a.vertex[2] + 25.0).abs() < 1e-9, "{:?}", a.vertex);
        assert!((a.degrees().unwrap() - 60.0).abs() < 1e-9);
        assert_eq!(a.principal(), Some(2));
        assert!(a.first[0] > 110.0 && a.second[0] < 110.0);
    }

    /// Two planes whose line of intersection is oblique to x, y and z: an angle no view shows true,
    /// so `principal` (and with it the record's `axis`) is none and the plan refuses it.
    #[test]
    fn an_oblique_pair_of_planes_has_no_principal_view() {
        let h = 0.5f64.sqrt();
        let a = from_planes(
            [([0.0; 3], [1.0, 0.0, 0.0]), ([0.0; 3], [0.0, h, h])],
            [[0.0, 5.0, 5.0], [5.0, 0.0, 0.0]],
        )
        .unwrap();
        assert!((a.degrees().unwrap() - 90.0).abs() < 1e-9, "{a:?}");
        assert_eq!(a.principal(), None);
    }

    #[test]
    fn parallel_planes_have_no_angle() {
        let z = [0.0, 0.0, 1.0];
        assert_eq!(
            from_planes([([0.0; 3], z), ([0.0, 0.0, 5.0], z)], [[1.0; 3], [2.0; 3]]).unwrap_err(),
            "angular reference planes are parallel or coincident"
        );
    }

    #[test]
    fn reasons_print_six_figures() {
        assert_eq!(sig6(59.123456789), "59.1235");
        assert_eq!(sig6(60.0), "60");
        assert_eq!(sig6(0.5), "0.5");
    }
}
