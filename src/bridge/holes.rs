//! quiddity's hole records, read once for every reader of the Analysis: the sheet demand
//! ([`crate::demand`]), the dimensions ([`crate::dims`]) and lint ([`crate::lint`]). It reads the
//! Analysis alone, so lint may use it and still never read the Plan or the Spec.
//!
//! - [`Hole`] is a hole record, typed and in the part's frame;
//! - [`principal`] is the one rule for a direction lying along x, y or z;
//! - [`size_key`] is the one resolution sizes are compared at, and [`Hole::requirement`] the
//!   machining requirement identical holes share;
//! - [`established`] says whether a recognised hole pattern is one requirement.
//!
//! The record's fields are quiddity's (`holes.py`'s `HoleRecord`, `countersinks.py`'s
//! `CounterSink`): a countersink's sizes are `major_diameter` and `included_angle`.

use serde_json::{Map, Value};

use super::{Feature, HolePattern};
use crate::model::Analysis;

/// The identity frame: quiddity's coordinates when the Analysis carries no recognition frame.
const IDENTITY: [[f64; 3]; 4] = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// A counterbore or spotface: its diameter, and its depth when quiddity gives one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recess {
    pub diameter: f64,
    pub depth: Option<f64>,
}

/// A countersink: its major diameter, and its included angle (degrees) when quiddity gives one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countersink {
    pub diameter: f64,
    pub angle: Option<f64>,
}

/// A recognised hole, in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Hole<'a> {
    pub feature: &'a Feature,
    /// Its location (the opening's centre, on its axis).
    pub at: [f64; 3],
    /// Its drilling direction as quiddity orients it, unit length.
    pub direction: [f64; 3],
    /// The principal axis (0, 1, 2) it lies along and its sign along it; none when oblique.
    pub principal: Option<(usize, i8)>,
    pub diameter: f64,
    /// A fact of the feature (`bottom: through`), never read from a missing depth (ADR4-7).
    pub through: bool,
    /// quiddity's bottom (`through`, `flat`, `drill_point`, …), if it gives one.
    pub bottom: Option<&'a str>,
    /// The bore's depth (for a through hole, the thickness it passes through), if given.
    pub depth: Option<f64>,
    pub counterbore: Option<Recess>,
    pub spotface: Option<Recess>,
    pub countersink: Option<Countersink>,
}

/// The principal axis a direction lies along, if it does: draftwright's `_is_principal_axis`,
/// exactly one component of the unit direction left after snapping those under 1e-6 to zero.
pub fn principal(d: [f64; 3]) -> Option<usize> {
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if n == 0.0 {
        return None;
    }
    let along: Vec<usize> = (0..3).filter(|&k| (d[k] / n).abs() >= 1e-6).collect();
    (along.len() == 1).then(|| along[0])
}

/// A length, angle or direction component as sizes are compared: to 1e-6, quiddity's own
/// identity (`HoleSpec`) less floating-point noise.
pub fn size_key(x: f64) -> i64 {
    (x * 1e6).round() as i64
}

/// A record's point or vector field, if it is three numbers.
pub fn point(v: Option<&Value>) -> Option<[f64; 3]> {
    let a = v?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

/// quiddity's working frame in the part's: the recognition record's, or the identity when the
/// Analysis has none.
pub fn frame(analysis: &Analysis) -> [[f64; 3]; 4] {
    analysis.recognition.as_ref().map_or(IDENTITY, |r| r.frame)
}

/// quiddity's local point *l* in the part's frame.
pub fn to_part(frame: &[[f64; 3]; 4], l: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|k| frame[0][k] + (0..3).map(|j| l[j] * frame[j + 1][k]).sum::<f64>())
}

/// quiddity's local direction *d* in the part's frame.
pub fn dir_to_part(frame: &[[f64; 3]; 4], d: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|k| (0..3).map(|j| d[j] * frame[j + 1][k]).sum::<f64>())
}

/// A recess or countersink record's two sizes (*first* required, *second* optional): none when
/// absent, an error when unreadable.
fn sized(
    p: &Map<String, Value>,
    name: &str,
    first: &str,
    second: &str,
) -> Result<Option<(f64, Option<f64>)>, String> {
    match p.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(r)) => {
            let d = r
                .get(first)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("its {name} has no {first}"))?;
            Ok(Some((d, r.get(second).and_then(Value::as_f64))))
        }
        Some(v) => Err(format!("its {name} is {v}, not a record")),
    }
}

impl<'a> Hole<'a> {
    /// Reads hole feature *f*, whose coordinates are in quiddity's *frame*; an error says what
    /// cannot be read.
    pub fn read(f: &'a Feature, frame: &[[f64; 3]; 4]) -> Result<Hole<'a>, String> {
        let p = &f.parameters;
        let axis = point(p.get("axis")).ok_or("it has no axis")?;
        let location = point(p.get("location")).ok_or("it has no location")?;
        let d = dir_to_part(frame, axis);
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if n == 0.0 {
            return Err("its axis is a zero vector".into());
        }
        let direction = d.map(|c| c / n);
        let bottom = p.get("bottom").and_then(Value::as_str);
        let recess = |name: &str| {
            sized(p, name, "diameter", "depth")
                .map(|r| r.map(|(diameter, depth)| Recess { diameter, depth }))
        };
        Ok(Hole {
            feature: f,
            at: to_part(frame, location),
            direction,
            principal: principal(direction).map(|k| (k, if direction[k] > 0.0 { 1 } else { -1 })),
            diameter: p
                .get("diameter")
                .and_then(Value::as_f64)
                .ok_or("it has no diameter")?,
            through: bottom == Some("through"),
            bottom,
            depth: p.get("depth").and_then(Value::as_f64),
            counterbore: recess("cbore")?,
            spotface: recess("spotface")?,
            countersink: sized(p, "csink", "major_diameter", "included_angle")?
                .map(|(diameter, angle)| Countersink { diameter, angle }),
        })
    }

    /// What makes two holes one machining requirement (quiddity's `HoleSpec`, compared at
    /// [`size_key`]): the signed drilling direction, so holes from opposite faces stay apart
    /// (ISSUE-1229-a, ISSUE-1517-a, LINT-hole_coverage-opposed_bores_distinct); every size; and
    /// the depth only for a blind hole. Holes drilled the same way from faces at different
    /// heights are one requirement, as draftwright groups them.
    pub fn requirement(&self) -> Vec<i64> {
        let opt = |v: Option<f64>| v.map_or(i64::MIN, size_key);
        let mut k: Vec<i64> = self.direction.iter().map(|c| size_key(*c)).collect();
        k.extend([
            size_key(self.diameter),
            self.through as i64,
            if self.through { 0 } else { opt(self.depth) },
        ]);
        for r in [self.counterbore, self.spotface] {
            match r {
                Some(r) => k.extend([1, size_key(r.diameter), opt(r.depth)]),
                None => k.push(0),
            }
        }
        match self.countersink {
            Some(c) => k.extend([1, size_key(c.diameter), opt(c.angle)]),
            None => k.push(0),
        }
        k
    }
}

/// The Analysis's hole features, each read in the part's frame, or why it cannot be.
pub fn read(analysis: &Analysis) -> Vec<(&Feature, Result<Hole<'_>, String>)> {
    let frame = frame(analysis);
    analysis
        .features
        .iter()
        .filter(|f| f.kind == "hole")
        .map(|f| (f, Hole::read(f, &frame)))
        .collect()
}

/// Whether recognised pattern *p* is one machining requirement, given the readable *holes*:
/// the indices of its members in *holes*, or why it is not. Its members must all be readable
/// holes, more than one, with one [`Hole::requirement`]; a bolt circle needs five or more, as
/// fewer do not establish a circle (ISSUE-1596-a; draftwright's concentric corroboration of a
/// smaller one, LINT-hole_coverage-bolt_circle_corroboration, is not ported).
pub fn established(p: &HolePattern, holes: &[&Hole]) -> Result<Vec<usize>, String> {
    let members: Vec<usize> = p
        .members
        .iter()
        .map(|m| holes.iter().position(|h| &h.feature.id == m))
        .collect::<Option<_>>()
        .ok_or_else(|| {
            format!(
                "its pattern {} has a member that cannot be called out with it",
                p.id
            )
        })?;
    if members.len() < 2 {
        return Err(format!("its pattern {} has one member", p.id));
    }
    let first = holes[members[0]].requirement();
    if members.iter().any(|&m| holes[m].requirement() != first) {
        return Err(format!(
            "its pattern {} is not of identical holes, so each is called out as itself",
            p.id
        ));
    }
    if p.arrangement == "bolt_circle" && members.len() < 5 {
        return Err(format!(
            "quiddity's bolt circle {} has fewer than five members, which does not establish a circle (ISSUE-1596-a), so its holes are dimensioned singly",
            p.id
        ));
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn feature(parameters: Value) -> Feature {
        serde_json::from_value(json!({
            "id": "f:hole:1", "kind": "hole", "faces": [0], "parameters": parameters,
            "fingerprint": { "version": "test", "family": "holes", "subtype": "HoleRecord",
                             "sizes": {}, "position": [0.0, 0.0, 0.0], "neighbourhood": [] },
        }))
        .unwrap()
    }

    #[test]
    fn a_principal_axis_is_exactly_one_component() {
        assert_eq!(principal([0.0, 0.0, -1.0]), Some(2));
        assert_eq!(principal([1e-9, 2.0, 0.0]), Some(1));
        assert_eq!(principal([0.0, 0.01, 1.0]), None);
        assert_eq!(principal([0.0; 3]), None);
    }

    /// quiddity's countersink fields are `major_diameter` and `included_angle`.
    #[test]
    fn a_countersink_is_read_by_quidditys_field_names() {
        let f = feature(json!({
            "axis": [0.0, 0.0, -1.0], "location": [5.0, 5.0, 10.0], "diameter": 6.6,
            "depth": 10.0, "bottom": "through", "cbore": null, "spotface": null,
            "csink": { "major_diameter": 12.4, "included_angle": 90.0, "drill_diameter": 6.6,
                       "depth": 2.9, "axis": [0.0, 0.0, -1.0], "location": [5.0, 5.0, 10.0] },
        }));
        let h = Hole::read(&f, &IDENTITY).unwrap();
        assert_eq!(
            h.countersink,
            Some(Countersink {
                diameter: 12.4,
                angle: Some(90.0)
            })
        );
        assert_eq!(h.principal, Some((2, -1)));
        let mut odd = f.clone();
        odd.parameters["csink"] = json!({ "diameter": 12.4 });
        assert_eq!(
            Hole::read(&odd, &IDENTITY).unwrap_err(),
            "its csink has no major_diameter"
        );
    }

    /// A through hole's depth is not part of its requirement; a blind hole's is.
    #[test]
    fn the_requirement_reads_depth_only_when_blind() {
        let base = json!({ "axis": [0.0, 0.0, 1.0], "location": [0.0, 0.0, 0.0],
                           "diameter": 6.0, "depth": 5.0, "bottom": "through" });
        let mut other = base.clone();
        other["depth"] = json!(8.0);
        let (a, b) = (feature(base.clone()), feature(other.clone()));
        let key = |f: &Feature| Hole::read(f, &IDENTITY).unwrap().requirement();
        assert_eq!(key(&a), key(&b));
        let mut blind = (base, other);
        blind.0["bottom"] = json!("flat");
        blind.1["bottom"] = json!("flat");
        assert_ne!(key(&feature(blind.0)), key(&feature(blind.1)));
    }
}
