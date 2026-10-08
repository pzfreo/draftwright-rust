//! View planning from the part's features: a turned part's reduced view set, and section A–A
//! planned, placed and drawn (its cut face hatched, its cutting plane marked on the plan).

#[path = "common/corpus.rs"]
mod corpus;

use std::path::Path;

use draftwright::model::{self, Spec};
use draftwright::output::{Curve, Item};
use draftwright::text::Font;

fn fixture(name: &str) -> (draftwright::haecceity::Part, model::Analysis) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    model::analyse_with(
        &dir.join("parts").join(format!("{name}.step")),
        Some(&dir.join("recognition").join(format!("{name}.json"))),
    )
    .unwrap()
}

fn ids(plan: &model::Plan) -> Vec<&str> {
    plan.views.iter().map(|v| v.id.as_str()).collect()
}

/// ISSUE-1262-a, ADR2-13: a plain part turned about z is drawn as its profile (front) and end
/// (plan) views and the isometric; the side view, which repeats the front, is left out and the
/// basis says why. A spec that names the side view keeps it.
#[test]
fn a_turned_part_leaves_out_the_view_that_repeats_its_profile() {
    let (_, analysis) = fixture("turned");
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    assert_eq!(ids(&plan), ["v:front", "v:plan", "v:iso"]);
    for v in &plan.views[..2] {
        assert!(
            v.basis.contains("turned about z") && v.basis.contains("side view"),
            "{}",
            v.basis
        );
    }
    let spec: Spec =
        serde_json::from_value(serde_json::json!({ "views": { "side": {} } })).unwrap();
    let plan = model::plan(&analysis, &spec).unwrap();
    assert_eq!(ids(&plan), ["v:front", "v:plan", "v:side", "v:iso"]);
    // Without its features, the part is not known to be turned.
    let (_, bare) = model::analyse(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/turned.step"),
    )
    .unwrap();
    let plan = model::plan(&bare, &Spec::default()).unwrap();
    assert_eq!(ids(&plan).len(), 4);
}

/// A prismatic part is not reduced.
#[test]
fn a_prismatic_part_keeps_its_three_views() {
    for name in ["prismatic", "plate"] {
        let (_, analysis) = fixture(name);
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        assert_eq!(
            ids(&plan),
            ["v:front", "v:plan", "v:side", "v:iso"],
            "{name}"
        );
    }
}

#[test]
fn an_unknown_view_names_the_section_among_the_choices() {
    let (_, analysis) = fixture("plate");
    let spec: Spec =
        serde_json::from_value(serde_json::json!({ "views": { "rear": {} } })).unwrap();
    let e = model::plan(&analysis, &spec).unwrap_err();
    assert!(e.choices.contains(&"section_aa".to_string()), "{e}");
}

/// nist_ftc_06: its counterbored z-axis holes warrant section A–A (draftwright places it too,
/// tests/fixtures/sheets/ftc06), cut through the row of four at y = 50.8, right of the side view
/// in the front's row; the cut face is hatched at 45° inside the section's area, the plan
/// carries the cutting-plane line with its letters, and the view is captioned. A spec can omit
/// it, and then no room is reserved for it.
#[test]
fn section_a_a_is_planned_placed_and_drawn() {
    let Some(dir) = corpus::dir() else {
        eprintln!(
            "SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS \
             (DRAFTWRIGHT_CORPUS_REQUIRED=1 makes this a failure)"
        );
        return;
    };
    let (part, analysis) = corpus::analyse(&dir, "nist/nist_ftc_06_asme1_rd.stp", "ftc06");
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    assert_eq!(
        ids(&plan),
        ["v:front", "v:plan", "v:side", "v:iso", "v:section_aa"]
    );
    let view = |id: &str| plan.views.iter().find(|v| v.id == id).unwrap();
    let (front, side, section) = (view("v:front"), view("v:side"), view("v:section_aa"));
    let cut = section.cut.as_ref().unwrap();
    assert!((cut.origin[1] - 50.8).abs() < 1e-9, "{cut:?}");
    assert_eq!(cut.normal, [0.0, 1.0, 0.0]);
    assert_eq!(
        section.at[1], front.at[1],
        "the section shares the front's row"
    );
    // The section's block is its box and a 20 mm band each side; draftwright starts it 10 mm
    // past the side view's band, so the two bands share 10 mm of blank paper.
    assert!(
        (section.area[0] + 20.0 - (side.area[2] + 10.0)).abs() < 1e-9,
        "right of the side view's area"
    );
    for v in plan
        .views
        .iter()
        .filter(|v| v.id != "v:side" && v.id != "v:section_aa")
    {
        let (a, b) = (v.area, section.area);
        assert!(
            a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1],
            "{}",
            v.id
        );
    }

    let font =
        Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font);
    let a = section.area;
    let hatch = drawing.layers[2]
        .items
        .iter()
        .filter(|i| match i {
            Item::Edge(Curve::Line(p, q)) => {
                ((q[1] - p[1]) - (q[0] - p[0])).abs() < 1e-9
                    && [p, q]
                        .iter()
                        .all(|r| a[0] <= r[0] && r[0] <= a[2] && a[1] <= r[1] && r[1] <= a[3])
            }
            _ => false,
        })
        .count();
    assert!(hatch > 20, "{hatch} hatch lines");
    let texts: Vec<&str> = drawing.text.iter().map(|t| t.text.as_str()).collect();
    assert!(texts.contains(&"SECTION A\u{2013}A"), "{texts:?}");
    assert_eq!(texts.iter().filter(|t| **t == "A").count(), 2);

    let spec: Spec =
        serde_json::from_value(serde_json::json!({ "views": { "section_aa": { "omit": true } } }))
            .unwrap();
    let omitted = model::plan(&analysis, &spec).unwrap();
    assert_eq!(ids(&omitted), ["v:front", "v:plan", "v:side", "v:iso"]);
}
