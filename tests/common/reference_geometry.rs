//! A reference becoming controlling reflows text, while its witness geometry stays fixed.
use draftwright::dims::{PlannedDimension, label_ink};
use draftwright::text::Font;

pub fn assert_local(was: &PlannedDimension, now: &PlannedDimension) {
    if was.label == now.label {
        assert_eq!(now.placed, was.placed, "{}", now.id);
        return;
    }
    assert_eq!(was.label, format!("({})", now.label), "{}", now.id);
    assert_eq!(now.view, was.view);
    assert_eq!(now.value, was.value);
    let (old, new) = (was.placed.as_ref().unwrap(), now.placed.as_ref().unwrap());
    assert_eq!(new.lines, old.lines, "{} witnesses", now.id);
    assert_eq!(new.arrows, old.arrows, "{} arrows", now.id);
    assert_eq!(new.rotation, old.rotation);
    let angle = new.rotation.to_radians();
    let font = Font::new(include_bytes!("../../fonts/IBMPlexMono-Regular.ttf").to_vec()).unwrap();
    let ink = label_ink(&now.terms, &font);
    let corners = [
        ink.min,
        [ink.max[0], ink.min[1]],
        ink.max,
        [ink.min[0], ink.max[1]],
    ];
    let mut expected = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in corners {
        let q = [
            new.label_at[0] + p[0] * angle.cos() - p[1] * angle.sin(),
            new.label_at[1] + p[0] * angle.sin() + p[1] * angle.cos(),
        ];
        for k in 0..2 {
            expected[k] = expected[k].min(q[k]);
            expected[k + 2] = expected[k + 2].max(q[k]);
        }
    }
    for (actual, expected) in new.label_box.iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-9, "{} text bounds", now.id);
    }
    let k = if new.rotation == 0.0 { 0 } else { 1 };
    // The text keeps its centre (relabelled in place) or, where H3's settlement holds it to an
    // arrow tail or a span end, the edge it was settled against; it never wanders.
    let (a, b) = (new.label_box, old.label_box);
    assert!(
        (a[k] + a[k + 2] - b[k] - b[k + 2]).abs() < 1e-9
            || (a[k] - b[k]).abs() < 1e-9
            || (a[k + 2] - b[k + 2]).abs() < 1e-9,
        "{} text centre or settled edge",
        now.id
    );
    assert_eq!(new.label_at[1 - k], old.label_at[1 - k]);
}
