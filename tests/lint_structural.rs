//! Lint's structural checks of the drawn sheet (`lint::structural`, draftwright's `lint_drawing`):
//!
//! - **Each rule at its threshold**, on sheets built through the public types: firing just past
//!   it and not at it.
//! - **The port's own drawings** of the captured corpus parts (as `tests/lint.rs` draws them:
//!   the corpus's, with their features from `tests/fixtures/recognition/corpus/`, under the
//!   default spec): every finding, counted by part and code, is listed in
//!   `tests/fixtures/known_structural.json` with a verdict and the evidence. A new, changed or
//!   vanished entry fails. The placement problems of `tests/fixtures/known_placement.json` judge
//!   the same collisions from the plan (`annotation_overlap` there is any overlap of two
//!   dimensions' label boxes, `annotation_off_sheet` any of a dimension's label corners or line
//!   ends off the drawable area); the reasons here cross-reference them. Set
//!   `LINT_REPORT=1` to print each finding, and `LINT_OWN_ALL=1` to draw connector109 too.
//! - **Python's captured sheets** (`tests/fixtures/sheets/`): the checks run on what the facts
//!   record and are compared with draftwright's `lint` list, code by code. The facts carry each
//!   view's box (visible and hidden lines, where draftwright's lint boxes the visible lines
//!   alone, so the facts' box holds lint's), each label's text box, and the page; they carry no
//!   annotation's whole ink and no title block box. So `view_overlap`, `view_out_of_bounds` and
//!   `derived_view_identifier_reused` are compared whole (a finding on the facts' larger boxes
//!   that draftwright did not report would fail; none does, so lint's boxes give none either);
//!   `annotation_overlap` is compared over the pairs of labelled annotations, which is every pair
//!   draftwright compares but those with a title block, projection symbol, section wing or
//!   arrowhead (compared by whole ink); `annotation_out_of_bounds` over the labels alone (a label
//!   off the page is ink off the page, so the port's findings must be among draftwright's); and
//!   `scale_not_stated` is not run on the facts (no title block box to give the sheet one), but
//!   every captured sheet records draftwright's `scale_note` and no finding, as the rule says.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use draftwright::lint::{self, LintIssue, Severity, structural};
use draftwright::model;
use draftwright::output::{Drawing, Drawn, DrawnView, Kind, Sheet};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    part: String,
    code: String,
    count: usize,
    /// `rust-correct`, `rust-wrong`, `equivalent`, `undetermined` or `not-applicable`.
    verdict: String,
    reason: String,
}

const VERDICTS: [&str; 5] = [
    "rust-correct",
    "rust-wrong",
    "equivalent",
    "undetermined",
    "not-applicable",
];

/// The codes this module ports.
const CODES: [&str; 6] = [
    "derived_view_identifier_reused",
    "scale_not_stated",
    "annotation_overlap",
    "annotation_out_of_bounds",
    "view_overlap",
    "view_out_of_bounds",
];

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// An A4 sheet (drawable 10..287 × 10..200) with *views* and *items* and nothing else drawn.
fn sheet(views: &[(&str, [f64; 4])], items: Vec<Drawn>) -> Drawing {
    Drawing {
        page: [297.0, 210.0],
        layers: Vec::new(),
        link: None,
        text: Vec::new(),
        annotations: Vec::new(),
        sheet: Sheet {
            drawable: Some([10.0, 10.0, 287.0, 200.0]),
            views: views
                .iter()
                .map(|(name, bbox)| DrawnView {
                    name: name.to_string(),
                    bbox: *bbox,
                })
                .collect(),
            items,
        },
        problems: Vec::new(),
        spec: None,
    }
}

fn item(name: &str, kind: Kind, ink: [f64; 4], label: Option<[f64; 4]>) -> Drawn {
    Drawn {
        name: name.into(),
        kind,
        ink,
        label,
    }
}

/// A label whose ink is its text.
fn label(name: &str, b: [f64; 4]) -> Drawn {
    item(name, Kind::Dimension, b, Some(b))
}

fn codes(issues: &[LintIssue]) -> Vec<&str> {
    issues.iter().map(|i| i.code.as_str()).collect()
}

/// A title block and a scale note, well inside the sheet and apart.
fn furniture() -> Vec<Drawn> {
    vec![
        item(
            "title_block",
            Kind::TitleBlock,
            [107.0, 11.0, 286.0, 40.0],
            None,
        ),
        item(
            "scale_note",
            Kind::ScaleNote,
            [260.0, 46.0, 272.0, 48.2],
            Some([260.0, 46.0, 272.0, 48.2]),
        ),
    ]
}

#[test]
fn a_finished_sheet_states_its_scale() {
    let mut items = furniture();
    assert!(structural::check(&sheet(&[], items.clone())).is_empty());
    // The title block without the scale note: an error, also when other text says SCALE (only
    // the scale note states it).
    items[1].kind = Kind::Dimension;
    let found = structural::check(&sheet(&[], items.clone()));
    assert_eq!(codes(&found), ["scale_not_stated"]);
    let found = structural::check(&sheet(&[], items[..1].to_vec()));
    assert_eq!(codes(&found), ["scale_not_stated"]);
    assert_eq!(found[0].severity, Severity::Error);
    // Neither: a fragment, with nothing to state a scale about.
    assert!(structural::check(&sheet(&[], Vec::new())).is_empty());
}

#[test]
fn views_overlap_when_their_boxes_do() {
    let front = [20.0, 20.0, 100.0, 80.0];
    let touching = [100.0, 20.0, 150.0, 80.0];
    assert!(
        structural::check(&sheet(
            &[("v:front", front), ("v:side", touching)],
            Vec::new()
        ))
        .is_empty()
    );
    let into = [99.999, 20.0, 150.0, 80.0];
    let found = structural::check(&sheet(&[("v:front", front), ("v:side", into)], Vec::new()));
    assert_eq!(codes(&found), ["view_overlap"]);
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].view.as_deref(), Some("v:front"));
    assert_eq!(found[0].related, ["v:side"]);
    assert!(
        found[0]
            .message
            .starts_with("view 'front' bbox [x=20.0–100.0"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_view_stays_inside_the_drawable_area() {
    let at_edge = [10.0 - 0.5e-9, 20.0, 100.0, 200.0];
    assert!(structural::check(&sheet(&[("v:front", at_edge)], Vec::new())).is_empty());
    let past = [10.0 - 2e-9, 20.0, 100.0, 200.5];
    let found = structural::check(&sheet(&[("v:front", past)], Vec::new()));
    assert_eq!(codes(&found), ["view_out_of_bounds", "view_out_of_bounds"]);
    assert!(found.iter().all(|i| i.severity == Severity::Error));
    assert!(
        found[0].message.contains("(left by 0.0 mm)"),
        "{}",
        found[0].message
    );
    assert!(
        found[1].message.contains("(above by 0.5 mm)"),
        "{}",
        found[1].message
    );
}

#[test]
fn every_annotation_stays_inside_the_drawable_area() {
    // A dimension's whole ink counts, not only its label.
    let inside = item(
        "d:a",
        Kind::Dimension,
        [10.0, 50.0, 60.0, 60.0],
        Some([30.0, 55.0, 40.0, 58.0]),
    );
    assert!(structural::check(&sheet(&[], vec![inside.clone()])).is_empty());
    let mut out = inside;
    out.ink[1] = 9.0;
    let found = structural::check(&sheet(&[], vec![out]));
    assert_eq!(codes(&found), ["annotation_out_of_bounds"]);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].annotation.as_deref(), Some("d:a"));
    assert!(
        found[0].message.contains("(below by 1.0 mm)"),
        "{}",
        found[0].message
    );
    // The title block may straddle the edge by half its border's stroke (0.075 mm), no more.
    let mut items = furniture();
    items[0].ink[2] = 287.075;
    assert!(structural::check(&sheet(&[], items.clone())).is_empty());
    items[0].ink[2] = 287.076;
    assert_eq!(
        codes(&structural::check(&sheet(&[], items))),
        ["annotation_out_of_bounds"]
    );
}

#[test]
fn labels_overlap_by_more_than_half_a_millimetre_both_ways() {
    let a = [50.0, 50.0, 60.0, 53.0];
    let check =
        |b: [f64; 4]| structural::check(&sheet(&[], vec![label("d:a", a), label("d:b", b)]));
    // 0.5 mm each way is not an overlap; 0.51 mm each way is.
    assert!(check([59.5, 52.5, 70.0, 56.0]).is_empty());
    assert!(check([59.49, 52.5, 70.0, 56.0]).is_empty());
    assert!(check([59.5, 52.49, 70.0, 56.0]).is_empty());
    let found = check([59.49, 52.49, 70.0, 56.0]);
    assert_eq!(codes(&found), ["annotation_overlap"]);
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].annotation.as_deref(), Some("d:a"));
    assert_eq!(found[0].related, ["d:b"]);
    assert!(
        found[0].message.contains("overlap by 0.5×0.5 mm"),
        "{}",
        found[0].message
    );
    // The labels' text boxes are compared, not the dimensions' whole ink (witness lines of
    // stacked dimensions overlap by design).
    let stacked = [
        item(
            "d:a",
            Kind::Dimension,
            [20.0, 50.0, 90.0, 70.0],
            Some([50.0, 66.0, 60.0, 69.0]),
        ),
        item(
            "d:b",
            Kind::Dimension,
            [20.0, 50.0, 90.0, 80.0],
            Some([50.0, 76.0, 60.0, 79.0]),
        ),
    ];
    assert!(structural::check(&sheet(&[], stacked.to_vec())).is_empty());
    // A thing with no label takes part by its whole ink; a section's hatch and cutting-plane ink
    // take none.
    let block = item(
        "title_block",
        Kind::TitleBlock,
        [55.0, 40.0, 120.0, 52.0],
        None,
    );
    let hatch = item(
        "section_ink",
        Kind::SectionInk,
        [40.0, 40.0, 80.0, 60.0],
        None,
    );
    let found = structural::check(&sheet(&[], vec![label("d:a", a), block, hatch]));
    assert_eq!(codes(&found), ["scale_not_stated", "annotation_overlap"]);
    assert_eq!(found[1].related, ["title_block"]);
}

#[test]
fn one_identifier_names_one_derived_view() {
    let b = |x: f64| [x, 20.0, x + 10.0, 30.0];
    let views = [
        ("v:front", b(20.0)),
        ("v:section_aa", b(40.0)),
        ("v:detail_b", b(60.0)),
    ];
    assert!(structural::check(&sheet(&views, Vec::new())).is_empty());
    let views = [
        ("v:section_aa", b(40.0)),
        ("v:detail_a", b(60.0)),
        ("v:section_ab", b(80.0)),
    ];
    let found = structural::check(&sheet(&views, Vec::new()));
    assert_eq!(codes(&found), ["derived_view_identifier_reused"]);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].message,
        "derived-view identifier 'A' names multiple views: 'section_aa', 'detail_a'"
    );
}

/// The boxes the sheet records are the curves' own: a conic arc and a Bézier bound at their
/// turning points, not their poles.
#[test]
fn a_curves_box_is_tight() {
    use draftwright::output::Curve;
    let close = |a: [f64; 4], b: [f64; 4]| (0..4).all(|k| (a[k] - b[k]).abs() < 1e-9);
    let arc = |t0: f64, t1: f64| Curve::Conic {
        centre: [1.0, 1.0],
        radii: [2.0, 2.0],
        x_axis: [1.0, 0.0],
        y_axis: [0.0, 1.0],
        t0,
        t1,
    };
    let pi = std::f64::consts::PI;
    assert!(close(arc(0.0, 2.0 * pi).bbox(), [-1.0, -1.0, 3.0, 3.0]));
    assert!(close(
        arc(0.25 * pi, 0.75 * pi).bbox(),
        [1.0 - 2f64.sqrt(), 1.0 + 2f64.sqrt(), 1.0 + 2f64.sqrt(), 3.0]
    ));
    // y = 6t(1-t)(1-2t) turns at ±1/√3; its poles reach ±2.
    let cubic = Curve::Beziers(vec![vec![[0.0, 0.0], [1.0, 2.0], [2.0, -2.0], [3.0, 0.0]]]);
    let r = 1.0 / 3f64.sqrt();
    assert!(close(cubic.bbox(), [0.0, -r, 3.0, r]), "{:?}", cubic.bbox());
    // A rational quarter circle (weights 1, 1/√2, 1).
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let quarter = Curve::Rational(vec![vec![[1.0, 0.0, 1.0], [w, w, w], [0.0, 1.0, 1.0]]]);
    assert!(close(quarter.bbox(), [0.0, 0.0, 1.0, 1.0]));
}

/// The port's own drawings of the captured parts: every structural finding is known.
#[test]
fn structural_findings_on_the_ports_own_drawings_are_known() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let known: Vec<Known> =
        serde_json::from_str(&std::fs::read_to_string(fixture("known_structural.json")).unwrap())
            .unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    // As tests/lint.rs: ctc04 has no features, and connector109 takes minutes to draw, so it is
    // drawn only with LINT_OWN_ALL=1 (its entries stay listed, and are checked then).
    let all = std::env::var_os("LINT_OWN_ALL").is_some();
    let names: Vec<String> = facts::names()
        .into_iter()
        .filter(|n| corpus::recognition(n).is_some())
        .filter(|n| all || n != "connector109")
        .collect();
    let linted: Vec<Vec<LintIssue>> = std::thread::scope(|s| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let (dir, font) = (&dir, &font);
                s.spawn(move || {
                    let f = facts::load(name);
                    let (part, analysis) = corpus::analyse(dir, &f.binding.file, name);
                    let plan = model::plan(&analysis, &model::Spec::default()).unwrap();
                    let drawing = model::render(&part, &analysis, &plan, font).unwrap();
                    // The furniture the checks rely on is recorded on every finished sheet.
                    let kinds: Vec<Kind> = drawing.sheet.items.iter().map(|d| d.kind).collect();
                    assert!(
                        kinds.contains(&Kind::TitleBlock) && kinds.contains(&Kind::ScaleNote),
                        "{name}"
                    );
                    assert_eq!(
                        kinds.iter().filter(|k| **k == Kind::Dimension).count(),
                        drawing.annotations.len(),
                        "{name}: every annotation drawn is measured"
                    );
                    assert_eq!(drawing.sheet.views.len(), plan.views.len(), "{name}");
                    let issues = structural::check(&drawing);
                    // The placement problems judge the same collisions from the plan: a
                    // dimension the plan finds off the sheet (a label corner or a line end) has
                    // ink off it, and two labels whose boxes the plan finds apart cannot overlap.
                    let flagged = |code: &str| -> BTreeSet<String> {
                        issues
                            .iter()
                            .filter(|i| i.code == code)
                            .flat_map(|i| i.annotation.iter().chain(&i.related).cloned())
                            .collect()
                    };
                    let off = flagged("annotation_out_of_bounds");
                    for p in plan
                        .problems
                        .iter()
                        .filter(|p| p.code == "annotation_off_sheet")
                    {
                        assert!(
                            off.contains(&p.subjects[0]),
                            "{name}: {} has no ink off the sheet",
                            p.id
                        );
                    }
                    let placed_overlaps: BTreeSet<String> = plan
                        .problems
                        .iter()
                        .filter(|p| p.code == "annotation_overlap")
                        .flat_map(|p| p.subjects.iter().cloned())
                        .collect();
                    for i in issues.iter().filter(|i| i.code == "annotation_overlap") {
                        let pair = i.annotation.iter().chain(&i.related);
                        let dims = pair.clone().filter(|n| n.starts_with("d:")).count();
                        assert!(
                            dims < 2 || pair.into_iter().all(|n| placed_overlaps.contains(n)),
                            "{name}: {} overlap the plan does not report",
                            i.id
                        );
                    }
                    issues
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut found: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (name, issues) in names.iter().zip(linted) {
        let ids: BTreeSet<&str> = issues.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids.len(), issues.len(), "{name}: finding IDs are unique");
        for issue in &issues {
            assert!(CODES.contains(&issue.code.as_str()), "{}", issue.code);
            if std::env::var_os("LINT_REPORT").is_some() {
                eprintln!("{name}: {} {} [{}]", issue.code, issue.message, issue.id);
            }
            *found.entry((name.clone(), issue.code.clone())).or_default() += 1;
        }
    }
    let mut listed: BTreeMap<(String, String), usize> = BTreeMap::new();
    for k in &known {
        assert!(
            VERDICTS.contains(&k.verdict.as_str()) && !k.reason.is_empty(),
            "{k:?}"
        );
        assert!(CODES.contains(&k.code.as_str()), "{k:?}");
        let old = listed.insert((k.part.clone(), k.code.clone()), k.count);
        assert!(old.is_none(), "{k:?} is listed twice");
    }
    let listed: BTreeMap<(String, String), usize> = listed
        .into_iter()
        .filter(|((part, _), _)| names.contains(part))
        .collect();
    assert_eq!(
        found, listed,
        "structural findings on the port's own drawings (left) differ from known_structural.json (right)"
    );
}

/// What draftwright's lint reports in *codes* on a captured sheet, by code: each finding's
/// annotation and related annotations, or its message when it names none.
fn python(f: &facts::Facts, code: &str) -> Vec<String> {
    let mut out: Vec<String> = f
        .lint
        .iter()
        .filter(|i| i.code == code)
        .map(|i| match &i.annotation {
            Some(a) => std::iter::once(a.clone())
                .chain(i.related.iter().cloned())
                .collect::<Vec<_>>()
                .join("+"),
            None => i.message.clone(),
        })
        .collect();
    out.sort();
    out
}

/// The same for the port's findings, by draftwright's names.
fn port(issues: &[LintIssue], code: &str) -> Vec<String> {
    let mut out: Vec<String> = issues
        .iter()
        .filter(|i| i.code == code)
        .map(|i| match &i.annotation {
            Some(a) => std::iter::once(a.clone())
                .chain(i.related.iter().cloned())
                .collect::<Vec<_>>()
                .join("+"),
            None => i.message.clone(),
        })
        .collect();
    out.sort();
    out
}

/// The checks on what Python's captured sheets record agree with draftwright's lint (see the
/// module's note for what the facts allow).
#[test]
fn structural_checks_on_pythons_sheets_agree_with_pythons_lint() {
    let mut compared = 0;
    for name in facts::names() {
        let f = facts::load(&name);
        let [w, h] = f.sheet.page;
        let labelled = f.annotations.iter().filter_map(|a| {
            let b = a.label_bbox?;
            let kind = if a.name == "scale_note" {
                Kind::ScaleNote
            } else {
                Kind::Dimension
            };
            Some(item(&a.name, kind, b, Some(b)))
        });
        let drawing = Drawing {
            page: f.sheet.page,
            layers: Vec::new(),
            link: None,
            text: Vec::new(),
            annotations: Vec::new(),
            sheet: Sheet {
                drawable: Some([10.0, 10.0, w - 10.0, h - 10.0]),
                views: f
                    .views
                    .iter()
                    .filter_map(|v| {
                        Some(DrawnView {
                            name: format!("v:{}", v.name),
                            bbox: v.bbox?,
                        })
                    })
                    .collect(),
                items: labelled.collect(),
            },
            problems: Vec::new(),
            spec: None,
        };
        assert!(
            f.annotations.iter().any(|a| a.name == "scale_note")
                && f.annotations.iter().any(|a| a.kind == "TitleBlock")
                && python(&f, "scale_not_stated").is_empty(),
            "{name}: draftwright states the scale on a finished sheet"
        );
        let issues = structural::check(&drawing);
        for code in [
            "derived_view_identifier_reused",
            "annotation_overlap",
            "view_overlap",
            "view_out_of_bounds",
        ] {
            assert_eq!(port(&issues, code), python(&f, code), "{name}: {code}");
            compared += python(&f, code).len();
        }
        let theirs = python(&f, "annotation_out_of_bounds");
        for p in port(&issues, "annotation_out_of_bounds") {
            assert!(
                theirs.contains(&p),
                "{name}: {p} off the page, which draftwright did not report"
            );
        }
    }
    // ctc04's two labels: '8× ⌀14 THRU' (hc_plan6) and '150.0°' (m_angle_1), 4.7 × 2.7 mm.
    assert_eq!(
        compared, 1,
        "draftwright's findings in these codes on the captured sheets"
    );
}

/// The review reports the structural findings beside coverage (`Review::structural`; draftwright
/// runs them first in one list). `Review::lint` stays coverage alone, as `tests/lint.rs` pins it:
/// the plate's own drawing has nothing off the sheet and coverage finds nothing (its y location
/// ran 0.1 mm past the left border until placement's fall-through, stream d-placement-fit). On
/// A4 at 5:1 (the crowded plate of `tests/issue_drafting.rs`) every annotation the structural
/// check finds past the border the plan reports too (`annotation_off_sheet`). On A4 at 10:1 the
/// plate runs off the sheet.
#[test]
fn the_review_reports_structural_findings_beside_coverage() {
    let step = fixture("parts/plate.step");
    let features = fixture("recognition/plate.json");
    let review = draftwright::api::review(&step, Some(&features), &model::Spec::default()).unwrap();
    assert!(review.lint.is_empty());
    assert!(review.structural.is_empty(), "{:#?}", review.structural);
    assert!(
        review
            .problems
            .iter()
            .all(|p| p.code != "annotation_off_sheet")
    );
    let mut spec = model::Spec::default();
    spec.sheet.size = Some("A4".into());
    spec.sheet.scale = Some("5:1".into());
    let review = draftwright::api::review(&step, Some(&features), &spec).unwrap();
    let off: Vec<&str> = review
        .structural
        .iter()
        .filter(|i| i.code == "annotation_out_of_bounds")
        .map(|i| i.id.as_str())
        .collect();
    assert!(!off.is_empty(), "{:#?}", review.structural);
    for id in off {
        let subject = id
            .strip_prefix("l:annotation_out_of_bounds:")
            .and_then(|s| s.rsplit_once(':'))
            .unwrap()
            .0;
        assert!(
            review
                .problems
                .iter()
                .any(|p| p.id == format!("p:annotation_off_sheet:{subject}")),
            "{id}: the plan reports no annotation_off_sheet for {subject}"
        );
    }
    let mut spec = model::Spec::default();
    spec.sheet.size = Some("A4".into());
    spec.sheet.scale = Some("10:1".into());
    let review = draftwright::api::review(&step, Some(&features), &spec).unwrap();
    let (part, mut analysis) = model::analyse(&step).unwrap();
    model::recognise(
        &part,
        &mut analysis,
        &std::fs::read_to_string(&features).unwrap(),
    )
    .unwrap();
    let plan = model::plan(&analysis, &spec).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
            .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let structural = structural::check(&drawing);
    assert!(
        codes(&structural).contains(&"view_out_of_bounds"),
        "{structural:#?}"
    );
    assert_eq!(review.structural, structural);
    assert_eq!(review.lint, lint::lint(&analysis, &drawing));
    assert!(
        lint::lint(&analysis, &drawing)
            .iter()
            .all(|i| !CODES.contains(&i.code.as_str()))
    );
}
