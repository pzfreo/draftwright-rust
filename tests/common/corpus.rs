//! The sampled corpus parts (`tests/fixtures/sheets/`) read as the port reads them: from the
//! corpus (`DRAFTWRIGHT_CORPUS`, by default `~/repos/quiddity/tests/corpus`), with Python
//! quiddity's recognition for each (`tests/fixtures/recognition/corpus/<name>.json.gz`, written
//! by `tools/recognise.py`) when there is one.
//!
//! A test that needs the corpus skips when it is missing, unless `DRAFTWRIGHT_CORPUS_REQUIRED=1`
//! (as in CI and the verify line), which makes a missing corpus a failure.

#![allow(dead_code)]

use std::io::Read as _;
use std::path::{Path, PathBuf};

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis};

/// The corpus directory, or `None` (the caller skips) when it is not found. Panics instead when
/// `DRAFTWRIGHT_CORPUS_REQUIRED=1`, so a run that must cover the corpus cannot pass without it.
pub fn dir() -> Option<PathBuf> {
    let dir = std::env::var_os("DRAFTWRIGHT_CORPUS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("repos/quiddity/tests/corpus"))
        })
        .filter(|d| d.is_dir());
    if dir.is_none() && std::env::var_os("DRAFTWRIGHT_CORPUS_REQUIRED").is_some_and(|v| v == "1") {
        panic!(
            "DRAFTWRIGHT_CORPUS_REQUIRED=1 but the corpus is not found: set DRAFTWRIGHT_CORPUS to \
             quiddity's tests/corpus (default ~/repos/quiddity/tests/corpus)"
        );
    }
    dir
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
