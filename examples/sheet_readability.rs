//! Audit saved survey Plans with the same measured paper-space readability tests.
//! cargo run --release --example sheet_readability -- SURVEY
use draftwright::{model, sheet_fit};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let directory = std::env::args().nth(1).expect("survey directory");
    let directory = Path::new(&directory);
    let mut rows = BTreeMap::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".analysis.json")
        else {
            continue;
        };
        let a: model::Analysis = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let p: model::Plan = serde_json::from_slice(
            &std::fs::read(directory.join(format!("{name}.plan.json"))).unwrap(),
        )
        .unwrap();
        let r = sheet_fit::readability(&a, &p);
        rows.insert(name.to_owned(), json!({"border_contacts":sheet_fit::border_contacts(&a, &p), "minimum_scale":sheet_fit::min_legible_scale(&a), "failures":r.failures(), "steps":r.steps,"openings":r.openings+r.native_openings,"native_openings":r.native_openings,"text_arrows":r.text_arrows,"depiction":r.depiction,"lanes":r.lanes,"reason":r.to_string()}));
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
