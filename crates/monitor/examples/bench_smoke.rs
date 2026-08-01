use std::time::Instant;

use luna_monitor::Monitor;

fn main() {
    const ITERATIONS: u64 = 1_000;
    let start = Instant::now();
    let mut snapshots = 0usize;
    for _ in 0..ITERATIONS {
        let mut monitor = Monitor::new(64 * 1024);
        monitor
            .execute("assemble addi x1,x0,1")
            .expect("benchmark source must assemble");
        monitor.execute("step").expect("benchmark step must execute");
        snapshots = monitor
            .snapshot_bytes()
            .expect("benchmark snapshot must encode")
            .len();
    }
    let elapsed = start.elapsed();
    let nanos = elapsed.as_nanos();
    println!(
        "bench-smoke iterations={ITERATIONS} elapsed_ms={} ns_per_iteration={} snapshot_bytes={snapshots}",
        elapsed.as_millis(),
        nanos / u128::from(ITERATIONS)
    );
}
