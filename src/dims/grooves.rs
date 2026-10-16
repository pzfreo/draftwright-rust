//! Turned grooves (quiddity's `Groove`, draftwright's `GrooveFeature`): one callout each,
//! `10 WIDE × ø122`, its axial width and floor diameter (`render_grooves` and `_groove_label` in
//! `annotations/from_model.py`).
//!
//! - **View:** the groove's width is axial, so it reads where the turning axis lies in the view,
//!   the groove a notch in the silhouette: [`profile_views`] its axis (draftwright's `_EDGE_ON`:
//!   x and z in the front, y in the side). draftwright skips the groove when that view is not
//!   shown; the port falls back to the next view that shows the profile.
//! - **Grouping:** none. Each groove is called out on its own at its own axial position, as
//!   draftwright does (two identical grooves on one shaft must each be dimensioned).
//! - **Leader site:** the floor's silhouette, the record's `at` (on the axis) moved out by the
//!   floor radius across the axis.
//!
//! A groove whose record lacks its width, floor diameter, position or a principal axis is
//! reported (`feature_not_dimensioned`), never dropped; so is one on a part of several turned
//! profiles whose record names none of them (ADR3-12: it has no one owner). A boss that is a groove's floor is left
//! to the groove ([`super::bosses`]).

use serde_json::Value;

use super::envelope::profile_views;
use super::{AXES, Context, Measurement, P3, Planner, Problem, Shape, Want, fmt, point, want_id};
use crate::bridge::Feature;
use crate::bridge::holes::principal;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["groove"],
    wants,
};

/// The warning for a recognised feature the planner cannot state (as [`super::bosses`] gives).
pub(super) fn not_dimensioned(f: &Feature, why: &str) -> Problem {
    Problem {
        id: format!("p:feature_not_dimensioned:{}", f.id),
        code: "feature_not_dimensioned".into(),
        severity: "warning".into(),
        message: format!(
            "{} is not dimensioned: {why}.",
            crate::wording::capital(&crate::wording::feature(f))
        ),
        subjects: vec![f.id.clone()],
        ..Default::default()
    }
}

/// A record's size *name*: a positive number, or why not.
pub(super) fn size(f: &Feature, name: &str) -> Result<f64, String> {
    f.parameters
        .get(name)
        .and_then(Value::as_f64)
        .filter(|x| *x > 0.0)
        .ok_or_else(|| format!("its record gives no positive {name}"))
}

/// The groove's callout, or why it cannot be stated.
fn want(cx: &Context, f: &Feature) -> Result<Want, String> {
    let p = &f.parameters;
    let width = size(f, "width")?;
    let diameter = size(f, "diameter")?;
    let at = cx
        .frame
        .point(point(p.get("at")).ok_or("its record gives no at")?);
    let axis = p
        .get("axis")
        .and_then(|a| cx.frame.letter(a))
        .ok_or("its record gives no axis")?;
    let k = principal(axis)
        .ok_or("its axis is not along x, y or z, so no standard view shows its profile")?;
    let r = 0.5 * diameter;
    let sites: Vec<(P3, f64)> = (0..3)
        .filter(|&j| j != k)
        .flat_map(|j| {
            [-r, r].map(|d| {
                let mut s = at;
                s[j] += d;
                (s, 0.0)
            })
        })
        .collect();
    let views = profile_views(k);
    Ok(Want {
        id: want_id(&f.id, "groove.length"),
        views,
        value: width,
        terms: vec![
            fmt(width),
            "WIDE".into(),
            "×".into(),
            format!("ø{}", fmt(diameter)),
        ],
        measurements: vec![
            Measurement {
                feature: f.id.clone(),
                parameter: "groove.length".into(),
                value: width,
            },
            Measurement {
                feature: f.id.clone(),
                parameter: "groove.diameter".into(),
                value: diameter,
            },
        ],
        turning: None,
        basis: format!(
            "a groove's axial width and floor diameter are called out together, in a view that shows the profile about {}",
            AXES[k]
        ),
        shape: Shape::Callout { sites },
    })
}

/// ADR3-12: on a part of several turned profiles (the turned steps' recorded profiles), a groove
/// whose record names none of them cannot name its one owner, so it is refused rather than
/// called out on whichever body happens to lie under it.
fn owner(cx: &Context, f: &Feature) -> Result<(), String> {
    let mut profiles: Vec<&Value> = cx
        .analysis
        .features
        .iter()
        .filter(|s| s.kind == "turned_step")
        .filter_map(|s| s.parameters.get("profile"))
        .collect();
    profiles.sort_by_key(|p| p.to_string());
    profiles.dedup();
    if profiles.len() < 2
        || f.parameters
            .get("profile")
            .is_some_and(|p| profiles.contains(&p))
    {
        return Ok(());
    }
    Err(format!(
        "the part has {} turned profiles and its record names none of them, so it has no one owner",
        profiles.len()
    ))
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for f in cx.analysis.features.iter().filter(|f| f.kind == "groove") {
        match owner(cx, f).and_then(|()| want(cx, f)) {
            Ok(w) => out.push(w),
            Err(why) => problems.push(not_dimensioned(f, &why)),
        }
    }
    (out, problems)
}
