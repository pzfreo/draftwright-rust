#[path = "common/corpus.rs"]
mod corpus;
use draftwright::{mcp, stateless::Cache};
use serde_json::{Value, json};
use std::path::Path;

fn open(name: &str, part: &str) -> (Cache, Value) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("e2-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let cache = Cache::new(root);
    let step = match part {
        "plate" => {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/mounting_plate.step")
        }
        "m10127" => corpus::dir().unwrap().join("mfcadpp/10127.step"),
        "grm03" => corpus::dir()
            .unwrap()
            .join("gramel/GRM-03_thumbwheel_drive_screw.step"),
        _ => corpus::dir().unwrap().join("nist/nist_ctc_01_asme1_rd.stp"),
    };
    let feature = match part {
        "plate" => "plate",
        "m10127" => "10127",
        "grm03" => "GRM-03_thumbwheel_drive_screw",
        _ => "nist_ctc_01_asme1_rd",
    };
    let summary = call(
        &cache,
        "inspect_part",
        json!({"step":step,"features":format!("tools/mcp-eval/features/{feature}.features.json")}),
    );
    (cache, summary)
}
fn call(cache: &Cache, tool: &str, args: Value) -> Value {
    let reply = mcp::reply(cache, &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":args}})).unwrap();
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let wire_bytes = serde_json::to_vec(&reply).unwrap().len();
    assert!(wire_bytes < 4096, "{tool}: {wire_bytes} bytes");
    eprintln!("E2_WIRE {tool} {wire_bytes}");
    let result = reply["result"]["structuredContent"].clone();
    mcp::validate(&mcp::schema(tool, true).unwrap(), &result).unwrap();
    result
}
fn query(summary: &Value, id: &str) -> Value {
    json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"id":id})
}

#[test]
fn summary_points_to_live_task_items_within_wire_budget() {
    for part in ["plate", "m10127", "grm03", "ctc01"] {
        let (cache, summary) = open(part, part);
        let keys = summary["key_items"]
            .as_array()
            .expect("summary includes key items");
        assert!(!keys.is_empty());
        for item in keys {
            call(
                &cache,
                "item_options",
                query(&summary, item["id"].as_str().unwrap()),
            );
            call(
                &cache,
                "get_item",
                query(&summary, item["id"].as_str().unwrap()),
            );
        }
        let repeated = call(
            &cache,
            "inspect_part",
            json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"]}),
        );
        assert_eq!(summary, repeated);
        if part != "grm03" {
            assert!(keys.iter().any(|i| i["id"] == "d:envelope/width.length"));
        }
        if part == "m10127" {
            assert!(keys.iter().any(|i| i["id"] == "d:f:hole:1/bore.diameter"));
        }
    }
}

#[test]
fn options_list_valid_hole_fits_and_every_dimension_field() {
    let (cache, summary) = open("fits", "m10127");
    let result = call(
        &cache,
        "item_options",
        query(&summary, "d:f:hole:1/bore.diameter"),
    );
    let fields = &result["settable"];
    let fits = fields["features/f:hole:1/fit"]["choices"]
        .as_array()
        .expect("hole fit choices");
    assert_eq!(fits.len(), 21);
    assert!(fits.contains(&json!("H7")));
    assert!(!fits.contains(&json!("g6")));
    for fit in fits {
        call(
            &cache,
            "preview_ops",
            json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":[{"op":"set","path":"features/f:hole:1/fit","value":fit}]}),
        );
    }
    for field in [
        "omit",
        "tolerance",
        "view",
        "place/view",
        "place/side",
        "place/lane",
        "place/along",
        "place/pinned",
    ] {
        assert!(
            fields
                .get(format!("dimensions/d:f:hole:1~1bore.diameter/{field}"))
                .is_some(),
            "{field}"
        );
    }
    assert_eq!(
        fields["dimensions/d:f:hole:1~1bore.diameter/tolerance"]["forms"],
        json!(["±T", "+U -L"])
    );
}

#[test]
fn options_explain_sheet_and_bare_view_keys() {
    let (cache, summary) = open("sheet-view", "plate");
    let view = call(&cache, "item_options", query(&summary, "v:iso"));
    assert!(view["settable"].get("views/iso/omit").is_some());
    for field in ["relative_to", "side", "gap", "align", "pinned"] {
        assert!(
            view["settable"]
                .get(format!("views/iso/place/{field}"))
                .is_some()
        );
    }
    let sheet = call(&cache, "item_options", query(&summary, "sheet"));
    for field in ["size", "scale", "title", "fit_limits", "fit_deviations"] {
        assert!(sheet["settable"].get(format!("sheet/{field}")).is_some());
    }
    assert!(
        sheet["settable"]["sheet/title"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("number"))
    );
}

#[test]
fn refusals_give_valid_choices_and_an_executable_corrected_op() {
    let (cache, summary) = open("refusals", "m10127");
    for bad in [
        json!({"op":"set","path":"dimensions/d:f:hole:1~1bore.diameter/tolerance","value":"H7"}),
        json!({"op":"set","path":"dimensions/d:f:hole:1~1bore.diameter/fit","value":"H7"}),
        json!({"op":"set","path":"dimensions/d:envelope~1width.length/tolerance","value":{"symmetric":0.02}}),
        json!({"op":"set","path":"views/v:iso/omit","value":true}),
        json!({"op":"set","path":"sheet/title/drawing_number","value":"DW-0001"}),
    ] {
        let args = json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":[bad]});
        let error = mcp::call(&cache, "preview_ops", &args).unwrap_err();
        assert!(!error.choices.is_empty(), "{error}");
        let (_, correction) = error
            .message
            .split_once("Corrected op: ")
            .expect("refusal includes correction");
        let corrected: Value = serde_json::from_str(correction).unwrap();
        call(
            &cache,
            "preview_ops",
            json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":[corrected]}),
        );
    }
    assert_eq!(
        cache
            .head(summary["part_ref"].as_str().unwrap(), "default")
            .unwrap(),
        summary["spec_version"]
    );
}

#[test]
fn one_feature_problem_query_returns_all_member_fixes() {
    let (cache, summary) = open("feature-problems", "ctc01");
    // Use authored bad pins to exercise member fixes. The two historical
    // corridor overruns are correctly eliminated by ISSUE-563-b.
    let edited = call(
        &cache,
        "apply_ops",
        json!({"part_ref":summary["part_ref"], "parent":summary["spec_version"], "ops":[
            {"op":"set", "path":"dimensions/d:f:slot:1~1slot_length.length/place", "value":{"side":"below", "lane":99, "pinned":true}},
            {"op":"set", "path":"dimensions/d:f:slot:1~1slot_width.length/place", "value":{"side":"left", "lane":99, "pinned":true}}
        ]}),
    );
    let version = &edited["spec_version"];
    let result = call(
        &cache,
        "list_problems",
        json!({"part_ref":summary["part_ref"],"spec_version":version,"filter":{"feature":"f:slot:1"}}),
    );
    let problems = result["items"].as_array().unwrap();
    assert_eq!(
        problems
            .iter()
            .filter(|p| p["code"] == "annotation_off_sheet")
            .count(),
        2
    );
    assert!(
        problems
            .iter()
            .all(|p| p["code"] != "annotation_outside_area")
    );
    assert_eq!(result["next_cursor"], Value::Null);
    let mut ops = Vec::new();
    for p in problems {
        ops.extend(
            p["fix"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .cloned(),
        );
    }
    assert!(!ops.is_empty());
    let preview = call(
        &cache,
        "preview_ops",
        json!({"part_ref":summary["part_ref"],"spec_version":version,"ops":ops}),
    );
    let delta: Value = serde_json::from_slice(
        &cache
            .artifact_bytes(preview["delta_ref"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(delta["problems_resolved"].as_array().unwrap().len(), 4);
    for p in problems
        .iter()
        .filter(|p| p["code"] != "extension_through_feature")
    {
        assert!(
            delta["problems_resolved"]
                .as_array()
                .unwrap()
                .iter()
                .any(|resolved| resolved["id"] == p["id"])
        );
    }
    assert!(delta["problems_added"].as_array().unwrap().is_empty());
}

#[test]
fn inspection_batches_a_filtered_problem_page_with_registration() {
    let (cache, summary) = open("inspect-problems", "ctc01");
    let result = call(
        &cache,
        "inspect_part",
        json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"problem_filter":{"feature":"f:slot:1"}}),
    );
    let problems = call(
        &cache,
        "list_problems",
        json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"filter":{"feature":"f:slot:1"}}),
    );
    assert_eq!(result["problem_page"], problems);
    let actual = result["problem_page"]["items"].as_array().unwrap();
    assert_eq!(actual.len(), 1);
    assert!(
        actual
            .iter()
            .all(|p| p["code"] == "extension_through_feature")
    );
    let all = call(
        &cache,
        "inspect_part",
        json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"problem_filter":{}}),
    );
    let mut found = all["problem_page"]["items"].as_array().unwrap().clone();
    let mut cursor = all["problem_page"]["next_cursor"].clone();
    while !cursor.is_null() {
        let page = call(
            &cache,
            "list_problems",
            json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"cursor":cursor}),
        );
        assert!(!page["items"].as_array().unwrap().is_empty());
        found.extend(page["items"].as_array().unwrap().iter().cloned());
        cursor = page["next_cursor"].clone();
    }
    assert_eq!(
        found.len(),
        all["problem_page"]["total"].as_u64().unwrap() as usize
    );
    let session = cache
        .session(
            summary["part_ref"].as_str().unwrap(),
            &cache
                .get_version(summary["spec_version"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        found
            .iter()
            .map(|p| p["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        session
            .state()
            .problems
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn fit_options_respect_nominal_table_boundaries() {
    for nominal in [0.0, 0.001, 250.0, 250.001] {
        let step = corpus::dir().unwrap().join("mfcadpp/10127.step");
        let (part, mut analysis) = draftwright::model::analyse_native(&step).unwrap();
        let hole = analysis
            .features
            .iter_mut()
            .find(|f| f.kind == "hole")
            .unwrap();
        let id = hole.id.clone();
        hole.parameters.insert("diameter".into(), json!(nominal));
        let session =
            draftwright::Session::new(part, analysis, draftwright::model::Spec::default()).unwrap();
        let options =
            serde_json::to_value(session.options(&format!("d:{id}/bore.diameter")).unwrap())
                .unwrap();
        let fields = options["settable"].as_object().expect("settable paths");
        let fits = fields.get(&format!("features/{id}/fit"));
        assert_eq!(
            fits.is_some(),
            nominal > 0.0 && nominal <= 250.0,
            "{nominal}"
        );
    }
}

#[test]
fn batch_refusal_corrects_the_named_failure_and_reports_its_index() {
    let (cache, summary) = open("batch-refusal", "plate");
    let title = json!({"op":"set","path":"sheet/title/drawing_number","value":"DW-0001"});
    for ops in [
        json!([{"op":"set","path":"sheet/scale","value":"1:1"}, title]),
        json!([{"op":"set","path":"sheet/title/title","value":"PLATE"}, title]),
    ] {
        let error = mcp::call(
            &cache,
            "preview_ops",
            &json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":ops}),
        ).unwrap_err();
        let (_, correction) = error.message.split_once("Corrected op: ").unwrap();
        let corrected: Value = serde_json::from_str(correction).unwrap();
        assert!(error.message.contains("Replace ops[1]."), "{error}");
        assert_eq!(corrected["path"], "sheet/title/number");
    }
    let error = mcp::call(
        &cache, "preview_ops",
        &json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":[title,
            {"op":"set","path":"dimensions/d:envelope~1width.length/tolerance","value":{"symmetric":0.02}}
        ]}),
    ).unwrap_err();
    assert!(error.message.contains("Replace ops[1]."), "{error}");
    let (_, correction) = error.message.split_once("Corrected op: ").unwrap();
    let corrected: Value = serde_json::from_str(correction).unwrap();
    assert_eq!(corrected["value"], "±0.02");
    let error = mcp::call(
        &cache,
        "preview_ops",
        &json!({"part_ref":summary["part_ref"],"spec_version":summary["spec_version"],"ops":[
            {"op":"set","path":"dimensions/d:envelope~1width.length/tolerance","value":"nonsense"}, title
        ]}),
    ).unwrap_err();
    assert!(!error.message.contains("Corrected op:"), "{error}");
}

#[test]
fn compact_nullable_schema_types_keep_validation_strict() {
    // Published inputs are shallow; server preflight retains the original strict shape.
    let preflight: Value =
        serde_json::from_str(include_str!("../schemas/mcp-validation.json")).unwrap();
    let schema = &preflight["inspect_part"];
    for spec in [
        json!({"sheet":{"size":null,"fit_limits":null}}),
        json!({"sheet":{"size":"A4","fit_limits":true}}),
    ] {
        mcp::validate(schema, &json!({"part_ref":"part","spec":spec})).unwrap();
    }
    for spec in [
        json!({"sheet":{"size":42}}),
        json!({"sheet":{"fit_limits":"yes"}}),
    ] {
        assert!(mcp::validate(schema, &json!({"part_ref":"part","spec":spec})).is_err());
    }
    for (types, accepted, refused) in [
        (json!(["string", "null"]), json!("A4"), json!(42)),
        (json!(["number", "null"]), json!(0.02), json!("0.02")),
        (json!(["boolean", "null"]), json!(true), json!([])),
        (json!(["object", "null"]), json!({}), json!([])),
    ] {
        let schema = json!({"type":types});
        mcp::validate(&schema, &accepted).unwrap();
        mcp::validate(&schema, &Value::Null).unwrap();
        assert!(mcp::validate(&schema, &refused).is_err());
    }
}

#[test]
fn compact_prefix_suffix_choices_and_authored_dimension_outputs_stay_strict() {
    let preflight: Value =
        serde_json::from_str(include_str!("../schemas/mcp-validation.json")).unwrap();
    let input = &preflight["inspect_part"];
    for name in [
        "fit_limits",
        "fit_deviations",
        "centre_marks",
        "centre_lines",
        "thread_depiction",
        "knurl_depiction",
    ] {
        for value in [json!(true), json!(false), Value::Null] {
            mcp::validate(
                input,
                &json!({"part_ref":"part","spec":{"sheet":{name:value}}}),
            )
            .unwrap();
        }
        for bad_name in [format!("{name}\n"), format!("{name}_extra")] {
            assert!(
                mcp::validate(
                    input,
                    &json!({"part_ref":"part","spec":{"sheet":{bad_name:true}}})
                )
                .is_err()
            );
        }
        assert!(
            mcp::validate(
                input,
                &json!({"part_ref":"part","spec":{"sheet":{name:"yes"}}})
            )
            .is_err()
        );
    }
    for name in [
        "thread_lines",
        "knurl_marks",
        "centre_depiction",
        "_depiction",
    ] {
        assert!(
            mcp::validate(
                input,
                &json!({"part_ref":"part","spec":{"sheet":{name:true}}})
            )
            .is_err()
        );
    }
    let discovered = mcp::call(&Cache::new("unused"), "spec_schema", &json!({})).unwrap();
    let output = json!({"type":"object", "properties":{
        "spec_version":{"type":"string"}, "spec":{"$ref":discovered["spec_read"]}},
        "required":["spec_version","spec"], "additionalProperties":false,
        "$defs":discovered["spec"]["$defs"]});
    let result = |dimension| {
        json!({"spec_version":"v","spec":{
            "dimensions":{"d:authored:test":dimension},
            "feature_declarations":[{"id":"f:declared:test","kind":"hole","faces":[]}]
        }})
    };
    mcp::validate(
        &output,
        &result(json!({"kind":"linear","between":["a","b"],"along":"true"})),
    )
    .unwrap();
    for dimension in [
        json!([]),
        Value::Null,
        json!({"kind":"bad"}),
        json!({"between":[]}),
        json!({"between":["a","b","c"]}),
        json!({"between":[1]}),
        json!({"along":"bad"}),
    ] {
        assert!(mcp::validate(&output, &result(dimension)).is_err());
    }
}
