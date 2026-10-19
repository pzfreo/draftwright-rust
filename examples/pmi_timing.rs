//! Plan-only timings from identical saved analyses, excluding recognition and serialization.
use draftwright::model::{self, Analysis, Spec};
use std::{path::Path, time::Instant};
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanos: i64,
}
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn getpid() -> i32;
    fn proc_pid_rusage(pid: i32, flavor: i32, usage: *mut std::ffi::c_void) -> i32;
}
fn cpu() -> f64 {
    let mut t = Timespec {
        seconds: 0,
        nanos: 0,
    };
    let clock = if cfg!(target_os = "macos") { 12 } else { 2 };
    // SAFETY: t is a valid, uniquely borrowed native timespec for this synchronous call.
    assert_eq!(unsafe { clock_gettime(clock, &mut t) }, 0);
    t.seconds as f64 + t.nanos as f64 * 1e-9
}
fn instructions() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let mut usage = [0u64; 37];
        // SAFETY: the v4 buffer is large enough for proc_pid_rusage and names this process.
        if unsafe { proc_pid_rusage(getpid(), 4, usage.as_mut_ptr().cast()) } == 0 {
            return Some(usage[31]);
        }
    }
    None
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let dir = Path::new(&args[1]);
    let mut report = Vec::new();
    for name in [
        "cgb243",
        "threaded_connector_109",
        "ftc06",
        "ftc09",
        "ftc10",
    ] {
        let raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(format!("{name}.json"))).unwrap())
                .unwrap();
        let a: Analysis = serde_json::from_value(raw["analysis"].clone()).unwrap();
        let mut runs = Vec::new();
        for _ in 0..5 {
            let begin = instructions();
            let c = cpu();
            let w = Instant::now();
            let p = std::hint::black_box(model::plan_recovered(&a, &Spec::default()).unwrap());
            let wall = w.elapsed().as_secs_f64();
            let cpu = cpu() - c;
            let count = begin.zip(instructions()).map(|(a, b)| b - a);
            std::hint::black_box(p);
            runs.push((cpu, wall, count));
        }
        runs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (cpu, wall, count) = runs[2];
        eprintln!("{name}: {cpu:.4}s CPU, {wall:.4}s wall, {count:?} instructions");
        report.push(
            serde_json::json!({"part":name,"cpu":cpu,"wall":wall,"instructions":count,"runs":runs}),
        );
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
