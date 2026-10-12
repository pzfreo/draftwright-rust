//! Plan-time schedules replace crowded hole ladders, while the rescue remains independent.
use draftwright::{
    model::{self, Analysis, Plan, Spec},
    text::Font,
};
#[path = "common/corpus.rs"]
mod corpus;
use std::path::Path;
use std::sync::OnceLock;

fn plate() -> &'static (haecceity::Part, Analysis) {
    static PART: OnceLock<(haecceity::Part, Analysis)> = OnceLock::new();
    PART.get_or_init(|| {
        model::analyse_native(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/parts/scattered_holes.step"),
        )
        .unwrap()
    })
}
fn setting(value: &str) -> Spec {
    model::edit(
        &Spec::default(),
        &serde_json::json!([{"op":"set","path":"hole_table","value":value}]),
    )
    .unwrap()
}
fn tables(p: &Plan) -> serde_json::Value {
    serde_json::to_value(p)
        .unwrap()
        .get("hole_tables")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]))
}
#[test]
fn crowding_selects_a_schedule_before_any_item_is_dropped() {
    let p = model::plan_escalated(&plate().1, &Spec::default()).unwrap();
    assert!(tables(&p).as_array().is_some_and(|t| !t.is_empty()));
    assert!(!p.problems.iter().any(|p| p.code.ends_with("_dropped")));
}
#[test]
fn the_spec_selects_always_never_and_default_auto() {
    let a = model::plan_escalated(&plate().1, &setting("always")).unwrap();
    let n = model::plan_escalated(&plate().1, &setting("never")).unwrap();
    assert!(tables(&a).as_array().is_some_and(|t| !t.is_empty()));
    assert!(tables(&n).as_array().is_some_and(|t| t.is_empty()));
    assert!(
        model::edit(
            &Spec::default(),
            &serde_json::json!([{"op":"set","path":"hole_table","value":"sometimes"}])
        )
        .is_err()
    );
    assert_eq!(setting("auto"), Spec::default());
}
#[test]
fn a_pinned_callout_keeps_its_ink_when_others_are_tabulated() {
    let mut spec = setting("never");
    let before = model::plan_escalated(&plate().1, &spec).unwrap();
    let held = before
        .dimensions
        .iter()
        .find(|d| d.kind == "callout" && d.id.starts_with("d:f:hole:"))
        .unwrap();
    spec = model::edit(&spec, &serde_json::json!([
        {"op":"set","path":"hole_table","value":"always"},
        {"op":"set","path":format!("dimensions/{}/place",draftwright::items::key(&held.id)),"value":{"pinned":true}}
    ])).unwrap();
    let p = model::plan_escalated(&plate().1, &spec).unwrap();
    let pin = p.dimensions.iter().find(|d| d.id == held.id).unwrap();
    assert!(pin.placed.is_some());
    assert!(!matches!(
        pin.state,
        draftwright::items::State::Covered { .. }
    ));
    assert!(tables(&p).as_array().is_some_and(|t| !t.is_empty()));
}
#[test]
fn rows_and_tags_credit_hole_size_count_and_both_locations() {
    let (part, analysis) = plate();
    let p = model::plan_escalated(analysis, &setting("always")).unwrap();
    let t = tables(&p);
    assert_eq!(t[0]["rows"].as_array().unwrap().len(), 18);
    assert!(
        t[0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["cells"].as_array().unwrap().len() == 5
                && r["measurements"].as_array().unwrap().len() >= 3)
    );
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let drawing = model::render(part, analysis, &p, &font).unwrap();
    for finding in draftwright::lint::lint(analysis, &drawing) {
        assert!(
            !matches!(
                finding.code.as_str(),
                "hole_requirement_missing" | "feature_not_located"
            ),
            "{}",
            finding.message
        );
    }
}
#[test]
fn the_dense_acceptance_parts_have_no_hole_location_tower() {
    let Some(corpus) = corpus::dir() else { return };
    for name in [
        "nist_ctc_02_asme1_rc",
        "nist_ctc_04_asme1_rd",
        "nist_ftc_06_asme1_rd",
    ] {
        let (_, analysis) =
            model::analyse_native(&Path::new(&corpus).join(format!("nist/{name}.stp"))).unwrap();
        let p = model::plan_escalated(&analysis, &Spec::default()).unwrap();
        assert!(
            tables(&p).as_array().is_some_and(|t| !t.is_empty()),
            "{name}"
        );
        let read = draftwright::bridge::holes::read(&analysis);
        for tag in p.balloons.iter().filter(|b| b.certifying) {
            let h = read
                .iter()
                .find(|(f, _)| tag.features.contains(&f.id))
                .unwrap()
                .1
                .as_ref()
                .unwrap();
            let v = p.views.iter().find(|v| v.id == tag.view).unwrap();
            let frame = haecceity::hlr::View::new(v.direction, v.up).unwrap();
            let centre = frame.map(std::array::from_fn(|k| {
                (analysis.envelope[0][k] + analysis.envelope[1][k]) * 0.5
            }));
            let at = frame.map(h.at);
            let hole = [
                v.at[0] + v.scale * (at[0] - centre[0]),
                v.at[1] + v.scale * (at[1] - centre[1]),
            ];
            let width = tag.tag.len() as f64 * draftwright::dims::TEXT * 0.6;
            let clearance = ((tag.at[0] - hole[0]).abs() - width * 0.5)
                .max(0.0)
                .hypot(((tag.at[1] - hole[1]).abs() - draftwright::dims::TEXT * 0.5).max(0.0));
            let opening = [
                h.diameter,
                h.counterbore.map_or(0.0, |r| r.diameter),
                h.spotface.map_or(0.0, |r| r.diameter),
                h.countersink.map_or(0.0, |r| r.diameter),
            ]
            .into_iter()
            .fold(0.0, f64::max);
            assert!(
                clearance > opening * v.scale * 0.5,
                "{name} {} crosses its hole's outer seat",
                tag.tag
            );
        }
        for v in ["v:front", "v:plan", "v:side"] {
            let n = p
                .dimensions
                .iter()
                .filter(|d| {
                    d.view == v
                        && d.placed.is_some()
                        && d.measurements.iter().any(|m| {
                            m.feature.starts_with("f:hole")
                                && m.parameter.starts_with("location.length.")
                        })
                })
                .count();
            assert!(n <= 8, "{name} {v}: {n} location tiers");
        }
    }
}

#[test]
fn an_omitted_hole_location_is_not_silently_reintroduced_by_a_table() {
    let analysis = &plate().1;
    let first = model::plan(analysis, &Spec::default()).unwrap();
    let id = first
        .dimensions
        .iter()
        .find(|d| d.id.starts_with("d:f:hole:") && d.id.ends_with("/location.length.x"))
        .unwrap()
        .id
        .clone();
    let spec = model::edit(&setting("always"), &serde_json::json!([{"op":"set", "path":format!("dimensions/{}/omit", draftwright::items::key(&id)),"value":true}])).unwrap();
    let plan = model::plan_escalated(analysis, &spec).unwrap();
    let feature = id.strip_prefix("d:").unwrap().split('/').next().unwrap();
    assert!(tables(&plan).as_array().unwrap().iter().all(|t| {
        t["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["feature"] != feature)
    }));
    assert!(
        plan.omitted
            .iter()
            .any(|o| o.id == id && o.state == draftwright::items::State::OmittedByUser)
    );
}

#[test]
fn a_changed_coordinate_cannot_borrow_a_number_from_another_cell() {
    let (part, analysis) = plate();
    let plan = model::plan_escalated(analysis, &setting("always")).unwrap();
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let mut drawing = model::render(part, analysis, &plan, &font).unwrap();
    let a = drawing
        .annotations
        .iter()
        .find(|a| a.name.starts_with("hole_schedule_") && a.name.ends_with("/x"))
        .unwrap();
    let feature = a.claims[0].feature.clone();
    for &run in &a.runs {
        drawing.text[run].text = "9999".into();
    }
    let findings = draftwright::lint::lint(analysis, &drawing);
    assert!(
        findings
            .iter()
            .any(|f| f.code == "feature_not_located" && f.subjects.contains(&feature)),
        "{findings:?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.code == "hole_requirement_missing")
    );
}

#[test]
fn a_schedule_without_a_proven_datum_corner_is_refused() {
    let mut analysis = plate().1.clone();
    analysis.principal_profiles.clear();
    let plan = model::plan_escalated(&analysis, &setting("always")).unwrap();
    assert!(tables(&plan).as_array().unwrap().is_empty());
    assert!(
        plan.problems
            .iter()
            .any(|p| p.code == "hole_table_unavailable")
    );
    assert!(
        plan.dimensions
            .iter()
            .any(|d| d.kind == "callout" && d.placed.is_some())
    );
}

#[test]
fn approved_fit_limits_survive_in_each_size_cell() {
    let analysis = &plate().1;
    let ops: Vec<_> = analysis.features.iter().filter(|f| f.kind == "hole").map(|f| serde_json::json!({"op":"set","path":format!("features/{}/fit",draftwright::items::key(&f.id)),"value":"H7"})).collect();
    let spec = model::edit(&setting("always"), &serde_json::json!(ops)).unwrap();
    let plan = model::plan_escalated(analysis, &spec).unwrap();
    assert!(
        tables(&plan)[0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["cells"][1].as_str().unwrap().contains("H7 (+"))
    );
}

#[test]
fn semantic_pmi_callouts_remain_source_carrying_beside_the_schedule() {
    let (_, analysis) = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/editor_pmi.step"),
    )
    .unwrap();
    let plan = model::plan_escalated(&analysis, &Spec::default()).unwrap();
    assert!(tables(&plan).as_array().is_some_and(|t| !t.is_empty()));
    let semantic: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements.iter().any(|m| {
                m.feature.starts_with("pmi:")
                    && matches!(m.parameter.as_str(), "diameter" | "thread")
            })
        })
        .collect();
    assert_eq!(semantic.len(), 3);
    assert!(semantic.iter().all(|d| d.placed.is_some()));
}

#[test]
fn a_bare_pin_on_a_table_covered_location_does_not_restore_the_ladder() {
    let analysis = &plate().1;
    let base = model::plan_recovered(analysis, &Spec::default()).unwrap();
    let covered = base.omitted.iter().find(|o| o.id.starts_with("d:f:hole:") && o.id.ends_with("/location.length.x") && matches!(&o.state,draftwright::items::State::Covered {by} if by.starts_with("hole_schedule_"))).unwrap();
    let spec = model::edit(&Spec::default(), &serde_json::json!([{"op":"set","path":format!("dimensions/{}/place",draftwright::items::key(&covered.id)),"value":{"pinned":true}}])).unwrap();
    let after = model::plan_recovered(analysis, &spec).unwrap();
    assert!(after.omitted.iter().any(
        |o| o.id == covered.id && matches!(o.state, draftwright::items::State::Covered { .. })
    ));
    assert!(
        after
            .problems
            .iter()
            .any(|p| p.code == "pinned_item_not_drawn")
    );
    for d in base.dimensions.iter().filter(|d| d.kind == "linear") {
        assert_eq!(
            d.placed,
            after
                .dimensions
                .iter()
                .find(|a| a.id == d.id)
                .unwrap()
                .placed,
            "{}",
            d.id
        );
    }
}

#[test]
fn non_hole_pins_keep_the_settled_schedule_and_isometric_frame() {
    let Some(corpus) = corpus::dir() else { return };
    let (_, analysis) =
        model::analyse_native(&Path::new(&corpus).join("nist/nist_ftc_08_asme1_rc.stp")).unwrap();
    let base = model::plan_recovered(&analysis, &Spec::default()).unwrap();
    assert!(!tables(&base).as_array().unwrap().is_empty());
    let ops: Vec<_> = [
        "d:f:slot:3/slot_width.length",
        "d:f:fillet:7/fillet.radius",
        "d:f:slot:4/location_slot.length",
    ]
    .iter()
    .map(|id| serde_json::json!({"op":"set","path":format!("dimensions/{}/place",draftwright::items::key(id)),"value":{"pinned":true}}))
    .collect();
    let spec = model::edit(&Spec::default(), &serde_json::json!(ops)).unwrap();
    let after = model::plan_recovered(&analysis, &spec).unwrap();
    let before = tables(&base);
    let after_tables = tables(&after);
    assert_eq!(
        before.as_array().unwrap().len(),
        after_tables.as_array().unwrap().len()
    );
    for (before, after) in before
        .as_array()
        .unwrap()
        .iter()
        .zip(after_tables.as_array().unwrap())
    {
        // The basis reports pre-table leader conflicts, which held labels can change.
        // The printed statements, datum and actual table geometry must stay settled.
        for field in ["id", "origin", "rows", "covered", "state"] {
            assert_eq!(before[field], after[field], "{field}");
        }
        assert_eq!(before["blocks"][0]["placed"], after["blocks"][0]["placed"]);
    }
    for view in &base.views {
        let held = after.views.iter().find(|v| v.id == view.id).unwrap();
        assert_eq!((view.at, view.scale), (held.at, held.scale), "{}", view.id);
    }
    for (id, edit) in &spec.dimensions {
        assert!(edit.place.as_ref().unwrap().pinned);
        assert_eq!(
            base.dimensions.iter().find(|d| d.id == *id).unwrap().placed,
            after
                .dimensions
                .iter()
                .find(|d| d.id == *id)
                .unwrap()
                .placed,
            "{id}"
        );
    }
}

#[test]
fn table_recess_and_depth_symbols_are_drawn_inside_the_size_cell() {
    let Some(corpus) = corpus::dir() else { return };
    let (part, analysis) =
        model::analyse_native(&Path::new(&corpus).join("nist/nist_ftc_06_asme1_rd.stp")).unwrap();
    let plan = model::plan_recovered(&analysis, &Spec::default()).unwrap();
    let table = tables(&plan);
    assert!(
        table[0]["rows"][0]["cells"][1]
            .as_str()
            .unwrap()
            .contains('⌴')
    );
    let block = &table[0]["blocks"][0]["placed"];
    let at = &block["cells_at"][2][1];
    let cell = [
        block["columns"][1].as_f64().unwrap(),
        at[1].as_f64().unwrap() - draftwright::tables::ROW * 0.5,
        block["columns"][2].as_f64().unwrap(),
        at[1].as_f64().unwrap() + draftwright::tables::ROW * 0.5,
    ];
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let strokes = drawing
        .layers
        .iter()
        .flat_map(|l| &l.items)
        .filter(|i| {
            let draftwright::output::Item::Edge(draftwright::output::Curve::Line(a, b)) = i else {
                return false;
            };
            [a, b].iter().all(|p| {
                p[0] > cell[0] + 1e-6
                    && p[0] < cell[2] - 1e-6
                    && p[1] > cell[1] + 1e-6
                    && p[1] < cell[3] - 1e-6
            })
        })
        .count();
    assert!(
        strokes >= 3,
        "size cell has no drafting-symbol strokes: {strokes}"
    );
}

#[test]
fn the_cgb245_view_pin_reproducer_keeps_its_schedule() {
    let Some(corpus) = corpus::dir() else { return };
    let (_, analysis) =
        model::analyse_native(&corpus.join("cadgenbench_inputs/cgb245.step")).unwrap();
    let base = model::plan_recovered(&analysis, &Spec::default()).unwrap();
    assert!(!tables(&base).as_array().unwrap().is_empty());
    let ordinary = model::plan_recovered(&analysis, &setting("never")).unwrap();
    let iso = base.views.iter().find(|v| v.id == "v:iso").unwrap();
    let before = ordinary.views.iter().find(|v| v.id == "v:iso").unwrap();
    assert_eq!(
        (iso.at, iso.area, iso.scale),
        (before.at, before.area, before.scale)
    );
    for k in 0..2 {
        assert!(iso.at[k] + iso.scale * analysis.iso_bounds[k] >= draftwright::layout::MARGIN);
        assert!(
            iso.at[k] + iso.scale * analysis.iso_bounds[k + 2]
                <= base.sheet.page[k] - draftwright::layout::MARGIN
        );
    }
    let mut ops: Vec<_> = ["d:envelope/height.length", "d:f:slot:1/location_slot.length", "d:f:slot:1/slot_length.length", "d:f:slot:1/slot_width.length", "d:step_level/step_height.length.h13_226", "d:step_level/step_height.length.h9_226"].iter().map(|id| serde_json::json!({"op":"set", "path":format!("dimensions/{}/place",draftwright::items::key(id)), "value":{"pinned":true}})).collect();
    for (view, side, align) in [("plan", "above", "middle"), ("iso", "right", "start")] {
        ops.push(serde_json::json!({"op":"set", "path":format!("views/{view}/place"), "value":{"relative_to":"front", "side":side, "align":align, "gap":0, "pinned":true}}));
    }
    let spec = model::edit(&Spec::default(), &serde_json::json!(ops)).unwrap();
    let after = model::plan_recovered(&analysis, &spec).unwrap();
    assert_eq!(tables(&base)[0]["rows"], tables(&after)[0]["rows"]);
    for id in spec.dimensions.keys() {
        assert_eq!(
            base.dimensions.iter().find(|d| &d.id == id).unwrap().placed,
            after
                .dimensions
                .iter()
                .find(|d| &d.id == id)
                .unwrap()
                .placed,
            "{id}"
        );
    }
}

#[test]
fn rounded_datums_and_stacked_openings_are_unambiguous() {
    let Some(corpus) = corpus::dir() else { return };
    for name in [
        "nist_ctc_02_asme1_rc",
        "nist_ctc_04_asme1_rd",
        "nist_ctc_05_asme1_rd",
    ] {
        let (_, analysis) =
            model::analyse_native(&corpus.join(format!("nist/{name}.stp"))).unwrap();
        let plan = model::plan_recovered(&analysis, &Spec::default()).unwrap();
        assert!(!plan.hole_tables.is_empty(), "{name}");
        for table in &plan.hole_tables {
            assert_eq!(table.origin, Some(analysis.envelope[0]));
            for row in &table.rows {
                assert!(
                    row.cells[1]
                        .split_whitespace()
                        .all(|t| !(t.parse::<f64>().is_ok() && t.ends_with(".0"))),
                    "{name}: redundant native size precision"
                );
                assert!(
                    row.cells[2..4].iter().all(|c| !c.ends_with(".0")),
                    "{name}: redundant coordinate precision"
                );
                assert!(
                    row.measurements
                        .iter()
                        .filter(|m| m.parameter.starts_with("location.length."))
                        .all(|m| m.value >= -1e-6)
                );
                for other in &table.rows {
                    if row.feature != other.feature && row.cells[2..4] == other.cells[2..4] {
                        assert_ne!(
                            row.cells[4], other.cells[4],
                            "{name}: indistinguishable entry faces"
                        );
                        assert!(
                            plan.balloons
                                .iter()
                                .any(|b| b.features.contains(&row.feature)
                                    && b.features.contains(&other.feature))
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn an_incomplete_bolt_circle_states_pcd_and_each_angle() {
    let Some(corpus) = corpus::dir() else { return };
    let (_, analysis) =
        model::analyse_native(&corpus.join("nist/nist_ctc_05_asme1_rd.stp")).unwrap();
    let plan = model::plan_recovered(&analysis, &Spec::default()).unwrap();
    let table = plan
        .hole_tables
        .iter()
        .find(|t| t.view.as_deref() == Some("v:plan"))
        .unwrap();
    let circle = table.pcd.as_ref().unwrap();
    assert!((circle.diameter - 444.5).abs() < 1e-4);
    assert_eq!(circle.members.len(), 10);
    let mut angles: Vec<_> = table
        .rows
        .iter()
        .filter(|r| circle.members.contains(&r.feature))
        .map(|r| r.cells[5].trim_end_matches('°').parse::<f64>().unwrap())
        .collect();
    angles.sort_by(f64::total_cmp);
    assert_eq!(
        angles,
        [30., 60., 90., 120., 150., 210., 240., 270., 300., 330.]
    );
    assert!(table.blocks[0].rows[0][1].contains("PCD"));
    assert!(
        !table.blocks[0]
            .rows
            .iter()
            .flatten()
            .any(|c| c.contains("EQ SP"))
    );
}

#[test]
fn schedule_labels_clear_the_rendered_part_and_other_annotations() {
    let Some(corpus) = corpus::dir() else { return };
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    for file in [
        "nist/nist_ctc_02_asme1_rc.stp",
        "nist/nist_ctc_05_asme1_rd.stp",
        "cadgenbench_inputs/cgb245.step",
    ] {
        let (part, analysis) = model::analyse_native(&corpus.join(file)).unwrap();
        let plan = model::plan_recovered(&analysis, &Spec::default()).unwrap();
        let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
        for issue in draftwright::lint::structural::check(&drawing) {
            if issue
                .annotation
                .as_ref()
                .is_some_and(|a| a.starts_with("hole_schedule_"))
                || issue
                    .related
                    .iter()
                    .any(|a| a.starts_with("hole_schedule_"))
            {
                assert!(
                    !matches!(
                        issue.code.as_str(),
                        "view_annotation_overlap" | "annotation_overlap" | "annotation_ink_overlap"
                    ),
                    "{file}: {issue:?}"
                );
            }
        }
    }
}

#[test]
fn a_changed_entry_cell_is_reported_even_when_xy_is_correct() {
    let (part, analysis) = plate();
    let plan = model::plan_recovered(analysis, &setting("always")).unwrap();
    let font = Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let mut drawing = model::render(part, analysis, &plan, &font).unwrap();
    assert!(
        !draftwright::lint::lint(analysis, &drawing)
            .iter()
            .any(|i| i.code == "hole_table_entry_invalid")
    );
    let entry = drawing
        .annotations
        .iter()
        .find(|a| a.name.ends_with("/entry"))
        .unwrap();
    let feature = entry.claims[0].feature.clone();
    for &run in &entry.runs {
        drawing.text[run].text = "9999 / +X".into();
    }
    assert!(
        draftwright::lint::lint(analysis, &drawing)
            .iter()
            .any(|i| i.code == "hole_table_entry_invalid" && i.subjects.contains(&feature))
    );
}
