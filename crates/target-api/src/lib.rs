#![no_std]
#![forbid(unsafe_code)]

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetContext {
    pub x: [u64; 32],
    pub f: [u64; 32],
    pub pc: u64,
    pub fcsr: u32,
    pub mstatus: u64,
    pub mepc: u64,
    pub mcause: u64,
    pub mtval: u64,
}

impl TargetContext {
    pub const fn empty() -> Self {
        Self {
            x: [0; 32],
            f: [0xffff_ffff_0000_0000; 32],
            pc: 0,
            fcsr: 0,
            mstatus: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
        }
    }
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Breakpoint = 1,
    IllegalInstruction = 2,
    InstructionAccessFault = 3,
    LoadAccessFault = 4,
    StoreAccessFault = 5,
    EnvironmentCall = 6,
    UnknownTrap = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    pub address: u64,
    pub original_word: u32,
    pub enabled: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetCapabilities {
    pub xlen: u16,
    pub flen: u16,
    pub supports_f: bool,
    pub supports_d: bool,
    pub supports_compressed: bool,
    pub supports_watchpoints: bool,
    pub hart_count: u16,
}

impl TargetCapabilities {
    pub const RV64_BARE_METAL_V1: Self = Self {
        xlen: 64,
        flen: 64,
        supports_f: true,
        supports_d: true,
        supports_compressed: false,
        supports_watchpoints: false,
        hart_count: 1,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_has_boxed_float_zeroes() {
        let context = TargetContext::empty();
        assert_eq!(context.x, [0; 32]);
        assert_eq!(context.f[0], 0xffff_ffff_0000_0000);
    }

    #[test]
    fn v1_capabilities_are_explicit() {
        assert_eq!(TargetCapabilities::RV64_BARE_METAL_V1.xlen, 64);
        assert!(!TargetCapabilities::RV64_BARE_METAL_V1.supports_compressed);
        assert_eq!(TargetCapabilities::RV64_BARE_METAL_V1.hart_count, 1);
    }
}
