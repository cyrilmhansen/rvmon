#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatFormat {
    Binary32,
    Binary64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatClass {
    Zero,
    Subnormal,
    Normal,
    Infinite,
    QuietNaN,
    SignalingNaN,
    InvalidBox,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatDisplay {
    pub format: FloatFormat,
    pub bits: u128,
    pub exact_hex: String,
    pub shortest_decimal: String,
    pub class: FloatClass,
}

pub fn binary32(bits: u32) -> FloatDisplay {
    let exponent = (bits >> 23) & 0xff;
    let fraction = bits & 0x007f_ffff;
    let class = classify(u64::from(exponent), u64::from(fraction), 0x0040_0000);
    let shortest_decimal = match class {
        FloatClass::Zero => signed_zero(bits & 0x8000_0000 != 0),
        FloatClass::Infinite => signed_infinity(bits & 0x8000_0000 != 0),
        FloatClass::QuietNaN => "qNaN".into(),
        FloatClass::SignalingNaN => "sNaN".into(),
        _ => f32::from_bits(bits).to_string(),
    };
    FloatDisplay {
        format: FloatFormat::Binary32,
        bits: u128::from(bits),
        exact_hex: format!("0x{bits:08x}"),
        shortest_decimal,
        class,
    }
}

pub fn binary64(bits: u64) -> FloatDisplay {
    let exponent = (bits >> 52) & 0x7ff;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let class = classify(exponent, fraction, 0x0008_0000);
    let shortest_decimal = match class {
        FloatClass::Zero => signed_zero(bits & 0x8000_0000_0000_0000 != 0),
        FloatClass::Infinite => signed_infinity(bits & 0x8000_0000_0000_0000 != 0),
        FloatClass::QuietNaN => "qNaN".into(),
        FloatClass::SignalingNaN => "sNaN".into(),
        _ => f64::from_bits(bits).to_string(),
    };
    FloatDisplay {
        format: FloatFormat::Binary64,
        bits: u128::from(bits),
        exact_hex: format!("0x{bits:016x}"),
        shortest_decimal,
        class,
    }
}

pub fn boxed_binary32(raw: u64) -> FloatDisplay {
    if raw >> 32 != 0xffff_ffff {
        return FloatDisplay {
            format: FloatFormat::Binary32,
            bits: u128::from(raw),
            exact_hex: format!("0x{raw:016x}"),
            shortest_decimal: "qNaN (invalid NaN-box)".into(),
            class: FloatClass::InvalidBox,
        };
    }
    binary32(raw as u32)
}

fn classify(exponent: u64, fraction: u64, quiet_bit: u64) -> FloatClass {
    match (exponent, fraction) {
        (0, 0) => FloatClass::Zero,
        (0, _) => FloatClass::Subnormal,
        (0xff | 0x7ff, 0) => FloatClass::Infinite,
        (exponent, fraction)
            if (exponent == 0xff || exponent == 0x7ff) && fraction & quiet_bit != 0 =>
        {
            FloatClass::QuietNaN
        }
        (0xff | 0x7ff, _) => FloatClass::SignalingNaN,
        _ => FloatClass::Normal,
    }
}

fn signed_zero(negative: bool) -> String {
    if negative { "-0" } else { "0" }.into()
}

fn signed_infinity(negative: bool) -> String {
    if negative { "-inf" } else { "inf" }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_exact_binary32_bits_and_roundtrip_decimal() {
        let value = binary32(0x3fc0_0000);
        assert_eq!(value.exact_hex, "0x3fc00000");
        assert_eq!(value.shortest_decimal, "1.5");
        assert_eq!(value.class, FloatClass::Normal);
        assert_eq!(
            value.shortest_decimal.parse::<f32>().unwrap().to_bits(),
            0x3fc0_0000
        );
    }

    #[test]
    fn distinguishes_signed_zero_nan_and_subnormal() {
        assert_eq!(binary32(0x8000_0000).shortest_decimal, "-0");
        assert_eq!(binary32(0x0000_0001).class, FloatClass::Subnormal);
        assert_eq!(binary32(0x7fc0_0042).class, FloatClass::QuietNaN);
        assert_eq!(binary32(0x7f80_0042).class, FloatClass::SignalingNaN);
    }

    #[test]
    fn displays_binary64_and_invalid_box_exactly() {
        let value = binary64(0x3ff0_0000_0000_0000);
        assert_eq!(value.exact_hex, "0x3ff0000000000000");
        assert_eq!(value.shortest_decimal, "1");
        let invalid = boxed_binary32(0x0000_0000_3f80_0000);
        assert_eq!(invalid.class, FloatClass::InvalidBox);
        assert_eq!(invalid.exact_hex, "0x000000003f800000");
    }
}
