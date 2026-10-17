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
            let analysis_key: [u8; 32] =
                Sha256::digest(serde_json::to_vec(&analysis).unwrap()).into();
            // Native recovery prepares its established-route policy before the
            // first candidate; inject the same unsafe drawing on either side of
            // that preparation, under the planner's complete exact input key.
            for policy in [(false, false), (false, true)] {
                let key =
                    serde_json::to_vec(&(analysis_key, policy, &named, true, arrangement, false))
                        .unwrap();
                crate::planning::store(
                    &analysis,
                    "candidate",
                    key,
                    Ok::<Plan, Invalid>(unsafe_try.clone()),
                );
            }
        }
        recover_uncached(&analysis, &Spec::default(), first, None).unwrap()
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

#[test]
fn covered_aliases_follow_a_location_into_a_drawn_schedule_only() {
    let mut plan = recorded("1004-view-gate-first.plan.json.gz");
    let alias = "d:f:slot:extra/location_slot_across.length";
    let owner = "d:f:hole:extra/location.length.y";
    for (id, by) in [(alias, owner), (owner, "schedule")] {
        plan.omitted.push(crate::dims::Omitted {
            id: id.into(),
            label: "20".into(),
            basis: "same physical span".into(),
            state: ItemState::Covered { by: by.into() },
        });
    }
    let claim = ("f:slot:extra".into(), "location_slot_across.length".into());
    assert!(!carried(&plan).contains(&claim));
    let rows = vec![vec!["20".into()]];
    plan.hole_tables.push(crate::hole_table::HoleTable {
        sheet: 1,
        id: "schedule".into(),
        view: Some("v:plan".into()),
        origin: None,
        pcd: None,
        covered: vec![],
        rows: vec![],
        default: true,
        basis: String::new(),
        state: ItemState::Shown,
        blocks: vec![crate::tables::PlannedTable {
            sheet: 1,
            id: "schedule".into(),
            rows: rows.clone(),
            row_claims: vec![],
            default: true,
            basis: String::new(),
            state: ItemState::Shown,
            placed: Some(crate::tables::lay(&rows, &[10.0], [10.0, 10.0])),
        }],
    });
    assert!(carried(&plan).contains(&claim));
    plan.hole_tables[0].blocks[0].placed = None;
    assert!(
        !carried(&plan).contains(&claim),
        "an unplaced table carries no alias"
    );
    plan.omitted.last_mut().unwrap().state = ItemState::Covered { by: alias.into() };
    assert!(
        !carried(&plan).contains(&claim),
        "a cycle without drawn ink is not coverage"
    );
}

#[test]
fn recess_scale_reference_uses_the_established_fitted_choice_before_a4_ranking() {
    let root = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })
        .filter(|p| p.is_dir());
    let Some(root) = root else {
        assert!(
            std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"),
            "required corpus missing"
        );
        return;
    };
    let (_, a) = analyse_native(&root.join("mfcadpp/10033.step")).unwrap();
    let reference =
        crate::planning::with(&a, || established_drawing(&a, &Spec::default())).unwrap();
    assert_eq!(reference.sheet.scale, "2:1");
    let mut earlier = a.clone();
    earlier.features.retain(|f| f.kind != "section_recess");
    let screen_first = plan_recovered(&earlier, &Spec::default()).unwrap();
    assert_eq!(
        (
            screen_first.sheet.size.as_str(),
            screen_first.sheet.scale.as_str()
        ),
        ("A4", "1:1")
    );
    assert!(carried(&screen_first).is_subset(&carried(&reference)));
    let (_, channels) = analyse_native(&root.join("nist/nist_ftc_08_asme1_rc.stp")).unwrap();
    assert_eq!(crate::sheet_fit::preparation_scale(&channels), 0.5);
    assert_eq!(crate::sheet_fit::min_legible_scale(&channels), 1.0);
}
