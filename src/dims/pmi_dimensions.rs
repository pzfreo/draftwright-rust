//! The part's AP242 dimensions (sizes and locations, toleranced or basic) on the sheet: stage 2b
//! of drawing PMI, draftwright's `model/pmi_lowering.py` (`lower_ap242_hole_tolerances`,
//! `lower_ap242_nominal_diameters`, `lower_ap242_nominal_step_lengths`) and
//! `annotations/_pmi_dimensions.py` (`render_pmi`). Each dimension record of the Analysis's PMI,
//! with where its faces put it ([`crate::pmi_lowering::dimension_geometry`]), either:
//!
//! - **annotates a model dimension**, which then states it (draftwright's rule: a source
//!   requirement on a quantity a recognised feature's dimension already states is given to that
//!   dimension, so the quantity is stated once): a bore diameter whose cylinders are the holes a
//!   callout calls out (their axis lines, their axis and their diameter) gives the callout its
//!   tolerance (`6× ⌀10 +0.09 -0.0 THRU`; a limit pair as its deviations, `⌀35 ±0.2`), or, with
//!   no tolerance, is stated by it; a plain length between a turned step's two shoulders is
//!   stated by the step's length;
//! - **stands beside them**, drawn as its own dimension (draftwright's standalone fallback): a
//!   length along x, y or z between its two proven stations, in the front view (x and z; the plan
//!   when only the plan's witness is proven), or the side then the plan (y); a diameter by a
//!   leader to its cylinder where it is seen end-on; a radius by a leader to a proven point of
//!   its arc. Its label is the record's (`⌀62 +0.03/0`, `30`); a basic dimension's is boxed. It
//!   is an ordinary planned dimension, placed with the others;
//! - **or is not drawn, and says why**: a record whose reading left blockers, whose faces lie in
//!   several placements of the part, or whose faces prove no witness (`pmi_not_lowered`, with
//!   draftwright's reasons); a kind the port does not draw yet: an angle, a length oblique to
//!   the views' axes, a diameter or radius not along x, y or z, a callout whose holes carry
//!   different requirements (draftwright splits it) (`pmi_dimensions_not_ported`).
//!
//! Two open questions for the maintainer stay refused, as the reading refuses them: a dimension
//! on supplemental (invisible) geometry, and an ISO 286 class stated without its limits.

use std::collections::{BTreeMap, BTreeSet};

use super::turned::Turned;
use super::{Context, Measurement, P3, Problem, Shape, Want, end_on, fmt, tolerance, want_id};
use crate::model::Spec;
use crate::pmi::{Category, Reading, Record};
use crate::pmi_lowering::Cylinder;

/// What the plan keeps of a model item's PMI tolerance, by its ID's stem (the ID before `@`,
/// which routing does not change): the band the PMI put on its label, and the sources it came
/// from, so a spec's own tolerance can take its place. (Every item that states a PMI dimension
/// claims it: a measurement of feature `pmi:<source>`.)
#[derive(Clone, Debug, Default)]
pub(super) struct Note {
    /// The band as printed.
    pub band: String,
    /// The item's terms before the band.
    pub before: Vec<String>,
    /// The sources whose tolerance it is.
    pub banded: Vec<String>,
}

/// A record's tolerance: its signed upper and lower deviations, and its limits when it states
/// them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Req {
    upper: f64,
    lower: f64,
    limits: Option<(f64, f64)>,
}

/// A requirement as a comparable key (its numbers' bits).
type Key = (u64, u64, u64, u64);

impl Req {
    fn key(&self) -> Key {
        let (lo, hi) = self
            .limits
            .map_or((u64::MAX, u64::MAX), |(l, h)| (l.to_bits(), h.to_bits()));
        (self.upper.to_bits(), self.lower.to_bits(), lo, hi)
    }

    /// Puts it on *w*'s label: a symmetric band as `±T`, an asymmetric limit pair as the limits
    /// in place of the value, any other as `+U -L` (draftwright's `bore_callout_value` and
    /// `_fmt_tolerance`). The note to undo it, or none when the label states no value.
    fn print(&self, w: &mut Want) -> Option<Note> {
        let before = w.terms.clone();
        let symmetric = self.upper > 0.0 && self.upper == -self.lower;
        let (ok, band) = match self.limits {
            Some((lo, hi)) if !symmetric => (tolerance::limits(w, lo, hi), format!("{lo} - {hi}")),
            _ => {
                let text = tolerance::band(self.upper, self.lower);
                (tolerance::insert(w, &text), text)
            }
        };
        ok.then_some(Note {
            band,
            before,
            banded: Vec::new(),
        })
    }
}

/// The feature a PMI dimension's claim names.
pub(super) fn feature(source: &str) -> String {
    format!("pmi:{source}")
}

/// Whether a planned item is a basic PMI dimension: its label is then boxed.
pub(super) fn basic(measurements: &[Measurement]) -> bool {
    measurements
        .iter()
        .any(|m| m.feature.starts_with("pmi:") && m.parameter.ends_with(".basic"))
}

/// A claim on a PMI dimension: its feature, its kind as the parameter (`.basic` for a basic
/// one), its value.
fn claim(r: &Record, value: f64) -> Measurement {
    Measurement {
        feature: feature(&r.source_id),
        parameter: if r.basic {
            format!("{}.basic", r.kind)
        } else {
            r.kind.clone()
        },
        value,
    }
}

/// What [`lower`] returns: the standalone items, the problems, and the notes.
pub(super) type Lowered = (Vec<Want>, Vec<Problem>, BTreeMap<String, Note>);

/// The stem of a dimension ID: `d:<feature>/<parameter>`.
pub(super) fn stem(id: &str) -> &str {
    id.rsplit_once('@').map_or(id, |(s, _)| s)
}

/// Why a record is not drawn: its code and its reason.
struct Refusal(&'static str, String);

fn not_lowered(why: impl Into<String>) -> Refusal {
    Refusal("pmi_not_lowered", why.into())
}

fn not_ported(why: impl Into<String>) -> Refusal {
    Refusal("pmi_dimensions_not_ported", why.into())
}

/// A record's tolerance: its upper and lower deviations, signed, from its deviations or from its
/// limits less its nominal (in decimal, as draftwright's `_requirement` computes them), or none
/// when it states no tolerance.
fn requirement(r: &Record, value: f64) -> Option<Req> {
    match (r.lower_bound, r.upper_bound, r.lower_tol, r.upper_tol) {
        (Some(lo), Some(hi), _, _) => Some(Req {
            upper: decimal_sub(hi, value),
            lower: decimal_sub(lo, value),
            limits: Some((lo, hi)),
        }),
        (_, _, lo, hi) if lo.is_some() || hi.is_some() => Some(Req {
            upper: hi.unwrap_or(0.0),
            lower: lo.unwrap_or(0.0),
            limits: None,
        }),
        _ => None,
    }
}

/// *a* − *b* in decimal, as Python's `Decimal(str(a)) - Decimal(str(b))`: each from its
/// shortest round-trip digits, so 35 − 34.8 is 0.2, not 0.20000000000000284.
fn decimal_sub(a: f64, b: f64) -> f64 {
    let parse = |v: f64| -> Option<(i128, u32)> {
        let s = format!("{v}");
        let (neg, s) = s
            .strip_prefix('-')
            .map_or((false, s.as_str()), |t| (true, t));
        let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
        let digits: i128 = format!("{whole}{frac}").parse().ok()?;
        Some((if neg { -digits } else { digits }, frac.len() as u32))
    };
    match (parse(a), parse(b)) {
        (Some((x, p)), Some((y, q))) if p.max(q) <= 18 => {
            let scale = p.max(q);
            let d = x * 10i128.pow(scale - p) - y * 10i128.pow(scale - q);
            format!("{}e-{scale}", d).parse().unwrap_or(a - b)
        }
        _ => a - b,
    }
}

/// The record's label as callout terms: its words, a leading `ø` as the port's `⌀`.
fn terms(label: &str) -> Vec<String> {
    label
        .split_whitespace()
        .map(|t| match t.strip_prefix('ø') {
            Some(rest) => format!("⌀{rest}"),
            None => t.to_string(),
        })
        .collect()
}

/// The world axis an `X`/`Y`/`Z` letter names.
fn axis(letter: Option<&str>) -> Option<usize> {
    ["X", "Y", "Z"].iter().position(|x| Some(*x) == letter)
}

/// A callout's holes, each its feature, centre and diameter: the bore diameters it states, with
/// its sites (the planner keeps them in the same order).
fn holes_of(w: &Want) -> Vec<(String, P3, f64)> {
    let Shape::Callout { sites } = &w.shape else {
        return Vec::new();
    };
    let bores: Vec<&Measurement> = w
        .measurements
        .iter()
        .filter(|m| m.parameter == "bore.diameter")
        .collect();
    if bores.len() != sites.len() {
        return Vec::new();
    }
    bores
        .iter()
        .zip(sites)
        .map(|(m, (c, _))| (m.feature.clone(), *c, m.value))
        .collect()
}

/// The callout (by index into *wants*) whose holes the record's cylinders are, and which of its
/// holes: a hole along the record's axis whose diameter is its nominal (to 1e-4 mm) and whose
/// centre lies on one of its cylinders' axis lines (to 0.01 mm across the axis), draftwright's
/// `_internal_member_matches` and `_hole_tolerance_proposal`. `None` when no callout has such a
/// hole, or when the holes are in several callouts (draftwright's ambiguous correlation): the
/// record then stands beside them.
fn hole_callout(
    wants: &[Want],
    cylinders: &[Cylinder],
    k: usize,
    value: f64,
) -> Option<(usize, BTreeSet<usize>)> {
    let mut found: Vec<(usize, BTreeSet<usize>)> = Vec::new();
    for (i, w) in wants.iter().enumerate() {
        if stem(&w.id).rsplit_once('/').map(|(_, p)| p) != Some("bore.diameter")
            || w.views.first() != Some(&end_on(k))
        {
            continue;
        }
        let members: BTreeSet<usize> = holes_of(w)
            .iter()
            .enumerate()
            .filter(|(_, (_, c, d))| {
                (d - value).abs() <= 1e-4
                    && cylinders.iter().any(|cy| {
                        cy.principal() == Some(k)
                            && (0..3).all(|j| j == k || (cy.origin[j] - c[j]).abs() <= 0.01)
                    })
            })
            .map(|(m, _)| m)
            .collect();
        if !members.is_empty() {
            found.push((i, members));
        }
    }
    (found.len() == 1).then(|| found.remove(0))
}

/// The turned step length (by index into *wants*) a plain length states: one whose two ends lie
/// at the record's stations along its axis (to 0.01 mm) and whose value is its nominal
/// (draftwright's `lower_ap242_nominal_step_lengths`, which needs exactly one such step).
fn step_length(wants: &[Want], stations: &[P3], k: usize, value: f64) -> Option<usize> {
    let found: Vec<usize> = wants
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            let Shape::Linear { a, b, axis, .. } = &w.shape else {
                return false;
            };
            let ends = |p: f64, q: f64| {
                ((p - stations[0][k]).abs() <= 0.01 && (q - stations[1][k]).abs() <= 0.01)
                    || ((q - stations[0][k]).abs() <= 0.01 && (p - stations[1][k]).abs() <= 0.01)
            };
            *axis == k
                && stem(&w.id).ends_with("/step.length")
                && (w.value - value).abs() <= 1e-6
                && ends(a[k], b[k])
        })
        .map(|(i, _)| i)
        .collect();
    (found.len() == 1).then(|| found[0])
}

/// Whether a plain diameter's label is the size as a callout prints it: `ø6`, or `ø6 THRU`
/// (draftwright's `_canonical_nominal_diameter_label`).
fn canonical(r: &Record, value: f64) -> bool {
    let size = format!("ø{}", fmt(value));
    r.label == size || r.label == format!("{size} THRU")
}

/// Whether a site of a planned item lies on cylinder *c*: a circle (centre, radius) on its axis
/// with its radius, or a point (radius 0) on its surface within its extent, to 0.01 mm.
fn on_cylinder(site: &(P3, f64), c: &Cylinder) -> bool {
    let (p, r) = site;
    let d: P3 = [0, 1, 2].map(|i| p[i] - c.origin[i]);
    let t = d[0] * c.direction[0] + d[1] * c.direction[1] + d[2] * c.direction[2];
    let off = [0, 1, 2].map(|i| d[i] - t * c.direction[i]);
    let across = (off[0] * off[0] + off[1] * off[1] + off[2] * off[2]).sqrt();
    if *r > 0.0 {
        across <= 0.01 && (r - c.radius).abs() <= 0.01
    } else {
        (across - c.radius).abs() <= 0.01 && c.interval[0] - 0.01 <= t && t <= c.interval[1] + 0.01
    }
}

/// The turned step's or boss's diameter (by index into *wants*) whose sites all lie on the
/// record's external cylinders and whose diameter is its nominal (draftwright's
/// `_external_owner_matches`: the cylinder's axis line, radius and extent); `None` when no item
/// is, or several are (draftwright's ambiguous ownership: the record stands beside them). An
/// item only some of whose sites the cylinders name states a size the record does not cover
/// alone: refused.
fn external_owner(
    wants: &[Want],
    cylinders: &[Cylinder],
    value: f64,
    turned: Option<&Turned>,
) -> Option<Result<usize, Refusal>> {
    let mut whole = Vec::new();
    let mut partly = Vec::new();
    for (i, w) in wants.iter().enumerate() {
        let parameter = stem(&w.id).rsplit_once('/').map(|(_, p)| p);
        // A part turned whole: its overall diameter, about the turning axis.
        if parameter == Some("od.diameter")
            && let (Some(t), Some(axis)) = (turned, w.turning)
            && (w.value - value).abs() <= 0.01
            && cylinders.iter().all(|c| {
                c.principal() == Some(axis)
                    && (0..3).all(|j| j == axis || (c.origin[j] - t.on[j]).abs() <= 0.01)
            })
        {
            whole.push(i);
            continue;
        }
        if !matches!(parameter, Some("step.diameter" | "boss.diameter"))
            || (w.value - value).abs() > 0.01
        {
            continue;
        }
        let Shape::Callout { sites } = &w.shape else {
            continue;
        };
        let named = sites
            .iter()
            .filter(|s| cylinders.iter().any(|c| on_cylinder(s, c)))
            .count();
        if named == sites.len() && named > 0 {
            whole.push(i);
        } else if named > 0 {
            partly.push(i);
        }
    }
    match (whole.len(), partly.len()) {
        (1, 0) => Some(Ok(whole[0])),
        (0, 1) => Some(Err(not_ported(format!(
            "its cylinder is one of those {} states together; draftwright gives it to its own step, which the port does not split yet",
            wants[partly[0]].id
        )))),
        _ => None,
    }
}

/// A callout's holes of one requirement: its band's bits (upper, lower), and the holes.
type Group = (Option<Key>, Vec<usize>);

/// What one record becomes.
enum Outcome {
    Refused(Refusal),
    /// Drawn on its own.
    Standalone(Want),
    /// Stated by a callout: its holes, and the band it puts on it.
    Hole(usize, BTreeSet<usize>, Option<Req>),
    /// Stated by another item as it is.
    Stated(usize, &'static str),
    /// Stated by a turned step's or boss's diameter, which takes its band.
    Band(usize, Req),
}

/// The Analysis's PMI dimensions: their standalone items, the problems, and the notes for the
/// items they are on. *wants* are the planners' items; a callout or step that states a record
/// takes its band and its source here.
pub(super) fn lower(cx: &Context, wants: &mut [Want]) -> Lowered {
    let mut standalone = Vec::new();
    let mut problems = Vec::new();
    let mut notes: BTreeMap<String, Note> = BTreeMap::new();
    let Some(Reading::Read(report)) = &cx.analysis.pmi else {
        return (standalone, problems, notes);
    };
    let records: Vec<&Record> = report
        .records
        .iter()
        .filter(|r| r.category == Category::Dimension)
        .collect();
    let mut outcomes: Vec<Outcome> = records
        .iter()
        .map(|r| outcome(r, wants, cx.turned.as_ref()))
        .collect();

    // A callout states a band only when every hole it calls out has the same one: one with
    // holes of different requirements is split by requirement when it is a plain count group,
    // as draftwright splits it (`lower_ap242_hole_tolerances`); a pattern's is not split.
    let mut by_callout: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (j, o) in outcomes.iter().enumerate() {
        if let Outcome::Hole(i, ..) = o {
            by_callout.entry(*i).or_default().push(j);
        }
    }
    let mut split_off: Vec<Want> = Vec::new();
    for (i, js) in by_callout {
        let n = holes_of(&wants[i]).len();
        let id = wants[i].id.clone();
        // Each hole's band, from the records that name it.
        let mut bands: Vec<BTreeSet<Key>> = vec![BTreeSet::new(); n];
        let mut reqs: BTreeMap<Key, Req> = BTreeMap::new();
        for &j in &js {
            if let Outcome::Hole(_, members, Some(q)) = &outcomes[j] {
                reqs.insert(q.key(), *q);
                for &m in members {
                    bands[m].insert(q.key());
                }
            }
        }
        let toleranced: Vec<usize> = js
            .iter()
            .copied()
            .filter(|&j| matches!(outcomes[j], Outcome::Hole(_, _, Some(_))))
            .collect();
        if bands.iter().any(|b| b.len() > 1) {
            for &j in &toleranced {
                outcomes[j] = Outcome::Refused(not_lowered(format!(
                    "ambiguous hole tolerance ownership: a hole {id} calls out has conflicting AP242 requirements"
                )));
            }
            bands.iter_mut().for_each(BTreeSet::clear);
        }
        let key = |m: usize| bands[m].first().copied();
        let keys: Vec<Option<Key>> = (0..n).map(key).collect();
        let uniform = keys.iter().all(|k| *k == keys[0]);
        // A plain count group, or a grid or line of holes (whose pitches still locate the
        // holes once its callout is split); a bolt circle's callout states equal spacing of all
        // its holes, so it is not split.
        let plain = wants[i].terms.first().is_some_and(|t| t.ends_with('×'))
            && !wants[i].terms.iter().any(|t| t.starts_with("EQ SP"));
        // The groups: each its key and its holes, in the callout's order.
        let mut groups: Vec<Group> = Vec::new();
        if uniform || !plain {
            groups.push((if uniform { keys[0] } else { None }, (0..n).collect()));
            if !uniform && !toleranced.is_empty() {
                let covered = keys.iter().filter(|k| k.is_some()).count();
                let distinct: BTreeSet<_> = keys.iter().flatten().collect();
                let why = if distinct.len() > 1 {
                    format!(
                        "the {n} holes of the bolt circle {id} calls out carry different AP242 tolerances; draftwright gives each its own, which the port does not yet for a bolt circle"
                    )
                } else {
                    format!(
                        "its tolerance covers {covered} of the {n} holes of the bolt circle {id} calls out; draftwright gives them their own, which the port does not yet for a bolt circle"
                    )
                };
                for &j in &toleranced {
                    if matches!(outcomes[j], Outcome::Hole(..)) {
                        outcomes[j] = Outcome::Refused(not_ported(why.clone()));
                    }
                }
            }
        } else {
            for (m, k) in keys.iter().enumerate() {
                match groups.iter_mut().find(|(g, _)| g == k) {
                    Some((_, ms)) => ms.push(m),
                    None => groups.push((*k, vec![m])),
                }
            }
        }
        let parts = split(&wants[i], &groups);
        for (g, (mut w, members)) in parts.into_iter().enumerate() {
            let stating: Vec<usize> = js
                .iter()
                .copied()
                .filter(|&j| match &outcomes[j] {
                    Outcome::Hole(_, ms, _) => ms.iter().any(|m| members.contains(m)),
                    _ => false,
                })
                .collect();
            let mut note = None;
            if let Some(q) = groups[g].0.map(|k| reqs[&k]) {
                let with: Vec<usize> = stating
                    .iter()
                    .copied()
                    .filter(|&j| matches!(outcomes[j], Outcome::Hole(_, _, Some(_))))
                    .collect();
                if let Some(mut printed) = q.print(&mut w) {
                    printed.banded = with.iter().map(|&j| records[j].source_id.clone()).collect();
                    w.basis += &format!(
                        "; its tolerance {} is the part's AP242 PMI ({}): draftwright gives a toleranced bore diameter to the hole callout whose holes it names (lower_ap242_hole_tolerances)",
                        printed.band,
                        printed.banded.join(", ")
                    );
                    note = Some(printed);
                } else {
                    for j in with {
                        outcomes[j] = Outcome::Refused(not_lowered(format!(
                            "{id}'s label states no value for its tolerance to follow"
                        )));
                    }
                }
            }
            let stating: Vec<usize> = stating
                .into_iter()
                .filter(|&j| matches!(outcomes[j], Outcome::Hole(..)))
                .collect();
            let nominal: Vec<String> = stating
                .iter()
                .filter(|&&j| matches!(outcomes[j], Outcome::Hole(_, _, None)))
                .map(|&j| records[j].source_id.clone())
                .collect();
            if !nominal.is_empty() {
                w.basis += &format!(
                    "; it states the part's AP242 diameter ({}), which draftwright gives to the hole callout whose holes it names (lower_ap242_nominal_diameters)",
                    nominal.join(", ")
                );
            }
            for &j in &stating {
                w.measurements.push(claim(records[j], w.value));
            }
            if let Some(n) = note {
                notes.insert(stem(&w.id).to_string(), n);
            }
            if g == 0 {
                wants[i] = w;
            } else {
                split_off.push(w);
            }
        }
    }
    standalone.extend(split_off);

    // A turned step's or boss's diameter states a band only when every record on it agrees.
    let mut by_item: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (j, o) in outcomes.iter().enumerate() {
        if let Outcome::Band(i, _) = o {
            by_item.entry(*i).or_default().push(j);
        }
    }
    for (i, js) in by_item {
        let reqs: BTreeMap<Key, Req> = js
            .iter()
            .filter_map(|&j| match outcomes[j] {
                Outcome::Band(_, q) => Some((q.key(), q)),
                _ => None,
            })
            .collect();
        let id = wants[i].id.clone();
        let q = *reqs.values().next().expect("a band");
        // A counted item (`2× ø130`) states its size of each of its diameters: the records must
        // name as many distinct cylinders.
        let count: usize = wants[i]
            .terms
            .first()
            .and_then(|t| t.strip_suffix('×'))
            .and_then(|n| n.parse().ok())
            .unwrap_or(1);
        let mut named: Vec<[f64; 2]> = Vec::new();
        for &j in &js {
            for c in records[j].geometry.iter().flat_map(|g| &g.cylinders) {
                if !named.iter().any(|n| {
                    (n[0] - c.interval[0]).abs() <= 0.01 && (n[1] - c.interval[1]).abs() <= 0.01
                }) {
                    named.push(c.interval);
                }
            }
        }
        if named.len() < count {
            for j in js {
                outcomes[j] = Outcome::Refused(not_ported(format!(
                    "it names {} of the {count} diameters {id} states together; draftwright gives it to its own step, which the port does not split yet",
                    named.len()
                )));
            }
            continue;
        }
        let w = &mut wants[i];
        let printed = if reqs.len() == 1 { q.print(w) } else { None };
        let Some(mut printed) = printed else {
            for j in js {
                outcomes[j] = Outcome::Refused(not_lowered(format!(
                    "ambiguous external diameter tolerance ownership: {id} is named by AP242 diameters of different tolerances"
                )));
            }
            continue;
        };
        let sources: Vec<String> = js.iter().map(|&j| records[j].source_id.clone()).collect();
        w.basis += &format!(
            "; its tolerance {} is the part's AP242 PMI ({}): draftwright gives a toleranced external diameter to the turned step or boss whose cylinder it names (lower_ap242_external_diameter_tolerances)",
            printed.band,
            sources.join(", ")
        );
        for &j in &js {
            w.measurements.push(claim(records[j], w.value));
        }
        printed.banded = sources;
        notes.insert(stem(&w.id).to_string(), printed);
    }

    for (r, o) in records.iter().zip(outcomes) {
        match o {
            Outcome::Refused(Refusal(code, why)) => problems.push(refusal(r, code, &why)),
            Outcome::Standalone(w) => standalone.push(w),
            Outcome::Hole(..) | Outcome::Band(..) => {}
            Outcome::Stated(i, how) => {
                let w = &mut wants[i];
                w.basis += &format!("; it states the part's AP242 {how} ({})", r.source_id);
                w.measurements.push(claim(r, w.value));
            }
        }
    }
    (standalone, problems, notes)
}

fn refusal(r: &Record, code: &str, why: &str) -> Problem {
    let open = if why.contains("supplemental geometry") || why.contains("stated without limits") {
        " (an open question for the maintainer: it stays undrawn until it is decided)"
    } else {
        ""
    };
    Problem {
        id: format!("p:{code}:{}", r.source_id),
        code: code.into(),
        severity: if code == "pmi_not_lowered" {
            "error"
        } else {
            "info"
        }
        .into(),
        message: format!(
            "PMI dimension {} ({}) is not drawn: {why}{open}.",
            r.source_id,
            if r.label.is_empty() {
                &r.kind
            } else {
                &r.label
            }
        ),
        subjects: vec![r.source_id.clone()],
    }
}

/// One record's outcome against the planners' items (and the part's turning, when it is turned
/// whole).
fn outcome(r: &Record, wants: &[Want], turned: Option<&Turned>) -> Outcome {
    if !r.blockers.is_empty() {
        return Outcome::Refused(not_lowered(r.blockers.join("; ")));
    }
    let Some(g) = &r.geometry else {
        return Outcome::Refused(not_lowered(
            "its attachment was not derived (the Analysis predates PMI dimensions; analyse the part again)",
        ));
    };
    if !g.reasons.is_empty() {
        return Outcome::Refused(not_lowered(g.reasons.join("; ")));
    }
    let Some(value) = r.value else {
        return Outcome::Refused(not_lowered("the file states no nominal value"));
    };
    let band = requirement(r, value);
    let k = axis(g.axis.as_deref());
    match r.kind.as_str() {
        "angular" => Outcome::Refused(not_ported(
            "it is an angle, and the port draws no angular dimensions yet",
        )),
        "linear" | "thickness" => {
            let Some(k) = k else {
                return Outcome::Refused(not_ported(
                    "its stations are oblique to the views' axes; the port draws a dimension along x, y or z only",
                ));
            };
            if !r.basic
                && band.is_none()
                && r.label.trim() == fmt(value)
                && let Some(i) = step_length(wants, &g.stations, k, value)
            {
                return Outcome::Stated(
                    i,
                    "length, which draftwright gives to the turned step it spans (lower_ap242_nominal_step_lengths)",
                );
            }
            Outcome::Standalone(linear(r, g, k, value))
        }
        "diameter" => {
            let Some(k) = k else {
                return Outcome::Refused(not_ported(
                    "its cylinder's axis is not along x, y or z; the port draws a diameter where its cylinder is seen end-on",
                ));
            };
            let internal = g.cylinders.iter().all(|c| c.internal);
            if !r.basic
                && internal
                && let Some((i, members)) = hole_callout(wants, &g.cylinders, k, value)
            {
                // A plain diameter is stated by the callout only in words the callout can
                // head (draftwright's `_nominal_label_for_owner`).
                if band.is_some() || canonical(r, value) {
                    return Outcome::Hole(i, members, band);
                }
            }
            if !r.basic && !internal {
                match external_owner(wants, &g.cylinders, value, turned) {
                    Some(Ok(i)) => match band {
                        Some(b) => return Outcome::Band(i, b),
                        None if canonical(r, value) => {
                            return Outcome::Stated(
                                i,
                                "diameter, which draftwright gives to the turned step or boss whose cylinder it names (lower_ap242_nominal_diameters)",
                            );
                        }
                        None => {}
                    },
                    Some(Err(refusal)) => return Outcome::Refused(refusal),
                    None => {}
                }
            }
            // draftwright's `_standalone_cylinder_blocker`: one leader names one cylinder.
            let first = &g.cylinders[0];
            if g.cylinders[1..]
                .iter()
                .any(|c| (0..3).any(|j| (c.origin[j] - first.origin[j]).abs() > 0.01))
            {
                return Outcome::Refused(not_lowered(
                    "standalone diameter fallback references multiple distinct cylinder axis lines; no single truthful leader target exists",
                ));
            }
            let n = g.cylinders.len() as f64;
            let centre = g.cylinders.iter().fold([0.0; 3], |s, c| {
                let m = c.midpoint();
                [s[0] + m[0] / n, s[1] + m[1] / n, s[2] + m[2] / n]
            });
            Outcome::Standalone(leader(
                r,
                k,
                value,
                "diameter",
                vec![(centre, first.radius)],
                "to its cylinder",
            ))
        }
        "radius" => {
            let Some(k) = k else {
                return Outcome::Refused(not_ported(
                    "its arc's axis is not along x, y or z; the port draws a radius where its arc is seen end-on",
                ));
            };
            let site = g.sites[0];
            Outcome::Standalone(leader(
                r,
                k,
                value,
                "radius",
                vec![(site, 0.0)],
                "to a proven point of its arc",
            ))
        }
        other => Outcome::Refused(not_lowered(format!(
            "it states {}; a dimension is drawn along a straight projected path",
            match other {
                "curve_length" => "a length along a curve",
                "curved_dist" => "a distance along a curve",
                "oriented" => "a distance along a stated direction",
                _ => "a quantity draftwright has no drawing for",
            }
        ))),
    }
}

/// The source's own item: its feature `pmi:<source>`, its kind as the parameter.
fn own(
    r: &Record,
    value: f64,
    view: &'static str,
    views: Vec<&'static str>,
    shape: Shape,
    basis: String,
) -> Want {
    let m = claim(r, value);
    Want {
        id: want_id(&m.feature, &m.parameter, view),
        views,
        value,
        terms: terms(&r.label),
        measurements: vec![m],
        basis,
        shape,
        turning: None,
    }
}

/// A length between its two proven stations, along world axis *k*, where draftwright draws it
/// (`_pmi_front_linear` and the y branch of `_place_pmi_record`): x and z in the front view (x in
/// the plan when only the plan's witness is proven), y in the side view, else the plan. Its ends
/// sit across the axis where draftwright's witness does (`_pmi_witness_from_bbox`): the faces'
/// box's middle in the front and side views, the stations' mean in the plan (for y, when it lies
/// on the faces).
fn linear(r: &Record, g: &crate::pmi::Geometry, k: usize, value: f64) -> Want {
    let s = &g.stations;
    let b = r.ref_bbox.unwrap_or_else(|| {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in s {
            for i in 0..3 {
                b[i] = b[i].min(p[i]);
                b[i + 3] = b[i + 3].max(p[i]);
            }
        }
        b
    });
    let mid = |i: usize| 0.5 * (b[i] + b[i + 3]);
    let mean = |i: usize| s.iter().map(|p| p[i]).sum::<f64>() / s.len() as f64;
    let plan = g.view.as_deref() == Some("plan");
    let (views, at): (Vec<&'static str>, [f64; 3]) = match k {
        0 if plan => (vec!["plan"], [0.0, mean(1), mid(2)]),
        0 => (vec!["front"], [0.0, mean(1), mid(2)]),
        2 => (vec!["front"], [mid(0), mean(1), 0.0]),
        _ => {
            let x = mean(0);
            let x = if b[0] - 1e-6 <= x && x <= b[3] + 1e-6 {
                x
            } else {
                mid(0)
            };
            let views = match g.view.as_deref() {
                Some("plan") => vec!["plan"],
                Some("side") => vec!["side"],
                _ => vec!["side", "plan"],
            };
            (views, [x, 0.0, mid(2)])
        }
    };
    let lo = s.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
    let hi = s.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
    let (mut a, mut e) = (at, at);
    a[k] = lo;
    e[k] = hi;
    let view = views[0];
    let basis = format!(
        "source-authored: the AP242 {} dimension {} ({}), between its two proven stations along {}, in the {view} view (draftwright's render_pmi); no recognised feature's dimension states it, so it stands beside them{}",
        r.kind,
        r.source_id,
        r.label,
        ["x", "y", "z"][k],
        if r.basic {
            "; basic, so its label is boxed"
        } else {
            ""
        }
    );
    own(
        r,
        value,
        view,
        views,
        Shape::Linear {
            a,
            b: e,
            axis: k,
            a_edge: false,
            b_edge: false,
            order: 1,
        },
        basis,
    )
}

/// A diameter or radius led to its feature where it is seen end-on.
fn leader(r: &Record, k: usize, value: f64, what: &str, sites: Vec<(P3, f64)>, how: &str) -> Want {
    let view = end_on(k);
    let basis = format!(
        "source-authored: the AP242 {what} {} ({}), led {how} in the {view} view, which sees it end-on (draftwright's render_pmi); no recognised feature's dimension states it, so it stands beside them{}",
        r.source_id,
        r.label,
        if r.basic {
            "; basic, so its label is boxed"
        } else {
            ""
        }
    );
    own(r, value, view, vec![view], Shape::Callout { sites }, basis)
}

/// A spec's own tolerance or fit on a size wins over an imported one (draftwright's
/// `_hole_ownership_conflict`: "existing authored ownership wins"): the PMI band comes off the
/// kept item's label, its claims with it, and each source it came from is reported not drawn.
pub(super) fn yield_to_spec(
    spec: &Spec,
    notes: &BTreeMap<String, Note>,
    kept: &mut [(Want, bool, Option<&'static str>, Vec<&'static str>)],
    problems: &mut Vec<Problem>,
) {
    for (w, ..) in kept.iter_mut() {
        let Some(note) = notes.get(stem(&w.id)) else {
            continue;
        };
        let band = &note.band;
        let tolerance = spec
            .dimensions
            .get(&w.id)
            .is_some_and(|e| e.tolerance.is_some());
        let fit = w.measurements.iter().any(|m| {
            m.parameter == "bore.diameter"
                && spec
                    .features
                    .get(&m.feature)
                    .is_some_and(|e| e.fit.is_some())
        });
        if !tolerance && !fit {
            continue;
        }
        w.terms = note.before.clone();
        let gone: Vec<String> = note.banded.iter().map(|s| feature(s)).collect();
        w.measurements.retain(|m| !gone.contains(&m.feature));
        for s in &note.banded {
            problems.push(Problem {
                id: format!("p:pmi_not_lowered:{s}"),
                code: "pmi_not_lowered".into(),
                severity: "error".into(),
                message: format!(
                    "PMI dimension {s} ({band} on {}) is not drawn: the spec gives that size its own {}, and a requirement already authored on a size wins over an imported one (draftwright's lower_ap242_hole_tolerances).",
                    w.id,
                    if tolerance { "tolerance" } else { "fit" }
                ),
                subjects: vec![s.clone(), w.id.clone()],
            });
        }
    }
}

/// The PMI dimensions the plan leaves out with the items that state them (the spec omits them,
/// or another item conveys them without claiming them), each reported not drawn.
pub(super) fn omitted(
    all: &[Want],
    kept: &[(Want, bool, Option<&'static str>, Vec<&'static str>)],
    omitted: &[super::Omitted],
) -> Vec<Problem> {
    let claimed: BTreeSet<&str> = kept
        .iter()
        .flat_map(|(w, ..)| w.measurements.iter().map(|m| m.feature.as_str()))
        .filter(|f| f.starts_with("pmi:"))
        .collect();
    let mut out = Vec::new();
    for o in omitted {
        let Some(w) = all.iter().find(|w| w.id == o.id) else {
            continue;
        };
        for m in &w.measurements {
            let Some(source) = m.feature.strip_prefix("pmi:") else {
                continue;
            };
            if claimed.contains(m.feature.as_str()) {
                continue;
            }
            out.push(Problem {
                id: format!("p:pmi_not_drawn:{source}"),
                code: "pmi_not_drawn".into(),
                severity: "warning".into(),
                message: format!(
                    "PMI dimension {source} is not drawn: the plan leaves out {}, which states it ({}).",
                    o.id, o.basis
                ),
                subjects: vec![source.to_string(), o.id.clone()],
            });
        }
    }
    out
}

/// The pad between a basic dimension's label and its box, mm.
pub const BOX_PAD: f64 = 0.75;

/// A basic dimension's label box takes in its frame, so the collision checks see the ink drawn.
pub(super) fn finish(out: &mut [super::PlannedDimension]) {
    for d in out.iter_mut().filter(|d| basic(&d.measurements)) {
        if let Some(p) = &mut d.placed {
            let b = &mut p.label_box;
            *b = [
                b[0] - BOX_PAD,
                b[1] - BOX_PAD,
                b[2] + BOX_PAD,
                b[3] + BOX_PAD,
            ];
        }
    }
}

/// A callout cut into *groups* of its holes (by index), each its own callout of those holes: its
/// count, measurements and sites theirs, its ID its first hole's. One group is the callout as it
/// is.
fn split(w: &Want, groups: &[Group]) -> Vec<(Want, Vec<usize>)> {
    if groups.len() == 1 {
        return vec![(w.clone(), groups[0].1.clone())];
    }
    let holes = holes_of(w);
    let Shape::Callout { sites } = &w.shape else {
        return vec![(w.clone(), groups[0].1.clone())];
    };
    groups
        .iter()
        .map(|(_, members)| {
            let features: BTreeSet<&str> = members.iter().map(|&m| holes[m].0.as_str()).collect();
            let mut part = w.clone();
            let n = members.len();
            // A grid's arrangement, `(2×2)`, is no longer this callout's; a face it names is.
            part.terms.retain_mut(|t| {
                if !t.starts_with('(') {
                    return true;
                }
                match t.split_once(") ") {
                    Some((_, face)) => {
                        *t = face.to_string();
                        true
                    }
                    None => false,
                }
            });
            if n > 1 {
                part.terms[0] = format!("{n}×");
            } else {
                part.terms.remove(0);
            }
            part.measurements
                .retain(|m| features.contains(m.feature.as_str()));
            part.shape = Shape::Callout {
                sites: members.iter().map(|&m| sites[m]).collect(),
            };
            let view = w.views[0];
            part.id = want_id(&holes[members[0]].0, "bore.diameter", view);
            part.basis += &format!(
                "; {n} of the {} identical holes, called out apart from the others because their AP242 requirements differ (draftwright splits a count group by requirement){}",
                holes.len(),
                if stem(&w.id).starts_with("d:f:hole_pattern:") {
                    "; the pattern's pitches still locate them"
                } else {
                    ""
                }
            );
            (part, members.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_become_deviations_in_decimal() {
        assert_eq!(decimal_sub(35.0, 34.8), 0.2);
        assert_eq!(decimal_sub(35.2, 35.0), 0.2);
        assert_eq!(decimal_sub(6.35, 6.35), 0.0);
        assert_eq!(decimal_sub(0.5588, 0.4318), 0.127);
    }

    #[test]
    fn symmetric_limits_print_as_a_band_and_asymmetric_ones_as_limits() {
        let callout = || Want {
            id: "d:f:hole:1/bore.diameter@plan".into(),
            views: vec!["plan"],
            value: 35.0,
            terms: vec!["2×".into(), "⌀35".into(), "THRU".into()],
            measurements: Vec::new(),
            basis: String::new(),
            shape: Shape::Callout { sites: Vec::new() },
            turning: None,
        };
        let limits = |lo: f64, hi: f64| Req {
            upper: decimal_sub(hi, 35.0),
            lower: decimal_sub(lo, 35.0),
            limits: Some((lo, hi)),
        };
        let mut w = callout();
        assert_eq!(limits(34.8, 35.2).print(&mut w).unwrap().band, "±0.2");
        assert_eq!(w.terms, ["2×", "⌀35", "±0.2", "THRU"]);
        let mut w = callout();
        let note = limits(34.8, 35.1).print(&mut w).unwrap();
        assert_eq!(w.terms, ["2×", "⌀34.8", "-", "⌀35.1", "THRU"]);
        assert_eq!(note.before, callout().terms);
        let mut w = callout();
        let shaft = Req {
            upper: -0.009,
            lower: -0.025,
            limits: None,
        };
        shaft.print(&mut w).unwrap();
        assert_eq!(w.terms, ["2×", "⌀35", "-0.009 -0.025", "THRU"]);
    }

    #[test]
    fn labels_read_as_callout_terms() {
        assert_eq!(terms("ø62 +0.03/0"), ["⌀62", "+0.03/0"]);
        assert_eq!(terms("ø34.8 - ø35.2"), ["⌀34.8", "-", "⌀35.2"]);
        assert_eq!(terms("0.30 ±0.02 INCH"), ["0.30", "±0.02", "INCH"]);
        assert_eq!(terms("R12.7 ±0.508"), ["R12.7", "±0.508"]);
    }
}
