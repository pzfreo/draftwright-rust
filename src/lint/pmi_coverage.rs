//! PMI coverage (Python's `linting/pmi_coverage.py`, `_lint_pmi`): every AP242 PMI source of the
//! part is on the drawing or reported, and a drawn feature control frame states its source.
//!
//! The requirements are the Analysis's PMI reading ([`Reading`]): its records and the sources it
//! did not read. The evidence is the drawing's marks: a record is drawn when an annotation is named
//! `pmi:<source>` (a feature control frame, datum feature symbol, label or drawing-wide
//! requirement) or claims a measurement of feature `pmi:<source>` (a dimension stating a PMI
//! dimension). Which records lower to a drawing item at all is [`lower`]'s decision, a function of
//! the Analysis alone that drawing the part also takes; lint asks it only to tell a record not
//! lowered from one lowered but not drawn, never for what to count. In the order Python reports
//! them:
//!
//! - `pmi_not_extracted`: the reading was refused (`AP242 PMI could not be inventoried`), or a
//!   source was not read or read in part (`partially_extracted`: a record with blockers). An
//!   error, as Python's `annotate` mode makes it; a source of no draftwright category (a shape
//!   aspect, property or standard haecceity reports, which draftwright's census does not hold)
//!   is info, as the plan reports it. A material the title block states is drawn, not reported.
//! - `pmi_not_lowered`: a record that is not drawn and that [`lower`] does not lower: its reading's
//!   blockers, its modifiers or its attachment stop it, a surface or common label has no proven
//!   site, a drawing-wide requirement ([`sheet_requirements`]) is ambiguous or not a supported
//!   default, a thread or knurl no one feature owns ([`manufacturing`]: its text disagrees with
//!   its cylinder, or no feature or several prove it), or it is a face finish (read, not drawn
//!   yet); a dimension that does not lower ([`dimension`]). Error; a manufacturing requirement of
//!   a kind draftwright does not support is a warning.
//! - `pmi_not_rendered`: a record [`lower`] or [`sheet_requirements`] lowers, a thread or knurl
//!   [`manufacturing`] gives to its owner (its owner's callout claims it), or a dimension that
//!   lowers, that no annotation draws or states. Error.
//! - `pmi_value_mismatch`: a frame whose text runs do not state its source's magnitude (one
//!   number, within half the display's 13th significant digit, as Python allows), its zone
//!   (`⌀`, `S⌀` or none) and its datum letters (repeats removed). Error.
//! - `pmi_source_unknown`: an annotation that claims a source the reading does not hold (a
//!   fabricated provenance); `pmi_unreconciled`: one that claims a source when the part has no
//!   PMI reading to check it against (warning).
//!
//! - `step_material_ambiguous`: the parts state more than one material, so none can be the
//!   title block's; `step_material_unavailable`: the one material stated is one part's of
//!   several (draftwright's 'not owned by the single shaped source product definition').
//!   Warnings.
//!
//! Python's `pmi_present_but_ignored` is not-applicable: the port has no switch that ignores a
//! part's PMI. Its other codes wait on the drawing or on a decision: of the manufacturing notes,
//! finish symbols and requirement table that `pmi_source_text_mismatch`, `pmi_source_site_mismatch`
//! and `manufacturing_reference_unresolved` check, the port draws the drawing-wide notes, the
//! default finish and the threads and knurls (on their callouts or in their table), not a face's
//! finish, and now checks edge-note and face-finish text runs independently of their formatter.
//! The filled glyph-path check remains pending; and the title block's material
//! and general tolerance against the file (`step_material_mismatch` and `_disagreement`,
//! `step_general_tolerance_*`) turn on whether the caller authored the field, which only the Spec
//! says.

use std::collections::BTreeSet;

use super::{LintIssue, Mark, Severity};
use crate::model::Analysis;
use crate::pmi::{Category, Identities, Outcome, Reading, Record, Report};
use crate::pmi_lowering::{lower, manufacturing, sheet_requirements};

/// Python's `_SUPPORTED_MANUFACTURING_REQUIREMENTS`.
const MANUFACTURING: [&str; 5] = [
    "external_thread",
    "internal_thread",
    "knurl",
    "edge_condition",
    "surface_finish",
];

fn issue(code: &str, subject: &str, severity: Severity, message: String) -> LintIssue {
    LintIssue {
        sheet: None,
        id: format!("l:{code}:{subject}"),
        code: code.into(),
        severity,
        message,
        at: None,
        view: None,
        subjects: Vec::new(),
        requirements: Vec::new(),
        measurements: Vec::new(),
        sources: Vec::new(),
        stage: None,
        annotation: None,
        related: Vec::new(),
        evidence: None,
        fix: None,
    }
}

/// A source's issue: named by it and implicating it.
fn sourced(code: &str, source: &str, severity: Severity, message: String) -> LintIssue {
    LintIssue {
        sources: vec![source.to_string()],
        ..issue(code, source, severity, message)
    }
}

fn category_name(c: Category) -> &'static str {
    match c {
        Category::Dimension => "dimension",
        Category::GeometricTolerance => "geometric_tolerance",
        Category::Datum => "datum",
        Category::ManufacturingRequirement => "manufacturing_requirement",
        Category::SurfaceLabel => "surface_label",
        Category::Other => "other",
    }
}

/// The PMI sources the marks draw or state: each `pmi:<id>` annotation name and claim, read as
/// the source of the record whose item ID it is (a record's ID is its semantic identity,
/// [`crate::pmi::Report::identities`]); any other (a material's census source, an ID no record
/// has) as it is.
fn drawn<'a>(marks: &'a [Mark], ids: &'a Identities) -> BTreeSet<&'a str> {
    let source = |name: &'a str| {
        name.strip_prefix("pmi:")
            .map(|s| ids.source(name).unwrap_or(s))
    };
    let mut out = BTreeSet::new();
    for m in marks {
        out.extend(source(&m.name));
        for c in &m.claims {
            out.extend(source(&c.feature));
        }
    }
    out
}

/// The PMI coverage findings on a drawing whose annotations are *marks*.
pub fn coverage(analysis: &Analysis, marks: &[Mark]) -> Vec<LintIssue> {
    let ids = match &analysis.pmi {
        Some(Reading::Read(r)) => r.identities(),
        _ => Identities::default(),
    };
    let on_sheet = drawn(marks, &ids);
    let report = match &analysis.pmi {
        Some(Reading::Read(r)) => r,
        Some(Reading::Refused(why)) => {
            let mut out = vec![issue(
                "pmi_not_extracted",
                "part",
                Severity::Error,
                format!("AP242 PMI could not be inventoried: {why}"),
            )];
            out.extend(unreconciled(&on_sheet));
            return out;
        }
        None => return unreconciled(&on_sheet),
    };
    let mut out = Vec::new();
    // A material the title block states (its cell's run named `pmi:material:<part>:0`) is on the
    // drawing, as draftwright's title block accounts for its material fact.
    for s in report.sources.iter().filter(|s| {
        matches!(
            s.outcome,
            Outcome::NotExtracted | Outcome::PartiallyExtracted
        ) && !(s.id.starts_with("material:") && on_sheet.contains(s.id.as_str()))
    }) {
        let severity = if s.category == Category::Other {
            Severity::Info
        } else {
            Severity::Error
        };
        out.push(sourced(
            "pmi_not_extracted",
            &s.id,
            severity,
            format!(
                "AP242 {} {} was discovered but not extracted: {}",
                category_name(s.category),
                s.id,
                s.reason
            ),
        ));
    }
    // Lowering does not depend on which views are drawn, only where an item goes. A drawing-wide
    // requirement lowers to the sheet (the title block, the finish symbol, a general note).
    let (items, mut why_not) = lower(analysis, &|_| true);
    let (sheet, more) = sheet_requirements(analysis);
    why_not.extend(more);
    let mut lowered: BTreeSet<&str> = items.iter().map(|l| l.source.as_str()).collect();
    lowered.extend(sheet.iter().map(|r| r.source.as_str()));
    // A thread or knurl given to its owner lowers to that owner's callout.
    let manufacturing = manufacturing(analysis);
    lowered.extend(manufacturing.owned.iter().map(|o| o.source.as_str()));
    lowered.extend(
        report
            .records
            .iter()
            .filter(|r| r.category == Category::Dimension && dimension(r).is_ok())
            .map(|r| r.source_id.as_str()),
    );
    // A record whose identity another shares has no drawing item to lower to.
    lowered.retain(|s| ids.ids.contains_key(*s));
    for r in &report.records {
        let id = r.source_id.as_str();
        if on_sheet.contains(id) || lowered.contains(id) {
            continue;
        }
        let reason = match dimension(r) {
            Err(why) if r.category == Category::Dimension && r.kind != "common_label" => why,
            _ => why_not
                .iter()
                .find(|p| p.refs.first().is_some_and(|s| s == id))
                .map_or_else(
                    || {
                        if r.blockers.is_empty() {
                            "it is not lowered to a drawing item".to_string()
                        } else {
                            r.blockers.join("; ")
                        }
                    },
                    |p| p.message.clone(),
                ),
        };
        let severity = if r.category == Category::ManufacturingRequirement
            && !MANUFACTURING.contains(&r.kind.as_str())
        {
            Severity::Warning
        } else {
            Severity::Error
        };
        out.push(sourced(
            "pmi_not_lowered",
            id,
            severity,
            format!("AP242 source {id} did not produce a drawing item: {reason}"),
        ));
    }
    for r in &report.records {
        let id = r.source_id.as_str();
        if lowered.contains(id) && !on_sheet.contains(id) {
            out.push(sourced(
                "pmi_not_rendered",
                id,
                Severity::Error,
                format!(
                    "AP242 source {id} reached a drawing item ({}) but produced no annotation",
                    ids.ids[id]
                ),
            ));
        }
    }
    for m in marks {
        let Some(id) = m.name.strip_prefix("pmi:") else {
            continue;
        };
        let id = ids.source(&m.name).unwrap_or(id);
        if let Some(r) = report.records.iter().find(|r| {
            r.source_id == id
                && r.category == Category::GeometricTolerance
                && r.value.is_some_and(|v| v > 0.0)
        }) && let Some(found) = mismatch(r, &m.text)
        {
            let mut i = sourced("pmi_value_mismatch", id, Severity::Error, found);
            i.id = format!("l:pmi_value_mismatch:{}", m.name);
            i.annotation = Some(m.name.clone());
            out.push(i);
        }
    }
    out.extend(manufacturing_text(report, marks, &ids));
    out.extend(materials(report));
    let known: BTreeSet<&str> = report
        .records
        .iter()
        .map(|r| r.source_id.as_str())
        .chain(report.sources.iter().map(|s| s.id.as_str()))
        .collect();
    for id in on_sheet.iter().filter(|s| !known.contains(*s)) {
        out.push(sourced(
            "pmi_source_unknown",
            id,
            Severity::Error,
            format!(
                "a declaration claims AP242 source {id}, which the reconciled document does not \
                 contain — the provenance is fabricated, not merely unmatched"
            ),
        ));
    }
    out
}

/// Source-scoped manufacturing text, read from the source itself, independently of the
/// lowering/row formatter. Whitespace may change when notes wrap; words and parameters may not.
/// This checks the text runs. The independent audit of the filled glyph paths remains pending.
fn manufacturing_text(report: &Report, marks: &[Mark], ids: &Identities) -> Vec<LintIssue> {
    let mut out = Vec::new();
    for m in marks {
        let Some(source) = m.name.strip_prefix("pmi:") else {
            continue;
        };
        let source = ids.source(&m.name).unwrap_or(source);
        let Some(r) = report
            .records
            .iter()
            .find(|r| r.source_id == source && r.category == Category::ManufacturingRequirement)
        else {
            continue;
        };
        let words: Vec<&str> = m.text.split_whitespace().collect();
        let states = match r.kind.as_str() {
            "edge_condition" => {
                // The general notes block numbers each note's first row. Its annotation owns
                // only that note's runs, including every wrapped continuation row.
                let note = match words.first().and_then(|s| s.parse::<usize>().ok()) {
                    Some(n) if n > 0 => &words[1..],
                    _ => &words[..],
                };
                !note.is_empty() && note == r.label.split_whitespace().collect::<Vec<_>>()
            }
            "surface_finish" => {
                let parameters: Vec<_> = r
                    .fields
                    .iter()
                    .filter_map(|(name, field)| match field {
                        crate::pmi::Field::Length(mm) => Some((name.as_str(), mm * 1000.0)),
                        _ => None,
                    })
                    .collect();
                // ISO 1302's parameter labels carry values in micrometres. An arbitrary
                // number somewhere in a label cannot stand for the named parameter.
                let mut printed = words.iter().copied();
                let states = !parameters.is_empty()
                    && parameters.iter().all(|(name, expected)| {
                        name.split_whitespace()
                            .all(|word| printed.next() == Some(word))
                            && expected.is_finite()
                            && printed
                                .next()
                                .and_then(|word| word.parse::<f64>().ok())
                                .is_some_and(|value| value.is_finite() && value == *expected)
                    });
                states && printed.next().is_none()
            }
            _ => continue,
        };
        if !states {
            let mut i = sourced(
                "pmi_source_text_mismatch",
                source,
                Severity::Error,
                format!(
                    "{} does not state AP242 {} {} as {:?}",
                    m.name,
                    r.kind.replace('_', " "),
                    source,
                    r.label
                ),
            );
            i.id = format!("l:pmi_source_text_mismatch:{}", m.name);
            i.annotation = Some(m.name.clone());
            i.view = m.view.clone();
            out.push(i);
        }
    }
    out
}

/// Whether dimension record *r* lowers to a drawing item (draftwright's typed dimension), or why
/// not: a function of the record alone. The kinds stage 2b (`dims::pmi_dimensions`) draws, a length
/// along x, y or z between two proven stations, a diameter or radius seen end-on or an angle a view
/// shows true, lower; a record its reading or its faces block, a common label (which [`lower`]
/// lowers as a label), an angle no view shows true and an oblique or curved quantity do not
/// (draftwright checks the faces when it extracts, so it says `pmi_not_extracted` where the port's
/// reading holds the record and lowering refuses it). Where a lowered dimension then goes, a model
/// dimension that states it or its own, is the drawing's; one the drawing does not state is
/// `pmi_not_rendered`.
fn dimension(r: &Record) -> Result<(), String> {
    if !r.blockers.is_empty() {
        return Err(r.blockers.join("; "));
    }
    if r.kind == "common_label" {
        return Err("it is a common label, text on its faces, which lowers as a label".into());
    }
    let Some(g) = &r.geometry else {
        return Err("its attachment was not derived".into());
    };
    if !g.reasons.is_empty() {
        return Err(g.reasons.join("; "));
    }
    if r.value.is_none() {
        return Err("the file states no nominal value".into());
    }
    match (r.kind.as_str(), &g.axis) {
        ("linear" | "thickness" | "diameter" | "radius", Some(_)) => Ok(()),
        ("linear" | "thickness", None) => Err(
            "its stations are oblique to the views' axes; the port draws a length along x, y or z"
                .into(),
        ),
        ("diameter" | "radius", None) => Err(
            "its axis is not along x, y or z; the port draws a diameter or radius seen end-on"
                .into(),
        ),
        // As the plan's `pmi_dimensions` decides: drawn where a view shows the angle true.
        ("angular", _) => match g.angle.as_ref().map(|a| a.principal()) {
            Some(Some(_)) => Ok(()),
            Some(None) => Err("supported angular ink requires explicit coplanar rays in a principal view: the plane of its sides is oblique to every view, so none shows the angle true".into()),
            None => Err("the sides of its angle were not derived (analyse the part again)".into()),
        },
        (kind, _) => Err(format!(
            "the port draws no {} dimension",
            kind.replace('_', " ")
        )),
    }
}

/// Python's `lint_step_title_defaults`, as far as the reading decides it: whether the file
/// states one material a title block may take. Several are ambiguous; one stated by one part of
/// several is not the drawing's.
fn materials(report: &Report) -> Vec<LintIssue> {
    match report.materials.as_slice() {
        [] => Vec::new(),
        [m] if !report.owns(m) => vec![sourced(
            "step_material_unavailable",
            &m.source_id,
            Severity::Warning,
            format!(
                "STEP material {} is unusable: material is not owned by the single shaped source \
                 product definition (it is part {}'s, of {} parts)",
                m.source_id,
                m.part,
                report.parts.len()
            ),
        )],
        [m] if m.designation.trim().is_empty() => vec![sourced(
            "step_material_unavailable",
            &m.source_id,
            Severity::Warning,
            format!(
                "STEP material {} is unusable: material designation is empty",
                m.source_id
            ),
        )],
        [_] => Vec::new(),
        several => vec![LintIssue {
            sources: several.iter().map(|m| m.source_id.clone()).collect(),
            ..issue(
                "step_material_ambiguous",
                "part",
                Severity::Warning,
                "STEP contains multiple material properties; no material was selected".into(),
            )
        }],
    }
}

/// Annotations claiming PMI sources on a part with no PMI reading to check them against.
fn unreconciled(on_sheet: &BTreeSet<&str>) -> Vec<LintIssue> {
    if on_sheet.is_empty() {
        return Vec::new();
    }
    vec![LintIssue {
        sources: on_sheet.iter().map(|s| s.to_string()).collect(),
        ..issue(
            "pmi_unreconciled",
            "part",
            Severity::Warning,
            format!(
                "this drawing declares {} AP242 source reference(s), and no AP242 census was \
                 available to check them — the claims are unverified, not satisfied",
                on_sheet.len()
            ),
        )
    }]
}

/// What a frame's compartments *text* (its runs, joined by spaces: `⌖ ⌀0.05Ⓜ A B`) state
/// that differs from record *r*, or none when they state its magnitude, zone and datums.
fn mismatch(r: &Record, text: &str) -> Option<String> {
    let expected = r.value?;
    let zone_of = |r: &Record| {
        ["spherical_diameter_zone", "diameter_zone"]
            .into_iter()
            .find(|z| r.gtol_modifiers.iter().any(|m| m == z))
    };
    let expected_zone = zone_of(r);
    let mut datums: Vec<&str> = Vec::new();
    for d in &r.datum_refs {
        if !datums.contains(&d.as_str()) {
            datums.push(d);
        }
    }
    // The compartments after the characteristic's symbol: one states the zone, the magnitude
    // and the material condition, the rest a datum letter each.
    let mut values = Vec::new();
    let mut zones = Vec::new();
    let mut letters = Vec::new();
    for cell in text.split_whitespace().skip(1) {
        let (zone, rest) = if let Some(rest) = cell.strip_prefix("S⌀") {
            (Some("spherical_diameter_zone"), rest)
        } else if let Some(rest) = cell.strip_prefix('⌀') {
            (Some("diameter_zone"), rest)
        } else {
            (None, cell)
        };
        let number = rest.trim_end_matches(['Ⓜ', 'Ⓛ']);
        match number.parse::<f64>() {
            Ok(v) if v.is_finite() => {
                values.push(v);
                zones.push(zone);
            }
            _ => letters.push(cell),
        }
    }
    // Python's display bound: half a unit of the 13th significant digit of the source value.
    let bound = 0.5 * 10f64.powi(expected.abs().log10().floor() as i32 - 12);
    let zone = zones.first().copied().flatten();
    if values.len() == 1
        && (values[0] - expected).abs() <= bound
        && zones.len() == 1
        && zone == expected_zone
        && letters == datums
    {
        return None;
    }
    let stated = if values.is_empty() {
        "no numeric value".to_string()
    } else {
        values
            .iter()
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let list = |v: &[&str]| {
        if v.is_empty() {
            "none".to_string()
        } else {
            v.join(" ")
        }
    };
    Some(format!(
        "pmi:{} states {stated} with zone {} and datums {} for AP242 source {}; source tolerance \
         is {expected} with zone {} and datums {}",
        r.source_id,
        zone.unwrap_or("none"),
        list(&letters),
        r.source_id,
        expected_zone.unwrap_or("none"),
        list(&datums)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: f64, modifiers: &[&str], datums: &[&str]) -> Record {
        let mut r: Record = serde_json::from_value(serde_json::json!({
            "source_id": "geometric_tolerance:#1", "category": "geometric_tolerance",
            "kind": "position", "part": 0, "part21_id": 1, "label": "", "dominant_axis": "Z",
            "datum_refs": [], "gtol_modifiers": [], "basic": false, "faces": [], "face_ids": [],
            "face_groups": [], "edges": [], "instances": [], "blockers": [],
        }))
        .unwrap();
        r.value = Some(value);
        r.gtol_modifiers = modifiers.iter().map(|s| s.to_string()).collect();
        r.datum_refs = datums.iter().map(|s| s.to_string()).collect();
        r
    }

    #[test]
    fn a_dimension_lowers_when_its_kind_and_faces_are_drawn() {
        let mut r = frame(30.0, &[], &[]);
        r.category = Category::Dimension;
        r.kind = "linear".into();
        r.geometry = Some(crate::pmi::Geometry {
            axis: Some("X".into()),
            ..Default::default()
        });
        assert_eq!(dimension(&r), Ok(()));
        let mut oblique = r.clone();
        oblique.geometry.as_mut().unwrap().axis = None;
        assert!(dimension(&oblique).unwrap_err().contains("oblique"));
        let mut unproven = r.clone();
        unproven.geometry.as_mut().unwrap().reasons =
            vec!["linear reference groups occupy the same station".into()];
        assert_eq!(
            dimension(&unproven),
            Err("linear reference groups occupy the same station".into())
        );
        let mut blocked = r.clone();
        blocked.blockers = vec!["the dimension modifier Reference is not carried".into()];
        assert!(dimension(&blocked).unwrap_err().contains("Reference"));
        let mut label = r.clone();
        label.kind = "common_label".into();
        assert!(dimension(&label).unwrap_err().contains("common label"));
        // An angle lowers where a view shows it true, as the plan draws it.
        let mut angle = r.clone();
        angle.kind = "angular".into();
        assert!(dimension(&angle).unwrap_err().contains("not derived"));
        // The sides as the Analysis carries them (the lint names no planner type, ADR1-9).
        let sides = |second: [f64; 3]| {
            serde_json::from_value(serde_json::json!({
                "vertex": [0.0, 0.0, 0.0], "first": [1.0, 0.0, 0.0], "second": second,
            }))
            .unwrap()
        };
        angle.geometry.as_mut().unwrap().angle = Some(sides([0.5, 0.866, 0.0]));
        assert_eq!(dimension(&angle), Ok(()));
        angle.geometry.as_mut().unwrap().angle = Some(sides([0.5, 0.6, 0.6]));
        assert!(
            dimension(&angle)
                .unwrap_err()
                .contains("oblique to every view")
        );
    }

    #[test]
    fn one_material_of_the_drawing_is_its_title_blocks() {
        let mut report: Report = serde_json::from_value(serde_json::json!({
            "schema": "draftwright.pmi/1", "parts": ["plate"], "sources": [], "records": [],
            "accounting": {"semantic": 0, "consumed": 0, "reported": 0, "presentation": 0,
                "validation": 0},
            "materials": [{"source_id": "material:#9", "part": 0, "designation": "AlMg3"}],
        }))
        .unwrap();
        assert!(materials(&report).is_empty());
        // A material stating no designation is reported, not taken as no material.
        let mut unnamed = report.clone();
        unnamed.materials[0].designation.clear();
        let found = materials(&unnamed);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].code, "step_material_unavailable");
        assert!(found[0].message.contains("designation is empty"));
        // One part's material, of two parts drawn: not the drawing's.
        report.parts.push("pin".into());
        let found = materials(&report);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(
            (found[0].code.as_str(), found[0].severity),
            ("step_material_unavailable", Severity::Warning)
        );
        assert_eq!(found[0].sources, ["material:#9"]);
        // Each part's own: none can be the title block's.
        let mut steel = report.materials[0].clone();
        steel.source_id = "material:#12".into();
        steel.part = 1;
        report.materials.push(steel);
        let found = materials(&report);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].code, "step_material_ambiguous");
        assert_eq!(found[0].sources, ["material:#9", "material:#12"]);
    }

    #[test]
    fn a_frame_states_its_magnitude_zone_and_datums() {
        let r = frame(0.05, &["diameter_zone"], &["A", "B", "A"]);
        assert_eq!(mismatch(&r, "⌖ ⌀0.05Ⓜ A B"), None);
        assert!(mismatch(&r, "⌖ ⌀0.06Ⓜ A B").is_some());
        assert!(mismatch(&r, "⌖ 0.05Ⓜ A B").is_some());
        assert!(mismatch(&r, "⌖ S⌀0.05 A B").is_some());
        assert!(mismatch(&r, "⌖ ⌀0.05 A").is_some());
        assert!(mismatch(&r, "⌖ ⌀0.05 A B 7").is_some());
        assert!(mismatch(&r, "⌖ A B").unwrap().contains("no numeric value"));
        // Within the display's rounding of the 13th significant digit.
        let r = frame(0.1 + 0.2, &[], &[]);
        assert_eq!(mismatch(&r, "⏥ 0.3"), None);
    }

    /// LINT-code-pmi_source_text_mismatch: source-scoped notes compare their own words, never
    /// the shared note formatter; a finish compares its parameter and value, not just a number.
    #[test]
    fn manufacturing_source_text_is_checked_independently_of_its_identity() {
        use crate::pmi::Field;
        use serde_json::json;
        let mut edge = frame(0., &[], &[]);
        edge.source_id = "manufacturing_requirement:#2".into();
        edge.category = Category::ManufacturingRequirement;
        edge.kind = "edge_condition".into();
        edge.label = "Break sharp edges and remove burrs 0.2 max".into();
        let mut finish = edge.clone();
        finish.source_id = "surface_texture:#3".into();
        finish.kind = "surface_finish".into();
        finish.label = "Ra 0.8 µm".into();
        finish.fields = vec![("Ra".into(), Field::Length(0.0008))];
        let report: Report = serde_json::from_value(json!({
            "schema": "draftwright.pmi/1", "parts": ["plate"], "sources": [],
            "records": [edge, finish], "accounting": {"semantic": 0, "consumed": 0, "reported": 0, "presentation": 0, "validation": 0}, "materials": []
        }))
        .unwrap();
        let mut a: Analysis = serde_json::from_value(json!({
            "schema": "draftwright.analysis/5",
            "binding": {"source_sha256": "", "part": "", "face_count": 0, "loader": ""},
            "envelope": [[0., 0., 0.], [1., 1., 1.]], "iso_bounds": [0., 0., 0., 0.]
        }))
        .unwrap();
        let ids = report.identities();
        let marks = |edge: &str, finish: &str| {
            vec![
                Mark {
                    name: ids.id("manufacturing_requirement:#2").unwrap().into(),
                    view: None,
                    text: edge.into(),
                    claims: vec![],
                },
                Mark {
                    name: ids.id("surface_texture:#3").unwrap().into(),
                    view: None,
                    text: finish.into(),
                    claims: vec![],
                },
            ]
        };
        a.pmi = Some(Reading::Read(report.clone()));
        let mismatches = |marks: &[Mark]| {
            coverage(&a, marks)
                .into_iter()
                .filter(|i| i.code == "pmi_source_text_mismatch")
                .collect::<Vec<_>>()
        };
        let good = marks("1  Break sharp edges and remove\nburrs  0.2 max", "Ra 0.80");
        assert!(mismatches(&good).is_empty());
        for text in [
            "1 Break sharp edges and remove burrs",
            "1 Deburr all edges",
            "1 Break sharp edges and remove burrs 0.2 max EXCEPT HOLES",
        ] {
            let found = mismatches(&marks(text, "Ra 0.8"));
            assert_eq!(found.len(), 1, "{text}: {found:?}");
            assert_eq!(found[0].sources, ["manufacturing_requirement:#2"]);
            assert_eq!(found[0].severity, Severity::Error);
            assert_eq!(found[0].annotation.as_deref(), Some(good[0].name.as_str()));
        }
        for text in ["Ra 9.9", "Rz 0.8", "Ra 0.8 Ra 0.8", "Ra NaN", ""] {
            let found = mismatches(&marks(&good[0].text, text));
            assert_eq!(found.len(), 1, "{text}: {found:?}");
            assert_eq!(found[0].sources, ["surface_texture:#3"]);
        }
        // Text on an unrelated annotation cannot rescue either source's own corrupted text.
        let mut wrong = marks("1 Deburr", "Ra 9.9");
        wrong.push(Mark {
            name: "n:1".into(),
            view: None,
            text: format!("{} Ra 0.8", report.records[0].label),
            claims: vec![],
        });
        assert_eq!(mismatches(&wrong).len(), 2);
        // Structured AP242 textures may also name a multi-word length parameter.
        let mut multi = report.clone();
        multi.records[1]
            .fields
            .push(("sampling length".into(), Field::Length(0.25)));
        let mark = Mark {
            name: "pmi:surface_texture:#3".into(),
            view: None,
            text: "Ra 0.80 sampling length 250".into(),
            claims: vec![],
        };
        assert!(manufacturing_text(&multi, &[mark], &multi.identities()).is_empty());
    }
}
