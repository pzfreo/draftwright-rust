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
fn engine_inputs_ignore_stray_files_but_include_source_and_assets() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("engine-input-filter");
    let _ = std::fs::remove_dir_all(&scratch);
    for directory in ["src/nested", "schemas/mcp", "fonts"] {
        std::fs::create_dir_all(scratch.join(directory)).unwrap();
    }
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "build_support.rs",
        "src/nested/a.rs",
        "schemas/mcp/a.json",
        "fonts/a.ttf",
        "fonts/b.otf",
    ] {
        std::fs::write(scratch.join(file), "input").unwrap();
    }
    let before = build_support::inputs(&scratch).unwrap();
    let hash = build_support::fingerprint(&scratch, &before).unwrap();
    for file in [
        "src/.DS_Store",
        "src/nested/.a.rs.swp",
        "src/nested/a.rs~",
        "schemas/mcp/.DS_Store",
        "schemas/mcp/a.json.bak",
        "fonts/.DS_Store",
    ] {
        std::fs::write(scratch.join(file), "stray").unwrap();
    }
    let after = build_support::inputs(&scratch).unwrap();
    assert_eq!(before, after);
    assert_eq!(hash, build_support::fingerprint(&scratch, &after).unwrap());
    std::fs::write(scratch.join("src/nested/b.rs"), "new source").unwrap();
    let added = build_support::inputs(&scratch).unwrap();
    assert_ne!(hash, build_support::fingerprint(&scratch, &added).unwrap());
    std::fs::remove_file(scratch.join("src/nested/b.rs")).unwrap();
    assert_eq!(before, build_support::inputs(&scratch).unwrap());
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
    let description = |name: &str| {
        tools.iter().find(|tool| tool["name"] == name).unwrap()["description"]
            .as_str()
            .unwrap()
    };
    for term in [
        "features/f:hole:1/fit",
        "±T",
        "views/iso/omit",
        "sheet/title/number",
        "Batch",
    ] {
        assert!(description("apply_ops").contains(term), "{term}");
    }
    assert!(description("inspect_part").contains("key_items"));
    assert!(description("inspect_part").contains("problem_filter"));
    assert!(description("list_problems").contains("filter.feature"));
    assert!(description("item_options").contains("diameter"));
    let schema_bytes: usize = tools
        .iter()
        .map(|tool| {
            serde_json::to_vec(&tool["inputSchema"]).unwrap().len()
                + serde_json::to_vec(&tool["outputSchema"]).unwrap().len()
        })
        .sum();
    // 38,500 after D1 trimmed them; sheet/fit_limits added about 500 bytes across the tools.
    // G3 added the `parts` tool (about 850 bytes) and a `part` argument on four tools
    // (about 150 bytes), bringing the total to 40,501. F4's sheet/projection (#13) added about
    // 540 bytes across the twelve tools that carry a spec: 41,041. K1's spec `faces` (datums
    // and controls on faces, #43) added about 400 bytes the same way: 41,444. K1b's
    // sheet/default_finish (#43) added about 530 bytes the same way: 41,972.
    // Authored gears add 32 bytes to each of twelve Spec-carrying schemas: 42,356.
    // Typed authored details (around string/list, from, exact scale) add 5,376 bytes
    // across twelve standalone Spec schemas: 47,732 total. Keep measured headroom small.
    assert!(schema_bytes < 48000, "schemas: {schema_bytes} bytes");
    eprintln!("schema bytes per session: {schema_bytes}");
}
