//! Free-direction slots (quiddity's `OrientedSlot`, the Analysis's `oriented_slot` features):
//! one callout carrying both requirements, `ORIENTED SLOT 2 WIDE × 4.5 LONG`
//! (LINT-oriented_slot_coverage-width_length, ISSUE-1432-a; draftwright's `render_oriented_slots`
//! and `_oriented_slot_label` in `annotations/from_model.py`).
//!
//! - **Its view** looks along the slot's dominant run axis (draftwright's `_END_ON` of the slot
//!   frame's axis, the run direction's largest component).
//! - **Its tip** is on the slot's physical rim: the record's section boundary (`source.section`,
//!   in the passage frame `source.frame`) at the end of its run nearer the viewer
//!   (`_oriented_slot_rim_polygon`), its edges' midpoints the leader's candidate targets, so a
//!   rotated slot's leader lands on its rotated rim, not on an axis-aligned box.
//! - Equal slots seen in one view (one width and length) share one counted callout,
//!   `2× ORIENTED SLOT …`, claiming every member. draftwright leaves the members of oriented slot
//!   patterns undrawn (#1430: patterns have no grammar there); the Analysis does not record the
//!   patterns (the bridge reads their members as oriented slots), so their sizes are stated here
//!   through the counted callout, never left unstated.
//!
//! A record whose sizes, frame or section cannot be read is `feature_not_dimensioned`.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Context, Measurement, P3, Planner, Problem, Shape, Want, end_on, fmt, key, want_id};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, to_part};

pub(super) const PLANNER: Planner = Planner {
    kinds: &["oriented_slot"],
    wants,
};

fn triple(v: Option<&Value>) -> Option<P3> {
    super::point(v)
}

/// A slot read in the part's frame: its width and length, its dominant run axis, and its rim's
/// edge midpoints at the run's end nearer the viewer of that axis's view.
struct Read {
    width: f64,
    length: f64,
    axis: usize,
    rim: Vec<P3>,
}

fn read(analysis: &crate::model::Analysis, f: &Feature) -> Result<Read, String> {
    let p = &f.parameters;
    let size = |name: &str| {
        p.get(name)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| format!("its record gives no positive {name}"))
    };
    let (width, length) = (size("width")?, size("length")?);
    let source = p.get("source").ok_or("its record gives no passage")?;
    let fr = source
        .get("frame")
        .ok_or("its record gives no passage frame")?;
    let q = frame(analysis);
    let origin = triple(fr.get("origin")).ok_or("its passage frame has no origin")?;
    let [run, u, v] = ["run", "u", "v"].map(|k| triple(fr.get(k)));
    let (Some(run), Some(u), Some(v)) = (run, u, v) else {
        return Err("its passage frame has no run, u or v direction".into());
    };
    let (origin, run, u, v) = (
        to_part(&q, origin),
        dir_to_part(&q, run),
        dir_to_part(&q, u),
        dir_to_part(&q, v),
    );
    let interval: Vec<f64> = source
        .get("run_interval")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let [lo, hi] = <[f64; 2]>::try_from(interval).map_err(|_| "its passage has no run interval")?;
    let boundary: Vec<[f64; 2]> = source
        .get("section")
        .and_then(|s| s.get("boundary"))
        .and_then(Value::as_array)
        .map(|b| {
            b.iter()
                .filter_map(|e| {
                    let c = e.get("point")?.as_array()?;
                    Some([c.first()?.as_f64()?, c.get(1)?.as_f64()?])
                })
                .collect()
        })
        .unwrap_or_default();
    if boundary.len() < 3 {
        return Err("its passage section has no boundary".into());
    }
    let axis = (0..3)
        .max_by(|&i, &j| run[i].abs().total_cmp(&run[j].abs()))
        .expect("three axes");
    let toward = super::TOWARD
        .iter()
        .find(|(n, _)| *n == end_on(axis))
        .map(|(_, t)| *t)
        .expect("a view along each axis");
    let facing = (0..3).map(|k| run[k] * toward[k]).sum::<f64>() > 0.0;
    let station = if facing { hi } else { lo };
    let at = |c: [f64; 2]| -> P3 {
        [0, 1, 2].map(|k| origin[k] + station * run[k] + c[0] * u[k] + c[1] * v[k])
    };
    let corners: Vec<P3> = boundary.iter().map(|c| at(*c)).collect();
    let rim = (0..corners.len())
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            [0, 1, 2].map(|k| 0.5 * (a[k] + b[k]))
        })
        .collect();
    Ok(Read {
        width,
        length,
        axis,
        rim,
    })
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut problems = Vec::new();
    // Slots by view axis, width and length, in the Analysis's order of their first member.
    type Key = (usize, i64, i64);
    let mut groups: BTreeMap<Key, Vec<(&Feature, Read)>> = BTreeMap::new();
    let mut order: Vec<Key> = Vec::new();
    for f in cx
        .analysis
        .features
        .iter()
        .filter(|f| f.kind == "oriented_slot")
    {
        match read(cx.analysis, f) {
            Ok(r) => {
                let k = (r.axis, key(r.width), key(r.length));
                if !groups.contains_key(&k) {
                    order.push(k);
                }
                groups.entry(k).or_default().push((f, r));
            }
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!(
                    "{} is not dimensioned: {why}.",
                    crate::wording::capital(&crate::wording::feature(f))
                ),
                subjects: vec![f.id.clone()],
                ..Default::default()
            }),
        }
    }
    let mut out = Vec::new();
    for k in order {
        let members = &groups[&k];
        let (first, r0) = &members[0];
        let n = members.len();
        let mut terms = Vec::new();
        if n > 1 {
            terms.push(format!("{n}×"));
        }
        terms.extend([
            "ORIENTED".into(),
            "SLOT".into(),
            fmt(r0.width),
            "WIDE".into(),
            "×".into(),
            fmt(r0.length),
            "LONG".into(),
        ]);
        out.push(Want {
            id: want_id(&first.id, "oriented_slot_width.length"),
            views: vec![end_on(r0.axis)],
            value: r0.width,
            terms,
            measurements: members
                .iter()
                .flat_map(|(f, r)| {
                    [
                        ("oriented_slot_width.length", r.width),
                        ("oriented_slot_length.length", r.length),
                    ]
                    .map(|(parameter, value)| Measurement {
                        feature: f.id.clone(),
                        parameter: parameter.into(),
                        value,
                    })
                })
                .collect(),
            turning: None,
            basis: if n > 1 {
                format!(
                    "{n} free-direction slots of one width and length are called out once and counted, in the view along their run, the leader to a rim"
                )
            } else {
                "a free-direction slot's width and length in one callout, in the view along its run, the leader to its rim".into()
            },
            shape: Shape::Callout {
                sites: members
                    .iter()
                    .flat_map(|(_, r)| r.rim.iter().map(|p| (*p, 0.0)))
                    .collect(),
            },
        });
    }
    (out, problems)
}
