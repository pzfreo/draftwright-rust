//! `draftwright-mcp`, driven as an MCP client drives it: JSON-RPC 2.0 lines through its stdin and
//! stdout.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn part(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts")
        .join(name)
        .display()
        .to_string()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcp");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The replies to *messages*, sent to one server after the initialisation handshake, by id.
fn session(messages: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_draftwright-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let handshake = [
        json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    ];
    for m in handshake.iter().chain(messages) {
        writeln!(stdin, "{m}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let replies: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // One reply to each request, none to the notification.
    assert_eq!(replies.len(), messages.len() + 1);
    let init = &replies[0]["result"];
    assert_eq!(init["protocolVersion"], "2025-06-18");
    assert_eq!(init["serverInfo"]["name"], "draftwright");
    assert!(init["capabilities"]["tools"].is_object());
    replies[1..].to_vec()
}

fn call(id: u32, tool: &str, arguments: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": { "name": tool, "arguments": arguments } })
}

/// A tool result's first content, read as JSON.
fn first(reply: &Value) -> Value {
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn the_tools_are_the_operations() {
    let replies = session(&[json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })]);
    let tools = replies[0]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, draftwright::api::OPERATIONS);
    for t in tools {
        assert!(t["description"].as_str().unwrap().len() > 40);
        assert_eq!(t["inputSchema"]["type"], "object");
        for required in t["inputSchema"]["required"].as_array().unwrap() {
            assert!(t["inputSchema"]["properties"][required.as_str().unwrap()].is_object());
        }
    }
    // plan, render and review take the analyse tool's Analysis, inline; plan then needs no step.
    for t in tools {
        let name = t["name"].as_str().unwrap();
        let schema = &t["inputSchema"];
        let takes = ["plan", "render", "review"].contains(&name);
        assert_eq!(
            schema["properties"]["analysis"].is_object(),
            takes,
            "{name}"
        );
        if takes {
            assert_eq!(schema["properties"]["analysis"]["type"], "object");
            let required = schema["required"].as_array().unwrap();
            assert_eq!(required.contains(&json!("step")), name != "plan", "{name}");
        }
    }
}

/// The analyse tool's result, passed back as `analysis`, gives the same plan and review as
/// analysing the STEP again; one made from another file is a tool error naming both digests.
#[test]
fn an_analysis_is_passed_back_in_place_of_analysing() {
    let step = part("turned.step");
    let replies = session(&[
        call(1, "analyse", json!({ "step": step })),
        call(2, "analyse", json!({ "step": part("prismatic.step") })),
    ]);
    let (analysis, other) = (first(&replies[0]), first(&replies[1]));
    let replies = session(&[
        call(1, "plan", json!({ "step": step })),
        call(2, "plan", json!({ "analysis": analysis })),
        call(3, "review", json!({ "step": step })),
        call(4, "review", json!({ "step": step, "analysis": analysis })),
        call(
            5,
            "render",
            json!({ "step": step, "analysis": other,
            "output": scratch("other").display().to_string() }),
        ),
    ]);
    assert_eq!(replies[0]["result"]["isError"], false);
    assert_eq!(replies[0]["result"], replies[1]["result"]);
    assert_eq!(replies[2]["result"]["isError"], false);
    assert_eq!(replies[2]["result"], replies[3]["result"]);
    assert_eq!(replies[4]["result"]["isError"], true);
    let error = &first(&replies[4])["error"];
    assert_eq!(error["code"], "invalid", "{error}");
    for digest in [&analysis, &other] {
        let sha = digest["binding"]["source_sha256"].as_str().unwrap();
        assert!(error["message"].as_str().unwrap().contains(sha), "{error}");
    }
}

#[test]
fn a_drawing_is_planned_edited_rendered_and_resumed() {
    let step = part("turned.step");
    let out = scratch("turned");
    let replies = session(&[
        call(1, "plan", json!({ "step": step })),
        call(
            2,
            "edit",
            json!({ "ops": [{ "op": "set", "path": "views/side/omit", "value": true }] }),
        ),
        call(
            3,
            "edit",
            json!({ "ops": [{ "op": "rename", "path": "sheet/size" }] }),
        ),
        call(
            4,
            "plan",
            json!({ "step": step, "spec": { "views": { "top": {} } } }),
        ),
        call(5, "draw", json!({})),
        json!({ "jsonrpc": "2.0", "id": 6, "method": "resources/list" }),
        json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" }),
    ]);
    let plan = first(&replies[0]);
    assert_eq!(replies[0]["result"]["isError"], false);
    assert_eq!(plan["views"].as_array().unwrap().len(), 4);
    let spec = first(&replies[1]);
    assert_eq!(spec["views"]["side"]["omit"], true);
    // Refusals are tool errors that say what would be accepted.
    assert_eq!(replies[2]["result"]["isError"], true);
    assert_eq!(
        first(&replies[2])["error"]["choices"],
        json!(["set", "remove"])
    );
    assert_eq!(replies[3]["result"]["isError"], true);
    let views = &first(&replies[3])["error"]["choices"];
    assert!(
        views.as_array().unwrap().contains(&json!("plan")),
        "{views}"
    );
    // An unknown tool or method is a protocol error.
    assert_eq!(replies[4]["error"]["code"], -32602);
    assert_eq!(replies[5]["error"]["code"], -32601);
    assert_eq!(replies[6]["result"], json!({}));

    let replies = session(&[
        call(
            1,
            "render",
            json!({ "step": step, "spec": spec,
            "output": out.display().to_string(), "formats": ["svg", "dxf", "pdf"] }),
        ),
        call(2, "review", json!({ "step": step, "spec": spec })),
    ]);
    let written = first(&replies[0])["written"].clone();
    assert_eq!(written.as_array().unwrap().len(), 3, "{}", replies[0]);
    let review = replies[1]["result"]["content"].as_array().unwrap();
    assert_eq!(review.len(), 2);
    assert!(review[1]["text"].as_str().unwrap().contains("<svg"));
    let findings: Value = serde_json::from_str(review[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(findings["lint"], json!([]), "{findings}");
    let reads: Vec<Value> = written
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, f)| call(i as u32 + 1, "spec", json!({ "drawing": f })))
        .collect();
    for reply in session(&reads) {
        let back = first(&reply);
        assert_eq!(back["views"]["side"]["omit"], true, "{reply}");
        assert_eq!(back["binding"]["part"], "turned");
    }
}

/// Geometry the kernel did not resolve is a tool error coded `failed`, naming the cause.
#[test]
fn unresolved_geometry_is_a_failed_tool_error() {
    let replies = session(&[call(
        1,
        "review",
        json!({ "step": part("unresolved.step") }),
    )]);
    assert_eq!(replies[0]["result"]["isError"], true);
    let error = &first(&replies[0])["error"];
    assert_eq!(error["code"], "failed", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("faces [3]"),
        "{error}"
    );
}
