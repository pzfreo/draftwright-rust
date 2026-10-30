//! Deliberately bounded catalogue-shape guesses, using quiddity's recognised features.
use haecceity::Part;
use quiddity::recognition::{EvidenceStatus, RecognitionDocument};
use serde_json::Value;

pub(super) fn custom(name: &str) -> bool {
    let token = name.split_whitespace().next().unwrap_or_default();
    token.split_once('-').is_some_and(|(prefix, number)| {
        prefix.len() >= 2
            && prefix.chars().all(|c| c.is_ascii_alphabetic())
            && !["ISO", "DIN", "BS", "ANSI"].contains(&prefix.to_uppercase().as_str())
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
    })
}

pub(super) fn name_reason(name: &str) -> Option<String> {
    if custom(name) {
        return None;
    }
    let words = name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), " ");
    words
        .split_whitespace()
        .find(|word| {
            [
                "bolt",
                "bolts",
                "screw",
                "screws",
                "washer",
                "washers",
                "nut",
                "nuts",
                "bearing",
                "bearings",
                "dowel",
                "dowels",
                "setscrew",
                "setscrews",
            ]
            .contains(word)
        })
        .map(|word| format!("name: {word}"))
}

fn number(value: &Value, field: &str) -> f64 {
    value[field]
        .as_f64()
        .filter(|v| v.is_finite())
        .unwrap_or(f64::NAN)
}

fn point(value: &Value, field: &str) -> Option<[f64; 2]> {
    let a = value[field].as_array()?;
    let p = [a.first()?.as_f64()?, a.get(1)?.as_f64()?];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

/// A complete turned profile proves there are no slots, steps, holes or other features
/// beyond the explicit allowed segments. Tiny zero-sweep seam segments are ignored.
pub(super) fn shape_reason(part: &Part, doc: &RecognitionDocument) -> Option<String> {
    if part.solids.len() != 1 || !part.unresolved_faces().is_empty() {
        return None;
    }
    for record in doc.records.iter().filter(|r| {
        r.evidence_status == EvidenceStatus::Verified
            && r.family == "turned_bodies"
            && r.faces.len() == part.faces.len()
    }) {
        let p = serde_json::to_value(&record.parameters).ok()?;
        let (diameter, length) = (number(&p, "od"), number(&p, "length"));
        if !(0.5..=100.0).contains(&diameter) || length <= 0.0 || !length.is_finite() {
            continue;
        }
        let bores = p["bores"].as_array()?;
        let profile: Vec<_> = p["profile"]
            .as_array()?
            .iter()
            .filter(|s| {
                // Kernel torus seams can publish a second segment with zero arc sweep.
                s["kind"] != "torus" || number(s, "arc_sweep").abs() > 1e-7
            })
            .collect();
        let planes: Vec<_> = profile
            .iter()
            .filter(|s| s["kind"] == "plane_shoulder")
            .collect();
        let cylinders: Vec<_> = profile.iter().filter(|s| s["kind"] == "cylinder").collect();
        if profile.len() == 4
            && planes.len() == 2
            && cylinders.len() == 2
            && cylinders.iter().filter(|s| s["side"] == "external").count() == 1
            && cylinders.iter().filter(|s| s["side"] == "internal").count() == 1
            && bores.len() == 1
            && length >= 0.05
            && length <= diameter * 0.25
        {
            let bore = number(&bores[0], "diameter");
            if bore >= 0.2 * diameter
                && bore <= 0.9 * diameter
                && point(&bores[0], "span")
                    .is_some_and(|span| super::close(span[1] - span[0], length))
            {
                return Some("shape: plain thin-annulus washer".into());
            }
        }
        // A dowel needs a single constant-diameter barrel, two solid end planes and
        // one small chamfer/round at EACH end. Plain pins and stepped shafts stay made.
        if profile.len() == 5
            && planes.len() == 2
            && cylinders.len() == 1
            && bores.is_empty()
            && diameter <= 50.0
            && (1.0..=10.0).contains(&(length / diameter))
            && cylinders[0]["side"] == "external"
        {
            let barrel = (point(cylinders[0], "start"), point(cylinders[0], "end"));
            let ends: Vec<_> = profile
                .iter()
                .filter(|s| s["kind"] == "cone" || s["kind"] == "torus")
                .collect();
            if let (Some(lo), Some(hi)) = barrel
                && super::close(lo[1] * 2.0, diameter)
                && super::close(hi[1], lo[1])
                && ends.len() == 2
                && ends.iter().all(|s| s["side"] == "external")
            {
                let matches = |end: &&&Value, at: f64| {
                    let (Some(a), Some(b)) = (point(end, "start"), point(end, "end")) else {
                        return false;
                    };
                    let axial = (a[0] - b[0]).abs();
                    let radial = (a[1] - b[1]).abs();
                    axial > 1e-5
                        && axial <= diameter * 0.2
                        && radial > 1e-5
                        && radial <= diameter * 0.2
                        && (a[0] - at).abs().min((b[0] - at).abs()) <= super::DISTANCE
                        && planes.iter().any(|plane| {
                            point(plane, "end").is_some_and(|p| {
                                (p[0] - a[0]).abs().min((p[0] - b[0]).abs()) <= super::DISTANCE
                                    && p[1] > 0.0
                                    && p[1] < diameter / 2.0
                            })
                        })
                };
                if ends.iter().any(|end| matches(end, lo[0]))
                    && ends.iter().any(|end| matches(end, hi[0]))
                {
                    return Some("shape: short solid dowel with two treated ends".into());
                }
            }
        }
    }
    threaded_fastener(doc)
}

fn threaded_fastener(doc: &RecognitionDocument) -> Option<String> {
    let threads: Vec<_> = doc
        .records
        .iter()
        .filter(|r| {
            r.evidence_status == EvidenceStatus::Verified
                && r.family == "modelled_threads"
                && r.parameters["side"] == "external"
        })
        .collect();
    if threads.len() != 1 {
        return None;
    }
    let t = serde_json::to_value(&threads[0].parameters).ok()?;
    let (d, length, pitch) = (
        number(&t, "major_diameter"),
        number(&t, "length"),
        number(&t, "pitch"),
    );
    if !(1.0..=30.0).contains(&d)
        || !(0.5..=12.0).contains(&(length / d))
        || !(0.05..=0.4).contains(&(pitch / d))
        || t["starts"] != 1
    {
        return None;
    }
    let vector =
        |v: &Value| -> Option<[f64; 3]> { Some([v[0].as_f64()?, v[1].as_f64()?, v[2].as_f64()?]) };
    let direction = vector(&t["axis"]["direction"])?;
    let origin = vector(&t["axis"]["point"])?;
    // This bounded rule accepts axis-aligned heads/drives from the recognised turned-step
    // and prismatic-pocket families. Oblique/unknown head evidence remains make.
    let axis = (0..3).find(|&i| (direction[i].abs() - 1.0).abs() < 1e-6)?;
    let axis_name = ["x", "y", "z"][axis];
    let near_end = |a: f64| {
        (a - number(&t, "start"))
            .abs()
            .min((a - number(&t, "end")).abs())
            <= d
    };
    for head in doc
        .records
        .iter()
        .filter(|r| r.evidence_status == EvidenceStatus::Verified && r.family == "turned_steps")
    {
        let h = serde_json::to_value(&head.parameters).ok()?;
        let (hd, lo, hi) = (number(&h, "diameter"), number(&h, "lo"), number(&h, "hi"));
        let head_origin = vector(&h["profile"]["axis_origin"])?;
        if h["axis"] != axis_name
            || !(1.3..=2.5).contains(&(hd / d))
            || !(0.2..=1.5).contains(&((hi - lo) / d))
            || (0..3).any(|i| i != axis && (head_origin[i] - origin[i]).abs() > super::DISTANCE)
            || !near_end(lo * direction[axis]) && !near_end(hi * direction[axis])
        {
            continue;
        }
        for drive in doc.records.iter().filter(|r| {
            r.evidence_status == EvidenceStatus::Verified && r.family == "prismatic_pockets"
        }) {
            let p = serde_json::to_value(&drive.parameters).ok()?;
            let at = vector(&p["at"])?;
            if p["axis"] == axis_name
                && p["sides"] == 6
                && number(&p, "depth") > 0.0
                && number(&p, "depth") <= hi - lo
                && at[axis] >= lo
                && at[axis] <= hi
                && (0..3).all(|i| i == axis || (at[i] - origin[i]).abs() <= super::DISTANCE)
            {
                return Some("shape: externally threaded headed fastener with hex drive".into());
            }
        }
    }
    None
}
