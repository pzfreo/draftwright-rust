//! Maintainer sheet policy, 2026-10-12: measured content, bounded paper, valid ink.
use draftwright::model::{self, Analysis, Spec};
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
fn smallest_fit_can_only_grow_one_sheet_with_enlargement() {
    let a = part();
    let fit = model::plan_escalated(&a, &named("A4", "1:1")).unwrap();
    let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
    let p = model::recover(&a, &Spec::default(), first).unwrap();
    assert!(
        matches!(p.sheet.size.as_str(), "A4" | "A3"),
        "{}",
        p.sheet.basis
    );
    if p.sheet.size == "A3" {
        assert!(p.sheet.scale_factor > fit.sheet.scale_factor);
    }
    assert!(p.sheet.basis.contains("smallest fit"), "{}", p.sheet.basis);
}

#[test]
fn complete_underfilled_sheet_is_enlarged_and_fill_candidates_are_explained() {
    let a = part();
    let first = model::plan_escalated(&a, &named("A4", "1:10")).unwrap();
    let p = model::recover(&a, &Spec::default(), first).unwrap();
    assert!(p.sheet.scale_factor > 0.1);
    assert!(p.sheet.basis.contains("fill"), "{}", p.sheet.basis);
    assert!(p.sheet.basis.contains("60–85%"), "{}", p.sheet.basis);
}

#[test]
fn named_sheet_overrides_smallest_fit_but_still_chooses_scale_by_fill() {
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
fn grm03_keeps_a4_with_an_enlarged_head_without_losing_measurements() {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let a = model::analyse_native(&dir.join("gramel/GRM-03_thumbwheel_drive_screw.step"))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let fill = serde_json::to_value(&p).unwrap()["sheet"]["fill"]
        .as_f64()
        .unwrap();
    assert_eq!(
        (p.sheet.size.as_str(), p.sheet.scale.as_str()),
        ("A4", "2:1")
    );
    assert!(
        fill > 0.0 && fill < 0.60,
        "the detail supplies the fine scale: {fill}"
    );
    assert!(
        p.views
            .iter()
            .any(|v| v.detail.is_some() && v.scale == 10.0)
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
    assert!(
        p.sheet.basis.starts_with("smallest fit A3 at 1:1;"),
        "{}",
        p.sheet.basis
    );
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
        "annotation_ink_overlap" | "layout_recovery_exhausted" | "annotation_off_sheet"
    )));
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

/// Maintainer rule, 2026-10-15: A2 and larger are for parts that need them at 1:1. A part about
/// 100 mm across, whose views fit A3 at 1:1, never takes a large sheet by default, even when its
/// first estimate is A2 and A2 would score fewer findings (assembly_plate_pin: its ladder
/// estimate is A2 at 1:1). A named large sheet is still the user's to choose.
#[test]
fn a_part_about_100_mm_never_gets_a2_or_larger_by_default() {
    for name in [
        "assembly_plate_pin.step",
        "hw_assembly_plate_pin_intent.step",
    ] {
        let a = gz_part(name);
        let p = model::plan_recovered(&a, &Spec::default()).unwrap();
        assert!(
            matches!(p.sheet.size.as_str(), "A4" | "A3"),
            "{name}: {} at {}: {}",
            p.sheet.size,
            p.sheet.scale,
            p.sheet.basis
        );
        let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
        let p = model::recover(&a, &Spec::default(), first).unwrap();
        assert!(
            matches!(p.sheet.size.as_str(), "A4" | "A3"),
            "{name} from an A2 estimate: {}",
            p.sheet.basis
        );
    }
    let a = part();
    let first = model::plan_escalated(&a, &named("A2", "1:1")).unwrap();
    let mut s = Spec::default();
    s.sheet.size = Some("A2".into());
    assert_eq!(model::recover(&a, &s, first).unwrap().sheet.size, "A2");
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
        ("A4", "2:1"),
        "{}",
        plan.sheet.basis
    );
    // The merged detail planner supplies the former fine main scale in head B.
    // Keep an exact scale assertion and prove that the enlarged geometry is present.
    let head = plan
        .views
        .iter()
        .find(|v| v.id == "v:detail_b")
        .expect("head B");
    assert_eq!(head.scale, 10.0);
    assert_eq!(head.detail.as_ref().unwrap().kind, "turned-head");
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
fn large_paper_fallthrough_records_both_failed_small_allocations() {
    let Some(dir) = corpus::dir() else { return };
    let a = model::analyse_native(&dir.join("mfcadpp/10190.step"))
        .unwrap()
        .1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(p.sheet.size, "A1");
    assert!(p.sheets.is_empty());
    for paper in ["A4", "A3"] {
        assert!(
            p.sheet.basis.contains(&format!("{paper} multi-sheet:")),
            "{}",
            p.sheet.basis
        );
    }
    assert!(p.sheet.basis.contains("1 required drops"));
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
