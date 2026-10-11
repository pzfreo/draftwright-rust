#[path = "../build_support.rs"]
mod build_support;

use std::path::Path;

#[test]
fn engine_version_matches_source_and_invalidates_on_content_changes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = build_support::inputs(root).unwrap();
    assert!(files.iter().all(|p| !p.is_absolute()));
    let computed = build_support::fingerprint(root, &files).unwrap();
    assert_eq!(draftwright::stateless::ENGINE_VERSION, computed);
    let cache_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("engine-version-cache");
    let _ = std::fs::remove_dir_all(&cache_root);
    let cache = draftwright::stateless::Cache::new(&cache_root);
    let key = cache
        .register(&root.join("tests/fixtures/parts/mounting_plate.step"), None)
        .unwrap();
    assert_eq!(key.split_once('-').unwrap().1, computed);
    let obsolete = format!("{}-{}", key.split_once('-').unwrap().0, "0".repeat(64));
    assert!(
        cache
            .summary(&obsolete, &draftwright::model::Spec::default())
            .is_err()
    );
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("engine-fingerprint");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("src")).unwrap();
    std::fs::write(scratch.join("src/a.rs"), "one").unwrap();
    std::fs::write(scratch.join("Cargo.lock"), "lock").unwrap();
    let files = vec!["Cargo.lock".into(), "src/a.rs".into()];
    let before = build_support::fingerprint(&scratch, &files).unwrap();
    let reversed = files.iter().rev().cloned().collect::<Vec<_>>();
    assert_eq!(
        before,
        build_support::fingerprint(&scratch, &reversed).unwrap()
    );
    std::fs::write(scratch.join("src/a.rs"), "two").unwrap();
    let source_change = build_support::fingerprint(&scratch, &files).unwrap();
    assert_ne!(before, source_change);
    std::fs::write(scratch.join("Cargo.lock"), "new lock").unwrap();
    let lock_change = build_support::fingerprint(&scratch, &files).unwrap();
    assert_ne!(source_change, lock_change);
    std::fs::rename(scratch.join("src/a.rs"), scratch.join("src/b.rs")).unwrap();
    let renamed = vec!["Cargo.lock".into(), "src/b.rs".into()];
    assert_ne!(
        lock_change,
        build_support::fingerprint(&scratch, &renamed).unwrap()
    );
    assert!(
        build_support::inputs(root)
            .unwrap()
            .iter()
            .any(|p| p == Path::new("src/stateless.rs"))
    );
}

#[test]
fn tools_explain_their_operations_without_source_and_fit_schema_budget() {
    let cache = draftwright::stateless::Cache::new("unused");
    let reply = draftwright::mcp::reply(
        &cache,
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
        }),
    )
    .unwrap();
    let tools = reply["result"]["tools"].as_array().unwrap();
    for tool in tools {
        let description = tool["description"].as_str().unwrap();
        assert!(!description.contains("see docs/"), "{description}");
        assert!(description.len() >= 80, "{description}");
    }
    let schema_bytes: usize = tools
        .iter()
        .map(|tool| {
            serde_json::to_vec(&tool["inputSchema"]).unwrap().len()
                + serde_json::to_vec(&tool["outputSchema"]).unwrap().len()
        })
        .sum();
    assert!(schema_bytes < 38500, "schemas: {schema_bytes} bytes");
    eprintln!("schema bytes per session: {schema_bytes}");
}
