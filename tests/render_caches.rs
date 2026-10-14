#[path = "common/corpus.rs"]
mod corpus;

use std::path::Path;
use std::time::Instant;

use draftwright::api;
use draftwright::model::{self, Spec};
use draftwright::text::Font;
use serde_json::json;

fn check(file: &str, only: &str) {
    let path = if file == "mounting_plate.step" {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/parts")
            .join(file)
    } else {
        let Some(dir) = corpus::dir() else { return };
        dir.join(file)
    };
    let (part, analysis) = model::analyse_native(&path).unwrap();
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let mut spec = Spec::default();
    let plan = model::plan(&analysis, &spec).unwrap();
    spec.sheet.size = Some(plan.sheet.size.clone());
    spec.sheet.scale = Some(plan.sheet.scale.clone());
    let plan = model::plan(&analysis, &spec).unwrap();
    let id = plan
        .dimensions
        .iter()
        .find(|d| d.kind == "linear" && d.placed.is_some())
        .unwrap()
        .id
        .clone();
    let pointer = id.replace('~', "~0").replace('/', "~1");
    let mut cache = model::Projections::default();
    model::render_with(&part, &analysis, &plan, &font, &mut cache).unwrap();
    let mut previous = plan;
    for (edit, value) in [("omit", json!(true)), ("tolerance", json!("±0.1"))] {
        if edit != only {
            continue;
        }
        let changed = model::edit(
            &spec,
            &json!([{"op": "set", "path": format!("dimensions/{pointer}/{edit}"), "value": value}]),
        )
        .unwrap();
        let plan = model::plan(&analysis, &changed).unwrap();
        let before = cache.stats();
        let changed_views = plan
            .views
            .iter()
            .filter(|v| {
                !previous.views.iter().any(|old| {
                    old.direction == v.direction
                        && old.up == v.up
                        && old.cut == v.cut
                        && old.at == v.at
                        && old.scale == v.scale
                })
            })
            .count();
        let start = Instant::now();
        let kept = model::render_with(&part, &analysis, &plan, &font, &mut cache).unwrap();
        let elapsed = start.elapsed();
        eprintln!(
            "TIMING {file} {edit}: {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
        let fresh = model::render(&part, &analysis, &plan, &font).unwrap();
        assert_eq!(kept.svg(), fresh.svg(), "{file} {edit}");
        assert_eq!(kept.dxf(), fresh.dxf(), "{file} {edit}");
        assert_eq!(
            kept.pdf(&|_| Some(api::MONO.to_vec())),
            fresh.pdf(&|_| Some(api::MONO.to_vec())),
            "{file} {edit}"
        );
        assert_eq!(kept.sheet, fresh.sheet);
        assert_eq!(kept.annotations, fresh.annotations);
        assert_eq!(kept.problems, fresh.problems);
        assert_eq!(
            cache.stats().projections,
            before.projections,
            "{file} {edit}: HLR recomputed"
        );
        assert_eq!(
            cache.stats().bounds,
            before.bounds + changed_views,
            "{file} {edit}: unchanged view bounds recomputed"
        );
        let before = cache.stats();
        let repeated = model::render_with(&part, &analysis, &plan, &font, &mut cache).unwrap();
        assert_eq!(repeated.svg(), fresh.svg());
        assert_eq!(cache.stats(), before);
        previous = plan;
    }
}

#[test]
fn plate_omit_reuses_geometry() {
    check("mounting_plate.step", "omit");
}
#[test]
fn plate_tolerance_reuses_geometry() {
    check("mounting_plate.step", "tolerance");
}
#[test]
fn mfcadpp_omit_reuses_geometry() {
    check("mfcadpp/10127.step", "omit");
}
#[test]
fn mfcadpp_tolerance_reuses_geometry() {
    check("mfcadpp/10127.step", "tolerance");
}
#[test]
fn turned_omit_reuses_geometry() {
    check("gramel/GRM-03_thumbwheel_drive_screw.step", "omit");
}
#[test]
fn turned_tolerance_reuses_geometry() {
    check("gramel/GRM-03_thumbwheel_drive_screw.step", "tolerance");
}
#[test]
fn nist_omit_reuses_geometry() {
    check("nist/nist_ctc_01_asme1_rd.stp", "omit");
}
#[test]
fn nist_tolerance_reuses_geometry() {
    check("nist/nist_ctc_01_asme1_rd.stp", "tolerance");
}

#[test]
fn individual_items_preserve_ink_bounds_and_local_text_runs() {
    use draftwright::output::{Annotation, Item, Sheet, TextRun};
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step");
    let (part, a) = model::analyse_native(&path).unwrap();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let fresh = model::render(&part, &a, &plan, &font).unwrap();
    let mut items: Vec<Item> = Vec::new();
    let mut text: Vec<TextRun> = Vec::new();
    let mut annotations: Vec<Annotation> = Vec::new();
    let mut sheet = Sheet::default();
    for d in &plan.dimensions {
        let fragment = model::render_item(model::AnnotationItem::Dimension(d));
        if let Some(drawn) = fragment.drawn.first() {
            assert_eq!(
                Some(drawn),
                fresh.sheet.items.iter().find(|i| i.name == d.id)
            );
        }
        assert!(
            fragment
                .annotations
                .iter()
                .flat_map(|a| &a.runs)
                .all(|r| *r < fragment.text.len())
        );
        fragment.append(&mut items, &mut text, &mut annotations, &mut sheet);
    }
    assert_eq!(annotations, fresh.annotations);
    let n = items.len();
    assert_eq!(items, fresh.layers[2].items[..n]);
    // A view's visible bounds belong to geometry, independent of the item renderer.
    let mut cache = model::Projections::default();
    let geometry = model::render_geometry(&part, &a, &plan, &mut cache).unwrap();
    assert_eq!(geometry.views.len(), plan.views.len());
    for (name, view) in &geometry.views {
        let drawn = fresh.sheet.views.iter().find(|v| v.name == *name).unwrap();
        assert_eq!(view.bbox, Some(drawn.bbox));
        assert_eq!(
            view.visible,
            drawn
                .edges
                .iter()
                .cloned()
                .map(Item::Edge)
                .collect::<Vec<_>>()
        );
    }
    let split = model::render_annotations(&part, &a, &plan, &font, geometry, &mut cache).unwrap();
    assert_eq!(split.svg(), fresh.svg());
    assert_eq!(split.sheet, fresh.sheet);
}

#[test]
fn geometry_cache_tracks_placement_scale_direction_up_cut_and_part() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step");
    let (part, a) = model::analyse(&path).unwrap();
    let mut plan = model::plan(&a, &Spec::default()).unwrap();
    plan.views.truncate(1);
    let mut cache = model::Projections::default();
    for change in 0..8 {
        let old = cache.stats();
        let v = &mut plan.views[0];
        match change {
            1 => v.at[0] += 1.25,
            2 => v.scale *= 0.5,
            3 => v.direction = [1.0, 2.0, 3.0],
            4 => v.up = [1.0, 0.0, 0.0],
            5 => {
                v.cut = Some(model::Cut {
                    label: "A".into(),
                    origin: [30.0, 20.0, 2.5],
                    normal: [0.0, 1.0, 0.0],
                    marked_in: "v:plan".into(),
                    xref: None,
                })
            }
            6 => v.cut.as_mut().unwrap().origin[1] += 1.0,
            7 => v.cut.as_mut().unwrap().normal = [1.0, 0.0, 0.0],
            _ => (),
        }
        let kept = model::render_geometry(&part, &a, &plan, &mut cache).unwrap();
        let fresh =
            model::render_geometry(&part, &a, &plan, &mut model::Projections::default()).unwrap();
        assert_eq!(kept.views[0].1.visible, fresh.views[0].1.visible);
        assert_eq!(kept.views[0].1.hidden, fresh.views[0].1.hidden);
        assert_eq!(kept.views[0].1.bbox, fresh.views[0].1.bbox);
        assert_eq!(cache.stats().bounds, old.bounds + 1);
        assert_eq!(
            cache.stats().projections,
            old.projections + usize::from(![1, 2].contains(&change))
        );
        let stats = cache.stats();
        model::render_geometry(&part, &a, &plan, &mut cache).unwrap();
        assert_eq!(cache.stats(), stats);
    }
    let (other, a) = model::analyse(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/turned.step"),
    )
    .unwrap();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let before = cache.stats();
    let kept = model::render_geometry(&other, &a, &plan, &mut cache).unwrap();
    let fresh =
        model::render_geometry(&other, &a, &plan, &mut model::Projections::default()).unwrap();
    assert_eq!(
        cache.stats().projections,
        before.projections + plan.views.len()
    );
    for ((_, kept), (_, fresh)) in kept.views.iter().zip(&fresh.views) {
        assert_eq!(kept.visible, fresh.visible);
        assert_eq!(kept.hidden, fresh.hidden);
        assert_eq!(kept.bbox, fresh.bbox);
    }
}

fn pmi_fixture(stem: &str) -> Vec<u8> {
    use std::io::Read as _;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pmi/step");
    let path = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("{stem}."))
        })
        .unwrap();
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

#[test]
fn one_pmi_read_feeds_material_threads_and_frames_and_item_fragments() {
    use draftwright::pmi::{Reader, Reading};
    let bytes = pmi_fixture("hw_thumbwheel_notes_add");
    let (part, mut a) = model::analyse_native_step(Path::new("thumbwheel.step"), &bytes).unwrap();
    let original = a.pmi.clone();
    let mut reader = Reader::default();
    a.pmi = reader.attach(&bytes, &part);
    assert_eq!(reader.reads(), 1);
    assert_eq!(a.pmi, original);
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("PMI not read")
    };
    assert!(report.title_material().is_some());
    assert!(
        !draftwright::pmi_lowering::manufacturing(&a)
            .owned
            .is_empty()
    );
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert!(!plan.gdt.is_empty());
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let mut items = Vec::new();
    let mut text = Vec::new();
    let mut annotations = Vec::new();
    let mut sheet = draftwright::output::Sheet::default();
    // Start with a dimension so the frames' local indices must be rebased.
    for d in &plan.dimensions {
        model::render_item(model::AnnotationItem::Dimension(d)).append(
            &mut items,
            &mut text,
            &mut annotations,
            &mut sheet,
        );
    }
    for g in &plan.gdt {
        let fragment = model::render_item(model::AnnotationItem::Gdt(g));
        if let Some(drawn) = fragment.drawn.first() {
            assert_eq!(
                Some(drawn),
                drawing.sheet.items.iter().find(|i| i.name == g.id)
            );
        }
        fragment.append(&mut items, &mut text, &mut annotations, &mut sheet);
    }
    assert_eq!(items, drawing.layers[2].items[..items.len()]);
    assert_eq!(
        annotations,
        drawing
            .annotations
            .iter()
            .filter(|i| i.view.is_some())
            .cloned()
            .collect::<Vec<_>>()
    );
    for _ in 0..3 {
        a.pmi = reader.attach(&bytes.clone(), &part);
        model::plan(&a, &Spec::default()).unwrap();
    }
    assert_eq!(reader.reads(), 1);
    let changed = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("Stainless steel 303", "Stainless steel 304")
        .into_bytes();
    a.pmi = reader.attach(&changed, &part);
    assert_eq!(reader.reads(), 2);
    assert_ne!(a.pmi, original);
    let Some(Reading::Read(report)) = &a.pmi else {
        panic!("changed PMI not read")
    };
    assert_eq!(
        report.title_material().unwrap().designation,
        "Stainless steel 304"
    );
    assert_eq!(reader.attach(&changed, &part), a.pmi);
    assert_eq!(reader.reads(), 2);
}

#[test]
fn empty_and_refused_pmi_changes_are_read_once_and_replace_old_reports() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step");
    let (part, _) = model::analyse(&path).unwrap();
    let mut reader = draftwright::pmi::Reader::default();
    let invalid = b"ISO-10303-21; #1=GEOMETRIC_TOLERANCE(";
    let refused = reader.attach(invalid, &part);
    assert!(matches!(
        refused,
        Some(draftwright::pmi::Reading::Refused(_))
    ));
    assert_eq!(reader.attach(invalid, &part), refused);
    assert_eq!(reader.reads(), 1);
    assert_eq!(reader.attach(b"", &part), None);
    assert_eq!(reader.attach(b"", &part), None);
    assert_eq!(reader.reads(), 2);
    assert_eq!(reader.attach(invalid, &part), refused);
    assert_eq!(reader.reads(), 3);
}
