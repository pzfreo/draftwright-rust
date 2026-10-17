//! A client can combine the advertised envelope with the on-demand Spec shape.
use draftwright::{mcp, stateless::Cache};
use serde_json::{Value, json};

pub fn with_full_spec(tool: &str, output: bool) -> Value {
    let discovered = mcp::call(&Cache::new("unused"), "spec_schema", &json!({})).unwrap();
    let mut full = discovered["spec"].clone();
    let definitions = full.as_object_mut().unwrap().remove("$defs").unwrap();
    let mut schema = mcp::schema(tool, output).unwrap();
    let fragment = if output {
        discovered["spec_read"].as_str().unwrap()
    } else {
        "#/$defs/author_spec"
    };
    fn replace(value: &mut Value, fragment: &str) -> usize {
        let mut count = 0;
        if let Some(object) = value.as_object_mut() {
            if let Some(spec) = object.get_mut("properties").and_then(|p| p.get_mut("spec")) {
                *spec = json!({"$ref":fragment});
                count += 1;
            }
            for child in object.values_mut() {
                count += replace(child, fragment);
            }
        } else if let Some(array) = value.as_array_mut() {
            for child in array {
                count += replace(child, fragment);
            }
        }
        count
    }
    assert_eq!(replace(&mut schema, fragment), 1);
    schema["$defs"] = definitions;
    schema["$defs"]["author_spec"] = full;
    schema
}
