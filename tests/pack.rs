use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use draftwright::pack::{self, Pack, PackSpec, PartSpec, Supply};
use serde_json::json;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/assembly")
        .join(name)
}

fn small() -> &'static Pack {
    static PACK: OnceLock<Pack> = OnceLock::new();
    PACK.get_or_init(|| pack::build(&fixture("pack-small.step.gz"), &PackSpec::default()).unwrap())
}

#[test]
fn groups_quantities_items_csv_and_furniture() {
    let pack = small();
    let rows = &pack.report.rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().map(|r| r.item).collect::<Vec<_>>(), [1, 2, 3]);
    let pin = rows.iter().find(|r| r.name == "pin A").unwrap();
    assert_eq!(pin.qty, 2);
    assert_eq!(pin.products.len(), 2);
    assert_eq!(pin.supply, Supply::Make);
    assert_eq!(rows[2].supply, Supply::Buy);
    assert!(rows[2].buy_reasons[0].contains("screw"));
    assert!(rows[2].sheets.is_empty());
    assert!(
        rows.iter().all(|r| r.error.is_none()),
        "{}",
        serde_json::to_string(rows).unwrap()
    );
    assert_eq!(pack.report.pages, pack.drawings.len());
    assert!(
        pack.report
            .findings
            .iter()
            .any(|f| f.code == "pack_geometry_merge" && f.products.len() == 2)
    );
    assert_eq!(pack.report.csv().lines().count(), 4);
    assert!(pack.report.csv().contains("\"2\",\"make\""));
    for row in rows.iter().filter(|r| r.supply == Supply::Make) {
        for &page in &row.sheets {
            let svg = pack.drawings[page - 1].svg();
            assert!(svg.contains(&format!(">{}/{}<", page, pack.report.pages)));
            for value in [
                &row.name,
                "ITEM",
                "QTY",
                &row.item.to_string(),
                &row.qty.to_string(),
            ] {
                assert!(
                    svg.contains(&format!(">{value}<")),
                    "missing {value} on page {page}"
                );
            }
        }
    }
}

#[test]
fn overrides_reserve_items_keep_guesses_and_change_quantity_and_supply() {
    let key = small().report.rows[2].key.clone();
    let spec = PackSpec {
        parts: [(
            key,
            PartSpec {
                supply: Some(Supply::Make),
                item: Some(1),
                quantity: Some(8),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    let pack = pack::build(&fixture("pack-small.step.gz"), &spec).unwrap();
    assert_eq!(
        pack.report.rows.iter().map(|r| r.item).collect::<Vec<_>>(),
        [2, 3, 1]
    );
    let row = &pack.report.rows[2];
    assert_eq!((row.supply, row.qty), (Supply::Make, 8));
    assert!(!row.sheets.is_empty());
    assert!(
        pack.report
            .findings
            .iter()
            .any(|f| f.code == "pack_buy_guess" && f.products.contains(&row.key))
    );
    let again = pack::build(&fixture("pack-small.step.gz"), &spec).unwrap();
    assert_eq!(
        serde_json::to_value(&pack.report).unwrap(),
        serde_json::to_value(&again.report).unwrap()
    );
}

#[test]
fn strict_spec_duplicate_items_conflicting_group_intent_and_separation() {
    for value in [
        json!({"typo":1}),
        json!({"parts":{"p":{"suply":"buy"}}}),
        json!({"parts":{"p":{"drawing":{"typo":1}}}}),
    ] {
        assert!(serde_json::from_value::<PackSpec>(value).is_err());
    }
    let mut spec = PackSpec::default();
    spec.parts
        .insert("does-not-exist".into(), PartSpec::default());
    assert!(
        pack::build(&fixture("pack-small.step.gz"), &spec)
            .err()
            .unwrap()
            .message
            .contains("unknown pack part key")
    );
    spec.parts.clear();
    for row in &small().report.rows[..2] {
        spec.parts.insert(
            row.key.clone(),
            PartSpec {
                item: Some(7),
                ..Default::default()
            },
        );
    }
    assert!(
        pack::build(&fixture("pack-small.step.gz"), &spec)
            .err()
            .unwrap()
            .message
            .contains("duplicate")
    );
    let pin = &small().report.rows[1];
    spec.parts.clear();
    spec.parts.insert(
        pin.products[0].clone(),
        PartSpec {
            supply: Some(Supply::Make),
            ..Default::default()
        },
    );
    spec.parts.insert(
        pin.products[1].clone(),
        PartSpec {
            supply: Some(Supply::Buy),
            ..Default::default()
        },
    );
    assert!(
        pack::build(&fixture("pack-small.step.gz"), &spec)
            .err()
            .unwrap()
            .message
            .contains("conflicting supply")
    );
    spec.parts.get_mut(&pin.products[1]).unwrap().separate = true;
    let pack = pack::build(&fixture("pack-small.step.gz"), &spec).unwrap();
    assert_eq!(pack.report.rows.len(), 4);
    assert!(pack.report.rows.iter().all(|r| r.qty == 1));
}

#[test]
fn one_failed_drawing_keeps_its_row_and_other_drawings() {
    let row = &small().report.rows[0];
    let drawing = serde_json::from_value(json!({"sheet":{"size":"INVALID"}})).unwrap();
    let spec = PackSpec {
        parts: [(
            row.key.clone(),
            PartSpec {
                drawing: Some(drawing),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    let pack = pack::build(&fixture("pack-small.step.gz"), &spec).unwrap();
    assert_eq!(pack.report.rows.len(), 3);
    assert!(pack.report.rows[0].error.is_some());
    assert!(pack.report.rows[0].sheets.is_empty());
    assert!(pack.report.rows[1].error.is_none());
    assert!(!pack.report.rows[1].sheets.is_empty());
    assert_eq!(pack.report.pages, 1);
    assert!(pack.drawings[0].svg().contains(">1/1<"));
}

#[test]
fn handed_parts_do_not_merge() {
    let pack = pack::build(&fixture("pack-handed.step.gz"), &Default::default()).unwrap();
    assert_eq!(pack.report.rows.len(), 2);
    assert!(pack.report.rows.iter().all(|r| r.qty == 1));
}

#[test]
fn gramil_products_quantities_and_custom_screw_are_made() {
    let pack = pack::build(&fixture("gramil.step.gz"), &Default::default()).unwrap();
    assert_eq!(pack.report.rows.len(), 12);
    assert_eq!(
        pack.report
            .rows
            .iter()
            .filter(|r| r.supply == Supply::Make)
            .count(),
        8
    );
    assert_eq!(
        pack.report
            .rows
            .iter()
            .filter(|r| r.supply == Supply::Buy)
            .count(),
        4
    );
    for row in &pack.report.rows {
        assert_eq!(
            row.qty,
            if row.name.contains("GRM-06") {
                4
            } else if row.name.contains("GRM-07") {
                2
            } else {
                1
            }
        );
        assert_eq!(
            row.supply,
            if row.name.starts_with("GRM-") || row.name == "push rod" {
                Supply::Make
            } else {
                Supply::Buy
            },
            "{}",
            row.name
        );
        assert!(row.error.is_none(), "{}: {:?}", row.name, row.error);
    }
    assert_eq!(
        pack.report
            .findings
            .iter()
            .filter(|f| f.code == "pack_buy_guess")
            .count(),
        4
    );
}

#[test]
fn single_part_keeps_ordinary_views_measurements_and_all_sheets() {
    let step = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parts/plate.step");
    let pack = pack::build(&step, &Default::default()).unwrap();
    let row = &pack.report.rows[0];
    assert_eq!((row.qty, row.supply), (1, Supply::Make));
    let (part, analysis) = draftwright::model::analyse_native(&step).unwrap();
    let plan = draftwright::api::plan_analysed(&analysis, &Default::default()).unwrap();
    let font = draftwright::text::Font::new(draftwright::api::MONO.to_vec()).unwrap();
    let ordinary = draftwright::model::render_sheets(&part, &analysis, &plan, &font).unwrap();
    assert_eq!(ordinary.len(), pack.drawings.len());
    // Geometry layers are identical; pack-only title block furniture lives in dimensions.
    for (before, after) in ordinary.iter().zip(&pack.drawings) {
        assert_eq!(before.page, after.page);
        // A one-part pack keeps even the ordinary generic-name/file-stem title default.
        // Only the pack identifier runs are added to the searchable text layer.
        let after_runs: Vec<_> = after
            .text
            .iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        for run in &before.text {
            assert!(
                after_runs.contains(&serde_json::to_value(run).unwrap()),
                "lost or changed text run: {}",
                run.text
            );
        }

        for index in 0..2 {
            assert_eq!(before.layers[index].items, after.layers[index].items);
        }
        assert_eq!(before.annotations.len(), after.annotations.len());
        for (a, b) in before.annotations.iter().zip(&after.annotations) {
            assert_eq!((&a.name, &a.view, &a.claims), (&b.name, &b.view, &b.claims));
            let labels = |d: &draftwright::output::Drawing,
                          annotation: &draftwright::output::Annotation| {
                annotation
                    .runs
                    .iter()
                    .map(|&i| d.text[i].text.clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(labels(before, a), labels(after, b));
        }
    }
}

#[test]
fn cli_writes_requested_pack_files_and_refuses_unknown_format() {
    let dir = std::env::temp_dir().join(format!("draftwright-pack-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_draftwright"))
        .arg("pack")
        .arg(fixture("pack-small.step.gz"))
        .arg("-o")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("pack.json")).unwrap()).unwrap();
    assert_eq!(report["rows"].as_array().unwrap().len(), 3);
    assert!(
        std::fs::read(dir.join("pack.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("pack.csv"))
            .unwrap()
            .lines()
            .count(),
        4
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_draftwright"))
        .arg("pack")
        .arg(fixture("pack-small.step.gz"))
        .arg("-o")
        .arg(&dir)
        .args(["--formats", "svg"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn nested_drawing_spec_keeps_multiple_sheets_and_pack_fields_on_each_page() {
    let drawing = serde_json::from_value(json!({
        "sheet": {"size": "A4", "scale": "1:1", "title": {"company": "SHOP"}},
        "sheets": {"max": 2}, "views": {"iso": {"sheet": 2}},
        "tables": [{"id": "t:1", "sheet": 2, "rows": [["SECOND SHEET"]]}]
    }))
    .unwrap();
    let spec = PackSpec {
        parts: [(
            small().report.rows[0].key.clone(),
            PartSpec {
                drawing: Some(drawing),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    let pack = pack::build(&fixture("pack-small.step.gz"), &spec).unwrap();
    assert_eq!(pack.report.rows[0].sheets, [1, 2]);
    assert_eq!(pack.report.rows[1].sheets, [3]);
    for (i, drawing) in pack.drawings[..2].iter().enumerate() {
        let texts: Vec<_> = drawing.text.iter().map(|t| t.text.as_str()).collect();
        for value in ["plate", "SHOP", "ITEM", "QTY", "1", &format!("{}/3", i + 1)] {
            assert!(texts.contains(&value), "page {}: {}", i + 1, value);
        }
        assert_eq!(texts.contains(&"SECOND SHEET"), i == 1);
    }
    let (_, files) = pack::render(&fixture("pack-small.step.gz"), &spec, &["pdf"]).unwrap();
    assert!(String::from_utf8_lossy(&files[0].1).contains("/Count 3"));
}

#[test]
fn all_buy_pack_keeps_every_row_with_no_drawing_pages() {
    let spec = PackSpec {
        parts: small()
            .report
            .rows
            .iter()
            .map(|r| {
                (
                    r.key.clone(),
                    PartSpec {
                        supply: Some(Supply::Buy),
                        ..Default::default()
                    },
                )
            })
            .collect(),
        ..Default::default()
    };
    let (report, files) = pack::render(
        &fixture("pack-small.step.gz"),
        &spec,
        &["pdf", "csv", "json"],
    )
    .unwrap();
    assert_eq!(report.rows.len(), 3);
    assert_eq!(report.pages, 0);
    assert!(
        report
            .rows
            .iter()
            .all(|r| r.sheets.is_empty() && r.supply == Supply::Buy)
    );
    assert!(String::from_utf8_lossy(&files[0].1).contains("/Count 0"));
}

#[test]
fn keys_use_names_meaningful_ids_and_ordered_occurrences_without_entities() {
    let parts = [
        ("unique name", "SOLID"),
        ("ASSEMBLY", "catalogue-123"),
        ("ASSEMBLY", "COMPOUND"),
        ("ASSEMBLY", "ASSEMBLY"),
        ("", ""),
        ("", "SOLID"),
        ("ASSEMBLY#2", "SOLID"),
        ("repeat", "unique name"),
        ("repeat", "SOLID"),
    ]
    .into_iter()
    .enumerate()
    .map(|(part, (name, product_id))| draftwright::model::FilePart {
        part,
        name: name.into(),
        product_id: product_id.into(),
        faces: 0,
        instances: 1,
    })
    .collect::<Vec<_>>();
    assert_eq!(
        pack::part_keys(&parts),
        [
            "unique name",
            "catalogue-123",
            "ASSEMBLY#2#",
            "ASSEMBLY#3",
            "unnamed#1",
            "unnamed#2",
            "ASSEMBLY#2",
            "repeat#1",
            "repeat#2"
        ]
    );
}

fn edited_fixture(name: &str, source: &str, edit: impl FnOnce(String) -> String) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pack")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = draftwright::model::read_step_file(&fixture(source)).unwrap();
    let path = dir.join("assembly.step");
    std::fs::write(&path, edit(String::from_utf8(bytes).unwrap())).unwrap();
    path
}

#[test]
fn reexport_entity_numbers_do_not_change_keys_or_saved_group_intent() {
    let path = edited_fixture("numbered", "pack-small.step.gz", |s| {
        s.replace("pin A", "pin_1").replace("pin B", "pin_2")
    });
    let spec: PackSpec =
        serde_json::from_value(json!({"parts":{"pin":{"item":7,"quantity":9,"supply":"buy"}}}))
            .unwrap();
    let before = pack::build(&path, &spec).unwrap();
    let merged = before.report.rows.iter().find(|r| r.key == "pin").unwrap();
    assert_eq!(
        (&*merged.name, merged.qty, merged.item, merged.supply),
        ("pin", 9, 7, Supply::Buy)
    );
    assert_eq!(merged.products, ["pin_1", "pin_2"]);
    let path = edited_fixture("renumbered", "pack-small.step.gz", |s| {
        let s = s.replace("pin A", "pin_1").replace("pin B", "pin_2");
        let mut out = String::new();
        let mut input = s.chars().peekable();
        let mut quoted = false;
        while let Some(c) = input.next() {
            if c == '\'' {
                quoted = !quoted;
            }
            out.push(c);
            if c == '#' && !quoted {
                let mut n = String::new();
                while input.peek().is_some_and(|c| c.is_ascii_digit()) {
                    n.push(input.next().unwrap());
                }
                if !n.is_empty() {
                    out.push_str(&(n.parse::<usize>().unwrap() + 100_000).to_string());
                }
            }
        }
        out
    });
    let after = pack::build(&path, &spec).unwrap();
    let identity = |p: &Pack| {
        p.report
            .rows
            .iter()
            .map(|r| (&r.key, &r.name, &r.products, r.item, r.qty, r.supply))
            .map(|r| serde_json::to_value(r).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(identity(&before), identity(&after));
    let invalid: PackSpec =
        serde_json::from_value(json!({"parts":{"SOLID@definition:110":{}}})).unwrap();
    let error = pack::build(&path, &invalid).err().unwrap();
    for key in ["pin", "pin_1", "pin_2", "plate", "M2 screw"] {
        assert!(error.message.contains(key), "{error}");
    }
}

#[test]
fn merged_buy_reasons_are_distinct_in_csv_and_json() {
    let path = edited_fixture("bought-twins", "pack-small.step.gz", |s| {
        s.replace("pin A", "peg_screw_1")
            .replace("pin B", "peg_screw_2")
    });
    let pack = pack::build(&path, &Default::default()).unwrap();
    let row = pack
        .report
        .rows
        .iter()
        .find(|r| r.key == "peg_screw")
        .unwrap();
    assert_eq!(row.name, "peg_screw");
    assert_eq!(row.qty, 2);
    assert_eq!(row.buy_reasons, ["name: screw"]);
    let csv = pack.report.csv();
    let line = csv
        .lines()
        .find(|line| line.contains("peg_screw\""))
        .unwrap();
    assert_eq!(line.matches("name: screw").count(), 1);
    assert_eq!(
        serde_json::to_value(row).unwrap()["buy_reasons"],
        json!(["name: screw"])
    );
}

#[test]
fn unnamed_catalogue_shapes_use_recognised_features_and_custom_shapes_stay_made() {
    // No names or product IDs with supply hints survive: exercise only shape evidence.
    let path = edited_fixture("anonymous-shapes", "pack-shapes.step.gz", |mut s| {
        for i in 1..=9 {
            s = s.replace(&format!("body_{i}"), "ASSEMBLY");
        }
        s
    });
    let pack = pack::build(&path, &Default::default()).unwrap();
    assert_eq!(pack.report.rows.len(), 9);
    for (index, rule) in [
        (0, "thin-annulus washer"),
        (1, "two treated ends"),
        (2, "two treated ends"),
        (5, "externally threaded headed fastener"),
    ] {
        let row = pack
            .report
            .rows
            .iter()
            .find(|r| r.part_indices == [index])
            .unwrap();
        assert_eq!(row.supply, Supply::Buy, "{index}: {:?}", row.buy_reasons);
        assert!(
            row.buy_reasons
                .iter()
                .any(|r| r.starts_with("shape:") && r.contains(rule)),
            "{index}: {:?}",
            row.buy_reasons
        );
        assert!(
            pack.report
                .findings
                .iter()
                .any(|f| f.code == "pack_buy_guess"
                    && f.products == row.products
                    && f.message.contains(rule))
        );
    }
    for index in [3, 4, 6, 7, 8] {
        let row = pack
            .report
            .rows
            .iter()
            .find(|r| r.part_indices == [index])
            .unwrap();
        assert_eq!(row.supply, Supply::Make);
        assert!(row.buy_reasons.is_empty());
    }
    let washer = pack
        .report
        .rows
        .iter()
        .find(|r| r.part_indices == [0])
        .unwrap();
    let spec = PackSpec {
        parts: [(
            washer.key.clone(),
            PartSpec {
                supply: Some(Supply::Make),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    let overridden = pack::build(&path, &spec).unwrap();
    let washer = overridden
        .report
        .rows
        .iter()
        .find(|r| r.part_indices == [0])
        .unwrap();
    assert_eq!(washer.supply, Supply::Make);
    assert!(!washer.sheets.is_empty());
    assert!(!washer.buy_reasons.is_empty());
}

#[test]
fn renamed_gramil_custom_screw_and_bolt_still_do_not_match_catalogue_shapes() {
    let path = edited_fixture("anonymous-gramil", "gramil.step.gz", |s| {
        s.replace("GRM-03 thumbwheel drive screw", "neutral body a")
            .replace("GRM-05 depth-lock bolt", "neutral body b")
    });
    let pack = pack::build(&path, &Default::default()).unwrap();
    assert!(pack.report.rows.iter().any(|r| r.name == "neutral body a"));
    assert!(pack.report.rows.iter().any(|r| r.name == "neutral body b"));
    for index in [9, 10, 11] {
        let row = pack
            .report
            .rows
            .iter()
            .find(|r| r.part_indices == [index])
            .unwrap();
        assert_eq!(row.supply, Supply::Make, "{}", row.name);
        assert!(row.buy_reasons.is_empty());
        assert!(row.error.is_none());
    }
}
