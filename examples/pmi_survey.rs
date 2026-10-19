//! Reproducible issue-122 survey input/output; reports go to the requested external path.
use draftwright::model::{self, Spec};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};
fn marks(a: &model::Analysis, p: &model::Plan) -> Vec<draftwright::lint::Mark> {
    let mut marks = Vec::new();
    for d in p
        .dimensions
        .iter()
        .filter(|d| d.placed.is_some() && !d.reference)
    {
        let mut claims: Vec<_> = d
            .measurements
            .iter()
            .map(|m| draftwright::output::Claim {
                feature: m.feature.clone(),
                parameter: m.parameter.clone(),
            })
            .collect();
        if d.kind == "linear"
            && let Some(view) = p.views.iter().find(|v| v.id == d.view)
        {
            // The survey uses principal views; detail evidence is left to the native planner audit.
            if view.detail.is_none() {
                claims.retain(|c| !c.feature.starts_with("station_span:"));
                claims.push(draftwright::output::Claim {
                    feature: "station_ink".into(),
                    parameter: "witnesses".into(),
                });
                let frame = draftwright::output::DrawnFrame {
                    direction: view.direction,
                    up: view.up,
                    at: view.at,
                    scale: view.scale,
                };
                if let Some((k, x, y)) = draftwright::lint::location::ink_span(
                    a,
                    &frame,
                    &d.placed.as_ref().unwrap().lines,
                ) {
                    claims.push(draftwright::output::Claim {
                        feature: format!("station_span:{k}|{x:.9}|{y:.9}"),
                        parameter: "station.distance".into(),
                    });
                }
            }
        }
        marks.push(draftwright::lint::Mark {
            name: d.id.clone(),
            view: Some(d.view.clone()),
            text: d.label.clone(),
            claims,
        });
    }
    for table in p
        .hole_table
        .iter()
        .chain(&p.hole_tables)
        .filter(|t| t.blocks.iter().any(|b| b.placed.is_some()))
    {
        let origin = table.origin.unwrap_or(a.envelope[0]);
        for row in &table.rows {
            for m in &row.measurements {
                let Some(k) = ["x", "y", "z"]
                    .iter()
                    .position(|axis| m.parameter == format!("location.length.{axis}"))
                else {
                    continue;
                };
                let Some(at) = a.features.iter().find(|f| f.id == m.feature).and_then(|f| {
                    draftwright::bridge::holes::Hole::read(f, &draftwright::bridge::holes::frame(a))
                        .ok()
                        .map(|h| h.at[k])
                }) else {
                    continue;
                };
                let value = at - origin[k];
                marks.push(draftwright::lint::Mark {
                    name: format!("{}/{}/{k}", table.id, row.tag),
                    view: table.view.clone(),
                    text: format!("{value}"),
                    claims: vec![
                        draftwright::output::Claim {
                            feature: m.feature.clone(),
                            parameter: m.parameter.clone(),
                        },
                        draftwright::output::Claim {
                            feature: "station_ink".into(),
                            parameter: "coordinate".into(),
                        },
                        draftwright::output::Claim {
                            feature: format!("station_span:{k}|{:.9}|{at:.9}", origin[k]),
                            parameter: "station.distance".into(),
                        },
                    ],
                });
            }
        }
    }
    marks
}
fn measure(a: &model::Analysis, name: &str, out: &Path, repeats: usize) {
    eprintln!("{name}: planning");
    let mut elapsed = Vec::new();
    let mut last = None;
    for _ in 0..repeats {
        let start = Instant::now();
        let p = model::plan_recovered(a, &Spec::default()).unwrap();
        elapsed.push(start.elapsed().as_secs_f64());
        last = Some(p);
    }
    elapsed.sort_by(f64::total_cmp);
    let seconds = elapsed[elapsed.len() / 2];
    std::fs::write(out.join(format!("{name}.json")),serde_json::to_vec(&serde_json::json!({"analysis":a,"plan":last.unwrap(),"seconds":seconds,"runs":elapsed})).unwrap()).unwrap();
    eprintln!("{name}: {seconds:.4}s");
}
fn compare(before: &Path, after: &Path) {
    let mut report = Vec::new();
    let mut paths: Vec<_> = std::fs::read_dir(before)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| !p.to_str().unwrap().ends_with(".analysis.json"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let b: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let n: serde_json::Value =
            serde_json::from_slice(&std::fs::read(after.join(path.file_name().unwrap())).unwrap())
                .unwrap();
        let a: model::Analysis = serde_json::from_value(b["analysis"].clone()).unwrap();
        let old: model::Plan = serde_json::from_value(b["plan"].clone()).unwrap();
        let new: model::Plan = serde_json::from_value(n["plan"].clone()).unwrap();
        if !matches!(&a.pmi,Some(draftwright::pmi::Reading::Read(r)) if !r.records.is_empty()) {
            assert_eq!(
                old.dimensions, new.dimensions,
                "{name}: no-PMI dimension identity changed"
            );
            assert_eq!(
                serde_json::to_value(&old.views).unwrap(),
                serde_json::to_value(&new.views).unwrap(),
                "{name}: no-PMI view identity changed"
            );
        }
        let previous = draftwright::lint::location::coverage(&a, &marks(&a, &old));
        let current = draftwright::lint::location::coverage(&a, &marks(&a, &new));
        let missing = |issues: &[draftwright::lint::LintIssue]| {
            issues
                .iter()
                .filter(|i| i.code == "feature_not_located")
                .map(|i| i.id.clone())
                .collect::<std::collections::BTreeSet<_>>()
        };
        let lost: Vec<_> = missing(&current)
            .difference(&missing(&previous))
            .cloned()
            .collect();
        let removed: Vec<_> = old
            .dimensions
            .iter()
            .filter(|d| {
                !d.id.starts_with("d:pmi:")
                    && !new.dimensions.iter().any(|n| n.id == d.id)
                    && new.omitted.iter().any(|o| {
                        o.id == d.id && matches!(o.state, draftwright::items::State::Covered { .. })
                    })
            })
            .map(|d| d.id.clone())
            .collect();
        let origins: Vec<_> = new
            .dimensions
            .iter()
            .filter(|d| {
                d.basis.contains("measured from")
                    && old
                        .dimensions
                        .iter()
                        .any(|b| b.id == d.id && (b.value - d.value).abs() > 1e-6)
            })
            .map(|d| d.id.clone())
            .collect();
        let count = |issues: &[draftwright::lint::LintIssue], code: &str| {
            issues.iter().filter(|i| i.code == code).count()
        };
        let row = serde_json::json!({"part":name,"removed_as_covered":removed,"origins_changed":origins,"not_located":[count(&previous,"feature_not_located"),count(&current,"feature_not_located")],"located_twice":[count(&previous,"feature_located_twice"),count(&current,"feature_located_twice")],"lost_locations":lost,"seconds":[b["seconds"],n["seconds"]]});
        println!("{row}");
        report.push(row);
    }
    assert!(
        report
            .iter()
            .all(|r| r["lost_locations"].as_array().unwrap().is_empty()),
        "survey lost controlling locations"
    );
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[1] == "--compare" {
        compare(Path::new(&args[2]), Path::new(&args[3]));
        return;
    }
    if args[1] == "--replay" {
        let before = Path::new(&args[2]);
        let out = Path::new(&args[3]);
        std::fs::create_dir_all(out).unwrap();
        let repeats = args.get(4).map_or(3, |s| s.parse().unwrap());
        let mut paths: Vec<_> = std::fs::read_dir(before)
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| !p.to_str().unwrap().ends_with(".analysis.json"))
            .collect();
        paths.sort();
        for path in paths {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let a: model::Analysis = serde_json::from_value(value["analysis"].clone()).unwrap();
            measure(
                &a,
                path.file_stem().unwrap().to_str().unwrap(),
                out,
                repeats,
            );
        }
        return;
    }
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();
    let mut inputs = vec![
        (
            "cgb243".to_string(),
            root.join("cadgenbench_inputs/cgb243.step.gz"),
        ),
        (
            "threaded_connector_109".into(),
            root.join("cadgenbench/threaded_connector_109.step"),
        ),
    ];
    for n in ["06", "09", "10"] {
        let rev = if n == "10" { "rb" } else { "rd" };
        inputs.push((
            format!("ftc{n}"),
            root.join(format!("nist/nist_ftc_{n}_asme1_{rev}.stp")),
        ));
    }
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pmi/step");
    for p in std::fs::read_dir(fixture).unwrap() {
        let path = p.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if name.starts_with("nist_ftc")
            || name.starts_with("nist_ctc")
            || name.starts_with("GRM")
            || name.starts_with("bracket_positions")
        {
            inputs.push((name.split('.').next().unwrap().into(), path));
        }
    }
    let repeats: usize = args.get(3).map_or(3, |s| s.parse().unwrap());
    for (name, path) in inputs {
        let raw = std::fs::read(path).unwrap();
        let bytes = if raw.starts_with(&[31, 139]) {
            let mut b = vec![];
            flate2::read::GzDecoder::new(&raw[..])
                .read_to_end(&mut b)
                .unwrap();
            b
        } else {
            raw
        };
        eprintln!("{name}: analysing");
        let (_, a) =
            model::analyse_native_step(Path::new(&format!("{name}.step")), &bytes).unwrap();
        measure(&a, &name, &out, repeats);
    }
}
