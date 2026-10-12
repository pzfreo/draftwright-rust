//! Turned parts: the overall diameter and length of a part turned whole, and each turned step's
//! own diameter and length (draftwright's `render_rotational`, `render_diameters` and
//! `render_step_lengths`, over the groups `model/compiled.py` approves).
//!
//! - **Bands** are the Analysis's `turned_step` features that no groove owns (lint's
//!   [`bands`]: draftwright's `groove_owns_turned_step_band`, the groove's callout states that
//!   band), each read in the part's frame. When groove ownership cannot be decided, or a record
//!   cannot be read, the step is reported (`feature_not_dimensioned`), never dropped or guessed.
//! - **The overall diameter** of a part turned whole is `envelope`/`od.diameter`, in a view that
//!   shows the profile. It also states the diameter of the band it is
//!   (`_share_unique_outer_diameter`): the one band of the largest diameter on the part's axis
//!   line, or two such bands with separate spans inside the box (`2× ø130`, two flanges). Three
//!   or more do not share it, and each gets its own diameter.
//! - **A step's diameter** is `step.diameter`, a callout `ø6` led to the band's silhouette at
//!   its middle, in the front view for a band along x or z (draftwright's row below and column
//!   left of the front view; the next profile view when the front is omitted), and led to its
//!   circle in the front view, which looks along it, for a band along y (draftwright's radial
//!   leaders). A band along z whose diameter prints as a coaxial hole's bore, counterbore or
//!   spotface does reads `ø70 OD` (`_external_diameter_rider`), and exactly two bands along z
//!   with the same diameter at different stations share one `2× ø6` callout
//!   (`_counted_column_items`). A band that owns an AP242 thread or knurl
//!   ([`crate::pmi_lowering::manufacturing`]) prints it after its diameter (`ø5 SEE MFG 3`), on a
//!   callout of its own, or on the overall diameter when that states the band alone (beyond
//!   draftwright, whose overall diameter prints none and whose lint then finds it not drawn).
//! - **A step's length** is `step.length`, the band's span along its axis, in its first profile
//!   view (front for x and z, side for y), its witnesses at the band's shoulders: a chain, one
//!   item per band, for each turned profile (the bands of one axis line and one recorded
//!   profile, `_profile_groups_from_plan`). Three or more bands of one profile within 10 % of
//!   their mean length are one `N× L` (`_draw_step_chain`'s collapse) when each prints as `L`
//!   (draftwright collapses them whatever they print, so its `3× 23.3` can stand for a 23 and a
//!   23.7); along y, each contiguous run of three or more equal lengths is one `N× L` instead
//!   (`_render_y_profile`). draftwright
//!   gives a turned step a datum-to-shoulder baseline ladder only from authored PMI
//!   (`add_authored_step_positions`), which the port does not read, so its steps are a chain.
//! - **The overall length** of a part turned whole is stated only when its profile's chain (its
//!   bands' and grooves' spans, adjoining) does not run end to end along the axis: a surviving
//!   chain conveys it (`_step_chain_covers_extent`; ISSUE-955-a's "a surviving chain does not
//!   release a redundant height").
//!
//! Not ported, as placement decisions the planners cannot see (they do not know the scale):
//! draftwright's dropping of the smallest diameters when its row or column overflows, its
//! stagger and dense-chain drop, its enlarged details for a crowded head of short bands, and its
//! moving of a second parallel profile to the other profile view when both would share a line.
//! Every item is planned, and placement reports what collides.

use std::collections::BTreeMap;

use serde_json::Value;

use super::envelope::{corner, profile_views};
use super::grooves::not_dimensioned;
use super::{AXES, Context, Measurement, P3, Planner, Problem, Shape, Want, fmt, point, want_id};
use crate::bridge::{Feature, holes};
use crate::lint::turned::{Band, band, bands};
use crate::model::Analysis;
use crate::pmi_lowering::Manufacturing;

pub(super) const PLANNER: Planner = Planner {
    kinds: &["turned_step"],
    wants,
};

/// A turned step and its band, read in the part's frame.
type Banded<'a> = (&'a Feature, Band);

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

/// Whether two points lie on one line along world axis *k* (within 1e-3 mm across it).
fn same_line(k: usize, a: P3, b: P3) -> bool {
    (0..3).all(|j| j == k || (a[j] - b[j]).abs() <= 1e-3)
}

/// The bands the overall diameter of part *t* also states: the one band of its diameter on its
/// axis line, or two with separate spans inside the box (`_share_unique_outer_diameter`, with
/// `_two_distinct_od_bands`); none when three or more share it.
fn shared<'a>(t: &Turned, read: &'a [Banded<'a>], envelope: [P3; 2]) -> Vec<&'a Banded<'a>> {
    let matches: Vec<&Banded> = read
        .iter()
        .filter(|(_, b)| b.axis == t.axis && same_line(t.axis, b.on, t.on))
        .filter(|(_, b)| (b.diameter - t.diameter).abs() <= 1e-6)
        .collect();
    let k = t.axis;
    let [lo, hi] = envelope;
    let two_apart = |a: &Band, b: &Band| {
        [a, b]
            .iter()
            .all(|c| lo[k] - 1e-6 <= c.lo && c.lo < c.hi && c.hi <= hi[k] + 1e-6)
            && (a.hi < b.lo - 1e-6 || b.hi < a.lo - 1e-6)
    };
    match matches[..] {
        [_] => matches,
        [a, b] if two_apart(&a.1, &b.1) => matches,
        _ => Vec::new(),
    }
}

/// Whether the bands and grooves of one turned profile adjoin from one end of the part's box to
/// the other along world axis *k* (draftwright's `_selected_chain_covers_extent`: shared stations
/// within 1e-3 mm, overlapping spans do not join).
fn chain_covers(profiles: &[Vec<(f64, f64)>], envelope: [P3; 2], k: usize) -> bool {
    let tol = 1e-3 + 1e-9;
    let [lo, hi] = envelope;
    profiles.iter().any(|spans| {
        let mut sorted = spans.clone();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let mut reachable = vec![lo[k]];
        for (a, b) in sorted {
            if reachable.iter().any(|s| (a - s).abs() <= tol) {
                reachable.push(b);
            }
        }
        reachable.iter().any(|s| (s - hi[k]).abs() <= tol)
    })
}

/// The grooves' spans along their axes, by the profile their record names, in the part's frame:
/// (profile, axis, a point on the axis, low end, high end).
fn groove_spans<'a>(cx: &'a Context) -> Vec<(Option<&'a Value>, usize, P3, f64, f64)> {
    let mut out = Vec::new();
    for g in cx.analysis.features.iter().filter(|f| f.kind == "groove") {
        let p = &g.parameters;
        let (Some(at), Some(axis), Some(width)) = (
            point(p.get("at")),
            p.get("axis").and_then(|a| cx.frame.letter(a)),
            p.get("width").and_then(Value::as_f64),
        ) else {
            continue;
        };
        let Some(k) = holes::principal(axis) else {
            continue;
        };
        let at = cx.frame.point(at);
        out.push((
            p.get("profile"),
            k,
            at,
            at[k] - 0.5 * width,
            at[k] + 0.5 * width,
        ));
    }
    out
}

/// The diameters a hole coaxial with a band along *k* through *on* prints (its bore, counterbore
/// and spotface), as `fmt` prints them.
fn coaxial_bores(analysis: &Analysis, k: usize, on: P3) -> Vec<String> {
    let mut out = Vec::new();
    for (_, h) in holes::read(analysis) {
        let Ok(h) = h else { continue };
        if h.principal.map(|(a, _)| a) != Some(k) || !same_line(k, h.at, on) {
            continue;
        }
        out.push(fmt(h.diameter));
        for r in [h.counterbore, h.spotface].into_iter().flatten() {
            out.push(fmt(r.diameter));
        }
    }
    out
}

/// The leader sites of a band's diameter: its silhouette at its middle, across each radial axis
/// (in a profile view); or, along y, its circle (in the front view, which looks along it).
fn sites(b: &Band) -> Vec<(P3, f64)> {
    let mut mid = b.on;
    mid[b.axis] = 0.5 * (b.lo + b.hi);
    let r = 0.5 * b.diameter;
    if b.axis == 1 {
        return vec![(mid, r)];
    }
    (0..3)
        .filter(|&j| j != b.axis)
        .flat_map(|j| {
            [-r, r].map(|d| {
                let mut s = mid;
                s[j] += d;
                (s, 0.0)
            })
        })
        .collect()
}

/// The views a band's diameter reads in, preferred first.
fn diameter_views(k: usize) -> Vec<&'static str> {
    if k == 1 {
        vec![super::end_on(1)]
    } else {
        profile_views(k)
    }
}

/// The steps' diameters, those the overall diameter does not state.
fn diameters(
    cx: &Context,
    read: &[Banded],
    carried: &[&str],
    manufacturing: &Manufacturing,
) -> Vec<Want> {
    let mut items: Vec<(Vec<&Banded>, Vec<String>)> = Vec::new();
    for fb in read {
        let (f, b) = fb;
        if carried.contains(&f.id.as_str()) {
            continue;
        }
        let text = fmt(b.diameter);
        let mut terms = vec![format!("ø{text}")];
        // A thread or knurl the band carries follows its diameter (`_manufacturing_suffix`),
        // before the `OD` rider (`_external_diameter_rider`).
        terms.extend(manufacturing.terms(&f.id).0);
        if b.axis == 2 && coaxial_bores(cx.analysis, b.axis, b.on).contains(&text) {
            terms.push("OD".into());
        }
        items.push((vec![fb], terms));
    }
    // Along z, exactly two bands of one printed diameter at different stations share a callout.
    let mut by_text: BTreeMap<(i64, Vec<String>), Vec<usize>> = BTreeMap::new();
    for (i, (members, terms)) in items.iter().enumerate() {
        let (f, b) = members[0];
        // A band carrying a thread or knurl keeps its own callout (one annotation per owner).
        if b.axis == 2 && manufacturing.on(&f.id).is_empty() {
            by_text
                .entry((super::key(b.diameter), terms.clone()))
                .or_default()
                .push(i);
        }
    }
    let mut gone = Vec::new();
    for pair in by_text.values() {
        if let [i, j] = pair[..] {
            let (a, b) = (&items[i].0[0].1, &items[j].0[0].1);
            if (a.lo + a.hi - b.lo - b.hi).abs() > 1e-9 {
                let second = items[j].0[0];
                items[i].0.push(second);
                items[i].1.insert(0, "2×".into());
                gone.push(j);
            }
        }
    }
    items
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !gone.contains(i))
        .map(|(_, (members, terms))| {
            let (f, b) = members[0];
            let views = diameter_views(b.axis);
            let n = members.len();
            Want {
                id: want_id(&f.id, "step.diameter"),
                views,
                value: b.diameter,
                measurements: members
                    .iter()
                    .map(|(g, c)| Measurement {
                        feature: g.id.clone(),
                        parameter: "step.diameter".into(),
                        value: c.diameter,
                    })
                    .chain(claims(manufacturing, &members, b.diameter))
                    .collect(),
                basis: format!("{}{}", if n == 2 {
                    format!(
                        "two turned steps about {} have one diameter at different stations, so one callout states both (draftwright's counted column), in a view that shows the profile",
                        AXES[b.axis]
                    )
                } else if b.axis == 1 {
                    "a turned step's diameter, led to its circle in the front view, which looks along y (draftwright's radial leaders)".into()
                } else {
                    format!(
                        "a turned step's diameter, led to its silhouette in a view that shows the profile about {}{}",
                        AXES[b.axis],
                        if terms.last().is_some_and(|t| t == "OD") {
                            "; OD, as a coaxial bore prints the same size"
                        } else {
                            ""
                        }
                    )
                }, carries(manufacturing, &members)),
                terms,
                shape: Shape::Callout {
                    sites: members.iter().flat_map(|(_, c)| sites(c)).collect(),
                },
                turning: None,
            }
        })
        .collect()
}

/// One step-length item: from *first* (its ID's band) over *members*' spans, printing *terms*.
fn length(first: &Feature, members: &[&Banded], terms: Vec<String>, basis: String) -> Want {
    let b0 = &members[0].1;
    let k = b0.axis;
    let lo = members
        .iter()
        .map(|(_, b)| b.lo)
        .fold(f64::INFINITY, f64::min);
    let hi = members
        .iter()
        .map(|(_, b)| b.hi)
        .fold(f64::NEG_INFINITY, f64::max);
    let views = profile_views(k);
    Want {
        id: want_id(&first.id, "step.length"),
        views,
        value: if members.len() == 1 {
            hi - lo
        } else {
            mean(members)
        },
        terms,
        measurements: members
            .iter()
            .map(|(g, b)| Measurement {
                feature: g.id.clone(),
                parameter: "step.length".into(),
                value: b.hi - b.lo,
            })
            .collect(),
        basis,
        shape: Shape::Linear {
            a: corner(k, lo, b0.on),
            b: corner(k, hi, b0.on),
            axis: k,
            a_edge: true,
            b_edge: true,
            order: 1,
        },
        turning: None,
    }
}

/// The claims a diameter of *members*, *value* across, makes on the AP242 threads and knurls
/// they carry: one measurement of feature `pmi:<source>` each.
fn claims<'a>(
    manufacturing: &'a Manufacturing,
    members: &'a [&Banded],
    value: f64,
) -> impl Iterator<Item = Measurement> + 'a {
    members.iter().flat_map(move |(f, _)| {
        manufacturing
            .on(&f.id)
            .into_iter()
            .map(move |o| Measurement {
                feature: o.id.clone(),
                parameter: o.aspect.into(),
                value,
            })
    })
}

/// The clause a diameter's basis gives the threads and knurls *members* carry, or none.
fn carries(manufacturing: &Manufacturing, members: &[&Banded]) -> String {
    let sources: Vec<&str> = members
        .iter()
        .flat_map(|(f, _)| manufacturing.on(&f.id))
        .map(|o| o.source.as_str())
        .collect();
    if sources.is_empty() {
        return String::new();
    }
    format!(
        "; it carries the part's AP242 thread or knurl ({}), {}, as draftwright gives one to the turned step whose cylinder it names (lower_ap242_manufacturing_requirements)",
        sources.join(", "),
        if manufacturing.tags.is_empty() {
            "printed after its diameter"
        } else {
            "referred to the manufacturing requirements table"
        }
    )
}

fn mean(members: &[&Banded]) -> f64 {
    members.iter().map(|(_, b)| b.hi - b.lo).sum::<f64>() / members.len() as f64
}

/// One profile's step lengths, its bands in order along the axis.
fn chain(bands: &[&Banded]) -> Vec<Want> {
    let k = bands[0].1.axis;
    let one = |m: &Banded| {
        length(
            m.0,
            &[m],
            vec![fmt(m.1.hi - m.1.lo)],
            format!(
                "a turned step's length along {}, one link of its profile's chain, in a view that shows the profile",
                AXES[k]
            ),
        )
    };
    let repeated = |run: &[&Banded], value: f64, why: &str| {
        length(
            run[0].0,
            run,
            vec![format!("{}×", run.len()), fmt(value)],
            format!(
                "{} turned steps along {} {why}, so one length states them all ({}× {}), in a view that shows the profile",
                run.len(),
                AXES[k],
                run.len(),
                fmt(value)
            ),
        )
    };
    if k == 1 {
        // Along y: each contiguous run of three or more equal lengths is one `N× L`.
        let mut out = Vec::new();
        let mut collapsed = false;
        let mut i = 0;
        while i < bands.len() {
            let mut j = i + 1;
            while j < bands.len()
                && (bands[j - 1].1.hi - bands[j].1.lo).abs() <= 1e-4
                && fmt(bands[j].1.hi - bands[j].1.lo) == fmt(bands[i].1.hi - bands[i].1.lo)
            {
                j += 1;
            }
            if j - i >= 3 {
                let run = &bands[i..j];
                out.push(repeated(run, mean(run), "adjoin with one length"));
                collapsed = true;
            } else {
                out.extend(bands[i..j].iter().map(|m| one(m)));
            }
            i = j;
        }
        if collapsed {
            return out;
        }
    }
    let lengths: Vec<f64> = bands.iter().map(|(_, b)| b.hi - b.lo).collect();
    let m = mean(bands);
    let spread = lengths.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - lengths.iter().copied().fold(f64::INFINITY, f64::min);
    // draftwright collapses lengths within 10 % of their mean whatever they print; the port also
    // asks that each prints as the one length stated, so `3× 23.3` never stands for a 23 or a 23.7.
    if bands.len() >= 3 && spread <= 0.10 * m && lengths.iter().all(|l| fmt(*l) == fmt(m)) {
        return vec![repeated(bands, m, "have one length")];
    }
    bands.iter().map(|m| one(m)).collect()
}

/// A turned part's dimensions: the overall diameter and length of a part turned whole, and each
/// band's diameter and length.
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let steps: Vec<&Feature> = cx
        .analysis
        .features
        .iter()
        .filter(|f| f.kind == "turned_step")
        .collect();
    let mut read: Vec<Banded> = Vec::new();
    match bands(cx.analysis) {
        Ok(own) => {
            for f in own {
                match band(&cx.frame.0, f) {
                    Ok(b) => read.push((f, b)),
                    Err(why) => problems.push(not_dimensioned(f, &why)),
                }
            }
        }
        Err(why) => {
            for f in &steps {
                problems.push(not_dimensioned(
                    f,
                    &format!("groove ownership of its band cannot be decided ({why})"),
                ));
            }
        }
    }
    // The bands of each turned profile, in order along the axis: one axis line, one record.
    let mut profiles: Vec<Vec<&Banded>> = Vec::new();
    for fb in &read {
        let (f, b) = fb;
        let key = f.parameters.get("profile");
        match profiles.iter_mut().find(|p| {
            let (g, c) = p[0];
            c.axis == b.axis && same_line(b.axis, c.on, b.on) && g.parameters.get("profile") == key
        }) {
            Some(p) => p.push(fb),
            None => profiles.push(vec![fb]),
        }
    }
    for p in &mut profiles {
        p.sort_by(|x, y| x.1.lo.total_cmp(&y.1.lo).then(x.1.hi.total_cmp(&y.1.hi)));
    }

    let manufacturing = crate::pmi_lowering::manufacturing(cx.analysis);
    let mut carried: Vec<&str> = Vec::new();
    if let Some(t) = &cx.turned {
        let mut share = shared(t, &read, cx.analysis.envelope);
        // A band that carries a thread or knurl keeps a diameter of its own when another band
        // shares the overall diameter (ISSUE-1298-a: one annotation per owner).
        if share.len() > 1
            && share
                .iter()
                .any(|(f, _)| !manufacturing.on(&f.id).is_empty())
        {
            share.clear();
        }
        carried = share.iter().map(|(f, _)| f.id.as_str()).collect();
        let grooves = groove_spans(cx);
        let spans: Vec<Vec<(f64, f64)>> = profiles
            .iter()
            .filter(|p| p[0].1.axis == t.axis)
            .map(|p| {
                let (f, b) = p[0];
                let key = f.parameters.get("profile");
                p.iter()
                    .map(|(_, c)| (c.lo, c.hi))
                    .chain(
                        grooves
                            .iter()
                            .filter(|(g, k, at, ..)| {
                                *k == b.axis && *g == key && same_line(b.axis, *at, b.on)
                            })
                            .map(|(.., lo, hi)| (*lo, *hi)),
                    )
                    .collect()
            })
            .collect();
        let covered = chain_covers(&spans, cx.analysis.envelope, t.axis);
        out.extend(overall(cx, t, &share, covered, &manufacturing));
    }
    out.extend(diameters(cx, &read, &carried, &manufacturing));
    for p in &profiles {
        let [lo, hi] = cx.analysis.envelope;
        let k = p[0].1.axis;
        let full = cx.turned.as_ref().is_some_and(|t| t.axis == k)
            && (p[0].1.lo - lo[k]).abs() <= 1e-3
            && (p[p.len() - 1].1.hi - hi[k]).abs() <= 1e-3
            && p.windows(2).all(|w| (w[0].1.hi - w[1].1.lo).abs() <= 1e-3);
        if full {
            // Keep individual links so exactly one, rather than an entire counted
            // chain, can become reference beside the controlling overall length.
            for b in p {
                out.extend(chain(std::slice::from_ref(b)));
            }
        } else {
            out.extend(chain(p));
        }
    }
    (out, problems)
}

/// The overall diameter and, unless the step chain conveys it, the overall length of part *t*,
/// turned whole. The diameter also states the bands in *share*.
fn overall(
    cx: &Context,
    t: &Turned,
    share: &[&Banded],
    covered: bool,
    manufacturing: &Manufacturing,
) -> Vec<Want> {
    let [lo, hi] = cx.analysis.envelope;
    let mut out = Vec::new();
    let views = profile_views(t.axis);
    // Across the axis, in the first of its profile views: the radial axis that view shows.
    let radial = (0..3)
        .find(|&j| j != t.axis && profile_views(j).contains(&views[0]))
        .unwrap_or((t.axis + 1) % 3);
    let mut terms = vec![format!("ø{}", fmt(t.diameter))];
    if share.len() == 2 {
        terms.insert(0, "2×".into());
    }
    // A thread or knurl the bands it states carry follows the diameter (beyond draftwright,
    // whose overall diameter prints none and whose lint then finds the requirement not drawn).
    if let Some((f, _)) = share.first() {
        terms.extend(manufacturing.terms(&f.id).0);
    }
    let mut measurements = vec![Measurement {
        feature: "envelope".into(),
        parameter: "od.diameter".into(),
        value: t.diameter,
    }];
    measurements.extend(share.iter().map(|(f, b)| Measurement {
        feature: f.id.clone(),
        parameter: "step.diameter".into(),
        value: b.diameter,
    }));
    measurements.extend(claims(manufacturing, share, t.diameter));
    let stands = match share.len() {
        1 => "; it is also the diameter of the one band that size",
        2 => "; it is also the diameter of the two separate bands that size",
        _ => "",
    };
    out.push(Want {
        id: want_id("envelope", "od.diameter"),
        views: views.clone(),
        value: t.diameter,
        terms,
        measurements,
        turning: Some(t.axis),
        basis: format!(
            "the part is turned whole about {}: its largest diameter is its overall size across the axis, in a view that shows the profile{stands}{}",
            AXES[t.axis],
            carries(manufacturing, share)
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
        id: want_id("envelope", &format!("{name}.length")),
        views,
        value: hi[k] - lo[k],
        terms: vec![fmt(hi[k] - lo[k])],
        measurements: vec![Measurement {
            feature: "envelope".into(),
            parameter: format!("{name}.length"),
            value: hi[k] - lo[k],
        }],
        turning: None,
        basis: if covered {"the overall length along the turning axis; the chain reaches both ends, so one member is reference"} else {"the overall length along the turning axis: the step chain does not run end to end"}.into(),
        shape: Shape::Linear {
            a: corner(k, lo[k], lo),
            b: corner(k, hi[k], lo),
            axis: k,
            a_edge: true,
            b_edge: true,
            order: 2,
        },
    });
    out
}
