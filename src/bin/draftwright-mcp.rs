//! The MCP server over the document model (ADR R1): MCP over stdio, one JSON-RPC 2.0 message per
//! line, with one tool for each operation of [`draftwright::api`]. Results are their documents as
//! JSON text (and, from `review`, the drawing as SVG text); an invalid request is a tool error
//! carrying what would be accepted.

use std::io::{BufRead, Write};

use draftwright::api;
use serde_json::{Value, json};

/// The MCP revisions spoken, newest first. Tools are the same in each.
const VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Draftwright makes engineering drawings from STEP files. A drawing \
is described by a spec (the author's decisions; JSON) and previewed as a plan (every view, the \
sheet and scale, each with whether it is a default and why, and any problems). Nothing is kept \
between calls: pass the spec to each call that takes one, and keep the latest. Usual order: plan \
the STEP with no spec to see the defaults; edit the spec to change what is wrong; plan or review \
again; render to write the files. A rendered drawing carries its spec, which the spec tool reads \
back, so work can resume from a drawing alone.";

const SPEC: &str = "The spec: the author's decisions, keyed by stable IDs. Omit it, or pass {}, \
for all defaults. Use the spec from the plan or the edit tool's result. Shape: {\"sheet\": \
{\"size\": \"A3\", \"scale\": \"2:1\", \"title\": {\"title\": \"BRACKET\", \"number\": \
\"DWG-001\", \"material\": \"6082-T6\"}}, \"views\": {\"iso\": {\"omit\": true}}, \
\"dimension_source\": \"auto\", \"dimensions\": {\"d:envelope/width.length@front\": {\"omit\": \
true}}}. Unknown fields are refused.";

/// The tools: name, description, input schema.
fn tools() -> Value {
    let path = |what: &str| json!({ "type": "string", "description": what });
    let spec = json!({ "type": "object", "description": SPEC });
    let features = path(
        "Path of Python quiddity's feature recognition JSON for the same STEP file \
         (tools/recognise.py); without it the analysis has no features.",
    );
    json!([
        {
            "name": "analyse",
            "description": "Read a STEP file and return its Analysis: the binding (the file's \
                SHA-256, the part's name and face count) and the envelope (the part's box, mm). \
                It is regenerated from the STEP, never edited.",
            "inputSchema": {
                "type": "object",
                "properties": { "step": path("Path of the STEP file."), "features": features },
                "required": ["step"],
            },
        },
        {
            "name": "plan",
            "description": "Plan the drawing of a STEP file under a spec, without drawing it: \
                the sheet (size, page, scale, title block), each view (id, direction, centre on \
                the page) and problems. Every item says whether it is a default and gives its \
                basis, the reason it was chosen. The plan's \"spec\" is the spec bound to this \
                STEP file; keep it for later calls.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "step": path("Path of the STEP file."),
                    "features": features,
                    "spec": spec,
                },
                "required": ["step"],
            },
        },
        {
            "name": "edit",
            "description": "Apply edits to a spec and return the edited spec. Nothing is \
                stored: pass the result to plan, review or render. Each edit is {\"op\": \"set\", \
                \"path\": …, \"value\": …} or {\"op\": \"remove\", \"path\": …}, with a path into \
                the spec: sheet/size (A4, A3, A2, A1, A0), sheet/scale (ISO 5455, e.g. 2:1, 1:1, \
                1:2), sheet/title/<field> (title, number, material, …), views/<front|plan|side|iso>/omit \
                (true leaves the view out), dimension_source (auto: the defaults less what is \
                omitted; authored: exactly the dimensions listed), dimensions/<id>/omit (true \
                leaves that dimension out; IDs are the plan's, d:<feature>/<parameter>@<view>, \
                with each / inside the ID written ~1, e.g. \
                dimensions/d:envelope~1width.length@front/omit). A dimension's ID names the view \
                that carries it, so omitting that view renames it: take IDs from the latest plan. \
                A refused edit says why and, where it chose from a list, the choices; plan \
                validates the values against the part.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "spec": spec,
                    "ops": {
                        "type": "array",
                        "description": "The edits, applied in order.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "op": { "type": "string", "enum": ["set", "remove"] },
                                "path": { "type": "string", "description": "Slash-separated, e.g. sheet/scale." },
                                "value": { "description": "The new value, for set." },
                            },
                            "required": ["op", "path"],
                        },
                    },
                },
                "required": ["ops"],
            },
        },
        {
            "name": "render",
            "description": "Draw a STEP file under a spec and write the drawing beside the \
                output stem, one file per format (out.svg, out.dxf, out.pdf). Each file embeds \
                the spec. Returns the paths written and the problems (the plan's, and what could not \
                be drawn as planned).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "step": path("Path of the STEP file."),
                    "features": features,
                    "spec": spec,
                    "output": path("Output path; each format's extension is appended (a drawing format's extension \
                    already on it is replaced)."),
                    "formats": {
                        "type": "array",
                        "items": { "type": "string", "enum": api::FORMATS },
                        "description": "Which formats to write; all of them if omitted.",
                    },
                },
                "required": ["step", "output"],
            },
        },
        {
            "name": "review",
            "description": "Draw a STEP file under a spec without writing a file, to check it by \
                eye: returns the problems (the plan's, and what could not be drawn as planned) and \
                lint's findings as JSON, then the drawing as \
                SVG text (page in mm; layers part, hidden and dims). Lint judges the drawn sheet \
                against the part alone: each finding names the features (subjects), what is \
                wrong, and a fix.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "step": path("Path of the STEP file."),
                    "features": features,
                    "spec": spec,
                },
                "required": ["step"],
            },
        },
        {
            "name": "spec",
            "description": "Read back the spec a drawing (SVG, DXF or PDF) was made from, to \
                resume work on it.",
            "inputSchema": {
                "type": "object",
                "properties": { "drawing": path("Path of a drawing that draftwright rendered.") },
                "required": ["drawing"],
            },
        },
    ])
}

fn main() {
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => answer(&message),
            Err(e) => Some(failure(Value::Null, -32700, &format!("Parse error: {e}"))),
        };
        if let Some(reply) = reply {
            let mut out = stdout.lock();
            if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
}

fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The reply to one message; none to a notification.
fn answer(message: &Value) -> Option<Value> {
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        return Some(failure(
            id,
            -32600,
            "Invalid request: a JSON-RPC 2.0 object with a method",
        ));
    };
    // A notification (no id), such as notifications/initialized, needs no reply.
    let id = message.get("id")?.clone();
    let params = message.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => {
            let asked = params["protocolVersion"].as_str().unwrap_or("");
            let version = VERSIONS
                .iter()
                .find(|v| **v == asked)
                .unwrap_or(&VERSIONS[0]);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "draftwright", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            if !api::OPERATIONS.contains(&name) {
                return Some(failure(id, -32602, &format!("Unknown tool: {name:?}")));
            }
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let text = |x: &Value| json!({ "type": "text", "text": serde_json::to_string_pretty(x).expect("JSON") });
            match api::call(name, &args) {
                Ok(result) if name == "review" => json!({
                    "content": [
                        text(&json!({ "problems": result["problems"], "lint": result["lint"] })),
                        { "type": "text", "text": result["svg"] },
                    ],
                    "isError": false,
                }),
                Ok(result) => json!({ "content": [text(&result)], "isError": false }),
                Err(error) => json!({
                    "content": [text(&json!({ "error": error }))],
                    "isError": true,
                }),
            }
        }
        other => return Some(failure(id, -32601, &format!("Method not found: {other}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}
