#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use luna_diag::{Diagnostic, Result};
use luna_isa::{Addi, Branch, FRegisterRType, Instruction, Jal, Jalr, Load, Lui, RType, Store};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisassembledLine {
    pub address: u64,
    pub word: u32,
    pub instruction: Instruction,
    pub text: String,
}

pub fn disassemble_word(
    address: u64,
    word: u32,
    symbols: &BTreeMap<u64, String>,
) -> DisassembledLine {
    let instruction = luna_isa::decode(word);
    let text = format_instruction(address, instruction, symbols);
    DisassembledLine {
        address,
        word,
        instruction,
        text,
    }
}

pub fn disassemble_bytes(
    bytes: &[u8],
    origin: u64,
    symbols: &BTreeMap<u64, String>,
) -> Result<Vec<DisassembledLine>> {
    if bytes.len() % 2 != 0 {
        return Err(Diagnostic::error(
            "DISASM-ALIGN-001",
            "instruction bytes must have a two-byte length",
        ));
    }
    let mut lines = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let halfword = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        if halfword & 0x3 != 0x3 {
            return Err(Diagnostic::error(
                "DISASM-C-001",
                "compressed 16-bit instruction is not enabled in this disassembler",
            ));
        }
        if offset + 4 > bytes.len() {
            return Err(Diagnostic::error(
                "DISASM-ALIGN-001",
                "instruction bytes must have a four-byte length",
            ));
        }
        let address = origin
            .checked_add(offset as u64)
            .ok_or_else(|| Diagnostic::error("DISASM-ADDRESS-001", "address overflow"))?;
        lines.push(disassemble_word(
            address,
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            symbols,
        ));
        offset += 4;
    }
    Ok(lines)
}

fn format_instruction(
    address: u64,
    instruction: Instruction,
    symbols: &BTreeMap<u64, String>,
) -> String {
    match instruction {
        Instruction::Addi(Addi { rd, rs1, imm }) => format!("addi x{rd},x{rs1},{imm}"),
        Instruction::Add(RType { rd, rs1, rs2 }) => format!("add x{rd},x{rs1},x{rs2}"),
        Instruction::Sub(RType { rd, rs1, rs2 }) => format!("sub x{rd},x{rs1},x{rs2}"),
        Instruction::Lui(Lui { rd, imm20 }) => format!("lui x{rd},{imm20}"),
        Instruction::Lw(Load { rd, rs1, imm }) => format!("lw x{rd},{imm}(x{rs1})"),
        Instruction::Sw(Store { rs2, rs1, imm }) => format!("sw x{rs2},{imm}(x{rs1})"),
        Instruction::Ld(Load { rd, rs1, imm }) => format!("ld x{rd},{imm}(x{rs1})"),
        Instruction::Sd(Store { rs2, rs1, imm }) => format!("sd x{rs2},{imm}(x{rs1})"),
        Instruction::Beq(Branch { rs1, rs2, imm }) => format!(
            "beq x{rs1},x{rs2},{}",
            relative_target(address, i64::from(imm), symbols)
        ),
        Instruction::Bne(Branch { rs1, rs2, imm }) => format!(
            "bne x{rs1},x{rs2},{}",
            relative_target(address, i64::from(imm), symbols)
        ),
        Instruction::Jal(Jal { rd, imm }) => {
            format!(
                "jal x{rd},{}",
                relative_target(address, i64::from(imm), symbols)
            )
        }
        Instruction::Jalr(Jalr { rd, rs1, imm }) => format!("jalr x{rd},{imm}(x{rs1})"),
        Instruction::FAddS(FRegisterRType {
            rd,
            rs1,
            rs2,
            rm: _,
        }) => {
            format!("fadd.s f{rd},f{rs1},f{rs2}")
        }
        Instruction::FAddD(FRegisterRType {
            rd,
            rs1,
            rs2,
            rm: _,
        }) => {
            format!("fadd.d f{rd},f{rs1},f{rs2}")
        }
        Instruction::Illegal(word) => format!(".word 0x{word:08x}"),
    }
}

fn relative_target(address: u64, offset: i64, symbols: &BTreeMap<u64, String>) -> String {
    address
        .checked_add_signed(offset)
        .and_then(|target| symbols.get(&target))
        .cloned()
        .unwrap_or_else(|| offset.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use luna_isa::{Addi, Branch, Instruction, encode_addi, encode_branch};

    #[test]
    fn formats_canonical_integer_instruction() {
        let word = encode_addi(Addi {
            rd: 1,
            rs1: 0,
            imm: 1,
        })
        .unwrap();
        let line = disassemble_word(0, word, &BTreeMap::new());
        assert_eq!(line.text, "addi x1,x0,1");
        assert_eq!(
            line.instruction,
            Instruction::Addi(Addi {
                rd: 1,
                rs1: 0,
                imm: 1
            })
        );
    }

    #[test]
    fn substitutes_a_symbol_for_relative_targets() {
        let word = encode_branch(
            "beq",
            Branch {
                rs1: 1,
                rs2: 1,
                imm: 8,
            },
        )
        .unwrap();
        let symbols = BTreeMap::from([(12, String::from("done"))]);
        assert_eq!(disassemble_word(4, word, &symbols).text, "beq x1,x1,done");
    }

    #[test]
    fn preserves_illegal_words_as_reassemblable_data() {
        let line = disassemble_word(0, 0, &BTreeMap::new());
        assert_eq!(line.instruction, Instruction::Illegal(0));
        assert_eq!(line.text, ".word 0x00000000");
    }

    #[test]
    fn rejects_partial_instruction_units() {
        let error = disassemble_bytes(&[0x93], 0, &BTreeMap::new()).unwrap_err();
        assert_eq!(error.code, "DISASM-ALIGN-001");
    }

    #[test]
    fn rejects_compressed_units_instead_of_misdecoding_them() {
        let error = disassemble_bytes(&[0x01, 0x00], 0, &BTreeMap::new()).unwrap_err();
        assert_eq!(error.code, "DISASM-C-001");
    }

    #[test]
    fn canonical_text_reassembles_to_the_same_word() {
        let source = "lw x3,-8(x4)";
        let word = luna_assembler::assemble(source).unwrap().text;
        let line = disassemble_bytes(&word, 0, &BTreeMap::new())
            .unwrap()
            .remove(0);
        let rebuilt = luna_assembler::assemble(&line.text).unwrap();
        assert_eq!(rebuilt.text, word);
    }

    #[test]
    fn all_supported_integer_forms_round_trip() {
        for source in [
            "addi x1,x0,-7",
            "add x5,x6,x7",
            "sub x5,x6,x7",
            "lui x3,74565",
            "lw x3,8(x4)",
            "sw x3,-8(x4)",
            "ld x3,-8(x4)",
            "sd x3,8(x4)",
            "beq x1,x2,-4",
            "bne x1,x2,6",
            "jal x1,2048",
            "jalr x1,0(x4)",
            "fadd.s f3,f1,f2",
            "fadd.d f3,f1,f2",
        ] {
            let original = luna_assembler::assemble(source).unwrap().text;
            let line = disassemble_bytes(&original, 0, &BTreeMap::new())
                .unwrap()
                .remove(0);
            let rebuilt = luna_assembler::assemble(&line.text).unwrap().text;
            assert_eq!(rebuilt, original, "round-trip failed for {source}");
        }
    }
}
