//! The captured sheet facts (`tools/capture_sheet_facts.py`) read back whole: every sheet in the
//! sample loads, and carries what the comparison relies on.

#[path = "common/facts.rs"]
mod facts;

#[test]
fn every_captured_sheet_loads_with_its_views_and_measurements() {
    let names = facts::names();
    for required in ["m10127", "string_post", "ctc01"] {
        assert!(
            names.iter().any(|n| n == required),
            "{required} not captured"
        );
    }
    for name in &names {
        let f = facts::load(name);
        assert_eq!(f.binding.sha256.len(), 64, "{name}");
        assert!(
            f.sheet.page[0] > 0.0 && f.sheet.scale.contains(':'),
            "{name}"
        );
        assert!(f.title_block.contains_key("title"), "{name}");
        // Every view has its camera: an orthonormal basis for the page's x and y, read from
        // draftwright's view coordinates and recorded to 6 decimals, hence the tolerance.
        // (`direction` is `x_axis × up` by construction of the capture, so it is not checked.)
        for v in &f.views {
            let (Some(x), Some(y), Some(_), Some(_), Some(_), Some(_)) =
                (v.x_axis, v.up, v.direction, v.origin, v.look_at, v.scale)
            else {
                panic!("{name} {}: no camera", v.name);
            };
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            for (a, b, want) in [(x, x, 1.0), (y, y, 1.0), (x, y, 0.0)] {
                assert!(
                    (dot(a, b) - want).abs() < 1e-5,
                    "{name} {}: basis not orthonormal",
                    v.name
                );
            }
            assert!(v.kind.is_some(), "{name} {}: kind unknown", v.name);
        }
        // The camera against independent data: each feature an annotation names, projected by
        // its view's camera, lands inside the box of that view's projected geometry
        // (`Drawing.view_bounds`, taken from the placed drawing, not from the camera). A detail
        // view crops the part, so it is not checked; nor is a blend, whose frame origin is its
        // axis and can lie off the material (on cgb245's side view, 0.39 mm outside the box).
        let mut checked = 0;
        for a in &f.annotations {
            let Some(v) = f
                .views
                .iter()
                .find(|v| a.view.as_deref() == Some(v.name.as_str()))
            else {
                continue;
            };
            if v.kind.as_deref() == Some("detail") {
                continue;
            }
            let b = v
                .bbox
                .unwrap_or_else(|| panic!("{name} {}: no box", v.name));
            let named = a.features.iter().flatten();
            let measured = a.measurements.iter().filter_map(|m| m.feature.as_ref());
            for feature in named.chain(measured).filter(|f| f.kind != "blend") {
                let Some(p) = feature.origin else { continue };
                let q = v.page(p).unwrap();
                assert!(
                    q[0] > b[0] - 1e-3
                        && q[0] < b[2] + 1e-3
                        && q[1] > b[1] - 1e-3
                        && q[1] < b[3] + 1e-3,
                    "{name} {} {}: {:?} lands at {q:?}, outside the view's box {b:?}",
                    v.name,
                    a.name,
                    feature.key
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "{name}: no feature placed in a view");
        // Every measuring annotation prints something and names its view.
        let measuring: Vec<_> = f
            .annotations
            .iter()
            .filter(|a| !a.measurements.is_empty())
            .collect();
        assert!(!measuring.is_empty(), "{name}: no measurements");
        for a in measuring {
            assert!(!a.text.is_empty(), "{name} {}: no text", a.name);
            assert!(a.view.is_some(), "{name} {}: no view", a.name);
        }
    }
}
