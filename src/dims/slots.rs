//! Through slots (quiddity's `Slot`, draftwright's `SlotFeature`): width, length, an obround's
//! end radius and the location of its low end, in the view that looks through it
//! (`annotations/_slots.py`, `compiled.py`'s slot location).
//!
//! Each lone slot states:
//! - its **width** across its width axis, and its **length** along its long axis, as linear
//!   dimensions whose extension lines start at the slot's own edges (draftwright's witnesses);
//!   slots whose widths span the same stretch of a view's horizontal axis (or any axis in the
//!   front) share one width, counted (`2× 6`), as draftwright's shared width does;
//! - for an **obround** (its ends proven arcs: quiddity's `end_radius`), the end radius as a
//!   leader to an end, `2× R3`. A straight slot (no end radius, perhaps a `corner_radius`) states
//!   no radius: its corner blends are the fillets' (draftwright prints none for it either);
//! - its **location**: the distance along its long axis from the part's box to its low end
//!   (draftwright's `location_slot.length`, datum `bbox`). As in draftwright, its position across
//!   the width axis is not dimensioned.
//!
//! A slot inside a recognised slot pattern (`slot_pattern`) is the pattern's, as in draftwright;
//! the pattern's grouped callout is not ported, and the pattern is reported as not dimensioned.
//! Blind slots (draftwright's rectangular and round-bottom blind slots) reach the Analysis as
//! `section_recess` features, which no planner reads yet.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{
    AXES, Context, Measurement, P3, Planner, Problem, QFrame, Shape, Want, end_on, fmt, key,
    want_id,
};
use crate::bridge::Feature;
use crate::bridge::holes::principal;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["slot"],
    wants,
};

/// A slot in the part's frame.
struct Slot<'a> {
    feature: &'a Feature,
    /// The world axes it runs along, spans across and passes through.
    long: usize,
    wide: usize,
    deep: usize,
    width: f64,
    length: f64,
    end_radius: Option<f64>,
    /// Its ends along `long`, low first.
    ends: [f64; 2],
    /// Its centre line's coordinate on `wide`, and its mid-thickness on `deep`.
    centre: f64,
    mid: f64,
}

impl Slot<'_> {
    /// The point at *long* along it and *wide* across it, at mid-thickness.
    fn at(&self, long: f64, wide: f64) -> P3 {
        let mut p = [0.0; 3];
        p[self.long] = long;
        p[self.wide] = wide;
        p[self.deep] = self.mid;
        p
    }

    /// Its edge on *axis* (`long` or `wide`) facing *sign*.
    fn edge(&self, axis: usize, sign: f64) -> f64 {
        if axis == self.long {
            if sign > 0.0 {
                self.ends[1]
            } else {
                self.ends[0]
            }
        } else {
            self.centre + sign * 0.5 * self.width
        }
    }
}

fn number(f: &Feature, name: &str) -> Result<f64, String> {
    f.parameters
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("its record gives no {name}"))
}

/// A slot as quiddity records it (`long_axis`, `width_axis` as letters of its frame; `lo`, `hi`
/// along the first, `w_center` on the second, `d_lo`, `d_hi` through it), or why it cannot be
/// dimensioned.
fn read<'a>(f: &'a Feature, frame: &QFrame) -> Result<Slot<'a>, String> {
    let p = &f.parameters;
    let letter = |name: &str| {
        p.get(name)
            .and_then(Value::as_str)
            .and_then(|a| AXES.iter().position(|x| *x == a))
            .ok_or_else(|| format!("its record gives no {name}"))
    };
    let (ql, qw) = (letter("long_axis")?, letter("width_axis")?);
    if ql == qw {
        return Err("its long and width axes are the same".into());
    }
    let qd = 3 - ql - qw;
    let principal_of = |q: usize| {
        principal(frame.letter(&Value::from(AXES[q])).expect("a letter"))
            .ok_or("its axes lie along no axis of the part, so no standard view looks through it")
    };
    let (long, wide) = (principal_of(ql)?, principal_of(qw)?);
    let deep = 3 - long - wide;
    let (width, length) = (number(f, "width")?, number(f, "length")?);
    let (lo, hi, w) = (number(f, "lo")?, number(f, "hi")?, number(f, "w_center")?);
    let mid = 0.5 * (number(f, "d_lo")? + number(f, "d_hi")?);
    let point = |t: f64| {
        let mut q = [0.0; 3];
        q[ql] = t;
        q[qw] = w;
        q[qd] = mid;
        frame.point(q)
    };
    let (a, b) = (point(lo), point(hi));
    // quiddity rounds `lo`, `hi` and `w_center` to 0.01 mm in its own frame, whose origin is not
    // round in the part's, so a slot 55 mm from an edge reads 54.998 there. The box around its
    // faces (the fingerprint's position) is exact; it is taken when it agrees within that
    // rounding, and the record otherwise.
    let c = f.fingerprint.position;
    let exact = |k: usize, recorded: f64| {
        if (c[k] - recorded).abs() <= 0.01 {
            c[k]
        } else {
            recorded
        }
    };
    let along = exact(long, 0.5 * (a[long] + b[long]));
    Ok(Slot {
        feature: f,
        long,
        wide,
        deep,
        width,
        length,
        end_radius: p.get("end_radius").and_then(Value::as_f64),
        ends: [along - 0.5 * length, along + 0.5 * length],
        centre: exact(wide, a[wide]),
        mid: a[deep],
    })
}

/// The world axis an extension line runs along, and its sign, from a dimension measured along
/// *m* in *view* out to the strip placement puts it in (`placement::strip`: horizontal ones above
/// the plan and below the front and side, vertical ones left of the plan and right of the front
/// and side), under the standard views' orientations (`model`: the plan shows x across and y up,
/// the front x and z, the side y and z).
fn outward(view: &str, m: usize) -> (usize, f64) {
    match (view, m) {
        ("plan", 0) => (1, 1.0),
        ("plan", _) => (0, -1.0),
        ("front", 0) | ("side", 1) => (2, -1.0),
        ("front", _) => (0, 1.0),
        _ => (1, 1.0),
    }
}

/// The world axis a view shows across the page.
fn horizontal(view: &str) -> usize {
    if view == "side" { 1 } else { 0 }
}

/// A linear size of *s* along *m*, its extension lines from the slot's edge facing their strip.
fn size(s: &Slot, view: &str, m: usize, lo: f64, hi: f64, order: u8) -> Shape {
    let (e, sign) = outward(view, m);
    let witness = s.edge(e, sign);
    let point = |t: f64| {
        let mut p = s.at(0.0, 0.0);
        p[m] = t;
        p[e] = witness;
        p
    };
    Shape::Linear {
        a: point(lo),
        b: point(hi),
        axis: m,
        a_edge: false,
        b_edge: false,
        order,
    }
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let analysis = cx.analysis;
    let [lo, _] = analysis.envelope;
    let patterns: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot_pattern")
        .collect();
    let mut problems = Vec::new();
    let mut slots = Vec::new();
    for f in analysis.features.iter().filter(|f| f.kind == "slot") {
        if patterns
            .iter()
            .any(|p| f.faces.iter().all(|i| p.faces.binary_search(i).is_ok()))
        {
            continue; // the pattern's (reported as not dimensioned)
        }
        match read(f, &cx.frame) {
            Ok(s) => slots.push(s),
            Err(why) => problems.push(Problem {
                id: format!("p:feature_not_dimensioned:{}", f.id),
                code: "feature_not_dimensioned".into(),
                severity: "warning".into(),
                message: format!("{} is not dimensioned: {why}.", f.id),
                subjects: vec![f.id.clone()],
            }),
        }
    }

    // Widths spanning the same stretch of one axis share one dimension where draftwright solves
    // them together: across the view, or in the front.
    let mut widths: BTreeMap<(&str, usize, i64, i64, i64), Vec<usize>> = BTreeMap::new();
    let mut order = Vec::new();
    for (i, s) in slots.iter().enumerate() {
        let view = end_on(s.deep);
        let k = if view == "front" || s.wide == horizontal(view) {
            (
                view,
                s.wide,
                key(s.edge(s.wide, -1.0)),
                key(s.edge(s.wide, 1.0)),
                key(s.width),
            )
        } else {
            (view, s.wide, i64::MIN, i as i64, 0)
        };
        let members = widths.entry(k).or_default();
        if members.is_empty() {
            order.push(k);
        }
        members.push(i);
    }

    let mut out = Vec::new();
    for k in order {
        let members = &widths[&k];
        let s = &slots[members[0]];
        let view = k.0;
        let n = members.len();
        let mut terms = Vec::new();
        if n > 1 {
            terms.push(format!("{n}×"));
        }
        terms.push(fmt(s.width));
        out.push(Want {
            id: want_id(&s.feature.id, "slot_width.length", view),
            views: vec![view],
            value: s.width,
            terms,
            measurements: members
                .iter()
                .map(|&m| Measurement {
                    feature: slots[m].feature.id.clone(),
                    parameter: "slot_width.length".into(),
                    value: slots[m].width,
                })
                .collect(),
            turning: None,
            basis: if n > 1 {
                format!(
                    "{n} slots whose widths span the same stretch in {} share one width, counted, in the view that looks through them",
                    AXES[s.wide]
                )
            } else {
                "a slot's width, across it, in the view that looks through it".into()
            },
            shape: size(
                s,
                view,
                s.wide,
                s.edge(s.wide, -1.0),
                s.edge(s.wide, 1.0),
                0,
            ),
        });
    }

    for s in &slots {
        let view = end_on(s.deep);
        let id = &s.feature.id;
        let m = |parameter: &str, value: f64| Measurement {
            feature: id.clone(),
            parameter: parameter.into(),
            value,
        };
        out.push(Want {
            id: want_id(id, "slot_length.length", view),
            views: vec![view],
            value: s.length,
            terms: vec![fmt(s.length)],
            measurements: vec![m("slot_length.length", s.length)],
            turning: None,
            basis: "a slot's length, end to end along it, in the view that looks through it".into(),
            shape: size(s, view, s.long, s.ends[0], s.ends[1], 0),
        });
        if let Some(r) = s.end_radius {
            out.push(Want {
                id: want_id(id, "slot_end_radius.radius", view),
                views: vec![view],
                value: r,
                terms: vec!["2×".into(), format!("R{}", fmt(r))],
                measurements: vec![m("slot_end_radius.radius", r)],
                turning: None,
                basis: "an obround slot's ends are proven arcs: their radius is called out once for both, with a leader to an end".into(),
                shape: Shape::Callout {
                    sites: vec![
                        (s.at(s.ends[0] + r, s.centre), r),
                        (s.at(s.ends[1] - r, s.centre), r),
                    ],
                },
            });
        }
        let d = s.ends[0] - lo[s.long];
        if d.abs() < 1e-6 {
            problems.push(Problem {
                id: format!("p:location_on_datum:{id}/{}", AXES[s.long]),
                code: "location_on_datum".into(),
                severity: "info".into(),
                message: format!(
                    "{id} starts at the part's {} face, so its location along {} measures nothing and is not dimensioned.",
                    ["left", "front", "bottom"][s.long],
                    AXES[s.long]
                ),
                subjects: vec![id.clone()],
            });
            continue;
        }
        let Shape::Linear { mut a, b, .. } = size(s, view, s.long, s.ends[0], s.ends[0], 1) else {
            unreachable!("a size is linear")
        };
        a[s.long] = lo[s.long];
        out.push(Want {
            id: want_id(id, "location_slot.length", view),
            views: vec![view],
            value: d,
            terms: vec![fmt(d)],
            measurements: vec![m("location_slot.length", d)],
            turning: None,
            basis: format!(
                "located along its length ({}) from the part's {} face to its near end, in the view that looks through it; its position across is not dimensioned, as in draftwright",
                AXES[s.long],
                ["left", "front", "bottom"][s.long]
            ),
            shape: Shape::Linear {
                a,
                b,
                axis: s.long,
                a_edge: true,
                b_edge: false,
                order: 1,
            },
        });
    }
    (out, problems)
}
