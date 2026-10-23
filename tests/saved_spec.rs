//! A saved Spec opened across kernel upgrades (#191): refusals name the entry they refuse,
//! `Session::open_spec` sets aside entries the part no longer has, and `api::spec_ops` turns one
//! Spec into another.
use draftwright::{
    api,
    items::Op,
    model,
    session::{Session, SetAside},
};
use serde_json::{Value, json};

fn plate() -> Session {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts/mounting_plate.step");
    let (part, analysis) = model::analyse_native(&path).unwrap();
    Session::new(part, analysis, model::Spec::default()).unwrap()
}

fn spec(value: Value) -> model::Spec {
    serde_json::from_value(value).unwrap()
}

fn set_aside(path: &str, entry: Value, reason_code: &str) -> SetAside {
    SetAside {
        path: path.into(),
        entry,
        reason_code: reason_code.into(),
    }
}

/// Each "there is no …" refusal names the escaped Spec entry as data; its message and choices are
/// unchanged, and other refusals carry no path.
#[test]
fn refusals_name_the_entry_the_part_does_not_have() {
    let mut s = plate();
    // f:hole:1 is on the plate (in f:hole_pattern:1), but its bore diameter is not drawn.
    assert!(s.analysis().features.iter().any(|f| f.id == "f:hole:1"));
    let cases = [
        (
            json!({"dimensions":{"d:f:hole:1/bore.diameter":{"omit":true}}}),
            "dimensions/d:f:hole:1~1bore.diameter",
        ),
        (
            json!({"dimensions":{"d:f:hole:99/bore.diameter":{"omit":true}}}),
            "dimensions/d:f:hole:99~1bore.diameter",
        ),
        (
            json!({"features":{"f:hole:99":{"suppress":true}}}),
            "features/f:hole:99",
        ),
        (
            json!({"features":{"f:hole:99":{"fit":"H7"}}}),
            "features/f:hole:99",
        ),
        (
            json!({"annotations":{"pmi:gone":{"omit":true}}}),
            "annotations/pmi:gone",
        ),
        (json!({"views":{"gone":{"omit":true}}}), "views/gone"),
        (
            json!({"precision":{"d:f:hole:99/bore.diameter":2}}),
            "precision/d:f:hole:99~1bore.diameter",
        ),
    ];
    for (value, path) in cases {
        let e = s.set_spec(&spec(value.clone())).unwrap_err();
        assert_eq!(e.code, "invalid", "{value}");
        assert_eq!(e.path.as_deref(), Some(path), "{value}");
        assert_eq!(serde_json::to_value(&e).unwrap()["path"], path);
    }
    let e = s
        .set_spec(&spec(
            json!({"dimensions":{"d:f:hole:1/bore.diameter":{"omit":true}}}),
        ))
        .unwrap_err();
    assert_eq!(
        e.message,
        "There is no dimension \"d:f:hole:1/bore.diameter\" on this part."
    );
    assert_eq!(e.choices.len(), 5);
    let e = s
        .set_spec(&spec(
            json!({"dimensions":{"d:f:hole:4/bore.diameter":{"place":{"view":"iso"}}}}),
        ))
        .unwrap_err();
    assert_eq!(e.path, None);
    assert!(serde_json::to_value(&e).unwrap().get("path").is_none());
}

/// What the part still has is applied; each entry it no longer has is set aside with the entry
/// and why; the older `preferred_printer` is read as `preferred_max_size`.
#[test]
fn open_spec_sets_aside_what_the_part_no_longer_has() {
    let mut s = plate();
    let saved = json!({
        "schema": "draftwright.spec/2",
        "sheet": {"preferred_printer": "A4"},
        "dimensions": {
            "d:f:hole:4/bore.diameter": {"omit": true},
            "d:f:hole:1/bore.diameter": {"omit": true},
            "d:f:hole:99/bore.diameter": {"place": {"side": "above"}},
        },
        "features": {"f:hole:99": {"suppress": true}},
        "annotations": {"pmi:gone": {"omit": true}},
        "views": {"gone": {"omit": true}, "iso": {"omit": true},
                  "detail_a": {"detail": {"around": "f:hole:99", "scale_factor": 4}}},
    });
    let opened = s.open_spec(&saved.to_string()).unwrap();
    let mut got = opened.set_aside.clone();
    got.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(
        got,
        [
            set_aside(
                "annotations/pmi:gone",
                json!({"omit": true}),
                "no_annotation"
            ),
            set_aside(
                "dimensions/d:f:hole:1~1bore.diameter",
                json!({"omit": true}),
                "no_dimension"
            ),
            set_aside(
                "dimensions/d:f:hole:99~1bore.diameter",
                json!({"place": {"side": "above"}}),
                "no_feature"
            ),
            set_aside(
                "features/f:hole:99",
                json!({"suppress": true}),
                "no_feature"
            ),
            set_aside(
                "views/detail_a",
                // Read in its current form: the scale factor is a number.
                json!({"detail": {"around": "f:hole:99", "scale_factor": 4.0}}),
                "no_feature"
            ),
            set_aside("views/gone", json!({"omit": true}), "no_view"),
        ]
    );
    assert_eq!(opened.spec, serde_json::to_value(s.spec()).unwrap());
    assert_eq!(opened.spec["sheet"]["preferred_max_size"], "A4");
    assert!(opened.spec["sheet"].get("preferred_printer").is_none());
    assert_eq!(
        opened.spec["dimensions"],
        json!({"d:f:hole:4/bore.diameter": {"omit": true}})
    );
    assert_eq!(opened.spec["views"], json!({"iso": {"omit": true}}));
    assert!(opened.spec.get("features").is_none_or(|f| f == &json!({})));
    // Applied: the bore is omitted and the iso view is not drawn.
    assert!(
        s.plan()
            .dimensions
            .iter()
            .all(|d| d.id != "d:f:hole:4/bore.diameter")
    );
    assert!(s.plan().views.iter().all(|v| v.id != "v:iso"));
    // A Spec the part has whole is opened as set_spec opens it, with nothing set aside.
    let mut t = plate();
    let whole = t.open_spec(&opened.spec.to_string()).unwrap();
    assert!(whole.set_aside.is_empty());
    assert_eq!(t.spec(), s.spec());
}

/// A refusal that is not about an entry the part lacks still refuses the whole Spec, and the
/// session is unchanged.
#[test]
fn open_spec_still_refuses_other_refusals() {
    let mut s = plate();
    let before = serde_json::to_value(s.spec()).unwrap();
    for saved in [
        json!({"features": {"f:hole:99": {"suppress": true}},
               "dimensions": {"d:f:hole:4/bore.diameter": {"place": {"view": "iso"}}}}),
        json!({"dimensions": {"d:f:hole:99/bore.diameter": {"omit": true}},
               "sheet": {"preferred_max_sheets": 0}}),
    ] {
        let e = s.open_spec(&saved.to_string()).unwrap_err();
        assert_eq!(e.code, "invalid", "{saved}");
        assert_eq!(e.path, None, "{saved}");
        assert_eq!(serde_json::to_value(s.spec()).unwrap(), before);
    }
    assert_eq!(s.open_spec("{").unwrap_err().code, "invalid");
}

/// The ops are the editor's `specOperations` and the MCP's `spec_diff`: sorted keys, `schema`
/// skipped at the root, `~` and `/` escaped, arrays and scalars replaced whole, a key missing or
/// null in *after* removed.
#[test]
fn spec_ops_turn_one_spec_into_another() {
    let before = json!({"schema": "a", "x": {"b": 1, "c": [1, 2], "k": {"schema": 1}}, "d": 1,
                        "n": null});
    let after = json!({"schema": "b", "x": {"c": [1], "e/f~": {"g": 2}, "k": {"schema": 2}},
                       "h": null, "z": {}, "n": {"m": true}});
    assert_eq!(
        api::spec_ops(&before, &after),
        [
            Op::remove("d"),
            Op::set("n/m", json!(true)),
            Op::remove("x/b"),
            Op::set("x/c", json!([1])),
            Op::set("x/e~1f~0/g", json!(2)),
            Op::set("x/k/schema", json!(2)),
        ]
    );
    assert!(api::spec_ops(&after, &after).is_empty());
    assert_eq!(api::spec_ops(&json!(1), &json!(2)), [Op::set("", json!(2))]);

    // Round trip through the Spec editor: the ops turn before into after, and back.
    let s = plate();
    let before = serde_json::to_value(s.spec()).unwrap();
    let mut after = before.clone();
    after["dimensions"] = json!({
        "d:f:hole:4/bore.diameter": {"omit": true},
        "d:envelope/width.length": {"place": {"view": "plan"}},
    });
    after["sheet"]["size"] = json!("A3");
    after["views"] = json!({"iso": {"omit": true}});
    let ops = api::spec_ops(&before, &after);
    assert!(
        ops.iter()
            .any(|op| op.path == "dimensions/d:f:hole:4~1bore.diameter/omit")
    );
    let edited = model::edit(s.spec(), &serde_json::to_value(&ops).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&edited).unwrap(), after);
    let back = api::spec_ops(&after, &before);
    let restored = model::edit(&edited, &serde_json::to_value(&back).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), before);
}
