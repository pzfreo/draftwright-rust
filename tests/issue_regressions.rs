//! Issue regressions (`docs/rules/issues.yaml`) ported on the parts their Python tests build: the
//! boxes, pockets and channels of draftwright's `tests/test_issue_<n>_*.py`, made by
//! `tests/fixtures/recognition/issues/make.py` and recognised by Python quiddity 0.3.9. Each test
//! names the rule it checks and what draftwright's own run of the same part gives where the port
//! is compared with it. None needs the corpus.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::lint::section_recesses::{Grammar, grammar, recesses};
use draftwright::model::{self, Analysis, Plan, PlannedView, Spec};
use draftwright::text::Font;
use serde_json::{Value, json};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/recognition/issues")
        .join(path)
}

fn font() -> Font {
    Font::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"))
        .unwrap()
}

/// The recognition document of the issue part *name*.
fn recognition(name: &str) -> Value {
    let mut json = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture(&format!("{name}.json.gz"))).unwrap())
        .read_to_string(&mut json)
        .unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The STEP file of the issue part *name*; a gzipped one is unpacked once into the test's
/// scratch directory.
fn step(name: &str) -> PathBuf {
    let plain = fixture(&format!("{name}.step"));
    if plain.exists() {
        return plain;
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.step"));
    static UNPACK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = UNPACK.lock().unwrap();
    if !out.exists() {
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(
            std::fs::File::open(fixture(&format!("{name}.step.gz"))).unwrap(),
        )
        .read_to_end(&mut bytes)
        .unwrap();
        // Written whole then renamed, so another test binary never reads half a file.
        let partial = out.with_extension(format!("step.{}", std::process::id()));
        std::fs::write(&partial, bytes).unwrap();
        std::fs::rename(&partial, &out).unwrap();
    }
    out
}

/// The issue part *name*, recognised with *document* (its own unless changed).
fn read_with(name: &str, document: &Value) -> (draftwright::haecceity::Part, Analysis) {
    let (part, mut a) = model::analyse(&step(name)).unwrap();
    model::recognise(&part, &mut a, &document.to_string()).unwrap();
    (part, a)
}

fn analysed(name: &str) -> (draftwright::haecceity::Part, Analysis) {
    read_with(name, &recognition(name))
}

fn planned(name: &str, spec: &Spec) -> Plan {
    model::plan(&analysed(name).1, spec).unwrap()
}

/// A record broken as one of draftwright's tests breaks it: the reason the reader gives, and the
/// change to the record's geometry.
type Breakage = (&'static str, fn(&mut Value));

fn spec(value: Value) -> Spec {
    serde_json::from_value(value).unwrap()
}

/// Where world point *p* lands on the page in view *v* (the centre of the part's box lands on
/// `at`; the page's x is `up × direction`).
fn page_of(v: &PlannedView, analysis: &Analysis, p: [f64; 3]) -> [f64; 2] {
    let [lo, hi] = analysis.envelope;
    let (t, u) = (v.direction, v.up);
    let right = [
        u[1] * t[2] - u[2] * t[1],
        u[2] * t[0] - u[0] * t[2],
        u[0] * t[1] - u[1] * t[0],
    ];
    let q = [0, 1, 2].map(|k| p[k] - 0.5 * (lo[k] + hi[k]));
    let dot = |a: [f64; 3]| a[0] * q[0] + a[1] * q[1] + a[2] * q[2];
    [v.at[0] + v.scale * dot(right), v.at[1] + v.scale * dot(u)]
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] - 1e-9 && a[2] > b[0] + 1e-9 && a[1] < b[3] - 1e-9 && a[3] > b[1] + 1e-9
}

/// The recognition document rounds a record's coordinates to 0.001 mm in quiddity's working
/// frame, whose origin is off that grid on these parts (pocket_square's lies at -0.5634 on every
/// axis), so a pocket read back in the part's frame is up to 0.0005 mm from draftwright's, which
/// reads the shape itself.
const DOCUMENT: f64 = 1e-3;

/// A pocket as draftwright's own run of the part has it (its `PocketFeature`): the centre of its
/// frame, the axis it is cut along, its width and length axes and sizes, and its callout.
struct Pocket {
    part: &'static str,
    centre: [f64; 3],
    depth_axis: usize,
    width: (usize, f64),
    long: (usize, f64),
    label: &'static str,
}

const POCKETS: [Pocket; 6] = [
    Pocket {
        part: "pocket_square",
        centre: [10.0, 10.0, 10.0],
        depth_axis: 2,
        width: (0, 20.0),
        long: (1, 20.0),
        label: "20 × 20 × 10 DEEP",
    },
    Pocket {
        part: "pocket_z",
        centre: [10.0, 10.0, 10.0],
        depth_axis: 2,
        width: (1, 14.0),
        long: (0, 22.0),
        label: "14 × 22 × 10 DEEP",
    },
    Pocket {
        part: "pocket_x",
        centre: [10.0, 10.0, -10.0],
        depth_axis: 0,
        width: (1, 14.0),
        long: (2, 22.0),
        label: "14 × 22 × 10 DEEP",
    },
    Pocket {
        part: "pocket_y",
        centre: [10.0, -10.0, 10.0],
        depth_axis: 1,
        width: (2, 14.0),
        long: (0, 22.0),
        label: "14 × 22 × 10 DEEP",
    },
    Pocket {
        part: "curved_centered",
        centre: [0.0, 0.0, 14.0],
        depth_axis: 2,
        width: (0, 6.0),
        long: (1, 24.0),
        label: "6 × 24 ×",
    },
    Pocket {
        part: "curved_rotated",
        centre: [25.0, 22.0, 33.0],
        depth_axis: 0,
        width: (2, 6.0),
        long: (1, 24.0),
        label: "6 × 24 ×",
    },
];

/// ISSUE-916-a: a pocket's callout leader starts on the pocket's rim, at the end of the axis
/// matching a size it states, and leaves the part cleanly, whichever way the pocket is cut. On
/// draftwright's blind pocket (square, and 22 × 14 cut along z, x and y) and two pockets with a
/// cylindrical mouth, the callout is in the view that looks into the pocket and its arrow tip
/// is the midpoint of one of the rim's sides: on the width's or the length's centre line, at half
/// the other size from the centre (draftwright's `_tip_is_on_rim`, and its axis-matching case
/// where the tip lies on the centre line). Leaving cleanly: no plan problem but
/// `annotation_outside_area` (info) names the callout, and the sheet lint
/// (`lint::structural`) finds nothing on the drawing; a leader taken back across its own view
/// would be reported (`annotation_line_over_view`), so the leader leaves the part's outline once.
/// On pocket_square draftwright's tip is at (75, 140) and the port's at (74.9996, 139.9996).
/// Differences: draftwright accepts a tip anywhere along a side, the port takes a side's midpoint
/// (equivalent: both on the rim and on the matching axis); its third case forces the exit across
/// the width by planting notes in the other corridors, which the port's spec cannot place (each
/// exit is seen here on the parts as they come); and the callout of a pocket with a cylindrical
/// mouth prints `12.0`, draftwright's `12` (`a_cylindrical_mouth_states_its_greatest_depth`).
#[test]
fn a_pocket_leader_starts_on_its_rim_on_the_matching_axis() {
    let view_of = ["v:side", "v:front", "v:plan"];
    let mut axes_used = [false; 2];
    for p in &POCKETS {
        let (part, analysis) = analysed(p.part);
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        let callout = plan
            .dimensions
            .iter()
            .find(|d| {
                d.id.ends_with(&format!(
                    "/pocket_width.length@{}",
                    &view_of[p.depth_axis][2..]
                ))
            })
            .unwrap_or_else(|| panic!("{}: no pocket callout", p.part));
        assert_eq!(callout.view, view_of[p.depth_axis], "{}", p.part);
        // A curved pocket's depth is checked in a_cylindrical_mouth_states_its_greatest_depth.
        if p.label.ends_with('×') {
            assert!(callout.label.starts_with(p.label), "{}", p.part);
        } else {
            assert_eq!(callout.label, p.label, "{}", p.part);
        }
        let v = plan.views.iter().find(|v| v.id == callout.view).unwrap();
        let tip = callout.placed.as_ref().unwrap().arrows[0][0];
        let mut on_side = None;
        for (i, (k, size)) in [p.width, p.long].into_iter().enumerate() {
            for s in [-1.0, 1.0] {
                let mut end = p.centre;
                end[k] += s * 0.5 * size;
                let at = page_of(v, &analysis, end);
                if (tip[0] - at[0]).hypot(tip[1] - at[1]) < DOCUMENT * v.scale {
                    on_side = Some(i);
                }
            }
        }
        let Some(i) = on_side else {
            panic!("{}: the tip {tip:?} is not on the rim", p.part);
        };
        axes_used[i] = true;
        for q in &plan.problems {
            assert!(
                !q.subjects.contains(&callout.id) || q.code == "annotation_outside_area",
                "{}: {} {:?}",
                p.part,
                q.code,
                q.subjects
            );
        }
        let drawing = model::render(&part, &analysis, &plan, &font()).unwrap();
        let sheet = draftwright::lint::structural::check(&drawing);
        assert!(sheet.is_empty(), "{}: {sheet:#?}", p.part);
    }
    // Both kinds of exit occur: from a side across the width and from an end of the length.
    assert_eq!(axes_used, [true, true]);
}

/// ISSUE-1485-a: a pocket whose mouth is a cylinder owns its greatest depth, not a uniform one,
/// and a record whose mouth and floor disagree is refused. On draftwright's pocket cut into an
/// x cylinder of radius 20 (centred, offset off the crest, turned over, and moved and turned to
/// stand along z), the one callout claims `pocket_max_depth.length` and never
/// `pocket_depth.length`, prints `MAX DEEP`, and states draftwright's depth: 12, or √364 − 8 on
/// the offset pocket, whose floor is lower than the mouth's crest. Five of draftwright's broken
/// records (a floor through the mouth, a mouth off its published centroid intersection, an end
/// of no known surface, two cylindrical ends, a mouth opening towards its floor) are refused by
/// the reader with draftwright's reasons, and the plan states nothing for them and reports the
/// recess (an info `dimensions_not_ported`, whose message, that no section recess is dimensioned
/// yet, is out of date since dims::recesses). Not ported here: a planar substitution that cannot certify it (lint's coverage, owned
/// by the stream porting it), and repeated curved pockets as one `N×` group (quiddity's
/// `section_recess_patterns` are not read). Difference: the centred depth prints `12.0` where
/// draftwright prints `12`. The document rounds the floor's run interval to 0.001 mm but gives
/// the mouth's axis to the micrometre, so the depth reads 12.000483 (rust-wrong in what is
/// printed, from the input: draftwright's own run reads the shape exactly).
#[test]
fn a_cylindrical_mouth_states_its_greatest_depth() {
    let offset = 364f64.sqrt() - 8.0;
    for (part, depth, length) in [
        ("curved_centered", 12.0, 24.0),
        ("curved_offset", offset, 6.0),
        ("curved_negative", 12.0, 24.0),
        ("curved_rotated", 12.0, 24.0),
    ] {
        let plan = planned(part, &Spec::default());
        let mut callouts = plan
            .dimensions
            .iter()
            .filter(|d| d.id.contains("section_recess") && d.kind == "callout");
        let callout = callouts.next().unwrap();
        assert!(callouts.next().is_none(), "{part}");
        let claims: Vec<(&str, f64)> = callout
            .measurements
            .iter()
            .map(|m| (m.parameter.as_str(), m.value))
            .collect();
        assert_eq!(claims[0], ("pocket_width.length", 6.0), "{part}");
        assert_eq!(claims[1].0, "pocket_length.length", "{part}");
        assert!((claims[1].1 - length).abs() < 1e-9, "{part}");
        assert_eq!(claims[2].0, "pocket_max_depth.length", "{part}");
        assert!(
            (claims[2].1 - depth).abs() < DOCUMENT,
            "{part}: {}",
            claims[2].1
        );
        assert!(
            plan.dimensions
                .iter()
                .flat_map(|d| &d.measurements)
                .all(|m| m.parameter != "pocket_depth.length"),
            "{part}"
        );
        let printed = draftwright::dims::fmt(claims[2].1);
        assert_eq!(
            callout.label,
            format!(
                "6 × {} × {printed} MAX DEEP",
                draftwright::dims::fmt(length)
            ),
            "{part}"
        );
    }
    assert_eq!(
        planned("curved_offset", &Spec::default())
            .dimensions
            .iter()
            .find(|d| d.id.contains("section_recess") && d.kind == "callout")
            .unwrap()
            .label,
        "6 × 6 × 11.1 MAX DEEP"
    );

    let base = recognition("curved_centered");
    let broken: [Breakage; 5] = [
        ("positive depth over its complete profile", |g| {
            // The floor halfway up the mouth's height range over the section (draftwright's
            // test): the mouth's crest is at 20 above its axis, its lowest point (at the
            // section's ends, 12 off the crest) at 16.
            let axis = g["ends"]["high"]["surface"]["axis_point"][2]
                .as_f64()
                .unwrap();
            g["run_interval"][0] = json!(axis + 18.0);
        }),
        ("published centroid intersection", |g| {
            let high = g["run_interval"][1].as_f64().unwrap();
            g["run_interval"][1] = json!(high + 0.5);
        }),
        ("known surface type", |g| {
            g["ends"]["low"]["surface"] = json!({ "type": "sphere" });
        }),
        ("one cylindrical end", |g| {
            g["ends"]["low"]["surface"] = g["ends"]["high"]["surface"].clone();
        }),
        ("open away from its planar floor", |g| {
            g["ends"]["high"]["surface"]["branch"] = json!("negative");
        }),
    ];
    for (reason, change) in broken {
        let mut doc = base.clone();
        let features = doc["quiddity"]["features"].as_array_mut().unwrap();
        let f = features
            .iter_mut()
            .find(|f| f["family"] == "section_recesses")
            .unwrap();
        change(&mut f["record"]["geometry"]);
        let (_, analysis) = read_with("curved_centered", &doc);
        let recess = recesses(&analysis)[0];
        match grammar(recess) {
            Err(why) => assert!(why.reason().contains(reason), "{reason}: {}", why.reason()),
            Ok(_) => panic!("{reason}: the broken record is read"),
        }
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        assert!(
            plan.dimensions
                .iter()
                .flat_map(|d| &d.measurements)
                .all(|m| m.feature != recess.id),
            "{reason}"
        );
        assert!(
            plan.problems
                .iter()
                .any(|q| q.subjects == [recess.id.clone()]),
            "{reason}: the refused recess is not reported"
        );
    }
}

/// ISSUE-917-a, in part: an open U-channel is one feature with one width, and neither a bounded
/// pocket nor a centred rebate is a channel. On draftwright's base-and-walls channel (walls 12.5
/// and 12.5, and 10 and 15), pierced by a counterbored hole and a cross hole, the plan states one
/// channel width, 25, in the view along the channel, and the overall width and depth, 50 and 50,
/// as draftwright does; the holes' callouts are draftwright's (`2× ⌀8 THRU`, `⌀4 THRU ⌴ ⌀12 ↧ 4`).
/// A pocket bounded on all sides is dimensioned as a pocket (`20 × 30 × 8 DEEP`, no channel
/// width), and a rebate centred across a block is left to the height ladder (shoulders 20 and 40,
/// as draftwright's sheet), its recess reported, not dimensioned. Not ported: the independent
/// wall thickness (draftwright's `12.5`, or `10` with the opposite wall derived) and the base's
/// `12`: plate thickness is not dimensioned, and each plate is reported `dimensions_not_ported`.
#[test]
fn an_open_channel_is_one_feature_with_one_width() {
    let labels = |plan: &Plan, part: &str| -> Vec<String> {
        plan.dimensions
            .iter()
            .filter(|d| d.id.contains(part))
            .map(|d| d.label.clone())
            .collect()
    };
    for part in ["u_channel", "u_channel_asymmetric"] {
        let plan = planned(part, &Spec::default());
        let widths: Vec<_> = plan
            .dimensions
            .iter()
            .filter(|d| d.id.contains("/channel_width.length@"))
            .collect();
        assert_eq!(widths.len(), 1, "{part}");
        assert_eq!(widths[0].label, "25", "{part}");
        assert!(widths[0].placed.is_some(), "{part}");
        assert_eq!(labels(&plan, "d:envelope/width"), ["50"], "{part}");
        assert_eq!(labels(&plan, "d:envelope/depth"), ["50"], "{part}");
        let callouts: Vec<&str> = plan
            .dimensions
            .iter()
            .filter(|d| {
                d.id.ends_with("/bore.diameter@front") || d.id.ends_with("/bore.diameter@plan")
            })
            .map(|d| d.label.as_str())
            .collect();
        assert_eq!(callouts, ["2× ⌀8 THRU", "⌀4 THRU ⌴ ⌀12 ↧ 4"], "{part}");
        assert!(
            plan.dimensions
                .iter()
                .all(|d| !d.id.contains("/pocket_") && !d.id.contains("plate")),
            "{part}"
        );
        let plates: Vec<_> = plan
            .problems
            .iter()
            .filter(|q| q.code == "dimensions_not_ported" && q.subjects[0].starts_with("f:plate:"))
            .collect();
        assert_eq!(plates.len(), 3, "{part}");
    }

    let pocket = planned("bounded_pocket", &Spec::default());
    assert!(pocket.dimensions.iter().all(|d| !d.id.contains("channel")));
    assert_eq!(
        labels(&pocket, "/pocket_width.length@"),
        ["20 × 30 × 8 DEEP"]
    );

    let rebate = planned("centred_rebate", &Spec::default());
    assert!(
        rebate
            .dimensions
            .iter()
            .all(|d| !d.id.contains("section_recess"))
    );
    assert_eq!(labels(&rebate, "/step_position.length."), ["20", "40"]);
    let (_, analysis) = analysed("centred_rebate");
    let recess = &recesses(&analysis)[0].id;
    assert!(
        rebate
            .problems
            .iter()
            .any(|q| q.code == "dimensions_not_ported" && &q.subjects[0] == recess)
    );
}

/// ISSUE-1593-a, in part: no dimension is drawn into the title block, and one the block displaces
/// is still drawn, never discarded. On draftwright's four cases (a 40 × 30 × 12 box, an
/// 80 × 60 × 30 box, the same stepped, a bored plate), on the automatic sheet and on the sheet
/// draftwright's defect was seen on (A4 at 2:1 for the small box, 1:1 for the others), no placed
/// dimension's label, lines or arrowheads reach into the block, the overall depth is drawn with
/// draftwright's label, and no problem is an error. Where there is no room at all (the tall box
/// pinned to A4 at 2:1, whose side view runs off the sheet), the depth keeps its place and is
/// reported (`annotation_over_title_block`, `annotation_off_sheet`), not dropped. Not ported:
/// draftwright withholds such a dimension as an error naming the block and re-plans onto a
/// sheet that fits (its recovery ladder, ISSUE-1299-a); the port reports it where it stands.
/// Difference, undetermined: on the stepped box the port's first fit is A3 at 1:1, draftwright's
/// sheet A4 at 1:1 (the port's layout reserves the views' bands before placing; draftwright fits
/// the dimensions into A4's whitespace after).
#[test]
fn the_title_block_keeps_dimensions_out_without_dropping_them() {
    for (part, defect, depth) in [
        ("tb_plain_box", "2:1", "30"),
        ("tb_tall_box", "1:1", "60"),
        ("tb_stepped", "1:1", "60"),
        ("tb_bored_plate", "1:1", "60"),
    ] {
        let (_, analysis) = analysed(part);
        for asked in [
            Spec::default(),
            spec(json!({ "sheet": { "size": "A4", "scale": defect } })),
        ] {
            let plan = model::plan(&analysis, &asked).unwrap();
            let tb = plan.sheet.title_block;
            let within = |p: [f64; 2]| tb[0] < p[0] && p[0] < tb[2] && tb[1] < p[1] && p[1] < tb[3];
            for d in &plan.dimensions {
                let p = d.placed.as_ref().unwrap();
                let ink = overlap(p.label_box, tb)
                    || p.lines
                        .iter()
                        .chain(&p.arrows)
                        .flatten()
                        .any(|q| within(*q));
                assert!(
                    !ink,
                    "{part} {}: {} is in the title block",
                    plan.sheet.scale, d.id
                );
            }
            let d = plan
                .dimensions
                .iter()
                .find(|d| d.id == "d:envelope/depth.length@side")
                .unwrap();
            assert_eq!(d.label, depth, "{part}");
            assert!(
                plan.problems.iter().all(|q| q.severity != "error"),
                "{part} {}: {:?}",
                plan.sheet.scale,
                plan.problems
            );
        }
    }
    assert_eq!(planned("tb_stepped", &Spec::default()).sheet.size, "A3");
    let crowded = planned(
        "tb_tall_box",
        &spec(json!({ "sheet": { "size": "A4", "scale": "2:1" } })),
    );
    let depth = "d:envelope/depth.length@side";
    assert!(
        crowded
            .dimensions
            .iter()
            .any(|d| d.id == depth && d.placed.is_some())
    );
    for code in ["annotation_over_title_block", "annotation_off_sheet"] {
        assert!(
            crowded
                .problems
                .iter()
                .any(|q| q.code == code && q.subjects[0] == depth),
            "{code}"
        );
    }
}

/// The section recesses of *analysis* that the reader takes as hexagonal pockets: each one's
/// axis and mouth centre in the part's frame, depth and across-flats size.
fn hexes(analysis: &Analysis) -> Vec<(String, usize, f64, f64, [f64; 3])> {
    draftwright::lint::pockets::hexes(analysis)
        .into_iter()
        .map(|h| {
            let (axis, at) = h.placed.unwrap();
            (h.feature.id.clone(), axis, h.depth, h.across_flats, at)
        })
        .collect()
}

/// ISSUE-1598-a, in part: a hex nut pocket is taken only from a regular-hex section, and counted
/// hexes keep distinct claims. On draftwright's whistle frame, as in its test, six section
/// recesses read as hexagonal pockets along z, 4.3 across flats and 1.9 deep, their mouths on the
/// part's bottom face (z = 0), within the document's 0.001 mm; the plan states them in one
/// callout, `6× HEX 4.3 A/F × 1.9 DEEP` (draftwright's label on the same part), claiming each
/// pocket's own across-flats size and depth (twelve claims, two per pocket). A pocket record whose
/// six vertices are not a regular hexagon (one moved 0.05 mm, one pulled to the centre, one
/// repeated, two swapped), that has five vertices or no depth is not read as a hex pocket, with
/// draftwright's reasons, and the plan states nothing for it. Not ported: declared hex pockets
/// (no declarations, R4 decision 3) and the label of a pocket of which only one size is approved
/// (the port's spec cannot approve one term of a callout); the rank of hex callouts over
/// incidental flats in leader placement.
#[test]
fn a_hex_pocket_needs_a_regular_hexagon_and_keeps_each_claim() {
    let name = "whistle_frame_reference";
    let (_, analysis) = analysed(name);
    let found = hexes(&analysis);
    assert_eq!(found.len(), 6);
    for (id, axis, depth, across, at) in &found {
        assert_eq!(*axis, 2, "{id}");
        assert!((depth - 1.9).abs() < DOCUMENT, "{id}: {depth}");
        assert!((across - 4.3).abs() < DOCUMENT, "{id}: {across}");
        assert!(at[2].abs() < DOCUMENT, "{id}: {at:?}");
    }
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    let callouts: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("/polygon_across_flats.length@"))
        .collect();
    assert_eq!(callouts.len(), 1);
    assert_eq!(callouts[0].label, "6× HEX 4.3 A/F × 1.9 DEEP");
    let claims: std::collections::BTreeSet<(&str, &str)> = callouts[0]
        .measurements
        .iter()
        .map(|m| (m.feature.as_str(), m.parameter.as_str()))
        .collect();
    assert_eq!(claims.len(), 12);
    for (id, ..) in &found {
        for parameter in ["polygon_across_flats.length", "pocket_depth.length"] {
            assert!(
                claims.contains(&(id.as_str(), parameter)),
                "{id} {parameter}"
            );
        }
    }

    let base = recognition(name);
    let hex_index = base["quiddity"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .position(|f| f["record"]["classification"]["section_shape"] == "hexagonal")
        .unwrap();
    let broken: [Breakage; 6] = [
        ("not regular", |g| {
            let x = g["profile"]["boundary"][2]["point"][0].as_f64().unwrap();
            g["profile"]["boundary"][2]["point"][0] = json!(x + 0.05);
        }),
        ("convex ordered section", |g| {
            g["profile"]["boundary"][2]["point"] = json!([0.0, 0.0]);
        }),
        ("convex ordered section", |g| {
            g["profile"]["boundary"][2] = g["profile"]["boundary"][1].clone();
        }),
        ("convex ordered section", |g| {
            g["profile"]["boundary"].as_array_mut().unwrap().swap(2, 3);
        }),
        ("six straight physical walls", |g| {
            g["profile"]["boundary"].as_array_mut().unwrap().pop();
        }),
        ("depth must be positive", |g| {
            let low = g["run_interval"][0].clone();
            g["run_interval"][1] = low;
        }),
    ];
    for (reason, change) in broken {
        let mut doc = base.clone();
        change(&mut doc["quiddity"]["features"][hex_index]["record"]["geometry"]);
        let (_, analysis) = read_with(name, &doc);
        assert_eq!(hexes(&analysis).len(), 5, "{reason}");
        let refused: Vec<_> = recesses(&analysis)
            .into_iter()
            .filter_map(|f| grammar(f).err().map(|why| (f, why)))
            .filter(|(_, why)| why.reason().contains(reason))
            .collect();
        assert_eq!(refused.len(), 1, "{reason}");
        let plan = model::plan(&analysis, &Spec::default()).unwrap();
        let id = &refused[0].0.id;
        assert!(
            plan.dimensions
                .iter()
                .flat_map(|d| &d.measurements)
                .all(|m| &m.feature != id),
            "{reason}"
        );
        assert!(
            plan.dimensions
                .iter()
                .any(|d| d.label == "5× HEX 4.3 A/F × 1.9 DEEP"),
            "{reason}"
        );
    }
}

/// ISSUE-1613-a, in part: a circular seat keeps its arc and its full set of size and location
/// requirements. On draftwright's whistle frame, as in its tests, three section recesses read as
/// circular seats along y: ⌀14.3 (to 0.0003 mm), 6 long, a 62.89° arc (to 0.0001°, a minor arc,
/// not a rectangular channel), their axis 11.1 above the bottom (to 0.0003 mm). The plan states
/// six requirements for each seat and nothing twice: one callout `3× ø14.3 × 6 LONG × 62.9° ARC`
/// (draftwright's label) claiming each seat's diameter, run and sweep (nine claims), and five
/// location dimensions carrying nine claims, as draftwright's: one x and one z shared by the
/// coaxial seats (`11.1`) and a y for each. Not ported: declared seats and the refusal of a
/// declared wrong radius or arc midpoint (no declarations, R4 decision 3); the seats' axis marks.
/// Difference: the shared x location prints `15.6`, draftwright's `15.7`. The seats' axis lies
/// on the part's midplane, 15.65 from the box, which draftwright reads exactly (and prints
/// 15.65 to one decimal as 15.7); the document's 0.001 mm rounding in quiddity's working frame
/// gives the port 15.6495 (rust-wrong in what is printed, from the input).
#[test]
fn a_circular_seat_keeps_its_arc_and_six_requirements() {
    let (_, analysis) = analysed("whistle_frame_reference");
    let seats: Vec<_> = recesses(&analysis)
        .into_iter()
        .filter(|f| matches!(grammar(f), Ok(Grammar::CircularChannel { .. })))
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(seats.len(), 3);
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    let callout = plan
        .dimensions
        .iter()
        .find(|d| d.id.contains("/seat_diameter.diameter@"))
        .unwrap();
    assert_eq!(callout.label, "3× ø14.3 × 6 LONG × 62.9° ARC");
    assert_eq!(callout.measurements.len(), 9);
    for m in &callout.measurements {
        let (want, within) = match m.parameter.as_str() {
            "seat_diameter.diameter" => (14.3, 3e-4),
            "seat_run.length" => (6.0, 1e-9),
            "seat_sweep.angle" => (62.8888, 1e-4),
            other => panic!("{other}"),
        };
        assert!(
            (m.value - want).abs() < within,
            "{}: {}",
            m.parameter,
            m.value
        );
    }
    let locations: Vec<_> = plan
        .dimensions
        .iter()
        .filter(|d| d.id.contains("/seat_location.location."))
        .collect();
    assert_eq!(locations.len(), 5);
    assert_eq!(
        locations
            .iter()
            .map(|d| d.measurements.len())
            .sum::<usize>(),
        9
    );
    for seat in &seats {
        let mut owned: Vec<&str> = plan
            .dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .filter(|m| &m.feature == seat)
            .map(|m| m.parameter.as_str())
            .collect();
        owned.sort();
        assert_eq!(
            owned,
            [
                "seat_diameter.diameter",
                "seat_location.location.x",
                "seat_location.location.y",
                "seat_location.location.z",
                "seat_run.length",
                "seat_sweep.angle",
            ],
            "{seat}"
        );
    }
    let z = locations
        .iter()
        .find(|d| d.id.ends_with("/seat_location.location.z@front"))
        .unwrap();
    assert_eq!((z.label.as_str(), z.measurements.len()), ("11.1", 3));
    assert!((z.measurements[0].value - 11.1).abs() < 3e-4);
    let x = locations
        .iter()
        .find(|d| d.id.contains("/seat_location.location.x@"))
        .unwrap();
    assert_eq!((x.label.as_str(), x.measurements.len()), ("15.6", 3));
}
