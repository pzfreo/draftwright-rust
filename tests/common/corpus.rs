//! The sampled corpus parts (`tests/fixtures/sheets/`) read as the port reads them: from the
//! corpus (`DRAFTWRIGHT_CORPUS`, by default `~/repos/quiddity/tests/corpus`), with Python
//! quiddity's recognition for each (`tests/fixtures/recognition/corpus/<name>.json.gz`, written
//! by `tools/recognise.py`) when there is one.

#![allow(dead_code)]

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis};

pub fn dir() -> Option<PathBuf> {
    let dir = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })?;
    dir.is_dir().then_some(dir)
}

/// The recognition JSON for sampled part *name*, if it has one.
pub fn recognition(name: &str) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/recognition/corpus")
        .join(format!("{name}.json.gz"));
    let file = std::fs::File::open(path).ok()?;
    let mut text = String::new();
    flate2::read::GzDecoder::new(file)
        .read_to_string(&mut text)
        .unwrap();
    Some(text)
}

/// The part *file* (relative to the corpus *dir*) analysed, with sampled part *name*'s features.
pub fn analyse(dir: &Path, file: &str, name: &str) -> (Part, Analysis) {
    let (part, mut analysis) = model::analyse(&dir.join(file)).unwrap();
    if let Some(json) = recognition(name) {
        model::recognise(&part, &mut analysis, &json).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    (part, analysis)
}
