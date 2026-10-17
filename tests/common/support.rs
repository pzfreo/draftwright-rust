//! Scheduling helpers for integration tests. Keep aggregate assertions in the caller.
#![allow(dead_code, unused_macros)]

/// Map independent jobs in input order, with at most four workers and never more workers
/// than available CPUs. The cap also bounds nested work when nextest runs several cases.
pub fn parallel<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(4)
        .min(items.len());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut out = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(item) = items.get(i) else {
                            return done;
                        };
                        done.push((i, f(item)));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, value)| value).collect()
}

/// Named cases let nextest schedule and hash-shard each part separately.
macro_rules! part_cases {
    ($check:ident; $($name:ident => $file:literal),+ $(,)?) => {
        mod $check {
            $(#[test] fn $name() { super::$check($file); })+
        }
    };
}

/// Single inventory for both the generated tests and the fixture completeness assertion.
macro_rules! pmi_files {
    ($emit:ident $(, $check:ident)?) => { $emit!($($check;)?
        assembly_plate_pin => "assembly_plate_pin.step",
        bolt_thread_knurl => "bolt_thread_knurl.step",
        bracket_positions => "bracket_positions.step",
        hw_assembly_plate_pin_intent => "hw_assembly_plate_pin_intent.step",
        hw_every_kind => "hw_every_kind.step",
        hw_spool_fits_intent => "hw_spool_fits_intent.step",
        hw_string_post_tapped_intent => "hw_string_post_tapped_intent.step",
        hw_thread_removed => "hw_thread_removed.step",
        hw_thread_replaced => "hw_thread_replaced.step",
        hw_thumbwheel_notes_add => "hw_thumbwheel_notes_add.step",
        hw_thumbwheel_thread_knurl_intent => "hw_thumbwheel_thread_knurl_intent.step",
        nist_ctc_01_asme1_ap242_e1 => "nist_ctc_01_asme1_ap242-e1.stp",
        nist_ctc_01_merge => "nist_ctc_01_merge.step",
        nist_ctc_02_asme1_ap242_e2 => "nist_ctc_02_asme1_ap242-e2.stp",
        nist_ftc_07_asme1_ap242_e2 => "nist_ftc_07_asme1_ap242-e2.stp",
        nist_ftc_10_asme1_ap242_e2 => "nist_ftc_10_asme1_ap242-e2.stp",
        nist_stc_06_asme1_ap242_e3 => "nist_stc_06_asme1_ap242-e3.stp",
        nist_stc_09_asme1_ap242_e4 => "nist_stc_09_asme1_ap242-e4.stp",
        nist_stc_10_asme1_ap242_e2 => "nist_stc_10_asme1_ap242-e2.stp",
        spool_fits => "spool_fits.step",
        string_post_tapped => "string_post_tapped.step",
        thumbwheel_thread_knurl => "thumbwheel_thread_knurl.step",
    ); };
}
macro_rules! pmi_inventory {
    ($($name:ident => $file:literal),+ $(,)?) => { pub const PMI_FILES: &[&str] = &[$($file),+]; };
}
pmi_files!(pmi_inventory);
macro_rules! pmi_cases {
    ($check:ident) => {
        pmi_files!(part_cases, $check);
    };
}
