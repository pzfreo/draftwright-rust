//! Native S4 edit latency, including delta construction and sheet assembly.
#[path = "../tests/common/session_edits.rs"]
mod session_edits;
use draftwright::Session;
use draftwright::api::Features;
use draftwright::model::Spec;
use std::path::Path;
use std::time::Instant;

fn main() {
    println!("part,open_ms,samples,p50_ms,p90_ms,max_ms,max_delta_bytes,plans_per_edit");
    for file in std::env::args().skip(1) {
        let start = Instant::now();
        let mut session =
            Session::open(Path::new(&file), Features::Native, Spec::default()).unwrap();
        let open = start.elapsed().as_secs_f64() * 1000.0;
        let ops = session_edits::edits(&session);
        let mut samples = Vec::new();
        let mut bytes = 0;
        // Warm the geometry once, then repeat the complete S0 sequence ten times.
        for _ in 0..10 {
            for edit in &ops {
                let plans = session.stats().plans;
                let start = Instant::now();
                let delta = session.apply(edit, true).unwrap();
                // Include JSON transfer encoding in the measurement.
                let encoded = serde_json::to_vec(&delta).unwrap();
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
                bytes = bytes.max(encoded.len());
                assert_eq!(session.stats().plans, plans + 1);
            }
            for _ in &ops {
                session.undo().unwrap();
            }
        }
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        println!(
            "{},{open:.3},{n},{:.3},{:.3},{:.3},{bytes},1",
            Path::new(&file).file_stem().unwrap().to_string_lossy(),
            samples[n / 2],
            samples[(n * 9).div_ceil(10) - 1],
            samples[n - 1]
        );
        assert!(
            samples[(n * 9).div_ceil(10) - 1] <= 150.0,
            "{file}: native edit p90 exceeds 150 ms"
        );
    }
}
