//! Native regressions for text association and collision-driven lane promotion.
use draftwright::model::{self, Spec};
#[path = "common/corpus.rs"]
mod corpus;

fn plan(path: &str) -> Option<model::Plan> {
    let a = model::analyse_native(&corpus::dir()?.join(path)).unwrap().1;
    Some(model::plan_recovered(&a, &Spec::default()).unwrap())
}

fn adjacent(d: &draftwright::dims::PlannedDimension) -> bool {
    let p = d.placed.as_ref().unwrap();
    let k = usize::from(p.rotation != 0.0);
    let lo = p.arrows[0][0][k].min(p.arrows[1][0][k]);
    let hi = p.arrows[0][0][k].max(p.arrows[1][0][k]);
    let b = p.label_box;
    (b[k] >= lo - 1e-6 && b[k + 2] <= hi + 1e-6)
        || (b[k + 2] <= lo && lo - b[k + 2] < 5.7 - 1e-6)
        || (b[k] >= hi && b[k] - hi < 5.7 - 1e-6)
}

#[test]
fn short_native_dimensions_keep_their_text_on_their_span() {
    for (name, labels) in [
        ("mfcadpp/1000.step", vec!["6.9", "8.9", "10.9"]),
        ("mfcadpp/10020.step", vec!["4.5", "1.2"]),
        ("mfcadpp/10007.step", vec!["14.0"]),
        ("gramel/string_post.step", vec!["1.1"]),
    ] {
        let Some(p) = plan(name) else {
            eprintln!("SKIPPED: the corpus is not found");
            return;
        };
        for label in labels {
            let ds: Vec<_> = p
                .dimensions
                .iter()
                .filter(|d| d.kind == "linear" && d.label == label && d.placed.is_some())
                .collect();
            assert!(!ds.is_empty(), "{name}: missing {label}");
            assert!(
                ds.iter().all(|d| adjacent(d)),
                "{name}: {label} away from its own span"
            );
        }
    }
}

#[test]
fn stc09_slot_length_stays_beside_its_peer_and_gdt_corridors() {
    use std::io::Read as _;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes =
        std::fs::read(root.join("tests/fixtures/pmi/step/nist_stc_09_asme1_ap242-e4.stp.gz"))
            .unwrap();
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut step)
        .unwrap();
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("label_lanes");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stc09.stp");
    std::fs::write(&path, step).unwrap();
    let a = model::analyse_native(&path).unwrap().1;
    let p = model::plan_recovered(&a, &Spec::default()).unwrap();
    let placed = |id: &str| {
        p.dimensions
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .placed
            .as_ref()
            .unwrap()
    };
    let slot = placed("d:f:slot:3/slot_length.length");
    let peer = placed("d:f:slot:1/slot_length.length");
    assert!(
        (slot.arrows[0][0][1] - peer.arrows[0][0][1]).abs() <= 14.0 + 1e-6,
        "a slot must remain within the next lane of its neighbouring size"
    );
    assert!(
        slot.lines[..2]
            .iter()
            .all(|[a, b]| (a[1] - b[1]).abs() < 60.0),
        "the size must not gain witnesses running across the location tower"
    );
    assert_eq!(p.gdt.len(), 28);
}
