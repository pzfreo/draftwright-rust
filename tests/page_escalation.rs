//! Finished-sheet policy (maintainer 2026-10-17), with Python recovery replay.
//! The low-level layout::recover stand-ins retain the historical bounded retry contract;
//! model::recover exhausts clean readable A4 and its allocation before larger paper.
//! Corpus tests protect complete measurements, explicit overrides and
//! the veto on extra ink over other views. Deliberate Python differences are recorded in
//! tests/fixtures/known_sheet_policy.json. model::plan remains the unrecovered estimate.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;
#[path = "common/sheet_rule.rs"]
mod sheet_rule;

use draftwright::api::{self, Features};
use draftwright::layout::{self, Attempt, Findings, Retry};
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::json;

const A4: [f64; 2] = [297.0, 210.0];
const A3: [f64; 2] = [420.0, 297.0];

fn hard() -> Findings {
    Findings {
        hard: 1,
        dropped: 0,
    }
}

fn dropped() -> Findings {
    Findings {
        hard: 0,
        dropped: 1,
    }
}

fn tried(attempts: &[Attempt]) -> Vec<(Retry, bool)> {
    attempts.iter().map(|a| (a.retry, a.taken)).collect()
}

/// draftwright's `test_hard_layout_recovery_probes_one_scale_each_way_before_spending_paper` and
/// the order of `test_crowded_head_uses_larger_scale_before_spending_the_sheet`: a hard failure
/// at 2:1 on A4 tries 1:1, then 5:1 on A4, then A3, which is taken; nothing past it is tried.
#[test]
fn a_hard_failure_tries_a_scale_each_way_on_its_sheet_before_a_larger_sheet() {
    let mut calls = Vec::new();
    let attempts = layout::recover(2.0, A4, false, hard(), None, &mut |r| {
        calls.push(r);
        Some(if r == Retry::Sheet("A3") {
            Findings::default()
        } else {
            hard()
        })
    });
    assert_eq!(
        tried(&attempts),
        [
            (Retry::Scale(1.0), false),
            (Retry::Scale(5.0), false),
            (Retry::Sheet("A3"), true)
        ]
    );
    assert_eq!(calls.len(), 3);
}

/// draftwright's `test_hard_layout_precedes_completeness_in_automatic_page_scale_verdict`: a
/// 10:1 plan on A3 with overlapping labels and a lost item takes the hard-layout recovery (the
/// nearer scale each way: 5:1, as nothing is larger than 10:1), and the clean 5:1 is taken. A
/// try that recovers the lost item but leaves a hard failure is not taken, nor one that is valid
/// but loses an item.
#[test]
fn hard_layout_validity_comes_before_completeness() {
    let both = Findings {
        hard: 1,
        dropped: 1,
    };
    let attempts = layout::recover(10.0, A3, false, both, None, &mut |r| {
        Some(if r == Retry::Scale(5.0) {
            Findings::default()
        } else {
            both
        })
    });
    assert_eq!(tried(&attempts), [(Retry::Scale(5.0), true)]);

    // Each rejected try: a hard failure without the loss, then the loss without a hard failure.
    let mut answers = vec![hard(), dropped()].into_iter();
    let attempts = layout::recover(2.0, A3, false, both, None, &mut |_| {
        Some(answers.next().unwrap_or_default())
    });
    assert_eq!(
        tried(&attempts),
        [
            (Retry::Scale(1.0), false),
            (Retry::Scale(5.0), false),
            (Retry::Sheet("A2"), true)
        ]
    );
}

/// draftwright's `test_required_drop_without_iso_still_uses_bounded_scale_and_page_recovery`: a
/// required loss on a valid plan tries the next two larger scales on its sheet, then the larger
/// sheets.
#[test]
fn a_required_loss_tries_larger_scales_then_larger_sheets() {
    let attempts = layout::recover(2.0, A4, false, dropped(), None, &mut |r| {
        Some(if r == Retry::Sheet("A3") {
            Findings::default()
        } else {
            dropped()
        })
    });
    assert_eq!(
        tried(&attempts),
        [
            (Retry::Scale(5.0), false),
            (Retry::Scale(10.0), false),
            (Retry::Sheet("A3"), true)
        ]
    );
}

/// draftwright's `test_recovery_does_not_trade_a_required_drop_for_unreadable_ink` (the veto
/// cases): a try that places the lost item but makes the ink unreadable is rejected, and the
/// next sheet, clean, is taken. The non-veto case (an info observation is no hard failure) is
/// [`an_info_or_view_box_finding_is_not_a_hard_failure`].
#[test]
fn a_required_loss_is_not_traded_for_unreadable_ink() {
    let attempts = layout::recover(1.0, A4, false, dropped(), None, &mut |r| {
        Some(match r {
            Retry::Sheet("A3") => Findings::default(),
            _ => hard(),
        })
    });
    assert_eq!(attempts.last().unwrap().retry, Retry::Sheet("A3"));
    assert!(
        attempts[..attempts.len() - 1]
            .iter()
            .all(|a| !a.taken && a.findings == Some(hard()))
    );
}

/// draftwright's `test_complete_detail_drawing_stays_on_its_original_page`: a valid, complete plan
/// tries nothing, so never spends a larger sheet.
#[test]
fn a_valid_complete_plan_tries_nothing() {
    let attempts = layout::recover(2.0, A4, false, Findings::default(), None, &mut |_| {
        panic!("a valid, complete plan is re-planned")
    });
    assert!(attempts.is_empty());
}

/// The recovery is bounded: at most two scales and the four larger sheets, and a plan that
/// cannot be made (`None`) is passed over, never taken. When nothing is clean, nothing is taken.
#[test]
fn the_recovery_is_bounded_and_keeps_the_first_plan_when_nothing_is_clean() {
    for first in [hard(), dropped()] {
        let attempts = layout::recover(1.0, A4, false, first, None, &mut |r| {
            (r != Retry::Scale(0.5)).then_some(hard())
        });
        assert_eq!(attempts.len(), 2 + 4, "{first:?}");
        assert!(attempts.iter().all(|a| !a.taken));
    }
}

/// draftwright's `test_explicit_a4_remains_fixed_instead_of_escalating`, on the ladder: a sheet the
/// spec names is never left, only its scale tried.
#[test]
fn a_named_sheet_is_never_left() {
    for first in [hard(), dropped()] {
        let attempts = layout::recover(2.0, A4, true, first, None, &mut |_| Some(hard()));
        assert!(
            attempts.iter().all(|a| matches!(a.retry, Retry::Scale(_))),
            "{attempts:?}"
        );
    }
}

fn corpus_part(name: &str) -> Option<Analysis> {
    let dir = corpus::dir()?;
    let f = facts::load(name);
    Some(corpus::analyse(&dir, &f.binding.file, name).1)
}

fn spec(sheet: serde_json::Value) -> Spec {
    serde_json::from_value(serde_json::json!({ "sheet": sheet })).unwrap()
}

/// The plan with draftwright's recovery applied (`model::recover` on the first try, its hole items
/// with no place dropped and escalated: `model::plan_escalated`), as `model::plan_recovered`.
fn recovered(analysis: &Analysis, spec: &Spec) -> Plan {
    let first = model::plan_escalated(analysis, spec).unwrap();
    model::recover(analysis, spec, first).unwrap()
}

fn codes(plan: &Plan) -> Vec<&str> {
    plan.problems.iter().map(|p| p.code.as_str()).collect()
}

fn sheet(plan: &Plan) -> (&str, &str) {
    (plan.sheet.size.as_str(), plan.sheet.scale.as_str())
}

/// On the corpus: ctc01's first choice (A3 at 1:5, draftwright's too) has no hard layout finding
/// and loses nothing, so the recovery tries nothing and says nothing, as draftwright's. Until the
/// layout kept the isometric off the plan's bands (drawing quality 3), one label the isometric
/// displaced ran off the sheet and the recovery re-planned ctc01 at A3 1:10; no captured part is
/// re-planned now, so the re-plan itself is held by the stand-ins above and by
/// `tests/issue_drafting.rs::the_recovery_tries_the_same_page_first_and_keeps_every_outcome`'s
/// rule over every part. A scale or sheet the spec names leaves it as it is.
#[test]
fn a_plan_without_ink_off_the_sheet_keeps_its_first_choice_on_ctc01() {
    let first = ("A3", "1:5");
    let Some(analysis) = corpus_part("ctc01") else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    for spec in [
        Spec::default(),
        spec(serde_json::json!({ "scale": first.1 })),
        spec(serde_json::json!({ "size": first.0 })),
    ] {
        let plan = recovered(&analysis, &spec);
        assert!(!codes(&plan).contains(&"annotation_off_sheet"));
        assert!(!plan.sheet.basis.contains("layout recovery"));
        if spec.sheet.size.is_none() && spec.sheet.scale.is_none() {
            // Stage H: interior callouts (H7) and label settlement (H3) leave this recognised
            // fixture with a valid A4 1:10 smallest fit, fuller than A3 1:5, so the sheet rule
            // (H6), not the first estimate, decides it.
            sheet_rule::assert_rule("ctc01", &plan);
            continue;
        }
        assert_eq!(sheet(&plan), first);
        assert!(!codes(&plan).iter().any(|c| c.starts_with("layout_re")));
    }
}

/// draftwright's own recovery on cgb245, whose outcome the port's matches (equivalent): A2 at 1:2
/// is kept and the failed recovery said (`layout_recovery_exhausted`). draftwright's first choice
/// is invalid (an ink overlap), so it tries 1:5 first and rejects it for two
/// `location_ref_dropped` blockers (hole locations closer than 4.7 mm on the page), 1:1 for
/// `view_out_of_bounds`, A1 and A0 for a required drop (`tests/fixtures/sheets/cgb245.json.gz`).
/// The port keeps every location, so its gate counts the same spacing as locations too close to
/// read. Its first choice has no hard layout finding (since the layout kept the isometric off the
/// plan's bands, drawing quality 3, the label that ran off the sheet stands right of the plan)
/// but two locations too close to read, a loss, so it tries the next two larger scales and then
/// each larger sheet: A2 at 1:1 and 2:1, A1 and A0 at 1:1, each with hard findings, and each but
/// 2:1 still with two locations too close to read; none is taken. (A2 at 1:1 dropped 3 hole items
/// placed off the sheet, `src/hole_table.rs`, until the layout reserved the band right of the
/// front and the plan for the front's lanes, stream d-quality-lanes: they now find room.) Before the
/// gate counted them, the port took 1:5, valid by its placement checks but with location witness lines as close as
/// 0.6 mm.
#[test]
fn a_try_with_hole_locations_too_close_to_read_is_not_taken() {
    let Some(analysis) = corpus_part("cgb245") else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let no_tables = Spec {
        hole_table: draftwright::hole_table::Setting::Never,
        ..Spec::default()
    };
    let plan = recovered(&analysis, &no_tables);
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    assert!(!codes(&plan).contains(&"annotation_off_sheet"));
    let said = plan
        .problems
        .iter()
        .find(|p| p.code == "layout_recovery_exhausted")
        .unwrap_or_else(|| panic!("{:?}", codes(&plan)));
    assert_eq!(said.severity, "warning");
    let readability = draftwright::sheet_fit::readability(&analysis, &plan);
    assert_eq!(readability.lanes, 2);
    assert!(readability.steps > 0);
    assert!(
        said.message.contains(&readability.to_string()),
        "{}",
        said.message
    );
    assert!(said.message.contains("physically fitting drawing is kept"));
    for tried in [
        "A3 1:1 (principal projection row/column exceeds usable paper",
        "A2 1:2 fill",
        "A0 2:1 fill",
    ] {
        assert!(
            plan.sheet.basis.contains(tried),
            "{tried}: {}",
            plan.sheet.basis
        );
    }
    assert!(!plan.sheet.basis.contains("layout recovery"));
}

/// draftwright's location legibility (`_legible_locations`, `_discard_short_refs`): an ordinate
/// under 4.7 mm on the page past the last legible one, or a nonzero span under 1 mm, is too close
/// to read; the rest are measured from the last one kept.
#[test]
fn hole_locations_closer_than_draftwrights_spacing_are_counted() {
    assert_eq!(layout::MIN_LOC_SEP, 4.7);
    // At 1:1: 10 and 14 are 4 apart (crowded), 16 is 6 past 10 (kept), 20 is 4 past 16.
    assert_eq!(layout::crowded_locations(&[20.0, 10.0, 14.0, 16.0], 1.0), 2);
    // At 5:1 every gap is legible; at 1:5 a 3 mm span is 0.6 mm on the page.
    assert_eq!(layout::crowded_locations(&[20.0, 10.0, 14.0, 16.0], 5.0), 0);
    assert_eq!(layout::crowded_locations(&[3.0, 40.0], 0.2), 1);
    assert_eq!(layout::crowded_locations(&[], 1.0), 0);
}

/// Complete drawings still compete by fill, and the choice is checked against the rule over the
/// candidates each basis records (`common/sheet_rule.rs`), not against a pinned output.
/// draftwright took GRM-03 to 5:1 and m10127 to A3 at 2:1; both now must be what the rule picks.
#[test]
fn a_valid_plan_is_reconsidered_for_fill() {
    for name in ["grm03", "m10127"] {
        let Some(analysis) = corpus_part(name) else {
            eprintln!("SKIPPED: the corpus is not found");
            return;
        };
        let plan = recovered(&analysis, &Spec::default());
        sheet_rule::assert_rule(name, &plan);
        assert!(plan.sheet.default, "{name}");
    }
}

/// A4 remains the whole-part default. Local floors get enlarged crops; the
/// A4-to-A3 exception must be completely readable and never reaches A2+.
#[test]
fn local_floors_use_details_and_never_buy_large_paper() {
    let Some(dir) = corpus::dir() else { return };
    for (file, expected) in [
        ("mfcadpp/10060.step", ("A4", "1:1")),
        ("mfcadpp/10000.step", ("A4", "1:1")),
    ] {
        let path = dir.join(file);
        let plan = api::plan_from(&path, Features::Native, &Spec::default()).unwrap();
        sheet_rule::assert_rule(file, &plan);
        assert_eq!(sheet(&plan), expected, "{file}: {}", plan.sheet.basis);
        let analysis = api::analyse_from(&path, Features::Native).unwrap();
        assert_eq!(
            draftwright::sheet_fit::readability(&analysis, &plan).failures(),
            usize::from(file.ends_with("10000.step"))
        );
        if file.ends_with("10000.step") {
            let compressed =
                api::plan_analysed(&analysis, &spec(json!({"size":"A4", "scale":"1:1"}))).unwrap();
            assert!(
                draftwright::sheet_fit::readability(&analysis, &compressed).steps > 0,
                "{file}"
            );
            assert!(
                draftwright::sheet_fit::readability(&analysis, &compressed).native_openings > 0,
                "the native pocket must also exceed 2 mm: {file}"
            );
            assert!(plan.sheet.basis.contains("A4 multi-sheet:"));
            assert_eq!(plan.sheet.scale, "1:1");
            assert_eq!(plan.views.iter().filter(|v| v.detail.is_some()).count(), 1);
            assert!(
                plan.problems
                    .iter()
                    .any(|p| p.code == "layout_recovery_exhausted")
            );
            assert!(
                plan.dimensions
                    .iter()
                    .filter(|d| d.view.starts_with("v:detail_"))
                    .all(|d| !d.reference)
            );
        }
    }
}

/// Local step intervals recover in a detail on the established A4 drawing.
/// Whole-part enlargements still cannot buy longer or obstructed callouts.
#[test]
fn compressed_steps_recover_in_an_a4_detail_without_buying_longer_callouts() {
    let Some(dir) = corpus::dir() else { return };
    let path = dir.join("mfcadpp/10063.step");
    let plan = api::plan_from(&path, Features::Native, &Spec::default()).unwrap();
    let analysis = api::analyse_from(&path, Features::Native).unwrap();
    assert_eq!(sheet(&plan), ("A4", "1:1"));
    assert_eq!(
        draftwright::sheet_fit::readability(&analysis, &plan).failures(),
        3
    );
    assert!(plan.views.iter().all(|v| v.detail.is_none()));
    assert_eq!(
        draftwright::sheet_fit::readability(&analysis, &plan).whole_drawing_failures(),
        0
    );
    assert!(plan.dimensions.iter().all(|d| d.placed.is_some()));
    assert!(
        plan.problems
            .iter()
            .any(|p| p.code == "layout_recovery_exhausted")
    );
    assert!(!plan.problems.iter().any(|p| p.code.ends_with("_dropped")
        || matches!(
            p.code.as_str(),
            "annotation_off_sheet" | "annotation_over_title_block"
        )));
    assert!(plan.sheet.basis.contains("A4 multi-sheet:"));
    assert!(
        plan.sheet
            .basis
            .contains("leader length exceeds retained route by more than 2 mm")
    );
    sheet_rule::assert_rule("10063", &plan);
}

/// The non-veto case of `test_recovery_does_not_trade_a_required_drop_for_unreadable_ink`: an
/// info observation is no hard failure, and nor is the port's `annotation_over_view` (judged by
/// the view's box; draftwright's `view_annotation_overlap` by its line-work). m1000's first
/// choice, A4 at 1:1 (draftwright's too), has ink past its areas (info) and is kept. Until the
/// layout reserved the front's right band for its lanes (stream d-quality-lanes) it also had a
/// label over the side view's box (the seat's z location); no captured part has one now, so the
/// same first plan is given that warning and must still be kept.
#[test]
fn an_info_or_view_box_finding_is_not_a_hard_failure() {
    let Some(analysis) = corpus_part("m1000") else {
        return;
    };
    let expected = recovered(&analysis, &Spec::default());
    sheet_rule::assert_rule("m1000", &expected);
    assert!(!codes(&expected).contains(&"layout_recovery_exhausted"));

    let id = "d:f:section_recess:4/seat_location.location.z";
    let mut first = model::plan_escalated(&analysis, &Spec::default()).unwrap();
    first.problems.push(model::Problem {
        id: format!("p:annotation_over_view:{id}:v:side"),
        code: "annotation_over_view".into(),
        severity: "warning".into(),
        message: format!("{id} lies over the v:side view."),
        subjects: vec![id.into(), "v:side".into()],
        ..Default::default()
    });
    let kept = model::recover(&analysis, &Spec::default(), first).unwrap();
    assert_eq!(sheet(&kept), sheet(&expected));
    let found = codes(&kept);
    assert!(!found.contains(&"layout_recovery_exhausted"), "{found:?}");
}

/// The maintainer's directive of 2026-10-09 (nothing off the sheet: escalate scale or sheet as
/// draftwright does, never clip): every planning entry point applies the recovery. cgb245's plan
/// from the library (`api::plan_analysed`) and from the JSON operation the service and MCP answer
/// (`api::call("plan")`) is the recovered plan, the same each time: A2 at 1:2, its failed
/// recovery said (`layout_recovery_exhausted`, which only `model::recover` says). The drawing's
/// reply (`review`) says it too. A sheet and scale the spec names are kept and nothing is tried;
/// a sheet it names alone is kept. (ctc01 held this until the layout kept the isometric off the
/// plan's bands, drawing quality 3: its label off the sheet is gone, and with it the re-plan.)
#[test]
fn every_planning_entry_point_applies_the_recovery() {
    let Some(analysis) = corpus_part("cgb245") else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let no_tables = Spec {
        hole_table: draftwright::hole_table::Setting::Never,
        ..Spec::default()
    };
    let plan = api::plan_analysed(&analysis, &no_tables).unwrap();
    let as_json = serde_json::to_value(&plan).unwrap();
    // Final gutters follow raw recovery; they preserve its sheet, scales and view inventory.
    let raw = recovered(&analysis, &no_tables);
    assert_eq!(sheet(&plan), sheet(&raw));
    assert_eq!(
        plan.views
            .iter()
            .map(|v| (&v.id, v.scale))
            .collect::<Vec<_>>(),
        raw.views
            .iter()
            .map(|v| (&v.id, v.scale))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        as_json,
        serde_json::to_value(model::plan_recovered(&analysis, &no_tables).unwrap()).unwrap()
    );
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    assert!(codes(&plan).contains(&"layout_recovery_exhausted"));
    let again = api::plan_analysed(&analysis, &no_tables).unwrap();
    assert_eq!(serde_json::to_value(&again).unwrap(), as_json);
    let reply = api::call("plan", &json!({ "analysis": analysis, "spec": no_tables })).unwrap();
    assert_eq!(reply, as_json);

    let step = corpus::dir()
        .unwrap()
        .join(facts::load("cgb245").binding.file);
    let review = api::review_analysed(&step, &analysis, &no_tables).unwrap();
    let said: Vec<&str> = review.problems.iter().map(|p| p.code.as_str()).collect();
    assert!(said.contains(&"layout_recovery_exhausted"), "{said:?}");

    let pinned = api::plan_analysed(&analysis, &spec(json!({ "size": "A2", "scale": "1:2" })));
    let pinned = pinned.unwrap();
    assert_eq!(sheet(&pinned), ("A2", "1:2"));
    assert!(!codes(&pinned).iter().any(|c| c.starts_with("layout_re")));
    let named = api::plan_analysed(&analysis, &spec(json!({ "size": "A2" }))).unwrap();
    assert_eq!(named.sheet.size, "A2");
}

/// FTC-07 retains its established fitting A2 1:2 drawing when no supported
/// standard sheet meets the stronger readability floor. Larger partial trials
/// must not replace its measurements or buy paper without a clean readable result.
/// FTC-07's smallest complete fit remains A2 at 1:2. Wrapping clears the old
/// pitch-line clash at A1 1:1, but that enlargement lengthens established leaders
/// beyond their unchanged two-millimetre budget, so the complete A2 is retained.
#[test]
fn ftc07_reports_its_readability_refusal_on_the_established_fitted_sheet() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let step = dir.join("nist/nist_ftc_07_asme1_rd.stp");
    let plan = api::plan_from(&step, Features::Native, &Spec::default()).unwrap();
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    let analysis = api::analyse_from(&step, Features::Native).unwrap();
    // The table-free drawing the sheet is chosen by ([`model::recover`]'s hole-table path).
    let never = Spec {
        hole_table: draftwright::hole_table::Setting::Never,
        ..Default::default()
    };
    let chosen = api::plan_analysed(&analysis, &never).unwrap();
    assert_eq!(sheet(&chosen), ("A2", "1:2"));
    assert!(chosen.sheet.basis.starts_with("whole-part paper A2 at 1:2"));
    assert!(draftwright::sheet_fit::readability(&analysis, &chosen).failures() > 0);
    assert!(
        chosen
            .problems
            .iter()
            .any(|p| p.code == "layout_recovery_exhausted" && p.severity == "warning")
    );
    sheet_rule::assert_rule("ftc07", &chosen);
    assert!(
        chosen.sheet.basis.contains("A2 1:2 fill"),
        "{}",
        chosen.sheet.basis
    );
    let a1 = chosen
        .sheet
        .basis
        .split("; ")
        .find(|c| c.starts_with("A1 1:1 "))
        .unwrap_or_else(|| panic!("{}", chosen.sheet.basis));
    assert!(a1.contains("leader length quality veto"), "{a1}");
    assert!(
        chosen
            .sheet
            .basis
            .contains("lengthens an established leader by more than 2 mm")
    );
    assert!(
        !chosen.problems.iter().any(|p| matches!(
            p.code.as_str(),
            "annotation_overlap" | "annotation_ink_overlap" | "annotation_off_sheet"
        )),
        "{:?}",
        chosen.problems
    );
    let first =
        model::plan_escalated(&analysis, &spec(json!({"size":"A3", "scale":"1:5"}))).unwrap();
    let again = model::recover(&analysis, &Spec::default(), first).unwrap();
    assert_eq!(sheet(&again), sheet(&plan));
    assert!(again.sheet.basis.contains("additional view conflicts"));
}

/// Wrapped compound bores retain the established physical route budget after
/// schedules and native recess annotations finish restoring their own geometry.
#[test]
fn wrapped_counterbores_keep_the_existing_short_leader_budget() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let plan = api::plan_from(
        &dir.join("nist/nist_ftc_06_asme1_rd.stp"),
        Features::Native,
        &Spec::default(),
    )
    .unwrap();
    assert_eq!(sheet(&plan), ("A1", "1:2"));
    assert_eq!(
        plan.hole_tables
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["hole_schedule_front"]
    );
    let baseline: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/leader_routes.json")).unwrap();
    for id in [
        "d:f:hole:5/bore.diameter",
        "d:f:hole_pattern:2/bore.diameter",
    ] {
        let d = plan.dimensions.iter().find(|d| d.id == id).unwrap();
        assert_eq!(d.label.lines().count(), 2);
        let p = d.placed.as_ref().unwrap();
        let length: f64 = p
            .lines
            .iter()
            .map(|s| (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]))
            .sum();
        let before = baseline["parts"]["nist_ftc_06_asme1_rd"][id]["length"]
            .as_f64()
            .unwrap();
        assert!(length <= before + 2.0 + 1e-6, "{id}: {before} -> {length}");
        assert!(d.basis.contains("final wrapped-callout polish"));
    }
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "annotation_ink_overlap")
    );
}

/// The port's own gate in `model::recover` (a try with more annotations over a view not their
/// own than the first plan is not taken), on a stand-in part: no corpus part meets it since the
/// layout reserves the bands between the views (a probe of the 30 quality parts on five sheets,
/// seven scales and each view omitted finds every plan with an item over a view also failing a
/// hard check), so mfcadpp 1004 (Rust quiddity's recognition) has its hole `f:hole:1` moved off
/// the part, to y = -60, z = 120, where its y location's line in the side view runs from it across
/// the front and the plan. Pinned at A4 2:1 it has a hard finding and drops three hole items; the
/// recovery's first try, A4 at 1:1, is otherwise valid and complete but lays that line over the
/// two views, which the pinned first plan does not, so the gate passes it over; A4 at 5:1 and A3 at 2:1 fail, and A2 at
/// 2:1, clean, is taken.
#[test]
fn composed_corridors_clear_a_previously_overlapping_off_part_fixture() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let current = api::analyse_from(&dir.join("mfcadpp/1004.step"), Features::Native).unwrap();
    // Keep the historical stand-in fixed while native recognition adds more
    // dimensions. The current recognition/3 unsafe proposal is also replayed
    // through the production recovery in model::recovery_tests.
    use std::io::Read as _;
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/sheet-policy/1004-view-gate.analysis.json.gz"),
        )
        .unwrap(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let analysis: Analysis = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        analysis.binding.source_sha256,
        current.binding.source_sha256
    );
    let mut json = serde_json::to_value(&analysis).unwrap();
    let hole = json["features"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["id"] == "f:hole:1")
        .expect("1004 has f:hole:1");
    hole["parameters"]["location"][1] = json!(-60.0);
    hole["parameters"]["location"][2] = json!(120.0);
    let analysis: Analysis = serde_json::from_value(json).unwrap();
    let over = |p: &Plan| {
        p.problems
            .iter()
            .filter(|q| {
                matches!(
                    q.code.as_str(),
                    "annotation_over_view" | "annotation_line_over_view"
                )
            })
            .count()
    };
    // The try the gate decides, A4 at 1:1, lays a line over two views; the first plan none.
    let gated = model::plan_escalated(&analysis, &spec(json!({ "size": "A4", "scale": "1:1" })));
    assert_eq!(over(&gated.unwrap()), 2);
    let first = model::plan_escalated(&analysis, &spec(json!({ "size": "A4", "scale": "2:1" })));
    let first = first.unwrap();
    assert_eq!(over(&first), 0, "{:?}", codes(&first));
    let before = over(&first);
    let plan = model::recover(&analysis, &Spec::default(), first).unwrap();
    assert!(over(&plan) <= before);
    assert_eq!(sheet(&plan), ("A4", "1:1"), "{}", plan.sheet.basis);
    let gate = sheet_rule::tried(&plan.sheet.basis)
        .into_iter()
        .find(|t| t.size == "A4" && t.scale == "1:1")
        .expect("A4 at 1:1 is tried");
    // The whole corridor now clears both foreign views and fits physically,
    // making the formerly gated proposal a valid complete candidate.
    assert!(gate.valid && gate.over == 0, "{}", plan.sheet.basis);
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "annotation_off_sheet")
    );
    sheet_rule::assert_follows("1004 stand-in", &plan, ("A4", "2:1"));
}

/// A raw seed can have worse routes than its established fitted drawing. The
/// quality comparison must use the fitted reference, preserving exact pairs.
#[test]
fn a_fitted_route_reference_keeps_pair_identity_without_restoring_raw_crossings() {
    fn pairs(p: &Plan) -> std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> {
        p.problems
            .iter()
            .filter(|q| {
                q.severity != "info"
                    && (q.code.starts_with("annotation_")
                        || matches!(
                            q.code.as_str(),
                            "leader_through_feature" | "extension_through_feature"
                        ))
            })
            .map(|q| (q.code.clone(), q.subjects.clone(), q.refs.clone()))
            .collect()
    }
    fn measurements(p: &Plan) -> std::collections::BTreeSet<(String, String)> {
        p.dimensions
            .iter()
            .filter(|d| d.placed.is_some())
            .flat_map(|d| &d.measurements)
            .map(|m| (m.feature.clone(), m.parameter.clone()))
            .collect()
    }
    let Some(dir) = corpus::dir() else { return };
    let a = api::analyse_from(&dir.join("mfcadpp_holdout/419.step"), Features::Native).unwrap();
    let reference = api::plan_analysed(&a, &spec(json!({"size":"A3", "scale":"2:1"}))).unwrap();
    let p = api::plan_analysed(&a, &Spec::default()).unwrap();
    assert_eq!(sheet(&p), ("A3", "1:1"));
    assert_eq!(
        pairs(&reference)
            .iter()
            .filter(|(code, _, _)| code == "annotation_leader_crossing")
            .count(),
        0
    );
    assert!(pairs(&p).is_subset(&pairs(&reference)));
    assert!(measurements(&reference).is_subset(&measurements(&p)));
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);

    for d in p.dimensions.iter().filter(|d| d.kind == "linear") {
        let Some(placed) = &d.placed else { continue };
        if placed.lines.len() < 3 || placed.arrows.len() != 2 {
            continue;
        }
        // Witnesses keep source endpoint order; heads are sorted along the
        // measured span. Match their geometry rather than array positions.
        for [tip, _] in &placed.arrows {
            assert!(
                placed.lines[..2].iter().any(|[a, b]| {
                    let vector = [b[0] - a[0], b[1] - a[1]];
                    let cross = (tip[0] - a[0]) * vector[1] - (tip[1] - a[1]) * vector[0];
                    cross.abs() / vector[0].hypot(vector[1]) < 1e-6
                }),
                "{}: arrow must stay on its witness",
                d.id
            );
        }
    }

    use std::io::Read as _;
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(
        include_bytes!("fixtures/pmi/step/assembly_plate_pin.step.gz").as_slice(),
    )
    .read_to_end(&mut data)
    .unwrap();
    let (_, a) =
        model::analyse_native_step(std::path::Path::new("assembly_plate_pin.step"), &data).unwrap();
    let reference = api::plan_analysed(&a, &spec(json!({"size":"A4", "scale":"1:5"}))).unwrap();
    let p = api::plan_analysed(&a, &Spec::default()).unwrap();
    assert_eq!(sheet(&p), ("A4", "1:5"));
    // A fully pinned pair deliberately keeps a different frame. The route
    // reference is the actual fitted drawing archived from the pre-policy base.
    let saved: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/sheet_policy_routes.json")).unwrap();
    assert_eq!(
        a.binding.source_sha256,
        saved["source_sha256"].as_str().unwrap()
    );
    let expected: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
        serde_json::from_value(saved["pairs"].clone()).unwrap();
    assert_eq!(pairs(&p), expected);
    assert!(measurements(&reference).is_subset(&measurements(&p)));
    assert_eq!(
        p.gdt.iter().filter(|g| g.placed.is_some()).count(),
        reference.gdt.iter().filter(|g| g.placed.is_some()).count()
    );
    assert!(draftwright::sheet_fit::readability(&a, &p).failures() > 0);
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted"
                && q.message.contains("without new crossing pairs"))
    );
}
