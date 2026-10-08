//! Slot and boss dimensions (`src/dims/slots.rs`, `src/dims/bosses.rs`) on the sampled sheets'
//! parts, with the features draftwright's own recogniser found for them
//! (`tests/fixtures/recognition/sheets/`): what each states, in which view, and that it is drawn.
//! The item-by-item comparison with draftwright's sheets is `tests/dimensions.rs`.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::io::Read as _;
use std::path::Path;

use draftwright::dims::PlannedDimension;
use draftwright::model::{self, Analysis, Plan, Spec};

fn analysed(corpus: &Path, name: &str) -> Analysis {
    let f = facts::load(name);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/recognition/sheets/{name}.json.gz"));
    let mut json = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_string(&mut json)
        .unwrap();
    let (part, mut a) = model::analyse(&corpus.join(&f.binding.file)).unwrap();
    let (r, features) = draftwright::bridge::read(&part, &a.binding.source_sha256, &json).unwrap();
    a.recognition = Some(r);
    a.features = features;
    a
}

/// The plan's items whose parameter starts with *prefix*, as (parameter, view, label), sorted.
fn items(plan: &Plan, prefix: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<_> = plan
        .dimensions
        .iter()
        .filter_map(|d| {
            let (_, rest) = d.id.split_once('/')?;
            let (parameter, view) = rest.split_once('@')?;
            parameter
                .starts_with(prefix)
                .then(|| (parameter.to_string(), view.to_string(), d.label.clone()))
        })
        .collect();
    out.sort();
    out
}

fn of_kind<'a>(a: &'a Analysis, kind: &str) -> Vec<&'a str> {
    a.features
        .iter()
        .filter(|f| f.kind == kind)
        .map(|f| f.id.as_str())
        .collect()
}

fn not_ported(plan: &Plan, ids: &[&str]) -> Vec<String> {
    plan.problems
        .iter()
        .filter(|p| p.code == "dimensions_not_ported")
        .filter(|p| p.subjects.iter().any(|s| ids.contains(&s.as_str())))
        .map(|p| p.id.clone())
        .collect()
}

fn drawn(plan: &Plan, d: &PlannedDimension) {
    assert!(
        d.placed.is_some(),
        "{} is not drawn: {:#?}",
        d.id,
        plan.problems
            .iter()
            .filter(|p| p.subjects.contains(&d.id))
            .collect::<Vec<_>>()
    );
}

fn strs(v: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    v.iter()
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
        .collect()
}

#[test]
fn cgb245_slots_state_width_length_end_radius_and_location() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "cgb245");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // Three through slots in the plan: two obrounds (6 × 15.0, ends R3) and one straight slot
    // (7.0 × 16.9, corner radius 2: no end radius), each located from the box along x.
    assert_eq!(
        items(&plan, "slot_"),
        strs(&[
            ("slot_end_radius.radius", "plan", "2× R3"),
            ("slot_end_radius.radius", "plan", "2× R3"),
            ("slot_length.length", "plan", "15.0"),
            ("slot_length.length", "plan", "15.0"),
            ("slot_length.length", "plan", "16.9"),
            ("slot_width.length", "plan", "6"),
            ("slot_width.length", "plan", "6"),
            ("slot_width.length", "plan", "7.0"),
        ])
    );
    assert_eq!(
        items(&plan, "location_slot"),
        strs(&[
            ("location_slot.length", "plan", "181.6"),
            ("location_slot.length", "plan", "286.0"),
            ("location_slot.length", "plan", "79.1"),
        ])
    );
    for d in plan.dimensions.iter().filter(|d| d.id.contains("slot")) {
        drawn(&plan, d);
        assert!(d.default && !d.basis.is_empty(), "{d:?}");
    }
    assert_eq!(
        not_ported(&plan, &of_kind(&a, "slot")),
        Vec::<String>::new()
    );
}

#[test]
fn ctc01_slot_locations_are_exact() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "ctc01");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // quiddity's ends are rounded to 0.01 in its own frame; the faces' box gives 55, not 54.998.
    let mut values: Vec<f64> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("/location_slot.length@"))
        .map(|d| d.value)
        .collect();
    values.sort_by(f64::total_cmp);
    assert_eq!(values.len(), 2);
    assert!(
        (values[0] - 55.0).abs() < 1e-6 && (values[1] - 635.0).abs() < 1e-6,
        "{values:?}"
    );
    assert_eq!(
        items(&plan, "slot_end_radius"),
        strs(&[("slot_end_radius.radius", "plan", "2× R20")])
    );
}

#[test]
fn ftc06_bosses_are_counted_called_out_and_their_heights_stated() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "ftc06");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        items(&plan, "boss"),
        strs(&[
            ("boss.diameter", "front", "2× ø22.9"),
            ("boss.diameter", "front", "2× ø31.8"),
            ("boss.diameter", "plan", "4× ø44.5"),
            ("boss_height.length", "front", "38.1"),
            ("boss_height.length", "front", "38.1"),
            ("boss_height.length", "front", "38.1"),
            ("boss_height.length", "front", "38.1"),
            ("boss_height.length", "side", "12.1"),
            ("boss_height.length", "side", "12.1"),
            ("boss_height.length", "side", "16.3"),
            ("boss_height.length", "side", "16.3"),
        ])
    );
    let bosses = of_kind(&a, "boss");
    for d in plan.dimensions.iter().filter(|d| {
        d.measurements
            .iter()
            .any(|m| bosses.contains(&m.feature.as_str()))
    }) {
        drawn(&plan, d);
    }
    assert_eq!(not_ported(&plan, &bosses), Vec::<String>::new());
}

#[test]
fn a_turned_parts_coaxial_bosses_are_profile_bands_and_a_groove_floor_is_the_grooves() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "connector109");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // Five boss records: the ø122 × 6 one is groove 217's floor, so four bosses are stated, each
    // as a band of the profile in the front view, and none is located (the turning axis does).
    assert_eq!(
        items(&plan, "boss"),
        strs(&[
            ("boss.diameter", "front", "ø110"),
            ("boss.diameter", "front", "ø118"),
            ("boss.diameter", "front", "ø122"),
            ("boss.diameter", "front", "ø130.9"),
            ("boss_height.length", "front", "199.7"),
            ("boss_height.length", "front", "20"),
            ("boss_height.length", "front", "4.8"),
            ("boss_height.length", "front", "6.4"),
        ])
    );
    let bosses = of_kind(&a, "boss");
    assert!(plan.dimensions.iter().all(|d| {
        !d.id.contains("/location")
            || !d
                .measurements
                .iter()
                .any(|m| bosses.contains(&m.feature.as_str()))
    }));
    for d in plan.dimensions.iter().filter(|d| d.id.contains("/boss")) {
        assert_eq!(d.kind, "linear", "{}", d.id);
        drawn(&plan, d);
    }
}

#[test]
fn slot_and_boss_items_can_be_omitted_by_id() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "m10127");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let ids: Vec<String> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("/boss"))
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(ids.len(), 2, "{ids:?}");
    let mut spec = Spec::default();
    spec.dimensions.insert(
        ids[0].clone(),
        serde_json::from_str(r#"{"omit": true}"#).unwrap(),
    );
    let again = model::plan(&a, &spec).unwrap();
    assert!(again.dimensions.iter().all(|d| d.id != ids[0]));
    assert!(again.omitted.iter().any(|o| o.id == ids[0]));
    // Planning is deterministic.
    assert_eq!(
        serde_json::to_string(&plan).unwrap(),
        serde_json::to_string(&model::plan(&a, &Spec::default()).unwrap()).unwrap()
    );
}

#[test]
fn equal_widths_across_a_view_share_one_counted_dimension() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut a = analysed(&corpus, "cgb245");
    // Turn cgb245's slots a quarter turn in the plan (quiddity's letters swapped): their widths
    // now run across the view, and the two 6 mm obrounds span the same stretch of x.
    for f in a.features.iter_mut().filter(|f| f.kind == "slot") {
        f.parameters.insert("long_axis".into(), "y".into());
        f.parameters.insert("width_axis".into(), "z".into());
    }
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        items(&plan, "slot_width"),
        strs(&[
            ("slot_width.length", "plan", "2× 6"),
            ("slot_width.length", "plan", "7.0"),
        ])
    );
    let shared = plan.dimensions.iter().find(|d| d.label == "2× 6").unwrap();
    assert_eq!(shared.measurements.len(), 2);
}
