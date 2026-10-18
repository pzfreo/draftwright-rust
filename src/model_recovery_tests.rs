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

#[test]
fn native_pocket_mouths_use_bounded_readability_details_and_short_proven_routes() {
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
    let (part, a) = analyse_native(&root.join("mfcadpp/10000.step")).unwrap();
    assert_eq!(crate::sheet_fit::preparation_scale(&a), 0.5);
    assert_eq!(crate::sheet_fit::min_legible_scale(&a), 2.0);
    let mut forced = Spec::default();
    forced.sheet.size = Some("A4".into());
    forced.sheet.scale = Some("1:1".into());
    let first = plan_recovered(&a, &forced).unwrap();
    assert!(crate::sheet_fit::readability(&a, &first).native_openings > 0);
    let recovered = plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(crate::sheet_fit::readability(&a, &recovered).failures(), 0);
    assert!(matches!(recovered.sheet.size.as_str(), "A4" | "A3"));
    assert_eq!(recovered.sheet.scale, "1:1");
    let local_interval = recovered
        .dimensions
        .iter()
        .find(|d| {
            d.measurements
                .iter()
                .any(|m| m.parameter.starts_with("step_interval.length"))
        })
        .expect("exact adjacent-height interval in a local crop");
    let mut missing_interval = recovered.clone();
    missing_interval
        .dimensions
        .retain(|d| d.id != local_interval.id);
    assert!(
        crate::sheet_fit::readability(&a, &missing_interval).steps > 0,
        "a crop alone cannot qualify an unannotated baseline interval"
    );

    let disabled: Spec =
        serde_json::from_value(serde_json::json!({"sheet":{"automatic_details":false}})).unwrap();
    let without = plan_recovered(&a, &disabled).unwrap();
    assert_eq!(without.sheet.size, "A4");
    assert!(without.views.iter().all(|v| v.detail.is_none()));
    assert!(crate::sheet_fit::readability(&a, &without).failures() > 0);
    assert!(
        without
            .problems
            .iter()
            .any(|p| p.code == "layout_recovery_exhausted")
    );
    assert!(carried(&first).is_subset(&carried(&recovered)));
    assert!(no_new_drawing_issues(&first, &recovered));
    // Enlarging a native mouth must preserve the independently recorded leader
    // allowance, even though the historical drawing was below the new floor.
    let recorded: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/sheet-policy/10000-main85-callouts.json"
    ))
    .unwrap();
    let drawing = render(&part, &a, &recovered, crate::dims::font()).unwrap();
    let structural = crate::lint::structural::check(&drawing);
    for issue in &structural {
        assert!(
            recorded["structural"]
                .as_array()
                .unwrap()
                .iter()
                .any(|old| old["code"] == issue.code
                    && old["id"] == issue.id
                    && old["severity"] == serde_json::to_value(issue.severity).unwrap()),
            "new rendered finding: {issue:?}"
        );
    }
    for (id, before) in recorded["lengths"].as_object().unwrap() {
        let after = recovered
            .dimensions
            .iter()
            .find(|d| &d.id == id)
            .and_then(|d| d.placed.as_ref())
            .expect("retained callout");
        let length: f64 = after
            .lines
            .iter()
            .map(|s| ((s[1][0] - s[0][0]).powi(2) + (s[1][1] - s[0][1]).powi(2)).sqrt())
            .sum();
        assert!(length <= before.as_f64().unwrap() + 2.0 + 1e-6, "{id}");
    }
    assert!(
        recovered.views.iter().any(|v| v.detail.is_some()),
        "local mouth and height floors require enlarged details"
    );
}

#[test]
fn an_installed_native_schedule_is_the_fitted_reference_for_readability_trials() {
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
    for name in [
        "nist/nist_ctc_02_asme1_rc.stp",
        "nist/nist_ftc_06_asme1_rd.stp",
    ] {
        let (_, a) = analyse_native(&root.join(name)).unwrap();
        crate::planning::with(&a, || {
            let spec = Spec::default();
            established_drawing(&a, &spec).unwrap();
            let first = plan_escalated(&a, &spec).unwrap();
            assert!(!first.hole_tables.is_empty());
            let initially_unreadable = crate::sheet_fit::readability(&a, &first).failures() > 0;
            let reference =
                recover_candidates(&a, &spec, first.clone(), true, false, None, true).unwrap();
            assert_eq!(
                serde_json::to_value(&reference).unwrap(),
                serde_json::to_value(&first).unwrap(),
                "the guarded installed schedule is already the historical fitted drawing"
            );
            let recovered = recover_tables(&a, &spec, first, None, false).unwrap();
            assert!(carried(&reference).is_subset(&carried(&recovered)));
            assert!(repeated_linear_labels(&recovered) <= repeated_linear_labels(&reference));
            assert!(no_new_drawing_issues(&reference, &recovered));
            assert!(crate::sheet_fit::routes_no_worse(
                &a, &reference, &recovered
            ));
            assert!(
                !initially_unreadable
                    || crate::sheet_fit::readability(&a, &recovered).failures() == 0
                    || recovered
                        .problems
                        .iter()
                        .any(|p| p.code == "layout_recovery_exhausted")
            );
        });
    }
}

#[test]
fn native_corridor_violations_in_the_seed_cannot_veto_the_fitted_reference() {
    let evidence: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/sheet_policy_native_routes.json"
    ))
    .unwrap();
    let corpus = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })
        .filter(|p| p.is_dir());
    for row in evidence["parts"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let path = if let Some(name) = name.strip_prefix("corpus/") {
            let Some(root) = &corpus else {
                assert!(
                    std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"),
                    "required corpus missing"
                );
                continue;
            };
            root.join(name)
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name.strip_prefix("fixtures/").unwrap())
        };
        let (_, a) = analyse_native(&path).unwrap();
        assert_eq!(a.binding.source_sha256, row["sha256"].as_str().unwrap());
        let p = plan_recovered(&a, &Spec::default()).unwrap();
        let r = crate::sheet_fit::readability(&a, &p);
        for (key, value) in [
            ("steps", r.steps),
            ("openings", r.openings),
            ("text_arrows", r.text_arrows),
            ("depiction", r.depiction),
            ("lanes", r.lanes),
        ] {
            assert!(
                value as u64 <= row["readability"][key].as_u64().unwrap(),
                "{name}: {key}: {r}"
            );
        }
        let expected: Vec<(String, Vec<String>, Vec<String>)> =
            serde_json::from_value(row["findings"].clone()).unwrap();
        let mut counts = BTreeMap::new();
        for item in expected {
            *counts.entry(item).or_insert(0usize) += 1;
        }
        for q in p
            .problems
            .iter()
            .filter(|q| q.severity != "info" && q.code != "layout_recovery_exhausted")
        {
            let key = (q.code.clone(), q.subjects.clone(), q.refs.clone());
            let remaining = counts
                .get_mut(&key)
                .unwrap_or_else(|| panic!("{name}: new finding {key:?}"));
            assert!(*remaining > 0, "{name}: extra finding {key:?}");
            *remaining -= 1;
        }
        let required: BTreeSet<(String, String)> =
            serde_json::from_value(row["claims"].clone()).unwrap();
        assert!(
            required.is_subset(&carried(&p)),
            "{name}: lost a measurement"
        );
    }
}

#[test]
fn short_reference_text_and_a_bore_callout_fit_their_local_carriers() {
    let Some(root) = std::env::var_os("DRAFTWRIGHT_CORPUS") else {
        assert!(std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_none_or(|v| v != "1"));
        return;
    };
    let (_, a) = analyse_native(&std::path::Path::new(&root).join("mfcadpp/10119.step")).unwrap();
    let reference = plan_recovered(&a, &Spec::default()).unwrap();
    assert!(matches!(reference.sheet.size.as_str(), "A4" | "A3"));
    assert_eq!(crate::sheet_fit::readability(&a, &reference).failures(), 0);
    crate::planning::with(&a, || {
        established_drawing(&a, &Spec::default()).unwrap();
        let mut spec: Spec = serde_json::from_value(
            serde_json::json!({"sheet":{"size":"A3","scale":"1:1","automatic_details":true}}),
        )
        .unwrap();
        for name in crate::detail::NAMES {
            spec.views.entry(name.into()).or_default();
        }
        let mut p = plan_escalated_with(&a, &spec, true, None, true).unwrap();
        crate::sheet_fit::repair_readability(&a, &mut p);
        crate::sheet_fit::retain_callouts(&a, &reference, &mut p);
        assert_eq!(crate::sheet_fit::readability(&a, &p).failures(), 0);
        assert!(drawn_sheets_fit(&a, &p));
        assert!(findings(&p).clean());
        assert!(crate::sheet_fit::routes_no_worse(&a, &reference, &p));
        assert!(no_new_drawing_issues(&reference, &p));
        assert!(carries_required(&reference, &p));
    });
}
