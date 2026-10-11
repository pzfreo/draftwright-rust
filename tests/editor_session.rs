use draftwright::{
    api,
    items::Op,
    model,
    session::{ItemFilter, Session},
    text::Font,
};
use serde_json::json;
fn session() -> Session {
    let bytes = include_bytes!("fixtures/parts/editor_pmi.step");
    let (part, analysis) =
        model::analyse_native_step(std::path::Path::new("editor_pmi.step"), bytes).unwrap();
    Session::new(part, analysis, model::Spec::default()).unwrap()
}
#[test]
fn semantic_threads_use_native_lowering_and_keep_their_source() {
    let s = session();
    let item = s
        .plan()
        .dimensions
        .iter()
        .find(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
        .expect("semantic thread has a native owner");
    assert!(item.placed.is_some());
    assert!(
        item.label.contains("M5 x 0.8-6H RH") && item.label.contains("THRU"),
        "{}",
        item.label
    );
    let report = s.pmi_report();
    let source = report["extraction"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "internal_thread")
        .unwrap()["source_id"]
        .as_str()
        .unwrap();
    assert!(
        report["lowered"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["source_id"] == source)
    );
    assert!(
        !report["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["source_id"] == source)
    );
    assert!(
        s.plan()
            .problems
            .iter()
            .all(|p| !p.refs.contains(&source.to_string()))
    );
}
#[test]
fn geometry_choices_respect_visibility_and_stable_identity() {
    let s = session();
    let before = s.stats();
    let all = s.all_options();
    let width = s.options("d:envelope/width.length").unwrap();
    assert_eq!(
        serde_json::to_value(&all[&width.id]).unwrap(),
        serde_json::to_value(&width).unwrap()
    );
    assert_eq!(before, s.stats(), "queries do not plan");
    assert!(
        width
            .placements
            .iter()
            .any(|p| p.view.as_deref() == Some("plan"))
    );
    assert!(
        !width
            .placements
            .iter()
            .any(|p| p.view.as_deref() == Some("side"))
    );
    let datum = s
        .items(&ItemFilter::default())
        .into_iter()
        .find(|a| a.item.kind == "datum_feature" && a.item.label.as_deref() == Some("A"))
        .unwrap();
    let options = s.options(&datum.item.id).unwrap();
    assert!(
        options
            .placements
            .iter()
            .any(|p| p.view.as_deref() == Some("side"))
    );
    assert!(
        !options
            .placements
            .iter()
            .any(|p| p.view.as_deref() == Some("plan"))
    );
}
#[test]
fn fragments_partition_full_ink_and_annotation_edits_keep_view_svg() {
    let mut s = session();
    let full = s.drawing().svg();
    let fragments = s.svg_fragments(None);
    for element in ["<line ", "<path ", "<circle ", "<text "] {
        assert_eq!(
            full.matches(element).count(),
            fragments
                .values()
                .map(|s| s.matches(element).count())
                .sum::<usize>(),
            "{element}"
        );
    }
    let delta = s
        .apply(
            &[Op::set(
                "dimensions/d:envelope~1width.length/tolerance",
                json!("±0.05"),
            )],
            true,
        )
        .unwrap();
    let changed = s.svg_fragments(Some(&delta));
    assert!(!changed.keys().any(|id| id.starts_with("v:")));
    assert_eq!(fragments["v:front"], s.svg_fragments(None)["v:front"]);
}
#[test]
fn pmi_replacement_is_transactional_and_keeps_the_plan_render_oracle() {
    let mut s = session();
    let before = serde_json::to_vec(s.state()).unwrap();
    assert!(s.set_pmi(b"invalid").is_err());
    assert_eq!(before, serde_json::to_vec(s.state()).unwrap());
    assert!(
        s.set_pmi_ops(
            include_bytes!("fixtures/parts/editor_pmi.step"),
            &[Op::set("unknown", json!(true))]
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_vec(s.state()).unwrap());
    let delta = s
        .set_pmi_ops(
            include_bytes!("fixtures/parts/editor_pmi.step"),
            &[Op::set(
                "dimensions/d:envelope~1width.length/tolerance",
                json!("±0.02"),
            )],
        )
        .unwrap();
    assert_eq!(delta.plans, 1);
    s.set_pmi(include_bytes!("fixtures/parts/editor_pmi.step"))
        .unwrap();
    let plan = api::plan_analysed(s.analysis(), s.spec()).unwrap();
    assert_eq!(
        serde_json::to_vec(&plan).unwrap(),
        serde_json::to_vec(s.plan()).unwrap()
    );
    assert_eq!(
        s.drawing().svg(),
        model::render(
            s.part(),
            s.analysis(),
            &plan,
            &Font::new(api::MONO.to_vec()).unwrap()
        )
        .unwrap()
        .svg()
    );
}
#[test]
fn curved_edges_are_unfilled_and_faces_keep_fill() {
    let s = session();
    let svg = s.drawing().svg();
    assert!(svg.contains("<circle "));
    for tag in svg.split('<').filter(|tag| tag.starts_with("circle ")) {
        assert!(tag.split('>').next().unwrap().contains("fill=\"none\""));
    }
    assert!(
        svg.split('<').filter(|t| t.starts_with("path ")).any(|t| !t
            .split('>')
            .next()
            .unwrap()
            .contains("fill=\"none\"")),
        "glyph faces keep inherited fill"
    );
}

#[test]
fn symbolic_bore_fits_are_printed_with_their_iso_286_limits() {
    let s = session();
    for fit in ["H7", "H11"] {
        let report = s.pmi_report();
        let reading = &report["extraction"]["records"];
        let record = reading
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["fit"] == fit)
            .unwrap();
        assert_eq!(record["blockers"], json!([]));
        assert!(record["lower_bound"].is_null() && record["upper_bound"].is_null());
        assert!(
            report["lowered"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["fit"] == fit)
        );
        assert!(
            s.plan()
                .dimensions
                .iter()
                .any(|d| d.placed.is_some() && d.label.contains(fit))
        );
    }
    // The class and its limits from ISO 286 (draftwright's `fits` table), never the class alone.
    let labels: Vec<String> = s
        .plan()
        .dimensions
        .iter()
        .map(|d| d.label.clone())
        .collect();
    for fit in ["H7", "H11"] {
        let label = labels.iter().find(|l| l.contains(fit)).unwrap();
        assert!(
            label.contains(&format!("{fit} (+0.")) && label.contains("/0)"),
            "{label:?} states {fit} with its limits"
        );
    }
    let mut hidden = model::Spec::default();
    hidden.sheet.fit_limits = Some(false);
    let (part, analysis) = model::analyse_native_step(
        std::path::Path::new("editor_pmi.step"),
        include_bytes!("fixtures/parts/editor_pmi.step"),
    )
    .unwrap();
    let quiet = Session::new(part, analysis, hidden).unwrap();
    for fit in ["H7", "H11"] {
        let label = quiet
            .plan()
            .dimensions
            .iter()
            .map(|d| d.label.clone())
            .find(|l| l.contains(fit))
            .unwrap();
        assert!(
            !label.contains("(+"),
            "{label:?} states {fit} alone when the sheet hides fit limits"
        );
    }
    let raw = std::str::from_utf8(include_bytes!("fixtures/parts/editor_pmi.step"))
        .unwrap()
        .replace("H7", "h7");
    let (part, analysis) =
        model::analyse_native_step(std::path::Path::new("shaft_class.step"), raw.as_bytes())
            .unwrap();
    let lowercase = Session::new(part, analysis, model::Spec::default()).unwrap();
    assert!(
        !lowercase
            .plan()
            .dimensions
            .iter()
            .any(|d| d.label.contains("h7")),
        "A shaft class may not be assigned to a bore."
    );
    assert!(
        s.spec().features.is_empty(),
        "PMI does not author feature overrides"
    );
}

#[test]
fn semantic_thread_ports_preserve_the_stated_fields_support_and_consumed_provenance() {
    use draftwright::haecceity::{self, p21, pmi as hpmi, step};
    use draftwright::pmi::{Field, Outcome};
    use std::io::Read as _;
    for (file, ids) in [
        ("hw_assembly_plate_pin_intent.step.gz", vec![1444, 1543]),
        ("hw_string_post_tapped_intent.step.gz", vec![3143]),
        ("hw_thumbwheel_thread_knurl_intent.step.gz", vec![704]),
    ] {
        let raw = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/pmi/step")
                .join(file),
        )
        .unwrap();
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(raw.as_slice())
            .read_to_end(&mut bytes)
            .unwrap();
        let part = haecceity::read_step(&bytes).unwrap();
        let defs = step::read_part_definitions(&bytes).unwrap();
        let doc = p21::Document::parse(bytes).unwrap();
        let reading = hpmi::read(&doc, &defs).unwrap();
        let report = draftwright::pmi::report(&doc, &defs, &part, &reading).unwrap();
        for id in ids {
            let r = report
                .records
                .iter()
                .find(|r| r.source_id == format!("manufacturing_requirement:#{id}"))
                .unwrap();
            assert_eq!(r.kind, "internal_thread");
            assert!(!r.faces.is_empty(), "{file}: thread support is retained");
            let input = &reading.parts[r.part];
            let prov = &reading.provenance.parts[r.part];
            let index = prov.threads.iter().position(|v| v.contains(&id)).unwrap();
            let thread = &input.threads[index];
            assert!(r.fields.contains(&(
                "designation".into(),
                Field::Text(thread.qualifier.clone().unwrap())
            )));
            assert!(
                r.fields
                    .contains(&("fit class".into(), Field::Text(thread.fit_class.clone())))
            );
            let attrs: Vec<_> = input
                .attributes
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    a.name == "thread manufacturing"
                        && a.on == Some(hpmi::NoteOwner::Feature(thread.feature))
                })
                .collect();
            assert_eq!(attrs.len(), 1);
            for instance in prov.threads[index]
                .iter()
                .chain(&prov.attributes[attrs[0].0])
            {
                assert!(
                    r.instances.contains(instance),
                    "{file}: #{instance} provenance lost"
                );
            }
            assert_eq!(
                report
                    .sources
                    .iter()
                    .find(|s| s.id == r.source_id)
                    .unwrap()
                    .outcome,
                Outcome::Extracted
            );
        }
    }
}

#[test]
fn pmi_placement_rejects_unproven_views_transactionally() {
    let mut s = session();
    let datum = s
        .plan()
        .gdt
        .iter()
        .find(|g| g.kind == "datum_feature" && g.label == "A")
        .unwrap()
        .id
        .clone();
    let path = format!("annotations/{}/place", draftwright::items::key(&datum));
    let before = serde_json::to_vec(s.state()).unwrap();
    let error = s
        .apply(&[Op::set(&path, json!({"view":"plan"}))], true)
        .unwrap_err();
    assert_eq!(serde_json::to_value(&error).unwrap()["code"], "invalid");
    assert_eq!(before, serde_json::to_vec(s.state()).unwrap());
    let frame = s
        .plan()
        .gdt
        .iter()
        .find(|g| g.kind == "control_frame")
        .unwrap()
        .id
        .clone();
    let options = s.options(&frame).unwrap();
    let view = ["front", "plan", "side"]
        .into_iter()
        .find(|view| {
            !options
                .placements
                .iter()
                .any(|p| p.view.as_deref() == Some(view))
        })
        .unwrap();
    assert!(
        s.apply(
            &[Op::set(
                format!("annotations/{}/place", draftwright::items::key(&frame)),
                json!({"view":view})
            )],
            true
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_vec(s.state()).unwrap());
    s.apply(&[Op::set(path, json!({"view":"side","pinned":true}))], true)
        .unwrap();
    assert_eq!(
        s.plan().gdt.iter().find(|g| g.id == datum).unwrap().side,
        "above"
    );
    s.apply(
        &[Op::set(
            format!("annotations/{}/place", draftwright::items::key(&datum)),
            json!({"view":"side","side":"below","pinned":true}),
        )],
        true,
    )
    .unwrap();
    assert_eq!(
        s.plan().gdt.iter().find(|g| g.id == datum).unwrap().side,
        "below",
        "S3 explicit strip choices stay pinned"
    );
    s.apply(&[Op::set("views/side/omit", json!(true))], true)
        .unwrap();
    s.apply(&[Op::set("views/side/omit", json!(false))], true)
        .unwrap();
    assert!(
        s.plan()
            .gdt
            .iter()
            .any(|g| g.id == datum && g.placed.is_some())
    );
}

#[test]
fn semantic_thread_nominal_disagreement_is_not_lowered() {
    assert!(
        session()
            .plan()
            .dimensions
            .iter()
            .any(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
    );
    let raw = std::str::from_utf8(include_bytes!("fixtures/parts/editor_pmi.step"))
        .unwrap()
        .replace("M5x0.8", "M6x0.8");
    let (part, analysis) =
        model::analyse_native_step(std::path::Path::new("bad_thread.step"), raw.as_bytes())
            .unwrap();
    let s = Session::new(part, analysis, model::Spec::default()).unwrap();
    assert!(
        !s.plan()
            .dimensions
            .iter()
            .any(|d| d.measurements.iter().any(|m| m.parameter == "thread"))
    );
    assert!(
        s.pmi_report()["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "internal_thread")
    );
}
