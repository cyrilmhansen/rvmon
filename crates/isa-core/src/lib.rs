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

#[cfg(test)]
mod tests {
    use super::encode_addi;

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
}
