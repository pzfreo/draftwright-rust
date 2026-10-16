//! Text/feature catalogue rules, checked through the same public representations as planning.
use std::path::{Path, PathBuf};

use draftwright::{dims, model, text::Font};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts/rules-text")
        .join(format!("{name}.step"))
}

/// ARCH0008-1: symbols unsupported by the bundled font remain real ink with finite bounds;
/// a callout's display recipe does not become a measurement parameter.
#[test]
fn callout_symbols_are_geometry_and_claims_are_semantic_roles() {
    let font = Font::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fonts")
            .join(dims::FONT),
    )
    .unwrap();
    for (symbol, strokes) in [("↧", 4), ("⌴", 3), ("⌵", 2)] {
        let ink = dims::label_ink(&[symbol.into()], &font);
        assert_eq!(ink.strokes.len(), strokes, "{symbol}");
        assert!(
            ink.glyphs.is_empty(),
            "{symbol} must not depend on a missing glyph"
        );
        assert!(ink.min.iter().chain(&ink.max).all(|x| x.is_finite()));
        assert!(ink.min[0] < ink.max[0] && ink.min[1] < ink.max[1]);
    }
    let (_, a) = model::analyse_native(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step"),
    )
    .unwrap();
    let plan = model::plan(&a, &model::Spec::default()).unwrap();
    let callouts: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| d.kind == "callout")
        .collect();
    assert!(!callouts.is_empty());
    for d in callouts {
        assert!(d.label.contains('⌀'));
        assert!(!d.measurements.is_empty());
        for m in &d.measurements {
            assert!(a.features.iter().any(|f| f.id == m.feature));
            assert!(!m.parameter.contains('⌀') && !m.parameter.contains('×'));
            assert!(m.parameter.contains('.'), "a semantic role: {m:?}");
        }
    }
}

fn document(name: &str) -> quiddity::recognition::RecognitionDocument {
    let (part, _) = model::analyse(&fixture(name)).unwrap();
    let recognition = quiddity::correspondence::recognise(&part);
    quiddity::recognition::document(&recognition, part.faces.len())
}

/// ISSUE-958-a: the exact Python solids. Exercise quiddity's document, including recess families
/// the native reader does not yet take, so an unread family cannot make a negative test vacuous.
#[test]
fn separate_solids_do_not_invent_features_and_preserve_real_slots_pads_and_pockets() {
    for name in ["detached", "three-body-pocket", "three-body-channel"] {
        let doc = document(name);
        for family in ["slots", "pads", "pockets", "channels", "section_recesses"] {
            assert!(
                !doc.records.iter().any(|r| r.family == family),
                "{name}: false {family}: {:?}",
                doc.records
            );
        }
        let (part, a) = model::analyse(&fixture(name)).unwrap();
        let native = draftwright::bridge::native::read(&part, &doc).unwrap();
        assert!(
            !native
                .features
                .iter()
                .any(|f| matches!(f.kind.as_str(), "slot" | "pad"))
        );
        let mut a = a;
        model::recognise_native(&part, &mut a).unwrap();
        let plan = model::plan(&a, &model::Spec::default()).unwrap();
        assert!(
            !plan
                .dimensions
                .iter()
                .flat_map(|d| &d.measurements)
                .any(|m| m.parameter.starts_with("slot_") || m.parameter.starts_with("pad_"))
        );
    }
    for (name, family) in [
        ("two-slots", "slots"),
        ("two-pads", "pads"),
        ("two-pockets", "pockets"),
    ] {
        let doc = document(name);
        assert_eq!(
            doc.records.iter().filter(|r| r.family == family).count(),
            2,
            "{name}: {family}"
        );
    }
    // The two-channel positive case is deliberately not claimed by this partial test:
    // quiddity 7cb26d8 reports one channel. examples/recognition_document.rs and the two
    // channel fixtures retain a reproduction; ISSUE-958-a stays pending in the catalogue.
}
