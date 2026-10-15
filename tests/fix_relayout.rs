//! #32: applying the proposed fixes for a leader crossing re-spaces the dimension stack. The
//! editor's spindle with its reviewer's PMI (`tests/fixtures/samples/spindle-pmi.step.gz`: g6 on
//! Ø15, datums A and B, position on the Ø4 cross hole, heights from A) has datum B's leader
//! crossing the 52 and 22 heights, and a fix proposed for each. Both applied, as the editor
//! applies them (Session `apply`, committed), used to put 52 and 22 on one offset line with the
//! 52's text stranded at the sheet's top left, while the problem count dropped.
//!
//! The checks here read the plan's ink independently of the engine's own lint helpers.

use std::io::Read as _;

use draftwright::{lint, model, session::Session};

fn session() -> Session {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/samples/spindle-pmi.step.gz"
    );
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let (part, analysis) =
        model::analyse_native_step(std::path::Path::new("spindle.step"), &bytes).unwrap();
    Session::new(part, analysis, model::Spec::default()).unwrap()
}

type Line = [[f64; 2]; 2];

/// A linear dimension's dimension line: the axis it runs along, its offset across, the span
/// between its extension lines and the line's extent.
fn dimension_line(lines: &[Line]) -> Option<(usize, f64, [f64; 2], [f64; 2])> {
    let [e0, e1, d] = lines else { return None };
    let k = if (d[0][1] - d[1][1]).abs() < 1e-6 {
        0
    } else {
        1
    };
    let (a, b) = (e0[0][k], e1[0][k]);
    Some((
        k,
        d[0][1 - k],
        [a.min(b), a.max(b)],
        [d[0][k].min(d[1][k]), d[0][k].max(d[1][k])],
    ))
}

/// The leader crossings on datum B, and the ops their fixes propose.
fn b_crossings(s: &Session) -> (Vec<String>, Vec<draftwright::items::Op>) {
    let crossings: Vec<_> = s
        .plan()
        .problems
        .iter()
        .filter(|p| {
            p.code == "annotation_leader_crossing"
                && p.subjects.first().is_some_and(|d| d.ends_with("t=B"))
        })
        .collect();
    (
        crossings.iter().map(|p| p.subjects[1].clone()).collect(),
        crossings.iter().flat_map(|p| p.fix.clone()).collect(),
    )
}

#[test]
fn applying_both_proposed_fixes_keeps_the_stack_apart_and_text_on_its_lines() {
    let mut s = session();
    let (moved, ops) = b_crossings(&s);
    assert_eq!(
        moved,
        ["d:envelope/height.length", "d:f:turned_step:2/step.length"],
        "the review sample's two crossings"
    );
    assert_eq!(ops.len(), 2);
    let errors = |s: &Session| -> Vec<String> {
        lint::structural::check(s.drawing())
            .into_iter()
            .filter(|i| i.severity == lint::Severity::Error)
            .map(|i| i.id)
            .collect()
    };
    let lint_before = errors(&s);
    let delta = s.apply(&ops, true).unwrap();

    // The datum-B crossings are gone, and nothing hard is added in their place.
    assert!(b_crossings(&s).0.is_empty());
    let added: Vec<_> = delta
        .problems_added
        .iter()
        .filter(|p| p.severity == "error" || p.severity == "warning")
        .map(|p| format!("{} {:?}", p.code, p.subjects))
        .collect();
    assert!(added.is_empty(), "new findings: {added:?}");
    let new_lint: Vec<_> = errors(&s)
        .into_iter()
        .filter(|i| !lint_before.contains(i))
        .collect();
    assert!(new_lint.is_empty(), "new lint errors: {new_lint:?}");

    let linear: Vec<_> = s
        .plan()
        .dimensions
        .iter()
        .filter(|d| d.kind == "linear")
        .filter_map(|d| Some((d.id.as_str(), d.placed.as_ref()?)))
        .collect();
    for id in &moved {
        assert!(linear.iter().any(|(d, _)| d == id), "{id} is drawn");
    }
    // No two dimensions on one offset line over overlapping spans.
    for (i, (a, pa)) in linear.iter().enumerate() {
        for (b, pb) in &linear[i + 1..] {
            let (Some(la), Some(lb)) = (dimension_line(&pa.lines), dimension_line(&pb.lines))
            else {
                continue;
            };
            let overlap = la.2[1].min(lb.2[1]) - la.2[0].max(lb.2[0]);
            assert!(
                la.0 != lb.0 || (la.1 - lb.1).abs() >= 0.5 || overlap <= 0.5,
                "{a} and {b} share the offset line {}",
                la.1
            );
        }
    }
    // Every dimension's text within its normal distance of its own line: 1 mm across it (LIFT)
    // and, beyond a line run on to it, 1 mm past its end; a text height is the bound.
    for (id, p) in &linear {
        let Some((k, offset, _, extent)) = dimension_line(&p.lines) else {
            continue;
        };
        let b = p.label_box;
        let along = (extent[0] - b[k + 2]).max(b[k] - extent[1]).max(0.0);
        let across = (b[1 - k] - offset).max(offset - b[3 - k]).max(0.0);
        assert!(
            along.hypot(across) <= 3.0,
            "{id}'s text is {along:.1} mm along and {across:.1} mm across from its line"
        );
    }
}

/// The fix checks: an edit that still puts two dimensions on one line (both pinned to lane 0,
/// which the stack does not re-space) is reported as a new hard finding, not as having resolved
/// the crossings.
#[test]
fn a_shared_line_left_by_an_edit_is_a_new_error() {
    let mut s = session();
    let (_, mut ops) = b_crossings(&s);
    for op in &mut ops {
        let mut v = serde_json::to_value(&*op).unwrap();
        v["value"]["pinned"] = serde_json::json!(true);
        v["value"].as_object_mut().unwrap().remove("along");
        *op = serde_json::from_value(v).unwrap();
    }
    let delta = s.apply(&ops, true).unwrap();
    let shared: Vec<_> = delta
        .problems_added
        .iter()
        .filter(|p| p.code == "dimension_line_shared")
        .collect();
    assert_eq!(shared.len(), 1, "{:?}", delta.problems_added);
    assert_eq!(shared[0].severity, "error");
    assert!(
        lint::structural::check(s.drawing())
            .iter()
            .any(|i| i.code == "dimension_line_shared" && i.severity == lint::Severity::Error)
    );
}
