//! Room for the GD&T in the layout ([`draftwright::layout::Strips::reserve_gdt`],
//! [`draftwright::dims::gdt::demand`]): the bands around the views hold the feature control
//! frames, datum feature symbols and labels lowering puts in them, and `dims::gdt` places each
//! inside its view's reserved area and on the sheet when a clear place there exists. Pinned on
//! two AP242 fixtures, through `plan_recovered` as the library plans:
//!
//! - nist_ctc_02 e2: every item inside its area and on the sheet (before, datum feature symbols
//!   E and J, `pmi:datum:#75` and `#78`, stood off the sheet's left edge);
//! - nist_stc_09 e4: no datum symbol past its area, and every frame past it is one whose own
//!   strip has no clear place at all (its leader would cross a label in every tier), so it
//!   stands in a strip the layout did not reserve for it (reported, never dropped).

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::layout::{Gdt, Lanes, Strips};
use draftwright::model::{self, Analysis, Plan, Spec};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// An AP242 fixture's Analysis (`tests/fixtures/pmi/step/<stem>.*.gz`), from a scratch file of
/// its own.
fn analysed(stem: &str) -> Analysis {
    let dir = root().join("tests/fixtures/pmi/step");
    let gz = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().split('.').next() == Some(stem))
        .unwrap();
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::read(&gz).unwrap().as_slice())
        .read_to_end(&mut bytes)
        .unwrap();
    let file = gz.file_name().unwrap().to_string_lossy();
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("gdt_band")
        .join(stem);
    std::fs::create_dir_all(&out).unwrap();
    let path = out.join(file.strip_suffix(".gz").unwrap());
    std::fs::write(&path, &bytes).unwrap();
    model::analyse_native(&path).unwrap().1
}

/// The plan's GD&T items reported with *code*.
fn reported<'a>(plan: &'a Plan, code: &str) -> Vec<&'a draftwright::dims::gdt::PlannedGdt> {
    plan.gdt
        .iter()
        .filter(|g| {
            plan.problems
                .iter()
                .any(|p| p.code == code && p.subjects.first() == Some(&g.id))
        })
        .collect()
}

#[test]
fn ctc_02_frames_and_datum_symbols_are_inside_their_bands_and_on_the_sheet() {
    let a = analysed("nist_ctc_02_asme1_ap242-e2");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert_eq!(plan.gdt.len(), 10, "five frames and five datum symbols");
    assert!(plan.gdt.iter().all(|g| g.placed.is_some()));
    let off: Vec<&str> = reported(&plan, "annotation_off_sheet")
        .iter()
        .map(|g| g.id.as_str())
        .collect();
    assert!(off.is_empty(), "off the sheet: {off:?}");
    let outside: Vec<&str> = reported(&plan, "annotation_outside_area")
        .iter()
        .map(|g| g.id.as_str())
        .collect();
    assert!(outside.is_empty(), "past their areas: {outside:?}");
    // E and J stand left of the front view, inside the sheet's drawing area.
    let drawable = 10.0;
    for id in ["pmi:datum:#75", "pmi:datum:#78"] {
        let g = plan.gdt.iter().find(|g| g.id == id).unwrap();
        assert_eq!((g.view.as_str(), g.side.as_str()), ("v:front", "left"));
        let b = g.placed.as_ref().unwrap().label_box;
        assert!(b[0] >= drawable, "{id} at {b:?}");
    }
}

#[test]
fn stc_09_datum_symbols_are_inside_their_bands_and_frames_past_them_had_no_clear_strip() {
    let a = analysed("nist_stc_09_asme1_ap242-e4");
    let plan = model::plan_recovered(&a, &Spec::default()).unwrap();
    assert!(reported(&plan, "annotation_off_sheet").is_empty());
    let outside = reported(&plan, "annotation_outside_area");
    for g in &outside {
        assert_eq!(g.kind, "control_frame", "{} past its area", g.id);
        assert!(
            g.basis.contains("strip has no clear place, so it stands"),
            "{} is past its area in its own strip: {}",
            g.id,
            g.basis
        );
    }
    // 12 frames and 4 datum symbols stood past their areas before the reservation; 6 frames,
    // each moved to a strip across or beside its view, remain.
    assert_eq!(
        outside.len(),
        6,
        "{:?}",
        outside.iter().map(|g| &g.id).collect::<Vec<_>>()
    );
}

/// The reservation is the strip's dimension lanes and its items' tiers packed first fit: two
/// frames side by side share a tier, two over one another take two; an item past the view's
/// end widens the band beyond that end; a part with no GD&T reserves nothing.
#[test]
fn the_reservation_stacks_overlapping_items_in_tiers_beyond_the_lanes() {
    let frame = |at: f64| Gdt {
        view: "front",
        side: "below",
        at,
        span: [0.0, 32.0],
        ends: [-100.0, 100.0],
        depth: 6.0,
        lead: 3.7,
    };
    let lanes = Lanes {
        front_below: 2,
        ..Lanes::default()
    };
    let lane_band = draftwright::layout::lane_band(2);
    let band = |items: &[Gdt], scale: f64| {
        let mut s = Strips::without_features();
        s.reserve_gdt(&lanes, items, scale);
        s.fv_bottom
    };
    let one = lane_band + 2.0 + 6.0;
    assert_eq!(band(&[frame(-50.0)], 1.0), one);
    // 100 mm apart at 1:1, side by side; 20 mm apart at 1:5, over one another.
    let two = [frame(-50.0), frame(50.0)];
    assert_eq!(band(&two, 1.0), one);
    assert_eq!(band(&two, 0.2), one + 2.0 + 6.0);
    // A frame hung right of a leader 10 mm short of the view's right end runs 22 mm past it.
    let mut s = Strips::without_features();
    let right = s.right;
    s.reserve_gdt(&lanes, &[frame(90.0)], 1.0);
    assert_eq!(s.right, right.max(22.0));
    let mut s = Strips::without_features();
    s.reserve_gdt(&lanes, &[], 1.0);
    assert_eq!(s, Strips::without_features());
}
