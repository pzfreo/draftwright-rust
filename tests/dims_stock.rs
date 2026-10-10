//! Whole-part polygonal stock (`src/dims/stock.rs`, ARCH0015-4): its size across flats and its
//! length in place of the overall extents, on fixture hexagonal bars
//! (`tests/fixtures/recognition/polygonal_stock/`, built or copied by `make.py` there; no corpus
//! part is stock), checked against draftwright's sheets of the same parts (`<name>.facts.json.gz`):
//! what each states, in which view, and that it is drawn. The bars lie along z, x and y, and one is
//! turned 17° about its axis and another moved off the origin, so the plan is checked under rigid
//! motion too.

#[path = "common/facts.rs"]
mod facts;

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::model::{self, Analysis, DimensionSpec, Plan, Spec};

const PARTS: [&str; 5] = ["plain", "x", "y", "roll17", "translated"];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recognition/polygonal_stock")
}

/// A fixture bar with its features, its recognition JSON edited by *edit*.
fn fixture(name: &str, edit: impl Fn(&mut serde_json::Value)) -> Analysis {
    let mut text = String::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(dir().join(format!("{name}.json.gz"))).unwrap(),
    )
    .read_to_string(&mut text)
    .unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    edit(&mut json);
    let (part, mut a) = model::analyse(&dir().join(format!("{name}.step"))).unwrap();
    model::recognise(&part, &mut a, &json.to_string()).unwrap();
    a
}

/// The stock record in a recognition document.
fn record(json: &mut serde_json::Value) -> &mut serde_json::Value {
    json["quiddity"]["features"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["record_type"] == "PolygonalStock")
        .map(|f| &mut f["record"])
        .unwrap()
}

/// The plan's dimensions as (parameter, view, label, value), sorted.
fn items(plan: &Plan) -> Vec<(String, String, String, f64)> {
    let mut out: Vec<_> = plan
        .dimensions
        .iter()
        .map(|d| {
            let (_, parameter) = d.id.split_once('/').unwrap();
            (
                parameter.to_string(),
                d.view.trim_start_matches("v:").to_string(),
                d.label.clone(),
                d.value,
            )
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

/// draftwright's sheet of *name*: each measuring annotation as (parameter, view, label, value).
fn python(name: &str) -> Vec<(String, String, String, f64)> {
    let f = facts::load_file(&dir().join(format!("{name}.facts.json.gz")));
    let mut out: Vec<_> = f
        .annotations
        .iter()
        .flat_map(|a| {
            a.measurements.iter().map(|m| {
                (
                    m.parameter.clone(),
                    a.view.clone().unwrap(),
                    a.text.join(" "),
                    m.value.unwrap(),
                )
            })
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

fn stock_id(a: &Analysis) -> String {
    let ids: Vec<_> = a
        .features
        .iter()
        .filter(|f| f.kind == "polygonal_stock")
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(ids.len(), 1, "{ids:?}");
    ids[0].clone()
}

/// draftwright's `HEX <A/F> A/F` callout in the view down the bar and its length in the view that
/// shows its profile, and nothing else: no overall extent (#1082: the part has no envelope
/// feature), the same parameters, views, labels and values as its sheets, each drawn.
#[test]
fn hexagonal_stock_states_its_size_across_flats_and_length_as_draftwright_does() {
    for name in PARTS {
        let a = fixture(name, |_| {});
        let id = stock_id(&a);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let ours = items(&plan);
        let theirs = python(name);
        assert_eq!(
            ours.len(),
            theirs.len(),
            "{name}: {ours:?} against {theirs:?}"
        );
        for (o, t) in ours.iter().zip(&theirs) {
            assert_eq!((&o.0, &o.1, &o.2), (&t.0, &t.1, &t.2), "{name}");
            assert!((o.3 - t.3).abs() < 1e-3, "{name}: {o:?} against {t:?}");
        }
        for d in &plan.dimensions {
            assert!(
                d.measurements.iter().all(|m| m.feature == id),
                "{name}: {d:?}"
            );
            assert!(d.default && d.placed.is_some(), "{name}: {d:?}");
            assert!(d.basis.contains("in place of the overall"), "{d:?}");
        }
        assert!(plan.omitted.is_empty(), "{name}: {:?}", plan.omitted);
        let unstated: Vec<_> = plan
            .problems
            .iter()
            .filter(|p| p.subjects.contains(&id))
            .collect();
        assert!(unstated.is_empty(), "{name}: {unstated:#?}");
    }
}

/// Rigid motion leaves the plan's statements where they were: the bar turned 17° about its axis
/// states what the plain bar does (draftwright's
/// `test_rotation_preserves_stock_across_flats_instead_of_using_bbox_width`, where the box's width
/// would change), and the bars along x and y state the same sizes in the views that face them.
#[test]
fn rigid_motion_moves_the_views_but_not_the_statements() {
    let plain = items(&model::plan(&fixture("plain", |_| {}), &Spec::default()).unwrap());
    let rolled = items(&model::plan(&fixture("roll17", |_| {}), &Spec::default()).unwrap());
    assert_eq!(plain, rolled);
    let labels = |v: &[(String, String, String, f64)]| {
        v.iter()
            .map(|(p, _, l, _)| (p.clone(), l.clone()))
            .collect::<Vec<_>>()
    };
    for name in ["x", "y"] {
        let turned = items(&model::plan(&fixture(name, |_| {}), &Spec::default()).unwrap());
        assert_eq!(labels(&turned), labels(&plain), "{name}");
    }
    let views = |name: &str| {
        items(&model::plan(&fixture(name, |_| {}), &Spec::default()).unwrap())
            .into_iter()
            .map(|(p, v, ..)| (p, v))
            .collect::<Vec<_>>()
    };
    let s = |p: &str, v: &str| (p.to_string(), v.to_string());
    assert_eq!(
        views("x"),
        [
            s("polygon_across_flats.length", "side"),
            s("stock_length.length", "front")
        ]
    );
    assert_eq!(
        views("y"),
        [
            s("polygon_across_flats.length", "front"),
            s("stock_length.length", "side")
        ]
    );
}

/// Honest failure: a stock record that breaks quiddity's contract (five sides; a flat moved off its
/// side) is reported on the stock, and the overall extents stay, so the part's size
/// is still stated.
#[test]
fn a_stock_record_that_cannot_be_read_is_reported_and_the_extents_stay() {
    type Edit = fn(&mut serde_json::Value);
    let edits: [(&str, Edit); 2] = [
        ("six sides", |j| record(j)["side_count"] = 5.into()),
        ("physical supports", |j| {
            record(j)["flat_centres"][0] = serde_json::json!([0.0, -20.0, -15.0])
        }),
    ];
    for (why, edit) in edits {
        let a = fixture("plain", edit);
        let id = stock_id(&a);
        let plan = model::plan(&a, &Spec::default()).unwrap();
        let reported: Vec<_> = plan
            .problems
            .iter()
            .filter(|p| p.subjects.contains(&id))
            .collect();
        assert_eq!(reported.len(), 1, "{why}: {reported:#?}");
        assert_eq!(reported[0].code, "feature_not_dimensioned");
        assert!(reported[0].message.contains(why), "{:?}", reported[0]);
        assert_eq!(
            items(&plan)
                .into_iter()
                .map(|(p, v, l, _)| (p, v, l))
                .collect::<Vec<_>>(),
            [
                ("depth.length".into(), "side".into(), "34.6".into()),
                ("height.length".into(), "front".into(), "30".into()),
                ("width.length".into(), "front".into(), "40".into()),
            ],
            "{why}"
        );
    }
}

/// The spec may omit the stock's length like any other dimension; the overall height does not come
/// back in its place.
#[test]
fn omitting_the_stock_length_does_not_restore_the_overall_height() {
    let a = fixture("plain", |_| {});
    let id = format!("d:{}/stock_length.length", stock_id(&a));
    let mut spec = Spec::default();
    spec.dimensions.insert(
        id.clone(),
        DimensionSpec {
            omit: true,
            ..Default::default()
        },
    );
    let plan = model::plan(&a, &spec).unwrap();
    assert_eq!(
        plan.omitted
            .iter()
            .map(|o| o.id.as_str())
            .collect::<Vec<_>>(),
        [id.as_str()]
    );
    assert_eq!(
        items(&plan)
            .into_iter()
            .map(|(p, ..)| p)
            .collect::<Vec<_>>(),
        ["polygon_across_flats.length"]
    );
}

/// A stock is the whole part, so quiddity gives at most one; two are both reported (as lint
/// reports them unverifiable), and the overall extents stay.
#[test]
fn two_stock_records_are_reported_and_the_extents_stay() {
    let a = fixture("plain", |j| {
        let features = j["quiddity"]["features"].as_array_mut().unwrap();
        let mut copy = features
            .iter()
            .find(|f| f["record_type"] == "PolygonalStock")
            .unwrap()
            .clone();
        copy["index"] = features.len().into();
        // On other faces, or the bridge cannot tell the two apart.
        copy["constituent_faces"] = serde_json::json!([0, 1]);
        copy["defining_faces"] = serde_json::json!([0, 1]);
        features.push(copy);
    });
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let reported: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == "feature_not_dimensioned")
        .map(|p| p.message.as_str())
        .collect();
    assert_eq!(reported.len(), 2, "{reported:#?}");
    assert!(
        reported
            .iter()
            .all(|m| m.contains("quiddity gives 2 polygonal stocks"))
    );
    assert_eq!(
        items(&plan)
            .into_iter()
            .map(|(p, ..)| p)
            .collect::<Vec<_>>(),
        ["depth.length", "height.length", "width.length"]
    );
}

/// LINT-polygonal_stock_coverage-af_length on the port's own drawing: the drawn callout and length
/// state both of each bar's requirements, so lint finds none missing (as draftwright's sheets of
/// these bars have no `polygonal_stock_` finding); omitting the length in the spec leaves it
/// missing.
#[test]
fn lint_finds_the_stock_stated() {
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    for name in PARTS {
        let a = fixture(name, |_| {});
        let (part, _) = model::analyse(&dir().join(format!("{name}.step"))).unwrap();
        let findings = |spec: &Spec| -> Vec<String> {
            let plan = model::plan(&a, spec).unwrap();
            let drawing = model::render(&part, &a, &plan, &font).unwrap();
            draftwright::lint::lint(&a, &drawing)
                .into_iter()
                .filter(|i| i.code.starts_with("polygonal_stock_"))
                .map(|i| i.code)
                .collect()
        };
        assert!(findings(&Spec::default()).is_empty(), "{name}");
        let python = facts::load_file(&dir().join(format!("{name}.facts.json.gz")));
        assert!(
            !python
                .lint
                .iter()
                .any(|f| f.code.starts_with("polygonal_stock_")),
            "{name}"
        );
        let mut spec = Spec::default();
        spec.dimensions.insert(
            format!("d:{}/stock_length.length", stock_id(&a)),
            DimensionSpec {
                omit: true,
                ..Default::default()
            },
        );
        assert_eq!(
            findings(&spec),
            ["polygonal_stock_requirement_missing"],
            "{name}"
        );
    }
}
