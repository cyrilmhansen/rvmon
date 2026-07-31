use luna_isa::{FloatConversion, FloatConversionKind, encode_f_convert};
use luna_machine::Machine;

fn run(kind: FloatConversionKind, value: u64, rounding_mode: u8) -> (u64, u64) {
    let mut machine = Machine::new(16);
    machine.f[1] = match kind {
        FloatConversionKind::SFromD => value,
        FloatConversionKind::DFromS => 0xffff_ffff_0000_0000 | (value & 0xffff_ffff),
    };
    let word = encode_f_convert(FloatConversion {
        kind,
        rd: 2,
        rs1: 1,
        rm: rounding_mode,
    })
    .expect("conversion instruction must be generated from pinned R2");
    machine
        .load(0, &word.to_le_bytes())
        .expect("probe memory must fit");
    machine.step().expect("conversion must execute");
    let result = match kind {
        FloatConversionKind::SFromD => machine.f[2] & 0xffff_ffff,
        FloatConversionKind::DFromS => machine.f[2],
    };
    (result, u64::from(machine.fflags()))
}

fn main() {
    let cases = [
        (FloatConversionKind::SFromD, 0x3ff0_0000_1000_0000, 0),
        (FloatConversionKind::SFromD, 0x3ff0_0000_1000_0000, 3),
        (FloatConversionKind::DFromS, 0x3fc0_0000, 0),
    ];
    for (kind, value, rounding_mode) in cases {
        let (result, flags) = run(kind, value, rounding_mode);
        print!("{result:016x}{flags:016x}");
    }
    println!();
}
