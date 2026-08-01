use luna_diag::{Diagnostic, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegisterEdit {
    Integer { index: usize, value: u64 },
    Float { index: usize, bits: u64 },
}

pub(crate) fn parse_integer_edit(argument: &str) -> Result<RegisterEdit> {
    let parts: Vec<_> = argument.split_whitespace().collect();
    let [name, value] = parts.as_slice() else {
        return Err(Diagnostic::error(
            "MON-REG-001",
            "set expects <x-register> <u64-value>",
        ));
    };
    let index = parse_integer_name(name).ok_or_else(|| {
        Diagnostic::error(
            "MON-REG-002",
            "set register must be x0..x31 or an ABI alias",
        )
    })?;
    let value = parse_u64(value).map_err(|message| Diagnostic::error("MON-REG-003", message))?;
    Ok(RegisterEdit::Integer { index, value })
}

pub(crate) fn parse_float_edit(argument: &str) -> Result<RegisterEdit> {
    let parts: Vec<_> = argument.split_whitespace().collect();
    let [name, value] = parts.as_slice() else {
        return Err(Diagnostic::error(
            "MON-REG-004",
            "setf expects <f-register> <u64-bit-pattern>",
        ));
    };
    let index = parse_float_name(name)
        .ok_or_else(|| Diagnostic::error("MON-REG-005", "setf register must be f0..f31"))?;
    let bits = parse_u64(value).map_err(|message| Diagnostic::error("MON-REG-006", message))?;
    Ok(RegisterEdit::Float { index, bits })
}

pub(crate) fn parse_integer_name(name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    if let Some(index) = name.strip_prefix('x').and_then(|value| value.parse().ok()) {
        return (index < 32).then_some(index);
    }
    match name.as_str() {
        "zero" => Some(0),
        "ra" => Some(1),
        "sp" => Some(2),
        "gp" => Some(3),
        "tp" => Some(4),
        "t0" => Some(5),
        "t1" => Some(6),
        "t2" => Some(7),
        "s0" | "fp" => Some(8),
        "s1" => Some(9),
        "a0" => Some(10),
        "a1" => Some(11),
        "a2" => Some(12),
        "a3" => Some(13),
        "a4" => Some(14),
        "a5" => Some(15),
        "a6" => Some(16),
        "a7" => Some(17),
        "s2" => Some(18),
        "s3" => Some(19),
        "s4" => Some(20),
        "s5" => Some(21),
        "s6" => Some(22),
        "s7" => Some(23),
        "s8" => Some(24),
        "s9" => Some(25),
        "s10" => Some(26),
        "s11" => Some(27),
        "t3" => Some(28),
        "t4" => Some(29),
        "t5" => Some(30),
        "t6" => Some(31),
        _ => None,
    }
}

pub(crate) fn parse_float_name(name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    name.strip_prefix('f')
        .and_then(|value| value.parse().ok())
        .filter(|index| *index < 32)
}

pub(crate) fn format_raw(value: u64) -> String {
    format!("0x{value:016x}")
}

fn parse_u64(value: &str) -> std::result::Result<u64, &'static str> {
    let value = value.replace('_', "");
    if value.is_empty() {
        return Err("register value is empty");
    }
    if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| "register value does not fit in 64 bits")
    } else {
        value
            .parse::<u64>()
            .map_err(|_| "register value must be decimal or 0x hexadecimal")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_aliases_and_exact_float_bits() {
        assert_eq!(
            parse_integer_edit("a0 0xffff_ffff").unwrap(),
            RegisterEdit::Integer {
                index: 10,
                value: 0xffff_ffff
            }
        );
        assert_eq!(
            parse_float_edit("f3 0xffff_ffff_0000_0001").unwrap(),
            RegisterEdit::Float {
                index: 3,
                bits: 0xffff_ffff_0000_0001
            }
        );
    }

    #[test]
    fn rejects_wrong_classes_and_overflow() {
        assert_eq!(parse_integer_edit("f1 1").unwrap_err().code, "MON-REG-002");
        assert_eq!(parse_float_edit("x1 1").unwrap_err().code, "MON-REG-005");
        assert_eq!(
            parse_integer_edit("x1 0x10000000000000000")
                .unwrap_err()
                .code,
            "MON-REG-003"
        );
    }

    #[test]
    fn formats_register_bits_without_host_float_conversion() {
        assert_eq!(format_raw(0x8000_0000_0000_0001), "0x8000000000000001");
    }
}
