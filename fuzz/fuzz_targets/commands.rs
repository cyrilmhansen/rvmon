#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let mut monitor = luna_monitor::Monitor::new(4096);
    let _ = monitor.execute(&input);
});
