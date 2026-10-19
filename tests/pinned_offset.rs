//! F5 (#12): a pinned callout placed by its bend's offset from the automatic leader tip, which
//! can sit inside the view. The editor's sample plate (draftwright-editor public/samples).
use draftwright::dims::Placed;
use draftwright::items::{Op, key};
use draftwright::model::{Problem, Spec};
use draftwright::{Session, api};
use serde_json::json;
use std::path::Path;

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/support.rs"]
#[macro_use]
mod support;

const ID: &str = "d:f:hole_pattern:1/bore.diameter";

fn session() -> Session {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/samples/editor-plate.step");
    Session::open(&path, api::Features::Native, Spec::default()).unwrap()
}

fn ink(s: &Session) -> Placed {
    s.plan()
        .dimensions
        .iter()
        .find(|d| d.id == ID)
        .unwrap()
        .placed
        .clone()
        .unwrap()
}

fn pin(s: &mut Session, offset: [f64; 2]) {
    s.apply(
        &[Op::set(
            format!("dimensions/{}/place", key(ID)),
            json!({"offset": offset, "pinned": true}),
        )],
        true,
    )
    .unwrap();
}

fn against(s: &Session) -> Vec<&Problem> {
    s.plan()
        .problems
        .iter()
        .filter(|p| p.subjects.first().map(String::as_str) == Some(ID))
        .collect()
}

/// The plate's projected outline in the plan view: 100 × 60 about the view's origin.
fn outline(s: &Session) -> [f64; 4] {
    let v = s.plan().views.iter().find(|v| v.id == "v:plan").unwrap();
    [
        v.at[0] - 50.0 * v.scale,
        v.at[1] - 30.0 * v.scale,
        v.at[0] + 50.0 * v.scale,
        v.at[1] + 30.0 * v.scale,
    ]
}

fn inside(p: [f64; 2], o: [f64; 4]) -> bool {
    o[0] < p[0] && p[0] < o[2] && o[1] < p[1] && p[1] < o[3]
}

#[test]
fn pinning_the_automatic_offset_keeps_bend_and_label() {
    let mut s = session();
    let before = ink(&s);
    let at = s.callout_offset(ID).unwrap();
    assert_eq!(at.view, "v:plan");
    assert_eq!([at.tip, at.bend], before.lines[0]);
    // The automatic bend lies inside the view (#12's report: a bare side/lane pin cannot).
    assert!(inside(at.bend, outline(&s)), "{at:?}");
    // Through JSON, as the editor sends it.
    let offset: [f64; 2] =
        serde_json::from_str(&serde_json::to_string(&at.offset).unwrap()).unwrap();
    pin(&mut s, offset);
    assert_eq!(ink(&s), before);
    assert_eq!(s.callout_offset(ID).unwrap(), at);
    assert!(
        against(&s).is_empty(),
        "{:?}",
        against(&s).iter().map(|p| &p.code).collect::<Vec<_>>()
    );
    // The embedded Spec keeps the intent, and re-planning it reproduces the ink.
    let spec = s.plan().spec.clone();
    assert_eq!(
        spec.dimensions[ID].place.as_ref().unwrap().offset,
        Some(offset)
    );
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/samples/editor-plate.step");
    let again = Session::open(&path, api::Features::Native, spec).unwrap();
    assert_eq!(ink(&again), before);
}

#[test]
fn an_offset_into_open_space_inside_the_view_places_the_label_there() {
    let mut s = session();
    let o = outline(&s);
    let tip = s.callout_offset(ID).unwrap().tip;
    // Open plate below the ⌀8 bore, between the pattern's lower middle and right holes.
    let centre = [0.5 * (o[0] + o[2]), 0.5 * (o[1] + o[3])];
    let bend = [centre[0] + 10.6, centre[1] - 24.2];
    pin(&mut s, [bend[0] - tip[0], bend[1] - tip[1]]);
    let p = ink(&s);
    let [leader_tip, at] = p.lines[0];
    assert!((at[0] - bend[0]).abs() < 1e-9 && (at[1] - bend[1]).abs() < 1e-9);
    assert!(inside(at, o), "{p:?}");
    assert!(inside([p.label_box[0], p.label_box[1]], o), "{p:?}");
    assert!(inside([p.label_box[2], p.label_box[3]], o), "{p:?}");
    // The leader runs right, so the text reads to the right of the bend.
    assert!(p.label_box[0] > at[0], "{p:?}");
    // The leader still ends on the hole's rim, radially: the arrow points at its tip.
    assert_eq!(p.arrows[0][0], leader_tip);
    let rim = (leader_tip[0] - tip[0]).hypot(leader_tip[1] - tip[1]);
    assert!(rim < 6.6, "the tip stays on the same hole: {p:?}");
    assert_eq!(
        s.callout_offset(ID).unwrap().bend,
        at,
        "the editor reads the bend it placed"
    );
    assert!(
        against(&s).is_empty(),
        "{:?}",
        against(&s)
            .iter()
            .map(|p| (&p.code, &p.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_offset_over_the_view_geometry_is_kept_and_reported() {
    let mut s = session();
    let o = outline(&s);
    let tip = s.callout_offset(ID).unwrap().tip;
    // On the ⌀8 bore at the plate's centre.
    let bend = [0.5 * (o[0] + o[2]), 0.5 * (o[1] + o[3]) - 0.3];
    pin(&mut s, [bend[0] - tip[0], bend[1] - tip[1]]);
    let p = ink(&s);
    assert!((p.lines[0][1][0] - bend[0]).abs() < 1e-9);
    assert!((p.lines[0][1][1] - bend[1]).abs() < 1e-9);
    let codes: Vec<&str> = against(&s).iter().map(|p| p.code.as_str()).collect();
    assert!(codes.contains(&"leader_through_feature"), "{codes:?}");
    let crossing = against(&s)
        .into_iter()
        .find(|p| p.code == "leader_through_feature")
        .unwrap();
    assert!(crossing.message.contains("kept"), "{}", crossing.message);
    assert_eq!(crossing.severity, "warning");
}

#[test]
fn offset_is_a_pinned_callout_form_of_its_own() {
    let mut s = session();
    let path = format!("dimensions/{}/place", key(ID));
    for place in [
        json!({"offset":[5,5]}),
        json!({"offset":[5,5],"side":"above","pinned":true}),
        json!({"offset":[5,5],"lane":1,"pinned":true}),
        json!({"offset":[5,1e6],"pinned":true}),
        json!({"offset":[5],"pinned":true}),
    ] {
        assert!(
            s.apply(&[Op::set(&path, place.clone())], false).is_err(),
            "{place}"
        );
    }
    assert!(s.callout_offset("d:envelope/width.length").is_err());
    // Options advertise both supported free placement forms.
    let fields = s.options(ID).unwrap().settable;
    assert!(fields.contains_key(&format!("{path}/offset")));
    let fields = s.options("d:envelope/width.length").unwrap().settable;
    assert!(fields.keys().any(|k| k.ends_with("/place/offset")));
}

// Every callout drawn with a leader on several parts, pinned at the offset callout_offset
// reports: its ink is the automatic ink, the whole plan is the bare pin's, and the reported
// offset is unchanged. One callout per part is then dragged, and its bend lands where it was
// put. GRM-03's views move after packing, the frame the review found the reported tip in.
part_cases!(pinning_every_callout_at_its_reported_offset_moves_nothing;
    editor_plate => "editor-plate",
    grm03 => "gramel/GRM-03_thumbwheel_drive_screw.step",
    cgb203 => "cadgenbench_inputs/cgb203.step",
    ftc06 => "nist/nist_ftc_06_asme1_rd.stp",
);
fn pinning_every_callout_at_its_reported_offset_moves_nothing(file: &str) {
    let Some(dir) = corpus::dir() else {
        return;
    };
    let part = if file == "editor-plate" {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/samples/editor-plate.step")
    } else {
        dir.join(file)
    };
    let open = |part: &Path| Session::open(part, api::Features::Native, Spec::default()).unwrap();
    let placed = |s: &Session, id: &str| -> Placed {
        s.plan()
            .dimensions
            .iter()
            .find(|d| d.id == id)
            .and_then(|d| d.placed.clone())
            .unwrap()
    };
    let set = |s: &mut Session, id: &str, place: serde_json::Value| {
        s.apply(
            &[Op::set(format!("dimensions/{}/place", key(id)), place)],
            true,
        )
        .unwrap();
    };
    {
        let part = &part;
        let base = open(part);
        let ids: Vec<String> = base
            .plan()
            .dimensions
            .iter()
            .filter(|d| d.kind == "callout" && d.placed.is_some())
            .map(|d| d.id.clone())
            .collect();
        assert!(!ids.is_empty(), "{part:?}");
        let jobs: Vec<_> = ids.iter().enumerate().collect();
        support::parallel(&jobs, |&(n, id)| {
            let at = base.callout_offset(id).unwrap();
            assert_eq!(at.bend, placed(&base, id).lines[0][1], "{part:?} {id}");
            let mut s = open(part);
            set(&mut s, id, json!({"offset": at.offset, "pinned": true}));
            assert_eq!(placed(&s, id), placed(&base, id), "{part:?} {id} jumped");
            assert_eq!(s.callout_offset(id).unwrap(), at, "{part:?} {id}");
            let mut bare = open(part);
            set(&mut bare, id, json!({"pinned": true}));
            assert_eq!(
                s.plan().dimensions,
                bare.plan().dimensions,
                "{part:?} {id}: an offset at the automatic bend is a bare pin"
            );
            if n == 0 {
                let drag = [at.offset[0] + 2.0, at.offset[1] + 1.5];
                set(&mut s, id, json!({"offset": drag, "pinned": true}));
                let bend = placed(&s, id).lines[0][1];
                let want = [at.bend[0] + 2.0, at.bend[1] + 1.5];
                assert!(
                    (bend[0] - want[0]).abs() < 1e-6 && (bend[1] - want[1]).abs() < 1e-6,
                    "{part:?} {id}: bend {bend:?}, put at {want:?}"
                );
                assert_eq!(s.callout_offset(id).unwrap().offset, drag, "{part:?} {id}");
            }
        });
    }
}
