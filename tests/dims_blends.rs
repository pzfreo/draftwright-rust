//! G3: blend radii, exact occurrence credit and editable general radius notes.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::items::State;
use draftwright::model::{self, Spec};
use serde_json::json;

fn missing(a: &model::Analysis, p: &model::Plan, part: &draftwright::haecceity::Part) -> usize {
    let drawing = model::render(
        part,
        a,
        p,
        &draftwright::text::Font::new(include_bytes!("../fonts/IBMPlexMono-Regular.ttf").to_vec())
            .unwrap(),
    )
    .unwrap();
    draftwright::lint::lint(a, &drawing)
        .iter()
        .filter(|i| i.code == "blend_requirement_missing")
        .count()
}

/// ISSUE-1433-a, ARCH0015-1: numerous equal radii use notes with one exact credit per blend.
#[test]
fn many_blends_are_stated_without_counted_radius_clutter() {
    let Some(dir) = corpus::dir() else { return };
    for file in [
        "cadgenbench_inputs/cgb245.step",
        "nist/nist_ctc_01_asme1_rd.stp",
    ] {
        let (part, a) = model::analyse_native(&dir.join(file)).unwrap();
        let p = model::plan(&a, &Spec::default()).unwrap();
        let n = a.features.iter().filter(|f| f.kind == "blend").count();
        assert!(n > 20);
        if file.contains("cgb245") {
            assert_eq!(missing(&a, &p, &part), 0, "{file}");
            assert!(p.notes.iter().any(|n| n.placed.is_some()));
        } else {
            assert_eq!(
                missing(&a, &p, &part),
                23,
                "unlocated CTC-01 exceptions and their default stay missing"
            );
            assert!(p.notes.iter().all(|n| n.placed.is_none()));
        }
        assert!(
            p.notes
                .iter()
                .all(|n| !n.text.contains("REF SITE") && !n.text.contains("UNSTATED"))
        );
        for note in p.notes.iter().filter(|n| n.placed.is_some()) {
            for d in p.dimensions.iter().filter(|d| d.placed.is_some()) {
                assert!(!d.measurements.iter().any(|m| matches!(
                    m.parameter.as_str(),
                    "blend.radius" | "fillet.radius"
                )
                    && (m.value - note.measurements[0].value).abs() < 0.05));
            }
        }
        assert!(p.notes.iter().any(|n| n.text.contains("RADII R")), "{file}");
        assert!(
            p.dimensions
                .iter()
                .all(|d| !d.label.contains("31× R") && !d.label.contains("82× R"))
        );
        let again = model::plan(&a, &Spec::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(again).unwrap()
        );
    }
    let (part, a) = model::analyse_native(&dir.join("nist/nist_ctc_02_asme1_rc.stp")).unwrap();
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        missing(&a, &p, &part) >= 21,
        "the 18 unlocated R10 blends cannot receive note credit"
    );
    assert!(p.notes.iter().all(|n| n.placed.is_none()));
    assert!(
        p.notes
            .iter()
            .all(|n| !n.text.contains("UNSTATED") && !n.text.contains("REF SITE"))
    );
}

/// ISSUE-1433-a: a small equal group keeps a compact leader; omission loses every credit.
#[test]
fn few_blends_share_a_radius_callout_and_lose_credit_when_hidden() {
    let Some(dir) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&dir.join("gramel/string_post.step")).unwrap();
    let p = model::plan(&a, &Spec::default()).unwrap();
    let radius = p
        .dimensions
        .iter()
        .find(|d| d.measurements.iter().any(|m| m.parameter == "blend.radius"))
        .expect("blend radius callout");
    assert_eq!(radius.label, "2× R0.2");
    assert_eq!(radius.measurements.len(), 2);
    for f in a.features.iter().filter(|f| f.kind == "blend") {
        let sites = &a.blend_sites[&f.id];
        assert!(!sites.is_empty());
        for &site in sites {
            assert!(
                f.faces.iter().any(|&face| {
                    let surface = &part.faces[face].surface;
                    let Some((u, v)) = surface.parameters(site, None) else {
                        return false;
                    };
                    let projected = surface.value(u, v);
                    part.domain(face).is_some_and(|d| d.contains(u, v))
                        && (0..3).all(|k| (site[k] - projected[k]).abs() < 1e-6)
                }),
                "{}'s leader evidence is outside its defining surface",
                f.id
            );
        }
    }
    assert_eq!(missing(&a, &p, &part), 0);
    let spec = model::edit(&Spec::default(), &json!([{"op":"set", "path":format!("dimensions/{}/omit", draftwright::items::key(&radius.id)), "value":true}])).unwrap();
    let hidden = model::plan(&a, &spec).unwrap();
    assert_eq!(missing(&a, &hidden, &part), 2);
}

/// ISSUE-1351-a / LINT-_registry-structured_note_satisfaction: a note is editable by stable ID;
/// only drawn exact claims count. Prose cannot replace a hidden structured note.
#[test]
fn a_general_radius_note_can_be_hidden_and_moved_without_borrowing_prose_credit() {
    let Some(dir) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&dir.join("cadgenbench_inputs/cgb245.step")).unwrap();
    let p = model::plan(&a, &Spec::default()).unwrap();
    let note = p
        .notes
        .iter()
        .find(|n| n.text.contains("RADII R"))
        .expect("general radius note");
    let path = format!("annotations/{}", draftwright::items::key(&note.id));
    let with_prose = model::edit(
        &Spec::default(),
        &json!([
            {"op":"add", "path":"notes", "value":{"text":"BREAK ALL SHARP EDGES"}}
        ]),
    )
    .unwrap();
    let mixed = model::plan(&a, &with_prose).unwrap();
    assert_eq!(missing(&a, &mixed, &part), 0);
    assert!(mixed.notes_block.as_ref().unwrap().ruled);
    assert!(!mixed.radius_notes_block.as_ref().unwrap().ruled);
    assert!(mixed.notes.iter().all(|n| n.placed.is_some()));
    let exception = p
        .dimensions
        .iter()
        .find(|d| {
            d.placed.is_some()
                && d.measurements.iter().all(|m| m.parameter == "blend.radius")
                && !d.measurements.is_empty()
        })
        .unwrap();
    let less = model::edit(&Spec::default(), &json!([
        {"op":"set", "path":format!("dimensions/{}/omit", draftwright::items::key(&exception.id)), "value":true}
    ])).unwrap();
    let less = model::plan(&a, &less).unwrap();
    assert_eq!(
        missing(&a, &less, &part),
        exception.measurements.len()
            + note
                .measurements
                .iter()
                .filter(|m| m.parameter == "blend.radius")
                .count()
    );
    assert!(
        less.notes
            .iter()
            .any(|n| n.id == note.id && n.placed.is_none())
    );
    let spec = model::edit(
        &Spec::default(),
        &json!([
            {"op":"set", "path":format!("{path}/omit"), "value":true},
            {"op":"add", "path":"notes", "value":{"text":note.text}}
        ]),
    )
    .unwrap();
    let hidden = model::plan(&a, &spec).unwrap();
    assert!(missing(&a, &hidden, &part) > 0);
    assert_eq!(
        hidden.notes.iter().find(|n| n.id == note.id).unwrap().state,
        State::OmittedByUser
    );
    let moved_spec = model::edit(&Spec::default(), &json!([{"op":"set", "path":format!("{path}/place"), "value":{"view":"front", "side":"above", "lane":1}}])).unwrap();
    let moved = model::plan(&a, &moved_spec).unwrap();
    let moved_note = moved.notes.iter().find(|n| n.id == note.id).unwrap();
    assert_ne!(moved_note.placed, note.placed);
    assert_eq!(missing(&a, &moved, &part), 0);
    let unplaced_spec = model::edit(
        &moved_spec,
        &json!([{"op":"set", "path":"views/front/omit", "value":true}]),
    )
    .unwrap();
    let unplaced = model::plan(&a, &unplaced_spec).unwrap();
    assert!(
        unplaced
            .notes
            .iter()
            .find(|n| n.id == note.id)
            .unwrap()
            .placed
            .is_none()
    );
    assert!(
        unplaced
            .problems
            .iter()
            .any(|p| p.code == "notes_not_placed" && p.subjects.contains(&note.id))
    );
    assert!(missing(&a, &unplaced, &part) > 0);

    let font =
        draftwright::text::Font::new(include_bytes!("../fonts/IBMPlexMono-Regular.ttf").to_vec())
            .unwrap();
    let mut drawing = model::render(&part, &a, &p, &font).unwrap();
    let index = drawing
        .annotations
        .iter()
        .position(|n| n.name == note.id)
        .unwrap();
    for text in [
        "R3, 33 BLENDS; OTHER BLEND RADII UNSTATED",
        "R3, 33 BLENDS; REF SITE XYZ (1, 2, 3)",
    ] {
        let mut unlocated = drawing.clone();
        for &run in &unlocated.annotations[index].runs {
            unlocated.text[run].text = text.into();
        }
        assert_eq!(
            draftwright::lint::lint(&a, &unlocated)
                .iter()
                .filter(|i| i.code == "blend_requirement_missing")
                .count(),
            33
        );
    }
    let mut annotation = serde_json::to_value(&drawing.annotations[index]).unwrap();
    assert!(annotation["claims"].as_array().unwrap().is_empty());
    let lost = annotation["satisfies"].as_array_mut().unwrap().remove(0)["feature"]
        .as_str()
        .unwrap()
        .to_string();
    drawing.annotations[index] = serde_json::from_value(annotation).unwrap();
    drawing.annotations.push(drawing.annotations[index].clone());
    let missing: Vec<_> = draftwright::lint::lint(&a, &drawing)
        .into_iter()
        .filter(|i| i.code == "blend_requirement_missing")
        .collect();
    assert_eq!(
        missing.len(),
        1,
        "duplicate notes cannot borrow another occurrence's credit"
    );
    assert_eq!(missing[0].subjects, [lost]);
}

/// ISSUE-1450-a: same-face evidence decides ownership; an equal but unrelated boss survives.
#[test]
fn a_blend_owns_its_cylindrical_face_without_erasing_unrelated_equal_bosses() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parts/prismatic.step");
    let (_, mut a) = model::analyse_native(&fixture).unwrap();
    let template = a.features[0].clone();
    a.features.clear();
    for (id, kind, faces, parameters) in [
        (
            "f:blend:1",
            "blend",
            vec![1],
            json!({"radius":2.0,"path":{"at":[10.0,10.0,0.0],"direction":[0.0,0.0,1.0]}}),
        ),
        (
            "f:boss:1",
            "boss",
            vec![1],
            json!({"diameter":4.0,"height":10.0,"axis":[0.0,0.0,1.0],"location":[10.0,10.0,10.0]}),
        ),
        (
            "f:boss:2",
            "boss",
            vec![2],
            json!({"diameter":4.0,"height":10.0,"axis":[0.0,0.0,1.0],"location":[20.0,10.0,10.0]}),
        ),
    ] {
        let mut f = template.clone();
        f.id = id.into();
        f.kind = kind.into();
        f.faces = faces;
        f.parameters = parameters.as_object().unwrap().clone();
        a.features.push(f);
    }
    a.envelope = [[0.0, 0.0, 0.0], [40.0, 30.0, 10.0]];
    a.levels.clear();
    a.level_supports.clear();
    a.risers.clear();
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        !p.dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:boss:1" && m.parameter == "boss.diameter")
    );
    assert!(
        p.dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:boss:2" && m.parameter == "boss.diameter")
    );
    assert!(
        !draftwright::lint::coverage::inventory(&a)
            .iter()
            .flat_map(|(_, rs)| rs)
            .any(|r| r.feature == "f:boss:1")
    );
    a.features[1].parameters.insert("height".into(), json!(5.0));
    let short = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        !short
            .dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:boss:1" && m.parameter == "boss.diameter")
    );
    assert!(
        short
            .dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:boss:1" && m.parameter == "boss_height.length")
    );
    a.features[1].faces.clear();
    let p = model::plan(&a, &Spec::default()).unwrap();
    assert!(
        p.dimensions
            .iter()
            .flat_map(|d| &d.measurements)
            .any(|m| m.feature == "f:boss:1" && m.parameter == "boss.diameter")
    );
}

/// A bare pin retains automatic presentation, including consolidated radius ownership.
#[test]
fn a_bare_pin_keeps_a_shared_radius_leader_and_its_default() {
    let Some(dir) = corpus::dir() else { return };
    let (part, a) = model::analyse_native(&dir.join("cadgenbench_inputs/cgb245.step")).unwrap();
    let p = model::plan(&a, &Spec::default()).unwrap();
    let shared = p
        .dimensions
        .iter()
        .find(|d| {
            d.measurements.iter().any(|m| m.parameter == "blend.radius")
                && d.measurements
                    .iter()
                    .any(|m| m.parameter == "fillet.radius")
        })
        .unwrap();
    let spec = model::edit(
        &Spec::default(),
        &json!([{
            "op":"set", "path":format!("dimensions/{}/place", draftwright::items::key(&shared.id)),
            "value":{"pinned":true}
        }]),
    )
    .unwrap();
    let pinned = model::plan(&a, &spec).unwrap();
    let held = pinned
        .dimensions
        .iter()
        .find(|d| d.id == shared.id)
        .unwrap();
    assert_eq!(held.label, shared.label);
    assert_eq!(held.placed, shared.placed);
    assert_eq!(
        serde_json::to_value(&held.measurements).unwrap(),
        serde_json::to_value(&shared.measurements).unwrap()
    );
    assert_eq!(
        pinned.dimensions.iter().map(|d| &d.id).collect::<Vec<_>>(),
        p.dimensions.iter().map(|d| &d.id).collect::<Vec<_>>()
    );
    assert_eq!(
        pinned
            .notes
            .iter()
            .map(|n| (&n.text, &n.placed))
            .collect::<Vec<_>>(),
        p.notes
            .iter()
            .map(|n| (&n.text, &n.placed))
            .collect::<Vec<_>>()
    );
    assert_eq!(missing(&a, &pinned, &part), 0);
}
