#[path = "common/support.rs"]
mod support;

#[test]
fn every_pmi_fixture_has_a_scheduled_case() {
    let mut actual: Vec<_> = std::fs::read_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pmi/step"
    ))
    .unwrap()
    .map(|e| e.unwrap().file_name().into_string().unwrap())
    .filter_map(|name| name.strip_suffix(".gz").map(str::to_owned))
    .collect();
    actual.sort();
    assert_eq!(actual, support::PMI_FILES);
}
