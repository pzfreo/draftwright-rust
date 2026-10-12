//! Feature footprints from the Analysis, shared by leader scoring and collision reporting.
//! These are drafting obstacles, not projected line-work (ADR R1). Circular openings are exact;
//! side-on cylinders and slots use their projected bounds. Principal-face inner loops fill gaps
//! in recognition. The Analysis retains no complete outer wire, so solid bounds detect re-entry
//! and measure the distance to an exit without pretending to describe a concave silhouette.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::placement::Frame;
use super::{Measurement, P2, P3};
use crate::bridge::holes::{self, dir_to_part, point, to_part};
use crate::lint::profiles::{Kind, Wire};
use crate::model::Analysis;

const HAIR: f64 = 0.05;

#[derive(Clone)]
enum Shape {
    Circle(P2, f64),
    Box([f64; 4]),
    Loop(Vec<[P2; 2]>),
}

struct Footprint {
    id: String,
    shape: Shape,
}

pub(super) struct Footprints {
    features: Vec<Footprint>,
    solids: Vec<[f64; 4]>,
    view: String,
    members: BTreeMap<String, BTreeSet<String>>,
}

type Projections = BTreeMap<(String, Vec<u64>), Rc<Footprints>>;

/// Plan-local projections, keyed by the resolved frame so a repacked view cannot reuse stale ink.
#[derive(Default)]
pub(crate) struct Cache {
    views: RefCell<Projections>,
}

impl Cache {
    pub(super) fn get(
        &self,
        analysis: &Analysis,
        view: &crate::model::PlannedView,
    ) -> Rc<Footprints> {
        let key = view
            .direction
            .iter()
            .chain(&view.up)
            .chain(&view.at)
            .chain(std::iter::once(&view.scale))
            .map(|v| v.to_bits())
            .collect();
        self.views
            .borrow_mut()
            .entry((view.id.clone(), key))
            .or_insert_with(|| {
                Rc::new(Footprints::new(
                    analysis,
                    &Frame::new(analysis, view),
                    view.id.trim_start_matches("v:"),
                ))
            })
            .clone()
    }
}

fn bounds(points: impl Iterator<Item = P2>) -> [f64; 4] {
    points.fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        },
    )
}

fn box_points(b: [P3; 2]) -> impl Iterator<Item = P3> {
    (0..8).map(move |i| [0, 1, 2].map(|k| b[(i >> k) & 1][k]))
}

fn clip(s: [P2; 2], b: [f64; 4]) -> Option<(f64, f64)> {
    let d = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        if d[k].abs() < 1e-12 {
            if s[0][k] <= b[k] || s[0][k] >= b[k + 2] {
                return None;
            }
        } else {
            let a = (b[k] - s[0][k]) / d[k];
            let c = (b[k + 2] - s[0][k]) / d[k];
            lo = lo.max(a.min(c));
            hi = hi.min(a.max(c));
        }
    }
    (hi > lo).then_some((lo, hi))
}

impl Shape {
    fn spans(&self, s: [P2; 2]) -> Vec<(f64, f64)> {
        let d = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
        let len2 = d[0] * d[0] + d[1] * d[1];
        if len2 < 1e-18 {
            return vec![];
        }
        match self {
            Self::Box(b) => clip(s, *b).into_iter().collect(),
            Self::Circle(c, r) => {
                let q = [s[0][0] - c[0], s[0][1] - c[1]];
                let b = q[0] * d[0] + q[1] * d[1];
                let disc = b * b - len2 * (q[0] * q[0] + q[1] * q[1] - r * r);
                if disc <= 0.0 {
                    return vec![];
                }
                let lo = ((-b - disc.sqrt()) / len2).max(0.0);
                let hi = ((-b + disc.sqrt()) / len2).min(1.0);
                if hi > lo { vec![(lo, hi)] } else { vec![] }
            }
            Self::Loop(edges) => {
                let cross = |a: P2, b: P2| a[0] * b[1] - a[1] * b[0];
                let mut cuts = vec![0.0, 1.0];
                for [a, b] in edges {
                    let e = [b[0] - a[0], b[1] - a[1]];
                    let den = cross(d, e);
                    if den.abs() < 1e-12 {
                        continue;
                    }
                    let q = [a[0] - s[0][0], a[1] - s[0][1]];
                    let (t, u) = (cross(q, e) / den, cross(q, d) / den);
                    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                        cuts.push(t);
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
                cuts.windows(2)
                    .filter_map(|w| {
                        let t = 0.5 * (w[0] + w[1]);
                        let p = [s[0][0] + t * d[0], s[0][1] + t * d[1]];
                        let inside = edges
                            .iter()
                            .filter(|[a, b]| {
                                (a[1] > p[1]) != (b[1] > p[1])
                                    && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                            })
                            .count()
                            % 2
                            == 1;
                        inside.then_some((w[0], w[1]))
                    })
                    .collect()
            }
        }
    }

    fn crossed(&self, s: [P2; 2]) -> bool {
        let len = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
        // Reject disjoint bounds before testing loop edges. Convex shapes need one interval.
        let Some((a, b)) = clip(s, self.bounds()) else {
            return false;
        };
        if (b - a) * len <= HAIR {
            return false;
        }
        if let Self::Box(_) = self {
            return a * len > HAIR;
        }
        if let Self::Circle(c, r) = self
            && (s[0][0] - c[0]).hypot(s[0][1] - c[1]) <= *r
        {
            return false;
        }
        self.spans(s)
            .iter()
            .any(|(a, b)| (b - a) * len > HAIR && a * len > HAIR)
    }

    fn bounds(&self) -> [f64; 4] {
        match self {
            Self::Circle(c, r) => [c[0] - r, c[1] - r, c[0] + r, c[1] + r],
            Self::Box(b) => *b,
            Self::Loop(e) => bounds(e.iter().flatten().copied()),
        }
    }
}

fn wire(w: &Wire, frame: &Frame) -> Shape {
    let mut lines = Vec::new();
    for e in &w.edges {
        let (a, b, m) = (frame.page(e.start), frame.page(e.end), frame.page(e.mid));
        if e.kind == Kind::Circle
            && let (Some(c), Some(r)) = (e.centre, e.radius)
        {
            let c = frame.page(c);
            let r = r * frame.scale();
            let angle = |p: P2| (p[1] - c[1]).atan2(p[0] - c[0]);
            let start = angle(a);
            let mut sweep = (angle(b) - start).rem_euclid(std::f64::consts::TAU);
            let mid = (angle(m) - start).rem_euclid(std::f64::consts::TAU);
            if (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-6 {
                sweep = std::f64::consts::TAU;
            } else if mid > sweep {
                sweep -= std::f64::consts::TAU;
            }
            let n = (sweep.abs() / (std::f64::consts::PI / 64.0)).ceil() as usize;
            let mut last = a;
            for j in 1..=n {
                let t = start + sweep * j as f64 / n as f64;
                let next = if j == n {
                    b
                } else {
                    [c[0] + r * t.cos(), c[1] + r * t.sin()]
                };
                lines.push([last, next]);
                last = next;
            }
        } else {
            lines.push([a, b]);
        }
    }
    Shape::Loop(lines)
}

impl Footprints {
    pub(super) fn new(analysis: &Analysis, frame: &Frame, view: &str) -> Self {
        let qframe = holes::frame(analysis);
        let mut features = Vec::new();
        for (_, read) in holes::read(analysis) {
            let Ok(h) = read else { continue };
            let at = frame.page(h.at);
            let along = frame.page([0, 1, 2].map(|k| h.at[k] + h.direction[k]));
            let radial = h
                .counterbore
                .map_or(h.diameter, |r| r.diameter)
                .max(h.spotface.map_or(0.0, |r| r.diameter))
                .max(h.countersink.map_or(0.0, |r| r.diameter))
                * 0.5;
            let shape = if (at[0] - along[0]).hypot(at[1] - along[1]) < 1e-6 {
                Shape::Circle(at, radial * frame.scale())
            } else {
                // The face-box centre is in the part's frame and does not assume which end
                // a record's drilling direction starts at.
                let c = h.feature.fingerprint.position;
                let depth = h.depth.unwrap_or_else(|| {
                    (0..3)
                        .map(|k| {
                            (analysis.envelope[1][k] - analysis.envelope[0][k])
                                * h.direction[k].abs()
                        })
                        .sum()
                });
                let b = [0, 1].map(|j| {
                    [0, 1, 2].map(|k| {
                        c[k] + if j == 0 { -1.0 } else { 1.0 }
                            * (0.5 * depth * h.direction[k].abs()
                                + radial * (1.0 - h.direction[k] * h.direction[k]).max(0.0).sqrt())
                    })
                });
                Shape::Box(bounds(box_points(b).map(|p| frame.page(p))))
            };
            features.push(Footprint {
                id: h.feature.id.clone(),
                shape,
            });
        }
        for f in &analysis.features {
            if f.kind == "double_d_bore"
                && let (Some(axis), Some(location), Some(diameter)) = (
                    point(f.parameters.get("axis")),
                    point(f.parameters.get("location")),
                    f.parameters.get("major_diameter").and_then(|v| v.as_f64()),
                )
            {
                let c = f.fingerprint.position;
                let direction = dir_to_part(&qframe, axis);
                let at = frame.page(to_part(&qframe, location));
                let end = frame.page([0, 1, 2].map(|k| c[k] + direction[k]));
                let middle = frame.page(c);
                let shape = if (middle[0] - end[0]).hypot(middle[1] - end[1]) < 1e-6 {
                    Shape::Circle(at, 0.5 * diameter * frame.scale())
                } else {
                    let depth = (0..3)
                        .map(|k| {
                            (analysis.envelope[1][k] - analysis.envelope[0][k]) * direction[k].abs()
                        })
                        .sum::<f64>();
                    let b = [0, 1].map(|j| {
                        [0, 1, 2].map(|k| {
                            c[k] + if j == 0 { -1.0 } else { 1.0 }
                                * (0.5 * depth * direction[k].abs()
                                    + 0.5
                                        * diameter
                                        * (1.0 - direction[k] * direction[k]).max(0.0).sqrt())
                        })
                    });
                    Shape::Box(bounds(box_points(b).map(|p| frame.page(p))))
                };
                features.push(Footprint {
                    id: f.id.clone(),
                    shape,
                });
            }
            if f.kind == "section_recess"
                && let Ok(crate::lint::section_recesses::Grammar::Pocket(p)) =
                    crate::lint::section_recesses::grammar(f)
            {
                let b = [0, 1].map(|j| {
                    [0, 1, 2].map(|k| {
                        p.origin[k]
                            + if j == 0 { -1.0 } else { 1.0 }
                                * 0.5
                                * if k == p.axis {
                                    p.depth
                                } else if k == p.long_axis {
                                    p.length
                                } else {
                                    p.width
                                }
                    })
                });
                features.push(Footprint {
                    id: f.id.clone(),
                    shape: Shape::Box(bounds(
                        box_points(b).map(|p| frame.page(to_part(&qframe, p))),
                    )),
                });
            }
        }
        for f in &analysis.features {
            if f.kind == "slot" {
                let p = &f.parameters;
                let axis = |name| {
                    p.get(name)
                        .and_then(|v| v.as_str())
                        .and_then(|a| ["x", "y", "z"].iter().position(|v| *v == a))
                };
                let number = |name| p.get(name).and_then(|v| v.as_f64());
                if let (
                    Some(l),
                    Some(w),
                    Some(lo),
                    Some(hi),
                    Some(c),
                    Some(width),
                    Some(dlo),
                    Some(dhi),
                ) = (
                    axis("long_axis"),
                    axis("width_axis"),
                    number("lo"),
                    number("hi"),
                    number("w_center"),
                    number("width"),
                    number("d_lo"),
                    number("d_hi"),
                ) && l != w
                {
                    let mut b = [[0.0; 3]; 2];
                    b[0][l] = lo;
                    b[1][l] = hi;
                    b[0][w] = c - 0.5 * width;
                    b[1][w] = c + 0.5 * width;
                    b[0][3 - l - w] = dlo;
                    b[1][3 - l - w] = dhi;
                    features.push(Footprint {
                        id: f.id.clone(),
                        shape: Shape::Box(bounds(
                            box_points(b).map(|p| frame.page(to_part(&qframe, p))),
                        )),
                    });
                }
            }
        }
        // Only end-on principal faces describe opening footprints. Duplicate boundary faces
        // share their feature ID, so a shaft counts each crossed feature just once.
        for source in &analysis.principal_profiles {
            for face in &source.faces {
                let axis = [0, 1, 2].map(|k| f64::from(k == face.axis));
                let a = frame.page([0.0; 3]);
                let b = frame.page(axis);
                if (a[0] - b[0]).hypot(a[1] - b[1]) > 1e-6 || face.unsplit {
                    continue;
                }
                for w in &face.inner {
                    let shape = wire(w, frame);
                    let b = shape.bounds();
                    let matching = features
                        .iter()
                        .filter(|f| {
                            let c = f.shape.bounds();
                            (0..4).all(|k| (b[k] - c[k]).abs() < 0.1 * frame.scale())
                        })
                        .min_by_key(|f| &f.id);
                    if matching.is_some() {
                        continue;
                    }
                    // An unrecognised opening belongs to the view's outline, not an invented
                    // feature ID. Recognised non-round openings use their actual feature.
                    let centre = [0.5 * (b[0] + b[2]), 0.5 * (b[1] + b[3])];
                    let id = analysis
                        .features
                        .iter()
                        .filter(|f| {
                            matches!(
                                f.kind.as_str(),
                                "pocket" | "section_recess" | "double_d_bore" | "polygonal_bore"
                            )
                        })
                        .find(|f| {
                            let p = frame.page(f.fingerprint.position);
                            (p[0] - centre[0]).hypot(p[1] - centre[1]) < 0.1 * frame.scale()
                        })
                        .map_or_else(|| format!("v:{view}"), |f| f.id.clone());
                    features.push(Footprint { id, shape });
                }
            }
        }
        let solids = if analysis.principal_profiles.is_empty() {
            vec![frame.outline()]
        } else {
            analysis
                .principal_profiles
                .iter()
                .map(|s| bounds(box_points(s.bounds).map(|p| frame.page(p))))
                .collect()
        };
        let mut members: BTreeMap<String, BTreeSet<String>> = analysis
            .recognition
            .iter()
            .flat_map(|r| &r.patterns)
            .map(|p| (p.id.clone(), p.members.iter().cloned().collect()))
            .collect();
        for f in &analysis.features {
            if f.kind == "slot_pattern"
                && let Some(slots) = f.parameters.get("slots").and_then(|v| v.as_array())
            {
                members.insert(
                    f.id.clone(),
                    slots
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect(),
                );
            }
        }
        Self {
            features,
            solids,
            view: format!("v:{view}"),
            members,
        }
    }

    pub(super) fn crossed(&self, shaft: [P2; 2], _owners: &[Measurement]) -> BTreeSet<String> {
        self.crossed_ids(shaft)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Whether *shaft* crosses a feature's footprint, the view's material aside: an interior
    /// leader runs over material by design, and its ink proof keeps it off every edge.
    pub(super) fn crosses_feature(&self, shaft: [P2; 2]) -> bool {
        self.features.iter().any(|f| f.shape.crossed(shaft))
    }

    pub(super) fn crossing_count(&self, shaft: [P2; 2]) -> usize {
        self.crossed_ids(shaft).len()
    }

    fn crossed_ids(&self, shaft: [P2; 2]) -> BTreeSet<&str> {
        let mut result: BTreeSet<&str> = self
            .features
            .iter()
            .filter(|f| f.shape.crossed(shaft))
            .map(|f| f.id.as_str())
            .collect();
        let len = (shaft[1][0] - shaft[0][0]).hypot(shaft[1][1] - shaft[0][1]);
        if self
            .solids
            .iter()
            .any(|b| clip(shaft, *b).is_some_and(|(a, c)| a * len > HAIR && (c - a) * len > HAIR))
        {
            result.insert(self.view.as_str());
        }
        result
    }

    fn owned(&self, id: &str, owners: &[Measurement]) -> bool {
        owners.iter().any(|m| {
            m.feature == id
                || self
                    .members
                    .get(&m.feature)
                    .is_some_and(|members| members.contains(id))
        })
    }

    /// An interior witness leaves its own opening at the near rim, keeping the normal gap.
    pub(super) fn interior_witness(&self, line: &mut [P2; 2], owners: &[Measurement]) -> bool {
        let exit = self
            .features
            .iter()
            .filter(|f| self.owned(&f.id, owners))
            .flat_map(|f| f.shape.spans(*line))
            .filter(|(a, _)| *a <= 1e-9)
            .map(|(_, b)| b)
            .max_by(f64::total_cmp);
        if let Some(exit) = exit {
            let d = [line[1][0] - line[0][0], line[1][1] - line[0][1]];
            let length = d[0].hypot(d[1]);
            let t = exit + super::GAP / length.max(1e-12);
            if t >= 1.0 {
                return false;
            }
            line[0] = [line[0][0] + t * d[0], line[0][1] + t * d[1]];
        }
        true
    }

    /// A dimension witnesses its own feature (all members for a pattern). Unlike a leader,
    /// its extension does not select one member to label, so only other features are obstacles.
    pub(super) fn crossed_extension(
        &self,
        shaft: [P2; 2],
        owners: &[Measurement],
    ) -> BTreeSet<String> {
        self.features
            .iter()
            .filter(|f| f.id != self.view && !self.owned(&f.id, owners) && f.shape.crossed(shaft))
            .map(|f| f.id.clone())
            .collect()
    }
    /// Travel to an exit is a length tie-break, never a crossing or a containment warning.
    pub(super) fn inside_length(&self, shaft: [P2; 2]) -> f64 {
        let len = (shaft[1][0] - shaft[0][0]).hypot(shaft[1][1] - shaft[0][1]);
        self.solids
            .iter()
            .filter_map(|b| clip(shaft, *b))
            .map(|(a, b)| (b - a) * len)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis() -> Analysis {
        serde_json::from_value(serde_json::json!({
            "schema":crate::model::ANALYSIS_SCHEMA,
            "binding":{"source_sha256":"test","part":"test","face_count":1,"loader":"test"},
            "envelope":[[-10.0,-10.0,-10.0],[10.0,10.0,10.0]],"iso_bounds":[0.0,0.0,0.0,0.0],
            "features":[{"id":"f:hole:1","kind":"hole","faces":[0],
                "parameters":{"axis":[0.0,0.0,1.0],"location":[2.0,0.0,3.0],"diameter":4.0,"depth":6.0,"bottom":"flat"},
                "fingerprint":{"version":"test","family":"holes","subtype":"HoleRecord","sizes":{},"axis":[0.0,0.0,1.0],
                    "position":[2.0,0.0,0.0],"neighbourhood":[]}}]
        })).unwrap()
    }

    #[test]
    fn cylinders_project_as_end_on_circles_and_side_on_depth_bands() {
        let a = analysis();
        let plan = Frame::looking(&a, [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [50.0, 50.0], 2.0);
        let front = Frame::looking(&a, [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [50.0, 50.0], 2.0);
        let p = Footprints::new(&a, &plan, "plan");
        let f = Footprints::new(&a, &front, "front");
        assert!(matches!(p.features[0].shape, Shape::Circle(_, 4.0)));
        assert!(matches!(f.features[0].shape, Shape::Box(_)));
        let shaft = [front.page([-2.0, 0.0, 2.0]), front.page([6.0, 0.0, 2.0])];
        assert!(f.crossed(shaft, &[]).contains("f:hole:1"));
        let outside = [front.page([-2.0, 0.0, 4.0]), front.page([6.0, 0.0, 4.0])];
        assert!(!f.crossed(outside, &[]).contains("f:hole:1"));
    }

    #[test]
    fn each_feature_counts_once_and_a_target_exemption_stays_near_the_tip() {
        let footprint = |id: &str, c| Footprint {
            id: id.into(),
            shape: Shape::Circle(c, 1.0),
        };
        let f = Footprints {
            view: "v:plan".into(),
            solids: vec![],
            members: BTreeMap::new(),
            features: vec![
                footprint("f:own", [0.0, 0.0]),
                footprint("f:other", [5.0, 0.0]),
                footprint("f:other", [5.0, 0.0]),
                footprint("f:own", [8.0, 0.0]),
            ],
        };
        let owner = [Measurement {
            feature: "f:own".into(),
            parameter: "diameter".into(),
            value: 2.0,
        }];
        assert_eq!(
            f.crossed([[0.0, 0.0], [10.0, 0.0]], &owner),
            BTreeSet::from(["f:own".into(), "f:other".into()])
        );
        assert!(f.crossed([[0.0, 0.0], [-3.0, 0.0]], &owner).is_empty());
    }

    #[test]
    fn a_pattern_exempts_its_attached_member_but_not_a_later_member_crossing() {
        let f = Footprints {
            view: "v:plan".into(),
            solids: vec![],
            members: BTreeMap::from([(
                "f:pattern".into(),
                BTreeSet::from(["f:a".into(), "f:b".into()]),
            )]),
            features: vec![
                Footprint {
                    id: "f:a".into(),
                    shape: Shape::Circle([0.0, 0.0], 1.0),
                },
                Footprint {
                    id: "f:b".into(),
                    shape: Shape::Circle([5.0, 0.0], 1.0),
                },
            ],
        };
        let owner = [Measurement {
            feature: "f:pattern".into(),
            parameter: "pitch".into(),
            value: 5.0,
        }];
        assert!(f.crossed([[0.0, 0.0], [-3.0, 0.0]], &owner).is_empty());
        assert_eq!(
            f.crossed([[0.0, 0.0], [8.0, 0.0]], &owner),
            BTreeSet::from(["f:b".into()])
        );
        assert!(
            f.crossed_extension([[0.0, 0.0], [8.0, 0.0]], &owner)
                .is_empty()
        );
    }

    #[test]
    fn collision_reports_shafts_and_both_extensions_with_distinct_ids() {
        use super::super::{Placed, PlannedDimension};
        use crate::model::{PlannedSheet, PlannedView};
        let a = analysis();
        let view:PlannedView=serde_json::from_value(serde_json::json!({
            "id":"v:plan","direction":[0.0,0.0,1.0],"up":[0.0,1.0,0.0],"at":[50.0,50.0],
            "scale":1.0,"area":[20.0,20.0,100.0,100.0],"default":true,"basis":"","state":{"kind":"shown"}})).unwrap();
        let sheet: PlannedSheet = serde_json::from_value(serde_json::json!({
            "size":"A4","page":[297.0,210.0],"scale":"1:1","scale_factor":1.0,
            "arrangement":"columns","title_block":[200.0,10.0,280.0,30.0],"default":true,"basis":"",
            "state":{"kind":"shown"},"title":{}}))
        .unwrap();
        let frame = Frame::new(&a, &view);
        let line = [frame.page([-2.0, 0.0, 3.0]), frame.page([6.0, 0.0, 3.0])];
        let dim = |id: &str, kind: &str, lines| PlannedDimension {
            region: None,
            id: id.into(),
            kind: kind.into(),
            view: "v:plan".into(),
            value: 1.0,
            label: id.into(),
            terms: vec![id.into()],
            measurements: vec![],
            default: true,
            from_spec: vec![],
            basis: String::new(),
            state: crate::items::State::Shown,
            placed: Some(Placed {
                lines,
                arrows: vec![],
                label_at: [90.0, 90.0],
                rotation: 0.0,
                label_box: [90.0, 90.0, 95.0, 93.0],
            }),
        };
        let dims = [
            dim("d:callout", "callout", vec![line, line]),
            dim("d:linear", "linear", vec![line, line, line]),
        ];
        let problems = super::super::placement::collisions(&a, &dims, &sheet, &[view]);
        let features: Vec<_> = problems
            .iter()
            .filter(|p| p.code.ends_with("through_feature"))
            .collect();
        assert_eq!(features.len(), 3, "{features:?}");
        assert_eq!(
            features
                .iter()
                .filter(|p| p.code == "leader_through_feature")
                .count(),
            1
        );
        assert_eq!(
            features
                .iter()
                .map(|p| &p.id)
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        assert!(
            features
                .iter()
                .all(|p| p.severity == "warning" && p.subjects[1] == "f:hole:1")
        );
        assert!(
            features
                .iter()
                .filter(|p| p.code == "extension_through_feature")
                .all(|p| p.fix.is_empty())
        );
    }

    #[test]
    fn plan_cache_reuses_a_projection_but_rebuilds_after_view_repacking() {
        let a = analysis();
        let mut view: crate::model::PlannedView = serde_json::from_value(serde_json::json!({
            "id":"v:plan","direction":[0.0,0.0,1.0],"up":[0.0,1.0,0.0],"at":[50.0,50.0],
            "scale":1.0,"area":[20.0,20.0,100.0,100.0],"default":true,"basis":"","state":{"kind":"shown"}})).unwrap();
        let cache = Cache::default();
        let first = cache.get(&a, &view);
        assert!(Rc::ptr_eq(&first, &cache.get(&a, &view)));
        view.at[0] += 10.0;
        let moved = cache.get(&a, &view);
        assert!(!Rc::ptr_eq(&first, &moved));
        assert_eq!(first.features.len(), moved.features.len());
        for (a, b) in first.features.iter().zip(&moved.features) {
            assert!((b.shape.bounds()[0] - a.shape.bounds()[0] - 10.0).abs() < 1e-9);
        }
    }

    #[test]
    fn containing_features_allow_an_exit_but_still_report_reentry() {
        let vertices = [
            [0.0, 0.0],
            [6.0, 0.0],
            [6.0, 4.0],
            [4.0, 4.0],
            [4.0, 1.0],
            [2.0, 1.0],
            [2.0, 4.0],
            [0.0, 4.0],
        ];
        let f = Footprints {
            features: vec![Footprint {
                id: "f:cavity".into(),
                shape: Shape::Loop(
                    (0..vertices.len())
                        .map(|i| [vertices[i], vertices[(i + 1) % vertices.len()]])
                        .collect(),
                ),
            }],
            solids: vec![],
            view: "v:plan".into(),
            members: BTreeMap::new(),
        };
        let exit = [[1.0, 2.0], [-1.0, 2.0]];
        let reentry = [[1.0, 2.0], [7.0, 2.0]];
        assert!(f.crossed(exit, &[]).is_empty());
        assert!(f.crossed_extension(exit, &[]).is_empty());
        assert_eq!(f.crossed(reentry, &[]), BTreeSet::from(["f:cavity".into()]));
        assert_eq!(
            f.crossed_extension(reentry, &[]),
            BTreeSet::from(["f:cavity".into()])
        );
    }

    #[test]
    fn circles_reject_tangency_and_allow_any_initial_exit() {
        let f = Shape::Circle([0.0, 0.0], 2.0);
        assert!(f.crossed([[-3.0, 0.0], [3.0, 0.0]]));
        assert!(f.crossed([[-3.0, 0.0], [3.0, 0.0]]));
        assert!(!f.crossed([[-3.0, 2.0], [3.0, 2.0]]));
        assert!(!f.crossed([[2.0, 0.0], [4.0, 0.0]]));
        assert!(!f.crossed([[0.0, 0.0], [4.0, 0.0]]));
        assert!(!f.crossed([[0.0, 0.0], [4.0, 0.0]]));
    }

    #[test]
    fn boxes_count_side_on_bands_but_not_their_edges_or_short_touches() {
        let f = Shape::Box([-1.0, -2.0, 1.0, 2.0]);
        assert!(f.crossed([[-4.0, 0.0], [4.0, 0.0]]));
        assert!(!f.crossed([[-4.0, 2.0], [4.0, 2.0]]));
        assert!(!f.crossed([[0.99, 0.0], [4.0, 0.0]]));
        assert!(!f.crossed([[0.0, 0.0], [0.0, 0.0]]));
    }

    #[test]
    fn a_concave_loop_reports_reentry_after_the_exempt_tip() {
        let vertices = [
            [0.0, 0.0],
            [6.0, 0.0],
            [6.0, 4.0],
            [4.0, 4.0],
            [4.0, 1.0],
            [2.0, 1.0],
            [2.0, 4.0],
            [0.0, 4.0],
        ];
        let edges = (0..vertices.len())
            .map(|i| [vertices[i], vertices[(i + 1) % vertices.len()]])
            .collect();
        let f = Shape::Loop(edges);
        assert!(f.crossed([[1.0, 2.0], [7.0, 2.0]]));
        assert!(!f.crossed([[1.0, 2.0], [-1.0, 2.0]]));
    }
    #[test]
    fn interior_witness_leaves_its_own_rim_without_skipping_an_unrelated_feature() {
        let f = Footprints {
            view: "v:plan".into(),
            solids: vec![],
            members: BTreeMap::new(),
            features: vec![
                Footprint {
                    id: "f:own".into(),
                    shape: Shape::Circle([0., 0.], 3.),
                },
                Footprint {
                    id: "f:other".into(),
                    shape: Shape::Circle([7., 0.], 1.),
                },
            ],
        };
        let owners = [Measurement {
            feature: "f:own".into(),
            parameter: "pitch.length".into(),
            value: 10.,
        }];
        let mut line = [[1., 0.], [10., 0.]];
        assert!(f.interior_witness(&mut line, &owners));
        assert_eq!(line[0], [4., 0.]);
        assert_eq!(
            f.crossed_extension(line, &owners),
            BTreeSet::from(["f:other".into()])
        );
        let mut unowned = [[1., 0.], [10., 0.]];
        assert!(f.interior_witness(&mut unowned, &[]));
        assert_eq!(unowned[0], [1., 0.]);
        let mut short = [[1., 0.], [2., 0.]];
        assert!(!f.interior_witness(&mut short, &owners));
    }
}
