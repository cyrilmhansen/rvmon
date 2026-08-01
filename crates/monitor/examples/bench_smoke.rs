use std::time::Instant;

use luna_monitor::Monitor;

fn main() {
    const SAMPLES: usize = 10;
    const ITERATIONS_PER_SAMPLE: u64 = 100;
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut snapshots = 0usize;
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            let mut monitor = Monitor::new(64 * 1024);
            monitor
                .execute("assemble addi x1,x0,1")
                .expect("benchmark source must assemble");
            monitor
                .execute("step")
                .expect("benchmark step must execute");
            snapshots = monitor
                .snapshot_bytes()
                .expect("benchmark snapshot must encode")
                .len();
        }
        samples.push(start.elapsed().as_nanos() / u128::from(ITERATIONS_PER_SAMPLE));
    }
    samples.sort_unstable();
    let p50 = samples[SAMPLES / 2];
    let p95 = samples[(SAMPLES * 95).div_ceil(100).saturating_sub(1)];
    let total_iterations = SAMPLES as u64 * ITERATIONS_PER_SAMPLE;
    println!(
        "bench-smoke os={} arch={} samples={SAMPLES} iterations={total_iterations} p50_ns={} p95_ns={} snapshot_bytes={snapshots}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        p50,
        p95,
    );
}
