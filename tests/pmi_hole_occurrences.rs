//! AP242 edition 4 hole occurrences (NIST STC-09): haecceity holds each occurrence's
//! `basic_round_hole` definition, so the dimensions and tolerances on a pattern of holes are read
//! whole and lowered; the definitions themselves, which draftwright does not read (XCAF reads
//! none), are each a source not projected, never dropped.

use std::io::Read as _;
use std::path::Path;

use draftwright::haecceity::{self, p21, pmi as hpmi, step};
use draftwright::pmi::{self, Category, Outcome};

const STC_09: &str = "tests/fixtures/pmi/step/nist_stc_09_asme1_ap242-e4.stp.gz";

fn stc_09() -> Vec<u8> {
    let raw = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(STC_09)).unwrap();
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(&raw[..])
        .read_to_end(&mut out)
        .unwrap();
    out
}

#[test]
fn each_hole_definition_is_a_source_not_projected() {
    let bytes = stc_09();
    let part = haecceity::read_step(&bytes).unwrap();
    let defs = step::read_part_definitions(&bytes).unwrap();
    let doc = p21::Document::parse(bytes.clone()).unwrap();
    let read = hpmi::read(&doc, &defs).unwrap();
    let r = pmi::report(&doc, &defs, &part, &read).unwrap();
    let holes = &read.parts[0].holes;
    assert_eq!(holes.len(), 25, "STC-09's hole occurrences");
    for (i, h) in holes.iter().enumerate() {
        let id = format!("hole:0:{i}");
        let s = r
            .sources
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("{id} ({}) lost", h.definition.name));
        assert_eq!(s.outcome, Outcome::NotExtracted, "{id}");
        assert_eq!(s.category, Category::Other, "{id}");
        assert_eq!(s.entity, "basic_round_hole_occurrence", "{id}");
        assert!(
            s.reason
                .ends_with(&format!("(hole definition {:?})", h.definition.name)),
            "{id}: {}",
            s.reason
        );
        assert_eq!(
            s.instances, read.provenance.parts[0].features[h.feature.0],
            "{id}"
        );
    }
    // No finding on a hole occurrence is left, on a record or as a source.
    assert!(
        r.sources
            .iter()
            .all(|s| !s.reason.contains("basic_round_hole")),
        "a hole occurrence is still reported"
    );
    assert!(
        r.records
            .iter()
            .all(|x| x.blockers.iter().all(|b| !b.contains("basic_round_hole"))),
        "a record still carries a hole occurrence's finding"
    );
}

#[test]
fn a_diameter_on_a_pattern_of_holes_is_read_on_every_hole() {
    let bytes = stc_09();
    let part = haecceity::read_step(&bytes).unwrap();
    let r = pmi::read(&bytes, &part).unwrap();
    // Each diameter's feature is a 'pattern of features' composite group: its 'unified
    // geometric item' (one hole) and its hole occurrences (the pattern's other holes), so it
    // applies to all of them (draftwright, reading XCAF, takes the unified item's hole alone).
    for (source, holes) in [
        ("dimension:#9866", 3),
        ("dimension:#9889", 3),
        ("dimension:#10063", 4),
        ("dimension:#10191", 3),
        ("dimension:#10215", 2),
        ("dimension:#10239", 3),
        ("dimension:#10287", 2),
        ("dimension:#10311", 4),
    ] {
        let x = r
            .records
            .iter()
            .find(|x| x.source_id == source)
            .unwrap_or_else(|| panic!("{source} not read"));
        assert!(x.blockers.is_empty(), "{source}: {:?}", x.blockers);
        let g = x.geometry.as_ref().unwrap();
        assert!(g.reasons.is_empty(), "{source}: {:?}", g.reasons);
        assert_eq!(g.cylinders.len(), holes, "{source}'s holes");
    }
}
