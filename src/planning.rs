//! Preparation shared by sheet-search candidates during one immutable planning call.

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::model::Analysis;

type Values = BTreeMap<(&'static str, Vec<u8>), Box<dyn Any>>;

#[derive(Clone)]
struct Active {
    analysis: usize,
    values: Rc<RefCell<Values>>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}

struct Scope(Option<Active>);

impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = self.0.take());
    }
}

pub(crate) fn with<T>(analysis: &Analysis, run: impl FnOnce() -> T) -> T {
    let identity = analysis as *const Analysis as usize;
    let previous = ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let previous = active.clone();
        if active.as_ref().is_none_or(|a| a.analysis != identity) {
            *active = Some(Active {
                analysis: identity,
                values: Rc::new(RefCell::new(BTreeMap::new())),
            });
        }
        previous
    });
    let _scope = Scope(previous);
    run()
}

/// The address is used only while `with` borrows that Analysis, never across calls.
/// Keys describe every additional input; neither preparation nor resolved ink survives the call.
pub(crate) fn memo<T: Clone + 'static>(
    analysis: &Analysis,
    name: &'static str,
    key: Vec<u8>,
    prepare: impl FnOnce() -> T,
) -> T {
    let values = ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .filter(|a| a.analysis == analysis as *const Analysis as usize)
            .map(|a| a.values.clone())
    });
    let Some(values) = values else {
        return prepare();
    };
    let key = (name, key);
    if let Some(value) = values.borrow().get(&key) {
        return value.downcast_ref::<T>().expect("preparation type").clone();
    }
    let value = prepare();
    values.borrow_mut().insert(key, Box::new(value.clone()));
    value
}

pub(crate) fn lookup<T: Clone + 'static>(
    analysis: &Analysis,
    name: &'static str,
    key: &[u8],
) -> Option<T> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let active = active
            .as_ref()
            .filter(|a| a.analysis == analysis as *const Analysis as usize)?;
        active
            .values
            .borrow()
            .get(&(name, key.to_vec()))
            .map(|v| v.downcast_ref::<T>().expect("preparation type").clone())
    })
}

pub(crate) fn store<T: 'static>(analysis: &Analysis, name: &'static str, key: Vec<u8>, value: T) {
    ACTIVE.with(|active| {
        if let Some(active) = active
            .borrow()
            .as_ref()
            .filter(|a| a.analysis == analysis as *const Analysis as usize)
        {
            active
                .values
                .borrow_mut()
                .insert((name, key), Box::new(value));
        }
    });
}

pub(crate) fn enabled(analysis: &Analysis) -> bool {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .is_some_and(|a| a.analysis == analysis as *const Analysis as usize)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis() -> Analysis {
        serde_json::from_value(serde_json::json!({
            "schema": crate::model::ANALYSIS_SCHEMA,
            "binding": {"source_sha256": "test", "part": "test", "face_count": 0, "loader": "test"},
            "envelope": [[0, 0, 0], [10, 10, 10]], "iso_bounds": [0, 0, 10, 10],
            "features": [], "levels": [], "level_supports": [], "risers": [], "principal_profiles": []
        })).unwrap()
    }

    #[test]
    fn nested_calls_share_preparation_and_a_later_analysis_edit_does_not() {
        let mut a = analysis();
        let preparations = std::cell::Cell::new(0);
        let read = |a: &Analysis| {
            memo(a, "test", Vec::new(), || {
                preparations.set(preparations.get() + 1);
                a.envelope
            })
        };
        with(&a, || {
            assert_eq!(read(&a), a.envelope);
            with(&a, || assert_eq!(read(&a), a.envelope));
            assert_eq!(preparations.get(), 1);
            let b = a.clone();
            with(&b, || assert_eq!(read(&b), b.envelope));
            assert_eq!(read(&a), a.envelope);
            assert_eq!(preparations.get(), 2);
        });
        assert!(!enabled(&a));
        a.envelope[1][0] = 20.0;
        with(&a, || assert_eq!(read(&a), a.envelope));
        assert_eq!(preparations.get(), 3);
    }

    #[test]
    fn unwinding_restores_the_outer_scope_and_cached_values_are_owned() {
        let a = analysis();
        let b = a.clone();
        with(&a, || {
            let mut first = memo(&a, "test", Vec::new(), || vec![1]);
            first.push(2);
            let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with(&b, || {
                    store(&b, "test", Vec::new(), vec![3]);
                    panic!("refused candidate");
                });
            }));
            assert!(refused.is_err());
            assert_eq!(lookup::<Vec<i32>>(&a, "test", &[]), Some(vec![1]));
        });
        assert!(!enabled(&a));
    }
}
