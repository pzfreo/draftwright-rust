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
//!   smallest that leaves room for both arrows and the whole label. Witness stations select
//!   the support half-rays, without imposing a radius. Both the physical sector and the equal
//!   sector on the two extended supports are tried, in increasing radius, in 1 mm steps, whose
//!   ink stays on the sheet, its label off the views, the title block and the other items, and
//!   whose lines cross no other view, dimension or label. With none, it keeps the smallest radius
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

impl From<&crate::lint::profile_angles::Corner> for Angle {
    fn from(c: &crate::lint::profile_angles::Corner) -> Self {
        Self {
            vertex: c.vertex,
            first: c.first,
            second: c.second,
        }
    }
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
pub(crate) fn from_planes(planes: [(P3, P3); 2], stations: [P3; 2]) -> Result<Angle, &'static str> {
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
#[derive(Clone)]
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
        // Witness stations select the physical half-rays; their distance from the
        // intersection must not force the arc out past the middle of a long face.
        let mut lo = radial_half + PAD;
        let mut hi = (2.0 * tangent_half + 2.0 * radial_half + 2.0 * PAD + 4.0 * ARROW) / sweep
            + radial_half
            + PAD;
        // Fit the actual text gap and the two arrow lengths instead of adding
        // their radial and tangential extents as if all ran along one line.
        for _ in 0..40 {
            let radius = 0.5 * (lo + hi);
            let gap = (tangent_half + PAD).atan2(radius - radial_half - PAD);
            if (0.5 * sweep - gap) * radius > ARROW {
                hi = radius;
            } else {
                lo = radius;
            }
        }
        let minimum = hi + 1e-6;
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

    /// The same ink with its label at *fraction* of the sweep from its start (0.5 is the
    /// bisector), turned to read from below or the right.
    fn along(&self, fraction: f64) -> Ink {
        let mut g = self.clone();
        g.middle = g.start + g.sweep * fraction;
        let tangent = (g.middle + std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::TAU);
        g.text_angle = if std::f64::consts::FRAC_PI_2 < tangent
            && tangent <= 3.0 * std::f64::consts::FRAC_PI_2
        {
            tangent - std::f64::consts::PI
        } else {
            tangent
        };
        g
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
        self.placed_at(radius, terms, false)
    }

    fn placed_at(&self, radius: f64, terms: &[String], literal: bool) -> Option<Placed> {
        let end = self.start + self.sweep;
        let gap = (self.tangent_half + PAD).atan2(radius - self.radial_half - PAD);
        let pieces = [(self.start, self.middle - gap), (self.middle + gap, end)].map(|(a, b)| {
            if literal {
                (a.clamp(self.start, end), b.clamp(self.start, end))
            } else {
                (a, b)
            }
        });
        if !literal && pieces.iter().any(|(a, b)| (b - a) * radius <= ARROW) {
            return None;
        }
        let mut lines = Vec::new();
        for (length, angle) in self.lengths.iter().zip([self.start, end]) {
            lines.push([
                self.point(length.min(radius - GAP - OVERSHOOT) + GAP, angle),
                self.point(radius + OVERSHOOT, angle),
            ]);
        }
        for (a, b) in pieces.into_iter().filter(|(a, b)| a < b) {
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

pub(super) fn intent(
    frame: &Frame,
    angle: &Angle,
    terms: &[String],
    p: &crate::placement_intent::Place,
) -> Option<Placed> {
    let ink = label_ink(terms, font());
    let mut geometry = Ink::new(
        frame,
        angle,
        [ink.max[0] - ink.min[0], ink.max[1] - ink.min[1]],
    )?;
    if let Some(a) = p.along {
        geometry.middle = geometry.start + geometry.sweep * a.fraction();
        let tangent =
            (geometry.middle + std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::TAU);
        geometry.text_angle = if std::f64::consts::FRAC_PI_2 < tangent
            && tangent <= 3.0 * std::f64::consts::FRAC_PI_2
        {
            tangent - std::f64::consts::PI
        } else {
            tangent
        };
    }
    geometry.placed_at(
        geometry.minimum + p.lane.unwrap_or(0) as f64 * STEP,
        terms,
        true,
    )
}

/// Reserve only the bounded notch grammar's minimum exterior arc. General profile
/// corners remain disabled; ordinary linear lane estimates cannot size acute arcs.
pub(crate) fn notch_reservations(
    a: &crate::model::Analysis,
    spec: &crate::model::Spec,
    scale: f64,
) -> Vec<(String, crate::layout::Rect)> {
    let mut out = Vec::new();
    for n in crate::notches::requirements(a) {
        let id = super::notches::id(&n.id, "notch.angle");
        if spec.dimensions.get(&id).is_some_and(|d| d.omit) {
            continue;
        }
        let (view, direction, up) = match n.axis {
            0 => ("side", [1., 0., 0.], [0., 0., 1.]),
            1 => ("front", [0., -1., 0.], [0., 0., 1.]),
            _ => ("plan", [0., 0., 1.], [0., 1., 0.]),
        };
        let frame = Frame::looking(a, direction, up, [0.; 2], scale);
        let terms = vec![format!("{}°", super::fmt(n.angle.degrees()))];
        let ink = label_ink(&terms, font());
        let size = [ink.max[0] - ink.min[0], ink.max[1] - ink.min[1]];
        // Invariant: The notch reader proves nonzero rays and angle in this principal plane.
        let physical = Ink::new(&frame, &(&n.angle).into(), size).unwrap();
        let mut extended = Ink::new(
            &frame,
            &Angle {
                vertex: n.angle.vertex,
                first: [0, 1, 2].map(|k| 2. * n.angle.vertex[k] - n.angle.first[k]),
                second: [0, 1, 2].map(|k| 2. * n.angle.vertex[k] - n.angle.second[k]),
            },
            size,
        )
        // Invariant: Reversing both proven notch rays preserves their nonzero angle.
        .unwrap();
        extended.lengths = [0.; 2];
        let terms_ref = &terms;
        if let Some(p) = (0..TRIES)
            .flat_map(|i| {
                [&physical, &extended]
                    .into_iter()
                    .filter_map(move |g| g.placed(g.minimum + i as f64 * STEP, terms_ref))
            })
            .find(|p| !boxes_overlap(p.label_box, frame.outline()))
        {
            let mut bounds = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for q in ink_points(&p) {
                for k in 0..2 {
                    bounds[k] = bounds[k].min(q[k] - 1.);
                    bounds[k + 2] = bounds[k + 2].max(q[k] + 1.);
                }
            }
            out.push((view.into(), bounds));
        }
    }
    out
}

/// Places each angular item of *items* in its carrying view (*views_of*) about its vertex (see
/// the module's notes), clear of what *placed* already holds where it can be; *placed* takes
/// its ink.
#[allow(clippy::too_many_arguments)]
pub(super) fn place(
    frames: &std::collections::BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    sides: &[Option<super::placement::Side>],
    fixed: &[bool],
    obstacles: &[super::PlannedDimension],
    placed: &mut [Option<Placed>],
    page: &Page,
    notch_void: &dyn Fn(&str, &Placed) -> bool,
) {
    for i in 0..items.len() {
        if placed[i].is_some() {
            continue;
        }
        let (Shape::Angular { angle }, Some(view)) = (&items[i].shape, views_of[i]) else {
            continue;
        };
        // Profile-corner angles come last, around everything else ([`place_profile`]).
        if super::profile_angles::is_profile_angle(&items[i]) {
            continue;
        }
        let terms = &items[i].terms;
        let ink = label_ink(terms, font());
        let size = [ink.max[0] - ink.min[0], ink.max[1] - ink.min[1]];
        let Some(geometry) = Ink::new(&frames[view], angle, size) else {
            continue;
        };
        // Extending both supports reverses both rays and preserves the measured
        // included angle (also for obtuse angles). Search radius first, so a compact
        // exterior sector wins over an enormous arc around the original witnesses.
        let mut extended = Ink::new(
            &frames[view],
            &Angle {
                vertex: angle.vertex,
                first: [0, 1, 2].map(|k| 2.0 * angle.vertex[k] - angle.first[k]),
                second: [0, 1, 2].map(|k| 2.0 * angle.vertex[k] - angle.second[k]),
            },
            size,
        )
        // Invariant: The original rays passed the same Ink::new test before reversal.
        .expect("reversing both rays preserves the angle");
        extended.lengths = [0.0; 2];
        let mut candidates = (0..TRIES).flat_map(|n| {
            [&geometry, &extended]
                .into_iter()
                .filter_map(move |g| g.placed(g.minimum + n as f64 * STEP, terms))
        });
        let others: Vec<&Placed> = placed
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .filter_map(|(_, q)| q.as_ref())
            .chain(obstacles.iter().filter_map(|d| d.placed.as_ref()))
            .collect();
        let own = format!("v:{view}");
        let exterior = items[i]
            .measurements
            .iter()
            .any(|m| m.feature.starts_with("geometry:"));
        let in_void = |c: &Placed| {
            items[i]
                .measurements
                .iter()
                .any(|m| m.parameter == "notch.angle")
                && notch_void(view, c)
        };
        let furniture = |c: &Placed| {
            !boxes_overlap(c.label_box, page.title_block())
                && page
                    .views()
                    .iter()
                    .all(|(id, b)| (*id == own && in_void(c)) || !boxes_overlap(c.label_box, *b))
                && !c.lines.iter().any(|l| crosses(*l, page.title_block()))
                && (!exterior
                    || c.lines
                        .iter()
                        .skip(2)
                        .all(|l| !crosses(*l, frames[view].outline())))
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
                        && super::placement::dimension_lines_clear(c, q)
                })
        };
        let mut first = None;
        let mut fallback = None;
        let mut moves = Vec::new();
        let chosen = candidates.find(|c| {
            first.get_or_insert_with(|| c.clone());
            if !furniture(c) {
                return false;
            }
            fallback.get_or_insert_with(|| c.clone());
            if !on_sheet(c) || !(page.fits(c, view) || (in_void(c) && page.fits_interior(c, view)))
            {
                return false;
            }
            if clear(c) {
                return true;
            }
            // A newly supplied notch must fit around established dimensions. It cannot
            // evict a previously readable datum or short leader to acquire its arc.
            if !items[i]
                .measurements
                .iter()
                .any(|m| m.parameter == "notch.angle")
                && let Some(room) = super::placement::make_room(
                    i, c, frames, items, views_of, sides, fixed, placed, obstacles, page,
                )
            {
                moves = room;
                return true;
            }
            false
        });
        for (j, p) in moves {
            placed[j] = Some(p);
        }
        placed[i] = chosen.or(fallback).or(first);
    }
}

/// Withdraws each placed profile-corner angle whose witness lines a callout routed after it
/// now meets (see [`place_profile`]); the caller places them again, last.
pub(super) fn withdraw_profile_contacts(items: &[Want], placed: &mut [Option<Placed>]) {
    let callouts: Vec<Placed> = placed
        .iter()
        .zip(items)
        .filter(|(_, w)| matches!(w.shape, Shape::Callout { .. }))
        .filter_map(|(q, _)| q.clone())
        .collect();
    for (q, w) in placed.iter_mut().zip(items) {
        if super::profile_angles::is_profile_angle(w)
            && q.as_ref().is_some_and(|c| {
                callouts
                    .iter()
                    .any(|l| super::leaders::meets_extensions(&c.lines[..2], &l.lines, l.label_box))
            })
        {
            *q = None;
        }
    }
}

/// Places each profile-corner angle of *items* (see [`super::profile_angles`]) after every
/// other item, one at a time, in its true-shape view: inside the corner's material (an arc
/// whose sides are the corner's own edges, its label on proven blank face) or in the equal
/// sector between the extended edges, outside the material. Only a place clear of the sheet's
/// furniture, the other views, every placed item and the view's line work is taken; with
/// none the angle stays unplaced (dropped): it never takes a fallback, moves another item or
/// asks for more paper. One already placed by its authored placement intent is kept.
#[allow(clippy::too_many_arguments)]
pub(super) fn place_profile(
    frames: &std::collections::BTreeMap<&str, Frame>,
    items: &[Want],
    views_of: &[Option<&'static str>],
    obstacles: &[super::PlannedDimension],
    placed: &mut [Option<Placed>],
    page: &Page,
    views: &[crate::model::PlannedView],
    proofs: &std::collections::BTreeMap<String, std::rc::Rc<super::interior::Proof>>,
) {
    for i in 0..items.len() {
        // An authored placement (`dimensions/<id>/place`) has already been honoured.
        if !super::profile_angles::is_profile_angle(&items[i]) || placed[i].is_some() {
            continue;
        }
        let (Shape::Angular { angle }, Some(view)) = (&items[i].shape, views_of[i]) else {
            continue;
        };
        let own = format!("v:{view}");
        let (Some(frame), Some(v)) = (frames.get(view), views.iter().find(|v| v.id == own)) else {
            continue;
        };
        let Some(proof) = proofs.get(&own) else {
            continue;
        };
        let terms = &items[i].terms;
        let ink = label_ink(terms, font());
        let size = [ink.max[0] - ink.min[0], ink.max[1] - ink.min[1]];
        let Some(physical) = Ink::new(frame, angle, size) else {
            continue;
        };
        let Some(mut extended) = Ink::new(
            frame,
            &Angle {
                vertex: angle.vertex,
                first: [0, 1, 2].map(|k| 2.0 * angle.vertex[k] - angle.first[k]),
                second: [0, 1, 2].map(|k| 2.0 * angle.vertex[k] - angle.second[k]),
            },
            size,
        ) else {
            continue;
        };
        extended.lengths = [0.0; 2];
        // Every other item, and whether a thin line of the angle may cross its lines: other
        // dimensions, centre lines and cutting-plane lines may be crossed transversely (ISO
        // 129-1); leaders, frames and symbols never. No item's label is ever touched.
        let others: Vec<(&Placed, bool)> = placed
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .filter_map(|(j, q)| {
                let crossable = matches!(
                    items[j].shape,
                    Shape::Linear { .. } | Shape::Aligned { .. } | Shape::Angular { .. }
                );
                q.as_ref().map(|q| (q, crossable))
            })
            .chain(obstacles.iter().filter_map(|d| {
                let crossable = d.kind == "linear"
                    || (d.id.starts_with("ink:")
                        && !d.id.contains(":arrow:")
                        && !d.id.contains(":letter:"));
                d.placed.as_ref().map(|q| (q, crossable))
            }))
            .collect();
        // A placed callout whose leader or padded label meets the angle's witness lines reads
        // as touching an extension line (the maintainer's 2026-10-12 contact rule).
        let callouts: Vec<&Placed> = placed
            .iter()
            .zip(items)
            .enumerate()
            .filter(|(j, (_, w))| *j != i && matches!(w.shape, Shape::Callout { .. }))
            .filter_map(|(_, (q, _))| q.as_ref())
            .chain(
                obstacles
                    .iter()
                    .filter(|d| d.kind == "callout")
                    .filter_map(|d| d.placed.as_ref()),
            )
            .collect();
        let off_leaders = |c: &Placed| {
            callouts
                .iter()
                .all(|q| !super::leaders::meets_extensions(&c.lines[..2], &q.lines, q.label_box))
        };
        // Within the view's reserved area: an angle never claims paper the layout did not give
        // its view.
        let area = page.area(view);
        let in_area = |c: &Placed| {
            area.is_none_or(|b| {
                ink_points(c).all(|q| q[0] >= b[0] && q[0] <= b[2] && q[1] >= b[1] && q[1] <= b[3])
            })
        };
        let clear = |c: &Placed, strict: bool| {
            off_leaders(c)
                && in_area(c)
                && page.fits_interior(c, view)
                && others.iter().all(|&(q, crossable)| {
                    super::placement::annotations_clear(c, q)
                        && if strict || !crossable {
                            super::placement::dimension_lines_clear(c, q)
                        } else {
                            super::placement::no_shared_strokes(c, q)
                        }
                })
        };
        // Its label must stand on proven blank paper: a blank face inside the corner, or
        // exterior paper clear of every projected edge. Thin arc and witness lines may cross the
        // view's own line work, as ISO 129-1 lets dimension lines do where unavoidable.
        let label_clear = |c: &Placed| {
            proof.blank(v, c.label_box) || proof.exterior_clear(v, c.label_box, &[], &[])
        };
        // The label may slide off the bisector; its arc pieces must still carry their arrows.
        let inks: Vec<(Ink, usize)> = [0.5, 0.35, 0.65, 0.25, 0.75]
            .iter()
            .flat_map(|&f| [(physical.along(f), 2), (extended.along(f), 0)])
            .collect();
        // First a place whose lines cross nothing (inside the corner its witness stubs lie on
        // the corner's own edges), then one whose label alone is clear.
        let search = |strict: bool| {
            (0..TRIES).find_map(|n| {
                inks.iter().find_map(|(g, stubs)| {
                    g.placed(g.minimum + n as f64 * STEP, terms).filter(|c| {
                        clear(c, strict)
                            && label_clear(c)
                            && (!strict || proof.lines_clear(v, &c.lines[*stubs..], &c.arrows))
                    })
                })
            })
        };
        let chosen = search(true).or_else(|| search(false));
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
