#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use luna_diag::{Diagnostic, Result};
use luna_isa::{
    Addi, Branch, FRegisterRType, FloatConversion, FloatConversionKind, GENERATED_OPCODES,
    Instruction, Jal, Jalr, Load, Lui, RType, Store,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisassembledLine {
    pub address: u64,
    pub word: u32,
    pub instruction: Instruction,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisassemblyRegionKind {
    Code,
    Data,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisassemblyRegion {
    pub offset: usize,
    pub length: usize,
    pub kind: DisassemblyRegionKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisassemblyOptions {
    pub enable_compressed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataLine {
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompressedLine {
    pub address: u64,
    pub bits: u16,
    pub text: String,
    pub legal: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisassembledItem {
    Instruction(DisassembledLine),
    Compressed(CompressedLine),
    Data(DataLine),
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

/// Disassemble an image whose caller has explicitly marked code and data
/// regions. Regions must be contiguous, non-empty, non-overlapping and cover
/// the complete byte slice. Data is emitted as reassemblable `.byte` lines and
/// is never passed to the instruction decoder.
pub fn disassemble_regions(
    bytes: &[u8],
    origin: u64,
    regions: &[DisassemblyRegion],
    symbols: &BTreeMap<u64, String>,
) -> Result<Vec<DisassembledItem>> {
    disassemble_regions_with_options(
        bytes,
        origin,
        regions,
        symbols,
        DisassemblyOptions::default(),
    )
}

pub fn disassemble_regions_with_options(
    bytes: &[u8],
    origin: u64,
    regions: &[DisassemblyRegion],
    symbols: &BTreeMap<u64, String>,
    options: DisassemblyOptions,
) -> Result<Vec<DisassembledItem>> {
    let mut cursor = 0usize;
    let mut items = Vec::new();
    for region in regions {
        if region.length == 0 || region.offset != cursor {
            return Err(Diagnostic::error(
                "DISASM-REGION-001",
                "regions must be contiguous and non-empty",
            ));
        }
        let end = region
            .offset
            .checked_add(region.length)
            .ok_or_else(|| Diagnostic::error("DISASM-REGION-002", "region range overflows"))?;
        if end > bytes.len() {
            return Err(Diagnostic::error(
                "DISASM-REGION-002",
                "region exceeds the byte image",
            ));
        }
        let address = origin
            .checked_add(region.offset as u64)
            .ok_or_else(|| Diagnostic::error("DISASM-ADDRESS-001", "address overflow"))?;
        match region.kind {
            DisassemblyRegionKind::Code => {
                let lines =
                    disassemble_code(&bytes[region.offset..end], address, symbols, options)?;
                items.extend(lines);
            }
            DisassemblyRegionKind::Data => {
                for (chunk_index, chunk) in bytes[region.offset..end].chunks(16).enumerate() {
                    let chunk_address =
                        address
                            .checked_add((chunk_index * 16) as u64)
                            .ok_or_else(|| {
                                Diagnostic::error("DISASM-ADDRESS-001", "address overflow")
                            })?;
                    let text = format!(
                        ".byte {}",
                        chunk
                            .iter()
                            .map(|byte| format!("0x{byte:02x}"))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    items.push(DisassembledItem::Data(DataLine {
                        address: chunk_address,
                        bytes: chunk.to_vec(),
                        text,
                    }));
                }
            }
        }
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(Diagnostic::error(
            "DISASM-REGION-001",
            "regions must cover the complete byte image",
        ));
    }
    Ok(items)
}

fn disassemble_code(
    bytes: &[u8],
    origin: u64,
    symbols: &BTreeMap<u64, String>,
    options: DisassemblyOptions,
) -> Result<Vec<DisassembledItem>> {
    let mut items = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if offset + 2 > bytes.len() {
            return Err(Diagnostic::error(
                "DISASM-ALIGN-001",
                "instruction bytes must have a two-byte length",
            ));
        }
        let halfword = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let address = origin
            .checked_add(offset as u64)
            .ok_or_else(|| Diagnostic::error("DISASM-ADDRESS-001", "address overflow"))?;
        if halfword & 0x3 != 0x3 {
            if !options.enable_compressed {
                return Err(Diagnostic::error(
                    "DISASM-C-001",
                    "compressed 16-bit instruction is not enabled in this disassembler",
                ));
            }
            items.push(DisassembledItem::Compressed(decode_compressed(
                address, halfword, symbols,
            )));
            offset += 2;
        } else {
            if offset + 4 > bytes.len() {
                return Err(Diagnostic::error(
                    "DISASM-ALIGN-001",
                    "instruction bytes must have a four-byte length",
                ));
            }
            let word = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            items.push(DisassembledItem::Instruction(disassemble_word(
                address, word, symbols,
            )));
            offset += 4;
        }
    }
    Ok(items)
}

fn sign_extend(value: u32, bits: u8) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

fn compressed_prime_register(bits: u16) -> u8 {
    (((bits >> 7) & 0x7) as u8) + 8
}

fn compressed_imm6(bits: u16) -> i32 {
    sign_extend(
        u32::from(((bits >> 2) & 0x1f) | (((bits >> 12) & 1) << 5)),
        6,
    )
}

fn compressed_shamt(bits: u16) -> u32 {
    u32::from(((bits >> 2) & 0x1f) | (((bits >> 12) & 1) << 5))
}

fn compressed_lw_offset(bits: u16) -> u32 {
    (((u32::from(bits) >> 10) & 0x7) << 6)
        | (((u32::from(bits) >> 6) & 1) << 2)
        | (((u32::from(bits) >> 3) & 0x7) << 3)
}

fn compressed_lwsp_offset(bits: u16) -> u32 {
    (((u32::from(bits) >> 4) & 0x7) << 3) | (((u32::from(bits) >> 2) & 0x3) << 6)
}

fn compressed_ldsp_offset(bits: u16) -> u32 {
    (((u32::from(bits) >> 4) & 0x7) << 3) | (((u32::from(bits) >> 10) & 0x7) << 6)
}

fn compressed_swsp_offset(bits: u16) -> u32 {
    (((u32::from(bits) >> 9) & 0xf) << 2) | (((u32::from(bits) >> 7) & 0x3) << 6)
}

fn compressed_sdsp_offset(bits: u16) -> u32 {
    (((u32::from(bits) >> 10) & 0x7) << 3) | (((u32::from(bits) >> 7) & 0x7) << 6)
}

fn compressed_addi16sp_immediate(bits: u16) -> i32 {
    let value = (((u32::from(bits) >> 12) & 1) << 9)
        | (((u32::from(bits) >> 6) & 1) << 4)
        | (((u32::from(bits) >> 5) & 1) << 6)
        | (((u32::from(bits) >> 3) & 0x3) << 7)
        | (((u32::from(bits) >> 2) & 1) << 5);
    sign_extend(value, 10)
}

fn compressed_lui_immediate(bits: u16) -> i32 {
    let value = (((u32::from(bits) >> 12) & 1) << 17) | (((u32::from(bits) >> 2) & 0x1f) << 12);
    sign_extend(value, 18)
}

fn compressed_jump_immediate(bits: u16) -> i32 {
    let value = (((u32::from(bits) >> 12) & 1) << 11)
        | (((u32::from(bits) >> 11) & 1) << 4)
        | (((u32::from(bits) >> 9) & 0x3) << 8)
        | (((u32::from(bits) >> 8) & 1) << 10)
        | (((u32::from(bits) >> 7) & 1) << 6)
        | (((u32::from(bits) >> 6) & 1) << 7)
        | (((u32::from(bits) >> 3) & 0x7) << 1)
        | (((u32::from(bits) >> 2) & 1) << 5);
    sign_extend(value, 12)
}

fn compressed_branch_immediate(bits: u16) -> i32 {
    let value = (((u32::from(bits) >> 12) & 1) << 8)
        | (((u32::from(bits) >> 10) & 0x3) << 3)
        | (((u32::from(bits) >> 5) & 0x3) << 6)
        | (((u32::from(bits) >> 3) & 0x3) << 1)
        | (((u32::from(bits) >> 2) & 1) << 5);
    sign_extend(value, 9)
}

fn decode_compressed(address: u64, bits: u16, symbols: &BTreeMap<u64, String>) -> CompressedLine {
    let matched = GENERATED_OPCODES.iter().find(|opcode| {
        opcode.instruction_bits == 16 && (u32::from(bits) & opcode.mask) == opcode.match_value
    });
    let text = matched
        .and_then(|opcode| format_compressed(opcode.mnemonic, bits, address, symbols))
        .unwrap_or_else(|| format!(".half 0x{bits:04x}"));
    CompressedLine {
        address,
        bits,
        legal: !text.starts_with(".half "),
        text,
    }
}

fn format_compressed(
    mnemonic: &str,
    bits: u16,
    address: u64,
    symbols: &BTreeMap<u64, String>,
) -> Option<String> {
    let rd = ((bits >> 7) & 0x1f) as u8;
    let rs2 = ((bits >> 2) & 0x1f) as u8;
    let rd_prime = compressed_prime_register(bits);
    let rs2_prime = (rs2 & 0x7) + 8;
    let imm6 = compressed_imm6(bits);
    match mnemonic {
        "c.addi4spn" => {
            let immediate = (((u32::from(bits) >> 11) & 0x3) << 4)
                | (((u32::from(bits) >> 7) & 0xf) << 6)
                | (((u32::from(bits) >> 5) & 0x3) << 2);
            (immediate != 0).then(|| format!("c.addi4spn x{rd_prime},x2,{immediate}"))
        }
        "c.lw" => Some(format!(
            "c.lw x{rd_prime},{}(x{})",
            compressed_lw_offset(bits),
            compressed_prime_register(bits)
        )),
        "c.sw" => Some(format!(
            "c.sw x{},{}(x{})",
            rs2_prime,
            compressed_lw_offset(bits),
            compressed_prime_register(bits)
        )),
        "c.nop" => Some("c.nop".into()),
        "c.addi" => (rd != 0).then(|| format!("c.addi x{rd},x{rd},{imm6}")),
        "c.li" => (rd != 0).then(|| format!("c.li x{rd},{imm6}")),
        "c.addi16sp" => {
            let immediate = compressed_addi16sp_immediate(bits);
            (rd == 2 && immediate != 0).then(|| format!("c.addi16sp x2,{immediate}"))
        }
        "c.lui" => {
            let immediate = compressed_lui_immediate(bits);
            (rd != 0 && rd != 2 && immediate != 0).then(|| format!("c.lui x{rd},{immediate}"))
        }
        "c.andi" => (rd_prime != 0).then(|| format!("c.andi x{rd_prime},x{rd_prime},{imm6}")),
        "c.sub" | "c.xor" | "c.or" | "c.and" => {
            (rs2_prime != 0).then(|| format!("{mnemonic} x{rd_prime},x{rs2_prime}"))
        }
        "c.j" => Some(format!(
            "c.j {}",
            relative_target(address, i64::from(compressed_jump_immediate(bits)), symbols)
        )),
        "c.beqz" | "c.bnez" => Some(format!(
            "{mnemonic} x{rd_prime},{}",
            relative_target(
                address,
                i64::from(compressed_branch_immediate(bits)),
                symbols
            )
        )),
        "c.lwsp" => (rd != 0).then(|| format!("c.lwsp x{rd},{}(x2)", compressed_lwsp_offset(bits))),
        "c.jr" => {
            let rs1 = rd;
            (rs1 != 0).then(|| format!("c.jr x{rs1}"))
        }
        "c.mv" => (rd != 0 && rs2 != 0).then(|| format!("c.mv x{rd},x{rs2}")),
        "c.ebreak" => Some("c.ebreak".into()),
        "c.jalr" => (rd != 0).then(|| format!("c.jalr x{rd}")),
        "c.add" => (rd != 0 && rs2 != 0).then(|| format!("c.add x{rd},x{rs2}")),
        "c.swsp" => Some(format!(
            "c.swsp x{rs2},{}(x2)",
            compressed_swsp_offset(bits)
        )),
        "c.ld" => Some(format!(
            "c.ld x{rd_prime},{}(x{})",
            compressed_lw_offset(bits),
            compressed_prime_register(bits)
        )),
        "c.sd" => Some(format!(
            "c.sd x{},{}(x{})",
            rs2_prime,
            compressed_lw_offset(bits),
            compressed_prime_register(bits)
        )),
        "c.addiw" => (rd != 0).then(|| format!("c.addiw x{rd},x{rd},{imm6}")),
        "c.srli" | "c.srai" => {
            let shamt = compressed_shamt(bits);
            (rd_prime != 0 && shamt != 0)
                .then(|| format!("{mnemonic} x{rd_prime},x{rd_prime},{shamt}"))
        }
        "c.subw" | "c.addw" => {
            (rs2_prime != 0).then(|| format!("{mnemonic} x{rd_prime},x{rs2_prime}"))
        }
        "c.slli" => {
            let shamt = compressed_shamt(bits);
            (rd != 0).then(|| format!("c.slli x{rd},x{rd},{shamt}"))
        }
        "c.ldsp" => (rd != 0).then(|| format!("c.ldsp x{rd},{}(x2)", compressed_ldsp_offset(bits))),
        "c.sdsp" => Some(format!(
            "c.sdsp x{rs2},{}(x2)",
            compressed_sdsp_offset(bits)
        )),
        _ => None,
    }
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
        Instruction::Auipc(Lui { rd, imm20 }) => format!("auipc x{rd},{imm20}"),
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
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::SFromD,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.s.d f{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::DFromS,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.d.s f{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::WFromS,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.w.s x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::WuFromS,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.wu.s x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::SFromW,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.s.w f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::SFromWu,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.s.wu f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::WFromD,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.w.d x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::WuFromD,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.wu.d x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::DFromW,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.d.w f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::DFromWu,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.d.wu f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::LFromS,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.l.s x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::LuFromS,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.lu.s x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::SFromL,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.s.l f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::SFromLu,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.s.lu f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::LFromD,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.l.d x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::LuFromD,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.lu.d x{rd},f{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::DFromL,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.d.l f{rd},x{rs1}"),
        Instruction::FloatConversion(FloatConversion {
            kind: FloatConversionKind::DFromLu,
            rd,
            rs1,
            rm: _,
        }) => format!("fcvt.d.lu f{rd},x{rs1}"),
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
    fn opt_in_compressed_mode_walks_a_mixed_16_and_32_bit_code_stream() {
        let mut bytes = vec![0x01, 0x00];
        bytes.extend(luna_assembler::assemble("addi x1,x0,1").unwrap().text);
        let regions = [DisassemblyRegion {
            offset: 0,
            length: bytes.len(),
            kind: DisassemblyRegionKind::Code,
        }];
        let error = disassemble_regions(&bytes, 0, &regions, &BTreeMap::new()).unwrap_err();
        assert_eq!(error.code, "DISASM-C-001");

        let items = disassemble_regions_with_options(
            &bytes,
            0,
            &regions,
            &BTreeMap::new(),
            DisassemblyOptions {
                enable_compressed: true,
            },
        )
        .unwrap();
        assert_eq!(items.len(), 2);
        let DisassembledItem::Compressed(compressed) = &items[0] else {
            panic!("first item must be compressed");
        };
        assert_eq!(compressed.text, "c.nop");
        assert!(compressed.legal);
        let DisassembledItem::Instruction(instruction) = &items[1] else {
            panic!("second item must be 32-bit");
        };
        assert_eq!(instruction.text, "addi x1,x0,1");
        assert_eq!(instruction.address, 2);
    }

    #[test]
    fn marks_an_invalid_compressed_encoding_without_stopping_the_stream() {
        let regions = [DisassemblyRegion {
            offset: 0,
            length: 2,
            kind: DisassemblyRegionKind::Code,
        }];
        let items = disassemble_regions_with_options(
            &[0x00, 0x00],
            0,
            &regions,
            &BTreeMap::new(),
            DisassemblyOptions {
                enable_compressed: true,
            },
        )
        .unwrap();
        let DisassembledItem::Compressed(line) = &items[0] else {
            panic!("invalid compressed unit must remain a compressed item");
        };
        assert_eq!(line.text, ".half 0x0000");
        assert!(!line.legal);
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
            "auipc x3,74565",
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
            "fcvt.s.d f3,f1",
            "fcvt.d.s f3,f1",
            "fcvt.w.s x3,f1",
            "fcvt.wu.s x3,f1",
            "fcvt.s.w f3,x1",
            "fcvt.s.wu f3,x1",
            "fcvt.w.d x3,f1",
            "fcvt.wu.d x3,f1",
            "fcvt.d.w f3,x1",
            "fcvt.d.wu f3,x1",
            "fcvt.l.s x3,f1",
            "fcvt.lu.s x3,f1",
            "fcvt.s.l f3,x1",
            "fcvt.s.lu f3,x1",
            "fcvt.l.d x3,f1",
            "fcvt.lu.d x3,f1",
            "fcvt.d.l f3,x1",
            "fcvt.d.lu f3,x1",
        ] {
            let original = luna_assembler::assemble(source).unwrap().text;
            let line = disassemble_bytes(&original, 0, &BTreeMap::new())
                .unwrap()
                .remove(0);
            let rebuilt = luna_assembler::assemble(&line.text).unwrap().text;
            assert_eq!(rebuilt, original, "round-trip failed for {source}");
        }
    }

    #[test]
    fn disassembles_explicit_code_and_data_regions_without_crossing_boundaries() {
        let first = luna_assembler::assemble("addi x1,x0,1").unwrap().text;
        let second = luna_assembler::assemble("addi x2,x0,2").unwrap().text;
        let mut bytes = first.clone();
        bytes.extend([0xde, 0xad, 0xbe]);
        bytes.extend(second.clone());
        let regions = [
            DisassemblyRegion {
                offset: 0,
                length: first.len(),
                kind: DisassemblyRegionKind::Code,
            },
            DisassemblyRegion {
                offset: first.len(),
                length: 3,
                kind: DisassemblyRegionKind::Data,
            },
            DisassemblyRegion {
                offset: first.len() + 3,
                length: second.len(),
                kind: DisassemblyRegionKind::Code,
            },
        ];
        let items = disassemble_regions(&bytes, 0x1000, &regions, &BTreeMap::new()).unwrap();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], DisassembledItem::Instruction(_)));
        assert!(matches!(items[1], DisassembledItem::Data(_)));
        assert!(matches!(items[2], DisassembledItem::Instruction(_)));
        let DisassembledItem::Data(data) = &items[1] else {
            panic!("middle region must remain data");
        };
        assert_eq!(data.address, 0x1004);
        assert_eq!(data.text, ".byte 0xde,0xad,0xbe");
    }

    #[test]
    fn renders_data_regions_as_reassemblable_byte_directives() {
        let bytes: Vec<u8> = (0..17).collect();
        let regions = [DisassemblyRegion {
            offset: 0,
            length: bytes.len(),
            kind: DisassemblyRegionKind::Data,
        }];
        let items = disassemble_regions(&bytes, 0, &regions, &BTreeMap::new()).unwrap();
        assert_eq!(items.len(), 2);
        for item in items {
            let DisassembledItem::Data(data) = item else {
                panic!("data region must not decode as an instruction");
            };
            let rebuilt = luna_assembler::assemble(&data.text).unwrap();
            assert_eq!(rebuilt.text, data.bytes);
        }
    }

    #[test]
    fn rejects_non_contiguous_overlapping_and_out_of_range_regions() {
        let bytes = [0u8; 4];
        let error = disassemble_regions(
            &bytes,
            0,
            &[
                DisassemblyRegion {
                    offset: 0,
                    length: 1,
                    kind: DisassemblyRegionKind::Data,
                },
                DisassemblyRegion {
                    offset: 2,
                    length: 2,
                    kind: DisassemblyRegionKind::Data,
                },
            ],
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, "DISASM-REGION-001");

        let error = disassemble_regions(
            &bytes,
            0,
            &[
                DisassemblyRegion {
                    offset: 0,
                    length: 2,
                    kind: DisassemblyRegionKind::Data,
                },
                DisassemblyRegion {
                    offset: 1,
                    length: 2,
                    kind: DisassemblyRegionKind::Data,
                },
            ],
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, "DISASM-REGION-001");

        let error = disassemble_regions(
            &bytes,
            0,
            &[DisassemblyRegion {
                offset: 0,
                length: 5,
                kind: DisassemblyRegionKind::Data,
            }],
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, "DISASM-REGION-002");
    }
}
