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
        // Every view has its camera: an orthonormal, right-handed basis (page x, page y, towards
        // the viewer), so a point off the look-at point lands at its page-plane offsets scaled,
        // whatever its depth. The bases are recorded to 6 decimals, hence the tolerances.
        for v in &f.views {
            let (Some(x), Some(y), Some(d), Some(origin), Some(c), Some(s)) =
                (v.x_axis, v.up, v.direction, v.origin, v.look_at, v.scale)
            else {
                panic!("{name} {}: no camera", v.name);
            };
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            for (a, b, want) in [
                (x, x, 1.0),
                (y, y, 1.0),
                (d, d, 1.0),
                (x, y, 0.0),
                (y, d, 0.0),
                (x, d, 0.0),
            ] {
                assert!(
                    (dot(a, b) - want).abs() < 1e-5,
                    "{name} {}: basis not orthonormal",
                    v.name
                );
            }
            let cross = [
                x[1] * y[2] - x[2] * y[1],
                x[2] * y[0] - x[0] * y[2],
                x[0] * y[1] - x[1] * y[0],
            ];
            assert!(
                (0..3).all(|i| (cross[i] - d[i]).abs() < 1e-5),
                "{name} {}: basis not right-handed (x × up ≠ direction)",
                v.name
            );
            let p = std::array::from_fn(|i| c[i] + 3.0 * x[i] - 2.0 * y[i] + 5.0 * d[i]);
            let q = v.page(p).unwrap();
            assert!(
                (q[0] - (origin[0] + 3.0 * s)).abs() < 1e-4
                    && (q[1] - (origin[1] - 2.0 * s)).abs() < 1e-4,
                "{name} {}: an off-axis point lands at {q:?}",
                v.name
            );
            assert!(v.kind.is_some(), "{name} {}: kind unknown", v.name);
        }
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
