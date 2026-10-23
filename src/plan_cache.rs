//! An opt-in on-disk memo of finished plans, so that processes planning the same part under the
//! same Spec (CI's test shards and acceptance measurements) plan it once and read it after.
//!
//! An entry is keyed by the engine's source fingerprint ([`crate::stateless::ENGINE_VERSION`]:
//! every source file, asset, the manifest and the lockfile), the planning function, and the
//! complete Analysis and Spec as JSON, so a plan is only ever read by the engine that made it,
//! for exactly the inputs it was made from. A miss plans as usual; refusals are not stored.
//!
//! The entry holds the plan's JSON and the planning state the plan's JSON leaves out (table
//! escalation, callout geometry, axis overruns), and is only written when reading it back gives
//! a plan identical to the one made, field for field (`Debug`), so a cached plan is the plan.
//!
//! Nothing here is used unless asked for: [`plan_recovered_in`] and [`plan_in`] take the
//! directory, and the command line's planning reads `DRAFTWRIGHT_PLAN_CACHE` ([`dir`]).
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hole_table::TableOutcome;
use crate::model::{self, Analysis, Invalid, Plan, Spec};

/// The variable naming the command line's plan cache directory.
pub const VARIABLE: &str = "DRAFTWRIGHT_PLAN_CACHE";
const FORMAT: &str = "draftwright.plan-cache/1";

/// The command line's plan cache directory: `DRAFTWRIGHT_PLAN_CACHE`, when set and not empty.
pub fn dir() -> Option<PathBuf> {
    std::env::var_os(VARIABLE)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// [`model::plan_recovered`], read from or written to the cache in *dir* when given.
pub fn plan_recovered_in(
    dir: Option<&Path>,
    analysis: &Analysis,
    spec: &Spec,
) -> Result<Plan, Invalid> {
    match dir {
        Some(dir) => cached(dir, "recovered", analysis, spec, model::plan_recovered),
        None => model::plan_recovered(analysis, spec),
    }
}

/// [`model::plan`], read from or written to the cache in *dir* when given.
pub fn plan_in(dir: Option<&Path>, analysis: &Analysis, spec: &Spec) -> Result<Plan, Invalid> {
    match dir {
        Some(dir) => cached(dir, "plan", analysis, spec, model::plan),
        None => model::plan(analysis, spec),
    }
}

/// The entry's file name: the digest of everything the plan is a function of.
fn key(kind: &str, analysis: &Analysis, spec: &Spec) -> String {
    let mut hash = Sha256::new();
    for part in [
        FORMAT.as_bytes(),
        env!("DRAFTWRIGHT_ENGINE_VERSION").as_bytes(),
        std::env::consts::ARCH.as_bytes(),
        std::env::consts::OS.as_bytes(),
        if cfg!(debug_assertions) {
            b"debug"
        } else {
            b"release"
        },
        kind.as_bytes(),
        &serde_json::to_vec(analysis).expect("an Analysis serialises"),
        &serde_json::to_vec(spec).expect("a Spec serialises"),
    ] {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part);
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn cached(
    dir: &Path,
    kind: &str,
    analysis: &Analysis,
    spec: &Spec,
    compute: fn(&Analysis, &Spec) -> Result<Plan, Invalid>,
) -> Result<Plan, Invalid> {
    let key = key(kind, analysis, spec);
    let path = dir.join(format!("{key}.json"));
    if let Some(plan) = read(&path) {
        return Ok(plan);
    }
    // One process plans a key while the others wanting it wait, then read its entry. The lock
    // is advisory and only orders work: without it (or on any I/O error) each plans for itself.
    let lock = fs::create_dir_all(dir)
        .and_then(|()| File::create(dir.join(format!("{key}.lock"))))
        .ok()
        .filter(|f| f.lock().is_ok());
    if lock.is_some()
        && let Some(plan) = read(&path)
    {
        return Ok(plan);
    }
    let plan = compute(analysis, spec)?;
    write(dir, &path, &plan);
    drop(lock);
    Ok(plan)
}

fn read(path: &Path) -> Option<Plan> {
    let text = fs::read(path).ok()?;
    let entry: Entry = serde_json::from_slice(&text).ok()?;
    Some(entry.into_plan())
}

/// Stores *plan* when its entry reads back as the same plan; otherwise stores nothing.
fn write(dir: &Path, path: &Path, plan: &Plan) {
    let Ok(text) = serde_json::to_vec(&Entry::of(plan)) else {
        return;
    };
    let Ok(entry) = serde_json::from_slice::<Entry>(&text) else {
        return;
    };
    let (read, made) = (format!("{:?}", entry.into_plan()), format!("{plan:?}"));
    if read != made {
        // Planning state this entry does not carry: say where, beside the entry, and plan
        // afresh every time instead.
        let at = read
            .bytes()
            .zip(made.bytes())
            .take_while(|(r, m)| r == m)
            .count();
        let context = |s: &str| {
            s.get(at.saturating_sub(200)..(at + 200).min(s.len()))
                .unwrap_or("")
                .to_string()
        };
        let _ = fs::write(
            path.with_extension("refused"),
            format!("read: {}\nmade: {}\n", context(&read), context(&made)),
        );
        return;
    }
    let staged = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let done = File::create(&staged)
        .and_then(|mut f| f.write_all(&text))
        .and_then(|()| fs::rename(&staged, path));
    if done.is_err() {
        let _ = fs::remove_file(&staged);
    }
}

/// A plan and the planning state its JSON leaves out.
#[derive(Serialize, Deserialize)]
struct Entry {
    plan: Plan,
    hidden: Hidden,
}

#[derive(Serialize, Deserialize)]
struct Hidden {
    table_outcome: u8,
    table_restore: Option<Box<Entry>>,
    callout_widths: BTreeMap<String, f64>,
    callout_tips: BTreeMap<String, (String, [f64; 2])>,
    dimension_labels: BTreeMap<String, (String, [f64; 2])>,
    /// Each view's line work's overruns, in order.
    overruns: Vec<Vec<f64>>,
    /// Each problem's fixes as held: a problem's JSON states its offered fixes instead.
    problems: Vec<RawProblem>,
}

#[derive(Serialize, Deserialize)]
struct RawProblem {
    fix: Vec<crate::items::Op>,
    fixes: Vec<RawFix>,
}

#[derive(Serialize, Deserialize)]
struct RawFix {
    description: String,
    ops: Vec<crate::items::Op>,
    minimum_scope: Option<crate::remedy_scope::Scope>,
}

impl Entry {
    fn of(plan: &Plan) -> Self {
        Entry {
            plan: plan.clone(),
            hidden: Hidden {
                table_outcome: match plan.table_outcome {
                    TableOutcome::Kept => 0,
                    TableOutcome::NoPlace => 1,
                    TableOutcome::Balloons => 2,
                    TableOutcome::None => 3,
                },
                table_restore: plan
                    .table_restore
                    .as_deref()
                    .map(|p| Box::new(Entry::of(p))),
                callout_widths: plan.callout_widths.clone(),
                callout_tips: plan.callout_tips.clone(),
                dimension_labels: plan.dimension_labels.clone(),
                overruns: plan
                    .views
                    .iter()
                    .map(|v| v.line_work.iter().map(|w| w.overrun).collect())
                    .collect(),
                problems: plan
                    .problems
                    .iter()
                    .map(|p| RawProblem {
                        fix: p.fix.clone(),
                        fixes: p
                            .fixes
                            .iter()
                            .map(|f| RawFix {
                                description: f.description.clone(),
                                ops: f.ops.clone(),
                                minimum_scope: f.minimum_scope,
                            })
                            .collect(),
                    })
                    .collect(),
            },
        }
    }

    fn into_plan(self) -> Plan {
        let Entry { mut plan, hidden } = self;
        plan.table_outcome = match hidden.table_outcome {
            0 => TableOutcome::Kept,
            1 => TableOutcome::NoPlace,
            2 => TableOutcome::Balloons,
            _ => TableOutcome::None,
        };
        plan.table_restore = hidden.table_restore.map(|e| Box::new(e.into_plan()));
        plan.callout_widths = hidden.callout_widths;
        plan.callout_tips = hidden.callout_tips;
        plan.dimension_labels = hidden.dimension_labels;
        for (view, overruns) in plan.views.iter_mut().zip(hidden.overruns) {
            for (work, overrun) in view.line_work.iter_mut().zip(overruns) {
                work.overrun = overrun;
            }
        }
        for (problem, raw) in plan.problems.iter_mut().zip(hidden.problems) {
            problem.fix = raw.fix;
            problem.fixes = raw
                .fixes
                .into_iter()
                .map(|f| crate::fixes::Fix {
                    description: f.description,
                    ops: f.ops,
                    minimum_scope: f.minimum_scope,
                })
                .collect();
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cached plan is the plan: written on the first call, read on the second, identical in
    /// every field to planning afresh (including what the plan's JSON leaves out); another Spec
    /// or planning function is another entry.
    #[test]
    fn a_cached_plan_is_the_plan_and_other_inputs_miss() {
        let dir =
            std::env::temp_dir().join(format!("draftwright-plan-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts");
        for file in ["mounting_plate.step", "turned.step", "scattered_holes.step"] {
            let (_, analysis) = model::analyse_native(&root.join(file)).unwrap();
            let spec = Spec::default();
            let fresh = model::plan_recovered(&analysis, &spec).unwrap();
            let first = plan_recovered_in(Some(&dir), &analysis, &spec).unwrap();
            let entry = dir.join(format!("{}.json", key("recovered", &analysis, &spec)));
            assert!(entry.is_file(), "{file}: the plan is stored");
            let second = plan_recovered_in(Some(&dir), &analysis, &spec).unwrap();
            assert_eq!(format!("{first:?}"), format!("{fresh:?}"), "{file}");
            assert_eq!(
                format!("{second:?}"),
                format!("{fresh:?}"),
                "{file}: read back"
            );
            assert!(
                fresh.views.iter().any(|v| !v.line_work.is_empty())
                    || !fresh.callout_tips.is_empty(),
                "{file}: the part exercises planning state the plan's JSON leaves out"
            );
            assert_ne!(
                key("plan", &analysis, &spec),
                key("recovered", &analysis, &spec)
            );
            let mut other = spec.clone();
            other.sheet.view_gutter = Some(3.0);
            assert_ne!(
                key("recovered", &analysis, &other),
                key("recovered", &analysis, &spec)
            );
            let direct = plan_in(Some(&dir), &analysis, &spec).unwrap();
            let direct_fresh = model::plan(&analysis, &spec).unwrap();
            assert_eq!(
                format!("{direct:?}"),
                format!("{direct_fresh:?}"),
                "{file}: plan"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
