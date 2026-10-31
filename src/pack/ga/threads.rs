//! Reuse recognised physical threads and the ordinary ISO 6410 drawing carriers.
//! Grouping already proves the face correspondence; STEP entities identify those
//! same faces in each placed instance without another recognition pass.
use super::*;
use std::collections::BTreeSet;

pub(super) fn prepare(
    part: &haecceity::Part,
    a: &mut Analysis,
    file: &StepFile,
    frame: &haecceity::step::Placement,
    reused: &BTreeMap<usize, (&haecceity::Part, &Analysis)>,
    aliases: &BTreeMap<usize, crate::pack::Alias>,
) -> BTreeSet<usize> {
    // A carrier preserves solid order; surface models have no such ownership.
    if part.faces.iter().any(|f| f.solid.is_none()) {
        return BTreeSet::new();
    }
    let source: BTreeMap<_, _> = (0..part.faces.len())
        .filter_map(|f| part.face_source(f).map(|s| ((s.instance, s.entity), f)))
        .collect();
    let mut enabled = BTreeSet::new();
    for (&representative, (local, analysis)) in reused {
        if analysis.squared.is_some() {
            continue;
        }
        let threads: Vec<_> = analysis
            .features
            .iter()
            .filter(|f| f.kind == "modelled_thread")
            .collect();
        if threads.is_empty() {
            continue;
        }
        let mut features = Vec::new();
        let complete = file
            .parts
            .iter()
            .enumerate()
            .filter(|(i, _)| aliases.get(i).map_or(*i, |a| a.part) == representative)
            .all(|(index, definition)| {
                let alias = aliases.get(&index);
                let alignment = alias.map_or(haecceity::step::IDENTITY, |a| a.placement);
                definition
                    .placements
                    .iter()
                    .enumerate()
                    .all(|(placement, placed)| {
                        let motion = projection::compose(
                            frame,
                            &projection::compose(&placed.placement, &alignment),
                        );
                        threads.iter().all(|thread| {
                            let faces: Option<Vec<_>> = thread
                                .faces
                                .iter()
                                .map(|&face| {
                                    let original = local.face_source(face)?.entity;
                                    let entity = if index == representative {
                                        original
                                    } else {
                                        *alias?.faces.get(&original)?
                                    };
                                    placed.instances.iter().find_map(|&instance| {
                                        source.get(&(instance, entity)).copied()
                                    })
                                })
                                .collect();
                            let Some(mut faces) = faces else {
                                return false;
                            };
                            faces.sort_unstable();
                            let mut feature = (*thread).clone();
                            feature.id = format!("f:ga_thread:{index}:{placement}:{}", thread.id);
                            feature.faces = faces;
                            let Some(axis) = feature.parameters.get_mut("axis") else {
                                return false;
                            };
                            for field in ["point", "direction"] {
                                let Some(p) = axis[field].as_array().and_then(|p| {
                                    if p.len() != 3 {
                                        return None;
                                    }
                                    Some([p[0].as_f64()?, p[1].as_f64()?, p[2].as_f64()?])
                                }) else {
                                    return false;
                                };
                                let q = [0, 1, 2].map(|k| {
                                    (if field == "point" { motion[k][3] } else { 0. })
                                        + (0..3).map(|j| motion[k][j] * p[j]).sum::<f64>()
                                });
                                axis[field] = serde_json::json!(q);
                            }
                            feature.fingerprint.position = [0, 1, 2].map(|k| {
                                motion[k][3]
                                    + (0..3)
                                        .map(|j| motion[k][j] * thread.fingerprint.position[j])
                                        .sum::<f64>()
                            });
                            feature.fingerprint.axis = thread.fingerprint.axis.map(|p| {
                                [0, 1, 2].map(|k| (0..3).map(|j| motion[k][j] * p[j]).sum())
                            });
                            features.push(feature);
                            true
                        })
                    })
            });
        if complete {
            enabled.insert(representative);
            a.features.extend(features);
        }
    }
    enabled
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognised_threads_retain_placed_source_faces_and_standard_carriers() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assembly/pack-shapes.step.gz");
        let bytes = model::read_step_file(&path).unwrap();
        let file = StepFile::read(&bytes).unwrap();
        let (local, mut local_analysis) = model::analyse_step_part(&path, &bytes, Some(5)).unwrap();
        let recognition = quiddity::correspondence::recognise(&local);
        let doc = quiddity::recognition::document(&recognition, local.faces.len());
        model::recognise_document(&local, &mut local_analysis, &doc).unwrap();
        let original = local_analysis
            .features
            .iter()
            .find(|f| f.kind == "modelled_thread")
            .unwrap();
        let (whole, mut a, frame) = model::analyse_pack_assembly("threads", &bytes, None).unwrap();
        let mut spec = Spec::default();
        spec.sheet.hidden = Some(false);
        let mut plan = crate::api::plan_analysed(&a, &spec).unwrap();
        plan.views.retain(|v| v.id == "v:front");
        let reused = BTreeMap::from([(5, (&local, &local_analysis))]);
        let enabled = prepare(&whole, &mut a, &file, &frame, &reused, &BTreeMap::new());
        assert_eq!(enabled, BTreeSet::from([5]));
        let thread = a
            .features
            .iter()
            .find(|f| f.kind == "modelled_thread")
            .unwrap();
        assert_eq!(thread.faces.len(), original.faces.len());
        for &f in &thread.faces {
            let source = whole.face_source(f).unwrap();
            assert!(
                file.parts[5].placements[0]
                    .instances
                    .contains(&source.instance)
            );
            assert!(
                original
                    .faces
                    .iter()
                    .any(|&i| local.face_source(i).unwrap().entity == source.entity)
            );
        }
        let carrier = crate::modelled_threads::carrier(&whole, &a, &spec, true).unwrap();
        assert!(carrier.faces.len() < whole.faces.len());
        let mut projections = model::Projections::default();
        projection::prepare(
            &whole,
            &a,
            &plan,
            &file,
            &frame,
            &reused,
            &BTreeMap::new(),
            None,
            &enabled,
            &mut projections,
        )
        .unwrap();
        let drawing = model::render_geometry(&whole, &a, &plan, &mut projections).unwrap();
        assert_eq!(
            projections.stats().projections,
            0,
            "composed conventional geometry must seed the renderer's thread-intent key"
        );
        assert!(!drawing.views[0].1.visible.is_empty());
        assert!(crate::modelled_threads::carrier(&whole, &a, &spec, false).is_none());
        plan.views[0].line_style.thread_depiction = Some(false);
        let mut physical = model::Projections::default();
        projection::prepare(
            &whole,
            &a,
            &plan,
            &file,
            &frame,
            &reused,
            &BTreeMap::new(),
            None,
            &enabled,
            &mut physical,
        )
        .unwrap();
        let physical_drawing = model::render_geometry(&whole, &a, &plan, &mut physical).unwrap();
        assert_eq!(physical.stats().projections, 0);
        assert!(
            physical_drawing.views[0].1.visible.len() > drawing.views[0].1.visible.len(),
            "physical helical flanks must replace the conventional carrier when disabled"
        );
    }
}
