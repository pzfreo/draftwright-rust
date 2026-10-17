//! The title block's material from the STEP file (draftwright's
//! `builder._resolve_title_document_defaults`): the file's one usable material fills the
//! MATERIAL cell when the spec states none; a material the spec states, even blank, wins; several
//! materials, or one of a file of several parts, fill nothing. The drawn cell states its source
//! (`pmi:material:<part>:0`), so the coverage lint does not report the material as not drawn.
//!
//! draftwright's MATERIAL text for each AP242 fixture of `tests/fixtures/pmi/step/` was read from
//! its `build_drawing(..., pmi="annotate")` title block (`title_field_specs`, draftwright
//! origin/main), and is [`PYTHON`]: no entry, no material drawn. draftwright cannot draw
//! bracket_positions (its projection raises), so its entry is what its rule takes from its reading
//! (`read_material_properties`: one fact, #31503, with no reason).

#[path = "common/support.rs"]
#[macro_use]
mod support;

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::Part;
use draftwright::lint;
use draftwright::model::{self, Analysis, Plan, Spec};
use draftwright::output::Drawing;

/// draftwright's MATERIAL cell, by fixture. assembly_plate_pin states two materials (one per part):
/// ambiguous, none drawn; the NIST AP242 files state none, and so do haecceity's written
/// hw_thread_removed and hw_thread_replaced (no material in the file).
const PYTHON: [(&str, &str); 11] = [
    ("bolt_thread_knurl", "Steel C45"),
    ("bracket_positions", "Aluminium 6061-T6"),
    ("hw_every_kind", "Aluminium 6082-T6"),
    ("hw_spool_fits_intent", "Aluminium 6082-T6"),
    ("hw_string_post_tapped_intent", "Brass CW614N"),
    ("hw_thumbwheel_notes_add", "Stainless steel 303"),
    ("hw_thumbwheel_thread_knurl_intent", "Stainless steel 303"),
    ("nist_ctc_01_merge", "Aluminium 6082-T6"),
    ("spool_fits", "Aluminium 6082-T6"),
    ("string_post_tapped", "Brass CW614N"),
    ("thumbwheel_thread_knurl", "Stainless steel 303"),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The AP242 fixtures, by stem: (file name, bytes).
fn fixtures(only: &str) -> BTreeMap<String, (String, Vec<u8>)> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(root().join("tests/fixtures/pmi/step")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "gz") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let file = name.strip_suffix(".gz").unwrap().to_string();
        if file != only && file.split('.').next() != Some(only) {
            continue;
        }
        let stem = file.split('.').next().unwrap().to_string();
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(std::fs::File::open(&p).unwrap())
            .read_to_end(&mut bytes)
            .unwrap();
        out.insert(stem, (file, bytes));
    }
    out
}

/// The part and its Analysis (no recognition: the title block does not need it), read from a
/// scratch file of its own, as the tests run in parallel (in one process under cargo test, in
/// one process each under nextest, so the directory names the process too).
fn analysed(file: &str, bytes: &[u8]) -> (Part, Analysis) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("title_block_material")
        .join(format!("{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file);
    std::fs::write(&path, bytes).unwrap();
    let out = model::analyse(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

fn material(plan: &Plan) -> Option<String> {
    plan.sheet
        .title_cells
        .iter()
        .find(|c| c.field == "material")
        .unwrap()
        .value
        .clone()
}

fn spec_with_material(m: &str) -> Spec {
    let mut spec = Spec::default();
    spec.sheet.title.insert("material".into(), m.into());
    spec
}

fn render(part: &Part, a: &Analysis, plan: &Plan) -> Drawing {
    let font =
        draftwright::text::Font::open(&root().join("fonts").join(draftwright::dims::FONT)).unwrap();
    model::render(part, a, plan, &font).unwrap()
}

/// Whether the lint reports the reading's material source as not drawn.
fn material_unreported(a: &Analysis, d: &Drawing) -> bool {
    !lint::lint(a, d)
        .iter()
        .any(|i| i.code == "pmi_not_extracted" && i.sources.iter().any(|s| s == "material:0:0"))
}

pmi_cases!(the_title_block_states_the_files_material_as_draftwright_does);
fn the_title_block_states_the_files_material_as_draftwright_does(only: &str) {
    let python: BTreeMap<&str, &str> = PYTHON.into_iter().collect();
    for (stem, (file, bytes)) in fixtures(only)
        .into_iter()
        .filter(|(_, (file, _))| file == only)
    {
        let (_, a) = analysed(&file, &bytes);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert_eq!(
            material(&plan).as_deref(),
            python.get(stem.as_str()).copied(),
            "{stem}: the MATERIAL cell"
        );
    }
}

#[test]
fn a_material_the_spec_states_wins_even_blank() {
    let (file, bytes) = fixtures("bolt_thread_knurl")
        .remove("bolt_thread_knurl")
        .unwrap();
    let (part, a) = analysed(&file, &bytes);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let d = render(&part, &a, &plan);
    let stated = d
        .annotations
        .iter()
        .find(|n| n.name == "pmi:material:0:0")
        .expect("the material cell states its source");
    assert_eq!(d.text[stated.runs[0]].text, "Steel C45");
    assert!(material_unreported(&a, &d));

    // The spec's material is drawn instead; it is not the file's, so the file's is not drawn.
    let plan = model::plan(&a, &spec_with_material("Brass CW614N")).unwrap();
    assert_eq!(material(&plan).as_deref(), Some("Brass CW614N"));
    let d = render(&part, &a, &plan);
    assert!(!d.annotations.iter().any(|n| n.name == "pmi:material:0:0"));
    assert!(!material_unreported(&a, &d));

    // A blank one clears the cell (draftwright's explicit empty value).
    let plan = model::plan(&a, &spec_with_material("")).unwrap();
    assert_eq!(material(&plan), None);

    // The same material, stated by the spec, still states the file's.
    let plan = model::plan(&a, &spec_with_material("Steel C45")).unwrap();
    assert!(material_unreported(&a, &render(&part, &a, &plan)));
}

#[test]
fn several_materials_fill_nothing() {
    let (file, bytes) = fixtures("assembly_plate_pin")
        .remove("assembly_plate_pin")
        .unwrap();
    let (_, a) = analysed(&file, &bytes);
    let Some(draftwright::pmi::Reading::Read(r)) = &a.pmi else {
        panic!("assembly_plate_pin's PMI is read")
    };
    assert_eq!(r.materials.len(), 2);
    assert!(r.title_material().is_none());
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(material(&plan), None);
}

#[test]
fn a_spec_material_contradiction_warns_with_both_values_and_keeps_the_spec() {
    let (file, bytes) = fixtures("bolt_thread_knurl")
        .remove("bolt_thread_knurl")
        .unwrap();
    let (_, a) = analysed(&file, &bytes);
    for value in ["Brass CW614N", "", "  "] {
        let plan = model::plan(&a, &spec_with_material(value)).unwrap();
        let p = plan
            .problems
            .iter()
            .find(|p| p.code == "step_material_disagreement")
            .expect("the Spec cannot silently contradict the STEP material");
        assert_eq!(p.severity, "warning");
        assert_eq!(p.subjects, ["sheet"]);
        assert!(p.refs.iter().any(|r| r == "sheet/title/material"));
        assert!(
            p.message.contains("Steel C45") && p.message.contains(value),
            "{p:?}"
        );
        assert_eq!(
            material(&plan).as_deref(),
            (!value.trim().is_empty()).then_some(value.trim())
        );
    }
    for spec in [
        Spec::default(),
        spec_with_material("Steel C45"),
        spec_with_material(" Steel C45 "),
    ] {
        let plan = model::plan(&a, &spec).unwrap();
        assert!(
            !plan
                .problems
                .iter()
                .any(|p| p.code == "step_material_disagreement")
        );
    }
    let (file, bytes) = fixtures("assembly_plate_pin")
        .remove("assembly_plate_pin")
        .unwrap();
    let (_, a) = analysed(&file, &bytes);
    let plan = model::plan(&a, &spec_with_material("Steel C45")).unwrap();
    assert!(
        !plan
            .problems
            .iter()
            .any(|p| p.code == "step_material_disagreement"),
        "several parts do not prove one contradictory material"
    );
}
