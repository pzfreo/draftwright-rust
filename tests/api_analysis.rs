//! `plan`, `render` and `review` from an Analysis document (`docs/model.md`, "The five
//! operations"): the caller passes what `analyse` returned instead of analysing again. The output
//! is byte-identical to the re-analysing path's, and an Analysis that is not this file's, not of
//! this version, not well formed, or given beside features, is refused as `invalid`.

#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/facts.rs"]
mod facts;

use std::path::{Path, PathBuf};

use draftwright::api::{self, DPI};
use draftwright::model::{ANALYSIS_SCHEMA, Analysis, Spec};
use serde_json::{Value, json};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("api_analysis");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// A spec that changes the defaults, so the comparison covers more than the default sheet.
fn edited() -> Spec {
    serde_json::from_value(json!({ "views": { "iso": { "omit": true } },
        "sheet": { "title": { "title": "BRACKET" } } }))
    .unwrap()
}

/// Every output of *step* (with *features*) is the same from its Analysis as from re-analysis:
/// through the typed API, and through `api::call` with the Analysis carried inline.
fn same_from_analysis(step: &Path, features: Option<&Path>) {
    let analysis = api::analyse(step, features).unwrap();
    let formats = ["svg", "dxf", "pdf", "png"];
    for spec in [Spec::default(), edited()] {
        let plan = api::plan(step, features, &spec).unwrap();
        let from = api::plan_analysed(&analysis, &spec).unwrap();
        assert_eq!(
            serde_json::to_string(&plan).unwrap(),
            serde_json::to_string(&from).unwrap()
        );

        let (plan, files) = api::render_at(step, features, &spec, &formats, 40.0).unwrap();
        let (from, from_files) =
            api::render_analysed(step, &analysis, &spec, &formats, 40.0).unwrap();
        assert_eq!(
            serde_json::to_string(&plan).unwrap(),
            serde_json::to_string(&from).unwrap()
        );
        assert_eq!(files.len(), 4);
        assert_eq!(files, from_files, "{}", step.display());

        let review = api::review(step, features, &spec).unwrap();
        let from = api::review_analysed(step, &analysis, &spec).unwrap();
        assert_eq!(
            serde_json::to_string(&review).unwrap(),
            serde_json::to_string(&from).unwrap()
        );
    }

    // The same through `api::call`, as the adapters send it.
    let step_arg = step.display().to_string();
    let features_arg = features.map(|f| f.display().to_string());
    let document = api::call(
        "analyse",
        &json!({ "step": step_arg, "features": features_arg }),
    )
    .unwrap();
    let spec = serde_json::to_value(edited()).unwrap();
    let analysed = json!({ "step": step_arg, "features": features_arg, "spec": spec });
    let given = json!({ "step": step_arg, "analysis": document, "spec": spec });
    for op in ["plan", "review"] {
        assert_eq!(
            api::call(op, &analysed).unwrap(),
            api::call(op, &given).unwrap(),
            "{op}"
        );
    }
    let alone = json!({ "analysis": document, "spec": spec });
    assert_eq!(
        api::call("plan", &analysed).unwrap(),
        api::call("plan", &alone).unwrap()
    );
    let stem = step.file_stem().unwrap().to_string_lossy().into_owned();
    let mut written = Vec::new();
    for (args, out) in [(analysed, "analysed"), (given, "given")] {
        let mut args = args;
        let output = scratch(&format!("{stem}.{out}"));
        args["output"] = json!(output.display().to_string());
        let reply = api::call("render", &args).unwrap();
        let files: Vec<Vec<u8>> = reply["written"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| std::fs::read(f.as_str().unwrap()).unwrap())
            .collect();
        written.push((reply["problems"].clone(), files));
    }
    assert_eq!(written[0].1.len(), 3);
    assert_eq!(written[0], written[1]);
}

#[test]
fn a_part_with_recognition_draws_the_same_from_its_analysis() {
    let features = fixture("recognition/plate.json");
    let analysis = api::analyse(&fixture("parts/plate.step"), Some(&features)).unwrap();
    assert!(!analysis.features.is_empty());
    same_from_analysis(&fixture("parts/plate.step"), Some(&features));
}

#[test]
fn a_part_without_recognition_draws_the_same_from_its_analysis() {
    same_from_analysis(&fixture("parts/prismatic.step"), None);
}

/// A sampled corpus part with its recognition (GRM-03, a turned part with holes and chamfers).
#[test]
fn a_corpus_part_draws_the_same_from_its_analysis() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let step = dir.join(facts::load("grm03").binding.file);
    let features = scratch("grm03.recognition.json");
    std::fs::write(&features, corpus::recognition("grm03").unwrap()).unwrap();
    let analysis = api::analyse(&step, Some(&features)).unwrap();
    assert!(!analysis.features.is_empty());
    same_from_analysis(&step, Some(&features));
}

#[test]
fn an_analysis_of_another_file_is_refused() {
    let step = fixture("parts/plate.step");
    let other = fixture("parts/prismatic.step");
    let analysis = api::analyse(&other, None).unwrap();
    let theirs = analysis.binding.source_sha256.clone();
    let ours = api::analyse(&step, None).unwrap().binding.source_sha256;
    let refusals = [
        api::render_analysed(&step, &analysis, &Spec::default(), &["svg"], DPI).map(|_| ()),
        api::review_analysed(&step, &analysis, &Spec::default()).map(|_| ()),
        api::call(
            "plan",
            &json!({ "step": step, "analysis": serde_json::to_value(&analysis).unwrap() }),
        )
        .map(|_| ()),
        api::call(
            "review",
            &json!({ "step": step, "analysis": serde_json::to_value(&analysis).unwrap() }),
        )
        .map(|_| ()),
    ];
    for refusal in refusals {
        let e = refusal.unwrap_err();
        assert_eq!(e.code, "invalid", "{e}");
        assert!(e.message.contains(&theirs), "{e}");
        assert!(e.message.contains(&ours), "{e}");
    }

    // The same file read to a different face count is another reading, also refused.
    let mut miscounted = api::analyse(&step, None).unwrap();
    miscounted.binding.face_count += 1;
    let e = api::review_analysed(&step, &miscounted, &Spec::default()).unwrap_err();
    assert_eq!(e.code, "invalid", "{e}");
    assert!(e.message.contains("faces"), "{e}");
}

#[test]
fn an_analysis_of_another_version_is_refused() {
    let step = fixture("parts/prismatic.step");
    let mut analysis: Analysis = api::analyse(&step, None).unwrap();
    analysis.schema = "draftwright.analysis/99".into();
    for e in [
        api::render_analysed(&step, &analysis, &Spec::default(), &["svg"], DPI).unwrap_err(),
        api::review_analysed(&step, &analysis, &Spec::default()).unwrap_err(),
        api::plan_analysed(&analysis, &Spec::default()).unwrap_err(),
    ] {
        assert_eq!(e.code, "invalid", "{e}");
        assert!(e.message.contains("draftwright.analysis/99"), "{e}");
        assert!(e.choices.contains(&ANALYSIS_SCHEMA.to_string()), "{e:?}");
    }
    // Drawing combines the Analysis with a fresh reading of the part, so it takes only the
    // version this build writes (plan alone also reads version 2, as `model::plan` does).
    analysis.schema = "draftwright.analysis/2".into();
    let e = api::review_analysed(&step, &analysis, &Spec::default()).unwrap_err();
    assert_eq!(e.choices, [ANALYSIS_SCHEMA]);
}

#[test]
fn an_analysis_beside_features_is_refused() {
    let step = fixture("parts/plate.step");
    let features = fixture("recognition/plate.json");
    let analysis = api::call("analyse", &json!({ "step": step, "features": features })).unwrap();
    for op in ["plan", "render", "review"] {
        let e = api::call(
            op,
            &json!({ "step": step, "features": features, "analysis": analysis,
                "output": scratch("both").display().to_string() }),
        )
        .unwrap_err();
        assert_eq!(e.code, "invalid", "{op}: {e}");
        assert!(e.message.contains("not both"), "{op}: {e}");
    }
}

#[test]
fn a_malformed_analysis_is_refused() {
    let step = fixture("parts/prismatic.step");
    let mut document = api::call("analyse", &json!({ "step": step })).unwrap();
    document["unexpected"] = json!(1);
    let mut missing = document.clone();
    missing.as_object_mut().unwrap().remove("unexpected");
    missing.as_object_mut().unwrap().remove("binding");
    for bad in [document, missing, json!("analysis.json"), json!([1, 2])] {
        for op in ["plan", "render", "review"] {
            let e = api::call(
                op,
                &json!({ "step": step, "analysis": bad,
                    "output": scratch("malformed").display().to_string() }),
            )
            .unwrap_err();
            assert_eq!(e.code, "invalid", "{op}: {e}");
            assert!(
                e.message.starts_with("the analysis is not valid"),
                "{op}: {e}"
            );
        }
    }
    // Without an analysis or a step, plan says what it needs.
    let e = api::call("plan", &json!({})).unwrap_err();
    assert!(e.message.contains("step"), "{e}");
}

/// The plan of an Analysis with no STEP named reads no file, so it is the Analysis's alone.
#[test]
fn plan_from_an_analysis_needs_no_step() {
    let analysis = api::call("analyse", &json!({ "step": fixture("parts/turned.step") })).unwrap();
    let plan: Value = api::call("plan", &json!({ "analysis": analysis })).unwrap();
    assert_eq!(plan["schema"], "draftwright.plan/1");
    assert_eq!(plan["spec"]["binding"]["part"], "turned");
}

/// The command line's `--analysis` names an Analysis file that `draftwright analyse` printed.
#[test]
fn the_command_line_takes_an_analysis_file() {
    let run = |args: &[&str]| -> (Option<i32>, Value) {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_draftwright"))
            .args(args)
            .output()
            .unwrap();
        (
            out.status.code(),
            serde_json::from_slice(&out.stdout).unwrap(),
        )
    };
    let step = fixture("parts/plate.step").display().to_string();
    let features = fixture("recognition/plate.json").display().to_string();
    let (code, analysis) = run(&["analyse", &step, "--features", &features]);
    assert_eq!(code, Some(0));
    let file = scratch("plate.analysis.json");
    std::fs::write(&file, analysis.to_string()).unwrap();
    let file = file.display().to_string();
    let (_, plan) = run(&["plan", &step, "--features", &features]);
    assert_eq!(run(&["plan", "--analysis", &file]), (Some(0), plan.clone()));
    assert_eq!(run(&["plan", &step, "--analysis", &file]), (Some(0), plan));
    let (_, review) = run(&["review", &step, "--features", &features]);
    assert_eq!(
        run(&["review", &step, "--analysis", &file]),
        (Some(0), review)
    );
    let (code, both) = run(&[
        "review",
        &step,
        "--features",
        &features,
        "--analysis",
        &file,
    ]);
    assert_eq!(code, Some(2), "{both}");
    let other = fixture("parts/prismatic.step").display().to_string();
    let (code, refused) = run(&["review", &other, "--analysis", &file]);
    assert_eq!(code, Some(2), "{refused}");
    assert_eq!(refused["error"]["code"], "invalid");
}
