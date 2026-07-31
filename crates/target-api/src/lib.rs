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
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAccessFault = 5,
    StoreAccessFault = 7,
    EnvironmentCall = 8,
    UnknownTrap = 255,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopEvent {
    pub reason: StopReason,
    pub pc: u64,
    pub instruction_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryAccessKind {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAccess {
    pub kind: MemoryAccessKind,
    pub address: u64,
    pub width: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Retired {
        pc_before: u64,
        pc_after: u64,
        memory_access: Option<MemoryAccess>,
    },
    Stopped(StopEvent),
    BudgetExhausted {
        pc: u64,
        instruction_count: u64,
    },
}

/// Stable boundary between the monitor/debugger and a target implementation.
///
/// The host simulator and a future QEMU/GDB transport implement the same
/// contract. The trait deliberately exposes target-shaped state and byte
/// access, not a host process, socket, or UI representation.
pub trait TargetBackend {
    type Error;

    fn capabilities(&self) -> TargetCapabilities;
    fn context(&self) -> TargetContext;
    fn read_memory(&self, address: u64, destination: &mut [u8]) -> Result<(), Self::Error>;
    fn write_memory(&mut self, address: u64, source: &[u8]) -> Result<(), Self::Error>;
    fn step(&mut self) -> Result<ExecutionOutcome, Self::Error>;
    fn run(&mut self, max_steps: u64) -> Result<ExecutionOutcome, Self::Error>;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    pub address: u64,
    pub original_word: u32,
    pub enabled: bool,
}

impl Breakpoint {
    pub const fn disabled() -> Self {
        Self {
            address: 0,
            original_word: 0,
            enabled: false,
        }
    }
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

    #[test]
    fn stop_reasons_match_risc_v_exception_codes() {
        assert_eq!(StopReason::InstructionAccessFault as u64, 1);
        assert_eq!(StopReason::Breakpoint as u64, 3);
        assert_eq!(StopReason::EnvironmentCall as u64, 8);
    }

    #[test]
    fn context_layout_matches_trap_assembly_contract() {
        assert_eq!(core::mem::size_of::<TargetContext>(), 560);
        assert_eq!(core::mem::align_of::<TargetContext>(), 8);
    }
}
