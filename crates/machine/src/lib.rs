#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};
use luna_isa::{Instruction, decode};
use luna_memory::Memory;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub x: [u64; 32],
    pub pc: u64,
    pub instructions: u64,
    pub memory: Memory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult {
    pub pc_before: u64,
    pub pc_after: u64,
    pub instruction: Instruction,
}

impl Machine {
    pub fn new(memory_size: usize) -> Self {
        Self {
            x: [0; 32],
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
    pub fn step(&mut self) -> Result<StepResult> {
        let pc_before = self.pc;
        let word = self.memory.load32(self.pc)?;
        let instruction = decode(word);
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
            Instruction::Illegal(_) => {
                return Err(Diagnostic::error(
                    "TRAP-ILLEGAL-INSTRUCTION",
                    "illegal instruction",
                ));
            }
        }
        self.x[0] = 0;
        self.pc = self
            .pc
            .checked_add(4)
            .ok_or_else(|| Diagnostic::error("TRAP-PC-OVERFLOW", "program counter overflow"))?;
        self.instructions += 1;
        Ok(StepResult {
            pc_before,
            pc_after: self.pc,
            instruction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luna_isa::{Addi, Lui, RType, encode_addi, encode_lui, encode_r};

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
    }
}
