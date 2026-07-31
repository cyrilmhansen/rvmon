#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use luna_diag::{Diagnostic, Result};
use luna_isa::{
    Addi, Branch, Jal, Jalr, Load, Lui, RType, Store, encode_addi, encode_branch, encode_jal,
    encode_jalr, encode_load, encode_lui, encode_r, encode_store,
};

mod parser;
pub use parser::{Operand, OperandKind, ParsedLine, parse_line};

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
        "sp" => Ok(2),
        "gp" => Ok(3),
        "tp" => Ok(4),
        "t0" => Ok(5),
        "t1" => Ok(6),
        "t2" => Ok(7),
        "s0" | "fp" => Ok(8),
        "s1" => Ok(9),
        "a0" => Ok(10),
        "a1" => Ok(11),
        "a2" => Ok(12),
        "a3" => Ok(13),
        "a4" => Ok(14),
        "a5" => Ok(15),
        "a6" => Ok(16),
        "a7" => Ok(17),
        "s2" => Ok(18),
        "s3" => Ok(19),
        "s4" => Ok(20),
        "s5" => Ok(21),
        "s6" => Ok(22),
        "s7" => Ok(23),
        "s8" => Ok(24),
        "s9" => Ok(25),
        "s10" => Ok(26),
        "s11" => Ok(27),
        "t3" => Ok(28),
        "t4" => Ok(29),
        "t5" => Ok(30),
        "t6" => Ok(31),
        _ => Err(Diagnostic::error("ASM-REGISTER-001", "unknown register")),
    }
}

fn operand_text(operand: &Operand) -> Result<&str> {
    match &operand.kind {
        OperandKind::Register(value) | OperandKind::Integer(value) | OperandKind::Symbol(value) => {
            Ok(value)
        }
        _ => Err(
            Diagnostic::error("ASM-OPERAND-002", "expected a scalar operand")
                .at(operand.span.line, operand.span.column),
        ),
    }
}

fn memory_operand(operand: &Operand) -> Result<(i16, u8)> {
    let OperandKind::Memory { offset, base } = &operand.kind else {
        return Err(
            Diagnostic::error("ASM-MEMORY-001", "memory operand must be imm(register)")
                .at(operand.span.line, operand.span.column),
        );
    };
    let immediate = offset.parse::<i16>().map_err(|_| {
        Diagnostic::error("ASM-IMMEDIATE-001", "invalid memory immediate")
            .at(operand.span.line, operand.span.column)
    })?;
    Ok((immediate, register(base)?))
}

pub fn assemble(source: &str) -> Result<ObjectImage> {
    let parsed = parse_line(source)?;
    if !parsed.labels.is_empty() {
        return Err(Diagnostic::error(
            "ASM-LABEL-003",
            "labels are only valid when assembling a program",
        ));
    }
    assemble_parsed(&parsed)
}

fn assemble_parsed(parsed: &ParsedLine) -> Result<ObjectImage> {
    let mnemonic = parsed.mnemonic.as_deref().unwrap_or("");
    let parts = &parsed.operands;
    let word = match mnemonic {
        "addi" if parts.len() == 3 => encode_addi(Addi {
            rd: register(operand_text(&parts[0])?)?,
            rs1: register(operand_text(&parts[1])?)?,
            imm: operand_text(&parts[2])?.parse::<i16>().map_err(|_| {
                Diagnostic::error("ASM-IMMEDIATE-001", "invalid signed immediate")
                    .at(parts[2].span.line, parts[2].span.column)
            })?,
        })?,
        "add" | "sub" if parts.len() == 3 => encode_r(
            mnemonic,
            RType {
                rd: register(operand_text(&parts[0])?)?,
                rs1: register(operand_text(&parts[1])?)?,
                rs2: register(operand_text(&parts[2])?)?,
            },
        )?,
        "lui" if parts.len() == 2 => encode_lui(Lui {
            rd: register(operand_text(&parts[0])?)?,
            imm20: operand_text(&parts[1])?.parse::<u32>().map_err(|_| {
                Diagnostic::error("ASM-IMMEDIATE-001", "invalid U immediate")
                    .at(parts[1].span.line, parts[1].span.column)
            })?,
        })?,
        "lw" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1])?;
            encode_load(
                "lw",
                Load {
                    rd: register(operand_text(&parts[0])?)?,
                    rs1,
                    imm,
                },
            )?
        }
        "sw" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1])?;
            encode_store(
                "sw",
                Store {
                    rs2: register(operand_text(&parts[0])?)?,
                    rs1,
                    imm,
                },
            )?
        }
        "beq" | "bne" if parts.len() == 3 => encode_branch(
            mnemonic,
            Branch {
                rs1: register(operand_text(&parts[0])?)?,
                rs2: register(operand_text(&parts[1])?)?,
                imm: operand_text(&parts[2])?.parse::<i16>().map_err(|_| {
                    Diagnostic::error("ASM-IMMEDIATE-001", "invalid branch immediate")
                        .at(parts[2].span.line, parts[2].span.column)
                })?,
            },
        )?,
        "jal" if parts.len() == 2 => encode_jal(Jal {
            rd: register(operand_text(&parts[0])?)?,
            imm: operand_text(&parts[1])?.parse::<i32>().map_err(|_| {
                Diagnostic::error("ASM-IMMEDIATE-001", "invalid jump immediate")
                    .at(parts[1].span.line, parts[1].span.column)
            })?,
        })?,
        "jalr" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1])?;
            encode_jalr(Jalr {
                rd: register(operand_text(&parts[0])?)?,
                rs1,
                imm,
            })?
        }
        "" => return Err(Diagnostic::error("ASM-OPERAND-001", "missing instruction")),
        _ if mnemonic.is_empty() => {
            return Err(Diagnostic::error("ASM-OPERAND-001", "missing instruction"));
        }
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
    let lines: Vec<_> = source.lines().map(parse_line).collect::<Result<_>>()?;
    let mut symbols = BTreeMap::new();
    let mut pc = 0u64;
    for line in &lines {
        for label in &line.labels {
            if symbols.insert(label.clone(), pc).is_some() {
                return Err(Diagnostic::error("ASM-LABEL-002", "duplicate label"));
            }
        }
        if line.mnemonic.is_some() {
            pc = pc
                .checked_add(4)
                .ok_or_else(|| Diagnostic::error("ASM-ADDRESS-001", "program is too large"))?;
        }
    }

    let mut text = Vec::new();
    pc = 0;
    for line in lines {
        if line.mnemonic.is_none() {
            continue;
        }
        let resolved = resolve_control_label(line, pc, &symbols)?;
        let image = assemble_parsed(&resolved)?;
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

fn resolve_control_label(
    mut line: ParsedLine,
    pc: u64,
    symbols: &BTreeMap<String, u64>,
) -> Result<ParsedLine> {
    let index = match line.mnemonic.as_deref() {
        Some("beq") | Some("bne") => Some(2),
        Some("jal") => Some(1),
        _ => None,
    };
    if let Some(index) = index {
        if let Some(operand) = line.operands.get_mut(index) {
            if let OperandKind::Symbol(label) = &operand.kind {
                let target = symbols.get(label).ok_or_else(|| {
                    Diagnostic::error("ASM-SYMBOL-001", format!("unknown symbol: {label}"))
                        .at(operand.span.line, operand.span.column)
                })?;
                let offset = *target as i64 - pc as i64;
                operand.kind = OperandKind::Integer(offset.to_string());
            }
        }
    }
    Ok(line)
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
