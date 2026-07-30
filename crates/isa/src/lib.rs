#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

include!(concat!(env!("OUT_DIR"), "/opcode.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Addi {
    pub rd: u8,
    pub rs1: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Addi(Addi),
    Illegal(u32),
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

pub fn decode(word: u32) -> Instruction {
    if word & ADDI_MASK == ADDI_MATCH {
        let imm = ((word as i32) >> 20) as i16;
        Instruction::Addi(Addi {
            rd: ((word >> 7) & 31) as u8,
            rs1: ((word >> 15) & 31) as u8,
            imm,
        })
    } else {
        Instruction::Illegal(word)
    }
}

pub fn encode(instruction: Instruction) -> Result<u32> {
    match instruction {
        Instruction::Addi(addi) => encode_addi(addi),
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
}
