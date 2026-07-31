#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_floatfmt::{FloatDisplay, binary64, boxed_binary32};
use luna_isa::FloatMoveKind;
use luna_isa::{FloatConversionKind, Instruction, decode};
use luna_memory::Memory;
use luna_target_api::{
    ExecutionOutcome, MemoryAccess, MemoryAccessKind, TargetBackend, TargetCapabilities,
    TargetContext,
};

pub const FFLAG_NX: u32 = 1 << 0;
pub const FFLAG_UF: u32 = 1 << 1;
pub const FFLAG_OF: u32 = 1 << 2;
pub const FFLAG_DZ: u32 = 1 << 3;
pub const FFLAG_NV: u32 = 1 << 4;
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
                if !is_valid_rounding_mode(rounding_mode) {
                    return Err(Diagnostic::error(
                        "TRAP-FP-RM-001",
                        "reserved floating-point rounding mode",
                    ));
                }
                let left = boxed_f32(self.f[instruction.rs1 as usize]);
                let right = boxed_f32(self.f[instruction.rs2 as usize]);
                let (result, flags) = add_s(left, right, rounding_mode);
                self.f[instruction.rd as usize] = 0xffff_ffff_0000_0000 | u64::from(result);
                self.fcsr |= flags;
            }
            Instruction::FAddD(instruction) => {
                let rounding_mode = if instruction.rm == 7 {
                    self.frm()
                } else {
                    instruction.rm
                };
                if !is_valid_rounding_mode(rounding_mode) {
                    return Err(Diagnostic::error(
                        "TRAP-FP-RM-001",
                        "reserved floating-point rounding mode",
                    ));
                }
                let (result, flags) = add_d(
                    self.f[instruction.rs1 as usize],
                    self.f[instruction.rs2 as usize],
                    rounding_mode,
                );
                self.f[instruction.rd as usize] = result;
                self.fcsr |= flags;
            }
            Instruction::FloatMove(instruction) => match instruction.kind {
                FloatMoveKind::XFromW => {
                    if instruction.rd != 0 {
                        self.x[instruction.rd as usize] =
                            (self.f[instruction.rs1 as usize] as u32 as i32 as i64) as u64;
                    }
                }
                FloatMoveKind::WFromX => {
                    self.f[instruction.rd as usize] =
                        0xffff_ffff_0000_0000 | (self.x[instruction.rs1 as usize] & 0xffff_ffff);
                }
                FloatMoveKind::XFromD => {
                    if instruction.rd != 0 {
                        self.x[instruction.rd as usize] = self.f[instruction.rs1 as usize];
                    }
                }
                FloatMoveKind::DFromX => {
                    self.f[instruction.rd as usize] = self.x[instruction.rs1 as usize];
                }
            },
            Instruction::FloatConversion(instruction) => {
                let rounding_mode = if instruction.rm == 7 {
                    self.frm()
                } else {
                    instruction.rm
                };
                if !is_valid_rounding_mode(rounding_mode) {
                    return Err(Diagnostic::error(
                        "TRAP-FP-RM-001",
                        "reserved floating-point rounding mode",
                    ));
                }
                let (result, flags) = match instruction.kind {
                    FloatConversionKind::SFromD => {
                        let (result, flags) = convert_binary(
                            self.f[instruction.rs1 as usize],
                            FORMAT_D,
                            FORMAT_S,
                            rounding_mode,
                        );
                        (0xffff_ffff_0000_0000 | (result & 0xffff_ffff), flags)
                    }
                    FloatConversionKind::DFromS => convert_binary(
                        boxed_f32(self.f[instruction.rs1 as usize]) as u64,
                        FORMAT_S,
                        FORMAT_D,
                        rounding_mode,
                    ),
                    FloatConversionKind::WFromS | FloatConversionKind::WuFromS => {
                        convert_binary_to_integer(
                            boxed_f32(self.f[instruction.rs1 as usize]) as u64,
                            FORMAT_S,
                            matches!(instruction.kind, FloatConversionKind::WFromS),
                            32,
                            rounding_mode,
                        )
                    }
                    FloatConversionKind::WFromD | FloatConversionKind::WuFromD => {
                        convert_binary_to_integer(
                            self.f[instruction.rs1 as usize],
                            FORMAT_D,
                            matches!(instruction.kind, FloatConversionKind::WFromD),
                            32,
                            rounding_mode,
                        )
                    }
                    FloatConversionKind::LFromS | FloatConversionKind::LuFromS => {
                        convert_binary_to_integer(
                            boxed_f32(self.f[instruction.rs1 as usize]) as u64,
                            FORMAT_S,
                            matches!(instruction.kind, FloatConversionKind::LFromS),
                            64,
                            rounding_mode,
                        )
                    }
                    FloatConversionKind::LFromD | FloatConversionKind::LuFromD => {
                        convert_binary_to_integer(
                            self.f[instruction.rs1 as usize],
                            FORMAT_D,
                            matches!(instruction.kind, FloatConversionKind::LFromD),
                            64,
                            rounding_mode,
                        )
                    }
                    FloatConversionKind::SFromW | FloatConversionKind::SFromWu => {
                        let value = self.x[instruction.rs1 as usize];
                        let (negative, magnitude) =
                            if matches!(instruction.kind, FloatConversionKind::SFromW) {
                                let signed = value as i32;
                                (signed < 0, signed.unsigned_abs() as u64)
                            } else {
                                (false, value as u32 as u64)
                            };
                        let (result, flags) =
                            convert_integer_to_binary(negative, magnitude, FORMAT_S, rounding_mode);
                        (0xffff_ffff_0000_0000 | (result & 0xffff_ffff), flags)
                    }
                    FloatConversionKind::DFromW | FloatConversionKind::DFromWu => {
                        let value = self.x[instruction.rs1 as usize];
                        let (negative, magnitude) =
                            if matches!(instruction.kind, FloatConversionKind::DFromW) {
                                let signed = value as i32;
                                (signed < 0, signed.unsigned_abs() as u64)
                            } else {
                                (false, value as u32 as u64)
                            };
                        convert_integer_to_binary(negative, magnitude, FORMAT_D, rounding_mode)
                    }
                    FloatConversionKind::SFromL | FloatConversionKind::SFromLu => {
                        let value = self.x[instruction.rs1 as usize];
                        let (negative, magnitude) =
                            if matches!(instruction.kind, FloatConversionKind::SFromL) {
                                let signed = value as i64;
                                (signed < 0, signed.unsigned_abs())
                            } else {
                                (false, value)
                            };
                        let (result, flags) =
                            convert_integer_to_binary(negative, magnitude, FORMAT_S, rounding_mode);
                        (0xffff_ffff_0000_0000 | (result & 0xffff_ffff), flags)
                    }
                    FloatConversionKind::DFromL | FloatConversionKind::DFromLu => {
                        let value = self.x[instruction.rs1 as usize];
                        let (negative, magnitude) =
                            if matches!(instruction.kind, FloatConversionKind::DFromL) {
                                let signed = value as i64;
                                (signed < 0, signed.unsigned_abs())
                            } else {
                                (false, value)
                            };
                        convert_integer_to_binary(negative, magnitude, FORMAT_D, rounding_mode)
                    }
                };
                if matches!(
                    instruction.kind,
                    FloatConversionKind::SFromD
                        | FloatConversionKind::DFromS
                        | FloatConversionKind::SFromW
                        | FloatConversionKind::SFromWu
                        | FloatConversionKind::DFromW
                        | FloatConversionKind::DFromWu
                        | FloatConversionKind::SFromL
                        | FloatConversionKind::SFromLu
                        | FloatConversionKind::DFromL
                        | FloatConversionKind::DFromLu
                ) {
                    self.f[instruction.rd as usize] = result;
                } else if instruction.rd != 0 {
                    self.x[instruction.rd as usize] = result;
                }
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

const ROUND_RNE: u8 = 0;
const ROUND_RTZ: u8 = 1;
const ROUND_RDN: u8 = 2;
const ROUND_RUP: u8 = 3;
const ROUND_RMM: u8 = 4;

#[derive(Clone, Copy)]
struct BinaryFormat {
    fraction_bits: u32,
    precision: u32,
    bias: i32,
    exponent_mask: u64,
    sign_mask: u64,
    quiet_nan_mask: u64,
    canonical_nan: u64,
}

const FORMAT_S: BinaryFormat = BinaryFormat {
    fraction_bits: 23,
    precision: 24,
    bias: 127,
    exponent_mask: 0x7f80_0000,
    sign_mask: 0x8000_0000,
    quiet_nan_mask: 0x0040_0000,
    canonical_nan: 0x7fc0_0000,
};

const FORMAT_D: BinaryFormat = BinaryFormat {
    fraction_bits: 52,
    precision: 53,
    bias: 1023,
    exponent_mask: 0x7ff0_0000_0000_0000,
    sign_mask: 0x8000_0000_0000_0000,
    quiet_nan_mask: 0x0008_0000_0000_0000,
    canonical_nan: 0x7ff8_0000_0000_0000,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BigUint {
    limbs: Vec<u64>,
}

impl BigUint {
    fn from_u64(value: u64) -> Self {
        if value == 0 {
            Self::default()
        } else {
            Self { limbs: vec![value] }
        }
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn bit_len(&self) -> usize {
        self.limbs.last().map_or(0, |limb| {
            (self.limbs.len() - 1) * 64 + (64 - limb.leading_zeros() as usize)
        })
    }

    fn bit(&self, index: usize) -> bool {
        self.limbs
            .get(index / 64)
            .is_some_and(|limb| limb & (1 << (index % 64)) != 0)
    }

    fn any_below(&self, exclusive: usize) -> bool {
        if exclusive == 0 {
            return false;
        }
        let full_limbs = (exclusive / 64).min(self.limbs.len());
        if self.limbs[..full_limbs].iter().any(|limb| *limb != 0) {
            return true;
        }
        let remaining = exclusive % 64;
        remaining != 0
            && self
                .limbs
                .get(full_limbs)
                .is_some_and(|limb| limb & ((1u64 << remaining) - 1) != 0)
    }

    fn shl_bits(&mut self, shift: usize) {
        if self.is_zero() || shift == 0 {
            return;
        }
        let whole = shift / 64;
        let partial = shift % 64;
        if whole != 0 {
            let old_len = self.limbs.len();
            self.limbs.resize(old_len + whole, 0);
            self.limbs.copy_within(0..old_len, whole);
            self.limbs[..whole].fill(0);
        }
        if partial != 0 {
            let mut carry = 0;
            for limb in &mut self.limbs {
                let next = *limb >> (64 - partial);
                *limb = (*limb << partial) | carry;
                carry = next;
            }
            if carry != 0 {
                self.limbs.push(carry);
            }
        }
    }

    fn add_assign(&mut self, other: &Self) {
        let length = self.limbs.len().max(other.limbs.len());
        self.limbs.resize(length, 0);
        let mut carry = 0u64;
        for index in 0..length {
            let (sum, carry1) =
                self.limbs[index].overflowing_add(other.limbs.get(index).copied().unwrap_or(0));
            let (sum, carry2) = sum.overflowing_add(carry);
            self.limbs[index] = sum;
            carry = u64::from(carry1 || carry2);
        }
        if carry != 0 {
            self.limbs.push(carry);
        }
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.limbs.len().cmp(&other.limbs.len()) {
            std::cmp::Ordering::Equal => self.limbs.iter().rev().cmp(other.limbs.iter().rev()),
            ordering => ordering,
        }
    }

    fn sub_assign(&mut self, other: &Self) {
        debug_assert!(self.cmp(other) != std::cmp::Ordering::Less);
        let mut borrow = 0u64;
        for index in 0..self.limbs.len() {
            let right = other.limbs.get(index).copied().unwrap_or(0);
            let (difference, borrow1) = self.limbs[index].overflowing_sub(right);
            let (difference, borrow2) = difference.overflowing_sub(borrow);
            self.limbs[index] = difference;
            borrow = u64::from(borrow1 || borrow2);
        }
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    fn low_u64(&self) -> u64 {
        self.limbs.first().copied().unwrap_or(0)
    }

    fn shifted_u64(&self, shift: usize) -> u64 {
        (0..64).fold(0, |value, bit| {
            value | u64::from(self.bit(shift + bit)) << bit
        })
    }
}

fn is_valid_rounding_mode(rounding_mode: u8) -> bool {
    rounding_mode <= ROUND_RMM
}

fn add_s(left: u32, right: u32, rounding_mode: u8) -> (u32, u32) {
    let (result, flags) = add_binary(u64::from(left), u64::from(right), FORMAT_S, rounding_mode);
    (result as u32, flags)
}

fn add_d(left: u64, right: u64, rounding_mode: u8) -> (u64, u32) {
    add_binary(left, right, FORMAT_D, rounding_mode)
}

fn convert_binary(
    value: u64,
    source: BinaryFormat,
    destination: BinaryFormat,
    rounding_mode: u8,
) -> (u64, u32) {
    if is_nan_binary(value, source) {
        return (
            destination.canonical_nan,
            if is_signaling_nan_binary(value, source) {
                FFLAG_NV
            } else {
                0
            },
        );
    }
    if is_infinite_binary(value, source) {
        return (
            if value & source.sign_mask != 0 {
                destination.sign_mask | destination.exponent_mask
            } else {
                destination.exponent_mask
            },
            0,
        );
    }
    let negative = value & source.sign_mask != 0;
    let (significand, exponent) = finite_components(value, source);
    round_binary(
        BigUint::from_u64(significand),
        exponent,
        negative,
        destination,
        rounding_mode,
    )
}

fn convert_integer_to_binary(
    negative: bool,
    magnitude: u64,
    destination: BinaryFormat,
    rounding_mode: u8,
) -> (u64, u32) {
    round_binary(
        BigUint::from_u64(magnitude),
        0,
        negative && magnitude != 0,
        destination,
        rounding_mode,
    )
}

fn convert_binary_to_integer(
    value: u64,
    source: BinaryFormat,
    signed: bool,
    bits: u32,
    rounding_mode: u8,
) -> (u64, u32) {
    let negative = value & source.sign_mask != 0;
    if is_nan_binary(value, source) || is_infinite_binary(value, source) {
        return (invalid_integer_result(signed, negative, bits), FFLAG_NV);
    }
    let (significand, exponent) = finite_components(value, source);
    let (magnitude, inexact) = if exponent >= 0 {
        let mut magnitude = BigUint::from_u64(significand);
        magnitude.shl_bits(exponent as usize);
        (magnitude, false)
    } else {
        let (rounded, inexact) = round_big(
            &BigUint::from_u64(significand),
            (-exponent) as usize,
            negative,
            rounding_mode,
        );
        (BigUint::from_u64(rounded), inexact)
    };

    let max_positive = BigUint::from_u64(if bits == 32 {
        0x7fff_ffff
    } else {
        0x7fff_ffff_ffff_ffff
    });
    let min_magnitude = BigUint::from_u64(if bits == 32 {
        0x8000_0000
    } else {
        0x8000_0000_0000_0000
    });
    let max_unsigned = BigUint::from_u64(if bits == 32 { 0xffff_ffff } else { u64::MAX });
    let invalid = if signed {
        if negative {
            magnitude.cmp(&min_magnitude) == std::cmp::Ordering::Greater
        } else {
            magnitude.cmp(&max_positive) == std::cmp::Ordering::Greater
        }
    } else {
        negative && !magnitude.is_zero()
            || magnitude.cmp(&max_unsigned) == std::cmp::Ordering::Greater
    };
    if invalid {
        return (invalid_integer_result(signed, negative, bits), FFLAG_NV);
    }

    let result = if signed {
        let raw = if negative {
            0u64.wrapping_sub(magnitude.low_u64())
        } else {
            magnitude.low_u64()
        };
        if bits == 32 {
            sign_extend_32(raw as u32)
        } else {
            raw
        }
    } else {
        if bits == 32 {
            sign_extend_32(magnitude.low_u64() as u32)
        } else {
            magnitude.low_u64()
        }
    };
    (result, if inexact { FFLAG_NX } else { 0 })
}

fn invalid_integer_result(signed: bool, negative: bool, bits: u32) -> u64 {
    if bits == 32 {
        if signed && negative {
            sign_extend_32(0x8000_0000)
        } else if signed {
            sign_extend_32(0x7fff_ffff)
        } else if negative {
            0
        } else {
            sign_extend_32(0xffff_ffff)
        }
    } else if signed && negative {
        0x8000_0000_0000_0000
    } else if signed {
        0x7fff_ffff_ffff_ffff
    } else if negative {
        0
    } else {
        u64::MAX
    }
}

fn sign_extend_32(value: u32) -> u64 {
    (value as i32 as i64) as u64
}

fn add_binary(left: u64, right: u64, format: BinaryFormat, rounding_mode: u8) -> (u64, u32) {
    let left_nan = is_nan_binary(left, format);
    let right_nan = is_nan_binary(right, format);
    if (left_nan && is_signaling_nan_binary(left, format))
        || (right_nan && is_signaling_nan_binary(right, format))
    {
        return (format.canonical_nan, FFLAG_NV);
    }
    if left_nan || right_nan {
        return (format.canonical_nan, 0);
    }
    let left_infinite = is_infinite_binary(left, format);
    let right_infinite = is_infinite_binary(right, format);
    if left_infinite && right_infinite && (left ^ right) & format.sign_mask != 0 {
        return (format.canonical_nan, FFLAG_NV);
    }
    if left_infinite {
        return (left, 0);
    }
    if right_infinite {
        return (right, 0);
    }

    let left_sign = left & format.sign_mask != 0;
    let right_sign = right & format.sign_mask != 0;
    let (left_significand, left_exponent) = finite_components(left, format);
    let (right_significand, right_exponent) = finite_components(right, format);
    let common_exponent = left_exponent.min(right_exponent);
    let mut left_magnitude = BigUint::from_u64(left_significand);
    let mut right_magnitude = BigUint::from_u64(right_significand);
    left_magnitude.shl_bits((left_exponent - common_exponent) as usize);
    right_magnitude.shl_bits((right_exponent - common_exponent) as usize);

    let (negative, magnitude) = if left_sign == right_sign {
        left_magnitude.add_assign(&right_magnitude);
        (left_sign, left_magnitude)
    } else {
        match left_magnitude.cmp(&right_magnitude) {
            std::cmp::Ordering::Greater => {
                left_magnitude.sub_assign(&right_magnitude);
                (left_sign, left_magnitude)
            }
            std::cmp::Ordering::Less => {
                right_magnitude.sub_assign(&left_magnitude);
                (right_sign, right_magnitude)
            }
            std::cmp::Ordering::Equal => (rounding_mode == ROUND_RDN, BigUint::default()),
        }
    };
    round_binary(magnitude, common_exponent, negative, format, rounding_mode)
}

fn finite_components(value: u64, format: BinaryFormat) -> (u64, i32) {
    let fraction_mask = (1u64 << format.fraction_bits) - 1;
    let fraction = value & fraction_mask;
    let exponent_field = (value & format.exponent_mask) >> format.fraction_bits;
    if exponent_field == 0 {
        (fraction, 1 - format.bias - format.fraction_bits as i32)
    } else {
        (
            (1u64 << format.fraction_bits) | fraction,
            exponent_field as i32 - format.bias - format.fraction_bits as i32,
        )
    }
}

fn round_binary(
    magnitude: BigUint,
    exponent: i32,
    negative: bool,
    format: BinaryFormat,
    rounding_mode: u8,
) -> (u64, u32) {
    if magnitude.is_zero() {
        return (if negative { format.sign_mask } else { 0 }, 0);
    }
    let precision = format.precision as i32;
    let emin = 1 - format.bias;
    let emax = format.bias;
    let exponent_of_most_significant = exponent + magnitude.bit_len() as i32 - 1;
    if exponent_of_most_significant > emax {
        return overflow_result(negative, format, rounding_mode);
    }

    let sign_bit = if negative { format.sign_mask } else { 0 };
    if exponent_of_most_significant >= emin {
        let shift = magnitude.bit_len() as i32 - precision;
        let (mut significand, inexact) = if shift > 0 {
            round_big(&magnitude, shift as usize, negative, rounding_mode)
        } else {
            let mut shifted = magnitude;
            shifted.shl_bits((-shift) as usize);
            (shifted.low_u64(), false)
        };
        let mut exponent_field = exponent_of_most_significant;
        if significand == 1u64 << format.precision {
            significand >>= 1;
            exponent_field += 1;
        }
        if exponent_field > emax {
            return overflow_result(negative, format, rounding_mode);
        }
        let fraction_mask = (1u64 << format.fraction_bits) - 1;
        let fraction = significand & fraction_mask;
        let encoded_exponent = ((exponent_field + format.bias) as u64) << format.fraction_bits;
        return (
            sign_bit | encoded_exponent | fraction,
            u32::from(inexact) * FFLAG_NX,
        );
    }

    let subnormal_exponent = emin - (precision - 1);
    let shift = subnormal_exponent - exponent;
    let (significand, inexact) = if shift > 0 {
        round_big(&magnitude, shift as usize, negative, rounding_mode)
    } else {
        let mut shifted = magnitude;
        shifted.shl_bits((-shift) as usize);
        (shifted.low_u64(), false)
    };
    let minimum_normal_significand = 1u64 << (format.precision - 1);
    if significand >= minimum_normal_significand {
        let result = sign_bit | (1u64 << format.fraction_bits);
        return (result, u32::from(inexact) * FFLAG_NX);
    }
    let flags = if inexact { FFLAG_UF | FFLAG_NX } else { 0 };
    (sign_bit | significand, flags)
}

fn round_big(value: &BigUint, shift: usize, negative: bool, rounding_mode: u8) -> (u64, bool) {
    let mut significand = value.shifted_u64(shift);
    let inexact = value.any_below(shift);
    if !inexact {
        return (significand, false);
    }
    let half = shift != 0 && value.bit(shift - 1);
    let below_half = shift > 1 && value.any_below(shift - 1);
    let increment = match rounding_mode {
        ROUND_RNE => half && (below_half || significand & 1 != 0),
        ROUND_RTZ => false,
        ROUND_RDN => negative,
        ROUND_RUP => !negative,
        ROUND_RMM => half,
        _ => false,
    };
    if increment {
        significand += 1;
    }
    (significand, true)
}

fn overflow_result(negative: bool, format: BinaryFormat, rounding_mode: u8) -> (u64, u32) {
    let sign_bit = if negative { format.sign_mask } else { 0 };
    let infinity_field = format.exponent_mask;
    let max_finite_exponent = infinity_field - (1u64 << format.fraction_bits);
    let max_fraction = (1u64 << format.fraction_bits) - 1;
    let directed_to_infinity = match rounding_mode {
        ROUND_RTZ => false,
        ROUND_RDN => negative,
        ROUND_RUP => !negative,
        _ => true,
    };
    let exponent = if directed_to_infinity {
        infinity_field
    } else {
        max_finite_exponent
    };
    (
        sign_bit
            | exponent
            | if directed_to_infinity {
                0
            } else {
                max_fraction
            },
        FFLAG_OF | FFLAG_NX,
    )
}

fn is_nan_binary(value: u64, format: BinaryFormat) -> bool {
    value & format.exponent_mask == format.exponent_mask
        && value & ((1u64 << format.fraction_bits) - 1) != 0
}

fn is_signaling_nan_binary(value: u64, format: BinaryFormat) -> bool {
    is_nan_binary(value, format) && value & format.quiet_nan_mask == 0
}

fn is_infinite_binary(value: u64, format: BinaryFormat) -> bool {
    value & (format.exponent_mask | ((1u64 << format.fraction_bits) - 1)) == format.exponent_mask
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

    fn execute_fadd(
        mnemonic: &str,
        left: u64,
        right: u64,
        rounding_mode: u8,
        frm: u8,
    ) -> (u64, u8) {
        let mut machine = Machine::new(64);
        machine.f[1] = if mnemonic == "fadd.s" {
            0xffff_ffff_0000_0000 | left
        } else {
            left
        };
        machine.f[2] = if mnemonic == "fadd.s" {
            0xffff_ffff_0000_0000 | right
        } else {
            right
        };
        machine.fcsr = u32::from(frm) << 5;
        let word = luna_isa::encode_f_r(
            mnemonic,
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: rounding_mode,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        let result = if mnemonic == "fadd.s" {
            machine.f[3] & 0xffff_ffff
        } else {
            machine.f[3]
        };
        (result, machine.fflags())
    }

    #[test]
    fn fadd_s_honors_static_and_dynamic_rounding_modes() {
        let expected = [
            (ROUND_RNE, 0x3f80_0000),
            (ROUND_RTZ, 0x3f80_0000),
            (ROUND_RDN, 0x3f80_0000),
            (ROUND_RUP, 0x3f80_0001),
            (ROUND_RMM, 0x3f80_0001),
        ];
        for (rounding_mode, expected_bits) in expected {
            let (result, flags) =
                execute_fadd("fadd.s", 0x3f80_0000, 0x3380_0000, rounding_mode, 0);
            assert_eq!(result as u32, expected_bits);
            assert_eq!(flags, FFLAG_NX as u8);
        }

        let (dynamic_result, dynamic_flags) =
            execute_fadd("fadd.s", 0x3f80_0000, 0x3380_0000, 7, ROUND_RUP);
        assert_eq!(dynamic_result as u32, 0x3f80_0001);
        assert_eq!(dynamic_flags, FFLAG_NX as u8);
    }

    #[test]
    fn fadd_d_honors_rounding_and_rejects_reserved_modes() {
        let (result, flags) = execute_fadd(
            "fadd.d",
            0x3ff0_0000_0000_0000,
            0x3ca0_0000_0000_0000,
            ROUND_RUP,
            0,
        );
        assert_eq!(result, 0x3ff0_0000_0000_0001);
        assert_eq!(flags, FFLAG_NX as u8);

        let mut machine = Machine::new(64);
        machine.f[1] = 1.0f32.to_bits() as u64 | 0xffff_ffff_0000_0000;
        machine.f[2] = 0;
        let word = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 5,
            },
        )
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        assert_eq!(machine.step().unwrap_err().code, "TRAP-FP-RM-001");

        machine.pc = 4;
        machine.fcsr = u32::from(6u8) << 5;
        let dynamic_word = luna_isa::encode_f_r(
            "fadd.s",
            luna_isa::FRegisterRType {
                rd: 3,
                rs1: 1,
                rs2: 2,
                rm: 7,
            },
        )
        .unwrap();
        machine.load(4, &dynamic_word.to_le_bytes()).unwrap();
        assert_eq!(machine.step().unwrap_err().code, "TRAP-FP-RM-001");
    }

    #[test]
    fn fflags_follow_riscv_fcsr_bit_positions() {
        assert_eq!(FFLAG_NX, 1 << 0);
        assert_eq!(FFLAG_UF, 1 << 1);
        assert_eq!(FFLAG_OF, 1 << 2);
        assert_eq!(FFLAG_DZ, 1 << 3);
        assert_eq!(FFLAG_NV, 1 << 4);
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

    fn execute_conversion(
        kind: luna_isa::FloatConversionKind,
        value: u64,
        rounding_mode: u8,
        frm: u8,
    ) -> (u64, u8) {
        let mut machine = Machine::new(64);
        machine.f[1] = match kind {
            luna_isa::FloatConversionKind::SFromD => value,
            luna_isa::FloatConversionKind::DFromS => 0xffff_ffff_0000_0000 | (value & 0xffff_ffff),
            _ => panic!("format conversion helper received integer conversion"),
        };
        machine.fcsr = u32::from(frm) << 5;
        let word = luna_isa::encode_f_convert(luna_isa::FloatConversion {
            kind,
            rd: 3,
            rs1: 1,
            rm: rounding_mode,
        })
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        (machine.f[3], machine.fflags())
    }

    fn execute_integer_conversion(
        kind: luna_isa::FloatConversionKind,
        value: u64,
        rounding_mode: u8,
    ) -> (u64, u64, u8) {
        let mut machine = Machine::new(64);
        match kind {
            luna_isa::FloatConversionKind::WFromS | luna_isa::FloatConversionKind::WuFromS => {
                machine.f[1] = 0xffff_ffff_0000_0000 | (value & 0xffff_ffff);
            }
            luna_isa::FloatConversionKind::WFromD | luna_isa::FloatConversionKind::WuFromD => {
                machine.f[1] = value
            }
            luna_isa::FloatConversionKind::LFromS | luna_isa::FloatConversionKind::LuFromS => {
                machine.f[1] = 0xffff_ffff_0000_0000 | (value & 0xffff_ffff);
            }
            luna_isa::FloatConversionKind::LFromD | luna_isa::FloatConversionKind::LuFromD => {
                machine.f[1] = value
            }
            luna_isa::FloatConversionKind::SFromW
            | luna_isa::FloatConversionKind::SFromWu
            | luna_isa::FloatConversionKind::DFromW
            | luna_isa::FloatConversionKind::DFromWu
            | luna_isa::FloatConversionKind::SFromL
            | luna_isa::FloatConversionKind::SFromLu
            | luna_isa::FloatConversionKind::DFromL
            | luna_isa::FloatConversionKind::DFromLu => machine.x[1] = value,
            _ => panic!("integer conversion helper received format conversion"),
        }
        let word = luna_isa::encode_f_convert(luna_isa::FloatConversion {
            kind,
            rd: 3,
            rs1: 1,
            rm: rounding_mode,
        })
        .unwrap();
        machine.load(0, &word.to_le_bytes()).unwrap();
        machine.step().unwrap();
        (machine.x[3], machine.f[3], machine.fflags())
    }

    #[test]
    fn executes_w_integer_float_conversions_with_rounding_and_bounds() {
        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::WFromS,
            1.75f32.to_bits() as u64,
            ROUND_RNE,
        );
        assert_eq!(integer, 2);
        assert_eq!(flags, FFLAG_NX as u8);

        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::WFromS,
            (-1.75f32).to_bits() as u64,
            ROUND_RDN,
        );
        assert_eq!(integer, u64::MAX - 1);
        assert_eq!(flags, FFLAG_NX as u8);

        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::WFromD,
            f64::INFINITY.to_bits(),
            ROUND_RNE,
        );
        assert_eq!(integer, 0x0000_0000_7fff_ffff);
        assert_eq!(flags, FFLAG_NV as u8);
    }

    #[test]
    fn executes_unsigned_w_and_integer_to_float_conversions() {
        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::WuFromS,
            (-1.0f32).to_bits() as u64,
            ROUND_RNE,
        );
        assert_eq!(integer, 0);
        assert_eq!(flags, FFLAG_NV as u8);

        let (_, float, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::SFromWu,
            0xffff_ffff,
            ROUND_RNE,
        );
        assert_eq!(float, 0xffff_ffff_4f80_0000);
        assert_eq!(flags, FFLAG_NX as u8);

        let (_, float, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::DFromW,
            0x0000_0000_8000_0000,
            ROUND_RNE,
        );
        assert_eq!(float, (-2147483648i64 as f64).to_bits());
        assert_eq!(flags, 0);
    }

    #[test]
    fn executes_l_integer_float_conversions_at_rv64_boundaries() {
        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::LFromS,
            1.75f32.to_bits() as u64,
            ROUND_RNE,
        );
        assert_eq!(integer, 2);
        assert_eq!(flags, FFLAG_NX as u8);

        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::LuFromS,
            (-1.0f32).to_bits() as u64,
            ROUND_RNE,
        );
        assert_eq!(integer, 0);
        assert_eq!(flags, FFLAG_NV as u8);

        let (_, float, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::DFromL,
            i64::MIN as u64,
            ROUND_RNE,
        );
        assert_eq!(float, (i64::MIN as f64).to_bits());
        assert_eq!(flags, 0);

        let (_, float, flags) =
            execute_integer_conversion(luna_isa::FloatConversionKind::DFromLu, u64::MAX, ROUND_RNE);
        assert_eq!(float, 0x43f0_0000_0000_0000);
        assert_eq!(flags, FFLAG_NX as u8);

        let (integer, _, flags) = execute_integer_conversion(
            luna_isa::FloatConversionKind::LFromD,
            f64::INFINITY.to_bits(),
            ROUND_RNE,
        );
        assert_eq!(integer, 0x7fff_ffff_ffff_ffff);
        assert_eq!(flags, FFLAG_NV as u8);
    }

    #[test]
    fn executes_float_format_conversions_with_exact_bits_and_rounding() {
        let (result, flags) = execute_conversion(
            luna_isa::FloatConversionKind::SFromD,
            1.5f64.to_bits(),
            7,
            ROUND_RNE,
        );
        assert_eq!(result, 0xffff_ffff_3fc0_0000);
        assert_eq!(flags, 0);

        let halfway = 0x3ff0_0000_1000_0000;
        let (rne, rne_flags) =
            execute_conversion(luna_isa::FloatConversionKind::SFromD, halfway, ROUND_RNE, 0);
        let (rup, rup_flags) =
            execute_conversion(luna_isa::FloatConversionKind::SFromD, halfway, ROUND_RUP, 0);
        assert_eq!(rne, 0xffff_ffff_3f80_0000);
        assert_eq!(rup, 0xffff_ffff_3f80_0001);
        assert_eq!(rne_flags, FFLAG_NX as u8);
        assert_eq!(rup_flags, FFLAG_NX as u8);

        let (result, flags) = execute_conversion(
            luna_isa::FloatConversionKind::DFromS,
            1.5f32.to_bits() as u64,
            7,
            ROUND_RNE,
        );
        assert_eq!(result, 1.5f64.to_bits());
        assert_eq!(flags, 0);
    }

    #[test]
    fn executes_float_moves_without_changing_bits_or_flags() {
        let mut machine = Machine::new(16);
        machine.f[1] = 0xffff_ffff_8000_0001;
        machine
            .load(
                0,
                &luna_isa::encode_f_move(luna_isa::FloatMove {
                    kind: luna_isa::FloatMoveKind::XFromW,
                    rd: 2,
                    rs1: 1,
                })
                .unwrap()
                .to_le_bytes(),
            )
            .unwrap();
        machine.step().unwrap();
        assert_eq!(machine.x[2], 0xffff_ffff_8000_0001);
        assert_eq!(machine.fflags(), 0);

        machine = Machine::new(16);
        machine.x[1] = 0x1234_5678_8765_4321;
        machine
            .load(
                0,
                &luna_isa::encode_f_move(luna_isa::FloatMove {
                    kind: luna_isa::FloatMoveKind::WFromX,
                    rd: 2,
                    rs1: 1,
                })
                .unwrap()
                .to_le_bytes(),
            )
            .unwrap();
        machine.step().unwrap();
        assert_eq!(machine.f[2], 0xffff_ffff_8765_4321);
        assert_eq!(machine.fflags(), 0);

        machine = Machine::new(16);
        machine.f[1] = 0x7ff8_0000_0000_0042;
        machine
            .load(
                0,
                &luna_isa::encode_f_move(luna_isa::FloatMove {
                    kind: luna_isa::FloatMoveKind::XFromD,
                    rd: 2,
                    rs1: 1,
                })
                .unwrap()
                .to_le_bytes(),
            )
            .unwrap();
        machine.step().unwrap();
        assert_eq!(machine.x[2], 0x7ff8_0000_0000_0042);

        machine = Machine::new(16);
        machine.x[1] = 0x8000_0000_0000_0000;
        machine
            .load(
                0,
                &luna_isa::encode_f_move(luna_isa::FloatMove {
                    kind: luna_isa::FloatMoveKind::DFromX,
                    rd: 2,
                    rs1: 1,
                })
                .unwrap()
                .to_le_bytes(),
            )
            .unwrap();
        machine.step().unwrap();
        assert_eq!(machine.f[2], 0x8000_0000_0000_0000);
        assert_eq!(machine.fflags(), 0);
    }

    #[test]
    fn format_conversion_handles_infinities_and_signaling_nan() {
        let (negative_infinity, flags) = execute_conversion(
            luna_isa::FloatConversionKind::SFromD,
            f64::NEG_INFINITY.to_bits(),
            ROUND_RNE,
            0,
        );
        assert_eq!(negative_infinity, 0xffff_ffff_ff80_0000);
        assert_eq!(flags, 0);

        let (nan, flags) = execute_conversion(
            luna_isa::FloatConversionKind::DFromS,
            0x7f80_0001,
            ROUND_RNE,
            0,
        );
        assert_eq!(nan, 0x7ff8_0000_0000_0000);
        assert_eq!(flags, FFLAG_NV as u8);
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
