//! Measure the existing ISSUE-1187-a sweep on quality.py's exact saved plans.
//! cargo run --release --example leader_crossing_routes -- /tmp/head-quality
use std::{collections::BTreeMap, error::Error, fs, path::PathBuf};

use draftwright::{
    dims,
    model::{Analysis, Plan},
};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("quality directory required")?,
    );
    let mut rows = Vec::new();
    for part in include_str!("../tools/quality_parts.txt").lines() {
        let name = std::path::Path::new(part)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let plan: Plan =
            serde_json::from_slice(&fs::read(directory.join(format!("{name}.plan.json")))?)?;
        let analysis: Analysis =
            serde_json::from_slice(&fs::read(directory.join(format!("{name}.analysis.json")))?)?;
        let mut measured = BTreeMap::new();
        for p in &plan.problems {
            let kind = match p.code.as_str() {
                "annotation_leader_crossing" if p.crossing_kinds.iter().any(|k| k == "leader") => {
                    "leader_leader"
                }
                "annotation_ink_overlap" if p.ink_is_leader == Some(true) => "leader_text",
                _ => continue,
            };
            let id = &p.subjects[0];
            let count = *measured.entry(id).or_insert_with(|| {
                dims::clear_routes(&analysis, &plan.sheet, &plan.views, &plan.dimensions, id)
            });
            rows.push(
                serde_json::json!({"part": part, "kind": kind, "subjects": p.subjects,
                "clear_routes": count}),
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
