//! Preparation shared by sheet-search candidates during one immutable planning call.
//! Exact cross-call results use bounded memos with short lock scopes.
//! Sessions may retain bounded preparation for the same Analysis across calls; each memo key
//! still describes all additional inputs, including the Spec for planned candidates.

use sha2::{Digest, Sha256};
use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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

type Retained = VecDeque<([u8; 32], Rc<RefCell<Values>>)>;
thread_local! {
    // Four complete input sets, only for interactive sessions. Ordinary planning and its
    // timing calibration keep the existing per-call lifetime. No raw pointer survives a call.
    static RETAINED: RefCell<Retained> = const { RefCell::new(VecDeque::new()) };
}

/// Preview and commit often have identical inputs. Keep their pure preparation bounded and
/// bound to the entire Analysis, including recognition. Spec-dependent candidates already
/// key on the complete Spec; Analysis-only preparation can also survive a title edit. Every
/// invocation still runs the recovered planner. Oversized scopes are cleared on re-entry,
/// bounding retention by 256 entries plus the existing single-call planning budget.
pub(crate) fn with_retained<T>(analysis: &Analysis, run: impl FnOnce() -> T) -> T {
    let key: [u8; 32] =
        Sha256::digest(serde_json::to_vec(analysis).expect("analysis serializes")).into();
    let values = RETAINED.with(|retained| {
        let mut retained = retained.borrow_mut();
        let values = retained
            .iter()
            .position(|(k, _)| *k == key)
            .and_then(|i| retained.remove(i))
            .map(|(_, values)| values)
            .unwrap_or_else(|| Rc::new(RefCell::new(BTreeMap::new())));
        if values.borrow().len() > 256 {
            values.borrow_mut().clear();
        }
        retained.push_front((key, values.clone()));
        retained.truncate(4);
        values
    });
    let previous = ACTIVE.with(|active| {
        active.replace(Some(Active {
            analysis: analysis as *const Analysis as usize,
            values,
        }))
    });
    let _scope = Scope(previous);
    run()
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
/// Keys describe every additional input. Ordinary scopes end with the call; session scopes
/// retain only entries bound to the complete Analysis, with all other inputs in each key.
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

/// Placement policies held in the planning scope are inputs to exact result caches too.
pub(crate) fn policy_key(analysis: &Analysis) -> (bool, bool) {
    (
        lookup::<bool>(analysis, "canonical-circular-recipes", &[]).unwrap_or(false),
        lookup::<bool>(analysis, "established-route-policy", &[]).unwrap_or(false),
    )
}

type SharedValues<T> = VecDeque<([u8; 32], Arc<T>)>;

/// Bounded exact results across immutable calls. Only reference/queue updates hold
/// the lock: cloning and destroying a large Plan must not stall other planners.
pub(crate) struct BoundedMemo<T> {
    capacity: usize,
    values: Mutex<SharedValues<T>>,
}

impl<T: Clone> BoundedMemo<T> {
    pub(crate) const fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            values: Mutex::new(VecDeque::new()),
        }
    }

    pub(crate) fn get(&self, key: &[u8; 32]) -> Option<T> {
        let stored = self
            .values
            .lock()
            .expect("planning memo lock")
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| Arc::clone(value));
        stored.map(|value| value.as_ref().clone())
    }

    pub(crate) fn insert(&self, key: [u8; 32], value: T) {
        let value = Arc::new(value);
        let mut values = self.values.lock().expect("planning memo lock");
        let previous = values.iter().position(|(k, _)| *k == key);
        let replaced = previous.and_then(|at| values.remove(at));
        let evicted = if values.len() == self.capacity {
            values.pop_front()
        } else {
            None
        };
        values.push_back((key, value));
        drop(values);
        drop((replaced, evicted));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Spec;

    #[test]
    fn shared_memo_clones_and_drops_values_outside_its_lock() {
        use std::sync::{Arc, Weak};
        struct Probe {
            cache: Weak<BoundedMemo<Probe>>,
            value: u8,
        }
        impl Clone for Probe {
            fn clone(&self) -> Self {
                assert!(self.cache.upgrade().unwrap().values.try_lock().is_ok());
                Self {
                    cache: self.cache.clone(),
                    value: self.value,
                }
            }
        }
        impl Drop for Probe {
            fn drop(&mut self) {
                if let Some(cache) = self.cache.upgrade() {
                    assert!(cache.values.try_lock().is_ok());
                }
            }
        }
        let cache = Arc::new(BoundedMemo::new(2));
        let value = |n| Probe {
            cache: Arc::downgrade(&cache),
            value: n,
        };
        cache.insert([1; 32], value(1));
        cache.insert([2; 32], value(2));
        let mut owned = cache.get(&[1; 32]).unwrap();
        owned.value = 99;
        assert_eq!(cache.get(&[1; 32]).unwrap().value, 1);
        cache.insert([1; 32], value(3));
        cache.insert([3; 32], value(4));
        assert!(cache.get(&[2; 32]).is_none());
        assert_eq!(cache.get(&[1; 32]).unwrap().value, 3);
    }

    fn analysis() -> Analysis {
        serde_json::from_value(serde_json::json!({
            "schema": crate::model::ANALYSIS_SCHEMA,
            "binding": {"source_sha256": "test", "part": "test", "face_count": 0, "loader": "test"},
            "envelope": [[0, 0, 0], [10, 10, 10]], "iso_bounds": [0, 0, 10, 10],
            "features": [], "levels": [], "level_supports": [], "risers": [], "principal_profiles": []
        })).unwrap()
    }

    #[test]
    fn retained_preparation_binds_the_complete_analysis_and_spec() {
        let mut a = analysis();
        let mut s = Spec::default();
        let preparations = std::cell::Cell::new(0);
        let pure_preparations = std::cell::Cell::new(0);
        let read = |a: &Analysis, s: &Spec| {
            with_retained(a, || {
                memo(a, "retained-pure", Vec::new(), || {
                    pure_preparations.set(pure_preparations.get() + 1);
                    a.envelope
                });
                memo(a, "retained-test", serde_json::to_vec(s).unwrap(), || {
                    preparations.set(preparations.get() + 1);
                    (a.envelope, s.sheet.scale.clone())
                })
            })
        };
        assert_eq!(read(&a, &s), (a.envelope, None));
        assert_eq!(read(&a.clone(), &s), (a.envelope, None));
        assert_eq!(preparations.get(), 1);
        assert!(!enabled(&a));
        // The STEP binding stays the same: changed consumed evidence must still invalidate.
        a.envelope[1][0] = 20.0;
        assert_eq!(read(&a, &s), (a.envelope, None));
        assert_eq!(preparations.get(), 2);
        s.sheet.scale = Some("2:1".into());
        assert_eq!(read(&a, &s), (a.envelope, s.sheet.scale.clone()));
        assert_eq!(preparations.get(), 3);
        assert_eq!(read(&a, &s), (a.envelope, s.sheet.scale.clone()));
        assert_eq!(preparations.get(), 3);
        assert_eq!(
            pure_preparations.get(),
            2,
            "title/scale edits reuse only Analysis-dependent preparation"
        );
        assert!(!enabled(&a));
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
