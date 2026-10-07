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
        // Every view has its camera, and its look-at point lands on its origin.
        for v in &f.views {
            let (Some(d), Some(origin), Some(c)) = (v.direction, v.origin, v.look_at) else {
                panic!("{name} {}: no camera", v.name);
            };
            assert!((d.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() < 1e-5);
            let p = v.page(c).unwrap();
            assert!((p[0] - origin[0]).abs() < 1e-9 && (p[1] - origin[1]).abs() < 1e-9);
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
