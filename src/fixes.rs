//! Proportionate remedies. Scope describes authored Spec paths, never a preview or a
//! prediction of which other ink the layout solver may move.
use crate::items::Op;
use serde::{Deserialize, Serialize, ser::SerializeStruct};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Local,
    View,
    Sheet,
}

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
        .unwrap_or(Sheet)
}

pub fn scope(ops: &[Op]) -> Scope {
    let broadest = ops
        .iter()
        .map(|op| path_scope(&op.path))
        .max()
        .unwrap_or(Scope::Sheet);
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
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Fix {
    pub description: String,
    pub ops: Vec<Op>,
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
        assert_eq!(scope(&[]), Sheet);
    }
}
