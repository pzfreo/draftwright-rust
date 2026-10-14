use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use draftwright::{api, model};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "corpus.rs"]
mod corpus;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn records(
    name: &str,
    path: &Path,
    saved: &BTreeMap<String, Vec<model::Spec>>,
) -> BTreeMap<String, Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = BTreeMap::new();
    let (part, analysis) = match model::analyse_native(path) {
        Ok(read) => read,
        Err(e) => {
            out.insert(
                format!("{name}:default"),
                json!({"refused": e.message.replace(path.to_str().unwrap(), name)}),
            );
            return out;
        }
    };
    let mut specs = vec![("default".to_string(), model::Spec::default())];
    let stem = path.file_stem().unwrap().to_string_lossy();
    let fixture = root.join(format!("tests/fixtures/identity/{stem}.spec1.json"));
    if fixture.is_file() {
        specs.push((
            "spec1".into(),
            serde_json::from_slice(&std::fs::read(fixture).unwrap()).unwrap(),
        ));
    }
    if let Some(tested) = saved.get(&analysis.binding.source_sha256) {
        for spec in tested {
            if !specs.iter().any(|(_, s)| s == spec) {
                specs.push((
                    format!("spec:{}", hash(&serde_json::to_vec(spec).unwrap())),
                    spec.clone(),
                ));
            }
        }
    }
    let font =
        draftwright::text::Font::open(&root.join("fonts").join(draftwright::dims::FONT)).unwrap();
    let mut projections = model::Projections::default();
    for (spec_name, spec) in specs {
        let key = format!("{name}:{spec_name}");
        let planned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            model::plan_recovered(&analysis, &spec)
        }));
        let plan = match planned {
            Ok(Ok(plan)) => plan,
            Ok(Err(e)) => {
                out.insert(key, json!({"refused": e.message, "choices": e.choices}));
                continue;
            }
            Err(e) => {
                let message = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .expect("string panic");
                eprintln!("BASELINE_PANIC {key}: {message}");
                out.insert(key, json!({"panicked": message}));
                continue;
            }
        };
        let drawing = model::render_with(&part, &analysis, &plan, &font, &mut projections).unwrap();
        let pdf = drawing.pdf(&|font| std::fs::read(root.join("fonts").join(font)).ok());
        let png = drawing.png(api::DPI).unwrap();
        out.insert(
            key,
            json!({
                "plan": hash(&serde_json::to_vec(&plan).unwrap()),
                "svg": hash(drawing.svg().as_bytes()), "pdf": hash(&pdf),
                "png": { std::env::consts::OS: hash(&png) }
            }),
        );
    }
    eprintln!("SINGLE_SHEET {name}: {} specs", out.len());
    out
}

pub fn compare() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let record = std::env::var("DRAFTWRIGHT_RECORD_SINGLE_SHEET").as_deref() == Ok("1");
    let Some(corpus) = corpus::dir() else {
        assert!(!record, "Recording needs the complete corpus");
        eprintln!("SKIPPED: single-sheet corpus hashes; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("single_sheet");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut inputs: BTreeMap<String, PathBuf> =
        std::fs::read_to_string(root.join("tools/quality_parts.txt"))
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .map(|s| (format!("corpus/{s}"), corpus.join(s)))
            .collect();
    // The editor slice's 10127 is outside the thirty quality parts.
    inputs.insert(
        "corpus/mfcadpp/10127.step".into(),
        corpus.join("mfcadpp/10127.step"),
    );
    for dir in ["parts", "pmi/step"] {
        for e in std::fs::read_dir(root.join("tests/fixtures").join(dir)).unwrap() {
            let path = e.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.ends_with(".step") || name.ends_with(".stp") {
                inputs.insert(format!("{dir}/{name}"), path);
            } else if let Some(file) = name.strip_suffix(".gz") {
                let mut bytes = Vec::new();
                flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
                    .read_to_end(&mut bytes)
                    .unwrap();
                let target = scratch.join(file);
                std::fs::write(&target, bytes).unwrap();
                inputs.insert(format!("{dir}/{file}"), target);
            }
        }
    }
    let saved: BTreeMap<String, Vec<model::Spec>> = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/known_single_sheet_specs.json")).unwrap(),
    )
    .unwrap();
    let inputs: Vec<_> = inputs.into_iter().collect();
    let next = AtomicUsize::new(0);
    let mut found = BTreeMap::new();
    std::thread::scope(|s| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                s.spawn(|| {
                    let mut out = BTreeMap::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some((name, path)) = inputs.get(i) else {
                            break;
                        };
                        out.extend(records(name, path, &saved));
                    }
                    out
                })
            })
            .collect();
        for worker in workers {
            found.extend(worker.join().unwrap());
        }
    });
    let fixture = root.join("tests/fixtures/known_single_sheet.json");
    let known: BTreeMap<String, Value> =
        serde_json::from_slice(&std::fs::read(&fixture).unwrap()).unwrap();
    // Current main's canonical-pin repair intentionally changes some pre-I1 pinned sheets.
    // Keep the original manifest; these exact replacements were recorded from d1193921
    // independently of I1, and all other cases still compare against the original bytes.
    let pin_fixture = root.join("tests/fixtures/known_single_sheet_main_pins.json");
    let mut pins: Value = serde_json::from_slice(&std::fs::read(&pin_fixture).unwrap()).unwrap();
    assert_eq!(
        pins["upstream_commit"],
        "d1193921abbe3ab9035f54df1e298beb042a7569"
    );
    let mut portable = known.clone();
    for row in portable.values_mut() {
        row.as_object_mut().unwrap().remove("png");
    }
    assert_eq!(
        hash(&serde_json::to_vec(&portable).unwrap()),
        pins["base_portable_sha256"]
    );
    let adjustments: BTreeMap<String, Value> =
        serde_json::from_value(pins["cases"].clone()).unwrap();
    let pinned_specs: std::collections::BTreeSet<_> = saved
        .values()
        .flatten()
        .filter(|spec| {
            let value = serde_json::to_value(spec).unwrap();
            ["dimensions", "annotations", "views"].iter().any(|name| {
                value[name]
                    .as_object()
                    .is_some_and(|entries| entries.values().any(|e| e["place"]["pinned"] == true))
            }) && value
                .get("hole_table")
                .is_none_or(|policy| policy == "auto")
        })
        .map(|spec| hash(&serde_json::to_vec(spec).unwrap()))
        .collect();
    let mut expected = known.clone();
    for (name, replacement) in &adjustments {
        assert!(
            known.contains_key(name),
            "upstream added a baseline case: {name}"
        );
        assert!(
            pinned_specs.contains(name.rsplit(":spec:").next().unwrap()),
            "upstream replacement is not an automatic pinned schedule: {name}"
        );
        expected.insert(name.clone(), replacement.clone());
    }
    // Deliberate drawing changes merged after I1 (e.g. L1's leader refinement) replace exact
    // expectations case by case; every format of every case is still compared exactly.
    let approved_fixture = root.join("tests/fixtures/known_single_sheet_approved.json");
    let mut approved: Value =
        serde_json::from_slice(&std::fs::read(&approved_fixture).unwrap()).unwrap();
    let approved_cases: BTreeMap<String, Value> =
        serde_json::from_value(approved["cases"].clone()).unwrap();
    let per_stream: u64 = approved["streams"]
        .as_object()
        .unwrap()
        .values()
        .map(|s| s["cases"].as_u64().unwrap())
        .sum();
    assert_eq!(per_stream, approved_cases.len() as u64);
    for (name, replacement) in &approved_cases {
        assert!(
            known.contains_key(name),
            "approved replacement for an unknown case: {name}"
        );
        expected.insert(name.clone(), replacement.clone());
    }
    if !record {
        assert_eq!(
            found.keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        );
    }
    for (name, old) in &expected {
        let current = found
            .get(name)
            .unwrap_or_else(|| panic!("lost baseline case {name}"));
        if old.get("refused").is_some() || old.get("panicked").is_some() {
            assert_eq!(current, old, "{name}: refusal");
            continue;
        }
        for format in ["plan", "svg", "pdf"] {
            assert_eq!(current[format], old[format], "{name}: {format}");
        }
        if let Some(png) = old["png"].get(std::env::consts::OS) {
            assert_eq!(&current["png"][std::env::consts::OS], png, "{name}: PNG");
        }
    }
    if record {
        for (name, old) in known {
            if approved_cases.contains_key(&name) {
                let current = &found[&name]["png"];
                let prior = approved["cases"][&name]["png"].as_object_mut().unwrap();
                for (platform, value) in current.as_object().unwrap() {
                    prior.insert(platform.clone(), value.clone());
                }
                found.insert(name, old);
                continue;
            }
            if adjustments.contains_key(&name) {
                let current = &found[&name]["png"];
                let prior = pins["cases"][&name]["png"].as_object_mut().unwrap();
                for (platform, value) in current.as_object().unwrap() {
                    prior.insert(platform.clone(), value.clone());
                }
                // Linux recording extends the upstream manifest, never rewrites base bytes.
                found.insert(name, old);
                continue;
            }
            if let Some(prior) = old.get("png").and_then(Value::as_object)
                && let Some(current) = found
                    .get_mut(&name)
                    .and_then(|r| r.get_mut("png"))
                    .and_then(Value::as_object_mut)
            {
                for (platform, hash) in prior {
                    current
                        .entry(platform.clone())
                        .or_insert_with(|| hash.clone());
                }
            }
        }
        std::fs::write(
            &pin_fixture,
            serde_json::to_string_pretty(&pins).unwrap() + "\n",
        )
        .unwrap();
        std::fs::write(
            &approved_fixture,
            serde_json::to_string_pretty(&approved).unwrap() + "\n",
        )
        .unwrap();
        std::fs::write(
            fixture,
            serde_json::to_string_pretty(&found).unwrap() + "\n",
        )
        .unwrap();
    }
}
