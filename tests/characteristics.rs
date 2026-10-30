//! Numbered AP242 PMI exercises the real lossless writer, reader and drawing pipeline.
use draftwright::{
    api,
    characteristics::{self, Mode},
    haecceity::{
        self,
        p21::{self, Attribute as A, Document, RawEntity},
    },
    items,
    model::{self, Spec},
    pmi::{Category, Field, Reading},
    text::Font,
};
use std::{collections::BTreeMap, io::Read as _, path::Path};
fn bytes(name: &str) -> Vec<u8> {
    let b = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(name)).unwrap();
    if name.ends_with(".gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&b[..])
            .read_to_end(&mut out)
            .unwrap();
        out
    } else {
        b
    }
}
fn analysed(b: &[u8]) -> (haecceity::Part, model::Analysis) {
    model::analyse_native_bytes("numbered.step", b).unwrap()
}
fn on() -> Spec {
    let mut s = Spec::default();
    s.sheet.balloons = Mode::All;
    s
}
fn report(a: &model::Analysis) -> &draftwright::pmi::Report {
    let Some(Reading::Read(r)) = &a.pmi else {
        panic!("PMI")
    };
    r
}
/// A small set of existing drawn requirements on a real part. Original instances and geometry
/// are untouched; the writer appends only UDA properties and representation items.
fn fixture(name: &str) -> (Vec<u8>, usize) {
    let b = bytes(name);
    let (_, mut a) = analysed(&b);
    if let Some(Reading::Read(r)) = &mut a.pmi {
        for (i, r) in r.records.iter_mut().enumerate() {
            if matches!(
                r.category,
                Category::Dimension | Category::GeometricTolerance | Category::Datum
            ) {
                r.fields.push((
                    "characteristic number".into(),
                    Field::Text((i + 101).to_string()),
                ));
            }
        }
    }
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let mut candidates: Vec<_> = p
        .characteristics
        .iter()
        .filter(|c| c.drawn_item.is_some())
        .collect();
    candidates.sort_by_key(|c| {
        let r = report(&a)
            .records
            .iter()
            .find(|r| r.source_id == c.source)
            .unwrap();
        (
            r.category,
            std::cmp::Reverse(r.geometry.as_ref().map_or(0, |g| g.cylinders.len())),
        )
    });
    let mut counts = BTreeMap::new();
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|c| {
            let r = report(&a)
                .records
                .iter()
                .find(|r| r.source_id == c.source)
                .unwrap();
            let n = counts.entry(r.category).or_insert(0);
            *n += 1;
            *n <= 2
        })
        .collect();
    assert!(!selected.is_empty());
    let doc = Document::parse(b).unwrap();
    let ctx = doc
        .ids()
        .find_map(|id| match doc.get(id).unwrap() {
            RawEntity::Simple {
                name, attributes, ..
            } if name == "REPRESENTATION_CONTEXT" => Some(A::EntityRef(id)),
            RawEntity::Complex { parts, .. }
                if parts.iter().any(|p| p.name == "REPRESENTATION_CONTEXT") =>
            {
                Some(A::EntityRef(id))
            }
            _ => None,
        })
        .unwrap();
    let mut e = p21::Edit::new();
    // Edition 4 extends characterized_definition to size/tolerance items (UDA owners).
    // The lossless edit keeps all source instances, upgrading only the FILE_SCHEMA header.
    e.set_file_schema(vec![haecceity::express::TARGET_SCHEMA.into()]);
    for (i, c) in selected.iter().enumerate() {
        let r = report(&a)
            .records
            .iter()
            .find(|r| r.source_id == c.source)
            .unwrap();
        let n = 101 + i as i64 * 7;
        let val = if i % 2 == 0 {
            A::String(n.to_string())
        } else {
            A::Integer(n)
        };
        let v = e.add(p21::simple(
            if i % 2 == 0 {
                "DESCRIPTIVE_REPRESENTATION_ITEM"
            } else {
                "INTEGER_REPRESENTATION_ITEM"
            },
            vec![A::String("characteristic number".into()), val],
        ));
        let rep = e.add(p21::simple(
            "REPRESENTATION",
            vec![A::String("".into()), A::List(vec![v.to_ref()]), ctx.clone()],
        ));
        let prop = e.add(p21::simple(
            "PROPERTY_DEFINITION",
            vec![
                A::String("characteristic number".into()),
                A::String("user defined attribute".into()),
                A::EntityRef(r.part21_id),
            ],
        ));
        e.add(p21::simple(
            "PROPERTY_DEFINITION_REPRESENTATION",
            vec![prop.to_ref(), rep.to_ref()],
        ));
        let gp = e.add(p21::simple(
            "GENERAL_PROPERTY",
            vec![
                A::String("".into()),
                A::String("characteristic number".into()),
                A::Unset,
            ],
        ));
        e.add(p21::simple(
            "GENERAL_PROPERTY_ASSOCIATION",
            vec![A::String("".into()), A::Unset, gp.to_ref(), prop.to_ref()],
        ));
    }
    let written = doc.apply(&e).unwrap();
    let d = Document::parse(written.bytes.clone()).unwrap();
    let edition = haecceity::express::edition(&d.file_schema().unwrap()[0]).unwrap();
    let errors: Vec<_> = edition
        .validate_document(d.graph())
        .into_iter()
        .filter(|v| v.id > doc.max_id())
        .collect();
    assert!(errors.is_empty(), "{name}: {errors:?}");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/balloons-fixtures");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(
            Path::new(name)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches(".gz"),
        ),
        &written.bytes,
    )
    .unwrap();
    (written.bytes, selected.len())
}
fn draw(
    part: &haecceity::Part,
    a: &model::Analysis,
    p: &model::Plan,
) -> draftwright::output::Drawing {
    model::render(part, a, p, &Font::new(api::MONO.to_vec()).unwrap()).unwrap()
}
#[test]
fn numbered_pmi_round_trips_to_items_and_clear_rings() {
    for name in [
        "tests/fixtures/parts/editor_pmi.step",
        "tests/fixtures/pmi/step/GRM-03_thumbwheel_drive_screw_AP242_PMI.step.gz",
        "tests/fixtures/pmi/step/nist_ctc_02_asme1_ap242-e2.stp.gz",
    ] {
        let (b, n) = fixture(name);
        let (part, a) = analysed(&b);
        let off = model::plan_recovered(&a, &Spec::default()).unwrap();
        let p = model::plan_recovered(&a, &on()).unwrap();
        let stem = Path::new(name).file_stem().unwrap().to_string_lossy();
        std::fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("target/balloons-fixtures/{stem}.plan.json")),
            serde_json::to_vec(&p).unwrap(),
        )
        .unwrap();
        eprintln!("{name}: {n} numbers, {} rings", p.inspection_balloons.len());
        assert_eq!(p.characteristics.len(), n, "{name}");
        assert_eq!(
            p.inspection_balloons.len(),
            n,
            "{name}: {:?}",
            p.problems
                .iter()
                .filter(|p| p.code.starts_with("characteristic"))
                .collect::<Vec<_>>()
        );
        assert!(off.inspection_balloons.is_empty());
        assert!(
            p.characteristics.iter().all(|c| items::items(&p)
                .iter()
                .any(|i| Some(&i.id) == c.item.as_ref()
                    && i.characteristic_numbers.contains(&c.number)))
        );
        let again = model::plan_recovered(&a, &on()).unwrap();
        assert_eq!(p.inspection_balloons, again.inspection_balloons);
        let d = draw(&part, &a, &p);
        let old = draw(&part, &a, &off);
        let counted = |d: &draftwright::output::Drawing| {
            let mut counts = BTreeMap::new();
            for i in draftwright::lint::structural::check(d) {
                if [
                    "annotation_overlap",
                    "annotation_ink_overlap",
                    "annotation_leader_crossing",
                ]
                .contains(&i.code.as_str())
                {
                    *counts.entry(i.code).or_insert(0usize) += 1;
                }
            }
            counts
        };
        assert_eq!(counted(&old), counted(&d), "{name}");
        assert!(
            !draftwright::lint::lint(&a, &d)
                .iter()
                .any(|i| i.code.starts_with("characteristic")),
            "{name}"
        );
        let mut broken = d.clone();
        let id = &p.inspection_balloons[0].id;
        broken.sheet.items.retain(|i| &i.name != id);
        assert!(
            draftwright::lint::lint(&a, &broken)
                .iter()
                .any(|i| i.code == "characteristic_balloon_missing")
        );
    }
}
#[test]
fn suppression_and_unavailable_characteristics_are_reported() {
    let (b, _) = fixture("tests/fixtures/parts/editor_pmi.step");
    let (_, a) = analysed(&b);
    let mut s = on();
    let p = model::plan_recovered(&a, &s).unwrap();
    let c = &p.characteristics[0];
    s.balloons.items.insert(
        c.source.clone(),
        characteristics::Override {
            suppress: true,
            position: None,
        },
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.problems
            .iter()
            .any(|p| p.code == "characteristic_balloon_missing")
    );
    let mut s = on();
    for c in &p.characteristics {
        if let Some(i) = &c.item {
            if p.dimensions.iter().any(|d| &d.id == i) {
                s.dimensions.entry(i.clone()).or_default().omit = true;
            } else {
                s.annotations.entry(i.clone()).or_default().omit = true;
            }
        }
    }
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.problems
            .iter()
            .any(|p| p.code == "characteristic_not_drawn")
    );
}
#[test]
fn numbers_survive_relayout_and_each_ring_stays_on_its_carriers_sheet() {
    let (b, _) = fixture("tests/fixtures/parts/editor_pmi.step");
    let (part, a) = analysed(&b);
    let p = model::plan_recovered(&a, &on()).unwrap();
    // The current allocator keeps annotation-bearing views on sheet 1. An unnumbered
    // authored table on sheet 2 verifies that rendering does not repeat their rings there.
    let s:Spec=serde_json::from_value(serde_json::json!({"sheet":{"balloons":"all"},"sheets":{"max":2},"tables":[{"id":"t:1","rows":[["GENERAL"],["Unnumbered row"]],"sheet":2}]})).unwrap();
    let q = model::plan_recovered(&a, &s).unwrap();
    assert!(!q.sheets.is_empty());
    assert_eq!(
        p.characteristics
            .iter()
            .map(|c| (&c.source, &c.number))
            .collect::<Vec<_>>(),
        q.characteristics
            .iter()
            .map(|c| (&c.source, &c.number))
            .collect::<Vec<_>>()
    );
    let pages =
        model::render_sheets(&part, &a, &q, &Font::new(api::MONO.to_vec()).unwrap()).unwrap();
    for (index, page) in pages.iter().enumerate() {
        for b in &q.inspection_balloons {
            assert_eq!(
                page.sheet.items.iter().any(|i| i.name == b.id),
                b.sheet as usize == index + 1
            );
        }
    }
    for b in &q.inspection_balloons {
        let c = q
            .characteristics
            .iter()
            .find(|c| c.source == b.source)
            .unwrap();
        assert_eq!(c.sheet, Some(b.sheet));
    }
}

#[test]
fn grouped_occurrences_and_relative_pins_keep_the_files_numbers() {
    let (b, _) = fixture("tests/fixtures/parts/editor_pmi.step");
    let (_, a) = analysed(&b);
    let mut s = on();
    let p = model::plan_recovered(&a, &s).unwrap();
    let grouped = p
        .characteristics
        .iter()
        .find(|c| {
            report(&a)
                .records
                .iter()
                .find(|r| r.source_id == c.source)
                .unwrap()
                .geometry
                .as_ref()
                .is_some_and(|g| g.cylinders.len() > 1)
        })
        .unwrap();
    assert_eq!(grouped.balloon_ids.len(), 1);
    s.balloons.occurrences = true;
    let many = model::plan_recovered(&a, &s).unwrap();
    let c = many
        .characteristics
        .iter()
        .find(|c| c.source == grouped.source)
        .unwrap();
    assert!(c.balloon_ids.len() > 2);
    assert!(
        many.inspection_balloons
            .iter()
            .filter(|b| b.source == c.source)
            .count()
            > 1,
        "{:?}",
        many.problems
            .iter()
            .filter(|p| p.code.starts_with("characteristic"))
            .collect::<Vec<_>>()
    );
    assert!(
        many.inspection_balloons
            .iter()
            .filter(|b| b.source == c.source)
            .all(|b| b.number == c.number)
    );
    let b = &p.inspection_balloons[0];
    let carrier = p
        .characteristics
        .iter()
        .find(|c| c.source == b.source)
        .unwrap()
        .drawn_item
        .as_deref()
        .unwrap();
    let rect = p
        .dimensions
        .iter()
        .find(|d| d.id == carrier)
        .and_then(|d| d.placed.as_ref())
        .map(|p| p.label_box)
        .or_else(|| {
            p.gdt
                .iter()
                .find(|g| g.id == carrier)
                .and_then(|g| g.placed.as_ref())
                .map(|p| p.label_box)
        })
        .unwrap();
    let offset = [
        b.at[0] - (rect[0] + rect[2]) / 2.,
        b.at[1] - (rect[1] + rect[3]) / 2.,
    ];
    s.balloons.occurrences = false;
    s.balloons.items.insert(
        b.source.clone(),
        characteristics::Override {
            suppress: false,
            position: Some(offset),
        },
    );
    let q = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(
        q.inspection_balloons
            .iter()
            .find(|c| c.source == b.source)
            .unwrap()
            .at,
        b.at
    );
    s.balloons.items.get_mut(&b.source).unwrap().position = Some([0., 0.]);
    let q = model::plan_recovered(&a, &s).unwrap();
    assert!(!q.inspection_balloons.iter().any(|c| c.source == b.source));
    assert!(
        q.problems
            .iter()
            .any(|p| p.code == "characteristic_balloon_missing")
    );
}

#[test]
fn hole_rows_use_number_rings_separate_from_their_table_tags() {
    let (b, _) = fixture("tests/fixtures/parts/editor_pmi.step");
    let (part, a) = analysed(&b);
    let mut s = on();
    s.hole_table = draftwright::hole_table::Setting::Always;
    let p = model::plan_recovered(&a, &s).unwrap();
    std::fs::write(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/balloons-fixtures/table.plan.json"),
        serde_json::to_vec(&p).unwrap(),
    )
    .unwrap();
    let rows: Vec<_> = p
        .characteristics
        .iter()
        .filter(|c| {
            c.drawn_item
                .as_ref()
                .is_some_and(|id| id.starts_with("hole_schedule_"))
        })
        .collect();
    assert!(!rows.is_empty(), "{:?}", p.characteristics);
    for c in rows {
        assert_eq!(
            p.inspection_balloons
                .iter()
                .filter(|b| b.source == c.source && Some(&b.item) == c.drawn_item.as_ref())
                .count(),
            1,
            "{:?}",
            p.problems
        );
    }
    assert!(!p.balloons.is_empty());
    assert!(
        p.inspection_balloons
            .iter()
            .all(|b| b.number.bytes().all(|b| b.is_ascii_digit()))
    );
    let d = draw(&part, &a, &p);
    assert!(
        !draftwright::lint::lint(&a, &d)
            .iter()
            .any(|i| i.code.starts_with("characteristic"))
    );
}
