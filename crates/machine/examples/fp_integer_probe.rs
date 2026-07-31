use luna_isa::{FloatConversion, FloatConversionKind, encode_f_convert};
use luna_machine::Machine;

fn run(kind: FloatConversionKind, value: u64, rounding_mode: u8) -> (u64, u64) {
    let mut machine = Machine::new(16);
    match kind {
        FloatConversionKind::WFromS | FloatConversionKind::WuFromS => {
            machine.f[1] = 0xffff_ffff_0000_0000 | (value & 0xffff_ffff);
        }
        FloatConversionKind::WFromD | FloatConversionKind::WuFromD => machine.f[1] = value,
        FloatConversionKind::LFromS | FloatConversionKind::LuFromS => {
            machine.f[1] = 0xffff_ffff_0000_0000 | (value & 0xffff_ffff)
        }
        FloatConversionKind::LFromD | FloatConversionKind::LuFromD => machine.f[1] = value,
        FloatConversionKind::SFromW
        | FloatConversionKind::SFromWu
        | FloatConversionKind::DFromW
        | FloatConversionKind::DFromWu
        | FloatConversionKind::SFromL
        | FloatConversionKind::SFromLu
        | FloatConversionKind::DFromL
        | FloatConversionKind::DFromLu => machine.x[1] = value,
        _ => panic!("integer conversion probe received a format conversion"),
    }
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
        FloatConversionKind::WFromS
        | FloatConversionKind::WuFromS
        | FloatConversionKind::WFromD
        | FloatConversionKind::WuFromD => machine.x[2] & 0xffff_ffff,
        FloatConversionKind::LFromS
        | FloatConversionKind::LuFromS
        | FloatConversionKind::LFromD
        | FloatConversionKind::LuFromD => machine.x[2],
        FloatConversionKind::SFromW
        | FloatConversionKind::SFromWu
        | FloatConversionKind::SFromL
        | FloatConversionKind::SFromLu => machine.f[2] & 0xffff_ffff,
        FloatConversionKind::DFromW
        | FloatConversionKind::DFromWu
        | FloatConversionKind::DFromL
        | FloatConversionKind::DFromLu => machine.f[2],
        _ => unreachable!(),
    };
    (result, u64::from(machine.fflags()))
}

fn main() {
    let cases = [
        (FloatConversionKind::WFromS, 0x3fe0_0000, 0),
        (FloatConversionKind::WFromS, 0xbfe0_0000, 2),
        (FloatConversionKind::WuFromS, 0x4060_0000, 0),
        (FloatConversionKind::WuFromS, 0xbf80_0000, 0),
        (FloatConversionKind::WFromD, 0x7ff0_0000_0000_0000, 0),
        (FloatConversionKind::SFromW, (-123i32 as i64) as u64, 0),
        (FloatConversionKind::SFromWu, 0xffff_ffff, 0),
        (FloatConversionKind::DFromW, 0x0000_0000_8000_0000, 0),
        (FloatConversionKind::LFromS, 0x3fe0_0000, 0),
        (FloatConversionKind::LuFromS, 0xbf80_0000, 0),
        (FloatConversionKind::LFromD, 0x7ff0_0000_0000_0000, 0),
        (FloatConversionKind::DFromL, i64::MIN as u64, 0),
        (FloatConversionKind::DFromLu, u64::MAX, 0),
    ];
    for (kind, value, rounding_mode) in cases {
        let (result, flags) = run(kind, value, rounding_mode);
        print!("{result:016x}{flags:016x}");
    }
    println!();
}
