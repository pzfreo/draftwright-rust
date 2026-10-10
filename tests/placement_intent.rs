//! S3: seeded edits preserve pinned geometry, changes re-resolve intent, exports replay it.
#[path = "common/corpus.rs"]
mod corpus;

use draftwright::model::{self, Analysis, Plan, Spec};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

fn fixture(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(p)
}
fn plate() -> (draftwright::haecceity::Part, Analysis) {
    let editor = Path::new("/Users/paul/repos/draftwright-editor/public/samples/plate.step");
    let fallback = fixture("parts/mounting_plate.step");
    model::analyse_native(if editor.exists() { editor } else { &fallback }).unwrap()
}
fn slice() -> Vec<(String, Analysis)> {
    let mut parts = vec![("plate".into(), plate().1)];
    if let Some(c) = corpus::dir() {
        let m = c.join("mfcadpp/10127.step");
        let editor = Path::new("/Users/paul/repos/draftwright-editor/web/samples/10127.step");
        parts.push((
            "10127".into(),
            model::analyse_native(if m.exists() { &m } else { editor })
                .unwrap()
                .1,
        ));
        for (name, file) in [
            ("GRM-03", "gramel/GRM-03_thumbwheel_drive_screw.step"),
            ("CTC-01", "nist/nist_ctc_01_asme1_rd.stp"),
        ] {
            parts.push((name.into(), model::analyse_native(&c.join(file)).unwrap().1));
        }
    }
    parts
}
fn pmi_part() -> (draftwright::haecceity::Part, Analysis) {
    let file = fixture("pmi/step/nist_ctc_01_asme1_ap242-e1.stp.gz");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(file).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("placement_intent")
        .join(format!("{}-{n}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("nist_ctc_01_asme1_ap242-e1.stp");
    std::fs::write(&path, bytes).unwrap();
    model::analyse_native(&path).unwrap()
}
fn pmi() -> Analysis {
    pmi_part().1
}
fn spec(id: &str, place: Value) -> Spec {
    serde_json::from_value(json!({"dimensions":{id:{"place":place}}})).unwrap()
}
fn geometry(p: &Plan, id: &str) -> Value {
    if let Some(d) = p.dimensions.iter().find(|d| d.id == id) {
        return serde_json::to_value(&d.placed).unwrap();
    }
    if let Some(g) = p.gdt.iter().find(|g| g.id == id) {
        return serde_json::to_value(&g.placed).unwrap();
    }
    if let Some(v) = p.views.iter().find(|v| v.id == id) {
        return json!({"at":v.at,"area":v.area,"scale":v.scale,"direction":v.direction,"up":v.up});
    }
    Value::Null
}
fn path(id: &str) -> String {
    if id.starts_with("v:") {
        format!("views/{}", id.trim_start_matches("v:"))
    } else {
        format!(
            "{}/{}",
            if id.starts_with("pmi:") {
                "annotations"
            } else {
                "dimensions"
            },
            draftwright::items::key(id)
        )
    }
}
fn set(s: &Spec, id: &str, field: &str, value: Value) -> Spec {
    model::edit(
        s,
        &json!([{"op":"set","path":format!("{}/{field}",path(id)),"value":value}]),
    )
    .unwrap()
}
fn side(p: &Plan, id: &str) -> &'static str {
    match p
        .dimensions
        .iter()
        .find(|d| d.id == id)
        .and_then(|d| d.placed.as_ref())
        .map(|p| p.rotation)
    {
        Some(90.0) => "right",
        _ => "above",
    }
}
fn random(seed: &mut u64, n: usize) -> usize {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 32) as usize % n
}
fn pin(s: &Spec, p: &Plan, id: &str, lane: usize) -> Spec {
    let place = if p.views.iter().any(|v| v.id == id) {
        json!({"relative_to":"sheet","side":"above","gap":10+lane*5,"align":"start","pinned":true})
    } else {
        let view = draftwright::items::items(p)
            .into_iter()
            .find(|i| i.id == id)
            .unwrap()
            .view
            .unwrap();
        json!({"view":view.trim_start_matches("v:"),"side":side(p,id),"lane":lane,"along":"middle","pinned":true})
    };
    set(s, id, "place", place)
}

#[test]
fn placement_vocabulary_is_validated_without_page_coordinates() {
    let s = spec(
        "d:envelope/width.length",
        json!({"view":"front","side":"above","lane":2,"along":0.25,"pinned":true}),
    );
    for bad in [
        json!({"at":[10,20]}),
        json!({"lane":-1}),
        json!({"lane":1001}),
        json!({"along":1.1}),
        json!({"side":"north"}),
        json!({"view":"iso"}),
    ] {
        assert!(model::edit(&s,&json!([{"op":"set","path":"dimensions/d:envelope~1width.length/place","value":bad}])).is_err(),"{bad}");
    }
    for along in [
        json!(0),
        json!(1),
        json!("start"),
        json!("middle"),
        json!("end"),
        json!("outside"),
    ] {
        set(&s, "d:envelope/width.length", "place/along", along);
    }
    let cycle = json!({"views":{"front":{"place":{"relative_to":"plan","side":"below"}},"plan":{"place":{"relative_to":"front","side":"above"}}}});
    assert!(
        model::edit(
            &Spec::default(),
            &json!([{"op":"set","path":"views","value":cycle["views"]}])
        )
        .is_err()
    );
    for (_, a) in slice() {
        let base = model::plan_recovered(&a, &Spec::default()).unwrap();
        let id = &base
            .dimensions
            .iter()
            .find(|d| d.id.starts_with("d:envelope/"))
            .unwrap()
            .id;
        let explicit = spec(id, json!({"pinned":false}));
        let p = model::plan_recovered(&a, &explicit).unwrap();
        for d in &base.dimensions {
            assert_eq!(geometry(&base, &d.id), geometry(&p, &d.id));
        }
        for v in &base.views {
            assert_eq!(geometry(&base, &v.id), geometry(&p, &v.id));
        }
        for problem in base.problems.iter().filter(|p| {
            p.code.starts_with("annotation_")
                && p.subjects
                    .iter()
                    .any(|id| base.dimensions.iter().any(|d| d.id == *id))
        }) {
            assert!(!problem.fix.is_empty());
            model::edit(
                &Spec::default(),
                &serde_json::to_value(&problem.fix).unwrap(),
            )
            .unwrap();
        }
    }
    let (_, a) = plate();
    let p = model::plan(&a, &s).unwrap();
    let d = p
        .dimensions
        .iter()
        .find(|d| d.id == "d:envelope/width.length")
        .unwrap();
    assert!(d.from_spec.contains(&"place".into()));
    let a = pmi();
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let angle = base
        .dimensions
        .iter()
        .find(|d| d.kind == "angular")
        .unwrap();
    for along in [
        json!("start"),
        json!("middle"),
        json!("end"),
        json!("outside"),
    ] {
        let s = spec(
            &angle.id,
            json!({"view":"plan","lane":2,"along":along,"pinned":true}),
        );
        let p = model::plan_recovered(&a, &s).unwrap();
        let held = geometry(&p, &angle.id);
        assert!(!held.is_null());
        let changed = set(&s, "d:envelope/width.length", "omit", json!(true));
        assert_eq!(
            geometry(&model::plan_recovered(&a, &changed).unwrap(), &angle.id),
            held
        );
    }
    let (_, a) = plate();
    let legacy = set(&Spec::default(), &d.id, "view", json!("plan"));
    let legacy = set(&legacy, &d.id, "place", json!({"pinned":true}));
    assert_eq!(
        model::plan_recovered(&a, &legacy)
            .unwrap()
            .dimensions
            .iter()
            .find(|i| i.id == d.id)
            .unwrap()
            .view,
        "v:plan"
    );
    let bad = spec(&d.id, json!({"side":"left"}));
    assert_eq!(
        model::plan(&a, &bad).unwrap_err().choices,
        vec!["above", "below"]
    );
}

#[test]
fn seeded_edits_on_slice_parts_preserve_pinned_geometry() {
    let mut parts = slice();
    parts.push(("CTC-01 AP242".into(), pmi()));
    for (name, a) in parts {
        let base = model::plan_recovered(&a, &Spec::default()).unwrap();
        let eligible: Vec<String> = base
            .dimensions
            .iter()
            .filter(|d| d.placed.is_some() && d.kind != "angular")
            .map(|d| d.id.clone())
            .chain(
                base.gdt
                    .iter()
                    .filter(|g| g.placed.is_some())
                    .map(|g| g.id.clone()),
            )
            .chain(
                base.views
                    .iter()
                    .filter(|v| v.id == "v:iso")
                    .map(|v| v.id.clone()),
            )
            .collect();
        for seed in [0x563, 0x314159] {
            let mut rng = seed;
            let n = 1 + random(&mut rng, eligible.len().min(4));
            let mut pool = eligible.clone();
            let mut pins = BTreeSet::new();
            let mut s = Spec::default();
            for _ in 0..n {
                let id = pool.remove(random(&mut rng, pool.len()));
                s = pin(&s, &base, &id, random(&mut rng, 3));
                pins.insert(id);
            }
            let pinned = model::plan_recovered(&a, &s).unwrap();
            let snapshot: BTreeMap<_, _> = pins
                .iter()
                .map(|id| (id.clone(), geometry(&pinned, id)))
                .collect();
            assert!(
                snapshot.values().all(|v| !v.is_null()),
                "{name}: {snapshot:?}"
            );
            let mut distribution = BTreeMap::new();
            for problem in &pinned.problems {
                *distribution.entry(problem.code.clone()).or_insert(0) += 1;
            }
            eprintln!("pins {name} seed={seed} N={n}: {distribution:?}");
            let anchors: BTreeSet<_> = draftwright::items::items(&pinned)
                .into_iter()
                .filter(|i| pins.contains(&i.id))
                .filter_map(|i| i.view)
                .collect();
            let mut free: Vec<String> = eligible
                .iter()
                .filter(|id| !pins.contains(*id) && !id.starts_with("v:"))
                .cloned()
                .collect();
            free.extend(
                base.views
                    .iter()
                    .filter(|v| !pins.contains(&v.id) && !anchors.contains(&v.id))
                    .map(|v| v.id.clone()),
            );
            for _ in 0..16 {
                let id = &free[random(&mut rng, free.len())];
                s = if id.starts_with("v:") {
                    set(
                        &s,
                        id,
                        "place",
                        json!({"relative_to":"sheet","side":"above","gap":random(&mut rng,40),"align":"end"}),
                    )
                } else {
                    match random(&mut rng, 3) {
                        0 => set(&s, id, "omit", json!(random(&mut rng, 2) == 0)),
                        1 if id.starts_with("d:") => set(
                            &s,
                            id,
                            "tolerance",
                            json!(if random(&mut rng, 2) == 0 {
                                "±0.1"
                            } else {
                                "±0.2"
                            }),
                        ),
                        _ => set(
                            &s,
                            id,
                            "place",
                            json!({"side":side(&base,id),"lane":random(&mut rng,4),"along":random(&mut rng,100) as f64/100.0}),
                        ),
                    }
                };
                let p = model::plan_recovered(&a, &s).unwrap();
                for (id, expected) in &snapshot {
                    assert_eq!(
                        serde_json::to_vec(&geometry(&p, id)).unwrap(),
                        serde_json::to_vec(expected).unwrap(),
                        "{name} seed={seed} pin={id}"
                    );
                }
            }
        }
    }
}

#[test]
fn sheet_and_scale_changes_keep_or_report_every_pin() {
    for (name, a) in slice()
        .into_iter()
        .chain(std::iter::once(("AP242".into(), pmi())))
    {
        let base = model::plan_recovered(&a, &Spec::default()).unwrap();
        let mut s = Spec::default();
        let mut pins = Vec::new();
        for id in base
            .dimensions
            .iter()
            .filter(|d| d.placed.is_some())
            .take(3)
            .map(|d| d.id.clone())
            .chain(
                base.gdt
                    .iter()
                    .filter(|g| g.placed.is_some())
                    .take(2)
                    .map(|g| g.id.clone()),
            )
            .chain(std::iter::once("v:front".into()))
        {
            s = pin(&s, &base, &id, 1);
            pins.push(id);
        }
        let before = serde_json::to_value(&s).unwrap();
        for (size, scale) in [("A4", "1:5"), ("A3", "2:1"), ("A0", "1:1")] {
            let changed=model::edit(&s,&json!([{"op":"set","path":"sheet/size","value":size},{"op":"set","path":"sheet/scale","value":scale}])).unwrap();
            let p = model::plan_recovered(&a, &changed).unwrap();
            let after = serde_json::to_value(&p.spec).unwrap();
            for id in &pins {
                assert_eq!(
                    before.pointer(&format!("/{}/place", path(id))),
                    after.pointer(&format!("/{}/place", path(id))),
                    "{name} {id}"
                );
                assert!(
                    !geometry(&p, id).is_null()
                        || p.problems
                            .iter()
                            .any(|p| p.subjects.contains(id) || p.refs.contains(id)),
                    "{name} {id}"
                );
            }
        }
    }
}

#[test]
fn colliding_and_off_sheet_pins_are_reported_and_never_dropped() {
    let (_, a) = plate();
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let hole = base
        .dimensions
        .iter()
        .find(|d| d.kind == "callout" && d.id.starts_with("d:f:hole"))
        .unwrap();
    let s = spec(
        &hole.id,
        json!({"view":hole.view.trim_start_matches("v:"),"lane":100,"pinned":true}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(!geometry(&p, &hole.id).is_null());
    assert!(
        !p.problems
            .iter()
            .any(|p| p.code.ends_with("_dropped") && p.subjects.contains(&hole.id))
    );
    let problem = p
        .problems
        .iter()
        .find(|p| p.code == "annotation_off_sheet" && p.subjects.contains(&hole.id))
        .unwrap();
    assert!(problem.fix.iter().all(|op| op.path.ends_with("/place")));
    let fixed = model::edit(&p.spec, &serde_json::to_value(&problem.fix).unwrap()).unwrap();
    let repaired = model::plan_recovered(&a, &fixed).unwrap();
    assert!(
        !repaired
            .problems
            .iter()
            .any(|p| p.code == "annotation_off_sheet" && p.subjects.contains(&hole.id))
    );

    if let Some(c) = corpus::dir() {
        let a = model::analyse_native(&c.join("cadgenbench_inputs/cgb245.step"))
            .unwrap()
            .1;
        let base = model::plan_recovered(&a, &Spec::default()).unwrap();
        let covered = base
            .omitted
            .iter()
            .find(|o| matches!(o.state, draftwright::items::State::Covered { .. }))
            .expect("cgb245 consolidates its coincident slot widths");
        let s = spec(&covered.id, json!({"pinned":true}));
        let p = model::plan_recovered(&a, &s).unwrap();
        let mut claims = BTreeSet::new();
        for d in &p.dimensions {
            for m in &d.measurements {
                assert!(
                    claims.insert((&m.feature, &m.parameter)),
                    "a pin duplicated {m:?}"
                );
            }
        }
        if geometry(&p, &covered.id).is_null() {
            assert!(p.problems.iter().any(|p| p.subjects.contains(&covered.id)));
        }
    }
    let mut s = Spec::default();
    for name in ["front", "plan"] {
        s = set(
            &s,
            &format!("v:{name}"),
            "place",
            json!({"relative_to":"sheet","side":"above","gap":10,"align":"start","pinned":true}),
        );
    }
    let p = model::plan_recovered(&a, &s).unwrap();
    for id in ["v:front", "v:plan"] {
        assert!(p.problems.iter().any(|p| p.code == "pinned_view_overlap"
            && p.subjects.contains(&id.to_string())
            && !p.fix.is_empty()));
    }
    for problem in p
        .problems
        .iter()
        .filter(|p| p.code == "pinned_view_overlap")
    {
        model::edit(&p.spec, &serde_json::to_value(&problem.fix).unwrap()).unwrap();
    }
    let s = set(&s, "v:front", "place/gap", json!(1000));
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.problems
            .iter()
            .any(|p| p.code == "pinned_view_off_sheet" && p.subjects.contains(&"v:front".into()))
    );
    assert!(p.views.iter().find(|v| v.id == "v:front").unwrap().at[1] > 1000.0);
}

#[test]
fn views_and_pmi_have_relative_placement() {
    let a = pmi();
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let g = base.gdt.iter().find(|g| g.placed.is_some()).unwrap();
    let mut s:Spec=serde_json::from_value(json!({"views":{"plan":{"place":{"relative_to":"front","side":"above","gap":20,"align":"middle","pinned":true}}},"annotations":{g.id.clone():{"place":{"view":"front","side":"left","lane":1,"along":"middle","pinned":true}}}})).unwrap();
    let p = model::plan_recovered(&a, &s).unwrap();
    let front = p.views.iter().find(|v| v.id == "v:front").unwrap();
    let plan = p.views.iter().find(|v| v.id == "v:plan").unwrap();
    assert!((plan.area[1] - front.area[3] - 20.0).abs() < 1e-9);
    let placed = p.gdt.iter().find(|i| i.id == g.id).unwrap();
    assert_eq!(placed.view, "v:front");
    assert_eq!(placed.side, "left");
    let held = geometry(&p, &g.id);
    for d in base.dimensions.iter().take(4) {
        s = set(&s, &d.id, "omit", json!(true));
    }
    assert_eq!(
        geometry(&model::plan_recovered(&a, &s).unwrap(), &g.id),
        held
    );
    s = set(&s, "v:front", "omit", json!(true));
    let p = model::plan_recovered(&a, &s).unwrap();
    assert!(
        p.problems
            .iter()
            .any(|p| p.subjects.contains(&g.id) && !p.fix.is_empty())
    );
    assert_eq!(p.spec.annotations[&g.id].place, s.annotations[&g.id].place);

    // Free views fill around the held footprint rather than moving it.
    let (_, a) = plate();
    let base = model::plan_recovered(&a, &Spec::default()).unwrap();
    let s = set(
        &Spec::default(),
        "v:front",
        "place",
        json!({"relative_to":"sheet","side":"above","gap":100,"align":"start","pinned":true}),
    );
    let p = model::plan_recovered(&a, &s).unwrap();
    assert_ne!(geometry(&base, "v:front"), geometry(&p, "v:front"));
    let front = p.views.iter().find(|v| v.id == "v:front").unwrap();
    assert!(
        p.views
            .iter()
            .filter(|v| v.id != "v:front")
            .all(|v| !overlap(v.area, front.area)),
        "views: {:?}",
        p.views.iter().map(|v| (&v.id, v.area)).collect::<Vec<_>>()
    );
}
fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

#[test]
fn exported_spec_with_place_renders_identically() {
    for (part, a) in [plate(), pmi_part()] {
        let s = spec(
            "d:envelope/width.length",
            json!({"side":"above","along":"end","pinned":true}),
        );
        let s = set(
            &s,
            "v:plan",
            "place",
            json!({"relative_to":"front","side":"above","gap":10,"align":"middle","pinned":true}),
        );
        let base = model::plan_recovered(&a, &s).unwrap();
        let s = if let Some(g) = base.gdt.iter().find(|g| g.placed.is_some()) {
            pin(&s, &base, &g.id, 1)
        } else {
            s
        };
        let p = model::plan_recovered(&a, &s).unwrap();
        let font = draftwright::text::Font::new(
            std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"),
            )
            .unwrap(),
        )
        .unwrap();
        let first = model::render(&part, &a, &p, &font).unwrap();
        let fonts = |name: &str| {
            std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("fonts")
                    .join(name),
            )
            .ok()
        };
        let files = vec![
            first.svg().into_bytes(),
            first.dxf(),
            first.pdf(&fonts),
            first.png(25.0).unwrap(),
        ];
        for file in files {
            let embedded = model::embedded_spec(&file).unwrap().unwrap();
            assert_eq!(
                embedded.dimensions["d:envelope/width.length"].place,
                s.dimensions["d:envelope/width.length"].place
            );
            assert_eq!(embedded.annotations, p.spec.annotations);
            assert_eq!(embedded.views, p.spec.views);
            let again = model::plan_recovered(&a, &embedded).unwrap();
            let drawn = model::render(&part, &a, &again, &font).unwrap();
            assert_eq!(first.svg(), drawn.svg());
            assert_eq!(first.dxf(), drawn.dxf());
            assert_eq!(first.pdf(&fonts), drawn.pdf(&fonts));
            assert_eq!(first.png(25.0).unwrap(), drawn.png(25.0).unwrap());
        }
    }
}
