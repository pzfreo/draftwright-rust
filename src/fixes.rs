//! Proportionate remedies. Scope describes authored Spec paths, never a preview or a
//! prediction of which other ink the layout solver may move.
use crate::items::Op;
use serde::{Deserialize, Serialize, ser::SerializeStruct};

pub use crate::remedy_scope::Scope;

/// First matching pattern wins; `*` matches one escaped key and `**` its descendants.
/// Keep sheet allocation before view intent. Only known item fields are local: new or
/// unknown paths conservatively re-plan the sheet until deliberately added to this table.
pub fn path_scope(path: &str) -> Scope {
    use Scope::*;
    const PATTERNS: &[(&str, Scope)] = &[
        ("views/*/sheet", Sheet),
        ("tables/*/sheet", Sheet),
        ("sheets/**", Sheet),
        ("sheet/size", Sheet),
        ("sheet/scale", Sheet),
        ("hole_table", Sheet),
        ("dimension_source", Sheet),
        ("dimension_schemes/**", Sheet),
        ("pmi/**", Sheet),
        ("views/*", View),
        ("views/*/place/**", View),
        ("views/*/scale", View),
        ("views/*/omit", View),
        ("views/*/detail/**", View),
        ("dimensions/*", Local),
        ("dimensions/*/place/**", Local),
        ("dimensions/*/view", Local),
        ("dimensions/*/omit", Local),
        ("dimensions/*/text/**", Local),
        ("dimensions/*/style", Local),
        ("dimensions/*/tolerance/**", Local),
        ("dimensions/*/fit", Local),
        ("annotations/*", Local),
        ("annotations/*/place/**", Local),
        ("annotations/*/omit", Local),
        ("annotations/*/text/**", Local),
        ("features/*/fit", Local),
        ("sheet/title/*", Local),
    ];
    let parts: Vec<_> = path.trim_matches('/').split('/').collect();
    PATTERNS
        .iter()
        .find_map(|(pattern, scope)| {
            let pattern: Vec<_> = pattern.split('/').collect();
            let recursive = pattern.last() == Some(&"**");
            let count = pattern.len() - usize::from(recursive);
            ((recursive && parts.len() >= count || parts.len() == pattern.len())
                && pattern[..count]
                    .iter()
                    .zip(&parts)
                    .all(|(a, b)| *a == "*" || a == b))
            .then_some(*scope)
        })
        .unwrap_or(crate::remedy_scope::UNBOUND_SCOPE)
}

// An object-valued set addresses its nested Spec paths too. Keep the root's
// conservative scope: replacing a collection may remove entries absent in its value.
fn value_scope(path: &str, value: &serde_json::Value) -> Scope {
    let mut scope = path_scope(path);
    if scope == Scope::Sheet {
        return scope;
    }
    if let serde_json::Value::Object(fields) = value {
        for (name, value) in fields {
            scope = scope.max(value_scope(
                &format!("{path}/{}", crate::items::key(name)),
                value,
            ));
        }
    }
    scope
}

pub fn scope(ops: &[Op]) -> Scope {
    let broadest = ops
        .iter()
        .map(|op| {
            op.value.as_ref().map_or_else(
                || path_scope(&op.path),
                |value| value_scope(&op.path, value),
            )
        })
        .max()
        .unwrap_or(crate::remedy_scope::UNBOUND_SCOPE);
    if broadest == Scope::Sheet {
        return broadest;
    }
    // Local/view remedies act on one item/view. A bundle editing several owners is
    // sheet-wide even if each individual path would be a narrower edit.
    let owner = |op: &Op| {
        let count = if op.path.starts_with("sheet/title/") {
            3
        } else {
            2
        };
        op.path
            .trim_matches('/')
            .split('/')
            .take(count)
            .collect::<Vec<_>>()
            .join("/")
    };
    let first = owner(&ops[0]);
    if ops.iter().any(|op| owner(op) != first) {
        Scope::Sheet
    } else {
        broadest
    }
}

/// One alternative: its operations apply together. Scope is always derived, including
/// when reading old or externally supplied JSON; a caller cannot understate it.
#[derive(Clone, Debug, Deserialize)]
pub struct Fix {
    pub description: String,
    pub ops: Vec<Op>,
}
impl PartialEq for Fix {
    fn eq(&self, other: &Self) -> bool {
        self.ops == other.ops && self.description() == other.description()
    }
}
impl Fix {
    pub fn new(description: impl Into<String>, ops: Vec<Op>) -> Self {
        Self {
            description: description.into(),
            ops,
        }
    }
    pub fn scope(&self) -> Scope {
        scope(&self.ops)
    }
    pub fn description(&self) -> String {
        if self.scope() != Scope::Sheet || self.description.starts_with("Re-plans the sheet:") {
            return self.description.clone();
        }
        let effects: Vec<_> = self
            .ops
            .iter()
            .map(|op| {
                let value = op.value.as_ref().map(|v| {
                    v.as_str()
                        .map(String::from)
                        .unwrap_or_else(|| v.to_string())
                });
                match op.path.as_str() {
                    "sheet/size" => format!("paper {}", value.unwrap_or("automatic".into())),
                    "sheet/scale" => format!("scale {}", value.unwrap_or("automatic".into())),
                    "hole_table" => format!("hole table {}", value.unwrap_or("auto".into())),
                    "sheets/max" => format!("sheet count {}", value.unwrap_or("automatic".into())),
                    "sheets/hole_table" => format!(
                        "hole table allocation {}",
                        value.unwrap_or("automatic".into())
                    ),
                    "dimension_source" => {
                        format!("dimension source {}", value.unwrap_or("auto".into()))
                    }
                    path if path.starts_with("dimension_schemes/") => format!(
                        "{} dimension scheme {}",
                        op.op,
                        path.trim_start_matches("dimension_schemes/")
                    ),
                    path if path.starts_with("pmi") => {
                        format!("PMI mode {}", value.unwrap_or("automatic".into()))
                    }
                    path if path.ends_with("/sheet") => format!(
                        "{} sheet allocation {}",
                        path.trim_end_matches("/sheet").replace('/', " "),
                        value.unwrap_or("automatic".into())
                    ),
                    path => format!(
                        "{} {path}{}",
                        op.op,
                        value.map(|v| format!(" to {v}")).unwrap_or_default()
                    ),
                }
            })
            .collect();
        format!(
            "Re-plans the sheet: {}. {}",
            effects.join(", "),
            self.description
        )
    }
}
impl Serialize for Fix {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Fix", 3)?;
        s.serialize_field("scope", &self.scope())?;
        s.serialize_field("description", &self.description())?;
        s.serialize_field("ops", &self.ops)?;
        s.end()
    }
}

pub(crate) fn attach(analysis: &crate::model::Analysis, plan: &mut crate::model::Plan) {
    use sha2::{Digest, Sha256};
    // Preview and commit revisit identical settled ink. Reuse only exact input
    // results, in the planner's existing bounded cache, without another search.
    static MEMO: crate::planning::BoundedMemo<Vec<crate::model::Problem>> =
        crate::planning::BoundedMemo::new(64);
    let key: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&(&*plan, crate::planning::policy_key(analysis)))
            .expect("fix inputs serialize"),
    )
    .into();
    if let Some(problems) = MEMO.get(&key) {
        plan.problems = problems;
        return;
    }
    crate::planning::with(analysis, || attach_inner(analysis, plan));
    MEMO.insert(key, plan.problems.clone());
}

/// Add remedies only after selecting the drawing. As with the existing placement
/// fixes, compatibility and Spec edits are checked; layout effects need preview/apply.
/// No planner or full preview is invoked to generate a suggestion.
fn attach_inner(analysis: &crate::model::Analysis, plan: &mut crate::model::Plan) {
    let relevant = |p: &crate::model::Problem| {
        matches!(
            p.code.as_str(),
            "annotation_leader_crossing"
                | "extension_through_feature"
                | "leader_through_feature"
                | "annotation_overlap"
                | "annotation_ink_overlap"
                | "label_centerline_overlap"
        )
    };
    if !plan.problems.iter().any(relevant) {
        return;
    }
    let options = crate::dims::placement_options_with_spec(analysis, &plan.views, &plan.spec);
    // Reuse schedule eligibility only, without installing or laying out a table.
    // Always + no pins avoids select's canonical-plan branches.
    let sheet_remedy = if !crate::placement_intent::has_pins(&plan.spec)
        && plan.spec.hole_table == crate::hole_table::Setting::Auto
        && plan.hole_tables.is_empty()
        && plan.hole_table.is_none()
        && plan.spec.dimension_source == crate::model::DimensionSource::Auto
    {
        let mut table = plan.clone();
        table.spec.hole_table = crate::hole_table::Setting::Always;
        let selection = crate::hole_table::select(analysis, &table);
        crate::layout::SCALES
            .into_iter()
            .find(|scale| {
                *scale < plan.sheet.scale_factor
                    && *scale >= crate::sheet_fit::min_legible_scale(analysis)
                    && *scale >= crate::sheets::floor(&plan.spec).unwrap_or(0.0)
            })
            .filter(|_| !selection.tables.is_empty())
            .map(|scale| {
                let scale = crate::model::scale_name(scale);
                Fix::new(
                    format!("Try a hole table at {scale}; preview the sheet-wide result."),
                    vec![
                        Op::set("hole_table", serde_json::json!("always")),
                        Op::set("sheet/scale", serde_json::json!(scale)),
                    ],
                )
            })
    } else {
        None
    };
    let snapshot = plan.clone();
    for problem in plan.problems.iter_mut().filter(|p| relevant(p)) {
        // finish() can run more than once; don't duplicate earlier remedies.
        if problem
            .fixes
            .iter()
            .any(|f| f.description.ends_with("preview the placement."))
        {
            continue;
        }
        let old = problem.offered_fixes();
        let mut local = Vec::new();
        // Keep existing annotation/pin remedies; otherwise validate compatible item
        // placements with the existing ink checks, never a full sheet preview.
        if (problem.fix.is_empty()
            || problem
                .subjects
                .first()
                .is_some_and(|id| plan.dimensions.iter().any(|d| &d.id == id)))
            && let Some(fix) =
                crate::dims::local_placement_fix(analysis, &snapshot, problem, &options)
        {
            local.push(fix);
        }
        if local.is_empty() {
            local.extend(old);
        } else {
            local.extend(old.into_iter().filter(|f| f.scope() != Scope::Local));
        }
        if let Some(fix) = &sheet_remedy {
            local.push(fix.clone());
        }
        let mut unique = Vec::<Fix>::new();
        for fix in local {
            if !unique.iter().any(|f| f.ops == fix.ops) {
                unique.push(fix);
            }
        }
        unique.sort_by_key(Fix::scope);
        if let Some(fix) = unique.first() {
            problem.fix = fix.ops.clone();
        }
        problem.fixes = unique;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alternatives_sort_bundles_without_reordering_operations() {
        use crate::model::Problem;
        let sheet = vec![
            Op::set("hole_table", "always".into()),
            Op::set("sheet/scale", "1:10".into()),
        ];
        let p = Problem {
            fix: sheet.clone(),
            fixes: vec![
                Fix::new("Move view", vec![Op::remove("views/iso/place")]),
                Fix::new(
                    "Move label",
                    vec![Op::set("dimensions/d:a~1b/place/lane", 1.into())],
                ),
            ],
            ..Default::default()
        };
        let wire = serde_json::to_value(&p).unwrap();
        assert_eq!(wire["fixes"][0]["scope"], "local");
        assert_eq!(wire["fixes"][1]["scope"], "view");
        assert_eq!(wire["fixes"][2]["scope"], "sheet");
        assert_eq!(
            wire["fixes"][2]["ops"],
            serde_json::to_value(sheet).unwrap()
        );
        let description = wire["fixes"][2]["description"].as_str().unwrap();
        assert!(description.contains("Re-plans the sheet"));
        assert!(description.contains("hole table always") && description.contains("1:10"));
        let reread: Problem = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(p, reread);
        assert_eq!(serde_json::to_value(reread).unwrap(), wire);
        let understated: Fix = serde_json::from_value(serde_json::json!({
            "scope":"local", "description":"Change paper", "ops":[{"op":"set","path":"sheet/size","value":"A3"}]
        })).unwrap();
        assert_eq!(understated.scope(), Scope::Sheet);
    }
    #[test]
    fn classification_table() {
        use Scope::*;
        for (path, expected) in [
            ("dimensions/d:a~1b/place", Local),
            ("dimensions/d/place/side", Local),
            ("dimensions/d/place/lane", Local),
            ("dimensions/d/place/along", Local),
            ("dimensions/d/place/offset", Local),
            ("dimensions/d/text/template", Local),
            ("dimensions/d/style", Local),
            ("dimensions/d/omit", Local),
            ("dimensions/d/tolerance", Local),
            ("annotations/g/place", Local),
            ("features/f/fit", Local),
            ("views/iso/place/gap", View),
            ("views/iso/scale", View),
            ("views/plan/omit", View),
            ("views/detail_a", View),
            ("views/section_aa", View),
            ("sheet/size", Sheet),
            ("sheet/scale", Sheet),
            ("sheets/max", Sheet),
            ("views/iso/sheet", Sheet),
            ("tables/0/sheet", Sheet),
            ("hole_table", Sheet),
            ("dimension_source", Sheet),
            ("dimension_schemes/xy", Sheet),
            ("pmi/mode", Sheet),
            ("unknown", Sheet),
            ("dimensions/d/new_field", Sheet),
            ("views/iso/new_field", Sheet),
            ("dimensions", Sheet),
            ("", Sheet),
        ] {
            assert_eq!(path_scope(path), expected, "{path}");
        }
        assert_eq!(
            scope(&[
                Op::set("dimensions/d/omit", true.into()),
                Op::remove("sheet/scale")
            ]),
            Sheet
        );
        assert_eq!(
            scope(&[
                Op::set("dimensions/d/place/lane", 1.into()),
                Op::set("dimensions/d/omit", false.into())
            ]),
            Local
        );
        assert_eq!(
            scope(&[
                Op::remove("dimensions/a/tolerance"),
                Op::remove("dimensions/b/tolerance")
            ]),
            Sheet
        );
        assert_eq!(
            scope(&[
                Op::remove("views/front/omit"),
                Op::remove("views/side/omit")
            ]),
            Sheet
        );
        assert_eq!(
            scope(&[Op::set("views/iso", serde_json::json!({"sheet":2}))]),
            Sheet
        );
        assert_eq!(
            scope(&[Op::set(
                "views/detail_a",
                serde_json::json!({"detail":{"around":"f:hole:1","scale_factor":2}})
            )]),
            View
        );
        assert_eq!(
            scope(&[Op::set(
                "dimensions/d",
                serde_json::json!({"place":{"lane":1},"omit":false})
            )]),
            Local
        );
        assert_eq!(
            scope(&[Op::set(
                "dimensions/d",
                serde_json::json!({"new_field":true})
            )]),
            Sheet
        );
        assert_eq!(
            scope(&[Op::set("dimensions", serde_json::json!({"d":{}}))]),
            Sheet
        );
        assert_eq!(scope(&[]), Sheet);
    }
}
