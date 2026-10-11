//! S4 part B: deltas are checked against a fresh plan and render after every slice edit.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::api;
use draftwright::items::{Op, key};
use draftwright::model::{self, Spec};
use draftwright::session::{Delta, ItemFilter, ProblemFilter, Session};
use draftwright::text::Font;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Instant;
#[path = "common/session_edits.rs"]
mod session_edits;
use session_edits::edits;

fn path(file: &str) -> Option<PathBuf> {
    if file == "plate" {
        let editor = Path::new("/Users/paul/repos/draftwright-editor/public/samples/plate.step");
        return Some(if editor.exists() {
            editor.into()
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step")
        });
    }
    corpus::dir().map(|c| c.join(file))
}
fn open(file: &str) -> Option<Session> {
    let (part, analysis) = model::analyse_native(&path(file)?).unwrap();
    Some(Session::new(part, analysis, Spec::default()).unwrap())
}
fn benchmark(file: &str) {
    let Some(mut s) = open(file) else { return };
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let initial_plan = serde_json::to_vec(s.plan()).unwrap();
    let initial_svg = s.drawing().svg();
    let mut replay: draftwright::session::State =
        serde_json::from_slice(&serde_json::to_vec(s.state()).unwrap()).unwrap();
    let mut times = Vec::new();
    let sequence = edits(&s);
    for ops in &sequence {
        let before = s.stats();
        let start = Instant::now();
        let delta = s.apply(ops, true).unwrap();
        assert_eq!(delta.plans, 1);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(s.stats().plans, before.plans + 1, "{file}: plans per edit");
        let wire: Delta = serde_json::from_slice(&serde_json::to_vec(&delta).unwrap()).unwrap();
        wire.apply_to(&mut replay).unwrap();
        let full = model::plan_recovered(s.analysis(), s.spec()).unwrap();
        assert_eq!(
            serde_json::to_vec(&replay.plan).unwrap(),
            serde_json::to_vec(&full).unwrap(),
            "{file}: plan delta"
        );
        let fresh = model::render(s.part(), s.analysis(), &full, &font).unwrap();
        let applied = replay
            .render(
                s.part(),
                s.analysis(),
                &font,
                &mut model::Projections::default(),
            )
            .unwrap();
        assert_eq!(applied.svg(), fresh.svg(), "{file}: render delta");
        assert_eq!(s.drawing().svg(), fresh.svg(), "{file}: session render");
        assert_eq!(applied.dxf(), fresh.dxf());
        assert_eq!(applied.annotations, fresh.annotations);
        assert_eq!(applied.sheet, fresh.sheet);
        let saved = serde_json::to_vec(s.plan()).unwrap();
        let svg = s.drawing().svg();
        s.undo().unwrap().apply_to(&mut replay).unwrap();
        s.redo().unwrap().apply_to(&mut replay).unwrap();
        assert_eq!(serde_json::to_vec(s.plan()).unwrap(), saved);
        assert_eq!(s.drawing().svg(), svg);
    }
    for _ in &sequence {
        s.undo().unwrap().apply_to(&mut replay).unwrap();
    }
    assert_eq!(serde_json::to_vec(s.plan()).unwrap(), initial_plan);
    assert_eq!(serde_json::to_vec(&replay.plan).unwrap(), initial_plan);
    assert_eq!(s.drawing().svg(), initial_svg);
    times.sort_by(f64::total_cmp);
    assert!(
        times[(times.len() * 9).div_ceil(10) - 1] <= 150.0,
        "{file}: native p90 exceeded 150 ms: {times:?}"
    );
    eprintln!(
        "SESSION_LATENCY {file}: samples={} p90={:.3} ms max={:.3} ms",
        times.len(),
        times[(times.len() * 9).div_ceil(10) - 1],
        times.last().unwrap()
    );
}
#[test]
fn plate_benchmark_deltas() {
    benchmark("plate");
}
#[test]
fn dense_benchmark_deltas() {
    benchmark("mfcadpp/10127.step");
}
#[test]
fn turned_benchmark_deltas() {
    benchmark("gramel/GRM-03_thumbwheel_drive_screw.step");
}
#[test]
fn nist_benchmark_deltas() {
    benchmark("nist/nist_ctc_01_asme1_rd.stp");
}

#[test]
fn preview_refusal_and_history_are_transactional() {
    let mut s = open("plate").unwrap();
    let initial = serde_json::to_vec(s.state()).unwrap();
    let stats = s.stats();
    let ops = edits(&s)[5].clone();
    for preview_ops in edits(&s) {
        s.apply(&preview_ops, false).unwrap();
        assert_eq!(serde_json::to_vec(s.state()).unwrap(), initial);
        assert_eq!(s.stats(), stats);
    }
    let preview = s.apply(&ops, false).unwrap();
    assert_eq!(preview.plans, 1);
    assert_eq!(serde_json::to_vec(s.state()).unwrap(), initial);
    assert_eq!(s.stats(), stats);
    assert!(s.undo().is_err());
    assert!(
        s.apply(&[Op::set("dimensions/unknown/omit", json!(true))], true)
            .is_err()
    );
    assert_eq!(serde_json::to_vec(s.state()).unwrap(), initial);
    let committed = s.apply(&ops, true).unwrap();
    assert_eq!(
        serde_json::to_vec(&preview).unwrap(),
        serde_json::to_vec(&committed).unwrap()
    );
    let held = s.stats();
    assert_eq!(held.projections, stats.projections);
    s.undo().unwrap();
    s.apply(&edits(&s)[1], false).unwrap();
    s.redo().unwrap();
    s.undo().unwrap();
    s.apply(&edits(&s)[1], true).unwrap();
    assert!(s.redo().is_err());
}

#[test]
fn queries_are_typed_filtered_and_do_not_plan() {
    let s = open("plate").unwrap();
    let stats = s.stats();
    let d = &s.plan().dimensions[0];
    let answer = s.item(&d.id).unwrap();
    assert_eq!(answer.value, Some(d.value));
    assert_eq!(answer.item.label, Some(d.label.clone()));
    assert!(answer.placement.is_some());
    let filtered = s.items(&ItemFilter {
        view: Some(d.view.clone()),
        kind: Some("dimension".into()),
        ..Default::default()
    });
    assert!(filtered.iter().any(|i| i.item.id == d.id));
    assert!(
        filtered
            .iter()
            .all(|i| i.item.view.as_ref() == Some(&d.view))
    );
    assert!(
        s.items(&ItemFilter {
            region: Some([0.0, 0.0, 0.1, 0.1]),
            ..Default::default()
        })
        .is_empty()
    );
    assert!(
        s.items(&ItemFilter {
            near: Some(d.id.clone()),
            radius: Some(5.0),
            ..Default::default()
        })
        .iter()
        .any(|i| i.item.id == d.id)
    );
    assert!(
        s.item(&format!("{}x", d.id))
            .unwrap_err()
            .choices
            .contains(&d.id)
    );
    let options = s.options(&d.id).unwrap();
    assert!(!options.placements.is_empty());
    assert!(options.operations.contains(&"tolerance".into()));
    let problems = s.problems(&ProblemFilter {
        subject: Some(d.id.clone()),
        ..Default::default()
    });
    assert!(problems.iter().all(|p| p.subjects.contains(&d.id)));
    assert_eq!(s.stats(), stats);
    let filtered = s.items(&ItemFilter {
        kind: Some(d.kind.clone()),
        feature: Some(d.measurements[0].feature.clone()),
        state: Some(d.state.clone()),
        ..Default::default()
    });
    assert!(filtered.iter().any(|i| i.item.id == d.id));
}

#[test]
fn furniture_and_view_deltas_report_changed_added_and_removed_items() {
    let mut s = open("plate").unwrap();
    let mut replay: draftwright::session::State =
        serde_json::from_slice(&serde_json::to_vec(s.state()).unwrap()).unwrap();
    let stats = s.stats();
    let delta = s
        .apply(&[Op::set("sheet/title/material", json!("Steel"))], true)
        .unwrap();
    assert!(delta.changed.iter().any(|i| i.item.id == "sheet"));
    assert!(delta.annotations.is_empty());
    assert!(delta.geometry.views.is_empty());
    assert_eq!(s.stats().annotations, stats.annotations);
    delta.apply_to(&mut replay).unwrap();
    let delta = s
        .apply(&[Op::set("views/iso/omit", json!(true))], true)
        .unwrap();
    assert!(delta.removed.contains(&"v:iso".into()));
    delta.apply_to(&mut replay).unwrap();
    let delta = s.undo().unwrap();
    assert!(delta.added.iter().any(|i| i.item.id == "v:iso"));
    delta.apply_to(&mut replay).unwrap();
    assert_eq!(
        serde_json::to_vec(&replay).unwrap(),
        serde_json::to_vec(s.state()).unwrap()
    );
}

#[test]
fn pmi_fragments_pins_and_problem_changes_replay() {
    use std::io::Read;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pmi/step/nist_ctc_01_asme1_ap242-e1.stp.gz");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(fixture).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("session-pmi");
    std::fs::create_dir_all(&dir).unwrap();
    let step = dir.join("ctc01.step");
    std::fs::write(&step, bytes).unwrap();
    let (part, analysis) = model::analyse_native(&step).unwrap();
    let mut s = Session::new(part, analysis, Spec::default()).unwrap();
    let g = &s.plan().gdt[0];
    let base = format!("annotations/{}", key(&g.id));
    let id = g.id.clone();
    let initial = serde_json::to_vec(s.state()).unwrap();
    let mut replay: draftwright::session::State =
        serde_json::from_slice(&serde_json::to_vec(s.state()).unwrap()).unwrap();
    let font = Font::new(api::MONO.to_vec()).unwrap();
    let mut saw_problem = false;
    let sequence = [
        vec![Op::set(format!("{base}/place"), json!({"pinned":true}))],
        vec![Op::set(format!("{base}/place/lane"), json!(100))],
        vec![Op::set(format!("{base}/omit"), json!(true))],
        vec![Op::set(format!("{base}/omit"), json!(false))],
        vec![Op::remove(format!("{base}/place"))],
    ];
    for ops in &sequence {
        let delta = s.apply(ops, true).unwrap();
        assert_eq!(delta.plans, 1);
        saw_problem |= !delta.problems_added.is_empty() || !delta.problems_resolved.is_empty();
        let wire: Delta = serde_json::from_slice(&serde_json::to_vec(&delta).unwrap()).unwrap();
        wire.apply_to(&mut replay).unwrap();
        assert_eq!(
            serde_json::to_vec(&replay).unwrap(),
            serde_json::to_vec(s.state()).unwrap()
        );
        let full = model::plan_recovered(s.analysis(), s.spec()).unwrap();
        assert_eq!(
            serde_json::to_vec(&replay.plan).unwrap(),
            serde_json::to_vec(&full).unwrap()
        );
        let fresh = model::render(s.part(), s.analysis(), &full, &font).unwrap();
        assert_eq!(
            replay
                .render(
                    s.part(),
                    s.analysis(),
                    &font,
                    &mut model::Projections::default()
                )
                .unwrap()
                .svg(),
            fresh.svg()
        );
        assert!(s.item(&id).is_ok());
    }
    assert!(saw_problem);
    for _ in sequence {
        s.undo().unwrap().apply_to(&mut replay).unwrap();
    }
    assert_eq!(serde_json::to_vec(s.state()).unwrap(), initial);
    assert_eq!(serde_json::to_vec(&replay).unwrap(), initial);
}
