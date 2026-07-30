#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcode {
    pub mnemonic: &'static str,
    pub extension: &'static str,
    pub mask: u32,
    pub match_value: u32,
    pub fields: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/opcode.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Addi {
    pub rd: u8,
    pub rs1: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RType {
    pub rd: u8,
    pub rs1: u8,
    pub rs2: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lui {
    pub rd: u8,
    pub imm20: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Addi(Addi),
    Add(RType),
    Sub(RType),
    Lui(Lui),
    Illegal(u32),
}

fn generated_opcode(mnemonic: &str) -> Result<&'static Opcode> {
    GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == mnemonic)
        .ok_or_else(|| {
            Diagnostic::error(
                "ISA-TABLE-001",
                "instruction is absent from generated R2 tables",
            )
        })
}

fn encode_r_type(mnemonic: &str, instruction: RType) -> Result<u32> {
    if [instruction.rd, instruction.rs1, instruction.rs2]
        .iter()
        .any(|register| *register > 31)
    {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "register out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    Ok(opcode.match_value
        | ((instruction.rs2 as u32) << 20)
        | ((instruction.rs1 as u32) << 15)
        | ((instruction.rd as u32) << 7))
}

pub fn encode_addi(addi: Addi) -> Result<u32> {
    if addi.rd > 31 || addi.rs1 > 31 || !(-2048..=2047).contains(&addi.imm) {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "addi register or immediate out of range",
        ));
    }
    let imm = (addi.imm as i32 as u32) & 0xfff;
    Ok((imm << 20) | ((addi.rs1 as u32) << 15) | ((addi.rd as u32) << 7) | ADDI_MATCH)
}

pub fn encode_r(mnemonic: &str, instruction: RType) -> Result<u32> {
    encode_r_type(mnemonic, instruction)
}

pub fn encode_lui(instruction: Lui) -> Result<u32> {
    if instruction.rd > 31 || instruction.imm20 > 0x000f_ffff {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "lui register or immediate out of range",
        ));
    }
    let opcode = generated_opcode("lui")?;
    let encoded = instruction.imm20 << 12;
    Ok(encoded | ((instruction.rd as u32) << 7) | opcode.match_value)
}

pub fn decode(word: u32) -> Instruction {
    if word & ADDI_MASK == ADDI_MATCH {
        let imm = ((word as i32) >> 20) as i16;
        Instruction::Addi(Addi {
            rd: ((word >> 7) & 31) as u8,
            rs1: ((word >> 15) & 31) as u8,
            imm,
        })
    } else {
        for (mnemonic, instruction) in [
            (
                "add",
                Instruction::Add(RType {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                }),
            ),
            (
                "sub",
                Instruction::Sub(RType {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                }),
            ),
        ] {
            if let Ok(opcode) = generated_opcode(mnemonic) {
                if word & opcode.mask == opcode.match_value {
                    return instruction;
                }
            }
        }
        if let Ok(opcode) = generated_opcode("lui") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::Lui(Lui {
                    rd: ((word >> 7) & 31) as u8,
                    imm20: (word >> 12) & 0x000f_ffff,
                });
            }
        }
        Instruction::Illegal(word)
    }
}

pub fn generated_opcodes() -> &'static [Opcode] {
    GENERATED_OPCODES
}

pub fn encode(instruction: Instruction) -> Result<u32> {
    match instruction {
        Instruction::Addi(addi) => encode_addi(addi),
        Instruction::Add(instruction) => encode_r_type("add", instruction),
        Instruction::Sub(instruction) => encode_r_type("sub", instruction),
        Instruction::Lui(instruction) => encode_lui(instruction),
        Instruction::Illegal(_) => Err(Diagnostic::error(
            "ISA-ENCODE-001",
            "cannot encode an illegal instruction",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_r2_encoding_round_trips() {
        let word = encode_addi(Addi {
            rd: 1,
            rs1: 0,
            imm: 1,
        })
        .unwrap();
        assert_eq!(word, 0x0010_0093);
        assert_eq!(
            decode(word),
            Instruction::Addi(Addi {
                rd: 1,
                rs1: 0,
                imm: 1
            })
        );
    }
    #[test]
    fn signed_immediate_decodes() {
        let word = encode_addi(Addi {
            rd: 2,
            rs1: 3,
            imm: -1,
        })
        .unwrap();
        assert_eq!(
            decode(word),
            Instruction::Addi(Addi {
                rd: 2,
                rs1: 3,
                imm: -1
            })
        );
    }

    #[test]
    fn registry_is_generated_for_the_selected_profile_sources() {
        assert!(generated_opcodes().len() > 100);
        assert!(
            generated_opcodes()
                .iter()
                .any(|opcode| opcode.mnemonic == "add")
        );
        assert!(
            generated_opcodes()
                .iter()
                .any(|opcode| opcode.mnemonic == "fadd.s")
        );
    }

    #[test]
    fn generated_r_type_and_u_type_encodings_round_trip() {
        let add = encode_r(
            "add",
            RType {
                rd: 5,
                rs1: 6,
                rs2: 7,
            },
        )
        .unwrap();
        assert_eq!(
            decode(add),
            Instruction::Add(RType {
                rd: 5,
                rs1: 6,
                rs2: 7
            })
        );
        let lui = encode_lui(Lui {
            rd: 3,
            imm20: 0x12345,
        })
        .unwrap();
        assert_eq!(
            decode(lui),
            Instruction::Lui(Lui {
                rd: 3,
                imm20: 0x12345
            })
        );
        let high = encode_lui(Lui {
            rd: 3,
            imm20: 0xfffff,
        })
        .unwrap();
        assert_eq!(
            decode(high),
            Instruction::Lui(Lui {
                rd: 3,
                imm20: 0xfffff
            })
        );
    }
}
