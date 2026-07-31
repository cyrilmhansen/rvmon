#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

pub use luna_isa_core::{ADDI_MASK, ADDI_MATCH, GENERATED_OPCODES, Opcode};

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
pub struct Load {
    pub rd: u8,
    pub rs1: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Store {
    pub rs2: u8,
    pub rs1: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Branch {
    pub rs1: u8,
    pub rs2: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jal {
    pub rd: u8,
    pub imm: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jalr {
    pub rd: u8,
    pub rs1: u8,
    pub imm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FRegisterRType {
    pub rd: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub rm: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Addi(Addi),
    Add(RType),
    Sub(RType),
    Lui(Lui),
    Auipc(Lui),
    Lw(Load),
    Sw(Store),
    Ld(Load),
    Sd(Store),
    Beq(Branch),
    Bne(Branch),
    Jal(Jal),
    Jalr(Jalr),
    FAddS(FRegisterRType),
    FAddD(FRegisterRType),
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
    luna_isa_core::encode_addi(addi.rd, addi.rs1, addi.imm).ok_or_else(|| {
        Diagnostic::error("ISA-OPERAND-001", "addi register or immediate out of range")
    })
}

pub fn encode_r(mnemonic: &str, instruction: RType) -> Result<u32> {
    encode_r_type(mnemonic, instruction)
}

pub fn encode_f_r(mnemonic: &str, instruction: FRegisterRType) -> Result<u32> {
    if [instruction.rd, instruction.rs1, instruction.rs2]
        .iter()
        .any(|register| *register > 31)
        || instruction.rm > 7
    {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "floating register or rounding mode out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    Ok(opcode.match_value
        | ((instruction.rs2 as u32) << 20)
        | ((instruction.rs1 as u32) << 15)
        | ((instruction.rm as u32) << 12)
        | ((instruction.rd as u32) << 7))
}

pub fn encode_u(mnemonic: &str, instruction: Lui) -> Result<u32> {
    if instruction.rd > 31 || instruction.imm20 > 0x000f_ffff {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "U-type register or immediate out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    let encoded = instruction.imm20 << 12;
    Ok(encoded | ((instruction.rd as u32) << 7) | opcode.match_value)
}

pub fn encode_lui(instruction: Lui) -> Result<u32> {
    encode_u("lui", instruction)
}

pub fn encode_load(mnemonic: &str, instruction: Load) -> Result<u32> {
    if instruction.rd > 31 || instruction.rs1 > 31 || !(-2048..=2047).contains(&instruction.imm) {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "load register or immediate out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    let imm = (instruction.imm as i32 as u32) & 0xfff;
    Ok((imm << 20)
        | ((instruction.rs1 as u32) << 15)
        | ((instruction.rd as u32) << 7)
        | opcode.match_value)
}

pub fn encode_store(mnemonic: &str, instruction: Store) -> Result<u32> {
    if instruction.rs2 > 31 || instruction.rs1 > 31 || !(-2048..=2047).contains(&instruction.imm) {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "store register or immediate out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    let imm = (instruction.imm as i32 as u32) & 0xfff;
    Ok(((imm >> 5) << 25)
        | ((instruction.rs2 as u32) << 20)
        | ((instruction.rs1 as u32) << 15)
        | ((imm & 0x1f) << 7)
        | opcode.match_value)
}

pub fn encode_branch(mnemonic: &str, instruction: Branch) -> Result<u32> {
    if instruction.rs1 > 31
        || instruction.rs2 > 31
        || !(-4096..=4094).contains(&instruction.imm)
        || instruction.imm % 2 != 0
    {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "branch register or aligned immediate out of range",
        ));
    }
    let opcode = generated_opcode(mnemonic)?;
    let imm = instruction.imm as i32 as u32;
    Ok(((imm >> 12) << 31)
        | (((imm >> 5) & 0x3f) << 25)
        | ((instruction.rs2 as u32) << 20)
        | ((instruction.rs1 as u32) << 15)
        | opcode.match_value
        | (((imm >> 1) & 0xf) << 8)
        | (((imm >> 11) & 1) << 7))
}

pub fn encode_jal(instruction: Jal) -> Result<u32> {
    if instruction.rd > 31
        || !(-1_048_576..=1_048_574).contains(&instruction.imm)
        || instruction.imm % 2 != 0
    {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "jal register or aligned immediate out of range",
        ));
    }
    let opcode = generated_opcode("jal")?;
    let imm = instruction.imm as i64 as u32;
    Ok((((imm >> 20) & 1) << 31)
        | (((imm >> 1) & 0x3ff) << 21)
        | (((imm >> 11) & 1) << 20)
        | (((imm >> 12) & 0xff) << 12)
        | ((instruction.rd as u32) << 7)
        | opcode.match_value)
}

pub fn encode_jalr(instruction: Jalr) -> Result<u32> {
    if instruction.rd > 31 || instruction.rs1 > 31 || !(-2048..=2047).contains(&instruction.imm) {
        return Err(Diagnostic::error(
            "ISA-OPERAND-001",
            "jalr register or immediate out of range",
        ));
    }
    let opcode = generated_opcode("jalr")?;
    let imm = (instruction.imm as i32 as u32) & 0xfff;
    Ok((imm << 20)
        | ((instruction.rs1 as u32) << 15)
        | ((instruction.rd as u32) << 7)
        | opcode.match_value)
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
        if let Ok(opcode) = generated_opcode("auipc") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::Auipc(Lui {
                    rd: ((word >> 7) & 31) as u8,
                    imm20: (word >> 12) & 0x000f_ffff,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("lw") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::Lw(Load {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    imm: (word as i32 >> 20) as i16,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("sw") {
            if word & opcode.mask == opcode.match_value {
                let immediate = (((word >> 25) & 0x7f) << 5) | ((word >> 7) & 0x1f);
                return Instruction::Sw(Store {
                    rs2: ((word >> 20) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    imm: ((immediate as i32) << 20 >> 20) as i16,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("ld") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::Ld(Load {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    imm: (word as i32 >> 20) as i16,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("sd") {
            if word & opcode.mask == opcode.match_value {
                let immediate = (((word >> 25) & 0x7f) << 5) | ((word >> 7) & 0x1f);
                return Instruction::Sd(Store {
                    rs2: ((word >> 20) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    imm: ((immediate as i32) << 20 >> 20) as i16,
                });
            }
        }
        let branch_imm = (((word >> 31) & 1) << 12)
            | (((word >> 25) & 0x3f) << 5)
            | (((word >> 8) & 0xf) << 1)
            | (((word >> 7) & 1) << 11);
        let branch_imm = ((branch_imm as i32) << 19 >> 19) as i16;
        for (mnemonic, instruction) in [
            (
                "beq",
                Instruction::Beq(Branch {
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                    imm: branch_imm,
                }),
            ),
            (
                "bne",
                Instruction::Bne(Branch {
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                    imm: branch_imm,
                }),
            ),
        ] {
            if let Ok(opcode) = generated_opcode(mnemonic) {
                if word & opcode.mask == opcode.match_value {
                    return instruction;
                }
            }
        }
        if let Ok(opcode) = generated_opcode("jal") {
            if word & opcode.mask == opcode.match_value {
                let imm = ((((word >> 31) & 1) << 20
                    | ((word >> 21) & 0x3ff) << 1
                    | ((word >> 20) & 1) << 11
                    | ((word >> 12) & 0xff) << 12) as i32)
                    << 11
                    >> 11;
                return Instruction::Jal(Jal {
                    rd: ((word >> 7) & 31) as u8,
                    imm,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("jalr") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::Jalr(Jalr {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    imm: (word as i32 >> 20) as i16,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("fadd.s") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::FAddS(FRegisterRType {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                    rm: ((word >> 12) & 7) as u8,
                });
            }
        }
        if let Ok(opcode) = generated_opcode("fadd.d") {
            if word & opcode.mask == opcode.match_value {
                return Instruction::FAddD(FRegisterRType {
                    rd: ((word >> 7) & 31) as u8,
                    rs1: ((word >> 15) & 31) as u8,
                    rs2: ((word >> 20) & 31) as u8,
                    rm: ((word >> 12) & 7) as u8,
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
        Instruction::Auipc(instruction) => encode_u("auipc", instruction),
        Instruction::Lw(instruction) => encode_load("lw", instruction),
        Instruction::Sw(instruction) => encode_store("sw", instruction),
        Instruction::Ld(instruction) => encode_load("ld", instruction),
        Instruction::Sd(instruction) => encode_store("sd", instruction),
        Instruction::Beq(instruction) => encode_branch("beq", instruction),
        Instruction::Bne(instruction) => encode_branch("bne", instruction),
        Instruction::Jal(instruction) => encode_jal(instruction),
        Instruction::Jalr(instruction) => encode_jalr(instruction),
        Instruction::FAddS(instruction) => encode_f_r("fadd.s", instruction),
        Instruction::FAddD(instruction) => encode_f_r("fadd.d", instruction),
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
    fn fadd_s_encoding_round_trips_with_dynamic_rounding() {
        let instruction = FRegisterRType {
            rd: 1,
            rs1: 2,
            rs2: 3,
            rm: 7,
        };
        let word = encode_f_r("fadd.s", instruction).unwrap();
        assert_eq!(decode(word), Instruction::FAddS(instruction));
    }

    #[test]
    fn fadd_d_encoding_round_trips_with_dynamic_rounding() {
        let instruction = FRegisterRType {
            rd: 4,
            rs1: 5,
            rs2: 6,
            rm: 7,
        };
        let word = encode_f_r("fadd.d", instruction).unwrap();
        assert_eq!(decode(word), Instruction::FAddD(instruction));
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
        let lw = encode_load(
            "lw",
            Load {
                rd: 4,
                rs1: 5,
                imm: -8,
            },
        )
        .unwrap();
        assert_eq!(
            decode(lw),
            Instruction::Lw(Load {
                rd: 4,
                rs1: 5,
                imm: -8
            })
        );
        let sw = encode_store(
            "sw",
            Store {
                rs2: 4,
                rs1: 5,
                imm: -8,
            },
        )
        .unwrap();
        assert_eq!(
            decode(sw),
            Instruction::Sw(Store {
                rs2: 4,
                rs1: 5,
                imm: -8
            })
        );
    }
}
