//! Datum placement regressions: inside dimension lanes and valid alternative attachments.
use std::path::Path;

use draftwright::dims::{PlannedDimension, gdt};
use draftwright::model::{self, Analysis, Plan, Spec};

fn spindle() -> (Analysis, Plan) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/samples/cnc-review-spindle.step.gz");
    let (_, a) = model::analyse_native(&path).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    (a, plan)
}

fn only_datum(a: &mut Analysis, letter: &str) {
    let Some(draftwright::pmi::Reading::Read(report)) = &mut a.pmi else {
        panic!("spindle has PMI")
    };
    report.records.retain(|r| {
        r.label == letter || r.category == draftwright::pmi::Category::GeometricTolerance
    });
}

fn isolated_spec(a: &Analysis) -> Spec {
    let mut s = Spec::default();
    if let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi {
        let ids = report.identities();
        for r in report
            .records
            .iter()
            .filter(|r| r.category == draftwright::pmi::Category::GeometricTolerance)
        {
            if let Ok(id) = ids.id(&r.source_id) {
                s.annotations.insert(
                    id.into(),
                    serde_json::from_value(serde_json::json!({"omit":true})).unwrap(),
                );
            }
        }
    }
    s
}

fn obstacle(plan: &Plan, line: [[f64; 2]; 2]) -> PlannedDimension {
    let mut d = plan
        .dimensions
        .iter()
        .find(|d| d.placed.is_some())
        .unwrap()
        .clone();
    d.id = "test:dimension".into();
    d.view = "v:front".into();
    let p = d.placed.as_mut().unwrap();
    p.lines = vec![line];
    p.arrows.clear();
    p.label_box = [350.0, 250.0, 360.0, 253.0];
    d
}

fn place(a: &Analysis, p: &Plan, dims: &[PlannedDimension]) -> gdt::PlannedGdt {
    let (items, _) = gdt::plan(a, &isolated_spec(a), &p.sheet, &p.views, dims);
    let datums: Vec<_> = items
        .into_iter()
        .filter(|g| g.kind == "datum_feature")
        .collect();
    assert_eq!(datums.len(), 1);
    datums.into_iter().next().unwrap()
}

/// Resetting an editor placement leaves an empty entry and restores every automatic choice.
fn reset_matches_automatic(a: &Analysis, p: &Plan, dims: &[PlannedDimension], g: &gdt::PlannedGdt) {
    let mut reset = isolated_spec(a);
    reset.annotations.insert(
        g.id.clone(),
        serde_json::from_value(serde_json::json!({"place":{}})).unwrap(),
    );
    let (items, _) = gdt::plan(a, &reset, &p.sheet, &p.views, dims);
    let item = items.iter().find(|i| i.id == g.id).unwrap();
    assert_eq!(item.placed, g.placed);
    assert_eq!(item.side, g.side);
}

#[test]
fn spindle_datums_have_no_crossings_or_overlaps() {
    let (a, plan) = spindle();
    assert_eq!(
        (plan.sheet.size.as_str(), plan.sheet.scale.as_str()),
        ("A4", "1:1")
    );
    let ids: Vec<_> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature")
        .map(|g| &g.id)
        .collect();
    assert_eq!(ids.len(), 2);
    let findings: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| {
            p.subjects.iter().any(|id| ids.contains(&id))
                && (p.code.contains("crossing")
                    || p.code.contains("overlap")
                    || p.code == "annotation_outside_area")
        })
        .collect();
    assert!(findings.is_empty(), "{findings:#?}");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (part, _) =
        model::analyse_native(&root.join("tests/fixtures/samples/cnc-review-spindle.step.gz"))
            .unwrap();
    let font =
        draftwright::text::Font::open(&root.join("fonts").join(draftwright::dims::FONT)).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let findings: Vec<_> = draftwright::lint::structural::check(&drawing)
        .into_iter()
        .filter(|p| p.subjects.iter().any(|id| ids.contains(&id)))
        .collect();
    assert!(findings.is_empty(), "rendered datum lint: {findings:#?}");
}

#[test]
fn datum_prefers_inside_the_first_dimension_lane() {
    let (mut a, p) = spindle();
    only_datum(&mut a, "B");
    let original = place(&a, &p, &[]);
    let tip = original.placed.unwrap().tip;
    let lane = tip[0] + 35.0;
    let d = obstacle(&p, [[lane, tip[1] - 10.0], [lane, tip[1] + 10.0]]);
    let g = place(&a, &p, std::slice::from_ref(&d));
    assert_eq!(g.side, "right");
    reset_matches_automatic(&a, &p, &[d], &g);
    let b = g.placed.unwrap().label_box;
    assert!(
        b[0] > tip[0] && b[2] < lane - 1.0,
        "inside gap: {b:?}, lane {lane}"
    );
}

#[test]
fn cylinder_datum_uses_the_other_proven_silhouette() {
    let (mut a, p) = spindle();
    only_datum(&mut a, "B");
    let original = place(&a, &p, &[]).placed.unwrap().tip;
    let d = obstacle(
        &p,
        [
            [original[0] + 5.0, 10.0],
            [original[0] + 5.0, p.sheet.page[1] - 10.0],
        ],
    );
    let g = place(&a, &p, std::slice::from_ref(&d));
    reset_matches_automatic(&a, &p, &[d], &g);
    assert_eq!(g.side, "left");
    let placed = g.placed.unwrap();
    assert!(placed.tip[0] < original[0]);
    // Both attachment points are on the same cylinder, at opposite radial extremes.
    let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi else {
        unreachable!()
    };
    let datum = report
        .records
        .iter()
        .find(|r| r.category == draftwright::pmi::Category::Datum && r.label == "B")
        .unwrap();
    let geometry = datum.geometry.as_ref().unwrap();
    let face = &a.face_attachments[datum.faces[0]];
    assert_eq!(face.surface.as_deref(), Some("cylinder"));
    assert!(!geometry.sites.is_empty());
    let view = p.views.iter().find(|v| v.id == "v:front").unwrap();
    let centre = [0, 1, 2].map(|k| (a.envelope[0][k] + a.envelope[1][k]) / 2.0);
    assert!(
        face.sites.iter().any(|s| {
            let page = [
                view.at[0] + view.scale * (s[0] - centre[0]),
                view.at[1] + view.scale * (s[2] - centre[2]),
            ];
            (page[0] - placed.tip[0]).hypot(page[1] - placed.tip[1]) < 1e-6
        }),
        "opposite silhouette must be a reader-proven trimmed-face point"
    );
}

#[test]
fn planar_datum_stands_on_a_face_extension_line() {
    let (mut a, p) = spindle();
    only_datum(&mut a, "A");
    let original = place(&a, &p, &[]).placed.unwrap().tip;
    let view = p.views.iter().find(|v| v.id == "v:front").unwrap();
    let half_width = (a.envelope[1][0] - a.envelope[0][0]) * view.scale / 2.0;
    let ends = [view.at[0] - half_width, view.at[0] + half_width];
    let d = obstacle(
        &p,
        [
            [ends[0] - 1.0, original[1] - 6.0],
            [ends[1] + 1.0, original[1] - 6.0],
        ],
    );
    let g = place(&a, &p, std::slice::from_ref(&d));
    reset_matches_automatic(&a, &p, &[d], &g);
    assert_eq!(g.side, "below");
    let placed = g.placed.unwrap();
    assert!(
        placed.tip[0] < ends[0] || placed.tip[0] > ends[1],
        "extension tip {:?}, outline {ends:?}",
        placed.tip
    );
    assert!((placed.tip[1] - original[1]).abs() < 1e-6);
    assert!(
        placed
            .lines
            .iter()
            .any(|[u, v]| (u[1] - original[1]).abs() < 1e-6
                && (v[1] - original[1]).abs() < 1e-6
                && u[0].min(v[0]) <= placed.tip[0]
                && u[0].max(v[0]) >= placed.tip[0]),
        "triangle must stand on the drawn extension"
    );
}

#[test]
fn cited_spindle_datums_have_separate_attachments_and_short_leaders() {
    let (_, plan) = spindle();
    let datum = |letter: &str| {
        plan.gdt
            .iter()
            .find(|g| g.kind == "datum_feature" && g.label == letter)
            .unwrap()
    };
    let (a, b) = (datum("A"), datum("B"));
    let (pa, pb) = (a.placed.as_ref().unwrap(), b.placed.as_ref().unwrap());
    let height = pa.label_box[3] - pa.label_box[1];
    assert!(
        (pa.tip[0] - pb.tip[0]).hypot(pa.tip[1] - pb.tip[1]) > height,
        "{pa:?} / {pb:?}"
    );
    let length: f64 = pb
        .lines
        .iter()
        .map(|[a, b]| (b[0] - a[0]).hypot(b[1] - a[1]))
        .sum();
    assert!(length < 20., "B leader: {length}");
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "annotation_leader_crossing" && p.subjects.contains(&b.id)),
        "{:?}",
        plan.problems
            .iter()
            .filter(|p| p.subjects.contains(&b.id))
            .collect::<Vec<_>>()
    );
    for fa in &pa.fills {
        for fb in &pb.fills {
            let bounds = |f: &Vec<[f64; 2]>| {
                f.iter().fold(
                    [
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ],
                    |r, p| {
                        [
                            r[0].min(p[0]),
                            r[1].min(p[1]),
                            r[2].max(p[0]),
                            r[3].max(p[1]),
                        ]
                    },
                )
            };
            let (a, b) = (bounds(fa), bounds(fb));
            assert!(
                a[2] < b[0] || b[2] < a[0] || a[3] < b[1] || b[3] < a[1],
                "triangles overlap: {a:?} / {b:?}"
            );
        }
    }
}

#[test]
fn grm03_declared_origin_datums_have_symbols_without_a_tolerance_citation() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step/GRM-03_thumbwheel_drive_screw_AP242_PMI.step.gz");
    let (_, a) = model::analyse_native(&path).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi else {
        panic!("GRM has PMI")
    };
    assert_eq!(
        report
            .records
            .iter()
            .filter(|r| r.category == draftwright::pmi::Category::Datum)
            .count(),
        2
    );
    let datums: Vec<_> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature")
        .collect();
    assert_eq!(datums.len(), 2, "every declared datum is author intent");
    assert!(datums.iter().all(|g| g.placed.is_some()));
    assert!(
        plan.problems.iter().all(|p| !matches!(
            p.code.as_str(),
            "annotation_leader_crossing" | "annotation_overlap" | "annotation_ink_overlap"
        )),
        "{:#?}",
        plan.problems
    );
}

#[test]
fn overlapping_datum_triangles_are_errors_even_with_separate_letter_boxes() {
    let (_, plan) = spindle();
    assert!(gdt::symbol_collisions(&plan.gdt).is_empty());
    let mut datums: Vec<_> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature")
        .cloned()
        .collect();
    assert_eq!(datums.len(), 2);
    // Corrupt only the filled ink: keeping the letters and leaders apart must
    // not conceal a symbol overlap (including a placement pinned by an editor).
    datums[1].placed.as_mut().unwrap().fills = datums[0].placed.as_ref().unwrap().fills.clone();
    let issues = gdt::symbol_collisions(&datums);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, "datum_symbol_overlap");
    assert_eq!(issues[0].severity, "error");
    assert!(datums.iter().all(|g| issues[0].subjects.contains(&g.id)));
}

#[test]
fn an_already_separated_seed_does_not_gain_datum_crossings() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pmi/step/hw_every_kind.step.gz");
    let (_, a) = model::analyse_native(&path).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(gdt::symbol_collisions(&plan.gdt).is_empty());
    let ids: Vec<_> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature")
        .map(|g| &g.id)
        .collect();
    let crossings: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| {
            p.code == "annotation_leader_crossing" && p.subjects.iter().any(|s| ids.contains(&s))
        })
        .collect();
    assert!(
        crossings.is_empty(),
        "the legal seed has no datum-leader crossings: {crossings:#?}"
    );
}

fn dense_datums_are_separate(name: &str, expected: usize) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step")
        .join(name);
    let (_, a) = model::analyse_native(&path).unwrap();
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    let shown: Vec<_> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "datum_feature" && g.placed.is_some())
        .collect();
    assert_eq!(
        shown.len(),
        expected,
        "separate the symbols without dropping datums"
    );
    let collisions = gdt::symbol_collisions(&plan.gdt);
    assert!(
        collisions.is_empty(),
        "coincident projections still need distinct symbols: {collisions:#?}"
    );
}

#[test]
fn ftc10_coincident_projected_datums_do_not_touch() {
    dense_datums_are_separate("nist_ftc_10_asme1_ap242-e2.stp.gz", 8);
}

#[test]
fn stc10_coincident_projected_datums_do_not_touch() {
    dense_datums_are_separate("nist_stc_10_asme1_ap242-e2.stp.gz", 8);
}

#[test]
fn ftc07_coincident_projected_datums_do_not_touch() {
    dense_datums_are_separate("nist_ftc_07_asme1_ap242-e2.stp.gz", 5);
}

#[test]
fn stc09_nearby_projected_datums_do_not_touch() {
    dense_datums_are_separate("nist_stc_09_asme1_ap242-e4.stp.gz", 5);
}
