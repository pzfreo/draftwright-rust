//! R5's overflow case measures complete session edits without increasing the planner budget.
#[path = "common/corpus.rs"]
mod corpus;
#[path = "common/overflow.rs"]
mod overflow;
use draftwright::Session;
use draftwright::api::Features;
use draftwright::items::Op;
use draftwright::model::Spec;
use serde_json::json;
use std::time::Instant;

#[cfg(target_os = "macos")]
fn instructions() -> Option<u64> {
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
    }
    // Darwin rusage_info_v4: 16-byte UUID then u64 fields; ri_instructions is word 31.
    // Read only this process; no sysctl, child-process access or wall-time inference.
    let mut info = [0_u64; 37];
    let result = unsafe { proc_pid_rusage(std::process::id() as i32, 4, info.as_mut_ptr()) };
    (result == 0 && info[31] != 0).then_some(info[31])
}
#[cfg(not(target_os = "macos"))]
fn instructions() -> Option<u64> {
    None
}

#[test]
fn r5_multi_sheet_latency() {
    let Some(corpus) = corpus::dir() else { return };
    let step = corpus.join("gramel/string_post.step");
    let spec: Spec = serde_json::from_value(json!({
        "sheet":{"size":"A4"}, "sheets":{"max":2,"smallest_scale":"5:1"},
        "tables": overflow::tables()
    }))
    .unwrap();
    let mut s = Session::open(&step, Features::Native, spec).unwrap();
    assert_eq!(s.plan().sheets.len(), 1);
    assert_eq!(s.plan().sheet.scale, "5:1");
    assert!(
        s.plan()
            .tables
            .iter()
            .any(|t| t.id == "t:1" && t.placed.is_some())
    );
    assert!(!s.plan().problems.iter().any(|p| matches!(
        p.code.as_str(),
        "table_not_placed" | "sheet_below_smallest_scale"
    )));
    let mut samples = Vec::new();
    let mut steps = Vec::new();
    let mut retired = Vec::new();
    let mut max_bytes = 0;
    for n in 0..10 {
        let before = s.stats();
        let first_instruction = instructions();
        let start = Instant::now();
        let delta = s
            .apply(
                &[Op::set("sheet/title/material", json!(format!("R5 {n}")))],
                true,
            )
            .unwrap();
        max_bytes = max_bytes.max(serde_json::to_vec(&delta).unwrap().len());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        if let (Some(first), Some(last)) = (first_instruction, instructions()) {
            retired.push(last - first);
        }
        assert_eq!(delta.plans, 1);
        assert_eq!(s.stats().plans, before.plans + 1);
        let previous = serde_json::to_value(before).unwrap()["sheet_steps"]
            .as_u64()
            .unwrap_or(0);
        let current = serde_json::to_value(s.stats()).unwrap()["sheet_steps"]
            .as_u64()
            .unwrap_or(0);
        steps.push(current - previous);
    }
    samples.sort_by(f64::total_cmp);
    let p90 = samples[8];
    eprintln!(
        "R5_LATENCY samples=10 p50_ms={:.3} p90_ms={p90:.3} max_ms={:.3} max_delta_bytes={max_bytes} sheet_steps={steps:?} plans_per_edit=1",
        samples[5], samples[9]
    );
    retired.sort_unstable();
    eprintln!(
        "R5_INSTRUCTIONS samples={} median={:?} per_edit={retired:?}",
        retired.len(),
        retired.get(retired.len() / 2)
    );

    // Measure the stateless MCP path too: reconstruction, immutable apply, paging and wire JSON.
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("r5-latency-mcp");
    let _ = std::fs::remove_dir_all(&root);
    let cache = draftwright::stateless::Cache::new(root);
    let key = cache.register(&step, None).unwrap();
    let parent = cache.head(&key, "default").unwrap();
    let initial = draftwright::mcp::call(
        &cache,
        "apply_ops",
        &json!({"part_ref":key,"parent":parent,"ops":[
            {"op":"set","path":"sheet/size","value":"A4"},
            {"op":"set","path":"sheets/max","value":2},
            {"op":"set","path":"sheets/smallest_scale","value":"5:1"},
            {"op":"set","path":"tables","value":overflow::tables()}
        ]}),
    )
    .unwrap();
    let mut parent = initial["spec_version"].as_str().unwrap().to_owned();
    let mut mcp_samples = Vec::new();
    let mut mcp_retired = Vec::new();
    let mut max_wire = 0;
    for n in 0..10 {
        let request = json!({"jsonrpc":"2.0","id":n,"method":"tools/call","params":{"name":"apply_ops","arguments":{"part_ref":key,"parent":parent,"ops":[{"op":"set","path":"sheet/title/material","value":format!("R5 {n}")}]}}});
        let first = instructions();
        let started = Instant::now();
        let reply = draftwright::mcp::reply(&cache, &request).unwrap();
        let wire = serde_json::to_vec(&reply).unwrap();
        mcp_samples.push(started.elapsed().as_secs_f64() * 1000.0);
        if let (Some(first), Some(last)) = (first, instructions()) {
            mcp_retired.push(last - first);
        }
        max_wire = max_wire.max(wire.len());
        assert!(wire.len() < 4096);
        let result = &reply["result"]["structuredContent"];
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        assert_eq!(result["delta"]["plans"], 1);
        parent = result["spec_version"].as_str().unwrap().to_owned();
    }
    mcp_samples.sort_by(f64::total_cmp);
    mcp_retired.sort_unstable();
    eprintln!(
        "R5_MCP_LATENCY samples=10 p50_ms={:.3} p90_ms={:.3} max_ms={:.3} max_wire_bytes={max_wire} plans_per_edit=1",
        mcp_samples[5], mcp_samples[8], mcp_samples[9]
    );
    eprintln!(
        "R5_MCP_INSTRUCTIONS samples={} median={:?} per_edit={mcp_retired:?}",
        mcp_retired.len(),
        mcp_retired.get(mcp_retired.len() / 2)
    );
    assert!(steps.iter().all(|n| (2..=4).contains(n)));
    if std::env::var("DRAFTWRIGHT_LATENCY_GATE").as_deref() != Ok("0") {
        assert!(
            p90 <= 150.0,
            "multi-sheet edit p90 {p90:.3} ms exceeds 150 ms"
        );
        assert!(
            mcp_samples[8] <= 150.0,
            "multi-sheet MCP p90 {:.3} ms exceeds 150 ms",
            mcp_samples[8]
        );
    }
}
