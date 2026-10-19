//! Maintainer sheet policy, 2026-10-17: paper cost, readability and measured content.
use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::json;
use std::path::Path;

fn part() -> Analysis {
    model::analyse(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/prismatic.step"),
    )
    .unwrap()
    .1
}
fn named(size: &str, scale: &str) -> Spec {
    serde_json::from_value(json!({"sheet": {"size": size, "scale": scale}})).unwrap()
}

#[test]
fn compact_manufacturing_defaults_keep_identity_plate_routes_and_coordinates() {
    let recorded: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/sheet-policy/release-main-routes.json"
    ))
    .unwrap();
    for name in ["plate_intent.step.gz", "plate_intent_rewritten.step.gz"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/identity")
            .join(name);
        let a = draftwright::api::analyse_from(&path, draftwright::api::Features::Native).unwrap();
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        let baseline = &recorded["parts"][format!("fixtures/identity/{name}")];
        assert_eq!(a.binding.source_sha256, baseline["source_sha256"]);
        assert_eq!((&*p.sheet.size, &*p.sheet.scale), ("A4", "1:5"));
        assert!(p.sheets.is_empty());
        assert!(p.gdt.iter().all(|g| g.placed.is_some()));
        let pairs: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
            serde_json::from_value(baseline["pairs"].clone()).unwrap();
        for q in p.problems.iter().filter(|q| {
            matches!(
                q.code.as_str(),
                "annotation_leader_crossing"
                    | "extension_through_feature"
                    | "leader_through_feature"
            )
        }) {
            assert!(pairs.contains(&(q.code.clone(), q.subjects.clone(), q.refs.clone())));
        }
        let required: Vec<draftwright::dims::Measurement> =
            serde_json::from_value(baseline["measurements"].clone()).unwrap();
        let mut claims: Vec<_> = p
            .dimensions
            .iter()
            .filter(|d| d.placed.is_some())
            .flat_map(|d| &d.measurements)
            .collect();
        claims.extend(
            p.notes
                .iter()
                .filter(|n| n.placed.is_some())
                .flat_map(|n| &n.measurements),
        );
        for table in p.hole_table.iter().chain(&p.hole_tables) {
            assert!(table.blocks.iter().all(|b| b.placed.is_some()));
            claims.extend(table.rows.iter().flat_map(|r| &r.measurements));
            claims.extend(&table.covered);
        }
        assert!(required.iter().all(|m| claims.contains(&m)));
    }
}

#[test]
fn clean_readable_a4_precedes_larger_paper() {
    let a = part();
    let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
    let p = model::recover(&a, &Spec::default(), first).unwrap();
    assert_eq!(p.sheet.size, "A4", "{}", p.sheet.basis);
    assert!(p.sheet.basis.contains("smallest fit"), "{}", p.sheet.basis);
}

#[test]
fn complete_underfilled_sheet_is_enlarged_and_fill_candidates_are_explained() {
    let a = part();
    let first = model::plan_escalated(&a, &named("A4", "1:10")).unwrap();
    let p = model::recover(&a, &Spec::default(), first).unwrap();
    assert!(p.sheet.scale_factor > 0.1);
    assert!(p.sheet.basis.contains("fill"), "{}", p.sheet.basis);
    assert!(p.sheet.basis.contains("floor 60%"), "{}", p.sheet.basis);
}

#[test]
fn named_sheet_overrides_paper_order_and_chooses_largest_readable_scale() {
    let a = part();
    let mut s = Spec::default();
    s.sheet.size = Some("A2".into());
    let first = model::plan_escalated(&a, &named("A2", "1:10")).unwrap();
    let p = model::recover(&a, &s, first).unwrap();
    assert_eq!(p.sheet.size, "A2");
    assert!(p.sheet.scale_factor > 0.1);
    assert!(p.sheet.basis.contains("fill"), "{}", p.sheet.basis);
    let s = named("A2", "1:10");
    let p = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A2", "1:10")
    );
}

#[test]
fn invalid_high_fill_never_beats_a_valid_complete_sheet() {
    let a = part();
    let first = model::plan_escalated(&a, &named("A4", "10:1")).unwrap();
    assert!(
        first
            .problems
            .iter()
            .any(|p| p.code == "page_fit_uncertain")
    );
    let p = model::recover(&a, &Spec::default(), first).unwrap();
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_off_sheet"
            | "annotation_overlap"
            | "annotation_ink_overlap"
            | "page_fit_uncertain"
            | "views_do_not_fit"
    )));
    assert!(p.sheet.basis.contains("fill"), "{}", p.sheet.basis);
}

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/sheet_rule.rs"]
mod sheet_rule;

#[test]
fn grm03_uses_an_a4_detail_for_its_local_flange_floor() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let a = model::analyse_native(&dir.join("gramel/GRM-03_thumbwheel_drive_screw.step"))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "10:1")
    );
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
    assert!(
        p.sheet.basis.contains("A4 ")
            && p.sheet
                .basis
                .contains("leader/text gains a 1 mm padded extension-line contact"),
        "{}",
        p.sheet.basis
    );
    // Preserve the original 60–85% guard on its exact captured drawing.
    // Failed leader association, rather than fill, requires A3.
    let roomy: model::Plan = serde_json::from_reader(flate2::read::GzDecoder::new(
        include_bytes!("fixtures/sheet-policy/grm03-a3-before.plan.json.gz").as_slice(),
    ))
    .unwrap();
    assert_eq!(roomy.binding.source_sha256, a.binding.source_sha256);
    let fill = draftwright::sheet_fit::fill(&a, &roomy);
    assert!((0.60..=0.85).contains(&fill), "GRM-03 fill {fill:.4}");
    // The old drawing keeps the unchanged 60–85% measurement guard above.
    // The automatic A4 result saturates below that guide rather than buying A3.
    assert!(p.sheet.basis.contains("fill floor not attainable"));
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    let without_details: Spec =
        serde_json::from_value(json!({"sheet":{"automatic_details":false}})).unwrap();
    let ordinary = model::plan_recovered(&a, &without_details).unwrap();
    assert_eq!(
        (ordinary.sheet.size.as_str(), ordinary.sheet.scale.as_str()),
        ("A3", "10:1")
    );
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    sheet_rule::assert_rule("grm03", &p);
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_off_sheet"
            | "annotation_outside_area"
            | "annotation_overlap"
            | "annotation_ink_overlap"
            | "page_fit_uncertain"
    )));
}

fn manufacturing_plate() -> Analysis {
    use std::io::Read as _;
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts/manufacturing_pmi.step.gz"),
    )
    .unwrap();
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut step)
        .unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("h6-pmi-plate")
        .join(format!("{:?}", std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("manufacturing-pmi.step");
    std::fs::write(&path, step).unwrap();
    model::analyse_native(&path).unwrap().1
}

/// The supplied plate: its smallest fit is A3 at 1:1 (no A4 candidate is valid: at 1:1 the
/// stacked views and their annotations need more height above the title block than A4 has).
/// The rule allows A3 1:1 and A2 2:1; on A2 the orientation view keeps A3's paper scale, so the
/// extra paper only enlarges the dimensioned views and A3 1:1 is the fuller. Before H6 it was
/// A2 at 1:1, a fifth of the sheet.
#[test]
fn specify_plate_pmi_fits_a3_at_full_size_with_every_frame_clear() {
    let a = manufacturing_plate();
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(p.sheet.basis.contains("A4 "), "{}", p.sheet.basis);
    sheet_rule::assert_rule("manufacturing-pmi", &p);
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "1:1")
    );
    assert_eq!(p.gdt.len(), 6);
    assert!(p.gdt.iter().all(|g| g.placed.is_some()));
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_off_sheet"
            | "annotation_overlap"
            | "annotation_ink_overlap"
            | "page_fit_uncertain"
    )));
    for v in &p.views {
        let b = v.area;
        let t = p.sheet.title_block;
        assert!(!(b[0] < t[2] && t[0] < b[2] && b[1] < t[3] && t[1] < b[3]));
    }
}

#[test]
fn automatic_overflow_never_uses_an_illegible_scale() {
    let mut a = part();
    a.envelope = [[0.0; 3], [100_000.0; 3]];
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(p.sheet.scale_factor >= 0.1);
    assert_eq!(p.sheet.size, "A0");
    assert!(
        p.problems
            .iter()
            .any(|p| matches!(p.code.as_str(), "page_fit_uncertain" | "views_do_not_fit"))
    );
}

#[test]
fn ctc03_valid_complete_sheet_beats_invalid_seed_despite_material_crossings() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("nist/nist_ctc_03_asme1_rc.stp"))
        .unwrap()
        .1;
    use std::io::Read as _;
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        include_bytes!("fixtures/sheet-policy/ctc03-invalid.plan.json.gz").as_slice(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let mut first: model::Plan = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(first.binding.source_sha256, a.binding.source_sha256);
    assert!(
        first
            .problems
            .iter()
            .any(|p| p.code == "annotation_ink_overlap")
    );
    // The seed was captured before G3 planned radii by note and leader; its radius callouts
    // are not outcomes the current planner owes on any sheet, so they are left out of it.
    first.dimensions.retain(|d| {
        !d.measurements
            .iter()
            .all(|m| matches!(m.parameter.as_str(), "blend.radius" | "fillet.radius"))
    });
    let mut spec = Spec::default();
    spec.sheet.size = Some("A2".into());
    first.spec.sheet.size = spec.sheet.size.clone();
    let p = model::recover(&a, &spec, first).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A2", "1:5")
    );
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_ink_overlap" | "annotation_off_sheet"
    )));
    if draftwright::sheet_fit::readability(&a, &p).failures() > 0 {
        assert!(
            p.problems
                .iter()
                .any(|problem| problem.code == "layout_recovery_exhausted"
                    && problem.severity == "warning")
        );
    }
    assert!(
        p.dimensions
            .iter()
            .all(|d| !matches!(d.state, draftwright::items::State::NotDrawable { .. }))
    );
}

#[test]
fn holes_set_legibility_floor_from_visible_opening_size() {
    let mut a = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step"),
    )
    .unwrap()
    .1;
    a.features[0].kind = "hole".into();
    a.features[0].parameters = json!({"diameter": 6.6}).as_object().unwrap().clone();
    assert!(
        draftwright::sheet_fit::min_legible_scale(&a) >= draftwright::sheet_fit::MIN_OPENING / 6.6
    );
    a.features[0].parameters = json!({"diameter": 0.6}).as_object().unwrap().clone();
    assert!(
        draftwright::sheet_fit::min_legible_scale(&a) >= draftwright::sheet_fit::MIN_OPENING / 0.6
    );
}

#[test]
fn packed_gdt_keeps_title_overflow_on_the_recipient_sheet() {
    let a = manufacturing_plate();
    let mut spec = Spec::default();
    spec.sheet.size = Some("A3".into());
    spec.sheet.title.insert("title".into(), "M".repeat(50));
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert_eq!(p.sheet.size, "A3");
    assert!(p.problems.iter().any(|q| q.code == "title_field_overflow"));
    assert_eq!(p.status, draftwright::completeness::Status::NeedsAttention);
}

#[test]
fn unavailable_legible_scale_is_a_warning() {
    let mut a = manufacturing_plate();
    a.features
        .iter_mut()
        .find(|f| f.kind == "hole")
        .unwrap()
        .parameters
        .insert("diameter".into(), json!(0.02));
    assert!(draftwright::sheet_fit::min_legible_scale(&a) > 10.0);
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted" && q.severity == "warning")
    );
    assert_eq!(p.status, draftwright::completeness::Status::NeedsAttention);
}

fn gz_part(name: &str) -> Analysis {
    use std::io::Read as _;
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pmi/step")
            .join(format!("{name}.gz")),
    )
    .unwrap();
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut step)
        .unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("large-sheets")
        .join(format!(
            "{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, step).unwrap();
    model::analyse_native(&path).unwrap().1
}

/// The former full-size-only cap accepted 1:5 holes below the new 2 mm
/// readability floor. Its larger readable trial changes crossing pairs, so the
/// fitted A4 is kept with an explicit refusal. The related intent fixture fits
/// cleanly on A3. Both try readable single-sheet paper before auxiliary allocation.
#[test]
fn small_parts_exhaust_single_sheet_paper_before_allocation() {
    for (name, size) in [
        ("assembly_plate_pin.step", "A4"),
        ("hw_assembly_plate_pin_intent.step", "A3"),
    ] {
        let a = gz_part(name);
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(p.sheet.size, size, "{name}: {}", p.sheet.basis);
        let readable = draftwright::sheet_fit::readability(&a, &p);
        if name == "assembly_plate_pin.step" {
            assert_eq!(p.sheet.scale, "1:5");
            assert_eq!(readable.openings, 8);
            assert_eq!(readable.failures(), 8);
            assert!(
                p.problems
                    .iter()
                    .any(|q| q.code == "layout_recovery_exhausted")
            );
        } else {
            assert_eq!(readable.failures(), 0);
        }
        sheet_rule::assert_rule(name, &p);
        assert!(p.sheet.basis.contains("A4 "), "{name}: {}", p.sheet.basis);
        if name == "assembly_plate_pin.step" {
            assert!(p.sheet.basis.contains("A3 "), "{}", p.sheet.basis);
        }
        assert!(p.gdt.iter().all(|g| g.placed.is_some()));
        assert!(!p.problems.iter().any(|p| matches!(
            p.code.as_str(),
            "annotation_ink_overlap" | "annotation_off_sheet" | "page_fit_uncertain"
        )));
        let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
        let p = model::recover(&a, &Spec::default(), first.clone()).unwrap();
        // A supplied framed drawing preserves its own established annotation
        // associations. A2 may remain when smaller paper buys a new crossing pair.
        assert!(
            matches!(p.sheet.size.as_str(), "A4" | "A3" | "A2"),
            "{name}: {}",
            p.sheet.basis
        );
        assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
        let issues = |p: &Plan| {
            let mut out = std::collections::BTreeMap::new();
            for q in p.problems.iter().filter(|q| {
                q.severity != "info"
                    && (q.code.starts_with("annotation_")
                        || matches!(
                            q.code.as_str(),
                            "leader_through_feature" | "extension_through_feature"
                        ))
            }) {
                *out.entry((q.code.clone(), q.subjects.clone(), q.refs.clone()))
                    .or_insert(0usize) += 1;
            }
            out
        };
        let before = issues(&first);
        for (key, count) in issues(&p) {
            assert!(
                count <= before.get(&key).copied().unwrap_or(0),
                "{name}: new seeded finding {key:?}"
            );
        }
        assert!(
            first
                .gdt
                .iter()
                .filter(|g| g.placed.is_some())
                .all(|g| p.gdt.iter().any(|q| q.id == g.id && q.placed.is_some()))
        );
        if name == "assembly_plate_pin.step" {
            assert!(p.sheet.basis.contains("A4 ") && p.sheet.basis.contains("A3 "));
            let saved: serde_json::Value =
                serde_json::from_str(include_str!("fixtures/sheet_policy_routes.json")).unwrap();
            let original: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
                serde_json::from_value(saved["pairs"].clone()).unwrap();
            assert!(
                issues(&first).keys().any(|key| !original.contains(key)),
                "the supplied seed must exercise a different fitted route reference"
            );
        }
    }
    let a = part();
    let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
    let mut s = Spec::default();
    s.sheet.size = Some("A2".into());
    assert_eq!(model::recover(&a, &s, first).unwrap().sheet.size, "A2");
}

#[test]
fn a_central_bore_keeps_its_dimension_and_drawn_common_axis_without_redundant_table_coordinates() {
    let a = gz_part("spool_fits.step");
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(p.dimensions.iter().any(|d| d.placed.is_some()
        && d.measurements.iter().any(|m| m.feature == "f:hole:7"
            && m.parameter == "bore.diameter"
            && m.value == 62.0)));
    assert!(p.dimensions.iter().any(|d| {
        d.placed.is_some()
            && d.measurements.iter().any(|m| {
                m.feature == "envelope" && m.parameter == "od.diameter" && m.value == 130.0
            })
    }));
    let plan = p.views.iter().find(|v| v.id == "v:plan").unwrap();
    let cross = plan
        .line_work
        .iter()
        .find(|w| w.id == "ink:v:plan:f:hole:7")
        .unwrap();
    assert_eq!(cross.centre, Some(plan.at));
    assert!(cross.curves.len() >= 2);
    assert!(
        p.views
            .iter()
            .flat_map(|v| &v.line_work)
            .any(|w| w.id.ends_with(":f:hole:7")
                && w.kind == "axis"
                && w.global_axis
                && !w.curves.is_empty())
    );
}

/// Different tolerances keep each of the small turned part's statements distinct. A
/// section over the title block must not push the end view off A4, and a transverse
/// hole's witness must not run through a step-length label.
#[test]
fn toleranced_string_post_is_complete_on_a4_without_hard_layout_findings() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let a = model::analyse_native(&dir.join("gramel/string_post.step"))
        .unwrap()
        .1;
    let spec: Spec = serde_json::from_str(include_str!(
        "fixtures/layout/string_post_tolerances.spec.json"
    ))
    .unwrap();
    let plan = model::plan_recovered(&a, &spec).unwrap();
    assert_eq!(
        (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
        ("A4", "5:1"),
        "{}",
        plan.sheet.basis
    );
    assert!(plan.views.iter().all(|v| v.detail.is_none()));
    let hard = [
        "annotation_off_sheet",
        "annotation_over_title_block",
        "annotation_overlap",
        "annotation_ink_overlap",
        "title_field_overflow",
        "page_fit_uncertain",
        "views_do_not_fit",
        "notes_not_placed",
        "table_not_placed",
    ];
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| hard.contains(&p.code.as_str())),
        "{:?}",
        plan.problems
    );
    for id in spec.dimensions.keys() {
        assert!(
            plan.dimensions
                .iter()
                .any(|d| &d.id == id && d.placed.is_some()),
            "missing toleranced statement {id}"
        );
    }
    sheet_rule::assert_rule("toleranced string post", &plan);
}

/// Smaller paper cannot buy extra sheets or lose the section/orientation views.
#[test]
fn tapped_string_posts_keep_one_sheet_and_gutters_keep_the_selected_frame() {
    for name in [
        "string_post_tapped.step",
        "hw_string_post_tapped_intent.step",
    ] {
        let a = gz_part(name);
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert!(p.sheets.is_empty(), "{name}: {}", p.sheet.basis);
        assert!(p.views.iter().all(|v| v.sheet == 1));
        assert!(p.views.iter().any(|v| v.id == "v:iso"));
        assert!(p.views.iter().any(|v| v.cut.is_some()));
        assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
        assert!(p.gdt.iter().all(|g| g.placed.is_some()));
        for gutter in [0., 1000.] {
            let mut spec = Spec::default();
            spec.sheet.view_gutter = Some(gutter);
            let q = model::plan_recovered(&a, &spec).unwrap();
            assert!(q.sheets.is_empty());
            assert_eq!(
                (&p.sheet.size, &p.sheet.scale),
                (&q.sheet.size, &q.sheet.scale)
            );
            for v in &p.views {
                assert_eq!(
                    v.scale,
                    q.views.iter().find(|w| w.id == v.id).unwrap().scale
                );
            }
        }
    }
}

/// The same small geometry with GD&T keeps the established readable scale. Its
/// reserved frame corridors stay with the aligned row during section correction.
#[test]
fn section_correction_preserves_the_gdt_string_posts_readable_scale() {
    use std::io::Read as _;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step/hw_string_post_tapped_intent.step.gz");
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut step)
        .unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sheet-fill-gdt-string-post");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("string-post.step");
    std::fs::write(&path, step).unwrap();
    let a = model::analyse_native(&path).unwrap().1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "5:1"),
        "{}",
        p.sheet.basis
    );
    assert!(p.sheets.is_empty());
    assert!(p.sheet.basis.contains("A4 "));
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
    assert!(!p.problems.iter().any(|p| matches!(
        p.code.as_str(),
        "annotation_off_sheet"
            | "annotation_overlap"
            | "annotation_ink_overlap"
            | "page_fit_uncertain"
    )));
}

/// Conservative section bands must not force a second sheet when the settled ink fits.
fn assert_small_single_sheet(name: &str) {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join(format!("mfcadpp/{name}.step")))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(
        matches!(p.sheet.size.as_str(), "A4" | "A3"),
        "{name}: {}",
        p.sheet.basis
    );
    assert!(p.sheets.is_empty(), "{name}: {}", p.sheet.basis);
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
    sheet_rule::assert_rule(name, &p);
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "2:1")
    );
    assert!(
        p.views
            .iter()
            .filter(|v| ["v:front", "v:plan", "v:side"].contains(&v.id.as_str()))
            .all(|v| v.sheet == 1)
    );
    assert!(p.views.iter().all(|v| v.sheet == 1));
    for (i, v) in p.views.iter().enumerate() {
        let [x0, y0, x1, y1] = v.area;
        assert!(
            x0 >= 10. && y0 >= 10. && x1 <= 410. && y1 <= 287.,
            "{name}: {} {:?}",
            v.id,
            v.area
        );
        assert!(p.views[i + 1..].iter().all(|w| !(v.area[0] < w.area[2]
            && v.area[2] > w.area[0]
            && v.area[1] < w.area[3]
            && v.area[3] > w.area[1])));
    }
    assert!(
        !p.problems.iter().any(|q| [
            "annotation_off_sheet",
            "views_overlap",
            "view_on_title_block",
            "cross_reference_missing",
            "layout_recovery_exhausted"
        ]
        .contains(&q.code.as_str())),
        "{name}: {:?}",
        p.problems
    );
    assert!(
        p.spec.sheets.is_none(),
        "allocation must not rewrite intent"
    );
}

#[test]
fn small_10224_fits_one_a3_sheet() {
    assert_small_single_sheet("10224");
}

#[test]
fn small_10255_fits_one_a3_sheet() {
    assert_small_single_sheet("10255");
}

/// This 279 mm principal projection already needs large paper at full size.
#[test]
fn a_part_needing_large_paper_at_full_size_keeps_large_paper() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("nist/nist_ftc_09_asme1_rd.stp"))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(
        matches!(p.sheet.size.as_str(), "A2" | "A1" | "A0"),
        "{}",
        p.sheet.basis
    );
    assert_eq!(p.sheet.scale, "1:1");
    assert!(p.sheets.is_empty());
    assert!(!p.sheet.basis.contains("R5 allocation before"));
}

#[test]
fn overflow_honours_named_paper_and_sheet_count() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("mfcadpp/10224.step"))
        .unwrap()
        .1;
    for (intent, size, count) in [
        (json!({"sheet":{"size":"A2"}}), "A2", 1),
        (json!({"sheets":{"max":1}}), "A3", 1),
    ] {
        let spec: Spec = serde_json::from_value(intent).unwrap();
        let p = model::plan_recovered(&a, &spec).unwrap();
        assert_eq!(p.sheet.size, size);
        assert_eq!(1 + p.sheets.len(), count);
        assert_eq!(p.spec.sheets, spec.sheets);
    }
    let spec: Spec = serde_json::from_value(json!({"sheets":{"max":2}})).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert!(
        matches!(p.sheet.size.as_str(), "A4" | "A3"),
        "{}",
        p.sheet.basis
    );
    assert!(p.sheets.is_empty());
    assert_eq!(p.spec.sheets, spec.sheets);
}

#[test]
fn small_overflow_respects_explicit_auxiliary_assignment() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("mfcadpp/10224.step"))
        .unwrap()
        .1;
    let spec: Spec =
        serde_json::from_value(json!({"sheets":{"max":2},"views":{"iso":{"sheet":2}}})).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert!(
        matches!(p.sheet.size.as_str(), "A4" | "A3"),
        "{}",
        p.sheet.basis
    );
    assert_eq!(p.sheets.len(), 1);
    assert_eq!(p.views.iter().find(|v| v.id == "v:iso").unwrap().sheet, 2);
    assert_eq!(p.spec.views["iso"].sheet, Some(2));
    assert_eq!(p.spec.sheets, spec.sheets);
}

#[test]
fn a_readable_fitted_drawing_is_not_replaced_by_a_compressed_seed() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("mfcadpp/10190.step"))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(p.sheet.size, "A1");
    assert_eq!(p.sheet.scale_factor, 10.0);
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
    assert!(!p.problems.iter().any(|q| matches!(
        q.code.as_str(),
        "annotation_off_sheet" | "annotation_over_title_block" | "layout_recovery_exhausted"
    )));
    assert!(p.sheets.is_empty());
    for paper in ["A4", "A3"] {
        assert!(
            p.sheet.basis.contains(&format!("{paper} ")),
            "{}",
            p.sheet.basis
        );
    }
    assert!(!p.problems.iter().any(|q| q.code.ends_with("_dropped")));
}

#[test]
fn a_crowded_plain_ladder_keeps_the_fitted_baseline_for_its_schedule() {
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&dir.join("nist/nist_ftc_09_asme1_rd.stp")).unwrap();
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(p.hole_table.is_some() || !p.hole_tables.is_empty());
    assert!(
        !p.problems.iter().any(|q| matches!(
            q.code.as_str(),
            "annotation_off_sheet" | "pinned_view_off_sheet" | "annotation_over_title_block"
        )),
        "{:?}",
        p.problems
    );
    for v in &p.views {
        let sheet = std::iter::once(&p.sheet)
            .chain(&p.sheets)
            .find(|s| s.number == v.sheet)
            .unwrap();
        assert!(
            v.area[0] >= 10.0 - 1e-6
                && v.area[1] >= 10.0 - 1e-6
                && v.area[2] <= sheet.page[0] - 10.0 + 1e-6
                && v.area[3] <= sheet.page[1] - 10.0 + 1e-6,
            "{}: {:?}",
            v.id,
            v.area
        );
    }
}

#[test]
fn a_requested_scale_prefers_one_larger_sheet_to_smaller_multisheet_paper() {
    let a = gz_part("hw_string_post_tapped_intent.step");
    let spec: Spec = serde_json::from_value(json!({"sheet":{"scale":"5:1"}})).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "5:1"),
        "{}",
        p.sheet.basis
    );
    assert!(p.sheets.is_empty());
    assert_eq!(p.spec.sheet.scale, spec.sheet.scale);
    assert_eq!(p.spec.sheet.size, spec.sheet.size);
    assert_eq!(p.spec.sheets, spec.sheets);
    assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
    assert!(p.gdt.iter().all(|g| g.placed.is_some()));
}

#[test]
fn fixed_scale_can_use_a_valid_smaller_sheet_without_changing_scale() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("gramel/GRM-03_thumbwheel_drive_screw.step"))
        .unwrap()
        .1;
    let spec: Spec = serde_json::from_value(json!({"sheet":{"scale":"10:1"}})).unwrap();
    let p = model::plan_recovered(&a, &spec).unwrap();
    assert_eq!(p.sheet.size, "A3", "{}", p.sheet.basis);
    assert_eq!(p.sheet.scale, "10:1");
    assert!(p.sheets.is_empty());
    assert_eq!(p.spec.sheet.scale, spec.sheet.scale);
}

/// A fit is readable only when the dimensioned profile intervals survive its scale.
/// This formerly selected A4 1:10: the 420 mm profile was 42 mm tall and its 20 mm
/// shoulder bands were shorter than the 2.1 mm printed capitals, despite no hard collision.
#[test]
fn connector109_keeps_a_readable_ordinary_main_without_an_unannotated_crop() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("cadgenbench/threaded_connector_109.step"))
        .unwrap()
        .1;
    let tiny = model::plan_recovered(&a, &named("A4", "1:10")).unwrap();
    assert!(draftwright::sheet_fit::illegible_steps(&a, &tiny) > 0);
    assert!(
        tiny.problems
            .iter()
            .any(|p| p.code == "legibility_floor_breached")
    );
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A3", "1:5"),
        "{}",
        p.sheet.basis
    );
    let readable = draftwright::sheet_fit::readability(&a, &p);
    assert_eq!(
        (
            readable.steps,
            readable.openings,
            readable.native_openings,
            readable.text_arrows,
            readable.lanes
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(readable.depiction, 2);
    assert!(draftwright::sheet_fit::border_contacts(&a, &p).is_empty());
    assert!(p.sheets.is_empty());
    assert!(p.views.iter().all(|v| v.detail.is_none()));
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted" && q.severity == "warning")
    );
    let mut without = Spec::default();
    without.sheet.automatic_details = Some(false);
    let ordinary = model::plan_recovered(&a, &without).unwrap();
    assert_eq!(
        (&p.sheet.size, &p.sheet.scale),
        (&ordinary.sheet.size, &ordinary.sheet.scale)
    );
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    let compressed = model::plan_recovered(&a, &named("A3", "1:5")).unwrap();
    assert!(draftwright::sheet_fit::readability(&a, &compressed).depiction > 0);
    assert!(p.sheet.basis.contains("A4 "));
    assert!(p.sheet.basis.contains("A3 "));
    assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
    sheet_rule::assert_rule("connector109", &p);
}

#[test]
fn step_legibility_uses_the_owning_view_and_keeps_baseline_intervals() {
    let mut a = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/turned.step"),
    )
    .unwrap()
    .1;
    let mut p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let mut d = p
        .dimensions
        .iter()
        .find(|d| d.placed.is_some())
        .unwrap()
        .clone();
    let f = &mut a.features[0];
    f.kind = "turned_step".into();
    f.parameters = json!({"lo": 0.0, "hi": 20.0}).as_object().unwrap().clone();
    d.measurements = vec![draftwright::dims::Measurement {
        feature: f.id.clone(),
        parameter: "step.length".into(),
        value: 20.0,
    }];
    p.dimensions = vec![d];
    let mut detail = p
        .views
        .iter()
        .find(|v| v.id == p.dimensions[0].view)
        .unwrap()
        .clone();
    for v in &mut p.views {
        v.scale = 0.1;
    }
    assert_eq!(draftwright::sheet_fit::illegible_steps(&a, &p), 1);
    detail.id = "v:detail_a".into();
    detail.scale = 0.2;
    p.dimensions[0].view = detail.id.clone();
    p.views.push(detail);
    assert_eq!(draftwright::sheet_fit::illegible_steps(&a, &p), 0);
    p.dimensions[0].measurements[0].parameter = "step_position.length".into();
    p.views.last_mut().unwrap().scale = 0.1;
    assert_eq!(draftwright::sheet_fit::illegible_steps(&a, &p), 1);
    let mut local = p.dimensions[0].clone();
    local.id = "d:local_step_length".into();
    local.measurements[0].parameter = "step.length".into();
    let mut carrier = p.views.last().unwrap().clone();
    carrier.id = "v:detail_b".into();
    carrier.scale = 0.2;
    local.view = carrier.id.clone();
    p.views.push(carrier);
    p.dimensions.push(local);
    assert_eq!(
        draftwright::sheet_fit::illegible_steps(&a, &p),
        0,
        "a readable local band states the interval while its datum stays on the parent"
    );
    p.views.last_mut().unwrap().scale = 0.1;
    assert_eq!(draftwright::sheet_fit::illegible_steps(&a, &p), 1);
}

#[test]
fn declining_optional_details_does_not_buy_new_annotation_crossings() {
    for file in [
        "hw_thumbwheel_notes_add.step",
        "thumbwheel_thread_knurl.step",
    ] {
        let a = gz_part(file);
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(p.sheet.size, "A4", "{file}: {}", p.sheet.basis);
        let recorded: serde_json::Value = serde_json::from_str(include_str!(
            "fixtures/sheet-policy/release-main-routes.json"
        ))
        .unwrap();
        let baseline = &recorded["parts"][format!("fixtures/pmi/step/{file}.gz")];
        assert_eq!(a.binding.source_sha256, baseline["source_sha256"]);
        let pairs: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
            serde_json::from_value(baseline["pairs"].clone()).unwrap();
        for q in p
            .problems
            .iter()
            .filter(|q| q.code == "annotation_leader_crossing")
        {
            assert!(pairs.contains(&(q.code.clone(), q.subjects.clone(), q.refs.clone())));
        }
        assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
        assert!(p.dimensions.iter().all(|d| d.placed.is_some()));
        assert!(p.gdt.iter().all(|g| g.placed.is_some()));
    }
    let Some(dir) = corpus::dir() else { return };
    let (_, a) = model::analyse_native(&dir.join("mfcadpp/1013.step")).unwrap();
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let recorded: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/sheet-policy/release-main-routes.json"
    ))
    .unwrap();
    let baseline = &recorded["parts"]["corpus/mfcadpp/1013.step"];
    assert_eq!(a.binding.source_sha256, baseline["source_sha256"]);
    let pairs: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
        serde_json::from_value(baseline["pairs"].clone()).unwrap();
    for q in p
        .problems
        .iter()
        .filter(|q| q.code == "annotation_leader_crossing")
    {
        assert!(pairs.contains(&(q.code.clone(), q.subjects.clone(), q.refs.clone())));
    }
    let readable = draftwright::sheet_fit::readability(&a, &p);
    assert_eq!(readable.failures(), 2, "{}", p.sheet.basis);
    assert!(readable.failures() <= baseline["readability"]["failures"].as_u64().unwrap() as usize);
    assert!(
        p.problems
            .iter()
            .any(|q| q.code == "layout_recovery_exhausted"),
        "{}",
        p.sheet.basis
    );
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        (
            baseline["sheet"]["size"].as_str().unwrap(),
            baseline["sheet"]["scale"].as_str().unwrap()
        ),
        "{}",
        p.sheet.basis
    );
}

#[test]
fn readable_reductions_do_not_return_to_a_larger_obstructed_estimate() {
    for file in ["assembly.step", "assembly_pin_pmi.step.gz"] {
        let mut path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assembly")
            .join(file);
        if let Some(name) = file.strip_suffix(".gz") {
            use std::io::Read as _;
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
                .read_to_end(&mut bytes)
                .unwrap();
            let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join("readable-reductions")
                .join(std::process::id().to_string());
            std::fs::create_dir_all(&dir).unwrap();
            path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
        }
        let (_, a) = model::analyse_native(&path).unwrap();
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A4", "1:2"),
            "{file}: {}",
            p.sheet.basis
        );
        assert_eq!(draftwright::sheet_fit::readability(&a, &p).failures(), 0);
        assert!(!p.problems.iter().any(|q| q.code == "annotation_over_view"));
        assert!(
            p.problems
                .iter()
                .filter(|q| q.code == "leader_through_feature")
                .count()
                <= 2
        );
    }
    for file in ["hw_spool_fits_intent.step", "spool_fits.step"] {
        let a = gz_part(file);
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert_eq!(
            (p.sheet.size.as_str(), p.sheet.scale.as_str()),
            ("A3", "1:2"),
            "{file}: {}",
            p.sheet.basis
        );
        let r = draftwright::sheet_fit::readability(&a, &p);
        assert_eq!(r.failures(), 0);
        assert!(
            !p.problems
                .iter()
                .any(|q| q.code == "layout_recovery_exhausted" && q.severity == "warning")
        );
        let recorded: serde_json::Value = serde_json::from_str(include_str!(
            "fixtures/sheet-policy/release-main-routes.json"
        ))
        .unwrap();
        let baseline = &recorded["parts"][format!("fixtures/pmi/step/{file}.gz")];
        assert_eq!(a.binding.source_sha256, baseline["source_sha256"]);
        let pairs: std::collections::BTreeSet<(String, Vec<String>, Vec<String>)> =
            serde_json::from_value(baseline["pairs"].clone()).unwrap();
        for q in p
            .problems
            .iter()
            .filter(|q| q.code == "leader_through_feature")
        {
            assert!(pairs.contains(&(q.code.clone(), q.subjects.clone(), q.refs.clone())));
        }
        if file == "spool_fits.step" {
            let recorded: serde_json::Value = serde_json::from_str(include_str!(
                "fixtures/sheet-policy/spool-main-measurements.json"
            ))
            .unwrap();
            assert_eq!(a.binding.source_sha256, recorded["source_sha256"]);
            let required: Vec<draftwright::dims::Measurement> =
                serde_json::from_value(recorded["measurements"].clone()).unwrap();
            assert_eq!(required.len(), 69);
            let mut claims: Vec<_> = p
                .dimensions
                .iter()
                .filter(|d| d.placed.is_some())
                .flat_map(|d| d.measurements.iter())
                .collect();
            claims.extend(
                p.hole_tables
                    .iter()
                    .flat_map(|t| t.rows.iter())
                    .flat_map(|r| r.measurements.iter()),
            );
            let inferred: Vec<_> = required.iter().filter(|m| !claims.contains(m)).collect();
            assert_eq!(inferred.len(), 0);
            for m in inferred {
                assert_eq!(m.feature, "f:hole:7");
                assert!(["location.length.x", "location.length.y"].contains(&m.parameter.as_str()));
                assert_eq!(m.value, 65.0);
            }
            // These two formerly tabulated coordinates are still fully defined:
            // the 62 mm bore shares the drawn axis of the 130 mm round stock.
            assert!(claims.iter().any(|m| m.feature == "envelope"
                && m.parameter == "od.diameter"
                && m.value == 130.0));
            let plan = p.views.iter().find(|v| v.id == "v:plan").unwrap();
            let cross = plan
                .line_work
                .iter()
                .find(|w| w.id == "ink:v:plan:f:hole:7")
                .unwrap();
            assert_eq!(cross.centre, Some(plan.at));
            assert!(cross.curves.len() >= 2);
            assert!(
                p.views
                    .iter()
                    .flat_map(|v| &v.line_work)
                    .any(|w| w.id.ends_with(":f:hole:7")
                        && w.kind == "axis"
                        && w.global_axis
                        && !w.curves.is_empty())
            );
        } else {
            assert!(!p.hole_tables.is_empty());
        }
    }
}
