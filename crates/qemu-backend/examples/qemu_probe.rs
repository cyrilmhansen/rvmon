use std::env;

use luna_qemu_backend::GdbRemote;
use luna_target_api::{ExecutionOutcome, TargetBackend};

fn main() {
    let port = env::args()
        .nth(1)
        .expect("usage: qemu_probe <tcp-port>")
        .parse::<u16>()
        .expect("TCP port must be an integer");
    let mut backend = GdbRemote::connect(("127.0.0.1", port)).expect("connect to QEMU GDB RSP");
    let initial_pc = backend.context().pc;

    let mut image_prefix = [0u8; 4];
    backend
        .read_memory(0x8000_0000, &mut image_prefix)
        .expect("read QEMU RAM image");
    assert_ne!(
        image_prefix, [0; 4],
        "guest image was not visible in QEMU RAM"
    );
    println!(
        "qemu-connect: pc=0x{initial_pc:016x} image={:02x}{:02x}{:02x}{:02x}",
        image_prefix[0], image_prefix[1], image_prefix[2], image_prefix[3]
    );

    let outcome = backend.step().expect("single-step QEMU target");
    assert!(matches!(outcome, ExecutionOutcome::Stopped(_)));
    println!("qemu-step: {:?}", outcome);
}
