//! Slot and boss dimensions (`src/dims/slots.rs`, `src/dims/bosses.rs`) on the sampled sheets'
//! parts, with the features draftwright's own recogniser found for them
//! (`tests/fixtures/recognition/sheets/`), and on corpus parts with features no sampled sheet has
//! (`tests/fixtures/recognition/dims/`: nist_ftc_07's square bosses, mfcadpp blind slots), checked
//! against the labels draftwright prints for them: what each states, in which view, and that it
//! is drawn. The item-by-item comparison with draftwright's sheets is `tests/dimensions.rs`.

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

/// A corpus part (*file*, under the corpus) with Python quiddity 0.3.9's recognition for it
/// (`tests/fixtures/recognition/dims/<name>.json.gz`, written by `tools/recognise.py` with
/// draftwright's own environment): parts whose features no sampled sheet has.
fn analysed_file(corpus: &Path, name: &str, file: &str) -> Analysis {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/recognition/dims/{name}.json.gz"));
    let mut json = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_string(&mut json)
        .unwrap();
    let (part, mut a) = model::analyse(&corpus.join(file)).unwrap();
    let (r, features) = draftwright::bridge::read(&part, &a.binding.source_sha256, &json).unwrap();
    a.recognition = Some(r);
    a.features = features;
    a
}

/// The plan's items that state a measurement of a feature of *kind*, as (parameter, view,
/// label), sorted.
fn stating(plan: &Plan, a: &Analysis, kind: &str) -> Vec<(String, String, String)> {
    let ids = of_kind(a, kind);
    let mut out: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| {
            d.measurements
                .iter()
                .any(|m| ids.contains(&m.feature.as_str()))
        })
        .map(|d| {
            let (_, rest) = d.id.split_once('/').unwrap();
            let (parameter, view) = rest.split_once('@').unwrap();
            (parameter.to_string(), view.to_string(), d.label.clone())
        })
        .collect();
    out.sort();
    out
}

#[test]
fn ctc01_hex_boss_states_its_size_across_flats_and_its_height() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed(&corpus, "ctc01");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // As on draftwright's sheet: HEX 100 A/F in the plan, its height 50 in the front. The port
    // also locates it (400, 225), repeating the hole grid it stands in the middle of
    // (known_dimensions.json, ARCH0009-6).
    assert_eq!(
        stating(&plan, &a, "polygonal_boss"),
        strs(&[
            ("boss_height.length", "front", "50"),
            ("location.length.x", "plan", "400"),
            ("location.length.y", "plan", "225"),
            ("polygon_across_flats.length", "plan", "HEX 100 A/F"),
        ])
    );
    let across = plan
        .dimensions
        .iter()
        .find(|d| d.id.contains("/polygon_across_flats.length@"))
        .unwrap();
    assert_eq!(across.kind, "callout");
    assert_eq!(across.terms, ["HEX", "100", "A/F"]);
    assert!((across.value - 100.0).abs() < 1e-6);
    for d in plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("polygonal_boss"))
    {
        assert!(d.placed.is_some(), "{} is not drawn", d.id);
    }
    assert_eq!(
        not_ported(&plan, &of_kind(&a, "polygonal_boss")),
        Vec::<String>::new()
    );
    // Each is editable by its ID.
    let mut spec = Spec::default();
    spec.dimensions.insert(
        across.id.clone(),
        serde_json::from_str(r#"{"omit": true}"#).unwrap(),
    );
    let again = model::plan(&a, &spec).unwrap();
    assert!(again.dimensions.iter().all(|d| d.id != across.id));
    assert!(again.omitted.iter().any(|o| o.id == across.id));
    assert!(
        again
            .dimensions
            .iter()
            .any(|d| d.id.contains("polygonal_boss:1/boss_height"))
    );
}

#[test]
fn ftc07_square_bosses_are_counted_across_flats_with_their_heights() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let a = analysed_file(&corpus, "ftc07", "nist/nist_ftc_07_asme1_rd.stp");
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // draftwright's sheet (captured 2026-10-08 at origin/main 586628b) prints `4-SIDED 25.4 A/F`
    // four times in the front and `2.5` four times in the side; the port counts the four
    // identical bosses once, as it does cylindrical ones.
    let mut want = vec![(
        "polygon_across_flats.length",
        "front",
        "4× 4-SIDED 25.4 A/F",
    )];
    want.extend([("boss_height.length", "side", "2.5"); 4]);
    let got: Vec<_> = stating(&plan, &a, "polygonal_boss")
        .into_iter()
        .filter(|(p, ..)| !p.starts_with("location"))
        .collect();
    let mut want = strs(&want);
    want.sort();
    assert_eq!(got, want);
    let across = plan
        .dimensions
        .iter()
        .find(|d| d.id.contains("/polygon_across_flats.length@"))
        .unwrap();
    assert_eq!(across.measurements.len(), 4);
    assert!(across.placed.is_some());
}

#[test]
fn blind_slots_are_called_out_as_draftwright_reads_their_sections() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    // draftwright's labels and views for the same parts (captured 2026-10-08 at origin/main
    // 586628b with tools/capture_sheet_facts.py): the section recesses its
    // `section_recess_fields` reads as rectangular or round-bottom blind slots.
    type Case<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str, &'a str)]);
    let cases: [Case; 3] = [
        (
            "m10047",
            "mfcadpp/10047.step",
            &[
                (
                    "round_bottom_blind_slot_flat_width.length",
                    "plan",
                    "ROUND-BOTTOM OPEN SLOT 1 BOTTOM FLAT × R0.5 × 2.4 LONG",
                ),
                (
                    "round_bottom_blind_slot_flat_width.length",
                    "side",
                    "ROUND-BOTTOM OPEN SLOT 1 BOTTOM FLAT × R4.4 × 4.7 LONG",
                ),
            ],
        ),
        (
            "m10245",
            "mfcadpp/10245.step",
            &[(
                "rectangular_blind_slot_width.length",
                "front",
                "OPEN SLOT 3.3 × 9.2 × 2.0 DEEP",
            )],
        ),
        (
            "m11512",
            "mfcadpp_regressions/11512.step",
            &[(
                "rectangular_blind_slot_width.length",
                "plan",
                "OPEN SLOT 2.5 × 11.7 × 6.7 DEEP",
            )],
        ),
    ];
    for (name, file, want) in cases {
        let a = analysed_file(&corpus, name, file);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        assert_eq!(stating(&plan, &a, "section_recess"), strs(want), "{name}");
        // The other section recesses (pockets, channels, passages) stay reported, one each.
        let recesses = of_kind(&a, "section_recess");
        let reported = not_ported(&plan, &recesses);
        assert_eq!(reported.len(), recesses.len() - want.len(), "{name}");
        for d in plan
            .dimensions
            .iter()
            .filter(|d| d.id.contains("blind_slot"))
        {
            assert!(d.placed.is_some(), "{name}: {} is not drawn", d.id);
            assert_eq!(d.measurements.len(), 3, "{name}: {}", d.id);
            assert!(
                !reported
                    .iter()
                    .any(|p| p.ends_with(&d.measurements[0].feature))
            );
        }
    }
}

/// m10127's one boss, moved along its axis and resized in quiddity's frame so that it spans
/// *span* (world coordinates along its axis, low first).
fn boss_spanning(a: &mut Analysis, span: [f64; 2]) {
    let frame = a.recognition.as_ref().unwrap().frame;
    let b = a.features.iter_mut().find(|f| f.kind == "boss").unwrap();
    let local: Vec<f64> = serde_json::from_value(b.parameters["axis"].clone()).unwrap();
    let mut top: Vec<f64> = serde_json::from_value(b.parameters["location"].clone()).unwrap();
    let dir: Vec<f64> = (0..3)
        .map(|k| (0..3).map(|j| local[j] * frame[j + 1][k]).sum())
        .collect();
    let k = (0..3).find(|&k| dir[k].abs() > 0.5).unwrap();
    let world_top: f64 = frame[0][k] + (0..3).map(|j| top[j] * frame[j + 1][k]).sum::<f64>();
    let target = if dir[k] > 0.0 { span[1] } else { span[0] };
    let t = (target - world_top) / dir[k];
    for j in 0..3 {
        top[j] += t * local[j];
    }
    b.parameters.insert("location".into(), top.into());
    b.parameters
        .insert("height".into(), (span[1] - span[0]).into());
}

#[test]
fn a_boss_height_between_the_parts_own_faces_is_the_overall_extents() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut a = analysed(&corpus, "m10127");
    // m10127's boss stands proud (5.7 along y on a part 32.3 deep): it keeps its height
    // (ISSUE-1154-a's control).
    let proud = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(
        items(&proud, "boss_height"),
        strs(&[("boss_height.length", "side", "5.7")])
    );
    // Flush with both faces of the part, its height is the overall depth's: one fact, one owner.
    let [lo, hi] = a.envelope;
    boss_spanning(&mut a, [lo[1], hi[1]]);
    let plan = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(items(&plan, "boss_height"), vec![]);
    let depth = plan
        .dimensions
        .iter()
        .find(|d| d.id.starts_with("d:envelope/depth.length@"))
        .unwrap();
    let gone = plan
        .omitted
        .iter()
        .find(|o| o.id.contains("/boss_height.length@"))
        .expect("the boss height is omitted");
    assert!(
        gone.basis.starts_with(&format!("conveyed by {}", depth.id)),
        "{}",
        gone.basis
    );
    // An owner the spec omits takes nothing: the boss height is stated again.
    let mut spec = Spec::default();
    spec.dimensions.insert(
        depth.id.clone(),
        serde_json::from_str(r#"{"omit": true}"#).unwrap(),
    );
    let kept = model::plan(&a, &spec).unwrap();
    assert_eq!(items(&kept, "boss_height").len(), 1);
    assert!(kept.omitted.iter().all(|o| !o.id.contains("boss_height")));
    // 0.5 mm short of a face is another plane: both are stated.
    boss_spanning(&mut a, [lo[1] + 0.5, hi[1]]);
    let short = model::plan(&a, &Spec::default()).unwrap();
    assert_eq!(items(&short, "boss_height").len(), 1);
}

#[test]
fn a_boss_with_no_height_is_reported_not_silently_left_short() {
    let Some(corpus) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut a = analysed(&corpus, "m10127");
    let b = a.features.iter_mut().find(|f| f.kind == "boss").unwrap();
    b.parameters
        .insert("height".into(), serde_json::Value::Null);
    let id = b.id.clone();
    let plan = model::plan(&a, &Spec::default()).unwrap();
    // Its diameter is still called out; its height is not, and a warning says so.
    assert_eq!(
        items(&plan, "boss"),
        strs(&[("boss.diameter", "front", "ø8.4")])
    );
    let p = plan
        .problems
        .iter()
        .find(|p| p.code == "feature_not_dimensioned" && p.subjects == [id.clone()])
        .expect("a problem names the boss");
    assert_eq!(p.severity, "warning");
    assert!(p.message.contains("height"), "{}", p.message);
}
