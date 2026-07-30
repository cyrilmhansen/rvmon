#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_isa::{Addi, Lui, RType, encode_addi, encode_lui, encode_r};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectImage {
    pub text: Vec<u8>,
    pub entry: u64,
}

fn register(name: &str) -> Result<u8> {
    if let Some(number) = name.strip_prefix('x') {
        return number
            .parse::<u8>()
            .ok()
            .filter(|n| *n < 32)
            .ok_or_else(|| Diagnostic::error("ASM-REGISTER-001", "invalid integer register"));
    }
    match name {
        "zero" => Ok(0),
        "ra" => Ok(1),
        _ => Err(Diagnostic::error("ASM-REGISTER-001", "unknown register")),
    }
}

pub fn assemble(source: &str) -> Result<ObjectImage> {
    let line = source.split('#').next().unwrap_or("").trim();
    let mut words = line.splitn(2, char::is_whitespace);
    let mnemonic = words.next().unwrap_or("");
    let operands = words.next().unwrap_or("").trim();
    let parts: Vec<_> = operands.split(',').map(str::trim).collect();
    let word = match mnemonic {
        "addi" if parts.len() == 3 => encode_addi(Addi {
            rd: register(parts[0])?,
            rs1: register(parts[1])?,
            imm: parts[2]
                .parse::<i16>()
                .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid signed immediate"))?,
        })?,
        "add" | "sub" if parts.len() == 3 => encode_r(
            mnemonic,
            RType {
                rd: register(parts[0])?,
                rs1: register(parts[1])?,
                rs2: register(parts[2])?,
            },
        )?,
        "lui" if parts.len() == 2 => encode_lui(Lui {
            rd: register(parts[0])?,
            imm20: parts[1]
                .parse::<i32>()
                .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid U immediate"))?,
        })?,
        "" => return Err(Diagnostic::error("ASM-OPERAND-001", "missing instruction")),
        _ => {
            return Err(Diagnostic::error(
                "ASM-BOOT-UNSUPPORTED",
                "bootstrap assembler accepts addi, add, sub and lui",
            ));
        }
    };
    Ok(ObjectImage {
        text: word.to_le_bytes().to_vec(),
        entry: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assembles_required_first_program() {
        assert_eq!(
            assemble("addi x1,x0,1").unwrap().text,
            [0x93, 0x00, 0x10, 0x00]
        );
    }
    #[test]
    fn supports_abi_aliases() {
        assert!(assemble("addi ra,zero,1").is_ok());
    }

    #[test]
    fn assembles_generated_integer_forms() {
        assert!(assemble("add x5,x6,x7").is_ok());
        assert!(assemble("sub x5,x6,x7").is_ok());
        assert!(assemble("lui x3,74565").is_ok());
    }
}
