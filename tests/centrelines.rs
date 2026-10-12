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
    let (_, p, drawn) = drawing(&dir.join("gramel/string_post.step"));
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
            for letter in drawn.sheet.items.iter().filter(|i| {
                i.kind == draftwright::output::Kind::CuttingPlane
                    || (i.kind == draftwright::output::Kind::Caption
                        && matches!(i.name.as_str(), "section_a_left" | "section_a_right"))
            }) {
                let l = letter.label.unwrap_or(letter.ink);
                if letter.kind == draftwright::output::Kind::CuttingPlane && d.kind == "callout" {
                    for [a, z] in &g.lines {
                        // The section glyph's full ink box is a hard reservation, including its wing.
                        let mut lo: f64 = 0.;
                        let mut hi: f64 = 1.;
                        for k in 0..2 {
                            let delta = z[k] - a[k];
                            if delta.abs() < 1e-9 {
                                if a[k] <= l[k] || a[k] >= l[k + 2] {
                                    hi = -1.;
                                    break;
                                }
                            } else {
                                let x = (l[k] - a[k]) / delta;
                                let y = (l[k + 2] - a[k]) / delta;
                                lo = lo.max(x.min(y));
                                hi = hi.min(x.max(y));
                            }
                        }
                        assert!(hi <= lo, "{} leader touches {}", d.id, letter.name);
                    }
                }
                assert!(
                    b[2] < l[0] || l[2] < b[0] || b[3] < l[1] || l[3] < b[1],
                    "{} label touches {}",
                    d.id,
                    letter.name
                );
            }
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

#[test]
fn fixed_ink_review_catches_the_captured_section_defects() {
    let Some(dir) = corpus::dir() else { return };
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/g6_section_contacts.json")).unwrap();
    for case in cases {
        let (_, _, mut drawn) = drawing(&dir.join(case["step"].as_str().unwrap()));
        let code = case["code"].as_str().unwrap();
        assert!(
            !draftwright::lint::structural::check(&drawn)
                .iter()
                .any(|i| i.code == code),
            "{}: fixed sheet still has {code}: {:?}",
            case["step"],
            draftwright::lint::structural::check(&drawn)
                .iter()
                .filter(|i| i.code == code)
                .collect::<Vec<_>>()
        );
        for captured in case["dimensions"].as_array().unwrap() {
            let g: draftwright::dims::Placed =
                serde_json::from_value(captured["placed"].clone()).unwrap();
            let item = drawn
                .sheet
                .items
                .iter_mut()
                .find(|i| i.name == captured["id"].as_str().unwrap())
                .unwrap();
            item.label = Some(g.label_box);
            item.lines = g.lines;
            item.elbow = Some(item.lines[0][1]);
        }
        let found: Vec<_> = draftwright::lint::structural::check(&drawn)
            .into_iter()
            .filter(|i| i.code == code)
            .collect();
        assert!(
            !found.is_empty(),
            "{}: captured {} was missed",
            case["source"],
            code
        );
        if code == "leader_centerline_overlap" {
            assert_eq!(
                found
                    .iter()
                    .filter(|i| i.related == ["section_line"])
                    .count(),
                2,
                "both original string_post leaders must be counted"
            );
        }
    }
}

#[test]
fn an_actual_section_hatch_is_not_a_label_on_its_axis() {
    let Some(dir) = corpus::dir() else { return };
    let (_, _, mut drawn) = drawing(&dir.join("gramel/string_post.step"));
    let contacts = |d: &draftwright::output::Drawing| {
        draftwright::lint::structural::check(d)
            .into_iter()
            .filter(|i| {
                i.code == "label_centerline_overlap"
                    && i.annotation.as_deref() == Some("section_hatch")
            })
            .count()
    };
    assert_eq!(contacts(&drawn), 0);
    let hatch = drawn
        .sheet
        .items
        .iter_mut()
        .find(|i| i.name == "section_hatch")
        .unwrap();
    eprintln!("CAPTURED_HATCH {} {:?}", hatch.name, hatch.ink);
    // Emulate Python's fallback on the same captured geometry, without changing any ink.
    hatch.label = Some(hatch.ink);
    assert!(
        contacts(&drawn) > 0,
        "the whole-ink fallback should flag this hatch as text"
    );
}

#[test]
fn callouts_do_not_borrow_hole_centre_strokes() {
    let Some(dir) = corpus::dir() else { return };
    for file in [
        "nist/nist_ctc_02_asme1_rc.stp",
        "nist/nist_ctc_04_asme1_rd.stp",
        "nist/nist_ftc_10_asme1_rb.stp",
    ] {
        let (_, p, drawn) = drawing(&dir.join(file));
        let contacts: Vec<_> = draftwright::lint::structural::check(&drawn)
            .into_iter()
            .filter(|i| {
                matches!(
                    i.code.as_str(),
                    "leader_centerline_overlap" | "label_section_furniture_overlap"
                )
            })
            .collect();
        assert!(contacts.is_empty(), "{file}: {contacts:?}");
        if file.ends_with("nist_ftc_10_asme1_rb.stp") {
            let d = p
                .dimensions
                .iter()
                .find(|d| d.id == "d:f:hole:17/bore.diameter")
                .unwrap();
            let v = p.views.iter().find(|v| v.id == d.view).unwrap();
            let centre = v
                .line_work
                .iter()
                .find(|w| w.id == "ink:v:side:f:hole:17")
                .unwrap()
                .centre
                .unwrap();
            let [tip, elbow] = d.placed.as_ref().unwrap().lines[0];
            let radius = d.value * v.scale / 2.;
            let u = [tip[0] - centre[0], tip[1] - centre[1]];
            assert!(
                (u[0].hypot(u[1]) - radius).abs() < 1e-6,
                "the revised tip must stay on the opening rim"
            );
            assert!(
                u[0] * (elbow[0] - tip[0]) + u[1] * (elbow[1] - tip[1]) > 0.,
                "the leader must leave the opening outward"
            );
        }
    }
}
