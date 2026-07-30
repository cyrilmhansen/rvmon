#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_isa::{decode, Instruction};
use luna_memory::Memory;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine { pub x: [u64; 32], pub pc: u64, pub instructions: u64, pub memory: Memory }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult { pub pc_before: u64, pub pc_after: u64, pub instruction: Instruction }

impl Machine {
    pub fn new(memory_size: usize) -> Self { Self { x: [0; 32], pc: 0, instructions: 0, memory: Memory::new(memory_size) } }
    pub fn load(&mut self, address: u64, bytes: &[u8]) -> Result<()> {
        for (offset, byte) in bytes.iter().enumerate() { self.memory.store8(address + offset as u64, *byte)?; }
        Ok(())
    }
    pub fn step(&mut self) -> Result<StepResult> {
        let pc_before = self.pc;
        let word = self.memory.load32(self.pc)?;
        let instruction = decode(word);
        match instruction {
            Instruction::Addi(addi) => {
                let value = self.x[addi.rs1 as usize].wrapping_add(addi.imm as i64 as u64);
                if addi.rd != 0 { self.x[addi.rd as usize] = value; }
            }
            Instruction::Illegal(_) => return Err(Diagnostic::error("TRAP-ILLEGAL-INSTRUCTION", "illegal instruction")),
        }
        self.x[0] = 0;
        self.pc = self.pc.checked_add(4).ok_or_else(|| Diagnostic::error("TRAP-PC-OVERFLOW", "program counter overflow"))?;
        self.instructions += 1;
        Ok(StepResult { pc_before, pc_after: self.pc, instruction })
    }
}
