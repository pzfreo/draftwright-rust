//! The command line over the document model (ADR R1). Each command prints JSON.
//!
//! - `draftwright analyse part.step`: the Analysis
//! - `draftwright plan part.step [--spec s.json]`: the Plan
//! - `draftwright render part.step [--spec s.json] -o out [--formats svg,dxf,pdf]`: writes the
//!   files and prints what was written, with the plan's problems
//! - `draftwright edit s.json ops.json`: the edited Spec
//! - `draftwright spec drawing.(svg|pdf)`: the Spec embedded in a rendered drawing
//!
//! Exit codes: 0 done; 2 the request is invalid (the JSON error says what would be accepted);
//! 1 something failed and nothing was written in its place.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use draftwright::model::{self, Spec};
use draftwright::text::Font;
use serde_json::json;

const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            println!("{}", serde_json::to_string_pretty(&out).expect("JSON"));
            ExitCode::SUCCESS
        }
        Err((code, error)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "ok": false, "error": error }))
                    .expect("JSON")
            );
            ExitCode::from(code)
        }
    }
}

type Failure = (u8, serde_json::Value);

fn invalid(message: impl Into<String>) -> Failure {
    (2, json!({ "code": "invalid", "message": message.into() }))
}

fn failed(message: impl Into<String>) -> Failure {
    (1, json!({ "code": "failed", "message": message.into() }))
}

/// The value after flag *name*, if given.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Failure> {
    let text =
        std::fs::read_to_string(path).map_err(|e| invalid(format!("cannot read {path}: {e}")))?;
    serde_json::from_str(&text).map_err(|e| invalid(format!("{path} is not valid: {e}")))
}

fn spec_of(args: &[String]) -> Result<Spec, Failure> {
    match flag(args, "--spec") {
        Some(path) => read_json(path),
        None => Ok(Spec::default()),
    }
}

fn run(args: &[String]) -> Result<serde_json::Value, Failure> {
    let usage = "usage: draftwright (analyse|plan|render|edit|spec) …";
    let command = args.first().ok_or_else(|| invalid(usage))?;
    let arg = |i: usize| {
        args.get(i)
            .map(String::as_str)
            .ok_or_else(|| invalid(usage))
    };
    match command.as_str() {
        "analyse" => {
            let (_, analysis) = model::analyse(Path::new(arg(1)?)).map_err(invalid)?;
            Ok(serde_json::to_value(analysis).expect("JSON"))
        }
        "plan" => {
            let (_, analysis) = model::analyse(Path::new(arg(1)?)).map_err(invalid)?;
            let plan = model::plan(&analysis, &spec_of(args)?)
                .map_err(|e| (2, serde_json::to_value(&e).expect("JSON")))?;
            Ok(serde_json::to_value(plan).expect("JSON"))
        }
        "render" => {
            let (part, analysis) = model::analyse(Path::new(arg(1)?)).map_err(invalid)?;
            let plan = model::plan(&analysis, &spec_of(args)?)
                .map_err(|e| (2, serde_json::to_value(&e).expect("JSON")))?;
            let out = PathBuf::from(
                flag(args, "-o").ok_or_else(|| invalid("render needs -o <output stem>"))?,
            );
            let formats: Vec<&str> = flag(args, "--formats")
                .unwrap_or("svg,dxf,pdf")
                .split(',')
                .collect();
            let font = Font::new(MONO.to_vec()).expect("the bundled font parses");
            let drawing = model::render(&part, &analysis, &plan, &font);
            let mut written = Vec::new();
            for format in formats {
                let bytes = match format {
                    "svg" => drawing.svg().into_bytes(),
                    "dxf" => drawing.dxf(),
                    "pdf" => drawing
                        .pdf(&|name| (name == "IBMPlexMono-Regular.ttf").then(|| MONO.to_vec())),
                    other => {
                        return Err(invalid(format!(
                            "{other:?} is not a format; choose from svg, dxf, pdf"
                        )));
                    }
                };
                let path = out.with_extension(format);
                // Written beside, then moved into place: a failure leaves any earlier file as it was.
                let staged = out.with_extension(format!("{format}.part"));
                std::fs::write(&staged, &bytes)
                    .map_err(|e| failed(format!("cannot write {}: {e}", staged.display())))?;
                std::fs::rename(&staged, &path)
                    .map_err(|e| failed(format!("cannot write {}: {e}", path.display())))?;
                written.push(path.display().to_string());
            }
            Ok(json!({ "written": written, "problems": plan.problems }))
        }
        "edit" => {
            let spec: Spec = read_json(arg(1)?)?;
            let ops: serde_json::Value = read_json(arg(2)?)?;
            let edited = model::edit(&spec, &ops)
                .map_err(|e| (2, serde_json::to_value(&e).expect("JSON")))?;
            Ok(serde_json::to_value(edited).expect("JSON"))
        }
        "spec" => {
            let bytes = std::fs::read(arg(1)?)
                .map_err(|e| invalid(format!("cannot read {}: {e}", arg(1).unwrap_or(""))))?;
            let spec = model::embedded_spec(&bytes)
                .ok_or_else(|| invalid("the file carries no draftwright spec"))?;
            Ok(serde_json::to_value(spec).expect("JSON"))
        }
        other => Err(invalid(format!("{other:?} is not a command. {usage}"))),
    }
}
