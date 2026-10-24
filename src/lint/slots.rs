//! Slot coverage (Python's `linting/slot_coverage.py`): a lone through slot states its width, its
//! length, and its end radius when its ends are proven arcs (LINT-slot_coverage-lone_slot,
//! LINT-code-slot_requirement_state).
//!
//! Each of the Analysis's `slot` features (quiddity's `Slot`: `width`, `length`, `end_radius`
//! when the ends are proven arcs, and its place: `long_axis`, `lo`/`hi` along it, `w_center` on
//! `width_axis`) has the requirements `slot_width.length`, `slot_length.length`, for an
//! obround `slot_end_radius.radius`, and its location `location_slot.length`, judged by
//! [`parameters::coverage`]. The location is draftwright's (`compiled.py`, `location_datum` is
//! `bbox` for a slot): the offset of the slot's low end along its long axis from the part's box
//! ([`location`]); a slot whose long axis lies along no axis of the part has a location that
//! cannot be checked.
//!
//! A slot that is a member of a recognised slot pattern (quiddity's `SlotArray` or `SlotGrid`, a
//! `slot_pattern` feature whose faces include the slot's) is the pattern's, not a lone slot,
//! as in Python. The pattern has its own requirements ([`pattern_wanted`];
//! LINT-slot_coverage-slot_pattern, Python's `_parameter_ids` and `_physical_requirement_count`):
//! its member's width and length (`slot_width.length`, `slot_length.length`), an obround's end
//! radius (`slot_end_radius.radius`), a linear array's pitch (`pitch.length`) or a grid's column
//! and row pitches (`grid_pitch.length.col`, `.row`, quiddity's `col_pitch` and `row_pitch`), and
//! its centre's location from the part's box in x and in y (`location_slot_pattern.length.x`,
//! `.y`: the array's centroid or the grid's centre, as the dimensions locate it), five
//! requirements for an array and six for a grid, one more for an obround. As in Python, a
//! pattern cut along anything but the part's z has no location vocabulary, and is one
//! requirement that cannot be checked; so is a pattern whose members cannot be read or are not
//! identical (never a guess from one member).

use serde_json::Value;

use super::parameters::{self, Wanted, fmt, fmt3};
use super::{LintIssue, Mark, Requirement, Severity};
use crate::bridge::Feature;
use crate::bridge::holes::{dir_to_part, frame, principal, to_part};
use crate::model::Analysis;

const AXES: [&str; 3] = ["x", "y", "z"];

/// A slot's location: the part's axis its long axis lies along, and the offset of its low end
/// along it from the part's box (the Analysis's envelope), or why it cannot be read.
pub fn location(analysis: &Analysis, f: &Feature) -> Result<(&'static str, f64), String> {
    let p = &f.parameters;
    let long = p
        .get("long_axis")
        .and_then(Value::as_str)
        .and_then(|a| AXES.iter().position(|x| *x == a))
        .ok_or("its record gives no long axis")?;
    let (lo, hi) = (parameters::size(f, "lo")?, parameters::size(f, "hi")?);
    let frame = frame(analysis);
    let mut d = [0.0; 3];
    d[long] = 1.0;
    let d = dir_to_part(&frame, d);
    let k = principal(d).ok_or("its long axis lies along no axis of the part")?;
    // The ends' coordinates along the part's axis k: quiddity's origin, and its long axis
    // carried onto k (its other axes are square to k, so they add nothing).
    let ends = [lo, hi].map(|t| frame[0][k] + t * d[k]);
    // The low end the planner dimensions ([`outline`]: the exact centre of the slot's faces less
    // half its length, where the record agrees with it). quiddity rounds `lo`, `hi` and `length`
    // to 0.01 mm each, so the recorded `lo` and that end can round apart on a 0.005 tie (cgb245:
    // 95.96 recorded, 95.9648 from the centre); lint judges the end the drawing states.
    let low = match outline(analysis, f) {
        Ok(o) if o.long == k => o.ends[0],
        _ => ends[0].min(ends[1]),
    };
    Ok((AXES[k], low - analysis.envelope[0][k]))
}

/// A through slot in the part's frame, as the planner places it (`dims::slots`): the axes it
/// runs along, spans across and passes through, its ends along the first, its centre line on the
/// second, its span through it, its width, and its end radius when its ends are proven arcs.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub long: usize,
    pub wide: usize,
    pub deep: usize,
    pub ends: [f64; 2],
    pub centre: f64,
    pub through: [f64; 2],
    pub width: f64,
    pub end_radius: Option<f64>,
}

/// Slot *f*'s [`Outline`], or why its record cannot be read. quiddity rounds `lo`, `hi` and
/// `w_center` to 0.01 mm in its own frame; the box around its faces (the fingerprint's position)
/// is exact and is taken when it agrees within that rounding, as the planner takes it.
pub fn outline(analysis: &Analysis, f: &Feature) -> Result<Outline, String> {
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
    let frame = frame(analysis);
    let world = |q: usize| {
        let mut d = [0.0; 3];
        d[q] = 1.0;
        principal(dir_to_part(&frame, d)).ok_or("its axes lie along no axis of the part")
    };
    let (long, wide) = (world(ql)?, world(qw)?);
    let deep = 3 - long - wide;
    let (width, length) = (
        parameters::size(f, "width")?,
        parameters::size(f, "length")?,
    );
    let (lo, hi) = (parameters::size(f, "lo")?, parameters::size(f, "hi")?);
    let w = parameters::size(f, "w_center")?;
    let (d0, d1) = (parameters::size(f, "d_lo")?, parameters::size(f, "d_hi")?);
    let point = |t: f64, d: f64| {
        let mut q = [0.0; 3];
        q[ql] = t;
        q[qw] = w;
        q[qd] = d;
        to_part(&frame, q)
    };
    let (a, b) = (point(lo, d0), point(hi, d1));
    let c = f.fingerprint.position;
    let exact = |k: usize, recorded: f64| {
        if (c[k] - recorded).abs() <= 0.01 {
            c[k]
        } else {
            recorded
        }
    };
    let along = exact(long, 0.5 * (a[long] + b[long]));
    Ok(Outline {
        long,
        wide,
        deep,
        ends: [along - 0.5 * length, along + 0.5 * length],
        centre: exact(wide, a[wide]),
        through: [a[deep].min(b[deep]), a[deep].max(b[deep])],
        width,
        end_radius: p.get("end_radius").and_then(Value::as_f64),
    })
}

/// The member slots of slot pattern *p*: the `slot` features inside its faces.
pub fn members<'a>(analysis: &'a Analysis, p: &Feature) -> Vec<&'a Feature> {
    analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot" && f.faces.iter().all(|i| p.faces.binary_search(i).is_ok()))
        .collect()
}

/// The slot patterns, and the lone slots: every `slot` feature not inside a pattern's faces.
fn split(analysis: &Analysis) -> (Vec<&Feature>, Vec<&Feature>) {
    let patterns: Vec<&Feature> = analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot_pattern")
        .collect();
    let lone = analysis
        .features
        .iter()
        .filter(|f| f.kind == "slot")
        .filter(|f| {
            !patterns
                .iter()
                .any(|p| f.faces.iter().all(|i| p.faces.binary_search(i).is_ok()))
        })
        .collect();
    (patterns, lone)
}

/// The lone slots' requirements.
pub fn wanted(analysis: &Analysis) -> Vec<Wanted<'_>> {
    let (_, lone) = split(analysis);
    let mut out = Vec::new();
    for f in lone {
        for (parameter, field) in [
            ("slot_width.length", "width"),
            ("slot_length.length", "length"),
        ] {
            let v = parameters::size(f, field);
            out.push(Wanted {
                feature: f,
                parameter: parameter.into(),
                callout: v
                    .as_ref()
                    .map_or(format!("its {field}"), |v| format!("{field} {}", fmt(*v))),
                values: v.map(|v| vec![v]),
            });
        }
        // An end radius quiddity did not prove (null) is no requirement: the ends may be square.
        if let Some(r) = f.parameters.get("end_radius").and_then(|v| v.as_f64()) {
            out.push(Wanted {
                feature: f,
                parameter: "slot_end_radius.radius".into(),
                values: Ok(vec![r]),
                callout: format!("R{}", fmt(r)),
            });
        }
        let at = location(analysis, f);
        out.push(Wanted {
            feature: f,
            parameter: "location_slot.length".into(),
            callout: at.as_ref().map_or("its location".into(), |(axis, v)| {
                format!("its low end {} from the part's {axis} edge", fmt(*v))
            }),
            values: at.map(|(_, v)| vec![v]),
        });
    }
    out
}

/// A slot record's centre in quiddity's frame (its `Slot.location`): mid-span along its long
/// axis, on its centre line across, mid-depth.
pub(crate) fn centre(f: &Feature) -> Result<[f64; 3], String> {
    let axis = |k: &str| {
        f.parameters
            .get(k)
            .and_then(Value::as_str)
            .and_then(|a| AXES.iter().position(|x| *x == a))
            .ok_or_else(|| format!("its record gives no {k}"))
    };
    let (long, wide) = (axis("long_axis")?, axis("width_axis")?);
    if long == wide {
        return Err("its long and width axes are the same".into());
    }
    let mut c = [0.0; 3];
    c[long] = (parameters::size(f, "lo")? + parameters::size(f, "hi")?) / 2.0;
    c[wide] = parameters::size(f, "w_center")?;
    c[3 - long - wide] = (parameters::size(f, "d_lo")? + parameters::size(f, "d_hi")?) / 2.0;
    Ok(c)
}

/// A slot pattern's requirements, or why it cannot be checked: its members (the record's `slots`,
/// by feature ID) unreadable or not identical, or the pattern not cut along the part's z.
pub fn pattern_wanted<'a>(
    analysis: &'a Analysis,
    p: &'a Feature,
) -> Result<Vec<Wanted<'a>>, String> {
    let ids: Vec<&str> = p
        .parameters
        .get("slots")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if ids.len() < 2 {
        return Err("its record names fewer than two member slots".into());
    }
    let members: Vec<&Feature> = ids
        .iter()
        .map(|id| {
            analysis
                .features
                .iter()
                .find(|f| f.id == *id && f.kind == "slot")
                .ok_or_else(|| format!("its member {id} is not a slot of the part"))
        })
        .collect::<Result<_, _>>()?;
    let first = members[0];
    let key = |f: &Feature| -> Result<(Value, Value, f64, f64, Option<f64>), String> {
        Ok((
            f.parameters.get("long_axis").cloned().unwrap_or_default(),
            f.parameters.get("width_axis").cloned().unwrap_or_default(),
            parameters::size(f, "width")?,
            parameters::size(f, "length")?,
            f.parameters.get("end_radius").and_then(Value::as_f64),
        ))
    };
    // [`centre`] checks that the slots' long and width axes are two distinct axes.
    centre(first)?;
    let (long0, wide0, width, length, end_radius) = key(first)?;
    for f in &members[1..] {
        let (long, wide, w, l, r) = key(f)?;
        let same = |a: f64, b: f64| (a - b).abs() < 1e-6;
        if (&long, &wide) != (&long0, &wide0)
            || !same(w, width)
            || !same(l, length)
            || r.is_some() != end_radius.is_some()
            || r.zip(end_radius).is_some_and(|(a, b)| !same(a, b))
        {
            return Err(format!(
                "its slots are not identical ({} and {})",
                first.id, f.id
            ));
        }
    }
    let frame = frame(analysis);
    // The slots' depth axis: quiddity's axis that is neither their long nor their width axis.
    let normal = AXES.map(|a| {
        let named = [&long0, &wide0].contains(&&Value::from(a));
        if named { 0.0 } else { 1.0 }
    });
    match principal(dir_to_part(&frame, normal)) {
        Some(2) => {}
        k => {
            return Err(format!(
                "it is cut along {}, and draftwright locates a slot pattern only when it is cut along z (`_parameter_ids`)",
                k.map_or("no axis of the part", |k| AXES[k])
            ));
        }
    }
    let centre = match p.parameters.get("center") {
        Some(c) => {
            parameters::place(&frame, Some(c)).ok_or("its record's centre is not a point")?
        }
        None => {
            let mut c = [0.0; 3];
            for f in &members {
                let at = to_part(&frame, centre(f)?);
                (0..3).for_each(|k| c[k] += at[k] / members.len() as f64);
            }
            c
        }
    };
    let want = |parameter: &str, values: Result<Vec<f64>, String>, callout: String| Wanted {
        feature: p,
        parameter: parameter.into(),
        values,
        callout,
    };
    let sized = |parameter: &str, v: Result<f64, String>, what: &str| {
        let callout = v
            .as_ref()
            .map_or(format!("its {what}"), |v| format!("{what} {}", fmt(*v)));
        want(parameter, v.map(|v| vec![v]), callout)
    };
    let mut out = vec![
        sized("slot_width.length", Ok(width), "member width"),
        sized("slot_length.length", Ok(length), "member length"),
    ];
    if let Some(r) = end_radius {
        out.push(want(
            "slot_end_radius.radius",
            Ok(vec![r]),
            format!("R{}", fmt(r)),
        ));
    }
    if p.parameters.get("row_pitch").is_some_and(|v| !v.is_null()) {
        for (name, field) in [("col", "col_pitch"), ("row", "row_pitch")] {
            out.push(sized(
                &format!("grid_pitch.length.{name}"),
                parameters::size(p, field),
                &format!("{name} pitch"),
            ));
        }
    } else {
        out.push(sized("pitch.length", parameters::size(p, "pitch"), "pitch"));
    }
    for k in [0, 1] {
        let d = centre[k] - analysis.envelope[0][k];
        out.push(want(
            &format!("location_slot_pattern.length.{}", AXES[k]),
            Ok(vec![d]),
            format!("its centre {} from the part's {} edge", fmt(d), AXES[k]),
        ));
    }
    Ok(out)
}

/// The slot coverage findings: the lone slots' requirements, each slot pattern's, and each slot
/// pattern whose requirements cannot be read as one requirement that cannot be checked.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let frame = frame(analysis);
    let noun = |f: &Feature| {
        let p = &f.parameters;
        let mut s = format!("slot {}", f.id);
        if let (Ok(w), Ok(l)) = (parameters::size(f, "width"), parameters::size(f, "length")) {
            s += &format!(" {} × {}", fmt(w), fmt(l));
        }
        if let Ok(r) = parameters::size(f, "end_radius") {
            s += &format!(", ends R{}", fmt(r));
        }
        if let Some(a) = parameters::direction(&frame, p.get("long_axis")) {
            s += &format!(", long along {}", fmt3(a));
        }
        s
    };
    let mut issues = parameters::coverage(
        "slot",
        &wanted(analysis),
        marks,
        noun,
        "a linear dimension or radius callout, in the view that sees the slot's outline",
    );
    let (patterns, _) = split(analysis);
    for p in patterns {
        let why = match pattern_wanted(analysis, p) {
            Ok(wanted) => {
                let noun = |f: &Feature| {
                    let mut s = format!("slot pattern {}", f.id);
                    if let Some(n) = f.parameters.get("slots").and_then(Value::as_array) {
                        s += &format!(" of {} slots", n.len());
                    }
                    s
                };
                issues.extend(parameters::coverage(
                    "slot",
                    &wanted,
                    marks,
                    noun,
                    "a grouped slot callout, pitch or location dimension, in the view that looks through the slots",
                ));
                continue;
            }
            Err(why) => why,
        };
        issues.push(LintIssue {
            sheet: None,
            id: format!("l:slot_requirement_unverifiable:{}/?", p.id),
            code: "slot_requirement_unverifiable".into(),
            severity: Severity::Warning,
            message: format!(
                "slot pattern {}: its requirements cannot be checked: {why}",
                p.id
            ),
            at: None,
            view: None,
            subjects: vec![p.id.clone()],
            requirements: vec![Requirement {
                feature: p.id.clone(),
                parameter: "?".into(),
            }],
            measurements: Vec::new(),
            sources: Vec::new(),
            stage: None,
            annotation: None,
            related: Vec::new(),
            evidence: Some(why),
            fix: None,
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Claim;
    use serde_json::{Value, json};

    fn part(features: &[(&str, &[usize], Value)]) -> Analysis {
        let features: Vec<Value> = features
            .iter()
            .enumerate()
            .map(|(i, (kind, faces, r))| {
                json!({
                    "id": format!("f:{kind}:{}", i + 1), "kind": kind, "faces": faces,
                    "parameters": r,
                    "fingerprint": { "version": "test", "family": "slots", "subtype": "Slot",
                                     "sizes": {}, "position": [0.0, 0.0, 0.0],
                                     "neighbourhood": [] },
                })
            })
            .collect();
        serde_json::from_value(json!({
            "schema": "draftwright.analysis/2",
            "binding": { "source_sha256": "", "part": "test", "face_count": 9, "loader": "test" },
            "envelope": [[0.0, 0.0, 0.0], [100.0, 100.0, 10.0]],
            "features": features,
            "iso_bounds": [0.0, 0.0, 0.0, 0.0],
        }))
        .unwrap()
    }

    fn slot(end_radius: Option<f64>) -> Value {
        json!({ "width": 6.0, "length": 14.97, "end_radius": end_radius, "long_axis": "z",
                "width_axis": "y", "lo": -110.93, "hi": -95.96, "w_center": 41.5,
                "d_lo": -13.74, "d_hi": -11.74 })
    }

    /// LINT-slot_coverage-lone_slot: width, length and location, and the end radius only when
    /// quiddity proved the ends are arcs. cgb245's slots, in its frame (quiddity's z is the
    /// part's x) and box (from x = -190): stated as `6`, `15.0`, `2× R3` and `79.1`, the low
    /// end's offset from the box along the long axis.
    #[test]
    fn a_lone_slot_states_its_width_length_location_and_proven_end_radius() {
        let mut a = part(&[("slot", &[0], slot(Some(3.0))), ("slot", &[1], slot(None))]);
        a.recognition = Some(
            serde_json::from_value(json!({
                "recogniser": "test",
                "frame": [[-0.000128, 7.68098, -3.64814], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0],
                          [1.0, 0.0, 0.0]],
            }))
            .unwrap(),
        );
        a.envelope = [[-190.000128, -84.0, -18.6], [190.0, 99.0, 6.6]];
        let w = wanted(&a);
        let names: Vec<String> = w
            .iter()
            .map(|w| format!("{} {}", w.feature.id, w.parameter))
            .collect();
        assert_eq!(
            names,
            [
                "f:slot:1 slot_width.length",
                "f:slot:1 slot_length.length",
                "f:slot:1 slot_end_radius.radius",
                "f:slot:1 location_slot.length",
                "f:slot:2 slot_width.length",
                "f:slot:2 slot_length.length",
                "f:slot:2 location_slot.length"
            ]
        );
        assert_eq!(
            location(&a, w[0].feature).map(|(k, v)| (k, fmt(v))),
            Ok(("x", "79.07".into()))
        );
        let mark = |text: &str, f: &str, p: &str| Mark {
            name: format!("m:{f}:{p}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: vec![Claim {
                feature: f.into(),
                parameter: p.into(),
            }],
        };
        let marks = [
            mark("6", "f:slot:1", "slot_width.length"),
            mark("15.0", "f:slot:1", "slot_length.length"),
            mark("2× R3", "f:slot:1", "slot_end_radius.radius"),
            mark("79.1", "f:slot:1", "location_slot.length"),
            mark("6", "f:slot:2", "slot_width.length"),
            mark("15.1", "f:slot:2", "slot_length.length"),
        ];
        let found: Vec<String> = coverage(&a, &marks)
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
            .collect();
        assert_eq!(
            found,
            [
                "slot_requirement_unverifiable slot_length.length",
                "slot_requirement_missing location_slot.length"
            ]
        );
        // A long axis along no axis of the part: the location cannot be checked.
        let first = w[0].feature.clone();
        a.recognition.as_mut().unwrap().frame[3] = [0.6, 0.8, 0.0];
        assert_eq!(
            location(&a, &first),
            Err("its long axis lies along no axis of the part".into())
        );
    }

    /// quiddity rounds a slot's `lo`, `hi` and `length` to 0.01 mm each, so its recorded `lo`
    /// (95.96) and the low end the planner draws (the exact centre of its faces, 103.444798, less
    /// half of 14.96: 95.9648) can fall either side of a 0.005 tie, as on cgb245. Lint judges the
    /// drawn end: `95.965` states the location and `95.960` (the recorded end) does not.
    #[test]
    fn a_slot_location_is_judged_at_the_end_the_planner_draws() {
        let mut a = part(&[(
            "slot",
            &[0],
            json!({ "width": 6.0, "length": 14.96, "end_radius": 3.0, "long_axis": "x",
                    "width_axis": "y", "lo": 95.96, "hi": 110.93, "w_center": 50.0,
                    "d_lo": 0.0, "d_hi": 10.0 }),
        )]);
        a.features[0].fingerprint.position = [103.444798, 50.0, 5.0];
        let f = &a.features[0];
        let (axis, at) = location(&a, f).unwrap();
        assert_eq!(axis, "x");
        assert_eq!(at, outline(&a, f).unwrap().ends[0] - a.envelope[0][0]);
        assert_eq!(fmt(at), "95.965");
        let unverified = |text: &str| -> Vec<String> {
            let marks = [Mark {
                name: "m:f:slot:1:location_slot.length".into(),
                view: Some("v:plan".into()),
                text: text.into(),
                claims: vec![Claim {
                    feature: "f:slot:1".into(),
                    parameter: "location_slot.length".into(),
                }],
            }];
            coverage(&a, &marks)
                .into_iter()
                .filter(|i| i.requirements[0].parameter == "location_slot.length")
                .map(|i| i.code)
                .collect()
        };
        assert!(unverified("95.965").is_empty());
        assert_eq!(unverified("95.960"), ["slot_requirement_unverifiable"]);
    }

    /// LINT-slot_coverage-slot_pattern: a linear array of three obround slots has its member's
    /// width, length and end radius, its pitch, and its centroid's x and y location as
    /// requirements (five and the radius); a grid has its column and row pitches instead of the
    /// pitch, and its centre is quiddity's. A pattern cut along anything but z, or of slots that
    /// are not identical, is one requirement that cannot be checked.
    #[test]
    fn a_slot_pattern_states_its_member_pitches_and_location() {
        let member = |w_center: f64| {
            json!({ "width": 6.0, "length": 20.0, "end_radius": 3.0, "long_axis": "x",
                    "width_axis": "y", "lo": 10.0, "hi": 30.0, "w_center": w_center,
                    "d_lo": 0.0, "d_hi": 10.0 })
        };
        let slots = json!(["f:slot:1", "f:slot:2", "f:slot:3"]);
        let array = json!({ "slots": slots, "pitch": 25.0, "direction": [0.0, 1.0, 0.0] });
        let mut a = part(&[
            ("slot", &[0], member(20.0)),
            ("slot", &[1], member(45.0)),
            ("slot", &[2], member(70.0)),
            ("slot_pattern", &[0, 1, 2], array),
        ]);
        let wanted = |a: &Analysis| -> Result<Vec<String>, String> {
            let p = a
                .features
                .iter()
                .find(|f| f.kind == "slot_pattern")
                .unwrap();
            Ok(pattern_wanted(a, p)?
                .iter()
                .map(|w| format!("{} {:?}", w.parameter, w.values.as_ref().unwrap()))
                .collect())
        };
        assert_eq!(
            wanted(&a).unwrap(),
            [
                "slot_width.length [6.0]",
                "slot_length.length [20.0]",
                "slot_end_radius.radius [3.0]",
                "pitch.length [25.0]",
                "location_slot_pattern.length.x [20.0]",
                "location_slot_pattern.length.y [45.0]"
            ]
        );
        let mark = |text: &str, ps: &[&str]| Mark {
            name: format!("m:{text}"),
            view: Some("v:plan".into()),
            text: text.into(),
            claims: ps
                .iter()
                .map(|p| Claim {
                    feature: "f:slot_pattern:4".into(),
                    parameter: p.to_string(),
                })
                .collect(),
        };
        let mut marks = vec![
            mark(
                "3× SLOT 6 × 20",
                &["slot_width.length", "slot_length.length"],
            ),
            mark("2× R3", &["slot_end_radius.radius"]),
            mark("2× 25", &["pitch.length"]),
            mark("20", &["location_slot_pattern.length.x"]),
            mark("45", &["location_slot_pattern.length.y"]),
        ];
        let found = |a: &Analysis, marks: &[Mark]| -> Vec<String> {
            coverage(a, marks)
                .into_iter()
                .map(|i| format!("{} {}", i.code, i.requirements[0].parameter))
                .collect()
        };
        assert!(found(&a, &marks).is_empty(), "{:?}", found(&a, &marks));
        marks[2].text = "2× 24".into();
        marks.remove(4);
        assert_eq!(
            found(&a, &marks),
            [
                "slot_requirement_unverifiable pitch.length",
                "slot_requirement_missing location_slot_pattern.length.y"
            ]
        );

        // A 3×1 grid about (20, 45): the column and row pitches.
        a.features[3].parameters = json!({ "slots": slots, "rows": 3, "cols": 1, "angle": 0.0,
                                           "col_pitch": 30.0, "row_pitch": 25.0,
                                           "center": [20.0, 45.0, 5.0] })
        .as_object()
        .unwrap()
        .clone();
        let names: Vec<String> = wanted(&a).unwrap()[3..].to_vec();
        assert_eq!(
            names,
            [
                "grid_pitch.length.col [30.0]",
                "grid_pitch.length.row [25.0]",
                "location_slot_pattern.length.x [20.0]",
                "location_slot_pattern.length.y [45.0]"
            ]
        );

        // Slots that differ, or a pattern cut along y.
        a.features[1].parameters["length"] = json!(21.0);
        assert_eq!(
            wanted(&a),
            Err("its slots are not identical (f:slot:1 and f:slot:2)".into())
        );
        for f in &mut a.features[..3] {
            f.parameters["length"] = json!(20.0);
            f.parameters["width_axis"] = json!("z");
        }
        assert!(wanted(&a).unwrap_err().starts_with("it is cut along y"));
        assert_eq!(found(&a, &marks), ["slot_requirement_unverifiable ?"]);
    }

    /// A pattern's members are the pattern's: no lone-slot requirement, and a pattern whose
    /// requirements cannot be read (here, it names no member slots) is reported unverifiable
    /// rather than left out.
    #[test]
    fn a_slot_patterns_members_are_not_lone_slots() {
        let a = part(&[
            ("slot", &[0, 1], slot(None)),
            ("slot", &[2, 3], slot(None)),
            ("slot", &[5], slot(None)),
            ("slot_pattern", &[0, 1, 2, 3], json!({ "pitch": 20.0 })),
        ]);
        let found: Vec<String> = coverage(&a, &[])
            .into_iter()
            .map(|i| format!("{} {}", i.code, i.subjects[0]))
            .collect();
        assert_eq!(
            found,
            [
                "slot_requirement_missing f:slot:3",
                "slot_requirement_missing f:slot:3",
                "slot_requirement_missing f:slot:3",
                "slot_requirement_unverifiable f:slot_pattern:4"
            ]
        );
    }
}
