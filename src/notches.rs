//! Planner-side memoization of source-only notch evidence. Lint never reads this cache.
pub(crate) fn requirements(a: &crate::model::Analysis) -> Vec<crate::lint::notch_evidence::Notch> {
    crate::planning::memo(a, "v_notches", Vec::new(), || {
        crate::lint::notch_evidence::requirements(a)
    })
}
