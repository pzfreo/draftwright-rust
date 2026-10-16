//! Real overflow after the bare string post's layout is repaired: an inspection
//! schedule occupies the spare first-sheet column. The geometry and scale stay
//! the same, and both automatic overflow prefixes are still required.
use serde_json::{Value, json};

pub fn tables() -> Value {
    let mut rows = vec![json!(["INSPECTION SCHEDULE", "CHECKED"])];
    rows.extend((0..6).map(|i| json!([format!("FORM AND SIZE {}", "X".repeat(30)), i])));
    json!([{"id":"t:1", "rows":rows, "prefer":"tr"}])
}
