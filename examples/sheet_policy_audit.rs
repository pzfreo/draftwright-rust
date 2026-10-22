//! Audit every corpus/fixture default and captured single-sheet spec without changing guards.
//! Usage: cargo run --release --example sheet_policy_audit -- CORPUS OUT
use draftwright::haecceity::hlr::View;
use draftwright::{layout, model};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn inputs(root: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let name = format!("{prefix}/{}", path.file_name().unwrap().to_string_lossy());
        if path.is_dir() {
            inputs(&path, &name, out);
        } else if [".step", ".stp", ".step.gz", ".stp.gz"]
            .iter()
            .any(|s| name.to_lowercase().ends_with(s))
        {
            out.push((name, path));
        }
    }
}
fn small(a: &model::Analysis, p: &model::Plan) -> bool {
    let size = |v: &model::PlannedView| {
        let view = View::new(v.direction, v.up).unwrap();
        let [lo, hi] = a.envelope;
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for i in 0..8 {
            let p = view.map([0, 1, 2].map(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] }));
            for k in 0..2 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        [max[0] - min[0], max[1] - min[1]]
    };
    let views: Vec<_> = p.views.iter().filter(|v| v.id != "v:iso").collect();
    let row: f64 = views
        .iter()
        .filter(|v| v.id != "v:plan")
        .map(|v| size(v)[0] + 2. * layout::dim_pad())
        .sum();
    let column: f64 = views
        .iter()
        .filter(|v| v.id == "v:plan" || v.id == "v:front")
        .map(|v| size(v)[1] + 2. * layout::dim_pad())
        .sum();
    row <= layout::PAGES[1].1 - 2. * layout::MARGIN
        && column <= layout::PAGES[1].2 - 2. * layout::MARGIN
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();
    let mut files = Vec::new();
    inputs(Path::new(&args[1]), "corpus", &mut files);
    inputs(Path::new("tests/fixtures"), "fixtures", &mut files);
    files.sort();
    let saved: BTreeMap<String, Vec<model::Spec>> = serde_json::from_slice(
        &std::fs::read("tests/fixtures/known_single_sheet_specs.json").unwrap(),
    )
    .unwrap();
    let next = AtomicUsize::new(0);
    let rows = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4).map(|_| scope.spawn(|| {
            let mut rows = BTreeMap::<String, Value>::new();
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((name, original)) = files.get(i) else {break};
                let dir = out.join(format!("part-{i:03}")); std::fs::create_dir_all(&dir).unwrap();
                let path = if original.extension().is_some_and(|e| e == "gz") {
                    let mut bytes = Vec::new(); flate2::read::GzDecoder::new(std::fs::File::open(original).unwrap()).read_to_end(&mut bytes).unwrap();
                    let p = dir.join(original.file_stem().unwrap()); std::fs::write(&p, bytes).unwrap(); p
                } else {original.clone()};
                let (_, a) = match model::analyse_native(&path) {Ok(a) => a, Err(e) => {rows.insert(format!("{name}:default"), json!({"refused":e.message})); continue}};
                let mut specs = vec![("default".to_string(), model::Spec::default())];
                let stem = path.file_stem().unwrap().to_string_lossy();
                let identity = PathBuf::from(format!("tests/fixtures/identity/{stem}.spec1.json"));
                if identity.is_file() {specs.push(("spec1".into(), serde_json::from_slice(&std::fs::read(identity).unwrap()).unwrap()));}
                let layout_spec = PathBuf::from(format!("tests/fixtures/layout/{stem}_tolerances.spec.json"));
                if layout_spec.is_file() {
                    specs.push(("layout-spec".into(), serde_json::from_slice(&std::fs::read(layout_spec).unwrap()).unwrap()));
                }
                for s in saved.get(&a.binding.source_sha256).into_iter().flatten() {
                    if !specs.iter().any(|(_, old)| old == s) {specs.push((format!("spec:{}", hash(&serde_json::to_vec(s).unwrap())), s.clone()));}
                }
                // Classification uses the same full-size seed views as F1, before allocation.
                for (n, s) in specs {
                    let key = format!("{name}:{n}");
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let seed = model::plan_escalated(&a, &s)?;
                        let is_small = small(&a, &seed);
                        model::plan_recovered(&a, &s).map(|p| (is_small,p))
                    }));
                    let row = match result {
                        Ok(Ok((is_small,p))) => {
                            let large = ["A2","A1","A0"].contains(&p.sheet.size.as_str());
                            if is_small && large {
                                std::fs::write(dir.join(format!("{}.plan.json", n.replace(':',"-"))), serde_json::to_vec(&p).unwrap()).unwrap();
                            }
                            json!({"small":is_small,"size":p.sheet.size,"scale":p.sheet.scale,"sheets":1+p.sheets.len(),"spec":s,"path":path,"basis":p.sheet.basis,"problems":p.problems,"envelope":a.envelope})
                        }, Ok(Err(e)) => json!({"refused":e.message}), Err(_) => json!({"panicked":true})
                    };
                    rows.insert(key,row);
                }
                let part_rows: BTreeMap<_,_> = rows.iter().filter(|(key,_)| key.starts_with(&format!("{name}:"))).collect();
                std::fs::write(dir.join("cases.json"), serde_json::to_vec(&part_rows).unwrap()).unwrap();
                eprintln!("AUDIT {}/{} {name}",i+1,files.len());
            }
            rows
        })).collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect::<BTreeMap<_, _>>()
    });
    std::fs::write(
        out.join("audit.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
    eprintln!("{} files, {} cases", files.len(), rows.len());
}
