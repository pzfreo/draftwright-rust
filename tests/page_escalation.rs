//! ISSUE-1299-a: automatic layout recovery is bounded and ordered: a larger scale on the same
//! page before a larger standard sheet, hard layout validity before completeness, a complete
//! drawing never spends a larger sheet, an explicit page is never escalated.
//!
//! draftwright's six tests (`tests/test_issue_1299_page_escalation.py`), as ported:
//! - the four built on stand-in drawings (`test_hard_layout_precedes_completeness_…`,
//!   `test_recovery_does_not_trade_a_required_drop_for_unreadable_ink`,
//!   `test_complete_detail_drawing_stays_on_its_original_page`, and the order of
//!   `test_crowded_head_uses_larger_scale_before_spending_the_sheet`) drive `layout::recover`
//!   with stand-in findings, as they drive draftwright's ladder with stand-in builds;
//! - `test_explicit_a4_remains_fixed_instead_of_escalating` and
//!   `test_exact_grm03_recovers_all_axial_stations_on_a4_with_pmi_off` run on corpus parts.
//!   GRM-03 is the corpus's `grm03_thumbwheel_drive_screw.step` (draftwright's fixture is its
//!   AP242 PMI twin, with PMI off). draftwright reaches 5:1 by measuring larger scales against the
//!   recovery detail its estimate reserved (`recover_detail`); the port plans no detail views and
//!   its 2:1 plan has no hard layout finding, so it keeps 2:1 (verdict: not-applicable). The
//!   five-step profile the crowded-head and explicit-A4 tests build with build123d has no STEP
//!   here; their order and pinning are held on stand-ins and corpus parts instead.
//!
//! The corpus tests apply `model::recover` to `model::plan`'s result: `model::plan` does not apply
//! it yet, as on the captured parts it would act only on the port's placement residuals and move
//! cgb245 and ctc01 off draftwright's sheets (docs/next-session.md).

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use draftwright::layout::{self, Attempt, Findings, Retry};
use draftwright::model::{self, Analysis, Plan, Spec};

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
    let attempts = layout::recover(2.0, A4, false, hard(), &mut |r| {
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
    let attempts = layout::recover(10.0, A3, false, both, &mut |r| {
        Some(if r == Retry::Scale(5.0) {
            Findings::default()
        } else {
            both
        })
    });
    assert_eq!(tried(&attempts), [(Retry::Scale(5.0), true)]);

    // Each rejected try: a hard failure without the loss, then the loss without a hard failure.
    let mut answers = vec![hard(), dropped()].into_iter();
    let attempts = layout::recover(2.0, A3, false, both, &mut |_| {
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
    let attempts = layout::recover(2.0, A4, false, dropped(), &mut |r| {
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
    let attempts = layout::recover(1.0, A4, false, dropped(), &mut |r| {
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
    let attempts = layout::recover(2.0, A4, false, Findings::default(), &mut |_| {
        panic!("a valid, complete plan is re-planned")
    });
    assert!(attempts.is_empty());
}

/// The recovery is bounded: at most two scales and the four larger sheets, and a plan that
/// cannot be made (`None`) is passed over, never taken. When nothing is clean, nothing is taken.
#[test]
fn the_recovery_is_bounded_and_keeps_the_first_plan_when_nothing_is_clean() {
    for first in [hard(), dropped()] {
        let attempts = layout::recover(1.0, A4, false, first, &mut |r| {
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
        let attempts = layout::recover(2.0, A4, true, first, &mut |_| Some(hard()));
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

/// The plan with draftwright's recovery applied (`model::recover`; `model::plan` does not apply it
/// yet).
fn recovered(analysis: &Analysis, spec: &Spec) -> Plan {
    let first = model::plan(analysis, spec).unwrap();
    model::recover(analysis, spec, first).unwrap()
}

fn codes(plan: &Plan) -> Vec<&str> {
    plan.problems.iter().map(|p| p.code.as_str()).collect()
}

fn sheet(plan: &Plan) -> (&str, &str) {
    (plan.sheet.size.as_str(), plan.sheet.scale.as_str())
}

/// On the corpus: cgb245's first choice (A2 at 1:2, draftwright's sheet too) has one label off
/// the sheet (d:f:hole:1's y location), and ctc01's (A3 at 1:5, draftwright's too) one; the
/// recovery tries the nearest smaller scale first, which is valid on both: A2 at 1:5 and A3 at
/// 1:10, each said (`layout_replanned`), the sheet's basis naming the recovery, the spec
/// unchanged. draftwright keeps 1:2 and 1:5: its placement has no label off the sheet there
/// (verdict: equivalent rule, differing inputs; the off-sheet labels are the port's placement
/// residuals, `known_placement.json`).
#[test]
fn a_plan_with_ink_off_the_sheet_is_re_planned_at_a_smaller_scale_on_its_sheet() {
    for (name, first, after) in [
        ("cgb245", ("A2", "1:2"), ("A2", "1:5")),
        ("ctc01", ("A3", "1:5"), ("A3", "1:10")),
    ] {
        let Some(analysis) = corpus_part(name) else {
            eprintln!("SKIPPED: the corpus is not found");
            return;
        };
        let plan = recovered(&analysis, &Spec::default());
        assert_eq!(sheet(&plan), after, "{name}");
        let said = plan
            .problems
            .iter()
            .find(|p| p.code == "layout_replanned")
            .unwrap_or_else(|| panic!("{name}: {:?}", codes(&plan)));
        assert_eq!(said.severity, "info");
        assert!(
            said.message
                .contains(&format!("{} at {}", first.0, first.1))
                && said.message.contains("annotation_off_sheet"),
            "{name}: {}",
            said.message
        );
        assert!(plan.sheet.default && plan.sheet.basis.contains("layout recovery"));
        assert!(plan.spec.sheet.size.is_none() && plan.spec.sheet.scale.is_none());
        assert!(!codes(&plan).contains(&"annotation_off_sheet"), "{name}");

        // A scale the spec names is never re-planned: the label stays off the sheet, reported.
        let pinned = recovered(&analysis, &spec(serde_json::json!({ "scale": first.1 })));
        assert_eq!(sheet(&pinned), first, "{name}");
        assert!(codes(&pinned).contains(&"annotation_off_sheet"));
        assert!(!codes(&pinned).contains(&"layout_replanned"));

        // A sheet the spec names is kept; its scale is recovered on it.
        let named = recovered(&analysis, &spec(serde_json::json!({ "size": first.0 })));
        assert_eq!(sheet(&named), after, "{name}");
    }
}

/// draftwright's `test_exact_grm03_recovers_all_axial_stations_on_a4_with_pmi_off`, and the
/// complete-drawing rule on real parts: a plan with no hard layout finding keeps its first
/// choice and says nothing of recovery. GRM-03 stays A4 at 2:1 (draftwright: 5:1, through the
/// recovery detail the port does not plan; not-applicable); m10127 stays A4 at 1:1 (draftwright
/// re-planned to A3 at 2:1 on its own drawing's ink overlap, which the port's plan does not have;
/// undetermined until placement agrees).
#[test]
fn a_plan_without_a_hard_failure_keeps_its_first_choice() {
    for (name, kept) in [("grm03", ("A4", "2:1")), ("m10127", ("A4", "1:1"))] {
        let Some(analysis) = corpus_part(name) else {
            eprintln!("SKIPPED: the corpus is not found");
            return;
        };
        let plan = recovered(&analysis, &Spec::default());
        assert_eq!(sheet(&plan), kept, "{name}");
        assert!(
            !codes(&plan).iter().any(|c| c.starts_with("layout_re")),
            "{name}: {:?}",
            codes(&plan)
        );
    }
}

/// The non-veto case of `test_recovery_does_not_trade_a_required_drop_for_unreadable_ink`: an
/// info observation is no hard failure, and nor is the port's `annotation_over_view` (judged by
/// the view's box; draftwright's `view_annotation_overlap` by its line-work). m1000's first
/// choice, A4 at 1:1 (draftwright's too), has a label over a view's box and ink past its areas
/// (info) and is kept.
#[test]
fn an_info_or_view_box_finding_is_not_a_hard_failure() {
    let Some(analysis) = corpus_part("m1000") else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let plan = recovered(&analysis, &Spec::default());
    assert_eq!(sheet(&plan), ("A4", "1:1"));
    let found = codes(&plan);
    assert!(found.contains(&"annotation_over_view"), "{found:?}");
    assert!(found.contains(&"annotation_outside_area"), "{found:?}");
    assert!(
        !found.iter().any(|c| c.starts_with("layout_re")),
        "{found:?}"
    );
}
