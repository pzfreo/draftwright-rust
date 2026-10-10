//! Machined flats (quiddity's `Flat`, draftwright's `FlatFeature`): one `52.6 A/F` callout per
//! across-flats definition (`render_flats`, `_flat_label` and `_flat_definition_count` in
//! `annotations/from_model.py`; the decimals from `_compile_groups` in `model/compiled.py`).
//!
//! - **Definitions:** flats on one stock region (the same axis, canonical direction, axis line,
//!   stock span and across size, as draftwright's `FlatFeature` canonicalises them) are one
//!   definition when every face has its reflection through the stock's axis (a double-D's two
//!   faces, a hex's six), and one per face otherwise.
//! - **Callouts:** definitions with the same presentation axis, direction and printed size share
//!   one callout, counted (`2× 17 A/F`) when there is more than one; the callout states every
//!   face's `flat.length`.
//! - **View:** the flats are seen edge-on looking down the stock: [`end_on`] the record's axis
//!   (draftwright's `presentation_axis`, which draws an exact x/z slant in the side view).
//! - **Decimals:** with one size the usual format; when the sheet's flats differ in size, the
//!   fewest decimals (up to two) that tell them apart, for every flat (draftwright's
//!   `flat_auto_decimals`, there over the sizes the spec keeps, here over every flat).
//! - **Leader site:** the faces' `at`.
//!
//! A flat whose record lacks its size, position or axis, or whose direction does not lie along
//! its named axis, is reported (`feature_not_dimensioned`), never dropped.

use std::collections::BTreeMap;

use serde_json::Value;

use super::grooves::{not_dimensioned, size};
use super::{AXES, Context, Measurement, P3, Planner, Problem, Shape, Want, end_on, fmt, point};
use crate::bridge::Feature;
use crate::bridge::holes::principal;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["flat"],
    wants,
};

/// A flat as draftwright's `FlatFeature` holds it, in quiddity's frame.
struct Flat<'a> {
    feature: &'a Feature,
    across: f64,
    /// The record's axis letter, and the one its view is chosen by.
    axis: usize,
    presentation: usize,
    /// The canonical unit direction (its named component positive), to 1e-6.
    direction: [i64; 3],
    /// The stock's axis line and span along the canonical direction, to 1e-3; none when the
    /// record does not give them, so the flat is a definition of its own.
    stock: Option<([i64; 2], [i64; 2])>,
    /// The face's `at`, in quiddity's frame.
    at: P3,
}

fn pair(v: Option<&Value>) -> Option<[f64; 2]> {
    match v?.as_array()?.as_slice() {
        [a, b] => Some([a.as_f64()?, b.as_f64()?]),
        _ => None,
    }
}

fn thousandths(x: f64) -> i64 {
    (x * 1e3).round() as i64
}

/// A flat's record, or why it cannot be dimensioned.
fn read(f: &Feature) -> Result<Flat<'_>, String> {
    let p = &f.parameters;
    let across = size(f, "across")?;
    let at = point(p.get("at")).ok_or("its record gives no at")?;
    let axis = p
        .get("axis")
        .and_then(Value::as_str)
        .and_then(|a| AXES.iter().position(|x| *x == a))
        .ok_or("its record gives no axis")?;
    let mut raw = [0.0; 3];
    raw[axis] = 1.0;
    if let Some(v) = p.get("axis_direction") {
        raw = point(Some(v)).ok_or("its record's axis_direction is not a vector")?;
    }
    let norm = raw.iter().map(|c| c * c).sum::<f64>().sqrt();
    let largest = raw.iter().fold(0.0f64, |m, c| m.max(c.abs()));
    if norm <= 1e-12 || raw[axis].abs() + norm * 1e-9 < largest {
        return Err(format!(
            "its axis_direction does not lie along its axis {}",
            AXES[axis]
        ));
    }
    let sign = if raw[axis] < 0.0 { -1.0 } else { 1.0 };
    let unit = raw.map(|c| sign * c / norm);
    let direction = unit.map(|c| (c * 1e6).round() as i64);
    // draftwright's `presentation_axis`: an exact x/z tie (recognition names it z) is drawn in
    // the x end-on view; the tie is judged on the direction rounded to six decimals, as
    // draftwright's canonical direction is.
    let [x, y, z] = direction.map(i64::abs);
    let aligned = (direction[axis] - 1_000_000).abs() <= 1000
        && (0..3).all(|j| j == axis || direction[j].abs() <= 1000);
    let presentation = if !aligned && x > y && z > y && x == z {
        0
    } else {
        axis
    };
    let stock = pair(p.get("axis_line"))
        .zip(pair(p.get("stock_span")))
        .map(|(line, [lo, hi])| {
            let span = if sign < 0.0 { [-hi, -lo] } else { [lo, hi] };
            (line.map(thousandths), span.map(thousandths))
        });
    Ok(Flat {
        feature: f,
        across,
        axis,
        presentation,
        direction,
        stock,
        at,
    })
}

/// How many definitions one stock region's faces make: one when every face has its reflection
/// through the stock's axis line (within 0.01 mm), else one per face.
fn definitions(members: &[&Flat]) -> usize {
    let first = members[0];
    let Some((line, _)) = first.stock else {
        return members.len();
    };
    let across: Vec<usize> = (0..3).filter(|&j| j != first.axis).collect();
    let centre = line.map(|c| c as f64 / 1e3);
    let reflected = |p: &Flat| {
        members.iter().any(|q| {
            (0..2).all(|i| (q.at[across[i]] - (2.0 * centre[i] - p.at[across[i]])).abs() <= 0.01)
        })
    };
    if members.len() > 1 && members.iter().all(|p| reflected(p)) {
        1
    } else {
        members.len()
    }
}

/// draftwright's flat decimals: none (the usual format) for one size; otherwise the fewest, up to
/// two, that tell the sizes (to 0.01) apart.
fn decimals(flats: &[Flat]) -> Option<usize> {
    let mut sizes: Vec<i64> = flats
        .iter()
        .map(|f| (f.across * 100.0).round() as i64)
        .collect();
    sizes.sort_unstable();
    sizes.dedup();
    if sizes.len() < 2 {
        None
    } else if sizes.iter().any(|s| s % 10 != 0) {
        Some(2)
    } else if sizes.iter().any(|s| s % 100 != 0) {
        Some(1)
    } else {
        None
    }
}

fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut flats = Vec::new();
    for f in cx.analysis.features.iter().filter(|f| f.kind == "flat") {
        match read(f) {
            Ok(flat) => flats.push(flat),
            Err(why) => problems.push(not_dimensioned(f, &why)),
        }
    }
    let places = decimals(&flats);
    let text = |a: f64| match places {
        Some(n) => format!("{a:.n$}"),
        None => fmt(a),
    };
    // Stock regions, then callouts of equal definitions, each in order of its first face.
    let mut regions: Vec<Vec<&Flat>> = Vec::new();
    for (i, f) in flats.iter().enumerate() {
        let same = |g: &Vec<&Flat>| {
            let h = g[0];
            f.stock.is_some()
                && (h.axis, h.presentation, h.direction, h.stock, text(h.across))
                    == (f.axis, f.presentation, f.direction, f.stock, text(f.across))
                && (h.across - f.across).abs() <= 1e-12
        };
        match regions.iter_mut().find(|g| same(g)) {
            Some(g) => g.push(&flats[i]),
            None => regions.push(vec![f]),
        }
    }
    type Batch = (usize, [i64; 3], i64, String);
    let mut order: Vec<Batch> = Vec::new();
    let mut batches: BTreeMap<Batch, Vec<Vec<&Flat>>> = BTreeMap::new();
    for g in regions {
        let k = (
            g[0].presentation,
            g[0].direction,
            (g[0].across * 1e12).round() as i64,
            text(g[0].across),
        );
        if !batches.contains_key(&k) {
            order.push(k.clone());
        }
        batches.entry(k).or_default().push(g);
    }

    let mut out = Vec::new();
    for k in order {
        let regions = &batches[&k];
        let members: Vec<&Flat> = regions.iter().flatten().copied().collect();
        let first = members[0];
        let Some(q) = cx
            .frame
            .letter(&Value::from(AXES[first.presentation]))
            .and_then(principal)
        else {
            for m in &members {
                problems.push(not_dimensioned(
                    m.feature,
                    "its stock axis is not along x, y or z, so no standard view looks down it",
                ));
            }
            continue;
        };
        let view = end_on(q);
        let n: usize = regions.iter().map(|g| definitions(g)).sum();
        let mut terms = Vec::new();
        if n > 1 {
            terms.push(format!("{n}×"));
        }
        terms.extend([text(first.across), "A/F".into()]);
        let faces = members.len();
        out.push(Want {
            id: super::want_id(&first.feature.id, "flat.length"),
            views: vec![view],
            value: first.across,
            terms,
            measurements: members
                .iter()
                .map(|m| Measurement {
                    feature: m.feature.id.clone(),
                    parameter: "flat.length".into(),
                    value: m.across,
                })
                .collect(),
            turning: None,
            basis: format!(
                "a flat's size across flats is called out where it is seen edge-on, looking down its stock ({}); {}",
                AXES[q],
                match (n, faces) {
                    (1, 1) => "one face".to_string(),
                    (1, f) => format!("its {f} faces reflect through the stock's axis, one definition"),
                    (n, f) => format!("{n} equal definitions ({f} faces) are called out once and counted"),
                }
            ),
            shape: Shape::Callout {
                sites: members.iter().map(|m| (cx.frame.point(m.at), 0.0)).collect(),
            },
        });
    }
    (out, problems)
}
