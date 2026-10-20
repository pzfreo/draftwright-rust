//! Check a local offer's intent as well as its ordinary edit operations.
use draftwright::{fixes::Scope, items, model};

pub fn constraint(op: &items::Op) -> bool {
    matches!(
        op.path.as_str(),
        "sheet/keep_layout" | "sheet/size" | "sheet/scale" | "sheets/max"
    )
}
pub fn edits<'a>(problem: &'a model::Problem, plan: &model::Plan) -> Vec<&'a items::Op> {
    assert_eq!(problem.offered_fixes()[0].scope(), Scope::Local);
    for (path, value) in [
        ("sheet/keep_layout", serde_json::json!(true)),
        ("sheet/size", serde_json::json!(plan.sheet.size)),
        ("sheet/scale", serde_json::json!(plan.sheet.scale)),
        ("sheets/max", serde_json::json!(1)),
    ] {
        let ops: Vec<_> = problem.fix.iter().filter(|op| op.path == path).collect();
        assert_eq!(ops.len(), 1, "one constraint per alternative: {path}");
        assert_eq!(ops[0].op, "set");
        assert_eq!(ops[0].value, Some(value));
    }
    assert!(plan.sheets.is_empty());
    problem.fix.iter().filter(|op| !constraint(op)).collect()
}
