//! Issue 122: the editor's actual plate, with semantic PMI written as Specify writes it.
use draftwright::{
    Session,
    haecceity::{
        p21::Document,
        pmi::{self as hp, *},
        step::read_part_definitions,
    },
    model::{self, Analysis, DimensionSource, PmiMode, Spec},
};
use std::path::Path;
const PLATE: &[u8] = include_bytes!("fixtures/parts/mounting_plate.step");
#[test]
fn supplied_sample_is_a_different_plate() {
    let a = analysed(include_bytes!("fixtures/parts/specify-sample-plate.step"));
    assert_eq!(a.envelope, [[-60., -40., 0.], [60., 40., 12.]]);
    assert!(
        !a.features
            .iter()
            .any(|f| f.kind == "hole" && (f.fingerprint.position[0] - 95.).abs() < 1e-6)
    );
}

fn mm(v: f64) -> Value {
    Value::length(
        Decimal::parse(&if v.fract() == 0. {
            format!("{v}.")
        } else {
            v.to_string()
        })
        .unwrap(),
        LengthUnit::Millimetre,
    )
}
fn face(p: &mut PartPmi, f: usize) -> FeatureId {
    p.features
        .push(Feature::Items(vec![Anchor::Face(FaceIndex(f))]));
    FeatureId(p.features.len() - 1)
}
fn distance(p: &mut PartPmi, from: usize, to: usize, value: f64, basic: bool) {
    let from = face(p, from);
    let to = face(p, to);
    p.dimensions.push(Dimension {
        kind: DimensionKind::Location {
            from,
            to,
            kind: LocationKind::LinearDistance,
            path: None,
            directed: false,
            angle: None,
        },
        nominal: Some(mm(value)),
        tolerance: if basic {
            DimTolerance::Basic
        } else {
            DimTolerance::Deviations(Bounds::new(mm(0.05), mm(-0.05)).unwrap())
        },
        qualifier: None,
        modifiers: vec![],
        principle: None,
    });
}
fn bytes(p: PartPmi) -> Vec<u8> {
    let parts = read_part_definitions(PLATE).unwrap();
    let doc = Document::parse(PLATE.to_vec()).unwrap();
    let (edit, _) = hp::write(
        &doc,
        &parts,
        &[(PartId(0), p)],
        hp::write::Mode::Replace,
        hp::write::PresentationPolicy::RemovePresentation,
    )
    .unwrap();
    doc.apply(&edit).unwrap().bytes
}
fn analysed(b: &[u8]) -> Analysis {
    model::analyse_native_step(Path::new("plate.step"), b)
        .unwrap()
        .1
}
fn plan(a: &Analysis, s: &Spec) -> model::Plan {
    model::plan_recovered(a, s).unwrap()
}
#[test]
fn specified_distance_from_either_end_owns_the_hole_location() {
    let a = analysed(PLATE);
    assert_eq!([a.envelope[0][0], a.envelope[1][0]], [-50., 50.]);
    for x in [50., 95.] {
        assert!(
            a.features.iter().any(|f| f.kind == "hole"
                && (f.fingerprint.position[0] - a.envelope[0][0] - x).abs() < 1e-5),
            "missing x={x}"
        );
    }
    let plain = plan(&a, &Spec::default());
    assert!(plain.dimensions.iter().any(|d| d.label == "2× 30"));
    for (from, v) in [(0, 95.), (5, 5.)] {
        let mut p = PartPmi::default();
        distance(&mut p, from, 12, v, false);
        let written = bytes(p);
        let a = analysed(&written);
        let result = plan(&a, &Spec::default());
        assert!(
            result
                .dimensions
                .iter()
                .any(|d| d.label == format!("{v} ±0.05")),
            "{:?}",
            result
                .dimensions
                .iter()
                .map(|d| &d.label)
                .collect::<Vec<_>>()
        );
        let hole = a
            .features
            .iter()
            .find(|f| f.kind == "hole" && f.faces.contains(&12))
            .unwrap();
        let id = format!("d:{}/location.length.x", hole.id);
        assert!(
            !result.dimensions.iter().any(|d| d.id == id),
            "engine location {id} duplicates PMI"
        );
        assert!(result.omitted.iter().any(|d|d.id==id&&matches!(d.state,draftwright::items::State::Covered{..})),"covered ledger missing {id}");
    }
}
#[test]
fn modes_keep_plain_drawings_and_strict_exposes_gaps() {
    let a = analysed(PLATE);
    let plain = plan(&a, &Spec::default());
    let (part, _) = model::analyse_native_step(Path::new("plate.step"), PLATE).unwrap();
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    let drawing = model::render(&part, &a, &plain, &font).unwrap();
    let svg = drawing.svg();
    for pmi in [PmiMode::Strict, PmiMode::Ignore] {
        let s = Spec {
            pmi,
            ..Default::default()
        };
        let p = plan(&a, &s);
        let drawing = model::render(&part, &a, &p, &font).unwrap();
        assert_eq!(svg, drawing.svg());
    }
    let mut p = PartPmi::default();
    distance(&mut p, 5, 12, 5., false);
    let written = bytes(p);
    let a = analysed(&written);
    let strict = plan(
        &a,
        &Spec {
            pmi: PmiMode::Strict,
            ..Default::default()
        },
    );
    assert_eq!(
        strict.dimensions.len(),
        1,
        "{:?}",
        strict.dimensions.iter().map(|d| &d.id).collect::<Vec<_>>()
    );
    assert!(
        strict
            .problems
            .iter()
            .any(|p| p.code == "feature_not_located")
    );
    assert!(
        strict
            .problems
            .iter()
            .any(|p| p.code == "feature_not_sized")
    );
    let mut authored = Spec {
        pmi: PmiMode::Strict,
        dimension_source: DimensionSource::Authored,
        ..Default::default()
    };
    authored
        .dimensions
        .insert("d:envelope/width.length".into(), Default::default());
    assert!(
        plan(&a, &authored)
            .dimensions
            .iter()
            .any(|d| d.id == "d:envelope/width.length")
    );
    let ignored = plan(
        &a,
        &Spec {
            pmi: PmiMode::Ignore,
            ..Default::default()
        },
    );
    assert_eq!(ignored.dimensions, plain.dimensions);
    assert_eq!(
        ignored
            .problems
            .iter()
            .filter(|p| p.code == "pmi_ignored")
            .count(),
        1
    );
    assert!(a.pmi.is_some());
    // The live replacement path observes the same policy and exposes extraction unchanged.
    let mut session = Session::new(
        part,
        analysed(PLATE),
        Spec {
            pmi: PmiMode::Strict,
            ..Default::default()
        },
    )
    .unwrap();
    session.set_pmi(&written).unwrap();
    assert_eq!(session.plan().dimensions.len(), 1);
}
#[test]
fn datum_origins_and_individual_basics_replace_pattern_pitches() {
    for b in [0, 5] {
        let mut p = PartPmi::default();
        for (letter, f) in [("A", 2), ("B", b), ("C", 1)] {
            let f = face(&mut p, f);
            p.datums
                .push(Datum::new(DatumLabel::new(letter).unwrap(), Some(f), vec![]).unwrap());
        }
        let a = analysed(PLATE);
        for f in a.features.iter().filter(|f| f.kind == "hole") {
            let x = f.fingerprint.position[0] - a.envelope[0][0];
            let y = f.fingerprint.position[1] - a.envelope[0][1];
            distance(
                &mut p,
                b,
                f.faces[0],
                if b == 5 { 100. - x } else { x },
                true,
            );
            distance(&mut p, 1, f.faces[0], y, true);
        }
        let a = analysed(&bytes(p));
        let result = plan(&a, &Spec::default());
        assert!(
            !result.dimensions.iter().any(|d| !d.id.starts_with("d:pmi:")
                && d.measurements
                    .iter()
                    .any(|m| m.parameter.contains("pitch.length"))),
            "{:?}",
            result
                .dimensions
                .iter()
                .filter(|d| d
                    .measurements
                    .iter()
                    .any(|m| m.parameter.contains("pitch.length")))
                .collect::<Vec<_>>()
        );
        assert!(!result.dimensions.iter().any(|d| {
            !d.id.starts_with("d:pmi:")
                && d.measurements
                    .iter()
                    .any(|m| m.parameter == "location.length.x")
        }));
        assert!(result.gdt.iter().all(|g| g.kind != "datum_feature"));
        assert!(
            !result
                .problems
                .iter()
                .any(|p| p.code == "feature_located_twice"),
            "{:?}",
            result
                .problems
                .iter()
                .filter(|p| p.code == "feature_located_twice")
                .collect::<Vec<_>>()
        );
    }
    let mut p = PartPmi::default();
    let f = face(&mut p, 5);
    p.datums
        .push(Datum::new(DatumLabel::new("B").unwrap(), Some(f), vec![]).unwrap());
    let a = analysed(&bytes(p));
    let result = plan(&a, &Spec::default());
    assert!(
        result
            .dimensions
            .iter()
            .filter(|d| d.id.ends_with("location.length.x"))
            .all(|d| d.basis.contains("datum B"))
    );
    assert!(
        result
            .dimensions
            .iter()
            .any(|d| d.id.ends_with("location.length.x") && d.value == 5.)
    );
    let s = Spec {
        origin: std::collections::BTreeMap::from([("x".into(), "min".into())]),
        ..Default::default()
    };
    assert!(
        plan(&a, &s)
            .dimensions
            .iter()
            .any(|d| d.id.ends_with("location.length.x")
                && d.value == 95.
                && d.basis.contains("origin/x min"))
    );
}
#[test]
fn invalid_modes_origins_and_transactional_edits_are_refused() {
    assert!(serde_json::from_str::<Spec>(r#"{"pmi":"annotate"}"#).is_err());
    let a = analysed(PLATE);
    for (axis, value) in [("w", "min"), ("x", "face:12"), ("x", "missing"), ("x", "")] {
        let s = Spec {
            origin: std::collections::BTreeMap::from([(axis.into(), value.into())]),
            ..Default::default()
        };
        assert!(model::plan(&a, &s).is_err(), "{axis} {value}");
    }
    assert!(
        model::edit(
            &Spec::default(),
            &serde_json::json!([{"op":"set","path":"origin/w","value":"min"}])
        )
        .is_err()
    );
}
#[test]
fn competing_authored_locations_are_kept_and_reported_by_both_checks() {
    let mut p = PartPmi::default();
    distance(&mut p, 0, 12, 95., false);
    distance(&mut p, 5, 12, 5., false);
    let b = bytes(p);
    let (part, a) = model::analyse_native_step(Path::new("plate.step"), &b).unwrap();
    let result = plan(&a, &Spec::default());
    for label in ["95 ±0.05", "5 ±0.05"] {
        assert!(result.dimensions.iter().any(|d| d.label == label));
    }
    let problem = result
        .problems
        .iter()
        .find(|p| p.code == "feature_located_twice")
        .expect("competing origins are reported");
    assert!(
        problem
            .subjects
            .iter()
            .filter(|s| s.starts_with("d:pmi:"))
            .count()
            >= 2
    );
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    let drawing = model::render(&part, &a, &result, &font).unwrap();
    let found = draftwright::lint::location::coverage(&a, &draftwright::lint::marks(&drawing));
    assert!(found.iter().any(
        |p| p.code == "feature_located_twice" && p.subjects.first() == problem.subjects.first()
    ));
}
#[test]
fn position_tolerances_cite_and_draw_datums_at_either_end() {
    for b in [0, 5] {
        let mut p = PartPmi::default();
        for (letter, f) in [("A", 2), ("B", b), ("C", 1)] {
            let f = face(&mut p, f);
            p.datums
                .push(Datum::new(DatumLabel::new(letter).unwrap(), Some(f), vec![]).unwrap());
        }
        let a = analysed(PLATE);
        for f in a.features.iter().filter(|f| f.kind == "hole") {
            let x = f.fingerprint.position[0] - a.envelope[0][0];
            let y = f.fingerprint.position[1] - a.envelope[0][1];
            distance(
                &mut p,
                b,
                f.faces[0],
                if b == 5 { 100. - x } else { x },
                true,
            );
            distance(&mut p, 1, f.faces[0], y, true);
            let target = face(&mut p, f.faces[0]);
            p.tolerances.push(GeometricTolerance {
                kind: ToleranceKind::Position,
                target: ToleranceTarget::Feature(target),
                magnitude: Some(mm(0.1)),
                zone: None,
                modifiers: vec![],
                unit_basis: None,
                maximum: None,
                unequal: None,
                datums: Some(
                    DatumSystem::new(
                        (0..3)
                            .map(|d| Compartment {
                                references: vec![DatumReference {
                                    datum: DatumId(d),
                                    modifiers: vec![],
                                }],
                                modifiers: vec![],
                            })
                            .collect(),
                    )
                    .unwrap(),
                ),
                auxiliary: vec![],
                description: None,
            });
        }
        let a = analysed(&bytes(p));
        let result = plan(&a, &Spec::default());
        assert_eq!(
            result
                .gdt
                .iter()
                .filter(|g| g.kind == "datum_feature" && g.placed.is_some())
                .count(),
            3
        );
        assert!(!result.dimensions.iter().any(|d| {
            !d.id.starts_with("d:pmi:")
                && d.measurements
                    .iter()
                    .any(|m| m.parameter.contains("pitch.length"))
        }));
        assert!(
            !result
                .dimensions
                .iter()
                .any(|d| d.id.ends_with("location.length.x"))
        );
    }
}
#[test]
fn cylindrical_datum_reports_the_origin_fallback_without_drawing_a_symbol() {
    let mut p = PartPmi::default();
    let f = face(&mut p, 12);
    p.datums
        .push(Datum::new(DatumLabel::new("B").unwrap(), Some(f), vec![]).unwrap());
    let a = analysed(&bytes(p));
    let result = plan(&a, &Spec::default());
    assert!(
        result
            .problems
            .iter()
            .any(|p| p.code == "datum_origin_unusable" && p.severity == "info")
    );
    assert!(
        result
            .dimensions
            .iter()
            .any(|d| d.id.ends_with("location.length.x") && d.value == 95.)
    );
    assert!(result.gdt.iter().all(|g| g.kind != "datum_feature"));
}
#[test]
fn strict_semantic_thread_callouts_do_not_add_engine_drill_sizes() {
    let a = analysed(include_bytes!("fixtures/parts/editor_pmi.step"));
    let p = plan(
        &a,
        &Spec {
            pmi: PmiMode::Strict,
            ..Default::default()
        },
    );
    let callouts: Vec<_> = p
        .dimensions
        .iter()
        .filter(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
        .collect();
    assert!(!callouts.is_empty());
    for d in callouts {
        assert!(d.label.contains("M5"), "{}", d.label);
        assert!(!d.label.contains("⌀4.2"), "{}", d.label);
    }
}
#[test]
fn one_pattern_member_plus_pitches_locates_the_array_without_a_second_origin() {
    let mut p = PartPmi::default();
    distance(&mut p, 0, 6, 20., true);
    distance(&mut p, 1, 6, 15., true);
    let a = analysed(&bytes(p));
    let result = plan(&a, &Spec::default());
    let failures: Vec<_> = result
        .problems
        .iter()
        .filter(|p| p.code == "feature_located_twice" || p.code == "feature_not_located")
        .collect();
    assert!(failures.is_empty(), "{failures:?}");
    assert!(
        result
            .dimensions
            .iter()
            .any(|d| d.id.contains("pitch.length"))
    );
}
#[test]
fn one_pmi_axis_does_not_cover_an_unrelated_coincident_hole_axis() {
    let mut p = PartPmi::default();
    distance(&mut p, 1, 12, 30., true);
    let a = analysed(&bytes(p));
    let result = plan(&a, &Spec::default());
    let failures: Vec<_> = result
        .problems
        .iter()
        .filter(|p| p.code == "feature_located_twice" || p.code == "feature_not_located")
        .collect();
    assert!(failures.is_empty(), "{failures:?}");
    let engine = result
        .dimensions
        .iter()
        .filter(|d| !d.id.starts_with("d:pmi:"))
        .flat_map(|d| &d.measurements)
        .any(|m| m.feature == "f:hole:4" && m.parameter == "location.length.y");
    assert!(engine, "the unrelated ⌀8 hole still needs its y location");
}
#[test]
fn ignore_renders_the_plain_drawing_including_cosmetic_ink() {
    let (part, a) = model::analyse_native_step(
        Path::new("plate.step"),
        include_bytes!("fixtures/parts/editor_pmi.step"),
    )
    .unwrap();
    let mut plain = a.clone();
    plain.pmi = None;
    let expected = plan(&plain, &Spec::default());
    let ignored = plan(
        &a,
        &Spec {
            pmi: PmiMode::Ignore,
            ..Default::default()
        },
    );
    let locating = |p: &model::Plan| {
        p.problems
            .iter()
            .filter(|p| p.code == "feature_not_located" || p.code == "feature_located_twice")
            .map(|p| &p.id)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(locating(&expected), locating(&ignored));
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    let mut first = model::render(&part, &plain, &expected, &font).unwrap();
    let mut second = model::render(&part, &a, &ignored, &font).unwrap();
    first.spec = None;
    second.spec = None;
    assert_eq!(first.svg(), second.svg());
}

#[test]
fn located_once_checks_native_ink_and_rejects_a_broken_witness() {
    let (part, a) = model::analyse_native_step(Path::new("plate.step"), PLATE).unwrap();
    let p = plan(
        &a,
        &Spec {
            hole_table: draftwright::hole_table::Setting::Never,
            ..Default::default()
        },
    );
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    let mut drawing = model::render(&part, &a, &p, &font).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| {
            d.placed.is_some()
                && d.measurements
                    .first()
                    .is_some_and(|m| m.feature == "f:hole:7" && m.parameter == "location.length.x")
        })
        .unwrap();
    let has_gap = |drawing: &draftwright::output::Drawing| {
        draftwright::lint::lint(&a, drawing).iter().any(|i| {
            i.code == "feature_not_located"
                && i.subjects.contains(&"f:hole:7".into())
                && i.id.ends_with("location.length.x")
        })
    };
    assert!(!has_gap(&drawing));
    let item = drawing
        .sheet
        .items
        .iter_mut()
        .find(|i| i.name == d.id)
        .unwrap();
    item.lines.clear();
    assert!(
        has_gap(&drawing),
        "source claims cannot repair missing witness ink"
    );
}

#[test]
fn retained_pmi_callouts_do_not_repeat_sizes_or_locations_in_generated_tables() {
    use std::io::Read as _;
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(
        include_bytes!("fixtures/pmi/step/nist_ftc_10_asme1_ap242-e2.stp.gz").as_slice(),
    )
    .read_to_end(&mut step)
    .unwrap();
    let a = analysed(&step);
    let p = plan(&a, &Spec::default());
    let tables: std::collections::BTreeSet<_> = p
        .hole_table
        .iter()
        .chain(&p.hole_tables)
        .map(|t| t.id.as_str())
        .collect();
    assert!(
        !tables.is_empty(),
        "the regression needs a generated coordinate schedule"
    );
    assert!(
        !p.problems.iter().any(|q| q.code == "feature_located_twice"
            && q.subjects.iter().any(|s| tables.contains(s.as_str()))),
        "{:?}",
        p.problems
            .iter()
            .filter(|q| q.code == "feature_located_twice")
            .collect::<Vec<_>>()
    );
    let retained: std::collections::BTreeSet<_> = p
        .dimensions
        .iter()
        .filter(|d| {
            d.kind == "callout"
                && d.placed.is_some()
                && d.measurements.iter().any(|m| m.feature.starts_with("pmi:"))
        })
        .flat_map(|d| &d.measurements)
        .filter(|m| m.feature.starts_with("f:hole:"))
        .map(|m| m.feature.as_str())
        .collect();
    let mut checked = 0;
    for row in p
        .hole_table
        .iter()
        .chain(&p.hole_tables)
        .flat_map(|t| &t.rows)
    {
        if retained.contains(row.feature.as_str()) {
            assert!(
                row.cells[1].is_empty(),
                "source size is already drawn for {}",
                row.feature
            );
            assert!(
                row.measurements
                    .iter()
                    .all(|m| m.parameter.starts_with("location."))
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the fixture must exercise retained PMI size callouts"
    );
}
