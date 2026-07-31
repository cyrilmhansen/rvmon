#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write;

use luna_assembler::assemble;
use luna_diag::{Diagnostic, Result};
use luna_disassembler::disassemble_word;
use luna_machine::{FFLAG_DZ, FFLAG_NV, FFLAG_NX, FFLAG_OF, FFLAG_UF, Machine};

pub struct Monitor {
    pub machine: Machine,
    symbols: BTreeMap<u64, String>,
    max_run_steps: u64,
}

impl Monitor {
    pub fn new(memory_size: usize) -> Self {
        Self {
            machine: Machine::new(memory_size),
            symbols: BTreeMap::new(),
            max_run_steps: 1000,
        }
    }

    pub fn execute(&mut self, command: &str) -> Result<String> {
        let command = command.trim();
        if command.is_empty() {
            return Ok(String::new());
        }
        let (name, argument) = command
            .split_once(char::is_whitespace)
            .map_or((command, ""), |(name, argument)| (name, argument.trim()));
        match name.to_ascii_lowercase().as_str() {
            "help" | "?" => Ok(help()),
            "regs" | "registers" => self.registers(),
            "assemble" | "a" => self.assemble(argument),
            "step" | "s" => self.step(),
            "run" | "r" => self.run(argument),
            "disasm" | "d" => self.disassemble(argument),
            "reset" => {
                self.machine = Machine::new(self.machine.memory_size());
                self.symbols.clear();
                Ok("machine reset".into())
            }
            "quit" | "exit" => Ok("bye".into()),
            _ => Err(Diagnostic::error(
                "MON-CMD-001",
                format!("unknown command: {name}; use help"),
            )),
        }
    }

    fn assemble(&mut self, source: &str) -> Result<String> {
        if source.is_empty() {
            return Err(Diagnostic::error(
                "MON-ASM-001",
                "assemble expects one source line",
            ));
        }
        let image = assemble(source)?;
        let address = self.machine.pc;
        self.machine.load(address, &image.text)?;
        let word = image
            .text
            .get(..4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four-byte instruction")));
        Ok(match word {
            Some(word) => format!("loaded 0x{word:08x} at 0x{address:016x}"),
            None => format!("loaded {} bytes at 0x{address:016x}", image.text.len()),
        })
    }

    fn step(&mut self) -> Result<String> {
        let address = self.machine.pc;
        let word = self.machine.memory.load32(address)?;
        let line = disassemble_word(address, word, &self.symbols);
        let result = self.machine.step()?;
        Ok(format!(
            "0x{address:016x}: {word:08x}  {:<28} -> pc=0x{:016x}",
            line.text, result.pc_after
        ))
    }

    fn run(&mut self, argument: &str) -> Result<String> {
        let limit = if argument.is_empty() {
            self.max_run_steps
        } else {
            argument.parse::<u64>().map_err(|_| {
                Diagnostic::error("MON-RUN-001", "run count must be an unsigned integer")
            })?
        };
        let start = self.machine.instructions;
        for _ in 0..limit {
            self.machine.step()?;
        }
        Ok(format!(
            "ran {} step(s); pc=0x{:016x}; total={}",
            self.machine.instructions - start,
            self.machine.pc,
            self.machine.instructions
        ))
    }

    fn disassemble(&self, argument: &str) -> Result<String> {
        let count = if argument.is_empty() {
            4
        } else {
            argument.parse::<usize>().map_err(|_| {
                Diagnostic::error("MON-DISASM-001", "disasm count must be an unsigned integer")
            })?
        };
        let mut output = String::new();
        for index in 0..count {
            let offset = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(4))
                .ok_or_else(|| Diagnostic::error("MON-DISASM-002", "address overflow"))?;
            let address = self
                .machine
                .pc
                .checked_add(offset)
                .ok_or_else(|| Diagnostic::error("MON-DISASM-002", "address overflow"))?;
            let word = self.machine.memory.load32(address)?;
            let line = disassemble_word(address, word, &self.symbols);
            writeln!(output, "0x{address:016x}: {word:08x}  {}", line.text).unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn registers(&self) -> Result<String> {
        let mut output = String::new();
        output.push_str("integer registers\n");
        for row in 0..8 {
            for column in 0..4 {
                let register = row * 4 + column;
                write!(
                    output,
                    "x{register:02}=0x{:016x}  ",
                    self.machine.x[register]
                )
                .unwrap();
            }
            output.push('\n');
        }
        output.push_str("floating registers (raw / binary32 / binary64)\n");
        for row in 0..8 {
            for column in 0..4 {
                let register = (row * 4 + column) as u8;
                let single = self.machine.format_f32(register)?;
                let double = self.machine.format_f64(register)?;
                writeln!(
                    output,
                    "f{register:02}=0x{:016x}  s:{}={} {:?}  d:{}={} {:?}",
                    self.machine.f[register as usize],
                    single.exact_hex,
                    single.shortest_decimal,
                    single.class,
                    double.exact_hex,
                    double.shortest_decimal,
                    double.class
                )
                .unwrap();
            }
        }
        writeln!(
            output,
            "fcsr=0x{:08x} frm={} fflags={}",
            self.machine.fcsr,
            self.machine.frm(),
            format_flags(self.machine.fflags())
        )
        .unwrap();
        Ok(output.trim_end().into())
    }
}

fn format_flags(flags: u8) -> String {
    let mut names = Vec::new();
    for (mask, name) in [
        (FFLAG_NV as u8, "NV"),
        (FFLAG_DZ as u8, "DZ"),
        (FFLAG_OF as u8, "OF"),
        (FFLAG_UF as u8, "UF"),
        (FFLAG_NX as u8, "NX"),
    ] {
        if flags & mask != 0 {
            names.push(name);
        }
    }
    if names.is_empty() {
        "-".into()
    } else {
        names.join("|")
    }
}

fn help() -> String {
    [
        "help                 show commands",
        "assemble <source>    assemble and load one source line at pc",
        "step                 execute one instruction",
        "run [count]          execute up to count instructions (default 1000)",
        "disasm [count]       show instructions from pc (default 4)",
        "regs                 show x/f registers and fcsr exactly",
        "reset                reset machine state",
        "quit                 leave the interactive monitor",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_steps_and_displays_integer_register_changes() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble addi x1,x0,1").unwrap();
        let step = monitor.execute("step").unwrap();
        assert!(step.contains("addi x1,x0,1"));
        assert_eq!(monitor.machine.x[1], 1);
        assert!(
            monitor
                .execute("regs")
                .unwrap()
                .contains("x01=0x0000000000000001")
        );
    }

    #[test]
    fn displays_floating_result_bits_and_flags() {
        let mut monitor = Monitor::new(128);
        monitor.machine.f[1] = 1.5f32.to_bits() as u64 | 0xffff_ffff_0000_0000;
        monitor.machine.f[2] = 2.25f32.to_bits() as u64 | 0xffff_ffff_0000_0000;
        monitor.execute("assemble fadd.s f3,f1,f2").unwrap();
        monitor.execute("step").unwrap();
        let registers = monitor.execute("regs").unwrap();
        assert!(registers.contains("0x40700000"));
        assert!(registers.contains("s:0x40700000=3.75"));
        assert!(registers.contains("fcsr=0x00000000"));
    }

    #[test]
    fn run_has_a_bounded_explicit_budget() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble jal x0,0").unwrap();
        let result = monitor.execute("run 3").unwrap();
        assert!(result.contains("ran 3 step(s)"));
        assert_eq!(monitor.machine.instructions, 3);
    }
}
