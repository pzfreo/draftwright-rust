//! Problems in drawing language with subjects that say what they are about, and reference
//! dimensions flagged (draftwright-rust #24, #25, #26):
//! - a message names a dimension by the text it prints and a feature by what a draftsman sees
//!   of it, never by an internal ID; the IDs stay in `subjects`;
//! - unclaimed geometry names its faces, not the sheet;
//! - a dimension drawn in brackets carries `reference: true` and the IDs that determine it.
//!
//! The editor's samples: `parts/mounting_plate.step` is byte for byte the editor's `plate.step`,
//! and `samples/specify-sample-spindle.step.gz` its turned spindle (draftwright-editor
//! `public/samples/specify-sample-spindle.step`).

#[path = "common/support.rs"]
mod support;

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::wording::carries_id;
use serde_json::{Value, json};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// The gzipped fixture *path*, analysed as the command line analyses it, from a scratch file of
/// its own named *name* (a binding names its part by the file's stem).
fn gz(path: &str, name: &str) -> Analysis {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("problem_wording")
        .join(format!("{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture(path)).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let scratch = dir.join(name);
    std::fs::write(&scratch, bytes).unwrap();
    let a = model::analyse_native(&scratch).unwrap().1;
    std::fs::remove_dir_all(&dir).unwrap();
    a
}

fn spindle() -> Analysis {
    gz(
        "samples/specify-sample-spindle.step.gz",
        "specify-sample-spindle.step",
    )
}

fn spec(v: Value) -> Spec {
    serde_json::from_value(v).unwrap()
}

/// #24, as reported on the editor's plate: the grid's column pitch placed above, where its
/// second extension line crosses the ⌀8 hole.
#[test]
fn the_plates_extension_crossing_reads_in_drawing_language() {
    let (_, a) = model::analyse_native(&fixture("parts/mounting_plate.step")).unwrap();
    let pitch = "d:f:hole_pattern:1/grid_pitch.length.col";
    let s = spec(json!({ "dimensions": { pitch: { "place": { "side": "above" } } } }));
    let plan = model::plan_recovered(&a, &s).unwrap();
    let p = plan
        .problems
        .iter()
        .find(|p| {
            p.code == "extension_through_feature" && p.subjects.first().is_some_and(|s| s == pitch)
        })
        .unwrap_or_else(|| panic!("no extension crossing for {pitch}: {:#?}", plan.problems));
    assert_eq!(p.subjects, [pitch, "f:hole:4"]);
    assert_eq!(
        p.message,
        "The second extension line of 2× 30 crosses the ⌀8 hole."
    );
    assert_eq!(carries_id(&p.message), None);
}

/// #25: the spindle's unclaimed faces are the problem's subjects, not the sheet.
#[test]
fn the_spindles_unclaimed_faces_are_its_subjects() {
    let a = spindle();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "unclaimed_geometry")
        .expect("the spindle has unclaimed faces");
    let unclaimed = plan.completeness.faces - plan.completeness.claimed_faces;
    assert_eq!(unclaimed, 3);
    assert_eq!(p.subjects.len(), unclaimed, "{:?}", p.subjects);
    for s in &p.subjects {
        let face: usize = s
            .strip_prefix("face:")
            .unwrap_or_else(|| panic!("{s} is not a face"))
            .parse()
            .unwrap();
        assert!(face < a.binding.face_count);
        assert!(
            !a.features.iter().any(|f| f.faces.contains(&face)),
            "{s} belongs to a feature"
        );
    }
    assert_eq!(carries_id(&p.message), None);
}

/// #25 across the test parts and every AP242 fixture: `sheet` is the subject only of problems about paper, scale, fit
/// or the title block drawn on it.
#[test]
fn only_sheet_problems_have_the_sheet_as_subject() {
    const SHEET: [&str; 15] = [
        "layout_replanned",
        "layout_recovery_exhausted",
        "scale_fallback_applied",
        "legibility_floor_breached",
        "views_do_not_fit",
        "page_fit_uncertain",
        "sheet_below_smallest_scale",
        "hole_table_unavailable",
        "general_tolerance_missing",
        "title_field_overflow",
        "annotation_off_sheet",
        "step_general_tolerance_disagreement",
        "step_material_disagreement",
        "sheet_label_replaced",
        "spec_entries_inactive",
    ];
    for (name, plan) in plans() {
        for p in &plan.problems {
            if p.subjects.iter().any(|s| s == "sheet") {
                assert!(
                    SHEET.contains(&p.code.as_str()),
                    "{name}: {} ({}) is not about the sheet: {}",
                    p.id,
                    p.code,
                    p.message
                );
            }
        }
    }
}

/// #26: the turned spindle's bracketed (12) is a reference, determined by the overall length and
/// the chain's other links; its controlling dimensions are not.
#[test]
fn the_spindles_bracketed_length_is_a_reference_with_what_determines_it() {
    // Independent profiles retain the reference/closure contract. Connected whole-part
    // profiles now default to baseline ordinates and have no redundant reference link.
    let mut a = spindle();
    for f in a.features.iter_mut().filter(|f| f.kind == "turned_step") {
        f.parameters["profile"]["body_key"] = json!([f.id]);
    }
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let d = plan
        .dimensions
        .iter()
        .find(|d| d.label == "(12)")
        .expect("the spindle's (12)");
    assert_eq!(d.id, "d:f:turned_step:3/step.length");
    assert!(d.reference);
    assert_eq!(
        d.determined_by,
        [
            "d:envelope/height.length",
            "d:f:turned_step:1/step.length",
            "d:f:turned_step:2/step.length"
        ]
    );
    for id in &d.determined_by {
        let by = plan.dimensions.iter().find(|o| o.id == *id).unwrap();
        assert!(!by.reference && by.determined_by.is_empty(), "{id}");
    }
    // On the wire: present on the reference, absent on the others.
    let wire = serde_json::to_value(&plan).unwrap();
    let dims = wire["dimensions"].as_array().unwrap();
    for w in dims {
        let bracketed = w["label"].as_str().unwrap().starts_with('(');
        assert_eq!(w.get("reference").is_some(), bracketed, "{w}");
        assert_eq!(w.get("determined_by").is_some(), bracketed, "{w}");
    }
    assert_eq!(
        dims.iter().find(|w| w["label"] == "(12)").unwrap()["reference"],
        true
    );
}

/// The specs a part is planned under, beside the defaults, to raise the spec's own problems:
/// one dimension omitted and toleranced; one given a band of nothing; an authored pair; each
/// principal view omitted; and, on the editor's plate, its grid pitch placed above and a fit on
/// a hole whose callout is omitted.
fn specs(part: &str, base: &Plan) -> Vec<(String, Spec)> {
    let ids: Vec<&str> = base.dimensions.iter().map(|d| d.id.as_str()).collect();
    let mut specs = Vec::new();
    if let Some(first) = ids.first() {
        specs.push((
            "omitted".into(),
            spec(json!({ "dimensions": { *first: { "omit": true, "tolerance": "±0.1" } } })),
        ));
    }
    if let Some(l) = base.dimensions.iter().find(|d| d.kind == "linear") {
        specs.push((
            "±0".into(),
            spec(json!({ "dimensions": { &l.id: { "tolerance": "±0" } } })),
        ));
    }
    if ids.len() >= 2 {
        specs.push((
            "authored".into(),
            spec(json!({ "dimension_source": "authored",
                         "dimensions": { ids[0]: {}, ids[1]: {} } })),
        ));
    }
    for v in ["front", "plan", "side"] {
        specs.push((
            format!("{v} omitted"),
            spec(json!({ "views": { v: { "omit": true } } })),
        ));
    }
    if part == "mounting_plate" {
        specs.push((
            "pitch above".into(),
            spec(json!({ "dimensions": {
                "d:f:hole_pattern:1/grid_pitch.length.col": { "place": { "side": "above" } } } })),
        ));
        specs.push((
            "fit, callout omitted".into(),
            spec(json!({ "features": { "f:hole:4": { "fit": "H7" } },
                         "dimensions": { "d:f:hole:4/bore.diameter": { "omit": true } } })),
        ));
    }
    specs
}

/// Every test part's plan under the defaults and [`specs`] (a spec a part refuses is left out),
/// and every AP242 fixture's under the defaults; made once for the tests that read them.
fn plans() -> &'static [(String, Plan)] {
    static PLANS: std::sync::OnceLock<Vec<(String, Plan)>> = std::sync::OnceLock::new();
    PLANS.get_or_init(|| {
        let mut parts: Vec<(String, Analysis)> = vec![
            (
                "plate (recognition)".into(),
                model::analyse_with(
                    &fixture("parts/plate.step"),
                    Some(&fixture("recognition/plate.json")),
                )
                .unwrap()
                .1,
            ),
            ("specify-sample-spindle".into(), spindle()),
            (
                "manufacturing_pmi".into(),
                gz("parts/manufacturing_pmi.step.gz", "manufacturing_pmi.step"),
            ),
        ];
        for e in std::fs::read_dir(fixture("parts")).unwrap() {
            let path = e.unwrap().path();
            // `unresolved.step` is refused before planning, by design.
            if path.extension().is_some_and(|x| x == "step")
                && let Ok((_, a)) = model::analyse_native(&path)
            {
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                parts.push((name, a));
            }
        }
        let mut files: Vec<String> = std::fs::read_dir(fixture("pmi/step"))
            .unwrap()
            .filter_map(|e| {
                let name = e.unwrap().file_name().into_string().unwrap();
                name.ends_with(".gz").then_some(name)
            })
            .collect();
        files.sort();
        parts.extend(support::parallel(&files, |file| {
            (
                file.clone(),
                gz(&format!("pmi/step/{file}"), file.trim_end_matches(".gz")),
            )
        }));
        let bases = support::parallel(&parts, |(_, a)| {
            model::plan_recovered(a, &Spec::default()).unwrap()
        });
        let jobs: Vec<(String, String, &Analysis, Spec)> = parts
            .iter()
            .zip(&bases)
            .filter(|((part, _), _)| !part.ends_with(".gz"))
            .flat_map(|((part, a), base)| {
                specs(part, base)
                    .into_iter()
                    .map(move |(name, s)| (part.clone(), name, a, s))
            })
            .collect();
        let mut out: Vec<(String, Plan)> = parts
            .iter()
            .map(|(part, _)| format!("{part}, defaults"))
            .zip(bases.iter().cloned())
            .collect();
        out.extend(
            support::parallel(&jobs, |(part, name, a, s)| {
                model::plan_recovered(a, s)
                    .ok()
                    .map(|plan| (format!("{part}, {name}"), plan))
            })
            .into_iter()
            .flatten(),
        );
        out
    })
}

/// #24 across the test parts and every AP242 fixture: no problem message carries an internal
/// ID (`d:`, `f:`, `v:`, `pmi:`, `n:`, `t:`, `face:`).
#[test]
fn no_problem_message_carries_an_internal_id() {
    let mut checked = 0;
    let mut found = Vec::new();
    for (name, plan) in plans() {
        for p in &plan.problems {
            checked += 1;
            if let Some(id) = carries_id(&p.message) {
                found.push(format!("{name}: {} names {id}: {}", p.code, p.message));
            }
        }
    }
    assert!(checked > 200, "{checked} problems checked");
    assert!(
        found.is_empty(),
        "{} messages carry IDs:\n{}",
        found.len(),
        found.join("\n")
    );
}
