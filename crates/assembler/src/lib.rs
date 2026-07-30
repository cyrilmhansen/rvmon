#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_isa::{Addi, encode_addi};

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
    let operands = line
        .strip_prefix("addi")
        .ok_or_else(|| {
            Diagnostic::error(
                "ASM-BOOT-UNSUPPORTED",
                "bootstrap assembler accepts addi only",
            )
        })?
        .trim();
    let parts: Vec<_> = operands.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(Diagnostic::error(
            "ASM-OPERAND-001",
            "addi requires rd, rs1, imm",
        ));
    }
    let instruction = Addi {
        rd: register(parts[0])?,
        rs1: register(parts[1])?,
        imm: parts[2]
            .parse::<i16>()
            .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid signed immediate"))?,
    };
    Ok(ObjectImage {
        text: encode_addi(instruction)?.to_le_bytes().to_vec(),
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
}
