//! Replay the view-conflict gate independently of improvements to the lane solver.
use super::*;
use std::io::Read;

fn recorded(name: &str) -> Plan {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sheet-policy")
        .join(name);
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn a_valid_candidate_crossing_two_views_is_rejected_even_when_the_planner_returns_it() {
    let dir = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })
        .filter(|d| d.is_dir());
    let Some(dir) = dir else {
        assert!(
            std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"),
            "required corpus missing"
        );
        return;
    };
    let (_, mut analysis) = analyse_native(&dir.join("mfcadpp/1004.step")).unwrap();
    let hole = analysis
        .features
        .iter_mut()
        .find(|f| f.id == "f:hole:1")
        .unwrap();
    hole.parameters["location"][1] = serde_json::json!(-60.0);
    hole.parameters["location"][2] = serde_json::json!(120.0);
    let first = recorded("1004-view-gate-first.plan.json.gz");
    let unsafe_try = recorded("1004-view-gate-unsafe.plan.json.gz");
    assert_eq!(first.binding.source_sha256, analysis.binding.source_sha256);
    assert_eq!(
        unsafe_try.binding.source_sha256,
        analysis.binding.source_sha256
    );
    assert_eq!(over_views(&first), 0);
    assert_eq!(over_views(&unsafe_try), 2);
    assert!(findings(&unsafe_try).clean());
    assert!(carried(&first).is_subset(&carried(&unsafe_try)));
    let named: Spec = serde_json::from_value(serde_json::json!({
        "sheet": {"size": "A4", "scale": "1:1"}
    }))
    .unwrap();
    // Replay the same real unsafe proposal for every arrangement request. This
    // exercises the recovery veto even after the ordinary planner repairs the
    // historical drawing. No production path selects a fixture or a part name.
    let plan = crate::planning::with(&analysis, || {
        for arrangement in [
            None,
            Some("columns"),
            Some("stacked-iso"),
            Some("staggered-side"),
        ] {
            let key = serde_json::to_vec(&(&named, true, arrangement)).unwrap();
            crate::planning::store(
                &analysis,
                "candidate",
                key,
                Ok::<Plan, Invalid>(unsafe_try.clone()),
            );
        }
        recover_uncached(&analysis, &Spec::default(), first).unwrap()
    });
    assert_eq!(over_views(&plan), 0);
    assert_ne!(
        (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    let gate = plan
        .sheet
        .basis
        .split_once("candidates: ")
        .unwrap()
        .1
        .split("; ")
        .find(|t| t.starts_with("A4 1:1 fill"))
        .expect("the unsafe A4 1:1 proposal is tested");
    assert!(
        gate.contains("valid and complete, 2 additional view conflicts"),
        "{}",
        plan.sheet.basis
    );
}

/// New information may use more paper, but may not add hard findings to
/// established dimension/GD&T ink. Exercise the same bounded reduction used
/// by automatic recovery, with its reference memo prepared normally.
#[test]
fn new_recess_locations_clear_established_pmi_ink() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes =
        std::fs::read(root.join("tests/fixtures/pmi/step/nist_stc_09_asme1_ap242-e4.stp.gz"))
            .unwrap();
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut step)
        .unwrap();
    let dir = std::env::temp_dir().join(format!("draftwright-recess-pmi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stc09.stp");
    std::fs::write(&path, step).unwrap();
    let a = analyse_native(&path).unwrap().1;
    crate::planning::with(&a, || {
        let before = established_drawing(&a, &Spec::default()).unwrap();
        let spec: Spec =
            serde_json::from_value(serde_json::json!({"sheet":{"size":"A1","scale":"1:5"}}))
                .unwrap();
        let trial = plan_escalated_with(&a, &spec, true, None, false).unwrap();
        let hard = |p: &Plan| -> BTreeSet<_> {
            p.problems
                .iter()
                .filter(|p| HARD_LAYOUT.contains(&p.code.as_str()))
                .map(|p| {
                    let mut ids = p.subjects.clone();
                    ids.sort();
                    (p.code.clone(), ids)
                })
                .collect()
        };
        let original = hard(&before);
        let actual = hard(&trial);
        assert!(
            carried(&before).is_subset(&carried(&trial)),
            "existing measurements lost"
        );
        let new: Vec<_> = actual.difference(&original).collect();
        assert!(
            new.is_empty(),
            "new hard findings: {new:#?}; grade {:?}; {}; new placements {:#?}",
            gdt_misplaced(&trial),
            trial.sheet.basis,
            trial
                .dimensions
                .iter()
                .filter(|d| d.id.starts_with("d:f:section_recess:"))
                .map(|d| (&d.id, &d.placed))
                .collect::<Vec<_>>()
        );
        let auto = plan_escalated_with(&a, &Spec::default(), false, None, false).unwrap();
        assert!(
            carried(&auto).is_subset(&carried(&trial)),
            "reduction loses {:?}",
            carried(&auto)
                .difference(&carried(&trial))
                .collect::<Vec<_>>()
        );
    });
}
