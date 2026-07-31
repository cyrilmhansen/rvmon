#![forbid(unsafe_code)]

use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatFormat {
    Binary16,
    Binary32,
    Binary64,
    Binary128,
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

pub fn binary16(bits: u16) -> FloatDisplay {
    let exponent = (bits >> 10) & 0x1f;
    let fraction = bits & 0x03ff;
    let class = classify(u64::from(exponent), u128::from(fraction), 0x1f, 0x0200);
    let shortest_decimal = match class {
        FloatClass::Zero => signed_zero(bits & 0x8000 != 0),
        FloatClass::Infinite => signed_infinity(bits & 0x8000 != 0),
        FloatClass::QuietNaN => "qNaN".into(),
        FloatClass::SignalingNaN => "sNaN".into(),
        _ => exact_binary_decimal(
            bits & 0x8000 != 0,
            if exponent == 0 {
                u128::from(fraction)
            } else {
                u128::from(0x0400 | fraction)
            },
            if exponent == 0 {
                -24
            } else {
                i32::from(exponent) - 15 - 10
            },
        ),
    };
    FloatDisplay {
        format: FloatFormat::Binary16,
        bits: u128::from(bits),
        exact_hex: format!("0x{bits:04x}"),
        shortest_decimal,
        class,
    }
}

pub fn binary32(bits: u32) -> FloatDisplay {
    let exponent = (bits >> 23) & 0xff;
    let fraction = bits & 0x007f_ffff;
    let class = classify(u64::from(exponent), u128::from(fraction), 0xff, 0x0040_0000);
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
    let class = classify(exponent, u128::from(fraction), 0x7ff, 0x0008_0000);
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

pub fn binary128(bits: u128) -> FloatDisplay {
    let exponent = ((bits >> 112) & 0x7fff) as u64;
    let fraction = bits & ((1u128 << 112) - 1);
    let class = classify(exponent, fraction, 0x7fff, 1u128 << 111);
    let shortest_decimal = match class {
        FloatClass::Zero => signed_zero(bits >> 127 != 0),
        FloatClass::Infinite => signed_infinity(bits >> 127 != 0),
        FloatClass::QuietNaN => "qNaN".into(),
        FloatClass::SignalingNaN => "sNaN".into(),
        _ => {
            let significand = if exponent == 0 {
                fraction
            } else {
                (1u128 << 112) | fraction
            };
            let binary_exponent = if exponent == 0 {
                1 - 16383 - 112
            } else {
                exponent as i32 - 16383 - 112
            };
            exact_binary_decimal(bits >> 127 != 0, significand, binary_exponent)
        }
    };
    FloatDisplay {
        format: FloatFormat::Binary128,
        bits,
        exact_hex: format!("0x{bits:032x}"),
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

pub fn parse_float_literal(format: FloatFormat, literal: &str) -> Option<u128> {
    let literal = literal.replace(['_', '\''], "");
    let lower = literal.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        return u128::from_str_radix(hex, 16)
            .ok()
            .filter(|bits| *bits <= format_mask(format));
    }
    match (format, lower.as_str()) {
        (FloatFormat::Binary16, "inf" | "+inf") => Some(0x7c00),
        (FloatFormat::Binary16, "-inf") => Some(0xfc00),
        (FloatFormat::Binary16, "nan" | "qnan") => Some(0x7e00),
        (FloatFormat::Binary16, "snan") => Some(0x7c01),
        (FloatFormat::Binary32, "inf" | "+inf") => Some(0x7f80_0000),
        (FloatFormat::Binary32, "-inf") => Some(0xff80_0000),
        (FloatFormat::Binary32, "nan" | "qnan") => Some(0x7fc0_0000),
        (FloatFormat::Binary32, "snan") => Some(0x7f80_0001),
        (FloatFormat::Binary64, "inf" | "+inf") => Some(0x7ff0_0000_0000_0000),
        (FloatFormat::Binary64, "-inf") => Some(0xfff0_0000_0000_0000),
        (FloatFormat::Binary64, "nan" | "qnan") => Some(0x7ff8_0000_0000_0000),
        (FloatFormat::Binary64, "snan") => Some(0x7ff0_0000_0000_0001),
        (FloatFormat::Binary128, "inf" | "+inf") => Some(0x7fff_0000_0000_0000_0000_0000_0000_0000),
        (FloatFormat::Binary128, "-inf") => Some(0xffff_0000_0000_0000_0000_0000_0000_0000),
        (FloatFormat::Binary128, "nan" | "qnan") => Some(0x7fff_8000_0000_0000_0000_0000_0000_0000),
        (FloatFormat::Binary128, "snan") => Some(0x7fff_0000_0000_0000_0000_0000_0001),
        (FloatFormat::Binary16, _) => lower
            .parse::<f64>()
            .ok()
            .map(|value| u128::from(f64_to_binary16(value))),
        (FloatFormat::Binary32, _) => lower
            .parse::<f32>()
            .ok()
            .map(|value| u128::from(value.to_bits())),
        (FloatFormat::Binary64, _) => lower
            .parse::<f64>()
            .ok()
            .map(|value| u128::from(value.to_bits())),
        (FloatFormat::Binary128, _) => None,
    }
}

fn format_mask(format: FloatFormat) -> u128 {
    match format {
        FloatFormat::Binary16 => 0xffff,
        FloatFormat::Binary32 => u128::from(u32::MAX),
        FloatFormat::Binary64 => u128::from(u64::MAX),
        FloatFormat::Binary128 => u128::MAX,
    }
}

fn f64_to_binary16(value: f64) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 48) & 0x8000) as u16;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    if exponent == 0x7ff {
        return sign
            | if fraction == 0 {
                0x7c00
            } else if fraction & (1 << 51) != 0 {
                0x7e00
            } else {
                0x7c01
            };
    }
    if exponent == 0 {
        return sign;
    }
    let unbiased = exponent - 1023;
    let significand = (1u64 << 52) | fraction;
    if unbiased > 15 {
        return sign | 0x7c00;
    }
    if unbiased >= -14 {
        let mut mantissa = round_shift(significand, 42);
        let mut half_exponent = unbiased + 15;
        if mantissa == 0x800 {
            mantissa = 0x400;
            half_exponent += 1;
        }
        if half_exponent >= 31 {
            return sign | 0x7c00;
        }
        return sign | ((half_exponent as u16) << 10) | (mantissa as u16 & 0x03ff);
    }
    let shift = 28 - unbiased;
    if shift > 63 {
        return sign;
    }
    let mantissa = round_shift(significand, shift as u32);
    if mantissa >= 0x400 {
        sign | 0x0400
    } else {
        sign | mantissa as u16
    }
}

fn round_shift(value: u64, shift: u32) -> u64 {
    if shift == 0 {
        return value;
    }
    let truncated = value >> shift;
    let remainder = value & ((1u64 << shift) - 1);
    let halfway = 1u64 << (shift - 1);
    if remainder > halfway || (remainder == halfway && truncated & 1 != 0) {
        truncated + 1
    } else {
        truncated
    }
}

fn classify(exponent: u64, fraction: u128, max_exponent: u64, quiet_bit: u128) -> FloatClass {
    match (exponent, fraction) {
        (0, 0) => FloatClass::Zero,
        (0, _) => FloatClass::Subnormal,
        (max, 0) if max == max_exponent => FloatClass::Infinite,
        (exponent, fraction) if exponent == max_exponent && fraction & quiet_bit != 0 => {
            FloatClass::QuietNaN
        }
        (max, _) if max == max_exponent => FloatClass::SignalingNaN,
        _ => FloatClass::Normal,
    }
}

fn exact_binary_decimal(negative: bool, significand: u128, exponent: i32) -> String {
    if significand == 0 {
        return signed_zero(negative);
    }
    let mut integer = DecimalInteger::from_u128(significand);
    if exponent >= 0 {
        for _ in 0..exponent {
            integer.mul_small(2);
        }
        let mut value = integer.to_string();
        if negative {
            value.insert(0, '-');
        }
        return value;
    }

    let scale = usize::try_from(-exponent).unwrap();
    for _ in 0..scale {
        integer.mul_small(5);
    }
    let digits = integer.to_string();
    let mut value = if digits.len() <= scale {
        format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
    } else {
        let split = digits.len() - scale;
        format!("{}.{}", &digits[..split], &digits[split..])
    };
    while value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    if negative {
        value.insert(0, '-');
    }
    value
}

struct DecimalInteger {
    limbs: Vec<u32>,
}

impl DecimalInteger {
    fn from_u128(mut value: u128) -> Self {
        let mut limbs = Vec::new();
        while value != 0 {
            limbs.push((value % 1_000_000_000) as u32);
            value /= 1_000_000_000;
        }
        Self { limbs }
    }

    fn mul_small(&mut self, multiplier: u32) {
        let mut carry = 0u64;
        for limb in &mut self.limbs {
            let product = u64::from(*limb) * u64::from(multiplier) + carry;
            *limb = (product % 1_000_000_000) as u32;
            carry = product / 1_000_000_000;
        }
        while carry != 0 {
            self.limbs.push((carry % 1_000_000_000) as u32);
            carry /= 1_000_000_000;
        }
    }

    fn to_string(&self) -> String {
        let mut output = self
            .limbs
            .last()
            .map(u32::to_string)
            .unwrap_or_else(|| "0".into());
        for limb in self.limbs.iter().rev().skip(1) {
            write!(&mut output, "{limb:09}").unwrap();
        }
        output
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

    #[test]
    fn displays_binary16_classes_and_exact_decimal() {
        let value = binary16(0x3e00);
        assert_eq!(value.format, FloatFormat::Binary16);
        assert_eq!(value.exact_hex, "0x3e00");
        assert_eq!(value.shortest_decimal, "1.5");
        assert_eq!(binary16(0x0001).class, FloatClass::Subnormal);
        assert_eq!(binary16(0x7c01).class, FloatClass::SignalingNaN);
        assert_eq!(binary16(0x7e42).class, FloatClass::QuietNaN);
    }

    #[test]
    fn displays_binary128_without_host_conversion() {
        let value = binary128(0x3fff_8000_0000_0000_0000_0000_0000_0000);
        assert_eq!(value.format, FloatFormat::Binary128);
        assert_eq!(value.exact_hex, "0x3fff8000000000000000000000000000");
        assert_eq!(value.shortest_decimal, "1.5");
        assert_eq!(
            binary128(0x7fff_8000_0000_0000_0000_0000_0000_0042).class,
            FloatClass::QuietNaN
        );
    }
}
