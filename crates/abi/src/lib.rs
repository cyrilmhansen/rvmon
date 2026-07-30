#![forbid(unsafe_code)]

/// Converts an ILP32 logical pointer to its RV64 register representation.
pub const fn sign_extend_pointer(value: u32) -> u64 {
    (value as i32 as i64) as u64
}

pub const LOW_POINTER_MAX: u32 = 0x7fff_ffff;
pub const HIGH_POINTER_MIN: u32 = 0x8000_0000;

pub const fn is_low_pointer(value: u32) -> bool {
    value <= LOW_POINTER_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_boundaries_are_sign_extended() {
        assert_eq!(sign_extend_pointer(0x7fff_ffff), 0x0000_0000_7fff_ffff);
        assert_eq!(sign_extend_pointer(0x8000_0000), 0xffff_ffff_8000_0000);
        assert_eq!(sign_extend_pointer(0xffff_ffff), 0xffff_ffff_ffff_ffff);
    }

    #[test]
    fn sign_extension_is_idempotent() {
        for value in [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
            let extended = sign_extend_pointer(value);
            assert_eq!(extended, sign_extend_pointer(extended as u32));
        }
    }
}
