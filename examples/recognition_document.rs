//! Inspect native evidence, including families draftwright's bridge does not read yet.
//! Used by docs/rules/text-batch.md to reproduce the pending cross-solid channel rule.
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: recognition_document <STEP>")?;
    let (part, _) = draftwright::model::analyse(Path::new(&path)).map_err(|e| e.to_string())?;
    let recognition = quiddity::correspondence::recognise(&part);
    let doc = quiddity::recognition::document(&recognition, part.faces.len());
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}
