use luna_diag::{Diagnostic, Result};

use luna_target_api::TargetContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegisterSnapshot {
    pub(crate) x: [u64; 32],
    pub(crate) f: [u64; 32],
    pub(crate) fcsr: u32,
}

impl RegisterSnapshot {
    pub(crate) fn from_context(context: &TargetContext) -> Self {
        Self {
            x: context.x,
            f: context.f,
            fcsr: context.fcsr,
        }
    }
}

pub(crate) fn format_changes(before: RegisterSnapshot, after: RegisterSnapshot) -> String {
    let mut changes = Vec::new();
    for index in 0..32 {
        if before.x[index] != after.x[index] {
            changes.push(format!("x{index:02}={}", format_raw(after.x[index])));
        }
    }
    for index in 0..32 {
        if before.f[index] != after.f[index] {
            changes.push(format!("f{index:02}={}", format_raw(after.f[index])));
        }
    }
    if before.fcsr != after.fcsr {
        changes.push(format!(
            "fcsr=0x{:08x} (frm={} fflags={})",
            after.fcsr,
            (after.fcsr >> 5) & 0x7,
            format_flags((after.fcsr & 0x1f) as u8)
        ));
    }
    if changes.is_empty() {
        "none".into()
    } else {
        changes.join("; ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegisterEdit {
    Integer { index: usize, value: u64 },
    Float { index: usize, bits: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CsrEdit {
    Fcsr(u32),
    Frm(u8),
    Fflags(u8),
}

pub(crate) fn parse_csr_edit(argument: &str) -> Result<CsrEdit> {
    let parts: Vec<_> = argument.split_whitespace().collect();
    let [name, value] = parts.as_slice() else {
        return Err(Diagnostic::error(
            "MON-REG-008",
            "setcsr expects <fcsr|frm|fflags> <value>",
        ));
    };
    let value = parse_u64(value).map_err(|message| Diagnostic::error("MON-REG-009", message))?;
    match name.to_ascii_lowercase().as_str() {
        "fcsr" => {
            let value = u32::try_from(value).map_err(|_| {
                Diagnostic::error("MON-REG-010", "fcsr value does not fit in 32 bits")
            })?;
            validate_fcsr(value)?;
            Ok(CsrEdit::Fcsr(value))
        }
        "frm" => {
            let value = u8::try_from(value)
                .map_err(|_| Diagnostic::error("MON-REG-010", "frm must be in the range 0..4"))?;
            if value > 4 {
                return Err(Diagnostic::error(
                    "MON-REG-011",
                    "frm rounding mode 5..7 is reserved",
                ));
            }
            Ok(CsrEdit::Frm(value))
        }
        "fflags" => {
            let value = u8::try_from(value)
                .map_err(|_| Diagnostic::error("MON-REG-010", "fflags must fit in 5 bits"))?;
            if value & !0x1f != 0 {
                return Err(Diagnostic::error(
                    "MON-REG-010",
                    "fflags must fit in 5 bits",
                ));
            }
            Ok(CsrEdit::Fflags(value))
        }
        _ => Err(Diagnostic::error(
            "MON-REG-008",
            "setcsr name must be fcsr, frm, or fflags",
        )),
    }
}

fn validate_fcsr(value: u32) -> Result<()> {
    if value & !0xff != 0 {
        return Err(Diagnostic::error(
            "MON-REG-010",
            "fcsr may only contain bits 0..7",
        ));
    }
    if (value >> 5) & 0x7 > 4 {
        return Err(Diagnostic::error(
            "MON-REG-011",
            "fcsr frm field contains a reserved rounding mode",
        ));
    }
    Ok(())
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

fn format_flags(flags: u8) -> String {
    let mut names = Vec::new();
    for (mask, name) in [
        (0x10, "NV"),
        (0x08, "DZ"),
        (0x04, "OF"),
        (0x02, "UF"),
        (0x01, "NX"),
    ] {
        if flags & mask != 0 {
            names.push(name);
        }
    }
    if names.is_empty() {
        "-".into()
    } else {
        names.join("|")
    }
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

    #[test]
    fn formats_only_exact_register_changes_and_fcsr_fields() {
        let mut before = RegisterSnapshot::from_context(&TargetContext::empty());
        let mut after = before;
        after.x[10] = 0x8000_0000_0000_0000;
        after.f[3] = 0x3ff0_0000_0000_0000;
        after.fcsr = (2 << 5) | 0x11;
        assert_eq!(
            format_changes(before, after),
            "x10=0x8000000000000000; f03=0x3ff0000000000000; fcsr=0x00000051 (frm=2 fflags=NV|NX)"
        );
        before = after;
        assert_eq!(format_changes(before, after), "none");
    }

    #[test]
    fn parses_csr_fields_and_rejects_reserved_or_out_of_range_values() {
        assert_eq!(parse_csr_edit("fcsr 0x51").unwrap(), CsrEdit::Fcsr(0x51));
        assert_eq!(parse_csr_edit("frm 4").unwrap(), CsrEdit::Frm(4));
        assert_eq!(
            parse_csr_edit("fflags 0x11").unwrap(),
            CsrEdit::Fflags(0x11)
        );
        assert_eq!(parse_csr_edit("frm 5").unwrap_err().code, "MON-REG-011");
        assert_eq!(
            parse_csr_edit("fcsr 0x100").unwrap_err().code,
            "MON-REG-010"
        );
        assert_eq!(
            parse_csr_edit("fflags 0x20").unwrap_err().code,
            "MON-REG-010"
        );
    }
}
