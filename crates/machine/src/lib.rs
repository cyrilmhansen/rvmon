#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_floatfmt::{FloatDisplay, binary64, boxed_binary32};
use luna_isa::{Instruction, decode};
use luna_memory::Memory;
use luna_target_api::{
    ExecutionOutcome, MemoryAccess, MemoryAccessKind, TargetBackend, TargetCapabilities,
    TargetContext,
};

pub const FFLAG_NV: u32 = 1 << 0;
pub const FFLAG_DZ: u32 = 1 << 1;
pub const FFLAG_OF: u32 = 1 << 2;
pub const FFLAG_UF: u32 = 1 << 3;
pub const FFLAG_NX: u32 = 1 << 4;
const SNAPSHOT_MAGIC: &[u8; 8] = b"RVMACH01";
const SNAPSHOT_VERSION: u32 = 1;
const MAX_SNAPSHOT_MEMORY: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub x: [u64; 32],
    pub f: [u64; 32],
    pub fcsr: u32,
    pub pc: u64,
    pub instructions: u64,
    pub memory: Memory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult {
    pub pc_before: u64,
    pub pc_after: u64,
    pub instruction: Instruction,
    pub memory_access: Option<MemoryAccess>,
}

impl Machine {
    pub fn new(memory_size: usize) -> Self {
        Self {
            x: [0; 32],
            f: [0xffff_ffff_0000_0000; 32],
            fcsr: 0,
            pc: 0,
            instructions: 0,
            memory: Memory::new(memory_size),
        }
    }
    pub fn load(&mut self, address: u64, bytes: &[u8]) -> Result<()> {
        for (offset, byte) in bytes.iter().enumerate() {
            self.memory.store8(address + offset as u64, *byte)?;
        }
        Ok(())
    }

    pub fn memory_size(&self) -> usize {
        self.memory.len()
    }

    pub fn fflags(&self) -> u8 {
        (self.fcsr & 0x1f) as u8
    }

    pub fn frm(&self) -> u8 {
        ((self.fcsr >> 5) & 0x7) as u8
    }

    pub fn format_f32(&self, register: u8) -> Result<FloatDisplay> {
        let value = self.f.get(register as usize).ok_or_else(|| {
            Diagnostic::error("FP-REGISTER-001", "floating register out of range")
        })?;
        Ok(boxed_binary32(*value))
    }

    pub fn format_f64(&self, register: u8) -> Result<FloatDisplay> {
        let value = self.f.get(register as usize).ok_or_else(|| {
            Diagnostic::error("FP-REGISTER-001", "floating register out of range")
        })?;
        Ok(binary64(*value))
    }

    pub fn step(&mut self) -> Result<StepResult> {
        let pc_before = self.pc;
        let word = self.memory.load32(self.pc)?;
        let instruction = decode(word);
        let mut memory_access = None;
        match instruction {
            Instruction::Addi(addi) => {
                let value = self.x[addi.rs1 as usize].wrapping_add(addi.imm as i64 as u64);
                if addi.rd != 0 {
                    self.x[addi.rd as usize] = value;
                }
            }
            Instruction::Add(instruction) => {
                let value =
                    self.x[instruction.rs1 as usize].wrapping_add(self.x[instruction.rs2 as usize]);
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = value;
                }
            }
            Instruction::Sub(instruction) => {
                let value =
                    self.x[instruction.rs1 as usize].wrapping_sub(self.x[instruction.rs2 as usize]);
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = value;
                }
            }
            Instruction::Lui(instruction) => {
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] =
                        ((instruction.imm20 << 12) as i32 as i64) as u64;
                }
            }
            Instruction::Auipc(instruction) => {
                if instruction.rd != 0 {
                    let offset = ((instruction.imm20 << 12) as i32 as i64) as u64;
                    self.x[instruction.rd as usize] = pc_before.wrapping_add(offset);
                }
            }
            Instruction::Lw(instruction) => {
                let address =
                    self.x[instruction.rs1 as usize].wrapping_add(instruction.imm as i64 as u64);
                let value = self.memory.load32(address)? as i32 as i64 as u64;
                memory_access = Some(MemoryAccess {
                    kind: MemoryAccessKind::Read,
                    address,
                    width: 4,
                });
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = value;
                }
            }
            Instruction::Sw(instruction) => {
                let address =
                    self.x[instruction.rs1 as usize].wrapping_add(instruction.imm as i64 as u64);
                self.memory
                    .store32(address, self.x[instruction.rs2 as usize] as u32)?;
                memory_access = Some(MemoryAccess {
                    kind: MemoryAccessKind::Write,
                    address,
                    width: 4,
                });
            }
            Instruction::Ld(instruction) => {
                let address =
                    self.x[instruction.rs1 as usize].wrapping_add(instruction.imm as i64 as u64);
                let value = self.memory.load64(address)?;
                memory_access = Some(MemoryAccess {
                    kind: MemoryAccessKind::Read,
                    address,
                    width: 8,
                });
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = value;
                }
            }
            Instruction::Sd(instruction) => {
                let address =
                    self.x[instruction.rs1 as usize].wrapping_add(instruction.imm as i64 as u64);
                self.memory
                    .store64(address, self.x[instruction.rs2 as usize])?;
                memory_access = Some(MemoryAccess {
                    kind: MemoryAccessKind::Write,
                    address,
                    width: 8,
                });
            }
            Instruction::Beq(instruction) => {
                if self.x[instruction.rs1 as usize] == self.x[instruction.rs2 as usize] {
                    self.pc = self.pc.wrapping_add(instruction.imm as i64 as u64);
                } else {
                    self.pc = self.pc.wrapping_add(4);
                }
            }
            Instruction::Bne(instruction) => {
                if self.x[instruction.rs1 as usize] != self.x[instruction.rs2 as usize] {
                    self.pc = self.pc.wrapping_add(instruction.imm as i64 as u64);
                } else {
                    self.pc = self.pc.wrapping_add(4);
                }
            }
            Instruction::Jal(instruction) => {
                let return_pc = self.pc.wrapping_add(4);
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = return_pc;
                }
                self.pc = self.pc.wrapping_add(instruction.imm as i64 as u64);
            }
            Instruction::Jalr(instruction) => {
                let return_pc = self.pc.wrapping_add(4);
                let target = self.x[instruction.rs1 as usize]
                    .wrapping_add(instruction.imm as i64 as u64)
                    & !1;
                if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = return_pc;
                }
                self.pc = target;
            }
            Instruction::FAddS(instruction) => {
                let rounding_mode = if instruction.rm == 7 {
                    self.frm()
                } else {
                    instruction.rm
                };
                if rounding_mode != 0 {
                    return Err(Diagnostic::error(
                        "TRAP-FP-RM-001",
                        "only round-to-nearest-even is implemented for fadd.s",
                    ));
                }
                let left = boxed_f32(self.f[instruction.rs1 as usize]);
                let right = boxed_f32(self.f[instruction.rs2 as usize]);
                let (result, flags) = add_s(left, right);
                self.f[instruction.rd as usize] = 0xffff_ffff_0000_0000 | u64::from(result);
                self.fcsr |= flags;
            }
            Instruction::FAddD(instruction) => {
                let rounding_mode = if instruction.rm == 7 {
                    self.frm()
                } else {
                    instruction.rm
                };
                if rounding_mode != 0 {
                    return Err(Diagnostic::error(
                        "TRAP-FP-RM-001",
                        "only round-to-nearest-even is implemented for fadd.d",
                    ));
                }
                let (result, flags) = add_d(
                    self.f[instruction.rs1 as usize],
                    self.f[instruction.rs2 as usize],
                );
                self.f[instruction.rd as usize] = result;
                self.fcsr |= flags;
            }
            Instruction::Illegal(_) => {
                return Err(Diagnostic::error(
                    "TRAP-ILLEGAL-INSTRUCTION",
                    "illegal instruction",
                ));
            }
        }
        self.x[0] = 0;
        if !matches!(
            instruction,
            Instruction::Beq(_) | Instruction::Bne(_) | Instruction::Jal(_) | Instruction::Jalr(_)
        ) {
            self.pc = self
                .pc
                .checked_add(4)
                .ok_or_else(|| Diagnostic::error("TRAP-PC-OVERFLOW", "program counter overflow"))?;
        }
        self.instructions += 1;
        Ok(StepResult {
            pc_before,
            pc_after: self.pc,
            instruction,
            memory_access,
        })
    }
}

impl TargetBackend for Machine {
    type Error = Diagnostic;

    fn capabilities(&self) -> TargetCapabilities {
        TargetCapabilities::RV64_BARE_METAL_TRACE_V1
    }

    fn context(&self) -> TargetContext {
        TargetContext {
            x: self.x,
            f: self.f,
            pc: self.pc,
            fcsr: self.fcsr,
            mstatus: 0,
            mepc: self.pc,
            mcause: 0,
            mtval: 0,
        }
    }

    fn read_memory(
        &mut self,
        address: u64,
        destination: &mut [u8],
    ) -> std::result::Result<(), Self::Error> {
        for (offset, byte) in destination.iter_mut().enumerate() {
            let address = address
                .checked_add(offset as u64)
                .ok_or_else(|| Diagnostic::error("MEM-ADDRESS-002", "address range overflow"))?;
            *byte = self.memory.load8(address)?;
        }
        Ok(())
    }

    fn write_memory(
        &mut self,
        address: u64,
        source: &[u8],
    ) -> std::result::Result<(), Self::Error> {
        let mut transaction = self.memory.transaction();
        for (offset, byte) in source.iter().enumerate() {
            let address = address
                .checked_add(offset as u64)
                .ok_or_else(|| Diagnostic::error("MEM-ADDRESS-002", "address range overflow"))?;
            transaction.write8(address, *byte);
        }
        self.memory.commit(transaction)
    }

    fn step(&mut self) -> std::result::Result<ExecutionOutcome, Self::Error> {
        let result = Machine::step(self)?;
        Ok(ExecutionOutcome::Retired {
            pc_before: result.pc_before,
            pc_after: result.pc_after,
            memory_access: result.memory_access,
        })
    }

    fn run(&mut self, max_steps: u64) -> std::result::Result<ExecutionOutcome, Self::Error> {
        for _ in 0..max_steps {
            Machine::step(self)?;
        }
        Ok(ExecutionOutcome::BudgetExhausted {
            pc: self.pc,
            instruction_count: self.instructions,
        })
    }

    fn snapshot_size(&self) -> Option<usize> {
        (self.memory.len() <= MAX_SNAPSHOT_MEMORY).then(|| machine_snapshot_size(self.memory.len()))
    }

    fn snapshot(
        &mut self,
        destination: &mut [u8],
    ) -> std::result::Result<Option<usize>, Self::Error> {
        if self.memory.len() > MAX_SNAPSHOT_MEMORY {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-006",
                "snapshot memory exceeds the 64 MiB limit",
            ));
        }
        let size = machine_snapshot_size(self.memory.len());
        if destination.len() < size {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-001",
                "snapshot destination is too small",
            ));
        }
        let mut position = 0;
        destination[position..position + 8].copy_from_slice(SNAPSHOT_MAGIC);
        position += 8;
        put_u32(destination, &mut position, SNAPSHOT_VERSION);
        put_u64(destination, &mut position, self.memory.len() as u64);
        for value in self.x {
            put_u64(destination, &mut position, value);
        }
        for value in self.f {
            put_u64(destination, &mut position, value);
        }
        put_u32(destination, &mut position, self.fcsr);
        put_u64(destination, &mut position, self.pc);
        put_u64(destination, &mut position, self.instructions);
        for address in 0..self.memory.len() {
            destination[position] = self.memory.load8(address as u64)?;
            position += 1;
        }
        Ok(Some(size))
    }

    fn restore_snapshot(&mut self, source: &[u8]) -> std::result::Result<bool, Self::Error> {
        if source.len() < 8 + 4 + 8 {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-002",
                "snapshot is truncated",
            ));
        }
        if &source[..8] != SNAPSHOT_MAGIC {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-003",
                "snapshot magic is invalid",
            ));
        }
        let mut position = 8;
        if take_u32(source, &mut position)? != SNAPSHOT_VERSION {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-004",
                "snapshot version is unsupported",
            ));
        }
        let memory_size = usize::try_from(take_u64(source, &mut position)?).map_err(|_| {
            Diagnostic::error("MACHINE-SNAPSHOT-005", "invalid snapshot memory size")
        })?;
        if memory_size > MAX_SNAPSHOT_MEMORY {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-006",
                "snapshot memory exceeds the 64 MiB limit",
            ));
        }
        let expected = machine_snapshot_size(memory_size);
        if source.len() != expected {
            return Err(Diagnostic::error(
                "MACHINE-SNAPSHOT-007",
                "snapshot length does not match its memory size",
            ));
        }
        let mut x = [0u64; 32];
        let mut f = [0u64; 32];
        for value in &mut x {
            *value = take_u64(source, &mut position)?;
        }
        for value in &mut f {
            *value = take_u64(source, &mut position)?;
        }
        let fcsr = take_u32(source, &mut position)?;
        let pc = take_u64(source, &mut position)?;
        let instructions = take_u64(source, &mut position)?;
        let memory_bytes = &source[position..position + memory_size];
        let mut memory = Memory::new(memory_size);
        for (address, byte) in memory_bytes.iter().copied().enumerate() {
            memory.store8(address as u64, byte)?;
        }
        self.x = x;
        self.f = f;
        self.fcsr = fcsr;
        self.pc = pc;
        self.instructions = instructions;
        self.memory = memory;
        Ok(true)
    }
}

fn machine_snapshot_size(memory_size: usize) -> usize {
    8 + 4 + 8 + (32 * 8) + (32 * 8) + 4 + 8 + 8 + memory_size
}

fn put_u32(destination: &mut [u8], position: &mut usize, value: u32) {
    destination[*position..*position + 4].copy_from_slice(&value.to_le_bytes());
    *position += 4;
}

fn put_u64(destination: &mut [u8], position: &mut usize, value: u64) {
    destination[*position..*position + 8].copy_from_slice(&value.to_le_bytes());
    *position += 8;
}

fn take_u32(source: &[u8], position: &mut usize) -> Result<u32> {
    let end = position
        .checked_add(4)
        .ok_or_else(|| Diagnostic::error("MACHINE-SNAPSHOT-008", "snapshot offset overflow"))?;
    if end > source.len() {
        return Err(Diagnostic::error(
            "MACHINE-SNAPSHOT-002",
            "snapshot is truncated",
        ));
    }
    let value = u32::from_le_bytes(source[*position..end].try_into().unwrap());
    *position = end;
    Ok(value)
}

fn take_u64(source: &[u8], position: &mut usize) -> Result<u64> {
    let end = position
        .checked_add(8)
        .ok_or_else(|| Diagnostic::error("MACHINE-SNAPSHOT-008", "snapshot offset overflow"))?;
    if end > source.len() {
        return Err(Diagnostic::error(
            "MACHINE-SNAPSHOT-002",
            "snapshot is truncated",
        ));
    }
    let value = u64::from_le_bytes(source[*position..end].try_into().unwrap());
    *position = end;
    Ok(value)
}

fn boxed_f32(value: u64) -> u32 {
    if value >> 32 == 0xffff_ffff {
        value as u32
    } else {
        0x7fc0_0000
    }
}

fn add_s(left: u32, right: u32) -> (u32, u32) {
    let left_nan = is_nan(left);
    let right_nan = is_nan(right);
    if (left_nan && is_signaling_nan(left)) || (right_nan && is_signaling_nan(right)) {
        return (0x7fc0_0000, FFLAG_NV);
    }
    if left_nan || right_nan {
        return (0x7fc0_0000, 0);
    }
    if is_infinite(left) && is_infinite(right) && ((left ^ right) & 0x8000_0000 != 0) {
        return (0x7fc0_0000, FFLAG_NV);
    }
    let left_value = f32::from_bits(left);
    let right_value = f32::from_bits(right);
    let result = left_value + right_value;
    if result.is_infinite() && left_value.is_finite() && right_value.is_finite() {
        return (result.to_bits(), FFLAG_OF | FFLAG_NX);
    }
    let inexact = result.is_finite()
        && (result - left_value != right_value || result - right_value != left_value);
    let underflow = inexact && result.abs() < f32::MIN_POSITIVE;
    (
        result.to_bits(),
        if underflow {
            FFLAG_UF | FFLAG_NX
        } else if inexact {
            FFLAG_NX
        } else {
            0
        },
    )
}

fn is_nan(value: u32) -> bool {
    value & 0x7f80_0000 == 0x7f80_0000 && value & 0x007f_ffff != 0
}

fn is_signaling_nan(value: u32) -> bool {
    is_nan(value) && value & 0x0040_0000 == 0
}

fn is_infinite(value: u32) -> bool {
    value & 0x7fff_ffff == 0x7f80_0000
}

fn add_d(left: u64, right: u64) -> (u64, u32) {
    let left_nan = is_nan_d(left);
    let right_nan = is_nan_d(right);
    if (left_nan && is_signaling_nan_d(left)) || (right_nan && is_signaling_nan_d(right)) {
        return (0x7ff8_0000_0000_0000, FFLAG_NV);
    }
    if left_nan || right_nan {
        return (0x7ff8_0000_0000_0000, 0);
    }
    if is_infinite_d(left) && is_infinite_d(right) && ((left ^ right) & (1 << 63) != 0) {
        return (0x7ff8_0000_0000_0000, FFLAG_NV);
    }
    let left_value = f64::from_bits(left);
    let right_value = f64::from_bits(right);
    let result = left_value + right_value;
    if result.is_infinite() && left_value.is_finite() && right_value.is_finite() {
        return (result.to_bits(), FFLAG_OF | FFLAG_NX);
    }
    let inexact = result.is_finite()
        && (result - left_value != right_value || result - right_value != left_value);
    let underflow = inexact && result.abs() < f64::MIN_POSITIVE;
    (
        result.to_bits(),
        if underflow {
            FFLAG_UF | FFLAG_NX
        } else if inexact {
            FFLAG_NX
        } else {
            0
        },
    )
}

fn is_nan_d(value: u64) -> bool {
    value & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000 && value & 0x000f_ffff_ffff_ffff != 0
}

fn is_signaling_nan_d(value: u64) -> bool {
    is_nan_d(value) && value & 0x0008_0000_0000_0000 == 0
}

fn is_infinite_d(value: u64) -> bool {
    value & 0x7fff_ffff_ffff_ffff == 0x7ff0_0000_0000_0000
}

#[cfg(test)]
mod tests {
    use super::*;
    use luna_isa::{Addi, Lui, RType, encode_addi, encode_lui, encode_r, encode_u};

    #[test]
    fn executes_generated_integer_instructions() {
        let mut machine = Machine::new(64);
        machine.x[6] = 40;
        machine.x[7] = 2;
        let words = [
            encode_addi(Addi {
                rd: 5,
                rs1: 0,
                imm: 1,
            })
            .unwrap(),
            encode_r(
                "add",
                RType {
                    rd: 8,
                    rs1: 6,
                    rs2: 7,
                },
            )
            .unwrap(),
            encode_r(
                "sub",
                RType {
                    rd: 9,
                    rs1: 6,
                    rs2: 7,
                },
            )
            .unwrap(),
            encode_lui(Lui { rd: 10, imm20: 1 }).unwrap(),
            encode_u("auipc", Lui { rd: 11, imm20: 1 }).unwrap(),
        ];
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        machine.load(0, &bytes).unwrap();
        for _ in words {
            machine.step().unwrap();
        }
        assert_eq!(machine.x[5], 1);
        assert_eq!(machine.x[8], 42);
        assert_eq!(machine.x[9], 38);
        assert_eq!(machine.x[10], 0x1000);
        assert_eq!(machine.x[11], 0x1000 + (words.len() as u64 - 1) * 4);
    }

    #[test]
    fn executes_lw_and_sw_with_sign_extension() {
        let mut machine = Machine::new(128);
        machine.x[4] = 64;
        machine.x[3] = 0xffff_ffff_8000_0001;
        machine.x[5] = 0xdead_beef_dead_beef;
        let words = [
            luna_isa::encode_store(
                "sw",
                luna_isa::Store {
                    rs2: 3,
                    rs1: 4,
                    imm: 0,
                },
            )
            .unwrap(),
            luna_isa::encode_load(
                "lw",
                luna_isa::Load {
                    rd: 5,
                    rs1: 4,
                    imm: 0,
                },
            )
            .unwrap(),
        ];
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        machine.load(0, &bytes).unwrap();
        machine.step().unwrap();
        machine.step().unwrap();
        assert_eq!(machine.memory.load32(64).unwrap(), 0x8000_0001);
        assert_eq!(machine.x[5], 0xffff_ffff_8000_0001);
    }

    #[test]
    fn loads_an_ilp32_pointer_table_with_four_byte_stride() {
        let mut machine = Machine::new(128);
        machine.x[10] = 64;
        let pointers = [0x0000_0000, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
        for (index, pointer) in pointers.iter().enumerate() {
            machine
                .memory
                .store32(64 + (index as u64 * 4), *pointer)
                .unwrap();
        }
        let words = [
            luna_isa::encode_load(
                "lw",
                luna_isa::Load {
                    rd: 5,
                    rs1: 10,
                    imm: 0,
                },
            )
            .unwrap(),
            luna_isa::encode_load(
                "lw",
                luna_isa::Load {
                    rd: 6,
                    rs1: 10,
                    imm: 4,
                },
            )
            .unwrap(),
            luna_isa::encode_load(
                "lw",
                luna_isa::Load {
                    rd: 7,
                    rs1: 10,
                    imm: 8,
                },
            )
            .unwrap(),
            luna_isa::encode_load(
                "lw",
                luna_isa::Load {
                    rd: 8,
                    rs1: 10,
                    imm: 12,
                },
            )
            .unwrap(),
        ];
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        machine.load(0, &bytes).unwrap();
        machine.x[5] = 0xdead_beef_dead_beef;
        machine.x[6] = 0x0123_4567_89ab_cdef;
        machine.x[7] = 0xaaaa_aaaa_aaaa_aaaa;
        machine.x[8] = 0x5555_5555_5555_5555;

        for _ in words {
            machine.step().unwrap();
        }

        assert_eq!(machine.x[5], 0x0000_0000_0000_0000);
        assert_eq!(machine.x[6], 0x0000_0000_7fff_ffff);
        assert_eq!(machine.x[7], 0xffff_ffff_8000_0000);
        assert_eq!(machine.x[8], 0xffff_ffff_ffff_ffff);
    }

    #[test]
    fn executes_branch_and_jump_pc_rules() {
        let mut machine = Machine::new(128);
        machine.x[1] = 7;
        machine.x[2] = 7;
        let branch = luna_isa::encode_branch(
            "beq",
            luna_isa::Branch {
                rs1: 1,
                rs2: 2,
                imm: 8,
            },
        )
        .unwrap();
        machine.load(0, &branch.to_le_bytes()).unwrap();
        assert_eq!(machine.step().unwrap().pc_after, 8);
        let jump = luna_isa::encode_jal(luna_isa::Jal { rd: 5, imm: 4 }).unwrap();
        machine.load(8, &jump.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_eq!(machine.x[5], 12);
        assert_eq!(machine.pc, 12);
    }

    #[test]
    fn executes_ld_and_sd_with_eight_byte_memory_events() {
        let mut machine = Machine::new(128);
        machine.x[4] = 64;
        machine.x[3] = 0x0102_0304_0506_0708;
        let store = luna_isa::encode_store(
            "sd",
            luna_isa::Store {
                rs2: 3,
                rs1: 4,
                imm: 8,
            },
        )
        .unwrap();
        let load = luna_isa::encode_load(
            "ld",
            luna_isa::Load {
                rd: 5,
                rs1: 4,
                imm: 8,
            },
        )
        .unwrap();
        machine.load(0, &store.to_le_bytes()).unwrap();
        machine.load(4, &load.to_le_bytes()).unwrap();
        assert_eq!(machine.step().unwrap().memory_access.unwrap().width, 8);
        assert_eq!(machine.step().unwrap().memory_access.unwrap().width, 8);
        assert_eq!(machine.x[5], machine.x[3]);
        assert_eq!(machine.memory.load8(72).unwrap(), 0x08);
    }

    #[test]
    fn executes_fadd_s_with_boxed_values_and_sticky_flags() {
        let mut machine = Machine::new(64);
        machine.f[1] = 0xffff_ffff_3fc0_0000;
        machine.f[2] = 0xffff_ffff_4010_0000;
        let word = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_eq!(machine.f[3], 0xffff_ffff_4070_0000);
        assert_eq!(machine.fflags(), 0);
    }

    #[test]
    fn fadd_s_sets_inexact_and_invalid_flags() {
        let mut machine = Machine::new(128);
        machine.f[1] = 0xffff_ffff_3f80_0000;
        machine.f[2] = 0xffff_ffff_0000_0001;
        let inexact = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(0, &inexact.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_ne!(machine.fflags() & FFLAG_NX as u8, 0);

        machine.pc = 4;
        machine.f[1] = 0xffff_ffff_7f80_0000;
        machine.f[2] = 0xffff_ffff_ff80_0000;
        let invalid = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(4, &invalid.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_ne!(machine.fflags() & FFLAG_NV as u8, 0);
        assert_eq!(machine.f[3] as u32, 0x7fc0_0000);
    }

    #[test]
    fn invalid_nan_box_is_quietly_canonicalized() {
        let mut machine = Machine::new(64);
        machine.f[1] = 0x0000_0000_3f80_0000;
        machine.f[2] = 0xffff_ffff_3f80_0000;
        let word = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_eq!(machine.f[3] as u32, 0x7fc0_0000);
        assert_eq!(machine.fflags(), 0);
    }

    #[test]
    fn executes_fadd_d_and_formats_exact_register_bits() {
        let mut machine = Machine::new(64);
        machine.f[1] = 1.5f64.to_bits();
        machine.f[2] = 2.25f64.to_bits();
        let word = luna_isa::encode_f_r(
            "fadd.d",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_eq!(machine.f[3], 3.75f64.to_bits());
        assert_eq!(
            machine.format_f64(3).unwrap().exact_hex,
            "0x400e000000000000"
        );
        assert_eq!(machine.format_f64(3).unwrap().shortest_decimal, "3.75");
        assert_eq!(
            machine.format_f32(3).unwrap().class,
            luna_floatfmt::FloatClass::InvalidBox
        );
    }

    #[test]
    fn fadd_d_sets_inexact_and_invalid_flags() {
        let mut machine = Machine::new(128);
        machine.f[1] = 1.0f64.to_bits();
        machine.f[2] = 0x0000_0000_0000_0001;
        let word = luna_isa::encode_f_r(
            "fadd.d",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_ne!(machine.fflags() & FFLAG_NX as u8, 0);

        machine.pc = 4;
        machine.f[1] = f64::INFINITY.to_bits();
        machine.f[2] = f64::NEG_INFINITY.to_bits();
        machine.load(4, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        assert_ne!(machine.fflags() & FFLAG_NV as u8, 0);
        assert_eq!(machine.f[3], 0x7ff8_0000_0000_0000);
    }

    #[test]
    fn target_backend_exposes_context_and_transactional_bytes() {
        let mut machine = Machine::new(16);
        machine.pc = 4;
        machine.x[1] = 0x8000_0000;
        machine.fcsr = 0x1f;

        let context = TargetBackend::context(&machine);
        assert_eq!(context.pc, 4);
        assert_eq!(context.mepc, 4);
        assert_eq!(context.x[1], 0x8000_0000);
        assert_eq!(context.fcsr, 0x1f);

        TargetBackend::write_memory(&mut machine, 2, &[0xaa, 0xbb, 0xcc]).unwrap();
        let mut bytes = [0; 3];
        TargetBackend::read_memory(&mut machine, 2, &mut bytes).unwrap();
        assert_eq!(bytes, [0xaa, 0xbb, 0xcc]);

        assert!(TargetBackend::write_memory(&mut machine, 15, &[1, 2]).is_err());
        assert_eq!(machine.memory.load8(15).unwrap(), 0);
    }

    #[test]
    fn target_backend_step_and_run_report_contractual_outcomes() {
        let mut machine = Machine::new(32);
        let addi = luna_isa::encode_addi(luna_isa::Addi {
            rd: 1,
            rs1: 0,
            imm: 1,
        })
        .unwrap();
        machine.load(0, &addi.to_le_bytes()).unwrap();
        machine.load(4, &addi.to_le_bytes()).unwrap();
        machine.load(8, &addi.to_le_bytes()).unwrap();

        assert_eq!(
            TargetBackend::step(&mut machine).unwrap(),
            ExecutionOutcome::Retired {
                pc_before: 0,
                pc_after: 4,
                memory_access: None,
            }
        );
        assert_eq!(machine.x[1], 1);
        assert_eq!(
            TargetBackend::run(&mut machine, 2).unwrap(),
            ExecutionOutcome::BudgetExhausted {
                pc: 12,
                instruction_count: 3,
            }
        );
    }

    #[test]
    fn target_backend_snapshot_roundtrips_complete_machine_state() {
        let mut machine = Machine::new(32);
        machine.pc = 8;
        machine.instructions = 17;
        machine.x[1] = 0x8000_0000;
        machine.f[2] = 0x3ff0_0000_0000_0000;
        machine.fcsr = 0x1f;
        machine.memory.store8(12, 0xa5).unwrap();

        let size = TargetBackend::snapshot_size(&machine).unwrap();
        let mut bytes = vec![0u8; size];
        assert_eq!(
            TargetBackend::snapshot(&mut machine, &mut bytes).unwrap(),
            Some(size)
        );

        let mut restored = Machine::new(1);
        assert!(TargetBackend::restore_snapshot(&mut restored, &bytes).unwrap());
        assert_eq!(restored, machine);
    }
}
