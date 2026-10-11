//! Authored tolerances and fits: what the spec says of a size reaches the label verbatim
//! (ARCH0011-1: a tolerance or fit belongs to a dimension), and nothing else does (ADR5-D2).
//!
//! - **A tolerance** is set on a dimension by its ID (`dimensions/<id>/tolerance`): `±T`, or
//!   `+U -L` (an upper and a lower deviation, both magnitudes), as draftwright's
//!   `.tolerance(t)` and `.tolerance(lo, hi)` print them (`_fmt_tolerance`). It is printed after
//!   the term that states the dimension's value, at the drawn precision (one decimal) or the
//!   authored digits if there are more: the drawn precision is a minimum, and an authored
//!   magnitude is never rounded into another requirement.
//! - **A fit** is set on a hole or a cylindrical boss (`features/<id>/fit`): an ISO 286 class
//!   from draftwright's table (`fits.py`: holes H, G, F; shafts h, g, f, js, k, n, p; grades 5 to
//!   11; nominal sizes up to 250 mm). It is printed after the diameter of the callout that states
//!   the feature's size, as the class and its limits (`⌀8.1 H7 (+0.015/0) THRU`), or the class
//!   alone when the sheet hides fit limits (`sheet/fit_limits: false`, draftwright's form).
//!
//! A band is withheld, and the plan says so, when the sheet cannot state it truthfully:
//! - **a band of nothing** (ARCH0019-2): with the authored digits kept, only a band whose every
//!   magnitude is zero rounds to zero, and `±0.0` states no tolerance;
//! - **a collapsed mark whose members disagree** (ARCH0019-3): an `n×` callout states one size
//!   for every member, so it states a fit only when every member has the same one. (A pattern's
//!   `n× p` pitch needs no such check here: the hole planner collapses only gaps equal within
//!   1e-4 mm, where draftwright's `_pitch_text` has to withhold a band over jittered ones.)
//!
//! An entry for a dimension the plan leaves out, or a fit on a feature no planned callout
//! states, is reported against its ID, never dropped in silence.

use std::collections::{BTreeMap, BTreeSet};

use super::{Omitted, Want, fmt, levenshtein};
use crate::items::{Op, State, key};
use crate::model::{Analysis, Invalid, Problem, Spec};

/// A kept item, as [`super::plan`] holds it: the want, whether it is a default, its view and the
/// views that look along it.
type Kept = (Want, bool, Option<&'static str>, Vec<&'static str>);

/// The feature kinds a fit can be set on, and the parameter whose size it qualifies.
const FITTED: [(&str, &str); 2] = [("hole", "bore.diameter"), ("boss", "boss.diameter")];
/// draftwright's fit table (`fits.py`): the hole classes, the shaft classes and the grades.
const HOLE_CLASSES: [&str; 3] = ["H", "G", "F"];
const SHAFT_CLASSES: [&str; 7] = ["h", "g", "f", "js", "k", "n", "p"];
const GRADES: std::ops::RangeInclusive<u32> = 5..=11;
/// The largest nominal size the table covers, mm (ISO 286's band up to and including 250).
const LARGEST: f64 = 250.0;
/// The accepted forms, as the refusal names them.
const FORMS: [&str; 2] = ["±0.05", "+0.1 -0.02"];

/// A magnitude as authored (`0.05`, `1`): digits with an optional fraction, no sign.
fn magnitude(s: &str) -> Option<&str> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, "0"));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    (digits(whole) && digits(frac)).then_some(s)
}

/// A magnitude printed as draftwright prints it: its authored digits, with at least the drawn
/// precision (one decimal), so `1` reads `1.0` and `0.05` stays `0.05`.
fn printed(m: &str) -> String {
    match m.split_once('.') {
        Some(_) => m.to_string(),
        None => format!("{m}.0"),
    }
}

fn zero(m: &str) -> bool {
    m.bytes().all(|b| b == b'0' || b == b'.')
}

/// An authored tolerance read: its printed text, and whether it is a band of nothing. The error
/// says what was wrong.
fn tolerance(text: &str) -> Result<(String, bool), String> {
    let t = text.trim();
    if let Some(m) = t.strip_prefix('±').and_then(magnitude) {
        return Ok((format!("±{}", printed(m)), zero(m)));
    }
    let parts: Vec<&str> = t.split_whitespace().collect();
    if let [upper, lower] = parts[..]
        && let (Some(u), Some(l)) = (
            upper.strip_prefix('+').and_then(magnitude),
            lower.strip_prefix('-').and_then(magnitude),
        )
    {
        return Ok((
            format!("+{} -{}", printed(u), printed(l)),
            zero(u) && zero(l),
        ));
    }
    // Close but wrong: the ASCII spellings of ± and a limit pair written with a slash.
    let close = t.replace("+/-", "±").replace("+-", "±").replace('/', " ");
    if close != t
        && let Ok((reads, _)) = tolerance(&close)
    {
        return Err(format!("{text:?} reads as {reads:?}"));
    }
    Err(format!(
        "{text:?} is not a tolerance draftwright reads: a symmetric band is \"±T\" and a pair of deviations \"+U -L\" (the upper above the nominal, the lower below it, both as magnitudes)"
    ))
}

/// An ISO 286 class from draftwright's table: its letters and grade, or why not.
pub(super) fn fit(code: &str) -> Result<(&str, u32), String> {
    let letters = code.trim_end_matches(|c: char| c.is_ascii_digit());
    let grade = &code[letters.len()..];
    if letters.is_empty() || grade.is_empty() || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!(
            "{code:?} is not a fit: a fit is an ISO 286 class, letters and a grade (H7, g6)"
        ));
    }
    let grade: u32 = grade
        .parse()
        .map_err(|_| format!("{code:?} is not a fit"))?;
    if !GRADES.contains(&grade) {
        return Err(format!(
            "the fit {code:?} has grade IT{grade}, outside draftwright's table (IT5 to IT11)"
        ));
    }
    if !HOLE_CLASSES.contains(&letters) && !SHAFT_CLASSES.contains(&letters) {
        return Err(format!(
            "the fit {code:?} is not in draftwright's table (holes H, G, F; shafts h, g, f, js, k, n, p)"
        ));
    }
    Ok((letters, grade))
}

/// ISO 286 nominal size bands: the upper bound, mm, of each "over the previous, up to and
/// including this" step (draftwright's `fits._BANDS`).
const BANDS: [f64; 10] = [3.0, 6.0, 10.0, 18.0, 30.0, 50.0, 80.0, 120.0, 180.0, 250.0];
/// Standard tolerance grades IT5 to IT11, µm, per band (ISO 286-1 Table 1; `fits._IT`).
const IT: [[i32; 10]; 7] = [
    [4, 5, 6, 8, 9, 11, 13, 15, 18, 20],
    [6, 8, 9, 11, 13, 16, 19, 22, 25, 29],
    [10, 12, 15, 18, 21, 25, 30, 35, 40, 46],
    [14, 18, 22, 27, 33, 39, 46, 54, 63, 72],
    [25, 30, 36, 43, 52, 62, 74, 87, 100, 115],
    [40, 48, 58, 70, 84, 100, 120, 140, 160, 185],
    [60, 75, 90, 110, 130, 160, 190, 220, 250, 290],
];
/// The shafts' fundamental upper deviation es, µm, per band, for the clearance letters (`fits._ES`);
/// the hole classes H, G and F mirror them (EI = -es).
const ES: [(&str, [i32; 10]); 3] = [
    ("h", [0; 10]),
    ("g", [-2, -4, -5, -6, -7, -9, -10, -12, -14, -15]),
    ("f", [-6, -10, -13, -16, -20, -25, -30, -36, -43, -50]),
];
/// The shafts' fundamental lower deviation ei, µm, per band, for the interference letters
/// (`fits._EI`).
const EI: [(&str, [i32; 10]); 3] = [
    ("k", [0, 1, 1, 1, 2, 2, 2, 3, 3, 4]),
    ("n", [4, 8, 10, 12, 15, 17, 20, 23, 27, 31]),
    ("p", [6, 12, 15, 18, 22, 26, 32, 37, 43, 50]),
];

/// A class's limit deviations at a nominal diameter, mm: `(lower, upper)`, as draftwright's
/// `fits.fit_deviation` gives them. None outside the table (a class or grade it does not hold,
/// a nominal not over 0 or above 250 mm).
pub(super) fn deviations(code: &str, nominal: f64) -> Option<(f64, f64)> {
    let (letters, grade) = fit(code).ok()?;
    let band = BANDS.iter().position(|&b| nominal > 0.0 && nominal <= b)?;
    let it = IT[(grade - 5) as usize][band];
    let table =
        |t: &[(&str, [i32; 10])], l: &str| t.iter().find(|(k, _)| *k == l).map(|(_, v)| v[band]);
    let lower = letters.to_lowercase();
    let (lo, hi) = if HOLE_CLASSES.contains(&letters) {
        let ei = -table(&ES, &lower)?;
        (ei, ei + it)
    } else if letters == "js" {
        return Some((-f64::from(it) / 2000.0, f64::from(it) / 2000.0));
    } else if let Some(es) = table(&ES, letters) {
        (es - it, es)
    } else {
        // k's tabulated deviation holds for IT4 to IT7 only; ISO 286 makes it 0 for coarser
        // grades (`fits.fit_deviation`, #29).
        let ei = if letters == "k" && grade > 7 {
            0
        } else {
            table(&EI, letters)?
        };
        (ei, ei + it)
    };
    Some((f64::from(lo) / 1000.0, f64::from(hi) / 1000.0))
}

/// A class with its limits, as the sheet prints it when they are shown: `H7 (+0.015/0)`.
pub(super) fn with_limits(code: &str, (lower, upper): (f64, f64)) -> String {
    format!("{code} {}", limits_term((lower, upper)))
}

/// The limits alone: `(+0.015/0)`.
pub(super) fn limits_term((lower, upper): (f64, f64)) -> String {
    format!("({}/{})", deviation(upper), deviation(lower))
}

/// Whether the sheet shows a fit's limits beside its class (`sheet/fit_limits`, shown unless the
/// spec says false).
pub(super) fn limits_shown(spec: &Spec) -> bool {
    spec.sheet.fit_limits != Some(false)
}

/// A limit deviation as a fit prints it (draftwright's `fits._fmt_dev`): signed, to the µm
/// digit, and an exact zero as `0`.
pub(super) fn deviation(v: f64) -> String {
    if v.abs() < 5e-5 {
        return "0".into();
    }
    let s = format!("{v:+.4}");
    s.strip_suffix('0').map_or(s.clone(), str::to_string)
}

/// Checks every tolerance and fit the spec authors reads as one: the form, never the part (that
/// is [`apply`]'s). A refusal names the entry.
pub(crate) fn check(spec: &Spec) -> Result<(), Invalid> {
    for (id, entry) in &spec.dimensions {
        if let Some(t) = &entry.tolerance {
            tolerance(t).map_err(|why| Invalid {
                message: format!("The tolerance of {id:?} cannot be read: {why}."),
                choices: FORMS.map(String::from).to_vec(),
            })?;
        }
    }
    for (id, entry) in &spec.features {
        if let Some(code) = &entry.fit {
            fit(code).map_err(|why| Invalid {
                message: format!("The fit of {id:?} cannot be read: {why}."),
                choices: vec![],
            })?;
        }
    }
    Ok(())
}

/// The parameter an ID names: `d:<feature>/<parameter>`.
fn parameter(id: &str) -> &str {
    id.rsplit_once('/').map_or(id, |(_, p)| p)
}

/// The term a band follows: the first that states a number and is not a count (`4×`).
fn value_term(terms: &[String]) -> Option<usize> {
    terms.iter().position(|t| {
        t.chars().any(|c| c.is_ascii_digit())
            && !(t.ends_with('×') && t.trim_end_matches('×').bytes().all(|b| b.is_ascii_digit()))
    })
}

/// The edit path of a dimension's tolerance: a `/` inside the ID is written `~1`.
fn path(id: &str) -> String {
    format!("dimensions/{}/tolerance", key(id))
}

fn problem(code: &str, subject: &str, message: String, subjects: Vec<String>) -> Problem {
    Problem {
        id: format!("p:{code}:{subject}"),
        code: code.into(),
        severity: "warning".into(),
        message,
        subjects,
        ..Default::default()
    }
}

/// Puts the spec's tolerances and fits on the kept items' labels, before they are placed, and
/// reports what cannot be stated. Returns, for each kept item, what of it the spec gave
/// (`tolerance`, `fit`). Refused: a fit on a feature that does not exist or cannot take one, or
/// that draftwright's table does not cover at its size, and a size given both a tolerance and a
/// fit.
pub(super) fn apply(
    analysis: &Analysis,
    spec: &Spec,
    kept: &mut [Kept],
    omitted: &[Omitted],
    problems: &mut Vec<Problem>,
) -> Result<Vec<Vec<String>>, Invalid> {
    check(spec)?;
    let patterns = analysis
        .recognition
        .as_ref()
        .map_or(&[][..], |r| r.patterns.as_slice());
    // Each fit, on a feature that can take it, at its size.
    let mut fits: BTreeMap<&str, &str> = BTreeMap::new();
    let mut nominals: BTreeMap<&str, f64> = BTreeMap::new();
    for (id, entry) in &spec.features {
        let Some(code) = entry.fit.as_deref() else {
            continue;
        };
        let Some(f) = analysis.features.iter().find(|f| f.id == *id) else {
            if let Some(p) = patterns.iter().find(|p| p.id == *id) {
                return Err(Invalid {
                    message: format!(
                        "{id:?} is a hole pattern; a fit belongs to a hole, so set it on each of its holes (features/<hole>/fit)."
                    ),
                    choices: p.members.clone(),
                });
            }
            let mut near: Vec<&str> = analysis.features.iter().map(|f| f.id.as_str()).collect();
            near.sort_by_key(|c| (levenshtein(id, c), *c));
            near.truncate(5);
            return Err(Invalid {
                message: format!("There is no feature {id:?} on this part."),
                choices: near.into_iter().map(String::from).collect(),
            });
        };
        if !FITTED.iter().any(|(k, _)| *k == f.kind) {
            return Err(Invalid {
                message: format!(
                    "{id:?} is a {}; a fit is set on a hole or a cylindrical boss.",
                    f.kind
                ),
                choices: vec![],
            });
        }
        let (letters, grade) = fit(code).expect("checked");
        let hole = f.kind == "hole";
        if hole != HOLE_CLASSES.contains(&letters) {
            let other = if hole {
                letters.to_uppercase()
            } else {
                letters.to_lowercase()
            };
            let (what, class) = if hole {
                ("hole", "an upper-case hole class")
            } else {
                ("boss", "a lower-case shaft class")
            };
            return Err(Invalid {
                message: format!(
                    "{code:?} reads as \"{other}{grade}\": {id:?} is a {what}, and takes {class}."
                ),
                choices: vec![format!("{other}{grade}")],
            });
        }
        let d = f.parameters.get("diameter").and_then(|v| v.as_f64());
        match d {
            Some(d) if d > 0.0 && d <= LARGEST => {}
            _ => {
                return Err(Invalid {
                    message: format!(
                        "The fit {code:?} of {id:?} cannot be resolved: its nominal ⌀{} is outside draftwright's ISO 286 table (up to {LARGEST} mm); give its dimension a tolerance instead.",
                        d.map_or("?".into(), fmt)
                    ),
                    choices: vec![],
                });
            }
        }
        fits.insert(id, code);
        nominals.insert(id, d.expect("in the table"));
    }

    let mut from_spec = Vec::with_capacity(kept.len());
    let mut stated: BTreeSet<&str> = BTreeSet::new();
    for (w, ..) in kept.iter_mut() {
        let mut given = Vec::new();
        let lead = parameter(&w.id).to_string();
        // The members whose size the label states, and their fits.
        let mut members: Vec<String> = Vec::new();
        if FITTED.iter().any(|(_, p)| *p == lead) {
            for m in &w.measurements {
                if m.parameter == lead && !members.contains(&m.feature) {
                    members.push(m.feature.clone());
                }
            }
        }
        let member_fits: Vec<Option<&str>> = members
            .iter()
            .map(|m| fits.get(m.as_str()).copied())
            .collect();
        for m in &members {
            if let Some((k, _)) = fits.get_key_value(m.as_str()) {
                stated.insert(*k);
            }
        }
        let authored = spec.dimensions.get(&w.id).and_then(|e| e.tolerance.clone());
        if authored.is_some() && member_fits.iter().any(Option::is_some) {
            let fitted: Vec<&str> = members
                .iter()
                .zip(&member_fits)
                .filter(|(_, f)| f.is_some())
                .map(|(m, _)| m.as_str())
                .collect();
            return Err(Invalid {
                message: format!(
                    "{:?} has a tolerance and {} a fit; a size takes one or the other. Remove {} or the fit.",
                    w.id,
                    fitted.join(", "),
                    path(&w.id)
                ),
                choices: vec![],
            });
        }
        let band = if let Some(t) = authored {
            let (text, nothing) = tolerance(&t).expect("checked");
            if nothing {
                problems.push(Problem {
                    fix: vec![Op::remove(path(&w.id))],
                    ..problem(
                        "tolerance_withheld",
                        &w.id,
                        format!(
                            "{} is not printed on {}: a band of nothing ({text}) states no tolerance (ARCH0019-2). Give it a band, or remove {}.",
                            t.trim(),
                            w.id,
                            path(&w.id)
                        ),
                        vec![w.id.clone()],
                    )
                });
                None
            } else {
                given.push("tolerance".to_string());
                w.basis += "; its tolerance is the spec's";
                Some(text)
            }
        } else if member_fits.iter().all(Option::is_some) && !member_fits.is_empty() {
            let first = member_fits[0].expect("all are some");
            if member_fits.iter().all(|f| *f == Some(first)) {
                given.push("fit".to_string());
                w.basis += &format!("; its fit is the spec's ({})", members.join(", "));
                let limits = nominals
                    .get(members[0].as_str())
                    .and_then(|&d| deviations(first, d))
                    .filter(|_| limits_shown(spec));
                Some(limits.map_or(first.to_string(), |l| with_limits(first, l)))
            } else {
                None
            }
        } else {
            None
        };
        // A collapsed mark whose members do not all have one fit states none (ARCH0019-3).
        if member_fits.iter().any(Option::is_some) && band.is_none() {
            let told: Vec<String> = members
                .iter()
                .zip(&member_fits)
                .map(|(m, f)| format!("{m} {}", f.unwrap_or("none")))
                .collect();
            problems.push(problem(
                "tolerance_withheld",
                &w.id,
                format!(
                    "No fit is printed on {}: its {} members do not all have the same fit ({}), and one label would claim it of every member (ARCH0019-3). Give them all the same fit, or none.",
                    w.id,
                    members.len(),
                    told.join(", ")
                ),
                std::iter::once(w.id.clone())
                    .chain(members.iter().cloned())
                    .collect(),
            ));
        }
        if let Some(text) = band {
            match value_term(&w.terms) {
                Some(i) => w.terms.insert(i + 1, text),
                None => {
                    given.clear();
                    problems.push(problem(
                        "tolerance_withheld",
                        &w.id,
                        format!(
                            "{text} is not printed on {}: its label states no value for it to follow.",
                            w.id
                        ),
                        vec![w.id.clone()],
                    ));
                }
            }
        }
        from_spec.push(given);
    }

    // What the spec says of something the plan does not state.
    for (id, entry) in &spec.dimensions {
        if entry.tolerance.is_some()
            && let Some(o) = omitted.iter().find(|o| o.id == *id)
        {
            // Brought back when the spec left it out; else the tolerance goes.
            let fix = match o.state {
                State::OmittedByUser => Op::remove(format!("dimensions/{}/omit", key(id))),
                _ => Op::remove(path(id)),
            };
            problems.push(Problem {
                fix: vec![fix],
                ..problem(
                    "tolerance_not_shown",
                    id,
                    format!(
                        "The tolerance of {id} is not printed: the plan leaves the dimension out ({}). Remove {} or bring the dimension back.",
                        o.basis,
                        path(id)
                    ),
                    vec![id.clone()],
                )
            });
        }
    }
    for (id, code) in &fits {
        if !stated.contains(id) {
            problems.push(Problem {
                fix: vec![Op::remove(format!("features/{}/fit", key(id)))],
                ..problem(
                    "tolerance_not_shown",
                    id,
                    format!(
                        "The fit {code} of {id} is not printed: no dimension in the plan states its size."
                    ),
                    vec![id.to_string()],
                )
            });
        }
    }
    Ok(from_spec)
}

/// What the spec gives a kept item's size, as [`apply`] decides it: its `tolerance` or its
/// members' `fit`, and whether that is printed (a band of nothing, or a fit the members do not
/// all share, is withheld). None when the spec gives it neither.
pub(super) fn authored(spec: &Spec, w: &Want) -> Option<(&'static str, bool)> {
    if let Some(t) = spec
        .dimensions
        .get(&w.id)
        .and_then(|e| e.tolerance.as_deref())
    {
        return Some(("tolerance", tolerance(t).is_ok_and(|(_, nothing)| !nothing)));
    }
    let lead = parameter(&w.id);
    if !FITTED.iter().any(|(_, p)| *p == lead) {
        return None;
    }
    let mut members: Vec<&str> = Vec::new();
    for m in &w.measurements {
        if m.parameter == lead && !members.contains(&m.feature.as_str()) {
            members.push(&m.feature);
        }
    }
    let fits: Vec<Option<&str>> = members
        .iter()
        .map(|m| spec.features.get(*m).and_then(|e| e.fit.as_deref()))
        .collect();
    if fits.iter().all(Option::is_none) {
        return None;
    }
    Some(("fit", fits.iter().all(|f| *f == fits[0])))
}

/// A band from two signed deviations, the upper and the lower, as draftwright prints an
/// imported requirement on a recognised feature's size (`_fmt_tolerance`): `±T` when they are
/// opposite, else `+U -L`, each with its authored digits and at least the drawn precision. A band
/// wholly above or below the nominal keeps its signs (`-0.014 -0.039`, ISO 129-1), which
/// draftwright's unsigned deviations cannot state.
pub(super) fn band(upper: f64, lower: f64) -> String {
    let m = |v: f64| printed(&format!("{}", v.abs()));
    if upper > 0.0 && upper == -lower {
        format!("±{}", m(upper))
    } else {
        let up = if upper < 0.0 { '-' } else { '+' };
        let down = if lower > 0.0 { '+' } else { '-' };
        format!("{up}{} {down}{}", m(upper), m(lower))
    }
}

/// Puts *text* after the term of *w*'s label that states its value, as a spec's tolerance is put
/// ([`apply`]); false when the label states no value for it to follow.
pub(super) fn insert(w: &mut super::Want, text: &str) -> bool {
    match value_term(&w.terms) {
        Some(i) => {
            w.terms.insert(i + 1, text.to_string());
            true
        }
        None => false,
    }
}

/// Puts a limit pair in place of the term of *w*'s label that states its value, keeping that
/// term's symbol (`⌀35` → `⌀34.8 - ⌀35.1`), as draftwright states an imported limit pair that is
/// not symmetric (`bore_callout_value`); false when the label states no value.
pub(super) fn limits(w: &mut super::Want, lower: f64, upper: f64) -> bool {
    let Some(i) = value_term(&w.terms) else {
        return false;
    };
    let prefix: String = w.terms[i]
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .collect();
    w.terms.splice(
        i..=i,
        [
            format!("{prefix}{lower}"),
            "-".into(),
            format!("{prefix}{upper}"),
        ],
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerances_read_as_draftwright_prints_them() {
        let read = |t: &str| tolerance(t).map(|r| r.0);
        assert_eq!(read("±0.05").unwrap(), "±0.05");
        assert_eq!(read("±1").unwrap(), "±1.0");
        assert_eq!(read("±0.02").unwrap(), "±0.02");
        assert_eq!(read("+0.1 -0.02").unwrap(), "+0.1 -0.02");
        assert_eq!(read("+0.1 -0").unwrap(), "+0.1 -0.0");
        assert!(tolerance("±0").unwrap().1 && tolerance("+0.0 -0").unwrap().1);
        assert!(!tolerance("+0.1 -0").unwrap().1);
        assert_eq!(read("+-0.05").unwrap_err(), "\"+-0.05\" reads as \"±0.05\"");
        assert_eq!(
            read("+0.1/-0.02").unwrap_err(),
            "\"+0.1/-0.02\" reads as \"+0.1 -0.02\""
        );
        for bad in [
            "0.05",
            "±-0.05",
            "±.05",
            "-0.02 +0.1",
            "±0.05mm",
            "±1e-2",
            "",
        ] {
            assert!(read(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn imported_bands_print_as_draftwright_prints_them() {
        assert_eq!(band(0.2, -0.2), "±0.2");
        assert_eq!(band(0.09, 0.0), "+0.09 -0.0");
        assert_eq!(band(0.0, -0.2), "+0.0 -0.2");
        assert_eq!(band(0.0762, -0.0254), "+0.0762 -0.0254");
        assert_eq!(band(1.0, -1.0), "±1.0");
        assert_eq!(band(-0.014, -0.039), "-0.014 -0.039");
        assert_eq!(band(0.039, 0.014), "+0.039 +0.014");
    }

    #[test]
    fn fit_classes_have_iso_286_limits() {
        // ISO 286-2: ⌀10 H7 is +0.015/0, ⌀10 G7 +0.020/+0.005, ⌀30 F8 +0.053/+0.020; a
        // band's upper bound belongs to it (⌀3 H11 is +0.060/0).
        let at = |c, d| deviations(c, d).map(|(l, u)| (deviation(u), deviation(l)));
        assert_eq!(at("H7", 10.0), Some(("+0.015".into(), "0".into())));
        assert_eq!(at("G7", 10.0), Some(("+0.020".into(), "+0.005".into())));
        assert_eq!(at("F8", 30.0), Some(("+0.053".into(), "+0.020".into())));
        assert_eq!(at("H11", 3.0), Some(("+0.060".into(), "0".into())));
        assert_eq!(at("H7", 250.5), None);
        // Shafts (`fits.fit_deviation("g6", 20) == (-0.020, -0.007)`), js symmetric, k8 from 0.
        assert_eq!(at("g6", 20.0), Some(("-0.007".into(), "-0.020".into())));
        assert_eq!(at("js6", 20.0), Some(("+0.0065".into(), "-0.0065".into())));
        assert_eq!(at("k8", 20.0), Some(("+0.033".into(), "0".into())));
        assert_eq!(at("H4", 10.0), None);
    }

    #[test]
    fn fits_are_draftwrights_table() {
        assert_eq!(fit("H7"), Ok(("H", 7)));
        assert_eq!(fit("js6"), Ok(("js", 6)));
        for bad in ["H4", "H12", "K7", "N7", "x6", "7", "H", "H 7", "Hh7"] {
            assert!(fit(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn what_the_spec_gives_a_size_includes_a_bosss_fit() {
        let want = |id: &str, parameter: &str, members: &[&str]| Want {
            id: id.into(),
            views: vec!["front"],
            value: 20.0,
            terms: vec!["ø20".into()],
            measurements: members
                .iter()
                .map(|f| super::super::Measurement {
                    feature: f.to_string(),
                    parameter: parameter.into(),
                    value: 20.0,
                })
                .collect(),
            basis: String::new(),
            shape: super::super::Shape::Callout { sites: Vec::new() },
            turning: None,
        };
        let mut spec = Spec::default();
        for f in ["f:boss:1", "f:hole:1"] {
            spec.features.insert(
                f.into(),
                serde_json::from_value(serde_json::json!({"fit": "H7"})).unwrap(),
            );
        }
        let boss = want("d:f:boss:1/boss.diameter", "boss.diameter", &["f:boss:1"]);
        assert_eq!(authored(&spec, &boss), Some(("fit", true)));
        let mixed = want(
            "d:f:hole:1/bore.diameter",
            "bore.diameter",
            &["f:hole:1", "f:hole:2"],
        );
        assert_eq!(authored(&spec, &mixed), Some(("fit", false)));
        let step = want(
            "d:f:turned_step:1/step.diameter",
            "step.diameter",
            &["f:boss:1"],
        );
        assert_eq!(authored(&spec, &step), None);
        spec.dimensions.insert(
            step.id.clone(),
            serde_json::from_value(serde_json::json!({"tolerance": "±0"})).unwrap(),
        );
        assert_eq!(authored(&spec, &step), Some(("tolerance", false)));
    }

    #[test]
    fn a_band_follows_the_value_not_the_count() {
        let terms = |t: &[&str]| t.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(value_term(&terms(&["4×", "⌀10", "THRU", "(2×2)"])), Some(1));
        assert_eq!(value_term(&terms(&["3× 72.4"])), Some(0));
        assert_eq!(value_term(&terms(&["HEX", "100", "A/F"])), Some(1));
        assert_eq!(value_term(&terms(&["28.8"])), Some(0));
    }
}
