use luna_isa::{FloatMove, FloatMoveKind, encode_f_move};
use luna_machine::Machine;

fn run(kind: FloatMoveKind, value: u64) -> u64 {
    let mut machine = Machine::new(16);
    match kind {
        FloatMoveKind::XFromW | FloatMoveKind::XFromD => machine.f[1] = value,
        FloatMoveKind::WFromX | FloatMoveKind::DFromX => machine.x[1] = value,
    }
    let word = encode_f_move(FloatMove {
        kind,
        rd: 2,
        rs1: 1,
    })
    .expect("move instruction must be generated from pinned R2");
    machine
        .load(0, &word.to_le_bytes())
        .expect("probe memory must fit");
    machine.step().expect("move must execute");
    match kind {
        FloatMoveKind::XFromW | FloatMoveKind::XFromD => machine.x[2],
        FloatMoveKind::WFromX | FloatMoveKind::DFromX => machine.f[2],
    }
}

fn main() {
    let cases = [
        (FloatMoveKind::XFromW, 0xffff_ffff_8000_0001),
        (FloatMoveKind::WFromX, 0x1234_5678_8765_4321),
        (FloatMoveKind::XFromD, 0x7ff8_0000_0000_0042),
        (FloatMoveKind::DFromX, 0x8000_0000_0000_0000),
    ];
    for (kind, value) in cases {
        print!("{:016x}", run(kind, value));
    }
    println!();
}
