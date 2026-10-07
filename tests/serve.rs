//! `draftwright serve`, driven as a host drives it: JSON lines through its stdin and stdout.

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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("serve");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The replies to *lines*, sent to one `draftwright serve` until its stdin closes.
fn serve(lines: &[String]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_draftwright"))
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for line in lines {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn the_service_answers_each_line() {
    let step = part("prismatic.step");
    let out = scratch("prismatic");
    let edit = json!({ "id": 3, "op": "edit", "spec": {},
        "ops": [{ "op": "set", "path": "views/iso/omit", "value": true },
                { "op": "set", "path": "sheet/title/material", "value": "Stahl ß ]]>" }] });
    let replies = serve(&[
        json!({ "id": 1, "op": "hello" }).to_string(),
        "not json".into(),
        json!({ "id": 2, "op": "plan", "step": step }).to_string(),
        edit.to_string(),
        json!({ "id": 4, "op": "edit", "spec": {},
            "ops": [{ "op": "rename", "path": "sheet/size" }] })
        .to_string(),
        json!({ "id": 5, "op": "plan", "step": step,
            "spec": { "sheet": { "scale": "3:1" } } })
        .to_string(),
        json!({ "id": 6, "op": "draw" }).to_string(),
        json!({ "id": 7, "op": "plan" }).to_string(),
    ]);
    assert_eq!(replies.len(), 8);
    assert_eq!(replies[0]["result"]["protocol"], 1);
    assert_eq!(replies[0]["result"]["ops"][0], "analyse");
    assert_eq!(replies[1]["ok"], false);
    assert_eq!(replies[1]["error"]["code"], "invalid");
    assert_eq!(replies[2]["id"], 2);
    assert_eq!(replies[2]["result"]["schema"], "draftwright.plan/1");
    assert_eq!(replies[2]["result"]["views"].as_array().unwrap().len(), 4);
    let spec = replies[3]["result"].clone();
    assert_eq!(spec["views"]["iso"]["omit"], true);
    assert_eq!(replies[4]["error"]["choices"], json!(["set", "remove"]));
    assert!(
        replies[5]["error"]["choices"]
            .as_array()
            .unwrap()
            .contains(&json!("2:1"))
    );
    assert_eq!(replies[6]["error"]["code"], "invalid");
    assert!(
        replies[7]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("step")
    );

    // Render with the edited spec, then read the spec back from every format.
    let replies = serve(&[
        json!({ "id": 1, "op": "render", "step": step, "spec": spec,
            "output": out.display().to_string() })
        .to_string(),
        json!({ "id": 2, "op": "review", "step": step, "spec": spec }).to_string(),
        json!({ "id": 3, "op": "render", "step": step, "output": out.display().to_string(),
            "formats": ["png"] })
        .to_string(),
    ]);
    assert_eq!(replies[0]["ok"], true, "{}", replies[0]);
    let written = replies[0]["result"]["written"].as_array().unwrap();
    assert_eq!(written.len(), 3);
    assert!(
        replies[1]["result"]["svg"]
            .as_str()
            .unwrap()
            .starts_with("<?xml")
    );
    assert_eq!(replies[1]["result"]["problems"], json!([]));
    // No features were given, so lint has no requirements to find unmet.
    assert_eq!(replies[1]["result"]["lint"], json!([]));
    assert_eq!(replies[2]["error"]["choices"], json!(["svg", "dxf", "pdf"]));
    let reads: Vec<String> = written
        .iter()
        .enumerate()
        .map(|(i, f)| json!({ "id": i, "op": "spec", "drawing": f }).to_string())
        .collect();
    for reply in serve(&reads) {
        let back = &reply["result"];
        assert_eq!(back["sheet"]["title"]["material"], "Stahl ß ]]>", "{reply}");
        assert_eq!(back["views"]["iso"]["omit"], true);
        assert_eq!(back["binding"]["part"], "prismatic");
    }
}
