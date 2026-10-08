//! Cylindrical bosses (quiddity's `BossRecord`, draftwright's `BossFeature`): diameter, height and
//! location (`render_boss_diameters` in `annotations/from_model.py`, the boss heights in
//! `compiled.py`).
//!
//! Each boss states:
//! - its **diameter**: on a part turned whole, a boss on the turning axis is one band of the
//!   profile, so its diameter is stated across it in a view that shows the profile (draftwright
//!   leads to it from its diameter column there); any other boss reads as a circle looking down
//!   its axis, so its diameter is called out with a leader in that view, identical bosses along
//!   one axis counted once (`2× ø22.9`), as draftwright collapses them;
//! - its **height**, when quiddity gives one: from its base to its top along its axis, in a view
//!   that shows its profile, the extension lines on its rim (draftwright's witness span);
//! - its **location**, as a hole's is: both coordinates of its axis from the part's box in the
//!   view that looks down it, one dimension per distinct distance. draftwright locates no boss;
//!   the port does, since a boss whose position is not stated cannot be made. A boss on a turning
//!   axis needs none (the centre line locates it), nor one a recognised hole runs down (the
//!   hole's location states it).
//!
//! A boss that is a groove's floor is the groove's, as in draftwright and lint
//! ([`crate::lint::bosses::groove_floor`]); it gets no dimensions of its own here.

use std::collections::BTreeMap;

use super::envelope::profile_views;
use super::{
    AXES, Context, Measurement, NAMES, P3, Planner, Problem, Shape, Want, add, end_on, fmt, key,
    point, scale3, want_id,
};
use crate::bridge::Feature;
use crate::bridge::holes::{self, principal};
use crate::lint::bosses::groove_floor;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["boss"],
    wants,
};

/// A boss in the part's frame.
struct Boss<'a> {
    feature: &'a Feature,
    /// The world axis it stands along.
    axis: usize,
    /// The centre of its top, and of its base when its height is known.
    top: P3,
    base: Option<P3>,
    diameter: f64,
    height: Option<f64>,
}

/// A boss as quiddity records it (`location` its top's centre, `axis` outward, `diameter`,
/// `height`), or why it cannot be dimensioned.
fn read<'a>(f: &'a Feature, frame: &super::QFrame) -> Result<Boss<'a>, String> {
    let p = &f.parameters;
    let top = frame.point(point(p.get("location")).ok_or("its record gives no location")?);
    let local = point(p.get("axis")).ok_or("its record gives no axis")?;
    let dir = holes::dir_to_part(&frame.0, local);
    let n = dir.iter().map(|c| c * c).sum::<f64>().sqrt();
    let axis = principal(dir)
        .ok_or("its axis is not along x, y or z, so no standard view looks down it")?;
    let diameter = p
        .get("diameter")
        .and_then(serde_json::Value::as_f64)
        .ok_or("its record gives no diameter")?;
    let height = p
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .filter(|h| *h > 0.0);
    Ok(Boss {
        feature: f,
        axis,
        top,
        base: height.map(|h| add(top, scale3(dir, -h / n))),
        diameter,
        height,
    })
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let analysis = cx.analysis;
    let [lo, _] = analysis.envelope;
    let mut problems = Vec::new();
    let mut bosses = Vec::new();
    for f in analysis.features.iter().filter(|f| f.kind == "boss") {
        if groove_floor(analysis, f) {
            continue;
        }
        match read(f, &cx.frame) {
            Ok(b) => bosses.push(b),
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!("{} is not dimensioned: {why}.", f.id),
                subjects: vec![f.id.clone()],
            }),
        }
    }
    let on_turning = |b: &Boss| {
        cx.turned.as_ref().is_some_and(|t| {
            b.axis == t.axis && (0..3).all(|j| j == t.axis || (b.top[j] - t.on[j]).abs() < 1e-3)
        })
    };
    // Recognised holes, by the axis they lie along and where they pierce the view down it.
    let drilled: Vec<(usize, P3)> = holes::read(analysis)
        .into_iter()
        .filter_map(|(_, h)| {
            let h = h.ok()?;
            Some((h.principal?.0, h.at))
        })
        .collect();

    let mut out = Vec::new();
    let mut circles: BTreeMap<(usize, i64), Vec<usize>> = BTreeMap::new();
    let mut order = Vec::new();
    let mut located: BTreeMap<(&'static str, usize, i64), Vec<usize>> = BTreeMap::new();
    for (i, b) in bosses.iter().enumerate() {
        let r = 0.5 * b.diameter;
        if on_turning(b) {
            let views = profile_views(b.axis);
            let j = (0..3)
                .find(|&j| j != b.axis && profile_views(j).contains(&views[0]))
                .expect("a profile view shows a radial axis");
            let at = match b.base {
                Some(base) => scale3(add(base, b.top), 0.5),
                None => b.top,
            };
            let mut a = at;
            let mut c = at;
            a[j] -= r;
            c[j] += r;
            out.push(Want {
                id: want_id(&b.feature.id, "boss.diameter", views[0]),
                views: vec![views[0]],
                value: b.diameter,
                terms: vec![format!("ø{}", fmt(b.diameter))],
                measurements: vec![Measurement {
                    feature: b.feature.id.clone(),
                    parameter: "boss.diameter".into(),
                    value: b.diameter,
                }],
                turning: None,
                basis: format!(
                    "the part is turned about {}, and this boss is a band of its profile: its diameter is stated across it in a view that shows the profile",
                    AXES[b.axis]
                ),
                shape: Shape::Linear {
                    a,
                    b: c,
                    axis: j,
                    a_edge: false,
                    b_edge: false,
                    order: 0,
                },
            });
        } else {
            let k = (b.axis, key(b.diameter));
            let members = circles.entry(k).or_default();
            if members.is_empty() {
                order.push(k);
            }
            members.push(i);
            let view = end_on(b.axis);
            let coaxial = drilled.iter().any(|(k, at)| {
                *k == b.axis && (0..3).all(|j| j == b.axis || (at[j] - b.top[j]).abs() < 1e-3)
            });
            if !coaxial {
                for a in (0..3).filter(|&a| a != b.axis) {
                    let d = b.top[a] - lo[a];
                    if d.abs() < 1e-6 {
                        let id = &b.feature.id;
                        problems.push(Problem {
                            id: format!("p:location_on_datum:{id}/{}", AXES[a]),
                            code: "location_on_datum".into(),
                            severity: "info".into(),
                            message: format!(
                                "{id} lies on the part's {} face, so its {} location measures nothing and is not dimensioned.",
                                ["left", "front", "bottom"][a],
                                AXES[a]
                            ),
                            subjects: vec![id.clone()],
                        });
                        continue;
                    }
                    located.entry((view, a, key(d))).or_default().push(i);
                }
            }
        }
        if let (Some(h), Some(base)) = (b.height, b.base) {
            let views = profile_views(b.axis);
            let j = (0..3)
                .find(|&j| j != b.axis && profile_views(j).contains(&views[0]))
                .expect("a profile view shows a radial axis");
            let rim = |p: P3| {
                let mut p = p;
                p[j] += r;
                p
            };
            out.push(Want {
                id: want_id(&b.feature.id, "boss_height.length", views[0]),
                views,
                value: h,
                terms: vec![fmt(h)],
                measurements: vec![Measurement {
                    feature: b.feature.id.clone(),
                    parameter: "boss_height.length".into(),
                    value: h,
                }],
                turning: None,
                basis: "a boss's height, from its base to its top along its axis, in a view that shows its profile".into(),
                shape: Shape::Linear {
                    a: rim(base),
                    b: rim(b.top),
                    axis: b.axis,
                    a_edge: false,
                    b_edge: false,
                    order: 0,
                },
            });
        }
    }

    for k in order {
        let members = &circles[&k];
        let first = &bosses[members[0]];
        let view = end_on(first.axis);
        let n = members.len();
        let mut terms = Vec::new();
        if n > 1 {
            terms.push(format!("{n}×"));
        }
        terms.push(format!("ø{}", fmt(first.diameter)));
        out.push(Want {
            id: want_id(&first.feature.id, "boss.diameter", view),
            views: vec![view],
            value: first.diameter,
            terms,
            measurements: members
                .iter()
                .map(|&m| Measurement {
                    feature: bosses[m].feature.id.clone(),
                    parameter: "boss.diameter".into(),
                    value: bosses[m].diameter,
                })
                .collect(),
            turning: None,
            basis: if n > 1 {
                format!("{n} identical bosses along {} are called out once and counted, where they are seen as circles", AXES[first.axis])
            } else {
                "a boss's diameter is called out where it is seen as a circle".into()
            },
            shape: Shape::Callout {
                sites: members
                    .iter()
                    .map(|&m| (bosses[m].top, 0.5 * bosses[m].diameter))
                    .collect(),
            },
        });
    }

    for ((view, a, _), owners) in located {
        let first = &bosses[owners[0]];
        let d = first.top[a] - lo[a];
        let parameter = format!("location.length.{}", AXES[a]);
        let view: &'static str = NAMES.iter().find(|n| **n == view).expect("a view");
        let mut from = first.top;
        from[a] = lo[a];
        out.push(Want {
            id: want_id(&first.feature.id, &parameter, view),
            views: vec![view],
            value: d,
            terms: vec![fmt(d)],
            measurements: owners
                .iter()
                .map(|&i| Measurement {
                    feature: bosses[i].feature.id.clone(),
                    parameter: parameter.clone(),
                    value: d,
                })
                .collect(),
            turning: None,
            basis: format!(
                "located in {} from the part's {} face, in the view that looks down the boss{}",
                AXES[a],
                ["left", "front", "bottom"][a],
                if owners.len() > 1 {
                    "; one dimension serves every boss at this distance"
                } else {
                    ""
                }
            ),
            shape: Shape::Linear {
                a: from,
                b: first.top,
                axis: a,
                a_edge: true,
                b_edge: false,
                order: 1,
            },
        });
    }
    (out, problems)
}
