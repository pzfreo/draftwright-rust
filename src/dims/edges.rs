//! Edge treatments: a fillet's radius (`R3`) and a chamfer's size (`C1` for equal legs at 45°,
//! otherwise `4.5 × 42.1°`), as draftwright calls them out (`annotations/_edge_callouts.py`).
//!
//! - **Grouping:** fillets that print the same radius share one `n× R` callout; chamfers share
//!   one when their legs and angle are equal, not merely their printed first leg (ISSUE-1254-a).
//!   The callout states every member's measurement, and points at the members of its most
//!   populous axis and view (ties to the first in axis, then view, order), as draftwright's
//!   `radius_jobs` and `chamfer_jobs` do; members along other axes are counted on it too.
//! - **View:** a prismatic treatment reads where its edge is seen end-on (the rounded edge as an
//!   arc, the bevel edge-on), [`end_on`] its axis; a turned one (quiddity's `turned`, a toroidal
//!   round or conical bevel about the shaft) reads in the shaft's profile, [`profile_views`]. A
//!   record is never guessed to be turned (ISSUE-1281-a).
//! - **Leader site:** the record's `at`. A turned treatment on a part turned whole is rotated
//!   about the turning axis onto the profile its view shows, as draftwright's
//!   `_turned_profile_site` does about the nearest external shaft (ISSUE-1276-a, in part: the
//!   port knows only the whole part's axis, so on any other part the physical site is kept).
//! - **Oriented chamfers** (quiddity's `OrientedChamfer`) are not dimensioned. quiddity records
//!   one only along an oblique edge (`oriented_chamfers.py` rejects a run within
//!   `AXIS_ALIGNED_COS` of an axis), which no standard view sees edge-on, and draftwright has no
//!   reviewed callout for it (`detect.py`'s `_UNCONSUMED_RECORDS`, #1365); the kind stays in
//!   `dims::UNDRAFTED`.
//!
//! A record whose sizes or direction cannot be read is reported (`feature_not_dimensioned`),
//! never dropped.

use std::collections::BTreeMap;

use serde_json::Value;

use super::envelope::profile_views;
use super::turned::Turned;
use super::{AXES, Context, Measurement, P3, Planner, Problem, Shape, Want, end_on, fmt, key};
use crate::bridge::Feature;
use crate::bridge::holes::{point, principal};

pub(super) const PLANNER: Planner = Planner {
    kinds: &["chamfer", "fillet"],
    wants,
};

/// Whether a chamfer of these sizes is called out `C<leg>`: draftwright's `_fmt_chamfer`.
fn c_form(leg1: f64, leg2: f64, angle: f64) -> bool {
    (leg1 - leg2).abs() < 0.05 && (angle - 45.0).abs() < 0.5
}

/// A recognised fillet or chamfer the callout grammar can state.
struct Edge<'a> {
    feature: &'a Feature,
    /// `fillet.radius` or `chamfer.length`.
    parameter: &'static str,
    /// The world axis of its edge, or of its shaft when it is turned.
    axis: usize,
    turned: bool,
    /// Its record's `at`, in the part's frame.
    at: P3,
    /// The radius, or the first leg.
    value: f64,
    /// The terms after the count.
    terms: Vec<String>,
    /// What makes two of them one requirement: the printed radius, or the legs and angle.
    same: Vec<i64>,
    measurements: Vec<Measurement>,
}

impl Edge<'_> {
    /// The views that can carry it, preferred first.
    fn views(&self) -> Vec<&'static str> {
        if self.turned {
            profile_views(self.axis)
        } else {
            vec![end_on(self.axis)]
        }
    }
}

/// A record's size *name*: a positive number, or why not.
fn size(f: &Feature, name: &str) -> Result<f64, String> {
    let x = f
        .parameters
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("its record gives no {name}"))?;
    if x.is_finite() && x > 0.0 {
        Ok(x)
    } else {
        Err(format!("its {name} is {x}, not a positive size"))
    }
}

/// A recognised fillet or chamfer as the grammar reads it, or why it cannot be stated.
fn edge<'a>(cx: &Context, f: &'a Feature) -> Result<Edge<'a>, String> {
    let frame = &cx.frame;
    let at = point(f.parameters.get("at"))
        .map(|l| frame.point(l))
        .ok_or("its record gives no point on it (at)")?;
    let turned = f.parameters.get("turned").and_then(Value::as_bool) == Some(true);
    let letter = f.parameters.get("axis").ok_or("its record gives no axis")?;
    let d = frame.letter(letter).ok_or("its axis is not x, y or z")?;
    let axis = principal(d).ok_or("its axis is not along x, y or z in the part's frame")?;
    let m = |parameter: &str, value: f64| Measurement {
        feature: f.id.clone(),
        parameter: parameter.into(),
        value,
    };
    if f.kind == "fillet" {
        let r = size(f, "radius")?;
        let label = format!("R{}", fmt(r));
        return Ok(Edge {
            feature: f,
            parameter: "fillet.radius",
            axis,
            turned,
            at,
            value: r,
            same: label.bytes().map(i64::from).collect(),
            terms: vec![label],
            measurements: vec![m("fillet.radius", r)],
        });
    }
    let (leg1, leg2, angle) = (size(f, "leg1")?, size(f, "leg2")?, size(f, "angle")?);
    let mut measurements = vec![m("chamfer.length", leg1)];
    let terms = if c_form(leg1, leg2, angle) {
        vec![format!("C{}", fmt(leg1))]
    } else {
        measurements.push(m("chamfer.angle", angle));
        vec![fmt(leg1), "×".into(), format!("{}°", fmt(angle))]
    };
    Ok(Edge {
        feature: f,
        parameter: "chamfer.length",
        axis,
        turned,
        at,
        value: leg1,
        // draftwright's key: the legs to 1e-3 mm and the angle to 0.01°.
        same: vec![
            (leg1 * 1e3).round() as i64,
            (leg2 * 1e3).round() as i64,
            (angle * 1e2).round() as i64,
        ],
        terms,
        measurements,
    })
}

/// Where the leader points: the record's point *at*, or for a turned treatment about *axis* on a
/// part turned whole about it (*whole*), that point turned about the axis onto the profile, on the
/// side it already lies. The site is set at the treatment's radius from the axis along both
/// transverse axes, so it projects onto the profile in either profile view, whichever placement
/// uses (only its projection is used).
fn site(whole: Option<&Turned>, at: P3, axis: usize, turned: bool) -> P3 {
    let Some(t) = whole.filter(|t| turned && t.axis == axis) else {
        return at;
    };
    let (j, k) = ((axis + 1) % 3, (axis + 2) % 3);
    let r = (at[j] - t.on[j]).hypot(at[k] - t.on[k]);
    let mut p = at;
    for c in [j, k] {
        let sign = if at[c] - t.on[c] < -1e-9 { -1.0 } else { 1.0 };
        p[c] = t.on[c] + sign * r;
    }
    p
}

/// A group's key: its family (chamfers 0, fillets 1) and what its members share ([`Edge::same`]).
type Key = (u8, Vec<i64>);

/// The fillets' and chamfers' callouts, with the problems met on the way.
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let analysis = cx.analysis;
    let mut problems = Vec::new();
    // A group is one requirement: fillets that print the same radius, chamfers with the same
    // legs and angle.
    let mut groups: BTreeMap<Key, Vec<Edge>> = BTreeMap::new();
    for f in &analysis.features {
        let family = match f.kind.as_str() {
            "chamfer" => 0,
            "fillet" => 1,
            _ => continue,
        };
        match edge(cx, f) {
            Ok(e) => groups.entry((family, e.same.clone())).or_default().push(e),
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!("{} is not dimensioned: {why}.", f.id),
                subjects: vec![f.id.clone()],
            }),
        }
    }
    // Chamfers before fillets, as draftwright's orchestrator runs them; within each, smallest
    // first.
    let mut groups: Vec<(Key, Vec<Edge>)> = groups.into_iter().collect();
    groups.sort_by_cached_key(|((family, same), members)| {
        let least = members.iter().map(|e| key(e.value)).min();
        (*family, least, same.clone())
    });

    let rank = |id: &str| analysis.features.iter().position(|f| f.id == id);
    let mut out = Vec::new();
    for ((family, _), members) in groups {
        // The presentation the leader points into: the most populous axis and view.
        let mut by: BTreeMap<(&str, &str), Vec<&Edge>> = BTreeMap::new();
        for e in &members {
            by.entry((AXES[e.axis], e.views()[0])).or_default().push(e);
        }
        let (_, visible) = by
            .iter()
            .min_by_key(|(k, v)| (std::cmp::Reverse(v.len()), **k))
            .expect("a group has a member");
        // Its representative, which names the item: the member first by its point, as
        // draftwright sorts by `frame.origin` (then by feature order, for a tie).
        let first = visible
            .iter()
            .min_by(|a, b| {
                (a.at.iter().zip(&b.at).map(|(x, y)| x.total_cmp(y)))
                    .find(|o| o.is_ne())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| rank(&a.feature.id).cmp(&rank(&b.feature.id)))
            })
            .expect("a presentation has a member");
        let views = first.views();
        let n = members.len();
        let mut terms = Vec::new();
        if n > 1 {
            terms.push(format!("{n}×"));
        }
        terms.extend(first.terms.iter().cloned());
        let (noun, size, seen) = if family == 0 {
            ("chamfer", "size", "its bevel is seen edge-on")
        } else {
            ("fillet", "radius", "its rounded edge is seen as an arc")
        };
        let mut basis = match (n, first.turned) {
            (1, false) => format!("a {noun}'s {size} is called out where {seen}"),
            (1, true) => format!("a turned {noun}'s {size} is called out in the shaft's profile"),
            (_, false) => format!(
                "equal {noun}s are called out once and counted, where {seen} (draftwright's n× callout)"
            ),
            (_, true) => format!(
                "equal turned {noun}s are called out once and counted, in the shaft's profile (draftwright's n× callout)"
            ),
        };
        if family == 0 && first.terms.len() > 1 {
            basis += "; its legs differ or its angle is not 45°, so it states its first leg and its angle";
        }
        let elsewhere = n - visible.len();
        if elsewhere > 0 {
            basis += &format!(
                "; {elsewhere} of them lie along another axis or read in another view and are counted here, as draftwright counts them"
            );
        }
        let sites = visible
            .iter()
            .map(|e| (site(cx.turned.as_ref(), e.at, e.axis, e.turned), 0.0))
            .collect();
        out.push(Want {
            id: super::want_id(&first.feature.id, first.parameter),
            views,
            value: first.value,
            terms,
            measurements: members
                .iter()
                .flat_map(|e| e.measurements.iter().cloned())
                .collect(),
            basis,
            shape: Shape::Callout { sites },
            turning: None,
        });
    }
    (out, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// draftwright's `_fmt_chamfer`: equal legs (within 0.05 mm) at 45° (within 0.5°) are `C`.
    #[test]
    fn the_c_form_is_equal_legs_at_45_degrees() {
        assert!(c_form(5.0, 5.0, 45.0));
        assert!(c_form(5.0, 5.04, 45.4));
        assert!(!c_form(5.0, 5.06, 45.0));
        assert!(!c_form(5.0, 5.0, 45.6));
        assert!(!c_form(31.75, 14.805, 25.0));
    }

    /// ISSUE-1276-a, in part: a turned treatment's leader site is set at its radius off the
    /// part's turning axis along both transverse axes, keeping its axial station and side, so it
    /// projects onto the profile in either profile view (front shows x, side y); anything else
    /// keeps its physical point.
    #[test]
    fn a_turned_site_turns_onto_the_profile() {
        let t = Turned {
            axis: 2,
            on: [1.0, 2.0, 0.0],
            diameter: 20.0,
        };
        // 3-4-5 off the axis, behind the front view's plane.
        let at = [1.0 + 3.0, 2.0 + 4.0, 7.0];
        assert_eq!(site(Some(&t), at, 2, true), [6.0, 7.0, 7.0]);
        let below = [1.0 - 3.0, 2.0 + 4.0, 7.0];
        assert_eq!(site(Some(&t), below, 2, true), [-4.0, 7.0, 7.0]);
        // Not turned, about another axis, or on a part not turned whole: unchanged.
        assert_eq!(site(Some(&t), at, 2, false), at);
        assert_eq!(site(Some(&t), at, 0, true), at);
        assert_eq!(site(None, at, 2, true), at);
    }
}
