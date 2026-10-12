//! Measure archived Plans without re-planning their sheet choices or geometry.
use draftwright::model::{Analysis, Plan};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let a: Analysis = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let p: Plan = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    println!(
        "{}",
        serde_json::json!({"fill": draftwright::sheet_fit::fill(&a,&p),
        "furniture": draftwright::sheet_fit::furniture_footprints(&a,&p)})
    );
}
