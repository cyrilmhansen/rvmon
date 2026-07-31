#![no_std]
#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcode {
    pub mnemonic: &'static str,
    pub extension: &'static str,
    pub mask: u32,
    pub match_value: u32,
    pub fields: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/opcode.rs"));

/// Encodes the first guest-supported instruction without allocating or
/// constructing a host diagnostic. The host wrapper converts `None` into its
/// stable diagnostic type.
pub const fn encode_addi(rd: u8, rs1: u8, imm: i16) -> Option<u32> {
    if rd > 31 || rs1 > 31 || imm < -2048 || imm > 2047 {
        return None;
    }
    let immediate = (imm as i32 as u32) & 0xfff;
    Some((immediate << 20) | ((rs1 as u32) << 15) | ((rd as u32) << 7) | ADDI_MATCH)
}

pub fn encode_branch(mnemonic: &str, rs1: u8, rs2: u8, imm: i16) -> Option<u32> {
    if rs1 > 31 || rs2 > 31 || !(-4096..=4094).contains(&imm) || imm % 2 != 0 {
        return None;
    }
    let opcode = GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == mnemonic)?;
    let immediate = imm as i32 as u32;
    Some(
        (((immediate >> 12) & 1) << 31)
            | (((immediate >> 5) & 0x3f) << 25)
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | opcode.match_value
            | (((immediate >> 1) & 0xf) << 8)
            | (((immediate >> 11) & 1) << 7),
    )
}

pub fn encode_jal(rd: u8, imm: i32) -> Option<u32> {
    if rd > 31 || !(-1_048_576..=1_048_574).contains(&imm) || imm % 2 != 0 {
        return None;
    }
    let opcode = GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == "jal")?;
    let immediate = imm as i64 as u32;
    Some(
        (((immediate >> 20) & 1) << 31)
            | (((immediate >> 1) & 0x3ff) << 21)
            | (((immediate >> 11) & 1) << 20)
            | (((immediate >> 12) & 0xff) << 12)
            | ((rd as u32) << 7)
            | opcode.match_value,
    )
}

pub fn encode_jalr(rd: u8, rs1: u8, imm: i16) -> Option<u32> {
    if rd > 31 || rs1 > 31 || !(-2048..=2047).contains(&imm) {
        return None;
    }
    let opcode = GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == "jalr")?;
    let immediate = (imm as i32 as u32) & 0xfff;
    Some((immediate << 20) | ((rs1 as u32) << 15) | ((rd as u32) << 7) | opcode.match_value)
}

pub fn encode_f_r(mnemonic: &str, rd: u8, rs1: u8, rs2: u8, rm: u8) -> Option<u32> {
    if rd > 31 || rs1 > 31 || rs2 > 31 || rm > 7 {
        return None;
    }
    let opcode = GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == mnemonic)?;
    Some(
        opcode.match_value
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | ((rm as u32) << 12)
            | ((rd as u32) << 7),
    )
}

#[cfg(test)]
mod tests {
    use super::{encode_addi, encode_branch, encode_f_r, encode_jal, encode_jalr};

    #[test]
    fn encodes_the_guest_first_instruction_without_allocation() {
        assert_eq!(encode_addi(1, 0, 1), Some(0x0010_0093));
    }

    #[test]
    fn rejects_addi_operands_outside_the_isa_range() {
        assert_eq!(encode_addi(32, 0, 1), None);
        assert_eq!(encode_addi(1, 0, 2048), None);
        assert_eq!(encode_addi(1, 0, -2049), None);
    }

    #[test]
    fn encodes_control_flow_from_generated_opcodes() {
        assert_eq!(encode_branch("beq", 1, 1, 8), Some(0x0010_8463));
        assert_eq!(encode_branch("bne", 1, 2, -8), Some(0xfe20_9ce3));
        assert_eq!(encode_jal(0, 12), Some(0x00c0_006f));
        assert_eq!(encode_jalr(0, 1, 0), Some(0x0000_8067));
        assert_eq!(encode_f_r("fadd.s", 3, 1, 2, 0), Some(0x0020_81d3));
        assert_eq!(encode_f_r("fadd.d", 6, 4, 5, 7), Some(0x0252_7353));
    }
}
