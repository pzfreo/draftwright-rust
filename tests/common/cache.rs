//! Process-local pure documents. Callers receive their own Analysis/Plan clones.
//! Do not use these helpers in determinism tests: both runs must execute independently.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use draftwright::haecceity::Part;
use draftwright::model::{self, Analysis, Plan, Spec};
use sha2::{Digest, Sha256};

type Key = [u8; 32];
type Analysed = (Arc<Part>, Analysis);
type Cells<T> = OnceLock<Mutex<HashMap<Key, Arc<OnceLock<T>>>>>;

fn get<T>(cache: &Cells<T>, key: Key, compute: impl FnOnce() -> T) -> Arc<OnceLock<T>> {
    let cell = cache
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .entry(key)
        .or_default()
        .clone();
    // Only this input is locked during expensive work; other parts can run concurrently.
    cell.get_or_init(compute);
    cell
}

/// Cache a native file's Part and Analysis, including its path and contents in the key.
/// Part is exposed read-only; mutable analyses are always independent clones.
pub fn native(path: &Path) -> Analysed {
    let bytes = std::fs::read(path).unwrap();
    analysed(&path.to_string_lossy(), &bytes, true, || {
        model::analyse_native(path).unwrap()
    })
}

/// Cache scratch-file readers by their logical filename, bytes and recognition mode.
/// The first caller's factory retains the original file-reading/recognition code path.
pub fn analysed(
    name: &str,
    bytes: &[u8],
    native: bool,
    compute: impl FnOnce() -> (Part, Analysis),
) -> Analysed {
    static CACHE: Cells<Analysed> = OnceLock::new();
    let mut hash = Sha256::new();
    hash.update([u8::from(native)]);
    hash.update((name.len() as u64).to_le_bytes());
    hash.update(name.as_bytes());
    hash.update(bytes);
    let cell = get(&CACHE, hash.finalize().into(), || {
        let (part, analysis) = compute();
        (Arc::new(part), analysis)
    });
    let (part, analysis) = cell.get().unwrap();
    (Arc::clone(part), analysis.clone())
}

/// Default-Spec plans only, keyed by the *entire* analysis (not just the source binding).
/// Tests that alter recognition/PMI/projection evidence must not get an unedited plan.
/// Direct and recovered planning use separate keys because their results can differ.
pub fn default_plan(analysis: &Analysis, recovered: bool) -> Plan {
    static CACHE: Cells<Plan> = OnceLock::new();
    let mut hash = Sha256::new();
    hash.update([u8::from(recovered)]);
    hash.update(serde_json::to_vec(analysis).unwrap());
    let cell = get(&CACHE, hash.finalize().into(), || {
        if recovered {
            model::plan_recovered(analysis, &Spec::default()).unwrap()
        } else {
            model::plan(analysis, &Spec::default()).unwrap()
        }
    });
    cell.get().unwrap().clone()
}
