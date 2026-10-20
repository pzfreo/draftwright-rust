//! Physical threads get one designation; conventional ink depicts their extent.
use super::*;
use crate::modelled_threads::{Thread, threads};

pub(super) const PLANNER: Planner = Planner {
    kinds: &["modelled_thread"],
    wants,
};
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for f in cx
        .analysis
        .features
        .iter()
        .filter(|f| f.kind == "modelled_thread")
    {
        let t = match Thread::read(f) {
            Ok(t) => t,
            Err(why) => {
                problems.push(not_depicted(f, why));
                continue;
            }
        };
        let k = crate::bridge::holes::principal(t.axis).unwrap_or_else(|| {
            (0..3)
                .max_by(|&a, &b| t.axis[a].abs().total_cmp(&t.axis[b].abs()))
                .expect("three coordinates")
        });
        let (text, standard) = t.designation();
        if !standard {
            problems.push(unmatched(f));
        }
        let at = t.at((t.span[0] + t.span[1]) / 2.);
        let sites = (0..3)
            .filter(|&j| j != k)
            .flat_map(|j| {
                [-1., 1.].map(move |side| {
                    let radial = [0, 1, 2].map(|i| f64::from(i == j) - t.axis[j] * t.axis[i]);
                    let radial = haecceity::geom::unit(radial).expect("transverse coordinate");
                    (
                        haecceity::geom::add(at, haecceity::geom::scale(radial, side * t.crest())),
                        0.,
                    )
                })
            })
            .collect();
        let mut views = super::envelope::profile_views(k);
        // Separate coaxial occurrences must remain distinguishable by axial
        // position; end-on leaders would otherwise share exactly the same rim.
        let coaxial = threads(cx.analysis).any(|other| {
            other.feature.id != f.id
                && haecceity::geom::dot(other.axis, t.axis).abs() > 1. - 1e-8
                && haecceity::geom::norm(haecceity::geom::cross(
                    haecceity::geom::sub(other.on, t.on),
                    t.axis,
                )) < 1e-6
        });
        if t.internal && !coaxial {
            views.insert(0, end_on(k));
        }
        out.push(Want {
            id:want_id(&f.id,"thread"), views,value:t.major,
            terms:vec![text],
            measurements:vec![Measurement {feature:f.id.clone(),parameter:"thread".into(),value:t.major},Measurement {feature:f.id.clone(),parameter:"thread.length".into(),value:t.span[1]-t.span[0]}],
            basis:"ISO 6410; designation inferred from a unique ISO 261 diameter/pitch match; thread extent is depicted; trimmed span retained only as measurement evidence; features/<id>/modelled_thread overrides depiction and designation".into(),
            shape:Shape::Callout {sites,arcs:Vec::new()},turning:None,
        });
    }
    (out, problems)
}
fn not_depicted(f: &crate::bridge::Feature, why: &str) -> Problem {
    Problem {
        id: format!("p:modelled_thread_not_depicted:{}", f.id),
        code: "modelled_thread_not_depicted".into(),
        severity: "info".into(),
        message: format!(
            "ISO 6410 conventional thread depiction is unavailable: {why}. The source modelled edges remain drawn."
        ),
        subjects: vec![f.id.clone()],
        ..Default::default()
    }
}
fn unmatched(f: &crate::bridge::Feature) -> Problem {
    Problem {id:format!("p:thread_designation_measured:{}",f.id),code:"thread_designation_measured".into(),severity:"info".into(),message:"No unique ISO 261 diameter/pitch match within the documented inference tolerance; the callout states measured diameter and pitch. Set features/<id>/modelled_thread/designation to author the intended designation.".into(),subjects:vec![f.id.clone()],..Default::default()}
}

/// Apply intent and PMI precedence after the immutable automatic wants memo.
pub(super) fn apply(a: &Analysis, s: &Spec, out: &mut Vec<Want>, problems: &mut Vec<Problem>) {
    let manufacturing = if s.pmi == crate::model::PmiMode::Ignore
        || !a.features.iter().any(|f| f.kind == "modelled_thread")
    {
        crate::pmi_lowering::Manufacturing::default()
    } else {
        crate::pmi_lowering::manufacturing(a)
    };
    for t in threads(a) {
        let setting = s
            .features
            .get(&t.feature.id)
            .and_then(|f| f.modelled_thread.as_ref());
        if !t.enabled(s) || s.sheet.thread_depiction == Some(false) {
            problems.push(not_depicted(
                t.feature,
                "conventional depiction is disabled by Spec",
            ));
        }
        let id = want_id(&t.feature.id, "thread");
        let source = if s.pmi == crate::model::PmiMode::Ignore {
            false
        } else {
            match &a.pmi {
                Some(crate::pmi::Reading::Read(r)) => r.records.iter().any(|r| {
                    r.kind.ends_with("thread")
                        && r.faces.iter().any(|i| t.feature.faces.contains(i))
                        && manufacturing.owned.iter().any(|o| {
                            o.source == r.source_id
                                && out.iter().any(|w| {
                                    w.measurements
                                        .iter()
                                        .any(|m| m.feature == o.id && m.parameter == "thread")
                                })
                        })
                }),
                _ => false,
            }
        };
        if source {
            problems.retain(|p| p.id != format!("p:thread_designation_measured:{}", t.feature.id));
            out.retain(|w| w.id != id);
            continue;
        }
        if let Some(text) = setting.and_then(|s| s.designation.as_ref()) {
            if let Some(w) = out.iter_mut().find(|w| w.id == id) {
                w.terms = vec![text.clone()];
                w.basis =
                    "ISO 6410; designation authored verbatim in Spec; thread extent is depicted"
                        .into();
            }
            problems.retain(|p| p.id != format!("p:thread_designation_measured:{}", t.feature.id));
        }
    }
}

/// Use the same circular-rim and generator attachment contract as other
/// cylindrical callouts. Four fixed mid-span points unnecessarily constrain
/// the shared router and can force leaders across neighbouring callouts.
pub(super) fn route_profile(a: &Analysis, w: &mut Want, view: &str) {
    if !matches!(w.shape, Shape::Callout { .. }) {
        return;
    }
    let Some(t) = w
        .measurements
        .iter()
        .filter(|m| m.parameter == "thread")
        .find_map(|m| {
            a.features
                .iter()
                .find(|f| f.id == m.feature && f.kind == "modelled_thread")
                .and_then(|f| Thread::read(f).ok())
        })
    else {
        return;
    };
    let Some((_, toward)) = TOWARD.iter().find(|(name, _)| *name == view) else {
        return;
    };
    let along = haecceity::geom::dot(t.axis, *toward);
    let sites = if along.abs() > 1. - 1e-6 {
        vec![(t.at((t.span[0] + t.span[1]) / 2.), t.crest())]
    } else {
        let radial = haecceity::geom::unit(haecceity::geom::cross(t.axis, *toward)).unwrap();
        [t.span[0], (t.span[0] + t.span[1]) / 2., t.span[1]]
            .into_iter()
            .flat_map(|z| {
                [-1., 1.].map(move |side| {
                    (
                        haecceity::geom::add(
                            t.at(z),
                            haecceity::geom::scale(radial, side * t.crest()),
                        ),
                        0.,
                    )
                })
            })
            .collect()
    };
    w.shape = Shape::Callout {
        sites,
        arcs: Vec::new(),
    };
}
