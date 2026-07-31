#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use luna_asm_lexer::tokenize;
use luna_diag::{Diagnostic, Result};
use luna_isa::{
    Addi, Branch, Jal, Jalr, Load, Lui, RType, Store, encode_addi, encode_branch, encode_jal,
    encode_jalr, encode_load, encode_lui, encode_r, encode_store,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectImage {
    pub text: Vec<u8>,
    pub entry: u64,
    pub symbols: BTreeMap<String, u64>,
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

fn memory_operand(value: &str) -> Result<(i16, u8)> {
    let (immediate, base) = value.split_once('(').ok_or_else(|| {
        Diagnostic::error("ASM-MEMORY-001", "memory operand must be imm(register)")
    })?;
    let base = base
        .strip_suffix(')')
        .ok_or_else(|| Diagnostic::error("ASM-MEMORY-001", "missing closing parenthesis"))?;
    let immediate = immediate
        .parse::<i16>()
        .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid memory immediate"))?;
    Ok((immediate, register(base)?))
}

pub fn assemble(source: &str) -> Result<ObjectImage> {
    let line = source.split('#').next().unwrap_or("").trim();
    tokenize(line)?;
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
                .parse::<u32>()
                .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid U immediate"))?,
        })?,
        "lw" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(parts[1])?;
            encode_load(
                "lw",
                Load {
                    rd: register(parts[0])?,
                    rs1,
                    imm,
                },
            )?
        }
        "sw" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(parts[1])?;
            encode_store(
                "sw",
                Store {
                    rs2: register(parts[0])?,
                    rs1,
                    imm,
                },
            )?
        }
        "beq" | "bne" if parts.len() == 3 => encode_branch(
            mnemonic,
            Branch {
                rs1: register(parts[0])?,
                rs2: register(parts[1])?,
                imm: parts[2].parse::<i16>().map_err(|_| {
                    Diagnostic::error("ASM-IMMEDIATE-001", "invalid branch immediate")
                })?,
            },
        )?,
        "jal" if parts.len() == 2 => encode_jal(Jal {
            rd: register(parts[0])?,
            imm: parts[1]
                .parse::<i32>()
                .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid jump immediate"))?,
        })?,
        "jalr" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(parts[1])?;
            encode_jalr(Jalr {
                rd: register(parts[0])?,
                rs1,
                imm,
            })?
        }
        "" => return Err(Diagnostic::error("ASM-OPERAND-001", "missing instruction")),
        _ => {
            return Err(Diagnostic::error(
                "ASM-BOOT-UNSUPPORTED",
                "bootstrap assembler accepts addi, add, sub, lui, lw, sw, beq, bne, jal and jalr",
            ));
        }
    };
    Ok(ObjectImage {
        text: word.to_le_bytes().to_vec(),
        entry: 0,
        symbols: BTreeMap::new(),
    })
}

pub fn assemble_program(source: &str) -> Result<ObjectImage> {
    let mut symbols = BTreeMap::new();
    let mut pc = 0u64;
    for raw_line in source.lines() {
        let mut line = raw_line.split('#').next().unwrap_or("").trim();
        while let Some((label, rest)) = line.split_once(':') {
            let label = label.trim();
            if label.is_empty()
                || !label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "_.$".contains(character))
            {
                return Err(Diagnostic::error("ASM-LABEL-001", "invalid label"));
            }
            if symbols.insert(label.to_string(), pc).is_some() {
                return Err(Diagnostic::error("ASM-LABEL-002", "duplicate label"));
            }
            line = rest.trim();
        }
        if !line.is_empty() {
            pc = pc
                .checked_add(4)
                .ok_or_else(|| Diagnostic::error("ASM-ADDRESS-001", "program is too large"))?;
        }
    }

    let mut text = Vec::new();
    pc = 0;
    for raw_line in source.lines() {
        let mut line = raw_line.split('#').next().unwrap_or("").trim();
        while let Some((_, rest)) = line.split_once(':') {
            line = rest.trim();
        }
        if line.is_empty() {
            continue;
        }
        let resolved = resolve_control_label(line, pc, &symbols)?;
        let image = assemble(&resolved)?;
        text.extend_from_slice(&image.text);
        pc += 4;
    }
    let entry = symbols.get("_start").copied().unwrap_or(0);
    Ok(ObjectImage {
        text,
        entry,
        symbols,
    })
}

fn resolve_control_label(line: &str, pc: u64, symbols: &BTreeMap<String, u64>) -> Result<String> {
    let mut words = line.splitn(2, char::is_whitespace);
    let mnemonic = words.next().unwrap_or("");
    let operands = words.next().unwrap_or("");
    let mut parts: Vec<_> = operands.split(',').map(str::trim).collect();
    let index = match mnemonic {
        "beq" | "bne" => Some(2),
        "jal" => Some(1),
        _ => None,
    };
    if let Some(index) = index {
        if let Some(label) = parts.get(index).copied() {
            if label.parse::<i64>().is_err() {
                let target = symbols.get(label).ok_or_else(|| {
                    Diagnostic::error("ASM-SYMBOL-001", format!("unknown symbol: {label}"))
                })?;
                let offset = *target as i64 - pc as i64;
                parts[index] = Box::leak(offset.to_string().into_boxed_str());
                return Ok(format!("{} {}", mnemonic, parts.join(",")));
            }
        }
    }
    Ok(line.to_string())
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
        assert!(assemble("lw x3,8(x4)").is_ok());
        assert!(assemble("sw x3,-8(x4)").is_ok());
        assert!(assemble("beq x1,x2,-4").is_ok());
        assert!(assemble("jal ra,2048").is_ok());
        assert!(assemble("jalr ra,0(x4)").is_ok());
    }

    #[test]
    fn assembles_program_with_labels_and_control_flow() {
        let image = assemble_program("_start: addi x1,x0,1\n       beq x1,x1,done\n       addi x1,x0,99\ndone:  addi x2,x0,7").unwrap();
        assert_eq!(image.symbols["_start"], 0);
        assert_eq!(image.symbols["done"], 12);
        assert_eq!(image.text.len(), 16);
    }
}
