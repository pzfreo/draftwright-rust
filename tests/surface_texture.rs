//! AP242 surface textures (ISO 1302) on the drawing (draftwright-rust #18): the part's own
//! texture is its default finish, drawn as the ISO 1302 symbol beside the title block with its
//! value; a face's texture is an ISO 1302 symbol on a leader from the face's edge line in the
//! view that sees the face edge-on, and neither is reported as read but not projected.
//!
//! The fixtures are hand-written copies of a plate (`tests/fixtures/surface_texture/README.md`).

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis, Plan, Spec};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The fixture *stem*'s part and Analysis, read from a scratch file of its own.
fn analysed(stem: &str) -> (Part, Analysis) {
    let gz = root().join(format!("tests/fixtures/surface_texture/{stem}.step.gz"));
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(gz).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("surface_texture")
        .join(format!("{}-{call}-{stem}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{stem}.step"));
    std::fs::write(&path, bytes).unwrap();
    let out = model::analyse_native(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

fn svg(part: &Part, a: &Analysis, plan: &Plan) -> String {
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    model::render(part, a, plan, &font).unwrap().svg()
}

/// No problem says a surface texture was read but not projected, or not lowered.
fn no_texture_lost(plan: &Plan) {
    for p in &plan.problems {
        assert!(
            !(p.refs.iter().any(|r| r.starts_with("surface_texture:"))
                || p.message
                    .contains("not projected to a draftwright record (surface texture")),
            "{p:?}"
        );
        assert!(
            !(p.code == "pmi_not_lowered" && p.message.contains("surface")),
            "{p:?}"
        );
    }
}

#[test]
fn a_part_wide_texture_is_the_general_symbol_by_the_title_block() {
    let (part, a) = analysed("plate_part_ra1_6");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let finish = plan
        .requirements
        .iter()
        .find(|r| r.kind == "default_surface_finish")
        .expect("the part's default finish");
    assert_eq!(finish.text, "Ra 1.6");
    assert_eq!(finish.carrier, "sheet");
    let placed = finish
        .placed
        .as_ref()
        .expect("its symbol beside the title block");
    // Beside the title block: standing in the band above its drawn grid, at its left edge.
    let block = plan.sheet.title_block;
    assert!((placed.glyph[0] - block[0]).abs() < 1e-9, "{placed:?}");
    assert!(
        placed.glyph[1] > block[1] && placed.glyph[1] < block[3],
        "{placed:?} {block:?}"
    );
    // ISO 1302's basic symbol (any process allowed): its two legs and bar.
    assert_eq!(placed.strokes.len(), 3);
    assert!(placed.arcs.is_empty());
    no_texture_lost(&plan);
    assert!(svg(&part, &a, &plan).contains("Ra 1.6"));
}

#[test]
fn a_face_texture_is_a_symbol_on_its_edge_line() {
    let (part, a) = analysed("plate_face_ra0_8");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // A face's texture is no default finish.
    assert!(
        plan.requirements
            .iter()
            .all(|r| r.kind != "default_surface_finish")
    );
    let finish = plan
        .gdt
        .iter()
        .find(|g| g.kind == "surface_finish")
        .expect("the face's surface texture symbol");
    assert_eq!(finish.cells, ["Ra 0.8"]);
    assert_eq!(finish.state, draftwright::items::State::Shown);
    // The plate's bottom face (normal -z) is edge-on in the front view, its edge line the
    // view's lower edge: the leader's arrowhead is on it, and the symbol stands below.
    assert_eq!(
        (finish.view.as_str(), finish.side.as_str()),
        ("v:front", "below")
    );
    let view = plan.views.iter().find(|v| v.id == "v:front").unwrap();
    let [lo, hi] = a.envelope;
    let bottom = view.at[1] - 0.5 * view.scale * (hi[2] - lo[2]);
    let half = 0.5 * view.scale * (hi[0] - lo[0]);
    let placed = finish.placed.as_ref().unwrap();
    assert!(
        (placed.tip[1] - bottom).abs() < 1e-6,
        "{:?} {bottom}",
        placed.tip
    );
    assert!((placed.tip[0] - view.at[0]).abs() <= half + 1e-6);
    assert_eq!(placed.arrows.len(), 1);
    assert_eq!(placed.arrows[0][0], placed.tip);
    assert!(placed.label_box[3] < bottom, "{placed:?}");
    assert_eq!(placed.runs.len(), 1);
    assert_eq!(placed.runs[0].text, "Ra 0.8");
    no_texture_lost(&plan);
    // Its record is read, with the face it applies to.
    let Some(draftwright::pmi::Reading::Read(report)) = &a.pmi else {
        panic!("PMI read");
    };
    let record = report
        .records
        .iter()
        .find(|r| r.kind == "surface_finish")
        .unwrap();
    assert_eq!(record.label, "Ra 0.8 µm");
    assert_eq!(record.faces.len(), 1);
    assert!(svg(&part, &a, &plan).contains("Ra 0.8"));
}

#[test]
fn material_removal_is_drawn_on_the_symbol() {
    // `material removal required` closes the short leg with a bar; `no material removal`
    // inscribes a circle. Read from the face fixture with its condition rewritten.
    let gz = root().join("tests/fixtures/surface_texture/plate_face_ra0_8.step.gz");
    let mut bytes = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(gz).unwrap())
        .read_to_string(&mut bytes)
        .unwrap();
    for (term, strokes, arcs) in [
        ("any process allowed", 3, 0),
        ("material removal required", 4, 0),
        ("no material removal", 3, 1),
    ] {
        let text = bytes.replace(
            "'material removal condition','any process allowed'",
            &format!("'material removal condition','{term}'"),
        );
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("surface_texture")
            .join(format!("{}-removal", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}.step", term.replace(' ', "_")));
        std::fs::write(&path, text).unwrap();
        let (_, a) = model::analyse_native(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let g = plan
            .gdt
            .iter()
            .find(|g| g.kind == "surface_finish")
            .unwrap();
        let p = g.placed.as_ref().unwrap();
        assert_eq!((p.strokes.len(), p.arcs.len()), (strokes, arcs), "{term}");
    }
}

/// Source-scoped face finishes are checked against the Analysis parameter, even when the
/// annotation retains its original semantic source ID after its text is corrupted.
#[test]
fn a_face_finish_text_cannot_disagree_with_its_source() {
    let (part, a) = analysed("plate_face_ra0_8");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let finish = plan
        .gdt
        .iter()
        .find(|g| g.kind == "surface_finish")
        .unwrap();
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    let mut drawing = model::render(&part, &a, &plan, &font).unwrap();
    let runs = drawing
        .annotations
        .iter()
        .find(|n| n.name == finish.id)
        .unwrap()
        .runs
        .clone();
    assert_eq!(runs.len(), 1);
    assert_eq!(drawing.text[runs[0]].text, "Ra 0.8");
    let mismatch = |d: &draftwright::output::Drawing| {
        draftwright::lint::lint(&a, d)
            .into_iter()
            .filter(|i| i.code == "pmi_source_text_mismatch")
            .collect::<Vec<_>>()
    };
    assert!(mismatch(&drawing).is_empty());
    drawing.text[runs[0]].text = "Ra 9.9".into();
    let found = mismatch(&drawing);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].sources.as_slice(),
        std::slice::from_ref(&finish.source)
    );
    assert_eq!(found[0].annotation.as_deref(), Some(finish.id.as_str()));
}
