use luna_isa::{FRegisterRType, encode_f_r};
use luna_machine::Machine;

fn run(mnemonic: &str, left: u64, right: u64) -> (u64, u64) {
    let mut machine = Machine::new(16);
    machine.f[1] = if mnemonic == "fadd.s" {
        0xffff_ffff_0000_0000 | left
    } else {
        left
    };
    machine.f[2] = if mnemonic == "fadd.s" {
        0xffff_ffff_0000_0000 | right
    } else {
        right
    };
    let word = encode_f_r(
        mnemonic,
        FRegisterRType {
            rd: 3,
            rs1: 1,
            rs2: 2,
            rm: 0,
        },
    )
    .expect("probe instruction must be generated from pinned R2");
    machine
        .load(0, &word.to_le_bytes())
        .expect("probe memory must fit");
    machine.step().expect("probe instruction must execute");
    let result = if mnemonic == "fadd.s" {
        machine.f[3] & 0xffff_ffff
    } else {
        machine.f[3]
    };
    (result, u64::from(machine.fflags()))
}

fn main() {
    let cases = [
        ("fadd.s", 0x3fc0_0000, 0x4010_0000),
        ("fadd.s", 0x7f7f_ffff, 0x7f7f_ffff),
        ("fadd.s", 0x0000_0001, 0x0000_0001),
        ("fadd.s", 0x7f80_0000, 0xff80_0000),
        ("fadd.d", 0x3ff8_0000_0000_0000, 0x4002_0000_0000_0000),
        ("fadd.d", 0x7fef_ffff_ffff_ffff, 0x7fef_ffff_ffff_ffff),
        ("fadd.d", 0x0000_0000_0000_0001, 0x0000_0000_0000_0001),
        ("fadd.d", 0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000),
    ];
    for (mnemonic, left, right) in cases {
        let (result, flags) = run(mnemonic, left, right);
        print!("{result:016x}{flags:016x}");
    }
    println!();
}
