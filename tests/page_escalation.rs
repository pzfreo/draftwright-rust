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
//! The corpus tests apply `model::recover` to `model::plan`'s result, as `model::plan_recovered`
//! does for every planning entry point of the library (`api`: the command line, the JSON-lines
//! service and MCP), under the maintainer's directive of 2026-10-09 (nothing off the sheet:
//! escalate scale or sheet as draftwright does, never clip). `model::plan` itself is one try.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

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
        assert_eq!(sheet(&plan), first);
        assert!(!codes(&plan).contains(&"annotation_off_sheet"));
        assert!(!codes(&plan).iter().any(|c| c.starts_with("layout_re")));
        assert!(!plan.sheet.basis.contains("layout recovery"));
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
    let plan = recovered(&analysis, &Spec::default());
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    assert!(!codes(&plan).contains(&"annotation_off_sheet"));
    let said = plan
        .problems
        .iter()
        .find(|p| p.code == "layout_recovery_exhausted")
        .unwrap_or_else(|| panic!("{:?}", codes(&plan)));
    assert_eq!(said.severity, "info");
    assert!(
        said.message
            .starts_with("A2 at 1:2 has 2 hole locations too close to read"),
        "{}",
        said.message
    );
    for tried in [
        // The four side-view hole axes also run off this rejected sheet (ISO 128 furniture).
        "A2 at 1:1 (7 hard layout findings, 2 hole locations too close to read)",
        "A2 at 2:1 (",
        "A1 at 1:1 (",
        "A0 at 1:1 (",
    ] {
        assert!(said.message.contains(tried), "{tried}: {}", said.message);
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
/// choice, A4 at 1:1 (draftwright's too), has ink past its areas (info) and is kept. Until the
/// layout reserved the front's right band for its lanes (stream d-quality-lanes) it also had a
/// label over the side view's box (the seat's z location); no captured part has one now, so the
/// same first plan is given that warning and must still be kept.
#[test]
fn an_info_or_view_box_finding_is_not_a_hard_failure() {
    let Some(analysis) = corpus_part("m1000") else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let plan = recovered(&analysis, &Spec::default());
    assert_eq!(sheet(&plan), ("A4", "1:1"));
    let found = codes(&plan);
    assert!(found.contains(&"annotation_outside_area"), "{found:?}");
    assert!(
        !found.iter().any(|c| c.starts_with("layout_re")),
        "{found:?}"
    );

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
    assert_eq!(sheet(&kept), ("A4", "1:1"));
    let found = codes(&kept);
    assert!(found.contains(&"annotation_over_view"), "{found:?}");
    assert!(
        !found.iter().any(|c| c.starts_with("layout_re")),
        "{found:?}"
    );
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
    let plan = api::plan_analysed(&analysis, &Spec::default()).unwrap();
    let as_json = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        as_json,
        serde_json::to_value(recovered(&analysis, &Spec::default())).unwrap()
    );
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    assert!(codes(&plan).contains(&"layout_recovery_exhausted"));
    let again = api::plan_analysed(&analysis, &Spec::default()).unwrap();
    assert_eq!(serde_json::to_value(&again).unwrap(), as_json);
    let reply = api::call("plan", &json!({ "analysis": analysis })).unwrap();
    assert_eq!(reply, as_json);

    let step = corpus::dir()
        .unwrap()
        .join(facts::load("cgb245").binding.file);
    let review = api::review_analysed(&step, &analysis, &Spec::default()).unwrap();
    let said: Vec<&str> = review.problems.iter().map(|p| p.code.as_str()).collect();
    assert!(said.contains(&"layout_recovery_exhausted"), "{said:?}");

    let pinned = api::plan_analysed(&analysis, &spec(json!({ "size": "A2", "scale": "1:2" })));
    let pinned = pinned.unwrap();
    assert_eq!(sheet(&pinned), ("A2", "1:2"));
    assert!(!codes(&pinned).iter().any(|c| c.starts_with("layout_re")));
    let named = api::plan_analysed(&analysis, &spec(json!({ "size": "A2" }))).unwrap();
    assert_eq!(named.sheet.size, "A2");
}

/// ftc07 with Rust quiddity's recognition (the command line's default) takes draftwright's first
/// choice, A2 at 1:2, directly, now that the layout reserves the band right of the front and the
/// plan for the front's height ladder (stream d-quality-lanes; the ladder ran across the side view
/// and A2 at 1:2 failed on it). From the A3 at 1:5 it took before, two hole locations too close
/// to read (a loss), the recovery tries the larger scales (A3 at 1:1 lays 14 more items over a
/// view than the first) and takes A2 at 1:2, valid and complete. This test held the port's own
/// gate (`model::recover`: a try with more annotations over a view not their own than the first
/// plan is not taken), which A1 at 1:1 met from A3 at 1:5; no default or pinned first choice of the
/// 30 quality parts meets it now (each part from 24 pinned sheets and scales, every valid and
/// complete try lays no more over a view than its first), so it has no corpus case.
#[test]
fn ftc07_takes_draftwrights_first_choice() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let step = dir.join("nist/nist_ftc_07_asme1_rd.stp");
    let plan = api::plan_from(&step, Features::Native, &Spec::default()).unwrap();
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    assert!(
        !codes(&plan).iter().any(|c| c.starts_with("layout_re")),
        "{:?}",
        codes(&plan)
    );

    let analysis = api::analyse_from(&step, Features::Native).unwrap();
    let first = model::plan_escalated(&analysis, &spec(json!({ "size": "A3", "scale": "1:5" })));
    let plan = model::recover(&analysis, &Spec::default(), first.unwrap()).unwrap();
    assert_eq!(sheet(&plan), ("A2", "1:2"));
    let said = plan
        .problems
        .iter()
        .find(|p| p.code == "layout_replanned")
        .unwrap_or_else(|| panic!("{:?}", codes(&plan)));
    assert!(
        said.message
            .contains("A3 at 1:5 has 2 hole locations too close to read")
            && said
                .message
                .contains("16 more items over a view than the first")
            && said
                .message
                .contains("A2 at 1:2 (valid and complete); A2 at 1:2 is used"),
        "{}",
        said.message
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
fn a_try_with_more_ink_over_the_views_than_the_first_is_not_taken() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found");
        return;
    };
    let analysis = api::analyse_from(&dir.join("mfcadpp/1004.step"), Features::Native).unwrap();
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
    let plan = model::recover(&analysis, &Spec::default(), first).unwrap();
    assert_eq!(sheet(&plan), ("A2", "2:1"));
    let said = plan
        .problems
        .iter()
        .find(|p| p.code.starts_with("layout_re"))
        .unwrap_or_else(|| panic!("{:?}", codes(&plan)));
    assert!(
        said.message
            .contains("A4 at 1:1 (valid and complete; 2 more items over a view than the first)"),
        "{}",
        said.message
    );
}
