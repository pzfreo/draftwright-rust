//! A part turned whole: its overall diameter and its length along the turning axis, in a view
//! that shows the profile. No turned step's own diameter or length is planned yet, so the planner
//! claims no feature kind and each `turned_step` is reported as not dimensioned.

use super::envelope::{corner, profile_views};
use super::{AXES, Context, Measurement, P3, Planner, Shape, Want, fmt, point, want_id};
use crate::bridge::{Feature, holes};
use crate::model::Analysis;

pub(super) const PLANNER: Planner = Planner { kinds: &[], wants };

/// A part turned whole: its axis (a world axis), a point on it, and its largest diameter, when
/// every turned step shares one principal axis and the largest step spans the part's box across
/// it.
pub(super) struct Turned {
    pub(super) axis: usize,
    pub(super) on: P3,
    pub(super) diameter: f64,
}

pub(super) fn turned(analysis: &Analysis, frame: &super::QFrame) -> Option<Turned> {
    let steps: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .collect();
    let mut found: Option<(usize, P3)> = None;
    let mut diameter: f64 = 0.0;
    for s in &steps {
        let p = &s.parameters;
        let k = holes::principal(frame.letter(p.get("axis")?)?)?;
        let on = frame.point(point(p.get("profile")?.get("axis_origin"))?);
        if let Some((k0, on0)) = found
            && (k0 != k || (0..3).any(|j| j != k && (on[j] - on0[j]).abs() > 1e-3))
        {
            return None;
        }
        found = Some((k, on));
        diameter = diameter.max(p.get("diameter")?.as_f64()?);
    }
    let (axis, on) = found?;
    let [lo, hi] = analysis.envelope;
    let whole = (0..3)
        .filter(|&j| j != axis)
        .all(|j| ((hi[j] - lo[j]) - diameter).abs() < 1e-3 * diameter.max(1.0));
    whole.then_some(Turned { axis, on, diameter })
}

/// The overall diameter and length of a part turned whole; nothing for any other part.
fn wants(cx: &Context) -> (Vec<Want>, Vec<super::Problem>) {
    let Some(t) = &cx.turned else {
        return (Vec::new(), Vec::new());
    };
    let [lo, hi] = cx.analysis.envelope;
    let mut out = Vec::new();
    let views = profile_views(t.axis);
    // Across the axis, in the first of its profile views: the radial axis that view shows.
    let radial = (0..3)
        .find(|&j| j != t.axis && profile_views(j).contains(&views[0]))
        .unwrap_or((t.axis + 1) % 3);
    out.push(Want {
        id: want_id("envelope", "od.diameter", views[0]),
        views: views.clone(),
        value: t.diameter,
        terms: vec![format!("ø{}", fmt(t.diameter))],
        measurements: vec![Measurement {
            feature: "envelope".into(),
            parameter: "od.diameter".into(),
            value: t.diameter,
        }],
        turning: Some(t.axis),
        basis: format!(
            "the part is turned whole about {}: its largest diameter is its overall size across the axis, in a view that shows the profile",
            AXES[t.axis]
        ),
        shape: Shape::Linear {
            a: corner(radial, lo[radial], lo),
            b: corner(radial, hi[radial], lo),
            axis: radial,
            a_edge: true,
            b_edge: true,
            order: 2,
        },
    });
    let k = t.axis;
    let name = super::envelope::NAMES[k];
    out.push(Want {
        id: want_id("envelope", &format!("{name}.length"), views[0]),
        views,
        value: hi[k] - lo[k],
        terms: vec![fmt(hi[k] - lo[k])],
        measurements: vec![Measurement {
            feature: "envelope".into(),
            parameter: format!("{name}.length"),
            value: hi[k] - lo[k],
        }],
        turning: None,
        basis: "the overall length along the turning axis, in a view that shows the profile; no step chain is planned yet to state it (as draftwright releases the overall length when its chain is dropped)".into(),
        shape: Shape::Linear {
            a: corner(k, lo[k], lo),
            b: corner(k, hi[k], lo),
            axis: k,
            a_edge: true,
            b_edge: true,
            order: 2,
        },
    });
    (out, Vec::new())
}
