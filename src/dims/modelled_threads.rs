//! Measured physical threads get one designation and their measured axial span.
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
        let Some(k) = crate::bridge::holes::principal(t.axis) else {
            problems.push(not_depicted(
                f,
                "no principal view can carry its designation; use an authored view",
            ));
            continue;
        };
        let (text, standard) = t.designation();
        if !standard {
            problems.push(unmatched(f));
        }
        let at = t.at((t.span[0] + t.span[1]) / 2.);
        let sites = (0..3)
            .filter(|&j| j != k)
            .flat_map(|j| {
                [-1., 1.].map(move |side| {
                    let mut p = at;
                    p[j] += side * t.crest();
                    (p, 0.)
                })
            })
            .collect();
        let mut views = super::envelope::profile_views(k);
        if t.internal {
            views.insert(0, end_on(k));
        }
        out.push(Want {
            id:want_id(&f.id,"thread"), views,value:t.major,
            terms:vec![format!("{text} × {} LONG",fmt(t.span[1]-t.span[0]))],
            measurements:vec![Measurement {feature:f.id.clone(),parameter:"thread".into(),value:t.major},Measurement {feature:f.id.clone(),parameter:"thread.length".into(),value:t.span[1]-t.span[0]}],
            basis:"ISO 6410; designation inferred from a unique ISO 261 diameter/pitch match; physical trimmed thread span (including partial turns), not guaranteed full engagement; features/<id>/modelled_thread overrides depiction and designation".into(),
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
                w.terms = vec![format!("{text} × {} LONG", fmt(t.span[1] - t.span[0]))];
                w.basis="ISO 6410; designation authored in Spec; measured physical trimmed thread span including partial turns".into();
            }
            problems.retain(|p| p.id != format!("p:thread_designation_measured:{}", t.feature.id));
        }
    }
}
