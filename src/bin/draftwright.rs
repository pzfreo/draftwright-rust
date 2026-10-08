//! The command line over the document model (ADR R1), through [`draftwright::api`]. Each command
//! prints JSON.
//!
//! - `draftwright analyse part.step [--features r.json]`: the Analysis
//! - `draftwright plan part.step [--features r.json] [--spec s.json]`: the Plan; or
//!   `draftwright plan [part.step] --analysis a.json [--spec s.json]`, from an Analysis without
//!   analysing again
//! - `draftwright render part.step [--features r.json | --analysis a.json] [--spec s.json] -o out
//!   [--formats svg,dxf,pdf,png] [--dpi 150]`: writes the files (`out.svg`, …; svg, dxf and pdf
//!   unless named, a PNG at `--dpi` pixels per inch) and prints what was written, with the
//!   problems: the plan's, and what could not be drawn as planned
//! - `draftwright review part.step [--features r.json | --analysis a.json] [--spec s.json]`: the
//!   drawing as SVG
//!   text, with the problems and lint's findings on the drawing
//! - `draftwright edit s.json ops.json`: the edited Spec
//! - `draftwright spec drawing.(svg|dxf|pdf|png)`: the Spec embedded in a rendered drawing
//! - `draftwright serve`: the JSON-lines service. One request per line on stdin, such as
//!   `{"id": 1, "op": "plan", "step": "part.step", "spec": {…}}`, and one reply per line on
//!   stdout: `{"id": 1, "ok": true, "result": …}` or `{"id": 1, "ok": false, "error": {"code",
//!   "message", "choices"?}}`. `{"op": "hello"}` gives the protocol version and the operations;
//!   the others, and their arguments, are those of [`draftwright::api::call`].
//!
//! `--features` names Python quiddity's recognition JSON for the same file
//! (`tools/recognise.py`, read by [`draftwright::bridge`]). `--analysis` names an Analysis that
//! `draftwright analyse` printed for the same STEP file (with its features), used in place of
//! analysing the file again; it is refused with `--features`, or when made from another file.
//!
//! Exit codes: 0 done; 2 the request is invalid (the JSON error says what would be accepted);
//! 1 something failed and nothing was written in its place.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use draftwright::api::{self, Error};
use serde_json::{Value, json};

/// The service's protocol version, given by `hello`.
const PROTOCOL: u32 = 1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("serve") {
        return serve();
    }
    match run(&args) {
        Ok(out) => {
            println!("{}", serde_json::to_string_pretty(&out).expect("JSON"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "ok": false, "error": error }))
                    .expect("JSON")
            );
            ExitCode::from(if error.code == "invalid" { 2 } else { 1 })
        }
    }
}

/// The value after flag *name*, if given.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn read_json(path: &str) -> Result<Value, Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::invalid(format!("cannot read {path}: {e}")))?;
    serde_json::from_str(&text).map_err(|e| Error::invalid(format!("{path} is not valid: {e}")))
}

/// Runs the command, its arguments put as [`api::call`] takes them.
fn run(args: &[String]) -> Result<Value, Error> {
    let usage = "usage: draftwright (analyse|plan|render|review|edit|spec|serve) …";
    let command = args.first().ok_or_else(|| Error::invalid(usage))?;
    let arg = |i: usize| {
        args.get(i)
            .map(String::as_str)
            .ok_or_else(|| Error::invalid(usage))
    };
    let spec = match flag(args, "--spec") {
        Some(path) => read_json(path)?,
        None => Value::Null,
    };
    let features = flag(args, "--features");
    let analysis = match flag(args, "--analysis") {
        Some(path) => read_json(path)?,
        None => Value::Null,
    };
    let request = match command.as_str() {
        // From an Analysis, plan needs no STEP file: the first argument may be a flag.
        "plan" if !analysis.is_null() => json!({
            "step": args.get(1).filter(|a| !a.starts_with("--")),
            "features": features,
            "analysis": analysis,
            "spec": spec,
        }),
        "analyse" | "plan" | "review" => {
            json!({ "step": arg(1)?, "features": features, "analysis": analysis, "spec": spec })
        }
        "render" => json!({
            "step": arg(1)?,
            "features": features,
            "analysis": analysis,
            "spec": spec,
            "output": flag(args, "-o").ok_or_else(|| Error::invalid("render needs -o <output stem>"))?,
            "formats": flag(args, "--formats").unwrap_or("svg,dxf,pdf").split(',').collect::<Vec<_>>(),
            "dpi": flag(args, "--dpi").map(|d| d.parse::<f64>().map_err(|_| Error::invalid(format!("--dpi {d:?} is not a number")))).transpose()?,
        }),
        "edit" => json!({ "spec": read_json(arg(1)?)?, "ops": read_json(arg(2)?)? }),
        "spec" => json!({ "drawing": arg(1)? }),
        other => {
            return Err(Error::invalid(format!(
                "{other:?} is not a command. {usage}"
            )));
        }
    };
    api::call(command, &request)
}

/// Answers requests, one JSON object per line, until stdin closes.
fn serve() -> ExitCode {
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(request) => reply(&request),
            Err(e) => json!({ "id": null, "ok": false, "error": Error::invalid(e.to_string()) }),
        };
        let mut out = stdout.lock();
        if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// A request's reply: its result, or a structured error.
fn reply(request: &Value) -> Value {
    let Some(fields) = request.as_object() else {
        let error = Error::invalid("a request is a JSON object");
        return json!({ "id": null, "ok": false, "error": error });
    };
    let id = fields.get("id").cloned().unwrap_or(Value::Null);
    let result = match fields.get("op").and_then(Value::as_str) {
        Some("hello") => Ok(json!({ "protocol": PROTOCOL, "ops": api::OPERATIONS })),
        Some(op) => api::call(op, request),
        None => Err(Error::invalid("a request needs \"op\"")),
    };
    match result {
        Ok(result) => json!({ "id": id, "ok": true, "result": result }),
        Err(error) => json!({ "id": id, "ok": false, "error": error }),
    }
}
