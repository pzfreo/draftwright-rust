//! Check a local offer's constraint as well as its ordinary edit operations.
use draftwright::{fixes::Scope, items, model};

pub fn edits<'a>(problem: &'a model::Problem, plan: &model::Plan) -> Vec<&'a items::Op> {
    assert_eq!(problem.offered_fixes()[0].scope(), Scope::Local);
    let guards: Vec<_> = problem
        .fix
        .iter()
        .filter(|op| op.path == "sheet/keep_layout")
        .collect();
    assert_eq!(guards.len(), 1, "one retained frame per alternative");
    assert_eq!(guards[0].op, "set");
    let frame: draftwright::local_layout::Layout =
        serde_json::from_value(guards[0].value.clone().unwrap()).unwrap();
    assert!(plan.sheets.is_empty());
    assert_eq!(frame.size, plan.sheet.size);
    assert_eq!(frame.scale, plan.sheet.scale);
    assert_eq!(frame.views.len(), plan.views.len());
    for view in &plan.views {
        let held = &frame.views[&view.id];
        assert_eq!(
            (held.at, held.area, held.scale),
            (view.at, view.area, view.scale)
        );
    }
    for item in items::items(plan)
        .into_iter()
        .filter(|i| i.state == items::State::Shown)
    {
        assert!(
            frame.required.contains(&item.id),
            "{} stays conveyed",
            item.id
        );
    }
    problem
        .fix
        .iter()
        .filter(|op| op.path != "sheet/keep_layout")
        .collect()
}
