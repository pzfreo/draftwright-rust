//! ISO 128 furniture belongs to the planned view, independently of dimensional intent.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::{
    api,
    model::{self, Spec},
    text::Font,
};
use serde_json::Value;
use std::path::Path;

fn drawing(file: &Path) -> (model::Analysis, model::Plan, draftwright::output::Drawing) {
    let (part, a) = model::analyse_native(file).unwrap();
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let d = model::render(&part, &a, &p, &Font::new(api::MONO.to_vec()).unwrap()).unwrap();
    (a, p, d)
}

fn work(p: &model::Plan, kind: &str) -> Vec<Value> {
    serde_json::to_value(p).unwrap()["views"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| v["line_work"].as_array().cloned().unwrap_or_default())
        .filter(|w| w["kind"] == kind)
        .collect()
}

#[test]
fn a_plate_marks_every_pattern_member_and_lint_reads_the_drawn_marks() {
    let (a, p, mut d) =
        drawing(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step"));
    assert!(work(&p, "cross").len() >= 4);
    assert!(d.svg().contains("stroke-dasharray=\"8 1.5 0.2 1.5\""));
    assert!(
        !draftwright::lint::lint(&a, &d)
            .iter()
            .any(|i| i.code == "feature_no_centermark")
    );
    d.sheet.centre_marks.clear();
    assert!(
        draftwright::lint::lint(&a, &d)
            .iter()
            .any(|i| i.code == "feature_no_centermark")
    );
}

#[test]
fn turned_axes_extend_beyond_the_profile_and_survive_omitted_dimensions() {
    let Some(dir) = corpus::dir() else { return };
    let (a, p, _) = drawing(&dir.join("gramel/GRM-05_depth_lock_bolt.step"));
    let axes = work(&p, "axis");
    assert!(!axes.is_empty());
    assert!(axes.iter().any(|w| w["global_axis"] == true));
    for v in &p.views {
        let frame = draftwright::haecceity::hlr::View::new(v.direction, v.up).unwrap();
        let corners: Vec<_> = (0..8)
            .map(|i| frame.map([0, 1, 2].map(|k| a.envelope[(i >> k) & 1][k])))
            .collect();
        for w in v.line_work.iter().filter(|w| w.global_axis) {
            let c = &w.curves[0];
            let (start, end) = (c.start(), c.end());
            let k = usize::from((end[1] - start[1]).abs() > (end[0] - start[0]).abs());
            let lo = corners.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
            let hi = corners
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            let half = (hi - lo) * v.scale / 2.;
            assert!(start[k].min(end[k]) <= v.at[k] - half - 4.99);
            assert!(start[k].max(end[k]) >= v.at[k] + half + 4.99);
        }
    }
    let mut s = Spec::default();
    for d in &p.dimensions {
        s.dimensions.entry(d.id.clone()).or_default().omit = true;
    }
    let omitted = model::plan_recovered(&a, &s).unwrap();
    assert_eq!(work(&p, "axis"), work(&omitted, "axis"));
}

#[test]
fn bolt_circle_furniture_is_radial_and_tangential() {
    let Some(dir) = corpus::dir() else { return };
    let (_, p, _) = drawing(&dir.join("nist/nist_ctc_05_asme1_rd.stp"));
    let circles = work(&p, "pitch_circle");
    assert!(!circles.is_empty());
    let radial = work(&p, "cross");
    assert!(radial.iter().any(|w| w["radial"] == true));
    for w in radial.iter().filter(|w| w["radial"] == true) {
        let c = &w["centre"];
        let circle = circles
            .iter()
            .find(|circle| {
                let q = &circle["curves"][0]["Conic"]["centre"];
                let r = circle["curves"][0]["Conic"]["radii"][0].as_f64().unwrap();
                let distance = (c[0].as_f64().unwrap() - q[0].as_f64().unwrap())
                    .hypot(c[1].as_f64().unwrap() - q[1].as_f64().unwrap());
                (distance - r).abs() < 0.05
            })
            .unwrap();
        let q = &circle["curves"][0]["Conic"]["centre"];
        let r = [0, 1].map(|k| c[k].as_f64().unwrap() - q[k].as_f64().unwrap());
        let vectors: Vec<_> = w["curves"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                [0, 1]
                    .map(|k| c["Line"][1][k].as_f64().unwrap() - c["Line"][0][k].as_f64().unwrap())
            })
            .collect();
        assert!((vectors[0][0] * r[1] - vectors[0][1] * r[0]).abs() < 1e-6);
        assert!((vectors[1][0] * r[0] + vectors[1][1] * r[1]).abs() < 1e-6);
    }
}

#[test]
fn section_cutting_line_is_planned_and_labels_keep_clear() {
    let Some(dir) = corpus::dir() else { return };
    let (_, p, _) = drawing(&dir.join("gramel/string_post.step"));
    let cutting = work(&p, "cutting_plane");
    assert_eq!(cutting.len(), 1);
    let ends = cutting[0]["curves"][0]["Line"].as_array().unwrap();
    let (x0, x1, y) = (
        ends[0][0].as_f64().unwrap(),
        ends[1][0].as_f64().unwrap(),
        ends[0][1].as_f64().unwrap(),
    );
    for d in &p.dimensions {
        if d.view == "v:plan"
            && let Some(g) = &d.placed
        {
            let b = g.label_box;
            assert!(
                b[1] > y || b[3] < y || b[2] < x0 || b[0] > x1,
                "{} label lies on A-A",
                d.id
            );
            if d.kind == "callout" {
                for [a, b] in &g.lines {
                    assert!(
                        (a[1] - y).abs() > 0.1
                            || (b[1] - y).abs() > 0.1
                            || a[0].max(b[0]) < x0
                            || a[0].min(b[0]) > x1,
                        "{} leader runs along A-A",
                        d.id
                    );
                }
            }
        }
    }
    assert!(p.views.iter().filter(|v| v.cut.is_some()).any(|v| {
        serde_json::to_value(v).unwrap()["line_work"]
            .as_array()
            .is_some_and(|w| w.iter().any(|w| w["kind"] == "axis"))
    }));
}
